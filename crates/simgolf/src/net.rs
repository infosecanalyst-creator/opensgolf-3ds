//! Downloads for Preferences > Content > Find more (docs/MODDING.md 7, docs/CONTENT_INDEX.md): the content index and the
//! packs it lists.
//!
//! A request runs in the background and the game polls it each frame. On the desktop an `https://` (or `http://`) address is
//! fetched by a small HTTPS client in Rust (ureq with rustls) on its own thread, when the game is built with the default
//! `net` feature; in a browser the page fetches it (`fetch()` in web/opensgolf.js). Local addresses (`file://...` or a
//! plain path) are read at once, everywhere, which is also how the tests and a local index work.

/// What a finished request gives: the bytes, or why there are none.
pub type Reply = Result<Vec<u8>, String>;

pub struct Request {
    inner: Inner,
}

enum Inner {
    Done(Option<Reply>),
    #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
    Thread(std::sync::mpsc::Receiver<Reply>),
    #[cfg(target_arch = "wasm32")]
    Page(i32),
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn sg_net_fetch(url: *const u8, len: usize) -> i32;
    fn sg_net_status(id: i32) -> i32;
    fn sg_net_take(id: i32, dst: *mut u8);
    fn sg_net_error(id: i32, dst: *mut u8, cap: usize) -> i32;
}

/// Starts fetching `url`; at most `max` bytes are accepted.
pub fn get(url: &str, max: u64) -> Request {
    if sg_core::index::is_local(url) {
        let r = sg_core::index::read_local(url).and_then(|d| {
            if d.len() as u64 > max {
                Err(sg_core::lang::trf("STR_NET_TOO_BIG", &[("URL", &url)]))
            } else {
                Ok(d)
            }
        });
        return Request { inner: Inner::Done(Some(r)) };
    }
    remote(url, max)
}

/// The download thread's name.
#[allow(dead_code)]
pub const NET_THREAD: &str = "opensgolf-net";

#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
fn remote(url: &str, max: u64) -> Request {
    let (tx, rx) = std::sync::mpsc::channel();
    let url = url.to_string();
    // named: a panic in it ends only the download (on Android the panic hook closes the app for any other thread, android.rs)
    let spawned = std::thread::Builder::new().name(NET_THREAD.into()).spawn(move || {
        let r = (|| -> Reply {
            // GitHub's API refuses requests without a User-Agent ("Check for updates", update.rs)
            let agent = concat!("OpenSGolf/", env!("CARGO_PKG_VERSION"));
            let mut resp = ureq::get(&url).header("User-Agent", agent).call().map_err(|e| format!("{url}: {e}"))?;
            resp.body_mut().with_config().limit(max).read_to_vec().map_err(|e| format!("{url}: {e}"))
        })();
        let _ = tx.send(r);
    });
    match spawned {
        Ok(_) => Request { inner: Inner::Thread(rx) },
        Err(e) => Request { inner: Inner::Done(Some(Err(e.to_string()))) },
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "net")))]
fn remote(url: &str, _max: u64) -> Request {
    Request { inner: Inner::Done(Some(Err(sg_core::lang::trf("STR_NET_NOT_BUILT", &[("URL", &url)])))) }
}

#[cfg(target_arch = "wasm32")]
fn remote(url: &str, _max: u64) -> Request {
    // SAFETY: the page copies the address out of the wasm memory before returning.
    let id = unsafe { sg_net_fetch(url.as_ptr(), url.len()) };
    Request { inner: Inner::Page(id) }
}

impl Request {
    /// The reply once the request has finished (once; None before and after).
    pub fn poll(&mut self) -> Option<Reply> {
        match &mut self.inner {
            Inner::Done(r) => r.take(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
            Inner::Thread(rx) => match rx.try_recv() {
                Ok(r) => {
                    self.inner = Inner::Done(None);
                    Some(r)
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.inner = Inner::Done(None);
                    Some(Err("the download stopped".into()))
                }
            },
            #[cfg(target_arch = "wasm32")]
            Inner::Page(id) => {
                let id = *id;
                // SAFETY: the calls copy into buffers of the sizes given.
                let n = unsafe { sg_net_status(id) };
                let r = match n {
                    -2 => return None,
                    -1 => {
                        let mut buf = vec![0u8; 512];
                        let k = unsafe { sg_net_error(id, buf.as_mut_ptr(), buf.len()) }.clamp(0, 512) as usize;
                        Err(String::from_utf8_lossy(&buf[..k]).into_owned())
                    }
                    n => {
                        let mut buf = vec![0u8; n.max(0) as usize];
                        unsafe { sg_net_take(id, buf.as_mut_ptr()) };
                        Ok(buf)
                    }
                };
                self.inner = Inner::Done(None);
                Some(r)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_requests_finish_at_once() {
        let dir = std::env::temp_dir().join(format!("sg-net-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.bin");
        std::fs::write(&f, b"hello").unwrap();
        let mut r = super::get(&format!("file://{}", f.display()), 100);
        assert_eq!(r.poll(), Some(Ok(b"hello".to_vec())));
        assert_eq!(r.poll(), None);
        assert!(super::get(&f.display().to_string(), 3).poll().unwrap().is_err(), "too big");
        assert!(super::get("/no/such/file", 100).poll().unwrap().is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

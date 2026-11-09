//! Downloads for Preferences > Content > Find more (docs/MODDING.md 7, docs/CONTENT_INDEX.md): the content index and the
//! packs it lists.
//!
//! A request runs in the background and the game polls it each frame. On the desktop an `https://` (or `http://`) address is
//! fetched by a small HTTPS client in Rust (ureq with rustls) on its own thread, when the game is built with the default
//! `net` feature; in a browser the page fetches it (`fetch()` in web/opensgolf.js). Local addresses (`file://...` or a
//! plain path) are read at once, everywhere, which is also how the tests and a local index work.
//!
//! When the Rust client cannot connect (most often an HTTPS-scanning antivirus or a company proxy whose certificate is
//! only in the system's store, not in the client's built-in list), the request is tried once more with the system's
//! `curl` (in Windows 10 and later, macOS and Linux), which trusts the system's certificates (the port's own; GitHub
//! issue #65). Where there is no curl the first error stands. An answer from the server (an HTTP error) is not retried.

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
        let _ = tx.send(fetch(&url, max, &|_, _| {}));
    });
    match spawned {
        Ok(_) => Request { inner: Inner::Thread(rx) },
        Err(e) => Request { inner: Inner::Done(Some(Err(e.to_string()))) },
    }
}

/// GitHub's API refuses requests without a User-Agent ("Check for updates", update.rs).
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
const AGENT: &str = concat!("OpenSGolf/", env!("CARGO_PKG_VERSION"));

/// Why a download failed: an answer that trying again cannot change (an HTTP error, too large), or anything else.
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
enum Failure {
    Final(String),
    Other(String),
}

/// The message for the HTTP error `code`: a refusal for now (GitHub's limit on requests without an account is 60 an
/// hour) or another one.
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
fn status_message(url: &str, code: u16) -> String {
    let key = if matches!(code, 403 | 429) { "STR_NET_BUSY" } else { "STR_NET_HTTP" };
    sg_core::lang::trf(key, &[("URL", &url), ("CODE", &code)])
}

/// Fetches `url` on the calling thread (a worker's: "Update now", autoupdate.rs, and `get`'s thread), at most `max` bytes,
/// calling `progress(bytes so far, the whole size when the server says it)` as the data comes in.
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
pub fn fetch(url: &str, max: u64, progress: &dyn Fn(u64, Option<u64>)) -> Reply {
    if sg_core::index::is_local(url) {
        let d = sg_core::index::read_local(url)?;
        if d.len() as u64 > max {
            return Err(sg_core::lang::trf("STR_NET_TOO_BIG", &[("URL", &url)]));
        }
        progress(d.len() as u64, Some(d.len() as u64));
        return Ok(d);
    }
    match fetch_rust(url, max, progress) {
        Ok(d) => Ok(d),
        Err(Failure::Final(e)) => Err(e),
        Err(Failure::Other(e)) => match fetch_curl(url, max, progress) {
            None => Err(e),
            Some(r) => {
                println!("{e}; tried the system's curl: {}", if r.is_ok() { "done" } else { "failed too" });
                r
            }
        },
    }
}

/// The Rust client (ureq with rustls and its built-in certificates).
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
fn fetch_rust(url: &str, max: u64, progress: &dyn Fn(u64, Option<u64>)) -> Result<Vec<u8>, Failure> {
    use std::io::Read;
    let mut resp = ureq::get(url).header("User-Agent", AGENT).call().map_err(|e| match e {
        ureq::Error::StatusCode(c) => Failure::Final(status_message(url, c)),
        e => Failure::Other(format!("{url}: {e}")),
    })?;
    let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
    if total.is_some_and(|t| t > max) {
        return Err(Failure::Final(sg_core::lang::trf("STR_NET_TOO_BIG", &[("URL", &url)])));
    }
    let mut r = resp.body_mut().with_config().limit(max).reader();
    let mut out = Vec::with_capacity(total.unwrap_or(0).min(max) as usize);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = r.read(&mut buf).map_err(|e| Failure::Other(format!("{url}: {e}")))?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        progress(out.len() as u64, total);
    }
    if total.is_some_and(|t| t != out.len() as u64) {
        return Err(Failure::Other(format!("{url}: the download stopped")));
    }
    Ok(out)
}

/// The system's curl; None where it cannot be started (not installed).
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
fn fetch_curl(url: &str, max: u64, progress: &dyn Fn(u64, Option<u64>)) -> Option<Reply> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("curl");
    // -f: an HTTP error is a failure (exit 22); only https and http, also after a redirect
    cmd.args(["-fsSL", "--proto", "=https,http", "--proto-redir", "=https,http", "--max-filesize", &max.to_string(), "-A", AGENT])
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: no console window flashes up
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| eprintln!("{url}: no system curl ({e})")).ok()?;
    let mut stdout = child.stdout.take()?;
    let mut out = Vec::new();
    let mut buf = vec![0u8; 1 << 16];
    let read = loop {
        match stdout.read(&mut buf) {
            Ok(0) => break Ok(()),
            Ok(n) if (out.len() + n) as u64 > max => break Err(sg_core::lang::trf("STR_NET_TOO_BIG", &[("URL", &url)])),
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                progress(out.len() as u64, None);
            }
            Err(e) => break Err(format!("{url}: {e}")),
        }
    };
    if read.is_err() {
        let _ = child.kill();
    }
    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut err);
    }
    let status = child.wait();
    if let Err(e) = read {
        return Some(Err(e));
    }
    Some(match status {
        Ok(s) if s.success() => Ok(out),
        _ => {
            // "curl: (22) The requested URL returned error: 403"
            let err = err.trim();
            let code = err.rsplit_once("error: ").and_then(|(_, c)| c.trim().parse::<u16>().ok());
            match code {
                Some(c) => Err(status_message(url, c)),
                None => Err(format!("{url}: curl: {}", err.trim_start_matches("curl: "))),
            }
        }
    })
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

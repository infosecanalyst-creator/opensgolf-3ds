//! Bringing the player's SimGolf files into a folder of the game's own (gamefiles.rs): a ZIP or ISO (iso.rs) is unpacked,
//! a folder is copied. The same filtering rules as the browser version's loader (web/opensgolf.js): the game folder is the
//! one that holds `Flics`, at any depth (a zip of the folder above it, of the CD, of an install folder); names are matched
//! without case; a Mac's `__MACOSX` folder and `._` files, the videos (.bik) and the Windows programs are left out.
//!
//! The zip is read from the file entry by entry (a whole game folder is hundreds of MB, too much to hold in a phone's
//! memory): stored and deflated entries, Zip64 for archives over 4 GB. Entries in other methods (Deflate64, LZMA: 7-Zip's
//! and Windows' stronger settings) and encrypted ones are counted and skipped.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Why an import failed.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Not a zip file, or a damaged one (what is wrong).
    BadZip(String),
    /// Invalid or unsupported disc image or installer archive.
    BadDisc(String),
    /// No `Flics` folder anywhere in it: not a SimGolf game folder.
    NoGame,
    /// A Mac disk image in Apple's .dmg format, which the game cannot read (a .cdr can be made instead).
    Dmg,
    /// Reading or writing failed (the system's message).
    Io(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e.to_string())
    }
}

/// True for a path (lowercase, forward slashes) the game does not need.
pub fn skip(low: &str) -> bool {
    let name = low.rsplit('/').next().unwrap_or(low);
    low.split('/').any(|c| c == "__macosx")
        || name.starts_with("._")
        || name == ".ds_store"
        || [".bik", ".exe", ".dll", ".lib"].iter().any(|e| name.ends_with(e))
}

/// The game folder in a list of paths (forward slashes): what comes before the first `Flics` component, "" for the top.
pub fn root_of<'a>(paths: impl IntoIterator<Item = &'a str>) -> Option<String> {
    paths.into_iter().find_map(|p| {
        let parts: Vec<&str> = p.split('/').collect();
        // a folder named Flics, not a file
        let i = parts[..parts.len().saturating_sub(1)].iter().position(|c| c.eq_ignore_ascii_case("flics"))?;
        Some(parts[..i].join("/"))
    })
}

/// A zip entry's name as a relative path, or None when it is unsafe (absolute, `..`) or empty.
pub(crate) fn safe_rel(name: &str) -> Option<PathBuf> {
    let mut p = PathBuf::new();
    for c in name.split('/') {
        match c {
            "" | "." => {}
            ".." => return None,
            c if c.contains(':') => return None,
            c => p.push(c),
        }
    }
    (!p.as_os_str().is_empty()).then_some(p)
}

struct Entry {
    name: String,
    method: u16,
    flags: u16,
    csize: u64,
    size: u64,
    local: u64,
}

fn u16le(d: &[u8], o: usize) -> u64 {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as u64)
}
fn u32le(d: &[u8], o: usize) -> u64 {
    d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64)
}
fn u64le(d: &[u8], o: usize) -> u64 {
    d.get(o..o + 8).map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
}

fn read_at(f: &mut File, at: u64, len: usize) -> Result<Vec<u8>, Error> {
    f.seek(SeekFrom::Start(at))?;
    let mut buf = vec![0; len];
    f.read_exact(&mut buf).map_err(|_| Error::BadZip("cut short".into()))?;
    Ok(buf)
}

/// The central directory: every file entry (folders left out).
fn entries(f: &mut File) -> Result<Vec<Entry>, Error> {
    let len = f.metadata()?.len();
    if len < 22 {
        return Err(Error::BadZip("too short".into()));
    }
    // the end of central directory record: the last "PK\5\6" (a comment of up to 64 KiB may follow it)
    let tail_len = len.min(22 + 0xffff) as usize;
    let tail_at = len - tail_len as u64;
    let tail = read_at(f, tail_at, tail_len)?;
    let e = (0..=tail_len - 22)
        .rev()
        .find(|&e| tail[e..e + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| Error::BadZip("no central directory".into()))?;
    let (mut count, mut cd_size, mut cd_at) = (u16le(&tail, e + 10), u32le(&tail, e + 12), u32le(&tail, e + 16));
    // Zip64: the locator just before the record points to the Zip64 end record, which has the real numbers
    if (count == 0xffff || cd_size == 0xffff_ffff || cd_at == 0xffff_ffff) && e >= 20 && tail[e - 20..e - 16] == [0x50, 0x4b, 0x06, 0x07] {
        let z = read_at(f, u64le(&tail, e - 20 + 8), 56)?;
        if z[..4] != [0x50, 0x4b, 0x06, 0x06] {
            return Err(Error::BadZip("bad Zip64 record".into()));
        }
        (count, cd_size, cd_at) = (u64le(&z, 32), u64le(&z, 40), u64le(&z, 48));
    }
    if cd_at.saturating_add(cd_size) > len {
        return Err(Error::BadZip("the central directory is past the end (a cut-short download?)".into()));
    }
    let cd = read_at(f, cd_at, cd_size as usize)?;
    let mut out = Vec::new();
    let mut p = 0usize;
    for _ in 0..count {
        if cd.get(p..p + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            return Err(Error::BadZip("bad central directory".into()));
        }
        let (nlen, xlen, clen) = (u16le(&cd, p + 28) as usize, u16le(&cd, p + 30) as usize, u16le(&cd, p + 32) as usize);
        let name = String::from_utf8_lossy(cd.get(p + 46..p + 46 + nlen).ok_or_else(|| Error::BadZip("bad central directory".into()))?)
            .replace('\\', "/");
        let mut en = Entry {
            name,
            method: u16le(&cd, p + 10) as u16,
            flags: u16le(&cd, p + 8) as u16,
            csize: u32le(&cd, p + 20),
            size: u32le(&cd, p + 24),
            local: u32le(&cd, p + 42),
        };
        // the Zip64 extra field holds the values that did not fit, in this order
        let mut x = p + 46 + nlen;
        let xend = x + xlen;
        while x + 4 <= xend {
            let (id, n) = (u16le(&cd, x), u16le(&cd, x + 2) as usize);
            if id == 1 {
                let mut o = x + 4;
                for v in [&mut en.size, &mut en.csize, &mut en.local] {
                    if *v == 0xffff_ffff {
                        *v = u64le(&cd, o);
                        o += 8;
                    }
                }
            }
            x += 4 + n;
        }
        p += 46 + nlen + xlen + clen;
        if !en.name.ends_with('/') {
            out.push(en);
        }
    }
    Ok(out)
}

/// What an unpacking did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Unzipped {
    /// Files written.
    pub files: usize,
    /// Files skipped because their compression method is not stored or deflate, or they are encrypted.
    pub unread: usize,
}

/// A CD image's name inside a zip: an .iso, or a Mac's .cdr (the same bytes).
fn cd_image_name(low: &str) -> bool {
    low.ends_with(".iso") || low.ends_with(".cdr")
}

/// Imports a ZIP, a CD image (.iso, or a Mac's .cdr), or a ZIP containing one. CD image detection uses the file's contents
/// so Android's temporary filename need not preserve the extension (and a misnamed image works). A .dmg is refused with
/// its own message. All output is staged and validated by gamefiles before installation.
pub fn unpack_game(source: &Path, dest: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<Unzipped, Error> {
    let mut file = File::open(source)?;
    let mut signature = [0; 2];
    file.read_exact(&mut signature)?;
    if signature != *b"PK" {
        if !crate::iso::is_cd_image(source) && crate::iso::is_dmg(source) {
            return Err(Error::Dmg);
        }
        return crate::iso::unpack(source, dest, progress);
    }
    let all = entries(&mut file)?;
    if root_of(all.iter().filter(|e| !skip(&e.name.to_lowercase())).map(|e| e.name.as_str())).is_some() {
        return unzip_game(source, dest, progress);
    }
    let images: Vec<_> = all.iter().filter(|e| !skip(&e.name.to_lowercase()) && cd_image_name(&e.name.to_lowercase())).collect();
    if images.len() != 1 {
        return Err(if images.is_empty() {
            if all.iter().any(|e| e.name.to_lowercase().ends_with(".dmg")) {
                Error::Dmg
            } else {
                Error::NoGame
            }
        } else {
            Error::BadDisc("ZIP contains several CD images; choose the SimGolf disc image itself".into())
        });
    }
    let image = images[0];
    if image.flags & 1 != 0 || ![0, 8].contains(&image.method) {
        return Err(Error::BadZip("the ISO is encrypted or uses unsupported compression".into()));
    }
    let work = crate::iso::Workspace::new(dest)?;
    let head = read_at(&mut file, image.local, 30)?;
    if head[..4] != [0x50, 0x4b, 3, 4] || image.size > 8 * 1024 * 1024 * 1024 {
        return Err(Error::BadZip("invalid ISO entry".into()));
    }
    file.seek(SeekFrom::Start(image.local + 30 + u16le(&head, 26) + u16le(&head, 28)))?;
    let input = file.take(image.csize);
    let mut reader: Box<dyn Read> = if image.method == 8 { Box::new(flate2::read::DeflateDecoder::new(input)) } else { Box::new(input) };
    let path = work.path.join("disc.iso");
    let mut output = File::create(&path)?;
    let mut copied = 0u64;
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| Error::BadZip(e.to_string()))?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > image.size {
            return Err(Error::BadZip("ISO exceeds its declared size".into()));
        }
        std::io::Write::write_all(&mut output, &buffer[..n])?;
        progress((copied / 1048576) as usize, image.size.div_ceil(1048576) as usize);
    }
    if copied != image.size {
        return Err(Error::BadZip("ISO entry is cut short".into()));
    }
    drop(output);
    crate::iso::unpack(&path, dest, progress)
}

/// Unpacks the game folder in `zip` into `dest` (made empty first): the files under the folder that holds `Flics`, that
/// folder's contents at `dest`'s top. `progress(done, total)` is called as entries are written.
pub fn unzip_game(zip: &Path, dest: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<Unzipped, Error> {
    let mut f = File::open(zip)?;
    let all = entries(&mut f)?;
    let wanted: Vec<&Entry> = all.iter().filter(|e| !skip(&e.name.to_lowercase())).collect();
    let root = root_of(wanted.iter().map(|e| e.name.as_str())).ok_or(Error::NoGame)?;
    let prefix = if root.is_empty() { String::new() } else { format!("{root}/") };
    let wanted: Vec<&Entry> = wanted.into_iter().filter(|e| e.name.starts_with(&prefix)).collect();
    let _ = std::fs::remove_dir_all(dest);
    std::fs::create_dir_all(dest)?;
    let mut done = Unzipped::default();
    for (i, e) in wanted.iter().enumerate() {
        progress(i, wanted.len());
        let Some(rel) = safe_rel(&e.name[prefix.len()..]) else { continue };
        if e.flags & 1 != 0 || (e.method != 0 && e.method != 8) {
            done.unread += 1;
            continue;
        }
        let head = read_at(&mut f, e.local, 30)?;
        if head[..4] != [0x50, 0x4b, 0x03, 0x04] {
            return Err(Error::BadZip(format!("bad entry {}", e.name)));
        }
        let start = e.local + 30 + u16le(&head, 26) + u16le(&head, 28);
        let body = read_at(&mut f, start, e.csize as usize)?;
        let data = if e.method == 0 {
            body
        } else {
            miniz_oxide::inflate::decompress_to_vec_with_limit(&body, e.size as usize + 1024)
                .map_err(|_| Error::BadZip(format!("damaged entry {}", e.name)))?
        };
        let out = dest.join(rel);
        if let Some(d) = out.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&out, data)?;
        done.files += 1;
    }
    progress(wanted.len(), wanted.len());
    Ok(done)
}

/// Copies the folder `src` into `dest` (made empty first), leaving out what the game does not need. `progress(files)` is
/// called as files are copied. Returns the number of files copied.
pub fn copy_tree(src: &Path, dest: &Path, progress: &mut dyn FnMut(usize)) -> Result<usize, Error> {
    let _ = std::fs::remove_dir_all(dest);
    std::fs::create_dir_all(dest)?;
    let mut n = 0;
    let mut stack = vec![(src.to_path_buf(), dest.to_path_buf())];
    while let Some((from, to)) = stack.pop() {
        for e in std::fs::read_dir(&from)? {
            let e = e?;
            let name = e.file_name();
            let low = name.to_string_lossy().to_lowercase();
            let kind = e.file_type()?;
            if kind.is_dir() {
                if low != "__macosx" {
                    std::fs::create_dir_all(to.join(&name))?;
                    stack.push((e.path(), to.join(&name)));
                }
            } else if !skip(&low) {
                std::fs::copy(e.path(), to.join(&name))?;
                n += 1;
                if n % 25 == 0 {
                    progress(n);
                }
            }
        }
    }
    progress(n);
    Ok(n)
}

/// Puts the folder `staging` in the place of `dest` (whatever was there goes).
pub fn install(staging: &Path, dest: &Path) -> Result<(), Error> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::rename(staging, dest)?;
    Ok(())
}

/// The folder an import is unpacked into before it takes `dest`'s place: beside it, so the move is a rename.
pub fn staging_for(dest: &Path) -> PathBuf {
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "game".into());
    dest.with_file_name(format!("{name}.importing"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opensgolf-import-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn rules_match_the_browser_loader() {
        assert!(skip("__macosx/simgolf/flics/a.fli"));
        assert!(skip("simgolf/._titlebase.pcx"));
        assert!(skip("simgolf/movies/intro.bik"));
        assert!(skip("simgolf/simgolf.exe"));
        assert!(!skip("simgolf/flics/a.fli"));
        assert_eq!(root_of(["a/b.txt", "Top/SimGolf/FLICS/x.fli"]), Some("Top/SimGolf".into()));
        assert_eq!(root_of(["Flics/x.fli"]), Some(String::new()));
        assert_eq!(root_of(["a/flics"]), None);
        assert_eq!(safe_rel("../evil"), None);
        assert_eq!(safe_rel("/x/./y"), Some(PathBuf::from("x/y")));
    }

    #[test]
    fn unzips_the_game_folder_from_any_depth() {
        let d = tmp("unzip");
        let files: Vec<(&str, &[u8])> = vec![
            ("__MACOSX/SimGolf Files/Program_Files_(ENGLISH)/Flics/._a.fli", b"fork"),
            ("SimGolf Files/readme.txt", b"outside"),
            ("SimGolf Files/Program_Files_(ENGLISH)/Flics/a.fli", b"flic"),
            ("SimGolf Files/Program_Files_(ENGLISH)/interface/TitleBASE.pcx", b"title"),
            ("SimGolf Files/Program_Files_(ENGLISH)/Data/Textures/t.pcx", b"tex"),
            ("SimGolf Files/Program_Files_(ENGLISH)/Themes/x.dat", b"theme"),
            ("SimGolf Files/Program_Files_(ENGLISH)/Movies/intro.bik", b"video"),
            ("SimGolf Files/Program_Files_(ENGLISH)/../../escape.txt", b"no"),
        ];
        let zip = d.join("SimGolf.zip");
        std::fs::write(&zip, sg_core::zip::write(&files)).unwrap();
        let dest = d.join("game");
        let mut calls = 0;
        let r = unzip_game(&zip, &dest, &mut |_, _| calls += 1).unwrap();
        assert_eq!(r, Unzipped { files: 4, unread: 0 });
        assert!(calls > 0);
        assert_eq!(std::fs::read(dest.join("Flics/a.fli")).unwrap(), b"flic");
        assert!(crate::gamefiles::missing(&dest).is_empty());
        assert!(!dest.join("Movies/intro.bik").exists() && !dest.join("readme.txt").exists());
        assert!(!d.join("escape.txt").exists());
        // not a game folder, not a zip
        let other = d.join("other.zip");
        std::fs::write(&other, sg_core::zip::write(&[("photos/a.jpg", b"x")])).unwrap();
        assert_eq!(unzip_game(&other, &dest, &mut |_, _| {}), Err(Error::NoGame));
        std::fs::write(&other, b"this is no zip at all, just some text that is long enough").unwrap();
        assert!(matches!(unzip_game(&other, &dest, &mut |_, _| {}), Err(Error::BadZip(_))));
        // copying a folder leaves out the same things, and install moves it into place
        let staging = staging_for(&d.join("copy"));
        assert_eq!(copy_tree(&dest, &staging, &mut |_| {}).unwrap(), 4);
        install(&staging, &d.join("copy")).unwrap();
        assert!(crate::gamefiles::missing(&d.join("copy")).is_empty() && !staging.exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}

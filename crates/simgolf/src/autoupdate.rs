//! "Update now" (the port's own): the newest release, found by "Check for updates" (update.rs), downloaded, checked and
//! installed in the place of this copy, which then starts the new one.
//!
//! Nothing is installed that OpenSGolf's release key has not signed. Each release carries `SHA256SUMS` (the SHA-256 of
//! every package) and `SHA256SUMS.sig`, its ed25519 signature, made by the release workflow with the key in the GitHub
//! Actions secret OPENSGOLF_UPDATE_KEY (.github/workflows/release.yml, docs/RELEASING.md). The game checks the signature
//! against the public key built into it ([`PUBLIC_KEY_HEX`]), then the package's SHA-256 against the signed list, before
//! a single file of this copy is touched; a release without the two files, a bad signature or a package that does not
//! match is refused, and the box offers the download page instead. So is every release while this build carries the
//! placeholder key.
//!
//! Where the package goes (each step leaves this copy as it was when it fails):
//! * Windows (.zip) and the Linux .tar.gz: unpacked into a staging folder inside the program's folder; then each of its
//!   files and folders takes the place of the old one, which is renamed NAME.old (Windows lets a running .exe be renamed,
//!   not replaced). The new program is started; it deletes the .old files when it starts ([`finish_after_restart`]).
//! * Linux AppImage (`$APPIMAGE`): the new AppImage is written beside the old, made executable and renamed over it.
//! * macOS: the whole OpenSGolf.app is unpacked beside the running one, which is renamed OpenSGolf.app.old (kept until
//!   the new one has started) before the new one takes its name; `open -n` starts it. A folder the game may not write to
//!   (/Applications for a standard user, a translocated app) is said so, with the download page.
//! * Android: the APK is handed to Android's package installer, which asks the player to confirm (android.rs).
//!
//! Test hooks: `SG_UPDATE_KEY=HEX` checks signatures against that public key instead of the built-in one, and
//! `SG_UPDATE_NOW=1` (with `SG_UPDATE_URL`, update.rs) chooses Update now as soon as the check finds a newer version.

// Android uses only the APK's place; the desktop installs are built there too, for the tests and to keep one code path
#![cfg_attr(target_os = "android", allow(dead_code))]

use crate::app::App;
use crate::update::Release;
use sg_core::lang::{tr, trf};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// OpenSGolf's release signing key: the ed25519 public key, as 64 hex digits, that `sgtool update-keygen` prints.
///
/// TODO(owner): this is a PLACEHOLDER (all zeros). Replace it with the public key of the pair whose private half is the
/// GitHub Actions secret OPENSGOLF_UPDATE_KEY (docs/RELEASING.md). Until then Update now refuses every release and offers
/// the download page, so nothing unsigned can be installed.
pub const PUBLIC_KEY_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The test hook's variable: a public key (hex) used instead of [`PUBLIC_KEY_HEX`].
pub const KEY_ENV: &str = "SG_UPDATE_KEY";

/// The release's list of package checksums (`sha256sum` output) and its signature (64 bytes, ed25519).
pub const SUMS: &str = "SHA256SUMS";
pub const SUMS_SIG: &str = "SHA256SUMS.sig";
const MAX_SUMS: u64 = 64 << 10;
/// The largest package accepted (the packages are tens of MB).
const MAX_PACKAGE: u64 = 1 << 30;

/// The note in the save folder that names what the new program removes when it starts, and the version it is.
pub const JOURNAL: &str = "update_cleanup.json";

/// A key from hex; None for anything else, or for the placeholder (all zeros).
pub fn parse_key(hex: &str) -> Option<[u8; 32]> {
    let k: [u8; 32] = hex_bytes(hex.trim())?.try_into().ok()?;
    (k != [0; 32]).then_some(k)
}

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The key releases are checked against: the test hook's, else the built-in one (None while it is the placeholder).
pub fn release_key() -> Option<[u8; 32]> {
    match std::env::var(KEY_ENV) {
        Ok(k) if !k.is_empty() => parse_key(&k),
        _ => parse_key(PUBLIC_KEY_HEX),
    }
}

/// Why an update was refused or failed. Every one leaves this copy as it was.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// This build carries the placeholder key.
    NoKey,
    /// The release has no package for this system (its name).
    NoPackage(String),
    /// The release has no SHA256SUMS or no SHA256SUMS.sig.
    Unsigned,
    /// The signature is not one of OpenSGolf's key.
    BadSignature,
    /// The signed list does not name the package.
    NotListed(String),
    /// The package's SHA-256 is not the signed one.
    HashMismatch(String),
    /// This copy does not run from a release package (where it runs from).
    NotPackaged(PathBuf),
    /// The game may not write where it is installed.
    NoAccess(PathBuf),
    /// A download or a file operation failed (the message).
    Failed(String),
}

impl Error {
    /// The player's message.
    pub fn message(&self) -> String {
        let refused = |why: String| trf("STR_UPDATE_REFUSED", &[("WHY", &why)]);
        match self {
            Error::NoKey => tr("STR_UPDATE_NOT_SET_UP").to_string(),
            Error::NoPackage(f) => refused(trf("STR_UPDATE_WHY_NO_FILE", &[("FILE", f)])),
            Error::Unsigned => refused(tr("STR_UPDATE_WHY_UNSIGNED").to_string()),
            Error::BadSignature => refused(tr("STR_UPDATE_WHY_BAD_SIGNATURE").to_string()),
            Error::NotListed(f) => refused(trf("STR_UPDATE_WHY_NOT_LISTED", &[("FILE", f)])),
            Error::HashMismatch(f) => refused(trf("STR_UPDATE_WHY_HASH", &[("FILE", f)])),
            Error::NotPackaged(d) => trf("STR_UPDATE_NOT_PACKAGED", &[("DIR", &d.display())]),
            Error::NoAccess(d) => trf("STR_UPDATE_NO_ACCESS", &[("DIR", &d.display())]),
            Error::Failed(e) => trf("STR_UPDATE_INSTALL_FAILED", &[("ERROR", e)]),
        }
    }
}

// ---- the signed checksums --------------------------------------------------------------------------------------------------

/// The lines of a SHA256SUMS file (`sha256sum` output: 64 hex digits, a space, a space or '*', the name): (name, digest in
/// lowercase hex). Blank lines are skipped; anything else is an error.
pub fn parse_sums(text: &[u8]) -> Result<Vec<(String, String)>, String> {
    let text = std::str::from_utf8(text).map_err(|_| "SHA256SUMS is not text".to_string())?;
    let mut out = Vec::new();
    for line in text.lines().map(|l| l.trim_end_matches('\r')).filter(|l| !l.trim().is_empty()) {
        let bad = || format!("SHA256SUMS: bad line \"{line}\"");
        let (hash, rest) = line.split_at_checked(64).ok_or_else(bad)?;
        if !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let name = rest.strip_prefix("  ").or_else(|| rest.strip_prefix(" *")).filter(|n| !n.is_empty()).ok_or_else(bad)?;
        out.push((name.to_string(), hash.to_ascii_lowercase()));
    }
    Ok(out)
}

/// Checks the signature `sig` (64 bytes, or 128 hex digits) of `sums` against `key`.
pub fn verify_sums(sums: &[u8], sig: &[u8], key: &[u8; 32]) -> Result<(), Error> {
    let raw: [u8; 64] = match sig.len() {
        64 => sig.try_into().map_err(|_| Error::BadSignature)?,
        _ => std::str::from_utf8(sig).ok().and_then(|s| hex_bytes(s.trim())).and_then(|b| b.try_into().ok()).ok_or(Error::BadSignature)?,
    };
    let pk = ed25519_compact::PublicKey::new(*key);
    pk.verify(sums, &ed25519_compact::Signature::new(raw)).map_err(|_| Error::BadSignature)
}

/// The signed SHA-256 of the package `name` (the signature checked first).
pub fn signed_hash(sums: &[u8], sig: &[u8], key: &[u8; 32], name: &str) -> Result<String, Error> {
    verify_sums(sums, sig, key)?;
    let list = parse_sums(sums).map_err(Error::Failed)?;
    list.into_iter().find(|(n, _)| n == name).map(|(_, h)| h).ok_or_else(|| Error::NotListed(name.to_string()))
}

/// Checks a downloaded package: the list's signature, then the package's SHA-256 against the list's.
pub fn check_package(sums: &[u8], sig: &[u8], key: &[u8; 32], name: &str, data: &[u8]) -> Result<(), Error> {
    let want = signed_hash(sums, sig, key, name)?;
    if sg_core::index::sha256_hex(data) != want {
        return Err(Error::HashMismatch(name.to_string()));
    }
    Ok(())
}

// ---- where this copy is installed ------------------------------------------------------------------------------------------

/// Where the package goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// The folder a Windows .zip or a Linux .tar.gz was unpacked into, and the program's file name there.
    Folder { dir: PathBuf, exe: String },
    /// The AppImage file the game runs from (Linux).
    AppImage(PathBuf),
    /// The OpenSGolf.app bundle (macOS).
    Bundle(PathBuf),
    /// The folder the APK is downloaded into (Android).
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    Android(PathBuf),
}

/// Where a copy whose program is `exe` is installed, on system `os` (`std::env::consts::OS`), with `appimage` the AppImage
/// it runs from. A program outside a release package (a build from the sources, a distribution's package) cannot update.
pub fn place_for(os: &str, exe: &Path, appimage: Option<PathBuf>) -> Result<Place, Error> {
    let dir = exe.parent().unwrap_or(Path::new("."));
    if os == "macos" {
        // OpenSGolf.app/Contents/MacOS/opensgolf
        let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let app = Some(dir)
            .filter(|d| name(d) == "MacOS")
            .and_then(Path::parent)
            .filter(|c| name(c) == "Contents")
            .and_then(Path::parent)
            .filter(|a| a.extension().is_some_and(|e| e == "app"));
        return app.map(|a| Place::Bundle(a.to_path_buf())).ok_or_else(|| Error::NotPackaged(dir.to_path_buf()));
    }
    if os == "linux" {
        if let Some(p) = appimage.filter(|p| p.is_file()) {
            return Ok(Place::AppImage(p));
        }
    }
    // a release package has the packs and the translations beside the program
    if !dir.join("lang").is_dir() || !dir.join("packs").is_dir() {
        return Err(Error::NotPackaged(dir.to_path_buf()));
    }
    let exe = exe.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| Error::NotPackaged(dir.to_path_buf()))?;
    Ok(Place::Folder { dir: dir.to_path_buf(), exe })
}

/// Where this copy is installed.
pub fn this_place() -> Result<Place, Error> {
    #[cfg(target_os = "android")]
    {
        Ok(Place::Android(crate::android::files_dir().join("update")))
    }
    #[cfg(not(target_os = "android"))]
    {
        let exe = std::env::current_exe().map_err(|e| Error::Failed(e.to_string()))?;
        let appimage = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()).map(PathBuf::from);
        place_for(std::env::consts::OS, &exe, appimage)
    }
}

// ---- unpacking -------------------------------------------------------------------------------------------------------------

/// A name inside an archive as a relative path; None for one that would leave the folder, and for a Mac's `__MACOSX`
/// and `._` files.
fn safe_rel(name: &str) -> Option<PathBuf> {
    let mut p = PathBuf::new();
    for c in name.split('/').filter(|c| !c.is_empty() && *c != ".") {
        if c == ".." || c.contains('\\') || c.contains(':') || c == "__MACOSX" || c.starts_with("._") {
            return None;
        }
        p.push(c);
    }
    (!p.as_os_str().is_empty()).then_some(p)
}

fn io_err(e: std::io::Error) -> Error {
    Error::Failed(e.to_string())
}

/// `e` as the game being refused `dir`, or a plain failure.
fn access_err(dir: &Path, e: std::io::Error) -> Error {
    if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30) {
        Error::NoAccess(dir.to_path_buf())
    } else {
        io_err(e)
    }
}

fn write_file(path: &Path, data: &[u8], mode: Option<u32>) -> Result<(), Error> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(io_err)?;
    }
    std::fs::write(path, data).map_err(io_err)?;
    #[cfg(unix)]
    if let Some(m) = mode {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(m & 0o777)).map_err(io_err)?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

/// Unpacks a .zip into `dest`, with the Unix permissions it carries (a Mac's `ditto` keeps the programs' x bit).
pub fn unpack_zip(data: Vec<u8>, dest: &Path) -> Result<(), Error> {
    let z = sg_core::zip::Zip::open(data).map_err(Error::Failed)?;
    let names: Vec<String> = z.names().map(str::to_string).collect();
    for n in names {
        let Some(rel) = safe_rel(&n) else { continue };
        let body = z.read(&n).ok_or_else(|| Error::Failed(format!("{n}: damaged in the download")))?;
        write_file(&dest.join(rel), &body, z.unix_mode(&n))?;
    }
    Ok(())
}

/// The data of a .gz file.
fn gunzip(d: &[u8]) -> Result<Vec<u8>, Error> {
    let bad = || Error::Failed("not a .tar.gz".into());
    if d.len() < 18 || d[..3] != [0x1f, 0x8b, 8] {
        return Err(bad());
    }
    let flags = d[3];
    let mut p = 10;
    if flags & 4 != 0 {
        p += 2 + u16::from_le_bytes([*d.get(p).ok_or_else(bad)?, *d.get(p + 1).ok_or_else(bad)?]) as usize;
    }
    for bit in [8, 16] {
        if flags & bit != 0 {
            p += d.get(p..).ok_or_else(bad)?.iter().position(|&b| b == 0).ok_or_else(bad)? + 1;
        }
    }
    if flags & 2 != 0 {
        p += 2;
    }
    miniz_oxide::inflate::decompress_to_vec_with_limit(d.get(p..).ok_or_else(bad)?, MAX_PACKAGE as usize * 4).map_err(|_| bad())
}

/// A NUL-ended field of a tar header.
fn tar_str(b: &[u8]) -> String {
    String::from_utf8_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).into_owned()
}

fn tar_octal(b: &[u8]) -> Option<u64> {
    let s = tar_str(b);
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(s, 8).ok()
}

/// Unpacks a .tar.gz (as GNU tar writes it: ustar headers, long names in 'L' entries or pax headers) into `dest`, with
/// the files' permissions. Links and other special entries are left out.
pub fn unpack_tar_gz(data: &[u8], dest: &Path) -> Result<(), Error> {
    let tar = gunzip(data)?;
    let bad = || Error::Failed("damaged .tar.gz".into());
    let mut p = 0;
    let mut long_name: Option<String> = None;
    while p + 512 <= tar.len() {
        let h = &tar[p..p + 512];
        if h.iter().all(|&b| b == 0) {
            break;
        }
        let size = tar_octal(&h[124..136]).ok_or_else(bad)? as usize;
        let body = tar.get(p + 512..p + 512 + size).ok_or_else(bad)?;
        p += 512 + size.div_ceil(512) * 512;
        let kind = h[156];
        match kind {
            b'L' => {
                long_name = Some(tar_str(body));
                continue;
            }
            b'x' => {
                // pax: "LEN path=NAME\n" records
                let text = String::from_utf8_lossy(body);
                long_name = text.lines().find_map(|l| l.split_once(' ')?.1.strip_prefix("path=").map(str::to_string)).or(long_name);
                continue;
            }
            b'g' => continue,
            _ => {}
        }
        let mut name = tar_str(&h[0..100]);
        if &h[257..262] == b"ustar" {
            let prefix = tar_str(&h[345..500]);
            if !prefix.is_empty() {
                name = format!("{prefix}/{name}");
            }
        }
        let name = long_name.take().unwrap_or(name);
        let Some(rel) = safe_rel(&name) else { continue };
        match kind {
            b'0' | 0 | b'7' => write_file(&dest.join(rel), body, tar_octal(&h[100..108]).map(|m| m as u32))?,
            b'5' => std::fs::create_dir_all(dest.join(rel)).map_err(io_err)?,
            _ => {}
        }
    }
    Ok(())
}

/// The first folder at `root` or up to two levels in that `ok` accepts (a package holds its files in a folder of its own).
fn find_root(root: &Path, ok: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut level = vec![root.to_path_buf()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for d in level {
            if ok(&d) {
                return Some(d);
            }
            let mut kids: Vec<PathBuf> = std::fs::read_dir(&d).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            kids.sort();
            next.extend(kids);
        }
        level = next;
    }
    None
}

// ---- installing ------------------------------------------------------------------------------------------------------------

/// What an install did: the program (or app bundle) to start, and the old files kept until it has started.
#[derive(Debug, PartialEq)]
pub struct Installed {
    pub exe: PathBuf,
    pub backups: Vec<PathBuf>,
}

/// The name an old file or folder is kept under until the new program has started: NAME.old.
pub fn backup_of(p: &Path) -> PathBuf {
    let mut n = p.file_name().unwrap_or_default().to_os_string();
    n.push(".old");
    p.with_file_name(n)
}

fn remove_any(p: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(p),
        Ok(_) => std::fs::remove_file(p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Moves each file and folder of `new_root` into `dir`, the one it replaces renamed NAME.old first. When a step fails,
/// everything moved so far is put back. Returns the .old names.
pub fn swap_in(new_root: &Path, dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut names: Vec<_> = std::fs::read_dir(new_root).map_err(io_err)?.flatten().map(|e| e.file_name()).collect();
    names.sort();
    let mut done: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let r = (|| -> std::io::Result<()> {
        for n in &names {
            let dest = dir.join(n);
            let old = if std::fs::symlink_metadata(&dest).is_ok() {
                let o = backup_of(&dest);
                remove_any(&o)?;
                std::fs::rename(&dest, &o)?;
                Some(o)
            } else {
                None
            };
            done.push((dest.clone(), old));
            std::fs::rename(new_root.join(n), &dest)?;
        }
        Ok(())
    })();
    match r {
        Ok(()) => Ok(done.into_iter().filter_map(|(_, o)| o).collect()),
        Err(e) => {
            for (dest, old) in done.into_iter().rev() {
                let _ = remove_any(&dest);
                if let Some(o) = old {
                    let _ = std::fs::rename(&o, &dest);
                }
            }
            Err(access_err(dir, e))
        }
    }
}

/// The staging folder a package is unpacked into, in `dir` (so that moving its files into place is a rename).
fn staging_in(dir: &Path) -> Result<PathBuf, Error> {
    let s = dir.join(".opensgolf-update");
    remove_any(&s).map_err(|e| access_err(dir, e))?;
    std::fs::create_dir_all(&s).map_err(|e| access_err(dir, e))?;
    Ok(s)
}

fn unpack(name: &str, data: Vec<u8>, dest: &Path) -> Result<(), Error> {
    if name.ends_with(".zip") {
        unpack_zip(data, dest)
    } else if name.ends_with(".tar.gz") {
        unpack_tar_gz(&data, dest)
    } else {
        Err(Error::Failed(format!("{name}: not an archive")))
    }
}

/// Windows and the Linux .tar.gz: the package's files in the place of those in `dir`.
pub fn install_folder(dir: &Path, exe: &str, name: &str, data: Vec<u8>) -> Result<Installed, Error> {
    let staging = staging_in(dir)?;
    let r = (|| {
        unpack(name, data, &staging)?;
        let root = find_root(&staging, &|d| d.join(exe).is_file()).ok_or_else(|| Error::Failed(format!("{name} does not hold {exe}")))?;
        let backups = swap_in(&root, dir)?;
        Ok(Installed { exe: dir.join(exe), backups })
    })();
    let _ = remove_any(&staging);
    r
}

/// Linux: the new AppImage written beside `path`, made executable and renamed over it (the running one keeps its open
/// copy until it ends).
pub fn install_appimage(path: &Path, data: &[u8]) -> Result<Installed, Error> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let tmp = dir.join(".opensgolf-update.AppImage");
    let r = (|| {
        std::fs::write(&tmp, data).map_err(|e| access_err(dir, e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(io_err)?;
        }
        std::fs::rename(&tmp, path).map_err(|e| access_err(dir, e))
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r.map(|()| Installed { exe: path.to_path_buf(), backups: Vec::new() })
}

/// macOS: the package's OpenSGolf.app in the place of `app`, the old one kept as OpenSGolf.app.old. (A file this program
/// downloads itself gets no com.apple.quarantine attribute, which only browsers and other quarantine-aware programs set,
/// so there is nothing to strip, and Gatekeeper does not ask again.)
pub fn install_bundle(app: &Path, data: Vec<u8>) -> Result<Installed, Error> {
    let parent = app.parent().unwrap_or(Path::new("."));
    let staging = staging_in(parent)?;
    let r = (|| {
        unpack_zip(data, &staging)?;
        let new = find_root(&staging, &|d| d.extension().is_some_and(|e| e == "app") && d.join("Contents/MacOS").is_dir())
            .ok_or_else(|| Error::Failed("the package holds no .app".into()))?;
        let old = backup_of(app);
        remove_any(&old).map_err(|e| access_err(parent, e))?;
        std::fs::rename(app, &old).map_err(|e| access_err(parent, e))?;
        if let Err(e) = std::fs::rename(&new, app) {
            let _ = std::fs::rename(&old, app);
            return Err(access_err(parent, e));
        }
        Ok(Installed { exe: app.to_path_buf(), backups: vec![old] })
    })();
    let _ = remove_any(&staging);
    r
}

/// Puts the checked package `data` (release file `name`) in `place`.
pub fn install(place: &Place, name: &str, data: Vec<u8>) -> Result<Installed, Error> {
    match place {
        Place::Folder { dir, exe } => install_folder(dir, exe, name, data),
        Place::AppImage(p) => install_appimage(p, &data),
        Place::Bundle(app) => install_bundle(app, data),
        Place::Android(dir) => {
            let f = dir.join(name);
            write_file(&f, &data, None)?;
            Ok(Installed { exe: f, backups: Vec::new() })
        }
    }
}

// ---- the whole update ------------------------------------------------------------------------------------------------------

/// How far an update is.
#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Checksums,
    Downloading { done: u64, total: Option<u64> },
    Verifying,
    Installing,
}

/// Downloads, checks and installs `release`'s package `name` in `place`, telling `stage` how far it is. The checksums'
/// signature is checked before the package is downloaded, and the package before anything is installed.
pub fn run(release: &Release, name: &str, place: &Place, key: &[u8; 32], stage: &dyn Fn(Stage)) -> Result<Installed, Error> {
    let url = release.asset_url(name).ok_or_else(|| Error::NoPackage(name.to_string()))?;
    let (Some(sums_url), Some(sig_url)) = (release.asset_url(SUMS), release.asset_url(SUMS_SIG)) else { return Err(Error::Unsigned) };
    stage(Stage::Checksums);
    let quiet = |_: u64, _: Option<u64>| {};
    let sums = crate::net::fetch(sums_url, MAX_SUMS, &quiet).map_err(Error::Failed)?;
    let sig = crate::net::fetch(sig_url, 1024, &quiet).map_err(Error::Failed)?;
    // nothing big is downloaded for a release that is not OpenSGolf's
    signed_hash(&sums, &sig, key, name)?;
    let data = crate::net::fetch(url, MAX_PACKAGE, &|done, total| stage(Stage::Downloading { done, total })).map_err(Error::Failed)?;
    stage(Stage::Verifying);
    check_package(&sums, &sig, key, name, &data)?;
    stage(Stage::Installing);
    install(place, name, data)
}

/// The note the new program reads at start: the version installed and the old files to remove.
#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct Journal {
    #[serde(default)]
    version: String,
    #[serde(default)]
    remove: Vec<PathBuf>,
}

/// Before the restart: the note for the new program, in the save folder `save`.
pub fn write_journal(save: &Path, version: &str, backups: &[PathBuf]) {
    let j = Journal { version: version.to_string(), remove: backups.to_vec() };
    if let Ok(t) = serde_json::to_string_pretty(&j) {
        sg_core::fsutil::write_file(save.join(JOURNAL), t.as_bytes());
    }
}

/// At start: the old files an update left are removed (only NAME.old ones; what cannot be removed yet, the old program
/// still ending on Windows, is tried again on the next start). Returns the version installed when it is `current`, the
/// update having worked.
pub fn finish_after_restart(save: &Path, current: &str) -> Option<String> {
    let f = save.join(JOURNAL);
    let j: Journal = serde_json::from_slice(&sg_core::fsutil::read_raw(&f)?).unwrap_or_default();
    let mut left = Vec::new();
    for p in j.remove {
        if !p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(".old")) {
            continue;
        }
        let mut gone = false;
        for i in 0..5 {
            if remove_any(&p).is_ok() {
                gone = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100 << i));
        }
        if !gone {
            left.push(p);
        }
    }
    if left.is_empty() {
        let _ = std::fs::remove_file(&f);
    } else {
        write_journal(save, "", &left);
    }
    (!j.version.is_empty() && j.version == current).then_some(j.version)
}

/// Starts the installed program (`installed.exe`) with this one's options and ends this one: on Unix it replaces this
/// process; on Windows it starts a new one; on macOS `open -n` starts the new app bundle. Returns only when that failed.
pub fn restart(installed: &Installed) -> std::io::Error {
    use std::process::Command;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if cfg!(target_os = "macos") && installed.exe.extension().is_some_and(|e| e == "app") {
        // -n: a new instance even while this one still runs
        return match Command::new("open").arg("-n").arg(&installed.exe).arg("--args").args(&args).spawn() {
            Ok(_) => std::process::exit(0),
            Err(e) => e,
        };
    }
    let mut cmd = Command::new(&installed.exe);
    cmd.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.exec()
    }
    #[cfg(not(unix))]
    {
        match cmd.spawn() {
            Ok(_) => std::process::exit(0),
            Err(e) => e,
        }
    }
}

/// An update under way on a thread of its own.
pub struct Job {
    version: String,
    stage: Arc<Mutex<Stage>>,
    shown: String,
    thread: Option<std::thread::JoinHandle<Result<Installed, Error>>>,
    /// Android: the APK is with the system's installer.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    installing: bool,
}

impl Job {
    pub fn start(release: Release, name: &'static str, place: Place, key: [u8; 32]) -> Result<Job, Error> {
        let stage = Arc::new(Mutex::new(Stage::Checksums));
        let s = stage.clone();
        let version = release.version().to_string();
        println!("update: OpenSGolf {version}, {name} into {place:?}");
        let thread = std::thread::Builder::new()
            .name("opensgolf-update".into())
            .spawn(move || run(&release, name, &place, &key, &|st| *s.lock().unwrap_or_else(|e| e.into_inner()) = st))
            .map_err(io_err)?;
        Ok(Job { version, stage, shown: String::new(), thread: Some(thread), installing: false })
    }

    /// The progress box's text.
    fn text(&self) -> String {
        let v = &self.version;
        match self.stage.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            Stage::Checksums => trf("STR_UPDATE_STAGE_SUMS", &[("VERSION", v)]),
            Stage::Downloading { done, total: Some(t) } => {
                let mb = |b: u64| format!("{:.1}", b as f64 / 1048576.0);
                trf("STR_UPDATE_STAGE_DOWNLOAD", &[("VERSION", v), ("DONE", &mb(done)), ("TOTAL", &mb(t))])
            }
            Stage::Downloading { done, total: None } => {
                trf("STR_UPDATE_STAGE_DOWNLOAD_MB", &[("VERSION", v), ("DONE", &format!("{:.1}", done as f64 / 1048576.0))])
            }
            Stage::Verifying => trf("STR_UPDATE_STAGE_VERIFY", &[("VERSION", v)]),
            Stage::Installing => trf("STR_UPDATE_STAGE_INSTALL", &[("VERSION", v)]),
        }
    }

    /// The thread's result once it has ended.
    fn finished(&mut self) -> Option<Result<Installed, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let r = self.thread.take()?.join().unwrap_or_else(|_| Err(Error::Failed("panic".into())));
        Some(r)
    }
}

impl App {
    /// "Update now": the newest release's package for this system, checked and installed on a thread; a box shows how
    /// far it is (`poll_update_job`). A build with the placeholder key, a copy not installed from a package or a system
    /// without one is told so, with the download page.
    pub fn start_update(&mut self) {
        if self.update_job.is_some() {
            return;
        }
        let Some(release) = self.update_release.clone() else { return };
        let Some(key) = release_key() else {
            println!("update: refused, this build carries the placeholder release key");
            return self.update_failed(&Error::NoKey.message());
        };
        let place = match this_place() {
            Ok(p) => p,
            Err(e) => {
                println!("update: {e:?}");
                return self.update_failed(&e.message());
            }
        };
        let appimage = matches!(place, Place::AppImage(_));
        let Some(name) = crate::update::asset_name(std::env::consts::OS, std::env::consts::ARCH, appimage) else {
            return self.update_failed(&Error::NoPackage(std::env::consts::OS.into()).message());
        };
        match Job::start(release, name, place, key) {
            Ok(j) => self.update_job = Some(j),
            Err(e) => self.update_failed(&e.message()),
        }
    }

    /// Each frame: the update's progress in the game's box, and once it is done the new version started (or why not).
    pub fn poll_update_job(&mut self) {
        let Some(job) = self.update_job.as_mut() else { return };
        #[cfg(target_os = "android")]
        if job.installing {
            let state = crate::android::install_state();
            if let Some(why) = state.strip_prefix("failed:") {
                eprintln!("update: Android's installer: {why}");
                self.update_job = None;
                self.update_failed(&trf("STR_UPDATE_ANDROID_FAILED", &[("ERROR", &why)]));
            }
            return;
        }
        let text = job.text();
        if self.notice.is_none() || text != job.shown {
            job.shown = text.clone();
            self.notice =
                Some(crate::gamefiles::Notice { lines: crate::gamefiles::wrap(&text, 560.0), confirm: None, alt: None, sel: None });
        }
        let Some(result) = job.finished() else { return };
        let version = job.version.clone();
        self.notice = None;
        let installed = match result {
            Ok(i) => i,
            Err(e) => {
                eprintln!("update: {e:?}");
                self.update_job = None;
                return self.update_failed(&e.message());
            }
        };
        write_journal(&crate::app::save_dir(), &version, &installed.backups);
        #[cfg(target_os = "android")]
        {
            if crate::android::install_apk(&installed.exe) {
                job.installing = true;
                self.show_notice(tr("STR_UPDATE_ANDROID_CONFIRM"));
            } else {
                self.update_job = None;
                self.update_failed(&trf("STR_UPDATE_ANDROID_FAILED", &[("ERROR", &"PackageInstaller")]));
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            self.update_job = None;
            println!("update: OpenSGolf {version} installed; starting it");
            self.release_audio();
            let e = restart(&installed);
            eprintln!("update: restart: {e}");
            self.show_notice(&trf("STR_UPDATE_RESTART_FAILED", &[("VERSION", &version)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_compact::{KeyPair, Seed};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opensgolf-autoupdate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn keys(seed: u8) -> KeyPair {
        KeyPair::from_seed(Seed::new([seed; 32]))
    }

    fn sign(kp: &KeyPair, data: &[u8]) -> Vec<u8> {
        kp.sk.sign(data, None).to_vec()
    }

    fn sums_for(files: &[(&str, &[u8])]) -> Vec<u8> {
        files.iter().map(|(n, d)| format!("{}  {n}\n", sg_core::index::sha256_hex(d))).collect::<String>().into_bytes()
    }

    /// A .tar.gz as `tar czf` writes it (ustar headers; a name over 100 characters in a GNU 'L' entry).
    fn tar_gz(files: &[(&str, &[u8], u32)]) -> Vec<u8> {
        fn header(name: &str, size: usize, mode: u32, kind: u8) -> Vec<u8> {
            let mut h = vec![0u8; 512];
            let n = name.as_bytes();
            h[..n.len().min(100)].copy_from_slice(&n[..n.len().min(100)]);
            h[100..107].copy_from_slice(format!("{mode:07o}").as_bytes());
            h[124..135].copy_from_slice(format!("{size:011o}").as_bytes());
            h[156] = kind;
            h[257..263].copy_from_slice(b"ustar ");
            h
        }
        let mut t = Vec::new();
        for (name, data, mode) in files {
            if name.len() > 100 {
                let long = format!("{name}\0");
                t.extend(header("././@LongLink", long.len(), 0, b'L'));
                t.extend_from_slice(long.as_bytes());
                t.resize(t.len().div_ceil(512) * 512, 0);
            }
            t.extend(header(name, data.len(), *mode, b'0'));
            t.extend_from_slice(data);
            t.resize(t.len().div_ceil(512) * 512, 0);
        }
        t.extend(vec![0u8; 1024]);
        let mut gz = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3];
        gz.extend(miniz_oxide::deflate::compress_to_vec(&t, 6));
        gz.extend_from_slice(&[0; 8]);
        gz
    }

    #[test]
    fn keys_and_the_placeholder() {
        assert_eq!(parse_key(PUBLIC_KEY_HEX), None, "the placeholder is no key");
        let kp = keys(3);
        let hex: String = kp.pk.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(parse_key(&hex), Some(*kp.pk));
        assert_eq!(parse_key(&hex.to_uppercase()), Some(*kp.pk));
        assert_eq!(parse_key(&hex[2..]), None);
        assert_eq!(parse_key("zz"), None);
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn sums_are_read() {
        let a = "a".repeat(64);
        let b = "B".repeat(64);
        let text = format!("{a}  opensgolf-macos.zip\r\n\n{b} *opensgolf-android.apk\n");
        let l = parse_sums(text.as_bytes()).unwrap();
        assert_eq!(l, vec![("opensgolf-macos.zip".into(), a.clone()), ("opensgolf-android.apk".into(), "b".repeat(64))]);
        for bad in [format!("{a} x"), format!("{a}  "), "abc  x".into(), format!("{}  x", "g".repeat(64)), "\u{e9}".repeat(40)] {
            assert!(parse_sums(bad.as_bytes()).is_err(), "{bad:?}");
        }
        assert!(parse_sums(&[0xff, 0xfe]).is_err());
        assert_eq!(parse_sums(b"").unwrap(), vec![]);
    }

    #[test]
    fn signatures_are_checked() {
        let kp = keys(1);
        let pkg = b"the package".as_slice();
        let sums = sums_for(&[("opensgolf-windows.zip", pkg)]);
        let sig = sign(&kp, &sums);
        let key = *kp.pk;
        assert_eq!(verify_sums(&sums, &sig, &key), Ok(()));
        // as hex text too
        let hex: String = sig.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(verify_sums(&sums, format!("{hex}\n").as_bytes(), &key), Ok(()));
        // a bad signature, a changed list, another key, a missing or cut signature
        let mut bad = sig.clone();
        bad[10] ^= 1;
        assert_eq!(verify_sums(&sums, &bad, &key), Err(Error::BadSignature));
        let mut changed = sums.clone();
        changed[0] = if changed[0] == b'0' { b'1' } else { b'0' };
        assert_eq!(verify_sums(&changed, &sig, &key), Err(Error::BadSignature));
        assert_eq!(verify_sums(&sums, &sig, &keys(2).pk), Err(Error::BadSignature));
        assert_eq!(verify_sums(&sums, b"", &key), Err(Error::BadSignature));
        assert_eq!(verify_sums(&sums, &sig[..63], &key), Err(Error::BadSignature));
        // the package against the list
        assert_eq!(check_package(&sums, &sig, &key, "opensgolf-windows.zip", pkg), Ok(()));
        assert_eq!(
            check_package(&sums, &sig, &key, "opensgolf-windows.zip", b"the packagE"),
            Err(Error::HashMismatch("opensgolf-windows.zip".into()))
        );
        assert_eq!(check_package(&sums, &sig, &key, "opensgolf-macos.zip", pkg), Err(Error::NotListed("opensgolf-macos.zip".into())));
        assert_eq!(check_package(&sums, &bad, &key, "opensgolf-windows.zip", pkg), Err(Error::BadSignature));
        // every refusal has its message
        for e in [Error::NoKey, Error::Unsigned, Error::BadSignature, Error::HashMismatch("x".into()), Error::NoAccess("/x".into())] {
            assert!(!e.message().is_empty());
        }
    }

    #[test]
    fn places() {
        let root = tmp("places");
        let mac = root.join("Applications/OpenSGolf.app/Contents/MacOS/opensgolf");
        assert_eq!(place_for("macos", &mac, None), Ok(Place::Bundle(root.join("Applications/OpenSGolf.app"))));
        assert!(matches!(place_for("macos", &root.join("bin/opensgolf"), None), Err(Error::NotPackaged(_))));
        // a folder from a package (lang and packs beside the program), a build from the sources
        let pkg = root.join("opensgolf-linux-x86_64");
        std::fs::create_dir_all(pkg.join("lang")).unwrap();
        std::fs::create_dir_all(pkg.join("packs")).unwrap();
        assert_eq!(place_for("linux", &pkg.join("opensgolf"), None), Ok(Place::Folder { dir: pkg.clone(), exe: "opensgolf".into() }));
        assert_eq!(
            place_for("windows", &pkg.join("opensgolf.exe"), None),
            Ok(Place::Folder { dir: pkg.clone(), exe: "opensgolf.exe".into() })
        );
        assert!(matches!(place_for("linux", &root.join("target/release/opensgolf"), None), Err(Error::NotPackaged(_))));
        // an AppImage that is there
        let ai = root.join("OpenSGolf.AppImage");
        std::fs::write(&ai, b"x").unwrap();
        assert_eq!(place_for("linux", Path::new("/tmp/.mount_x/usr/bin/opensgolf"), Some(ai.clone())), Ok(Place::AppImage(ai)));
        assert!(place_for("linux", Path::new("/tmp/.mount_x/usr/bin/opensgolf"), Some(root.join("gone"))).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tar_gz_unpacks_with_modes_and_long_names() {
        let root = tmp("tar");
        let long = format!("top/{}/f.txt", "d".repeat(110));
        let tgz = tar_gz(&[("top/opensgolf", b"#!/bin/sh\n", 0o755), (&long, b"long", 0o644), ("../evil", b"x", 0o644)]);
        unpack_tar_gz(&tgz, &root.join("out")).unwrap();
        assert_eq!(std::fs::read(root.join("out/top/opensgolf")).unwrap(), b"#!/bin/sh\n");
        assert_eq!(std::fs::read(root.join("out").join(&long)).unwrap(), b"long");
        assert!(!root.join("evil").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(root.join("out/top/opensgolf")).unwrap().permissions().mode() & 0o777, 0o755);
        }
        assert!(unpack_tar_gz(b"not gzip at all, no", &root.join("x")).is_err());
        assert_eq!(safe_rel("a/../b"), None);
        assert_eq!(safe_rel("__MACOSX/a"), None);
        assert_eq!(safe_rel("./a//b"), Some(PathBuf::from("a/b")));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A release in `dir` with file:// addresses: the package, SHA256SUMS and (when `signer`) its signature.
    fn fake_release(dir: &Path, name: &str, pkg: &[u8], signer: Option<&KeyPair>) -> Release {
        std::fs::create_dir_all(dir).unwrap();
        let sums = sums_for(&[(name, pkg), ("other.zip", b"other")]);
        std::fs::write(dir.join(name), pkg).unwrap();
        std::fs::write(dir.join(SUMS), &sums).unwrap();
        let url = |n: &str| format!("file://{}", dir.join(n).display());
        let mut assets = vec![(name.to_string(), url(name)), (SUMS.to_string(), url(SUMS))];
        if let Some(kp) = signer {
            std::fs::write(dir.join(SUMS_SIG), sign(kp, &sums)).unwrap();
            assets.push((SUMS_SIG.to_string(), url(SUMS_SIG)));
        }
        Release { tag: "v9.9.9".into(), assets }
    }

    /// An installed copy: the program (a copy of this test's own binary), lang and packs.
    fn fake_install(dir: &Path) -> Vec<u8> {
        std::fs::create_dir_all(dir.join("lang")).unwrap();
        std::fs::create_dir_all(dir.join("packs")).unwrap();
        std::fs::write(dir.join("lang/en.txt"), b"old text").unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), dir.join("opensgolf")).unwrap();
        std::fs::read(dir.join("opensgolf")).unwrap()
    }

    #[test]
    fn a_folder_install_end_to_end() {
        let root = tmp("e2e-folder");
        let kp = keys(9);
        let name = "opensgolf-linux-x86_64.tar.gz";
        let tgz = tar_gz(&[
            ("opensgolf-linux-x86_64/opensgolf", b"#!/bin/sh\necho new\n", 0o755),
            ("opensgolf-linux-x86_64/lang/en.txt", b"new text", 0o644),
            ("opensgolf-linux-x86_64/packs/free/pack.toml", b"id = \"free\"", 0o644),
            ("opensgolf-linux-x86_64/README.md", b"readme", 0o644),
        ]);
        let release = fake_release(&root.join("release"), name, &tgz, Some(&kp));
        let dir = root.join("install");
        let old_exe = fake_install(&dir);
        let place = place_for("linux", &dir.join("opensgolf"), None).unwrap();
        let stages = std::cell::RefCell::new(Vec::new());
        let got = run(&release, name, &place, &kp.pk, &|s| stages.borrow_mut().push(s)).unwrap();
        assert_eq!(got.exe, dir.join("opensgolf"));
        assert_eq!(got.backups, vec![dir.join("lang.old"), dir.join("opensgolf.old"), dir.join("packs.old")]);
        assert_eq!(std::fs::read(dir.join("opensgolf")).unwrap(), b"#!/bin/sh\necho new\n");
        assert_eq!(std::fs::read(dir.join("lang/en.txt")).unwrap(), b"new text");
        assert_eq!(std::fs::read(dir.join("README.md")).unwrap(), b"readme");
        assert!(dir.join("packs/free/pack.toml").is_file());
        assert_eq!(std::fs::read(dir.join("opensgolf.old")).unwrap(), old_exe, "the old program kept until the new one starts");
        assert!(!dir.join(".opensgolf-update").exists(), "the staging folder is gone");
        let st = stages.into_inner();
        assert_eq!(st.first(), Some(&Stage::Checksums));
        assert!(st.contains(&Stage::Downloading { done: tgz.len() as u64, total: Some(tgz.len() as u64) }));
        assert_eq!(st.last(), Some(&Stage::Installing));
        // the new program's start removes the old files and says which version it is
        let save = root.join("save");
        std::fs::create_dir_all(&save).unwrap();
        write_journal(&save, "9.9.9", &got.backups);
        assert_eq!(finish_after_restart(&save, "9.9.9"), Some("9.9.9".into()));
        assert!(got.backups.iter().all(|b| !b.exists()));
        assert!(!save.join(JOURNAL).exists());
        assert_eq!(finish_after_restart(&save, "9.9.9"), None, "once");
        // a journal naming anything but .old files removes nothing
        write_journal(&save, "1.0.0", &[dir.join("README.md")]);
        assert_eq!(finish_after_restart(&save, "9.9.9"), None, "another version: the update did not take");
        assert!(dir.join("README.md").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refusals_leave_the_copy_alone() {
        let root = tmp("e2e-refused");
        let kp = keys(9);
        let name = "opensgolf-linux-x86_64.tar.gz";
        let tgz = tar_gz(&[("opensgolf-linux-x86_64/opensgolf", b"new", 0o755)]);
        let dir = root.join("install");
        let old_exe = fake_install(&dir);
        let place = place_for("linux", &dir.join("opensgolf"), None).unwrap();
        let quiet = |_: Stage| {};
        // no signature in the release
        let unsigned = fake_release(&root.join("unsigned"), name, &tgz, None);
        assert_eq!(run(&unsigned, name, &place, &kp.pk, &quiet), Err(Error::Unsigned));
        // signed by another key
        let other = fake_release(&root.join("other"), name, &tgz, Some(&keys(4)));
        assert_eq!(run(&other, name, &place, &kp.pk, &quiet), Err(Error::BadSignature));
        // the package changed after it was signed
        let swapped = fake_release(&root.join("swapped"), name, &tgz, Some(&kp));
        std::fs::write(root.join("swapped").join(name), tar_gz(&[("opensgolf-linux-x86_64/opensgolf", b"evil", 0o755)])).unwrap();
        assert_eq!(run(&swapped, name, &place, &kp.pk, &quiet), Err(Error::HashMismatch(name.into())));
        // no package for this system
        assert_eq!(
            run(&swapped, "opensgolf-linux-arm64.tar.gz", &place, &kp.pk, &quiet),
            Err(Error::NoPackage("opensgolf-linux-arm64.tar.gz".into()))
        );
        // a signed package that does not hold the program
        let empty = tar_gz(&[("opensgolf-linux-x86_64/README.md", b"x", 0o644)]);
        let hollow = fake_release(&root.join("hollow"), name, &empty, Some(&kp));
        assert!(matches!(run(&hollow, name, &place, &kp.pk, &quiet), Err(Error::Failed(_))));
        assert_eq!(std::fs::read(dir.join("opensgolf")).unwrap(), old_exe);
        assert_eq!(std::fs::read(dir.join("lang/en.txt")).unwrap(), b"old text");
        assert!(!dir.join("opensgolf.old").exists() && !dir.join(".opensgolf-update").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failed_swap_is_put_back() {
        let root = tmp("swap");
        let (new, dir) = (root.join("new"), root.join("dir"));
        // the second name is as long as a name may be, so its backup's name (NAME.old) is too long and that step fails
        let long = "z".repeat(255);
        for d in [&new, &dir] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join(&long), d.to_string_lossy().as_bytes()).unwrap();
        }
        std::fs::write(new.join("a"), b"new a").unwrap();
        std::fs::write(dir.join("a"), b"old a").unwrap();
        std::fs::create_dir_all(new.join("lang")).unwrap();
        std::fs::write(new.join("lang/en.txt"), b"new").unwrap();
        assert!(swap_in(&new, &dir).is_err());
        assert_eq!(std::fs::read(dir.join("a")).unwrap(), b"old a", "put back");
        assert!(!dir.join("a.old").exists() && !dir.join("lang").exists());
        assert_eq!(std::fs::read(dir.join(&long)).unwrap(), dir.to_string_lossy().as_bytes());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_appimage_is_replaced() {
        let root = tmp("appimage");
        let kp = keys(5);
        let name = "opensgolf-linux-x86_64.AppImage";
        let new = b"\x7fELF new appimage".to_vec();
        let release = fake_release(&root.join("release"), name, &new, Some(&kp));
        let ai = root.join("Games/OpenSGolf.AppImage");
        std::fs::create_dir_all(ai.parent().unwrap()).unwrap();
        std::fs::write(&ai, b"old").unwrap();
        let place = place_for("linux", Path::new("/tmp/.mount_q/usr/bin/opensgolf"), Some(ai.clone())).unwrap();
        let got = run(&release, name, &place, &kp.pk, &|_| {}).unwrap();
        assert_eq!(got, Installed { exe: ai.clone(), backups: vec![] });
        assert_eq!(std::fs::read(&ai).unwrap(), new);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&ai).unwrap().permissions().mode() & 0o777, 0o755);
        }
        assert_eq!(std::fs::read_dir(ai.parent().unwrap()).unwrap().count(), 1, "nothing left beside it");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mac_bundle_and_a_windows_folder_are_replaced() {
        // the file operations of the macOS and Windows installs, run here on their packages' layouts
        let root = tmp("bundle");
        let kp = keys(6);
        let name = "opensgolf-macos.zip";
        let zip = sg_core::zip::write(&[
            ("opensgolf-macos/OpenSGolf.app/Contents/MacOS/opensgolf", b"new mac"),
            ("opensgolf-macos/OpenSGolf.app/Contents/Info.plist", b"plist"),
            ("opensgolf-macos/README.md", b"readme"),
            ("__MACOSX/opensgolf-macos/._README.md", b"fork"),
        ]);
        let release = fake_release(&root.join("release"), name, &zip, Some(&kp));
        let app = root.join("Applications/OpenSGolf.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::write(app.join("Contents/MacOS/opensgolf"), b"old mac").unwrap();
        let place = place_for("macos", &app.join("Contents/MacOS/opensgolf"), None).unwrap();
        let got = run(&release, name, &place, &kp.pk, &|_| {}).unwrap();
        assert_eq!(got, Installed { exe: app.clone(), backups: vec![root.join("Applications/OpenSGolf.app.old")] });
        assert_eq!(std::fs::read(app.join("Contents/MacOS/opensgolf")).unwrap(), b"new mac");
        assert_eq!(std::fs::read(root.join("Applications/OpenSGolf.app.old/Contents/MacOS/opensgolf")).unwrap(), b"old mac");
        let names: Vec<_> = std::fs::read_dir(root.join("Applications")).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(names.len(), 2, "the app and its backup: {names:?}");

        let name = "opensgolf-windows.zip";
        let zip = sg_core::zip::write(&[
            ("opensgolf-windows/opensgolf.exe", b"new exe"),
            ("opensgolf-windows/sgtool.exe", b"new tool"),
            ("opensgolf-windows/lang/en.txt", b"new"),
        ]);
        let release = fake_release(&root.join("release-win"), name, &zip, Some(&kp));
        let dir = root.join("OpenSGolf");
        std::fs::create_dir_all(dir.join("lang")).unwrap();
        std::fs::create_dir_all(dir.join("packs")).unwrap();
        std::fs::write(dir.join("opensgolf.exe"), b"old exe").unwrap();
        std::fs::write(dir.join("opensgolf.exe.old"), b"an older leftover").unwrap();
        let place = place_for("windows", &dir.join("opensgolf.exe"), None).unwrap();
        let got = run(&release, name, &place, &kp.pk, &|_| {}).unwrap();
        assert_eq!(got.exe, dir.join("opensgolf.exe"));
        assert_eq!(got.backups, vec![dir.join("lang.old"), dir.join("opensgolf.exe.old")]);
        assert_eq!(std::fs::read(dir.join("opensgolf.exe")).unwrap(), b"new exe");
        assert_eq!(std::fs::read(dir.join("opensgolf.exe.old")).unwrap(), b"old exe");
        assert_eq!(std::fs::read(dir.join("sgtool.exe")).unwrap(), b"new tool");
        assert!(dir.join("packs").is_dir(), "what the package does not hold stays");
        let _ = std::fs::remove_dir_all(&root);
    }
}

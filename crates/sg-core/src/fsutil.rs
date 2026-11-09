//! File helpers. The game was made for Windows, whose file names are case-insensitive, and its data refers to files with
//! whatever case the artists used. Linux file systems are case-sensitive, so every lookup into the game folder goes through
//! [`resolve`], which falls back to a case-insensitive walk when the exact path does not exist.
//!
//! All file access of the port goes through this module. Natively it is the disk. In a browser (wasm32) it is the page: the
//! player picks their own SimGolf folder, the page keeps its files in memory, and these functions call into the page
//! (web/opensgolf.js); lookups there are case-insensitive already. Files written in the browser are kept by the page.
//!
//! Content packs (docs/MODDING.md) sit over the game folder: [`read_file`] returns an enabled pack's replacement of a game
//! file when there is one (crate::vfs), the file itself otherwise.
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::{Path, PathBuf};
    pub fn read(p: &Path) -> Option<Vec<u8>> {
        std::fs::read(p).ok()
    }
    pub fn read_head(p: &Path, n: usize) -> Option<Vec<u8>> {
        use std::io::Read;
        let mut v = Vec::new();
        std::fs::File::open(p).ok()?.take(n as u64).read_to_end(&mut v).ok()?;
        Some(v)
    }
    pub fn write(p: &Path, data: &[u8]) -> bool {
        if let Some(d) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(p, data).is_ok()
    }
    pub fn remove(p: &Path) -> bool {
        std::fs::remove_file(p).is_ok()
    }
    pub fn exists(p: &Path) -> bool {
        p.exists()
    }
    pub fn is_dir(p: &Path) -> bool {
        p.is_dir()
    }
    pub fn list(p: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default()
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::path::{Path, PathBuf};
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn sg_fs_size(path: *const u8, len: usize) -> i32;
        fn sg_fs_read(path: *const u8, len: usize, dst: *mut u8);
        fn sg_fs_write(path: *const u8, len: usize, data: *const u8, data_len: usize);
        fn sg_fs_remove(path: *const u8, len: usize) -> i32;
        fn sg_fs_kind(path: *const u8, len: usize) -> i32;
        fn sg_fs_list(path: *const u8, len: usize) -> i32;
        fn sg_fs_list_item(index: i32, dst: *mut u8, cap: usize) -> i32;
    }
    fn s(p: &Path) -> String {
        p.to_string_lossy().replace('\\', "/")
    }
    pub fn read(p: &Path) -> Option<Vec<u8>> {
        let k = s(p);
        let n = unsafe { sg_fs_size(k.as_ptr(), k.len()) };
        if n < 0 {
            return None;
        }
        let mut v = vec![0u8; n as usize];
        unsafe { sg_fs_read(k.as_ptr(), k.len(), v.as_mut_ptr()) };
        Some(v)
    }
    pub fn write(p: &Path, data: &[u8]) -> bool {
        let k = s(p);
        unsafe { sg_fs_write(k.as_ptr(), k.len(), data.as_ptr(), data.len()) };
        true
    }
    pub fn remove(p: &Path) -> bool {
        let k = s(p);
        unsafe { sg_fs_remove(k.as_ptr(), k.len()) != 0 }
    }
    /// 0 missing, 1 file, 2 directory.
    fn kind(p: &Path) -> i32 {
        let k = s(p);
        unsafe { sg_fs_kind(k.as_ptr(), k.len()) }
    }
    pub fn exists(p: &Path) -> bool {
        kind(p) != 0
    }
    pub fn is_dir(p: &Path) -> bool {
        kind(p) == 2
    }
    pub fn list(p: &Path) -> Vec<PathBuf> {
        let k = s(p);
        let n = unsafe { sg_fs_list(k.as_ptr(), k.len()) };
        let mut out = Vec::new();
        let mut buf = vec![0u8; 1024];
        for i in 0..n.max(0) {
            let len = unsafe { sg_fs_list_item(i, buf.as_mut_ptr(), buf.len()) };
            if len > 0 {
                out.push(p.join(String::from_utf8_lossy(&buf[..len as usize]).to_string()));
            }
        }
        out
    }
}

/// Reads a file. A file of the game folder that an enabled content pack replaces is read from the pack (crate::vfs).
pub fn read_file(path: impl AsRef<Path>) -> Option<Vec<u8>> {
    let p = path.as_ref();
    crate::vfs::read_override(p).or_else(|| imp::read(p))
}

/// The first `n` bytes of a file (all of it when shorter), through the content packs as [`read_file`]. Natively only those
/// bytes are read; in a browser the page hands over the whole file, which is in memory there anyway.
pub fn read_head(path: impl AsRef<Path>, n: usize) -> Option<Vec<u8>> {
    let p = path.as_ref();
    #[cfg(not(target_arch = "wasm32"))]
    let d = crate::vfs::read_override(p).or_else(|| imp::read_head(p, n));
    #[cfg(target_arch = "wasm32")]
    let d = read_file(p);
    d.map(|mut d| {
        d.truncate(n);
        d
    })
}

/// Why a file could not be read, for problem reports: the system's own reason, and what the nearest folder that exists
/// holds (names and sizes, the first 16), so a report names a missing, renamed, locked or empty file.
pub fn why_unreadable(path: impl AsRef<Path>) -> String {
    let p = path.as_ref();
    #[cfg(not(target_arch = "wasm32"))]
    let reason = match std::fs::read(p) {
        Ok(d) => format!("reads now ({} bytes)", d.len()),
        Err(e) => e.to_string(),
    };
    #[cfg(target_arch = "wasm32")]
    let reason = "not in the browser's copy".to_string();
    let mut dir = p.parent();
    while let Some(d) = dir.filter(|d| !d.as_os_str().is_empty() && !is_dir(d)) {
        dir = d.parent();
    }
    let Some(d) = dir.filter(|d| !d.as_os_str().is_empty()) else { return reason };
    let mut items = list_dir(d);
    items.sort();
    let shown: Vec<String> = items
        .iter()
        .take(16)
        .map(|f| {
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            #[cfg(not(target_arch = "wasm32"))]
            let size = std::fs::metadata(f).ok().filter(|m| m.is_file()).map(|m| format!(" {}", m.len()));
            #[cfg(target_arch = "wasm32")]
            let size: Option<String> = None;
            format!("{name}{}", size.unwrap_or_else(|| "/".into()))
        })
        .collect();
    let more = if items.len() > 16 { format!(", {} more", items.len() - 16) } else { String::new() };
    // the file's own folder by name only (the path is in the message already); a folder further up in full
    let place = if Some(d) == p.parent() {
        format!("{} holds", d.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())
    } else {
        format!("missing folder; {} holds", d.display())
    };
    format!("{reason}; {place}: {}{more}", if shown.is_empty() { "nothing".into() } else { shown.join(", ") })
}

/// Reads a file as it is on disk (or in the browser's store), never through the content packs.
pub fn read_raw(path: impl AsRef<Path>) -> Option<Vec<u8>> {
    imp::read(path.as_ref())
}

/// The first `n` bytes of a file as it is on disk (or in the browser's store), never through the content packs.
pub fn read_head_raw(path: impl AsRef<Path>, n: usize) -> Option<Vec<u8>> {
    let p = path.as_ref();
    #[cfg(not(target_arch = "wasm32"))]
    let d = imp::read_head(p, n);
    #[cfg(target_arch = "wasm32")]
    let d = imp::read(p);
    d.map(|mut d| {
        d.truncate(n);
        d
    })
}

/// The entries of a directory on disk (or in the browser's store), unsorted, without what content packs add.
pub fn list_raw(path: impl AsRef<Path>) -> Vec<PathBuf> {
    imp::list(path.as_ref())
}

/// True for a folder on disk (or in the browser's store), not one a content pack makes.
pub fn is_dir_raw(path: impl AsRef<Path>) -> bool {
    imp::is_dir(path.as_ref())
}

pub fn read_text(path: impl AsRef<Path>) -> Option<String> {
    read_file(path).map(|d| String::from_utf8_lossy(&d).into_owned())
}

pub fn write_file(path: impl AsRef<Path>, data: &[u8]) -> bool {
    imp::write(path.as_ref(), data)
}

/// Deletes a file (a saved game the player chose to delete). Returns false when nothing was removed.
pub fn remove_file(path: impl AsRef<Path>) -> bool {
    imp::remove(path.as_ref())
}

/// True when the file or folder exists, on disk or in an enabled content pack's files over the game folder (crate::vfs).
pub fn exists(path: impl AsRef<Path>) -> bool {
    let p = path.as_ref();
    imp::exists(p) || crate::vfs::virtual_kind(p) != 0
}

/// True for a folder on disk or one that an enabled content pack's files make inside the game folder.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    let p = path.as_ref();
    imp::is_dir(p) || crate::vfs::virtual_kind(p) == 2
}

/// The entries of a directory (files and folders), unsorted: those on disk, then those the enabled content packs add inside
/// the game folder. A pack's replacement of a file on disk (a PNG for a PCX, TGA or BMP, an OGG for a WAV) is not listed
/// again: the file on disk stands for it.
pub fn list_dir(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let p = path.as_ref();
    let mut out = imp::list(p);
    let extra = crate::vfs::virtual_entries(p);
    if extra.is_empty() {
        return out;
    }
    let have: std::collections::HashSet<String> =
        out.iter().filter_map(|q| q.file_name()).map(|n| n.to_string_lossy().to_lowercase()).collect();
    for (name, _) in extra {
        let low = name.to_lowercase();
        let shadowed = have.contains(&low)
            || low.rsplit_once('.').is_some_and(|(stem, ext)| match ext {
                "png" => ["pcx", "tga", "bmp"].iter().any(|e| have.contains(&format!("{stem}.{e}"))),
                "ogg" => have.contains(&format!("{stem}.wav")),
                _ => false,
            });
        if !shadowed {
            out.push(p.join(name));
        }
    }
    out
}

/// `base` joined with a relative path written with forward slashes, matched case-insensitively component by component when
/// the exact path does not exist. Returns the exact join when nothing matches, so error messages name the expected file.
pub fn resolve(base: impl AsRef<Path>, rel: &str) -> PathBuf {
    let base = base.as_ref();
    let exact = rel.split('/').filter(|c| !c.is_empty()).fold(base.to_path_buf(), |p, c| p.join(c));
    if exists(&exact) {
        return exact;
    }
    let mut cur = base.to_path_buf();
    for comp in rel.split('/').filter(|c| !c.is_empty()) {
        let direct = cur.join(comp);
        if exists(&direct) {
            cur = direct;
            continue;
        }
        // any case, and a space for an underscore: some retail installs name the tropical trees' folders "Tree Cerc"
        // and "Tree Drac" where other copies have "Tree_Cerc" and "Tree_Drac" (#88)
        let want = loose_name(comp);
        let found = list_dir(&cur).into_iter().find(|p| p.file_name().map(|n| loose_name(&n.to_string_lossy()) == want).unwrap_or(false));
        match found {
            Some(p) => cur = p,
            None => return exact,
        }
    }
    cur
}

/// A file or folder name as `resolve` compares it: lower case, with spaces read as underscores.
fn loose_name(n: &str) -> String {
    n.to_lowercase().replace(' ', "_")
}

/// Lower-case file extension including the dot (".pcx"), or "".
pub fn ext_lower(p: &Path) -> String {
    p.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default()
}

/// All regular files under `dir`, recursively, sorted.
pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for p in list_dir(&d) {
            if is_dir(&p) {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_space_in_a_folder_name_reads_as_an_underscore() {
        let dir = std::env::temp_dir().join(format!("sg_loose_{}", std::process::id()));
        let sub = dir.join("Flics").join("Trees").join("Tropic").join("Tree Cerc");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("Cerc_Small.flc"), b"x").unwrap();
        // the game asks for the underscore name; this install has a space (#88)
        let p = resolve(&dir, "Flics/Trees/Tropic/Tree_Cerc/Cerc_Small.flc");
        assert_eq!(std::fs::read(&p).unwrap(), b"x");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_file_says_why_and_what_its_folder_holds() {
        let dir = std::env::temp_dir().join(format!("sg_why_{}", std::process::id()));
        let sub = dir.join("Tree_Cerc");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("Cerc_Small.FLC.bak"), [0u8; 5]).unwrap();
        // a missing file: the system's reason, then the folder's names and sizes
        let w = why_unreadable(sub.join("Cerc_Small.flc"));
        assert!(w.contains("Tree_Cerc holds: Cerc_Small.FLC.bak 5"), "{w}");
        // a missing folder: the nearest one that exists
        let w = why_unreadable(dir.join("Tree_Drac").join("Drac_Small.flc"));
        assert!(w.contains("holds: Tree_Cerc/"), "{w}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

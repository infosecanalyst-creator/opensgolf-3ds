//! File helpers. The game was made for Windows, whose file names are case-insensitive, and its data refers to files with
//! whatever case the artists used. Linux file systems are case-sensitive, so every lookup into the game folder goes through
//! [`resolve`], which falls back to a case-insensitive walk when the exact path does not exist.
use std::path::{Path, PathBuf};

pub fn read_file(path: impl AsRef<Path>) -> Option<Vec<u8>> {
    std::fs::read(path.as_ref()).ok()
}

/// `base` joined with a relative path written with forward slashes, matched case-insensitively component by component when
/// the exact path does not exist. Returns the exact join when nothing matches, so error messages name the expected file.
pub fn resolve(base: impl AsRef<Path>, rel: &str) -> PathBuf {
    let base = base.as_ref();
    let exact = rel.split('/').filter(|c| !c.is_empty()).fold(base.to_path_buf(), |p, c| p.join(c));
    if exact.exists() {
        return exact;
    }
    let mut cur = base.to_path_buf();
    for comp in rel.split('/').filter(|c| !c.is_empty()) {
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let want = comp.to_lowercase();
        let found = std::fs::read_dir(&cur).ok().and_then(|rd| {
            rd.flatten().map(|e| e.path()).find(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase() == want).unwrap_or(false))
        });
        match found {
            Some(p) => cur = p,
            None => return exact,
        }
    }
    cur
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
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file() {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

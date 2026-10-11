//! The content index (docs/MODDING.md, section 7, and docs/CONTENT_INDEX.md): `index.json`, the list of packs the game can
//! download from Preferences > Content > Find more, and installing a downloaded pack after its checksum is checked.
//!
//! The game itself only reads the index and the packs' .zip files; downloading is the game's (crates/simgolf/src/net.rs):
//! an HTTPS client on the desktop, the page's fetch() in a browser. Local addresses (`file://...` or a plain path) are read
//! here, which is also how the tests run.
use crate::packs::{Found, Kind, Manifest, Requirement, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The index format this game reads.
pub const FORMAT: u32 = 1;
/// The largest pack the game downloads (bytes).
pub const MAX_PACK_SIZE: u64 = 256 * 1024 * 1024;

/// One pack of the index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: Kind,
    /// An SPDX licence identifier (free licences only, docs/CONTENT_INDEX.md).
    #[serde(alias = "license")]
    pub licence: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    /// Where the pack's .zip is: an https:// address, or a path relative to the index.
    pub url: String,
    /// The SHA-256 of the .zip, 64 hex digits.
    pub sha256: String,
    /// The .zip's size in bytes.
    pub size: u64,
    #[serde(default)]
    pub requires: Vec<String>,
}

impl Entry {
    fn check(&self) -> Result<(), String> {
        if !crate::packs::valid_id(&self.id) || self.id == crate::packs::ORIGINAL {
            return Err(format!("bad id \"{}\"", self.id));
        }
        if self.name.trim().is_empty() {
            return Err(format!("{}: name is empty", self.id));
        }
        Version::parse(&self.version).map_err(|e| format!("{}: {e}", self.id))?;
        if self.sha256.len() != 64 || !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{}: sha256 must be 64 hex digits", self.id));
        }
        if self.url.trim().is_empty() {
            return Err(format!("{}: url is empty", self.id));
        }
        if self.size == 0 || self.size > MAX_PACK_SIZE {
            return Err(format!("{}: size {} is not between 1 and {MAX_PACK_SIZE}", self.id, self.size));
        }
        for r in &self.requires {
            Requirement::parse(r).map_err(|e| format!("{}: {e}", self.id))?;
        }
        Ok(())
    }
}

/// The whole index.json.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    #[serde(default = "format_one")]
    pub format: u32,
    pub packs: Vec<Entry>,
}

fn format_one() -> u32 {
    FORMAT
}

impl Index {
    /// Reads an index.json. Entries with a problem are left out and reported; a file that is not an index is an error.
    pub fn parse(data: &[u8]) -> Result<(Index, Vec<String>), String> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default = "format_one")]
            format: u32,
            packs: Vec<serde_json::Value>,
        }
        let raw: Raw = serde_json::from_slice(data).map_err(|e| format!("index.json: {e}"))?;
        if raw.format > FORMAT {
            return Err(format!("index.json: format {} is newer than this game reads ({FORMAT}): update the game", raw.format));
        }
        let mut warnings = Vec::new();
        let mut packs: Vec<Entry> = Vec::new();
        for (k, v) in raw.packs.into_iter().enumerate() {
            match serde_json::from_value::<Entry>(v).map_err(|e| e.to_string()).and_then(|e| e.check().map(|_| e)) {
                Ok(e) if packs.iter().any(|p| p.id == e.id) => warnings.push(format!("pack {}: {} is listed twice", k + 1, e.id)),
                Ok(e) => packs.push(e),
                Err(e) => warnings.push(format!("pack {}: {e}", k + 1)),
            }
        }
        Ok((Index { format: raw.format, packs }, warnings))
    }
}

/// A pack the player can download: not installed, or newer than the installed one.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub entry: Entry,
    /// The installed version, for an update.
    pub installed: Option<String>,
}

/// The packs of the index the player does not have, or has in an older version, by kind and name.
pub fn offers(index: &Index, found: &Found) -> Vec<Offer> {
    let mut v: Vec<Offer> = index
        .packs
        .iter()
        .filter_map(|e| match found.packs.iter().find(|p| p.manifest.id == e.id) {
            None => Some(Offer { entry: e.clone(), installed: None }),
            Some(p) if Version::parse(&e.version).map(|v| v > p.manifest.version()).unwrap_or(false) => {
                Some(Offer { entry: e.clone(), installed: Some(p.manifest.version.clone()) })
            }
            Some(_) => None,
        })
        .collect();
    v.sort_by_key(|o| (o.entry.kind as u8, o.entry.name.to_lowercase()));
    v
}

/// The SHA-256 of `data` in lower case hex.
pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks a downloaded .zip against its index entry: size, checksum, and a pack.toml with the entry's id and version.
pub fn check_download(entry: &Entry, data: &[u8]) -> Result<Manifest, String> {
    if data.len() as u64 != entry.size {
        return Err(format!("the download has {} bytes, the index says {}", data.len(), entry.size));
    }
    let sum = sha256_hex(data);
    if !sum.eq_ignore_ascii_case(&entry.sha256) {
        return Err(format!("the checksum does not match (got {sum}): the file is damaged or not the one the index lists"));
    }
    let (m, _) = crate::packs::open_zip(data.to_vec())?;
    if m.id != entry.id {
        return Err(format!("the pack's id is {}, the index says {}", m.id, entry.id));
    }
    if m.version() != Version::parse(&entry.version).unwrap_or(Version(vec![0])) {
        return Err(format!("the pack's version is {}, the index says {}", m.version, entry.version));
    }
    Ok(m)
}

/// Installs a downloaded pack into `packs_dir` (the save folder's packs) as `<id>.zip`, after [`check_download`]. An older
/// copy of the pack in `packs_dir` (a .zip) is replaced; one in a folder there must be removed by the player first.
pub fn install(entry: &Entry, data: &[u8], packs_dir: &Path, found: &Found) -> Result<PathBuf, String> {
    check_download(entry, data)?;
    let dest = packs_dir.join(format!("{}.zip", entry.id));
    let old = found.packs.iter().find(|p| p.manifest.id == entry.id).map(|p| PathBuf::from(&p.location));
    let old_here = old.filter(|o| o.parent() == Some(packs_dir));
    if let Some(o) = &old_here {
        if crate::fsutil::is_dir(o) {
            return Err(format!("an older version is installed in the folder {}: remove that folder first", o.display()));
        }
    }
    if !crate::fsutil::write_file(&dest, data) {
        return Err(format!("could not write {}", dest.display()));
    }
    if let Some(o) = old_here.filter(|o| *o != dest) {
        crate::fsutil::remove_file(o);
    }
    Ok(dest)
}

/// The address of a pack given in the index: absolute (has a scheme), or relative to the index's own address.
pub fn resolve_url(index_url: &str, url: &str) -> String {
    if url.contains("://") || url.starts_with('/') || index_url.is_empty() {
        return url.to_string();
    }
    match index_url.rfind('/') {
        Some(k) => format!("{}{url}", &index_url[..=k]),
        None => url.to_string(),
    }
}

/// True for addresses the game reads itself (`file://` or a plain path); false for http and https.
pub fn is_local(url: &str) -> bool {
    url.starts_with("file://") || !url.contains("://")
}

/// Reads a local address (`file:///path` or a plain path).
pub fn read_local(url: &str) -> Result<Vec<u8>, String> {
    let path = url.strip_prefix("file://").unwrap_or(url);
    // file://localhost/path
    let path = path.strip_prefix("localhost").unwrap_or(path);
    crate::fsutil::read_raw(path).ok_or_else(|| format!("cannot read {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_zip(id: &str, version: &str) -> Vec<u8> {
        let toml = format!("[pack]\nid = \"{id}\"\nname = \"Pack {id}\"\nversion = \"{version}\"\nkind = \"mod\"\nlicense = \"CC0-1.0\"\n");
        crate::zip::write(&[
            (&format!("{id}/pack.toml") as &str, toml.as_bytes()),
            (&format!("{id}/data/terrain.toml") as &str, b"[[terrain]]\nid = \"fairway\"\ncost = 200\n" as &[u8]),
        ])
    }

    fn entry_json(id: &str, version: &str, zip: &[u8], url: &str) -> String {
        format!(
            "{{\"id\": \"{id}\", \"name\": \"Pack {id}\", \"version\": \"{version}\", \"kind\": \"mod\", \"licence\": \"CC0-1.0\", \
             \"authors\": [\"Tester\"], \"description\": \"A test.\", \"url\": \"{url}\", \"sha256\": \"{}\", \"size\": {}, \
             \"requires\": [\"original >= 1.0\"]}}",
            sha256_hex(zip),
            zip.len()
        )
    }

    #[test]
    fn sha256_of_known_values() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn urls() {
        assert_eq!(resolve_url("https://x.org/content/index.json", "packs/a.zip"), "https://x.org/content/packs/a.zip");
        assert_eq!(resolve_url("https://x.org/index.json", "https://y.org/a.zip"), "https://y.org/a.zip");
        assert_eq!(resolve_url("file:///tmp/c/index.json", "a.zip"), "file:///tmp/c/a.zip");
        assert!(is_local("file:///tmp/a") && is_local("/tmp/a") && !is_local("https://x.org/a"));
    }

    #[test]
    fn parse_offer_check_and_install_from_a_local_index() {
        let dir = std::env::temp_dir().join(format!("sg-index-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (server, packs) = (dir.join("server"), dir.join("packs"));
        std::fs::create_dir_all(&server).unwrap();
        std::fs::create_dir_all(&packs).unwrap();

        // the "server": an index with two good packs and two bad entries, and the packs' zips next to it
        let (a1, a2, b) = (pack_zip("pack-a", "1.0.0"), pack_zip("pack-a", "1.1.0"), pack_zip("pack-b", "2.0"));
        std::fs::write(server.join("pack-a.zip"), &a2).unwrap();
        std::fs::write(server.join("pack-b.zip"), &b).unwrap();
        let index = format!(
            "{{\"format\": 1, \"packs\": [{}, {}, {{\"id\": \"Bad Id\"}}, {}]}}",
            entry_json("pack-a", "1.1.0", &a2, "pack-a.zip"),
            entry_json("pack-b", "2.0", &b, "pack-b.zip"),
            entry_json("pack-b", "2.0", &b, "pack-b.zip"),
        );
        std::fs::write(server.join("index.json"), &index).unwrap();
        let index_url = format!("file://{}", server.join("index.json").display());
        let (idx, warnings) = Index::parse(&read_local(&index_url).unwrap()).unwrap();
        assert_eq!(idx.packs.len(), 2);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(Index::parse(b"{\"format\": 99, \"packs\": []}").unwrap_err().contains("newer"));
        assert!(Index::parse(b"<html>").is_err());

        // pack-a 1.0.0 is installed: 1.1.0 is offered as an update, pack-b as new
        std::fs::write(packs.join("pack-a.zip"), &a1).unwrap();
        let found = crate::packs::discover(std::slice::from_ref(&packs));
        let offers = offers(&idx, &found);
        assert_eq!(offers.len(), 2);
        assert_eq!(offers[0].installed.as_deref(), Some("1.0.0"));
        assert_eq!(offers[1].installed, None);

        // a damaged download is refused
        let e = &offers[0].entry;
        let mut bad = a2.clone();
        bad[40] ^= 1;
        assert!(check_download(e, &bad).unwrap_err().contains("checksum"));
        assert!(check_download(e, &a2[..10]).unwrap_err().contains("bytes"));
        // a zip whose pack.toml says another version
        let mut wrong = e.clone();
        wrong.sha256 = sha256_hex(&a1);
        wrong.size = a1.len() as u64;
        assert!(check_download(&wrong, &a1).unwrap_err().contains("version"));

        // download (locally) and install both
        for o in &offers {
            let data = read_local(&resolve_url(&index_url, &o.entry.url)).unwrap();
            install(&o.entry, &data, &packs, &found).unwrap();
        }
        let found = crate::packs::discover(&[packs]);
        // (the built-in free base set is always found too)
        let ids: Vec<(&str, &str)> = found
            .packs
            .iter()
            .filter(|p| p.manifest.id != crate::packs::FREE)
            .map(|p| (p.manifest.id.as_str(), p.manifest.version.as_str()))
            .collect();
        assert_eq!(ids, [("pack-a", "1.1.0"), ("pack-b", "2.0")]);
        assert!(found.broken.is_empty(), "{:?}", found.broken);
        assert!(super::offers(&idx, &found).is_empty(), "nothing left to offer");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

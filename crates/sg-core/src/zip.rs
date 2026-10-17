//! A small reader for .zip files, enough for content packs (docs/MODDING.md): the central directory, and entries that are
//! stored or deflated. The whole archive is kept in memory (packs are small, and in a browser it is already in memory).
use std::collections::BTreeMap;

pub struct Zip {
    data: Vec<u8>,
    /// Entry name (as stored, forward slashes) -> (method, compressed size, uncompressed size, local header offset).
    entries: BTreeMap<String, (u16, usize, usize, usize)>,
    /// Entry name -> its Unix permission bits, for entries made on a Unix system (a Mac's `ditto`, Info-ZIP).
    modes: BTreeMap<String, u32>,
}

fn u16le(d: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
}
fn u32le(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

impl Zip {
    /// Reads the central directory. Errors name what is wrong with the file.
    pub fn open(data: Vec<u8>) -> Result<Zip, String> {
        if data.len() < 22 {
            return Err("not a zip file (too short)".into());
        }
        // the end of central directory record: the last "PK\5\6" (a comment of up to 64 KiB may follow it)
        let lowest = data.len().saturating_sub(22 + 0xffff);
        let eocd = (lowest..=data.len() - 22)
            .rev()
            .find(|&e| data[e..e + 4] == [0x50, 0x4b, 0x05, 0x06])
            .ok_or("not a zip file (no central directory)")?;
        let count = u16le(&data, eocd + 10).ok_or("bad zip")?;
        let mut p = u32le(&data, eocd + 16).ok_or("bad zip")?;
        let mut entries = BTreeMap::new();
        let mut modes = BTreeMap::new();
        for _ in 0..count {
            if data.get(p..p + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
                return Err("bad zip central directory".into());
            }
            let bad = || "bad zip central directory".to_string();
            let method = u16le(&data, p + 10).ok_or_else(bad)? as u16;
            let csize = u32le(&data, p + 20).ok_or_else(bad)?;
            let usize_ = u32le(&data, p + 24).ok_or_else(bad)?;
            let (nlen, xlen, clen) =
                (u16le(&data, p + 28).ok_or_else(bad)?, u16le(&data, p + 30).ok_or_else(bad)?, u16le(&data, p + 32).ok_or_else(bad)?);
            let local = u32le(&data, p + 42).ok_or_else(bad)?;
            let name = String::from_utf8_lossy(data.get(p + 46..p + 46 + nlen).ok_or_else(bad)?).replace('\\', "/");
            // "version made by"'s high byte 3: Unix, whose mode is the external attributes' high half
            let unix = u16le(&data, p + 4).ok_or_else(bad)? >> 8 == 3;
            let mode = (u32le(&data, p + 38).ok_or_else(bad)? >> 16) as u32 & 0o7777;
            p += 46 + nlen + xlen + clen;
            if !name.ends_with('/') {
                if unix && mode != 0 {
                    modes.insert(name.clone(), mode);
                }
                entries.insert(name, (method, csize, usize_, local));
            }
        }
        Ok(Zip { data, entries, modes })
    }

    /// The names of the files in the archive (folders are left out), sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// An entry's Unix permission bits (0o755 for a program), when the archive was made on a Unix system.
    pub fn unix_mode(&self, name: &str) -> Option<u32> {
        self.modes.get(name).copied()
    }

    /// The contents of an entry, by its exact name. None when it is missing, uses a method other than stored or deflate,
    /// or is damaged.
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let &(method, csize, size, local) = self.entries.get(name)?;
        if self.data.get(local..local + 4)? != [0x50, 0x4b, 0x03, 0x04] {
            return None;
        }
        let start = local + 30 + u16le(&self.data, local + 26)? + u16le(&self.data, local + 28)?;
        let body = self.data.get(start..start + csize)?;
        match method {
            0 => Some(body.to_vec()),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(body, size.max(1) * 2 + 1024).ok(),
            _ => None,
        }
    }
}

/// Writes a zip with the given (name, contents) entries, deflated. For tests and tools.
pub fn write(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let crc = |d: &[u8]| {
        let mut c = 0xFFFF_FFFFu32;
        for &b in d {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    };
    for (name, body) in files {
        let z = miniz_oxide::deflate::compress_to_vec(body, 6);
        let offset = out.len() as u32;
        let head = |sig: u32, central: bool| {
            let mut h = Vec::new();
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes());
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // version needed
            h.extend_from_slice(&0u16.to_le_bytes()); // flags
            h.extend_from_slice(&8u16.to_le_bytes()); // deflate
            h.extend_from_slice(&[0; 4]); // time, date
            h.extend_from_slice(&crc(body).to_le_bytes());
            h.extend_from_slice(&(z.len() as u32).to_le_bytes());
            h.extend_from_slice(&(body.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                h.extend_from_slice(&[0; 6]); // comment length, disk, internal attributes
                h.extend_from_slice(&[0; 4]); // external attributes
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name.as_bytes());
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend_from_slice(&z);
        central.extend(head(0x0201_4b50, true));
    }
    let cd = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd.to_le_bytes());
    out.extend_from_slice(&[0; 2]);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn zip_round_trip() {
        let z = super::write(&[("a/pack.toml", b"hello"), ("a/gfx/x.png", &[7u8; 300])]);
        let zip = super::Zip::open(z).unwrap();
        assert_eq!(zip.names().collect::<Vec<_>>(), ["a/gfx/x.png", "a/pack.toml"]);
        assert_eq!(zip.read("a/pack.toml").unwrap(), b"hello");
        assert_eq!(zip.read("a/gfx/x.png").unwrap(), vec![7u8; 300]);
        assert!(zip.read("missing").is_none());
        assert!(super::Zip::open(b"not a zip at all, sorry, no".to_vec()).is_err());
    }

    #[test]
    fn unix_modes_are_read() {
        let mut z = super::write(&[("app/run", b"#!/bin/sh"), ("app/notes.txt", b"hi")]);
        assert_eq!(super::Zip::open(z.clone()).unwrap().unix_mode("app/run"), None, "made on no Unix system");
        // the first central entry made on Unix (version made by 0x0314) with mode 0o100755
        let cd = u32::from_le_bytes(z[z.len() - 6..z.len() - 2].try_into().unwrap()) as usize;
        z[cd + 4..cd + 6].copy_from_slice(&0x0314u16.to_le_bytes());
        z[cd + 38..cd + 42].copy_from_slice(&(0o100755u32 << 16).to_le_bytes());
        let zip = super::Zip::open(z).unwrap();
        assert_eq!(zip.unix_mode("app/run"), Some(0o755));
        assert_eq!(zip.unix_mode("app/notes.txt"), None);
        assert_eq!(zip.read("app/run").unwrap(), b"#!/bin/sh");
    }
}

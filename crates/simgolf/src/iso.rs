//! ISO 9660/Joliet and the InstallShield 6 cabinets on the SimGolf CD. Reads files directly, never mounts a disc or runs
//! its installer. Cabinet layout reference: https://github.com/twogood/unshield (lib/cabfile.h).
use crate::import::{safe_rel, skip, Error, Unzipped};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn bad(message: impl Into<String>) -> Error {
    Error::BadDisc(message.into())
}

/// Scratch files owned by this import; removed on success and failure.
pub(crate) struct Workspace {
    pub path: PathBuf,
}
impl Workspace {
    pub fn new(dest: &Path) -> Result<Self, Error> {
        let parent = dest.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = parent.join(format!(".opensgolf-media-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&path)?;
        Ok(Self { path })
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn bytes(d: &[u8], at: usize, len: usize) -> Result<&[u8], Error> {
    d.get(at..at.checked_add(len).ok_or_else(|| bad("invalid offset"))?).ok_or_else(|| bad("truncated disc or cabinet record"))
}
fn word(d: &[u8], at: usize) -> Result<usize, Error> {
    Ok(u16::from_le_bytes(bytes(d, at, 2)?.try_into().unwrap()) as usize)
}
fn long(d: &[u8], at: usize) -> Result<usize, Error> {
    Ok(u32::from_le_bytes(bytes(d, at, 4)?.try_into().unwrap()) as usize)
}
fn wide(d: &[u8], at: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(bytes(d, at, 8)?.try_into().unwrap()))
}
fn string(d: &[u8], at: usize) -> Result<String, Error> {
    let tail = d.get(at..).ok_or_else(|| bad("invalid string offset"))?;
    let end = tail.iter().position(|&b| b == 0).ok_or_else(|| bad("unterminated cabinet filename"))?;
    // The original CD uses Windows-1252 filenames. Preserve the non-ASCII characters as well as ASCII paths.
    const CP1252: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•',
        '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    Ok(tail[..end].iter().map(|&b| if (128..160).contains(&b) { CP1252[(b - 128) as usize] } else { b as char }).collect())
}

struct Disc {
    file: File,
    len: u64,
}
#[derive(Clone)]
struct Entry {
    name: String,
    offset: u64,
    size: u64,
}
impl Disc {
    fn read(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, Error> {
        if offset.checked_add(size as u64).is_none_or(|end| end > self.len) {
            return Err(bad("disc image is cut short"));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        let mut data = vec![0; size];
        self.file.read_exact(&mut data)?;
        Ok(data)
    }
    fn entries(&mut self) -> Result<Vec<Entry>, Error> {
        let mut root = None;
        let mut unicode = false;
        for sector in 16..80 {
            let d = self.read(sector * 2048, 2048)?;
            if &d[1..6] != b"CD001" || d[6] != 1 {
                return Err(bad("not an ISO 9660 disc image"));
            }
            if d[0] == 255 {
                break;
            }
            let joliet = d[0] == 2 && [&b"%/@"[..], &b"%/C"[..], &b"%/E"[..]].contains(&&d[88..91]);
            if joliet || (d[0] == 1 && root.is_none()) {
                root = Some((long(&d, 158)? as u64 * 2048, long(&d, 166)?));
                unicode = joliet;
            }
        }
        let (offset, size) = root.ok_or_else(|| bad("ISO has no readable filesystem"))?;
        let mut stack = vec![(String::new(), offset, size, 0)];
        let mut visited = BTreeSet::new();
        let mut out = Vec::new();
        while let Some((parent, offset, size, depth)) = stack.pop() {
            if !visited.insert(offset) {
                continue;
            }
            if depth > 32 || size > 16 * 1024 * 1024 || visited.len() > 10000 {
                return Err(bad("ISO directory limits exceeded"));
            }
            let data = self.read(offset, size)?;
            let mut pos = 0;
            while pos < data.len() {
                let len = data[pos] as usize;
                if len == 0 {
                    pos = (pos / 2048 + 1) * 2048;
                    continue;
                }
                let record = bytes(&data, pos, len)?;
                if len < 34 {
                    return Err(bad("invalid ISO directory record"));
                }
                let raw = bytes(record, 33, record[32] as usize)?;
                pos += len;
                if raw == [0] || raw == [1] {
                    continue;
                }
                let name = if unicode {
                    if raw.len() % 2 != 0 {
                        return Err(bad("invalid Joliet filename"));
                    }
                    String::from_utf16(&raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>())
                        .map_err(|_| bad("invalid Joliet filename"))?
                } else {
                    String::from_utf8_lossy(raw).into_owned()
                };
                let name = name.split(';').next().unwrap_or("").trim_end_matches('.');
                if name.is_empty() || name.contains(['/', '\\', ':']) || name == ".." {
                    return Err(bad("unsafe ISO filename"));
                }
                let name = if parent.is_empty() { name.to_string() } else { format!("{parent}/{name}") };
                let offset = long(record, 2)? as u64 * 2048;
                let size = long(record, 10)? as u64;
                if record[25] & 0x80 != 0 {
                    return Err(bad("multi-extent ISO files are not supported"));
                }
                if offset.checked_add(size).is_none_or(|end| end > self.len) {
                    return Err(bad("ISO file extends beyond the disc"));
                }
                if record[25] & 2 != 0 {
                    stack.push((name, offset, size as usize, depth + 1));
                } else {
                    out.push(Entry { name, offset, size });
                }
                if out.len() > 100000 {
                    return Err(bad("too many ISO files"));
                }
            }
        }
        Ok(out)
    }
    fn copy(&mut self, entry: &Entry, path: &Path) -> Result<(), Error> {
        self.file.seek(SeekFrom::Start(entry.offset))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(path)?;
        if std::io::copy(&mut (&mut self.file).take(entry.size), &mut output)? != entry.size {
            return Err(bad("disc file is cut short"));
        }
        Ok(())
    }
}

pub fn unpack(source: &Path, dest: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<Unzipped, Error> {
    let file = File::open(source)?;
    let mut disc = Disc { len: file.metadata()?.len(), file };
    let entries = disc.entries()?;
    if let Some(root) = crate::import::root_of(entries.iter().map(|e| e.name.as_str())) {
        let prefix = if root.is_empty() { root } else { format!("{root}/") };
        let wanted: Vec<_> = entries.iter().filter(|e| e.name.starts_with(&prefix) && !skip(&e.name.to_lowercase())).collect();
        std::fs::create_dir_all(dest)?;
        let mut result = Unzipped::default();
        for (i, entry) in wanted.iter().enumerate() {
            if let Some(rel) = safe_rel(&entry.name[prefix.len()..]) {
                disc.copy(entry, &dest.join(rel))?;
                result.files += 1;
            }
            progress(i + 1, wanted.len());
        }
        return Ok(result);
    }
    let header =
        entries.iter().find(|e| e.name.rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case("data1.hdr"))).ok_or(Error::NoGame)?;
    let parent = header.name.rsplit_once('/').map_or("", |(p, _)| p);
    let work = Workspace::new(dest)?;
    let mut cabinets = BTreeMap::new();
    for entry in &entries {
        if entry.name.rsplit_once('/').map_or("", |(p, _)| p) != parent {
            continue;
        }
        let name = entry.name.rsplit('/').next().unwrap_or("").to_lowercase();
        if let Some(number) = name.strip_prefix("data").and_then(|n| n.strip_suffix(".cab")).and_then(|n| n.parse::<u16>().ok()) {
            let path = work.path.join(&name);
            disc.copy(entry, &path)?;
            cabinets.insert(number, path);
        }
    }
    if header.size > 16 * 1024 * 1024 {
        return Err(bad("installer header is too large"));
    }
    let data = disc.read(header.offset, header.size as usize)?;
    unpack_cabinets(&data, &cabinets, dest, progress)
}

fn unpack_cabinets(
    header: &[u8],
    cabinets: &BTreeMap<u16, PathBuf>,
    dest: &Path,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Unzipped, Error> {
    if bytes(header, 0, 4)? != b"ISc(" {
        return Err(bad("invalid InstallShield header"));
    }
    let version = long(header, 4)?;
    let major = if version >> 24 == 1 { (version >> 12) & 15 } else { (version & 65535) / 100 };
    if !(6..=16).contains(&major) {
        return Err(bad(format!("InstallShield {major} is not supported (expected version 6–16)")));
    }
    let descriptor = long(header, 12)?;
    bytes(header, descriptor, 48)?;
    let table = descriptor.checked_add(long(header, descriptor + 12)?).ok_or_else(|| bad("invalid file table"))?;
    let directories = long(header, descriptor + 28)?;
    let count = long(header, descriptor + 40)?;
    let records = table.checked_add(long(header, descriptor + 44)?).ok_or_else(|| bad("invalid file records"))?;
    if count > 100000 || directories > 10000 {
        return Err(bad("installer file limits exceeded"));
    }
    bytes(header, records, count * 87)?;
    bytes(header, table, directories * 4)?;
    struct Item {
        name: String,
        flags: usize,
        size: u64,
        compressed: u64,
        offset: u64,
        volume: u16,
        index: usize,
    }
    let mut items = Vec::new();
    for i in 0..count {
        let r = bytes(header, records + i * 87, 87)?;
        let flags = word(r, 0)?;
        if flags & 8 != 0 || wide(r, 18)? == 0 {
            continue;
        }
        let dir = word(r, 62)?;
        if dir >= directories {
            return Err(bad("invalid cabinet directory"));
        }
        let directory_at = table.checked_add(long(header, table + dir * 4)?).ok_or_else(|| bad("invalid directory offset"))?;
        let name_at = table.checked_add(long(r, 58)?).ok_or_else(|| bad("invalid filename offset"))?;
        let directory = string(header, directory_at)?.replace('\\', "/");
        let name = string(header, name_at)?;
        if name.contains(['/', '\\', ':']) {
            return Err(bad("unsafe cabinet filename"));
        }
        let name = if directory.is_empty() { name } else { format!("{directory}/{name}") };
        if skip(&name.to_lowercase()) {
            continue;
        }
        if name.starts_with('/') || safe_rel(&name).is_none() {
            return Err(bad("unsafe cabinet path"));
        }
        // InstallShield deduplicates identical files by referring to an earlier record. Resolve the data record while
        // keeping this entry's destination name; links never become filesystem symlinks.
        let mut r = r;
        let mut source_index = i;
        let mut links = BTreeSet::new();
        while r[84] & 1 != 0 {
            source_index = long(r, 76)?;
            if source_index >= count || !links.insert(source_index) {
                return Err(bad("invalid or cyclic cabinet file link"));
            }
            r = bytes(header, records + source_index * 87, 87)?;
        }
        if word(r, 0)? & 8 != 0 || wide(r, 18)? == 0 {
            return Err(bad("cabinet link refers to an invalid file"));
        }
        items.push(Item {
            name,
            flags: word(r, 0)?,
            size: wide(r, 2)?,
            compressed: wide(r, 10)?,
            offset: wide(r, 18)?,
            volume: word(r, 85)? as u16,
            index: source_index,
        });
    }
    if crate::import::root_of(items.iter().map(|i| i.name.as_str())).is_none() {
        return Err(Error::NoGame);
    }
    std::fs::create_dir_all(dest)?;
    let mut result = Unzipped::default();
    for (i, item) in items.iter().enumerate() {
        if item.size > 256 * 1024 * 1024 || item.compressed > 256 * 1024 * 1024 {
            return Err(bad("cabinet file is too large"));
        }
        let mut body = Vec::new();
        let mut remaining = if item.flags & 4 != 0 { item.compressed } else { item.size };
        let mut volume = item.volume;
        while remaining > 0 {
            let path = cabinets.get(&volume).ok_or_else(|| bad(format!("missing data{volume}.cab")))?;
            let mut file = File::open(path)?;
            let mut h = [0; 84];
            file.read_exact(&mut h)?;
            if &h[..4] != b"ISc(" {
                return Err(bad("invalid cabinet volume"));
            }
            let (offset, available) = if item.flags & 1 != 0 {
                if item.index == long(&h, 32)? {
                    (wide(&h, 60)?, wide(&h, if item.flags & 4 != 0 { 76 } else { 68 })?)
                } else if item.index == long(&h, 28)? {
                    (wide(&h, 36)?, wide(&h, if item.flags & 4 != 0 { 52 } else { 44 })?)
                } else {
                    return Err(bad("invalid split cabinet file"));
                }
            } else {
                (item.offset, remaining)
            };
            let n = available.min(remaining);
            if n == 0 || offset.checked_add(n).is_none_or(|end| end > file.metadata().map(|m| m.len()).unwrap_or(0)) {
                return Err(bad("cabinet file is cut short"));
            }
            file.seek(SeekFrom::Start(offset))?;
            let start = body.len();
            body.resize(start + n as usize, 0);
            file.read_exact(&mut body[start..])?;
            remaining -= n;
            volume = volume.checked_add(1).ok_or_else(|| bad("invalid cabinet volume number"))?;
        }
        if item.flags & 2 != 0 {
            for (i, b) in body.iter_mut().enumerate() {
                *b = (*b ^ 0xd5).rotate_right(2).wrapping_sub((i % 71) as u8);
            }
        }
        let output = dest.join(safe_rel(&item.name).ok_or_else(|| bad("unsafe output path"))?);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(output)?;
        let mut written = 0;
        if item.flags & 4 != 0 {
            let mut pos = 0;
            while pos < body.len() {
                let len = word(&body, pos)?;
                if len == 0 {
                    return Err(bad("empty compressed cabinet block"));
                }
                let data = bytes(&body, pos + 2, len)?;
                let block = miniz_oxide::inflate::decompress_to_vec_with_limit(data, 65536)
                    .map_err(|_| bad(format!("damaged cabinet file: {}", item.name)))?;
                written += block.len() as u64;
                if written > item.size {
                    return Err(bad("cabinet file exceeds its declared size"));
                }
                file.write_all(&block)?;
                pos += 2 + len;
            }
        } else {
            file.write_all(&body)?;
            written = body.len() as u64;
        }
        if written != item.size {
            return Err(bad(format!("incorrect extracted size: {}", item.name)));
        }
        result.files += 1;
        progress(i + 1, items.len());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put32(d: &mut [u8], at: usize, value: u32) {
        d[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn put64(d: &mut [u8], at: usize, value: u64) {
        d[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    fn record(name: &[u8], sector: u32, size: u32, directory: bool) -> Vec<u8> {
        let mut r = vec![0; (33 + name.len() + 1) & !1];
        r[0] = r.len() as u8;
        put32(&mut r, 2, sector);
        put32(&mut r, 10, size);
        r[25] = if directory { 2 } else { 0 };
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }
    fn image(unicode: bool) -> Vec<u8> {
        let mut d = vec![0; 23 * 2048];
        let descriptor = &mut d[16 * 2048..17 * 2048];
        descriptor[0] = if unicode { 2 } else { 1 };
        descriptor[1..6].copy_from_slice(b"CD001");
        descriptor[6] = 1;
        if unicode {
            descriptor[88..91].copy_from_slice(b"%/E");
        }
        descriptor[156..190].copy_from_slice(&record(&[0], 20, 2048, true));
        d[17 * 2048] = 255;
        d[17 * 2048 + 1..17 * 2048 + 6].copy_from_slice(b"CD001");
        d[17 * 2048 + 6] = 1;
        let encode =
            |s: &str| if unicode { s.encode_utf16().flat_map(u16::to_be_bytes).collect::<Vec<_>>() } else { s.as_bytes().to_vec() };
        let root = record(&encode("Flics"), 21, 2048, true);
        d[20 * 2048..20 * 2048 + root.len()].copy_from_slice(&root);
        let file = record(&encode("a.fli;1"), 22, 4, false);
        d[21 * 2048..21 * 2048 + file.len()].copy_from_slice(&file);
        d[22 * 2048..22 * 2048 + 4].copy_from_slice(b"flic");
        d
    }

    #[test]
    fn reads_iso_and_joliet_and_zipped_iso() {
        let work = Workspace::new(&std::env::temp_dir().join("opensgolf-test")).unwrap();
        for unicode in [false, true] {
            let source = work.path.join("disc.iso");
            let data = image(unicode);
            std::fs::write(&source, &data).unwrap();
            let dest = work.path.join(if unicode { "joliet" } else { "iso" });
            assert_eq!(unpack(&source, &dest, &mut |_, _| {}).unwrap().files, 1);
            assert_eq!(std::fs::read(dest.join("Flics/a.fli")).unwrap(), b"flic");
            let zip = work.path.join("disc.zip");
            std::fs::write(&zip, sg_core::zip::write(&[("Top/disc.ISO", &data)])).unwrap();
            assert_eq!(crate::import::unpack_game(&zip, &dest, &mut |_, _| {}).unwrap().files, 1);
        }
        let source = work.path.join("bad.iso");
        let mut data = image(false);
        put32(&mut data, 21 * 2048 + 2, u32::MAX);
        std::fs::write(&source, data).unwrap();
        assert!(matches!(unpack(&source, &work.path.join("bad"), &mut |_, _| {}), Err(Error::BadDisc(_))));
        assert_eq!(
            std::fs::read_dir(&work.path)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with(".opensgolf-media"))
                .count(),
            0
        );
    }

    #[test]
    fn extracts_cabinet_blocks_and_rejects_unsafe_paths() {
        let work = Workspace::new(&std::env::temp_dir().join("opensgolf-test")).unwrap();
        let mut header = vec![0; 700];
        header[..4].copy_from_slice(b"ISc(");
        put32(&mut header, 4, 0x01006000);
        put32(&mut header, 12, 20);
        put32(&mut header, 32, 492); // table at 512
        put32(&mut header, 48, 1); // directories
        put32(&mut header, 60, 1); // files
        put32(&mut header, 64, 64); // records at 576
        put32(&mut header, 512, 4);
        header[516..522].copy_from_slice(b"Flics\0");
        header[522..528].copy_from_slice(b"a.fli\0");
        let compressed = miniz_oxide::deflate::compress_to_vec(b"flic", 6);
        let mut body = (compressed.len() as u16).to_le_bytes().to_vec();
        body.extend(compressed);
        let r = &mut header[576..663];
        r[0] = 4;
        put64(r, 2, 4);
        put64(r, 10, body.len() as u64);
        put64(r, 18, 84);
        put32(r, 58, 10);
        r[85] = 1;
        let mut cabinet = vec![0; 84];
        cabinet[..4].copy_from_slice(b"ISc(");
        cabinet.extend(&body);
        let path = work.path.join("data1.cab");
        std::fs::write(&path, &cabinet).unwrap();
        let mut volumes = BTreeMap::from([(1, path)]);
        let dest = work.path.join("game");
        assert_eq!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}).unwrap().files, 1);
        assert_eq!(std::fs::read(dest.join("Flics/a.fli")).unwrap(), b"flic");
        // The compressed block (even its length prefix) may cross cabinet volumes.
        let mut first = cabinet[..84].to_vec();
        put32(&mut first, 32, 0);
        put64(&mut first, 60, 84);
        put64(&mut first, 76, 1);
        first.extend(&body[..1]);
        let mut second = cabinet[..84].to_vec();
        put32(&mut second, 28, 0);
        put32(&mut second, 32, 1);
        put64(&mut second, 36, 84);
        put64(&mut second, 52, (body.len() - 1) as u64);
        second.extend(&body[1..]);
        std::fs::write(&volumes[&1], first).unwrap();
        let path = work.path.join("data2.cab");
        std::fs::write(&path, second).unwrap();
        volumes.insert(2, path);
        header[576] = 5;
        assert_eq!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}).unwrap().files, 1);
        assert_eq!(std::fs::read(dest.join("Flics/a.fli")).unwrap(), b"flic");
        header[576] = 4;
        std::fs::write(&volumes[&1], cabinet).unwrap();
        // A deduplicated entry takes its bytes from its predecessor, including chains; cycles must fail.
        header.resize(750, 0);
        put32(&mut header, 60, 2);
        header[528..534].copy_from_slice(b"b.fli\0");
        let previous = header[576..663].to_vec();
        header[663..750].copy_from_slice(&previous);
        put32(&mut header, 663 + 58, 16);
        header[663 + 84] = 1;
        assert_eq!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}).unwrap().files, 2);
        assert_eq!(std::fs::read(dest.join("Flics/b.fli")).unwrap(), b"flic");
        put32(&mut header, 663 + 76, 1);
        assert!(matches!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}), Err(Error::BadDisc(_))));
        put32(&mut header, 60, 1);
        header[516..522].copy_from_slice(b"../xx\0");
        assert!(matches!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}), Err(Error::BadDisc(_))));
        header[516..522].copy_from_slice(b"Flics\0");
        put64(&mut header, 594, u64::MAX);
        assert!(matches!(unpack_cabinets(&header, &volumes, &dest, &mut |_, _| {}), Err(Error::BadDisc(_))));
    }

    /// Local integration check; copyrighted game data is never included in the repository.
    #[test]
    #[ignore = "set OPENSGOLF_TEST_MEDIA to an original SimGolf ISO or ZIP containing it"]
    fn original_disc_import() {
        let source = PathBuf::from(std::env::var_os("OPENSGOLF_TEST_MEDIA").expect("OPENSGOLF_TEST_MEDIA"));
        let work = Workspace::new(&std::env::temp_dir().join("opensgolf-test")).unwrap();
        let dest = work.path.join("game");
        let result = crate::import::unpack_game(&source, &dest, &mut |_, _| {}).unwrap();
        assert!(crate::gamefiles::missing(&dest).is_empty());
        let mut mixer = sg_core::mixer::Mixer::new(&dest.join("Sounds"));
        mixer.add_dir("simsfx", &dest.join("SimsFX"));
        assert!(mixer.list("").len() >= 317);
        assert!(mixer.play("GolfAmbience122.wav", 1.0, false) >= 0);
        let mut buffer = vec![0; 44100];
        mixer.mix(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0));
        if let Some(reference) = std::env::var_os("OPENSGOLF_TEST_AUDIO") {
            let reference = PathBuf::from(reference);
            // Unshield replaces directory spaces with underscores in its output.
            let key = |p: &Path| p.to_string_lossy().replace('\\', "/").replace('_', " ").to_lowercase();
            let extracted: BTreeMap<_, _> =
                sg_core::fsutil::walk(&dest).into_iter().map(|p| (key(p.strip_prefix(&dest).unwrap()), p)).collect();
            let mut compared = 0;
            for path in sg_core::fsutil::walk(&reference) {
                if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("wav")) {
                    let relative = path.strip_prefix(&reference).unwrap();
                    let actual = extracted.get(&key(relative)).unwrap_or_else(|| panic!("missing {}", relative.display()));
                    assert_eq!(std::fs::read(&path).unwrap(), std::fs::read(actual).unwrap(), "{}", relative.display());
                    compared += 1;
                }
            }
            assert!(compared >= 317);
            eprintln!("Compared {compared} original clips with independently extracted reference files");
        }
        eprintln!("Imported {} game files and {} original sounds", result.files, mixer.list("").len());
    }
}

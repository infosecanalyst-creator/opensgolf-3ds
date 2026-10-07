//! Parsers for the game's non-image data formats. Formats were worked out from the original files; see docs/FORMATS.md.
use crate::assets::{decode_pcx, u16le, u32le, Rgba};

/// .chr / .glf / .pro : character definition.
///   0x000 char[16] title, 0x010 char[16] name, 0x020 u8[16] raw attribute bytes (meaning not yet decoded), zero padding up to 0x230,
///   0x230 char[50] x 25 dialogue slots (empty slot = all zero), then 16 zero bytes -> 1826 bytes;
///   0x722 optional: "*PCXFILE" tag + 8-bit PCX, 140x420 = three 140x140 portraits stacked (happy, neutral, angry).
#[derive(Clone, Debug, Default)]
pub struct Character {
    pub title: String,
    pub name: String,
    pub attr: [u8; 16],
    /// Always 25 entries, "" when unused.
    pub lines: Vec<String>,
    /// 3 images when present, else empty.
    pub portraits: Vec<Rgba>,
}

fn cstr(d: &[u8], max: usize) -> String {
    let n = d.iter().take(max).position(|&b| b == 0).unwrap_or(max.min(d.len()));
    // The game's text is Windows-1252; Latin-1 is the same for every printable character it uses.
    d[..n].iter().map(|&b| b as char).collect()
}

const BASE_SIZE: usize = 1826;
const SLOTS_AT: usize = 0x230;
const SLOT_SIZE: usize = 50;
const SLOT_COUNT: usize = 25;

pub fn parse_character(d: &[u8]) -> Result<Character, String> {
    if d.len() < BASE_SIZE {
        return Err("character file too short".into());
    }
    let mut out = Character { title: cstr(&d[0..], 16), name: cstr(&d[16..], 16), ..Default::default() };
    out.attr.copy_from_slice(&d[0x20..0x30]);
    for i in 0..SLOT_COUNT {
        out.lines.push(cstr(&d[SLOTS_AT + i * SLOT_SIZE..], SLOT_SIZE));
    }
    if d.len() == BASE_SIZE {
        return Ok(out);
    }
    if d.len() < BASE_SIZE + 8 || &d[BASE_SIZE..BASE_SIZE + 8] != b"*PCXFILE" {
        return Err("unknown data after base record".into());
    }
    let sheet = decode_pcx(&d[BASE_SIZE + 8..])?;
    if sheet.w != 140 || sheet.h != 420 {
        return Err("unexpected portrait size".into());
    }
    let face = 140 * 140 * 4;
    for i in 0..3 {
        out.portraits.push(Rgba { w: 140, h: 140, px: sheet.px[i * face..(i + 1) * face].to_vec() });
    }
    Ok(out)
}

/// top10.sve : ten fixed 156-byte records. char[64] player, char[64] course, u32 a (descending sort key), u32 b, u32 c,
/// u32 0, u32 0, u16 0, u16 tier, i32 -1
#[derive(Clone, Debug, Default)]
pub struct Top10Entry {
    pub player: String,
    pub course: String,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub tier: u16,
}

pub fn parse_top10(d: &[u8]) -> Result<Vec<Top10Entry>, String> {
    const R: usize = 156;
    if d.is_empty() || !d.len().is_multiple_of(R) {
        return Err("top10 size is not a multiple of 156".into());
    }
    Ok(d.chunks(R)
        .map(|r| Top10Entry {
            player: cstr(r, 64),
            course: cstr(&r[64..], 64),
            a: u32le(r, 128),
            b: u32le(r, 132),
            c: u32le(r, 136),
            tier: u16le(r, 150),
        })
        .collect())
}

fn trim(s: &str) -> &str {
    s.trim_matches(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n')
}

/// Text from the disc as a String. The files are Windows-1252; bytes are mapped one to one (Latin-1).
pub fn latin1(d: &[u8]) -> String {
    d.iter().map(|&b| b as char).collect()
}

/// .dta : text tables. Lines starting with '*' are comments; other non-blank lines are CSV rows (fields trimmed).
/// celebrities.dta: name,type,skin,hair,shirt,pants. progolfers.dta: name,body,skin,hat,shirt,pants,<10 skill digits>.
pub type DtaRows = Vec<Vec<String>>;

pub fn parse_dta(text: &str) -> DtaRows {
    text.split('\n')
        .map(trim)
        .filter(|l| !l.is_empty() && !l.starts_with('*'))
        .map(|l| l.split(',').map(|f| trim(f).to_string()).collect())
        .collect()
}

/// One row of progolfers.dta. skill[] order (from the file's own header): power hitter, long driver, accurate driver, accurate irons,
/// accurate putter, draw shot, fade shot, high backspin shot, recovery skills, luck. Each is 0..15 (hex digit).
#[derive(Clone, Debug, Default)]
pub struct ProGolfer {
    pub name: String,
    pub body: i32,
    pub skin: i32,
    pub hat: i32,
    pub shirt: i32,
    pub pants: i32,
    pub skill: [i32; 10],
    /// Trailing number on some rows (30..115), meaning unknown.
    pub rating: i32,
}

/// C's atoi: optional sign and leading digits, 0 when none.
pub fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut v: i64 = 0;
    for c in rest.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        v = (v * 10 + (c - b'0') as i64).min(i32::MAX as i64 + 1);
    }
    (if neg { -v } else { v }) as i32
}

pub fn parse_pro_golfers(text: &str) -> Result<Vec<ProGolfer>, String> {
    let mut out = Vec::new();
    for r in parse_dta(text) {
        if r.len() < 7 {
            continue; // header or oddly shaped row
        }
        let s = r[6].as_bytes();
        // The skill field is ten hex digits, optionally followed by whitespace and a rating.
        let n = s.iter().take_while(|c| c.is_ascii_hexdigit()).count();
        if n < 10 {
            continue;
        }
        let mut g = ProGolfer {
            name: r[0].clone(),
            body: atoi(&r[1]),
            skin: atoi(&r[2]),
            hat: atoi(&r[3]),
            shirt: atoi(&r[4]),
            pants: atoi(&r[5]),
            ..Default::default()
        };
        for k in 0..10 {
            g.skill[k] = (s[k] as char).to_digit(16).unwrap_or(0) as i32;
        }
        if s.len() > 10 {
            g.rating = atoi(&r[6][10..]);
        }
        out.push(g);
    }
    if out.is_empty() {
        return Err("no golfers found".into());
    }
    Ok(out)
}

/// Themes/*/*.txt : conversation scripts. First line is the title; the rest are blank-line separated blocks, each a prompt line
/// followed by 1-3 indented reply lines. Text may contain placeholders such as PARTNER and DATA that the game substitutes.
#[derive(Clone, Debug, Default)]
pub struct StoryBlock {
    pub prompt: String,
    pub replies: Vec<String>,
}
#[derive(Clone, Debug, Default)]
pub struct Story {
    pub title: String,
    pub blocks: Vec<StoryBlock>,
}

pub fn parse_story(text: &str) -> Result<Story, String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut i = 0;
    while i < lines.len() && trim(lines[i]).is_empty() {
        i += 1;
    }
    if i >= lines.len() {
        return Err("empty story".into());
    }
    let mut out = Story { title: trim(lines[i]).to_string(), blocks: Vec::new() };
    i += 1;
    let mut cur: Option<StoryBlock> = None;
    for l in &lines[i..] {
        let t = trim(l);
        if t.is_empty() {
            if let Some(b) = cur.take() {
                out.blocks.push(b);
            }
            continue;
        }
        match cur.as_mut() {
            None => cur = Some(StoryBlock { prompt: t.to_string(), replies: Vec::new() }),
            Some(b) => b.replies.push(t.to_string()),
        }
    }
    if let Some(b) = cur.take() {
        out.blocks.push(b);
    }
    if out.blocks.is_empty() {
        return Err("no dialogue blocks".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dta_and_pro_golfers() {
        let text = "* comment\r\nname,body,skin,hat,shirt,pants,skills\r\nAnn Example, 1, 2, 3, 4, 5, 0123456789ABC 77\r\n";
        let g = parse_pro_golfers(text).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].name, "Ann Example");
        assert_eq!(g[0].skill, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(g[0].rating, 0); // "ABC 77": atoi of "ABC 77" is 0, as in the C++ original
        assert_eq!(g[0].pants, 5);
    }

    #[test]
    fn story_blocks() {
        let s = parse_story("\nTitle\nhello\n  reply one\n  reply two\n\nsecond\n").unwrap();
        assert_eq!(s.title, "Title");
        assert_eq!(s.blocks.len(), 2);
        assert_eq!(s.blocks[0].replies, vec!["reply one", "reply two"]);
    }

    #[test]
    fn top10_records() {
        let mut d = vec![0u8; 156 * 2];
        d[0..3].copy_from_slice(b"Bob");
        d[128] = 42;
        d[150] = 3;
        let t = parse_top10(&d).unwrap();
        assert_eq!((t[0].player.as_str(), t[0].a, t[0].tier), ("Bob", 42, 3));
    }
}

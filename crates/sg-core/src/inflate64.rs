//! Deflate64 ("enhanced deflate", zip method 9), the port's own decoder. Windows' built-in zipper (Send to > Compressed
//! folder) writes the big files of a folder this way, so a zip of SimGolf made there can hold every sound in it: a
//! browser, iPad's Files app and miniz cannot unpack those entries, and the game played the free set's sounds with the
//! original's art (issue #10). It is deflate (RFC 1951) with a 64 KiB window, length code 285 taking 16 extra bits over a
//! base of 3 (deflate's is a fixed 258), and distance codes 30 and 31 (32769 and 49153, 14 extra bits). Written after
//! zlib's puff.c (canonical Huffman codes decoded a bit at a time); web/opensgolf.js has the same in JavaScript for the
//! browser's unzip. Plain deflate is decoded too (`deflate64` false), for the tests.

const LBASE: [u32; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEXT: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DBASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385,
    24577, 32769, 49153,
];
const DEXT: [u8; 32] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14];

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    buf: u32,
    n: u32,
}

impl Bits<'_> {
    fn need(&mut self, k: u32) -> Result<u32, String> {
        while self.n < k {
            let b = *self.d.get(self.pos).ok_or("Deflate64: the data ends early")?;
            self.pos += 1;
            self.buf |= (b as u32) << self.n;
            self.n += 8;
        }
        let v = if k == 32 { self.buf } else { self.buf & ((1u32 << k) - 1) };
        self.buf = if k == 32 { 0 } else { self.buf >> k };
        self.n -= k;
        Ok(v)
    }
}

/// A canonical Huffman code: how many codes of each length, and the symbols in code order.
struct Huff {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huff {
    fn new(lengths: &[u8]) -> Result<Huff, String> {
        let mut count = [0u16; 16];
        for &l in lengths {
            count[l as usize] += 1;
        }
        let mut offs = [0u16; 16];
        for l in 1..15 {
            offs[l + 1] = offs[l] + count[l];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huff { count, symbol })
    }

    fn decode(&self, b: &mut Bits) -> Result<usize, String> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= b.need(1)? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                return Ok(self.symbol[(index + code - first) as usize] as usize);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("Deflate64: a bad code".into())
    }
}

/// Decodes a raw Deflate64 stream (`deflate64` true) or a plain deflate one. `size` is the expected size (a hint).
pub fn inflate(data: &[u8], size: usize, deflate64: bool) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut b = Bits { d: data, pos: 0, buf: 0, n: 0 };
    let (fixed_l, fixed_d) = {
        let mut l = [8u8; 288];
        l[144..256].fill(9);
        l[256..280].fill(7);
        (Huff::new(&l)?, Huff::new(&[5u8; 32])?)
    };
    loop {
        let last = b.need(1)?;
        match b.need(2)? {
            0 => {
                // stored: from the next byte boundary
                b.buf = 0;
                b.n = 0;
                let p = b.pos;
                let head = data.get(p..p + 4).ok_or("Deflate64: the data ends early")?;
                let len = u16::from_le_bytes([head[0], head[1]]) as usize;
                if len != !u16::from_le_bytes([head[2], head[3]]) as usize {
                    return Err("Deflate64: a bad stored block".into());
                }
                out.extend_from_slice(data.get(p + 4..p + 4 + len).ok_or("Deflate64: the data ends early")?);
                b.pos = p + 4 + len;
            }
            1 => codes(&mut b, &mut out, &fixed_l, &fixed_d, deflate64)?,
            2 => {
                let nlen = b.need(5)? as usize + 257;
                let ndist = b.need(5)? as usize + 1;
                let ncode = b.need(4)? as usize + 4;
                const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut cl = [0u8; 19];
                for &o in &ORDER[..ncode] {
                    cl[o] = b.need(3)? as u8;
                }
                let ch = Huff::new(&cl)?;
                let mut lengths = vec![0u8; nlen + ndist];
                let mut i = 0;
                while i < nlen + ndist {
                    let sym = ch.decode(&mut b)?;
                    let (val, rep) = match sym {
                        0..=15 => (sym as u8, 1),
                        16 => (*lengths.get(i.wrapping_sub(1)).ok_or("Deflate64: a repeat with nothing before it")?, 3 + b.need(2)?),
                        17 => (0, 3 + b.need(3)?),
                        _ => (0, 11 + b.need(7)?),
                    };
                    for _ in 0..rep {
                        *lengths.get_mut(i).ok_or("Deflate64: too many code lengths")? = val;
                        i += 1;
                    }
                }
                let l = Huff::new(&lengths[..nlen])?;
                let d = Huff::new(&lengths[nlen..])?;
                codes(&mut b, &mut out, &l, &d, deflate64)?;
            }
            _ => return Err("Deflate64: a bad block type".into()),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

fn codes(b: &mut Bits, out: &mut Vec<u8>, l: &Huff, d: &Huff, deflate64: bool) -> Result<(), String> {
    loop {
        let sym = l.decode(b)?;
        if sym < 256 {
            out.push(sym as u8);
            continue;
        }
        if sym == 256 {
            return Ok(());
        }
        let s = sym - 257;
        if s >= 29 {
            return Err("Deflate64: a bad length code".into());
        }
        let len = if deflate64 && s == 28 { 3 + b.need(16)? } else { LBASE[s] + b.need(LEXT[s] as u32)? } as usize;
        let ds = d.decode(b)?;
        if ds >= if deflate64 { 32 } else { 30 } {
            return Err("Deflate64: a bad distance code".into());
        }
        let dist = (DBASE[ds] + b.need(DEXT[ds] as u32)?) as usize;
        if dist > out.len() {
            return Err("Deflate64: a distance too far back".into());
        }
        let from = out.len() - dist;
        if dist >= len {
            out.extend_from_within(from..from + len);
        } else {
            for k in 0..len {
                out.push(out[from + k]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_deflate_matches_miniz() {
        let text: Vec<u8> = (0..20000u32).map(|i| (i * 7 % 251) as u8 ^ (i / 300) as u8).chain(b"SimGolf ".repeat(500)).collect();
        for level in [0, 1, 6, 9] {
            let z = miniz_oxide::deflate::compress_to_vec(&text, level);
            assert_eq!(inflate(&z, text.len(), false).unwrap(), text, "level {level}");
        }
    }

    /// The bytes in tests/data/deflate64.zip: 20000 bytes of an LCG, 45000 zeros, the same 20000 bytes again (65000 back:
    /// only Deflate64's distance codes reach them), then 35000 "ab" pairs.
    pub(crate) fn deflate64_sample() -> Vec<u8> {
        let mut x: u32 = 12345;
        let a: Vec<u8> = (0..20000)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                (x >> 24) as u8
            })
            .collect();
        [a.clone(), vec![0; 45000], a, b"ab".repeat(35000)].concat()
    }

    #[test]
    fn deflate64_from_7zip() {
        // made with `7z a -tzip -mm=Deflate64 -mx=9` (7-Zip 23.01)
        let d = include_bytes!("../tests/data/deflate64.zip");
        let z = crate::zip::Zip::open(d.to_vec()).unwrap();
        assert_eq!(z.read("deflate64.bin"), Some(deflate64_sample()));
        // a damaged stream is an error, not a panic
        assert!(inflate(&d[40..200], 155000, true).is_err());
    }
}

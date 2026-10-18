//! The zip compression methods past deflate, the port's own decoders: BZip2 (method 12), LZMA (14) and PPMd var. I rev. 1
//! (98), which 7-Zip, The Unarchiver and Keka write when asked for stronger compression. iPad's Files app, a browser's
//! DecompressionStream and miniz unpack none of them, so a zip of SimGolf made so lost its sounds there and the game
//! played the free set's with the original's art (issue #10). The same decoders as web/files.js (the browser's unzip);
//! each takes an entry's bytes as stored in the zip and its size from the zip's directory, and decodes the whole entry
//! into one buffer. `unpack` picks the decoder for a method (stored, deflate and Deflate64 too).

/// The name of a zip compression method, for messages.
pub fn method_name(method: u16) -> String {
    match method {
        0 => "stored".into(),
        8 => "Deflate".into(),
        9 => "Deflate64".into(),
        12 => "BZip2".into(),
        14 => "LZMA".into(),
        93 => "Zstandard".into(),
        95 => "XZ".into(),
        98 => "PPMd".into(),
        99 => "AES encryption".into(),
        m => format!("compression method {m}"),
    }
}

/// True for the methods `unpack` decodes.
pub fn supported(method: u16) -> bool {
    matches!(method, 0 | 8 | 9 | 12 | 14 | 98)
}

/// An entry's bytes, from its bytes as stored in the zip (`body`) and its size.
pub fn unpack(method: u16, body: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let out = match method {
        0 => body.get(..size).ok_or("stored: the data ends early")?.to_vec(),
        8 => miniz_oxide::inflate::decompress_to_vec_with_limit(body, size + 1024).map_err(|e| format!("deflate: {e:?}"))?,
        9 => crate::inflate64::inflate(body, size, true)?,
        12 => bunzip2(body, size)?,
        14 => unlzma(body, size)?,
        98 => unppmd(body, size)?,
        m => return Err(format!("{} is not supported", method_name(m))),
    };
    if out.len() != size {
        return Err(format!("{}: less data than the zip says", method_name(method)));
    }
    Ok(out)
}

// --- BZip2 -------------------------------------------------------------------------------------------------------------------

/// Bits read the highest first.
struct MsbBits<'a> {
    d: &'a [u8],
    pos: usize,
    buf: u64,
    n: u32,
}

impl MsbBits<'_> {
    fn bits(&mut self, k: u32) -> Result<u32, String> {
        while self.n < k {
            let b = *self.d.get(self.pos).ok_or("BZip2: the data ends early")?;
            self.pos += 1;
            self.buf = (self.buf << 8) | b as u64;
            self.n += 8;
        }
        self.n -= k;
        Ok(((self.buf >> self.n) & ((1u64 << k) - 1)) as u32)
    }
}

/// A canonical Huffman code of up to 20 bits (BZip2's): how many codes of each length, the symbols in code order.
struct Huff20 {
    count: [u16; 21],
    symbol: Vec<u16>,
}

impl Huff20 {
    fn new(lengths: &[u8]) -> Huff20 {
        let mut count = [0u16; 21];
        for &l in lengths {
            count[l as usize] += 1;
        }
        let mut offs = [0usize; 22];
        for l in 1..=20 {
            offs[l + 1] = offs[l] + count[l] as usize;
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            symbol[offs[l as usize]] = s as u16;
            offs[l as usize] += 1;
        }
        Huff20 { count, symbol }
    }

    fn decode(&self, b: &mut MsbBits) -> Result<usize, String> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..=20 {
            code |= b.bits(1)? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                return Ok(self.symbol[(index + code - first) as usize] as usize);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("BZip2: a bad code".into())
    }
}

/// BZip2 (zip method 12): "BZh" and the block size, then blocks of up to 900 000 bytes, each the Burrows-Wheeler
/// transform of the run length coded data, move-to-front coded, its runs of the front symbol as RUNA/RUNB digits and
/// Huffman coded with up to six tables switched every 50 symbols (after the format bzip2 1.0 writes). Streams written one
/// after another (a parallel bzip2's) are read on.
pub fn bunzip2(src: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let fail = |why: &str| Err(format!("BZip2: {why}"));
    let mut out = Vec::with_capacity(size);
    let mut b = MsbBits { d: src, pos: 0, buf: 0, n: 0 };
    while out.len() < size {
        if b.bits(8)? != 0x42 || b.bits(8)? != 0x5a || b.bits(8)? != 0x68 {
            return fail("not a bzip2 stream");
        }
        let level = b.bits(8)?.wrapping_sub(0x30);
        if !(1..=9).contains(&level) {
            return fail("a bad block size");
        }
        let max = level as usize * 100_000;
        let mut tt = vec![0u32; max];
        loop {
            let (m1, m2) = (b.bits(24)?, b.bits(24)?);
            if (m1, m2) == (0x177245, 0x385090) {
                b.bits(32)?; // the stream's checksum (the zip's own CRC is checked)
                b.n -= b.n % 8;
                break;
            }
            if (m1, m2) != (0x314159, 0x265359) {
                return fail("a bad block header");
            }
            b.bits(32)?; // the block's checksum
            if b.bits(1)? != 0 {
                return fail("a randomised block (bzip2 0.9.0's), which is not supported");
            }
            let orig_ptr = b.bits(24)? as usize;
            let used = b.bits(16)?;
            let mut seq = Vec::new();
            for i in 0..16 {
                if used & (0x8000 >> i) != 0 {
                    let m = b.bits(16)?;
                    seq.extend((0..16).filter(|j| m & (0x8000 >> j) != 0).map(|j| (i * 16 + j) as u8));
                }
            }
            if seq.is_empty() {
                return fail("a block with no symbols");
            }
            let alpha = seq.len() + 2;
            let (n_groups, n_sel) = (b.bits(3)? as usize, b.bits(15)? as usize);
            if !(2..=6).contains(&n_groups) || n_sel == 0 {
                return fail("bad tables");
            }
            let mut mtf_g: Vec<u8> = (0..n_groups as u8).collect();
            let mut sel = vec![0u8; n_sel];
            for s in sel.iter_mut() {
                let mut j = 0;
                while b.bits(1)? != 0 {
                    j += 1;
                    if j >= n_groups {
                        return fail("a bad table selector");
                    }
                }
                let v = mtf_g.remove(j);
                mtf_g.insert(0, v);
                *s = v;
            }
            let mut groups = Vec::new();
            for _ in 0..n_groups {
                let mut len = vec![0u8; alpha];
                let mut c = b.bits(5)? as i32;
                for l in len.iter_mut() {
                    loop {
                        if !(1..=20).contains(&c) {
                            return fail("a bad code length");
                        }
                        if b.bits(1)? == 0 {
                            break;
                        }
                        c += if b.bits(1)? != 0 { -1 } else { 1 };
                    }
                    *l = c as u8;
                }
                groups.push(Huff20::new(&len));
            }
            // the symbols: runs of the front byte (RUNA, RUNB as bijective base-2 digits), else a move to the front
            let mut mtf: [u8; 256] = std::array::from_fn(|i| i as u8);
            let mut freq = [0u32; 256];
            let (mut count, mut gi, mut left, mut run, mut run_bit) = (0usize, 0usize, 0u32, 0usize, 1usize);
            let mut h = &groups[0];
            loop {
                if left == 0 {
                    h = &groups[*sel.get(gi).ok_or("BZip2: more symbols than tables")? as usize];
                    gi += 1;
                    left = 50;
                }
                left -= 1;
                let sym = h.decode(&mut b)?;
                if sym <= 1 {
                    run += run_bit << sym;
                    run_bit <<= 1;
                    if run > max {
                        return fail("a block too long");
                    }
                    continue;
                }
                if run > 0 {
                    let v = seq[mtf[0] as usize];
                    if count + run > max {
                        return fail("a block too long");
                    }
                    tt[count..count + run].fill(v as u32);
                    freq[v as usize] += run as u32;
                    count += run;
                    run = 0;
                    run_bit = 1;
                }
                if sym == alpha - 1 {
                    break;
                }
                if count >= max {
                    return fail("a block too long");
                }
                let k = sym - 1;
                let v = mtf[k];
                mtf.copy_within(0..k, 1);
                mtf[0] = v;
                let v = seq[v as usize];
                freq[v as usize] += 1;
                tt[count] = v as u32;
                count += 1;
            }
            if orig_ptr >= count {
                return fail("a bad start of block");
            }
            // the inverse transform: each byte's next one in the low byte's sorted place (bzip2's own tt array)
            let mut cf = [0u32; 256];
            for i in 1..256 {
                cf[i] = cf[i - 1] + freq[i - 1];
            }
            for i in 0..count {
                let v = (tt[i] & 0xff) as usize;
                tt[cf[v] as usize] |= (i as u32) << 8;
                cf[v] += 1;
            }
            let (mut t, mut last, mut same) = ((tt[orig_ptr] >> 8) as usize, -1i32, 0);
            for _ in 0..count {
                let e = tt[t];
                let v = (e & 0xff) as u8;
                t = (e >> 8) as usize;
                if same == 4 {
                    // four of a byte, then how many more
                    if out.len() + v as usize > size {
                        return fail("more data than the zip says");
                    }
                    out.resize(out.len() + v as usize, last as u8);
                    same = 0;
                    last = -1;
                    continue;
                }
                if v as i32 == last {
                    same += 1;
                } else {
                    last = v as i32;
                    same = 1;
                }
                if out.len() >= size {
                    return fail("more data than the zip says");
                }
                out.push(v);
            }
        }
        if out.len() < size && b.pos >= src.len() {
            break;
        }
    }
    if out.len() != size {
        return fail("less data than the zip says");
    }
    Ok(out)
}

// --- LZMA ----------------------------------------------------------------------------------------------------------------------

struct RangeDec<'a> {
    d: &'a [u8],
    pos: usize,
    range: u32,
    code: u32,
}

impl RangeDec<'_> {
    fn next(&mut self) -> u32 {
        let b = self.d.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b as u32
    }
    fn bit(&mut self, p: &mut u16) -> u32 {
        let bound = (self.range >> 11) * *p as u32;
        let b = if self.code < bound {
            self.range = bound;
            *p += (2048 - *p) >> 5;
            0
        } else {
            self.range -= bound;
            self.code -= bound;
            *p -= *p >> 5;
            1
        };
        if self.range < 1 << 24 {
            self.range <<= 8;
            self.code = (self.code << 8) | self.next();
        }
        b
    }
    fn direct(&mut self, k: u32) -> u32 {
        let mut r = 0;
        for _ in 0..k {
            self.range >>= 1;
            let b = if self.code >= self.range {
                self.code -= self.range;
                1
            } else {
                0
            };
            r = (r << 1) | b;
            if self.range < 1 << 24 {
                self.range <<= 8;
                self.code = (self.code << 8) | self.next();
            }
        }
        r
    }
    fn tree(&mut self, p: &mut [u16], k: u32) -> u32 {
        let mut m = 1usize;
        for _ in 0..k {
            m = (m << 1) + self.bit(&mut p[m]) as usize;
        }
        m as u32 - (1 << k)
    }
    fn reverse(&mut self, p: &mut [u16], k: u32) -> u32 {
        let (mut m, mut r) = (1usize, 0);
        for i in 0..k {
            let b = self.bit(&mut p[m]);
            m = (m << 1) + b as usize;
            r |= b << i;
        }
        r
    }
    /// A match length less 2: the choice bits, then 3, 3 or 8 bits.
    fn length(&mut self, p: &mut [u16], ps: usize) -> u32 {
        if self.bit(&mut p[0]) == 0 {
            return self.tree(&mut p[2 + (ps << 3)..], 3);
        }
        if self.bit(&mut p[1]) == 0 {
            return 8 + self.tree(&mut p[130 + (ps << 3)..], 3);
        }
        16 + self.tree(&mut p[258..], 8)
    }
}

/// LZMA (zip method 14): after the zip's own 4 bytes (the LZMA SDK's version, the size of the properties) the 5 bytes of
/// properties (lc, lp, pb; the dictionary size, not needed as the whole entry is decoded into one buffer) and the range
/// coded stream (after the format of the LZMA SDK's LzmaSpec). The zip directory's size says where it ends (an end
/// marker, when there is one, is not read).
pub fn unlzma(src: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let fail = |why: &str| Err(format!("LZMA: {why}"));
    if src.len() < 9 {
        return fail("the data ends early");
    }
    let psize = u16::from_le_bytes([src[2], src[3]]) as usize;
    if psize < 5 || 4 + psize > src.len() || src[4] >= 225 {
        return fail("bad properties");
    }
    let d = src[4] as usize;
    let (lc, lp, pb) = (d % 9, d / 9 % 5, d / 45);
    let start = 4 + psize;
    if src.get(start) != Some(&0) {
        return fail("a bad start of the stream");
    }
    let mut rc = RangeDec { d: src, pos: start + 1, range: 0xffff_ffff, code: 0 };
    for _ in 0..4 {
        rc.code = (rc.code << 8) | rc.next();
    }
    if rc.code == rc.range {
        return fail("a bad start of the stream");
    }
    const IS_MATCH: usize = 0;
    const IS_REP: usize = 192;
    const IS_REP_G0: usize = 204;
    const IS_REP_G1: usize = 216;
    const IS_REP_G2: usize = 228;
    const IS_REP0_LONG: usize = 240;
    const POS_SLOT: usize = 432;
    const SPEC_POS: usize = 688;
    const ALIGN: usize = 803;
    const LEN: usize = 819;
    const REP_LEN: usize = 1333;
    const LIT: usize = 1847;
    let mut p = vec![1024u16; LIT + (0x300 << (lc + lp))];
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let (pb_mask, lp_mask) = ((1 << pb) - 1, (1 << lp) - 1);
    let (mut state, mut rep0, mut rep1, mut rep2, mut rep3) = (0usize, 0usize, 0usize, 0usize, 0usize);
    while out.len() < size {
        let o = out.len();
        let ps = o & pb_mask;
        if rc.bit(&mut p[IS_MATCH + (state << 4) + ps]) == 0 {
            let prev = out.last().copied().unwrap_or(0) as usize;
            let base = LIT + 0x300 * (((o & lp_mask) << lc) + (prev >> (8 - lc)));
            let mut sym = 1usize;
            if state >= 7 {
                if rep0 >= o {
                    return fail("a distance too far back");
                }
                let mut mb = out[o - rep0 - 1] as usize;
                while sym < 0x100 {
                    let m = (mb >> 7) & 1;
                    mb <<= 1;
                    let b = rc.bit(&mut p[base + ((1 + m) << 8) + sym]) as usize;
                    sym = (sym << 1) | b;
                    if m != b {
                        break;
                    }
                }
            }
            while sym < 0x100 {
                sym = (sym << 1) | rc.bit(&mut p[base + sym]) as usize;
            }
            out.push(sym as u8);
            state = if state < 4 {
                0
            } else if state < 10 {
                state - 3
            } else {
                state - 6
            };
            continue;
        }
        let len;
        if rc.bit(&mut p[IS_REP + state]) != 0 {
            if o == 0 {
                return fail("a repeat at the start");
            }
            if rc.bit(&mut p[IS_REP_G0 + state]) == 0 {
                if rc.bit(&mut p[IS_REP0_LONG + (state << 4) + ps]) == 0 {
                    state = if state < 7 { 9 } else { 11 };
                    if rep0 >= o {
                        return fail("a distance too far back");
                    }
                    out.push(out[o - rep0 - 1]);
                    continue;
                }
            } else {
                let dist;
                if rc.bit(&mut p[IS_REP_G1 + state]) == 0 {
                    dist = rep1;
                } else {
                    if rc.bit(&mut p[IS_REP_G2 + state]) == 0 {
                        dist = rep2;
                    } else {
                        dist = rep3;
                        rep3 = rep2;
                    }
                    rep2 = rep1;
                }
                rep1 = rep0;
                rep0 = dist;
            }
            len = rc.length(&mut p[REP_LEN..], ps);
            state = if state < 7 { 8 } else { 11 };
        } else {
            rep3 = rep2;
            rep2 = rep1;
            rep1 = rep0;
            len = rc.length(&mut p[LEN..], ps);
            state = if state < 7 { 7 } else { 10 };
            let slot = rc.tree(&mut p[POS_SLOT + ((len.min(3) as usize) << 6)..], 6);
            if slot < 4 {
                rep0 = slot as usize;
            } else {
                let k = (slot >> 1) - 1;
                let mut dist = ((2 | (slot & 1)) as u64) << k;
                if slot < 14 {
                    dist += rc.reverse(&mut p[SPEC_POS + dist as usize - slot as usize..], k) as u64;
                } else {
                    dist += (rc.direct(k - 4) as u64) << 4;
                    dist += rc.reverse(&mut p[ALIGN..], 4) as u64;
                }
                if dist == 0xffff_ffff {
                    break; // the end marker
                }
                rep0 = dist as usize;
            }
        }
        let len = len as usize + 2;
        if rep0 >= o {
            return fail("a distance too far back");
        }
        if o + len > size {
            return fail("more data than the zip says");
        }
        for k in 0..len {
            out.push(out[o + k - rep0 - 1]);
        }
    }
    if out.len() != size {
        return fail("less data than the zip says");
    }
    Ok(out)
}

// --- PPMd var. I rev. 1 ------------------------------------------------------------------------------------------------------

const UNIT: usize = 12;
const MAX_FREQ: u32 = 124;
const NI: usize = 38;
const EMPTY: u32 = 0xffff_ffff;
const DUMMY: usize = 24 * 32;
const EXP_ESCAPE: [u8; 16] = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];
const INIT_BIN_ESC: [u32; 8] = [0x3cdd, 0x1f3f, 0x59bf, 0x48f3, 0x64a1, 0x5abc, 0x6632, 0x6051];

/// The PPMd var. I model (7-Zip's CPpmd8): its memory is one buffer of 12-byte units, as theirs is, so it decodes the
/// same. A state is 6 bytes (symbol, freq, successor); a context 12 (number of states - 1, flags, sum of freqs, its
/// states, its suffix; one state in place of the last three when it has only one); a free unit 12 (stamp, next, units).
struct Ppmd8 {
    mem: Vec<u8>,
    size: usize,
    align: usize,
    max_order: u32,
    restore: u32,
    i2u: [u8; NI],
    u2i: [u8; 128],
    ns2bs: [u8; 256],
    ns2i: [u8; 260],
    free_list: [u32; NI],
    stamps: [u32; NI],
    see_summ: [u16; DUMMY + 1],
    see_shift: [u8; DUMMY + 1],
    see_count: [u8; DUMMY + 1],
    bin_summ: [u16; 25 * 64],
    min_ctx: usize,
    max_ctx: usize,
    found: usize,
    order_fall: u32,
    init_esc: u32,
    prev_success: u32,
    run_length: i32,
    init_rl: i32,
    glue_count: u32,
    lo_unit: usize,
    hi_unit: usize,
    text: usize,
    units_start: usize,
}

impl Ppmd8 {
    fn u8(&self, a: usize) -> u32 {
        self.mem[a] as u32
    }
    fn w8(&mut self, a: usize, v: u32) {
        self.mem[a] = v as u8;
    }
    fn u16(&self, a: usize) -> u32 {
        u16::from_le_bytes([self.mem[a], self.mem[a + 1]]) as u32
    }
    fn w16(&mut self, a: usize, v: u32) {
        self.mem[a..a + 2].copy_from_slice(&(v as u16).to_le_bytes());
    }
    fn u32(&self, a: usize) -> u32 {
        u32::from_le_bytes(self.mem[a..a + 4].try_into().unwrap())
    }
    fn w32(&mut self, a: usize, v: u32) {
        self.mem[a..a + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn freq(&self, s: usize) -> u32 {
        self.u8(s + 1)
    }
    fn succ(&self, s: usize) -> usize {
        self.u32(s + 2) as usize
    }
    fn set_succ(&mut self, s: usize, v: usize) {
        self.w32(s + 2, v as u32)
    }
    fn ns(&self, c: usize) -> u32 {
        self.u8(c)
    }
    fn flags(&self, c: usize) -> u32 {
        self.u8(c + 1)
    }
    fn summ(&self, c: usize) -> u32 {
        self.u16(c + 2)
    }
    fn stats(&self, c: usize) -> usize {
        self.u32(c + 4) as usize
    }
    fn suffix(&self, c: usize) -> usize {
        self.u32(c + 8) as usize
    }
    fn copy_state(&mut self, to: usize, from: usize) {
        self.mem.copy_within(from..from + 6, to);
    }
    fn swap(&mut self, a: usize, b: usize) {
        for k in 0..6 {
            self.mem.swap(a + k, b + k);
        }
    }
    fn copy12(&mut self, to: usize, from: usize, n: usize) {
        self.mem.copy_within(from..from + n * UNIT, to);
    }
    fn u2i(&self, nu: usize) -> usize {
        self.u2i[nu - 1] as usize
    }
    fn i2u(&self, i: usize) -> usize {
        self.i2u[i] as usize
    }
    /// The state of `sym` in context `c`.
    fn find(&self, c: usize, sym: u32) -> Result<usize, String> {
        let mut s = self.stats(c);
        while self.u8(s) != sym {
            s += 6;
            if s + 6 > self.mem.len() {
                return Err("PPMd: damaged data".into());
            }
        }
        Ok(s)
    }

    fn insert_node(&mut self, node: usize, indx: usize) {
        self.w32(node, EMPTY);
        self.w32(node + 4, self.free_list[indx]);
        self.w32(node + 8, self.i2u[indx] as u32);
        self.free_list[indx] = node as u32;
        self.stamps[indx] = self.stamps[indx].wrapping_add(1);
    }
    fn remove_node(&mut self, indx: usize) -> usize {
        let node = self.free_list[indx] as usize;
        self.free_list[indx] = self.u32(node + 4);
        self.stamps[indx] = self.stamps[indx].wrapping_sub(1);
        node
    }
    fn split_block(&mut self, ptr: usize, old: usize, new: usize) {
        let nu = self.i2u(old) - self.i2u(new);
        let ptr = ptr + self.i2u(new) * UNIT;
        let mut i = self.u2i(nu);
        if self.i2u(i) != nu {
            i -= 1;
            let k = self.i2u(i);
            self.insert_node(ptr + k * UNIT, nu - k - 1);
        }
        self.insert_node(ptr, i);
    }
    fn glue_free_blocks(&mut self) {
        let mut head = 0usize;
        let mut prev: Option<usize> = None;
        self.glue_count = 1 << 13;
        self.stamps = [0; NI];
        if self.lo_unit != self.hi_unit {
            self.w32(self.lo_unit, 0);
        }
        for i in 0..NI {
            let mut next = self.free_list[i] as usize;
            self.free_list[i] = 0;
            while next != 0 {
                let node = next;
                if self.u32(node + 8) != 0 {
                    match prev {
                        None => head = next,
                        Some(a) => self.w32(a, next as u32),
                    }
                    prev = Some(node + 4);
                    loop {
                        let node2 = node + self.u32(node + 8) as usize * UNIT;
                        if self.u32(node2) != EMPTY {
                            break;
                        }
                        let n = self.u32(node + 8).wrapping_add(self.u32(node2 + 8));
                        self.w32(node + 8, n);
                        self.w32(node2 + 8, 0);
                    }
                }
                next = self.u32(node + 4) as usize;
            }
        }
        match prev {
            None => head = 0,
            Some(a) => self.w32(a, 0),
        }
        while head != 0 {
            let mut node = head;
            head = self.u32(node + 4) as usize;
            let mut nu = self.u32(node + 8) as usize;
            if nu == 0 {
                continue;
            }
            while nu > 128 {
                self.insert_node(node, NI - 1);
                nu -= 128;
                node += 128 * UNIT;
            }
            let mut i = self.u2i(nu);
            if self.i2u(i) != nu {
                i -= 1;
                let k = self.i2u(i);
                self.insert_node(node + k * UNIT, nu - k - 1);
            }
            self.insert_node(node, i);
        }
    }
    fn alloc_units_rare(&mut self, indx: usize) -> usize {
        if self.glue_count == 0 {
            self.glue_free_blocks();
            if self.free_list[indx] != 0 {
                return self.remove_node(indx);
            }
        }
        let mut i = indx;
        loop {
            i += 1;
            if i == NI {
                let bytes = self.i2u(indx) * UNIT;
                self.glue_count = self.glue_count.wrapping_sub(1);
                return if self.units_start - self.text > bytes {
                    self.units_start -= bytes;
                    self.units_start
                } else {
                    0
                };
            }
            if self.free_list[i] != 0 {
                break;
            }
        }
        let r = self.remove_node(i);
        self.split_block(r, i, indx);
        r
    }
    fn alloc_units(&mut self, indx: usize) -> usize {
        if self.free_list[indx] != 0 {
            return self.remove_node(indx);
        }
        let bytes = self.i2u(indx) * UNIT;
        if bytes <= self.hi_unit - self.lo_unit {
            let r = self.lo_unit;
            self.lo_unit += bytes;
            return r;
        }
        self.alloc_units_rare(indx)
    }
    fn shrink_units(&mut self, old: usize, old_nu: usize, new_nu: usize) -> usize {
        let (i0, i1) = (self.u2i(old_nu), self.u2i(new_nu));
        if i0 == i1 {
            return old;
        }
        if self.free_list[i1] != 0 {
            let ptr = self.remove_node(i1);
            self.copy12(ptr, old, new_nu);
            self.insert_node(old, i0);
            return ptr;
        }
        self.split_block(old, i0, i1);
        old
    }
    fn free_units(&mut self, ptr: usize, nu: usize) {
        let i = self.u2i(nu);
        self.insert_node(ptr, i);
    }
    fn special_free_unit(&mut self, ptr: usize) {
        if ptr != self.units_start {
            self.insert_node(ptr, 0);
        } else {
            self.units_start += UNIT;
        }
    }
    fn move_units_up(&mut self, old: usize, nu: usize) -> usize {
        let indx = self.u2i(nu);
        if old > self.units_start + 16 * 1024 || old > self.free_list[indx] as usize {
            return old;
        }
        let ptr = self.remove_node(indx);
        self.copy12(ptr, old, nu);
        if old != self.units_start {
            self.insert_node(old, indx);
        } else {
            self.units_start += self.i2u(indx) * UNIT;
        }
        ptr
    }
    fn expand_text_area(&mut self) {
        let mut count = [0u32; NI];
        if self.lo_unit != self.hi_unit {
            self.w32(self.lo_unit, 0);
        }
        let mut node = self.units_start;
        while self.u32(node) == EMPTY {
            self.w32(node, 0);
            let nu = self.u32(node + 8) as usize;
            count[self.u2i(nu)] += 1;
            node += nu * UNIT;
        }
        self.units_start = node;
        for i in 0..NI {
            let mut at: Option<usize> = None;
            while count[i] != 0 {
                let get = |m: &Self, at: Option<usize>| match at {
                    None => m.free_list[i] as usize,
                    Some(a) => m.u32(a) as usize,
                };
                let mut n = get(self, at);
                while self.u32(n) == 0 {
                    let nx = self.u32(n + 4);
                    match at {
                        None => self.free_list[i] = nx,
                        Some(a) => self.w32(a, nx),
                    }
                    n = get(self, at);
                    self.stamps[i] = self.stamps[i].wrapping_sub(1);
                    count[i] -= 1;
                    if count[i] == 0 {
                        break;
                    }
                }
                at = Some(n + 4);
            }
        }
    }
    fn used_memory(&self) -> u32 {
        let v: u32 = (0..NI).map(|i| self.stamps[i].wrapping_mul(self.i2u[i] as u32)).fold(0, u32::wrapping_add);
        (self.size as u32)
            .wrapping_sub((self.hi_unit - self.lo_unit) as u32)
            .wrapping_sub((self.units_start - self.text) as u32)
            .wrapping_sub(v.wrapping_mul(UNIT as u32))
    }

    fn restart_model(&mut self) {
        self.free_list = [0; NI];
        self.stamps = [0; NI];
        self.text = self.align;
        self.hi_unit = self.text + self.size;
        self.lo_unit = self.hi_unit - self.size / 8 / UNIT * 7 * UNIT;
        self.units_start = self.lo_unit;
        self.glue_count = 0;
        self.order_fall = self.max_order;
        self.init_rl = -(self.max_order.min(12) as i32) - 1;
        self.run_length = self.init_rl;
        self.prev_success = 0;
        self.hi_unit -= UNIT;
        self.min_ctx = self.hi_unit;
        self.max_ctx = self.hi_unit;
        let c = self.min_ctx;
        self.w32(c + 8, 0);
        self.w8(c, 255);
        self.w8(c + 1, 0);
        self.w16(c + 2, 257);
        self.found = self.lo_unit;
        self.lo_unit += 128 * UNIT;
        self.w32(c + 4, self.found as u32);
        for i in 0..256 {
            let s = self.found + i * 6;
            self.w8(s, i as u32);
            self.w8(s + 1, 1);
            self.set_succ(s, 0);
        }
        let mut i = 0;
        for m in 0..25 {
            while self.ns2i[i] as usize == m {
                i += 1;
            }
            for k in 0..8 {
                let val = (16384 - INIT_BIN_ESC[k] / (i as u32 + 1)) as u16;
                for r in (0..64).step_by(8) {
                    self.bin_summ[m * 64 + k + r] = val;
                }
            }
        }
        let mut i = 0;
        for m in 0..24 {
            while self.ns2i[i + 3] as usize == m + 3 {
                i += 1;
            }
            for k in 0..32 {
                self.see_shift[m * 32 + k] = 3;
                self.see_summ[m * 32 + k] = ((2 * i + 5) << 3) as u16;
                self.see_count[m * 32 + k] = 7;
            }
        }
    }
    fn refresh(&mut self, ctx: usize, old_nu: usize, scale: u32) {
        let mut i = self.ns(ctx);
        let mut s = self.shrink_units(self.stats(ctx), old_nu, (i as usize + 2) >> 1);
        self.w32(ctx + 4, s as u32);
        let mut fl = (self.flags(ctx) & (0x10 + 0x04 * scale)) + 0x08 * (self.u8(s) >= 0x40) as u32;
        let mut esc = self.summ(ctx).wrapping_sub(self.freq(s));
        let mut f = (self.freq(s) + scale) >> scale;
        self.w8(s + 1, f);
        let mut sum = f;
        loop {
            s += 6;
            esc = esc.wrapping_sub(self.freq(s));
            f = (self.freq(s) + scale) >> scale;
            self.w8(s + 1, f);
            sum += f;
            fl |= 0x08 * (self.u8(s) >= 0x40) as u32;
            i -= 1;
            if i == 0 {
                break;
            }
        }
        self.w16(ctx + 2, sum.wrapping_add(esc.wrapping_add(scale) >> scale));
        self.w8(ctx + 1, fl);
    }
    fn cut_off(&mut self, ctx: usize, order: u32) -> usize {
        if self.ns(ctx) == 0 {
            let s = ctx + 2;
            if self.succ(s) >= self.units_start {
                let v = if order < self.max_order { self.cut_off(self.succ(s), order + 1) } else { 0 };
                self.set_succ(s, v);
                if self.succ(s) != 0 || order <= 9 {
                    return ctx;
                }
            }
            self.special_free_unit(ctx);
            return 0;
        }
        let tmp = (self.ns(ctx) as usize + 2) >> 1;
        let moved = self.move_units_up(self.stats(ctx), tmp);
        self.w32(ctx + 4, moved as u32);
        let mut i = self.ns(ctx) as i32;
        let st = self.stats(ctx);
        let mut s = st + i as usize * 6;
        loop {
            if self.succ(s) < self.units_start {
                let s2 = st + i as usize * 6;
                i -= 1;
                self.set_succ(s, 0);
                self.swap(s, s2);
            } else {
                let v = if order < self.max_order { self.cut_off(self.succ(s), order + 1) } else { 0 };
                self.set_succ(s, v);
            }
            if s == st {
                break;
            }
            s -= 6;
        }
        if i != self.ns(ctx) as i32 && order != 0 {
            self.w8(ctx, i as u32);
            let s = self.stats(ctx);
            if i < 0 {
                self.free_units(s, tmp);
                self.special_free_unit(ctx);
                return 0;
            }
            if i == 0 {
                let fl = (self.flags(ctx) & 0x10) + 0x08 * (self.u8(s) >= 0x40) as u32;
                self.w8(ctx + 1, fl);
                self.copy_state(ctx + 2, s);
                self.free_units(s, tmp);
                let f = (self.freq(ctx + 2) + 11) >> 3;
                self.w8(ctx + 3, f);
            } else {
                let scale = (self.summ(ctx) > 16 * i as u32) as u32;
                self.refresh(ctx, tmp, scale);
            }
        }
        ctx
    }
    fn restore_model(&mut self, c1: usize) {
        self.text = self.align;
        let mut c = self.max_ctx;
        while c != c1 {
            let n = self.ns(c).wrapping_sub(1) & 255;
            self.w8(c, n);
            if n == 0 {
                let s = self.stats(c);
                let fl = (self.flags(c) & 0x10) + 0x08 * (self.u8(s) >= 0x40) as u32;
                self.w8(c + 1, fl);
                self.copy_state(c + 2, s);
                self.special_free_unit(s);
                let f = (self.freq(c + 2) + 11) >> 3;
                self.w8(c + 3, f);
            } else {
                self.refresh(c, (n as usize + 3) >> 1, 0);
            }
            c = self.suffix(c);
        }
        while c != self.min_ctx {
            if self.ns(c) == 0 {
                let f = self.freq(c + 2);
                self.w8(c + 3, f - (f >> 1));
            } else {
                self.w16(c + 2, self.summ(c) + 4);
                if self.summ(c) > 128 + 4 * self.ns(c) {
                    self.refresh(c, (self.ns(c) as usize + 2) >> 1, 1);
                }
            }
            c = self.suffix(c);
        }
        if self.restore == 0 || self.used_memory() < (self.size as u32 >> 1) {
            self.restart_model();
        } else {
            while self.suffix(self.max_ctx) != 0 {
                self.max_ctx = self.suffix(self.max_ctx);
            }
            loop {
                self.cut_off(self.max_ctx, 0);
                self.expand_text_area();
                if self.used_memory() <= 3 * (self.size as u32 >> 2) {
                    break;
                }
            }
            self.glue_count = 0;
            self.order_fall = self.max_order;
        }
    }
    fn create_successors(&mut self, skip: bool, mut s1: usize, mut c: usize) -> Result<usize, String> {
        let up_branch = self.succ(self.found);
        let mut ps: Vec<usize> = Vec::with_capacity(17);
        if !skip {
            ps.push(self.found);
        }
        while self.suffix(c) != 0 {
            c = self.suffix(c);
            let s;
            if s1 != 0 {
                s = s1;
                s1 = 0;
            } else if self.ns(c) != 0 {
                s = self.find(c, self.u8(self.found))?;
                if self.freq(s) < MAX_FREQ - 9 {
                    self.w8(s + 1, self.freq(s) + 1);
                    self.w16(c + 2, self.summ(c) + 1);
                }
            } else {
                s = c + 2;
                let add = (self.ns(self.suffix(c)) == 0) as u32 & (self.freq(s) < 24) as u32;
                self.w8(s + 1, self.freq(s) + add);
            }
            let successor = self.succ(s);
            if successor != up_branch {
                c = successor;
                if ps.is_empty() {
                    return Ok(c);
                }
                break;
            }
            ps.push(s);
        }
        let up_sym = self.u8(up_branch);
        let up_succ = up_branch + 1;
        let fl = 0x10 * (self.u8(self.found) >= 0x40) as u32 + 0x08 * (up_sym >= 0x40) as u32;
        let up_freq = if self.ns(c) == 0 {
            self.freq(c + 2)
        } else {
            let s = self.find(c, up_sym)?;
            let cf = self.freq(s).wrapping_sub(1);
            let s0 = self.summ(c).wrapping_sub(self.ns(c)).wrapping_sub(cf);
            1 + if 2 * cf <= s0 { (5 * cf > s0) as u32 } else { cf.wrapping_add(2 * s0).wrapping_sub(3) / s0 }
        };
        while let Some(s) = ps.pop() {
            let c1 = if self.hi_unit != self.lo_unit {
                self.hi_unit -= UNIT;
                self.hi_unit
            } else if self.free_list[0] != 0 {
                self.remove_node(0)
            } else {
                let c1 = self.alloc_units_rare(0);
                if c1 == 0 {
                    return Ok(0);
                }
                c1
            };
            self.w8(c1, 0);
            self.w8(c1 + 1, fl);
            self.w8(c1 + 2, up_sym);
            self.w8(c1 + 3, up_freq);
            self.w32(c1 + 4, up_succ as u32);
            self.w32(c1 + 8, c as u32);
            self.set_succ(s, c1);
            c = c1;
        }
        Ok(c)
    }
    fn reduce_order(&mut self, mut s1: usize, mut c: usize) -> Result<usize, String> {
        let mut s;
        let c1 = c;
        let up_branch = self.text;
        let sym = self.u8(self.found);
        self.set_succ(self.found, up_branch);
        self.order_fall += 1;
        loop {
            if s1 != 0 {
                c = self.suffix(c);
                s = s1;
                s1 = 0;
            } else {
                if self.suffix(c) == 0 {
                    return Ok(c);
                }
                c = self.suffix(c);
                if self.ns(c) != 0 {
                    s = self.find(c, sym)?;
                    if self.freq(s) < MAX_FREQ - 9 {
                        self.w8(s + 1, self.freq(s) + 2);
                        self.w16(c + 2, self.summ(c) + 2);
                    }
                } else {
                    s = c + 2;
                    self.w8(s + 1, self.freq(s) + (self.freq(s) < 32) as u32);
                }
            }
            if self.succ(s) != 0 {
                break;
            }
            self.set_succ(s, up_branch);
            self.order_fall += 1;
        }
        if self.succ(s) <= up_branch {
            let s2 = self.found;
            self.found = s;
            let v = self.create_successors(false, 0, c)?;
            self.set_succ(s, v);
            self.found = s2;
        }
        if self.order_fall == 1 && c1 == self.max_ctx {
            self.set_succ(self.found, self.succ(s));
            self.text -= 1;
        }
        Ok(self.succ(s))
    }
    fn update_model(&mut self) -> Result<(), String> {
        let mut f_successor = self.succ(self.found);
        let (f_freq, f_symbol) = (self.freq(self.found), self.u8(self.found));
        let mut s = 0usize;
        if f_freq < MAX_FREQ / 4 && self.suffix(self.min_ctx) != 0 {
            let c = self.suffix(self.min_ctx);
            if self.ns(c) == 0 {
                s = c + 2;
                if self.freq(s) < 32 {
                    self.w8(s + 1, self.freq(s) + 1);
                }
            } else {
                s = self.find(c, f_symbol)?;
                if s != self.stats(c) && self.freq(s) >= self.freq(s - 6) {
                    self.swap(s, s - 6);
                    s -= 6;
                }
                if self.freq(s) < MAX_FREQ - 9 {
                    self.w8(s + 1, self.freq(s) + 2);
                    self.w16(c + 2, self.summ(c) + 2);
                }
            }
        }
        let mut c = self.max_ctx;
        if self.order_fall == 0 && f_successor != 0 {
            let cs = self.create_successors(true, s, self.min_ctx)?;
            if cs == 0 {
                self.set_succ(self.found, 0);
                self.restore_model(c);
            } else {
                self.set_succ(self.found, cs);
                self.max_ctx = cs;
            }
            return Ok(());
        }
        self.w8(self.text, f_symbol);
        self.text += 1;
        let mut successor = self.text;
        if self.text >= self.units_start {
            self.restore_model(c);
            return Ok(());
        }
        if f_successor == 0 {
            let cs = self.reduce_order(s, self.min_ctx)?;
            if cs == 0 {
                self.restore_model(c);
                return Ok(());
            }
            f_successor = cs;
        } else if f_successor < self.units_start {
            let cs = self.create_successors(false, s, self.min_ctx)?;
            if cs == 0 {
                self.restore_model(c);
                return Ok(());
            }
            f_successor = cs;
        }
        self.order_fall -= 1;
        if self.order_fall == 0 {
            successor = f_successor;
            if self.max_ctx != self.min_ctx {
                self.text -= 1;
            }
        }
        let ns0 = self.ns(self.min_ctx);
        let s0 = self.summ(self.min_ctx).wrapping_sub(ns0).wrapping_sub(f_freq);
        let flag = 0x08 * (f_symbol >= 0x40) as u32;
        while c != self.min_ctx {
            let ns1 = self.ns(c);
            if ns1 != 0 {
                if ns1 & 1 != 0 {
                    let old_nu = (ns1 as usize + 1) >> 1;
                    let i = self.u2i(old_nu);
                    if i != self.u2i(old_nu + 1) {
                        let ptr = self.alloc_units(i + 1);
                        if ptr == 0 {
                            self.restore_model(c);
                            return Ok(());
                        }
                        let old = self.stats(c);
                        self.copy12(ptr, old, old_nu);
                        self.insert_node(old, i);
                        self.w32(c + 4, ptr as u32);
                    }
                }
                self.w16(c + 2, self.summ(c) + (3 * ns1 + 1 < ns0) as u32);
            } else {
                let s2 = self.alloc_units(0);
                if s2 == 0 {
                    self.restore_model(c);
                    return Ok(());
                }
                self.copy_state(s2, c + 2);
                self.w32(c + 4, s2 as u32);
                let f = self.freq(s2);
                self.w8(s2 + 1, if f < MAX_FREQ / 4 - 1 { f << 1 } else { MAX_FREQ - 4 });
                self.w16(c + 2, self.freq(s2) + self.init_esc + (ns0 > 2) as u32);
            }
            let mut cf = 2 * f_freq * (self.summ(c) + 6);
            let sf = s0.wrapping_add(self.summ(c));
            if cf < 6 * sf {
                cf = 1 + (cf > sf) as u32 + (cf >= 4 * sf) as u32;
                self.w16(c + 2, self.summ(c) + 4);
            } else {
                cf = 4 + (cf > 9 * sf) as u32 + (cf > 12 * sf) as u32 + (cf > 15 * sf) as u32;
                self.w16(c + 2, self.summ(c) + cf);
            }
            let s2 = self.stats(c) + (ns1 as usize + 1) * 6;
            self.set_succ(s2, successor);
            self.w8(s2, f_symbol);
            self.w8(s2 + 1, cf);
            self.w8(c + 1, self.flags(c) | flag);
            self.w8(c, ns1 + 1);
            c = self.suffix(c);
        }
        self.max_ctx = f_successor;
        self.min_ctx = f_successor;
        Ok(())
    }
    fn rescale(&mut self) {
        let mc = self.min_ctx;
        let st = self.stats(mc);
        let mut s = self.found;
        let tmp: [u8; 6] = self.mem[s..s + 6].try_into().unwrap();
        self.mem.copy_within(st..s, st + 6);
        self.mem[st..st + 6].copy_from_slice(&tmp);
        s = st;
        let mut esc = self.summ(mc).wrapping_sub(self.freq(s));
        let adder = (self.order_fall != 0) as u32;
        self.w8(s + 1, (((self.freq(s) + 4) & 255) + adder) >> 1);
        let mut sum = self.freq(s);
        let mut i = self.ns(mc);
        loop {
            s += 6;
            esc = esc.wrapping_sub(self.freq(s));
            self.w8(s + 1, (self.freq(s) + adder) >> 1);
            sum += self.freq(s);
            if self.freq(s) > self.freq(s - 6) {
                let mut s1 = s;
                let tmp: [u8; 6] = self.mem[s1..s1 + 6].try_into().unwrap();
                loop {
                    self.copy_state(s1, s1 - 6);
                    s1 -= 6;
                    if s1 == st || tmp[1] as u32 <= self.freq(s1 - 6) {
                        break;
                    }
                }
                self.mem[s1..s1 + 6].copy_from_slice(&tmp);
            }
            i -= 1;
            if i == 0 {
                break;
            }
        }
        if self.freq(s) == 0 {
            let num_stats = self.ns(mc);
            loop {
                i += 1;
                s -= 6;
                if self.freq(s) != 0 {
                    break;
                }
            }
            esc = esc.wrapping_add(i);
            self.w8(mc, num_stats - i);
            if self.ns(mc) == 0 {
                let mut tmp: [u8; 6] = self.mem[st..st + 6].try_into().unwrap();
                tmp[1] = (2 * tmp[1] as u32).div_ceil(esc).min(MAX_FREQ / 3) as u8;
                let n = (num_stats as usize + 2) >> 1;
                self.free_units(st, n);
                let fl = (self.flags(mc) & 0x10) + 0x08 * (tmp[0] >= 0x40) as u32;
                self.w8(mc + 1, fl);
                self.found = mc + 2;
                self.mem[mc + 2..mc + 8].copy_from_slice(&tmp);
                return;
            }
            let (n0, n1) = ((num_stats as usize + 2) >> 1, (self.ns(mc) as usize + 2) >> 1);
            if n0 != n1 {
                let p = self.shrink_units(st, n0, n1);
                self.w32(mc + 4, p as u32);
            }
            self.w8(mc + 1, self.flags(mc) & !0x08);
            let mut s = self.stats(mc);
            self.w8(mc + 1, self.flags(mc) | (0x08 * (self.u8(s) >= 0x40) as u32));
            let mut i = self.ns(mc);
            loop {
                s += 6;
                self.w8(mc + 1, self.flags(mc) | (0x08 * (self.u8(s) >= 0x40) as u32));
                i -= 1;
                if i == 0 {
                    break;
                }
            }
        }
        self.w16(mc + 2, sum + esc - (esc >> 1));
        self.w8(mc + 1, self.flags(mc) | 4);
        self.found = self.stats(mc);
    }
    fn next_context(&mut self) -> Result<(), String> {
        let c = self.succ(self.found);
        if self.order_fall == 0 && c >= self.units_start {
            self.min_ctx = c;
            self.max_ctx = c;
        } else {
            self.update_model()?;
            self.min_ctx = self.max_ctx;
        }
        Ok(())
    }
}

/// Subbotin's carryless range decoder (PPMd var. I's).
struct Carryless<'a> {
    d: &'a [u8],
    pos: usize,
    low: u32,
    range: u32,
    code: u32,
}

impl Carryless<'_> {
    fn next(&mut self) -> u32 {
        let b = self.d.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b as u32
    }
    fn threshold(&mut self, total: u32) -> u32 {
        self.range /= total;
        self.code / self.range
    }
    fn decode(&mut self, start: u32, size: u32) {
        let start = start.wrapping_mul(self.range);
        self.low = self.low.wrapping_add(start);
        self.code = self.code.wrapping_sub(start);
        self.range = self.range.wrapping_mul(size);
        loop {
            if (self.low ^ self.low.wrapping_add(self.range)) >= 1 << 24 {
                if self.range >= 1 << 15 {
                    break;
                }
                self.range = self.low.wrapping_neg() & ((1 << 15) - 1);
            }
            self.code = (self.code << 8) | self.next();
            self.range <<= 8;
            self.low <<= 8;
        }
    }
}

/// PPMd var. I rev. 1 (zip method 98): a context model of up to 16 bytes before each byte, coded with Subbotin's carryless
/// range coder. The zip's two bytes first: the model's order - 1 (4 bits), its memory in MB - 1 (8 bits) and what is done
/// when the memory is full (4 bits: start again, or cut the model down; the third, "freeze", is not supported, as in
/// 7-Zip). A port of 7-Zip's Ppmd8.c and Ppmd8Dec.c (Igor Pavlov, after Dmitry Shkarin's PPMd var.I; both public domain).
pub fn unppmd(src: &[u8], size: usize) -> Result<Vec<u8>, String> {
    if src.len() < 2 {
        return Err("PPMd: the data ends early".into());
    }
    let head = u16::from_le_bytes([src[0], src[1]]) as u32;
    let (max_order, mem_size, restore) = ((head & 15) + 1, (((head >> 4) & 0xff) as usize + 1) << 20, head >> 12);
    if max_order < 2 || restore > 1 {
        return Err(format!("PPMd: unsupported settings (order {max_order}, method {restore})"));
    }
    let align = 4 - (mem_size & 3);
    let mut i2u = [0u8; NI];
    let mut u2i = [0u8; 128];
    let mut k = 0;
    for (i, u) in i2u.iter_mut().enumerate() {
        let step = if i >= 12 { 4 } else { (i >> 2) + 1 };
        for _ in 0..step {
            u2i[k] = i as u8;
            k += 1;
        }
        *u = k as u8;
    }
    let mut ns2bs = [6u8; 256];
    ns2bs[0] = 0;
    ns2bs[1] = 2;
    ns2bs[2..11].fill(4);
    let mut ns2i = [0u8; 260];
    let (mut m, mut k) = (5u8, 1);
    for (i, v) in ns2i.iter_mut().enumerate() {
        if i < 5 {
            *v = i as u8;
            continue;
        }
        *v = m;
        k -= 1;
        if k == 0 {
            m += 1;
            k = m as i32 - 4;
        }
    }
    let mut p = Ppmd8 {
        mem: vec![0; align + mem_size],
        size: mem_size,
        align,
        max_order,
        restore,
        i2u,
        u2i,
        ns2bs,
        ns2i,
        free_list: [0; NI],
        stamps: [0; NI],
        see_summ: [0; DUMMY + 1],
        see_shift: [0; DUMMY + 1],
        see_count: [0; DUMMY + 1],
        bin_summ: [0; 25 * 64],
        min_ctx: 0,
        max_ctx: 0,
        found: 0,
        order_fall: 0,
        init_esc: 0,
        prev_success: 0,
        run_length: 0,
        init_rl: 0,
        glue_count: 0,
        lo_unit: 0,
        hi_unit: 0,
        text: 0,
        units_start: 0,
    };
    let mut rc = Carryless { d: src, pos: 2, low: 0, range: 0xffff_ffff, code: 0 };
    for _ in 0..4 {
        rc.code = (rc.code << 8) | rc.next();
    }
    if rc.code == 0xffff_ffff {
        return Err("PPMd: a bad start of the stream".into());
    }
    p.restart_model();
    p.see_shift[DUMMY] = 7;
    p.see_count[DUMMY] = 64;
    let mut out = Vec::with_capacity(size);
    let mut mask = [0u8; 256];
    let mut ps = [0usize; 256];
    while out.len() < size {
        let sym = ppmd_symbol(&mut p, &mut rc, &mut mask, &mut ps)?;
        match sym {
            Some(v) => out.push(v),
            None => return Err("PPMd: the data ends early".into()),
        }
    }
    Ok(out)
}

/// The next byte, or None at an end marker (Ppmd8_DecodeSymbol).
fn ppmd_symbol(p: &mut Ppmd8, rc: &mut Carryless, mask: &mut [u8; 256], ps: &mut [usize; 256]) -> Result<Option<u8>, String> {
    let damaged = || Err("PPMd: damaged data".to_string());
    if p.ns(p.min_ctx) != 0 {
        let mc = p.min_ctx;
        let mut s = p.stats(mc);
        let count = rc.threshold(p.summ(mc));
        let mut hi = p.freq(s);
        if count < hi {
            rc.decode(0, hi);
            p.found = s;
            let sym = p.u8(s) as u8;
            // Update1_0
            p.prev_success = (2 * p.freq(s) >= p.summ(mc)) as u32;
            p.run_length += p.prev_success as i32;
            p.w16(mc + 2, p.summ(mc) + 4);
            p.w8(s + 1, p.freq(s) + 4);
            if p.freq(s) > MAX_FREQ {
                p.rescale();
            }
            p.next_context()?;
            return Ok(Some(sym));
        }
        p.prev_success = 0;
        let mut i = p.ns(mc);
        loop {
            s += 6;
            hi += p.freq(s);
            if hi > count {
                rc.decode(hi - p.freq(s), p.freq(s));
                p.found = s;
                let sym = p.u8(s) as u8;
                // Update1
                p.w8(s + 1, p.freq(s) + 4);
                p.w16(mc + 2, p.summ(mc) + 4);
                if p.freq(s) > p.freq(s - 6) {
                    p.swap(s, s - 6);
                    p.found = s - 6;
                    if p.freq(s - 6) > MAX_FREQ {
                        p.rescale();
                    }
                }
                p.next_context()?;
                return Ok(Some(sym));
            }
            i -= 1;
            if i == 0 {
                break;
            }
        }
        if count >= p.summ(mc) {
            return damaged();
        }
        rc.decode(hi, p.summ(mc) - hi);
        mask.fill(0xff);
        mask[p.u8(s) as usize] = 0;
        let mut i = p.ns(mc);
        loop {
            s -= 6;
            mask[p.u8(s) as usize] = 0;
            i -= 1;
            if i == 0 {
                break;
            }
        }
    } else {
        let mc = p.min_ctx;
        let one = mc + 2;
        let bi = p.ns2i[p.freq(one) as usize - 1] as usize * 64
            + p.ns2bs[p.ns(p.suffix(mc)) as usize] as usize
            + p.prev_success as usize
            + p.flags(mc) as usize
            + ((p.run_length >> 26) & 0x20) as usize;
        let prob = p.bin_summ[bi] as u32;
        rc.range >>= 14;
        if rc.code / rc.range < prob {
            rc.decode(0, prob);
            p.bin_summ[bi] = (prob + 128 - ((prob + 32) >> 7)) as u16;
            p.found = one;
            let sym = p.u8(one) as u8;
            // UpdateBin
            p.w8(one + 1, p.freq(one) + (p.freq(one) < 196) as u32);
            p.prev_success = 1;
            p.run_length += 1;
            p.next_context()?;
            return Ok(Some(sym));
        }
        rc.decode(prob, 16384 - prob);
        p.bin_summ[bi] = (prob - ((prob + 32) >> 7)) as u16;
        p.init_esc = EXP_ESCAPE[(p.bin_summ[bi] >> 10) as usize] as u32;
        mask.fill(0xff);
        mask[p.u8(one) as usize] = 0;
        p.prev_success = 0;
    }
    loop {
        let num_masked = p.ns(p.min_ctx);
        loop {
            p.order_fall += 1;
            if p.suffix(p.min_ctx) == 0 {
                return Ok(None);
            }
            p.min_ctx = p.suffix(p.min_ctx);
            if p.ns(p.min_ctx) != num_masked {
                break;
            }
        }
        let mc = p.min_ctx;
        let (mut hi, mut s, mut i) = (0u32, p.stats(mc), 0usize);
        let num = (p.ns(mc) - num_masked) as usize;
        while i != num {
            if s + 6 > p.mem.len() {
                return damaged();
            }
            if mask[p.mem[s] as usize] != 0 {
                hi += p.mem[s + 1] as u32;
                ps[i] = s;
                i += 1;
            }
            s += 6;
        }
        // MakeEscFreq
        let (see, mut freq_sum);
        if p.ns(mc) != 0xff {
            see = (p.ns2i[p.ns(mc) as usize + 2] as usize - 3) * 32
                + (p.summ(mc) > 11 * (p.ns(mc) + 1)) as usize
                + 2 * ((2 * p.ns(mc) < p.ns(p.suffix(mc)) + num_masked) as usize)
                + p.flags(mc) as usize;
            let r = (p.see_summ[see] >> p.see_shift[see]) as u32;
            p.see_summ[see] = p.see_summ[see].wrapping_sub(r as u16);
            freq_sum = r + (r == 0) as u32;
        } else {
            see = DUMMY;
            freq_sum = 1;
        }
        freq_sum += hi;
        let count = rc.threshold(freq_sum);
        if count < hi {
            let mut k = 0;
            hi = 0;
            loop {
                hi += p.freq(ps[k]);
                if hi > count {
                    break;
                }
                k += 1;
            }
            let s = ps[k];
            rc.decode(hi - p.freq(s), p.freq(s));
            if p.see_shift[see] < 7 {
                p.see_count[see] = p.see_count[see].wrapping_sub(1);
                if p.see_count[see] == 0 {
                    p.see_summ[see] = p.see_summ[see].wrapping_shl(1);
                    p.see_count[see] = (3u32 << p.see_shift[see]) as u8;
                    p.see_shift[see] += 1;
                }
            }
            p.found = s;
            let sym = p.u8(s) as u8;
            // Update2
            p.w16(mc + 2, p.summ(mc) + 4);
            p.w8(s + 1, p.freq(s) + 4);
            if p.freq(s) > MAX_FREQ {
                p.rescale();
            }
            p.run_length = p.init_rl;
            p.update_model()?;
            p.min_ctx = p.max_ctx;
            return Ok(Some(sym));
        }
        if count >= freq_sum {
            return damaged();
        }
        rc.decode(hi, freq_sum - hi);
        p.see_summ[see] = p.see_summ[see].wrapping_add(freq_sum as u16);
        while i != 0 {
            i -= 1;
            mask[p.u8(ps[i]) as usize] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The files of tests/data/{bzip2,lzma}.zip (Python's zipfile) and {bzip2,lzma,ppmd}-7z.zip (7-Zip 23.01): the same
    /// made-up game folder, its long file over several BZip2 blocks (`7z -mx=1`: 100 000-byte blocks).
    #[test]
    fn method_zips_unpack_the_same() {
        let read = |d: &[u8]| {
            let z = crate::zip::Zip::open(d.to_vec()).unwrap();
            let names: Vec<String> = z.names().map(String::from).collect();
            names.iter().map(|n| (n.clone(), z.read(n).unwrap_or_else(|| panic!("{n}")))).collect::<Vec<_>>()
        };
        let want = read(include_bytes!("../tests/data/bzip2.zip"));
        assert_eq!(want.len(), 4);
        assert_eq!(want.iter().find(|(n, _)| n == "SimGolf/Data/long.txt").unwrap().1.len(), 265000);
        for d in [
            &include_bytes!("../tests/data/bzip2-7z.zip")[..],
            include_bytes!("../tests/data/lzma.zip"),
            include_bytes!("../tests/data/lzma-7z.zip"),
            include_bytes!("../tests/data/ppmd-7z.zip"),
        ] {
            assert_eq!(read(d), want);
        }
    }

    #[test]
    fn damaged_streams_are_errors() {
        let junk: Vec<u8> = (0..400u32).map(|i| (i * 37 % 251) as u8).collect();
        assert!(bunzip2(&junk, 1000).is_err());
        assert!(unlzma(&[0, 0, 5, 0, 0x5d, 0, 0, 1, 0, 0, 9, 9, 9], 1000).is_err());
        let mut lz = vec![9, 20, 5, 0, 0x5d, 0, 0, 1, 0];
        lz.extend(&junk);
        assert!(unlzma(&lz, 100_000).is_err());
        let mut pp = vec![0x55, 0x00];
        pp.extend(&junk);
        // (PPMd decodes any bytes; damage shows as a bad checksum, or the data ending early)
        let _ = unppmd(&pp, 50_000);
        assert!(unppmd(&[0x50], 10).is_err());
        assert!(unpack(93, b"x", 1).is_err());
        assert_eq!(unpack(0, b"abc", 3).unwrap(), b"abc");
    }
}

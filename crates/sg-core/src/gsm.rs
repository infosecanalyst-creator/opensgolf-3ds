//! GSM 06.10 full-rate speech, as Windows' "GSM 6.10" codec stores it in a WAV (format 0x31, the "WAV49" packing): some
//! copies of the game shrunk their sounds to it. The port's own decoder, written from the published ETSI GSM 06.10
//! specification (its 16-bit fixed-point arithmetic, so the output is bit-exact with any conforming decoder).
//!
//! A WAV49 block is 65 bytes holding two 20 ms frames of 260 bits each, packed as one little-endian bit stream (the low
//! bit of each byte first), 320 samples at 8 kHz.

const MIN: i32 = -32768;
const MAX: i32 = 32767;

fn sat(v: i32) -> i32 {
    v.clamp(MIN, MAX)
}
fn add(a: i32, b: i32) -> i32 {
    sat(a + b)
}
fn sub(a: i32, b: i32) -> i32 {
    sat(a - b)
}
/// Rounded fractional multiply (Q15).
fn mult_r(a: i32, b: i32) -> i32 {
    if a == MIN && b == MIN {
        MAX
    } else {
        (a * b + 16384) >> 15
    }
}
fn asr(a: i32, n: i32) -> i32 {
    if n >= 16 {
        if a < 0 {
            -1
        } else {
            0
        }
    } else if n <= -16 {
        0
    } else if n < 0 {
        sat(a << -n)
    } else {
        a >> n
    }
}
fn asl(a: i32, n: i32) -> i32 {
    if n >= 16 {
        0
    } else if n <= -16 {
        if a < 0 {
            -1
        } else {
            0
        }
    } else if n < 0 {
        asr(a, -n)
    } else {
        sat(a << n)
    }
}

/// The parameters of one 20 ms frame (06.10 table 1.1).
#[derive(Default, Debug, PartialEq)]
struct Frame {
    larc: [i32; 8],
    nc: [i32; 4],
    bc: [i32; 4],
    mc: [i32; 4],
    xmaxc: [i32; 4],
    xmc: [[i32; 13]; 4],
}

/// Reads bits low bit first from a byte slice.
struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
}
impl Bits<'_> {
    fn get(&mut self, n: usize) -> i32 {
        let mut v = 0;
        for i in 0..n {
            let p = self.pos + i;
            let bit = self.d.get(p / 8).map_or(0, |b| (b >> (p % 8)) & 1);
            v |= (bit as i32) << i;
        }
        self.pos += n;
        v
    }
}

fn read_frame(b: &mut Bits) -> Frame {
    let mut f = Frame::default();
    for (i, n) in [6, 6, 5, 5, 4, 4, 3, 3].into_iter().enumerate() {
        f.larc[i] = b.get(n);
    }
    for j in 0..4 {
        f.nc[j] = b.get(7);
        f.bc[j] = b.get(2);
        f.mc[j] = b.get(2);
        f.xmaxc[j] = b.get(6);
        for x in &mut f.xmc[j] {
            *x = b.get(3);
        }
    }
    f
}

/// The decoder's memory from frame to frame.
pub struct Decoder {
    /// The reconstructed long-term residual: 120 past samples and the current subframe's 40.
    dp0: [i32; 160],
    /// The last valid long-term lag.
    nrp: i32,
    /// The decoded log area ratios of the previous and the current frame, `j` the current's.
    larpp: [[i32; 8]; 2],
    j: usize,
    /// The short-term synthesis filter's state.
    v: [i32; 9],
    /// The de-emphasis filter's state.
    msr: i32,
}

impl Default for Decoder {
    fn default() -> Self {
        Decoder { dp0: [0; 160], nrp: 40, larpp: [[0; 8]; 2], j: 0, v: [0; 9], msr: 0 }
    }
}

const QLB: [i32; 4] = [3277, 11469, 21299, 32767];
const FAC: [i32; 8] = [18431, 20479, 22527, 24575, 26623, 28671, 30719, 32767];
const B: [i32; 8] = [0, 0, 2048, -2560, 94, -1792, -341, -1144];
const MIC: [i32; 8] = [-32, -32, -16, -16, -8, -8, -4, -4];
const INVA: [i32; 8] = [13107, 13107, 13107, 13107, 19223, 17476, 31454, 29708];

impl Decoder {
    /// One frame's 160 samples.
    fn frame(&mut self, f: &Frame) -> [i16; 160] {
        let mut wt = [0i32; 160];
        for j in 0..4 {
            let erp = rpe_decode(f.xmaxc[j], f.mc[j], &f.xmc[j]);
            // long-term synthesis (06.10 4.3.2)
            let nr = if (40..=120).contains(&f.nc[j]) { f.nc[j] } else { self.nrp };
            self.nrp = nr;
            let brp = QLB[f.bc[j] as usize];
            for k in 0..40 {
                let drpp = mult_r(brp, self.dp0[120 + k - nr as usize]);
                self.dp0[120 + k] = add(erp[k], drpp);
            }
            wt[j * 40..j * 40 + 40].copy_from_slice(&self.dp0[120..160]);
            self.dp0.copy_within(40..160, 0);
        }
        // short-term synthesis (4.3.3): the log area ratios interpolated from the last frame's over the first 40 samples
        let prev = self.j;
        self.j ^= 1;
        let cur = self.j;
        for i in 0..8 {
            let t = add(f.larc[i], MIC[i]) << 10;
            let t = mult_r(INVA[i], sub(t, B[i] << 1));
            self.larpp[cur][i] = add(t, t);
        }
        let (o, n) = (self.larpp[prev], self.larpp[cur]);
        let mut s = [0i32; 160];
        let mut larp = [0i32; 8];
        for (from, to, mix) in [(0, 13, 0), (13, 27, 1), (27, 40, 2), (40, 160, 3)] {
            for i in 0..8 {
                larp[i] = match mix {
                    0 => add(add(o[i] >> 2, n[i] >> 2), o[i] >> 1),
                    1 => add(o[i] >> 1, n[i] >> 1),
                    2 => add(add(o[i] >> 2, n[i] >> 2), n[i] >> 1),
                    _ => n[i],
                };
            }
            let rp = larp.map(lar_to_rp);
            for k in from..to {
                let mut sri = wt[k];
                for i in (0..8).rev() {
                    sri = sub(sri, mult_r(rp[i], self.v[i]));
                    self.v[i + 1] = add(self.v[i], mult_r(rp[i], sri));
                }
                self.v[0] = sri;
                s[k] = sri;
            }
        }
        // de-emphasis, upscaling and truncation (4.3.5-4.3.7)
        let mut out = [0i16; 160];
        for k in 0..160 {
            self.msr = add(s[k], mult_r(self.msr, 28180));
            out[k] = (add(self.msr, self.msr) & !7) as i16;
        }
        out
    }
}

/// A log area ratio to its reflection coefficient (4.2.9.2).
fn lar_to_rp(lar: i32) -> i32 {
    let t = if lar == MIN { MAX } else { lar.abs() };
    let r = if t < 11059 {
        t << 1
    } else if t < 20070 {
        t + 11059
    } else {
        add(t >> 2, 26112)
    };
    if lar < 0 {
        -r
    } else {
        r
    }
}

/// The regular pulse excitation of one subframe (4.2.15-4.2.17 inverted): 13 pulses on a grid of every third sample.
fn rpe_decode(xmaxc: i32, mc: i32, xmc: &[i32; 13]) -> [i32; 40] {
    let mut exp = if xmaxc > 15 { (xmaxc >> 3) - 1 } else { 0 };
    let mut mant = xmaxc - (exp << 3);
    if mant == 0 {
        exp = -4;
        mant = 7;
    } else {
        while mant <= 7 {
            mant = (mant << 1) | 1;
            exp -= 1;
        }
        mant -= 8;
    }
    let temp1 = FAC[mant as usize];
    let temp2 = sub(6, exp);
    let temp3 = asl(1, sub(temp2, 1));
    let mut ep = [0i32; 40];
    for (i, &x) in xmc.iter().enumerate() {
        let t = mult_r(temp1, ((x << 1) - 7) << 12);
        ep[mc as usize + 3 * i] = asr(add(t, temp3), temp2);
    }
    ep
}

/// The samples of a WAV49 stream (whole 65-byte blocks; a partial last block is dropped).
pub fn decode_wav49(data: &[u8]) -> Vec<i16> {
    let mut dec = Decoder::default();
    let mut out = Vec::with_capacity(data.len() / 65 * 320);
    for block in data.chunks_exact(65) {
        let mut bits = Bits { d: block, pos: 0 };
        for _ in 0..2 {
            let f = read_frame(&mut bits);
            out.extend_from_slice(&dec.frame(&f));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_read_low_bit_first_and_the_second_starts_mid_byte() {
        // LARc[0] = 0x2a in the low six bits of the first byte, LARc[1] = 0x15 across the first two
        let mut block = [0u8; 65];
        block[0] = 0x2a | (0x01 << 6);
        block[1] = 0x05;
        // the second frame starts at bit 260: byte 32, high nibble
        block[32] = 0x70;
        block[33] = 0x03;
        let mut b = Bits { d: &block, pos: 0 };
        let f = read_frame(&mut b);
        assert_eq!((f.larc[0], f.larc[1]), (0x2a, 0x15));
        assert_eq!(b.pos, 260);
        let g = read_frame(&mut b);
        assert_eq!(g.larc[0], 0x37);
        assert_eq!(b.pos, 520);
    }

    #[test]
    fn a_wav49_file_decodes_bit_exact() {
        // a 0.2 s 440 Hz tone as ffmpeg's libgsm_ms writes it (tests/data/tone_gsm.wav), and the samples both ffmpeg's own
        // GSM decoder and libgsm's give for it (tone_gsm.s16)
        let w = crate::wav::decode(include_bytes!("../tests/data/tone_gsm.wav")).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate), (1, 16, 8000));
        let want: Vec<u8> = include_bytes!("../tests/data/tone_gsm.s16").to_vec();
        assert_eq!(w.pcm.len(), want.len());
        assert!(w.pcm == want, "first samples {:?}", &w.pcm[..16]);
        // a partial last block is dropped
        assert_eq!(decode_wav49(&[0u8; 65 * 3 + 10]).len(), 960);
    }

    #[test]
    fn rpe_and_reflection_tables() {
        // xmaxc 0: exponent -4, mantissa 7 (the smallest step); pulses on the grid Mc + 3i
        let e = rpe_decode(0, 2, &[7; 13]);
        assert!(e[2] > 0 && e[5] == e[2] && e[3] == 0 && e[0] == 0);
        assert_eq!(lar_to_rp(1000), 2000);
        assert_eq!(lar_to_rp(-15000), -26059);
        assert_eq!(lar_to_rp(MIN), -add(MAX >> 2, 26112));
    }
}

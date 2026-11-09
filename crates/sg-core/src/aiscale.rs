//! The built-in AI upscaler of "Make HD graphics" (hdgen): Real-ESRGAN's small general model (realesr-general-wdn-x4v3,
//! BSD-3-Clause, xinntao; data/README.md), run on the processor in plain Rust, so it needs no graphics card and nothing
//! installed and works the same on every system the game runs on.
//!
//! The model (SRVGGNetCompact): a 3x3 convolution from RGB to 64 features, 32 more of 64 to 64, each followed by a PReLU,
//! and one to 48 (3 colours x 4 x 4), shuffled into a picture four times the size and added to the input enlarged. Its
//! weights are kept as half floats (2.4 MB) and widened once.
//!
//! Only what can reach a visible pixel is worked out: a layer k convolutions from the end is needed within k pixels of an
//! opaque one, so the clear space round a sprite costs nearly nothing.
use std::sync::OnceLock;

const WEIGHTS: &[u8] = include_bytes!("../data/realesr-general-wdn-x4v3.f16");
/// Features.
const F: usize = 64;
/// Output channels of the last convolution: 3 colours x SCALE x SCALE.
const OUT: usize = 48;
pub const SCALE: usize = 4;
/// Convolutions after the first one, before the last.
const BODY: usize = 32;
/// Convolutions in all, and so how far (in pixels) a source pixel's effect reaches.
pub const REACH: usize = BODY + 2;

struct Conv {
    cin: usize,
    cout: usize,
    /// [tap (dy, dx)][cin][cout]
    w: Vec<f32>,
    b: Vec<f32>,
    /// The PReLU slopes after it (none after the last).
    slope: Option<Vec<f32>>,
}

fn f16(h: u16) -> f32 {
    let (s, e, m) = ((h >> 15) as u32, ((h >> 10) & 0x1f) as u32, (h & 0x3ff) as u32);
    let bits = match (e, m) {
        (0, 0) => s << 31,
        (0, _) => {
            // subnormal
            let v = m as f32 / 1024.0 / 16384.0;
            return if s == 1 { -v } else { v };
        }
        (31, _) => (s << 31) | 0x7f80_0000 | (m << 13),
        _ => (s << 31) | ((e + 112) << 23) | (m << 13),
    };
    f32::from_bits(bits)
}

fn model() -> &'static [Conv] {
    static M: OnceLock<Vec<Conv>> = OnceLock::new();
    M.get_or_init(|| {
        let vals: Vec<f32> = WEIGHTS.chunks_exact(2).map(|b| f16(u16::from_le_bytes([b[0], b[1]]))).collect();
        let mut at = 0;
        let mut take = |n: usize| {
            let v = vals[at..at + n].to_vec();
            at += n;
            v
        };
        let mut convs = Vec::new();
        for k in 0..REACH {
            let (cin, cout) = (if k == 0 { 3 } else { F }, if k == REACH - 1 { OUT } else { F });
            // stored as PyTorch's [cout][cin][dy][dx]
            let raw = take(cout * cin * 9);
            let b = take(cout);
            let mut w = vec![0.0; 9 * cin * cout];
            for o in 0..cout {
                for i in 0..cin {
                    for t in 0..9 {
                        w[(t * cin + i) * cout + o] = raw[(o * cin + i) * 9 + t];
                    }
                }
            }
            let slope = (k < REACH - 1).then(|| take(F));
            convs.push(Conv { cin, cout, w, b, slope });
        }
        debug_assert_eq!(at, vals.len());
        convs
    })
}

/// Pixels a kernel call works out along a row.
const P: usize = 6;

/// One row span of a convolution: `inp` and `out` are (w + 2 + P) x (h + 2) x channels, a border of one pixel round the
/// picture; the pixels x0..x1 of row y (picture coordinates) are worked out.
#[inline(always)]
fn conv_span<const FMA: bool>(c: &Conv, inp: &[f32], out: &mut [f32], stride: usize, y: usize, x0: usize, x1: usize) {
    let (cin, cout) = (c.cin, c.cout);
    let mut x = x0;
    while x < x1 {
        // P pixels at a time (past x1 the padding is written, and never read as needed)
        let mut ob = 0;
        while ob < cout {
            let mut acc = [[0f32; 16]; P];
            for a in acc.iter_mut() {
                a.copy_from_slice(&c.b[ob..ob + 16]);
            }
            for dy in 0..3 {
                let row = (y + dy) * stride;
                for dx in 0..3 {
                    let t = dy * 3 + dx;
                    let wt = &c.w[t * cin * cout..(t + 1) * cin * cout];
                    let base = (row + x + dx) * cin;
                    // the P pixels' inputs and the weights, checked once here rather than at every multiply-add
                    let px = &inp[base..base + P * cin];
                    let wt = &wt[..cin * cout];
                    assert!(ob + 16 <= cout);
                    for i in 0..cin {
                        // SAFETY: i < cin, ob + 16 <= cout and p < P, within the slices just taken
                        let w: [f32; 16] = unsafe { *(wt.as_ptr().add(i * cout + ob) as *const [f32; 16]) };
                        for (p, a) in acc.iter_mut().enumerate() {
                            let v = unsafe { *px.get_unchecked(p * cin + i) };
                            for o in 0..16 {
                                // fused where the processor has it (Rust does not fuse a * b + c by itself)
                                a[o] = if FMA { v.mul_add(w[o], a[o]) } else { a[o] + v * w[o] };
                            }
                        }
                    }
                }
            }
            let orow = ((y + 1) * stride + x + 1) * cout;
            for (p, a) in acc.iter().enumerate() {
                let dst = &mut out[orow + p * cout + ob..orow + p * cout + ob + 16];
                match &c.slope {
                    Some(s) => {
                        for o in 0..16 {
                            let v = a[o];
                            dst[o] = if v >= 0.0 { v } else { v * s[ob + o] };
                        }
                    }
                    None => dst.copy_from_slice(a),
                }
            }
            ob += 16;
        }
        x += P;
    }
}

/// A convolution over the row spans each row needs.
#[inline(always)]
fn conv_layer<const FMA: bool>(c: &Conv, inp: &[f32], out: &mut [f32], stride: usize, spans: &[Option<(usize, usize)>]) {
    for (y, s) in spans.iter().enumerate() {
        if let Some((x0, x1)) = *s {
            conv_span::<FMA>(c, inp, out, stride, y, x0, x1);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn conv_layer_avx2(c: &Conv, inp: &[f32], out: &mut [f32], stride: usize, spans: &[Option<(usize, usize)>]) {
    conv_layer::<true>(c, inp, out, stride, spans)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx2,fma")]
unsafe fn conv_layer_avx512(c: &Conv, inp: &[f32], out: &mut [f32], stride: usize, spans: &[Option<(usize, usize)>]) {
    conv_layer::<true>(c, inp, out, stride, spans)
}

fn run_layer(c: &Conv, inp: &[f32], out: &mut [f32], stride: usize, spans: &[Option<(usize, usize)>]) {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx512f") && std::env::var_os("SG_AI_NO_AVX512").is_none() {
            // SAFETY: the processor was just checked to have AVX-512 (and so AVX2 and FMA)
            return unsafe { conv_layer_avx512(c, inp, out, stride, spans) };
        }
        if std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma") {
            // SAFETY: the processor was just checked to have AVX2 and FMA
            return unsafe { conv_layer_avx2(c, inp, out, stride, spans) };
        }
    }
    // ARM's vector unit always has a fused multiply-add; elsewhere (a browser, an old PC) a multiply and an add
    conv_layer::<{ cfg!(target_arch = "aarch64") }>(c, inp, out, stride, spans)
}

/// Chessboard distance of each pixel to the nearest needed one (capped at `cap`).
fn distance(need: &[bool], w: usize, h: usize, cap: usize) -> Vec<u16> {
    let cap = cap.min(u16::MAX as usize - 1) as u16;
    let mut d: Vec<u16> = need.iter().map(|&n| if n { 0 } else { cap }).collect();
    for y in 0..h {
        for x in 0..w {
            let mut v = d[y * w + x];
            if x > 0 {
                v = v.min(d[y * w + x - 1] + 1);
            }
            if y > 0 {
                v = v.min(d[(y - 1) * w + x] + 1);
                if x > 0 {
                    v = v.min(d[(y - 1) * w + x - 1] + 1);
                }
                if x + 1 < w {
                    v = v.min(d[(y - 1) * w + x + 1] + 1);
                }
            }
            d[y * w + x] = v.min(cap);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut v = d[y * w + x];
            if x + 1 < w {
                v = v.min(d[y * w + x + 1] + 1);
            }
            if y + 1 < h {
                v = v.min(d[(y + 1) * w + x] + 1);
                if x + 1 < w {
                    v = v.min(d[(y + 1) * w + x + 1] + 1);
                }
                if x > 0 {
                    v = v.min(d[(y + 1) * w + x - 1] + 1);
                }
            }
            d[y * w + x] = v;
        }
    }
    d
}

/// The pixels a picture's upscale works out, on average over the layers (for estimates and tests): the share of the
/// picture's pixels times its size.
pub fn worked_px(need: &[bool], w: usize, h: usize) -> f64 {
    let dist = distance(need, w, h, REACH + 1);
    (0..REACH).map(|k| dist.iter().filter(|&&d| d as usize <= REACH - 1 - k).count() as f64).sum::<f64>() / REACH as f64
}

/// Multiply-adds per source pixel worked out (for estimates).
pub const MACS_PER_PX: f64 = (9 * (3 * F + BODY * F * F + F * OUT)) as f64;

/// A w x h RGB picture (bytes) four times the size, as RGB bytes. `need`: the pixels whose upscale is used (None: all);
/// the output elsewhere is the input enlarged.
pub fn upscale4(rgb: &[u8], w: usize, h: usize, need: Option<&[bool]>) -> Vec<u8> {
    let m = model();
    let all = vec![true; w * h];
    let need = need.unwrap_or(&all);
    let dist = distance(need, w, h, REACH + 1);
    let stride = w + 2 + P;
    let rows = h + 2;
    let mut a = vec![0f32; stride * rows * F];
    let mut b = vec![0f32; stride * rows * F];
    // the input, 3 channels a pixel, in a buffer laid out as the features' (channels 3 wide)
    for y in 0..h {
        for x in 0..w {
            let s = (y * w + x) * 3;
            let d = ((y + 1) * stride + x + 1) * 3;
            for k in 0..3 {
                a[d + k] = rgb[s + k] as f32 / 255.0;
            }
        }
    }
    for (k, c) in m.iter().enumerate() {
        // the output of convolution k is needed within REACH - 1 - k pixels of a needed pixel
        let reach = (REACH - 1 - k) as u16;
        let spans: Vec<Option<(usize, usize)>> = (0..h)
            .map(|y| {
                let row = &dist[y * w..(y + 1) * w];
                let x0 = row.iter().position(|&d| d <= reach)?;
                let x1 = row.iter().rposition(|&d| d <= reach)? + 1;
                Some((x0, x1))
            })
            .collect();
        // values a span does not cover are read as the padding's: zero them where the last layer left them
        b.iter_mut().for_each(|v| *v = 0.0);
        run_layer(c, &a, &mut b, stride, &spans);
        // a span's last P pixels can run past the picture: its right border is zero again for the next layer
        for y in 0..h {
            let r = ((y + 1) * stride + w + 1) * c.cout;
            b[r..((y + 2) * stride) * c.cout].iter_mut().for_each(|v| *v = 0.0);
        }
        std::mem::swap(&mut a, &mut b);
    }
    // a: OUT channels; shuffle and add the input enlarged
    let (ow, oh) = (w * SCALE, h * SCALE);
    let mut out = vec![0u8; ow * oh * 3];
    for y in 0..h {
        for x in 0..w {
            let s = ((y + 1) * stride + x + 1) * OUT;
            let used = dist[y * w + x] == 0;
            for c in 0..3 {
                let base = rgb[(y * w + x) * 3 + c] as f32 / 255.0;
                for i in 0..SCALE {
                    for j in 0..SCALE {
                        let v = if used { a[s + c * SCALE * SCALE + i * SCALE + j] + base } else { base };
                        out[((y * SCALE + i) * ow + x * SCALE + j) * 3 + c] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_widen() {
        assert_eq!(f16(0x3c00), 1.0);
        assert_eq!(f16(0xc000), -2.0);
        assert_eq!(f16(0x0000), 0.0);
        assert!((f16(0x3555) - 0.33325).abs() < 1e-4);
    }

    #[test]
    fn the_model_is_whole() {
        let m = model();
        assert_eq!(m.len(), REACH);
        assert_eq!((m[0].cin, m[0].cout), (3, F));
        assert_eq!((m[REACH - 1].cin, m[REACH - 1].cout), (F, OUT));
        assert_eq!(WEIGHTS.len() / 2, m.iter().map(|c| c.w.len() + c.b.len() + c.slope.as_ref().map_or(0, |s| s.len())).sum::<usize>());
    }

    #[test]
    fn a_flat_colour_stays_that_colour() {
        let (w, h): (usize, usize) = (12, 10);
        let rgb: Vec<u8> = (0..w * h).flat_map(|_| [120u8, 160, 80]).collect();
        let out = upscale4(&rgb, w, h, None);
        assert_eq!(out.len(), w * h * 48);
        // the middle (away from the edges' zero padding) keeps its colour within a few levels
        let (ow, mid) = (w * 4, (h * 2 * w * 4 + w * 2) * 3);
        let _ = ow;
        for k in 0..3 {
            assert!((out[mid + k] as i32 - [120, 160, 80][k]).abs() <= 6, "{:?}", &out[mid..mid + 3]);
        }
    }

    #[test]
    fn only_what_is_needed_is_worked_out_and_it_is_the_same() {
        // a small picture with a needed blob: the needed pixels match a run that works out everything
        let (w, h): (usize, usize) = (20, 16);
        let rgb: Vec<u8> = (0..w * h).flat_map(|i| [(i * 7 % 251) as u8, (i * 13 % 241) as u8, (i * 3 % 199) as u8]).collect();
        let need: Vec<bool> = (0..w * h).map(|i| (i % w).abs_diff(10) < 3 && (i / w).abs_diff(8) < 3).collect();
        let full = upscale4(&rgb, w, h, None);
        let part = upscale4(&rgb, w, h, Some(&need));
        for (i, &n) in need.iter().enumerate() {
            if n {
                let (x, y) = (i % w, i / w);
                for dy in 0..4 {
                    let o = ((y * 4 + dy) * w * 4 + x * 4) * 3;
                    assert_eq!(full[o..o + 12], part[o..o + 12]);
                }
            }
        }
    }
}

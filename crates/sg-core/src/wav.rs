//! WAV files. The original's own sounds are plain 16-bit PCM, but copies of the game that went through other tools (a
//! re-encoded download, a sound editor, a converter) hold the other common WAV codings. They are all turned into 8- or
//! 16-bit PCM here: 24- and 32-bit PCM, 32- and 64-bit float, A-law and mu-law, Microsoft ADPCM, IMA ADPCM, and
//! WAVE_FORMAT_EXTENSIBLE headers around any of them. More than two channels keep the first two.
use crate::assets::Wav;

fn u16at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

pub fn decode(d: &[u8]) -> Result<Wav, String> {
    if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE".into());
    }
    let mut pos = 12usize;
    let mut fmt: Option<&[u8]> = None;
    let mut frames_total: Option<usize> = None;
    while pos + 8 <= d.len() {
        let sz = u32at(d, pos + 4) as usize;
        let body = pos + 8;
        let avail = d.len() - body;
        let id = &d[pos..pos + 4];
        if id == b"fmt " && sz >= 16 && avail >= 16 {
            fmt = Some(&d[body..body + sz.min(avail)]);
        } else if id == b"fact" && sz >= 4 && avail >= 4 {
            frames_total = Some(u32at(d, body) as usize);
        } else if id == b"data" {
            let Some(f) = fmt else { return Err("data before fmt".into()) };
            return to_pcm(f, &d[body..body + sz.min(avail)], frames_total);
        }
        pos = pos.saturating_add(8).saturating_add(sz).saturating_add(sz & 1);
    }
    Err("no data chunk".into())
}

const PCM: u16 = 1;
const MS_ADPCM: u16 = 2;
const FLOAT: u16 = 3;
const ALAW: u16 = 6;
const MULAW: u16 = 7;
const IMA_ADPCM: u16 = 0x11;
const MP3: u16 = 0x55;
const EXTENSIBLE: u16 = 0xfffe;

fn to_pcm(f: &[u8], data: &[u8], frames_total: Option<usize>) -> Result<Wav, String> {
    let mut tag = u16at(f, 0);
    let ch = u16at(f, 2) as usize;
    let rate = u32at(f, 4);
    let align = u16at(f, 12) as usize;
    let bits = u16at(f, 14);
    if tag == EXTENSIBLE {
        // cbSize 22: valid bits, channel mask, then the sub-format GUID, whose first two bytes are the format tag
        if f.len() < 26 {
            return Err("short WAVE_FORMAT_EXTENSIBLE header".into());
        }
        tag = u16at(f, 24);
    }
    if ch == 0 || rate == 0 {
        return Err("WAV with no channels or no sample rate".into());
    }
    let out_ch = ch.min(2);
    if tag == PCM && (bits == 8 || bits == 16) && ch <= 2 {
        return Ok(Wav { channels: ch as u16, bits, sample_rate: rate, pcm: data.to_vec() });
    }
    // every sample of every channel in file order, as i16
    let samples: Vec<i16> = match (tag, bits) {
        (PCM, 8) => data.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        // the top two bytes of each little-endian sample
        (PCM, 16 | 24 | 32) => data.chunks_exact(bits as usize / 8).map(|s| i16::from_le_bytes([s[s.len() - 2], s[s.len() - 1]])).collect(),
        (FLOAT, 32) => {
            data.chunks_exact(4).map(|s| (f32::from_le_bytes([s[0], s[1], s[2], s[3]]).clamp(-1.0, 1.0) * 32767.0) as i16).collect()
        }
        (FLOAT, 64) => data
            .chunks_exact(8)
            .map(|s| (f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]).clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect(),
        (ALAW, 8) => data.iter().map(|&b| alaw(b)).collect(),
        (MULAW, 8) => data.iter().map(|&b| mulaw(b)).collect(),
        (MS_ADPCM, 4) => {
            if align < 7 * ch {
                return Err("MS ADPCM block too small".into());
            }
            let coefs = ms_adpcm_coefs(f);
            data.chunks(align).flat_map(|b| ms_adpcm_block(b, ch, &coefs)).collect()
        }
        (IMA_ADPCM, 4) => {
            if align < 4 * ch {
                return Err("IMA ADPCM block too small".into());
            }
            data.chunks(align).flat_map(|b| ima_adpcm_block(b, ch)).collect()
        }
        // MPEG layer 3 inside a WAV (copies repacked to save space): decoded whole; its own frames give the rate
        (MP3, _) => return mp3(data),
        _ => return Err(format!("unsupported WAV coding (format 0x{tag:04x}, {bits} bits)")),
    };
    let mut pcm = Vec::with_capacity(samples.len() / ch * out_ch * 2);
    for (i, v) in samples.iter().enumerate() {
        if i % ch < out_ch {
            pcm.extend_from_slice(&v.to_le_bytes());
        }
    }
    if let Some(n) = frames_total.filter(|_| matches!(tag, MS_ADPCM | IMA_ADPCM)) {
        // ADPCM pads its last block: the fact chunk gives the real length
        pcm.truncate(n * out_ch * 2);
    }
    Ok(Wav { channels: out_ch as u16, bits: 16, sample_rate: rate, pcm })
}

/// An MPEG audio stream (the data of a format 0x55 WAV, or a bare .mp3) as 16-bit PCM.
pub fn mp3(data: &[u8]) -> Result<Wav, String> {
    let d = nanomp3::decode_all::<i16>(data);
    let ch = match d.channels {
        Some(c) => c.num() as u16,
        None => return Err("MP3 inside a WAV: no audio frames found".into()),
    };
    if d.sample_rate == 0 || d.samples.is_empty() {
        return Err("MP3 inside a WAV: no audio frames found".into());
    }
    let mut pcm = Vec::with_capacity(d.samples.len() * 2);
    for v in d.samples {
        pcm.extend_from_slice(&v.to_le_bytes());
    }
    Ok(Wav { channels: ch, bits: 16, sample_rate: d.sample_rate, pcm })
}

/// G.711 A-law.
fn alaw(b: u8) -> i16 {
    let a = b ^ 0x55;
    let seg = ((a >> 4) & 7) as i32;
    let t = ((a & 0x0f) as i32) << 4;
    let t = if seg == 0 { t + 8 } else { (t + 0x108) << (seg - 1) };
    (if a & 0x80 != 0 { t } else { -t }) as i16
}

/// G.711 mu-law.
fn mulaw(b: u8) -> i16 {
    let u = !b;
    let t = ((((u & 0x0f) as i32) << 3) + 0x84) << ((u >> 4) & 7);
    (if u & 0x80 != 0 { 0x84 - t } else { t - 0x84 }) as i16
}

/// The predictor coefficient pairs of an MS ADPCM header (the standard seven when the header has none).
fn ms_adpcm_coefs(f: &[u8]) -> Vec<(i32, i32)> {
    let n = if f.len() >= 22 { u16at(f, 20) as usize } else { 0 };
    if n == 0 || f.len() < 22 + n * 4 {
        return vec![(256, 0), (512, -256), (0, 0), (192, 64), (240, 0), (460, -208), (392, -232)];
    }
    (0..n).map(|k| (u16at(f, 22 + k * 4) as i16 as i32, u16at(f, 24 + k * 4) as i16 as i32)).collect()
}

/// One block of Microsoft ADPCM, as interleaved samples.
fn ms_adpcm_block(b: &[u8], ch: usize, coefs: &[(i32, i32)]) -> Vec<i16> {
    const ADAPT: [i32; 16] = [230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230];
    if b.len() < 7 * ch {
        return Vec::new();
    }
    let rd = |o: usize| i16::from_le_bytes([b[o], b[o + 1]]) as i32;
    let c: Vec<(i32, i32)> = (0..ch).map(|k| coefs.get(b[k] as usize).copied().unwrap_or((0, 0))).collect();
    let mut delta: Vec<i32> = (0..ch).map(|k| rd(ch + 2 * k)).collect();
    let mut s1: Vec<i32> = (0..ch).map(|k| rd(3 * ch + 2 * k)).collect();
    let mut s2: Vec<i32> = (0..ch).map(|k| rd(5 * ch + 2 * k)).collect();
    let mut out = Vec::with_capacity((b.len() - 7 * ch) * 2 + 2 * ch);
    out.extend(s2.iter().chain(&s1).map(|&v| v as i16));
    let mut k = 0;
    for &byte in &b[7 * ch..] {
        for nib in [byte >> 4, byte & 0x0f] {
            let signed = if nib & 8 != 0 { nib as i32 - 16 } else { nib as i32 };
            let v = (((s1[k] * c[k].0 + s2[k] * c[k].1) >> 8) + signed * delta[k]).clamp(-32768, 32767);
            s2[k] = s1[k];
            s1[k] = v;
            delta[k] = ((ADAPT[nib as usize] * delta[k]) >> 8).max(16);
            out.push(v as i16);
            k = (k + 1) % ch;
        }
    }
    out
}

/// One block of IMA (DVI) ADPCM, as interleaved samples.
fn ima_adpcm_block(b: &[u8], ch: usize) -> Vec<i16> {
    const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
    const STEP: [i32; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143,
        157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552,
        1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487,
        12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
    ];
    if b.len() < 4 * ch {
        return Vec::new();
    }
    let mut pred: Vec<i32> = (0..ch).map(|k| i16::from_le_bytes([b[4 * k], b[4 * k + 1]]) as i32).collect();
    let mut idx: Vec<i32> = (0..ch).map(|k| (b[4 * k + 2] as i32).clamp(0, 88)).collect();
    let body = &b[4 * ch..];
    // after the header sample, each channel's samples come in runs of 8 (4 bytes, low nibble first), channels taking turns
    let groups = body.len() / (4 * ch);
    let per_ch = 1 + groups * 8;
    let mut planes: Vec<Vec<i16>> = (0..ch).map(|k| vec![pred[k] as i16]).collect();
    for g in 0..groups {
        for k in 0..ch {
            for &byte in &body[(g * ch + k) * 4..(g * ch + k) * 4 + 4] {
                for nib in [byte & 0x0f, byte >> 4] {
                    let step = STEP[idx[k] as usize];
                    let mut diff = step >> 3;
                    if nib & 4 != 0 {
                        diff += step;
                    }
                    if nib & 2 != 0 {
                        diff += step >> 1;
                    }
                    if nib & 1 != 0 {
                        diff += step >> 2;
                    }
                    pred[k] = if nib & 8 != 0 { pred[k] - diff } else { pred[k] + diff }.clamp(-32768, 32767);
                    idx[k] = (idx[k] + INDEX[nib as usize]).clamp(0, 88);
                    planes[k].push(pred[k] as i16);
                }
            }
        }
    }
    (0..per_ch).flat_map(|i| planes.iter().map(move |p| p[i])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A RIFF WAVE file around a fmt chunk body and data.
    fn riff(fmt: &[u8], fact: Option<u32>, data: &[u8]) -> Vec<u8> {
        let mut v = b"RIFF\0\0\0\0WAVE".to_vec();
        let mut chunk = |id: &[u8], body: &[u8]| {
            v.extend_from_slice(id);
            v.extend_from_slice(&(body.len() as u32).to_le_bytes());
            v.extend_from_slice(body);
            if body.len() % 2 == 1 {
                v.push(0);
            }
        };
        chunk(b"fmt ", fmt);
        if let Some(n) = fact {
            chunk(b"fact", &n.to_le_bytes());
        }
        chunk(b"data", data);
        v
    }
    fn fmt(tag: u16, ch: u16, rate: u32, align: u16, bits: u16, extra: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&tag.to_le_bytes());
        f.extend_from_slice(&ch.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * align as u32).to_le_bytes());
        f.extend_from_slice(&align.to_le_bytes());
        f.extend_from_slice(&bits.to_le_bytes());
        if !extra.is_empty() {
            f.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            f.extend_from_slice(extra);
        }
        f
    }
    fn i16s(w: &Wav) -> Vec<i16> {
        w.pcm.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]])).collect()
    }

    #[test]
    fn plain_pcm_is_kept_as_it_is() {
        let w = decode(&riff(&fmt(PCM, 2, 22050, 4, 16, &[]), None, &[1, 2, 3, 4])).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate, w.pcm), (2, 16, 22050, vec![1, 2, 3, 4]));
    }

    #[test]
    fn wide_float_law_and_extensible_codings_become_16_bit() {
        // 24-bit: 0x123456 -> 0x1234
        let w = decode(&riff(&fmt(PCM, 1, 8000, 3, 24, &[]), None, &[0x56, 0x34, 0x12])).unwrap();
        assert_eq!(i16s(&w), [0x1234]);
        let w = decode(&riff(&fmt(FLOAT, 1, 8000, 4, 32, &[]), None, &0.5f32.to_le_bytes())).unwrap();
        assert_eq!(i16s(&w), [16383]);
        // the same 16-bit PCM inside an extensible header: valid bits, mask, then the PCM GUID
        let mut ext = vec![16, 0, 4, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71];
        ext.truncate(22);
        let w = decode(&riff(&fmt(EXTENSIBLE, 1, 8000, 2, 16, &ext), None, &[0x34, 0x12])).unwrap();
        assert_eq!(i16s(&w), [0x1234]);
        // G.711: silence codes
        assert_eq!(mulaw(0xff), 0);
        assert_eq!(alaw(0xd5), 8);
        // three channels keep the first two
        let w = decode(&riff(&fmt(PCM, 3, 8000, 6, 16, &[]), None, &[1, 0, 2, 0, 3, 0])).unwrap();
        assert_eq!((w.channels, i16s(&w)), (2, vec![1, 2]));
        assert!(decode(&riff(&fmt(MP3, 1, 8000, 1, 0, &[]), None, &[0])).unwrap_err().contains("no audio frames"));
    }

    #[test]
    fn mp3_inside_a_wav_is_decoded() {
        // half a second of a 440 Hz tone, 22.05 kHz mono, as ffmpeg writes MP3 into a WAV (format 0x55)
        let w = decode(include_bytes!("../tests/data/tone_mp3.wav")).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate), (1, 16, 22050));
        let n = w.pcm.len() / 2;
        assert!((10000..13000).contains(&n), "{n} samples");
        let peak = w.pcm.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]).unsigned_abs()).max().unwrap();
        assert!((3000..4500).contains(&peak), "peak {peak}"); // ffmpeg's sine source is at 1/8 of full scale
    }

    #[test]
    fn adpcm_blocks_decode_to_their_samples() {
        // IMA, mono: header sample 1000 (index 0), then 8 nibbles of 0 (each adds step>>3 = 0 at step 7)
        let mut block = vec![0xe8, 0x03, 0, 0];
        block.extend_from_slice(&[0, 0, 0, 0]);
        let w = decode(&riff(&fmt(IMA_ADPCM, 1, 22050, 8, 4, &[9, 0]), Some(9), &block)).unwrap();
        assert_eq!(i16s(&w), vec![1000; 9]);
        // MS, mono: predictor 0 (256, 0), delta 16, sample1 200, sample2 100, then nibbles 1 and 0
        let block = [0, 16, 0, 200, 0, 100, 0, 0x10];
        let w = decode(&riff(&fmt(MS_ADPCM, 1, 22050, 8, 4, &[4, 0, 0, 0]), None, &block)).unwrap();
        // s2, s1, then s1*256>>8 + 1*16 = 216, then 216 + 0
        assert_eq!(i16s(&w), [100, 200, 216, 216]);
    }
}

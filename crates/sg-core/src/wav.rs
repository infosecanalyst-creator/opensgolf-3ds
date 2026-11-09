//! WAV files. The original's own sounds are plain 16-bit PCM, but copies of the game that went through other tools (a
//! re-encoded download, a sound editor, a converter) hold the other common WAV codings. They are all turned into 8- or
//! 16-bit PCM here: 24- and 32-bit PCM, 32- and 64-bit float, A-law and mu-law, Microsoft ADPCM, IMA ADPCM, GSM 6.10
//! (crate::gsm), MPEG layers 1, 2 and 3, and WAVE_FORMAT_EXTENSIBLE headers around any of them, in little-endian RIFF,
//! big-endian RIFX and RF64 files, and AIFF and AIFF-C files (PCM, float, G.711) too: one of the original's own sounds is
//! an AIFF called .wav. More than two channels keep the first two. [`probe`] tells from the headers alone
//! whether a file plays, and [`codec_name`] names a coding that does not (TrueSpeech, WMA and the like).
use crate::assets::Wav;

fn u16at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// A plausible RIFF chunk id: four printable ASCII characters.
fn is_id(d: &[u8], o: usize) -> bool {
    d.get(o..o + 4).is_some_and(|id| id.iter().all(|&c| (0x20..0x7f).contains(&c)))
}

/// The parts of a WAV file the decoder needs.
struct Parts<'a> {
    /// The fmt chunk's body, little-endian (a RIFX file's fields are swapped into it).
    fmt: Vec<u8>,
    data: &'a [u8],
    frames_total: Option<usize>,
    /// A big-endian RIFX file: its PCM samples are big-endian too.
    big: bool,
}

/// Finds the fmt, fact and data chunks of a RIFF (or big-endian RIFX, or RF64) WAVE file, as forgiving as the tools that
/// wrote them: chunks in any order (a LIST before the fmt, the data before the fmt), an odd-sized chunk without its pad
/// byte, a data chunk said to be longer than the file (cut at the end of the file) or of size 0 or 0xffffffff (written
/// by a tool that streamed it: the rest of the file).
fn parts(d: &[u8]) -> Result<Parts<'_>, String> {
    if d.len() < 12 || !matches!(&d[0..4], b"RIFF" | b"RIFX" | b"RF64") || &d[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE".into());
    }
    let big = &d[0..4] == b"RIFX";
    let size = |o: usize| if big { u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) } else { u32at(d, o) } as usize;
    let mut pos = 12usize;
    let mut fmt: Option<Vec<u8>> = None;
    let mut data: Option<&[u8]> = None;
    let mut frames_total: Option<usize> = None;
    while pos + 8 <= d.len() {
        let mut sz = size(pos + 4);
        let body = pos + 8;
        let avail = d.len() - body;
        let id = &d[pos..pos + 4];
        if id == b"data" && (sz == 0 || sz == u32::MAX as usize) && avail > 0 && !is_id(d, body) {
            sz = avail;
        }
        if id == b"fmt " && sz >= 16 && avail >= 16 && fmt.is_none() {
            let mut f = d[body..body + sz.min(avail)].to_vec();
            if big {
                for (o, w) in [(0, 2), (2, 2), (4, 4), (8, 4), (12, 2), (14, 2)] {
                    f[o..o + w].reverse();
                }
            }
            fmt = Some(f);
        } else if id == b"fact" && sz >= 4 && avail >= 4 {
            frames_total = Some(size(body));
        } else if id == b"data" && data.is_none() {
            data = Some(&d[body..body + sz.min(avail)]);
        }
        if fmt.is_some() && data.is_some() {
            break;
        }
        let next = body.saturating_add(sz);
        // an odd-sized chunk is followed by a pad byte, which some tools leave out
        pos = if sz & 1 == 1 && !is_id(d, next + 1) && is_id(d, next) { next } else { next.saturating_add(sz & 1) };
    }
    match (fmt, data) {
        (Some(fmt), Some(data)) => Ok(Parts { fmt, data, frames_total, big }),
        (None, _) => Err("no fmt chunk".into()),
        (_, None) => Err("no data chunk".into()),
    }
}

/// The format tag of a fmt chunk body, the sub-format's for a WAVE_FORMAT_EXTENSIBLE header.
fn format_tag(f: &[u8]) -> Result<u16, String> {
    let tag = u16at(f, 0);
    if tag != EXTENSIBLE {
        return Ok(tag);
    }
    // cbSize 22: valid bits, channel mask, then the sub-format GUID, whose first two bytes are the format tag
    if f.len() < 26 {
        return Err("short WAVE_FORMAT_EXTENSIBLE header".into());
    }
    Ok(u16at(f, 24))
}

/// An AIFF or AIFF-C file (big-endian, from Macs): the original's own "Sounds/Effects/Sheep 2.wav" is one. Its COMM chunk
/// (channels, frames, sample size, the rate as an 80-bit float, AIFF-C's compression) as a WAV fmt chunk body, and its
/// SSND chunk's samples turned to a WAV's (little-endian; 8-bit unsigned).
fn aiff(d: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let be16 = |o: usize| u16::from_be_bytes([d[o], d[o + 1]]);
    let be32 = |o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let aifc = &d[8..12] == b"AIFC";
    let mut pos = 12usize;
    let mut comm: Option<(u16, u16, u32, [u8; 4])> = None;
    let mut ssnd: Option<&[u8]> = None;
    while pos + 8 <= d.len() {
        let sz = be32(pos + 4) as usize;
        let body = pos + 8;
        let avail = d.len() - body;
        let id = &d[pos..pos + 4];
        if id == b"COMM" && sz >= 18 && avail >= 18 {
            // 80-bit extended: sign and 15-bit exponent, then a 64-bit mantissa with its leading one
            let e = (be16(body + 8) & 0x7fff) as i32;
            let m = u64::from_be_bytes(d[body + 10..body + 18].try_into().unwrap_or_default());
            let rate = (m as f64 * 2f64.powi(e - 16383 - 63)).round() as u32;
            let mut c = *b"NONE";
            if aifc && sz >= 22 && avail >= 22 {
                c.copy_from_slice(&d[body + 18..body + 22]);
            }
            comm = Some((be16(body), be16(body + 6), rate, c));
        } else if id == b"SSND" && sz >= 8 && avail >= 8 {
            let start = (body + 8).saturating_add(be32(body) as usize).min(d.len());
            ssnd = Some(&d[start..(body + sz.min(avail)).max(start)]);
        }
        pos = body.saturating_add(sz).saturating_add(sz & 1);
    }
    let (ch, bits, rate, c) = comm.ok_or("AIFF with no COMM chunk")?;
    let data = ssnd.ok_or("AIFF with no SSND chunk")?;
    let (tag, bits, mut data) = match &c {
        b"NONE" | b"twos" if matches!(bits, 8 | 16 | 24 | 32) => (PCM, bits, data.to_vec()),
        b"sowt" if matches!(bits, 16 | 24 | 32) => (PCM, bits, data.to_vec()),
        b"fl32" | b"FL32" => (FLOAT, 32, data.to_vec()),
        b"fl64" | b"FL64" => (FLOAT, 64, data.to_vec()),
        b"ulaw" | b"ULAW" => (MULAW, 8, data.to_vec()),
        b"alaw" | b"ALAW" => (ALAW, 8, data.to_vec()),
        _ => return Err(format!("AIFF-C coding '{}'", String::from_utf8_lossy(&c))),
    };
    if bits == 8 && tag == PCM {
        // AIFF's 8-bit samples are signed, a WAV's unsigned
        data.iter_mut().for_each(|b| *b ^= 0x80);
    } else if bits >= 16 && &c != b"sowt" {
        data.chunks_exact_mut(bits as usize / 8).for_each(|s| s.reverse());
    }
    let align = ch * bits.max(8) / 8;
    let mut f = Vec::with_capacity(16);
    for v in [tag as u32, ch as u32] {
        f.extend_from_slice(&(v as u16).to_le_bytes());
    }
    f.extend_from_slice(&rate.to_le_bytes());
    f.extend_from_slice(&(rate * align as u32).to_le_bytes());
    f.extend_from_slice(&align.to_le_bytes());
    f.extend_from_slice(&bits.to_le_bytes());
    Ok((f, data))
}

fn is_aiff(d: &[u8]) -> bool {
    d.len() >= 12 && &d[0..4] == b"FORM" && matches!(&d[8..12], b"AIFF" | b"AIFC")
}

pub fn decode(d: &[u8]) -> Result<Wav, String> {
    if is_aiff(d) {
        let (f, data) = aiff(d)?;
        return to_pcm(&f, &data, None);
    }
    let p = parts(d)?;
    if p.big {
        // a RIFX file (big-endian, from old Macs and some sound editors): only PCM and float are written so
        let tag = format_tag(&p.fmt)?;
        let bits = u16at(&p.fmt, 14) as usize;
        if !matches!(tag, PCM | FLOAT) {
            return Err(format!("{} in a big-endian (RIFX) WAV", codec_name(tag)));
        }
        let mut data = p.data.to_vec();
        if bits >= 16 {
            for s in data.chunks_exact_mut(bits / 8) {
                s.reverse();
            }
        }
        return to_pcm(&p.fmt, &data, p.frames_total);
    }
    to_pcm(&p.fmt, p.data, p.frames_total)
}

/// Whether [`decode`] plays this WAV, from its headers alone (the start's check of every one of the player's sounds,
/// Mixer::unplayable): Err names the coding when it does not.
pub fn probe(d: &[u8]) -> Result<(), String> {
    if is_aiff(d) {
        // (the samples' bytes are swapped on the way: cheap next to reading them)
        return aiff(d).map(|_| ());
    }
    let p = parts(d)?;
    let tag = format_tag(&p.fmt)?;
    let bits = u16at(&p.fmt, 14);
    let ok = match tag {
        PCM => matches!(bits, 8 | 16 | 24 | 32),
        FLOAT => matches!(bits, 32 | 64),
        ALAW | MULAW => bits == 8,
        MS_ADPCM | IMA_ADPCM => bits == 4 && !p.big,
        GSM | MPEG | MP3 => !p.big,
        _ => false,
    };
    if ok && u16at(&p.fmt, 2) > 0 && u32at(&p.fmt, 4) > 0 {
        Ok(())
    } else if ok {
        Err("WAV with no channels or no sample rate".into())
    } else if matches!(tag, PCM | FLOAT | ALAW | MULAW | MS_ADPCM | IMA_ADPCM) && !p.big {
        Err(format!("{}-bit {}", bits, codec_name(tag)))
    } else {
        Err(codec_name(tag))
    }
}

/// The name of a WAV format tag, for the player (the codings copies of the game were seen or are likely to use).
pub fn codec_name(tag: u16) -> String {
    match tag {
        PCM => "PCM".into(),
        MS_ADPCM => "Microsoft ADPCM".into(),
        FLOAT => "float PCM".into(),
        ALAW => "A-law".into(),
        MULAW => "mu-law".into(),
        0x10 => "OKI ADPCM".into(),
        IMA_ADPCM => "IMA ADPCM".into(),
        0x14 | 0x40 | 0x64 => "G.721/G.726 ADPCM".into(),
        0x22 => "DSP Group TrueSpeech".into(),
        GSM => "GSM 6.10".into(),
        0x32 => "MSN Audio".into(),
        0x42 => "G.723.1".into(),
        MPEG => "MPEG layer 1/2".into(),
        MP3 => "MP3".into(),
        0x92 => "Dolby AC-3".into(),
        0x130 => "Sipro ACELP.net".into(),
        0x160..=0x163 => "Windows Media Audio".into(),
        0x401 => "Intel Music Coder".into(),
        0x1600..=0x1601 | 0xa106 => "AAC".into(),
        0xf1ac => "FLAC".into(),
        0x674f..=0x6751 => "Ogg Vorbis inside a WAV".into(),
        _ => format!("WAV format 0x{tag:04x}"),
    }
}

const PCM: u16 = 1;
const MS_ADPCM: u16 = 2;
const FLOAT: u16 = 3;
const ALAW: u16 = 6;
const MULAW: u16 = 7;
const IMA_ADPCM: u16 = 0x11;
const GSM: u16 = 0x31;
const MPEG: u16 = 0x50;
const MP3: u16 = 0x55;
const EXTENSIBLE: u16 = 0xfffe;

fn to_pcm(f: &[u8], data: &[u8], frames_total: Option<usize>) -> Result<Wav, String> {
    let tag = format_tag(f)?;
    let ch = u16at(f, 2) as usize;
    let rate = u32at(f, 4);
    let align = u16at(f, 12) as usize;
    let bits = u16at(f, 14);
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
        // MPEG audio inside a WAV, layer 3 or layers 1 and 2 (copies repacked to save space): decoded whole; its own
        // frames give the rate
        (MP3 | MPEG, _) => return mp3(data),
        // GSM 6.10 (crate::gsm), mono at 8 kHz
        (GSM, _) if ch == 1 => crate::gsm::decode_wav49(data),
        (MS_ADPCM | IMA_ADPCM, _) | (PCM | FLOAT | ALAW | MULAW, _) => {
            return Err(format!("unsupported WAV coding ({bits}-bit {})", codec_name(tag)))
        }
        _ => return Err(format!("unsupported WAV coding ({})", codec_name(tag))),
    };
    let mut pcm = Vec::with_capacity(samples.len() / ch * out_ch * 2);
    for (i, v) in samples.iter().enumerate() {
        if i % ch < out_ch {
            pcm.extend_from_slice(&v.to_le_bytes());
        }
    }
    if let Some(n) = frames_total.filter(|_| matches!(tag, MS_ADPCM | IMA_ADPCM | GSM)) {
        // ADPCM and GSM pad their last block: the fact chunk gives the real length
        pcm.truncate(n * out_ch * 2);
    }
    Ok(Wav { channels: out_ch as u16, bits: 16, sample_rate: rate, pcm })
}

/// An MPEG audio stream, layer 1, 2 or 3 (the data of a format 0x50 or 0x55 WAV, or a bare .mp3 or .mp2) as 16-bit PCM.
pub fn mp3(data: &[u8]) -> Result<Wav, String> {
    let d = nanomp3::decode_all::<i16>(data);
    let ch = match d.channels {
        Some(c) => c.num() as u16,
        None => return Err("MPEG audio: no audio frames found".into()),
    };
    if d.sample_rate == 0 || d.samples.is_empty() {
        return Err("MPEG audio: no audio frames found".into());
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
                                                              // the same stream as a bare .mp3 (or a .wav that is one): told apart by its bytes
        let d = include_bytes!("../tests/data/tone_mp3.wav");
        let at = d.windows(4).position(|w| w == b"data").unwrap() + 8;
        let bare = crate::assets::decode_sound(&d[at..]).unwrap();
        assert_eq!(bare.sample_rate, 22050);
        // (within a frame or two of the WAV's: the bytes after its data chunk end the stream differently)
        assert!(bare.pcm.len().abs_diff(w.pcm.len()) <= 2 * 1152 * 2, "{} {}", bare.pcm.len(), w.pcm.len());
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

    #[test]
    fn mpeg_layer_2_inside_a_wav_is_decoded() {
        // half a second of a 440 Hz tone, 22.05 kHz mono, as ffmpeg writes MP2 into a WAV (format 0x50)
        let d = include_bytes!("../tests/data/tone_mp2.wav");
        assert_eq!(u16at(d, 20), MPEG);
        assert_eq!(probe(d), Ok(()));
        let w = decode(d).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate), (1, 16, 22050));
        let n = w.pcm.len() / 2;
        assert!((10000..13500).contains(&n), "{n} samples");
        let peak = w.pcm.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]).unsigned_abs()).max().unwrap();
        assert!((3000..4500).contains(&peak), "peak {peak}");
    }

    #[test]
    fn odd_files_are_read_as_their_writers_meant() {
        let f = fmt(PCM, 1, 8000, 2, 16, &[]);
        // a LIST chunk before the fmt, odd-sized and without its pad byte
        let mut v = b"RIFF\0\0\0\0WAVELIST\x03\0\0\0abc".to_vec();
        v.extend_from_slice(b"fmt \x10\0\0\0");
        v.extend_from_slice(&f);
        v.extend_from_slice(b"data\x04\0\0\0\x34\x12\x78\x56");
        assert_eq!(i16s(&decode(&v).unwrap()), [0x1234, 0x5678]);
        // a data chunk said to be longer than the file: cut at its end
        let mut v = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0".to_vec();
        v.extend_from_slice(&f);
        v.extend_from_slice(b"data\xff\0\0\0\x34\x12");
        assert_eq!(i16s(&decode(&v).unwrap()), [0x1234]);
        // the data before the fmt
        let mut v = b"RIFF\0\0\0\0WAVEdata\x02\0\0\0\x34\x12fmt \x10\0\0\0".to_vec();
        v.extend_from_slice(&f);
        assert_eq!(i16s(&decode(&v).unwrap()), [0x1234]);
        assert!(decode(&v[..22]).unwrap_err().contains("no fmt"));
        // a streamed data chunk of size 0: the rest of the file
        let mut v = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0".to_vec();
        v.extend_from_slice(&f);
        v.extend_from_slice(b"data\0\0\0\0\x34\x12");
        assert_eq!(i16s(&decode(&v).unwrap()), [0x1234]);
        // big-endian RIFX: sizes, fmt fields and samples
        let mut be = b"RIFX\0\0\0\0WAVEfmt \0\0\0\x10".to_vec();
        be.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0x1f, 0x40, 0, 0, 0x3e, 0x80, 0, 2, 0, 16]);
        be.extend_from_slice(b"data\0\0\0\x02\x12\x34");
        let w = decode(&be).unwrap();
        assert_eq!((w.sample_rate, i16s(&w)), (8000, vec![0x1234]));
        // RF64 (its sizes in a ds64 chunk; 0xffffffff in the data chunk's)
        let mut v = b"RF64\xff\xff\xff\xffWAVEds64\x04\0\0\0\0\0\0\0fmt \x10\0\0\0".to_vec();
        v.extend_from_slice(&f);
        v.extend_from_slice(b"data\xff\xff\xff\xff\x34\x12");
        assert_eq!(i16s(&decode(&v).unwrap()), [0x1234]);
    }

    #[test]
    fn the_headers_alone_say_what_plays_and_name_what_does_not() {
        assert_eq!(probe(&riff(&fmt(PCM, 1, 8000, 2, 16, &[]), None, &[0, 0])), Ok(()));
        assert_eq!(probe(&riff(&fmt(GSM, 1, 8000, 65, 0, &[64, 1]), None, &[0; 65])), Ok(()));
        assert_eq!(probe(&riff(&fmt(0x22, 1, 8000, 32, 1, &[]), None, &[0; 32])), Err("DSP Group TrueSpeech".into()));
        assert_eq!(probe(&riff(&fmt(PCM, 1, 8000, 2, 12, &[]), None, &[0, 0])), Err("12-bit PCM".into()));
        let mut ext = vec![16, 0, 4, 0, 0, 0, 0x31, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71];
        assert_eq!(probe(&riff(&fmt(EXTENSIBLE, 1, 8000, 65, 0, &ext), None, &[0; 65])), Ok(()));
        ext[6] = 0x61;
        ext[7] = 0x01;
        assert_eq!(probe(&riff(&fmt(EXTENSIBLE, 1, 8000, 65, 0, &ext), None, &[0; 65])), Err("Windows Media Audio".into()));
        assert!(decode(&riff(&fmt(0x22, 1, 8000, 32, 1, &[]), None, &[0; 32])).unwrap_err().contains("TrueSpeech"));
        // every coding the probe passes, decode takes (a GSM block as an example)
        assert_eq!(decode(&riff(&fmt(GSM, 1, 8000, 65, 0, &[64, 1]), Some(320), &[0; 65])).unwrap().pcm.len(), 640);
    }

    #[test]
    fn an_aiff_called_wav_is_read() {
        // as the original's "Sounds/Effects/Sheep 2.wav": mono 16-bit at 22050 Hz (40 0d ac 44: the 80-bit rate)
        let mut v = b"FORM\0\0\0\0AIFFCOMM\0\0\0\x12\0\x01\0\0\0\x02\0\x10\x40\x0d\xac\x44\0\0\0\0\0\0".to_vec();
        v.extend_from_slice(b"SSND\0\0\0\x0c\0\0\0\0\0\0\0\0\x12\x34\xff\xfe");
        assert_eq!(probe(&v), Ok(()));
        let w = decode(&v).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate), (1, 16, 22050));
        assert_eq!(i16s(&w), [0x1234, -2]);
        // 8-bit AIFF samples are signed
        v[27] = 8;
        let w = decode(&v).unwrap();
        assert_eq!((w.bits, w.pcm), (8, vec![0x92, 0xb4, 0x7f, 0x7e]));
        // an AIFF-C coding it does not know is named
        let mut c = b"FORM\0\0\0\0AIFCCOMM\0\0\0\x16\0\x01\0\0\0\x02\0\x10\x40\x0d\xac\x44\0\0\0\0\0\0ima4".to_vec();
        c.extend_from_slice(b"SSND\0\0\0\x08\0\0\0\0\0\0\0\0");
        assert_eq!(probe(&c), Err("AIFF-C coding 'ima4'".into()));
    }
}

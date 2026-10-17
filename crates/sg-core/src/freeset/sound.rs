//! Synthesised sounds: every sound file of the game gets a short sound of its kind, made from tones, noise and envelopes
//! (a click and whoosh for a drive, a plop for water, a babble of vowel sounds for a golfer's voice, a little tune for a
//! jingle). The set's own recordings (tools/freeset_art/sounds.py) come first; this is the fallback.
use super::enc::wav;
use super::hash;
use std::f32::consts::TAU;

const RATE: u32 = 22050;

/// Files the game's folders hold twice (the same piece under two names, or a piece the game never plays standing for
/// one of its kind): the set keeps one recording and plays it for both (tools/freeset_art/sounds.py, ALIASES; keep the
/// two lists the same). Paths as the manifest spells them, without the extension.
pub const ALIASES: &[(&str, &str)] = &[
    ("Sounds/Music/Misc_Music/New Age", "Sounds/New Age"),
    ("Sounds/Music/Tropical_Music/Steel Drums 2", "Sounds/World/Steel Drums 2"),
    ("Sounds/Music/Links_Music/Bagpipe1", "Sounds/World/Bagpipe"),
    ("Sounds/Music/Desert_Music/Spanish Guitar 2", "Sounds/World/Spanish Guitar wFlute 2"),
    ("Sounds/Steel Drum Short", "Sounds/Music/Tropical_Music/Steel Drum 1a"),
    ("Sounds/manifold music", "Sounds/Music/Misc_Music/Jazz"),
    ("Sounds/newhart-theme", "Sounds/Music/Misc_Music/End Of Year Short"),
];

/// The recording another file of the set plays (see [`ALIASES`]), by the manifest path without its extension.
pub fn alias(stem: &str) -> Option<&'static str> {
    ALIASES.iter().find(|(a, _)| a.eq_ignore_ascii_case(stem)).map(|(_, b)| *b)
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// A buffer of `secs` seconds to add sounds into.
struct Mix {
    s: Vec<f32>,
    rng: Rng,
}

impl Mix {
    fn new(secs: f32, seed: u32) -> Mix {
        Mix { s: vec![0.0; (secs * RATE as f32) as usize + 1], rng: Rng(seed | 1) }
    }
    fn len(&self) -> f32 {
        self.s.len() as f32 / RATE as f32
    }
    /// A tone from `t0` lasting `d` seconds, its pitch gliding from f0 to f1, with an attack/decay envelope.
    fn tone(&mut self, t0: f32, d: f32, f0: f32, f1: f32, amp: f32, harmonics: &[f32]) {
        let (a, n) = ((t0 * RATE as f32) as usize, (d * RATE as f32) as usize);
        let mut ph = 0.0f32;
        for i in 0..n {
            let k = a + i;
            if k >= self.s.len() {
                break;
            }
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * t;
            ph += f / RATE as f32;
            let env = (t * 40.0).min(1.0) * (1.0 - t).powf(1.5);
            let v: f32 = harmonics.iter().enumerate().map(|(h, &g)| g * ((h + 1) as f32 * ph * TAU).sin()).sum();
            self.s[k] += v * amp * env;
        }
    }
    /// Noise from `t0` for `d` seconds through a one-pole low-pass (`bright` 0..1), with an envelope of `attack` share.
    fn noise(&mut self, t0: f32, d: f32, amp: f32, bright: f32, attack: f32) {
        let (a, n) = ((t0 * RATE as f32) as usize, (d * RATE as f32) as usize);
        let mut lp = 0.0f32;
        for i in 0..n {
            let k = a + i;
            if k >= self.s.len() {
                break;
            }
            let t = i as f32 / n as f32;
            let env = if t < attack { t / attack.max(1e-3) } else { (1.0 - (t - attack) / (1.0 - attack).max(1e-3)).powi(2) };
            lp += (self.rng.next() - lp) * bright;
            self.s[k] += lp * amp * env;
        }
    }
    fn out(mut self) -> Vec<u8> {
        let peak = self.s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        if peak > 0.9 {
            for v in &mut self.s {
                *v *= 0.9 / peak;
            }
        }
        // a short fade at both ends against clicks
        let n = self.s.len();
        for i in 0..n.min(64) {
            let k = i as f32 / 64.0;
            self.s[i] *= k;
            self.s[n - 1 - i] *= k;
        }
        wav(RATE, &self.s)
    }
}

/// A note of the major scale from C5 (523 Hz), `step` scale steps up.
fn note(step: i32) -> f32 {
    const SCALE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let semis = SCALE[step.rem_euclid(7) as usize] + 12 * step.div_euclid(7);
    523.25 * 2f32.powf(semis as f32 / 12.0)
}

/// The sound for a file of the game (lower-case path), about `secs` long.
pub fn make(low: &str, secs: f32) -> Vec<u8> {
    let h = hash(low);
    let name = low.rsplit('/').next().unwrap_or(low);
    let has = |k: &str| name.contains(k);
    let voice = low.starts_with("simsfx/")
        || low.contains("/emotion/")
        || low.contains("/celebs/")
        || low.starts_with("sounds/male/")
        || low.starts_with("sounds/female/");
    if low.contains("ambience") {
        let mut m = Mix::new(secs.clamp(4.0, 8.0), h);
        let d = m.len();
        m.noise(0.0, d, 0.05, 0.03, 0.3);
        for k in 0..(d as usize * 2) {
            let t = (k as f32 * 0.47 + (h % 7) as f32 * 0.1) % (d - 0.4);
            let f = 2600.0 + 900.0 * ((k as u32 * 7 + h) % 5) as f32;
            m.tone(t, 0.08, f, f * 1.25, 0.12, &[1.0]);
            m.tone(t + 0.1, 0.07, f * 1.1, f * 0.9, 0.1, &[1.0]);
        }
        return m.out();
    }
    let jingle = [
        "buy", "harp", "music", "new age", "theme", "romeo", "sail", "steel", "bagpipe", "guitar", "tada", "spring", "winter", "twinkle",
        "test", "violin", "bass",
    ];
    if jingle.iter().any(|k| name.contains(k)) {
        // a little tune: a rising arpeggio and a closing chord, its notes picked by the file name
        let down = has("down") || has("winter") || has("bad");
        let len = secs.clamp(0.6, 3.0);
        let mut m = Mix::new(len, h);
        let n = ((len / 0.18) as i32).clamp(3, 10);
        let root = (h % 5) as i32 - 2;
        let pattern = [0, 2, 4, 7, 4, 2, 4, 7, 9, 7];
        let instrument: &[f32] = if has("harp") || has("guitar") {
            &[1.0, 0.5, 0.25]
        } else if has("steel") {
            &[1.0, 0.0, 0.6, 0.3]
        } else {
            &[1.0, 0.3, 0.1]
        };
        for k in 0..n - 1 {
            let step = if down { root + 7 - pattern[k as usize % 10] } else { root + pattern[k as usize % 10] };
            m.tone(k as f32 * len / n as f32, 0.35, note(step), note(step), 0.3, instrument);
        }
        let t = (n - 1) as f32 * len / n as f32;
        for s in if down { [0, 2, 5] } else { [0, 4, 7] } {
            m.tone(t, len - t, note(root + s), note(root + s), 0.18, instrument);
        }
        return m.out();
    }
    if voice {
        // a babble of vowel sounds: higher for women, rising when glad, falling when sad or cross
        let female =
            name.starts_with('f') || has("female") || has("fm") || has("princess") || has("model") || has("singer") || has("spears");
        let base = if female { 230.0 } else { 120.0 } * (1.0 + (h % 9) as f32 * 0.02);
        let mood: f32 = if has("happy") || has("lovely") || has("success") || has("good") || has("like") || has("yes") || has("celeb") {
            1.3
        } else if has("sad") || has("bad") || has("fail") || has("no") || has("dislike") || has("ugly") || has("tired") {
            0.75
        } else {
            1.0
        };
        let len = secs.clamp(0.3, 2.2);
        let mut m = Mix::new(len, h);
        let syl = ((len / 0.17) as usize).max(2);
        for k in 0..syl {
            let t = k as f32 * len / syl as f32;
            let p = k as f32 / syl as f32;
            let f0 = base * (1.0 + (mood - 1.0) * p) * (1.0 + ((h >> k) % 5) as f32 * 0.04);
            // a vowel: the voice's harmonics weighted by two formant bands
            let (f1, f2) = [(700.0, 1200.0), (400.0, 2000.0), (300.0, 900.0), (600.0, 1700.0)][((h >> (2 * k)) % 4) as usize];
            let harm: Vec<f32> = (1..=12)
                .map(|n| {
                    let f = f0 * n as f32;
                    0.9 / (1.0 + ((f - f1) / 250.0).powi(2)) + 0.5 / (1.0 + ((f - f2) / 300.0).powi(2))
                })
                .collect();
            m.tone(t, len / syl as f32 * 0.9, f0, f0 * (0.97 + 0.06 * mood.min(1.2) - 0.03), 0.22, &harm);
            if has("mad") || has("hard") {
                m.noise(t, 0.04, 0.1, 0.5, 0.2);
            }
        }
        return m.out();
    }
    let len = secs.clamp(0.15, 2.0);
    let mut m = Mix::new(len, h);
    if has("applause") {
        let len = secs.clamp(1.5, 3.0);
        m = Mix::new(len, h);
        for k in 0..(len * 60.0) as usize {
            let t = (k as f32 / 60.0 + m.rng.next().abs() * 0.02).min(len - 0.05);
            m.noise(t, 0.03, 0.25 * (1.0 - t / len).max(0.1), 0.6, 0.05);
        }
    } else if has("drive") || has("iron") || has("swing") || has("chip") || has("shotgun") {
        m.noise(0.0, 0.02, 0.9, 0.9, 0.02);
        m.tone(0.0, 0.05, 1800.0, 1200.0, 0.4, &[1.0, 0.3]);
        m.noise(0.02, (len - 0.05).min(0.4), 0.25, 0.15, 0.2);
    } else if has("putt") || has("bounce") || has("drop fairway") || has("bench") || has("building") || has("path") {
        m.tone(0.0, 0.12, 900.0, 700.0, 0.6, &[1.0, 0.2]);
        m.noise(0.0, 0.03, 0.3, 0.7, 0.05);
    } else if has("hole") {
        for k in 0..4 {
            m.tone(k as f32 * 0.09, 0.08, 1400.0 - k as f32 * 150.0, 1300.0 - k as f32 * 150.0, 0.4 / (k + 1) as f32, &[1.0, 0.4]);
        }
    } else if has("water") || has("splash") || has("lake") || has("dolphin") {
        m.noise(0.0, len.min(0.9), 0.6, 0.35, 0.05);
        m.tone(0.0, 0.2, 400.0, 150.0, 0.3, &[1.0]);
    } else if has("tree") || has("leaves") || has("maple") || has("palm") || has("flower") || has("grass") || has("rough") || has("sprout")
    {
        m.noise(0.0, len.min(1.0), 0.35, 0.25, 0.3);
    } else if has("sand") || has("bunker") || has("desert") || has("rocks") || has("fairway") {
        m.noise(0.0, len.min(0.7), 0.4, 0.5, 0.1);
        m.tone(0.0, 0.08, 300.0, 200.0, 0.3, &[1.0]);
    } else if has("cash") {
        m.tone(0.0, 0.3, 1568.0, 1568.0, 0.35, &[1.0, 0.5, 0.2]);
        m.tone(0.12, 0.6, 2093.0, 2093.0, 0.35, &[1.0, 0.5, 0.2]);
    } else if has("wrong") || has("unavailable") || has("belch") || has("unrest") || has("taunt") {
        m.tone(0.0, len.min(0.4), 140.0, 110.0, 0.4, &[1.0, 0.8, 0.6, 0.4]);
    } else if has("boing") {
        m.tone(0.0, 0.5, 200.0, 600.0, 0.5, &[1.0, 0.2]);
    } else if has("camera") {
        m.noise(0.0, 0.04, 0.8, 0.9, 0.05);
        m.noise(0.12, 0.05, 0.6, 0.9, 0.05);
    } else if has("cart") || has("bmw") || has("mix") || has("car") || has("hot air") {
        m = Mix::new(secs.clamp(0.6, 2.0), h);
        let d = m.len();
        m.tone(0.0, d, 70.0, 90.0, 0.4, &[1.0, 0.7, 0.5, 0.3, 0.2]);
    } else if has("sheep")
        || has("moose")
        || has("gila")
        || has("snake")
        || has("alligator")
        || has("fly")
        || has("seriama")
        || has("snore")
    {
        let f = 200.0 + (h % 400) as f32;
        m.tone(0.0, len.min(0.8), f, f * 0.8, 0.4, &[1.0, 0.6, 0.3]);
        if has("snake") || has("fly") {
            m.noise(0.0, len.min(0.8), 0.3, 0.8, 0.3);
        }
    } else if has("soda") {
        m.noise(0.0, 0.06, 0.5, 0.9, 0.05);
        m.noise(0.05, 0.6, 0.15, 0.7, 0.05);
    } else if has("text box") || has("check box") || has("button") || has("slide") {
        m.tone(0.0, 0.06, 1000.0 + (h % 400) as f32, 1100.0, 0.35, &[1.0, 0.3]);
    } else {
        // a soft two-note chime
        let r = (h % 5) as i32;
        m.tone(0.0, len.min(0.5), note(r), note(r), 0.3, &[1.0, 0.3]);
        m.tone(0.1, len.min(0.6), note(r + 2), note(r + 2), 0.25, &[1.0, 0.3]);
    }
    m.out()
}

#[cfg(test)]
mod tests {
    #[test]
    fn aliases_point_at_recordings() {
        let m = super::super::manifest();
        for (from, to) in super::ALIASES {
            let has = |p: &str| m.entries.iter().any(|e| e.path.eq_ignore_ascii_case(&format!("{p}.wav")));
            assert!(has(from), "{from} is not a sound of the game");
            assert!(has(to), "{to} is not a sound of the game");
            assert!(super::super::baked::get(&format!("{to}.ogg")).is_some(), "{to} has no recording");
            assert!(super::super::baked::get(&format!("{from}.ogg")).is_none(), "{from} is recorded twice");
        }
    }
}

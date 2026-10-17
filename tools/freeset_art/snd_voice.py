"""Made-up golfer voices for the free set: a source-filter formant synthesiser (a glottal pulse with breath, a cascade of
time-varying formant resonators, noise for the consonants) speaking an invented babble language, with moods (pitch,
range, tempo, tension, breathiness) and non-verbal sounds (laughs, sighs, yawns, gasps, hums, grunts, gulps...).
Nothing here is a word of any real language or of any game's voice language: syllables are drawn from our own small
inventory by a seeded generator.

    say(voice, script, mood, rng) -> samples at snd_dsp.SR

A script is a list of tokens: syllable strings ("ba", "NOO", "shee!") and specials ("<laugh 4>", "<sigh>", "<pause .2>").
Upper case marks stress; '!' '?' '.' '~' at a token's end shape the phrase's contour."""
import re

import numpy as np
from scipy import signal

import snd_dsp as d

FS = 44100  # synthesis rate (decimated to d.SR at the end)

# vowel formants (F1, F2, F3) of an adult male voice; other voices scale them
VOWELS = {
    "i": (280, 2250, 2950), "ih": (400, 1950, 2550), "e": (500, 1850, 2500), "ae": (650, 1700, 2400),
    "a": (740, 1180, 2450), "aw": (580, 880, 2400), "o": (470, 820, 2400), "u": (310, 880, 2250),
    "uu": (440, 1050, 2250), "uh": (620, 1220, 2400), "er": (490, 1350, 1700), "schwa": (520, 1450, 2450),
}
DIPHTHONGS = {"ai": ("a", "i"), "ay": ("e", "i"), "ow": ("a", "u"), "oy": ("o", "i"), "eo": ("e", "o")}
# grapheme -> vowel(s)
V_GRAPH = {"aa": "a", "ee": "i", "oo": "u", "oh": "o", "ah": "a", "eh": "e", "ih": "ih", "uh": "uh", "aw": "aw",
           "ae": "ae", "er": "er", "ai": "ai", "ay": "ay", "ow": "ow", "oy": "oy", "eo": "eo",
           "a": "a", "e": "e", "i": "ih", "o": "o", "u": "uu", "y": "i"}
# consonants: kind, voiced, F2 locus, noise centre, noise width, length
CONS = {
    "b": ("stop", True, 900, 1200, 2.0, 0.055), "p": ("stop", False, 900, 1200, 2.0, 0.07),
    "d": ("stop", True, 1700, 3800, 1.4, 0.05), "t": ("stop", False, 1700, 4200, 1.4, 0.065),
    "g": ("stop", True, 2100, 2200, 1.2, 0.055), "k": ("stop", False, 2100, 2400, 1.2, 0.07),
    "m": ("nasal", True, 1000, 0, 0, 0.07), "n": ("nasal", True, 1600, 0, 0, 0.065), "ng": ("nasal", True, 2100, 0, 0, 0.07),
    "l": ("approx", True, 1100, 0, 0, 0.06), "r": ("approx", True, 1150, 0, 0, 0.065), "w": ("approx", True, 700, 0, 0, 0.06),
    "y": ("approx", True, 2200, 0, 0, 0.05), "h": ("h", False, 0, 0, 0, 0.06),
    "s": ("fric", False, 1700, 6800, 2.5, 0.09), "z": ("fric", True, 1700, 6500, 2.5, 0.075),
    "sh": ("fric", False, 1900, 3300, 1.8, 0.09), "zh": ("fric", True, 1900, 3200, 1.8, 0.075),
    "f": ("fric", False, 900, 5000, 0.8, 0.08), "v": ("fric", True, 900, 4500, 0.8, 0.065),
    "ch": ("affr", False, 1900, 3400, 1.8, 0.09), "j": ("affr", True, 1900, 3300, 1.8, 0.08),
}
C_GRAPH = sorted(CONS, key=len, reverse=True)


class Voice:
    """A speaker: pitch, vocal tract size, breath, roughness."""

    def __init__(self, f0=120, tract=1.0, breath=0.08, tension=0.5, jitter=0.006, shimmer=0.04, vibrato=0.0,
                 rate=1.0, creak=0.0, nasal=0.0, name=""):
        self.f0, self.tract, self.breath, self.tension = f0, tract, breath, tension
        self.jitter, self.shimmer, self.vibrato, self.rate = jitter, shimmer, vibrato, rate
        self.creak, self.nasal, self.name = creak, nasal, name


# the cast: several voices of each kind, so a course sounds like many people
VOICES = {
    "m1": Voice(118, 1.0, 0.07, 0.55, name="everyman"),
    "m2": Voice(102, 0.95, 0.06, 0.6, creak=0.15, name="baritone"),
    "m3": Voice(136, 1.04, 0.10, 0.5, rate=1.1, name="bright tenor"),
    "m_old": Voice(112, 0.97, 0.16, 0.45, jitter=0.014, shimmer=0.08, vibrato=0.4, rate=0.85, name="old man"),
    "f1": Voice(212, 1.17, 0.14, 0.45, name="alto"),
    "f2": Voice(238, 1.2, 0.16, 0.42, rate=1.1, name="bright"),
    "f3": Voice(195, 1.14, 0.12, 0.5, creak=0.08, name="low"),
    "f_old": Voice(205, 1.13, 0.2, 0.4, jitter=0.014, shimmer=0.08, vibrato=0.5, rate=0.85, name="old woman"),
    # celebrities
    "action": Voice(84, 0.9, 0.05, 0.75, creak=0.35, rate=0.8, name="action star"),
    "aging": Voice(108, 0.96, 0.14, 0.45, vibrato=0.7, jitter=0.01, rate=0.85, name="aging star"),
    "comic": Voice(140, 1.03, 0.08, 0.6, rate=1.3, name="comedian"),
    "comic_f": Voice(225, 1.18, 0.1, 0.55, rate=1.25, name="comic"),
    "princess": Voice(265, 1.24, 0.15, 0.4, vibrato=0.3, name="princess"),
    "politician": Voice(110, 0.98, 0.06, 0.6, rate=0.9, name="politician"),
    "rocker": Voice(150, 1.0, 0.18, 0.8, creak=0.25, jitter=0.012, name="rock star"),
    "singer": Voice(250, 1.2, 0.18, 0.45, vibrato=0.55, name="pop singer"),
    "model": Voice(200, 1.16, 0.32, 0.35, rate=0.8, name="model"),
    "fitness": Voice(230, 1.18, 0.1, 0.65, rate=1.25, name="fitness coach"),
    "ranger": Voice(115, 0.98, 0.08, 0.5, rate=0.95, name="ranger"),
    "marshal": Voice(98, 0.95, 0.06, 0.65, rate=0.9, name="marshal"),
    "tray": Voice(240, 1.2, 0.12, 0.5, rate=1.15, name="tray server"),
    "tech": Voice(125, 1.0, 0.09, 0.5, name="lawn technician"),
}

# moods: pitch factor, pitch range (accent size), tempo, loudness, extra breath, tension change, final contour
MOODS = {
    "neutral": dict(pitch=1.0, rng=0.25, tempo=1.0, energy=1.0, breath=0.0, tense=0.0, final="."),
    "happy": dict(pitch=1.12, rng=0.45, tempo=1.12, energy=1.1, breath=0.02, tense=0.05, final="!"),
    "excited": dict(pitch=1.25, rng=0.6, tempo=1.2, energy=1.25, breath=0.03, tense=0.15, final="!"),
    "sad": dict(pitch=0.9, rng=0.15, tempo=0.78, energy=0.75, breath=0.12, tense=-0.15, final="."),
    "angry": dict(pitch=1.05, rng=0.4, tempo=1.15, energy=1.35, breath=0.0, tense=0.35, final="!"),
    "tired": dict(pitch=0.88, rng=0.12, tempo=0.72, energy=0.7, breath=0.18, tense=-0.2, final="."),
    "question": dict(pitch=1.05, rng=0.3, tempo=1.0, energy=1.0, breath=0.02, tense=0.0, final="?"),
    "awe": dict(pitch=1.08, rng=0.5, tempo=0.8, energy=0.95, breath=0.1, tense=-0.05, final="~"),
    "disgust": dict(pitch=0.95, rng=0.3, tempo=0.9, energy=1.0, breath=0.05, tense=0.25, final="."),
    "sly": dict(pitch=0.96, rng=0.35, tempo=0.9, energy=0.95, breath=0.08, tense=0.0, final="~"),
}

ONSETS = ["b", "d", "g", "k", "l", "m", "n", "p", "r", "s", "sh", "t", "v", "w", "y", "z", "f", "h", "ch", "j", ""]
NUCLEI = ["a", "aa", "e", "ee", "i", "o", "oo", "oh", "u", "ai", "ay", "ow", "oy", "eh", "ah", "er"]
CODAS = ["", "", "", "", "", "b", "d", "g", "l", "m", "n", "s", "sh", "p", "t", "k", "ng"]


def babble(rng, n, open_=0.75):
    """n random syllables of the made-up language (mostly open CV syllables, the odd closed one)."""
    out = []
    for _ in range(n):
        o = ONSETS[rng.integers(len(ONSETS))]
        v = NUCLEI[rng.integers(len(NUCLEI))]
        c = "" if rng.uniform() < open_ else CODAS[rng.integers(len(CODAS))]
        out.append(o + v + c)
    return out


def parse_syllable(s):
    """'shoo' -> (['sh'], 'u', []): consonant graphemes before the first vowel, the vowel, the consonants after it."""
    s = s.lower()
    vg = sorted(V_GRAPH, key=len, reverse=True)
    i = 0
    onset, coda, nucleus = [], [], None
    while i < len(s):
        c = next((g for g in C_GRAPH if s.startswith(g, i)), None)
        v = next((g for g in vg if s.startswith(g, i)), None)
        if nucleus is None:
            # a consonant starts the syllable unless it is a lone 'y' with no vowel after it
            if c and not (c == "y" and not any(ch in "aeiou" for ch in s[i + 1:])):
                onset.append(c)
                i += len(c)
            elif v:
                nucleus = V_GRAPH[v]
                i += len(v)
            else:
                i += 1
        elif c:
            coda.append(c)
            i += len(c)
        else:
            i += 1
    return onset, nucleus, coda


# ---- the synthesiser ----------------------------------------------------------------------------------------------------------

class Track:
    """Per-sample parameter tracks built segment by segment at FS."""

    KEYS = ("f1", "f2", "f3", "b1", "b2", "b3", "av", "ah", "f0", "nasal")

    def __init__(self):
        self.seg = []  # (n, dict of values, dict of end values or None)
        self.fric = []  # (start, n, centre, q, amp, voiced)
        self.bursts = []  # (start, centre, q, amp)
        self.pos = 0

    def add(self, dur, end=None, **v):
        n = max(1, int(dur * FS))
        self.seg.append((n, v, end))
        self.pos += n
        return self.pos - n

    def render(self):
        total = self.pos
        out = {k: np.zeros(total) for k in self.KEYS}
        p = 0
        last = {"f1": 500, "f2": 1500, "f3": 2500, "b1": 80, "b2": 100, "b3": 140, "av": 0, "ah": 0, "f0": 120, "nasal": 0}
        for n, v, end in self.seg:
            for k in self.KEYS:
                a = v.get(k, last[k])
                b = (end or {}).get(k, a)
                out[k][p:p + n] = np.linspace(a, b, n, endpoint=False)
                last[k] = b
            p += n
        return out


def _smooth(x, ms):
    k = max(1, int(ms / 1000 * FS))
    if k < 2:
        return x
    w = np.hanning(k)
    w /= w.sum()
    return np.convolve(np.pad(x, (k, k), mode="edge"), w, "same")[k:-k]


def _glottal(f0, oq, rng, voice):
    """A Rosenberg-like glottal flow derivative per sample for the f0 track, with jitter and shimmer."""
    n = len(f0)
    # slow random jitter (per ~period) and shimmer
    jit = _smooth(rng.standard_normal(n), 4) * voice.jitter * 6
    f = f0 * (1 + jit)
    ph = np.cumsum(f) / FS
    frac = ph % 1.0
    tp = oq * 0.7
    tn = oq * 0.3
    g = np.where(frac < tp, 0.5 * (1 - np.cos(np.pi * frac / tp)),
                 np.where(frac < tp + tn, np.cos(0.5 * np.pi * (frac - tp) / tn), 0.0))
    dg = np.diff(g, prepend=g[0])
    sh = 1 + _smooth(rng.standard_normal(n), 8) * voice.shimmer * 4
    return dg * sh * FS / np.maximum(f, 40) * 0.12, frac


def _cascade(x, tr, tract):
    """The vocal tract: resonators in cascade, F1-F3 following the tracks (updated every 32 samples), F4 and F5 fixed for
    the tract's size. Each is normalised to unity gain at 0 Hz, as in a Klatt cascade."""
    y = x
    blk = 32
    n = len(x)
    for fk, bk in (("f1", "b1"), ("f2", "b2"), ("f3", "b3")):
        out = np.zeros(n)
        zi = np.zeros(2)
        fr = tr[fk] * tract
        bws = tr[bk]
        for i in range(0, n, blk):
            f = min(float(fr[i]), FS * 0.45)
            r = np.exp(-np.pi * float(bws[i]) / FS)
            c = 2 * r * np.cos(2 * np.pi * f / FS)
            seg, zi = signal.lfilter([1 - c + r * r], [1, -c, r * r], y[i:i + blk], zi=zi)
            out[i:i + blk] = seg
        y = out
    for f, bw in ((3400 * tract, 260), (4300 * tract, 320)):
        if f < FS * 0.45:
            r = np.exp(-np.pi * bw / FS)
            c = 2 * r * np.cos(2 * np.pi * f / FS)
            y = signal.lfilter([1 - c + r * r], [1, -c, r * r], y)
    return y


class Speaker:
    def __init__(self, voice, mood, rng, pitch=1.0, energy=1.0):
        self.v = voice
        self.m = dict(MOODS[mood]) if isinstance(mood, str) else dict(mood)
        self.rng = rng
        self.tr = Track()
        self.base = voice.f0 * self.m["pitch"] * pitch
        self.energy = energy * self.m["energy"]
        self.accents = []  # (sample, factor)

    # -- building blocks
    def dur(self, x):
        return x / (self.v.rate * self.m["tempo"])

    def vowel_seg(self, vow, dur, amp, f0a, f0b, breath=0.0, end_vowel=None, end_amp=None):
        F = VOWELS[vow]
        E = VOWELS[end_vowel] if end_vowel else F
        bw = (70, 95, 130)
        b = self.v.breath + self.m["breath"] + breath
        ea = amp if end_amp is None else end_amp
        self.tr.add(dur, end=dict(f1=E[0], f2=E[1], f3=E[2], f0=f0b, av=ea * (1 - b * 0.5), ah=ea * b),
                    f1=F[0], f2=F[1], f3=F[2], b1=bw[0], b2=bw[1], b3=bw[2], av=amp * (1 - b * 0.5), ah=amp * b,
                    f0=f0a, nasal=0)

    def silence(self, dur, f0=None):
        self.tr.add(dur, av=0, ah=0, **({"f0": f0} if f0 else {}))

    def consonant(self, c, next_vowel, amp, f0, final=False):
        kind, voiced, locus, fc, q, ln = CONS[c]
        ln = self.dur(ln) * getattr(self, "cscale", 1.0)
        F = VOWELS.get(next_vowel, VOWELS["schwa"])
        if kind == "stop":
            # closure (a voice bar for voiced stops), then the burst and a little aspiration for the voiceless
            self.tr.add(ln * 0.7, f1=250, f2=locus, f3=F[2], av=amp * (0.12 if voiced else 0), ah=0, f0=f0)
            at = self.tr.pos
            self.tr.bursts.append((at, fc, q, amp * (0.55 if voiced else 0.9)))
            if not voiced:
                self.tr.add(ln * 0.45, f1=400, f2=(locus + F[1]) / 2, f3=F[2], av=0, ah=amp * 0.6, f0=f0)
            else:
                self.tr.add(ln * 0.15, f1=300, f2=locus, f3=F[2], av=amp * 0.5, ah=0, f0=f0)
        elif kind == "nasal":
            self.tr.add(ln, f1=260, f2=locus, f3=2300, b1=110, b2=200, b3=250, av=amp * 0.55, ah=0, f0=f0, nasal=1)
        elif kind == "approx":
            f1 = {"l": 350, "r": 400, "w": 300, "y": 280}[c]
            f3 = {"l": 2700, "r": 1650, "w": 2200, "y": 2900}[c]
            self.tr.add(ln, f1=f1, f2=locus, f3=f3, b1=90, b2=110, b3=150, av=amp * 0.75, ah=0, f0=f0, nasal=0)
        elif kind == "h":
            self.tr.add(ln, f1=F[0], f2=F[1], f3=F[2], av=0, ah=amp * 0.75, f0=f0)
        else:  # fricatives and affricates
            if kind == "affr":
                self.tr.add(ln * 0.35, f1=250, f2=locus, av=0, ah=0, f0=f0)
                self.tr.bursts.append((self.tr.pos, fc, q, amp * 0.6))
                ln *= 0.7
            start = self.tr.pos
            self.tr.add(ln, f1=300, f2=locus, f3=F[2], av=amp * (0.35 if voiced else 0), ah=0, f0=f0)
            self.tr.fric.append((start, int(ln * FS), fc, q, amp * (0.45 if voiced else 0.8), voiced))

    def syllable(self, syl, stress=False, f0=None, length=1.0, amp=1.0, end_pitch=None):
        onset, vow, coda = parse_syllable(syl)
        f0 = f0 or self.base
        a = amp * (1.25 if stress else 1.0)
        if vow is None:
            # no vowel: 'hmm', 'pff', 'tsk', 'brr' are their consonants, drawn out
            self.cscale = 2.6 * length
            for c in onset:
                self.consonant(c, "schwa", a * 0.9, f0)
            self.cscale = 1.0
            return
        for c in onset:
            self.consonant(c, DIPHTHONGS.get(vow, (vow,))[0], a * 0.9, f0)
        vd = self.dur(0.13 * length * (1.35 if stress else 1.0) * (1.1 if vow in DIPHTHONGS else 1.0))
        f0b = end_pitch or f0 * (0.97 if not stress else 1.02)
        # the vowel swells and fades a little, and its pitch moves: up into a stressed peak, then on to the end pitch
        fm = f0 * (1.06 if stress else 1.01)
        v1, v2 = DIPHTHONGS.get(vow, (vow, vow))
        self.vowel_seg(v1, vd * 0.3, a * 0.8, f0, fm, end_amp=a)
        self.vowel_seg(v1, vd * 0.7, a, fm, f0b, end_vowel=v2 if v2 != v1 else None, end_amp=a * 0.7)
        for c in coda:
            self.consonant(c, vow, a * 0.7, f0b, final=True)

    # -- non-verbal
    def laugh(self, n=4, high=1.0):
        f = self.base * 1.35 * high
        for k in range(n):
            self.consonant("h", "a", 0.9, f)
            vd = self.dur(0.085)
            self.vowel_seg("a" if self.v.f0 < 170 else "ae", vd, 1.0 - k * 0.12, f, f * 0.93, breath=0.15)
            self.silence(self.dur(0.045), f)
            f *= 0.94
        self.silence(0.04)

    def giggle(self, n=5):
        f = self.base * 1.6
        for k in range(n):
            self.consonant("h", "i", 0.7, f)
            self.vowel_seg("i", self.dur(0.06), 0.8 - k * 0.08, f, f * 0.95, breath=0.2)
            self.silence(self.dur(0.04), f)
            f *= 0.96

    def sigh(self, length=1.0):
        f = self.base * 0.95
        self.tr.add(0.28 * length, f1=700, f2=1200, f3=2500, av=0, ah=0.35, f0=f)  # in-breath (aspiration)
        self.silence(0.05)
        self.vowel_seg("a", 0.18 * length, 0.6, f * 1.05, f * 0.95, breath=0.45)
        self.vowel_seg("a", 0.45 * length, 0.55, f * 0.95, f * 0.72, breath=0.5, end_vowel="aw")
        self.tr.add(0.15, av=0, ah=0.12, f0=f * 0.7)

    def yawn(self):
        f = self.base
        self.vowel_seg("a", 0.25, 0.4, f * 0.95, f * 1.3, breath=0.35)
        self.vowel_seg("a", 0.45, 0.7, f * 1.3, f * 1.1, breath=0.3, end_vowel="aw")
        self.vowel_seg("aw", 0.35, 0.6, f * 1.1, f * 0.75, breath=0.35, end_vowel="o")
        self.consonant("m", "o", 0.5, f * 0.75)
        self.tr.add(0.12, f1=260, f2=1000, av=0.25, f0=f * 0.72, nasal=1)

    def gasp(self):
        self.tr.add(0.22, f1=750, f2=1250, f3=2500, end=dict(f1=500, f2=1700), av=0, ah=0.75, f0=self.base * 1.4)
        self.silence(0.05)

    def hum(self, notes, each=0.18, vowel_open=False):
        """A closed-mouth hum ('mm') of pitches (multiples of the base pitch)."""
        for k, p in enumerate(notes):
            f = self.base * p
            if vowel_open:
                self.vowel_seg("uu", each * 0.85, 0.7, f, f)
                self.silence(each * 0.15, f)
            else:
                self.tr.add(each, f1=260, f2=1000, f3=2300, b1=110, b2=200, b3=250, av=0.55, ah=0.02, f0=f, nasal=1)

    def grunt(self, hard=1.0):
        f = self.base * 0.9
        self.tr.add(0.02, f1=500, f2=1300, av=0, ah=0.4 * hard, f0=f)
        self.vowel_seg("uh", 0.07 * hard + 0.05, 1.2 * hard, f * 1.05, f * 0.8)
        self.tr.add(0.06, f1=300, f2=1300, av=0.3, ah=0.25, f0=f * 0.75)

    def creaky(self, vow, dur, f_mul=0.55, amp=0.7):
        f = self.base * f_mul
        self.vowel_seg(vow, dur, amp, f, f * 0.9)

    def lip_trill(self, dur=0.5):
        start = self.tr.pos
        self.tr.add(dur, f1=300, f2=800, f3=2200, av=0.8, ah=0.1, f0=self.base * 1.1)
        self.accents.append(("trill", start, int(dur * FS)))

    def cough(self, n=2):
        for k in range(n):
            self.tr.bursts.append((self.tr.pos, 1500, 0.8, 0.6))
            self.tr.add(0.1, f1=650, f2=1300, f3=2400, av=0.2, ah=0.9, f0=self.base * 1.1)
            self.tr.add(0.08, av=0, ah=0.1)
            self.silence(0.08)

    # -- rendering
    def render(self):
        tr = self.tr.render()
        n = len(tr["f0"])
        if n == 0:
            return np.zeros(1)
        for k in ("f1", "f2", "f3", "b1", "b2", "b3"):
            tr[k] = _smooth(tr[k], 22)
        f0 = _smooth(tr["f0"], 40)
        t = np.arange(n) / FS
        if self.v.vibrato:
            f0 = f0 * (1 + 0.012 * self.v.vibrato * np.sin(2 * np.pi * 5.3 * t) * np.clip(t / 0.3, 0, 1))
        # creak: drop to a rough low pitch in the quietest stretches of the phrase and at phrase ends
        av = _smooth(tr["av"], 14)
        ah = _smooth(tr["ah"], 11)
        tension = np.clip(self.v.tension + self.m["tense"], 0.05, 0.95)
        oq = 0.75 - 0.35 * tension
        glot, frac = _glottal(np.maximum(f0, 50), oq, self.rng, self.v)
        if self.v.creak:
            irr = 1 + self.v.creak * 0.6 * _smooth(self.rng.standard_normal(n), 3)
            glot *= np.clip(irr, 0.2, 2)
        # spectral tilt: softer voices lose highs
        tilt = 700 + 2600 * tension
        glot = signal.lfilter(*_lp_fs(tilt), glot)
        breath = self.rng.standard_normal(n)
        breath = signal.lfilter(*_hp_fs(400), breath)
        # breath is louder in the open phase of the glottis
        mod = 0.6 + 0.4 * (frac < oq)
        src = glot * av + breath * 0.05 * ah * mod + breath * 0.06 * ah
        for kind, s0, ln in [a for a in self.accents if a[0] == "trill"]:
            seg = np.arange(ln) / FS
            src[s0:s0 + ln] *= 0.35 + 0.65 * (0.5 + 0.5 * np.sin(2 * np.pi * 27 * seg))
        y = _cascade(src, tr, self.v.tract)
        # nasal murmur: damp the highs where nasal
        nas = _smooth(tr["nasal"], 10)
        if nas.any():
            y = y * (1 - nas) + signal.lfilter(*_lp_fs(700), y) * nas * 1.4
        # frication noise and bursts, filtered at their own place
        rms_v = np.sqrt(np.mean(y ** 2) + 1e-12)
        for (s0, ln, fc, q, amp, voiced) in self.tr.fric:
            nz = self.rng.standard_normal(ln)
            b, a = _bp_fs(fc * min(self.v.tract, 1.15), q)
            nz = signal.lfilter(b, a, nz)
            env = np.sin(np.linspace(0, np.pi, ln)) ** 0.6
            if voiced:
                env *= 0.7 + 0.3 * np.sin(2 * np.pi * np.cumsum(f0[s0:s0 + ln]) / FS)
            seg = nz / (np.std(nz) + 1e-9) * env * amp * rms_v * 0.55
            y[s0:s0 + ln] += seg[:max(0, min(ln, n - s0))]
        for (s0, fc, q, amp) in self.tr.bursts:
            ln = int(0.012 * FS)
            nz = self.rng.standard_normal(ln)
            b, a = _bp_fs(fc, q)
            nz = signal.lfilter(b, a, nz) * np.exp(-np.arange(ln) / (0.003 * FS))
            seg = nz / (np.std(nz) + 1e-9) * amp * rms_v * 0.9
            y[s0:s0 + ln] += seg[:max(0, min(ln, n - s0))]
        y *= self.energy
        # to the set's rate
        y = signal.resample_poly(y, d.SR, FS)
        return y


def _lp_fs(f):
    return signal.butter(1, min(f, FS * 0.45) / (FS / 2), "low")


def _hp_fs(f):
    return signal.butter(1, f / (FS / 2), "high")


def _bp_fs(fc, q):
    lo = max(100, fc / (1 + 0.5 / q * 2))
    hi = min(FS * 0.45, fc * (1 + 0.5 / q * 2))
    return signal.butter(2, [lo / (FS / 2), hi / (FS / 2)], "band")


TOKEN = re.compile(r"<(\w+)\s*([^>]*)>|([^\s<]+)")


def say(voice, script, mood="neutral", rng=None, pitch=1.0, energy=1.0):
    """Speaks a script (a string of tokens) and returns samples at d.SR."""
    rng = rng or np.random.default_rng(1)
    v = VOICES[voice] if isinstance(voice, str) else voice
    sp = Speaker(v, mood, rng, pitch, energy)
    toks = TOKEN.findall(script)
    words = [t for t in toks if t[2]]
    nwords = max(1, len(words))
    wi = 0
    for name, arg, word in toks:
        if name:
            args = arg.split()
            num = float(args[0]) if args and re.match(r"^[\d.]+$", args[0]) else None
            if name == "pause":
                sp.silence(num or 0.15)
            elif name == "laugh":
                sp.laugh(int(num or 4), float(args[1]) if len(args) > 1 else 1.0)
            elif name == "giggle":
                sp.giggle(int(num or 5))
            elif name == "sigh":
                sp.sigh(num or 1.0)
            elif name == "yawn":
                sp.yawn()
            elif name == "gasp":
                sp.gasp()
            elif name == "hum":
                sp.hum([float(a) for a in args] or [1, 1.12, 1.26, 1.12, 1], 0.17)
            elif name == "ooh":
                sp.hum([float(a) for a in args] or [1.0, 1.3, 1.0], 0.2, vowel_open=True)
            elif name == "grunt":
                sp.grunt(num or 1.0)
            elif name == "creak":
                sp.creaky(args[0] if args else "uh", float(args[1]) if len(args) > 1 else 0.4)
            elif name == "trill":
                sp.lip_trill(num or 0.5)
            elif name == "cough":
                sp.cough(int(num or 2))
            continue
        # a word: syllables joined by '-', stress in capitals, contour from the final punctuation
        punct = re.findall(r"[!?.~]+$", word)
        punct = punct[0] if punct else ""
        syls = [s for s in re.split(r"-", word.strip("!?.~,")) if s]
        pos = wi / max(1, nwords - 1) if nwords > 1 else 0.0
        decl = 1.0 - 0.12 * pos  # declination over the phrase
        last_word = wi == nwords - 1
        final = punct or (sp.m["final"] if last_word else "")
        for si, s in enumerate(syls):
            stress = s.isupper() and len(s) > 1 or s.endswith("'")
            s = s.strip("'")
            acc = 1 + sp.m["rng"] * (0.6 if stress else 0.15) * (1 if rng.uniform() < 0.8 else -0.5)
            # a lively, sing-song line: every syllable gets its own step up or down
            f = sp.base * decl * acc * (1 + rng.uniform(-0.09, 0.09) * (0.5 + sp.m["rng"]))
            last_syl = si == len(syls) - 1
            end = None
            length = 1.0
            if last_syl and final:
                if "?" in final:
                    end, length = f * 1.35, 1.35
                elif "!" in final:
                    f *= 1.12
                    end, length = f * 0.82, 1.25
                elif "~" in final:
                    end, length = f * 1.2, 1.6
                elif "." in final and last_word:
                    end, length = f * 0.8, 1.3
            if "..." in final and last_syl:
                length *= 1.5
            sp.syllable(s, stress, f, length, 1.0 if "!" not in final else 1.15, end)
        if "," in word or (punct and not last_word):
            sp.silence(sp.dur(0.12))
        else:
            sp.silence(sp.dur(0.025))
        wi += 1
    sp.silence(0.05)
    return sp.render()


def crowd_vowel(rng, d_, vowel="a", n=14, base=150, glide=(1.0, 1.0), spread=0.35):
    """Many people at once: an 'ooh', 'aww' or 'yay' from a crowd of voices with their own pitches and timing."""
    out = np.zeros(int(d_ * d.SR) + d.SR)
    for k in range(n):
        female = k % 2 == 1
        v = Voice(base * (1.85 if female else 1.0) * rng.uniform(0.85, 1.18), 1.17 if female else 1.0,
                  breath=0.2, tension=0.45, jitter=0.01)
        sp = Speaker(v, "neutral", rng)
        ln = d_ * rng.uniform(0.75, 1.0)
        f = v.f0
        sp.silence(rng.uniform(0, spread * 0.4))
        sp.vowel_seg(vowel, ln * 0.35, 0.8, f * glide[0], f * (glide[0] + glide[1]) / 2, breath=0.15)
        sp.vowel_seg(vowel, ln * 0.65, 0.7, f * (glide[0] + glide[1]) / 2, f * glide[1], breath=0.2)
        y = sp.render()
        y = y * np.hanning(len(y)) ** 0.3
        out[:len(y)] += y[:len(out)] * rng.uniform(0.6, 1.0)
    return out

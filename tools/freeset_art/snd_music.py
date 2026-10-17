"""The free set's music: instruments made in code (e-piano, plucked guitars and harps, mallets, steel pan, kalimba, flutes,
reeds, pads, basses, a small drum kit and hand percussion), a sequencer for chords, written melodies, bass lines, arpeggios
and drum patterns, and a mix bus with reverb, glue compression and a limiter. Pieces are in snd_songs.py.

Notes are MIDI numbers; times are in beats of the piece. A looping piece wraps its tails (reverb, ringing notes) round to
its start, so it loops without a seam."""
import re

import numpy as np
from scipy import signal

import snd_dsp as d

SR = d.SR
STATS = None  # set to a dict to collect each track's loudness (LUFS) when rendering
NOTE = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def midi(name):
    """'C4' 'F#5' 'Bb3' -> MIDI number (C4 = 60)."""
    m = re.fullmatch(r"([A-G])([#b]?)(-?\d)", name)
    if not m:
        raise ValueError(name)
    n = NOTE[m.group(1)] + {"#": 1, "b": -1, "": 0}[m.group(2)]
    return n + 12 * (int(m.group(3)) + 1)


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


QUAL = {
    "": [0, 4, 7], "maj": [0, 4, 7], "m": [0, 3, 7], "7": [0, 4, 7, 10], "maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10],
    "6": [0, 4, 7, 9], "m6": [0, 3, 7, 9], "9": [0, 4, 7, 10, 14], "maj9": [0, 4, 7, 11, 14], "m9": [0, 3, 7, 10, 14],
    "add9": [0, 4, 7, 14], "madd9": [0, 3, 7, 14], "sus2": [0, 2, 7], "sus4": [0, 5, 7], "7sus4": [0, 5, 7, 10],
    "13": [0, 4, 10, 14, 21], "dim": [0, 3, 6], "m7b5": [0, 3, 6, 10], "dim7": [0, 3, 6, 9], "aug": [0, 4, 8],
    "maj7#11": [0, 4, 7, 11, 18], "69": [0, 4, 7, 9, 14], "m11": [0, 3, 7, 10, 14, 17], "7b9": [0, 4, 7, 10, 13],
}


def chord(sym):
    """'Fmaj7' 'C7sus4' 'Am7/G' -> (root pitch class, intervals, bass pitch class)."""
    m = re.fullmatch(r"([A-G][#b]?)([^/]*)(?:/([A-G][#b]?))?", sym)
    if not m:
        raise ValueError(sym)

    def pc(s):
        return (NOTE[s[0]] + (1 if s[1:] == "#" else -1 if s[1:] == "b" else 0)) % 12
    root = pc(m.group(1))
    q = QUAL[m.group(2)]
    bass = pc(m.group(3)) if m.group(3) else root
    return root, q, bass


def voicing(sym, low=52, high=72):
    """A close voicing of a chord's upper notes between low and high (root left to the bass when there are 4+ notes)."""
    root, q, _ = chord(sym)
    ivs = q[1:] if len(q) >= 4 else q
    notes = []
    for iv in ivs:
        n = root + iv
        while n < low:
            n += 12
        while n >= high:
            n -= 12
        notes.append(n)
    return sorted(set(notes))


def parse_line(text):
    """A melody: 'C5:.5 r:.5 E5 G5:1.5 ...' -> [(beat, midi or None, beats, velocity)]; '>' before a note accents it,
    '_' after its length ties nothing but slurs (legato), durations default to 1 beat. '|' bar lines are ignored."""
    out = []
    t = 0.0
    for tok in text.split():
        if tok == "|":
            continue
        acc = tok.startswith(">")
        tok = tok.lstrip(">")
        if ":" in tok:
            n, ln = tok.split(":")
            ln = float(ln)
        else:
            n, ln = tok, 1.0
        if n != "r":
            out.append((t, midi(n), ln, 1.0 if acc else 0.8))
        t += ln
    return out


# ---- instruments: f (Hz), length (seconds the key is held), velocity 0..1 -> mono samples ------------------------------------

def _t(n):
    return np.arange(n) / SR


def _release(x, held, rel):
    """Fades a note out over `rel` seconds after it is released at `held` seconds."""
    n = len(x)
    t = _t(n)
    return x * np.clip(1 - (t - held) / max(rel, 1e-3), 0, 1)


def epiano(f, held, vel, rng):
    """A tine electric piano: two-operator FM (a bell-ish attack that mellows), a tine click, a slow decay."""
    tail = 0.6
    n = d.n_of(held + tail)
    t = _t(n)
    dec = 1.4 * (261 / f) ** 0.35 + 0.4
    idx = (0.6 + 2.2 * vel) * np.exp(-t / 0.35) + 0.25
    mod = np.sin(2 * np.pi * f * t) * idx
    car = np.sin(2 * np.pi * f * t + mod)
    tine = np.sin(2 * np.pi * f * 7.0 * t) * np.exp(-t / 0.03) * 0.25 * vel
    env = np.minimum(1, t / 0.002) * np.exp(-t / dec)
    y = (car + tine + 0.15 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.5)) * env
    return _release(y, held, 0.15) * (0.5 + 0.5 * vel)


def ks(f, held, vel, rng, bright=0.5, decay=3.0, body=None, tail=0.4):
    """A plucked string (Karplus-Strong, a period at a time) with a pick of `bright`ness and a decay time in seconds."""
    n = d.n_of(held + tail)
    p = SR / f
    pi = max(2, int(p))
    frac = p - pi
    exc = rng.uniform(-1, 1, pi)
    exc = d.lp(exc, 1500 + 7000 * bright * vel) if pi > 8 else exc
    exc -= exc.mean()
    g = 10 ** (-3 / (decay * f))  # per period loss for a 60 dB fall in `decay` seconds
    s = 0.5 - 0.25 * bright
    out = np.empty(n + pi)
    out[:pi] = exc
    i = pi
    prev = exc
    while i < n + pi:
        # two-point average (loss) plus a little fractional delay for tuning
        nxt = (prev * (1 - s) + np.roll(prev, 1) * s)
        nxt = nxt * (1 - frac * 0.5) + np.roll(nxt, 1) * frac * 0.5
        nxt *= g
        m = min(pi, n + pi - i)
        out[i:i + m] = nxt[:m]
        prev = nxt
        i += pi
    y = out[:n]
    if body:
        y = sum(d.peak(y, bf, bg, 1.2) for bf, bg in body) / len(body) + y * 0.4
    return _release(y, held, 0.08) * (0.4 + 0.6 * vel)


def nylon(f, held, vel, rng):
    return ks(f, held, vel, rng, bright=0.35, decay=2.6, body=((110, 6), (220, 4), (450, 3)), tail=0.5)


def harp(f, held, vel, rng):
    return ks(f, max(held, 1.2), vel, rng, bright=0.55, decay=3.5, tail=1.0)


def harpsichord(f, held, vel, rng):
    y = ks(f, held, vel, rng, bright=0.95, decay=2.0, tail=0.25)
    # the jack's pluck: a little bright click
    c = d.hp(rng.standard_normal(d.n_of(0.01)), 2500) * np.exp(-_t(d.n_of(0.01)) / 0.002) * 0.25
    y[:len(c)] += c
    return y


def uke(f, held, vel, rng):
    return ks(f, held, vel, rng, bright=0.55, decay=1.2, body=((300, 5), (600, 3)), tail=0.2)


def _mallet(f, held, vel, rng, ratios, amps, decays, tail=1.5, noise=0.02, trem=0.0):
    n = d.n_of(max(held, 0.2) + tail)
    t = _t(n)
    y = np.zeros(n)
    k = (261 / f) ** 0.3
    for r, a, dc in zip(ratios, amps, decays):
        if f * r < SR * 0.45:
            y += a * np.sin(2 * np.pi * f * r * t) * np.exp(-t / (dc * k))
    y *= np.minimum(1, t / 0.0015)
    if noise:
        m = d.n_of(0.012)
        y[:m] += d.lp(rng.standard_normal(m), 3000) * np.exp(-_t(m) / 0.003) * noise
    if trem:
        y *= 1 - trem * (0.5 + 0.5 * np.sin(2 * np.pi * 5.2 * t))
    return y * (0.45 + 0.55 * vel)


def marimba(f, held, vel, rng):
    return _mallet(f, held, vel, rng, (1, 3.93, 9.2), (1, 0.25, 0.06), (0.45, 0.08, 0.02), tail=0.9, noise=0.15)


def vibes(f, held, vel, rng):
    return _mallet(f, held, vel, rng, (1, 4.0, 10.0), (1, 0.18, 0.04), (1.6, 0.25, 0.05), tail=2.0, noise=0.05,
                   trem=0.25)


def glock(f, held, vel, rng):
    return _mallet(f, held, vel, rng, (1, 2.76, 5.4, 8.93), (1, 0.35, 0.12, 0.05), (1.1, 0.35, 0.12, 0.05), tail=1.5,
                   noise=0.05)


def celesta(f, held, vel, rng):
    return _mallet(f, held, vel, rng, (1, 2.0, 3.0, 4.2), (1, 0.3, 0.08, 0.03), (0.9, 0.3, 0.12, 0.05), tail=1.2,
                   noise=0.03)


def kalimba(f, held, vel, rng):
    return _mallet(f, held, vel, rng, (1, 5.92, 11.6), (1, 0.18, 0.05), (0.8, 0.05, 0.02), tail=1.2, noise=0.08)


def steelpan(f, held, vel, rng):
    n = d.n_of(max(held, 0.3) + 1.0)
    t = _t(n)
    k = (392 / f) ** 0.25
    bend = 1 + 0.01 * np.exp(-t / 0.02)
    ph = 2 * np.pi * np.cumsum(f * bend) / SR
    y = (np.sin(ph) * np.exp(-t / (1.0 * k)) + 0.55 * np.sin(2 * ph + 0.3) * np.exp(-t / (0.45 * k))
         + 0.25 * np.sin(3.01 * ph) * np.exp(-t / (0.25 * k)) + 0.12 * np.sin(4.04 * ph) * np.exp(-t / (0.12 * k)))
    y *= np.minimum(1, t / 0.004)
    return y * (0.45 + 0.55 * vel)


def flute(f, held, vel, rng, breathy=0.12, vib=0.006):
    n = d.n_of(held + 0.12)
    t = _t(n)
    v = 1 + vib * np.sin(2 * np.pi * 5.0 * t) * np.clip((t - 0.25) / 0.3, 0, 1)
    ph = 2 * np.pi * np.cumsum(f * v) / SR
    y = np.sin(ph) + 0.18 * np.sin(2 * ph) + 0.06 * np.sin(3 * ph)
    br = d.bp(rng.standard_normal(n), f * 2, 2.0) * breathy * 2
    env = d.adsr(n, a=0.05, d=0.2, s=0.85, r=0.1, hold=held)
    chiff = np.exp(-t / 0.03) * d.hp(rng.standard_normal(n), 2000) * 0.08
    return (y + br) * env * (0.5 + 0.5 * vel) + chiff * vel


def whistle(f, held, vel, rng):
    return flute(f, held, vel, rng, breathy=0.06, vib=0.01)


def panflute(f, held, vel, rng):
    return flute(f, held, vel, rng, breathy=0.3, vib=0.004)


def reed(f, held, vel, rng, bright=2200):
    """A soft reed (accordion / chanter) from two detuned band-limited saws through a body formant."""
    n = d.n_of(held + 0.1)
    t = _t(n)
    v = 1 + 0.003 * np.sin(2 * np.pi * 5.5 * t)
    y = d.saw_bl(f * v, held + 0.1, 24) + d.saw_bl(f * 1.004 * v, held + 0.1, 24)
    y = d.lp(y, bright)
    y = d.peak(y, 1100, 4, 1.5)
    env = d.adsr(n, a=0.03, d=0.1, s=0.9, r=0.08, hold=held)
    return y * env * 0.4 * (0.6 + 0.4 * vel)


def pad(f, held, vel, rng, cutoff=1800):
    """A warm pad: five detuned saws, low passed, slow attack and release."""
    n = d.n_of(held + 0.8)
    t = _t(n)
    y = np.zeros(n)
    for det in (-0.011, -0.004, 0.0, 0.005, 0.012):
        y += d.saw_bl(f * (1 + det), held + 0.8, 18)
    y = d.lp(d.lp(y, cutoff), cutoff * 1.3)
    env = d.adsr(n, a=0.35, d=0.5, s=0.8, r=0.75, hold=held)
    return y * env * 0.18 * (0.6 + 0.4 * vel)


def strings(f, held, vel, rng):
    n = d.n_of(held + 0.5)
    t = _t(n)
    vib = 1 + 0.004 * np.sin(2 * np.pi * 5.3 * t + rng.uniform(0, 6))
    y = sum(d.saw_bl(f * vib * (1 + det), held + 0.5, 30) for det in (-0.006, 0.0, 0.007))
    y = d.peak(d.lp(y, 3200), 900, 3, 1.0)
    env = d.adsr(n, a=0.12, d=0.3, s=0.85, r=0.4, hold=held)
    return y * env * 0.15 * (0.6 + 0.4 * vel)


def pizz(f, held, vel, rng):
    return ks(f, min(held, 0.3), vel, rng, bright=0.3, decay=0.6, tail=0.25)


def bass(f, held, vel, rng):
    """A warm round bass: sine and a soft low-passed saw with a little pluck at the start."""
    n = d.n_of(held + 0.12)
    t = _t(n)
    y = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t)
    saw = d.saw_bl(f, held + 0.12, 20)
    cut = 300 + 1200 * vel * np.exp(-t / 0.08)
    saw = d.sweep_filter(saw, "lp", cut, 0.8, 64)
    y = d.softclip(y * 0.7 + saw * 0.5, 1.3)
    env = d.adsr(n, a=0.004, d=0.35, s=0.65, r=0.08, hold=held)
    return y * env * (0.6 + 0.4 * vel)


def upright(f, held, vel, rng):
    y = ks(f, held, vel, rng, bright=0.15, decay=1.8, tail=0.2)
    thump = np.sin(2 * np.pi * f * _t(len(y))) * np.exp(-_t(len(y)) / 0.4) * 0.6
    return _release(d.lp(y, 1200) * 1.6 + thump, held, 0.08)


INSTR = {"epiano": epiano, "nylon": nylon, "harp": harp, "harpsichord": harpsichord, "uke": uke, "marimba": marimba,
         "vibes": vibes, "glock": glock, "celesta": celesta, "kalimba": kalimba, "steelpan": steelpan, "flute": flute,
         "whistle": whistle, "panflute": panflute, "reed": reed, "pad": pad, "strings": strings, "pizz": pizz,
         "bass": bass, "upright": upright, "pluck": lambda f, h, v, r: ks(f, h, v, r, 0.6, 1.5, tail=0.3)}


_CAL = {}


def calibration(name):
    """A gain that brings an instrument's typical note (a held A4, or A2 for the basses) to one loudness, so that track
    gains in the pieces are a mix, not a fix for how loud each instrument happens to come out."""
    if name not in _CAL:
        rng = np.random.default_rng(5)
        m = 45 if name in ("bass", "upright") else 69
        x = INSTR[name](hz(m), 0.6, 0.8, rng)
        _CAL[name] = 10 ** ((-20 - d.momentary_max(x)) / 20)
    return _CAL[name]


# ---- drums ---------------------------------------------------------------------------------------------------------------------

def kick(vel, rng):
    n = d.n_of(0.35)
    t = _t(n)
    f = 48 + 70 * np.exp(-t / 0.04)
    y = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.16)
    y[:d.n_of(0.004)] += d.lp(rng.standard_normal(d.n_of(0.004)), 3000) * 0.3
    return d.softclip(y * 1.2, 1.2) * vel


def snare(vel, rng, brush=False):
    n = d.n_of(0.25)
    t = _t(n)
    if brush:
        nz = d.band(rng.standard_normal(n), 800, 6000) * np.minimum(1, t / 0.01) * np.exp(-t / 0.08)
        return nz * vel * 0.5
    body = np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.05) * 0.6
    nz = d.band(rng.standard_normal(n), 1200, 8000) * np.exp(-t / 0.07)
    return (body + nz * 0.8) * vel * 0.7


def rim(vel, rng):
    n = d.n_of(0.08)
    t = _t(n)
    y = (np.sin(2 * np.pi * 480 * t) + 0.6 * np.sin(2 * np.pi * 1250 * t)) * np.exp(-t / 0.012)
    y += d.bp(rng.standard_normal(n), 2500, 2) * np.exp(-t / 0.006) * 0.6
    return y * vel * 0.6


def clap(vel, rng):
    n = d.n_of(0.2)
    t = _t(n)
    e = np.zeros(n)
    for k, off in enumerate((0.0, 0.011, 0.02)):
        e += (t >= off) * np.exp(-np.maximum(t - off, 0) / (0.006 if k < 2 else 0.05))
    return d.band(rng.standard_normal(n), 900, 5000) * e * vel * 0.5


def hat(vel, rng, open_=False):
    n = d.n_of(0.3 if open_ else 0.06)
    t = _t(n)
    y = d.hp4(rng.standard_normal(n), 7000) * np.exp(-t / (0.12 if open_ else 0.018))
    return y * vel * 0.35


def shaker(vel, rng):
    n = d.n_of(0.09)
    t = _t(n)
    y = d.band(rng.standard_normal(n), 4500, 10000) * np.minimum(1, t / 0.012) * np.exp(-t / 0.03)
    return y * vel * 0.4


def tamb(vel, rng):
    n = d.n_of(0.2)
    t = _t(n)
    y = sum(np.sin(2 * np.pi * f * t + rng.uniform(0, 6)) for f in (5200, 6900, 7900, 9300)) * np.exp(-t / 0.05)
    y += d.hp(rng.standard_normal(n), 6000) * np.exp(-t / 0.03)
    return y * vel * 0.18


def ride(vel, rng):
    n = d.n_of(1.2)
    t = _t(n)
    y = sum(np.sin(2 * np.pi * f * t + rng.uniform(0, 6)) * np.exp(-t / dc) for f, dc in
            ((3170, 0.6), (4420, 0.5), (5310, 0.4), (6650, 0.3), (8120, 0.25)))
    y = y * 0.12 + d.hp(rng.standard_normal(n), 5000) * np.exp(-t / 0.2) * 0.25
    return y * vel * 0.3


def conga(vel, rng, pitch=1.0):
    n = d.n_of(0.4)
    t = _t(n)
    f = 200 * pitch * (1 + 0.25 * np.exp(-t / 0.015))
    y = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.18)
    y += d.bp(rng.standard_normal(n), 1800, 1.5) * np.exp(-t / 0.008) * 0.4
    return y * vel * 0.6


def bongo(vel, rng):
    return conga(vel, rng, 1.9) * 0.8


def cajon(vel, rng, slap=False):
    if slap:
        a, b = snare(vel, rng) * 0.6, conga(vel * 0.3, rng, 1.6)
        out = np.zeros(max(len(a), len(b)))
        out[:len(a)] += a
        out[:len(b)] += b
        return out
    n = d.n_of(0.3)
    t = _t(n)
    f = 85 * (1 + 0.4 * np.exp(-t / 0.02))
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.12) * vel * 0.9


def bodhran(vel, rng):
    n = d.n_of(0.35)
    t = _t(n)
    f = 95 * (1 + 0.3 * np.exp(-t / 0.02))
    y = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.12)
    y += d.lp(rng.standard_normal(n), 1500) * np.exp(-t / 0.03) * 0.35
    return y * vel * 0.8


def castanet(vel, rng):
    n = d.n_of(0.05)
    t = _t(n)
    y = (np.sin(2 * np.pi * 2300 * t) + np.sin(2 * np.pi * 3600 * t)) * np.exp(-t / 0.006)
    return y * vel * 0.25


def woodblock(vel, rng):
    n = d.n_of(0.1)
    t = _t(n)
    return (np.sin(2 * np.pi * 850 * t) + 0.4 * np.sin(2 * np.pi * 2300 * t)) * np.exp(-t / 0.02) * vel * 0.4


def triangle(vel, rng):
    n = d.n_of(1.0)
    t = _t(n)
    y = sum(np.sin(2 * np.pi * f * t) * a for f, a in ((2600, 1), (7800, 0.4), (5200, 0.2))) * np.exp(-t / 0.4)
    return y * vel * 0.12


def chime_gliss(vel, rng, up=True):
    """A bar-chime sweep."""
    n = d.n_of(1.6)
    y = np.zeros(n)
    fs = np.geomspace(2000, 6500, 12)
    if not up:
        fs = fs[::-1]
    for k, f in enumerate(fs):
        i = d.n_of(k * 0.035)
        m = n - i
        t = _t(m)
        y[i:] += np.sin(2 * np.pi * f * t) * np.exp(-t / 0.5) * 0.15
    return y * vel


DRUMS = {"k": kick, "s": snare, "b": lambda v, r: snare(v, r, True), "r": rim, "c": clap, "h": hat,
         "o": lambda v, r: hat(v, r, True), "x": shaker, "t": tamb, "y": ride, "g": conga, "n": bongo,
         "j": cajon, "J": lambda v, r: cajon(v, r, True), "d": bodhran, "a": castanet, "w": woodblock, "i": triangle,
         "G": chime_gliss}


# ---- arrangement ---------------------------------------------------------------------------------------------------------------

class Piece:
    """A piece being written: tracks of note events, rendered and mixed at the end."""

    def __init__(self, name, tempo, beats_per_bar=4, swing=0.0, seed=None):
        self.name, self.tempo, self.bpb, self.swing = name, tempo, beats_per_bar, swing
        self.rng = d.rng_of(name, seed)
        self.tracks = {}  # name -> dict(instr, gain, pan, rev, events=[(beat, midi, beats, vel)], kind)

    @property
    def beat(self):
        return 60.0 / self.tempo

    def track(self, name, instr, gain=1.0, pan=0.0, rev=0.2, kind="pitched", width=0.0, hp=0, lp=0):
        if name not in self.tracks:
            self.tracks[name] = dict(instr=instr, gain=gain, pan=pan, rev=rev, kind=kind, width=width, hp=hp, lp=lp,
                                     events=[])
        return self.tracks[name]

    def note(self, track, beat, m, beats, vel=0.8):
        self.tracks[track]["events"].append((beat, m, beats, vel))

    def melody(self, track, at_beat, text, transpose=0, vel=1.0):
        for b, m, ln, v in parse_line(text):
            self.note(track, at_beat + b, m + transpose, ln, v * vel)
        return sum(float(t.split(":")[1]) if ":" in t else 1.0 for t in text.split() if t != "|")

    def drums(self, track, at_beat, bars, pattern, steps=16, vel=1.0, beats=None):
        """pattern: {drum letter: 'x...X...'} per bar ('X' accent, 'x' normal, 'o' ghost, '.' rest)."""
        beats = beats or self.bpb
        for bar in range(bars):
            for letter, pat in pattern.items():
                st = len(pat)
                for k, ch in enumerate(pat):
                    if ch == ".":
                        continue
                    v = {"X": 1.0, "x": 0.75, "o": 0.4}.get(ch, 0.7) * vel
                    self.note(track, at_beat + bar * beats + k * beats / st, letter, 0, v)

    def time_of(self, beat):
        """Seconds of a beat, the off-beat eighths (and sixteenths) pushed late by the swing."""
        b = beat
        if self.swing:
            pos = beat % 1.0
            if abs(pos - 0.5) < 1e-6:
                b += self.swing / 6
            elif abs(pos - 0.25) < 1e-6 or abs(pos - 0.75) < 1e-6:
                b += self.swing / 12
        return b * self.beat

    def render(self, total_beats, loop=False, master_lufs=-18.0, ir_len=2.0, tail=2.5, rev_damp=6000):
        dur = total_beats * self.beat
        n = d.n_of(dur + tail)
        mix = np.zeros((n, 2))
        send = np.zeros((n, 2))
        rng = self.rng
        cache = {}
        for name, tr in self.tracks.items():
            buf = np.zeros(n)
            for (b, m, ln, v) in tr["events"]:
                t0 = self.time_of(b) + rng.normal(0, 0.004 if tr["kind"] != "drums" else 0.003)
                t0 = max(0.0, t0)
                vv = float(np.clip(v * rng.uniform(0.92, 1.05), 0.05, 1.2))
                if tr["kind"] == "drums":
                    s = DRUMS[m](vv, rng)
                else:
                    # the same note again sounds the same (a cache keeps long pieces quick), velocities in steps
                    key = (tr["instr"], m, round(ln * self.beat, 2), round(vv, 1))
                    s = cache.get(key)
                    if s is None:
                        s = INSTR[tr["instr"]](hz(m), max(0.05, ln * self.beat), round(vv, 1), rng) * calibration(tr["instr"])
                        cache[key] = s
                d.put(buf, t0, s)
            if tr["hp"]:
                buf = d.hp(buf, tr["hp"])
            if tr["lp"]:
                buf = d.lp(buf, tr["lp"])
            p = (tr["pan"] + 1) / 2
            gl, gr = np.cos(p * np.pi / 2) * 1.414, np.sin(p * np.pi / 2) * 1.414
            st = np.stack([buf * gl, buf * gr], 1) * tr["gain"]
            if tr["width"]:
                # a little width: a short delayed copy on the far side
                dl = d.n_of(0.012)
                st[dl:, 1] += buf[:-dl] * tr["width"] * tr["gain"] * 0.5
                st[dl:, 0] -= buf[:-dl] * tr["width"] * tr["gain"] * 0.15
            mix += st
            send += st * tr["rev"]
            if STATS is not None:
                STATS[name] = round(d.lufs(st), 1) if np.any(st) else None
        ir = d.impulse_response(ir_len, d.rng_of(self.name, "ir"), damp=rev_damp)
        wet = d.convolve(send, ir)[:n]
        mix += wet * 0.9
        # glue and tone: gentle bus compression, a touch of air, a clean low end
        mix = np.stack([d.hp(mix[:, c], 30) for c in range(2)], 1)
        mix = np.stack([d.shelf_hi(mix[:, c], 6000, 1.5) for c in range(2)], 1)
        mix = d.compress(mix, thresh_db=-20, ratio=2.0, attack=0.01, release=0.2)
        if loop:
            m = d.n_of(dur)
            out = mix[:m].copy()
            rest = mix[m:]
            out[:len(rest)] += rest[:m]
            mix = out
        else:
            mix = mix[:n]
            # trim trailing silence
            lvl = np.max(np.abs(mix), 1)
            nz = np.nonzero(lvl > 1e-4)[0]
            end = (nz[-1] + d.n_of(0.1)) if len(nz) else n
            mix = d.fade(mix[:min(n, end)], 0, 0.3)
        return d.master(mix, master_lufs, "integrated", ceiling=-1.0)


# ---- helpers for writing parts ------------------------------------------------------------------------------------------------

def comp(p, track, at, chords, beats_each, rhythm=((0, 1.5), (1.5, 0.5), (2.5, 1.5)), low=55, high=74, vel=0.65):
    """Chords played on a rhythm of (beat offset, length) per chord."""
    for k, sym in enumerate(chords):
        b0 = at + k * beats_each
        for off, ln in rhythm:
            if off >= beats_each:
                continue
            for m in voicing(sym, low, high):
                p.note(track, b0 + off, m, ln, vel * (1.0 if off == 0 else 0.85))


def bassline(p, track, at, chords, beats_each, pattern=((0, 1.5, 0), (1.5, 0.5, 7), (2, 1.5, 0), (3.5, 0.5, 12)),
             octave=36, vel=0.85):
    """A bass line: (beat offset, length, interval above the chord's bass) per chord."""
    for k, sym in enumerate(chords):
        root, q, bass_pc = chord(sym)
        base = octave + bass_pc
        for off, ln, iv in pattern:
            if off >= beats_each:
                continue
            ivv = iv
            if iv == 7 and 7 not in q:
                ivv = q[2] if len(q) > 2 else 7
            p.note(track, at + k * beats_each + off, base + ivv, ln, vel)


def arp(p, track, at, chords, beats_each, step=0.5, order=(0, 1, 2, 3, 2, 1), low=60, high=84, vel=0.55, ln=None):
    for k, sym in enumerate(chords):
        notes = voicing(sym, low, high)
        root, q, _ = chord(sym)
        r = low + ((root - low) % 12)
        notes = sorted(set([r] + notes))
        b = 0.0
        i = 0
        while b < beats_each - 1e-6:
            m = notes[order[i % len(order)] % len(notes)]
            p.note(track, at + k * beats_each + b, m, ln or step * 1.5, vel)
            b += step
            i += 1


def pads(p, track, at, chords, beats_each, low=52, high=72, vel=0.6):
    for k, sym in enumerate(chords):
        for m in voicing(sym, low, high):
            p.note(track, at + k * beats_each, m, beats_each, vel)

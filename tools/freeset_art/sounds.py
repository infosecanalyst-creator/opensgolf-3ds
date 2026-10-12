#!/usr/bin/env python3
"""The free set's sounds and music: every sound file the game plays (crates/sg-core/src/freeset/manifest.txt, W lines),
made from CC0 recordings (Kenney's Impact Sounds, RPG Audio and Music Jingles; the uisfx interface sounds) and our own
synthesis (applause, water, wind, birds, animals, swishes) and little generated pieces of music, levelled to one
loudness and written as Ogg Vorbis to packs/opensgolf-free/art/Sounds/ (the engine takes an OGG for any WAV).

usage: sounds.py SOURCES_DIR [--only REGEX]
SOURCES_DIR holds the CC0 packs as fetch_sources.py lays them out (kenney/sounds/..., uisfx/...).
needs: numpy, ffmpeg (with libvorbis) on the PATH"""
import argparse
import hashlib
import os
import re
import subprocess
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layout  # noqa: E402

ROOT = layout.ROOT
OUT = os.path.join(ROOT, "packs", "opensgolf-free", "art")
SR = 22050

SRC = None


def seed(s):
    return int(hashlib.md5(s.lower().encode()).hexdigest()[:8], 16)


# ---- input and output ----------------------------------------------------------------------------------------------------

def load(rel, sr=SR):
    """A CC0 recording from the sources folder as mono float samples."""
    path = os.path.join(SRC, rel)
    raw = subprocess.run(["ffmpeg", "-v", "quiet", "-i", path, "-ac", "1", "-ar", str(sr), "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(np.float64)


def kenney(name):
    for d in ("sounds/impact", "sounds/rpg", "sounds/jingles/8-Bit jingles", "sounds/jingles/Hit jingles",
              "sounds/jingles/Pizzicato jingles", "sounds/jingles/Sax jingles", "sounds/jingles/Steel jingles"):
        p = os.path.join("kenney", d, name + ".ogg")
        if os.path.exists(os.path.join(SRC, p)):
            return load(p)
    raise FileNotFoundError(name)


def ui(name, flavour="soft"):
    return load(os.path.join("uisfx", flavour, name + ".ogg"))


def write_ogg(path, x, sr=SR, stereo=None, quality=2):
    x = np.asarray(x, np.float64)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    data = x.astype(np.float32)
    ch = 1
    if stereo is not None:
        data = np.stack([x, stereo], 1).astype(np.float32).reshape(-1)
        ch = 2
    subprocess.run(["ffmpeg", "-v", "quiet", "-y", "-f", "f32le", "-ar", str(sr), "-ac", str(ch), "-i", "-",
                    "-c:a", "libvorbis", "-q:a", str(quality), path], input=data.tobytes(), check=True)


def level(x, target_db=-18.0, peak=0.89):
    """Brings a sound to a common loudness (RMS over its loud part) without clipping."""
    x = np.asarray(x, np.float64)
    if not len(x) or np.max(np.abs(x)) < 1e-9:
        return x
    win = max(1, int(SR * 0.05))
    p = np.convolve(x * x, np.ones(win) / win, mode="same")
    loud = p[p > p.max() * 0.1]
    rms = np.sqrt(loud.mean()) if len(loud) else np.sqrt(p.mean())
    g = 10 ** (target_db / 20) / max(rms, 1e-9)
    y = x * g
    m = np.max(np.abs(y))
    if m > peak:
        y *= peak / m
    return y


def fit(x, secs, fade=0.05):
    """Pads or trims to about `secs` (never cutting a sound short by more than needed), with a short fade out."""
    n = int(secs * SR)
    if len(x) < n:
        x = np.r_[x, np.zeros(n - len(x))]
    else:
        x = x[:n].copy()
    f = min(int(fade * SR), len(x))
    if f:
        x[-f:] *= np.linspace(1, 0, f)
    return x


def mix(n, *parts):
    """Adds (offset seconds, samples, gain) parts into n samples."""
    out = np.zeros(n)
    for off, s, g in parts:
        i = int(off * SR)
        if i >= n:
            continue
        m = min(len(s), n - i)
        out[i:i + m] += s[:m] * g
    return out


# ---- synthesis -------------------------------------------------------------------------------------------------------------

def t_axis(secs):
    return np.arange(int(secs * SR)) / SR


def env(n, a=0.01, d=0.3, curve=4.0):
    t = np.arange(n) / SR
    e = np.minimum(1, t / max(a, 1e-4)) * np.exp(-np.maximum(t - a, 0) / max(d, 1e-4) * (curve / 4))
    return e


def lowpass(x, cutoff):
    """One-pole low pass (cutoff Hz, scalar or per sample)."""
    c = np.broadcast_to(np.asarray(cutoff, float), x.shape)
    a = 1 - np.exp(-2 * np.pi * c / SR)
    y = np.zeros_like(x)
    acc = 0.0
    for i in range(len(x)):
        acc += a[i] * (x[i] - acc)
        y[i] = acc
    return y


def lp_fast(x, cutoff, passes=2):
    """A fixed low pass made of moving averages (fast for long sounds)."""
    k = max(1, int(SR / max(cutoff, 1) / 2))
    y = x
    for _ in range(passes):
        y = np.convolve(y, np.ones(k) / k, mode="same")
    return y


def bandpass(x, lo, hi):
    return lp_fast(x, hi) - lp_fast(x, lo)


def noise(n, rng):
    return rng.standard_normal(n)


def reverb(x, wet=0.25, size=1.0, rng=None):
    """A small Schroeder reverb (four combs and two all-passes)."""
    out = np.zeros(len(x) + int(SR * 1.2 * size))
    xx = np.r_[x, np.zeros(len(out) - len(x))]
    acc = np.zeros_like(out)
    for d, g in ((1557, 0.8), (1617, 0.79), (1491, 0.78), (1422, 0.77)):
        d = int(d * size * SR / 44100)
        y = xx.copy()
        for i in range(d, len(y), d):
            seg = y[i - d:min(i, len(y) - d)]
            y[i:i + len(seg)] += seg * g
        acc += y
    acc /= 4
    for d, g in ((225, 0.5), (556, 0.5)):
        d = int(d * size * SR / 44100)
        y = np.zeros_like(acc)
        for i in range(len(acc)):
            yd = y[i - d] if i >= d else 0.0
            xd = acc[i - d] if i >= d else 0.0
            y[i] = -g * acc[i] + xd + g * yd
        acc = y
    return xx * (1 - wet) + acc * wet


def splash(rng, secs=1.1, big=1.0):
    n = int(secs * SR)
    t = np.arange(n) / SR
    burst = noise(n, rng) * np.exp(-t / (0.12 * big)) * np.minimum(1, t / 0.004)
    body = lp_fast(burst, 2500) * 0.9 + bandpass(burst, 800, 5000) * 0.5
    # bubbles: little rising sine blips
    for k in range(int(10 * big)):
        st = rng.uniform(0.04, secs * 0.7)
        f0 = rng.uniform(600, 1600)
        ln = rng.uniform(0.02, 0.06)
        m = int(ln * SR)
        tt = np.arange(m) / SR
        blip = np.sin(2 * np.pi * (f0 * tt + 2000 * tt * tt / ln)) * np.exp(-tt / (ln / 3))
        i = int(st * SR)
        body[i:i + m] += blip[:max(0, min(m, n - i))] * 0.25
    return body


def swish(rng, secs=0.35, pitch=1.0):
    n = int(secs * SR)
    t = np.arange(n) / SR
    e = np.sin(np.pi * np.clip(t / secs, 0, 1)) ** 2
    nz = noise(n, rng)
    return bandpass(nz, 500 * pitch, 3500 * pitch) * e


def claps(rng, secs, rate=40.0, swell=0.4, fade=1.5):
    """Applause: many short hand claps, swelling and dying away."""
    n = int(secs * SR)
    out = np.zeros(n)
    count = int(rate * secs)
    for k in range(count):
        st = rng.uniform(0, secs) ** 1.15 / secs ** 0.15
        i = int(st * SR)
        m = int(SR * 0.012)
        c = noise(m, rng) * np.exp(-np.arange(m) / (SR * 0.0025))
        c = bandpass(c, rng.uniform(600, 1100), rng.uniform(2600, 4200)) if m > 20 else c
        g = rng.uniform(0.3, 1.0)
        out[i:i + m] += c[:max(0, min(m, n - i))] * g
    t = np.arange(n) / SR
    e = np.minimum(1, t / swell) * np.clip((secs - t) / fade, 0, 1)
    return out * e


def crowd(rng, secs, vowel="aa", rise=0.0):
    """A crowd's 'ooh' or 'aww': noise through two vowel formants, gliding."""
    n = int(secs * SR)
    t = np.arange(n) / SR
    f1, f2 = {"oo": (350, 800), "aa": (700, 1150), "ah": (750, 1200)}[vowel]
    src = noise(n, rng)
    glide = 1 + rise * (t / secs - 0.3)
    y = bandpass(src, f1 * 0.8 * glide.mean(), f1 * 1.25 * glide.mean()) + 0.5 * bandpass(src, f2 * 0.85, f2 * 1.2)
    e = np.sin(np.pi * np.clip(t / secs, 0, 1)) ** 1.5
    return y * e


def chirp(f0, f1, secs, rng=None, harm=(1.0, 0.25)):
    n = int(secs * SR)
    t = np.arange(n) / SR
    f = f0 + (f1 - f0) * t / secs
    ph = 2 * np.pi * np.cumsum(f) / SR
    y = sum(a * np.sin(ph * (k + 1)) for k, a in enumerate(harm))
    return y * np.sin(np.pi * t / secs) ** 2


def birds(rng, secs, density=1.0):
    n = int(secs * SR)
    out = np.zeros(n)
    k = 0
    tt = rng.uniform(0.2, 1.0)
    while tt < secs - 0.4:
        f = rng.uniform(2400, 4200)
        song = rng.integers(2, 5)
        for j in range(song):
            d = rng.uniform(0.05, 0.12)
            c = chirp(f * rng.uniform(0.9, 1.2), f * rng.uniform(0.7, 1.4), d)
            i = int((tt + j * (d + 0.03)) * SR)
            out[i:i + len(c)] += c[:max(0, min(len(c), n - i))] * rng.uniform(0.15, 0.35)
        tt += rng.uniform(0.8, 2.5) / density
        k += 1
    return out


def wind(rng, secs, level_=0.4):
    n = int(secs * SR)
    nz = lp_fast(noise(n, rng), 500, 3)
    t = np.arange(n) / SR
    m = 0.6 + 0.4 * np.sin(2 * np.pi * t / rng.uniform(4, 7) + rng.uniform(0, 6))
    return nz * m * level_


def water_lap(rng, secs):
    n = int(secs * SR)
    out = lp_fast(noise(n, rng), 900, 2) * 0.3
    tt = 0.3
    while tt < secs - 0.3:
        i = int(tt * SR)
        m = int(rng.uniform(0.15, 0.35) * SR)
        b = bandpass(noise(m, rng), 300, 1800) * np.sin(np.pi * np.arange(m) / m) ** 2
        out[i:i + m] += b[:max(0, min(m, n - i))] * rng.uniform(0.3, 0.6)
        tt += rng.uniform(0.6, 1.6)
    return out


def tone(f, secs, harm=(1.0,), decay=0.5, attack=0.005):
    n = int(secs * SR)
    t = np.arange(n) / SR
    y = sum(a * np.sin(2 * np.pi * f * (k + 1) * t) for k, a in enumerate(harm))
    return y * np.minimum(1, t / attack) * np.exp(-t / decay)


def pluck(f, secs, bright=0.5, rng=None):
    """A Karplus-Strong plucked string, a period at a time."""
    rng = rng or np.random.default_rng(int(f))
    n = int(secs * SR)
    p = max(2, int(round(SR / f)))
    block = rng.uniform(-1, 1, p)
    blocks = []
    decay = 0.996 - (1 - bright) * 0.006
    for _ in range(n // p + 1):
        blocks.append(block)
        block = (block * (0.5 + bright * 0.25) + np.roll(block, -1) * (0.5 - bright * 0.25)) * decay
    out = np.concatenate(blocks)[:n]
    t = np.arange(n) / SR
    return out * 0.6 * np.clip((secs - t) / 0.05, 0, 1)


def note_freq(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


# ---- music -----------------------------------------------------------------------------------------------------------------

def instrument(kind, f, secs, rng):
    if kind == "pluck":
        return pluck(f, secs, 0.6, rng)
    if kind == "harp":
        return pluck(f, secs, 0.9, rng) * 0.9
    if kind == "steel":
        # a steel pan: inharmonic bell partials
        return tone(f, secs, (1.0, 0.0, 0.45, 0.0, 0.25), decay=0.55) + tone(f * 2.76, secs, (0.15,), decay=0.2)
    if kind == "piano":
        return tone(f, secs, (1.0, 0.45, 0.2, 0.1), decay=0.9, attack=0.004)
    if kind == "pad":
        n = int(secs * SR)
        t = np.arange(n) / SR
        y = sum(np.sin(2 * np.pi * f * d * t) for d in (1.0, 1.003, 0.997)) / 3 + 0.2 * np.sin(4 * np.pi * f * t)
        return y * np.minimum(1, t / 0.4) * np.clip((secs - t) / 0.5, 0, 1)
    if kind == "flute":
        n = int(secs * SR)
        t = np.arange(n) / SR
        vib = 1 + 0.004 * np.sin(2 * np.pi * 5.5 * t)
        y = np.sin(2 * np.pi * np.cumsum(f * vib) / SR) + 0.05 * noise(n, rng)
        return y * np.minimum(1, t / 0.08) * np.clip((secs - t) / 0.12, 0, 1) * 0.7
    if kind == "reed":
        # a bagpipe-like reed: a buzzy saw, softened
        n = int(secs * SR)
        t = np.arange(n) / SR
        saw = 2 * ((f * t) % 1) - 1
        return lp_fast(saw, 2200) * np.minimum(1, t / 0.05) * np.clip((secs - t) / 0.08, 0, 1) * 0.6
    return tone(f, secs)


SCALES = {"major": [0, 2, 4, 5, 7, 9, 11], "minor": [0, 2, 3, 5, 7, 8, 10], "dorian": [0, 2, 3, 5, 7, 9, 10]}


def compose(name, secs, style):
    """A short generated piece: a chord progression with a bass and an arpeggio, a simple melody on top."""
    rng = np.random.default_rng(seed(name))
    tempo = style.get("tempo", 96)
    beat = 60.0 / tempo
    key = style.get("key", 60)
    scale = SCALES[style.get("scale", "major")]
    prog = style.get("prog", [0, 5, 3, 4])
    lead, comp, bass = style.get("lead", "piano"), style.get("comp", "pluck"), style.get("bassi", "piano")
    n = int(secs * SR)
    out = np.zeros(n + SR * 2)
    bar = beat * 4
    bars = max(1, int(secs / bar))
    melody_prev = 4
    for b in range(bars):
        deg = prog[b % len(prog)]
        root = key + scale[deg % 7] + 12 * (deg // 7)
        chord = [root + scale[(deg + k) % 7] - scale[deg % 7] + (12 if (deg + k) % 7 < deg % 7 else 0) for k in (0, 2, 4)]
        t0 = b * bar
        # bass on beats 1 and 3
        for k in (0, 2):
            s = instrument(bass, note_freq(root - 24), beat * 1.8, rng) * 0.5
            i = int((t0 + k * beat) * SR)
            out[i:i + len(s)] += s[:len(out) - i]
        # arpeggio in eighths
        for k in range(8):
            m = chord[[0, 1, 2, 1, 0, 2, 1, 2][k]] - (12 if style.get("low_arp") else 0)
            s = instrument(comp, note_freq(m), beat * 1.2, rng) * 0.22
            i = int((t0 + k * beat / 2) * SR)
            out[i:i + len(s)] += s[:len(out) - i]
        # a melody: a few notes per bar near the last, on chord tones and steps
        if style.get("melody", True):
            k = 0.0
            while k < 4:
                dur = rng.choice([0.5, 1.0, 1.0, 1.5, 2.0])
                step = int(rng.choice([-2, -1, 0, 1, 1, 2]))
                melody_prev = int(np.clip(melody_prev + step, 0, 9))
                m = key + 12 + scale[melody_prev % 7] + 12 * (melody_prev // 7)
                if rng.uniform() < 0.85:
                    s = instrument(lead, note_freq(m), beat * dur * 1.1, rng) * 0.3
                    i = int((t0 + k * beat) * SR)
                    out[i:i + len(s)] += s[:len(out) - i]
                k += dur
    out = out[:n]
    f = min(n, int(SR * 1.5))
    out[-f:] *= np.linspace(1, 0, f)
    return reverb(out, wet=0.22, size=1.0)[:n]


MUSIC = {
    "buy1short": dict(tempo=100, key=62, prog=[0, 3, 4, 0], lead="piano", comp="pluck"),
    "buy2short": dict(tempo=92, key=60, prog=[0, 5, 1, 4], lead="flute", comp="pluck"),
    "buy3short": dict(tempo=108, key=65, prog=[0, 4, 5, 3], lead="steel", comp="pluck"),
    "buy4short": dict(tempo=84, key=57, scale="minor", prog=[0, 5, 6, 4], lead="piano", comp="harp"),
    "golf harp 6": dict(tempo=72, key=60, prog=[0, 5, 3, 4], lead="harp", comp="harp", melody=False),
    "golf harp 7": dict(tempo=76, key=62, prog=[0, 3, 5, 4], lead="harp", comp="harp"),
    "manifold music": dict(tempo=88, key=55, prog=[0, 4, 5, 3], lead="piano", comp="pluck"),
    "music idea": dict(tempo=96, key=60, scale="dorian", prog=[0, 3, 0, 4], lead="flute", comp="pluck"),
    "new age": dict(tempo=66, key=57, prog=[0, 5, 3, 4], lead="pad", comp="harp", low_arp=True),
    "newhart-theme": dict(tempo=110, key=64, prog=[0, 3, 4, 0], lead="piano", comp="pluck"),
    "romeo and juliet edit": dict(tempo=70, key=57, scale="minor", prog=[0, 3, 4, 0], lead="flute", comp="harp"),
    "sail boat": dict(tempo=90, key=62, prog=[0, 4, 3, 4], lead="steel", comp="pluck"),
    "steel drum short": dict(tempo=104, key=60, prog=[0, 3, 4, 0], lead="steel", comp="steel"),
    "bagpipe": dict(tempo=84, key=62, scale="dorian", prog=[0, 6, 0, 4], lead="reed", comp="reed", bassi="reed"),
    "spanish guitar wflute 2": dict(tempo=92, key=57, scale="minor", prog=[0, 6, 5, 4], lead="flute", comp="pluck",
                                    bassi="pluck"),
    "steel drums 2": dict(tempo=112, key=65, prog=[0, 3, 4, 3], lead="steel", comp="steel"),
}


# ---- the sounds of the game ------------------------------------------------------------------------------------------------

def sm(*xs):
    """The sum of sounds of different lengths."""
    n = max(len(x) for x in xs)
    return sum(np.r_[x, np.zeros(n - len(x))] for x in xs)


def silence(secs=0.1):
    return np.zeros(int(secs * SR))


def cat(*xs, gap=0.0):
    parts = []
    for x in xs:
        parts.append(x)
        if gap:
            parts.append(np.zeros(int(gap * SR)))
    return np.concatenate(parts)


def make(path, secs):
    """The samples of one sound file of the game (`path` as the manifest has it, under Sounds/), and its loudness target
    (dB); None for music handled by compose."""
    name = path.rsplit("/", 1)[-1][:-4].lower()
    low = path.lower()
    rng = np.random.default_rng(seed(path))
    k = kenney
    if name in MUSIC:
        return compose(name, secs, MUSIC[name]), -20
    if "simsfx/" in low or "/celebs/" in low and not any(w in name for w in ("grill", "jacuzzi", "harley", "bmw", "ferari",
                                                                                  "lamborgini", "classic", "punching", "arcade",
                                                                                  "basketball")):
        # people's voices: the free set keeps them quiet (no gibberish), a soft murmur only for the chattier lines
        return silence(min(secs, 0.3)), -40
    if low.startswith("sounds/male/") or low.startswith("sounds/female/"):
        return silence(0.2), -40
    if "/emotion/" in low:
        if "happy" in name or "success" in name:
            return k("jingles_PIZZI0%d" % (seed(name) % 3)) * 1.0, -26
        if "sad" in name or "failure" in name:
            return k("jingles_PIZZI1%d" % (seed(name) % 3 + 3)), -28
        return silence(0.2), -40
    if "/celebs/" in low:
        if "grill" in name:
            return lp_fast(noise(int(2.6 * SR), rng), 4000) * (0.6 + 0.4 * rng.random(int(2.6 * SR))) * 0.3, -26
        if "jacuzzi" in name:
            return splash(rng, 4.0, 0.6) * 0.4 + bandpass(noise(int(4 * SR), rng), 200, 900) * 0.4, -26
        if any(w in name for w in ("harley", "bmw", "ferari", "lamborgini", "classic")):
            t = t_axis(min(secs, 1.8))
            f = 45 + 25 * np.minimum(1, t / 0.6)
            eng = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * 0.4 + 0.3 * noise(len(t), rng)
            return lp_fast(eng, 900) * np.minimum(1, t / 0.1) * np.clip((t[-1] - t) / 0.4, 0, 1), -22
        if "punching" in name:
            return cat(k("impactPunch_medium_000"), silence(0.25), k("impactPunch_heavy_001"), silence(0.3), k("impactPunch_medium_002")), -20
        if "arcade" in name:
            return k("jingles_NES0%d" % (seed(name) % 9)), -24
        if "basketball" in name:
            return cat(k("impactSoft_heavy_000"), silence(0.3), k("impactSoft_heavy_002")), -22
        return silence(0.2), -40
    # ---- golf
    if "drive with ball" in name or name == "drive" or "tiger swing" in name:
        hit = sm(k("impactPlate_light_00%d" % (seed(name) % 5)) * 0.9, k("impactGeneric_light_001") * 0.5)
        return mix(int(0.6 * SR), (0.0, swish(rng, 0.28, 1.2), 0.5), (0.2, hit, 1.0)), -16
    if "golf swing" in name:
        return mix(int(0.5 * SR), (0.0, swish(rng, 0.4, 1.0), 1.0)), -22
    if name == "iron":
        return mix(int(0.5 * SR), (0.0, swish(rng, 0.25, 1.1), 0.4), (0.18, sm(k("impactMetal_light_002") * 0.6, k("impactPlate_light_001")), 1.0)), -17
    if name == "chip":
        return mix(int(0.5 * SR), (0.0, swish(rng, 0.2, 1.3), 0.3), (0.12, k("impactGeneric_light_003"), 1.0)), -19
    if name == "putt":
        return k("impactPlate_light_004") * 0.7, -22
    if "ball in hole" in name:
        return mix(int(0.9 * SR), (0.0, k("impactPlate_light_002"), 0.8), (0.08, k("impactMetal_light_001"), 0.5),
                   (0.2, k("impactMetal_light_003"), 0.3)), -18
    if "ball drop sand" in name:
        return k("footstep_snow_002") * 1.0, -22
    if "ball drop fairway" in name:
        base = mix(int(0.8 * SR), (0.0, k("impactSoft_medium_001"), 0.8), (0.28, k("impactSoft_medium_003"), 0.35))
        if "lucky" in name:
            base = mix(int(1.6 * SR), (0.0, base, 1.0), (0.4, k("jingles_PIZZI02"), 0.4))
        return base, -21
    if "ball tree leaves" in name:
        return mix(int(1.6 * SR), (0.0, bandpass(noise(int(1.2 * SR), rng), 1500, 7000) * env(int(1.2 * SR), 0.05, 0.3), 0.6),
                   (0.05, k("impactWood_light_002"), 1.0)), -20
    if "ball tree" in name:
        return k("impactWood_light_00%d" % (seed(name) % 5)), -19
    if "ball water" in name:
        return splash(rng, 1.1, 0.7), -18
    if "tiger bounce" in name:
        return mix(int(0.5 * SR), (0.0, k("impactSoft_medium_002"), 1.0), (0.22, k("impactSoft_medium_004"), 0.4)), -20
    if "cart away" in name:
        t = t_axis(3.6)
        whine = np.sin(2 * np.pi * np.cumsum(300 + 120 * np.minimum(1, t / 1.0)) / SR) * 0.3
        rumble = lp_fast(noise(len(t), rng), 300) * 0.8
        return (whine + rumble) * np.minimum(1, t / 0.2) * np.clip((3.6 - t) / 2.5, 0, 1), -24
    if "open soda" in name:
        return mix(int(2.0 * SR), (0.0, k("metalClick"), 0.8), (0.05, bandpass(noise(int(1.6 * SR), rng), 3000, 9000)
                                                                  * env(int(1.6 * SR), 0.01, 0.6), 0.5)), -22
    if "put flag back" in name:
        return k("metalLatch"), -22
    if "grunt" in name:
        return silence(0.2), -40
    if name.startswith("applause"):
        if "bad" in name:
            return mix(int(3.5 * SR), (0.0, claps(rng, 3.5, rate=14, swell=0.3, fade=1.2), 0.7), (0.0, crowd(rng, 1.6, "aa", -0.3), 0.5)), -22
        return claps(rng, secs, rate=70 if "good" in name else 45, swell=0.35, fade=2.0), -20
    if "ambience" in name:
        s = min(secs, 16.0)
        x = wind(rng, s, 0.25) + birds(rng, s, 1.0)
        if "wtr" in name:
            x += water_lap(rng, s) * 0.6
        return x, -28
    # ---- interface
    if name in ("button1", "button2", "button3"):
        return ui({"button1": "press", "button2": "select", "button3": "toggle-on"}[name]), -22
    if name == "check box":
        return ui("check"), -22
    if name == "text box":
        return ui("typing")[:int(0.06 * SR)], -26
    if name in ("unavailable", "wrong"):
        return ui("error" if name == "wrong" else "blocked"), -22
    if name == "slide pop up":
        return ui("open"), -22
    if name in ("bass up 2", "violin up 2"):
        return ui("level-up" if "violin" in name else "success"), -22
    if name in ("bass down 2", "violin down 2"):
        return ui("back" if "bass" in name else "cancel"), -22
    if name.startswith("test") or name == "old test 7":
        return k(["jingles_STEEL00", "jingles_SAX02", "jingles_PIZZI04", "jingles_STEEL05", "jingles_SAX07", "jingles_HIT03"][seed(name) % 6]), -22
    if name == "building":
        return mix(int(0.6 * SR), (0.0, k("impactWood_medium_001"), 1.0), (0.12, k("impactWood_light_003"), 0.5)), -20
    if name == "bench":
        return k("impactPlank_medium_002"), -20
    if name == "bridge":
        return mix(int(0.8 * SR), (0.0, k("impactWood_heavy_001"), 0.8), (0.15, k("impactPlank_medium_000"), 0.6)), -20
    if name == "path":
        return k("footstep_concrete_001"), -21
    if name.startswith("place bunker"):
        return k("footstep_snow_00%d" % (seed(name) % 5)), -21
    if name in ("place fairway", "place greentee", "place rough"):
        return k("footstep_grass_00%d" % (seed(name) % 5)), -21
    if name == "place water" or name == "place waterrocks":
        x = splash(rng, 1.0, 0.5)
        if "rocks" in name:
            x = mix(len(x), (0.0, x, 1.0), (0.1, k("impactMining_001"), 0.5))
        return x, -21
    if name.startswith("place rocks"):
        return mix(int(1.2 * SR), (0.0, k("impactMining_002"), 1.0), (0.18, k("impactMining_004"), 0.6)), -20
    # ---- effects
    if name == "cash":
        return mix(int(1.2 * SR), (0.0, k("handleCoins"), 1.0), (0.05, ui("purchase"), 0.6)), -18
    if name.startswith("tada"):
        return k(["jingles_SAX00", "jingles_STEEL02", "jingles_HIT00"][seed(name) % 3]), -20
    if name == "twinkle":
        return ui("achievement"), -22
    if name == "camera":
        return mix(int(0.5 * SR), (0.0, k("metalClick"), 1.0), (0.08, k("metalLatch") * 0.5, 0.6)), -22
    if name == "boing":
        t = t_axis(0.5)
        return np.sin(2 * np.pi * np.cumsum(180 + 220 * np.exp(-t * 6) * np.cos(t * 50)) / SR) * env(len(t), 0.005, 0.25), -22
    if name == "belch":
        return silence(0.2), -40
    if name.startswith("club 2 drop"):
        return mix(int(1.4 * SR), (0.0, k("impactMetal_light_000"), 1.0), (0.22, k("impactMetal_light_004"), 0.5),
                   (0.4, k("impactSoft_medium_000"), 0.3)), -21
    if name in ("desert1", "desert2", "desert3"):
        d = max(secs, 1.0)
        return wind(rng, d, 0.5) + (birds(rng, d, 0.5) * 0.5 if name != "desert1" else 0), -26
    if name in ("grass3", "spring"):
        return wind(rng, secs, 0.2) + birds(rng, secs, 1.5), -26
    if name == "winter":
        return wind(rng, secs, 0.7), -26
    if name == "lake1":
        return water_lap(rng, secs) + birds(rng, secs, 0.6) * 0.5, -26
    if name.startswith("alligator"):
        t = t_axis(1.5)
        return lp_fast(noise(len(t), rng), 220) * np.sin(np.pi * t / 1.5) ** 2 * (1 + 0.5 * np.sin(2 * np.pi * 18 * t)), -24
    if name == "dolphin":
        return cat(chirp(3000, 5200, 0.25), silence(0.06), chirp(4200, 2600, 0.2), silence(0.05), chirp(3200, 5600, 0.3)), -24
    if name == "fly":
        t = t_axis(2.5)
        return np.sign(np.sin(2 * np.pi * np.cumsum(190 + 30 * np.sin(2 * np.pi * 1.5 * t)) / SR)) * 0.2 * np.sin(np.pi * t / 2.5), -30
    if name.startswith("gila"):
        t = t_axis(1.2)
        return bandpass(noise(len(t), rng), 2000, 7000) * np.sin(np.pi * t / 1.2) ** 3, -27
    if name == "snake":
        t = t_axis(1.8)
        return bandpass(noise(len(t), rng), 3000, 9000) * np.minimum(1, t / 0.2) * np.clip((1.8 - t) / 0.5, 0, 1), -27
    if name.startswith("moose"):
        t = t_axis(1.5)
        f = 140 - 40 * t
        return lp_fast(np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)), 700) * np.sin(np.pi * t / 1.5), -24
    if name.startswith("sheep"):
        t = t_axis(0.9)
        f = 260 * (1 + 0.06 * np.sin(2 * np.pi * 9 * t))
        src = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR))
        return (bandpass(src, 600, 1100) + 0.6 * bandpass(src, 1500, 2300)) * np.sin(np.pi * t / 0.9), -24
    if name.startswith("seriama"):
        return cat(*[chirp(1800 + 200 * j, 1400 + 150 * j, 0.12, harm=(1.0, 0.4, 0.2)) for j in range(6)], gap=0.05), -24
    if name.startswith("snore"):
        t = t_axis(3.5)
        return lp_fast(noise(len(t), rng), 400) * (np.sin(np.pi * t / 1.75) ** 2), -28
    if name.startswith("hot air"):
        t = t_axis(3.4)
        return lp_fast(noise(len(t), rng), 1500) * np.minimum(1, t / 0.15) * np.clip((3.4 - t) / 0.6, 0, 1), -24
    if name in ("flower bed", "maple", "palm", "tree sprout 3"):
        n = int(min(secs, 1.6) * SR)
        rust = bandpass(noise(n, rng), 1200, 6000) * env(n, 0.03, 0.35)
        return mix(n, (0.0, rust, 0.5), (0.02, k("footstep_grass_001"), 0.7)), -22
    if name.startswith("taunt") or name.startswith("unrest"):
        return crowd(rng, 2.5, "aa", 0.2) * 0.6 + claps(rng, 2.5, rate=8, swell=0.5, fade=1.0) * 0.4, -26
    if name.startswith("timetunnel"):
        return swish(rng, 0.9, 0.6), -24
    if name == "shotgun":
        return k("impactPunch_heavy_000"), -22
    return None, -20


def main():
    global SRC
    ap = argparse.ArgumentParser()
    ap.add_argument("sources")
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    SRC = a.sources
    entries = []
    for line in open(layout.MANIFEST, encoding="utf-8"):
        if line.startswith("W "):
            parts = line.split()
            entries.append((" ".join(parts[1:-1]), float(parts[-1])))
    done = 0
    for path, secs in entries:
        if a.only and not re.search(a.only, path, re.I):
            continue
        x, db = make(path, secs)
        if x is None:
            print("no recipe:", path)
            continue
        x = level(x, db)
        music = path.rsplit("/", 1)[-1][:-4].lower() in MUSIC
        out = os.path.join(OUT, path[:path.rfind(".")] + ".ogg")
        write_ogg(out, x, SR, quality=1 if music else 2)
        done += 1
    print(done, "sounds")
    import build
    build.write_index()


if __name__ == "__main__":
    main()

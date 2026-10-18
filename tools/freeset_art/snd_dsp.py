"""The DSP kit of the free set's sounds (sounds.py): filters, envelopes, noise, reverb, a limiter and loudness (ITU-R
BS.1770 K-weighted LUFS), all on numpy float arrays at one sample rate."""
import numpy as np
from scipy import signal

SR = 22050


def secs(n):
    return n / SR


def n_of(t):
    return max(0, int(round(t * SR)))


def t_axis(d):
    return np.arange(n_of(d)) / SR


def zeros(d):
    return np.zeros(n_of(d))


def rng_of(*keys):
    import hashlib
    h = hashlib.md5("|".join(str(k) for k in keys).encode()).hexdigest()
    return np.random.default_rng(int(h[:12], 16))


# ---- filters (RBJ biquads, run with lfilter) ---------------------------------------------------------------------------------

def _biquad(kind, f, q=0.707, gain_db=0.0):
    f = float(np.clip(f, 10, SR * 0.49))
    w = 2 * np.pi * f / SR
    cw, sw = np.cos(w), np.sin(w)
    alpha = sw / (2 * q)
    a_ = 10 ** (gain_db / 40)
    if kind == "lp":
        b = [(1 - cw) / 2, 1 - cw, (1 - cw) / 2]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    elif kind == "hp":
        b = [(1 + cw) / 2, -(1 + cw), (1 + cw) / 2]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    elif kind == "bp":
        b = [alpha, 0, -alpha]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    elif kind == "peak":
        b = [1 + alpha * a_, -2 * cw, 1 - alpha * a_]
        a = [1 + alpha / a_, -2 * cw, 1 - alpha / a_]
    elif kind == "ls":
        sq = 2 * np.sqrt(a_) * alpha
        b = [a_ * ((a_ + 1) - (a_ - 1) * cw + sq), 2 * a_ * ((a_ - 1) - (a_ + 1) * cw), a_ * ((a_ + 1) - (a_ - 1) * cw - sq)]
        a = [(a_ + 1) + (a_ - 1) * cw + sq, -2 * ((a_ - 1) + (a_ + 1) * cw), (a_ + 1) + (a_ - 1) * cw - sq]
    elif kind == "hs":
        sq = 2 * np.sqrt(a_) * alpha
        b = [a_ * ((a_ + 1) + (a_ - 1) * cw + sq), -2 * a_ * ((a_ - 1) + (a_ + 1) * cw), a_ * ((a_ + 1) + (a_ - 1) * cw - sq)]
        a = [(a_ + 1) - (a_ - 1) * cw + sq, 2 * ((a_ - 1) - (a_ + 1) * cw), (a_ + 1) - (a_ - 1) * cw - sq]
    else:
        raise ValueError(kind)
    b, a = np.array(b), np.array(a)
    return b / a[0], a / a[0]


def filt(x, kind, f, q=0.707, gain_db=0.0):
    b, a = _biquad(kind, f, q, gain_db)
    return signal.lfilter(b, a, x)


def lp(x, f, q=0.707):
    return filt(x, "lp", f, q)


def hp(x, f, q=0.707):
    return filt(x, "hp", f, q)


def bp(x, f, q=1.0):
    return filt(x, "bp", f, q)


def lp4(x, f):
    return lp(lp(x, f, 0.54), f, 1.31)


def hp4(x, f):
    return hp(hp(x, f, 0.54), f, 1.31)


def band(x, lo, hi):
    return lp4(hp4(x, lo), hi)


def peak(x, f, db, q=1.0):
    return filt(x, "peak", f, q, db)


def shelf_hi(x, f, db):
    return filt(x, "hs", f, 0.707, db)


def shelf_lo(x, f, db):
    return filt(x, "ls", f, 0.707, db)


def sweep_filter(x, kind, f_curve, q=0.707, block=128):
    """A biquad whose frequency follows `f_curve` (one value per sample), updated every `block` samples."""
    y = np.zeros_like(x)
    zi = np.zeros(2)
    for i in range(0, len(x), block):
        f = float(np.mean(f_curve[i:i + block]))
        b, a = _biquad(kind, f, q)
        # keep the state continuous across coefficient changes (direct form II transposed state)
        seg, zi = signal.lfilter(b, a, x[i:i + block], zi=zi)
        y[i:i + block] = seg
    return y


def resonator(x, f, bw):
    """A two-pole resonator (unity gain at its peak) at f Hz with bandwidth bw Hz."""
    r = np.exp(-np.pi * bw / SR)
    c = 2 * r * np.cos(2 * np.pi * f / SR)
    return signal.lfilter([1 - r], [1, -c, r * r], x)


# ---- sources and envelopes ---------------------------------------------------------------------------------------------------

def noise(n, rng):
    return rng.standard_normal(n)


def pink(n, rng):
    """Pink noise by filtering white noise in the frequency domain."""
    w = rng.standard_normal(n + 1)
    f = np.fft.rfft(w)
    k = np.arange(len(f))
    k[0] = 1
    f /= np.sqrt(k)
    y = np.fft.irfft(f, n + 1)[:n]
    return y / (np.std(y) + 1e-12)


def brown(n, rng):
    y = np.cumsum(rng.standard_normal(n))
    y = hp(y, 20)
    return y / (np.std(y) + 1e-12)


def adsr(n, a=0.005, d=0.1, s=0.7, r=0.1, hold=None):
    """An ADSR envelope over n samples; the release starts at `hold` seconds (default: n - r)."""
    t = np.arange(n) / SR
    total = n / SR
    rel_at = total - r if hold is None else hold
    e = np.where(t < a, t / max(a, 1e-5), s + (1 - s) * np.exp(-(t - a) / max(d, 1e-5)))
    lvl_at_rel = s + (1 - s) * np.exp(-max(rel_at - a, 0) / max(d, 1e-5)) if rel_at > a else rel_at / max(a, 1e-5)
    e = np.where(t >= rel_at, lvl_at_rel * np.clip(1 - (t - rel_at) / max(r, 1e-5), 0, 1), e)
    return e


def perc(n, a=0.002, d=0.2):
    """A percussive envelope: linear attack, exponential decay (d is the time to fall by 1/e)."""
    t = np.arange(n) / SR
    return np.minimum(1, t / max(a, 1e-5)) * np.exp(-np.maximum(t - a, 0) / max(d, 1e-5))


def fade(x, fin=0.0, fout=0.0):
    x = x.copy()
    a, b = n_of(fin), n_of(fout)
    a, b = min(a, len(x)), min(b, len(x))
    sh = (-1,) + (1,) * (x.ndim - 1)
    if a:
        x[:a] *= (np.sin(np.linspace(0, np.pi / 2, a)) ** 2).reshape(sh)
    if b:
        x[-b:] *= (np.cos(np.linspace(0, np.pi / 2, b)) ** 2).reshape(sh)
    return x


def put(out, t, s, g=1.0):
    """Adds s into out at t seconds (clipped to out's length); works for mono and (n, 2) stereo."""
    i = n_of(t)
    if i >= len(out) or i < 0:
        return out
    m = min(len(s), len(out) - i)
    if out.ndim == 2 and s.ndim == 1:
        out[i:i + m] += (s[:m] * g)[:, None]
    else:
        out[i:i + m] += s[:m] * g
    return out


def fit(x, d, fout=0.02):
    """Pads with silence or trims to d seconds, with a short fade where it is cut."""
    n = n_of(d)
    if len(x) >= n:
        y = x[:n].copy()
        return fade(y, 0, min(fout, d / 4)) if len(x) > n else y
    pad = np.zeros((n - len(x),) + x.shape[1:])
    return np.concatenate([x, pad])


def sine(f, d, phase=0.0):
    """A sine of frequency f (scalar or per-sample array) for d seconds."""
    n = n_of(d)
    f = np.broadcast_to(np.asarray(f, float), (n,))
    return np.sin(2 * np.pi * np.cumsum(f) / SR + phase)


def phase_of(f, n):
    f = np.broadcast_to(np.asarray(f, float), (n,))
    return np.cumsum(f) / SR


def saw_bl(f, d, harmonics=None):
    """A band-limited saw (additive, harmonics below Nyquist) for a constant or slowly varying f."""
    n = n_of(d)
    f = np.broadcast_to(np.asarray(f, float), (n,))
    ph = 2 * np.pi * np.cumsum(f) / SR
    top = int(min(harmonics or 64, (SR / 2) / max(f.max(), 1)))
    y = np.zeros(n)
    for k in range(1, max(top, 1) + 1):
        y += np.sin(k * ph) / k * (f * k < SR * 0.45)
    return y * (2 / np.pi)


def modal(freqs, amps, decays, d, rng=None, detune=0.0):
    """A struck body: decaying sine partials (frequency Hz, amplitude, decay seconds)."""
    n = n_of(d)
    t = np.arange(n) / SR
    y = np.zeros(n)
    for f, a, dc in zip(freqs, amps, decays):
        if f >= SR * 0.48:
            continue
        ph = rng.uniform(0, 2 * np.pi) if rng is not None else 0.0
        ff = f * (1 + (rng.uniform(-detune, detune) if rng is not None and detune else 0))
        y += a * np.sin(2 * np.pi * ff * t + ph) * np.exp(-t / dc)
    return y


def click(rng, d=0.004, bright=6000):
    n = n_of(d)
    return lp(noise(n, rng), bright) * perc(n, 0.0002, d / 3)


# ---- space and dynamics ------------------------------------------------------------------------------------------------------

def impulse_response(length=1.6, rng=None, damp=5500, predelay=0.012, early=True, stereo=True):
    """A synthetic room/hall: decaying noise that darkens with time, a few early reflections; (n, 2)."""
    rng = rng or np.random.default_rng(7)
    n = n_of(length)
    t = np.arange(n) / SR
    chans = []
    for c in range(2 if stereo else 1):
        nz = rng.standard_normal(n)
        # darker as it decays: blend a bright and a dark copy
        dark = lp(nz, damp * 0.35)
        bright = lp(nz, damp)
        mixw = np.clip(t / length * 2.2, 0, 1)
        tail = (bright * (1 - mixw) + dark * mixw) * np.exp(-t * 6.9 / length)
        tail *= np.minimum(1, t / 0.02)
        ir = np.zeros(n)
        pd = n_of(predelay)
        ir[pd:] = tail[:n - pd]
        if early:
            for k in range(6):
                i = pd + n_of(rng.uniform(0.003, 0.035))
                if i < n:
                    ir[i] += rng.uniform(0.3, 0.7) * (1 if rng.uniform() < 0.5 else -1) * 3
        chans.append(ir / np.sqrt(np.sum(ir * ir)))
    return np.stack(chans, 1) if stereo else chans[0]


def convolve(x, ir):
    """Mono or stereo x with a mono or stereo IR; returns len(x) + len(ir) - 1 samples."""
    if x.ndim == 1 and ir.ndim == 1:
        return signal.fftconvolve(x, ir)
    if x.ndim == 1:
        x = np.stack([x, x], 1)
    if ir.ndim == 1:
        ir = np.stack([ir, ir], 1)
    return np.stack([signal.fftconvolve(x[:, c], ir[:, c]) for c in range(2)], 1)


def reverb(x, wet=0.2, ir=None, keep_len=True):
    ir = impulse_response() if ir is None else ir
    w = convolve(x, ir)
    if x.ndim == 1 and w.ndim == 2:
        w = w.mean(1)
    if keep_len:
        w = w[:len(x)]
        dry = x
    else:
        dry = np.concatenate([x, np.zeros((len(w) - len(x),) + x.shape[1:])])
    return dry * (1 - wet * 0.3) + w * wet


def softclip(x, drive=1.0):
    return np.tanh(x * drive) / np.tanh(drive)


def compress(x, thresh_db=-18, ratio=3.0, attack=0.005, release=0.12, makeup_db=0.0):
    """A feed-forward compressor on the (stereo-linked) level."""
    lvl = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), 1)
    a_att = np.exp(-1 / (attack * SR))
    a_rel = np.exp(-1 / (release * SR))
    # envelope follower: attack/release one-pole on the level, done with lfilter twice (approximation: max of two)
    env_a = signal.lfilter([1 - a_att], [1, -a_att], lvl)
    env_r = signal.lfilter([1 - a_rel], [1, -a_rel], lvl)
    env = np.maximum(env_a, env_r * 0.95)
    db = 20 * np.log10(env + 1e-9)
    over = np.maximum(db - thresh_db, 0)
    gain_db = -over * (1 - 1 / ratio) + makeup_db
    g = 10 ** (gain_db / 20)
    return x * (g if x.ndim == 1 else g[:, None])


def limit(x, ceiling_db=-1.0, lookahead=0.004, release=0.06):
    """A look-ahead peak limiter: the gain falls before a peak and recovers smoothly; output stays under the ceiling."""
    c = 10 ** (ceiling_db / 20)
    lvl = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), 1)
    need = np.minimum(1, c / np.maximum(lvl, 1e-9))
    la = max(1, n_of(lookahead))
    # minimum of the needed gain over the look-ahead window
    from scipy.ndimage import minimum_filter1d
    g = minimum_filter1d(need, size=2 * la + 1, mode="nearest")
    # smooth recovery
    a = np.exp(-1 / (release * SR))
    sm = np.empty_like(g)
    acc = 1.0
    # vectorised enough: process in blocks with a python loop over block minima
    for i in range(len(g)):
        v = g[i]
        acc = v if v < acc else acc * a + v * (1 - a)
        sm[i] = acc
    # a short moving average so the attack is not a step
    k = np.ones(la) / la
    sm = np.minimum(sm, np.convolve(sm, k, "same"))
    y = x * (sm if x.ndim == 1 else sm[:, None])
    m = np.max(np.abs(y)) if len(y) else 0
    if m > c:
        y *= c / m
    return y


def true_peak(x):
    """The 4x oversampled peak (dBFS)."""
    if not len(x):
        return -120.0
    if x.ndim == 2:
        return max(true_peak(x[:, 0]), true_peak(x[:, 1]))
    up = signal.resample_poly(x, 4, 1)
    return 20 * np.log10(np.max(np.abs(up)) + 1e-12)


# ---- loudness (ITU-R BS.1770-4) ----------------------------------------------------------------------------------------------

def _kweight(x):
    # the two stages of the K filter, designed for this sample rate (high shelf +4 dB at ~1.5 kHz, high pass at 38 Hz)
    y = filt(x, "hs", 1681.97, 0.7072, 3.9998)
    return filt(y, "hp", 38.135, 0.5003)


def _power(x):
    if x.ndim == 1:
        return _kweight(x) ** 2
    return sum(_kweight(x[:, c]) ** 2 for c in range(x.shape[1]))


def lufs(x):
    """Integrated loudness (gated), or the momentary maximum for sounds shorter than a block."""
    p = _power(x)
    blk, hop = n_of(0.4), n_of(0.1)
    if len(p) < blk:
        return -0.691 + 10 * np.log10(np.mean(p) + 1e-12) if len(p) else -120.0
    cs = np.concatenate([[0], np.cumsum(p)])
    starts = np.arange(0, len(p) - blk + 1, hop)
    z = (cs[starts + blk] - cs[starts]) / blk
    l = -0.691 + 10 * np.log10(z + 1e-12)
    z = z[l > -70]
    if not len(z):
        return -120.0
    rel = -0.691 + 10 * np.log10(np.mean(z) + 1e-12) - 10
    l2 = -0.691 + 10 * np.log10(z + 1e-12)
    z2 = z[l2 > rel]
    return -0.691 + 10 * np.log10(np.mean(z2) + 1e-12)


def momentary_max(x):
    """The loudest 400 ms (LUFS): the level a short effect is heard at."""
    p = _power(x)
    blk = n_of(0.4)
    if len(p) <= blk:
        return -0.691 + 10 * np.log10(np.mean(p) * len(p) / blk + 1e-12) if len(p) else -120.0
    cs = np.concatenate([[0], np.cumsum(p)])
    z = (cs[blk:] - cs[:-blk]) / blk
    return -0.691 + 10 * np.log10(z.max() + 1e-12)


def master(x, target, how="momentary", ceiling=-1.0):
    """Brings x to a loudness target (LUFS: integrated or momentary maximum), removes DC, limits true peaks to `ceiling`."""
    x = np.asarray(x, float)
    if not len(x) or np.max(np.abs(x)) < 1e-9:
        return x
    x = hp(x, 18) if x.ndim == 1 else np.stack([hp(x[:, c], 18) for c in range(2)], 1)
    for _ in range(3):
        cur = lufs(x) if how == "integrated" else momentary_max(x)
        x = x * 10 ** ((target - cur) / 20)
        tp = true_peak(x)
        if tp <= ceiling + 0.05:
            break
        x = limit(x, ceiling - 0.6)
    # final guard (oversampled peak)
    tp = true_peak(x)
    if tp > ceiling:
        x = x * 10 ** ((ceiling - tp) / 20)
    return x

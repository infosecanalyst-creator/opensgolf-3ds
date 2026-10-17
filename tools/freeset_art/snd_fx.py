"""The free set's sound effects, synthesised: golf (swings, club contact, ball landings, the cup), the interface (clicks,
pops, confirmations), building and terrain tools, crowds, nature and weather beds, animals, machines and little musical
stingers. Each function returns mono samples at snd_dsp.SR (or (n, 2) where it says so)."""
import numpy as np

import snd_dsp as d
import snd_music as mu

SR = d.SR
T = d.t_axis
N = d.n_of


# ---- golf ----------------------------------------------------------------------------------------------------------------------

def whoosh(rng, dur=0.32, pitch=1.0, peak_at=0.7, strength=1.0):
    """A club swishing through the air: noise through a band that sweeps up to the bottom of the swing and back."""
    n = N(dur)
    t = np.arange(n) / n
    env = np.exp(-((t - peak_at) / (0.22 if t.size else 1)) ** 2 * 3) * np.clip(t * 8, 0, 1)
    fc = (500 + 2600 * np.exp(-((t - peak_at) / 0.2) ** 2)) * pitch
    nz = d.noise(n, rng)
    y = d.sweep_filter(nz, "bp", fc, 1.6, 64)
    y += d.sweep_filter(nz, "bp", fc * 0.5, 1.2, 64) * 0.5
    return y * env * strength


def club_hit(rng, kind="driver"):
    """Club meets ball. driver: a bright titanium 'tink-thwack'; iron: a crisp compact click with turf; chip: soft;
    putt: a gentle insert 'tock'."""
    if kind == "driver":
        body = d.modal([2950, 4500, 6100, 1600], [0.6, 0.45, 0.25, 0.4], [0.05, 0.035, 0.02, 0.03], 0.25, rng, 0.02)
        snap = d.click(rng, 0.006, 9000) * 1.4
        thump = d.sine(180 * np.exp(-T(0.12) / 0.015) + 90, 0.12) * d.perc(N(0.12), 0.001, 0.018) * 0.3
        return d.put(np.zeros(N(0.3)), 0, d.hp(body, 900) * 0.9) + d.put(np.zeros(N(0.3)), 0, snap) + d.put(np.zeros(N(0.3)), 0, thump)
    if kind == "iron":
        body = d.modal([2300, 3700, 5200], [0.5, 0.35, 0.15], [0.025, 0.018, 0.01], 0.25, rng, 0.02)
        snap = d.click(rng, 0.005, 7000) * 1.3
        turf = d.band(d.noise(N(0.18), rng), 300, 2500) * d.perc(N(0.18), 0.004, 0.05) * 0.5
        out = np.zeros(N(0.3))
        d.put(out, 0, body)
        d.put(out, 0, snap)
        d.put(out, 0.004, turf)
        return out
    if kind == "chip":
        body = d.modal([1900, 3100], [0.35, 0.2], [0.02, 0.012], 0.2, rng)
        turf = d.band(d.noise(N(0.15), rng), 250, 2200) * d.perc(N(0.15), 0.003, 0.04) * 0.6
        out = np.zeros(N(0.25))
        d.put(out, 0, body)
        d.put(out, 0, d.click(rng, 0.004, 5000) * 0.8)
        d.put(out, 0.003, turf)
        return out
    # putt
    body = d.modal([1150, 2350, 3500], [0.6, 0.25, 0.08], [0.035, 0.02, 0.01], 0.25, rng)
    out = np.zeros(N(0.3))
    d.put(out, 0, body)
    d.put(out, 0, d.click(rng, 0.003, 4000) * 0.5)
    return out


def ball_whistle(rng, dur=0.35, f0=3200):
    """The ball leaving fast: a faint high whistle falling away."""
    n = N(dur)
    t = T(dur)
    y = d.bp(d.noise(n, rng), f0, 6) * np.exp(-t / (dur / 3)) * np.minimum(1, t / 0.01)
    return y


def thud_grass(rng, strength=1.0):
    """A ball landing on turf: a soft low thump and a little grass brush."""
    n = N(0.25)
    t = T(0.25)
    th = d.sine(180 * np.exp(-t / 0.03) + 90, 0.25) * d.perc(n, 0.002, 0.03)
    brush = d.band(d.noise(n, rng), 600, 4000) * d.perc(n, 0.003, 0.025) * 0.35
    return (th + brush) * strength


def roll_grass(rng, dur=0.5):
    n = N(dur)
    t = T(dur)
    y = d.band(d.noise(n, rng), 800, 5000) * np.exp(-t / (dur / 2.5)) * 0.18
    grains = (rng.uniform(size=n) < 0.004) * rng.uniform(0.3, 1, n)
    return y + d.bp(grains, 2500, 1.5) * np.exp(-t / (dur / 2)) * 0.5


def sand_plop(rng, dur=0.6):
    """A ball burying in a bunker: a muffled thump and a spray of sand grains settling."""
    n = N(dur)
    t = T(dur)
    th = d.sine(120 * np.exp(-t / 0.03) + 60, dur) * d.perc(n, 0.002, 0.04) * 0.8
    spray = d.band(d.noise(n, rng), 1500, 9000) * d.perc(n, 0.004, 0.09) * 0.45
    grains = (rng.uniform(size=n) < 0.02 * np.exp(-t / 0.15)) * rng.uniform(0.2, 1, n)
    grains = d.hp(grains, 2000) * 0.6
    return th + spray + grains


def water_plop(rng, size=1.0, dur=1.0):
    """A ball (or a stone) dropping into water: the bubble's rising tone, a splash, droplets and bubbles."""
    n = N(dur)
    out = np.zeros(n)
    m = N(0.07)
    tt = T(0.07)
    f = (420 + 1300 * (tt / 0.07) ** 1.5) / size ** 0.4
    plop = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tt / 0.03) * np.minimum(1, tt / 0.002)
    d.put(out, 0.004, plop * 0.9)
    sp = d.band(d.noise(N(0.45), rng), 700, 7000) * d.perc(N(0.45), 0.003, 0.07 * size) * 0.6 * size ** 0.5
    d.put(out, 0, sp)
    for k in range(int(9 * size)):
        st = rng.uniform(0.04, dur * 0.6)
        ln = rng.uniform(0.012, 0.035)
        f0 = rng.uniform(900, 2600)
        tt = T(ln)
        blip = np.sin(2 * np.pi * np.cumsum(f0 * (1 + 0.8 * tt / ln)) / SR) * np.exp(-tt / (ln / 3))
        d.put(out, st, blip * rng.uniform(0.08, 0.22) * np.exp(-st / 0.35))
    return out


def wood_knock(rng, f=520, dur=0.4, bright=1.0):
    body = d.modal([f, f * 2.6, f * 4.4, f * 6.9], [1, 0.5, 0.25, 0.1], [0.06, 0.03, 0.015, 0.01], dur, rng, 0.03)
    c = d.click(rng, 0.004, 4000 * bright) * 0.5
    out = np.zeros(N(dur))
    d.put(out, 0, body)
    d.put(out, 0, c)
    return out


def leaves(rng, dur=1.2, density=1.0, bright=1.0):
    """Leaves rustling: a dense crackle of tiny noise grains under a band of hiss, swelling then settling."""
    n = N(dur)
    t = T(dur)
    env = np.minimum(1, t / 0.05) * np.exp(-t / (dur / 2.2))
    hiss = d.band(d.noise(n, rng), 1800 * bright, 8500) * 0.25
    g = (rng.uniform(size=n) < 0.03 * density) * rng.uniform(-1, 1, n)
    g = d.band(g, 1500 * bright, 7000) * 1.6
    return (hiss + g) * env


def cup_rattle(rng):
    """The ball dropping into the cup: a hollow plastic knock, two quick rattles and a settle, plus a soft chime."""
    out = np.zeros(N(0.95))
    for k, (at, g) in enumerate(((0.0, 1.0), (0.075, 0.55), (0.125, 0.35), (0.16, 0.2), (0.19, 0.12))):
        body = d.modal([780, 1650, 2700, 3900], [1, 0.6, 0.3, 0.15], [0.04, 0.025, 0.015, 0.01], 0.15, rng, 0.04)
        d.put(out, at, body * g)
        d.put(out, at, d.click(rng, 0.003, 6000) * g * 0.6)
    # a little sparkle of success (two soft bell notes)
    for at, m in ((0.22, 84), (0.3, 91)):
        d.put(out, at, mu.glock(mu.hz(m), 0.3, 0.6, rng) * mu.calibration("glock") * 0.35)
    return out


def flagstick(rng):
    out = np.zeros(N(0.5))
    d.put(out, 0, d.modal([1900, 4200, 6100], [0.6, 0.3, 0.15], [0.05, 0.03, 0.02], 0.3, rng, 0.02))
    d.put(out, 0.06, d.modal([2100, 4600], [0.3, 0.15], [0.03, 0.02], 0.2, rng))
    d.put(out, 0, thud_grass(rng, 0.4))
    return out


def bounces(rng, n=4, surface="path", first=0.0):
    """A ball bouncing on a hard surface, each bounce quicker and quieter."""
    out = np.zeros(N(1.2))
    at = first
    gap = 0.22
    g = 1.0
    for k in range(n):
        if surface == "path":
            hit = d.modal([1800, 3300], [0.5, 0.25], [0.02, 0.01], 0.1, rng)
            c = d.click(rng, 0.003, 6000)
            hit[:len(c)] += c * 0.8
        else:
            hit = thud_grass(rng, 1.0)
        d.put(out, at, hit * g)
        at += gap
        gap *= 0.62
        g *= 0.55
    return out


def can_open(rng, dur=3.9, loud=False):
    """A soda can: the tab's click, the hiss, then the fizz popping."""
    out = np.zeros(N(dur))
    d.put(out, 0.05, d.modal([2600, 4100, 5800], [0.5, 0.3, 0.2], [0.012, 0.008, 0.006], 0.06, rng))
    d.put(out, 0.05, d.click(rng, 0.004, 9000) * 0.9)
    hiss_n = N(0.45)
    hiss = d.band(d.noise(hiss_n, rng), 3000, 10000) * d.perc(hiss_n, 0.003, 0.12) * (0.9 if loud else 0.6)
    d.put(out, 0.07, hiss)
    t = T(dur)
    fizz = (rng.uniform(size=len(t)) < 0.03 * np.exp(-t / 1.2)) * rng.uniform(0.1, 1, len(t))
    out += d.bp(fizz, 6000, 1.0) * 0.4 * (1.3 if loud else 1)
    return out


def club_rattle(rng, dur=2.2):
    """A golf bag set down: clubs clattering against each other, then the bag's soft thump."""
    out = np.zeros(N(dur))
    d.put(out, 0.0, thud_grass(rng, 0.9))
    at = 0.02
    for k in range(9):
        f = rng.uniform(1500, 3800)
        hit = d.modal([f, f * 2.7, f * 4.1], [0.5, 0.3, 0.15], [0.05, 0.03, 0.02], 0.25, rng, 0.03)
        d.put(out, at, hit * rng.uniform(0.25, 0.7) * np.exp(-at / 0.35))
        at += rng.uniform(0.02, 0.09)
    return out


# ---- interface -----------------------------------------------------------------------------------------------------------------

def pop(rng, f=900, dur=0.08, drop=0.6, amp=1.0, body=0.0):
    """A soft modern 'pop': a sine that falls quickly in pitch with a tiny attack, the click of a nice button."""
    t = T(dur)
    fr = f * (1 + drop * np.exp(-t / 0.008))
    y = np.sin(2 * np.pi * np.cumsum(fr) / SR) * d.perc(len(t), 0.0008, dur / 4)
    if body:
        y += np.sin(2 * np.pi * f * 0.5 * t) * d.perc(len(t), 0.001, dur / 3) * body
    return y * amp


def tick(rng, f=2400, dur=0.03):
    t = T(dur)
    return (np.sin(2 * np.pi * f * t) * 0.6 + d.bp(d.noise(len(t), rng), f * 1.5, 2) * 0.5) * d.perc(len(t), 0.0004, 0.005)


def bell_note(rng, m, held=0.25, vel=0.7, instr="glock"):
    return getattr(mu, instr)(mu.hz(m), held, vel, rng) * mu.calibration(instr)


def notes(rng, seq, instr="glock", vel=0.7, gap=0.07, held=0.2, dur=None):
    """A little run of notes (MIDI) on a mallet: the set's chimes and confirmations."""
    total = dur or (len(seq) * gap + 1.2)
    out = np.zeros(N(total))
    for k, m in enumerate(seq):
        if m is None:
            continue
        if isinstance(m, (list, tuple)):
            for mm in m:
                d.put(out, k * gap, bell_note(rng, mm, held, vel, instr) * 0.7)
        else:
            d.put(out, k * gap, bell_note(rng, m, held, vel * (1 - 0.04 * k), instr))
    return out


def swoosh_up(rng, dur=0.3, lo=400, hi=3500):
    n = N(dur)
    t = np.arange(n) / n
    fc = lo * (hi / lo) ** t
    y = d.sweep_filter(d.noise(n, rng), "bp", fc, 2.0, 64)
    return y * np.sin(np.pi * t) ** 1.5


# ---- building and terrain tools ------------------------------------------------------------------------------------------------

def thunk(rng, f=180, dur=0.35, bright=2500):
    """Something set down on the ground: a low wooden thump."""
    t = T(dur)
    fr = f * (1 + 0.5 * np.exp(-t / 0.015))
    y = np.sin(2 * np.pi * np.cumsum(fr) / SR) * d.perc(len(t), 0.002, 0.06)
    y += d.lp(d.noise(len(t), rng), bright) * d.perc(len(t), 0.001, 0.015) * 0.6
    return y


def grass_tuft(rng, dur=0.4, bright=1.0):
    """A clump of turf laid down: a soft padded thump with a quick brush of blades."""
    n = N(dur)
    t = T(dur)
    th = d.sine(140 * np.exp(-t / 0.02) + 80, dur) * d.perc(n, 0.004, 0.035) * 0.7
    brush = d.band(d.noise(n, rng), 900 * bright, 6000 * bright) * d.perc(n, 0.01, 0.06) * 0.35
    blades = (rng.uniform(size=n) < 0.01) * rng.uniform(-1, 1, n)
    blades = d.band(blades, 2000, 8000) * np.exp(-t / 0.08)
    return th + brush + blades


def sand_pour(rng, dur=0.7, amount=1.0):
    n = N(dur)
    t = T(dur)
    env = np.minimum(1, t / 0.04) * np.exp(-t / (dur / 2.5))
    hiss = d.band(d.noise(n, rng), 2000, 9500) * 0.3
    g = (rng.uniform(size=n) < 0.05) * rng.uniform(-1, 1, n)
    g = d.hp(g, 2500) * 1.2
    th = d.sine(110 * np.exp(-t / 0.03) + 60, dur) * d.perc(n, 0.003, 0.05) * 0.5
    return (hiss + g) * env * amount + th


def gravel(rng, dur=0.5, steps=2):
    out = np.zeros(N(dur))
    for k in range(steps):
        m = N(0.18)
        t = T(0.18)
        cr = (rng.uniform(size=m) < 0.08) * rng.uniform(-1, 1, m)
        cr = d.band(cr, 700, 6000) * np.exp(-t / 0.05) * 1.2
        cr += d.band(d.noise(m, rng), 400, 3000) * np.exp(-t / 0.03) * 0.25
        d.put(out, k * 0.16, cr * (1 - 0.3 * k))
    return out


def rocks(rng, dur=0.9, n=3):
    out = np.zeros(N(dur))
    at = 0.0
    for k in range(n):
        f = rng.uniform(500, 1100)
        hit = d.modal([f, f * 2.3, f * 3.9, f * 5.6], [1, 0.6, 0.3, 0.2], [0.03, 0.02, 0.012, 0.008], 0.2, rng, 0.05)
        d.put(out, at, hit * (1 - 0.25 * k))
        d.put(out, at, d.click(rng, 0.003, 7000) * 0.6 * (1 - 0.25 * k))
        d.put(out, at, thunk(rng, 120, 0.2) * 0.4 * (1 - 0.3 * k))
        at += rng.uniform(0.08, 0.18)
    return out


def sparkle(rng, dur=0.6, n=6, lo=84, hi=100):
    """A shimmer of tiny high bell grains (a placement's 'ta-da' finish)."""
    out = np.zeros(N(dur + 0.6))
    for k in range(n):
        m = rng.integers(lo, hi)
        d.put(out, k * dur / n, bell_note(rng, int(m), 0.1, 0.5, "celesta") * rng.uniform(0.2, 0.45))
    return out


# ---- people in numbers ---------------------------------------------------------------------------------------------------------

def claps(rng, dur, people=30, rate=5.0, swell=0.3, fade=2.0, start_spread=0.25):
    """Applause from `people` clappers, each with their own hands (pitch, colour) and tempo, starting a little apart."""
    n = N(dur)
    out = np.zeros(n)
    t = T(dur)
    hands = []
    for _ in range(min(people, 24)):
        fc = rng.uniform(900, 2400)
        q = rng.uniform(1.2, 2.5)
        m = N(0.03)
        c = d.bp(d.noise(m, rng), fc, q) * d.perc(m, 0.0005, 0.004)
        c += d.hp(d.noise(m, rng), 2500) * d.perc(m, 0.0003, 0.002) * 0.4
        hands.append(c / (np.max(np.abs(c)) + 1e-9))
    for p in range(people):
        h = hands[p % len(hands)]
        r = rate * rng.uniform(0.75, 1.3)
        at = rng.uniform(0, start_spread)
        stop = dur - fade * rng.uniform(0.2, 1.0)
        g = rng.uniform(0.4, 1.0)
        while at < stop:
            d.put(out, at, h * g * rng.uniform(0.7, 1.1))
            at += 1 / r * rng.uniform(0.85, 1.15)
    env = np.minimum(1, t / swell) * np.clip((dur - t) / fade, 0, 1) ** 1.3
    return out * env


def murmur(rng, dur, people=10, level=1.0):
    """A crowd talking quietly: many far-off babbling voices low passed together."""
    import snd_voice as v
    n = N(dur)
    out = np.zeros(n)
    voices = ["m1", "m2", "m3", "f1", "f2", "f3"]
    for k in range(people):
        at = rng.uniform(-0.5, dur * 0.5)
        while at < dur:
            words = " ".join(v.babble(rng, int(rng.integers(3, 7))))
            y = v.say(voices[k % len(voices)], words, "neutral", rng, pitch=rng.uniform(0.9, 1.1))
            y = d.lp(y, 1800) * rng.uniform(0.3, 0.7)
            if at < 0:
                y = y[N(-at):]
            d.put(out, max(at, 0), y)
            at += len(y) / SR + rng.uniform(0.2, 1.0)
    return out * level


# ---- nature ---------------------------------------------------------------------------------------------------------------------

def loop_noise(n, rng, kind="pink"):
    """Noise that loops seamlessly (made periodic in the frequency domain)."""
    w = rng.standard_normal(n)
    f = np.fft.rfft(w)
    k = np.arange(len(f)).astype(float)
    k[0] = 1
    if kind == "pink":
        f /= np.sqrt(k)
    elif kind == "brown":
        f /= k
    y = np.fft.irfft(f, n)
    return y / (np.std(y) + 1e-12)


def periodic_filter(x, fn, *a):
    """Filters a looping signal as a loop (three copies, keep the middle) so its ends meet."""
    y = fn(np.concatenate([x, x, x]), *a)
    return y[len(x):2 * len(x)]


def wind_bed(rng, dur, level=1.0, gust=0.5, tone=500, loop=True):
    n = N(dur)
    nz = loop_noise(n, rng, "pink") if loop else d.pink(n, rng)
    base = periodic_filter(nz, d.lp4, tone) if loop else d.lp4(nz, tone)
    t = T(dur)
    # gusts: a slow wobble that loops (whole cycles over the loop)
    cycles = max(1, int(round(dur / 6)))
    m = 0.65 + gust * 0.35 * np.sin(2 * np.pi * cycles * t / dur + rng.uniform(0, 6)) \
        + gust * 0.15 * np.sin(2 * np.pi * (cycles * 3 + 1) * t / dur + rng.uniform(0, 6))
    whistle = periodic_filter(nz, d.bp, tone * 2.2, 4.0) * 0.25 * np.clip(m - 0.7, 0, 1)
    return (base * m + whistle) * level


def bird_song(rng, kind="warbler", at_pitch=1.0):
    """One bird's phrase. warbler: quick trills; robin: rising and falling whistles; dove: soft low coos; hawk: a far cry;
    gull: a laughing call."""
    out = []
    if kind == "warbler":
        f = rng.uniform(3000, 4800) * at_pitch
        for k in range(int(rng.integers(5, 11))):
            ln = rng.uniform(0.035, 0.06)
            t = T(ln)
            fr = f * (1 + 0.25 * np.sin(np.pi * t / ln)) * rng.uniform(0.92, 1.08)
            out.append(np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.sin(np.pi * t / ln) ** 2)
            out.append(np.zeros(N(rng.uniform(0.01, 0.03))))
    elif kind == "robin":
        f = rng.uniform(2200, 3400) * at_pitch
        for k in range(int(rng.integers(3, 6))):
            ln = rng.uniform(0.09, 0.2)
            t = T(ln)
            a, b = f * rng.uniform(0.8, 1.3), f * rng.uniform(0.8, 1.3)
            fr = a + (b - a) * t / ln + 300 * np.sin(np.pi * t / ln)
            y = np.sin(2 * np.pi * np.cumsum(fr) / SR) + 0.15 * np.sin(4 * np.pi * np.cumsum(fr) / SR)
            out.append(y * np.sin(np.pi * t / ln) ** 1.5)
            out.append(np.zeros(N(rng.uniform(0.04, 0.12))))
    elif kind == "dove":
        f = rng.uniform(420, 520)
        for ln, fa, fb in ((0.35, 1.0, 1.1), (0.5, 1.1, 0.95), (0.3, 0.95, 0.9)):
            t = T(ln)
            fr = f * (fa + (fb - fa) * t / ln)
            y = np.sin(2 * np.pi * np.cumsum(fr) / SR) + 0.25 * np.sin(4 * np.pi * np.cumsum(fr) / SR)
            out.append(d.lp(y, 1200) * np.sin(np.pi * t / ln) ** 2)
            out.append(np.zeros(N(0.12)))
    elif kind == "hawk":
        ln = 1.1
        t = T(ln)
        fr = 2600 * (1.1 - 0.35 * t / ln)
        y = np.sin(2 * np.pi * np.cumsum(fr) / SR)
        y = y + 0.3 * d.bp(d.noise(len(t), rng), 2800, 3) * 0.3
        out.append(y * np.minimum(1, t / 0.05) * np.exp(-t / 0.5) * 0.7)
    elif kind == "gull":
        f = rng.uniform(1200, 1600)
        for k in range(int(rng.integers(3, 6))):
            ln = 0.16
            t = T(ln)
            fr = f * (1.2 - 0.4 * t / ln)
            y = sum(np.sin(h * 2 * np.pi * np.cumsum(fr) / SR) / h for h in range(1, 6))
            y = d.bp(y, 2200, 1.2) * np.sin(np.pi * t / ln)
            out.append(y)
            out.append(np.zeros(N(0.07)))
    elif kind == "parrot":
        for k in range(int(rng.integers(2, 4))):
            ln = rng.uniform(0.12, 0.25)
            t = T(ln)
            fr = rng.uniform(1500, 2500) * (1 + 0.3 * np.sin(2 * np.pi * 18 * t))
            y = sum(np.sin(h * 2 * np.pi * np.cumsum(fr) / SR) / h ** 1.2 for h in range(1, 6))
            out.append(d.bp(y, 2600, 1.0) * np.sin(np.pi * t / ln))
            out.append(np.zeros(N(0.08)))
    y = np.concatenate(out) if out else np.zeros(1)
    return y


def insects(rng, dur, kind="cricket", loop=True):
    """Crickets (pulsed chirps) or cicadas (a buzzing swell), looping."""
    n = N(dur)
    t = T(dur)
    if kind == "cricket":
        f = rng.uniform(4200, 5200)
        carrier = np.sin(2 * np.pi * f * t)
        rate = round(rng.uniform(2.5, 3.5) * dur) / dur
        chirp = (np.sin(2 * np.pi * rate * t) > 0.35).astype(float)
        pulse = 0.5 + 0.5 * np.sin(2 * np.pi * round(40 * dur) / dur * t)
        env = periodic_filter(chirp, d.lp, 80)
        return carrier * env * pulse * 0.3
    # cicada
    nz = loop_noise(n, rng, "white")
    y = periodic_filter(nz, d.bp, 5500, 3.0)
    am = 0.5 + 0.5 * np.sin(2 * np.pi * round(110 * dur) / dur * t)
    cycles = max(1, round(dur / 7))
    swell = np.clip(np.sin(2 * np.pi * cycles * t / dur), 0, 1) ** 2
    return y * am * swell * 0.4


def scatter(rng, dur, make, density, loop=True, gain=(0.4, 1.0), pan=False):
    """Calls dropped at random times over a bed; with `loop`, a call running past the end wraps round to the start."""
    n = N(dur)
    out = np.zeros(n)
    at = rng.uniform(0.2, 1.5) / density
    while at < dur:
        y = make() * rng.uniform(*gain)
        i = N(at)
        m = min(len(y), n - i)
        out[i:i + m] += y[:m]
        if loop and len(y) > m:
            rest = y[m:]
            out[:len(rest)] += rest[:n]
        at += rng.exponential(1.0 / density) + 0.3
    return out


def waves(rng, dur, level=1.0):
    """Surf: slow swells of filtered noise that break and wash back, looping."""
    n = N(dur)
    t = T(dur)
    nz = loop_noise(n, rng, "pink")
    k = max(1, round(dur / 5.5))
    ph = (k * t / dur) % 1.0
    swell = np.where(ph < 0.55, (ph / 0.55) ** 2, np.exp(-(ph - 0.55) / 0.18))
    low = periodic_filter(nz, d.lp4, 700) * swell
    hiss = periodic_filter(nz, d.band, 1500, 7000) * swell ** 2 * 0.5
    return (low + hiss) * level


def lake_lap(rng, dur):
    """Water lapping at a bank: soft irregular slaps and gurgles over a low wash, looping."""
    n = N(dur)
    bed = periodic_filter(loop_noise(n, rng, "pink"), d.lp4, 500) * 0.25

    def slap():
        m = N(rng.uniform(0.12, 0.3))
        t = np.arange(m) / m
        y = d.band(d.noise(m, rng), 250, 1600) * np.sin(np.pi * t) ** 2
        if rng.uniform() < 0.5:
            tt = T(0.04)
            y[:len(tt)] += np.sin(2 * np.pi * np.cumsum(rng.uniform(300, 700) * (1 + tt / 0.04)) / SR) * np.exp(-tt / 0.012) * 0.5
        return y
    return bed + scatter(rng, dur, slap, 1.6, gain=(0.3, 0.7))


def frog(rng):
    ln = rng.uniform(0.12, 0.2)
    t = T(ln)
    f0 = rng.uniform(140, 220)
    src = sum(np.sin(h * 2 * np.pi * f0 * t) / h for h in range(1, 12))
    am = 0.5 + 0.5 * np.sign(np.sin(2 * np.pi * rng.uniform(25, 40) * t))
    y = d.bp(src * am, 900, 1.5) * np.sin(np.pi * t / ln)
    return np.concatenate([y, np.zeros(N(0.08)), y * 0.8]) if rng.uniform() < 0.5 else y


def rain_bed(rng, dur, level=1.0):
    n = N(dur)
    nz = loop_noise(n, rng, "white")
    hiss = periodic_filter(nz, d.band, 1200, 9000) * 0.25
    drops = (rng.uniform(size=n) < 0.004) * rng.uniform(0.2, 1, n)
    drops = periodic_filter(drops, d.bp, 3000, 1.0) * 1.5
    return (hiss + drops) * level


# ---- animals -------------------------------------------------------------------------------------------------------------------

def bleat(rng, f0=280, dur=0.8):
    """A sheep's 'baa': a nasal, buzzy vowel with a fast wobble."""
    import snd_voice as v
    voice = v.Voice(f0, 1.25, breath=0.2, tension=0.75, jitter=0.02, vibrato=0.0, creak=0.3)
    sp = v.Speaker(voice, "neutral", rng)
    sp.consonant("b", "ae", 0.8, f0)
    sp.vowel_seg("ae", dur * 0.6, 1.0, f0 * 1.05, f0 * 0.95)
    sp.vowel_seg("ae", dur * 0.4, 0.7, f0 * 0.95, f0 * 0.8, end_vowel="e")
    y = sp.render()
    t = T(len(y) / SR)[:len(y)]
    return y * (1 + 0.45 * np.sin(2 * np.pi * 7.5 * t))


def moose(rng, dur=1.5):
    import snd_voice as v
    voice = v.Voice(95, 0.7, breath=0.25, tension=0.7, jitter=0.02, creak=0.4)
    sp = v.Speaker(voice, "neutral", rng)
    sp.vowel_seg("o", dur * 0.3, 0.8, 90, 120)
    sp.vowel_seg("o", dur * 0.5, 1.0, 120, 85, end_vowel="u")
    sp.vowel_seg("u", dur * 0.2, 0.5, 85, 70)
    return sp.render()


def hiss(rng, dur=1.2, f=5000):
    n = N(dur)
    t = T(dur)
    return d.band(d.noise(n, rng), f * 0.6, min(f * 1.8, 10500)) * np.minimum(1, t / 0.15) * np.clip((dur - t) / 0.4, 0, 1)


def rattle(rng, dur=1.6):
    n = N(dur)
    t = T(dur)
    rate = 55
    pulses = (np.sin(2 * np.pi * rate * t) > 0.6).astype(float)
    y = d.band(d.noise(n, rng), 3500, 10000) * d.lp(pulses, 400) * 2
    return y * np.minimum(1, t / 0.1) * np.clip((dur - t) / 0.4, 0, 1)


def gator(rng, dur=1.6):
    """A low growling hiss with a guttural rumble."""
    n = N(dur)
    t = T(dur)
    rumble = d.lp4(d.noise(n, rng), 160) * (1 + 0.6 * np.sin(2 * np.pi * 22 * t)) * 1.6
    h = d.band(d.noise(n, rng), 1500, 6000) * 0.3
    env = np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 1.2
    return (rumble + h) * env


def dolphin(rng):
    out = []
    for k in range(3):
        ln = rng.uniform(0.18, 0.32)
        t = T(ln)
        a, b = rng.uniform(4000, 6500), rng.uniform(5500, 9000)
        fr = a + (b - a) * (t / ln) ** 2 + 600 * np.sin(2 * np.pi * 9 * t)
        out.append(np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.sin(np.pi * t / ln) ** 1.5 * 0.7)
        out.append(np.zeros(N(0.05)))
        if rng.uniform() < 0.6:
            clicks = np.zeros(N(0.25))
            for c in range(int(rng.integers(8, 16))):
                d.put(clicks, c * 0.016, d.click(rng, 0.002, 9000))
            out.append(clicks * 0.6)
    return np.concatenate(out)


def fly(rng, dur=3.0):
    t = T(dur)
    f = 210 + 25 * np.sin(2 * np.pi * 0.9 * t) + 15 * np.sin(2 * np.pi * 2.3 * t)
    ph = 2 * np.pi * np.cumsum(f) / SR
    y = sum(np.sin(h * ph) / h for h in range(1, 14))
    y = d.peak(d.lp(y, 3000), 1200, 6, 1.5)
    dist = 0.6 + 0.4 * np.sin(2 * np.pi * 0.45 * t + 1)  # flying past and round
    return y * dist * np.minimum(1, t / 0.2) * np.clip((dur - t) / 0.5, 0, 1)


def seriama(rng):
    """A long-legged bird's yelping call: a run of loud barking notes that speed up and fall."""
    out = []
    f = 1500
    for k in range(9):
        ln = 0.09 - k * 0.004
        t = T(ln)
        fr = f * (1.15 - 0.3 * t / ln)
        y = sum(np.sin(h * 2 * np.pi * np.cumsum(fr) / SR) / h for h in range(1, 5))
        out.append(d.bp(y, 2000, 1.0) * np.sin(np.pi * t / ln))
        out.append(np.zeros(N(max(0.02, 0.07 - k * 0.006))))
        f *= 0.985
    return np.concatenate(out)


def snore(rng, dur=3.6):
    """In through the nose with a rattle, out with a soft whistle-puff, twice."""
    out = np.zeros(N(dur))
    for k, at in enumerate((0.1, dur / 2 + 0.05)):
        ln = dur / 2 * 0.55
        t = T(ln)
        rumble = d.lp(d.noise(len(t), rng), 600) * (0.5 + 0.5 * np.sign(np.sin(2 * np.pi * 32 * t))) * np.sin(np.pi * t / ln) ** 1.5
        d.put(out, at, rumble)
        ex = T(ln * 0.6)
        puff = d.bp(d.noise(len(ex), rng), 900, 2.0) * np.sin(np.pi * ex / ex[-1]) ** 2 * 0.4
        d.put(out, at + ln + 0.05, puff)
    return out


# ---- machines -----------------------------------------------------------------------------------------------------------------

def engine(rng, dur, rpm=(900, 3000), cyl=4, rough=0.2, bright=1800, revs=None):
    """A combustion engine: firing pulses at the cylinder rate through an exhaust resonance, revving along a curve."""
    t = T(dur)
    if revs is None:
        revs = rpm[0] + (rpm[1] - rpm[0]) * np.clip(np.sin(np.pi * t / dur * 0.9), 0, 1)
    fire = revs / 60 * cyl / 2
    ph = np.cumsum(fire) / SR
    frac = ph % 1.0
    pulse = np.exp(-frac * 8) * (1 + rough * d.lp(d.noise(len(t), rng), 50) * 3)
    y = d.lp(pulse - pulse.mean(), bright)
    y = d.peak(y, 120, 6, 1.2)
    y += d.lp(d.noise(len(t), rng), 900) * 0.15
    return d.softclip(y * 2, 1.5)


def vtwin(rng, dur):
    """A big twin's lilting idle-and-go: uneven pairs of firings."""
    t = T(dur)
    revs = 800 + 1600 * np.clip((t - dur * 0.3) / (dur * 0.5), 0, 1)
    f = revs / 60
    ph = np.cumsum(f) / SR
    frac = ph % 1.0
    pulse = np.exp(-frac * 14) + 0.8 * np.exp(-((frac - 0.32) % 1.0) * 14)
    y = d.lp(pulse - pulse.mean(), 900)
    y = d.peak(y, 90, 8, 1.0) + d.lp(d.noise(len(t), rng), 500) * 0.2
    return d.softclip(y * 2.5, 1.6)


def horn(rng, dur=0.6, f=330, two=True):
    t = T(dur)
    y = sum(np.sin(h * 2 * np.pi * f * t) / h for h in range(1, 12))
    if two:
        y += sum(np.sin(h * 2 * np.pi * f * 1.26 * t) / h for h in range(1, 12))
    y = d.peak(d.lp(y, 3000), 1100, 6, 1.0)
    return y * d.adsr(len(t), 0.02, 0.1, 0.9, 0.08)


def burner(rng, dur=3.4):
    """A hot-air balloon's burner: a roaring blast of filtered noise with a flickering flame."""
    n = N(dur)
    t = T(dur)
    roar = d.lp4(d.noise(n, rng), 1200) + d.band(d.noise(n, rng), 1500, 5000) * 0.3
    flick = 1 + 0.25 * d.lp(d.noise(n, rng), 15) * 3
    env = np.minimum(1, t / 0.12) * np.clip((dur - t) / 0.7, 0, 1)
    return roar * flick * env


def mower(rng, dur):
    y = engine(rng, dur, rpm=(2600, 3000), cyl=1, rough=0.3, bright=1400)
    blade = d.band(d.noise(N(dur), rng), 2000, 6000) * 0.15
    return y + blade


def electric_cart(rng, dur=3.8):
    """A golf cart pulling away: a motor whine rising, tyres rolling on grass, a bump, fading into the distance."""
    n = N(dur)
    t = T(dur)
    speed = np.clip(t / 1.5, 0, 1) ** 0.7
    whine = np.sin(2 * np.pi * np.cumsum(260 + 520 * speed) / SR) * 0.25
    whine += np.sin(2 * np.pi * np.cumsum(780 + 1560 * speed) / SR) * 0.05
    tyres = d.band(d.noise(n, rng), 150, 1200) * (0.3 + 0.5 * speed)
    rattle_ = d.band(d.noise(n, rng), 2000, 5000) * 0.08 * speed
    y = whine + tyres + rattle_
    d.put(y, 0.9, thunk(rng, 90, 0.3) * 0.5)
    dist = np.clip(1 - (t - 1.2) / (dur - 1.2), 0, 1) ** 1.5
    y = y * np.minimum(1, t / 0.15) * np.where(t < 1.2, 1, dist)
    return d.sweep_filter(y, "lp", 6000 - 4500 * np.clip((t - 1.2) / (dur - 1.2), 0, 1), 0.7)


def shutter(rng):
    out = np.zeros(N(0.6))
    d.put(out, 0, d.click(rng, 0.004, 9000) * 0.9)
    d.put(out, 0.0, d.modal([3200, 5100], [0.4, 0.2], [0.01, 0.006], 0.04, rng))
    d.put(out, 0.07, d.click(rng, 0.006, 6000) * 0.7)
    d.put(out, 0.07, d.modal([2400, 3900], [0.3, 0.15], [0.012, 0.008], 0.05, rng))
    # the little motor winding on
    m = N(0.25)
    t = T(0.25)
    w = np.sin(2 * np.pi * np.cumsum(900 + 300 * t / 0.25) / SR) * 0.15 * np.sin(np.pi * t / 0.25)
    w += d.band(d.noise(m, rng), 2000, 6000) * 0.05 * np.sin(np.pi * t / 0.25)
    d.put(out, 0.16, w)
    return out


def coins(rng, dur=1.4, n=10):
    """A little cascade of coins with a bright register 'ka-ching' bell on top."""
    out = np.zeros(N(dur))
    at = 0.0
    for k in range(n):
        f = rng.uniform(3500, 6500)
        c = d.modal([f, f * 1.42, f * 2.1], [0.5, 0.3, 0.15], [0.08, 0.05, 0.03], 0.3, rng, 0.02)
        d.put(out, at, c * rng.uniform(0.3, 0.8) * np.exp(-at / 0.5))
        at += rng.uniform(0.015, 0.06)
    for k, m in enumerate((88, 95)):
        d.put(out, 0.02 + 0.09 * k, bell_note(rng, m, 0.4, 0.8, "glock") * 0.7)
    return out

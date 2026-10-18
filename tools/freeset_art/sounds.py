#!/usr/bin/env python3
"""The free set's sounds and music: every sound file the game plays (crates/sg-core/src/freeset/manifest.txt, W lines),
all made in code: synthesised golf, interface, building, crowd, nature and animal sounds (snd_fx.py), golfers and
celebrities babbling in a made-up language (snd_voice.py), and original music (snd_music.py, snd_songs.py). Each is
levelled to its kind's loudness (EBU R128 / ITU-R BS.1770: music -18 LUFS integrated, effects by their loudest 100 ms),
its true peak kept under -1 dBFS, cut or padded to the length the manifest gives (music: its own length), and written as
Ogg Vorbis to packs/opensgolf-free/art/Sounds/ (the engine takes an OGG for any WAV).

usage: sounds.py [--only REGEX] [--report FILE]
needs: numpy, scipy, ffmpeg (with libvorbis) on the PATH"""
import argparse
import os
import re
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layout  # noqa: E402
import snd_dsp as d  # noqa: E402
import snd_fx as fx  # noqa: E402
import snd_music as mu  # noqa: E402
import snd_songs as songs  # noqa: E402
import snd_voice as vo  # noqa: E402

ROOT = layout.ROOT
OUT = os.path.join(ROOT, "packs", "opensgolf-free", "art")
SR = d.SR

# Files the game's folders hold twice (the same piece under two names): only the first is written, and the engine plays it
# for the second too (crates/sg-core/src/freeset/sound.rs, ALIASES; keep the two lists the same). The last three are not
# played by the game at all and stand in for pieces of the same kind.
ALIASES = {
    "Sounds/Music/Misc_Music/New Age.wav": "Sounds/New Age.wav",
    "Sounds/Music/Tropical_Music/Steel Drums 2.wav": "Sounds/World/Steel Drums 2.wav",
    "Sounds/Music/Links_Music/Bagpipe1.wav": "Sounds/World/Bagpipe.wav",
    "Sounds/Music/Desert_Music/Spanish Guitar 2.wav": "Sounds/World/Spanish Guitar wFlute 2.wav",
    "Sounds/Steel Drum Short.wav": "Sounds/Music/Tropical_Music/Steel Drum 1a.wav",
    "Sounds/manifold music.wav": "Sounds/Music/Misc_Music/Jazz.wav",
    "Sounds/newhart-theme.wav": "Sounds/Music/Misc_Music/End Of Year Short.wav",
}

# loudness targets: (dB, how): "fast" is the loudest 100 ms (K-weighted), "integrated" the gated programme loudness
UI, GOLF, LAND, VOICE, CROWD, ANIMAL, STING, BED, MUSIC = (
    (-19, "fast"), (-14, "fast"), (-17, "fast"), (-17, "fast"), (-18, "fast"), (-19, "fast"), (-16, "fast"),
    (-27, "integrated"), (-18, "integrated"))


def rng_for(path):
    return d.rng_of("opensgolf-free", path.lower())


# ---- voices ------------------------------------------------------------------------------------------------------------------

# what golfers say (SimsFX: the golfers' own lines; Sounds/Male, Female: shot comments): name pattern -> (mood, script)
# A script's syllables are our own babble; '<...>' are sounds (snd_voice.say).
LINES = [
    (r"badhole", "disgust", "<grumble 3> nee-ba-ZOO. pfft."),
    (r"bad", "angry", "<slidedown .22> BAH!"),
    (r"benchold", "tired", "<sigh .7> oof. hoo-ba-la."),
    (r"bench", "happy", "<sigh .8> aah~ mee-la-SHOO~"),
    (r"blind", "question", "hm? wee-da-POH?"),
    (r"celeb", "excited", "<gasp> <oooh> shee-ba-LOO!"),
    (r"scoke", "happy", "<slurp> aah~"),
    (r"coke", "happy", "<gulp> AAH! bee-lo-NAH~"),
    (r"crabgrass2", "disgust", "eew. gra-ba-NOOK! <grumble 3> tsk tsk. bla-ga-DEE."),
    (r"crab", "disgust", "eew, gra-ba-NOOK. <grumble 2>"),
    (r"easy", "sly", "pfft. nee-ba POO-ka~ <giggle 3>"),
    (r"greeting", "happy", "ha-LOH! wee-BA-boo!"),
    (r"happy0", "happy", "<whee> wee-BOO na-ZOO! <laugh 3>"),
    (r"happy1", "happy", "<laugh 3> yay-pa-LOO!"),
    (r"hard", "sad", "oof... ga-ROO-ba."),
    (r"haveadrink", "question", "drin-KA, moo-BEE?"),
    (r"hungry2", "sad", "<rumble> mm-mm... nom-ba-NEE? <rumble> nom-BA?"),
    (r"hungry", "sad", "<rumble> nom-ba?"),
    (r"hurryup", "angry", "ha-RA, ha-RA! ZOO-ba! <grumble 3>"),
    (r"lovely", "awe", "<oooh> lee-va-SHAY~"),
    (r"mad", "angry", "BAH-gat! DOO-ga-sha KRA-bo! <grumble 3> nee-ga BOM-ba-la, ZOO-ga-sha DEEN!"),
    (r"oops", "excited", "oop!"),
    (r"options", "question", "hm. shee-ba? noo-ba?"),
    (r"sad", "sad", "<aww> noo-ba..."),
    (r"same", "tired", "sa-ma... sa-ma... <sigh .6>"),
    (r"senic", "awe", "<oooh> oo-LAH lee-nay-VOO~"),
    (r"snack", "happy", "<munch 3> mm-MM!"),
    (r"storyno", "disgust", "nah-NAH-nah."),
    (r"storyok", "neutral", "oh-KEE-doh."),
    (r"storyyes", "excited", "YA! ya-ya-YA, shee-BOO!"),
    (r"story", "neutral", "so, bee-la noo-ma shee-DOH, ta-ba LOO-ma?"),
    (r"thirsty", "tired", "<creak uh .25> wa-wa... DREE?"),
    (r"tired", "tired", "<yawn> mm-ba..."),
    (r"tricky", "question", "hmm... tee-ka-ROO?"),
    (r"ugly", "disgust", "EEW! bla-BLA."),
    (r"variety2", "tired", "sa-ma, sa-MA."),
    (r"variety", "disgust", "sa-ma, sa-ma, SA-ma! <grumble 2> noo-va, zip-a-DEE?"),
    (r"waiting", "sly", "<hum 1 1.12 1 .9 1.12> ta-ta... HOO-ba?"),
    (r"water", "sad", "noo, noo, NOOOO..."),
    (r"wet", "neutral", "<trill .35> brr-BA!"),
    # shot comments
    (r"good shot", "excited", "<whee> ya-HOO!"),
    (r"bad shot", "angry", "<slidedown> ah-BAH! <grumble 2>"),
    (r"easy shot", "sly", "pfft. ba-ZEE~ <giggle 3>"),
    (r"hard shot", "sad", "oof... ga-ROO-ba, mo-ka-NEE."),
    (r"tricky shot", "question", "hmm. tee-ka-ROO-ba?"),
]

# the emotion clips by body type: (voice, pitch) per code; the mood from the file name
BODY_VOICE = {"malekls": ("m2", 1.0), "mkls": ("m2", 0.95), "malepls": ("m1", 1.0), "pls": ("m1", 1.05), "malepss": ("m3", 1.0),
              "malesss": ("m_old", 1.0), "sss": ("m1", 0.95), "femalepls": ("f1", 1.0), "femalepss": ("f2", 1.0),
              "femalesss": ("f_old", 1.0), "femaleskss": ("f2", 1.08), "femalesktt": ("f3", 1.0), "fm": ("f1", 1.05)}
EMOTION = {
    "happy": ("happy", "<giggle 4> wee-la-LOO~", "<laugh 4> ha-BOO-na!"),
    "sad": ("sad", "<aww> moo-ga-na...", "<sigh .9> oh-noo-ba..."),
    "success": ("excited", "<slideup> <whee> YA-HOO!", "<oooh> YES-ka! ya-HOO!"),
    "failure": ("angry", "<slidedown> <grumble 3> ah-BAH!", "<slidedown> ARGH-ba! <grumble 2>"),
}

CELEBS = {
    "action star char": ("action", "neutral", "GRA-bo. DA-bak."),
    "action star dislike": ("action", "angry", "<grunt> NAH-ga. BOOM-ba."),
    "action star interact": ("action", "neutral", "ha. TOH-ga, ba-ZOOK."),
    "action star like": ("action", "happy", "<laugh 2 .8> YA-ga! DA-bak."),
    "action star status q": ("action", "neutral", "GRA-bo DA-bak. noo-ma TOH-ga. ha. ZOOK-a-ma, BOOM-ba."),
    "action star walk": ("action", "neutral", "<grunt .6> hup. hup."),
    "aging star": ("aging", "awe", "ah, dah-ling~ shee-ma-RAH, la-la-LOO~"),
    "comedian char": ("comic", "happy", "so, ba-DOO-ga sha-NEE? ba-DA! <rimshot>"),
    "comedian don't like": ("comic", "disgust", "ee-yah... TOUGH-ba, tough-BA."),
    "comedian like": ("comic", "happy", "<laugh 4 1.1> HA! ba-DOO-ga!"),
    "female comic": ("comic_f", "happy", "an-NEE-ba, sho-sho, ZAP-pa-DOO! <rimshot>"),
    "fitness female": ("fitness", "excited", "HUP! HUP! wan-TOO, wan-TOO! ya-ya-YA!"),
    "i'll be back": ("action", "neutral", "a-BEE BAK."),
    "lawn technician": ("tech", "happy", "<mower 1.2> la-la-LOO, sho-ba-NEE~"),
    "marshall male": ("marshal", "neutral", "<ahem> pee-ka, PEE-ka! moo-ba-la DOH."),
    "movie princess": ("princess", "awe", "oh~ shee-la-MAY, la-la-LEE~"),
    "politician char": ("politician", "neutral", "my-FEL-lo ga-ROO-bas! na-MOO-ba, PRO-mis-sa, LOW-va-tax-a! <claps>"),
    "politician dislike": ("politician", "disgust", "hm. NO-ka-ment."),
    "politician like": ("politician", "happy", "<laugh 2> va-ROO-ba, ha-ha, VOH-ta!"),
    "ranger": ("ranger", "happy", "HOW-dee! pa-ROO-ba, ta-NEE~"),
    "rock star": ("rocker", "excited", "<guitar> YEAH-ya! ROCK-a-loo!"),
    "spears char": ("singer", "happy", "<sing>"),
    "spears char radio": ("singer", "happy", "<sing radio>"),
    "spears dislike": ("singer", "disgust", "oo, NO-pa. na-na."),
    "spears like": ("singer", "excited", "OH-ma-gosh! la-LOO~"),
    "super model": ("model", "sly", "mm~ dar-LEE-na. shee-ma, voo-LAH~ <sigh .7>"),
    "tray girl": ("tray", "happy", "drin-KA? SNAK-a? la-LOO~"),
}


def voice_for(path):
    """A golfer clip's speaker: men's and women's voices picked per file, so a course sounds like many people."""
    low = path.lower()
    name = os.path.basename(low)
    female = "/female/" in low
    if "benchold" in name:
        return "m_old"
    pool = ["f1", "f2", "f3"] if female else ["m1", "m2", "m3"]
    return pool[int(rng_for(path).integers(len(pool)))]


def extra_sound(tag, rng, sp_voice):
    """The non-verbal sounds a script can hold besides the synthesiser's own (snd_voice): gulps, slurps, munching,
    tummy rumbles, a rimshot, a mower, an electric guitar, claps, a polite cough."""
    if tag == "gulp":
        t = d.t_axis(0.12)
        y = np.sin(2 * np.pi * np.cumsum(420 * np.exp(-t / 0.04) + 160) / SR) * d.perc(len(t), 0.003, 0.03)
        return np.concatenate([y, np.zeros(d.n_of(0.08)), y * 0.7, np.zeros(d.n_of(0.1))])
    if tag == "slurp":
        n = d.n_of(0.5)
        fc = np.linspace(900, 2600, n) + 300 * np.sin(np.linspace(0, 40, n))
        y = d.sweep_filter(d.noise(n, rng), "bp", fc, 4.0) * np.sin(np.linspace(0, np.pi, n)) ** 0.7
        return np.concatenate([y * (0.6 + 0.4 * (rng.uniform(size=n) > 0.3)), np.zeros(d.n_of(0.1))])
    if tag.startswith("munch"):
        out = []
        for k in range(int(tag.split()[1]) if " " in tag else 3):
            m = d.n_of(0.12)
            t = d.t_axis(0.12)
            crunch = d.band(d.noise(m, rng), 1500, 7000) * (rng.uniform(size=m) < 0.25) * np.exp(-t / 0.04)
            out += [crunch * 1.5, vo.say(sp_voice, "<hum 1>", "happy", rng)[: d.n_of(0.12)] * 0.4, np.zeros(d.n_of(0.06))]
        return np.concatenate(out)
    if tag == "rumble":
        n = d.n_of(0.6)
        t = d.t_axis(0.6)
        y = d.lp(d.noise(n, rng), 300) * (0.6 + 0.4 * np.sin(2 * np.pi * 9 * t))
        y += np.sin(2 * np.pi * np.cumsum(90 + 60 * np.sin(2 * np.pi * 3 * t)) / SR) * 0.3
        return y * np.sin(np.pi * t / 0.6) ** 2 * 1.5
    if tag == "rimshot":
        p = mu.Piece("rimshot", 180)
        p.track("dr", None, 1.0, 0, 0.15, kind="drums")
        p.note("dr", 0, "r", 0, 0.9)
        p.note("dr", 0.5, "r", 0, 0.9)
        p.note("dr", 1.0, "k", 0, 0.8)
        p.note("dr", 1.0, "o", 0, 1.0)
        x = p.render(2, master_lufs=-20)
        return x.mean(1)
    if tag.startswith("mower"):
        return fx.mower(rng, 1.2) * np.sin(np.linspace(0, np.pi, d.n_of(1.2))) * 0.6
    if tag == "guitar":
        # a big friendly power chord: plucked strings pushed into a soft overdrive
        out = np.zeros(d.n_of(1.0))
        for k, m in enumerate((40, 47, 52, 55)):
            d.put(out, k * 0.012, mu.ks(mu.hz(m), 0.8, 1.0, rng, 0.8, 2.0) * 0.7)
        return d.lp(d.softclip(out * 6, 2.5), 3500) * 0.5
    if tag == "claps":
        return fx.claps(rng, 1.2, people=12, rate=5, swell=0.05, fade=0.6) * 2.5
    if tag.startswith("slidedown"):
        ln = float(tag.split()[1]) if " " in tag else 0.38
        return np.concatenate([fx.slide_whistle(rng, 1700, 420, ln) * 0.5, np.zeros(d.n_of(0.03))])
    if tag == "slideup":
        return np.concatenate([fx.slide_whistle(rng, 500, 1900, 0.3) * 0.5, np.zeros(d.n_of(0.03))])
    if tag == "ahem":
        sp = vo.Speaker(vo.VOICES[sp_voice] if isinstance(sp_voice, str) else sp_voice, "neutral", rng)
        sp.tr.add(0.04, f1=500, f2=1500, av=0, ah=0.6, f0=sp.base)
        sp.vowel_seg("uh", 0.1, 0.8, sp.base * 1.1, sp.base)
        sp.silence(0.05)
        sp.consonant("h", "e", 0.6, sp.base)
        sp.vowel_seg("e", 0.08, 0.8, sp.base * 1.2, sp.base * 1.1)
        sp.consonant("m", "e", 0.6, sp.base * 1.1)
        return np.concatenate([sp.render(), np.zeros(d.n_of(0.1))])
    return np.zeros(1)


def speak(voice, mood, script, rng, pitch=1.0):
    """A script that may hold the extra sounds above: spoken in pieces and joined."""
    parts = re.split(r"(<(?:gulp|slurp|munch[^>]*|rumble|rimshot|mower[^>]*|guitar|claps|ahem|slidedown[^>]*|slideup)>)", script)
    out = []
    for part in parts:
        if not part.strip():
            continue
        m = re.fullmatch(r"<([^>]*)>", part.strip())
        if m and re.match(r"gulp|slurp|munch|rumble|rimshot|mower|guitar|claps|ahem|slide", m.group(1)):
            out.append(extra_sound(m.group(1), rng, voice))
        else:
            out.append(vo.say(voice, part, mood, rng, pitch))
    return np.concatenate(out) if out else np.zeros(1)


def speak_fit(voice, mood, script, rng, secs, pitch=1.0):
    """`speak`, dropping words from the end (then speaking faster) until the line fits the clip's length."""
    words = script.split()
    y = speak(voice, mood, script, rng, pitch)
    while len(y) > d.n_of(secs) and len(words) > 1:
        words = words[:-1]
        last = words[-1]
        if not last.startswith("<") and not re.search(r"[!?.~]$", last):
            words[-1] = last.rstrip(",") + ("!" if mood in ("happy", "excited", "angry") else ".")
        y = speak(voice, mood, " ".join(words), rng, pitch)
    if len(y) > d.n_of(secs):
        y = speak(vo.Voice(**{**vars(vo.VOICES[voice]), "rate": vo.VOICES[voice].rate * len(y) / d.n_of(secs) * 1.05}),
                  mood, " ".join(words), rng, pitch) if isinstance(voice, str) else y
    return y


def sing(rng, radio=False):
    """The pop singer's hook: a sung made-up line over a little beat (and through a small radio speaker)."""
    p = mu.Piece("singer" + ("radio" if radio else ""), 112)
    p.track("bass", "bass", 0.5, 0, 0.05)
    p.track("comp", "epiano", 0.3, -0.3, 0.2)
    p.track("dr", None, 0.35, 0, 0.1, kind="drums")
    chords = ["Am7", "Fmaj7", "C", "G"]
    mu.comp(p, "comp", 0, chords, 2, ((0, 1), (1, 1)))
    mu.bassline(p, "bass", 0, chords, 2, ((0, 1, 0), (1, 1, 12)))
    p.drums("dr", 0, 2, {"k": "x...x...x...x...", "c": "....x.......x...", "h": "x.x.x.x.x.x.x.x."})
    backing = p.render(8, master_lufs=-24)
    backing = backing.mean(1)
    # the sung line: syllables on notes of A minor
    v = vo.VOICES["singer"]
    notes = [(0.0, 76, 0.5, "ba"), (0.5, 76, 0.5, "by"), (1.0, 79, 1.0, "OO"), (2.0, 77, 0.5, "na"), (2.5, 76, 0.5, "na"),
             (3.0, 74, 1.0, "LEE"), (4.0, 72, 0.5, "oh"), (4.5, 74, 0.5, "ba"), (5.0, 76, 1.5, "BY~"), (6.5, 72, 1.0, "yeah")]
    beat = 60 / 112
    line = np.zeros(len(backing) + d.n_of(1))
    for b, m, ln, syl in notes:
        sp = vo.Speaker(v, "happy", rng)
        f = mu.hz(m - 12)
        sp.syllable(syl.strip("~"), syl.isupper(), f, ln * beat / 0.13 * 0.8, 1.0, f)
        y = sp.render()
        d.put(line, b * beat, y)
    x = backing * 0.5 + line[:len(backing)] * 1.0
    if radio:
        x = d.band(x, 400, 3500)
        x = d.softclip(x * 3, 2) * 0.5
    return x


# ---- the recipes ---------------------------------------------------------------------------------------------------------------

def stinger(name, kind):
    """The 'ta-da' fanfares and other little musical stingers, made with the music kit."""
    p = mu.Piece(name, 132)
    if kind == "tada":
        # the big one (a hole in one!): a brass 'ta-ta-ta-TAAA' with a snare roll and a cymbal, glockenspiel sparkle
        p.track("brass", "brass", 0.9, 0.0, 0.3, width=0.5)
        p.track("glock", "glock", 0.45, 0.3, 0.35)
        p.track("bass", "bass", 0.6, 0, 0.05)
        p.track("dr", None, 0.55, 0, 0.2, kind="drums")
        for k, m in enumerate((67, 67, 67)):
            for mm in (m, m - 5):
                p.note("brass", k / 3, mm, 0.25, 0.8)
        for m in (60, 64, 67, 72):
            p.note("brass", 1.0, m, 2.2, 1.0)
        for k, m in enumerate((84, 88, 91, 96)):
            p.note("glock", 1.0 + k * 0.12, m, 0.4, 0.6)
        p.note("bass", 1.0, 36, 2.2, 0.9)
        for k in range(12):
            p.note("dr", k / 12, "s", 0, 0.3 + 0.05 * k)
        p.note("dr", 1.0, "k", 0, 1.0)
        p.note("dr", 1.0, "o", 0, 1.0)
        p.note("dr", 1.0, "G", 0, 0.7)
        return p.render(4)
    if kind == "tada2":
        p.track("vib", "vibes", 0.8, -0.2, 0.3)
        p.track("pan", "steelpan", 0.9, 0.2, 0.25)
        p.track("pad", "pad", 0.35, 0, 0.4)
        p.track("bass", "bass", 0.6, 0, 0.05)
        p.track("dr", None, 0.5, 0, 0.2, kind="drums")
        mel = [(0, 79), (0.5, 77), (0.75, 79), (1.0, 84)]
        for b, m in mel:
            p.note("pan", b, m, 0.4, 0.85)
        for m in (65, 69, 72, 76):
            p.note("pad", 0, m, 1.0, 0.6)
        for m in (67, 71, 74, 77):
            p.note("pad", 0.5, m, 0.5, 0.6)
        for m in (72, 76, 79, 83, 86):
            p.note("vib", 1.0, m, 3, 0.85)
            p.note("pad", 1.0, m - 12, 3, 0.6)
        p.note("bass", 0, 41, 0.5, 0.8)
        p.note("bass", 0.5, 43, 0.5, 0.8)
        p.note("bass", 1.0, 36, 3, 0.9)
        p.drums("dr", 0, 1, {"k": "x...............", "x": "x.x.x.x.........", "c": "....x..........."})
        p.note("dr", 1.0, "k", 0, 0.9)
        p.note("dr", 1.0, "G", 0, 0.5)
        return p.render(5)
    if kind == "tada3":
        p.track("kal", "kalimba", 0.9, 0.0, 0.3)
        p.track("harp", "harp", 0.6, -0.3, 0.35)
        p.track("glock", "glock", 0.4, 0.3, 0.35)
        p.track("pad", "pad", 0.3, 0, 0.4)
        for k, m in enumerate((67, 72, 76, 79, 84, 88)):
            p.note("harp", k * 0.12, m - 12, 0.5, 0.7)
        for b, m in ((0.75, 79), (1.0, 81), (1.25, 83), (1.5, 84)):
            p.note("kal", b, m, 0.3, 0.85)
            p.note("glock", b, m + 12, 0.3, 0.5)
        for m in (60, 64, 67, 71, 74):
            p.note("pad", 1.5, m, 3, 0.6)
            p.note("harp", 1.5, m, 3, 0.6)
        return p.render(5)
    if kind == "twinkle":
        p.track("cel", "celesta", 0.8, 0.0, 0.45)
        p.track("glock", "glock", 0.4, 0.3, 0.45)
        for k, m in enumerate((84, 88, 91, 96, 100)):
            p.note("cel", k * 0.18, m, 0.3, 0.7)
            p.note("glock", k * 0.18 + 0.09, m + 7, 0.2, 0.4)
        return p.render(2.5)
    raise ValueError(kind)


def make(path, secs):
    """(samples, (target dB, how), fit) for one sound file of the game; fit False keeps the sound's own length."""
    low = path.lower()
    name = os.path.splitext(os.path.basename(low))[0].strip()
    rng = rng_for(path)
    # ---- music
    m = songs.piece(name, secs)
    if m is not None:
        return m, MUSIC, False
    # ---- voices
    if "/celebs/" in low and name in CELEBS:
        voice, mood, script = CELEBS[name]
        if script.startswith("<sing"):
            return sing(rng, "radio" in script), VOICE, True
        return speak_fit(voice, mood, script, rng, secs), VOICE, True
    if low.startswith("simsfx/") or low.startswith("sounds/male/") or low.startswith("sounds/female/") \
            or "vox_grunt" in name:
        if "grunt" in name:
            return vo.say("m1", "<grunt 1.1>", "angry", rng), VOICE, True
        if low.startswith("simsfx/"):
            key = name if name.startswith("s") else name[1:]  # fBAD0 -> bad0; sCOKE0 (a sip) keeps its s
        else:
            key = re.sub(r" (fm2|fm|m2|m)$", "", name)
        for pat, mood, script in LINES:
            if re.search(pat, key):
                y = speak_fit(voice_for(path), mood, script, rng, secs)
                # a long clip gets a second, different line after a pause
                if len(y) / SR < secs * 0.55 and secs > 1.4:
                    more = speak(voice_for(path), mood, " ".join(vo.babble(rng, 3)) + ("!" if mood in ("happy", "excited", "angry") else "."), rng)
                    y = np.concatenate([y, np.zeros(d.n_of(0.15)), more])
                return y, VOICE, True
        return speak(voice_for(path), "neutral", " ".join(vo.babble(rng, 4)) + ".", rng), VOICE, True
    if "/emotion/" in low:
        key = name.replace(" mix", "").replace(" ", "")
        mood_key = next(k for k in EMOTION if k in key)
        body = key.replace(mood_key, "")
        voice, pitch = BODY_VOICE.get(body, ("m1", 1.0))
        mood, s1, s2 = EMOTION[mood_key]
        script = s1 if int(rng.integers(2)) == 0 else s2
        if voice.startswith("f") and mood_key == "happy":
            script = "<giggle 5> la-la-LOO~"
        return speak_fit(voice, mood, script, rng, secs, pitch), VOICE, True
    # ---- celebrity things
    if "/celebs/" in low:
        if name.startswith("grill"):
            n = d.n_of(secs)
            t = d.t_axis(secs)
            sizzle = d.band(d.noise(n, rng), 2500, 9000) * (0.5 + 0.5 * (rng.uniform(size=n) < 0.3)) * 0.4
            pops = (rng.uniform(size=n) < 0.003) * rng.uniform(0.3, 1, n)
            y = sizzle + d.bp(pops, 3000, 1.0) * 2
            y *= np.minimum(1, t / 0.3) * np.clip((secs - t) / 0.6, 0, 1)
            if "good" in name:
                d.put(y, 1.0, speak("m1", "happy", "mm-MM! YUM-ba!", rng) * 2)
            if "bad" in name:
                d.put(y, 1.0, extra_sound("ahem", rng, "m2") * 2)
                d.put(y, 1.5, vo.say("m2", "<cough 2>", "disgust", rng) * 2)
            return y, CROWD, True
        if name.startswith("jacuzzi"):
            n = d.n_of(secs)
            t = d.t_axis(secs)
            bed = d.band(d.noise(n, rng), 300, 2500) * 0.5 * (0.7 + 0.3 * np.sin(2 * np.pi * 0.7 * t))
            bub = np.zeros(n)
            for k in range(int(secs * 25)):
                st = rng.uniform(0, secs - 0.05)
                ln = rng.uniform(0.01, 0.03)
                tt = d.t_axis(ln)
                d.put(bub, st, np.sin(2 * np.pi * np.cumsum(rng.uniform(400, 1200) * (1 + tt / ln)) / SR) * np.exp(-tt / (ln / 3)) * 0.3)
            y = (bed + bub) * np.minimum(1, t / 0.3) * np.clip((secs - t) / 0.6, 0, 1)
            if "bad" in name:
                d.put(y, 1.2, speak("m2", "disgust", "<grunt .5> ee-YOO, BLA-ga.", rng) * 2)
            else:
                d.put(y, 1.5, speak("f1", "happy", "<sigh .7> aah~", rng) * 1.5)
            return y, CROWD, True
        if any(w in name for w in ("bmw", "ferari", "lamborgini")):
            hi = 1.3 if "ferari" in name or "lamborgini" in name else 1.0
            y = fx.engine(rng, secs, rpm=(1200 * hi, 4500 * hi), cyl=8 if hi > 1 else 6, bright=2500, rough=0.1)
            return d.fade(y, 0.05, 0.15), ANIMAL, True
        if "classic car" in name:
            y = np.zeros(d.n_of(secs))
            d.put(y, 0, fx.horn(rng, 0.28, 300) * 0.8)
            d.put(y, 0.38, fx.horn(rng, 0.4, 260) * 0.8)
            return y, ANIMAL, True
        if "harley" in name:
            return d.fade(fx.vtwin(rng, secs), 0.05, 0.3), ANIMAL, True
        if name.startswith("punching"):
            y = np.zeros(d.n_of(secs))
            hits = (0.1, 0.45, 0.75, 1.3) if "good" in name else (0.1, 0.9)
            for k, at in enumerate(hits):
                d.put(y, at, fx.thunk(rng, 90, 0.3, 1200) * 1.2)
                d.put(y, at, d.band(d.noise(d.n_of(0.06), rng), 400, 3500) * d.perc(d.n_of(0.06), 0.001, 0.015) * 0.6)
            if "bad" in name:
                d.put(y, 0.5, fx.whoosh(rng, 0.3, 0.8))
                d.put(y, 1.3, speak("m1", "sad", "oof!", rng) * 1.5)
            else:
                d.put(y, 1.8, speak("m1", "excited", "HAH!", rng) * 1.5)
            return y, CROWD, True
        if name.startswith("arcade"):
            # chiptune blips (square waves, a few notes)
            seqs = {"arcade idle": [72, 76, 79, 76, 72, 76, 79, 84], "arcade play": [60, 67, 72, 67, 64, 71, 76, 71],
                    "arcade play good": [72, 76, 79, 84, 88, 91, 96], "arcade play bad": [79, 76, 72, 67, 64, 60, 55]}
            seq = seqs.get(name, seqs["arcade play"])
            y = np.zeros(d.n_of(secs))
            step = 0.12 if "good" in name or "bad" in name else 0.16
            for k, m in enumerate(seq):
                t = d.t_axis(step * 0.9)
                sq = np.sign(np.sin(2 * np.pi * mu.hz(m) * t)) * 0.3 + np.sign(np.sin(2 * np.pi * mu.hz(m) * 0.5 * t)) * 0.1
                d.put(y, k * step, d.lp(sq, 4000) * np.exp(-t / 0.15))
            if "idle" in name or name == "arcade play":
                n_ = d.n_of(secs)
                y = y[:n_]
            return y, ANIMAL, True
        if name.startswith("basketball"):
            y = np.zeros(d.n_of(secs))
            for k, at in enumerate((0.05, 0.45, 0.85)):
                t = d.t_axis(0.25)
                b = np.sin(2 * np.pi * np.cumsum(140 * (1 + 0.4 * np.exp(-t / 0.01))) / SR) * np.exp(-t / 0.06)
                b += d.bp(d.noise(len(t), rng), 1200, 2) * np.exp(-t / 0.01) * 0.4
                d.put(y, at, b * (1 - 0.1 * k))
            return y, ANIMAL, True
        return speak("m1", "neutral", "ba-la-DOO.", rng), VOICE, True
    # ---- golf
    if "drive with ball" in name or name == "drive":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0.0, fx.club_hit(rng, "driver"))
        if "ball" in name:
            d.put(y, 0.03, fx.ball_whistle(rng, 0.35) * 0.25)
        else:
            d.put(y, 0.01, fx.whoosh(rng, 0.3, 1.2, 0.05, 0.4))
        return y, GOLF, True
    if name == "tiger swing 1":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0.0, fx.whoosh(rng, 0.35, 1.1, 0.85, 1.0) * 0.6)
        d.put(y, 0.3, fx.club_hit(rng, "driver") * 1.1)
        d.put(y, 0.33, fx.ball_whistle(rng, 0.5) * 0.3)
        d.put(y, 0.5, vo.crowd_vowel(rng, 0.8, "u", 10, glide=(1.0, 1.25)) * 0.12)
        return y, GOLF, True
    if name.startswith("golf swing"):
        perfect = "perfect" in name
        y = np.zeros(d.n_of(secs))
        dur = 0.32 if "noimag" in name else 0.42
        d.put(y, 0.0, fx.whoosh(rng, dur, 1.15 if "fm" in name else 1.0, 0.75, 1.0 if perfect else 0.85))
        if "grunt" in name:
            d.put(y, 0.12, vo.say("m1", "<grunt 1.0>", "angry", rng) * 0.9)
        return y, GOLF, True
    if name in ("iron", "chip", "putt"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.club_hit(rng, name))
        if name == "iron":
            d.put(y, 0.02, fx.ball_whistle(rng, 0.25, 2700) * 0.15)
        return y, (GOLF[0] - (3 if name == "putt" else 1 if name == "chip" else 0), "fast"), True
    if "ball in hole" in name:
        return fx.cup_rattle(rng), GOLF, True
    if "ball drop sand" in name:
        return fx.sand_plop(rng, 0.8), LAND, True
    if "ball drop fairway" in name:
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.thud_grass(rng))
        d.put(y, 0.2, fx.thud_grass(rng, 0.35))
        d.put(y, 0.3, fx.roll_grass(rng, 0.35))
        if "lucky" in name:
            d.put(y, 0.45, fx.notes(rng, [79, 84, 88, 91], "celesta", 0.6, 0.07) * 0.5)
        return y, LAND, True
    if "ball tree leaves" in name:
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.leaves(rng, 1.3, 1.2))
        d.put(y, 0.03, fx.wood_knock(rng, 640, 0.3) * 0.6)
        d.put(y, 0.6, fx.thud_grass(rng, 0.4))
        return y, LAND, True
    if name in ("ball tree", "ball tree2"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.wood_knock(rng, 470 if name == "ball tree" else 610, 0.35))
        if name == "ball tree2":
            d.put(y, 0.22, fx.wood_knock(rng, 820, 0.25) * 0.4)
            d.put(y, 0.35, fx.leaves(rng, 0.3, 0.6) * 0.5)
        return y, LAND, True
    if "ball water" in name:
        return fx.bloop(rng, 1.0), LAND, True
    if "tiger bounce" in name:
        return fx.bounces(rng, 3, "grass"), LAND, True
    if "cart away" in name:
        return fx.electric_cart(rng, secs), (ANIMAL[0] - 2, "fast"), True
    if "open soda" in name:
        return fx.can_open(rng, secs, "loud" in name), (LAND[0] - (0 if "loud" in name else 3), "fast"), True
    if "put flag back" in name:
        return fx.flagstick(rng), LAND, True
    if name.startswith("applause"):
        if "bad" in name:
            y = fx.claps(rng, secs, people=8, rate=3.5, swell=0.4, fade=2.5) * 0.8
            d.put(y, 0.0, vo.crowd_vowel(rng, 1.4, "aw", 14, glide=(1.15, 0.7)) * 0.35)
            d.put(y, 1.3, fx.sad_trombone(rng) * 0.35)
            return y, CROWD, True
        good = "good" in name
        y = fx.claps(rng, secs, people=45 if good else 30, rate=5.5 if good else 4.5, swell=0.25, fade=2.5)
        if good:
            for k in range(5):
                at = rng.uniform(0.1, 1.6)
                v = ["m1", "m3", "f2", "m2", "f1"][k]
                d.put(y, at, vo.say(v, rng.choice(["woo!", "WOO-hoo!", "yay!", "ya-HOO!"]), "excited", rng) * 0.12)
            t = d.t_axis(0.6)
            whistle_ = np.sin(2 * np.pi * np.cumsum(2200 + 900 * np.sin(np.pi * t / 0.6)) / SR) * np.sin(np.pi * t / 0.6) * 0.05
            d.put(y, 0.9, whistle_)
        return y, CROWD, True
    if "ambience" in name:
        s = secs
        bed = fx.wind_bed(rng, s, 0.35, 0.4, 450)
        birds = fx.scatter(rng, s, lambda: fx.bird_song(rng, str(rng.choice(["warbler", "robin", "robin"]))), 0.45, gain=(0.08, 0.2))
        doves = fx.scatter(rng, s, lambda: fx.bird_song(rng, "dove"), 0.08, gain=(0.05, 0.1))
        leaves_ = fx.scatter(rng, s, lambda: fx.leaves(rng, 1.5, 0.5, 0.8), 0.15, gain=(0.03, 0.07))
        # the wind is a light breeze, not a rumble: its lows are cut (as a loop, so the seam stays clean)
        x = fx.periodic_filter(bed, d.hp4, 160) + birds + doves + leaves_
        if "wtr" in name:
            x += fx.lake_lap(rng, s) * 0.5
        return x, (BED[0] - 2, "integrated"), False
    # ---- interface
    if name == "button1":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.pop(rng, 1050, 0.09, 0.5))
        d.put(y, 0, fx.tick(rng, 3200, 0.02) * 0.25)
        return y, UI, True
    if name == "button2":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.pop(rng, 1000, 0.08, 0.4))
        d.put(y, 0.055, fx.pop(rng, 1500, 0.12, 0.3) * 0.8)
        return y, UI, True
    if name == "button3":
        return fx.pop(rng, 760, 0.1, 0.5, body=0.3), UI, True
    if name == "check box":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.tick(rng, 2800, 0.03) * 0.6)
        d.put(y, 0.01, fx.pop(rng, 1600, 0.1, 0.25) * 0.8)
        return y, UI, True
    if name == "text box":
        return fx.tick(rng, 3000, 0.03), (UI[0] - 4, "fast"), True
    if name == "unavailable":
        return d.lp(fx.pop(rng, 320, 0.16, 0.3, body=0.5), 1500), UI, True
    if name == "wrong":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.bell_note(rng, 64, 0.12, 0.7, "marimba"))
        d.put(y, 0.11, fx.bell_note(rng, 60, 0.2, 0.7, "marimba"))
        return d.lp(y, 3000), UI, True
    if name == "slide pop up":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.swoosh_up(rng, 0.22, 500, 3000) * 0.5)
        d.put(y, 0.18, fx.pop(rng, 1300, 0.1, 0.4))
        d.put(y, 0.2, fx.notes(rng, [91, 96], "celesta", 0.4, 0.06) * 0.25)
        return y, UI, True
    if name in ("bass up 2", "bass down 2", "violin up 2", "violin down 2"):
        up = "up" in name
        seq = [60, 64, 67] if up else [67, 64, 60]
        instr = "marimba" if "bass" in name else "pizz"
        if "bass" in name:
            seq = [m - 12 for m in seq]
        return fx.notes(rng, seq, instr, 0.75, 0.07, dur=secs), UI, True
    if name == "building":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.thunk(rng, 150, 0.35) * 0.9)
        d.put(y, 0.08, fx.notes(rng, [72, 79, 84], "glock", 0.55, 0.05) * 0.4)
        return y, UI, True
    if name == "bench":
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.wood_knock(rng, 310, 0.3))
        d.put(y, 0.12, fx.wood_knock(rng, 360, 0.25) * 0.6)
        return y, UI, True
    if name == "bridge":
        y = np.zeros(d.n_of(secs))
        for k in range(4):
            d.put(y, k * 0.09, fx.wood_knock(rng, 260 + 40 * k, 0.3) * (1 - 0.15 * k))
        return y, UI, True
    if name == "path":
        return fx.gravel(rng, secs, 2), UI, True
    if name.startswith("place bunker"):
        return fx.sand_pour(rng, 0.55), UI, True
    if name in ("place fairway", "place greentee", "place rough"):
        return fx.grass_tuft(rng, 0.35, 1.2 if "green" in name else 0.8 if "rough" in name else 1.0), UI, True
    if name in ("place water", "place waterrocks"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.water_plop(rng, 0.8, 0.9) * 0.8)
        d.put(y, 0.15, fx.lake_lap(rng, 0.9)[:d.n_of(0.9)] * np.hanning(d.n_of(0.9)) * 0.5)
        if "rocks" in name:
            d.put(y, 0.25, fx.rocks(rng, 0.6, 2) * 0.6)
        return y, UI, True
    if name.startswith("place rocks"):
        return fx.rocks(rng, min(secs, 0.9), 3 if name == "place rocks 1" else 4), UI, True
    if name == "test 7":
        # the painting tune's note (the engine pitches it to each note of its tune): a soft mallet F with a little turf
        y = np.zeros(d.n_of(secs))
        f = mu.hz(77)
        d.put(y, 0, mu.marimba(f, 0.3, 0.7, rng) * mu.calibration("marimba"))
        d.put(y, 0, mu.kalimba(f, 0.3, 0.6, rng) * mu.calibration("kalimba") * 0.5)
        d.put(y, 0, fx.grass_tuft(rng, 0.3) * 0.15)
        return y, UI, True
    if name in ("test 2", "test 4", "old test 7"):  # tees and greens (Test 2): fine turf rolled out
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.grass_tuft(rng, 0.4, 1.3))
        d.put(y, 0.06, fx.leaves(rng, 0.6, 0.6, 1.2) * 0.35)
        if name != "test 2":
            d.put(y, 0.1, fx.notes(rng, [84, 91], "celesta", 0.4, 0.08) * 0.3)
        return y, (UI[0] - 2, "fast"), True
    if name == "test 9":  # rough
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.grass_tuft(rng, 0.45, 0.7))
        d.put(y, 0.04, fx.leaves(rng, 0.8, 1.2, 0.8) * 0.5)
        return y, (UI[0] - 2, "fast"), True
    if name == "test 10":  # sand
        return fx.sand_pour(rng, 0.9, 1.1), (UI[0] - 3, "fast"), True
    # ---- effects
    if name == "cash":
        return fx.coins(rng, min(secs, 1.6)), STING, True
    if name.startswith("tada"):
        kind = {"tada": "tada", "tada 2": "tada2", "tada 3": "tada3"}[name]
        x = stinger(name, kind).mean(1)
        return d.fade(x[:d.n_of(secs)], 0, 0.6), STING, True
    if name == "twinkle":
        return stinger(name, "twinkle").mean(1), STING, True
    if name == "camera":
        return fx.shutter(rng), UI, True
    if name == "boing":
        return fx.boing(rng, 0.55), UI, True
    if name == "belch":
        v = vo.Voice(85, 0.85, breath=0.1, tension=0.8, jitter=0.05, creak=0.9)
        sp = vo.Speaker(v, "neutral", rng)
        sp.vowel_seg("aw", 0.25, 1.0, 80, 65, end_vowel="o")
        return sp.render(), VOICE, True
    if name.startswith("club 2 drop"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.club_rattle(rng, 1.4))
        if "no wtr" not in name:
            d.put(y, 0.5, fx.water_plop(rng, 1.6, 1.4) * 0.8)
        return y, LAND, True
    if name in ("desert1", "desert2", "desert3"):
        s = secs
        y = fx.wind_bed(rng, s, 1.0, 0.8, 380, loop=False) * np.minimum(1, d.t_axis(s) / 0.3)
        if name != "desert1":
            d.put(y, s * 0.3, fx.bird_song(rng, "hawk") * 0.25)
        if name == "desert3":
            y += fx.insects(rng, s, "cicada") * 0.3
        return d.fade(y, 0.2, 0.6), (BED[0] + 6, "fast"), True
    if name == "grass3":
        s = secs
        y = fx.wind_bed(rng, s, 0.5, 0.4, 500, loop=False) + fx.insects(rng, s, "cricket") * 0.25
        d.put(y, 0.4, fx.bird_song(rng, "robin") * 0.3)
        return d.fade(y, 0.2, 0.6), (BED[0] + 6, "fast"), True
    if name == "spring":
        s = secs
        y = fx.wind_bed(rng, s, 0.25, 0.3, 600, loop=False)
        for k in range(4):
            d.put(y, 0.1 + k * 0.7, fx.bird_song(rng, ["warbler", "robin"][k % 2]) * 0.35)
        return d.fade(y, 0.1, 0.5), (ANIMAL[0], "fast"), True
    if name == "winter":
        s = secs
        y = fx.wind_bed(rng, s, 1.0, 0.9, 700, loop=False)
        return d.fade(y, 0.4, 0.8), (BED[0] + 6, "fast"), True
    if name == "lake1":
        s = secs
        y = fx.lake_lap(rng, s) + fx.scatter(rng, s, lambda: fx.frog(rng), 0.6, loop=False, gain=(0.1, 0.25))
        return d.fade(y, 0.3, 0.6), (BED[0] + 6, "fast"), True
    if name.startswith("alligator"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0.1, fx.gator(rng, 1.8))
        d.put(y, 2.2, fx.gator(rng, 1.4) * 0.7)
        return y, ANIMAL, True
    if name == "dolphin":
        return fx.dolphin(rng), ANIMAL, True
    if name == "fly":
        return fx.fly(rng, min(secs, 3.2)), (ANIMAL[0] - 4, "fast"), True
    if name.startswith("gila"):
        return fx.hiss(rng, 1.4, 4000), ANIMAL, True
    if name == "snake":
        y = fx.rattle(rng, 1.6)
        d.put(y, 0.9, fx.hiss(rng, 0.7, 6000) * 0.5)
        return y, ANIMAL, True
    if name.startswith("moose"):
        return fx.moose(rng, 1.5), ANIMAL, True
    if name.startswith("sheep"):
        return fx.bleat(rng, 300 if name == "sheep 1" else 360, 0.7), ANIMAL, True
    if name.startswith("seriama"):
        return fx.seriama(rng), ANIMAL, True
    if name.startswith("snore"):
        return fx.snore(rng, min(secs, 3.6)), (ANIMAL[0] - 3, "fast"), True
    if name.startswith("hot air"):
        return fx.burner(rng, min(secs, 3.4)), ANIMAL, True
    if name in ("flower bed", "maple", "palm", "tree sprout 3"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.thunk(rng, 110, 0.25, 1500) * 0.6)
        d.put(y, 0.02, fx.leaves(rng, 0.7 if name != "flower bed" else 1.0, 0.8, 1.1) * 0.6)
        tune = {"flower bed": [79, 84, 88, 91], "maple": [74, 79, 83], "palm": [76, 81, 84], "tree sprout 3": [72, 76, 79]}[name]
        d.put(y, 0.12, fx.notes(rng, tune, "celesta", 0.5, 0.06) * 0.45)
        return y, UI, True
    if name.startswith("taunt") or name.startswith("unrest"):
        s = secs
        y = fx.murmur(rng, s, 8, 1.0)
        if name.startswith("taunt"):
            d.put(y, 0.8, vo.crowd_vowel(rng, 1.2, "o", 12, glide=(0.9, 1.3)) * 0.4)
            d.put(y, 2.6, vo.crowd_vowel(rng, 1.3, "aw", 12, glide=(1.1, 0.8)) * 0.35)
        else:
            d.put(y, 1.5, vo.crowd_vowel(rng, 1.6, "uh", 14, base=120, glide=(1.0, 0.9)) * 0.3)
        return d.fade(y, 0.4, 0.8), CROWD, True
    if name.startswith("timetunnel"):
        y = np.zeros(d.n_of(secs))
        d.put(y, 0, fx.swoosh_up(rng, 0.8, 200, 5000) * 0.6)
        d.put(y, 0.05, fx.notes(rng, [72, 79, 84, 88, 91, 96], "celesta", 0.45, 0.06) * 0.4)
        return y, STING, True
    if name == "shotgun":
        y = np.zeros(d.n_of(secs))
        t = d.t_axis(0.3)
        bang = d.lp(d.noise(len(t), rng), 2500) * d.perc(len(t), 0.0005, 0.03)
        bang += np.sin(2 * np.pi * np.cumsum(120 * np.exp(-t / 0.05) + 50) / SR) * d.perc(len(t), 0.001, 0.08)
        d.put(y, 0, bang)
        d.put(y, 0.18, d.lp(bang, 900) * 0.25)
        d.put(y, 0.38, d.lp(bang, 600) * 0.1)
        return y, (GOLF[0] - 2, "fast"), True
    return None, None, True


def render(path, secs):
    x, target, fit = make(path, secs)
    if x is None:
        return path, None, None
    db, how = target
    x = np.asarray(x, float)
    name = os.path.splitext(os.path.basename(path.lower()))[0].strip()
    loop = "ambience" in name or name in songs.LOOPED
    if loop:
        # a loop is levelled as a loop (three copies, the middle kept), so its filters and limiter leave no seam
        n = len(x)
        x = d.master(np.concatenate([x, x, x]), db, how)[n:2 * n]
        return path, x, (db, how)
    if how == "fast":
        x = master_fast(x, db)
    else:
        x = d.master(x, db, how)
    if fit:
        x = d.fit(x, secs, 0.03)
    # no clicks at the ends
    x = d.fade(x, 0.0015, 0.004)
    return path, x, (db, how)


def fast_loudness(x, win=0.1):
    p = d._power(x)
    k = d.n_of(win)
    if len(p) <= k:
        return -0.691 + 10 * np.log10(np.sum(p) / k + 1e-12)
    cs = np.concatenate([[0], np.cumsum(p)])
    return -0.691 + 10 * np.log10(((cs[k:] - cs[:-k]) / k).max() + 1e-12)


def master_fast(x, db):
    x = d.hp(x, 18) if x.ndim == 1 else np.stack([d.hp(x[:, c], 18) for c in range(2)], 1)
    for _ in range(3):
        x = x * 10 ** ((db - fast_loudness(x)) / 20)
        if d.true_peak(x) <= -1.0:
            break
        x = d.limit(x, -1.6)
    tp = d.true_peak(x)
    if tp > -1.0:
        x *= 10 ** ((-1.0 - tp) / 20)
    return x


def write_ogg(path, x):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    ch = 1 if x.ndim == 1 else 2
    subprocess.run(["ffmpeg", "-v", "quiet", "-y", "-f", "f32le", "-ar", str(SR), "-ac", str(ch), "-i", "-",
                    "-c:a", "libvorbis", "-q:a", "0", "-map_metadata", "-1", "-fflags", "+bitexact", path],
                   input=x.astype(np.float32).tobytes(), check=True)


def entries():
    out = []
    for line in open(layout.MANIFEST, encoding="utf-8"):
        if line.startswith("W "):
            parts = line.rstrip("\n").split(" ")
            out.append((" ".join(parts[1:-1]), float(parts[-1])))
    return out


def job(args):
    path, secs = args
    try:
        path, x, target = render(path, secs)
    except Exception:  # every failing recipe is reported, not just the first
        import traceback
        return path, traceback.format_exc()
    if x is None:
        return path, None
    out = os.path.join(OUT, path[:path.rfind(".")] + ".ogg")
    write_ogg(out, x)
    return path, (len(x) / SR, target, round(d.lufs(x), 1), round(d.true_peak(x), 1))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 2)
    a = ap.parse_args()
    todo = [(p, s) for p, s in entries() if p not in ALIASES and (not a.only or re.search(a.only, p, re.I))]
    # files that only stand for another are not kept
    for p in ALIASES:
        f = os.path.join(OUT, p[:p.rfind(".")] + ".ogg")
        if os.path.exists(f):
            os.remove(f)
    done = 0
    failed = False
    with ProcessPoolExecutor(a.jobs) as ex:
        for path, info in ex.map(job, todo):
            if info is None or isinstance(info, str):
                print("FAILED:" if info else "no recipe:", path, info or "")
                failed = True
                continue
            done += 1
            print("%-52s %6.2fs  target %s  I %6.1f  TP %5.1f" % (path, info[0], info[1], info[2], info[3]), flush=True)
    print(done, "sounds")
    if failed:
        sys.exit(1)
    import build
    build.write_index()


if __name__ == "__main__":
    main()

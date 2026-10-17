"""The free set's music, written for OpenSGolf: a main theme and a theme for each course style (tropical, desert, links), a
calm piece, a jazz piece, and jingles drawn from them, arranged per slot of the game (title, customise, accomplishments,
year end, story chapters, the in-game playlists, the credits). All melodies and arrangements are our own.

    piece(name, secs) -> stereo samples (n, 2), or None when the name is not music"""
import numpy as np

import snd_dsp as d
import snd_music as mu

# ---- themes: chords per bar and melodies ('note:beats') --------------------------------------------------------------------

MAIN_A = ["Fmaj7", "Am7", "Bbmaj7", "C7sus4", "Fmaj7", "Dm7", "Gm7", "C7"]
MAIN_A_MEL = """r:.5 C5:.5 F5:.5 G5:.5 A5:1.5 C6:.5 | A5:1 G5:.5 E5:.5 C5:2 | r:.5 D5:.5 F5:.5 A5:.5 Bb5:1.5 A5:.5 |
G5:1 F5:.5 G5:.5 E5:2 | r:.5 C5:.5 F5:.5 G5:.5 A5:1.5 C6:.5 | D6:1 C6:.5 A5:.5 F5:1.5 G5:.5 |
A5:1.5 Bb5:.5 A5:.5 G5:.5 F5:.5 D5:.5 | E5:1 F5:.5 G5:.5 G5:2"""
MAIN_B = ["Bbmaj7", "Bbm6", "Am7", "D7", "Gm7", "Gm7/C", "Fmaj7", "C7"]
MAIN_B_MEL = """F5:1.5 G5:.5 A5:1 D6:1 | C6:1 Bb5:.5 G5:.5 F5:2 | E5:.5 A5:.5 C6:1.5 A5:.5 G5:1 |
F#5:1.5 A5:.5 D6:2 | Bb5:1 A5:.5 G5:.5 D6:2 | C6:1 Bb5:.5 A5:.5 G5:1.5 F5:.5 | A5:2 G5:.5 F5:.5 E5:1 |
G5:1 E5:.5 G5:.5 Bb5:1 C6:1"""
MAIN_END = ["Gm7", "C7sus4", "Fmaj9"]
MAIN_END_MEL = "A5:1.5 Bb5:.5 A5:.5 G5:.5 F5:.5 D5:.5 | E5:1 F5:.5 G5:.5 C6:2 | A5:4"

CALM = ["Dmaj7", "Gmaj7", "Em9", "A7sus4", "Bm7", "Gmaj7", "Em7", "Aadd9"]
CALM_MEL = """F#5:2 A5:1 C#6:1 | B5:3 A5:1 | G5:1.5 F#5:.5 E5:2 | D5:2 E5:2 | F#5:2 B5:1 D6:1 | C#6:2 B5:1 A5:1 |
G5:1 B5:1 A5:1 G5:1 | E5:4"""

TROP_A = ["G", "C", "D7", "G", "G", "C", "Am7", "G"]
TROP_A_MEL = """B4:.5 D5:.5 G5:1 F#5:.5 G5:.5 A5:1 | G5:.5 E5:.5 C5:1 r:.5 E5:.5 G5:1 | F#5:.5 A5:.5 C6:1 B5:.5 A5:.5 F#5:1 |
G5:2 r:.5 D5:.5 E5:.5 F#5:.5 | B4:.5 D5:.5 G5:1 F#5:.5 G5:.5 B5:1 | C6:1 B5:.5 A5:.5 G5:.5 E5:.5 G5:1 |
A5:.5 C6:.5 B5:.5 A5:.5 F#5:.5 A5:.5 D5:1 | G5:3 r:1"""
TROP_B = ["Em7", "Am7", "D7", "G", "C", "Cm6", "G/D", "D7"]
TROP_B_MEL = """G5:1 B5:1 D6:1.5 B5:.5 | C6:1 A5:1 E5:2 | F#5:.5 G5:.5 A5:1 C6:1 A5:1 | B5:2 G5:2 |
E5:.5 G5:.5 C6:1 E6:1 D6:1 | Eb6:1.5 D6:.5 C6:1 A5:1 | B5:1 G5:1 D5:1 E5:.5 F#5:.5 | A5:2 F#5:1 D5:1"""

DES_A = ["Am7", "Dm7", "G7", "Cmaj7", "Fmaj7", "Dm7", "E7sus4", "E7"]
DES_A_MEL = """E5:1.5 C5:.5 A4:1 B4:.5 C5:.5 | D5:1.5 F5:.5 A5:1.5 G5:.5 | F5:1 D5:.5 B4:.5 G4:1 A4:.5 B4:.5 |
C5:1 E5:.5 G5:.5 B5:2 | A5:1.5 G5:.5 F5:1 E5:.5 D5:.5 | F5:1 E5:.5 D5:.5 C5:1 D5:1 | E5:2 D5:1 B4:1 |
G#4:1 B4:1 D5:1 E5:1"""
DES_B = ["Fmaj7", "E7", "Am7", "Gsus4", "Fmaj7", "E7", "Am9", "Am9"]
DES_B_MEL = """C6:1.5 B5:.5 A5:1 F5:1 | G#5:1.5 B5:.5 E5:2 | C5:.5 E5:.5 A5:1 C6:1 B5:1 | A5:1 G5:1 D5:2 |
A5:1 C6:1 F5:1 A5:1 | G#5:1 E5:1 B4:1 D5:1 | C5:2 B4:.5 C5:.5 E5:1 | A4:4"""

# links: an air in 6/8 (beats are eighths)
LINKS_A = ["D", "C", "G", "D", "D", "C", "Em", "A7sus4"]
LINKS_A_MEL = """F#5:2 A5:1 D6:2 B5:1 | C6:2 A5:1 G5:2 E5:1 | D5:1 E5:1 G5:1 B5:2 A5:1 | A5:3 F#5:2 E5:1 |
F#5:2 A5:1 D6:2 E6:1 | E6:2 C6:1 A5:2 G5:1 | G5:2 B5:1 E6:2 D6:1 | D6:3 A5:3"""
LINKS_B = ["G", "D/F#", "Em", "D", "G", "D/F#", "Em7", "A7"]
LINKS_B_MEL = """B5:3 D6:2 B5:1 | A5:3 F#5:2 D5:1 | E5:2 G5:1 B5:2 G5:1 | F#5:3 A5:3 | B5:2 D6:1 G6:2 F#6:1 |
E6:2 D6:1 A5:2 F#5:1 | G5:2 E5:1 B5:2 G5:1 | A5:3 C#6:3"""
# the clubhouse air in 4/4 (harpsichord pieces)
CLUB = ["D", "A/C#", "Bm", "G", "D/F#", "Em7", "A7sus4", "A7"]
CLUB_MEL = """A5:1 F#5:.5 G5:.5 A5:1 D6:1 | C#6:1 E5:.5 F#5:.5 A5:2 | B5:1 D6:.5 C#6:.5 B5:1 F#5:1 |
G5:1 B5:.5 A5:.5 G5:1 D5:1 | F#5:.5 G5:.5 A5:.5 B5:.5 A5:1 D5:1 | G5:1 F#5:.5 E5:.5 B5:2 |
A5:1 D6:1 E6:1 D6:1 | C#6:2 E6:2"""

JAZZ = ["Gm7", "C7", "Fmaj7", "Dm7", "Gm7", "C7", "Am7", "D7", "Gm7", "C7", "Fmaj7", "D7b9", "Gm7", "C7", "Fmaj7", "Fmaj7"]
JAZZ_MEL = """r:.5 D5:.5 F5:.5 A5:.5 C6:1 Bb5:1 | A5:.5 G5:.5 E5:1 C5:2 | r:.5 C5:.5 E5:.5 G5:.5 A5:1.5 G5:.5 |
F5:1 E5:.5 D5:.5 A5:2 | r:.5 D5:.5 F5:.5 A5:.5 C6:1 D6:1 | E6:.5 D6:.5 C6:.5 Bb5:.5 G5:2 |
A5:.5 C6:.5 E6:1 D6:.5 C6:.5 A5:1 | F#5:1 A5:.5 C6:.5 Eb6:2 | D6:1.5 C6:.5 Bb5:1 G5:1 | E5:1 G5:.5 Bb5:.5 E6:2 |
D6:1 C6:.5 A5:.5 E5:2 | F#5:1 Eb6:.5 D6:.5 C6:2 | Bb5:.5 A5:.5 G5:.5 F5:.5 D5:1 G5:1 | E5:1 G5:.5 Bb5:.5 D6:1 C6:1 |
A5:4 | r:4"""

STORY = ["C", "Am7", "Dm7", "G7", "C", "Fmaj7", "Dm7 ", "G7"]
STORY_MEL = """E5:.5 G5:.5 C6:1 B5:.5 A5:.5 G5:1 | A5:.5 G5:.5 E5:1 C5:2 | D5:.5 F5:.5 A5:1 G5:.5 F5:.5 D5:1 |
B4:1 D5:.5 F5:.5 G5:2 | E5:.5 G5:.5 C6:1 D6:.5 C6:.5 G5:1 | A5:1.5 G5:.5 F5:.5 E5:.5 C5:1 | D5:1 F5:.5 A5:.5 C6:1 B5:1 |
G5:2 D5:1 B4:1"""

# a cheeky little rag (story chapters): stride bass, off-beat chords, a syncopated tune
RAG = ["C", "A7", "D7", "G7", "C", "C7", "F", "G7"]
RAG_MEL = """E5:.5 G5:.5 C6:.75 G5:.25 A5:.5 G5:.5 E5:1 | E5:.5 G5:.5 C#6:.75 A5:.25 G5:.5 E5:.5 C#5:1 |
F#5:.5 A5:.5 D6:.75 C6:.25 A5:.5 F#5:.5 D5:1 | G5:.5 B5:.5 D6:.5 F6:.5 E6:.5 D6:.5 B5:1 |
E5:.5 G5:.5 C6:.75 G5:.25 A5:.5 G5:.5 E5:1 | Bb5:.75 A5:.25 G5:.5 E5:.5 C5:1 r:1 |
A5:.5 C6:.5 F6:.75 E6:.25 D6:.5 C6:.5 A5:1 | G5:.5 F5:.5 D5:.5 B4:.5 C5:2"""

WALTZ = ["Fmaj7", "Dm7", "Gm7", "C7", "Am7", "D7", "Gm7", "C7"]
WALTZ_MEL = """C5:1 F5:1 A5:1 | A5:2 G5:1 | Bb5:1 D6:1 C6:1 | Bb5:2 G5:1 | C6:1 E5:1 A5:1 | F#5:2 A5:1 |
D6:1.5 C6:.5 Bb5:1 | G5:2 E5:1"""

SAIL = ["G", "C", "D", "G"]
SAIL_MEL = "D5:1 G5:2 B5:1 A5:2 | E5:1 G5:2 C6:1 B5:2 | A5:1 F#5:2 D5:1 E5:1 F#5:1 | G5:6"


# ---- styles: how a section is played -------------------------------------------------------------------------------------------

def _sec(p, at, chords, bpb, mel, style, level, mel_instr=None, mel_vel=1.0, transpose=0):
    """Plays one section of chords (one bar each) with its melody in a style, at a density `level` 0..3."""
    beats = len(chords) * bpb
    chords = [c.strip() for c in chords]
    s = style
    if mel:
        lead = mel_instr or s["lead"]
        _ensure(p, lead, ROLE["lead"] * s.get("lead_gain", 1.0), s.get("lead_rev", 0.25))
        p.melody(lead, at, mel, transpose, mel_vel)
        if s.get("double") and level >= 2:
            _ensure(p, s["double"], ROLE["double"] * s.get("double_gain", 1.0), 0.3, 0.25)
            p.melody(s["double"], at, mel, transpose + 12, mel_vel * 0.55)
    if s.get("pad") and (level <= 1 or s.get("pad_always")):
        mu.pads(p, "pad", at, chords, bpb, vel=0.5 if level else 0.65)
    if s.get("comp") and level >= 1:
        mu.comp(p, "comp", at, chords, bpb, s.get("comp_rhythm", ((0, 1.5), (1.5, 1), (3, 1))), *s.get("comp_range", (53, 72)),
                vel=s.get("comp_vel", 0.6))
    if s.get("arp") and level >= s.get("arp_from", 1):
        mu.arp(p, "arp", at, chords, bpb, s.get("arp_step", 0.5), s.get("arp_order", (0, 1, 2, 3, 2, 1)),
               *s.get("arp_range", (62, 84)), vel=s.get("arp_vel", 0.45))
    if s.get("bass") and level >= 1:
        if s.get("walk"):
            walking(p, "bass", at, chords, bpb)
        else:
            mu.bassline(p, "bass", at, chords, bpb, s.get("bass_pattern", ((0, 1.5, 0), (1.5, 0.5, 7), (2, 1.5, 0), (3.5, 0.5, 12))),
                        s.get("bass_oct", 36))
    if s.get("drums") and level >= 2:
        pat = s["drums"] if level >= 3 or "drums_light" not in s else s["drums_light"]
        p.drums("drums", at, len(chords), pat, beats=bpb)
    elif s.get("drums_soft") and level >= 1:
        p.drums("drums", at, len(chords), s["drums_soft"], beats=bpb)
    return beats


def walking(p, track, at, chords, bpb, octave=36):
    """A walking bass: root, chord tones, and a chromatic step into the next chord's root."""
    for k, sym in enumerate(chords):
        root, q, bpc = mu.chord(sym)
        nxt = mu.chord(chords[(k + 1) % len(chords)])[2]
        r = octave + bpc
        tones = [r, r + q[1], r + q[2], None]
        n_next = octave + nxt
        if n_next - r > 6:
            n_next -= 12
        tones[3] = n_next + (1 if (k % 2) else -1)
        for i, m in enumerate(tones[:bpb]):
            p.note(track, at + k * bpb + i, m, 0.95, 0.85 if i == 0 else 0.7)


def _ensure(p, instr, gain, rev, pan=0.0):
    """The melody track of an instrument (named after it)."""
    p.track(instr, instr, gain, pan, rev, width=0.3)


# the mix by role (instruments are calibrated to one loudness, snd_music.calibration); a style's *_gain scales its role
ROLE = dict(lead=1.0, double=0.38, pad=0.32, comp=0.36, arp=0.3, bass=0.62, drums=0.35)
# (a style's double_gain scales the octave doubling)


def setup(p, style):
    s = style
    g = lambda role: ROLE[role] * s.get(role + "_gain", 1.0)  # noqa: E731
    _ensure(p, s["lead"], g("lead"), s.get("lead_rev", 0.25))
    if s.get("double"):
        _ensure(p, s["double"], g("double"), 0.3, 0.25)
    if s.get("pad"):
        p.track("pad", s["pad"], g("pad"), 0.0, 0.4, width=0.8)
    if s.get("comp"):
        p.track("comp", s["comp"], g("comp"), s.get("comp_pan", -0.25), 0.22, width=0.4)
    if s.get("arp"):
        p.track("arp", s["arp"], g("arp"), s.get("arp_pan", 0.35), 0.3, width=0.3)
    if s.get("bass"):
        p.track("bass", s["bass"], g("bass"), 0.0, 0.03)
    if s.get("drums") or s.get("drums_soft"):
        p.track("drums", None, g("drums"), 0.0, 0.12, kind="drums", width=0.2)


POP = dict(lead="marimba", double="glock", pad="pad", comp="epiano", arp="pluck", bass="bass",
           comp_rhythm=((0, 1.5), (1.5, 1.0), (2.5, 0.5), (3, 1)), arp_order=(0, 2, 1, 3, 2, 1, 3, 2), arp_vel=0.38,
           drums={"k": "x.....x.x.......", "r": "....x.......x...", "x": "xoxoxoxoxoxoxoxo", "h": "..x...x...x...x."},
           drums_light={"k": "x.......x.......", "x": "x.o.x.o.x.o.x.o."},
           bass_pattern=((0, 1.5, 0), (1.5, 0.5, 7), (2, 1.0, 0), (3, 0.5, 12), (3.5, 0.5, 7)))
RAG_STYLE = dict(lead="piano", double="glock", comp="piano", comp_rhythm=((1, 0.5), (3, 0.5)), comp_range=(55, 67),
                 comp_vel=0.55, bass="piano", bass_pattern=((0, 0.5, 0), (2, 0.5, 7)), bass_oct=36,
                 drums={"b": "....x.......x...", "w": "x.......x.......", "x": "x.x.x.x.x.x.x.x."},
                 drums_light={"w": "x.......x......."}, comp_gain=1.4, bass_gain=2.4, double_gain=0.6, drums_gain=2.0)
BOSSA = dict(lead="vibes", double="flute", comp="nylon", comp_rhythm=((0, 0.5), (0.75, 0.5), (1.5, 0.75), (2.5, 0.5), (3.25, 0.5)),
             comp_range=(52, 68), comp_vel=0.5, bass="upright", bass_pattern=((0, 1.5, 0), (1.5, 0.5, 7), (2, 1.5, 7), (3.5, 0.5, 0)),
             drums={"r": "x..x..x...x..x..", "x": "xoxoxoxoxoxoxoxo", "k": "x..x....x..x...."},
             drums_light={"r": "x..x..x...x..x..", "x": "x.o.x.o.x.o.x.o."}, pad="epiano", pad_gain=0.6, comp_gain=1.2,
             bass_gain=1.2)
CALM_STYLE = dict(lead="flute", double="celesta", pad="pad", pad_always=True, arp="celesta", arp_step=0.5,
                  arp_order=(0, 2, 3, 1, 2, 3), arp_range=(64, 88), arp_vel=0.35, comp="harp",
                  comp_rhythm=((0, 4),), comp_range=(50, 66), comp_vel=0.45, bass="bass", bass_pattern=((0, 4, 0),),
                  drums_soft={"x": "..x...x...x...x."}, arp_gain=1.4)
TROP = dict(lead="steelpan", double="marimba", comp="marimba", comp_rhythm=((0.5, 0.25), (1.5, 0.25), (2.5, 0.25), (3.5, 0.25)),
            comp_range=(60, 76), comp_vel=0.5, arp="uke", arp_step=0.5, arp_order=(0, 1, 2, 1), arp_range=(55, 72), arp_vel=0.4,
            bass="bass", bass_pattern=((0, 1, 0), (1.5, 0.5, 7), (2, 1, 0), (3, 0.5, 7), (3.5, 0.5, 12)),
            drums={"k": "x.......x.......", "g": "..x..x....x..xo.", "n": "x..x..x...x.x..x", "x": "xoxoxoxoxoxoxoxo",
                   "w": "......x.......x."},
            drums_light={"x": "xoxoxoxoxoxoxoxo", "g": "..x.......x....."})
DESERT = dict(lead="panflute", double="kalimba", comp="nylon", comp_rhythm=((0, 1), (1, 0.5), (1.5, 0.5), (2.5, 1), (3.5, 0.5)),
              comp_range=(52, 70), comp_vel=0.55, arp="nylon", arp_step=0.5, arp_order=(0, 2, 1, 3, 2, 1, 2, 3),
              arp_range=(57, 76), arp_vel=0.42, bass="upright", bass_pattern=((0, 1.5, 0), (1.5, 0.5, 7), (2, 1.5, 0)),
              drums={"j": "x.....x...x.....", "J": "....x.......x...", "x": "x.xox.xox.xox.xo", "a": "..........x.x..."},
              drums_light={"x": "x.xox.xox.xox.xo", "j": "x.........x....."}, pad="pad")
LINKS = dict(lead="whistle", double="harp", pad="pad", pad_always=True, arp="harp", arp_step=1,
             arp_order=(0, 1, 2, 3, 2, 1), arp_range=(55, 79), arp_vel=0.45, bass="bass", bass_pattern=((0, 3, 0), (3, 3, 7)),
             bass_oct=36, drums={"d": "X..x..X..xo.", "t": "...x.....x.."}, drums_light={"d": "x.....x....."},
             lead_gain=0.85)
CLUBHOUSE = dict(lead="flute", double="glock", comp="harpsichord", comp_rhythm=((0, 0.5), (0.5, 0.5), (1, 0.5), (1.5, 0.5),
                                                                                   (2, 0.5), (2.5, 0.5), (3, 0.5), (3.5, 0.5)),
                 comp_range=(57, 74), comp_vel=0.35, arp="harpsichord", arp_step=0.25, arp_order=(0, 1, 2, 3, 2, 1, 2, 1),
                 arp_range=(62, 86), arp_vel=0.35, arp_from=2, bass="pizz", bass_pattern=((0, 1, 0), (1, 1, 7), (2, 1, 12), (3, 1, 7)),
                 bass_oct=36, drums_soft={"x": "..x...x...x...x."}, pad="strings", bass_gain=0.55)
JAZZ_STYLE = dict(lead="vibes", comp="epiano", comp_rhythm=((0, 0.6), (1.5, 0.4), (2.5, 1.2)), comp_range=(52, 70), comp_vel=0.55,
                  bass="upright", walk=True, drums={"y": "x..xx..xx..xx..x", "b": "....x.......x...", "k": "x.......o......."},
                  drums_light={"y": "x..xx..xx..xx..x"}, comp_gain=1.6, bass_gain=1.3)


def _render(p, beats, loop, secs=None, lufs=-18.0):
    x = p.render(beats, loop=loop, master_lufs=lufs)
    if secs and not loop and len(x) > d.n_of(secs + 3):
        x = d.fade(x[:d.n_of(secs + 3)], 0, 1.5)
    return x


def tempo_for(beats, secs, nominal):
    """A tempo near `nominal` that fits `beats` into `secs` exactly (for loops)."""
    return beats * 60.0 / secs


# ---- the pieces ------------------------------------------------------------------------------------------------------------------

def title(secs, name="Music Idea"):
    """The title tune (plays once): intro, theme A, theme B, a short return and an ending."""
    bars = 1 + 8 + 8 + 4 + 3
    tempo = (bars - 1.5) * 4 * 60 / (secs - 1.0)
    p = mu.Piece(name, tempo)
    setup(p, POP)
    p.track("fx", None, 0.5, 0.0, 0.5, kind="drums")
    p.note("fx", 0, "G", 0, 0.7)
    mu.pads(p, "pad", 0, ["Bbmaj7"], 2, vel=0.5)
    mu.pads(p, "pad", 2, ["C7sus4"], 2, vel=0.5)
    mu.comp(p, "comp", 0, ["Bbmaj7", "C7sus4"], 2, ((0, 2),), vel=0.5)
    at = 4
    at += _sec(p, at, MAIN_A, 4, MAIN_A_MEL, POP, 2)
    at += _sec(p, at, MAIN_B, 4, MAIN_B_MEL, dict(POP, lead="flute", double="vibes"), 3)
    a2 = MAIN_A[:4]
    at += _sec(p, at, a2, 4, " ".join(MAIN_A_MEL.split("|")[:4]), POP, 3)
    at += _sec(p, at, MAIN_END[:2], 4, " ".join(MAIN_END_MEL.split("|")[:2]), POP, 2)
    # the last chord rings
    mu.comp(p, "comp", at, ["Fmaj9"], 4, ((0, 4),), vel=0.6)
    mu.pads(p, "pad", at, ["Fmaj9"], 4, vel=0.6)
    p.melody("marimba", at, "A5:4")
    p.melody("glock", at, "A6:4", vel=0.5)
    p.note("bass", at, mu.midi("F2"), 4, 0.8)
    p.note("drums", at, "k", 0, 0.8)
    p.note("fx", at, "G", 0, 0.6)
    return _render(p, at + 4, False, secs)


def calm(secs, name="New Age", loop=False):
    bars = max(8, int(round(secs / (4 * 60 / 72))))
    reps = max(1, bars // 8)
    tempo = (reps * 32 + (0 if loop else 4)) * 60 / (secs - (0 if loop else 2.5))
    p = mu.Piece(name, tempo)
    setup(p, CALM_STYLE)
    at = 0
    for r in range(reps):
        lvl = [0, 1, 1, 0][r % 4] if r else 0
        at += _sec(p, at, CALM, 4, CALM_MEL if r % 2 == 0 else None, CALM_STYLE, lvl + (1 if r == 1 else 0),
                   mel_vel=0.9)
        if r % 2 == 1:
            p.melody("celesta", at - 32, CALM_MEL, 12, 0.6)
    if not loop:
        mu.pads(p, "pad", at, ["Dmaj9"], 4, vel=0.6)
        mu.comp(p, "comp", at, ["Dmaj9"], 4, ((0, 4),), 50, 66, vel=0.5)
        p.melody("flute", at, "F#5:4", vel=0.7)
        at += 4
    return _render(p, at, loop, secs)


def theme_piece(name, secs, style, sections, bpb=4, loop=True, nominal=100, ending=None, lufs=-18.0, intro=0):
    """A piece from (chords, melody, level[, instrument]) sections, at a tempo that fits secs (loops) or near nominal."""
    beats = sum(len(s[0]) for s in sections) * bpb + intro * bpb
    if loop:
        tempo = tempo_for(beats, secs, nominal)
    else:
        end_beats = (len(ending[0]) * bpb) if ending else 0
        tempo = (beats + end_beats + bpb * 0.5) * 60 / max(secs - 1.5, 2)
    p = mu.Piece(name, tempo, bpb)
    setup(p, style)
    if loop:
        LOOPED.add(name.lower())
    at = 0
    if intro:
        first = sections[0][0][:intro]
        mu.pads(p, "pad", 0, first, bpb, vel=0.5) if style.get("pad") else None
        if style.get("arp"):
            mu.arp(p, "arp", 0, first, bpb, style.get("arp_step", 0.5), style.get("arp_order", (0, 1, 2, 3)),
                   *style.get("arp_range", (62, 84)), vel=0.4)
        at = intro * bpb
    for sec in sections:
        ch, mel, lvl = sec[:3]
        tr = sec[3] if len(sec) > 3 else 0
        at += _sec(p, at, ch, bpb, mel, style, lvl, transpose=tr)
    if ending and not loop:
        ch, mel = ending
        at += _sec(p, at, ch[:-1], bpb, mel, style, 2) if len(ch) > 1 else 0
        last = ch[-1]
        if style.get("comp"):
            mu.comp(p, "comp", at, [last], bpb, ((0, bpb),), vel=0.6)
        if style.get("pad"):
            mu.pads(p, "pad", at, [last], bpb, vel=0.55)
        root = mu.chord(last)[0]
        p.note("bass", at, 36 + root, bpb, 0.8) if style.get("bass") else None
        top = max(mu.voicing(last, 67, 80))
        p.note(style["lead"], at, top, bpb, 0.8)
        at += bpb
    return _render(p, at, loop, secs, lufs)


def first_bars(mel, n):
    return " ".join(mel.split("|")[:n])


def last_bars(mel, n):
    return " ".join(mel.split("|")[-n:])


PIECES = {}
LOOPED = set()  # the names of the pieces made to loop (sounds.py levels them as loops)


def music(fn):
    PIECES[fn.__name__] = fn
    return fn


def piece(name, secs):
    """The music for a sound file of the game, by its lower-case file name (without extension)."""
    n = name.lower()
    S = secs
    if n == "music idea":
        return title(S)
    if n == "new age":
        return calm(S)
    if n == "golf harp 7":  # accomplishments
        st = dict(POP, lead="vibes", double="glock", arp="harp", arp_step=0.25, arp_order=(0, 1, 2, 3, 4, 3, 2, 1))
        return theme_piece(n, S, st, [(MAIN_A[:4], first_bars(MAIN_A_MEL, 4), 2)], loop=False,
                           ending=(["Gm7", "C7sus4", "Fmaj9"], first_bars(MAIN_END_MEL, 2)))
    if n == "golf harp 6":  # year end
        st = dict(CALM_STYLE, lead="vibes", double="celesta", comp="harp")
        return theme_piece(n, S, st, [(MAIN_B[:4], first_bars(MAIN_B_MEL, 4), 1), (MAIN_B[4:], last_bars(MAIN_B_MEL, 4), 2)],
                           loop=False, ending=(["Gm7", "C7sus4", "Fmaj9"], first_bars(MAIN_END_MEL, 2)))
    if n == "buy1short":  # a story chapter passed: a cheeky rag
        return theme_piece(n, S, RAG_STYLE, [(RAG, RAG_MEL, 2)], loop=False, ending=(["C"], ""))
    if n == "buy4short":  # an odd chapter
        st = dict(POP, lead="kalimba", double="celesta", arp=None)
        return theme_piece(n, S, st, [(STORY[:4], first_bars(STORY_MEL, 4), 1), (STORY[4:6], first_bars(STORY_MEL, 2), 2)],
                           loop=False, ending=(["Dm7", "G7", "C"], ""))
    if n == "buy3short":  # a happy ending
        st = dict(POP, lead="steelpan", double="glock")
        return theme_piece(n, S, st, [(MAIN_B, MAIN_B_MEL, 3)], loop=False,
                           ending=(["Gm7", "C7sus4", "Fmaj9"], first_bars(MAIN_END_MEL, 2)), intro=0)
    if n == "buy2short":
        st = dict(RAG_STYLE, lead="kazoo", double="piano")
        return theme_piece(n, S, st, [(RAG[4:], last_bars(RAG_MEL, 4), 3)], loop=False, ending=(["G7", "C"], ""))
    if n == "romeo and juliet edit":
        st = dict(lead="flute", double="vibes", comp="epiano", comp_rhythm=((1, 1), (2, 1)), comp_range=(55, 72), pad="strings",
                  pad_always=True, bass="bass", bass_pattern=((0, 1, 0),))
        return theme_piece(n, S, st, [(WALTZ, WALTZ_MEL, 2)], bpb=3, loop=False, ending=(["Fmaj9"], ""))
    if n == "sail boat":
        st = dict(LINKS, lead="whistle", comp="reed", comp_rhythm=((0, 3), (3, 3)), comp_range=(55, 70), comp_vel=0.4,
                  drums={"t": "x..x..x..x..", "d": "x.....x....."})
        return theme_piece(n, S, st, [(SAIL[:3], " ".join(SAIL_MEL.split("|")[:3]), 3)], bpb=6, loop=False,
                           ending=(["G"], ""))
    # the course styles' story jingles and playlists
    if n in ("steel drums 2",):  # tropical story chapter (also the playlist's Steel Drums 2)
        return theme_piece(n, S, TROP, [(TROP_A[:4], first_bars(TROP_A_MEL, 4), 3)], loop=False, ending=(["C", "D7", "G"], ""))
    if n == "steel drums 3":  # tropical odd chapter
        return theme_piece(n, S, TROP, [(TROP_B[:4], first_bars(TROP_B_MEL, 4), 2), (TROP_A[4:7], first_bars(last_bars(TROP_A_MEL, 4), 3), 3)],
                           loop=False, ending=(["G"], ""))
    if n == "steel drum 1a":
        return theme_piece(n, S, TROP, [(TROP_A, TROP_A_MEL, 2), (TROP_A, TROP_A_MEL, 3)], loop=True)
    if n == "steel drums 1b":
        return theme_piece(n, S, dict(TROP, lead="marimba", double="steelpan"),
                           [(TROP_B, TROP_B_MEL, 2), (TROP_A, TROP_A_MEL, 3)], loop=True)
    if n == "bagpipe1" or n == "bagpipe":  # links story chapter
        st = dict(LINKS, lead="reed", double="whistle")
        return theme_piece(n, S, st, [(LINKS_A, LINKS_A_MEL, 2), (LINKS_A[:4], first_bars(LINKS_A_MEL, 4), 3)], bpb=6,
                           loop=False, ending=(["D"], ""))
    if n == "bagpipe2":  # links odd chapter / playlist
        return theme_piece(n, S, LINKS, [(LINKS_B, LINKS_B_MEL, 2), (LINKS_A[:7], first_bars(LINKS_A_MEL, 7), 3)], bpb=6,
                           loop=False, ending=(["D"], ""))
    if n == "harpiscord 1":
        return theme_piece(n, S, CLUBHOUSE, [(CLUB, CLUB_MEL, 1), (CLUB, CLUB_MEL, 2)], loop=True)
    if n == "harpsicord 2":
        st = dict(CLUBHOUSE, lead="harp", double="flute")
        return theme_piece(n, S, st, [(LINKS_B[:4] + CLUB[4:], first_bars(LINKS_B_MEL, 0) or None, 1), (CLUB, CLUB_MEL, 2, 12)],
                           loop=True)
    if n == "harpsicord 1 short":
        return theme_piece(n, S, CLUBHOUSE, [(CLUB[4:], last_bars(CLUB_MEL, 4), 2)], loop=False, ending=(["D"], ""))
    if n in ("spanish guitar wflute 2", "spanish guitar 2"):  # desert story chapter / playlist
        return theme_piece(n, S, DESERT, [(DES_A, DES_A_MEL, 2), (DES_A, DES_A_MEL, 3)], loop=False, ending=(["Am9"], ""))
    if n == "spanish guitar 1":  # desert playlist: a sunny bossa
        return theme_piece(n, S, dict(BOSSA, lead="panflute", double="kalimba"), [(DES_B, DES_B_MEL, 2)], loop=True)
    if n == "spanish guitar 3":  # desert odd chapter
        return theme_piece(n, S, DESERT, [(DES_B[:6], first_bars(DES_B_MEL, 6), 2), (DES_A[4:], last_bars(DES_A_MEL, 4), 3)],
                           loop=False, ending=(["Am9"], ""))
    if n == "jazz":
        # the clubhouse lounge: a bossa, the second time round with the drums in
        return theme_piece(n, S, BOSSA, [(JAZZ, JAZZ_MEL, 2), (JAZZ, JAZZ_MEL, 3)], loop=True, nominal=120)
    if n == "end of year":
        st = dict(CALM_STYLE, lead="celesta", double="glock", comp="epiano", comp_rhythm=((0, 2), (2, 2)))
        return theme_piece(n, S, st, [(["Bbmaj7", "Bbm6", "Am7", "D7"], first_bars(MAIN_B_MEL, 4), 1),
                                      (MAIN_A[4:], last_bars(MAIN_A_MEL, 4), 2)],
                           loop=False, ending=(["Gm7", "C7sus4", "Fmaj9"], first_bars(MAIN_END_MEL, 2)))
    if n == "end of year short":
        st = dict(CALM_STYLE, lead="celesta", double="glock", comp="epiano")
        return theme_piece(n, S, st, [], loop=False, ending=(["Gm7", "C7sus4", "Fmaj9"], first_bars(MAIN_END_MEL, 2)))
    if n == "credit":
        return credits(S)
    return None


def credits(secs):
    """The credits: the course styles' themes one after another with short crossfades, then the main theme to finish."""
    parts = [
        theme_piece("credits trop", 22, TROP, [(TROP_A, TROP_A_MEL, 3)], loop=False, ending=(["G"], "")),
        theme_piece("credits des", 22, DESERT, [(DES_A, DES_A_MEL, 3)], loop=False, ending=(["Am9"], "")),
        theme_piece("credits links", 20, LINKS, [(LINKS_A, LINKS_A_MEL, 3)], bpb=6, loop=False, ending=(["D"], "")),
        title(48.0, "credits title"),
    ]
    out = parts[0]
    xf = d.n_of(1.2)
    for nx in parts[1:]:
        a = out.copy()
        a[-xf:] *= np.linspace(1, 0, xf)[:, None]
        b = nx.copy()
        b[:xf] *= np.linspace(0, 1, xf)[:, None]
        out = np.concatenate([a[:-xf], a[-xf:] + b[:xf], b[xf:]])
    return d.master(out, -18.0, "integrated")

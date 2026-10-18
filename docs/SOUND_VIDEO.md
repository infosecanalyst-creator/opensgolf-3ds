# Sound and video

## Sound

All 241 sound effects, voices and music tracks under `Sounds/` are ordinary PCM `.wav` files (22.05 or 44.1 kHz, mono or stereo, 16 bit),
so the original needs no decoder. Copies of the game that went through other tools do not always keep them that way, and a
copy whose sounds the old WAV reader refused played no sound at all, browser and desktop alike (issue #10). So
`crates/sg-core/src/wav.rs` reads every common WAV coding (8/16/24/32-bit PCM, 32/64-bit float, A-law, mu-law, Microsoft
ADPCM, IMA ADPCM, MP3 inside a WAV, WAVE_FORMAT_EXTENSIBLE; more than two channels keep the first two; checked against ffmpeg's decoding of
the same files), a copy whose sounds were converted to `.ogg` is read as the `.wav` names the game asks for, and macOS
`._` resource-fork files are ignored. Where the player's copy still fails (no `Sounds` folder,
or a clip in a coding not read) the free base set's sound of that name plays instead, the failure is logged once per
clip on stderr (`sound: cannot load ...`), and the title screen says once in a box what went wrong
(`STR_SOUND_CANNOT_PLAY`; settings.json `sound_problem_shown` keeps it from repeating). A clip the original itself lacks
stays silent, as in the original. In the browser the page also resumes the audio output on every click, tap or key
(Chrome's autoplay policy keeps a context made outside a gesture suspended), skips a Mac zip's `__MACOSX` entries, and says
when a zip holds entries in a compression it cannot unpack (it unpacks stored, deflate, Deflate64, BZip2, LZMA and PPMd).

`crates/sg-core/src/mixer.rs` has a small mixer (clips converted to 44.1 kHz stereo, up to 32 voices, looping,
case insensitive lookup by path under `Sounds/`); the game feeds the system's output device from it through cpal (CoreAudio, WASAPI, ALSA). The mixer can render to a buffer without a device, which is how
the unit tests check it (`cargo test`).

The game plays:

| When | Sound |
|---|---|
| tee shot / later shots / putt | `Golf_Sfx/Drive With Ball`, `Iron`, `Putt` |
| ball lands: fairway, sand, water, out of bounds | `Ball Drop Fairway`, `Ball Drop Sand`, `Ball Water`, `Ball Tree` |
| holed out | `Ball In Hole`, `ApplauseGood` (5 strokes or fewer) or `Applause`, `Effects/cash` |
| painting terrain, per tile (EXACT, below) | tee, green `Interface/Test 2`; fairway the painting tune on `Test 7`; rough class `Test 9` (rocks `Place Rocks 1`); sand `Test 10`; water `Place Water`; trees their growing sound |
| path, wall, raise, lower, save | `Interface/Path`, `Place Rocks Generic`, `Bass Up 2`, `Bass Down 2`, `Button1` |
| always | `GolfAmbience122` at low volume, looped |

M starts and cycles the music of the current theme (`Sounds/Music/<theme>_Music`), N mutes everything. `--mute` starts silent,
`--sound-log` prints every sound as it plays (also with `--png` runs, which use no audio device).

One buffer per slot (EXACT from the exe, DERIVED for the replay): the exe registers each sound slot as one sound object with one
DirectSound buffer, and every play (0x40c500 -> 0x4481b0 -> 0x484940) sets that object's pitch, volume, pan and start delay and
plays its buffer. A slot therefore never sounds twice at once: the port keeps the voice each slot last started
(`App::slot_voices`) and a request for a slot whose clip is still sounding (or waiting for its delayed start) leaves it playing,
as DirectSound's Play does on a buffer that is already playing. The applause after each of the pro's shots (slot 0x2e,
`Applause.wav`, 6 s, 1 s delay; the golfers' tick, decompile line ~135031) used to start a second copy over the first whenever the pro
shot again within six seconds, which sounded like several crowds clapping out of step.

Painting (EXACT, 0x420660 and 0x40c500): each painted tile plays a slot chosen by its terrain's class byte. A positional
play gets a random pitch of -299..300 cents; a fairway tile instead plays the next note of a 36-note tune ("Jesu, Joy of
Man's Desiring", table 0x4c2fa4, `(note - 5) * 100` cents, from a counter that is never reset), so painting fairway plays
the tune note by note. The note restarts on each tile (DERIVED: the play itself is inside sound.dll).

The title menu plays slot 0x80 (`Music Idea.wav`) when it opens (0x40f5c0, EXACT).

Which sound goes with which other event is my choice from the file names; the original's mapping lives in golf.exe, which is not read.
The volumes are guesses.

## Video

`Flics/SMSG_IntroFinal.bik` and `SMSG_ClosingFinal.bik` are Bink 1 files (BIKi, 800x600, 15 fps, one audio track, 55 s and 39 s).
Bink is RAD Game Tools' codec and the original plays it through `binkw32.dll`. `sgplay` parses the file header itself
(`sgplay FILE --info`) and uses an installed `ffmpeg` for decoding, because ffmpeg has an open implementation of both the Bink video
and Bink audio codecs (macOS: `brew install ffmpeg`; Windows: put `ffmpeg.exe` on the PATH). Files that are not Bink are measured with `ffprobe`. It runs ffmpeg twice (raw RGB frames, raw 44.1 kHz stereo audio); the audio clock drives
the picture. Esc or Space quits. `sgplay FILE --png out.png --at SECONDS` saves one frame (used for the headless test).

A native Bink decoder (no ffmpeg) is possible but a project of its own; this keeps the dependency to one well known tool.

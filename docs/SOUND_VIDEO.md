# Sound and video

## Sound

All 241 sound effects, voices and music tracks under `Sounds/` are ordinary PCM `.wav` files (22.05 or 44.1 kHz, mono or stereo, 16 bit),
so no decoder is needed. `crates/sg-core/src/mixer.rs` has a small mixer (clips converted to 44.1 kHz stereo, up to 32 voices, looping,
case insensitive lookup by path under `Sounds/`); the game feeds the system's output device from it through cpal (CoreAudio, WASAPI, ALSA). The mixer can render to a buffer without a device, which is how
the unit tests check it (`cargo test`).

The game plays:

| When | Sound |
|---|---|
| tee shot / later shots / putt | `Golf_Sfx/Drive With Ball`, `Iron`, `Putt` |
| ball lands: fairway, sand, water, out of bounds | `Ball Drop Fairway`, `Ball Drop Sand`, `Ball Water`, `Ball Tree` |
| holed out | `Ball In Hole`, `ApplauseGood` (5 strokes or fewer) or `Applause`, `Effects/cash` |
| painting terrain | `Interface/Place Fairway / GreenTee / Bunker / Water / Rocks / Rough`, `Building`, `Effects/Flower Bed` |
| path, wall, raise, lower, save | `Interface/Path`, `Place Rocks Generic`, `Bass Up 2`, `Bass Down 2`, `Button1` |
| always | `GolfAmbience122` at low volume, looped |

M starts and cycles the music of the current theme (`Sounds/Music/<theme>_Music`), N mutes everything. `--mute` starts silent,
`--sound-log` prints every sound as it plays (also with `--png` runs, which use no audio device).

Which sound goes with which event is my choice from the file names; the original's mapping lives in golf.exe, which is not read.
The volumes are guesses.

## Video

`Flics/SMSG_IntroFinal.bik` and `SMSG_ClosingFinal.bik` are Bink 1 files (BIKi, 800x600, 15 fps, one audio track, 55 s and 39 s).
Bink is RAD Game Tools' codec and the original plays it through `binkw32.dll`. `sgplay` parses the file header itself
(`sgplay FILE --info`) and uses an installed `ffmpeg` for decoding, because ffmpeg has an open implementation of both the Bink video
and Bink audio codecs (macOS: `brew install ffmpeg`; Windows: put `ffmpeg.exe` on the PATH). Files that are not Bink are measured with `ffprobe`. It runs ffmpeg twice (raw RGB frames, raw 44.1 kHz stereo audio); the audio clock drives
the picture. Esc or Space quits. `sgplay FILE --png out.png --at SECONDS` saves one frame (used for the headless test).

A native Bink decoder (no ffmpeg) is possible but a project of its own; this keeps the dependency to one well known tool.

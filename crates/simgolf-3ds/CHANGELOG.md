# Changelog: SimGolf for the New 3DS

(The repository's top-level CHANGELOG.md is upstream OpenSGolf's.)

## 2026-11-21

- Autosaves rotate by renaming instead of copying, so each autosave writes the SD card once instead of three times: less pause every two minutes.
- Missing ground textures now stop the game with a message that says where it looked and what to do.
- Failed saves and settings writes are logged or shown; choosing TITLE SCREEN after a failed save warns first and stays in the game (choose it again to leave anyway).

## 2026-11-20

- Autosave is safer: written to a temporary file first, older autosaves move along only once it worked, and a failed autosave says "Autosave failed - is the SD card full?".
- HOME > Close saves the game before quitting. Pressing HOME alone or sleeping no longer saves (that froze HOME on real hardware in another port).
- stage.txt gets only start, exit and failure lines; the detailed trace needs an empty /3ds/opensgolf/debug.txt.
- The 3DS build really uses its release profile (faster); the version is shown.
- The game waits for memory to settle on start instead of a fixed time.
- Install path: the program is /3ds/simgolf-3ds.3dsx.

## 2026-11-19

- Manual: drag the stylus to scroll the text, with a little momentum.

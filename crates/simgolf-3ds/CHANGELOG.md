# Changelog: SimGolf for the New 3DS

(The repository's top-level CHANGELOG.md is upstream OpenSGolf's.)

## 2026-11-27

- Saves are per course. Every course has its own three autosaves and its own saved game, so a second course can no longer erase the first. The title's Load Game lists courses; choose one to see its saves. Saves from older builds still open and are never written over.
- LOAD GAME asks first (NO chosen), says what it would load, autosaves the game in play, and shows "Loading...".
- EXIT GAME saves before it leaves and says so if the save fails (EXIT ANYWAY / NO).
- Pause menu in the house order: RESUME, MANUAL, SAVE GAME, LOAD GAME, SETTINGS, TITLE SCREEN, EXIT GAME. SETTINGS has golfer thoughts, sound, left hand and the controls chart. Left hand is also in the title's Settings.
- B (undo) says what it did and how much money came back. A line after the first hole opens says what to do next.
- A saved course opens with the view where you left it.
- The hole list no longer cuts a hole's type to "Bre...". The pause card on the top screen is opaque. THINK shows ON / OFF. Title and manual use the same navy look as the pause menu. One name on screen: OpenSGolf.

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

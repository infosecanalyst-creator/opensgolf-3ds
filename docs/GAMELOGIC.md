# Game logic (started)

The rules of SimGolf live in `golf.exe`, which is SafeDisc protected. This project does not unpack,
strip or read it, so rules cannot be copied from it. Game logic here is a clean-room reimplementation
that is *driven by what the data files show*, and everything that is a guess is marked as a placeholder.

## What exists: `crates/sg-core/src/shot.rs`

`ShotSim` plays one hole with one golfer along the demo course's route, as a state machine:
walk to the ball, address, swing, ball flight, lie check, repeat; inside 230 units of the hole it putts,
the ball rolls in, the golfer celebrates, then the round restarts at the tee.

From the data (not guessed):
* Animation lengths and the moment of impact come from the sprites: `Male*_PerfectSwing` is 20 frames at
  83 ms and the club is level in front of the golfer at about frame 11; `NormalAddress` is 18 frames;
  `Putt` is 33 frames (impact taken as frame 12, which is a guess).
* Facing and views come from `SPRITES.md`.
* Water, sand and out of bounds are detected from the tile type under the landing point.

Placeholders (explicitly not the original's behaviour):
* carry 900 world units for a full swing with plus or minus 12 percent and a 14 degree dispersion;
* walk speed 130 units per second; flight time and arc height;
* penalty handling: replay from the same spot with one extra stroke;
* putting: see the skills section below.

## Golfer skills

`Themes/Standard/progolfers.dta` documents itself in its comment header: name, body type, skin, hat, shirt,
pants, then ten skill levels as hex digits 0..F: power hitter, long driver, accurate driver, accurate irons,
accurate putter, draw shot, fade shot, high backspin shot, recovery skills, luck (see docs/FORMATS.md).
`simgolf --golfer "Nick Jacklaus"` plays the demo hole with that golfer's skills (name match is a case-insensitive substring).

What the levels DO is a placeholder, since the real formulas are in golf.exe, which is not read:
* carry = 900 x (0.8 + 0.2 x (power + long driver) / 15)
* dispersion cone = 24 - 20 x accuracy / 15 degrees (accurate driver off the tee, accurate irons near the green),
  widened up to 1.6x from sand and narrowed by recovery skills
* chance to hole a putt = 0.40 + 0.55 x accurate putter / 15 + 0.05 x luck / 15; a miss stops 20 to 55 units from the hole
* draw, fade and backspin are not modelled yet.

Typical result over many rounds of the demo hole: a top rated golfer needs 5 strokes, a 0 skill golfer 7.

## From the manual

Firm fairway makes the ball roll farther (12 percent run on) and rocks deflect it (40 to 90 units, random direction); both amounts are
placeholders. See docs/MANUAL_NOTES.md for everything the manual says about shots and skills.

## Money

See the club money section of docs/EDITING.md: what public sources say about the economy, and which parts are implemented or invented.

## Viewer

`simgolf` prints each event (`stroke 3: in the sand`). `F` makes the camera follow the golfer;
`--follow --time 5` renders a still of that moment.

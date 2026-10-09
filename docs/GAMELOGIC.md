# Game logic

The rules come from the publisher's golf.exe, read for facts only and restated in our own words (docs/PUBLISHER_EXE_NOTES.md).
Nothing from the exe's code is copied, and no game data is part of this project.

## The golfers (`crates/sg-core/src/golfer.rs`)

The exe keeps 152 golfer records and runs one update per game tick (87 ms) over all of them. The port does the same, field for
field (`Golfer`), and `Club::tick` runs these steps in the exe's order:

- **Arrivals.** Only invited people come: a new game invites twelve (three each of four skill classes), and every membership
  upgrade invites one more. A golfer is created whenever fewer than two are waiting, or every 128 ticks, up to eight waiting;
  waiting golfers stay in the clubhouse and go out in pairs as soon as the first tee is free.
- **The animation pass.** The exe's drawing routine advances each golfer's clip and holds the swing until its frames have been
  shown; reactions end their pause when the clip ends. The port runs this pass in the simulation, with the frame counts read from
  the player's own sprite files.
- **The golfer update.** Thoughts fade; golfers glance at nearby flower beds, landmarks, eyesores and weeds; hunger or thirst
  grow every 160 ticks; a golfer with mood below zero quits and never comes back (a member resigns). Before teeing off golfers
  detour to a snack bar, a bench, a ball washer, the putting green, the pro shop or the driving range when they need or like to,
  and pay for it. They walk by the exe's path search, preferring paths and avoiding water, give way to golfers ahead, complain
  about slow play, walking off the paths and steep slopes, and ride carts when a cart garage stands.
- **The shot.** When it is a golfer's turn (the one farther from the pin plays first), the shot planner chooses the target, club
  and launch; the golfer waits until the landing area is clear, addresses the ball and swings.
- **The ball.** Flight with curve, trees and buildings in the way, bounces by ground type, roll with slopes and walls, the cup,
  water drops and out of bounds penalties, and the golfer's reaction to where it stopped.
- **Hole end.** The green fee (the golfer's mood plus the hole and membership bonuses), the scorecard, the mood drop after each
  hole, and at the end of the round the membership points that can upgrade a member.

Mood events go through one routine (`Club::event`) as in the exe: the thought shows, small upsets only count when they repeat,
negative amounts are halved, an unhappy golfer can sour the partner's mood, and a negative event may start a weed where the
golfer stands.

## Shot planning (`crates/sg-core/src/planner.rs`)

- Maximum range from the golfer's skills, momentum and lie.
- For a long shot the golfer tries landing tiles around the line to the pin with simplified trial flights, in rounds of more and
  more samples, scoring how close each ends to the pin, the hazards around it and the risk of the next shot.
- The shot: club from the distance, launch speed by bisection, the random hook or slice scaled by skills, draws, fades, high
  backspin shots onto the green and low punches under trees, the spread of the launch speed by lie, and the golfer's thoughts
  about the shot (views, the hole, hazards, the shot type, a new club).

## The course as golfers see it (`crates/sg-core/src/course.rs`)

The tiles, flags, corner heights, building levels and walls in the exe's layout, rebuilt from the land after every change.

## The roster (`crates/sg-core/src/roster.rs`)

The people who come to play are compiled into the exe. When the player's game folder holds a readable exe they are read from it;
the retail exe is copy protected and is not touched, and a roster written for this port (own names, the exe's gender split and
skill classes) stands in. The special visitors are read from the Standard theme's golfer files.

## Not yet in

Stories between golfer pairs, the special visitors' verdicts, wagers and matches, tournaments, the player's own golfer (Gary),
and the course statistics routine that rates holes; the course report still measures +Len/+Acc/+Img with the older shot model
in `shot.rs`.

## Viewer

`F` makes the camera follow the first golfer on the course; `--follow --time S` renders a still after S seconds.

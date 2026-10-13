# Cut content

What the shipped SimGolf holds but never shows or offers: art in the game files the program never loads, art it loads but
does not draw, and code or text no path reaches. The port's optional **Cut content** extra (docs/PLAYING.md, OpenSGolf
extras; off by default) makes the pieces that can work safely placeable. Everything here is restated from the publisher's
golf.exe (its decompile) and the game files; markers as in the other notes: EXACT (read from the code), DERIVED (follows
from the code but needs a step of reasoning), WEAK (a search that found nothing, which is not proof).

## How it was found

- **Art the exe never loads** (DERIVED). The exe registers every sprite it draws through one routine (0x43d740) with the
  file's name as a string. For each of the 995 non-shadow `.flc` files under `Flics/`, the file counts as loaded when a
  registration names it (the decompile shortens long string symbols, so a registration whose name is a 10+ character
  prefix of the path counts too) or when any string of the exe contains its base name (palette loads, name tables). What
  is left (161 files) is listed below. A name built at run time from pieces (golfer clips are) would escape this test, so
  the golfer and celebrity clips below are WEAK.
- **Art loaded but not drawn** (WEAK). Sprite numbers registered whose number appears nowhere else in the decompile, after
  allowing for the numbers the drawing code computes (`base + level`, `base + type`, `base + design`).
- **Code and text no path reaches** (EXACT unless marked): from the decode notes, which give the evidence.

## Art the exe never loads

- **Landmarks** (10): Chapel, Chapel_ANIM, Chapel_dirt, Hgardens (a hanging garden), Railroad Tracks, Rock Garden,
  cathedral_ruins, gargoyle, pyramid, volcano. Also SundialB (a second sundial; only the first, `Landmarks/Sundial`, is
  registered: the test above counts SundialB as loaded because the registration's name is a prefix of it).
- **Scenic** (25): Aerie, Aspen, Bamboo, BlackPine, BridgeBrick, BridgeStone, CactusGroupB, Ghog_dighole and Ghog_popup (a
  gopher), HotSpring, JapaneseTree, Koi Pond, LinksWaterRock, Mangrove, Mesquite, Oasis, Scenic_Cattails, Trop Boulder,
  blimpMAX, copter, desert fountain, heather, rainbow sail, wave, wtow.
- **Homes** (20): Arcade_game and Arcade_gamePlay, Grill and Grill_open, jacuzzi and jacuzzi_on, mansion and mansion_dirt,
  punching_bag and punching_bag_hit, five cars (Ferarri, harley, lamborgini, bmw, classic_car), and the Regular houses
  Desert/B, Links/A, Parkland/B, Parkland/C, Tropical/A (the game draws its own house sprites 0x1c8..0x1ca instead).
- **Buildings** (25): Desert/DESdrL2400, DESputtL2flag, DhouseA and DhouseAbase, tenL1, tenL1_base, tenL2, tenL2_base (a
  Desert tennis club: the Desert's sport building is the spa); Park/House_A_dirt, houseBbase; Tropical/TROPDrivingR400L2,
  TROPPuttingGL2_ANIM, TROPhouseA, TROPhouseB; links/DR_400sign, DR_wall, House_A_links_dirt, LinksBridge_back, _front,
  _middle and _ripple (a Links bridge in four layers), PG_hutL2_ANIM, PG_yflag, rockwall02, rockwall03.
- **Bridges** (11): ASshallowA/B, RippleA, SCENICgen01 and 02 (the scenic bridge uses 03..10), middlerockA..C,
  shallowrockA..C. **Water** (4): ASdesrockC and its top, UWtropcoralD, waveTropB.
- **Trees** (15): Desert/CactusB in three sizes, Desert/TreeCactus in three sizes, Links/LinksTree3 and LinksTree4 in
  three sizes each, and TreePineLarge, Medium, Small (the game uses the spruce and fir sets).
- **Flowers** (14): Iron Bench and woodplank bench (two more bench designs; the strip has five), Purpleflowers,
  whiteflowers, swing, dandelion_02..04 (only dandelion_01 is the weed), and TropFB in six shapes (a tropical bed set; the
  game loads TropFlowers).
- **Employee** (7, WEAK): Bev_InCart and Bev_Walk (a drinks cart), RangerCartInOut, RangerCartMove, RangerCartOnly (a
  ranger's cart), FatGuyStride, GreeterAction1 (the greeter's action is GreeterAction2).
- **Celebrities** (5, WEAK): Comedian_Dont_Like, Comedian_Interaction, Comedian_Like, Politician_DontLike,
  Politician_Interact (reactions; the celebrity visitor that would play them is dead code, see below).
- **Golfers** (23, WEAK): the female FemaleSkSS body's clips (Cart, FailureA, Fidget, HandShake, Happy, LeanLeft,
  LeanRight, LineUpPutt, LookUp, NormalAddress, NormalSwing, NormalWalk, Pitch, PitchAddress, PointAt, Putt, PuttAddress,
  Sad, SitSq, Sitting, Sq, SuccessA, TiredWalk). Golfer clip names are built from pieces, so these may yet be reached.
- **Tees** (2): TVScoreCAP_L and TVScoreCAP_R (the ends of a TV scoreboard; the centre piece is registered).

## Art loaded but not drawn (WEAK)

Registered by 0x43d740 with no reference to the number found elsewhere: 0x205 BridgeXtra (per theme), 0x238
WaterFallTallA and 0x23b SprayShortA (per theme), 0x24a TVScoreCenter, 0x17c..0x17f 1Putt, 2Putt, 1Drive, 2Drive, 0xf5
blimpEAshadow. The scenic spot sprites 0xfa..0x102 and 0x127..0x12e (CactusGroupA, Rhody, Scenic_Tree, OGrass, Vineyard,
Gold, RoseBush, Rock Form, CowSkull, DeadGolfer, Desert Water Rock, grouse, Tropical Wildflowers, ScenicLinksRocks) are
named by the scenery feature names of 0x4074a0 ("rock formation", "natural bridge", "mysterious bones", "vineyard",
"rhododedron", ...), so they are scenery of the generator's spots, not cut content.

## Code and text no path reaches

- Green fee mode 2 (0x543cf4) doubles fees and changes repeated thoughts; nothing sets it (EXACT; golfer.rs).
- An unfinished tutorial: 0x4604f0 steps a counter through two camera tours (EXACT; PUBLISHER_EXE_NOTES.md).
- Mood type 44, "consolation", has no caller (DECODE_EVENTS_SHOTS.md).
- The celebrity visitor kind and its portrait use (DECODE_FACES.md; DECODE_TOURNAMENTS.md).
- The pair-selection screen builder 0x44bde0 has no caller (PARITY.md).
- Voices OOPS0, WATER0 (0x54/0x55) and BLIND (type 65, no caller) (DECODE_VOICES.md, DECODE_VOICES2.md).
- The landmark row of the building table has a price of 15 that is never used (landmarks cost (5 type + 25) x 2; EXE_COSTS.md).
- The golfer card's mode 0 and the Customise screen's mode 0 (DECODE_FACES.md, DECODE_CUSTOMISE.md).
- Landmark types 16..18 (water tower, radio antenna, oil pump) are only ever put down by the land generator as eyesores;
  the player cannot buy them (the strip shows types 0..13; EXACT, 0x432200).

## What the Cut content extra adds

Landmark types 19..32 (`sg_core::objects::CUT_LANDMARKS`), each the file in its own palette from the same folder. They are
on the second page of the landmark strip (press the Landmark button again). The original has no price or effect for them;
the port's choices are:

| Type | Art | Name |
|------|-----|------|
| 19 | Scenic/CactusGroupB | cactus garden |
| 20 | Scenic/BlackPine | black pine |
| 21 | Landmarks/cathedral_ruins | ruined cathedral |
| 22 | Landmarks/gargoyle | gargoyle |
| 23 | Scenic/Trop Boulder | tropical boulder |
| 24 | Landmarks/Rock Garden | rock garden |
| 25 | Landmarks/SundialB | garden sundial |
| 26 | Landmarks/Chapel | chapel |
| 27 | Scenic/Scenic_Cattails | cattail patch |
| 28 | Scenic/Koi Pond | koi pond |
| 29 | Scenic/HotSpring | hot spring |
| 30 | Scenic/Oasis | desert oasis |
| 31 | Scenic/desert fountain | desert fountain |
| 32 | Scenic/LinksWaterRock | sea rock |

- Price: $6,000 each (`sg_core::extras::CUT_LANDMARK_UNITS`), no free first one, no clearing charge (as landmarks).
- Effect: a golfer glancing at one has the scenic thought (+1 mood), the course planner counts it as a feature, and the
  thought text names it; types 19 and up have none of the landmark powers (those go by type & 3 and only types below 16
  have them), and no reach ring is drawn.
- One tile, like the original's landmarks; permanent like them.
- The table of placed objects stays below 255 records in use (tiles keep an object's index in a byte; the exe's placement,
  0x40e000, stops at 0xff): a cut landmark is refused with "There is no room for that building there" beyond that.

Not offered, and why: the volcano, pyramid and hanging garden are drawn several tiles wide (they would stand over their
neighbours on a one-tile footprint); the Desert tennis club, the Links bridge, the extra house and tree sets, the bridge
and water rock pieces belong to systems whose rules for them are unknown (offering them would mean inventing those rules);
the home props, cars, carts and the extra golfer clips need animation and behaviour the original never wrote; the unused
code paths above have no complete rules to restore.

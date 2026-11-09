# Saved games of the original (`Saved Games\<name>.sve`)

Facts only, in our own words, read from the publisher's unprotected golf.exe (its decompile, the disassembly and a PE reader
for the strings), the way the other DECODE notes were made (docs/REVERSE_ENGINEERING.md). Tags: EXACT (code or table read
directly), DERIVED (shape read, meaning inferred from how the code uses it), WEAK (a guess with some support), UNKNOWN.
Offsets are file offsets in decimal unless written in hex. Money is in units of $100; a month is 1024 ticks, a year 8192.

The importer is `crates/sg-core/src/sve.rs` (pure, no files; tests build files of this layout), and the app turns its result
into a saved game of its own (`crates/simgolf/src/app.rs` `original_save`, `share.rs` `import_original`).

## 1. Routines

| Address | What | Tag |
|---------|------|-----|
| 0x40b4a0 | Save: builds `saved games\` + name + `.sve` (strings 0x4c3f4c, 0x4c3f44) in the buffer 0x51a068, opens it with mode 0x8301 (write, create, truncate, binary) and permission 0x80, writes the 100-byte header, runs the block list in write mode, closes | EXACT |
| 0x40afa0 | The block list: sets the direction word 0x53e638 (0 write, 1 read) and passes each block to 0x40af70 | EXACT |
| 0x40af70 | One block: raw `_read` (0x4a583a) or `_write` (0x4a5b58) of the given bytes, no conversion, no length prefix | EXACT |
| 0x40b9b0 | Load (name, restore, header only, championship): opens `saved games\<name>` (or `Themes\Championship\<name>` with the championship argument, which then reads a second 100-byte header), reads the header, returns there when "header only" is set (the Load list's preview), else runs 0x40afa0(1) | EXACT |
| 0x40b840 / 0x40bbf0 | The championship course loader and its shorter block list (two headers, 56 blocks, then 8 more bytes): the `.cse` files of "Save Course for Championship", not `.sve` | EXACT (not used by the importer) |

After a full load (0x40b9b0): flags 0x59e7b8 = loaded flags with bits 4 and 8 cleared, bit 4 taken from the running game, and
0x40000000 set; the dock mode 0x567afc = 0; then the per-frame tables are rebuilt (0x42f7a0, 0x42f340, 0x42f2c0). With
"restore" set and a name other than "While Browsing.sve" it shows "Loading Game" (0x4c5304); if the loaded theme byte differs
from the one in memory the theme's art is reloaded (0x43dbe0). EXACT.

The hole being built after a load: the flag word keeps bit 0x40000 (the hole layout pass has planned the hole being built,
PUBLISHER_EXE_NOTES "Building holes") as the file has it (0x40bae7..0x40bb08 clear only bits 4 and 8), but the first rebuild,
0x42f7a0, ends by clearing 0x40000 (0x42f929, the only place the code masks that bit out of the flag word). The pass's
landing table (0x542dfc / 0x542e24) lies between the blocks 0x542414 and 0x543cfc of section 4, so it is not saved. So the
next frame plans the hole again, drawing from the generator, and sets the hole record's facing, length, marks and yardage
markers afresh. EXACT for the reads and writes (machine code of 0x40b9b0 and 0x42f7a0); DERIVED that 0x42f7a0 is the
"terrain refresh" of the layout pass's notes. The port does the same:
its load plans the hole being built again (`App::load_save_into`, `sync_course`), once, as a terrain refresh does.

## 2. Versioning and variants

* There is no magic, version number, checksum or length field: the file is a memory dump in a fixed order. EXACT.
* The layout here is the publisher's exe's, which already has the v1.02 patch's features (animals off in Preferences, the
  option bits block 0x5a5a00). A save written by an earlier or later build may differ; nothing in the file says so. WEAK
  (no save of another build was available to compare).
* Three blocks at the end depend on the game flag word 0x59e7b8, which the file itself holds (offset 282121) before them:
  bit 0x40000000 (every game started or loaded since the program began) adds 0x8c0 bytes, bit 0x200000 (a tournament
  running) adds 0xb7c0 + 0x19a28. So a file is 294733, 296973, 446773 or 449013 bytes long. EXACT. In practice every save
  of a game in progress has the bit 0x40000000 (the new game and load paths set it), so 296973 bytes is the normal size.
  DERIVED.
* The importer refuses a file whose length is not the one its flags call for, a header that is not text, and values no save
  can hold (difficulty outside 0..3, theme outside 0..3, offer slot or site above 15, a negative tick, next hole outside
  1..19, a tile type of 23 or more, a corner height above 20, an object kind outside -1..19). EXACT rules of the port's own.

## 3. Header (offset 0, 100 bytes)

The text at 0x51a068 when 0x40b4a0 writes it: "-" (0x4c52bc), the course name with its tier word (0x40daa0(0): the custom
name, string 0 below, else the site's course name, then Municipal, Golf Club, Country Club or Championship), ", "
(0x4c52b8), the day ((tick mod 1024) * 30 >> 10) + 1, " " (0x4c3f70), then 0x40d7b0(tick): the month name ((tick >> 10) & 7
from March, table 0x4c2908) and the year. EXACT. The whole 100-byte buffer is written, so bytes after the text's NUL are
whatever the buffer held before (the start of the path built just before, `saved games\<name>.sve`): only the text up to the
first NUL means anything. EXACT. The Load list shows this text for autosaves ('&' names) via a header-only load. EXACT.

## 4. The blocks (0x40afa0), in file order

"Port" says where the importer puts it; "-" means not carried over. Offsets are the same in every variant (the optional
blocks only come after the last one).

| Offset | Address | Bytes | What | Tag | Port |
|-------:|---------|------:|------|-----|------|
| 100 | 0x822c78 | 4 | Course id for the Top 10 table (rand(0xffff) at new game) | EXACT | `course_id` |
| 104 | 0x5685f0 | 4 | Next hole to build (holes open + 1) | EXACT | `club.next_hole` |
| 108 | 0x58d36c | 4 | Yards over open holes | EXACT | - (recomputed) |
| 112 | 0x59aafc | 4 | Par over open holes | EXACT | - (recomputed) |
| 116 | 0x5a34e0 | 1 | Theme: 0 Parkland, 1 Desert, 2 Tropical, 3 Links | DERIVED numbering | `theme` |
| 117 | 0x59ae78 | 4 | Fun rating | EXACT | - (statistics pass) |
| 121 | 0x541cd8 | 4 | Skill (sum of differentials) | EXACT | - (statistics pass) |
| 125, 129, 133 | 0x5a882c, 0x56949c, 0x5a636c | 4 each | Length, accuracy, imagination sums | EXACT | - (statistics pass) |
| 137 | 0x5a9cac | 4 | Ranked-hole flags term of the Top 10 score | EXACT | - (recomputed) |
| 141 | 0x5a9cb0 | 4 | Sold home sites, recomputed each frame (0x418c1f) | EXACT | - |
| 145 | 0x5787cc | 4 | Fame: tournament places (4 - place, at least 1) | EXACT writer, DERIVED port match | `club.trophies` |
| 149 | 0x561258 | 4 | Happy endings | EXACT | `club.happy_endings` |
| 153 | 0x571fd4 | 4 | Cash (units) | EXACT | `econ.cash` (x 100), `club.cash` |
| 157 | 0x59bf90 | 4 | Offer slot of the course in play (0..15) | EXACT | `land.slot_index` |
| 161 | 0x575ab0 | 0x28a0 | Hole records, 20 of 0x208 (section 5.1) | EXACT layout of the fields used | `club.holes` |
| 10561 | 0x5722e8 | 2500 | Tile type, index x * 50 + y (0x14 for land not owned) | EXACT | `land.ty` |
| 13061 | 0x56988c | 2500 | Tile byte: object index on a footprint, water depth, green look (bit 0x80 tricky) | EXACT | `land.var` |
| 15561 | 0x53caf0 | 5000 | Tile flags, shorts (0x20 path, 0x40 joined, 0x400 footprint, 0x800 weed, 0x4000 being worked on, 0x8000 obstacle, low 5 bits hole or kind + 1) | EXACT | `land.flags`; weeds to the staff's tile state |
| 20561 | 0x53bbac | 2500 | Walking cost per tile (the type's +0x25 when set), signed | EXACT | `land.layer` |
| 23061 | 0x5a4998 | 2601 | Corner heights, 51 x 51, index x * 51 + y, 3..15 | EXACT | `land.height` |
| 25662 | 0x838c1c | 361 | Height noise grid, 19 x 19 signed bytes (17 x 17 read, row 16 and column 16 copies of 0) | EXACT | `noise` |
| 26023 | 0x58bcb8 | 4096 | Object records, 256 of 16 (section 5.2) | EXACT | `land.objects` |
| 30119 | 0x4c284c | 4 | Celebrity homes sold | EXACT | `club.celeb_homes` |
| 30123 | 0x561250 | 4 | Last SGA evaluation score | EXACT | `club.sga_score` |
| 30127 | 0x5794b8 | 0x9c00 | Golfer records, 156 of 0x100 (152 slots and scratch) | EXACT | - |
| 70063 | 0x585850 | 0x1300 | Employees, 64 of 0x4c (section 5.3) | DERIVED | `employees` |
| 74927 | 0x834170 | 4 | Game tick | EXACT | `game_tick`, `econ.tick`, `club.tick` |
| 74931 | 0x5a6364 | 4 | Building kinds unlocked | EXACT | `club.unlocked` |
| 74935 | 0x53a450 | 4 | Land purchases | EXACT | `club.purchases` |
| 74939 | 0x822c88 | 4 | Difficulty 0..3 | EXACT | `difficulty` |
| 74943 | 0x5409ac | 1000 | Monthly skill history, 500 shorts by month | EXACT | `club.history[m][2]` |
| 75943 | 0x51b388 | 1000 | Monthly cash history (units) | EXACT | `club.history[m][0]` |
| 76943 | 0x53e63c | 1000 | Monthly fun history | EXACT | `club.history[m][1]` |
| 77943 | 0x4c2850 | 4 | Fee scratch word (overwritten at each finished hole) | EXACT | - |
| 77947 | 0x59b730 | 4 | Wager level of matches | EXACT | `club.wager_level` |
| 77951 | 0x59ae7c | 4 | Golfer creation counter | EXACT | `club.create_counter` |
| 77955 | 0x59dea0 | 128 | Visitor kind balancing table, 32 ints | EXACT | `club.kind_counts` |
| 78083 | 0x59c08c | 4 | Reaction types already narrated (below 32) | EXACT | `club.narrated[0]` |
| 78087 | 0x568600 | 1000 | Monthly event log, 500 shorts (code \| argument) | EXACT | `club.event_log` |
| 79087 | 0x822c70 | 4 | Landmarks free to place | EXACT | `club.free_landmarks` |
| 79091 | 0x5685f8 | 4 | Hole types already announced | EXACT | `club.types_announced` |
| 79095 | 0x5a5a24 | 4 | Last golfer slot made | EXACT | `club.last_created` |
| 79099 | 0x584210 | 2000 | Yearly ledger, 100 records of 10 shorts by year mod 100 (k0 green fees .. k7 other, k8 total) | EXACT | `econ.ledger` (x 100, years 0 to now) |
| 81099 | 0x543cfc | 4 | Landmark types available | EXACT | `club.landmarks_owned` |
| 81103 | 0x4c2e14 | 4 | Famous golfer who challenged the club | EXACT | `club.challenge_pro` |
| 81107 | 0x572cac | 4 | CEO visits (bit 0x80 while one is on the course) | EXACT | `club.ceo_count` (bit cleared) |
| 81111 | 0x59aaf8 | 4 | Heiress visits | EXACT | `club.donations` |
| 81115 | 0x56ae70 | 0x1700 | Drive statistics by golfer type: 32 records of 0xb8 (+0 yards total, +0x10 drives, +0x14 longest), index skill class * 4 + tee level (0x4289e0) | DERIVED | - |
| 87003 | 0x5422f8 | 240 | 10 records of 6 ints: counts drawn as rows of icons on a report; a new game seeds 9 of them at random (0x442180), a report draws them (0x45f870) | UNKNOWN | - |
| 87243 | 0x59ae80 | 456 | Yardage markers, 19 holes x 3 points (x, y ints, -1 none) | EXACT | `club.holes[h].markers` |
| 87699 | 0x59b048 | 4 | Tick of the last story beat | EXACT | `club.last_beat` |
| 87703 | 0x5736b0 | 0x2400 | 256 records of 0x24 (x, y, ... ints; -1 free), read by the staff routine 0x402a40 and set up by 0x409a90 | UNKNOWN | - |
| 96919 | 0x59fc60 | 0x3880 | Shot replay records, 16 of 0x388 | EXACT | - |
| 111383 | 0x5849e0 | 0xe70 | Membership, 84 of 0x2c (section 5.4) | EXACT | `club.members` |
| 115079 | 0x5a59f8 | 4 | Slot of a pending match (-1 none) | EXACT | - (no match carried) |
| 115083 | 0x5a5a04 | 32 | Skill levels: the pro's 16 bytes, then the opponent's (0x407e00 picks the half) | EXACT | `club.pro_skill` |
| 115115 | 0x56d1b8 | 6000 | Celebrity resident walkers, 100 of 0x3c | EXACT | - (they walk again) |
| 121115 | 0x4c2c94 | 18 | +8 (0x4c2c9c, a short): the pro's skill mask; the rest | EXACT / UNKNOWN | `club.pro_mask` |
| 121133 | 0x5689e8 | 800 | 100 records of 4 shorts, +4 a golfer's identity, cleared with the golfer (0x426670; -1 free) | UNKNOWN | - |
| 121933 | 0x5419d0 | 64 | 4 movers of 16 bytes (x, y, speed 0x28, state byte +0xd), started 1 in 100 per frame after tick 0x2000 at (0, rand 50 tiles) by 0x430360: fly-overs | DERIVED | - |
| 121997 | 0x572cb0 | 2560 | Animals, 128 of 0x14 | EXACT | - (they come back) |
| 124557 | 0x4d6088 | 0xa640 | Profiles 0..75, 0x230 each (+0 job, +0x10 name, +0x20 traits, +0x21 sex and age bits, +0x22 head, +0x23 body, +0x24..+0x28 colours, +0x2c fixed class, +0x30 biography); record 0 is the player's pro | EXACT | `club.roster` |
| 167117 | 0x543d10 | 105000 | Dialogue, 84 of 0x4e2 (25 slots of 50 bytes) | EXACT | `roster[i].sayings` |
| 272117 | 0x567b04 | 4 | SGA purse (thousands) | EXACT | `club.purse` |
| 272121 | 0x5a6378 | 2500 | Happy golfers per tile | EXACT | `course.happy` |
| 274621 | 0x56c7e4 | 2500 | Unhappy golfers per tile | EXACT | `course.unhappy` |
| 277121 | 0x53ea24 | 2500 | Landings per tile | EXACT | `course.landing` |
| 279621 | 0x542414 | 2500 | The tile map under land not yet owned (restored on purchase) | EXACT | `land.original` |
| 282121 | 0x59e7b8 | 4 | Game flags (0x200000 tournament, 0x1000000 sandbox, 0x2000000 reactions off, 0x4000000 championship, 0x40000000 loaded) | EXACT | sandbox only |
| 282125 | 0x4c1578 | 1056 | Accomplishments, 22 of 48: course name (40), tick +0x28 (0 = not earned), site +0x2c | EXACT | `club.awards`, `club.earned` |
| 283181 | 0x571fd8 | 782 | Property list, 17 records of 0x2e (section 5.5) | EXACT | `land.slot` |
| 283963 | 0x520640 | 1000 | Monthly members history | EXACT | `club.history[m][3]` |
| 284963 | 0x567328 | 100 | Theme pack folder name | EXACT | `theme_pack` |
| 285063 | 0x571d38 | 4 | Reaction types already narrated (32 and up) | EXACT | `club.narrated[1]` |
| 285067 | 0x56fcb0 | 0x1002 | Name pool (section 5.6) | EXACT | course name, best rounds' names |
| 289165 | 0x59d81c | 256 | Name offsets, 128 shorts (-1 none) | EXACT | (pool index) |
| 289421 | 0x5a46b8 | 256 | Name lengths with NUL, 128 shorts | EXACT | (pool index) |
| 289677 | 0x56a51c | 4 | Tee order counter | EXACT | `club.tee_counter` |
| 289681 | 0x5a6374 | 4 | Debt counter (0..3, the board's warnings) | EXACT | `econ.debt_stage` |
| 289685 | 0x56a524 | 40 | Best rounds, 10 ints | EXACT | `club.top_rounds` |
| 289725 | 0x5a47e0 | 4 | Tournament preparation state | EXACT | - (reset) |
| 289729 | 0x5830b8 | 2500 | Undo record per tile | EXACT | `land.undo` |
| 292229 | 0x59c090 | 2500 | Undo refund per tile (signed units) | EXACT | `land.refund` |
| 294729 | 0x5a5a00 | 4 | The Preferences' option bits | EXACT | - (the port keeps its own) |
| 294733 | 0x4e06c8 | 0x8c0 | Flag 0x40000000 only: profiles 76..79 (the pro of Pick a Pro and the three VIP files) | EXACT | `club.roster[76..80]` |
| 296973 | 0x58f338 | 0xb7c0 | Flag 0x200000 only: the tournament's copy of all 84 profiles | EXACT | - |
| 344013 | 0x520a28 | 105000 | Flag 0x200000 only: the tournament's copy of the dialogue | EXACT | - |

## 5. Records

### 5.1 Hole record (0x575ab0 + h * 0x208, h = 1..18; 0 unused, 19 the hole being edited)

+0 par (byte, 0 = not open), +1 tee facing, +2 forward tee to pin heading (signed bytes), +4 length (short, 25 a tile), +8 /
+0xc back tee x, y, +0x10 / +0x14 forward tee, +0x18 / +0x1c pin (ints, tiles), +0x20 tee shots, +0x24 shots planned, +0x28
strokes histogram, +0xd8 events, +0x158.. mood, quits, drives, fairways, greens in regulation, putts, longest drive, +0x1ec
play time, +0x1f0 money spent building it, +0x1f4 green fees taken, +0x1f8 upkeep, +0x1fc likeness to the previous hole,
+0x200 flags (1 Top 100, 2 Top 18, 0x20 / 0x40 dogleg, 0x1000 uphill, 0x2000 downhill). EXACT for +0, +8..+0x1f, +0x200
(read at many call sites, e.g. the hole name 0x407280, the staff creator 0x402970, the
statistics pass 0x42dea0); DERIVED for the rest (the port's hole record, golfer.rs `Hole`). The
importer carries +0 to +0x27 and +0x1ec to +0x203; the histogram, event and shot statistics start again.

### 5.2 Object record (0x58bcb8 + i * 16, 256)

+0 kind (short, -1 free; the creator 0x40e000 takes the first free record and gives up past 255), +2 x, +4 y (shorts,
anchor tile), +6 facing (byte, a rolling global & 3 when built), +7 flags (0x40 joined to the clubhouse), +8 an int (landmark
look for kind 4, building level for kinds 6 and up, the buyer for a home site), +0xc an int (a home site's lot value). EXACT
(0x40e000 and its readers). The tiles of a footprint hold the record's index in 0x56988c, so the importer keeps the records
at their indices (free ones included) up to the last one used.

### 5.3 Employee record (0x585850 + i * 0x4c, 64)

+0 / +4 position (map units, 1024 a tile), +0x10 / +0x11 post tile (signed bytes, -1 none), +0x12 state (1 active, 8
experienced, 0x10 walking home to leave), +0x13 job (-2 Club Pro, -3 Ranger, -4 Groundskeeper, -5 Soda Vendor, -6 the
player's pro), +0x14 action and +0x15 its countdown, +0x16 facing 0..7, +0x18 steps, +0x1a wait, +0x1c a short cleared on
arrival, +0x1e animation (11 standing), +0x20 golfer served (short), +0x22 month hired (tick >> 10, short), +0x24 wages paid
(units, short), +0x26 golfers served (short), +0x2a the name. EXACT for +0..+0x16, +0x1e, +0x22 (the creator 0x402970, the hire
code 0x40aa80, the employee list 0x436e50), DERIVED for the rest. The importer carries active employees not leaving, their
place, post, job, experience, facing, hire month, wages and count served; they start standing (what they were busy with is
not carried). The employee names (+0x2a) are not carried.

### 5.4 Member record (0x5849e0 + i * 0x2c, 84, by person)

+0 best round, +1 average (signed), +2 level (low 3 bits: 0 not invited, 1 visitor, 2 member, 3 silver, 4 gold, 5 platinum),
+3 + h per hole marks (1 SimFoto, 2 happy ending, 4 quit), +0x16 + h last round's card, +0x29 0xff gone for good, +0x2a rounds
(short). EXACT (DECODE_HOMES 1.1, golfer.rs `Member`).

### 5.5 Property list (0x571fd8 + slot * 0x2e)

Built by 0x46f2b0 for the 16 offer slots (a 17th record is zero): +0 the place's name (from the site table 0x4c1e90, stride
0x82), +0x18 / +0x1a map pin x, y (shorts), +0x1c site id (the index of the site table, whose order is `land::RECORDS`'s:
Monterey .. Jamaica), +0x1d acres (250 in sandbox, stored as -6), +0x1e terrain (relief), +0x1f lie, +0x20 theme, +0x22 / +0x23
0xff until bought. (DECODE_WORLD2 4.1 counts these from the site id and lists lie, theme and terrain in another order;
the writes in 0x46f2b0 give the order here.) EXACT (field writes in 0x46f2b0; the site table read from the exe). The course in play is record 0x59bf90.

### 5.6 Name pool (0x56fcb0, 0x1002 bytes; 0x45b8b0 adds, 0x45b9f0 reads)

128 names by index: offset (0x59d81c) and length with NUL (0x5a46b8), -1 when there is none; a name replaced is cut out and
the later offsets move down. Index 0 the course's own name, 1..18 hole names (the default names come from tables by par,
0x4c2e38 / 0x4c2e88 / 0x4c2ed8, in 0x407280), 0x14 + k the golfer of best round k. EXACT. The port has no hole names; the importer says so
when a save has any.

## 6. What the import leaves behind, and why

* Golfers on the course, their shots in the air and replays, stories under way (golfer records and story steps): the port
  runs golfers from its own state and new ones arrive at once. Carrying a round in progress across engines risks a golfer
  stuck between states.
* Animals, fly-overs and celebrity residents: they come back on their own. Hole statistics beyond those listed in 5.1:
  rebuilt as golfers play. The Preferences' bits: the port keeps its own preferences.
* A tournament or match under way: its backup blocks are read past; the game comes over between events.
* Blocks whose meaning is UNKNOWN (0x5422f8, 0x5736b0, 0x5689e8, most of 0x4c2c94) are not used.
* Other properties bought on the world map (the property list's purchase marks): only the course in play comes over.

## 7. What was verified

* The block list, sizes, order and the flag-dependent tail: EXACT, read from 0x40afa0 (writer and loader are the same
  routine). The importer's tests write files of this layout and read them back, and refuse truncated, padded, random and
  damaged files without panicking.
* No saved game made by the original was available while this was written (the game cannot run here and no player's save
  was at hand), so no real file has been read yet. The layouts of the records in section 5 rest on the exe's code and on
  the port's earlier decodes, not on a file. A real save that is refused or reads wrongly is the thing to report.

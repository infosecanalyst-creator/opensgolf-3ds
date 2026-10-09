# Decode: how a comment event becomes a sentence (Hole Stats rows and the Player Comments report)

Source: publisher exe only (`golf_pub.exe`, decompile `spec/golf_decomp.c`). Clean-room notes: facts and structure in my own words; the game's sentences are NOT reproduced. Each type gets the meaning, the build structure, the argument sources, and a 2 to 4 word quoted fragment for identification, plus an original paraphrase that an implementation can use instead.

Tags: EXACT = read directly from the code. DERIVED = follows from code plus a stated assumption. UNKNOWN = not established.

Routines involved (addresses in the publisher exe):

| Routine | Role |
|---|---|
| 0x469b00 | the sentence builder, `build(eventType, location, holeArg, golferIndex)`; appends to the global text buffer at 0x51a068 and leaves a colour word in the global 0x58b198 |
| 0x4546b0 | Player Comments report (F2) |
| the Hole Stats dialog routine (comment loop near decompile line 52307) | Hole Stats rows |
| 0x407700 | name of the scenery object on a map cell (appends to the buffer) |
| 0x4074a0 | name of a landmark kind (appends) |
| 0x4676e0 | golfer name (appends); 0x467560 adds an endearment word for a partner |
| 0x40a9a0 | club name; 0x407280 hole name; 0x469a20 course-opinion sentence (types 19 only); 0x45b7c0 replace first occurrence of a token in the buffer |
| 0x46c940 | "male" test of a golfer: 1 when bit 7 of the golfer profile byte at +0x21 is clear |

## 1. The calling context for both views (EXACT)

Both views call `build(type, storedLocation & 0x3fff, holeArg, 0x98)`.

* Golfer index 0x98 is a scratch golfer slot. Its 0x100 byte record (0x582cb8 to 0x582db8) is zeroed at the start of every call with this index. So in both views the speaker is a "null golfer": profile index 0, conversation state 0, all flag words 0, scorecard all 0, kind byte 0.
* Profile 0 is the first row of the 0x230 byte golfer profile table at 0x4d6088: title (+0x00) is the class title, name (+0x10) is the personal name. The default player name in this table is the one the HUD shows. Any `MYNAME` or `PARTNER` token in a sentence is replaced by a name produced by the name routine for golfer 0x98 and golfer 0x99 (0x98 xor 1). Slot 0x99 is never initialised for these views (DERIVED: it reads as zero, so profile 0 again). Net effect: both tokens become profile 0's personal name. UNKNOWN whether slot 0x99 holds stale data during play.
* `holeArg` : Hole Stats passes `holeNumber * 11`; the builder divides by 11 first, so the builder sees the 1 based hole number. The report passes -1, which divides to 0 (C truncation), so hole dependent types in the report read hole record 0, an unused record (UNKNOWN content, probably zero). This only matters for types 23 and 30 (and 19, 59 which are never shown).
* The 0x3fff mask removes the top two bits of the stored location word; those are not used by any sentence.
* Types with no case in the builder's switch (0, 64 and anything undefined) produce an empty sentence (only the surrounding quotes show). Types are only drawn when the reaction routine stored a non zero count, so this does not occur in practice.
* For golfer index below 0x98 (in game speech bubbles) the builder first looks for a user typed "signature saying" override for 20 event slots (types 62, 1, 4, 5, 31, 2, 3, 9, 12, 13, 28, 20, 39, 21, 14, 25, 15, 18, 26, 27); that does not apply to these views. Index 0x99 is the signature editor and wraps the object name in a literal placeholder form; also not applicable.

## 2. Text assembly (EXACT)

1. The caller puts a prefix in the buffer (see section 6), then calls the builder.
2. The builder appends a sentence made of fixed pieces. Pieces that need runtime words contain three literal tokens: `MYNAME`, `PARTNER` and `DATA`.
3. After the switch the builder saves the buffer, then builds three replacement strings: the speaker's name, the partner's name, and the "object phrase" (see section 3). It restores the sentence and replaces the FIRST occurrence only of each token, in this order: `MYNAME`, `PARTNER`, `DATA`. Sentences contain each token at most once.
4. Colour is set inside each sentence variant by writing one of three 32 bit words into the global at 0x58b198 (default 0x80006318 neutral, 0x800023e8 good, 0x80007d08 bad).

Articles and plurals are never computed by general grammar. The fixed pieces carry the article ("this", "that lovely", "the") and the object name is inserted bare. The only grammar logic is:
* type 10 chooses between two demonstratives by testing whether the inserted terrain name ends in the letter "s" (plural form) or not (EXACT).
* types 5/37/38/31 insert the plural form of the name when the terrain tile is a tree class (section 3.1).
* types 2 and 3 choose the preposition by tree class (section 3.1).
* type 11 chooses between two adjectives by testing bit 0x100 of the location word (so it varies with the cell position, pseudo randomly) (EXACT; the meaning of that bit is a side effect of the cell numbering).

## 3. Location argument to object or terrain name

### 3.1 Terrain table (static table at 0x4c1a40 in the exe, copied to 0x578350 at run time; 0x30 byte records, ids 0 to 22)

Fields used by the comment code: name at +0x00 (singular form), alternate name at +0x10 (plural form), class byte at +0x26. EXACT.

| id | singular name | class (+0x26) |
|---|---|---|
| 0 | tees | 0 |
| 1 | green | 1 |
| 2, 3 | fairway, firm fairway | 2 |
| 4, 5, 6 | rough, deep rough, mound | 4 |
| 7, 9 | sand trap, pot bunker | 7 |
| 8 | waste bunker | 8 |
| 10, 11, 12 | ravine, brush, rocks | 4 |
| 13 to 16 | tree, pine tree, palm tree, elm tree | 13 (tree class) |
| 17 | water | 17 |
| 18, 19 | wetlands, marsh | 4 |
| 20 | out of bounds | 18 |
| 21, 22 | building | 18 |

Name selection when the sentence uses a terrain id (EXACT):
* "Default form" (all types not listed below, with `DATA`): plural name (+0x10) when class is 13, else singular (+0x00). Plural names are: trees, pine trees, palm trees, elm trees.
* "Singular form" (types 10 and 12): always the +0x00 name.
* "Lie phrase" (types 2 and 3, `DATA` replaced by a whole phrase): a locative preposition plus the singular name. The preposition is the "beneath" form when class is 13, otherwise the plain "inside" form. Example result shape: `<in|under> the <terrain>`.
* Type 60 (not shown in these views) appends the same "beneath the" phrase after its main clause when the id is tree class.

The id range is 0 to 22; a stored id above 22 would read past the table (UNKNOWN, not expected).

### 3.2 Object phrase for scenic comments, types 11, 20, 28 (cell lookup)

The stored location is a cell number `a + 50 * b` where `a` and `b` are the two tile coordinates of the cell that triggered the comment (position components divided by 1024). The builder derives `a = loc mod 50`, `b = loc div 50` and calls the object namer, which reads the terrain id from the map byte at index `a * 50 + b`. EXACT. The namer appends a bare name (no article, no plural logic). Name logic, by tile:

* Building ids 21 and 22: read the object index stored for the cell. If that object is a landmark (record kind 4), the name is the landmark kind's plain name (19 kinds, ids 0 to 18; kinds 0 to 15 are scenic attractions, 16 to 18 are the "unsightly" ones used by the ugly comment: water tower, radio antenna, oil pump). If the record kind is 2: a generic flowerbed word. Otherwise: "landmark" for id 22 and "home site" for id 21. The landmark list has plain names such as sundial, barn, cannon, stonehenge, water mill, rock face, statue, lighthouse, Buddha, windmill, Easter Island head, pagoda, oriental house, dinosaur tarpit (the article carrying variants beginning with "n " exist for another caller and are NOT used by comments).
* Otherwise a switch on the tile class:
  * class 4 with id 4 (rough): if the cell's decoration flag 0x1000 is clear, a special one off name: a gold pot name when a course attribute byte equals 10, the desert theme gets a bones name, other themes get an ornamental grass name. If the flag is set, a flower bed name chosen by `cellObjectByte mod 5` from a per theme list: theme 1 (desert) five names, theme 2 (tropical) five names, themes 0 and 3 five names (colour plus flower kind words; one of them, a red rose bush, is shared between lists). Any other theme value returns no name.
  * class 4 with ids 5, 6 and others (ids 10 and 12 branches are unreachable with this table): a vineyard when a course attribute byte equals 13, otherwise a rose bush. Ids 10 and 12 have unreachable special branches (scenic tree, rock formation, natural bridge by theme).
  * class 7: a shrub name (rhododendron, spelled with a typo in the exe).
  * class 13 (trees): elm gives "scenic tree"; on the desert theme ids 13 to 15 give "scenic cactus"; id 13 and 14 give "scenic tree"; id 15 gives "statue"; any other id gives nothing.
  * class 17 (water): cell flag 0x20 set gives "scenic bridge"; else a dolphin when the cell has an object byte and the tile really is water; else on the desert theme "rock formation", otherwise "fountain".
  * class 18 (out of bounds, id 20): "fountain".
  * all other classes (tees, greens, fairways, waste bunker): "wildflower".
  * Meaning of the two course attribute bytes (table at 0x571ff4, stride 0x2e, indexed by the current course global 0x59bf90) and exact theme numbering of global 0x5a34e0: UNKNOWN (0 parkland, 1 desert, 2 tropical, 3 links is DERIVED from the panel art names and from the flower lists).
* Types 40 and 44 call the namer with a terrain id instead of a cell (cell 0,0 then supplies the decoration flags); see their rows.

### 3.3 Other location meanings

| Type | location meaning |
|---|---|
| 22 | index into the celebrity name table at 0x55d738 (record size 37), the same table the home owner screen uses (EXACT) |
| 39 | animal index into a table at 0x4c1998, record 18 bytes, name in the first 16. Nine entries (0 to 8): elk, crane, flamingo, sheep, crocodile, snake, Gila monster, road runner, duck. Value 9 or more reads garbage (UNKNOWN) |
| 36 | gender flag of the golfer that flipped out: 0 selects the feminine wording, anything else the masculine (EXACT) |
| 34 | reply selector 0, 1, 2 (three replies). Other values add no text but the colour still applies (EXACT) |
| 7 | always 0 when stored (a bridge comment is only recorded for 0) (EXACT) |
| 1, 2, 3, 10, 12, 13 | terrain id (EXACT for the use; DERIVED for the producers, which pass the id of the lie or tile) |
| most others | the constant 20, which the text ignores |

## 4. Event types: structure, arguments, colour

Columns: type decimal (hex); meaning; reaction delta from DECODE_HOLE_STATS section 3 (base, before the negative scaling); colour class the builder sets (G good, R bad, N neutral); reachable in the two views (a type is only stored, hence shown, when its delta is non zero, and both views only look at types 0 to 49).

IMPORTANT: colour is a property of the sentence, NOT of the delta sign. The two agree for all shown types except 26 (negative delta, neutral black) (EXACT).

Null golfer text = what these views print. "Other variants" are what an in game speech bubble could pick (never shown here).

### 4.1 Types stored and shown (delta not 0, type below 50)

| Type | Meaning | Delta | Colour | Structure, argument sources, null golfer text | Fragment | Original paraphrase |
|---|---|---|---|---|---|---|
| 1 (0x01) | good shot | +1 | G | Null golfer: a rhetorical question to the partner with the partner name at the end (token PARTNER). Other variants, picked by profile index bits (bit 0 and 1): bit 0 set gives one of two self boasts; bit 0 clear and bit 1 set gives a "nothing but <terrain>" remark (DATA, default form, location = landing terrain); and when the conversation state is odd the remark is praise addressed to the partner, starting with the speaker token and followed by one of two compliments by the male flag and profile bit 0 | "How'd you like that" | "Did you see that one, PARTNER?" |
| 2 (0x02) | bad lie, same terrain kind | -1 | R | Null golfer: a groan plus the lie phrase (DATA = `<in|under> the <terrain>` from location). Other variants (state 2 or 4): a remark addressed to the partner naming them and the lie | "Darn, I'm" | "Ugh, I'm stuck DATA." |
| 3 (0x03) | bad lie, different terrain | -2 | R | Null golfer: an exclamation chosen by profile index low 2 bits (null = first, a sarcastic cheerful one) then a clause with the lie phrase. Other variants: partner directed | "Great, now I'm" | "Wonderful, now I'm DATA." |
| 4 (0x04) | missed an easy shot | -2 | R | Null golfer: a plain observation that the shot looked easy. Variant (state 4): teasing the partner by name | "looks pretty easy" | "That one should have been easy." |
| 6 (0x06) | uses a slope | +1 | G | Fixed sentence, no arguments | "use the slope" | "I can use this slope." |
| 7 (0x07) | scenic bridge | +1 only if loc 0 | G (loc 0) | Walking variant when golfer flag 0x10000 is clear (null), riding variant when set. Fixed otherwise. A location other than 0 prints a "seen it before" sentence with neutral colour, but then no delta is given, so it is never stored | "walking over this" | "Love strolling across this bridge." |
| 8 (0x08) | bad design | -2 | R | Fixed sentence with a bleeped swear word in the middle (the swear is punctuation characters) | "Who designed this" | "Who laid out this course?" |
| 9 (0x09) | ball nearly hit someone | -3 | R | Fixed | "ball almost hit" | "That ball nearly took my head off." |
| 10 (0x0a) | forced to walk through an obstacle | -2 | R | Fixed opening, then a demonstrative chosen by the last letter of the singular terrain name: plural demonstrative if it ends in "s", else singular; then the singular name; then a question mark. Location = terrain id | "Must I walk through" | "Do I really have to walk through <this or these> <name>?" |
| 11 (0x0b) | scenic object | +1 | G | Opening ("Look at" form when conversation state is 1, else "Check out" form), adjective A or B by bit 0x100 of location (set gives the plainer adjective, clear gives the grander one), then DATA = object phrase from the cell (section 3.2), then an exclamation mark. State 1 also appends an endearment/partner name | "Check out this" | "Take a look at this <adjective> DATA!" |
| 12 (0x0c) | obstacle trouble (tree, rocks) | -1 | R | Fixed opening plus DATA = singular terrain name, location = terrain id | "Darn that stupid" | "Blasted DATA." |
| 13 (0x0d) | ball in water | -1 | R | Null golfer: one of three fixed sentences chosen by profile index mod 3 (null = first). State 2 or 4: "heard a splash" question that also names the partner. Location unused by the text | "Argh, I drowned" | "My ball is sleeping with the fishes." |
| 14 (0x0e) | thirsty | -1 | R | Fixed | "a little thirsty" | "I could use a drink." |
| 15 (0x0f) | hungry | -1 | R | Fixed | "get hungry" | "Time for some food." |
| 18 (0x12) | snack eaten | +1 if hunger over 7 | G | Fixed | "good snack" | "Nothing beats a snack." |
| 20 (0x14) | ugly object | -2 | R | Fixed frame with a vowel initial adjective ("an ugly") then DATA = object phrase from the cell (ugly landmarks are kinds 16 to 18), then a period | "what an ugly" | "What an eyesore, that DATA." |
| 21 (0x15) | slow play | -2 | R | Fixed, plural address ("these ..."), location unused | "tired of waiting" | "Can we speed this group up?" |
| 22 (0x16) | celebrity house | +1 | G | Fixed opener, then the celebrity name from the table, then a possessive suffix and the house word | "Hey that's" | "Look, that's NAME's place." |
| 23 (0x17) | hole too hard or too easy | -1, or -2 when difficulty above 0 | R | See section 4.3 (leading score word, then one of three). Only the hole's flag bits matter | "too hard" | "This hole is too tough." |
| 24 (0x18) | weeds | -2 | R | Fixed | "something growing here" | "Eww, the weeds are taking over." |
| 25 (0x19) | drink taken | +1 if thirst over 7 | G | Fixed | "foamy beverage" | "Ah, just what I needed to drink." |
| 26 (0x1a) | tired | -1 | N | Null golfer: first variant, a plain remark. Other variant (state not even): "You look a bit tired" plus the partner name or endearment. Neutral colour although the delta is negative | "starting to get tired" | "I'm getting worn out." |
| 27 (0x1b) | bench | +1 if fatigue over 59 | G | Fixed | "bench is comfortable" | "What a comfy bench." |
| 28 (0x1c) | lovely object | +1 | G | State odd (1, 3, 5): "Look at that lovely DATA" plus endearment; else four variants chosen by (male flag, profile index bit 0). Null golfer (male, bit 0 clear): the "I am liking that DATA." form. DATA = object phrase from the cell | "I am liking" | "I really like that DATA." |
| 29 (0x1d) | variety | +1 | G | Fixed | "variety on this" | "Good variety out here." |
| 30 (0x1e) | repetition | -2 | R | Uses hole H and hole H-1 flag words (word at +0x200 in each hole record) and pars. Order: both bit 0x20 (left bend) gives the dogleg-left sentence; else both bit 0x40 gives the dogleg-right sentence; else equal pars gives a "yet another par N" sentence with N printed in decimal and a period; else the generic "like the last hole". Report reads record 0 and the record before it (UNKNOWN content) | "like the last" | "This one feels like the last hole." |
| 32 (0x20) | choosing distance | +1 | G | Fixed | "go long or" | "Should I go long or play safe?" |
| 33 (0x21) | choosing side | +1 | G | Fixed | "left or right" | "Left side or right side?" |
| 34 (0x22) | reply to a partner's chat | +1 | G | By location 0, 1, 2: three fixed replies of rising warmth; other locations add nothing | "thanks for asking" | "Doing great, thanks." |
| 35 (0x23) | tantrum | -2 | R | Four fixed rants chosen by `((tick + 45 * golfer) div 80) mod 4`, where tick is the game tick counter at 0x834170. So the shown rant rotates about every 80 ticks while a view is open | "never play again" | "I'm done with this game." |
| 36 (0x24) | another golfer snapped | -3 | R | Two fixed variants by location: 0 feminine, non 0 masculine | "flipped out" | "That golfer has lost it." |
| 39 (0x27) | scared an animal | +1 when difficulty below 2 | G | Two phrasings chosen by the male flag (null golfer is male so the first); DATA = animal name from location | "scared that little" | "Oops, I startled that DATA." |
| 43 (0x2b) | steep slope | -2 | R | Fixed | "steep slope" | "Do I have to climb this hill?" |
| 44 (0x2c) | consolation | +1 | G | Fixed opener, then an opening parenthesis and the word for "nice", then DATA = object phrase from a TERRAIN ID (not a cell), then a closing parenthesis. Caller of this type not located (UNKNOWN reachability) | "Never mind" | "Never mind. (good DATA)" |
| 46 (0x2e) | downhill shot | +1 | G | Fixed | "nice downhill shot" | "A friendly downhill lie." |
| 47 (0x2f) | partner attitude | -2 | R | Four fixed lines chosen by profile index low 2 bits (null = first); each carries the PARTNER token | "your attitude stinks" | "PARTNER, lighten up." |

Delta 0 or conditional and therefore never shown in these views: 5, 16, 17, 31, 37, 38, 40, 41, 42, 45, 48, 49 (and 19, which never reaches the store). Their colour and builder text are listed in section 4.2 for completeness.

### 4.2 Types that are never stored (completeness)

| Type | Meaning | Colour | Structure |
|---|---|---|---|
| 5, 37, 38 (0x05, 0x25, 0x26) | hazard ahead (stay away, hit past, watch out) | N | Opening chosen by type, then DATA = default form terrain name. Variants by conversation state (partner directed ones add endearments) |
| 16, 17 (0x10, 0x11) | hook, slice | N | Fixed |
| 19 (0x13) | hole score | N for score based branches | Leading score word by `strokes - par` (see 4.3) then a course opinion by mood (0x469a20, ten sentences) or a skill remark; rewritten to type 23 before storage |
| 31 (0x1f) | lots of one terrain | N | Four plural-friendly frames by profile index low 2 bits, DATA = default form |
| 40 (0x28) | good shot exclamation | N | Eight exclamations by profile index low 3 bits, then (if location positive) a parenthesis with an object phrase from a terrain id |
| 41 (0x29) | after a shot | N | Two fixed lines by golfer slot parity |
| 42 (0x2a) | lucky bounce | N | Fixed, then the partner name |
| 45 (0x2d) | uphill shot | N | Fixed |
| 48, 49 (0x30, 0x31) | chat openers | N | 48 is a question to the partner; 49 is "I'm a" plus the profile title |

### 4.3 Type 23 detail (EXACT)

Type 19 and 23 share the builder entry. The builder FIRST appends a leading "score word" and then picks the sentence. For the null golfer the scorecard byte is 0, so the difference `score - par` equals minus the hole's par. Score words by difference: 0 par word, +1, +2, +3 bogey words, -1 birdie, -2 eagle, -3 double eagle, -4 triple eagle, anything else a frustrated sound.
Effects for the views:
* Hole Stats on a hole with par 2, 3, 4: the leading word is the eagle, double eagle or triple eagle word (an odd result of the null golfer, but it is what the code does). Par 5 and 6: the frustrated sound. (DERIVED from the zero scorecard; the screenshot in SCREENSHOT_NOTES shows the same style of "Score word. This hole is too easy." in a live bubble.)
* Report: hole 0, par byte probably 0, giving the par word (UNKNOWN).
Then sentence by hole flag word (bit 0x4 too hard, bit 0x8 too easy), tested with the null golfer so the "other golfer's score" gates are bypassed: flag 4 set gives the "too hard" line; else flag 8 set gives "too easy"; else the combined "too hard or easy" line. All three are red.

## 5. Colour rule (EXACT, both views identical)

After the builder returns, the caller reads the global colour word and maps it:

| global value | meaning | pen used |
|---|---|---|
| 0x800023e8 (good) | green | 0x80001284 (dark green) |
| 0x80007d08 (bad) | red | 0x80007d08 (red, unchanged) |
| anything else (default 0x80006318) | neutral | 0x80000000 (black) |

The Hole Stats code compares the global right after the builder and the buffer copy, with the same constants as the report; the decode doc's note "not exact" is resolved: the rule is exactly the report's. The whole line (including the percent prefix) uses the one pen. The words are 16 bit colour constants with a flag in bit 31; the 0x6318 neutral value is a grey that is never drawn because the neutral case maps to black.

Mapping type to colour: see the Colour column. Summary of shown types:
* G (green family): 1, 6, 7, 11, 18, 22, 25, 27, 28, 29, 32, 33, 34, 39, 44, 46.
* R (red): 2, 3, 4, 8, 9, 10, 12, 13, 14, 15, 20, 21, 23, 24, 30, 35, 36, 43, 47.
* N (black): 26 (despite its negative delta).
Only type 7 can in principle emit both neutral and green; the neutral form is not reachable.

## 6. Percentage, prefix, ordering, caps

### Hole Stats rows (EXACT)
* Copy the hole's 64 reaction counts (shorts at hole record +0xd8) into an int array. Five passes. Each pass scans types 0 to 49 only (types 50 to 63 ignored), picks the largest count with a strict greater-than test, so ties go to the lowest type number. If that count is 0, or the hole's counted rounds (record +0x20) is 0, nothing is drawn but the pass is still consumed. Otherwise draw the row and zero that count so the next pass finds the next one.
* Row text: `percent` then a percent sign and three spaces, then an opening single quote, then the sentence, then a closing single quote. `percent = count * 100 / rounds` with C truncating integer division; not capped at 100 (a type can exceed 100 percent since one round can store many reactions).
* The row is drawn centred at x 400 and advances by one strip pitch per drawn row. Rows are a maximum of five. Location text argument is the hole's own stored location for that type (`& 0x3fff`).
* Not open (par byte 0): the first row is a blinking red "under construction" line instead; comment rows still follow if counts exist.

### Player Comments report (EXACT)
* For each type 0 to 63 sum the counts over holes 1 to 18 (record +0xd8 of each hole), find per type the hole with the largest count (first hole wins ties, strict greater test starting from 0), and sum the hole rounds (record +0x20, int) over the 18 holes into one total.
* 20 passes. Each pass scans types 0 to 49 for the largest total (strict greater test, lowest type wins ties). If the total is 0, or the sum of rounds is 0, nothing is drawn; otherwise the row is drawn and that total zeroed. Row y pitch 15 only advances when a row is drawn. Maximum 20 rows.
* Column 1 (left aligned at x 182): single quote, sentence, single quote. The sentence uses the location stored in the best hole's record for that type, with `holeArg` -1 (record 0, see section 1).
* Column 2 (centred at x 504): the 1 based hole number of the best hole as plain decimal (never a name).
* Column 3 (centred at x 598): `total * 100 / sumOfRounds` truncating, then a single percent sign (no trailing spaces, no quotes). Not capped at 100.
* Colour: section 5, applied to column 1 only (the pen set before drawing the sentence; columns 2 and 3 are drawn after it with whatever pen is current, so they carry the same colour). DERIVED from call order: the pen is set once before the first draw and not reset until after the row.

## 7. Special handling summary

* Hole number substitution: no sentence prints a literal hole number except type 30's par number and the report's hole column. Types 23 and 30 take hole data from the hole argument (section 1).
* he/she: only type 36 uses the location as a gender selector (0 feminine). Other gendered wording is chosen by the male flag of the null golfer (type 39, type 28), which is male.
* Golfer name use: `MYNAME`/`PARTNER` become profile 0's personal name for the null golfer. Type 1 and type 47 are the types of this view that print the partner name.
* Per theme words: only the scenic object names (section 3.2). Course attribute bytes select a gold pot decoration and the vineyard word (UNKNOWN meaning).
* Tier words (types 51 to 53, not shown): driving range, pro shop and putting green messages choose new, upgraded, deluxe by globals 0x5a8c60, 0x5a8c58, 0x5a8c50 (values 1 to 3). NOTE: DECODE_HOLE_STATS section 7 says 0x5a8c60 is a "theme index" in the par rule; this routine uses it as the driving range tier. One of those readings is wrong (UNKNOWN which; recheck the par rule).
* Clock dependence: type 35 rotates with the tick counter (section 4.1).

## 8. Types 50 and above (never shown by either view, listed for completeness)

50 uses a separate routine (0x466b70). 51 to 53 driving range, pro shop, putting green: when `(tick + 5 * golfer) & 8` is 0 a fixed praise sentence, otherwise a tier sentence by the tier global (neutral). 54 club choice: fixed opener, club name from index (0 driver, 1 and 2 woods, 3 to 10 irons 2 to 9, 11 lob wedge, 12 sand wedge, 13 putter), fixed closer. 55 to 57 draw, fade, high soft shots (fixed, neutral). 58 reply to a partner question by location 0 to 2. 59 hole record remarks (by hole fields: last time score, top 100 name, top 18). 60 low running shot, optionally with the tree class "beneath" phrase. 61 small talk by location 3 to 7 and golfer parity. 62 greeting indexed by a profile byte (+0x22) plus 20 if not male, from a table with 0x44 byte slots at 0x4d55ec. 63 tournament talk by golfer kind and slot. 65 lost ball.

## 9. Suggested implementation notes

* Keep one function `commentText(type, location, hole, nullGolfer)` returning `{text, colour}`, with tokens replaced once each in the order given.
* For these views the speaker is the null golfer: use profile 0 name for both tokens, male, state 0, flags 0, scorecard zeros.
* Use your own sentence wording (section 4 paraphrases) but keep the structure (what is inserted, in what order, and which argument decides each variant).
* Replicate the percent formatting and the quotes only if matching the original look is wanted; they are cosmetic.

## 10. Port status
`include/sg/comments.h` and `src/comments.cpp` implement the builder with the port's own sentences: token replacement (each token once, fixed order), terrain name and plural rules, the lie phrase, scenic object names by tile class and theme (flower bed lists are own words), the three colour classes, type 23 score word from the null golfer, type 30 hole comparison, type 35 rotation by tick. Placeholders: the report builds type 23 and 30 sentences against the row's own hole (the exe reads record 0), terrain lookups for plain ground cells read as wildflower, the partner name is one fixed name. Glance events in the port feed types 11, 20 and 22 with a cell or celebrity location; other producers (shots, thirst, slow play) do not exist yet.

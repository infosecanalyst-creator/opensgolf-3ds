// Layout and rule tables for four more information screens plus the overview text positions: Histograph (F3), Professional
// Accomplishments (F10), World map / property chooser (F6), Buy Land pricing, and the Aura / Value / Routing text of the course
// overview. Values were read from the publisher exe's draw routines; docs/UI_SCREENS2.md has sources and the exact / derived /
// unknown marks. Coordinates are 800x600 screen pixels. Sheets are loaded from the disc folder at run time; nothing is copied here.
// Style follows include/sg/ui_screens.h (same Rect, Pt, Field and colour constants, reused from there).
#pragma once
#include <cstdint>
#include "sg/ui_screens.h"

namespace sg::ui_screens2 {

using ui_screens::Align;
using ui_screens::Field;
using ui_screens::Pt;
using ui_screens::Rect;

// ---- 1. Histograph (F3) ------------------------------------------------------------------------------------------------------
namespace histograph {
constexpr const char* kSheet = "histograph.pcx";          // Interface/infoscreens, + histograph_alpha.pcx; full panel blitted at (0,0)
constexpr const char* kTitle = "HISTOGRAPH";
constexpr Field title{375, 20, Align::Left};              // large face; left edge, text is about 50 px wide so it centres near 400 (derived)
// Legend labels under the plot, centred, body face, black. Labels: Skill, Cash, Fun, Event.
constexpr int kLegendY = 542;
constexpr int kLegendCx[4] = {177, 332, 488, 644};
constexpr const char* kLegend[4] = {"Skill", "Cash", "Fun", "Event"};
// Plot area. The exe sets this as the clip rectangle before drawing the lines.
constexpr Rect plotClip{106, 72, 598, 450};
constexpr int kBaselineY = 521;                           // y of value 0; a point of height h is drawn at 521 - h
constexpr int kFirstX = 107;                              // x of the origin; one sample per game month
constexpr int kMaxStep = 4, kMinStep = 1;
// x step per month = clamp(600 / (months + 1), 1, 4); months = game tick >> 10 (8 months per year, 1024 ticks each).
constexpr int stepPx(int months) { int s = 600 / (months + 1); return s < kMinStep ? kMinStep : (s > kMaxStep ? kMaxStep : s); }
// Axis tick labels: ten rows, k = 0..9 from the bottom.
constexpr int kTicks = 10;
constexpr int tickY(int k) { return 514 - 50 * k; }       // 514, 464 ... 64 (label vertical position, centred text)
constexpr int kLeftLabelCx = 77, kRightLabelCx = 727;
constexpr uint32_t kLeftLabelColour = 0x80004010u;        // same colour as the skill line
constexpr uint32_t kRightLabelColour = 0x80000000u;
// Series. Arrays hold 500 shorts, one slot per month (slot = (tick >> 10) % 500), written every month by the economy tick.
//   skill      = skill rating (DAT_00541cd8), colour 0x80004010
//   cash       = cash in money units (DAT_00571fd4, money = units * 100), black when >= 0, red (0x80007d08) when negative
//   fun        = fun rating (DAT_0059ae78), colour 0x800003e0
//   fourth     = count of hired staff records whose low three bits of the type byte exceed 1 and that are not marked 0xff (derived), 0x80000210
enum Series : int { Skill = 0, Cash = 1, Fun = 2, Staff = 3 };
constexpr uint32_t kColSkill = 0x80004010u, kColCash = 0x80000000u, kColCashNeg = 0x80007d08u, kColFun = 0x800003e0u, kColStaff = 0x80000210u;
constexpr int kStaffPxPerUnit = 4;                        // fourth series is drawn 4 px per unit, unscaled
// Height in pixels of a scaled value v (skill / scale, cash / scale, or fun): compressed above 500.
constexpr int barHeight(int v) { return v < 501 ? (v < 0 ? -v : v) / 2 : v / 10 + 200; }
// Cash divisor from the current cash (units): 1, then 2, 4, 10, 20, 40, 80 as cash passes 2500, 5000, 10000, 25000, 50000, 100000.
constexpr int cashScale(int cashUnits) {
    return cashUnits > 100000 ? 80 : cashUnits > 50000 ? 40 : cashUnits > 25000 ? 20 : cashUnits > 10000 ? 10 : cashUnits > 5000 ? 4
         : cashUnits > 2500 ? 2 : 1;
}
constexpr int skillScale(int skillNow) { return skillNow > 2500 ? 2 : 1; }   // 1 or 2
// Right axis label text for tick k (thousands of dollars): "$" + n + "k" where the exe's sign is the section sign.
//   k <= 4: n = 10 * s * k           (s = cashScale)
//   k >= 5: n = 50 * s * (k - 4)
constexpr int rightLabelThousands(int k, int s) { return k <= 4 ? 10 * s * k : 50 * s * (k - 4); }
// Left axis label: plain signed number. k <= 4: 100 * t * k ; k >= 5: 500 * t * (k - 4), with t = skillScale.
constexpr int leftLabelValue(int k, int t) { return k <= 4 ? 100 * t * k : 500 * t * (k - 4); }
// Event markers. Slot value: (code | arg) with code a multiple of 0x20 and arg in the low 5 bits.
// A marker is drawn at the month's x: a 2x2 dot at (x + 10, skill point y), a stem at x + 11 from the dot up to the label height, a second dot on top,
// and the text left aligned at (x + 14, 518 - labelHeight). labelHeight = (previous labelHeight % 450) + 10 so labels stagger upward by 10 px.
constexpr uint32_t kEventDotColour = 0x80000210u, kEventStemColour = 0x800003ffu, kEventTextColour = 0x80000210u;
constexpr int kEventLabelStep = 10, kEventLabelWrap = 450;
enum class EventArg : uint8_t { None, Number, TableName };
struct EventKind { int code; const char* prefix; EventArg arg; const char* suffix; };
constexpr EventKind kEvents[11] = {
    {0x20, "Hole ", EventArg::Number, " opened."},            // arg = hole number
    {0x40, "", EventArg::TableName, " built."},               // arg indexes the building name list at 0x4c26b0 (stride 20, "Pathway" first)
    {0x60, "Won match vs. ", EventArg::TableName, ""},        // arg indexes the opponent table at 0x58dd50 (stride 56)
    {0x80, "Hole ", EventArg::Number, " rated top 100."},
    {0xa0, "Hole ", EventArg::Number, " rated top 18!"},
    {0xc0, "", EventArg::TableName, " buys a home."},         // arg indexes the home owner name table at 0x55d738 (stride 37)
    {0xe0, "Gary Golf places ", EventArg::Number, " in tournament."},   // arg = finishing place
    {0x100, "", EventArg::TableName, " joins the board."},   // arg indexes the pro name list at 0x4c2c18 ("J.P.Bigdome", ...)
    {0x120, "Happy Ending.", EventArg::None, ""},
    {0x140, "Additional land purchased.", EventArg::None, ""},   // logged by Buy Land, arg = tract index
    {0x160, "Ivana donates a ", EventArg::TableName, ""}};   // arg = landmark, name built by the donation routine at 0x4074a0
constexpr Pt okPos{704, 551};
constexpr Rect okHit{704, 551, 44, 44};                   // exe test: 0x2c0..0x2eb by 0x227..0x252 inclusive; draw OkStates hover cut inside
}  // namespace histograph

// ---- 2. Professional accomplishments (F10) ---------------------------------------------------------------------------------------
namespace accomplishments {
constexpr const char* kSheetBoard = "bulletinboard&mantlewood.pcx";   // Interface/, full 800x600 backdrop copied to (0,0); cork variant exists
constexpr const char* kSheetBoardCork = "bulletinboard&mantlecork.pcx";   // loaded by another path (not this routine)
constexpr const char* kSheetParts = "TrophyParts_A.pcx";   // plaque strips and frame pieces; the non _A sheet has identical cuts
constexpr const char* kSheetTacs = "tacs&tees_A.pcx";      // polaroid frame, label strips, corner pieces
constexpr const char* kSnapshotFolder = "snapshots";       // run time files snapshots/accomp<class>.bmp, 200x160 crop
constexpr int kClasses = 22;
constexpr const char* kNames[kClasses] = {
    "1st Challenge hole", "1st Heroic hole", "1st skill upgrade", "1st Tournament", "1st Strategic hole", "First match victory",
    "First 9+ hole course", "1st Top 100 hole", "1st $500,000 Tournament", "1st Classic hole", "1st Top 18 hole",
    "First tournament victory (9+ holes)", "1st Grand Slam course (9+ holes)", "1st $1,000,000 Tournament", "First 18 hole course",
    "First tournament victory (18 hole)", "1st Grand Slam course (18 hole)", "Grand Slam Victory (Parkland)",
    "Grand Slam Victory (Desert)", "Grand Slam Victory (Tropical)", "Grand Slam Victory (Links)", "1st 100 star rating"};
// Plaque cuts on TrophyParts_A.pcx, 248x29, one per class: left column classes 0..10 at x 9, right column classes 11..21 at x 547.
constexpr Rect plaqueCut(int cls) { return {cls < 11 ? 9 : 547, 221 + 35 * (cls < 11 ? cls : cls - 11), 248, 29}; }
// Three alternative plaques (indices 29, 30, 31) at x 282, y 186 + 35 k, 248x30, used instead of classes 0, 1, 4 when the edition byte is below 2.
constexpr Rect altPlaqueCut(int k) { return {282, 186 + 35 * k, 248, 30}; }
constexpr int altIndexForClass(int cls) { return cls == 0 ? 0 : cls == 1 ? 1 : cls == 4 ? 2 : -1; }
// Other cuts on TrophyParts_A.pcx and their screen positions (cut == destination unless dst is given).
struct Piece { Rect cut; Pt dst; };
constexpr Piece kTowerBase{{283, 357, 248, 146}, {288, 351}};      // drawn after the plaques
constexpr Piece kTowerTop{{276, 292, 257, 47}, {284, 0}};          // dst y = 304 - 14 n  (n = earned count)
constexpr Piece kTowerCapA{{9, 116, 248, 30}, {288, 0}};           // drawn twice: dst y = 339 - 14 n and 335 - 14 n
constexpr Piece kTowerCapB{{9, 186, 248, 19}, {288, 0}};           // drawn once under the newest plaque: dst y = plaque y + 14
constexpr Piece kTowerFoot{{317, 490, 183, 52}, {313, 484}};
constexpr Piece kSignRight{{510, 13, 132, 162}, {519, 395}};
constexpr Piece kSignLeft{{649, 13, 130, 163}, {177, 395}};
constexpr int kTowerTopY(int n) { return 304 - 14 * n; }
constexpr int kTowerCapY0(int n) { return 339 - 14 * n; }
constexpr int kTowerCapY1(int n) { return 335 - 14 * n; }
constexpr int kPlaqueY0(int n) { return 349 - 14 * n; }            // newest earned plaque; the next one is 16 px lower (CapB sits at plaque y + 14), every older one after that 14 px lower
constexpr int kPlaqueX = 288;
// tacs&tees_A.pcx cuts.
constexpr Piece kFrame{{63, 251, 229, 209}, {0, 0}};               // polaroid frame, dst (x - 7, y - 173)
constexpr Piece kNoteCard{{58, 49, 207, 178}, {600, 350}};         // card behind the to do list
constexpr Piece kCornerLeft{{51, 482, 184, 80}, {51, 482}};
constexpr Piece kCornerRight{{732, 511, 53, 52}, {732, 511}};
constexpr Rect kCornerAlt{668, 511, 53, 52};                       // second corner piece, cut only
// Label strips for the to do list: cuts on tacs&tees_A.pcx per class (x, y, w, h), 22 entries.
constexpr Rect labelCut[kClasses] = {
    {400, 97, 135, 22},  {400, 133, 117, 21}, {400, 168, 125, 21}, {400, 207, 124, 18}, {400, 239, 133, 20}, {400, 273, 135, 21},
    {400, 310, 137, 20}, {400, 341, 129, 25}, {400, 381, 133, 32}, {400, 413, 110, 22}, {400, 448, 112, 24}, {400, 480, 177, 37},
    {400, 519, 160, 35}, {400, 556, 141, 31}, {610, 97, 129, 20}, {610, 130, 180, 36}, {610, 168, 161, 36}, {610, 202, 144, 40},
    {610, 239, 144, 33}, {610, 274, 142, 38}, {610, 307, 144, 35}, {610, 341, 155, 24}};
constexpr Rect altLabelCut[3] = {{610, 381, 132, 20}, {610, 415, 126, 20}, {610, 450, 109, 21}};   // for classes 0, 1, 4 in the low edition
// To do list (right side, classes not yet earned, first three in class order): label k at (616 + off / 2, 400 + off) where off starts at 0 and
// grows by (label height of the previous) - 4.
constexpr int kTodoCount = 3, kTodoX0 = 616, kTodoY0 = 400, kTodoStepAdjust = -4;
// Polaroids, one per earned class in ascending time order. k = 0, 1, 2 ...; even k left, odd k right.
constexpr int polaroidX(int k) { return 20 + ((k & 1) ? 515 : 0) + (35 * k) % 50; }
constexpr int polaroidY(int k) { return 443 - 14 * k; }           // caption baseline; frame at (x - 7, y - 173)
constexpr Pt framePos(int k) { return {polaroidX(k) - 7, polaroidY(k) - 173}; }
constexpr Rect photoDst(int k) { return {polaroidX(k), polaroidY(k) - 166, 200, 160}; }
constexpr Pt pinPos(int k) { return {polaroidX(k) + 100, polaroidY(k) - 179}; }   // sheet piece at 0x5a4558 (TacksandArrow_A), exact position, piece identity derived
constexpr uint32_t kCaptionColour = 0x80002108u;                  // left aligned at (polaroidX, polaroidY)
// Caption text: "<course name>  <day> <Month> <Year>" with day = ((tick & 1023) * 30 >> 10) + 1, Month from the eight names March ... October (tick >> 10 & 7),
// Year = 2001 + tick / 8192. Two spaces after the course name, one space after the day.
constexpr int kDayOfMonth(int tick) { return ((tick & 1023) * 30 >> 10) + 1; }
// Snapshot crop at record time: source origin = (clamp(sx - 100, 0, 600), clamp(sy - 100, 0, 440)) where (sx, sy) is the spot in screen pixels; size 200x160.
constexpr int kSnapW = 200, kSnapH = 160, kSnapMaxX = 600, kSnapMaxY = 0x1b8, kSnapBackX = 100, kSnapBackY = 100;
// Record lifecycle: record(class, worldX, worldY) stores the tick, the current site and the spot; 20 frames later (counter reaches 0x14) the screen opens.
constexpr int kOpenDelayFrames = 20;
constexpr int kF10Class = 99;                                      // F10 opens it with this class and the delay already at 20
constexpr const char* kSkillPrompt = "Add three skill points to your player...\n";   // shown afterwards when a skill upgrade flag is pending
// Trigger sites in the exe (class index: moment). Marked derived: the call sites pass the class, the condition is read from context only.
//  0 first Challenge hole, 1 first Heroic hole, 4 first Strategic hole, 9 first Classic hole: when a hole of that type is rated/opened
//  2 first skill upgrade; 5 first match victory; 6 first 9+ hole course; 14 first 18 hole course; 7 first Top 100 hole; 10 first Top 18 hole;
//  11/15 tournament win on 9+/18 hole course; 12/16 grand slam course; 17 + course type grand slam victory; 21 first 100 star rating.
// Classes 3, 8, 13 (first tournament, $500,000, $1,000,000) have no direct call in the code read; unknown.
}  // namespace accomplishments

// ---- 3. World map / property chooser (F6, shift+w) -----------------------------------------------------------------------------
namespace worldmap {
constexpr const char* kSheetBase = "WorldBase.pcx";       // Interface/, backdrop
constexpr const char* kSheetButton = "WorldButton.pcx";   // Interface/, button strips
constexpr const char* kSheetPins = "TacksandArrow_A.pcx"; // 20x24 pieces: x = 50 * variant, y = 50 * state (variant 0..3, state 0..3)
constexpr Field headline1{126, 19, Align::Center}, headline2{126, 33, Align::Center};   // "Where will you build" / "your golf course?"
constexpr const char* kHeadline1 = "Where will you build";
constexpr const char* kHeadline2 = "your golf course?";
constexpr Field cash{740, 18, Align::Center};             // money text; the text "Unlimited " + section sign replaces it in the unlimited-money mode
constexpr int kCards = 16;
// Card origins around the map (x, y), card slot index 0..15 (derived: matches the hover loop table at 0x4e3dc0).
constexpr Pt cardPos[kCards] = {{251, 13}, {302, 64}, {346, 115}, {384, 166}, {416, 220}, {437, 277}, {454, 337}, {465, 398},
                                {469, 458}, {463, 520}, {466, 13}, {509, 64}, {546, 115}, {580, 166}, {606, 220}, {623, 277}};
// Text inside a card, offsets from the card origin.
constexpr Pt kNameOff{0x60, 4};          // centred, black: property name
constexpr Pt kBonusOff{0x68, 0x10};      // centred, black: bonus text ("Scenic Cypress")
constexpr Pt kLine1Off{0x70, 0x1e};      // blue 0x80002108 centred: selected+available "Buy N acres of <flat|rolling|hilly>" ; otherwise "N acres: $P"
constexpr Pt kLine2Off{0x6a, 0x28};      // blue: "<inland|coastal|island> <parkland|desert|tropical|links> property"
constexpr Pt kLine3Off{0x68, 0x32};      // blue: "for only $P."  (hidden in unlimited-money mode)
constexpr Pt kPurchasedOff{0x70, 0x2d};  // blue: "Already Purchased." (selected purchased card)
constexpr Pt kArrowOff{0xa0, 10};        // piece from the pin sheet drawn beside the card, same state column as the map pin
// Legend row at the bottom: pin piece at (24,550), (124,550), (244,550) with labels left aligned at (44|144|264, 560), black.
constexpr Pt legendPin[3] = {{24, 550}, {124, 550}, {244, 550}};
constexpr Pt legendText[3] = {{0x2c, 0x230}, {0x90, 0x230}, {0x108, 0x230}};
constexpr const char* kLegend[3] = {"Available", "Insufficient funds", "Already purchased"};
// Button strips (cuts on WorldButton.pcx), exact; roles derived from the hit tests.
constexpr Rect kCutPanelA{250, 408, 76, 114}, kCutPanelB{724, 408, 76, 114}, kCutHoverLoad{550, 458, 76, 64}, kCutLoadIdle{400, 458, 76, 64},
               kCutHoverReset{550, 408, 76, 50}, kCutResetIdle{400, 408, 76, 50}, kCutCancel{732, 532, 68, 68};
constexpr Pt kButtonResetSave{724, 408}, kButtonLoad{724, 458}, kButtonCancelPos{732, 532};
// Pointer hit tests (distance from a centre, exe distance helper): Cancel (768,557) radius < 25 ; Reset / Save (760,476) radius < 25 ; Load (774,425) radius < 20.
constexpr Pt kCancelCentre{768, 557}, kSaveCentre{760, 476}, kLoadCentre{774, 425};
constexpr int kCancelRadius = 25, kSaveRadius = 25, kLoadRadius = 20;
constexpr const char* kButtonLabels[4] = {"Reset World", "Save Game", "Load Game", "Cancel"};   // "Reset World" before the game starts, "Save Game" after
constexpr int kCardPickRadius = 30;      // nearest card centre (origin + 25) or map pin within this distance becomes the hover choice
// Property record (stride 130 bytes at 0x4c1e90, 16 entries), offsets: +0 region name (24 bytes), +0x18 order byte then course name, +0x3a / +0x3c pin x / y on the
// world picture, +0x3e course type (0 parkland, 1 desert, 2 tropical, 3 links), +0x3f lie class (0 inland, 1 coastal, 2 island), +0x40 terrain (0 flat, 1 rolling, 2 hilly), +0x41 bonus text.
struct Site { const char* region; const char* course; int pinX, pinY; int type, lie, terrain; const char* bonus; };
constexpr Site kSites[16] = {
    {"Monterey", "Ocean's Edge", 53, 226, 0, 1, 1, "Scenic Cypress"},        {"San Diego", "Dolphin Coast", 53, 244, 0, 1, 0, "Dolphins"},
    {"Rocky Mtns.", "Jurassic Springs", 92, 226, 0, 0, 2, "Free Hotel"},    {"Las Vegas", "Ace in the Hole", 73, 241, 1, 0, 0, "Fun, Fun, Fun"},
    {"Phoenix", "Coyote Flats", 70, 262, 1, 0, 0, "Free Spa"},              {"Hawaii", "Flamingo Shores", 26, 327, 2, 1, 1, "Scenic Waterfall"},
    {"Oahu", "Island Palms", 23, 311, 2, 2, 0, "Japanese Garden"},          {"Nova Scotia", "Windy Point", 158, 250, 3, 1, 1, "Scenic Lighthouse"},
    {"Northeast", "Ravenwood Farms", 138, 254, 0, 0, 1, "Civil War Battlefield"}, {"Carolina", "Christmas Pines", 133, 272, 0, 0, 0, "Free Putting Green"},
    {"Ireland", "County Kincaide", 262, 232, 3, 0, 2, "Leprechauns"},       {"Scotland", "Harold's Keep", 266, 222, 3, 1, 0, "Free Castle"},
    {"Wales", "Thistle Runes", 272, 239, 3, 2, 1, "Stonehenge"},            {"Spain", "Sangria Bay", 275, 270, 1, 1, 1, "Scenic Vineyards"},
    {"Florida", "Ocean Grove", 125, 298, 2, 1, 0, "Free Pro Shop"},         {"Jamaica", "Scorpion Cove", 129, 342, 2, 0, 1, "Scenic Statues"}};
// Price by list slot (cheapest first), money units (money = units * 100): slot 0 = 50,000 ... slot 15 = 1,000,000.
constexpr int kSlotPriceUnits[16] = {500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000};
// Acres of the property in list slot s: (s + 4) * 10, then +20 (slot > 3) or +10 (slot <= 3) when the lie is inland, minus the same when island; 250 in the unlimited-money mode.
constexpr int acres(int slot, int lie) {
    int a = (slot + 4) * 10; int adj = slot > 3 ? 20 : 10;
    return lie == 0 ? a + adj : lie == 2 ? a - adj : a;
}
constexpr int kUnlimitedAcres = 250;
// List generation (exact flow, random source is the game's rng): slots 12..15 take one random site of each distinct course type (rejects a repeated type);
// slots 0..11 take random sites not yet listed, and slot 0 must be a parkland site. "Reset World" regenerates before the game starts.
constexpr int kTypeSlotsFirst = 12;
// Purchase effect: cash -= price; owned bit (1 << slot) set in a 32 bit mask; sound by course type: parkland 0x33 fixed, desert 0x78 + rnd3, tropical 0x6e + rnd3, links 0x73 + rnd3.
constexpr int kSoundBase[4] = {0x33, 0x78, 0x6e, 0x73};
constexpr const char* kTextNeedMoney1 = "You need more money before ";
constexpr const char* kTextNeedMoney2 = "you can purchase this property.";   // dark red 0x80006000, message box at (200, 200)
constexpr const char* kTextOff = "We're off to ";                          // + site name + "!" at (300, 285) white
constexpr const char* kTextWait = "... one moment please ...";             // white at (300, 300)
constexpr const char* kTextSavePrompt = "SAVE GAME: edit name then press Enter.";   // white, (384, 88) over a box at (120, 80, 528, 80)
constexpr const char* kTextSaveField = "Save File Name...";                  // edit field at (128, 102), 40 characters
constexpr const char* kTextSaved = "Game Saved.";
constexpr const char* kTextAlreadyBought = "Already Purchased.";
}  // namespace worldmap

// ---- 4. Buy Land: pricing and ownership ------------------------------------------------------------------------------------------
namespace buyland2 {
// Tile layout: the working map is 50 x 50 tiles stored x-major (index = x * 50 + y). Tracts are 3 x 3 blocks of 16 x 16 tiles covering tiles 1..48.
constexpr int kMapSide = 50, kTractSide = 16, kTractsAcross = 3;
constexpr int tractOriginX(int t) { return (t % 3) * kTractSide + 1; }    // map column of tract t (t = 0..8), left to right
constexpr int tractOriginY(int t) { return (t / 3) * kTractSide + 1; }    // map row, top to bottom
constexpr int kForSaleTile = 0x14;                                        // tile code of unowned land; the real terrain is kept in a shadow copy
// Price (money units; display = units * 100 through the money formatter):
//   f = 5 + sum over for-sale tiles in the tract of (rnd(3) == 0 ? 2^n : 0), n = number of land purchases made so far in this game
//   price = floor(f * 20 / 100) * 10
// The random draws happen once, the first time the screen is drawn after it opens; the prices are cached for the rest of that visit.
constexpr int priceUnits(int f) { return (f * 20 / 100) * 10; }
constexpr int kBaseF = 5, kRndDivisor = 3;
// Acres shown for a tract = for-sale tile count / 10 (integer division).
constexpr int acresShown(int forSaleTiles) { return forSaleTiles / 10; }
// Card text: "Buy tract #N" (N = 1..9), then "<acres> acres of <a>, <b>, and <c>" naming the three most numerous terrain types in the tract (terrain type
// names come from the terrain table at 0x578350, stride 48), then "Price: $" + money right aligned at (cardX + 165, ...). A tract with no tiles left shows
// "Already purchased." in 0x80002108 at (cardX + 4, cardY + 2).
constexpr int kCardColumnX[3] = {78, 334, 600};                            // text origin before the +4 offset; card k = column k / 3, row k % 3
constexpr int kCardRowY0 = 62, kCardRowPitch = 68;
constexpr int kCardPickX[3] = {15, 272, 538};                              // highlight / hit left edges
constexpr int kCardPickY[3] = {54, 122, 190};
constexpr int kTextWrapWidth = 165;
// Pointer hit area for the cards: x 15..774, y 54..257; OK area 662..725 by 533..596 (64x64).
constexpr const char* kPriceLabel = "Price: ";
constexpr const char* kLabelBuy = "Buy tract #";
constexpr const char* kLabelSold = "Already purchased.";
constexpr const char* kLabelNone = " I don't think I'll buy any land.\n";
// Effects of a confirmed purchase (exact): cash -= price; the year ledger short at 0x58421e + 20 * (year % 100) -= price;
// every for-sale tile of the tract is restored to its real terrain; event 0x140 | tract is logged (histograph text "Additional land purchased."); the caller then
// increments the purchase counter n and refreshes the course. The counter also feeds other rules (see the doc, section 4).
// Affordability: price <= 0, price <= cash, the unlimited-money flag, or the course tier below 4 all allow the purchase; otherwise the dialog
// "This change costs $X. You have only $Y." (message box at (200, 200), dark red 0x80006000) appears and the player stays on the screen.
constexpr const char* kTextCosts = "This change costs ";
constexpr const char* kTextOnly = ". You have only ";
}  // namespace buyland2

// ---- 5. Course overview text positions (routing, aura, value) ---------------------------------------------------------------------
namespace overview2 {
constexpr int kModeRouting = -1, kModeAura = 1, kModeValue = 2;           // mode word values in the exe
// Course name above the map: centred (400, 48), shifted down 10 px when a hole is selected. Text colour black.
constexpr Field courseName{400, 48, Align::Center};
// Aura mode.
constexpr const char* kAuraHelp = "Course AURA indicates where on your course players have made mostly happy comments and where they have made unhappy comments.";
constexpr Rect auraHelpBox{250, 110, 300, 0};                              // wrapped text block: left 250, top 110, width 300 (height by wrap)
constexpr Field auraHeading{400, 85, Align::Center};                       // "AURA"
constexpr Field auraLower{400, 188, Align::Center};                        // "COURSE AURA" (the fixed value is 0xbc)
constexpr Field auraUnhappy{136, 206, Align::Left}, auraHappy{624, 206, Align::Left};   // "Unhappy" left, "Happy" right
// Value mode.
constexpr Field valueLower{400, 188, Align::Center};                       // "HOME SITE VALUE"
constexpr Field valueLow{142, 206, Align::Left}, valueHigh{631, 206, Align::Left};     // "Low" left, "High" right
constexpr int kValueLeftX = 51, kValueRightX = 494;
constexpr int kValueRowY[5] = {86, 104, 121, 138, 156};
constexpr const char* kValueLeft[5] = {"Things which INCREASE home value:", "Close to water and trees.", "Close to a fun golf hole.",
                                       "Close to a top 100 or top 18 hole.", "Building a Marina"};
constexpr const char* kValueRight[5] = {"Things which DECREASE home value:", "Close to an unfun hole.", "Close to another building.",
                                        "Too close to green, fairway, or OB.", "Far away from the golf course."};
constexpr uint32_t kTextColour = 0x80000000u;
// Routing mode list. Hole i = 1..18: column = i < 10 ? left : right, row r = (i - 1) % 9.
constexpr int kRowY0 = 104, kRowPitch = 17;
constexpr int rowY(int hole) { return kRowY0 + kRowPitch * ((hole - 1) % 9); }
constexpr int colShift(int hole) { return hole < 10 ? 0 : 0x1f2; }         // 498 px shift for the right list
constexpr int holeTextX(int hole) { return 71 + colShift(hole) + (hole < 10 ? 0 : 0); }   // number centred at (71 | 569, rowY)
constexpr int parX(int hole) { return 132 + colShift(hole); }
constexpr int ydsX(int hole) { return 182 + colShift(hole); }
constexpr int timeX(int hole) { return 239 + colShift(hole); }
// Selected hole highlight: outer green rectangle, inner white rectangle, both filled.
constexpr Rect selOuter(int hole) { return {colShift(hole) + 0x20, rowY(hole) - 3, 0xee, 0x11}; }
constexpr Rect selInner(int hole) { return {colShift(hole) + 0x21, rowY(hole) - 2, 0xec, 0x0f}; }
constexpr uint32_t kSelOuterColour = 0x800023e8u, kSelInnerColour = 0x80007fffu;
constexpr uint32_t kRowColourFilled = 0x80000000u, kRowColourEmpty = 0x80006318u;   // empty slot text is grey
// Row text: hole number (the list index), PAR = par, YDS = yards, Time = (total ticks / rounds) / 40 followed by "m"; Time stays blank until a round is recorded.
constexpr const char* kTimeSuffix = "m";
constexpr int kTicksPerMinute = 40;
}  // namespace overview2

}  // namespace sg::ui_screens2

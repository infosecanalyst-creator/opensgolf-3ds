// SimGolf native port: layout tables for the Elevation, Buildings, Amenities and Employee dock panels.
//
// Every number here was read from the publisher exe's panel code (docs/DECODE_PANELS.md explains how and lists what is a
// placeholder). All coordinates are 800x600 screen pixels. Sprite rectangles are rectangles on the panel's own sheet in the
// game's Interface folder (sheet name without extension). A panel background is cut from its sheet at (x, y) and drawn at that
// same (x, y); the "_A" sheet with the same name holds its alpha mask. Colour key of the sheets: magenta, but the layout sheets
// use (248,0,248); test with r>=240, b>=240, g<=8.
//
// Hit tests use the exe's own distance metric (FUN_00467170): with a = |dx*xs|, b = |dy*ys|,
//   d = (b < a) ? (b + 2a) / 2 : (a + 2b) / 2,   and a point hits when d < r.
//
// Nothing in this header copies disc art. Money is in the exe's stored units of $100 (display = units * 100).
#pragma once

#include <cstdint>

namespace sg {
namespace panels {

struct Rect {
    int x, y, w, h;
};

// A hit region: d(xs * (px - cx), ys * (py - cy)) < r, d as described above.
struct Hit {
    int cx, cy, r;
    int xs, ys;
};

constexpr int absi(int v) { return v < 0 ? -v : v; }
constexpr int metric(int a, int b) {
    a = absi(a);
    b = absi(b);
    return b < a ? (b + 2 * a) / 2 : (a + 2 * b) / 2;
}
constexpr bool hitTest(const Hit& h, int px, int py) { return metric((px - h.cx) * h.xs, (py - h.cy) * h.ys) < h.r; }

// Dock modes of the exe (DAT_00567afc) and the sub-panel flags that pick one of the panels below.
//   mode 0 Build Course: Terrain panel, or Amenities when DAT_0055e924 != 0
//   mode 1 Add Buildings: Buildings panel, or Elevation when DAT_0055e928 != 0
//   mode 2 Golfers (Member panel), mode 4 Player (JoeCool panel); both show the Employee panel while DAT_00561254 != 0
enum DockMode { kModeBuildCourse = 0, kModeAddBuildings = 1, kModeGolfers = 2, kModePlayer = 4 };

// Theme index used by the exe (DAT_005a34e0).
enum Theme { kParkland = 0, kDesert = 1, kTropical = 2, kLinks = 3 };

// ---------------------------------------------------------------------------------------------------------------------
// Elevation panel (sheet "ElevationPanel", alpha "ElevationPanel_A"). Reached from the Buildings panel (its -2 button).
// ---------------------------------------------------------------------------------------------------------------------
namespace elevation {

inline constexpr const char* kSheet = "ElevationPanel";
inline constexpr Rect kBackground{215, 482, 585, 118};  // exact; drawn at (215,482)

enum Tool { kVertex = 0, kSquare = 1, kArea = 2, kAnalyze = 3 };

// Sprite states per tool button: 0 hover (drawn at the button position while the pointer is over the hit region),
// 1 selected (drawn while this is the current tool, DAT_00542f20), 2 never drawn by the panel code (spare/disabled look).
struct ToolButton {
    const char* tip;  // exact tooltip text
    Hit hit;
    int x, y;         // top-left where the sprites are drawn
    Rect state[3];
};

inline constexpr ToolButton kTools[4] = {
    {"Raise/lower vertex", {352, 547, 25, 1, 1}, 312, 501, {{0, 0, 88, 99}, {100, 0, 88, 99}, {200, 0, 88, 99}}},
    {"Raise/lower square", {467, 547, 25, 1, 1}, 429, 501, {{0, 100, 88, 99}, {100, 100, 88, 99}, {200, 100, 88, 99}}},
    {"Raise/lower area", {587, 547, 25, 1, 1}, 546, 501, {{0, 200, 88, 99}, {100, 200, 88, 99}, {200, 200, 88, 99}}},
    {"Analyze Golf Shot", {715, 547, 16, 1, 1}, 688, 501, {{400, 0, 72, 99}, {500, 0, 72, 99}, {600, 0, 72, 99}}},
};

// Small round buttons. The exe evaluates Buildings, then tools 0..3, then Undo, and a later match overwrites an earlier one,
// so the priority is Undo > tool 3 > tool 2 > tool 1 > tool 0 > Buildings.
struct RoundButton {
    const char* tip;
    Hit hit;
    int x, y;
    Rect hover;
};
inline constexpr RoundButton kToBuildings{"Buildings", {270, 499, 16, 1, 1}, 254, 483, {0, 400, 41, 44}};  // exact

// Undo button: hover sprite, then an "armed" sprite drawn while undo mode is on (flag 0x8000000 of DAT_0059e7b8); the third
// cut (100,450) is never drawn by the panel code.
struct UndoButton {
    const char* tip;
    Hit hit;
    int x, y;
    Rect hover, armed, spare;
};
inline constexpr UndoButton kUndo{
    "Undo", {267, 561, 12, 1, 1}, 253, 546, {0, 450, 34, 40}, {50, 450, 34, 40}, {100, 450, 34, 40}};

// Edit rules read from the exe (heights are bytes at 0x5a4998 + x*51 + y, range 3..13). The exe applies the edits from
// the KEYBOARD ('-' lowers, '=' raises; ASCII 45 and 61) at the cursor vertex (DAT_00543d08, DAT_00543d0c). Which mouse
// button the original used is not decoded (placeholder).
constexpr int kMinHeight = 3;
constexpr int kMaxHeight = 13;
// Lower clamps its upper bound to 13 only when the byte at 0x571ff6 + 46 * DAT_0059bf90 equals 2, otherwise to 10 (meaning of
// that byte is not decoded; treat as 13).
constexpr int kLowerAltMax = 10;
// Area tool: after the vertex edit, a 5x5 neighbourhood (rows x-2..x+2, columns y-2..y+2, row-major below) is nudged by 1:
// raise: cell < centre - k  => cell + 1;  lower: cell > centre + k  => cell - 1. (The odd 3 at row 3, column 4 is in the exe.)
inline constexpr int kAreaKernel[5][5] = {
    {6, 4, 3, 4, 6}, {4, 2, 1, 2, 4}, {3, 1, 0, 1, 3}, {4, 2, 1, 2, 3}, {6, 4, 3, 4, 6},
};
// Square tool: the 2x2 vertex block x..x+1, y-1..y. Raise adds 1 to the cells equal to the block minimum (if min < 13);
// lower subtracts 1 from the cells equal to the block maximum (if max > 3). The single vertex edit is skipped for squares.

// No money is charged anywhere in the elevation edit path (placeholder: free).
constexpr int kElevationCostUnits = 0;

}  // namespace elevation

// ---------------------------------------------------------------------------------------------------------------------
// Buildings panel (sheet "BuildingPanel", alpha "BuildingPanel_A"). Dock mode 1. Nine building lots, ids 6..14.
// ---------------------------------------------------------------------------------------------------------------------
namespace buildings {

inline constexpr const char* kSheet = "BuildingPanel";
inline constexpr Rect kBackground{216, 482, 584, 118};  // exact; drawn at (216,482)

// Per-theme sheet holding the lot pictures (index = Theme).
inline constexpr const char* kLayoutSheet[4] = {"Parklands Layout", "Desert Layout", "Trop Layout", "Links Layout"};

struct Lot {
    int id;                   // building table id (DAT_004c2854 after a click); lot index = id - 6
    const char* name;         // exact, from the building table at 0x4c26b0
    int footprint;            // tiles per side (square) of the level 0 building; level L builds footprint + L
    int costUnits;            // units of $100, level 0 (exact)
    const char* effect;       // exact hover text (third line of the info box)
    Hit hit;                  // hit region (ys = 2, so an ellipse twice as wide as tall)
    int padX, padY;           // where the isometric pad sprites are drawn
    Rect pad[3];              // 0 hover, 1 selected (bright green), 2 blue (never drawn by the panel code)
    int iconX, iconY;         // where the lot picture is drawn (one pixel higher while hovered and buildable)
    int iconCol;              // column on the layout sheet; the picture is the 75x100 cell at (75*col, kIconRowY[..])
};

// Pad cut rows: lots 0..4 use the first row of pads, lots 5..8 the second.
#define SG_PADS_ROW0 {{0, 0, 74, 64}, {100, 0, 77, 64}, {200, 0, 77, 64}}
#define SG_PADS_ROW1 {{0, 100, 77, 58}, {100, 100, 77, 58}, {200, 100, 77, 58}}

inline constexpr Lot kLots[9] = {
    {6, "Putting Green", 3, 100, "Helps imaginative golfers", {383, 531, 40, 1, 2}, 343, 501, SG_PADS_ROW0, 344, 453, 0},
    {7, "Snack Bar", 2, 150, "Feeds hungry golfers", {463, 531, 40, 1, 2}, 423, 501, SG_PADS_ROW0, 424, 453, 1},
    {8, "Pro Shop", 2, 200, "Improves accurate golfers", {543, 531, 40, 1, 2}, 503, 501, SG_PADS_ROW0, 504, 453, 2},
    {9, "Swim Club", 3, 300, "Golfers become members", {623, 531, 40, 1, 2}, 583, 501, SG_PADS_ROW0, 584, 453, 3},
    {10, "Driving Range", 5, 250, "Helps long hitters", {703, 531, 40, 1, 2}, 663, 501, SG_PADS_ROW0, 664, 453, 4},
    {11, "Cart Garage", 2, 400, "Golfers play faster", {423, 571, 40, 1, 2}, 384, 542, SG_PADS_ROW1, 384, 493, 5},
    {12, "Marina", 2, 1000, "Increases property values", {503, 571, 40, 1, 2}, 464, 542, SG_PADS_ROW1, 464, 493, 6},
    {13, "Resort Hotel", 4, 2500, "Golfers stay happier", {583, 571, 40, 1, 2}, 544, 542, SG_PADS_ROW1, 544, 493, 7},
    {14, "Airstrip", 6, 5000, "Higher greens fees", {663, 571, 40, 1, 2}, 624, 542, SG_PADS_ROW1, 624, 493, 8},
};
#undef SG_PADS_ROW0
#undef SG_PADS_ROW1

// The Clubhouse (id 15, footprint 4, cost 200 units) is in the building table but in no panel; its picture is the tenth
// column (col 9) of the layout sheet.
inline constexpr int kClubhouseId = 15;
inline constexpr int kClubhouseFootprint = 4;
inline constexpr int kClubhouseCostUnits = 200;

// Lot picture rows on the layout sheet, indexed by (2 * upgraded + buildable): the exe loads the four cuts of a column in
// the order y=0, y=200, y=100, y=300 into consecutive objects, and selects object 4*lot + 2*upgraded + buildable.
//   [0] grey, level 0   [1] colour, level 0   [2] grey, upgraded   [3] colour, upgraded
inline constexpr int kIconRowY[4] = {0, 200, 100, 300};
constexpr int kIconW = 75;
constexpr int kIconH = 100;

// Level of a lot (DAT_005a8c38[id], 0, 1 or 2) drives price and picture:
//   price units = costUnits * (level + 2) / 2   (x1, x1.5, x2); the info box shows only x1 or x1.5 (level != 0)
//   level >= 2 refuses with "You cannot upgrade this building any further."
//   the Snack Bar (id 7) is exempt from the "Upgraded " label and the x1.5 display price
// "upgraded" for the picture is level != 0. Buildable (colour picture and pad drawn) when
//   id < unlockCounter - level && level < 2 && (level == 0 || nextHoleNumber > 10)
// unlockCounter = DAT_005a6364 (6 in a normal new game, 17 in sandbox), nextHoleNumber = DAT_005685f0.
constexpr int kMaxLevel = 2;
constexpr int kNewGameUnlockCounter = 6;
constexpr int kSandboxUnlockCounter = 17;

// Level >= 1 click is refused with this text unless the course has more than 9 holes (exact strings).
inline constexpr const char* kMsgNoUpgradeYet = "Sorry, upgraded buildings are not available until you build beyond 9 holes.";
inline constexpr const char* kMsgMaxLevel = "You cannot upgrade this building any further.";
inline constexpr const char* kMsgLocked = "This building is will become available as you build additional holes.";

// Round buttons: switch to the Elevation panel, and Undo. Priority: the Elevation button, then Undo, then the lots.
inline constexpr Hit kToElevationHit{236, 526, 16, 1, 1};
inline constexpr Rect kToElevationHover{150, 480, 34, 34};  // drawn at (220, 510)
constexpr int kToElevationHoverX = 220, kToElevationHoverY = 510;
inline constexpr Hit kUndoHit{277, 567, 12, 1, 1};
inline constexpr Rect kUndoHover{40, 550, 34, 40};  // drawn at (261, 553)
inline constexpr Rect kUndoArmed{80, 550, 34, 40};  // drawn at (261, 553) while undo mode is on
inline constexpr Rect kUndoSpare{120, 550, 34, 40};  // never drawn by the panel code
constexpr int kUndoX = 261, kUndoY = 553;

// Info box shown after the pointer rests on a lot for more than 10 frames: a framed 160x100 box at
// (clamp((lot%5)*80 + (lot > 4 ? 431 : 391), 80, 720) - 80, 460), filled translucent, with three centred lines at y = 462
// (optional "Upgraded " + name), 472 ("Cost: " + dollars) and 482 (effect). Frame art: see kInfoFrame below.
constexpr int kInfoBoxW = 160, kInfoBoxH = 100, kInfoBoxY = 460;

}  // namespace buildings

// Pop-up frame used by the Buildings info box (and other popups): sheet "Pop_UpOk" (alpha "Pop_UpOk_A"), three theme rows.
// Row offset by theme: Parkland 0, Links 0, Tropical 100, Desert 200. Pieces are 3x3 with 1 pixel gutters; edges tile every 16
// pixels, the right column and bottom row pieces are 26 wide or tall.
namespace frame {
inline constexpr const char* kSheet = "Pop_UpOk";
constexpr int rowY(int theme) { return theme == kDesert ? 200 : theme == kTropical ? 100 : 0; }
// Offsets from the row origin (add rowY(theme) to y): top-left, top, top-right, left, right, bottom-left, bottom, bottom-right.
inline constexpr Rect kPiece[8] = {{0, 0, 16, 16},  {17, 0, 16, 16},  {34, 0, 26, 16},  {0, 17, 16, 16},
                                   {34, 17, 26, 16}, {0, 34, 16, 26}, {17, 34, 16, 26}, {34, 34, 26, 26}};
}  // namespace frame

// ---------------------------------------------------------------------------------------------------------------------
// Amenities panel (sheet "AmenitiesPanel", alpha "AmenitiesPanel_A"). Dock mode 0 with DAT_0055e924 != 0.
// ---------------------------------------------------------------------------------------------------------------------
namespace amenities {

inline constexpr const char* kSheet = "AmenitiesPanel";
inline constexpr Rect kBackground{214, 482, 586, 118};  // exact; drawn at (214,482)

// Slot indices are the exe's hit-test results. Sprite states per slot: 0 hover, 1 selected (current tool), 2 disabled look
// (drawn over the Home Site slot when no home sites are left, otherwise unused).
struct Slot {
    const char* tip;   // exact tooltip heading
    int toolId;        // value written to DAT_004c2854 (building table id), -1 = none/unavailable
    Hit hit;
    int x, y;          // top-left of the sprites
    Rect state[3];
};

inline constexpr Slot kSlots[10] = {
    {"Ballwasher", 3, {302, 563, 20, 1, 1}, 274, 536, {{0, 0, 60, 56}, {100, 0, 60, 56}, {200, 0, 60, 56}}},
    {"Pathway", 0, {333, 526, 20, 1, 1}, 302, 504, {{0, 100, 64, 50}, {100, 100, 64, 50}, {200, 100, 64, 50}}},
    {"Building Lot", 5, {363, 563, 20, 1, 1}, 334, 538, {{0, 200, 62, 50}, {100, 200, 62, 50}, {200, 200, 62, 50}}},  // Home Site
    {"Benches", 1, {470, 575, 18, 1, 1}, 449, 553, {{300, 0, 51, 47}, {400, 0, 51, 47}, {500, 0, 51, 47}}},
    {"Landmarks", 4, {591, 575, 18, 1, 1}, 571, 553, {{300, 100, 51, 45}, {400, 100, 51, 45}, {500, 100, 51, 45}}},
    {"Flower Bed", 2, {652, 575, 18, 1, 1}, 632, 553, {{300, 150, 51, 47}, {400, 150, 51, 47}, {500, 150, 51, 47}}},
    {"Scenic Trees", 16, {714, 575, 18, 1, 1}, 693, 553, {{300, 200, 51, 45}, {400, 200, 51, 45}, {500, 200, 51, 45}}},
    {"Undo", -1, {248, 567, 12, 1, 1}, 234, 553, {{0, 450, 36, 36}, {50, 450, 36, 36}, {100, 450, 36, 36}}},
    {"", -1, {0, 0, 0, 1, 1}, 762, 545, {{0, 500, 38, 36}, {50, 500, 38, 36}, {100, 500, 38, 36}}},  // art only, never hit-tested
    {"Scenic Bridge", 19, {531, 580, 12, 1, 1}, 510, 553, {{300, 50, 51, 45}, {400, 50, 51, 45}, {500, 50, 51, 45}}},
};
// The exe evaluates the back button, then slots 0..6, then slot 7 (Undo), then slot 9 (Scenic Bridge); a later match
// overwrites an earlier one, so the priority is 9 > 7 > 6 > 5 > ... > 0 > back.

// "Course Terrain" back button (returns to the Terrain panel: DAT_0055e924 = 0, DAT_00567afc = 0).
inline constexpr Hit kBackHit{269, 500, 16, 1, 1};
inline constexpr Rect kBackHover{0, 400, 34, 34};  // drawn at (254, 483)
constexpr int kBackX = 254, kBackY = 483;
inline constexpr const char* kBackTip = "Course Terrain";

// The Home Site slot is enabled only while DAT_0056d1b0 >= 1 (qualifying course sites minus Home Sites already placed).
// Variant pickers shown above the panel once the tool is selected: number of entries, first sprite id of the strip.
struct VariantPicker {
    int toolId;
    int count;
    int firstSpriteId;  // exe sprite-object ids; the art is in the per-theme building sprite sets (not mapped here)
};
inline constexpr VariantPicker kVariants[5] = {
    {1, 5, 0x208},   // Benches: 5 designs
    {4, 14, 0x168},  // Landmarks: 14 designs, only those whose bit is set in DAT_00543cfc (0x3fff = all, sandbox)
    {19, 8, 0x226},  // Scenic Bridge: 8 designs
    {2, 15, 0x1a3},  // Flower Bed: 5 designs times 3
    {16, 7, 0x12f},  // Scenic Trees: 7 designs
};
// Strip drawn behind the entries (cuts on the AmenitiesPanel sheet, drawn at y = 477): a left cap (advance 68), then
// middle pieces (advance 47) for count/2 pieces in all, then an end cap that depends on parity of count. The strip starts at
// x0 = anchor - (47 * count) / 4 + 544, where anchor is the first argument of the exe's picker call (-84 benches, 0
// landmarks, -16 bridge, 13 flower beds, 100 trees). Entry i has its centre at (x0 + 44 + (47 * i) / 2, 500 + (i odd ? 27 : 0)),
// its badge is drawn at (centre.x - 22, centre.y - 20), the entry hits when metric(px - cx, 2 * (py - cy)) < 20.
// (The flower bed strip is drawn three times as long: its entry count is multiplied by 3 before layout.)
inline constexpr Rect kStripLeft{380, 250, 68, 79};
inline constexpr Rect kStripMiddle{450, 250, 47, 79};
inline constexpr Rect kStripEndEven{600, 250, 67, 79};
inline constexpr Rect kStripEndOdd{500, 250, 67, 79};
constexpr int kStripY = 477;
// Entry badge states (47x41 cuts, y = 400): hovered, selected (variant == DAT_005a9f60), idle. The second row (y = 350, 47x40)
// is used only for an empty strip. Badge x for the four columns: 300 hover, 400 selected, 500 spare, 600 idle.
inline constexpr Rect kBadgeHover{300, 400, 47, 41};
inline constexpr Rect kBadgeSelected{400, 400, 47, 41};
inline constexpr Rect kBadgeSpare{500, 400, 47, 41};
inline constexpr Rect kBadgeIdle{600, 400, 47, 41};

// Landmark effect names by design number & 3 (exact): 0 Happy Golfers, 1 No Dandelions, 2 Skill Upgrade, 3 Happy Endings.
inline constexpr const char* kLandmarkEffect[4] = {"Happy Golfers", "No Dandelions", "Skill Upgrade", "Happy Endings"};
// Landmark price units = 250 + 50 * design (exact, from the placement code); it is 0 ("FREE!") when that design's bit is
// already set in DAT_00822c70 (a landmark you already own, for example a property bonus).
constexpr int landmarkPriceUnits(int design) { return 250 + 50 * design; }

// Building table rows used by this panel (id, name, footprint, level 0 price units, exact).
struct Item {
    int id;
    const char* name;
    int footprint;
    int costUnits;
};
inline constexpr Item kItems[8] = {
    {0, "Pathway", 1, 1},      {1, "Benches", 1, 2},     {2, "Flower Bed", 1, 5},    {3, "Ball Washer", 1, 50},
    {4, "Landmark", 1, 15},    {5, "Home Site", 2, 10},  {16, "Willow Tree", 1, 25}, {19, "Scenic Bridge", 1, 100},
};
// The tooltip shows the table price for Pathway, Benches, Flower Bed, Ball Washer, Home Site and Scenic Bridge. Landmarks show
// a price per design, trees show a byte at 0x578673 (probably the price of the selected design; Willow Tree table value 25).

}  // namespace amenities

// ---------------------------------------------------------------------------------------------------------------------
// Employee panel (sheet "EmployeePanel", alpha "EmployeePanel_A"). An overlay of dock modes 2 and 4 (DAT_00561254 != 0).
// ---------------------------------------------------------------------------------------------------------------------
namespace employees {

inline constexpr const char* kSheet = "EmployeePanel";
inline constexpr Rect kBackground{215, 474, 585, 126};  // exact; drawn at (215,474)

// Slot indices are the exe's hit-test results (0..15). Slots are tested in order and the first match wins; a slot match beats
// the two tab buttons below.
//  0 and 1: Hire employees / View employees (same place; slot 1 is never reached by the hit test)
//  2, 3: scroll left / right by one column (2 employees)
//  4 Move this employee, 5 Fire this employee, 6 Rename this employee (7 is a duplicate of 5 and never reached)
//  8..15: the eight visible employee portraits, column-major (slot = 8 + 2 * column + row)
// Sprite states: hover / pressed / disabled / idle depend on the slot, see StateRole below.
enum StateRole { kRoleHover = 0, kRolePressedOrArmed = 1, kRoleDisabled = 2, kRoleIdle = 3 };

struct Slot {
    const char* tip;
    Hit hit;
    int x, y;          // where the sprites are drawn
    Rect state[4];     // indexed by the roles above (unused entries are zero)
};

inline constexpr Slot kSlots[16] = {
    // Hire: hover (0,500), pressed (50,500) while a hire is under way, disabled/full (100,500) with 20 employees, idle is
    // slot 1's (50,550) drawn permanently at the same place.
    {"Hire employees", {270, 558, 16, 1, 1}, 254, 542, {{0, 500, 42, 42}, {50, 500, 42, 42}, {100, 500, 42, 42}, {50, 550, 42, 42}}},
    {"View employees", {270, 558, 16, 1, 1}, 254, 542, {{0, 550, 42, 42}, {50, 550, 42, 42}, {100, 550, 42, 42}, {50, 550, 42, 42}}},
    // Scroll arrows: hover (0,y); the enabled look (100,y) is drawn only while scrolling that way is possible.
    {"", {321, 549, 30, 4, 1}, 313, 519, {{0, 200, 16, 62}, {50, 200, 16, 62}, {100, 200, 16, 62}, {100, 200, 16, 62}}},
    {"", {604, 549, 30, 4, 1}, 596, 519, {{0, 300, 16, 62}, {50, 300, 16, 62}, {100, 300, 16, 62}, {100, 300, 16, 62}}},
    // Move / Fire / Rename: hover (x=400), armed (x=500, move mode only), disabled (x=600, no employee selected), idle (x=700).
    {"Move this employee", {647, 513, 16, 1, 1}, 631, 497, {{400, 0, 52, 48}, {500, 0, 52, 48}, {600, 0, 52, 48}, {700, 0, 52, 48}}},
    {"Fire this employee", {699, 510, 16, 1, 1}, 683, 494, {{400, 50, 52, 48}, {500, 50, 52, 48}, {600, 50, 52, 48}, {700, 50, 52, 48}}},
    {"Rename this employee", {756, 513, 16, 1, 1}, 740, 497, {{400, 100, 52, 48}, {500, 100, 52, 48}, {600, 100, 52, 48}, {700, 100, 52, 48}}},
    {"", {699, 510, 16, 1, 1}, 683, 494, {{400, 150, 52, 48}, {500, 150, 52, 48}, {600, 150, 52, 48}, {700, 150, 52, 48}}},
    // Portraits: hover (0, 0 or 50, 63x43), selected (100, same y); the third cut (200, y) is never drawn. Row 0 uses y = 0,
    // row 1 uses y = 50. The employee's own animated sprite is drawn at (x + 30, y + 24).
    {"", {368, 523, 16, 1, 1}, 336, 507, {{0, 0, 63, 43}, {100, 0, 63, 43}, {200, 0, 63, 43}, {0, 0, 0, 0}}},
    {"", {368, 566, 16, 1, 1}, 336, 550, {{0, 50, 63, 43}, {100, 50, 63, 43}, {200, 50, 63, 43}, {0, 0, 0, 0}}},
    {"", {431, 523, 16, 1, 1}, 399, 507, {{0, 0, 63, 43}, {100, 0, 63, 43}, {200, 0, 63, 43}, {0, 0, 0, 0}}},
    {"", {431, 566, 16, 1, 1}, 399, 550, {{0, 50, 63, 43}, {100, 50, 63, 43}, {200, 50, 63, 43}, {0, 0, 0, 0}}},
    {"", {494, 523, 16, 1, 1}, 462, 507, {{0, 0, 63, 43}, {100, 0, 63, 43}, {200, 0, 63, 43}, {0, 0, 0, 0}}},
    {"", {494, 566, 16, 1, 1}, 462, 550, {{0, 50, 63, 43}, {100, 50, 63, 43}, {200, 50, 63, 43}, {0, 0, 0, 0}}},
    {"", {557, 523, 16, 1, 1}, 525, 507, {{0, 0, 63, 43}, {100, 0, 63, 43}, {200, 0, 63, 43}, {0, 0, 0, 0}}},
    {"", {557, 566, 16, 1, 1}, 525, 550, {{0, 50, 63, 43}, {100, 50, 63, 43}, {200, 50, 63, 43}, {0, 0, 0, 0}}},
};

// Tab buttons that leave the panel. Hover sprite only.
struct Tab {
    const char* tip;
    Hit hit;
    int x, y;
    Rect hover;
    int targetMode;  // DockMode entered (the overlay flag is cleared)
};
inline constexpr Tab kToGolfers{"Golfers", {286, 492, 16, 1, 1}, 270, 475, {0, 400, 33, 33}, kModeGolfers};
inline constexpr Tab kToPlayer{"Gary Golf", {256, 510, 16, 1, 1}, 240, 494, {0, 450, 33, 33}, kModePlayer};  // tooltip is the player's name

constexpr int kMaxEmployees = 20;  // hiring is refused (error sound 0x18) at 20
constexpr int kVisibleSlots = 8;   // portraits per page; scroll step is 2 (one column)
constexpr int kFireCostUnits = 25; // $2,500, charged at the Fire click (exact)
constexpr int kHireFeeUnits = 0;   // no hiring fee was found (placeholder)
inline constexpr const char* kRenamePrompt = "Rename Employee...";

// Employee kinds, in the hire dialog order. Record type byte = -2 - kind. Each kind has a regular and a skilled variant
// (skilled shows the second name and wage, needs 6 or more holes).
struct Kind {
    const char* name;
    const char* skilledName;
    const char* blurb;          // line shown in the hire dialog
    int wageUnits[2];           // units of $100, shown as "per week" in the hire dialog; [0] regular, [1] skilled
    const char* statLabel[2];   // counter shown in the roster for the selected employee
};
inline constexpr Kind kKinds[4] = {
    {"Club Pro", "Celebrity", "Greeters...", {3, 7}, {"Players greeted:", "Players cheered:"}},
    {"Ranger", "Marshall", "Speed up play...", {2, 3}, {"Players rushed:", "Slackers intimidated:"}},
    {"Groundskeeper", "Technician", "Weed Killers...", {2, 4}, {"Weeds destroyed:", "Weeds eradicated:"}},
    {"Soda Vendor", "Refresher", "Thirst quenchers...", {2, 5}, {"Beverages served:", "Satisfied customers:"}},
};
inline constexpr const char* kSkilledRefusal =
    "You need to build up to a Daily Fee course (6 or more holes) before you can hire skilled employees.";

// Hire dialog ("HIRE AN EMPLOYEE", title at (377,107)): eight stacked choices, choice = 2 * kind + skilled. The pointer's y
// selects the choice: kind bands start at y = 171, 242, 313, 384 and are 40 tall, the second choice of a kind starts at
// band + 20 (the exe tests y only). Hire returns the choice and the exe turns it into kind = choice/2, skilled = choice&1.
inline constexpr int kHireBandY[4] = {171, 242, 313, 384};
constexpr int kHireBandH = 40;
constexpr int kHireBandSplit = 19;

// Wage payment (exact formula, units of $100): in the per-period money routine, every employee is paid
//   kKinds[kind].wageUnits[skilled]
// when rand(4 - d) <= tier, where d = DAT_00822c88 (probably the difficulty, 0..3), tier = 0 for next hole number <= 6,
// 1 for <= 10, 2 for <= 18. The routine runs when ticks % (1024 / (d + 2)) == 0 (a month is 1024 ticks).
constexpr int wagePeriodTicks(int d) { return 1024 / (d + 2); }

}  // namespace employees

}  // namespace panels
}  // namespace sg

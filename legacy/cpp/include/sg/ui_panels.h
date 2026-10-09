// SimGolf native port: screen layout of the dock panels (Elevation, Add Buildings, Amenities, Employees, plus the Golfers and
// Player panels), as read from the publisher exe's panel draw and hit routines. Companion text: docs/UI_PANELS.md.
//
// Conventions
//  * Screen is 800x600. "dst" is a screen position (top left of the sprite). A "cut" is a rectangle on the panel's own sheet in the
//    game's Interface folder (sheet name without extension; the sheet with "_A" appended is its alpha mask). A panel body is the
//    cut drawn at the same x, y it has on its sheet.
//  * Sheet colour key: magenta (255,0,255); the per-theme layout sheets use (248,0,248). Test r>=240, b>=240, g<=8.
//  * Hit tests use the exe's metric: with a = |dx*xs|, b = |dy*ys|, d = (b < a) ? (b + 2a) / 2 : (a + 2b) / 2, a point hits when
//    d < r. `Disc` plus `inDisc` implement it (xs, ys are the per-axis scale the exe passes).
//  * Money is stored by the exe in units of $100; every cost here is in units unless the name ends in Dollars.
//  * Theme index is the exe's: 0 Parkland, 1 Desert, 2 Tropical, 3 Links (sgview's themeExe() maps to it).
//  * Hover tooltips appear once the pointer has stayed on the same hit for more than 10 frames (kTipDelayFrames).
//
// Nothing here copies disc art; it only says where to cut and where to draw.
#pragma once

#include <cstdint>

namespace sg {
namespace ui_panels {

struct Rect {
    int x, y, w, h;
};
constexpr bool contains(const Rect& r, int px, int py) { return px >= r.x && px < r.x + r.w && py >= r.y && py < r.y + r.h; }

struct Disc {
    int cx, cy, r, xs, ys;
};
constexpr int iabs(int v) { return v < 0 ? -v : v; }
constexpr int metric(int a, int b) {
    a = iabs(a);
    b = iabs(b);
    return b < a ? (b + 2 * a) / 2 : (a + 2 * b) / 2;
}
constexpr bool inDisc(const Disc& d, int px, int py) { return metric((px - d.cx) * d.xs, (py - d.cy) * d.ys) < d.r; }
constexpr int clampi(int v, int lo, int hi) { return v < lo ? lo : (v > hi ? hi : v); }

constexpr int kTipDelayFrames = 10;   // tooltip shows when the hover counter exceeds this

// Generic tooltip bar (used by Elevation, Employee and the Undo buttons): a translucent bar from x-3n to x+3n (n = text length,
// x clamped to [3n, 800-3n]) at the pointer y minus 5, text centred above it at y minus 10 relative to the bar. Colours: bar
// 0x80000000, text 0x80007fff.
constexpr int tipBarCentreX(int pointerX, int textLen) { return clampi(pointerX, 3 * textLen, 800 - 3 * textLen); }

// ----------------------------------------------------------------------------------------------------------------------------
// Dock modes (DAT_00567afc). Dock button 0, 1, 2 toggle mode 0, 1, 2 (pressing the active one closes the panel, mode 5).
// Mode 3 is entered by clicking a golfer on the map (Player panel with a golfer selected); mode 4 is the Player panel with none.
enum DockMode { ModeBuildCourse = 0, ModeAddBuildings = 1, ModeGolfers = 2, ModePlayerSelected = 3, ModePlayer = 4, ModeClosed = 5 };
// Sub-panels: mode 0 shows Terrain or, when the Amenities flag is set, Amenities; mode 1 shows Buildings or, when the Elevation
// flag is set, Elevation; modes 2 and 4 and 3 can be covered by the Employee panel (overlay flag).
//   Terrain    --amenities button--> Amenities     --"Course Terrain" back button--> Terrain
//   Buildings  --elevation button--> Elevation     --"Buildings" back button-------> Buildings
//   Golfers/Player --employees button--> Employees --golfer tab--> Golfers, --player tab--> Player

// ----------------------------------------------------------------------------------------------------------------------------
// Build Course (Terrain) panel: pieces that were not part of the first rebuild.
inline constexpr Rect kTerrainBody = {216, 482, 584, 118};      // sheet BaseTerrainPanel, drawn at 216,482
// Tree buttons (slots 13..15) sit at a per-theme position; theme rows are Parkland, Desert, Tropical, Links.
struct XY { int x, y; };
inline constexpr XY kTerrainTreePos[4][3] = {
    {{704, 477}, {673, 530}, {735, 511}},
    {{704, 488}, {673, 533}, {735, 496}},
    {{704, 483}, {673, 519}, {735, 496}},
    {{704, 478}, {673, 512}, {735, 524}},
};
// Round button to the Amenities panel: hover cut on BaseTerrainPanel.
inline constexpr Disc  kTerrainToAmenitiesHit = {237, 525, 20, 1, 1};
inline constexpr Rect  kTerrainToAmenitiesCut = {150, 480, 34, 34};
inline constexpr XY    kTerrainToAmenitiesDst = {220, 510};
// Undo button: hover only (no armed look in this panel). Click shows the undo help popup and arms undo.
inline constexpr Disc  kTerrainUndoHit = {261, 578, 15, 1, 1};
inline constexpr Rect  kTerrainUndoCut = {40, 550, 34, 40};
inline constexpr XY    kTerrainUndoDst = {246, 562};
// Tile tooltip box: 160x112 at y 402, x = clamp(slotX + 32, 80, 720) - 80, with a header strip of 16 px pieces at y 389 (alpha
// pieces from TransPopups_A, see UI_PANELS.md). Name line, then the per-tile cost line.
constexpr int terrainTipX(int slotX) { return clampi(slotX + 32, 80, 720) - 80; }
inline constexpr Rect kTerrainTipBoxSize = {0, 402, 160, 112};
// The Tees button (slot 0) is drawn in its disabled state (cut 4 of the slot) while the current hole already has a tee placed,
// and for every hole from 19 on. The Green button (slot 1) gets an animated hint sprite while no green exists yet.

// ----------------------------------------------------------------------------------------------------------------------------
// Elevation panel. Sheet ElevationPanel, body {215,482,585,118} at the same place.
inline constexpr Rect kElevationBody = {215, 482, 585, 118};

struct ElevTool {
    const char* tip;
    Disc hit;
    XY dst;
    Rect hover;       // drawn while the pointer is over the button (plain look)
    Rect selected;    // drawn while this tool is current (bright green)
    // a third cut exists at x + 200 (x + 200 again for tool 3: 600) and is never drawn (dark look)
};
// tool ids: 0 vertex, 1 square, 2 area, 3 analyze golf shot. The tool buttons are also baked into the body (idle look).
inline constexpr ElevTool kElevTools[4] = {
    {"Raise/lower vertex", {352, 547, 25, 1, 1}, {312, 501}, {0, 0, 88, 99}, {100, 0, 88, 99}},
    {"Raise/lower square", {467, 547, 25, 1, 1}, {429, 501}, {0, 100, 88, 99}, {100, 100, 88, 99}},
    {"Raise/lower area", {587, 547, 25, 1, 1}, {546, 501}, {0, 200, 88, 99}, {100, 200, 88, 99}},
    {"Analyze Golf Shot", {715, 547, 16, 1, 1}, {688, 501}, {400, 0, 72, 99}, {500, 0, 72, 99}},
};
// Back to the Buildings panel ("Buildings"): hover cut only.
inline constexpr Disc kElevBackHit = {270, 499, 16, 1, 1};
inline constexpr Rect kElevBackCut = {0, 400, 41, 44};
inline constexpr XY   kElevBackDst = {254, 483};
// Undo ("Undo" tooltip): hover cut, and an armed cut drawn at the same place while undo mode is on.
inline constexpr Disc kElevUndoHit = {267, 561, 12, 1, 1};
inline constexpr Rect kElevUndoHover = {0, 450, 34, 40};
inline constexpr Rect kElevUndoArmed = {50, 450, 34, 40};
inline constexpr XY   kElevUndoDst = {253, 546};
inline constexpr const char* kElevBackTip = "Buildings";
inline constexpr const char* kElevUndoTip = "Undo";
// Hit result: -2 back, -3 undo, 0..3 tool, -1 none. Undo wins over everything, then tool 3, 2, 1, 0, then back.
constexpr int elevHit(int px, int py) {
    if (inDisc(kElevUndoHit, px, py)) return -3;
    for (int i = 3; i >= 0; --i)
        if (inDisc(kElevTools[i].hit, px, py)) return i;
    if (inDisc(kElevBackHit, px, py)) return -2;
    return -1;
}
// Draw order: body (alpha body first), hover cut of back button if hovered, undo armed cut if undo mode, hover cut of undo or of
// the hovered tool, then the selected cut of the current tool. A tool click sets the tool and clears the building tool.

// ----------------------------------------------------------------------------------------------------------------------------
// Add Buildings panel. Sheet BuildingPanel, body {216,482,584,118} at the same place. Nine lots in two rows (5 + 4).
inline constexpr Rect kBuildingsBody = {216, 482, 584, 118};
constexpr int kLotCount = 9;

struct Lot {
    int id;               // exe building id (6..14); the lot index is id - 6
    const char* name;
    int footprint;        // tiles
    int costUnits;        // level 0 cost, units of $100
    const char* tip;      // third line of the info box
    Disc hit;             // r 40 with ys 2 (centre is where the isometric slab sits)
    XY pad;               // pad sprite destination (at x 343+80i row 1, 384+80i row 2; y 501 / 542)
    XY icon;              // lot picture destination (one pixel higher while hovered and buildable)
};
inline constexpr Lot kLots[kLotCount] = {
    {6, "Putting Green", 3, 100, "Helps imaginative golfers", {383, 531, 40, 1, 2}, {343, 501}, {344, 453}},
    {7, "Snack Bar", 2, 150, "Feeds hungry golfers", {463, 531, 40, 1, 2}, {423, 501}, {424, 453}},
    {8, "Pro Shop", 2, 200, "Improves accurate golfers", {543, 531, 40, 1, 2}, {503, 501}, {504, 453}},
    {9, "Swim Club", 3, 300, "Golfers become members", {623, 531, 40, 1, 2}, {583, 501}, {584, 453}},
    {10, "Driving Range", 5, 250, "Helps long hitters", {703, 531, 40, 1, 2}, {663, 501}, {664, 453}},
    {11, "Cart Garage", 2, 400, "Golfers play faster", {423, 571, 40, 1, 2}, {384, 542}, {384, 493}},
    {12, "Marina", 2, 1000, "Increases property values", {503, 571, 40, 1, 2}, {464, 542}, {464, 493}},
    {13, "Resort Hotel", 4, 2500, "Golfers stay happier", {583, 571, 40, 1, 2}, {544, 542}, {544, 493}},
    {14, "Airstrip", 6, 5000, "Higher greens fees", {663, 571, 40, 1, 2}, {624, 542}, {624, 493}},
};
// The Clubhouse (id 15, footprint 4, cost 200 units) has no button in any panel.

// Pad cuts on BuildingPanel: lots 0..4 use the first row, lots 5..8 the second. Normal pad look is baked into the body.
constexpr Rect padHoverCut(int lot) { return lot < 5 ? Rect{0, 0, 74, 64} : Rect{0, 100, 77, 58}; }
constexpr Rect padSelectedCut(int lot) { return lot < 5 ? Rect{100, 0, 77, 64} : Rect{100, 100, 77, 58}; }
// A third (blue) cut at x 200 is cut by the loader but never drawn. Pads are drawn only for buildable lots.

// Lot pictures: theme layout sheet (Parklands Layout, Desert Layout, Trop Layout, Links Layout; exe theme order 0..3), cells
// 75x100, column = lot index. The exe picks object 4*lot + 2*upgraded + buildable, loaded in sheet row order 0, 200, 100, 300:
// grey level 1, colour level 1, grey upgraded, colour upgraded.
inline constexpr int kLotIconRowY[4] = {0, 200, 100, 300};
constexpr Rect lotIconCut(int lot, bool upgraded, bool buildable) { return Rect{lot * 75, kLotIconRowY[(upgraded ? 2 : 0) + (buildable ? 1 : 0)], 75, 100}; }
constexpr XY lotIconDst(int lot, bool hoveredAndBuildable) { return XY{(lot / 5 + (lot % 5) * 2) * 40 + 344, 453 + (lot / 5) * 40 - (hoveredAndBuildable ? 1 : 0)}; }

// Per-lot state the exe keeps: level[id] in 0..2 (0 base, 1 upgraded, 2 maxed), unlockCounter (6 in a normal game, 17 sandbox),
// nextHole (DAT_005685f0). A lot is buildable when
//   id < unlockCounter - level  &&  level < 2  &&  (level == 0 || nextHole > 10)
constexpr bool lotBuildable(int id, int level, int unlockCounter, int nextHole) {
    return id < unlockCounter - level && level < 2 && (level == 0 || nextHole > 10);
}
// Placement price in units: cost * (level + 2) / 2 + terrain extra (footprint scanned is footprint + level).
constexpr int lotPriceUnits(int baseCostUnits, int level, int terrainExtraUnits) { return baseCostUnits * (level + 2) / 2 + terrainExtraUnits; }
// Dollars shown in the info box: base * 100 for level 0, base * 300 / 2 when upgraded (the Snack Bar always shows level 0).
constexpr long lotInfoDollars(int lotIndex, int level) { return (lotIndex == 1 || level == 0) ? kLots[lotIndex].costUnits * 100L : kLots[lotIndex].costUnits * 300L / 2; }

// Info box above a hovered lot (after the delay): frame 160x100 at y 460 around the pop-up frame art (Pop_UpOk pieces), centred
// text at x = box centre: line 1 y 462 ("Upgraded " + name when level != 0 and not Snack Bar), line 2 y 472 "Cost: $N",
// line 3 y 482 the tip. Box centre x:
constexpr int lotInfoCentreX(int lot) { return clampi((lot % 5) * 80 + (lot > 4 ? 431 : 391), 80, 720); }
inline constexpr Rect kLotInfoBoxSize = {0, 460, 160, 100};   // x = lotInfoCentreX(lot) - 80

// Round buttons. Elevation (hover cut only), Undo (hover plus armed).
inline constexpr Disc kBuildToElevationHit = {236, 526, 16, 1, 1};
inline constexpr Rect kBuildToElevationCut = {150, 480, 34, 34};
inline constexpr XY   kBuildToElevationDst = {220, 510};
inline constexpr const char* kBuildToElevationTip = "Elevation";
inline constexpr Disc kBuildUndoHit = {277, 567, 12, 1, 1};
inline constexpr Rect kBuildUndoHover = {40, 550, 34, 40};
inline constexpr Rect kBuildUndoArmed = {80, 550, 34, 40};
inline constexpr XY   kBuildUndoDst = {261, 553};
// Hit result: -2 elevation, -3 undo, 0..8 lot, -1 none. Evaluation order in the exe: elevation, undo, then lots 0..8 (first wins).
constexpr int buildingsHit(int px, int py) {
    if (inDisc(kBuildToElevationHit, px, py)) return -2;
    if (inDisc(kBuildUndoHit, px, py)) return -3;
    for (int i = 0; i < kLotCount; ++i)
        if (inDisc(kLots[i].hit, px, py)) return i;
    return -1;
}
// Refusal texts (error sound 0x18, red popup) in the order the click checks them:
inline constexpr const char* kBuildRefuseUpgradeEarly = "Sorry, upgraded buildings are not available until you build beyond 9 holes.";
inline constexpr const char* kBuildRefuseMaxed = "You cannot upgrade this building any further.";
inline constexpr const char* kBuildRefuseLocked = "This building is will become available as you build additional holes.";   // typo is the exe's
inline constexpr const char* kUndoHelp1 = "To UNDO a previous action, right click on the map to display the effect of the undo. ";
inline constexpr const char* kUndoHelp2 = "Right click again to effect the change. Your money will be refunded!";

// ----------------------------------------------------------------------------------------------------------------------------
// Amenities panel. Sheet AmenitiesPanel, body {214,482,586,118} at the same place.
inline constexpr Rect kAmenitiesBody = {214, 482, 586, 118};
// Slot s: hover cut at (cx0, cy0), selected cut at cx0 + 100, disabled cut at cx0 + 200 (the right-hand group uses 300, 400, 500).
struct AmenitySlot {
    const char* tip;      // tooltip heading
    int tool;             // exe tool id armed by the click (-1 none)
    Disc hit;
    XY dst;
    Rect hoverCut;        // selected = hoverCut.x + 100, disabled = hoverCut.x + 200, same y, w, h
};
inline constexpr int kAmenitySlotCount = 10;
inline constexpr AmenitySlot kAmenitySlots[kAmenitySlotCount] = {
    {"Ballwasher",     3, {302, 563, 20, 1, 1}, {274, 536}, {0, 0, 60, 56}},
    {"Pathway",        0, {333, 526, 20, 1, 1}, {302, 504}, {0, 100, 64, 50}},
    {"Building Lot",   5, {363, 563, 20, 1, 1}, {334, 538}, {0, 200, 62, 50}},
    {"Benches",        1, {470, 575, 18, 1, 1}, {449, 553}, {300, 0, 51, 47}},
    {"Landmarks",      4, {591, 575, 18, 1, 1}, {571, 553}, {300, 100, 51, 45}},
    {"Flower Bed",     2, {652, 575, 18, 1, 1}, {632, 553}, {300, 150, 51, 47}},
    {"Scenic Trees",  16, {714, 575, 18, 1, 1}, {693, 553}, {300, 200, 51, 45}},
    {"Undo",          -1, {248, 567, 12, 1, 1}, {234, 553}, {0, 450, 36, 36}},
    {nullptr,         -1, {0, 0, 0, 1, 1},      {762, 545}, {0, 500, 38, 36}},   // slot 8: art only, never hit-tested
    {"Scenic Bridge", 19, {531, 580, 12, 1, 1}, {510, 553}, {300, 50, 51, 45}},
};
constexpr Rect amenitySelectedCut(int s) { return Rect{kAmenitySlots[s].hoverCut.x + 100, kAmenitySlots[s].hoverCut.y, kAmenitySlots[s].hoverCut.w, kAmenitySlots[s].hoverCut.h}; }
constexpr Rect amenityDisabledCut(int s) { return Rect{kAmenitySlots[s].hoverCut.x + 200, kAmenitySlots[s].hoverCut.y, kAmenitySlots[s].hoverCut.w, kAmenitySlots[s].hoverCut.h}; }
inline constexpr Disc kAmenBackHit = {269, 500, 16, 1, 1};   // "Course Terrain"
inline constexpr Rect kAmenBackCut = {0, 400, 34, 34};
inline constexpr XY   kAmenBackDst = {254, 483};
inline constexpr const char* kAmenBackTip = "Course Terrain";
// Hit result: -2 back, 0..7 and 9 slot, -1 none. Later checks win: 9, 7, 6, 5, 4, 3, 2, 1, 0, then back. The Home Site slot (2) is
// drawn disabled and cannot be armed while the free-site counter (DAT_0056d1b0) is below 1.
constexpr int amenitiesHit(int px, int py) {
    if (inDisc(kAmenitySlots[9].hit, px, py)) return 9;
    for (int i = 7; i >= 0; --i)
        if (i != 8 && inDisc(kAmenitySlots[i].hit, px, py)) return i;
    if (inDisc(kAmenBackHit, px, py)) return -2;
    return -1;
}
// Slot to draw as selected for the current building tool, -1 for none.
constexpr int amenitySlotForTool(int tool) {
    switch (tool) {
        case 3: return 0; case 0: return 1; case 5: return 2; case 1: return 3;
        case 4: return 4; case 2: return 5; case 16: return 6; case 19: return 9; default: return -1;
    }
}
// Variant strips (shown above the panel while the tool is armed): the pointer picks a design, then a map click places it.
//   x0 = 544 + anchor - trunc(n * 47 / 4); the strip is n/2 background segments, then an end piece, all at y 477.
//   Entry i centre: x0 + 44 + (i * 47) / 2, y 500 (even i) or 527 (odd i); entry hit: metric(dx, 2 * dy) < 20.
//   Entry picture is drawn anchored at the centre, size px square. Background pieces are cuts of AmenitiesPanel_A.
struct VariantStrip {
    const char* name;
    int tool;
    int anchor;
    int count;
    int firstSpriteId;   // exe sprite object id; entry i uses firstSpriteId + i (flowers: see UI_PANELS.md)
    int picSize;         // -1 means 240 or 140 by theme rule in the exe; 40 for most
};
inline constexpr VariantStrip kVariantStrips[5] = {
    {"Benches", 1, -84, 5, 0x208, 40},
    {"Landmarks", 4, 0, 14, 0x168, -1},
    {"Scenic Bridge", 19, -16, 8, 0x226, 40},
    {"Flower Bed", 2, 13, 5, 0x1a3, 40},     // 5 base shapes x 3 families = 15 entries
    {"Scenic Trees", 16, 100, 7, 0x12f, 100},
};
inline constexpr int kStripBackgroundY = 477;
inline constexpr int kStripEntryY[2] = {500, 527};
// Strip background cuts on AmenitiesPanel_A (alpha only): first {380,250,68,79}, middle {450,250,47,79} repeated, end piece
// {600,250,67,79} for an even count and {500,250,67,79} for an odd count.
inline constexpr Rect kStripCutFirst = {380, 250, 68, 79};
inline constexpr Rect kStripCutMiddle = {450, 250, 47, 79};
inline constexpr Rect kStripCutEndEven = {600, 250, 67, 79};
inline constexpr Rect kStripCutEndOdd = {500, 250, 67, 79};
constexpr int stripX0(int anchor, int count) { return 544 + anchor - (count * 47) / 4; }
constexpr int stripEntryCx(int x0, int i) { return x0 + 44 + (i * 47) / 2; }
constexpr int stripEntryCy(int i) { return (i & 1) ? 527 : 500; }
constexpr bool stripEntryHit(int x0, int i, int px, int py) { return metric(px - stripEntryCx(x0, i), (py - stripEntryCy(i)) * 2) < 20; }

// Amenity tooltips: heading, then "Cost: $N" (landmarks: design name uppercased, effect, cost or "FREE!").
inline constexpr const char* kLandmarkEffect[4] = {"Happy Golfers", "No Dandelions", "Skill Upgrade", "Happy Endings"};   // design & 3
inline constexpr int kAmenityCostUnits[7] = {50 /*ball washer*/, 1 /*path per tile*/, 10 /*home site*/, 2 /*bench*/, -1 /*landmark 250+50*design*/, 5 /*flower bed*/, 25 /*scenic tree*/};
inline constexpr int kBridgeCostUnits = 100;

// ----------------------------------------------------------------------------------------------------------------------------
// Employee panel (an overlay on the Golfers and Player panels). Sheet EmployeePanel, body {215,474,585,126} at the same place.
inline constexpr Rect kEmployeeBody = {215, 474, 585, 126};
constexpr int kMaxEmployees = 20;
constexpr int kEmployeePortraits = 8;      // 4 columns x 2 rows; scroll arrows move one column (2 employees)

// Hire button: hit (270,558) r16, dst (254,542). Hover cut {0,500,42,42}; pressed {50,500,42,42} (one frame, then the hire dialog
// opens); disabled {100,500,42,42} drawn while 20 employees exist. The {0..100,550} cuts are empty on the sheet.
inline constexpr Disc kEmpHireHit = {270, 558, 16, 1, 1};
inline constexpr XY   kEmpHireDst = {254, 542};
inline constexpr Rect kEmpHireHover = {0, 500, 42, 42}, kEmpHirePressed = {50, 500, 42, 42}, kEmpHireDisabled = {100, 500, 42, 42};
inline constexpr const char* kEmpHireTip = "Hire employees";

// Scroll arrows: hit uses xs = 4 (narrow horizontally): centre (dst.x + 8, dst.y + 30), r30. Hover cut {0,200,16,62} (left) or
// {0,300,16,62} (right); the enabled look is the third cut (x 100), drawn at the same dst while scrolling is possible.
inline constexpr Disc kEmpLeftHit = {321, 549, 30, 4, 1}, kEmpRightHit = {604, 549, 30, 4, 1};
inline constexpr XY   kEmpLeftDst = {313, 519}, kEmpRightDst = {596, 519};
inline constexpr Rect kEmpLeftHover = {0, 200, 16, 62}, kEmpLeftEnabled = {100, 200, 16, 62};
inline constexpr Rect kEmpRightHover = {0, 300, 16, 62}, kEmpRightEnabled = {100, 300, 16, 62};

// Action buttons for the selected employee. Cuts per button row y0 (Move 0, Fire 50, Rename 100): hover {400,y0}, armed {500,y0}
// (Move only), disabled {600,y0}, idle {700,y0}; all 52x48. Disabled is drawn while nobody is selected; with a selection the idle
// cut is drawn and the hover cut over it while hovered.
struct EmpAction { const char* tip; Disc hit; XY dst; int rowY; };
inline constexpr EmpAction kEmpActions[3] = {
    {"Move this employee", {647, 513, 16, 1, 1}, {631, 497}, 0},
    {"Fire this employee", {699, 510, 16, 1, 1}, {683, 494}, 50},
    {"Rename this employee", {756, 513, 16, 1, 1}, {740, 497}, 100},
};
constexpr Rect empActionCut(int a, int state) { return Rect{400 + state * 100, kEmpActions[a].rowY, 52, 48}; }   // 0 hover 1 armed 2 disabled 3 idle
inline constexpr int kEmpFireCostUnits = 25;     // $2,500 charged when firing

// Portrait slots, index p = 0..7 (column-major: p / 2 is the column, p & 1 the row). Hit: centre (dst.x + 32, dst.y + 16), r16.
inline constexpr XY kEmpPortraitDst[kEmployeePortraits] = {{336, 507}, {336, 550}, {399, 507}, {399, 550}, {462, 507}, {462, 550}, {525, 507}, {525, 550}};
constexpr Disc empPortraitHit(int p) { return Disc{kEmpPortraitDst[p].x + 32, kEmpPortraitDst[p].y + 16, 16, 1, 1}; }
// Hover cut {0, 50*(p&1), 63, 43}, selected cut {100, 50*(p&1), 63, 43}, spare {200, ...}. The employee sprite is drawn at
// (dst.x + 30, dst.y + 24).
constexpr Rect empPortraitHoverCut(int p) { return Rect{0, 50 * (p & 1), 63, 43}; }
constexpr Rect empPortraitSelectedCut(int p) { return Rect{100, 50 * (p & 1), 63, 43}; }

// Tabs (hit before slots are not tested: slots win when both match). Golfers tab: hit (286,492) r16, hover cut {0,400,33,33} at (270,475).
// Player tab: hit (256,510) r16, hover cut {0,450,33,33} at (240,494).
inline constexpr Disc kEmpGolfersTabHit = {286, 492, 16, 1, 1}, kEmpPlayerTabHit = {256, 510, 16, 1, 1};
inline constexpr Rect kEmpGolfersTabCut = {0, 400, 33, 33}, kEmpPlayerTabCut = {0, 450, 33, 33};
inline constexpr XY   kEmpGolfersTabDst = {270, 475}, kEmpPlayerTabDst = {240, 494};

// Hit result: -3 player tab, -2 golfers tab, 0 hire, 2 left arrow, 3 right arrow, 4 move, 5 fire, 6 rename, 8..15 portrait
// (index - 8), -1 none. Slots win over tabs; the first matching slot wins (slot 1 duplicates slot 0 and is never returned, slot 7
// duplicates 5).
constexpr int employeeHit(int px, int py) {
    if (inDisc(kEmpHireHit, px, py)) return 0;
    if (inDisc(kEmpLeftHit, px, py)) return 2;
    if (inDisc(kEmpRightHit, px, py)) return 3;
    for (int a = 0; a < 3; ++a)
        if (inDisc(kEmpActions[a].hit, px, py)) return 4 + a;
    for (int p = 0; p < kEmployeePortraits; ++p)
        if (inDisc(empPortraitHit(p), px, py)) return 8 + p;
    if (inDisc(kEmpPlayerTabHit, px, py)) return -3;
    if (inDisc(kEmpGolfersTabHit, px, py)) return -2;
    return -1;
}
// Selected employee text, centred at x 700: name y 534, "Hired: <date>" 546, "Paid: <dollars>" 558, counter label 572, counter 584.
inline constexpr int kEmpInfoX = 700;
inline constexpr int kEmpInfoY[5] = {534, 546, 558, 572, 584};

// Kinds. Record type byte = -2 - kind; the skilled flag is bit 8 of the record flags. Staff sprite set (portrait and map sprite
// files employee\<name>Walk / SQ / Action) = kind + 4 * skilled:
//   0 Greeter, 1 Ranger, 2 GK, 3 TrayGirl, 4 GolfCeleb, 5 Marshall, 6 LawnTech, 7 SodaVendor
struct StaffKind {
    const char* name;
    const char* skilledName;
    int wageUnits[2];            // regular, skilled; units of $100, shown in the hire dialog as "per week"
    const char* counter[2];      // label of the counter shown for a selected employee (regular, skilled)
    const char* blurbStart;      // dialog blurb begins with this
};
inline constexpr StaffKind kStaffKinds[4] = {
    {"Club Pro", "Celebrity", {3, 7}, {"Players greeted:", "Players cheered:"}, "Greeters"},
    {"Ranger", "Marshall", {2, 3}, {"Players rushed:", "Slackers intimidated:"}, "Speed up play"},
    {"Groundskeeper", "Technician", {2, 4}, {"Weeds destroyed:", "Weeds eradicated:"}, "Weed Killers"},
    {"Soda Vendor", "Refresher", {2, 5}, {"Beverages served:", "Satisfied customers:"}, "Thirst quenchers"},
};
inline constexpr const char* kStaffSpriteSet[8] = {"GreeterWalk/GreeterSQ/GreeterAction2", "RangerWalk/RangerSQ/RangerAction", "GKWalk/GKSq/GKAction",
    "TrayGirl_Walk/TrayGirl_SQ/TrayGirl_Action", "GolfCeleb_Walk/_SQ/_Action", "Marshall_Walk/_SQ/_Action", "LawnTech_Walk/_Sq/_Action",
    "SodaVendorWalk/SodaVendorSQ/SodaVendorAction"};

// Hire dialog ("HIRE AN EMPLOYEE", modal). Title at (377,107), sheet "hire". Eight choices, c = 2 * kind + skilled, in four bands. The
// exe tests only the pointer y: band k spans y in [kHireBandY[k], kHireBandY[k] + 40), upper half (y <= band + 19) regular,
// lower half skilled. The dialog returns kind * 3 + 1 + skilled to its caller. Choice text "Name: $N per week" at x 246.
inline constexpr int kHireBandY[4] = {171, 242, 313, 384};
constexpr int hireChoiceAt(int py) {
    for (int k = 0; k < 4; ++k)
        if (py >= kHireBandY[k] && py <= kHireBandY[k] + 39) return 2 * k + (py > kHireBandY[k] + 19 ? 1 : 0);
    return -1;
}
inline constexpr XY kHireTitle = {377, 107};
inline constexpr int kHireTextX = 246;
inline constexpr int kHirePortraitX = 576;
inline constexpr int kHirePortraitY[4] = {192, 262, 332, 402};   // 40x40 animated portraits, one per kind

inline constexpr const char* kSkilledRefusal = "You need to build up to a Daily Fee course (6 or more holes) before you can hire skilled employees.";

// ----------------------------------------------------------------------------------------------------------------------------
// Golfers panel (mode 2, sheet MemberPanel, body {215,474,585,126}).
inline constexpr Rect kGolfersBody = {215, 474, 585, 126};
// Roster grid: cells 121 wide, 21 high, origin (310,498), index = row + 4 * col with row and col clamped to 0..3 (so the grid is
// 4x4; the result indexes the list of golfers drawn this frame). Plate cut (0,0,121,44) is drawn under odd-numbered golfers.
constexpr int golfersRosterCell(int px, int py) {
    if (px <= 309 || py <= 497) return -1;
    return clampi((py - 498) / 21, 0, 3) + 4 * clampi((px - 310) / 121, 0, 3);
}
inline constexpr Disc kGolfersGolfersTabHit = {285, 492, 20, 1, 1};     // current tab, no action
inline constexpr Disc kGolfersPlayerTabHit = {256, 510, 20, 1, 1};      // goes to mode 4
inline constexpr Disc kGolfersEmployeesHit = {232, 538, 20, 1, 1};      // opens the Employee overlay
inline constexpr Disc kGolfersScrollLeftHit = {332, 592, 20, 1, 3};     // scrolls the list back 16 entries
inline constexpr Disc kGolfersScrollRightHit = {772, 592, 20, 1, 3};    // scrolls forward 16 entries
inline constexpr Rect kGolfersPlayerTabCut = {0, 400, 33, 33};  inline constexpr XY kGolfersPlayerTabDst = {240, 494};
inline constexpr Rect kGolfersEmployeesCut = {0, 450, 33, 33};  inline constexpr XY kGolfersEmployeesDst = {217, 522};
inline constexpr Rect kGolfersScrollLeftCut = {0, 150, 36, 13}; inline constexpr XY kGolfersScrollLeftDst = {309, 587};
inline constexpr Rect kGolfersScrollRightCut = {0, 250, 36, 13}; inline constexpr XY kGolfersScrollRightDst = {756, 587};
// Hit result: -3 player tab, -4 employees, -5 scroll left, -6 scroll right, -2 golfers tab, 0..15 roster cell, -1 none.
constexpr int golfersHit(int px, int py) {
    int r = -1;
    if (px > 309 && py > 497) r = golfersRosterCell(px, py);
    if (inDisc(kGolfersGolfersTabHit, px, py)) r = -2;
    if (inDisc(kGolfersPlayerTabHit, px, py)) r = -3;
    if (inDisc(kGolfersEmployeesHit, px, py)) r = -4;
    if (inDisc(kGolfersScrollLeftHit, px, py)) r = -5;
    if (inDisc(kGolfersScrollRightHit, px, py)) return -6;
    return r;
}
inline constexpr const char* kGolfersTabTip[3] = {"Golfers", "Gary Golf", "Hire Employees"};   // tips of -2, -3, -4 (player name is configurable)

// ----------------------------------------------------------------------------------------------------------------------------
// Player panel (modes 3 and 4, sheet JoeCoolPanel, body {214,474,586,126}).
inline constexpr Rect kPlayerBody = {214, 474, 586, 126};
struct PlayerButton {
    const char* tip;
    Disc hit;
    XY hoverDst;
    Rect hoverCut;
};
// Index = hit result of the exe. 0 is the round "player stats" button; 1..3 the three lower-left action buttons (practice, play,
// tournament; each is covered by its disabled cut {100,y} while a golfer is selected, mode 3, or when its condition fails);
// 4..8 the shot-shape ovals (sheet row r = index - 4, 80x50 cells, hover cut is the normal column); 9 golfers tab; 10 employees.
inline constexpr PlayerButton kPlayerButtons[11] = {
    {nullptr, {273, 559, 20, 1, 1}, {255, 542}, {0, 300, 42, 44}},                  // no tooltip; action: opens the player screen (FUN_004385d0, not decoded)
    {"Practice Round", {313, 519, 15, 1, 1}, {298, 503}, {0, 500, 38, 32}},
    {"Play ", {313, 548, 15, 1, 1}, {298, 535}, {0, 535, 38, 30}},
    {"Begin Tournament", {313, 580, 15, 1, 1}, {298, 565}, {0, 565, 38, 32}},
    {"Straight shot", {414, 505, 20, 1, 1}, {374, 485}, {0, 1, 80, 50}},            // shot value 0
    {"Fade shot, L to R", {493, 505, 20, 1, 1}, {453, 485}, {0, 51, 80, 50}},       // shot value -1
    {"Draw shot, R to L", {572, 505, 20, 1, 1}, {532, 485}, {0, 101, 80, 50}},      // shot value 1
    {"High backspin shot", {651, 505, 20, 1, 1}, {611, 485}, {0, 151, 80, 50}},     // shot value 3
    {"Low punch shot", {730, 505, 20, 1, 1}, {690, 485}, {0, 201, 80, 50}},         // shot value 4
    {"Golfers", {286, 490, 15, 1, 1}, {270, 475}, {0, 400, 33, 33}},
    {"Hire employee", {231, 540, 15, 1, 1}, {217, 522}, {0, 450, 33, 33}},
};
inline constexpr int kPlayerShotValue[5] = {0, -1, 1, 3, 4};   // DAT_0058f330 set by buttons 4..8
// Oval cells on JoeCoolPanel: row r (r = button - 4) at y = 1 + 50 * r, columns x = 0 normal, 100 selected (green), 200 dark, 300 spare.
// Evaluation order of the hit: 0 .. 3, then (mode 3 only) 4..8, then 9, then 10 (later wins).
constexpr int playerHit(int mode, int px, int py) {
    int r = -1;
    for (int i = 0; i < 4; ++i)
        if (inDisc(kPlayerButtons[i].hit, px, py)) r = i;
    if (mode == ModePlayerSelected)
        for (int i = 4; i < 9; ++i)
            if (inDisc(kPlayerButtons[i].hit, px, py)) r = i;
    if (inDisc(kPlayerButtons[9].hit, px, py)) r = 9;
    if (inDisc(kPlayerButtons[10].hit, px, py)) r = 10;
    return r;
}

}  // namespace ui_panels
}  // namespace sg

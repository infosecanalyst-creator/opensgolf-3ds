// Layout tables for the information screens (Financial Report, Membership Roster, Hole Stats, SGA report, shortcuts, end of year,
// player comments, course overview, buy land, hire, tournament results). Values are read from the publisher exe's draw routines;
// see docs/UI_SCREENS.md for sources, confidence and unknowns. All sheets are loaded from the disc folder at run time.
// Coordinates are 800x600 screen pixels. "Cut" rectangles are source rectangles on the named sheet; where a piece is blitted at
// its own sheet position the destination equals the cut origin.
#pragma once
#include <cstdint>

namespace sg::ui_screens {

struct Rect { int x, y, w, h; };
struct Pt { int x, y; };
enum class Align : uint8_t { Left, Center, Right };
struct Field { int x, y; Align align; };

// ---- shared -------------------------------------------------------------------------------------------------------------------
constexpr float kDimStrength = 0.25f;     // every screen first dims the scene to this strength
constexpr int kShadowOffset = 2;          // text shadow, both axes
// Colour words are 5-5-5 RGB under a 0x8000 flag half. rgb555 converts the low 15 bits to 8 bit channels (channel * 8).
constexpr uint32_t kColBlack = 0x80000000u, kColRed = 0x80007d08u, kColDarkRed = 0x80006000u, kColWhite = 0x80007fffu,
                   kColBrightGreen = 0x800023e8u, kColDarkGreen = 0x80001284u, kColBlue = 0x80002108u, kColYellow = 0x80007ff0u,
                   kColNearBlack = 0x80000018u;
constexpr int rgb555R(uint32_t c) { return int((c >> 10) & 31) * 8; }
constexpr int rgb555G(uint32_t c) { return int((c >> 5) & 31) * 8; }
constexpr int rgb555B(uint32_t c) { return int(c & 31) * 8; }

// OkStates.pcx (266x131): round OK tick. Index 0 idle, 1 hover, 2 disabled, 3 same as 0, 4 pressed.
constexpr Rect kOkCut[5] = {{1, 1, 44, 42}, {46, 1, 44, 42}, {91, 1, 44, 42}, {1, 1, 44, 42}, {46, 44, 44, 42}};

// Sheet paths relative to Interface/infoscreens/.
constexpr const char* kSheetFinance = "FINANCEreport.pcx";      // alpha: FINANCEreport_alpha.pcx
constexpr const char* kSheetRoster = "memberRoster.pcx";        // + _alpha, memberRoster_buttons.pcx, memberRoster_scrollbar.pcx
constexpr const char* kSheetHoleStat = "HoleSTAT.pcx";          // + HoleSTAT_alpha.pcx
constexpr const char* kSheetSga = "SGA.pcx";                    // alpha: SGAreport_alpha.pcx
constexpr const char* kSheetShortcuts = "shortcuts.pcx";        // + shortcuts_alpha.pcx
constexpr const char* kSheetEndYear = "ENDoYEAR.pcx";           // + ENDoYEAR_alpha.pcx
constexpr const char* kSheetComments = "PlayComt.pcx";          // + PlayComt_alpha.pcx
constexpr const char* kSheetTournament = "tournament result.pcx";   // + "tournament result_alpha.pcx"
constexpr const char* kSheetBuyLand = "buy_land.pcx";           // + buy_land_buttons.pcx
constexpr const char* kSheetHire = "hire.pcx";                  // + hire_alpha.pcx
constexpr const char* kSheetRouteBottom = "route screens_bottom.pcx";   // headers: route screens_course/_value/_aura/_employ.pcx

// ---- 1. Financial Report -----------------------------------------------------------------------------------------------------
namespace fin {
constexpr Rect art{8, 7, 784, 289};
constexpr Field title{218, 21, Align::Left};            // "FINANCIAL REPORT", large face
constexpr int kRows = 9;                                // 8 ledger rows plus Total
constexpr int kLabelCx = 98;                            // label centre x
constexpr int labelY(int row) { return 87 + 17 * row + (row == 8 ? 7 : 0); }
constexpr int kColumns = 8;                             // visible year columns (the exe addresses a ninth, off screen)
constexpr int kColPitch = 75;
constexpr int headerCx(int col) { return 222 + kColPitch * col; }   // year header centre, y 60
constexpr int kHeaderY = 60;
constexpr int valueRightX(int col) { return 256 + kColPitch * col; }
constexpr int valueY(int row) { return 86 + 17 * row + (row == 8 ? 7 : 0); }   // row 8 is the Total at y 229
constexpr const char* kRowLabels[kRows] = {"Greens Fees", "Home Sites", "Food/Drink", "Build course", "Facilities",
                                           "Salaries", "Maint./Interest", "Other", "Total"};
constexpr int kYearBase = 2001;                         // header text = base + year index
constexpr Rect okHit{726, 249, 44, 44};                 // draw kOkCut[1] on hover
constexpr Pt okPos{726, 249};
}  // namespace fin

// ---- 2. Membership Roster ----------------------------------------------------------------------------------------------------
namespace roster {
constexpr Rect art{0, 0, 800, 600};
constexpr Field title{212, 19, Align::Left};
constexpr int kHeadY = 60;
constexpr Field headMember{33, kHeadY, Align::Left};
constexpr Field headLow{170, kHeadY, Align::Center}, headHcp{209, kHeadY, Align::Center}, headRnds{249, kHeadY, Align::Center},
    headStatus{320, kHeadY, Align::Center};
constexpr int kHoleX0 = 381, kHolePitch = 21, kHoles = 18, kHoleCellW = 19;
constexpr int holeX(int h0) { return kHoleX0 + kHolePitch * h0; }   // h0 = 0..17, head text and icon x
constexpr int kVisibleRows = 22, kRowY0 = 89, kRowPitch = 20;
constexpr int rowY(int r) { return kRowY0 + kRowPitch * r; }
constexpr Field colName{28, 0, Align::Left}, colLow{170, 0, Align::Center}, colHcp{209, 0, Align::Left}, colRnds{249, 0, Align::Left},
    colStatus{320, 0, Align::Center};                    // y = rowY(r)
constexpr Pt tierBallPos{122, -4};                      // + rowY; y offset -4
constexpr int kIconDy = -4;
// memberRoster_buttons.pcx, 19x18 cuts. Tier ball: 0 Member (navy), 1 Silver, 2 Gold, 3 Resigned (red). Visitors draw none.
constexpr Rect tierBall[4] = {{46, 1, 19, 18}, {66, 1, 19, 18}, {86, 1, 19, 18}, {106, 1, 19, 18}};
// Hole flag icons: index 0 is the piece at y 1, index 1 the piece at y 20. Odd holes (1 based) use index 1, even holes index 0.
constexpr int iconVariant(int hole1Based) { return (hole1Based & 1) ? 1 : 0; }
constexpr Rect iconCamera[2] = {{126, 1, 19, 18}, {126, 20, 19, 18}};   // flag bit 0: Photo Opp
constexpr Rect iconHeart[2] = {{146, 1, 19, 18}, {146, 20, 19, 18}};    // flag bit 1: Happy Ending
constexpr Rect iconResigned[2] = {{106, 1, 19, 18}, {106, 20, 19, 18}}; // flag bit 2: resigned on this hole
constexpr Field legend[6] = {{82, 536, Align::Center}, {208, 536, Align::Center}, {326, 536, Align::Center},
                             {450, 536, Align::Center}, {573, 536, Align::Center}, {693, 536, Align::Center}};
constexpr const char* kLegendText[6] = {"Member", "Silver Member", "Gold Member", "Resigned", "Photo Opp", "Happy Ending"};
// Status text by code (flags & 7): 1 Visitor, 2 Member, 3 Silver Member, 4 Gold Member; -1 Resigned.
constexpr const char* statusText(int code) {
    return code == 1 ? "Visitor" : code == 2 ? "Member" : code == 3 ? "Silver Member" : code == 4 ? "Gold Member" : code == -1 ? "Resigned" : "";
}
constexpr Pt okPos{732, 548};
constexpr Rect okHit{732, 548, 44, 44};
// Scroll bar (shown when members > 22).
constexpr Pt trackPos{767, 80};
constexpr Rect upZone{767, 80, 18, 37}, downZone{767, 490, 18, 37};
constexpr Rect upHoverCut{166, 1, 18, 37}, downHoverCut{185, 1, 18, 37};  // memberRoster_buttons.pcx
constexpr Pt upHoverPos{767, 80}, downHoverPos{767, 490};
constexpr int kThumbX = 773, kThumbW = 6, kThumbY0 = 122, kTrackLen = 364;
constexpr uint32_t kThumbColour = kColWhite;
}  // namespace roster

// ---- 3. Hole Stats ----------------------------------------------------------------------------------------------------------
namespace holestat {
constexpr Rect top{0, 0, 800, 220}, rowStrip{0, 282, 800, 16}, bottom{0, 347, 800, 55};
constexpr Field title{385, 48, Align::Center};          // "HOLE STATS for N name (N)"
constexpr int kLeftLabelX = 190, kLeftValueCx = 356;    // Fun Factor, Length, Accuracy, Imagination
constexpr int leftRowY(int i) { return 83 + 21 * i; }   // 83, 104, 125, 146
constexpr int kRightLabelX = 441, kYardsValueCx = 536, kParValueCx = 536, kStatValueCx = 591;   // rotating stat and stroke average use 591
constexpr int kYardsY = 83, kRotStatY = 104, kParY = 125, kStrokeAvgY = 146;
constexpr Field histHead{190, 176, Align::Left};        // "Average shots on this hole"
constexpr int kHistCols = 6, kHistCx0 = 445, kHistPitch = 34, kHistLabelY = 168, kHistCountY = 188;
constexpr int histCx(int k) { return kHistCx0 + kHistPitch * k; }
constexpr Field commentsHead{190, 202, Align::Left};
constexpr int kCommentY0 = 220, kCommentPitch = 16, kCommentCx = 400, kMaxComments = 5;
constexpr int commentY(int i) { return kCommentY0 + kCommentPitch * i; }
constexpr int kOkX = 574, kOkDy = 8, kOkW = 44;         // OK at (574, yAfterRows + 8)
}  // namespace holestat

// ---- 4. SGA Evaluation report -------------------------------------------------------------------------------------------------
namespace sga {
constexpr Rect art{35, 26, 730, 419};
constexpr Field title{190, 42, Align::Left};            // "REPORT of the SIM GOLF ASSOCIATION", large face
constexpr Field selectionCriteria{75, 77, Align::Left}, grade{306, 77, Align::Left}, idealHead{497, 77, Align::Left};
constexpr int kRows = 10;
constexpr int rowY(int i) { return 104 + 17 * i; }
constexpr int kLabelCx = 119, kActualCx = 238, kIdealX = 490, kNotAcceptableCx = 386;
constexpr int kPipX0 = 303, kPipPitch = 14, kMaxPips = 10;
constexpr const char* kRowLabels[kRows] = {"Length of Course", "Number of Holes", "Time to Play", "Fun Factor", "Holes with Variety",
                                           "Scenic Holes", "Length Holes", "Accuracy Holes", "Imagination Holes", "Facilities on Site"};
constexpr Field recommendation{174, 288, Align::Left};   // "Committee recommendation"
constexpr Field zeroScore{370, 80, Align::Left};         // "0/100" when total <= 0
constexpr Field totalScore{396, 77, Align::Right};
constexpr Field verdictTitle{400, 310, Align::Center};   // tournament name or "Improvement Required." (third face for the latter)
constexpr Field prizeLine{400, 338, Align::Center};      // "N,000 first prize." third face
constexpr Pt okPos{701, 398};
}  // namespace sga

// ---- 5. Keyboard shortcuts -----------------------------------------------------------------------------------------------------
namespace shortcuts {
constexpr Rect art{0, 0, 800, 409};
constexpr Field title{287, 28, Align::Center};
constexpr Pt okPos{710, 362};
struct Entry { const char* key; const char* action; bool rightColumn; int y; };
constexpr int kLeftKeyCx = 68, kLeftActionX = 108, kRightKeyCx = 444, kRightActionX = 483;
constexpr int kEntries = 31;
// Actions are paraphrased; key texts as shown on screen. y is the text row.
constexpr Entry kList[kEntries] = {
    {"F1", "Course status report", false, 81},          {"F2", "Player comments report", false, 98},
    {"F3", "Histograph", false, 115},                   {"F4", "Financial report", false, 132},
    {"F5", "Course overview map", false, 149},          {"F6", "World map", false, 166},
    {"F7", "SGA evaluation", false, 183},               {"F8", "This list of shortcuts", false, 200},
    {"F9", "Membership roster", false, 217},            {"F10", "Professional accomplishments", false, 234},
    {"?", "Show the last message again", false, 251},   {"shift+b", "Sell a building lot", false, 268},
    {"shift+r", "Routing, aura and lot value screens", false, 285}, {"shift+w", "World map, switch course", false, 302},
    {"Tab", "Rotate the building or tree before placing", false, 319}, {"Esc", "Quit the game", false, 343},
    {"g", "Green and tee tool", true, 81},              {"f", "Fairway tool", true, 98},
    {"r", "Rough tool", true, 115},                     {"s", "Sand trap tool", true, 132},
    {"t", "Trees tool", true, 149},                     {"w", "Water tool", true, 166},
    {"p", "Path tool", true, 183},                      {"b", "Benches tool", true, 200},
    {"z", "Zoom in", true, 217},                        {"x", "Zoom out", true, 234},
    {"shift+p", "Pause or resume", true, 251},          {"shift+t", "Hide or show trees", true, 268},
    {"shift+n", "Toggle name labels", true, 285},       {"e", "Toggle elevation mode", true, 302},
    {"/", "Instant shot analysis", true, 319}};
}  // namespace shortcuts

// ---- 6. Tournament -------------------------------------------------------------------------------------------------------------
namespace tournament {
constexpr Rect headerPiece{0, 0, 800, 106}, rowTall{0, 180, 800, 26}, rowMid{0, 125, 800, 22}, rowThin{0, 224, 800, 18},
    endA{0, 274, 800, 51}, endB{0, 357, 800, 51};
constexpr Field title{320, 16, Align::Center}, name{175, 50, Align::Center}, headFinal{667, 50, Align::Center}, headPrize{700, 50, Align::Center};
constexpr int kHoleCellX0 = 163, kHoleCellPitch = 27, kHeadY = 50, kRowY0 = 77, kTotalCx = 669;
constexpr int holeCx(int h0) { return kHoleCellX0 + kHoleCellPitch * h0 + 13; }
// In-game leader board overlay: frame (0,8,144, rows*22+16), text left x 72, lines y 9, 21, 33, colour kColYellow.
constexpr int kBoardTextX = 72, kBoardY0 = 9, kBoardLinePitch = 12;
}  // namespace tournament

// ---- 7. End of year -----------------------------------------------------------------------------------------------------------
namespace endyear {
constexpr Rect top{187, 40, 429, 176}, strip{187, 291, 429, 15}, bottom{187, 374, 429, 55};
constexpr Field title{406, 55, Align::Center};          // "END of  YEAR: <year>" large face
constexpr Field headThis{465, 93, Align::Center}, headLast{566, 93, Align::Center}, highlights{404, 200, Align::Center};
constexpr int kRowLabelCx = 301, kThisRightX = 500, kLastRightX = 601, kHighlightCx = 404;
constexpr int rowY(int i) { return 115 + 20 * i; }      // cash, fun, skill, membership
constexpr int kOkX = 550;                                // OK at (550, y of the bottom piece)
}  // namespace endyear

// ---- 8. Player comments --------------------------------------------------------------------------------------------------------
namespace comments {
constexpr Rect top{148, 45, 505, 102}, strip{148, 224, 505, 17}, bottom{148, 321, 505, 61};
constexpr Field title{389, 61, Align::Center};
constexpr Field headComments{182, 96, Align::Left}, headHole{504, 96, Align::Center}, headFrequency{598, 96, Align::Center};
constexpr int kMaxRows = 20, kStripY0 = 124, kRowPitch = 15, kTextDy = 2;
constexpr int stripY(int r) { return kStripY0 + kRowPitch * r; }
constexpr int textY(int r) { return stripY(r) + kTextDy; }
constexpr int kTextX = 182, kHoleCx = 504, kFreqCx = 598, kOkX = 591;
}  // namespace comments

// ---- 9. Course overview (routing, value, aura, employees) -------------------------------------------------------------------------
namespace overview {
constexpr Rect bottomPiece{0, 253, 800, 347};
constexpr Rect headerStrip{0, 0, 800, 253};              // drawn from the mode's own sheet at (0,0)
constexpr int kMapOriginX = 0x184, kMapOriginY = 0x68, kTileStep = 8;   // before the iso transform
// Mode buttons (cuts from the upper part of the bottom sheet): idle, selected, destination.
struct ModeButton { const char* name; Rect idle, selected; Pt dst; };
constexpr ModeButton kModes[4] = {
    {"employees", {0, 0, 129, 113}, {0, 114, 129, 113}, {60, 313}},
    {"value", {130, 0, 130, 113}, {130, 114, 130, 113}, {612, 313}},
    {"routing", {261, 0, 134, 117}, {261, 118, 134, 117}, {182, 254}},
    {"aura", {396, 0, 139, 117}, {396, 118, 139, 117}, {483, 254}}};
constexpr Rect okBig{536, 0, 65, 65};  constexpr Pt okBigPos{71, 533};
constexpr Rect arrowUpCut{713, 1, 18, 37}, arrowDownCut{732, 1, 18, 37}, scrollTrackCut{751, 0, 18, 178};
constexpr Pt arrowUpPos{775, 64}, arrowDownPos{775, 205}, trackPos{775, 64};
constexpr Field titleMain{400, 15, Align::Center}, subTitle{400, 85, Align::Center}, lowerTitle{400, 188, Align::Center};
constexpr Field help1{400, 108, Align::Center}, help2{400, 126, Align::Center};
constexpr int kHeadY = 85;
constexpr int kLeftHoleX = 71, kLeftParX = 132, kLeftYdsX = 182, kLeftTimeX = 239;
constexpr int kRightHoleX = 564, kRightParX = 630, kRightYdsX = 680, kRightTimeX = 737;
}  // namespace overview

// ---- 10. Buy Land ------------------------------------------------------------------------------------------------------------------
namespace buyland {
constexpr int kTracts = 9;
constexpr Field title{400, 16, Align::Center};           // "TRACTS FOR SALE"
constexpr Field cashLabel{548, 263, Align::Left}, cashValue{720, 264, Align::Center};
constexpr int kBarX[3] = {38, 295, 560};
constexpr int kBarY[3] = {53, 121, 189};
constexpr Pt ballIdle[kTracts] = {{15, 54}, {15, 122}, {15, 190}, {272, 54}, {272, 122}, {272, 190}, {538, 54}, {538, 122}, {538, 190}};
constexpr int textX(int tract) { return tract >= 6 ? 604 : 82 + 256 * (tract / 3); }   // exe: x0 = 78, 334, 600 then +4 (first two columns 82, 338)
constexpr int textY(int tract) { return 62 + 68 * (tract % 3) + 2; }
// buy_land_buttons.pcx: numbered balls, yellow row at y 1, silver row at y 64, 59x61 each at x = 1 + 60 n.
constexpr Rect ballYellow(int n) { return {1 + 60 * n, 1, 59, 61}; }
constexpr Rect ballSilver(int n) { return {1 + 60 * n, 64, 59, 61}; }
constexpr Rect diamondHighlight{1, 127, 192, 95}, okCut{194, 127, 64, 64}, compassCut{259, 127, 72, 61};
constexpr Pt okPos{662, 533}, compassPos{71, 533};
constexpr int kMapOriginX = 0x180, kMapOriginY = 100, kTileStep = 8, kTractTiles = 16;
}  // namespace buyland

// ---- 11. Hire dialog (full details in ui_panels.h and docs/UI_PANELS.md section 6) -------------------------------------------------
namespace hire {
constexpr Rect art{0, 0, 800, 600};
constexpr Field title{377, 107, Align::Left};
constexpr Pt okPos{553, 422};
constexpr int kLineX = 246, kPortraitX = 576;
constexpr int kPortraitY[4] = {192, 262, 332, 402};
}  // namespace hire

}  // namespace sg::ui_screens

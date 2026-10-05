// sgview: native terrain viewer for the SimGolf port (SDL2 + legacy fixed-function OpenGL).
//
//   sgview --game "<dir>/Program_Files_(ENGLISH)" [--theme parkland|links|desert|tropical]
//          [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--center TX,TY] [--time SECONDS] [--follow] [--png out.png]
//
// Keys: arrows/WASD pan, Q/E rotate, +/- or mouse wheel zoom, 1-4 theme, R new demo course, P toggle scenery, F follow the golfer,
//       F2 screenshot, Esc quit.  Left-drag pans.
// The camera follows the original: ortho +-5000 depth, pitch about X, then yaw 45 degrees.
#ifdef __APPLE__
#define GL_SILENCE_DEPRECATION
#endif
#include <SDL.h>
#include <SDL_opengl.h>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <map>
#include <memory>
#include <set>
#include <array>
#include "sg/bodypal.h"
#include "sg/audio.h"
#include "sg/economy.h"
#include "sg/formats.h"
#include "sg/goals.h"
#include "sg/holes.h"
#include "sg/holestats.h"
#include "sg/comments.h"
#include "sg/reactions.h"
#include "sg/membership.h"
#include "sg/top10.h"
#include "sg/bestscores.h"
#include "sg/sga.h"
#include "sg/buildings.h"
#include "sg/homes.h"
#include "sg/tournament.h"
#include "sg/visitors.h"
#include "sg/properties.h"
#include "ui.h"
#include "sg/shot.h"
#include "sg/ui_panels.h"
#include "sg/ui_screens.h"
#include "sg/ui_screens2.h"
#include "sg/costs.h"
#include "sg/charrec.h"
#include "sg/sprites.h"
#include "sg/terrain.h"

#ifndef GL_GENERATE_MIPMAP
#define GL_GENERATE_MIPMAP 0x8191
#endif
#ifndef GL_CLAMP_TO_EDGE
#define GL_CLAMP_TO_EDGE 0x812F
#endif

using namespace sg;

static const char* kThemes[4] = {"Parkland", "Links", "Desert", "Tropical"};
static int themeExe(int appTheme) { static const int m[4] = {0, 3, 1, 2}; return m[appTheme]; }   // our order Parkland, Links, Desert, Tropical -> exe order Parkland, Desert, Tropical, Links

// A sprite plus its lazily created GL textures (one per view/frame).
struct GlSprite {
    Sprite s;
    std::vector<GLuint> tex;
};

// A placed object. Sprites are billboards anchored on the ground at (x, z).
struct Prop {
    float x, z;
    GlSprite *body = nullptr, *shadow = nullptr;
    bool flat = false;  // ground overlay (building base), drawn under everything else
    int frame = 0;      // frame within a view
    float heading = -1000;  // world heading in degrees (atan2(z, x)); only used by 8 view sprites
    int walker = -1;        // index into App::walkers, -1 for static props
    int golfer = -1;        // index into App::golfers, -1 for other props
    bool hidden = false;    // a pool golfer who is not on the course
    bool tree = false;      // scenery regenerated from the terrain after edits
    bool building = false;  // an amenity placed by the player (kept when props are regenerated)
    int emp = -1;           // index into App::emps for staff sprites
};

// One golfer on the course. A fixed pool is allocated so the routes the simulations point at never move.
struct BodySet;
struct Golfer {
    ShotSim sim;
    const BodySet* bset = nullptr;   // recoloured body sprites (docs/DECODE_BODIES.md), null falls back to the stock looks
    bool active = false;
    int look = 0, hole = 0, strokesRound = 0, lastStroke = -1;
    const char* lastEvent = nullptr;
    std::vector<float> route;
    double holeStart = 0;
    int memberId = 0;            // identity in the club roster (1..75), 0 for none
    sg::Visitor vis = sg::Visitor::None;
    unsigned skillMask = 7;      // PLACEHOLDER mapping from skills to the exe's length/accuracy/imagination bits
    int liked = 0, firstOverHole = 0; bool likedLast = false;
    sg::Reactor rx;              // mood history, speech timer and the needs counters (docs/DECODE_EVENTS_SHOTS.md, DECODE_EVENTS_NEEDS.md)
    unsigned lastTick = 0; int seenPlans = 0, seenLands = 0; float hz = 0; int visitedHole = -1; unsigned visits = 0; int seenObs = 0, clubMask = 0; float planYards = 0; bool leaving = false; int ordinal = 0, leaveWait = 0;   // visits: facilities used this round (4 range, 8 pro shop, 0x10 putting green)   // reaction clock and the shot plan hazard score (PLACEHOLDER formula)
    int greetedHole = -1; bool hurried = false;   // club pro greeted on this hole, ranger has hurried this golfer
    int holeStrokes[18] = {};    // strokes on each finished hole, for the info card scorecard
    int bodyCls = 0;             // body class 0..7 (male PLS, KLS, PSS, SSS; female PLS, SSS, PSS, SkTT) for the emotion clips
    int proIdx = -1;             // index into the pro table for the opponent in a match against a pro
    bool isPlayer = false;       // the player's own character out on a practice round (docs/DECODE_TOURNAMENTS.md section 7)
    int mood = 4;                // the golfer's mood value, which sets the green fee (the exe keeps it per golfer as a small integer)
};
constexpr int kMaxGolfers = 8;
constexpr int kLooks = 4;

struct Walker {
    float dist = 0;   // distance along the path, world units
    float speed = 0;  // world units per second
};

struct App {
    std::string gameDir;
    int theme = 0;
    uint32_t seed = 7;
    Terrain terrain;
    TextureCatalog catalog;
    Lighting light;
    std::map<std::string, GLuint> textures;
    std::map<GLuint, std::vector<Vertex>> batches;
    std::map<GLuint, std::vector<Vertex>> pathBatches;
    std::map<GLuint, std::vector<Vertex>> mudBatches;   // paths not joined to the clubhouse, drawn as mud tracks (manual p. 18)
    HoleInfo hole;
    std::string lastReport;
    std::map<GLuint, std::vector<Vertex>> wallBatches;  // retaining walls
    std::set<GLuint> waterTex;                          // textures that get the shimmer
    Economy econ;  // alpha-blended path overlays
    float camX = 0, camZ = 0, zoom = 0.36f, rot = 0;
    int drawW = 1024, drawH = 768;
    float dpi = 1.0f;  // drawable pixels per window point (2 on Retina)
    std::map<std::string, std::unique_ptr<GlSprite>> sprites;
    std::vector<Prop> props;
    bool showProps = true;
    std::vector<Walker> walkers;
    std::vector<float> pathLen;  // cumulative length of Terrain::path segments
    double time = 0;             // seconds, drives animation
    std::vector<Golfer> golfers;
    std::vector<HoleRoute> holes;                       // found from the painted tees and greens (see sg/holes.h)
    double simTime = 0, spawnTimer = 1e9;
    GlSprite* lookBody[kLooks][6] = {}; GlSprite* lookShadow[kLooks][6] = {};   // [look][GolferAnim]
    bool lookOk[kLooks] = {};
    int roundsStarted = 0;
    int difficulty = 1;              // 0..3 as in the exe (the standard game's value is not known yet; 1 is a guess)
    std::string courseName = "Demo Course";
    // Screens: the title menu, the property chooser and the course itself.
    enum { ScreenMenu, ScreenProperty, ScreenPlay, ScreenReport, ScreenCharacter, ScreenCustomise, ScreenSga, ScreenFinance, ScreenRoster, ScreenHoleStat, ScreenKeys, ScreenEoy, ScreenComments, ScreenHisto, ScreenBoard, ScreenBuyLand, ScreenOverview, ScreenAward, ScreenGolfer, ScreenDiff, ScreenThemes, ScreenLoad, ScreenCredits, ScreenTop10, ScreenPro, ScreenBest, ScreenPair, ScreenStats };
    int screen = ScreenPlay;
    ui::Image titleBase, titleUn, titleMo, worldBase, themeIcons[4], reportArt, holeArt, keysArt, eoyArt, comtArt, sgaArt, tourArt, histArt, boardArt, partsArt, tacsArt, pinArt, finArt, okArt, okRound, rosterArt, rosterBtn, rosterBar, dockArt, terrPanel, terrBtns;
    ui::Image amenArt, elevArt, bldgArt, empArt, layoutArt, hireArt, memberArt, faceArt, joeArt, landArt, landBtn, ovHead[4], ovBottom;   // dock panel sheets (docs/UI_PANELS.md)
    bool amenities = false, elevation = false, hireOpen = false;       // sub panels: Amenities (of Build Course), Elevation (of Add Buildings), the hire dialog
    int elevTool = 0, pHover = -99, pHoverLast = -99, pHoverFrames = 0, empSel = -1, empOff = 0;
    float vmx = -1, vmy = -1;        // mouse in virtual coordinates
    float testHx = -1, testHy = -1;     // --hover test hook
    int terrSlot = 0, terrHover = -1;   // Build Course panel: selected and hovered button slot
    bool showAdvisor = true;     // H toggles the advisor and story banners
    bool pauseToggle = false;    // set by the dock's pause button, handled in the main loop
    int panel = 0;               // open dock panel: 0 none, 1 terrain, 2 buildings, 3 people
    // Hole opening (decoded from the publisher exe, docs/PUBLISHER_EXE_NOTES.md): a finished hole must be opened (H) before golfers play it
    // and before the next hole can be built. `holes` below is the open holes only; `allHoles` is every tee and green pair found.
    std::vector<HoleRoute> allHoles;
    std::vector<long> openKeys;      // opened holes in opening order, keyed by the tee's tile (ty * 4096 + tx)
    int teeBlobs = 0;                // tee clusters on the map (a tee without a green is one that has no pair yet)
    bool autoOpen = true;            // demo and old course files: every hole counts as open
    int unlockLevel = 17;            // the exe's counter: building type b can be built when unlockLevel > b (a normal game starts at 6, sandbox at 17)
    std::string charName = "Gary Golf";   // the player's character (the exe's default name); also the advisor's voice
    bool charFemale = false, charEditing = false;
    sg::CharRec chr, chrUndo; bool chrReady = false;   // the player's character record (Customise screen)
    ui::Image t2Diff, t2DiffMo, t2Theme, t2ThemeMo, t2Load, t2LoadMo, t10Blank, t10Troph, creditsBg, creditsLogo; bool t2Ok = false, top10Ready = false, top10Show = false; int t2Hover = -1, t2Sel = -1, t2Scroll = 0, t2Confirm = 0, diffPending = -1, top10Return = 0; Uint32 creditStart = 0;
    bool cardSkills = false; ui::Image emblem[2], pairBase, pairBtn; std::vector<int> pairIds; unsigned pairSel = 0; int pairHover = -1; std::string pairMsg;
    ui::Image emblemUnused; int t2Go = 0, t2ConfirmHover = -1; double t2HoverSince = 0; int t2HoverPrev = -1;
    ui::Image t2Pro, bestArt, ciArt, ciShade, ibArt;
    // Generic popup menus (FUN_0046d6e0): 1 Information, 2 System Functions, 3 Preferences, 4 text prompt.
    sg::CharRec proPrev; std::string proPrevPath; bool proPrevOk = false;
    int popKind = 0, popHover = -1, popPrefs = 0x25; std::vector<std::string> popLines; std::string popBuf, popHead, lastMsg; bool browsing = false; int popPromptFor = 0; int t2Mode = 0; bool champ = false;
    sg::BestScores best; std::string thumbReq; GLuint t2Thumb = 0;   // Best N Hole Scores for this course; a save thumbnail is captured on the next frame
    struct SaveInfo { bool ok = false; int holes = 0, par = 0, yards = 0, fun = 0, len = 0, acc = 0, img = 0, theme = 0; double cash = 0; std::string designer, themeName; std::vector<std::pair<int, int>> hole; int record = 0; std::string recordBy; int prop = 0; } t2Info; std::string t2InfoPath;   // t2Mode 1 = the Load screen lists championship courses (.cse); champ = championship play (exe flag 0x4000000)
    std::vector<std::string> t2Files, creditLines; sg::Top10 top10;   // title side screens and the Top 10 table
    ui::Image cardArt; int cardG = -1, cardHover = -1, cardFrames = 0; float cardMx = 0, cardMy = 0;   // golfer info card (docs/DECODE_GOLFERCARD.md)
    ui::Image ballArt, cgBtn, custBg[2], headWin, headSel, headExp[2], headHalo[2]; std::vector<ui::Image> headCustom[2];
    int cuHover = -1, cuFrames = 0, cuFacePage = 0, cuFaceHover = -1, cuEdit = -1; bool cuFace = false, cuLoad = false; float cuMx = 0, cuMy = 0; std::string cuBuf; std::vector<std::string> cuFiles;
    int pendingProp = -1, pendingSandbox = 0;
    int siteSlot[16]; bool worldInit = false; int confirmIdx = -1, goTarget = -1, goFrames = 0; std::string worldMsg; double worldMsgUntil = 0; ui::Image worldBtn;   // World map: the price slot of each site (Reset World re-rolls it), the purchase confirm, the "off to" screen and the red message box
    int curProp = -1; unsigned careerOwned = 0; bool switchMode = false;   // World map (F6): the property being played, the properties bought so far, and the in-game switch screen
    sg::GoalTracker tracker;         // the 22 accomplishments (docs/DECODE_SOCIAL.md)
    sg::VisitorState vstate;         // CEO, commissioner and heiress counters
    sg::SocialRng srng{12345};
    sg::Roster roster;               // the 75 golfer identities and their membership tiers
    bool rosterReady = false;
    sg::Tournament tourney;
    std::vector<sg::TourPro> pros; bool prosTried = false;
    std::map<std::string, std::unique_ptr<BodySet>> bodySets;
    std::vector<sg::HoleStats> hstats;   // exe-style per hole statistics, one per open hole
    sg::GoalEvent pendingGoal; bool hasPendingGoal = false;
    int offerMonth = -1, fame = 0, declineWarn = -999;
    sg::SgaInput sgaIn; sg::SgaResult sgaLast; int sgaMode = 0;   // SGA screen: 0 evaluation, 1 offer, 2 results
    sg::TournamentResult tResult; std::vector<sg::Entrant> tField; std::string tName; int tPrize = 0; std::vector<int> tPars; std::map<int, std::vector<int>> tStrokes;
    int vipDay = 0, investors = 0;   // old fields kept for save compatibility
    int courseStage = 0;             // 0 Municipal, 1 Golf Club, 2 Country Club, 3 Championship (the course upgrades when holes 6, 10 and 18 open)
    int dockHover = -1;          // dock button under the mouse (index into kDock), -1 none
    int panelHover = -1;         // list row under the mouse in an open panel
    std::vector<std::string> storyLines;   // lines of a story file from the disc (loaded at run time, never copied into the repo)
    std::string storyTitle;
    size_t storyPos = 0; double storyNext = 0; char storyLetter = 'x'; int storiesDone = 0;
    // Per hole statistics for the course report (reset when the course changes shape).
    struct HoleStat { int plays = 0; double strokes = 0, seconds = 0, revenue = 0, mood = 0; int hist[6] = {}; int moodSum = 0; };   // moodSum: total of the mood changes golfers had on this hole (the exe's per-hole counter)
    std::vector<HoleStat> holeStats;
    std::vector<HoleRating> ratings;
    ui::Font font;
    bool uiOk = false;
    ui::View view;
    int rosterScroll = 0, hsHole = 0, hsRot = 0;
    int awardHole = -1, awardKind = 0; double awardCheck = 0, awardWait[18] = {};   // Top 100 / Top 18 popup state (PLACEHOLDER re-ask delay)
    std::vector<short> hgSkill = std::vector<short>(500, 0), hgCash = std::vector<short>(500, 0), hgFun = std::vector<short>(500, 0), hgStaff = std::vector<short>(500, 0), evLog = std::vector<short>(500, 0);
    int hgLastDay = 1, testBoard = 0;
    struct Mile { long tick = 0; std::string course; GLuint snap = 0; };
    Mile miles[22]; int snapPending = -1;                    // F10 board: the 22 accomplishments, a photo is taken the frame after each is earned
    struct YearRec { long long cash = 0; int fun = 0; double skill = 0; int members = 0; };
    std::vector<YearRec> yearHist;                                   // one record per finished year (End of Year screen)
    std::vector<std::pair<std::string, bool>> highlights;           // this year's notable events, true = bad news
    std::string eoyBoard; int eoyYear = 0, lastYear = 0;
    int hover = -1;                  // button or card under the mouse, -1 none
    int themePack = 0;               // chosen on the title screen; its text and golfers are not used yet
    bool resetClock = false;
    bool sandboxChoice = false;      // the property chooser was opened from Sandbox Mode
    std::string toast;
    double toastUntil = 0; int toastKind = 0;   // 1 = a notice (the dark green message panel of the real game), 0 = a short confirmation
    bool follow = false;         // camera follows the golfer
    // Course editing
    bool edit = false;
    int tool = 0;                // 0 paint, 1 raise (shift lowers), 2 path (shift removes)
    int pathKind = 1;
    int buildIdx = 0;            // selected amenity in kBuild (tool 4)
    struct Placed { int def, tx, ty, lvl = 1; };
    sg::BuildingSystem bsys;
    std::vector<Placed> buildings;
    // Amenities placed on single tiles. kind: 0 bench, 1 flower bed, 2 scenic tree, 3 scenic bridge, 4 ball washer, 5 landmark (var = design).
    struct Amen { int kind, tx, ty, var; };
    std::vector<Amen> amen;
    // Home sites (docs/DECODE_HOMES.md): value V climbs toward 12 times the lot value, celebrities buy the sites that pass the sale test.
    struct Home { int x, y, buyer = 0, value = 0; };
    std::vector<Home> homes; int homeSales = 0, homeMonth = -1, homeConfirm = -1; double homeConfirmUntil = 0;
    std::vector<sg::Celebrity> celebs;
    // Land (docs/UI_SCREENS2.md section 4): a 50 x 50 map is cut into nine 16 x 16 tracts; ownMask has a bit per tract. Courses that are not 50 x 50
    // (older saves, editor files) have no land model and everything is owned.
    struct Emp { int kind = 0; bool skilled = false; float x = 0, z = 0, heading = 0; int target = -1; int phase = 0; float timer = 0, cool = 0; float wx = 0, wz = 0; std::string name; int hired = 1; int n = 0; };   // n: what this employee has done (greeted, hurried, sold)
    int empMoveArm = -1, empRenameRow = -1;
    // Shot Analysis (the Analyze Golf Shot tool and the / key): sample first shots of golfers with all skills and without one group, drawn on the hole.
    bool anArm = false; int anHole = -1; std::vector<float> anPath[4][5]; float anYds[4] = {}; float anLand[4][5][2] = {};   // Move armed for a roster row (the next map click places it), the row being renamed
    std::vector<Emp> emps; int empCount[4][2] = {};   // staff walking the course, and the counters they earn (greeted/cheered and so on)
    int ovMode = -1, ovSel = -1; std::vector<float> ovAura, ovValue;   // course overview (F5): -1 routing, 1 aura, 2 home site value, 3 employees
    bool landModel = false; int ownMask = 0x1ff; int landBought = 0; bool landOffer = false;
    int blSel = -1, blPrice[9] = {}, blSale[9] = {}; bool blRolled = false; std::string blNote;
    int matchPro = -1, matchWager = 0, matchMonth = -1, matchPl = -1, matchOp = -1, matchLead = 0, matchDoneN = 0; bool matchOn = false, matchDone[18] = {};   // a pro's challenge, the wager per hole (units of $100), the two golfer slots and the holes decided so far
    int tutPage = -1; bool tutWasPaused = false;   // the tutorial: eleven pages on fun and nine on skill (structure from the exe, wording is my own); -1 when closed
    double tRevealStart = -1e9;   // tournament results reveal hole by hole (a presentation of the precomputed rounds, PLACEHOLDER for live play)
    bool showNames = true;   // name tags over golfers and employees, as in the real screenshots (Shift+N toggles, PLACEHOLDER key)
    bool playerPanel = false; int shotShape = 0, plHover = -1;   // the Player (JoeCool) panel behind the golfers tab with the player's name, and the shot oval picked (0 straight, 1 fade, 2 draw, 3 backspin, 4 punch)
    bool golfersMode = true;     // People dock: Golfers list (true) or the Employee overlay (false)
    int golfOff = 0;             // scroll offset of the Golfers list
    int followG = -1;            // golfer slot the camera follows (-1 none)
    int amenTool = 1;            // exe tool id armed on the Amenities panel (tool 5): 3 ball washer, 1 bench, 4 landmark, 2 flowers, 16 tree, 19 bridge
    int amenVar[5] = {0, 0, 0, 0, 0};   // chosen design per strip: bench, flower bed, scenic tree, scenic bridge, landmark
    unsigned amenCounter = 0;    // the exe's running selection counter, used for the variant rolls
    int raiseSign = 1;           // = selects raising, - selects lowering (the original's hotkeys); shift flips it
    bool paused = false; int hearts = 0; ui::Image shArt;
    bool skOpen = false, skConfirm = false, skWasPaused = false; int skLeft = 0, skHover = -1, skConfHover = -1; uint8_t skStart[10] = {};   // editable skills dialog (FUN_0045f0f0 with points to spend)
    int wallDir = 0;             // edge chosen under the mouse, for the wall tool            // 1 gravel, 2 paved
    int paintIdx = 0;
    int brush = 0;               // radius in tiles
    bool hasHit = false;
    float hitX = 0, hitZ = 0;    // ground point under the mouse
    int lastCell = -1;           // last tile/corner edited during a drag
    bool dirty = false;
    GLdouble mv[16] = {};
    float upp = 1;               // world units per drawable pixel (last render)
    std::string courseFile = "course.sgc";
    GolferSkills skills;
    // Sound
    std::unique_ptr<Mixer> mixer;
    AudioDevice audioDev;
    bool mute = false, soundLog = false, musicOn = false;
    int ambience = -1, music = -1;
    size_t musicIdx = 0;
};

static long holeKey(const App& app, const HoleRoute& r) {
    const int tx = (int)std::floor((r.teeX + app.terrain.w * kTileSize * 0.5f) / kTileSize), ty = (int)std::floor((r.teeZ + app.terrain.h * kTileSize * 0.5f) / kTileSize);
    return (long)ty * 4096 + tx;
}
static int countTeeBlobs(const Terrain& t) {
    std::vector<uint8_t> seen((size_t)t.w * t.h, 0); int n = 0;
    for (int y = 0; y < t.h; y++) for (int x = 0; x < t.w; x++) {
        const size_t i = (size_t)t.tileIndex(x, y);
        if (seen[i] || t.type[i] != TT_Tee) continue;
        std::vector<int> st{(int)i}; seen[i] = 1; int tiles = 0;
        while (!st.empty()) {
            const int c = st.back(); st.pop_back(); tiles++;
            for (int dy = -1; dy <= 1; dy++) for (int dx = -1; dx <= 1; dx++) {
                const int nx = c % t.w + dx, ny = c / t.w + dy;
                if (nx < 0 || ny < 0 || nx >= t.w || ny >= t.h) continue;
                const size_t j = (size_t)t.tileIndex(nx, ny);
                if (!seen[j] && t.type[j] == TT_Tee) { seen[j] = 1; st.push_back((int)j); }
            }
        }
        if (tiles >= 1) n++;
    }
    return n;
}
// Finds every tee and green pair and keeps the open ones in `holes` (what golfers play and what the report counts).
static void refreshHoles(App& app) {
    app.allHoles = findHoles(app.terrain);
    app.teeBlobs = countTeeBlobs(app.terrain);
    std::vector<long> keys;
    for (const HoleRoute& r : app.allHoles) keys.push_back(holeKey(app, r));
    if (app.autoOpen) app.openKeys = keys;
    else app.openKeys.erase(std::remove_if(app.openKeys.begin(), app.openKeys.end(), [&](long k) { return std::find(keys.begin(), keys.end(), k) == keys.end(); }), app.openKeys.end());
    app.holes.clear();
    for (long k : app.openKeys)
        for (size_t i = 0; i < keys.size(); i++) if (keys[i] == k) { app.holes.push_back(app.allHoles[i]); break; }
}

static void snd(App& app, const char* rel, float vol = 1.0f, bool loop = false, int* voiceOut = nullptr) {
    if (!app.mixer || app.mute) return;
    int id = app.mixer->play(rel, vol, loop);
    if (voiceOut) *voiceOut = id;
    if (app.soundLog) std::printf("  sound: %s%s\n", rel, id < 0 ? "  (missing)" : "");
}

static void startAmbience(App& app) {
    if (app.ambience >= 0 && app.mixer) app.mixer->stop(app.ambience);
    app.ambience = -1;
    snd(app, "GolfAmbience122.wav", 0.22f, true, &app.ambience);
}

static void toggleMusic(App& app) {
    if (!app.mixer) return;
    if (app.music >= 0) app.mixer->stop(app.music);
    app.music = -1;
    app.musicOn = !app.musicOn;
    if (!app.musicOn) return;
    static const char* kFolders[4] = {"music/misc_music/", "music/links_music/", "music/desert_music/", "music/tropical_music/"};
    auto tracks = app.mixer->list(kFolders[app.theme]);
    if (tracks.empty()) return;
    snd(app, tracks[app.musicIdx++ % tracks.size()].c_str(), 0.35f, true, &app.music);
}

struct PaintEntry { const char* name; int type; int vbyte; };
static const PaintEntry kPaint[] = {
    {"Fairway", 2, 0}, {"Firm fairway", 3, 0}, {"Green", 1, 0}, {"Tee", 0, 0}, {"Rough", 4, 0}, {"Deep rough", 5, 0},
    {"Woods", 13, 0}, {"Sand bunker", 7, 0}, {"Pot bunker", 9, 0}, {"Water shallow", 17, 0}, {"Water middle", 17, 1},
    {"Water deep", 17, 2}, {"Rock", 12, 0}, {"Brush", 11, 0}, {"Building lot", 22, 0},
    {"Cliff (editor id)", 31, 0}, {"Ravine (editor id)", 32, 0}, {"Flower bed (editor id)", 33, 0}, {"Zen sand (editor id)", 34, 0}, {"Grass bunker (editor id)", 35, 0},
    {"Waste bunker", 8, 0}, {"Stream", 10, 0}};
static const int kPaintCount = (int)(sizeof kPaint / sizeof kPaint[0]);

// Original pitch angles, chosen per resolution in Terrain::initSystem (38.68, 40.54, 40.83 degrees).
static double pitchFor(int w, int h) {
    if (w == 800 && h == 600) return 38.682186;
    if (w == 1280 && h == 1024) return 40.832218;
    return 40.541603;
}

namespace up = sg::ui_panels;

static GLuint textureFor(App& app, const std::string& path) {
    auto it = app.textures.find(path);
    if (it != app.textures.end()) return it->second;
    Bytes d; Rgba img; std::string err;
    GLuint id = 0;
    const bool tga = path.size() > 4 && strcasecmp(path.c_str() + path.size() - 4, ".tga") == 0;
    if (readFile(path, d) && (tga ? decodeTga(d, img, err) : decodeBmp(d, img, err))) {
        glGenTextures(1, &id);
        glBindTexture(GL_TEXTURE_2D, id);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_GENERATE_MIPMAP, GL_TRUE);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    }
    app.textures[path] = id;
    return id;
}

// Path overlay (our own model, see docs/PATHS.md): every path tile gets a centre piece, plus an arm toward each path
// neighbour, cut out of the game's cross shaped Path.tga (arms 0..0.33 / 0.66..1 of the tile, centre between).
static void buildPaths(App& app) {
    app.pathBatches.clear();
    app.mudBatches.clear();
    const Terrain& t = app.terrain;
    if (t.pathKind.empty()) return;
    const std::vector<uint8_t> connected = pathsConnectedToClubhouse(t);
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    auto vert = [&](float wx, float wz, float u, float v) {
        float y = t.heightAt(wx, wz) + 1.5f;
        float e = 12.0f;
        float nx = t.heightAt(wx - e, wz) - t.heightAt(wx + e, wz), nz = t.heightAt(wx, wz - e) - t.heightAt(wx, wz + e), ny = 2 * e;
        float l = std::sqrt(nx * nx + ny * ny + nz * nz);
        return Vertex{wx, y, wz, u, v, nx / l, ny / l, nz / l};
    };
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++) {
            int kind = t.pathAt(tx, ty);
            if (!kind) continue;
            const bool N = t.pathAt(tx, ty - 1), S = t.pathAt(tx, ty + 1), W = t.pathAt(tx - 1, ty), E = t.pathAt(tx + 1, ty);
            const int links = N + S + W + E;
            const char* file = kind == 2 ? "PathX.tga" : "Path.tga";
            const char* disc = kind == 2 ? "PathCurveX.tga" : "PathCurve.tga";
            // Rectangles in tile fractions {u0,v0,u1,v1}; piece textures: cross for arms and busy centres, disc for ends and bends.
            struct Piece { float r[4]; bool useDisc; };
            std::vector<Piece> pieces;
            const float a = 0.33f, b = 0.67f;
            const bool straight = (N && S && !W && !E) || (W && E && !N && !S);
            if (links >= 3 || straight) pieces.push_back({{a, a, b, b}, false});
            else pieces.push_back({{0.22f, 0.22f, 0.78f, 0.78f}, true});
            if (N) pieces.push_back({{a, 0, b, a}, false});
            if (S) pieces.push_back({{a, b, b, 1}, false});
            if (W) pieces.push_back({{0, a, a, b}, false});
            if (E) pieces.push_back({{b, a, 1, b}, false});
            for (const Piece& p : pieces) {
                GLuint tex = textureFor(app, app.gameDir + "/Data/Textures/" + kThemes[app.theme] + "/" + (p.useDisc ? disc : file));
                auto& batch = (connected[(size_t)t.tileIndex(tx, ty)] ? app.pathBatches : app.mudBatches)[tex];
                const int D = 2;
                for (int j = 0; j < D; j++)
                    for (int i = 0; i < D; i++) {
                        float u0 = p.r[0] + (p.r[2] - p.r[0]) * i / D, u1 = p.r[0] + (p.r[2] - p.r[0]) * (i + 1) / D;
                        float v0 = p.r[1] + (p.r[3] - p.r[1]) * j / D, v1 = p.r[1] + (p.r[3] - p.r[1]) * (j + 1) / D;
                        // Disc pieces map the whole texture onto the rectangle.
                        auto tu = [&](float u) { return p.useDisc ? (u - p.r[0]) / (p.r[2] - p.r[0]) : u; };
                        auto tv = [&](float v) { return p.useDisc ? (v - p.r[1]) / (p.r[3] - p.r[1]) : v; };
                        auto V = [&](float u, float v) { return vert(ox + (tx + u) * kTileSize, oz + (ty + v) * kTileSize, tu(u), tv(v)); };
                        Vertex q00 = V(u0, v0), q10 = V(u1, v0), q11 = V(u1, v1), q01 = V(u0, v1);
                        for (const Vertex* vv : {&q00, &q10, &q11, &q00, &q11, &q01}) batch.push_back(*vv);
                    }
            }
        }
}

// Retaining walls: PLACEHOLDER look, a vertical strip standing on the tile edge, kWallHeight units tall, plus a thin cap.
static void buildWalls(App& app) {
    app.wallBatches.clear();
    const Terrain& t = app.terrain;
    if (t.wallMask.empty()) return;
    const float kWallHeight = 30.0f, kCap = 5.0f;
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    GLuint tex = textureFor(app, app.gameDir + "/Data/Textures/" + kThemes[app.theme] + "/RetainingWallA.bmp");
    auto& batch = app.wallBatches[tex];
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++)
            for (int dir = 0; dir < 4; dir++) {
                if (!t.wallAt(tx, ty, dir)) continue;
                // A shared edge is drawn by the north / west tile only, unless it lies on the map border.
                static const int dx[4] = {0, 1, 0, -1}, dy[4] = {-1, 0, 1, 0};
                if ((dir == 1 || dir == 2) && t.wallAt(tx + dx[dir], ty + dy[dir], (dir + 2) & 3)) continue;
                float x0, z0, x1, z1, ix = 0, iz = 0;   // edge endpoints, and the inward direction for the cap
                const float X = ox + tx * kTileSize, Z = oz + ty * kTileSize;
                if (dir == 0) { x0 = X; z0 = Z; x1 = X + kTileSize; z1 = Z; iz = kCap; }
                else if (dir == 2) { x0 = X; z0 = Z + kTileSize; x1 = X + kTileSize; z1 = z0; iz = -kCap; }
                else if (dir == 3) { x0 = X; z0 = Z; x1 = X; z1 = Z + kTileSize; ix = kCap; }
                else { x0 = X + kTileSize; z0 = Z; x1 = x0; z1 = Z + kTileSize; ix = -kCap; }
                const int N = 4;
                for (int i = 0; i < N; i++) {
                    float a = (float)i / N, b = (float)(i + 1) / N;
                    float xa = x0 + (x1 - x0) * a, za = z0 + (z1 - z0) * a, xb = x0 + (x1 - x0) * b, zb = z0 + (z1 - z0) * b;
                    float ya = t.heightAt(xa, za), yb = t.heightAt(xb, zb);
                    float nx = dir == 3 ? -1.f : dir == 1 ? 1.f : 0.f, nz = dir == 0 ? -1.f : dir == 2 ? 1.f : 0.f;
                    Vertex p0{xa, ya, za, a, 0.3f, nx, 0, nz}, p1{xb, yb, zb, b, 0.3f, nx, 0, nz};
                    Vertex p2{xb, yb + kWallHeight, zb, b, 0.0f, nx, 0, nz}, p3{xa, ya + kWallHeight, za, a, 0.0f, nx, 0, nz};
                    for (const Vertex* q : {&p0, &p1, &p2, &p0, &p2, &p3}) batch.push_back(*q);
                    // cap
                    Vertex c0{xa, ya + kWallHeight, za, a, 0.3f, 0, 1, 0}, c1{xb, yb + kWallHeight, zb, b, 0.3f, 0, 1, 0};
                    Vertex c2{xb + ix, yb + kWallHeight, zb + iz, b, 0.4f, 0, 1, 0}, c3{xa + ix, ya + kWallHeight, za + iz, a, 0.4f, 0, 1, 0};
                    for (const Vertex* q : {&c0, &c1, &c2, &c0, &c2, &c3}) batch.push_back(*q);
                }
            }
}

static void rebuildBatches(App& app) {
    app.batches.clear();
    app.waterTex.clear();
    app.terrain.desert = app.theme == 2;   // the original swaps shallow water for its desert variant in this theme
    for (int y = 0; y < app.terrain.h; y++)
        for (int x = 0; x < app.terrain.w; x++) {
            std::vector<TileTri> tris;
            buildTileTriangles(app.terrain, x, y, tris);
            for (const TileTri& t : tris) {
                const std::string* path = app.catalog.pick(t.texType, t.set, t.variation);
                GLuint texId = path ? textureFor(app, *path) : 0;
                if (texId && (t.texType == TT_WaterShallow || t.texType == TT_Marsh || t.texType == TT_WaterMiddle || t.texType == TT_WaterDeep || t.texType == TT_WaterShallowDesert)) app.waterTex.insert(texId);
                auto& batch = app.batches[texId];
                for (const Vertex& v : t.v) batch.push_back(v);
            }
        }
    buildPaths(app);
    buildWalls(app);
}

static GlSprite* spriteFor(App& app, const std::string& rel, bool shadow, const std::string& pal = std::string()) {
    const std::string path = app.gameDir + "/Flics/" + rel;
    auto it = app.sprites.find(path + "|" + pal);
    if (it != app.sprites.end()) return it->second.get();
    auto gs = std::make_unique<GlSprite>();
    std::string err;
    const std::string key = path + "|" + pal;
    if (!loadSprite(path, gs->s, err, shadow, pal.empty() ? pal : app.gameDir + "/Flics/" + pal)) {
        if (!shadow) std::fprintf(stderr, "sprite: %s\n", err.c_str());  // many objects simply have no shadow file
        app.sprites[key] = nullptr;
        return nullptr;
    }
    gs->tex.assign(gs->s.frames.size(), 0);
    return (app.sprites[key] = std::move(gs)).get();
}

static GlSprite* spriteForRaw(App& app, const std::string& rel, bool shadow, const std::string& key, const uint8_t* pal) {
    const std::string path = app.gameDir + "/Flics/" + rel, k = path + "|raw" + key;
    auto it = app.sprites.find(k);
    if (it != app.sprites.end()) return it->second.get();
    auto gs = std::make_unique<GlSprite>(); std::string err;
    if (!loadSprite(path, gs->s, err, shadow, std::string(), shadow ? nullptr : pal)) { std::fprintf(stderr, "sprite raw: %s\n", err.c_str()); app.sprites[k] = nullptr; return nullptr; }
    gs->tex.assign(gs->s.frames.size(), 0);
    return (app.sprites[k] = std::move(gs)).get();
}
struct BodySet { GlSprite* body[6] = {}; GlSprite* shadow[6] = {}; bool ok = false; };
// One recoloured golfer: the composed palette is applied to the six animations the port plays. Shadows are shared and uncoloured.
static const BodySet* bodySetFor(App& app, const sg::BodyLook& l) {
    const std::string key = l.key();
    auto it = app.bodySets.find(key);
    if (it != app.bodySets.end()) return it->second->ok ? it->second.get() : nullptr;
    auto bs = std::make_unique<BodySet>(); uint8_t pal[768];
    if (sg::composeBodyPalette(app.gameDir, l, pal)) {
        static const char* kAnim[6] = {"_NormalWalk", "_NormalAddress", "_PerfectSwing", "_PuttAddress", "_Putt", "_Happy"};
        const std::string dir = l.female ? "Female/" : "Male/", set = sg::bodySetName(l);
        bs->ok = true;
        for (int i = 0; i < 6; i++) {
            const std::string anim = (i == 2 && l.female) ? "_NormalSwing" : kAnim[i];
            bs->body[i] = spriteForRaw(app, dir + set + anim + ".flc", false, key, pal);
            bs->shadow[i] = spriteFor(app, dir + set + anim + "Shadow.flc", true);
            if (!bs->body[i]) bs->ok = false;
        }
    }
    if (!bs->ok) std::fprintf(stderr, "bodyset %s failed (gameDir %s)\n", key.c_str(), app.gameDir.c_str());
    const BodySet* r = bs->ok ? bs.get() : nullptr;
    app.bodySets[key] = std::move(bs);
    return r;
}
static void assignBody(App& app, Golfer& g, int spin);
static GLuint spriteTexture(GlSprite& g, int view, int frame) {
    size_t i = (size_t)(view % g.s.views) * g.s.framesPerView + (size_t)(frame % g.s.framesPerView);
    if (!g.tex[i]) {
        const Rgba& img = g.s.frames[i];
        glGenTextures(1, &g.tex[i]);
        glBindTexture(GL_TEXTURE_2D, g.tex[i]);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    }
    return g.tex[i];
}

namespace {
struct Rng {
    uint32_t s;
    uint32_t next() { s ^= s << 13; s ^= s >> 17; s ^= s << 5; return s; }
    float unit() { return (next() & 0xFFFFFF) / float(0x1000000); }
    int range(int n) { return (int)(next() % (uint32_t)n); }
};
}  // namespace

static void addTrees(App& app) {
    static const char* kTrees[4][6] = {
        {"Trees/TreeMapleLarge", "Trees/TreeMapleMedium", "Trees/TreePineLarge", "Trees/TreePineMedium", "Trees/TreePineFirLg", "Trees/TreeMapleSmall"},
        {"Trees/Links/LinksPine_Tall", "Trees/Links/LinksPine_Med", "Trees/Links/LinksTree3_Tall", "Trees/Links/LinksTree3_Med", "Trees/Links/LinksTree4_Tall", "Trees/Links/LinksTree4_Med"},
        {"Trees/Desert/JoshuaTree_Lg", "Trees/Desert/JoshuaTree_Md", "Trees/Desert/CactusA_Lg", "Trees/Desert/CactusB_Md", "Trees/Desert/TreeCactusLg", "Trees/Desert/CactusC_Lg"},
        {"Trees/Tropic/TreePalm/TreePalmLg", "Trees/Tropic/TreePalm/TreePalmMed", "Trees/Tropic/Tree_Cerc/Cerc_Large", "Trees/Tropic/Tree_Drac/Drac_Large", "Trees/Tropic/Tree_Tall_Palm/TallPalm_Large", "Trees/Tropic/Tree_Cerc/Cerc_Med"}};
    const Terrain& t = app.terrain;
    auto tileCentre = [&](int tx, int ty, float& x, float& z) { x = tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f; z = ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f; };
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++) {
            if (t.type[(size_t)t.tileIndex(tx, ty)] != 13) continue;
            Rng rng{app.seed * 2654435761u + (uint32_t)(ty * 64 + tx) * 40503u + 12345u};
            rng.next(); rng.next();
            int n = 1 + rng.range(2);
            for (int k = 0; k < n; k++) {
                const char* base = kTrees[app.theme][rng.range(6)];
                Prop p;
                p.tree = true;
                tileCentre(tx, ty, p.x, p.z);
                p.x += (rng.unit() - 0.5f) * 80; p.z += (rng.unit() - 0.5f) * 80;
                // Colour variants live in separate palette files next to the sprites.
                std::string b = base, pal;
                if (b.find("Tropic/TreePalm/") != std::string::npos) pal = "Trees/Tropic/TreePalm/PalGreenPalm.pcx";
                else if (b.find("Tree_Cerc") != std::string::npos) pal = "Trees/Tropic/Tree_Cerc/PalCerc.pcx";
                else if (b.find("Tree_Drac") != std::string::npos) pal = "Trees/Tropic/Tree_Drac/PalDrac.pcx";
                else if (b.find("Tree_Tall_Palm") != std::string::npos) pal = "Trees/Tropic/Tree_Tall_Palm/PalTallPalm.pcx";
                p.body = spriteFor(app, std::string(base) + ".flc", false, pal);
                p.shadow = spriteFor(app, std::string(base) + "Shadow.flc", true);
                if (!p.body) continue;
                p.frame = p.body->s.framesPerView - 1;  // fully grown
                app.props.push_back(p);
            }
        }
}

static void refreshTrees(App& app) {
    app.props.erase(std::remove_if(app.props.begin(), app.props.end(), [](const Prop& p) { return p.tree; }), app.props.end());
    // addTrees appends; keep trees first so painter's order ties stay as before.
    size_t before = app.props.size();
    addTrees(app);
    std::rotate(app.props.begin(), app.props.begin() + before, app.props.end());
}

// Amenities. Sprite names are the level 1 files found per theme in Flics/Bldgs; costs are PLACEHOLDERS (the exe's building costs are not decoded).
// A Snack Bar visit pays 5 units, which is the exe's figure for building type 7 (assumed to be the snack bar).
struct BuildDef { const char* name; const char* sprite[4]; int cost; int visit; int unlock; const char* sprite2[4]; };   // unlock: the exe's building type index; sprite2: the level 2 art (null = reuse level 1)
static const BuildDef kBuild[] = {
    {"Snack Bar",    {"Bldgs/Park/ParkSnackL1", nullptr, nullptr, "Bldgs/Tropical/TROPsnackL1"}, 150, 5, 7, {"Bldgs/Park/ParkSnackL2", nullptr, "Bldgs/Desert/DESsnackL2", "Bldgs/Tropical/TROPSnackL2"}},
    {"Pro Shop",     {"Bldgs/Park/ProsL1", nullptr, "Bldgs/Desert/dproL1", "Bldgs/Tropical/TROPproshopL1"}, 200, 0, 8, {"Bldgs/Park/ProsL2", nullptr, "Bldgs/Desert/DESproL2", nullptr}},
    {"Cart Garage",  {"Bldgs/Park/cartL1", "Bldgs/links/Cart_garageL1", "Bldgs/Desert/DEScartL1", "Bldgs/Tropical/TROPcartL1"}, 400, 0, 11, {"Bldgs/Park/cartL2", "Bldgs/links/Cart_garageL2", "Bldgs/Desert/DEScartL2", "Bldgs/Tropical/TROPcartL2"}},
    {"Hotel",        {"Bldgs/Park/HotelL1", "Bldgs/links/HotelL1", "Bldgs/Desert/DesHotelL1", "Bldgs/Tropical/TROPhotelL1"}, 2500, 0, 13, {"Bldgs/Park/HotelL2", "Bldgs/links/HotelL2", nullptr, "Bldgs/Tropical/TROPhotelL2"}},
    {"Tennis Court", {"Bldgs/Park/tenL1", nullptr, "Bldgs/Desert/tenL1", nullptr}, 300, 0, 9, {"Bldgs/Park/tenL2", nullptr, "Bldgs/Desert/tenL2", nullptr}},   // matched to the Swim Club slot (9) by guess
    {"Marina",       {"Bldgs/Park/MarL1", nullptr, nullptr, "Bldgs/Tropical/TROPmarL1"}, 1000, 0, 12, {"Bldgs/Park/MarL2", nullptr, nullptr, "Bldgs/Tropical/TROPmarinaL2"}},
    {"Putting Green", {"Bldgs/Park/puttL1", "Bldgs/links/PG_hutL1", "Bldgs/Desert/dputtL1", "Bldgs/Tropical/TROPPuttingGL1"}, 100, 4, 6, {"Bldgs/Park/puttL2", "Bldgs/links/PG_hutL2", "Bldgs/Desert/DESputtL2", "Bldgs/Tropical/TROPPuttingGL2"}},   // PLACEHOLDER art: only the main piece of each multi part sprite
    {"Driving Range", {"Bldgs/Park/buildDRL1", "Bldgs/links/DR_shop", "Bldgs/Desert/dbuildDRL1", "Bldgs/Tropical/TROPDrivingRshopL1"}, 250, 8, 10, {"Bldgs/Park/buildDRL2", "Bldgs/links/DRL2_shop", "Bldgs/Desert/DESdrL2house", "Bldgs/Tropical/TROPDrivingRshopL2"}},
    {"Airstrip",     {"Bldgs/Park/airL1", nullptr, nullptr, nullptr}, 5000, 0, 14, {"Bldgs/Park/airL2", nullptr, nullptr, nullptr}},
};
static const int kBuildCount = (int)(sizeof kBuild / sizeof kBuild[0]);
static bool themeHas(const App& app, int d) { return kBuild[d].sprite[app.theme] != nullptr; }
static bool buildUnlocked(const App& app, int d) { return app.econ.sandbox || app.unlockLevel > kBuild[d].unlock; }
static bool buildAvailable(const App& app, int d) { return themeHas(app, d) && buildUnlocked(app, d); }
// The exe's building type names, in its table order (index = unlock level).
static const char* kTypeNames[15] = {"Pathway", "Benches", "Flower Bed", "Ball Washer", "Landmark", "Home Site", "Putting Green", "Snack Bar", "Pro Shop", "Swim Club", "Driving Range", "Cart Garage", "Marina", "Resort Hotel", "Airstrip"};
static const char* kCourseStage[4] = {"Municipal", "Golf Club", "Country Club", "Championship"};

static void addBuildingProp(App& app, const App::Placed& b) {
    const Terrain& t = app.terrain;
    Prop p;
    p.building = true;
    p.x = b.tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f;
    p.z = b.ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f;
    const char* base = (b.lvl >= 2 && kBuild[b.def].sprite2[app.theme]) ? kBuild[b.def].sprite2[app.theme] : kBuild[b.def].sprite[app.theme];
    if (!base) return;
    p.body = spriteFor(app, std::string(base) + ".flc", false);
    p.shadow = spriteFor(app, std::string(base) + "Shadow.flc", true);
    if (p.body) app.props.push_back(p);
}


// Facility effects on the shot simulation (docs/DECODE_BUILDINGS.md section 1): Pro Shop divides the error spread of accurate golfers by (E + 2),
// Driving Range adds 15 units of carry per level for long hitters, Cart Garage speeds golfers up. Units are PLACEHOLDER: the spread is taken relative to
// a level 0 shop, 15 range units as 100 world units, and the cart pace as 1 + E / 2.
static bool washerNearTee(const App& app, const std::vector<float>& route) {   // a ball washer within 0xc00 units (3 tiles) of the tee
    if (route.size() < 2) return false;
    for (const App::Amen& m : app.amen) if (m.kind == 4 && std::hypot((m.tx + 0.5f) * sg::kTileSize - route[0], (m.ty + 0.5f) * sg::kTileSize - route[1]) < 3.0f * sg::kTileSize) return true;
    return false;
}
static void applyFacilityEffects(const App& app, sg::ShotSim& sim) {
    sim.spreadDivisor = (float)app.bsys.accuracyDivisor() / 2.0f;
    sim.driveBonus = (float)app.bsys.driveBonus() * (100.0f / 15.0f);
    sim.paceScale = 1.0f;
    if (app.bsys.cartsActive()) sim.paceScale = 1.0f + 0.5f * (float)((app.bsys.cartSpeed() - 8) / 4);
}

// ---- Land ownership (tracts) ----
static int tractOfTile(int x, int y) { if (x < 1 || y < 1 || x > 48 || y > 48) return -1; return ((y - 1) / 16) * 3 + (x - 1) / 16; }
static bool isOwned(const App& app, int x, int y) {
    if (!app.landModel) return true;
    const int t = tractOfTile(x, y);
    return t >= 0 && ((app.ownMask >> t) & 1);
}

// ---- Amenities on single tiles (benches, flower beds, scenic trees, scenic bridge, ball washers, landmarks) ----
// Which picture belongs to which design number is not recorded, so the order of these lists is the port's own (PLACEHOLDER).
static const char* kBenchArt[5] = {"Flowers/Iron Bench", "Flowers/red bench", "Flowers/round wood bench", "Flowers/woodplank bench", "Flowers/lovers bench"};
static const char* kFlowerArt[5] = {"Flowers/Flowers_Single", "Flowers/WalledFlowers_Single", "Flowers/SinFlowers_Single", "Flowers/TropFB_Single", "Flowers/TropFlowers_Single"};
static const char* kTreeArt[7] = {"Trees/WillowTree", "Scenic/Aspen", "Scenic/Dogwood", "Scenic/Cypress", "Scenic/BlackPine", "Scenic/Bamboo", "Scenic/Bougainvillea"};
static const char* kLandmarkArt[19] = {"Landmarks/Pagoda", "Landmarks/Windmill", "Landmarks/Buddha", "Landmarks/Chapel", "Landmarks/LighthouseB", "Landmarks/Sundial", "Landmarks/barn",
    "Landmarks/civilwarstatue", "Landmarks/equestrian", "Landmarks/gargoyle", "Landmarks/pyramid", "Landmarks/volcano", "Landmarks/Radio Tower", "Landmarks/Gold",
    // Kinds 14 and 15 and the ugly kinds 16 to 18: which picture is which is not recorded (PLACEHOLDER picks from the Landmarks folder).
    "Landmarks/Rock Garden", "Landmarks/Easter", "Landmarks/Red Oil Pump", "Landmarks/TarPit", "Landmarks/Railroad Tracks"};
static const char* amenArt(const App::Amen& m) {
    switch (m.kind) {
        case 0: return kBenchArt[std::clamp(m.var, 0, 4)];
        case 1: return kFlowerArt[std::clamp(m.var, 0, 4)];
        case 2: return kTreeArt[std::clamp(m.var, 0, 6)];
        case 3: return "Scenic/ScenicBridge";
        case 4: return "Bldgs/upgrade bwasher";
        default: return kLandmarkArt[std::clamp(m.var, 0, 18)];
    }
}
static void addAmenityProps(App& app) {
    const Terrain& t = app.terrain;
    for (const App::Amen& m : app.amen) {
        Prop p; p.building = true; p.flat = m.kind == 1;
        p.x = m.tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f;
        p.z = m.ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f;
        const std::string base = amenArt(m);
        p.body = spriteFor(app, base + ".flc", false);
        p.shadow = spriteFor(app, base + "Shadow.flc", true);
        if (m.kind == 3) p.heading = (float)(m.var % 8) * 45.0f - 180.0f;
        if (p.body) app.props.push_back(p);
    }
}

// Home site art (docs/DECODE_HOMES.md section 3.2): a for-sale lot shows a sign, then a house under construction, then a finished house by its value
// V, and a sold site shows a celebrity's house. File names per theme are the ones on the disc; the choice between house A and B and between the
// celebrity house variants is a PLACEHOLDER (the exe picks from a view number and an estate table that is not decoded).
static const char* kHomeSign[4] = {"Bldgs/Park/sign", "Bldgs/links/sign_links", "Bldgs/Desert/DESsign", "Bldgs/Tropical/TropSign"};
static const char* kHomeConst[4] = {"Bldgs/Park/houseconst", "Bldgs/links/houseconst_links", "Bldgs/Desert/DEShouseconst", "Bldgs/Tropical/TropHouseConst"};
static const char* kHomeHouse[4][2] = {{"Bldgs/Park/House_A", "Bldgs/Park/houseB"}, {"Bldgs/links/House_A_links", "Bldgs/links/House_A_links"}, {"Bldgs/Desert/DhouseA", "Bldgs/Desert/DhouseA"}, {"Bldgs/Tropical/TROPhouseA", "Bldgs/Tropical/TROPhouseB"}};
static const char* kCelebHouse[6] = {"Homes/Sum_home", "Homes/political_home", "Homes/ArtDeco_Home", "Homes/Summer1stCelebHouse", "Homes/Summer2ndCelebHouse", "Homes/DesertCelebHouse"};
static const char* kCelebWalk[11] = {"Celebs/ActionStar", "Celebs/Female_PopSinger", "Celebs/Politician", "Celebs/Comedian", "Celebs/SuperModel", "Celebs/FitnessFem", "Celebs/FemComic", "Celebs/GenMale", "Celebs/MoviePrincess", "Celebs/RockStar", "Celebs/Basketball"};
// The palette file that goes with a home sprite on the disc (empty when the sprite carries its own).
static std::string homePalette(const std::string& base) {
    static const std::pair<const char*, const char*> pal[] = {
        {"Bldgs/Park/House_A", "Bldgs/Park/House_APal.pcx"}, {"Bldgs/Park/houseB", "Bldgs/Park/houseBpal.pcx"}, {"Bldgs/Park/houseconst", "Bldgs/Park/houseconpal.pcx"}, {"Bldgs/Park/sign", "Bldgs/Park/signpal.pcx"},
        {"Bldgs/links/House_A_links", "Bldgs/links/House_A_links_palette.pcx"}, {"Bldgs/links/houseconst_links", "Bldgs/links/housecon_palette.pcx"}, {"Bldgs/links/sign_links", "Bldgs/links/sign_links_palette.pcx"},
        {"Bldgs/Desert/DhouseA", "Bldgs/Desert/DhouseApal.pcx"}, {"Bldgs/Desert/DEShouseconst", "Bldgs/Desert/dhouseconpal.pcx"}, {"Bldgs/Desert/DESsign", "Bldgs/Desert/DESsignPal.pcx"},
        {"Bldgs/Tropical/TropHouseConst", "Bldgs/Tropical/TropHouseCon_palette.pcx"}, {"Bldgs/Tropical/TropSign", "Bldgs/Tropical/TropHouseSign_palette.pcx"},
        {"Homes/Sum_home", "Homes/Sum_homePal.pcx"}, {"Homes/political_home", "Homes/political_homePal.pcx"}, {"Homes/ArtDeco_Home", "Homes/ArtDeco_HomePal.pcx"},
        {"Homes/Summer1stCelebHouse", "Homes/1stSummer_palette.pcx"}, {"Homes/Summer2ndCelebHouse", "Homes/2ndSummer_palette.pcx"}, {"Homes/DesertCelebHouse", "Homes/DesertCelebHouse_palette.pcx"}};
    for (const auto& e : pal) if (base == e.first) return e.second;
    return std::string();
}
static void addHomeProps(App& app) {
    const Terrain& t = app.terrain;
    for (const App::Home& h : app.homes) {
        Prop p; p.building = true;
        p.x = h.x * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f;
        p.z = h.y * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f;
        std::string base;
        if (h.buyer > 0) base = kCelebHouse[app.theme == 2 ? 5 : (h.buyer + h.x) % 5];
        else { const int c = sg::homes::sizeClass(h.value); base = c == 0 ? kHomeSign[app.theme] : c == 1 ? kHomeConst[app.theme] : kHomeHouse[app.theme][h.x & 1]; }
        p.body = spriteFor(app, base + ".flc", false, homePalette(base));
        p.shadow = spriteFor(app, base + "Shadow.flc", true);
        if (p.body) app.props.push_back(p);
        if (h.buyer > 0 && h.buyer - 1 < (int)app.celebs.size()) {   // the resident stands by the house
            const char type = app.celebs[(size_t)(h.buyer - 1)].type;
            Prop r; r.building = true; r.x = p.x + kTileSize * 1.6f; r.z = p.z + kTileSize * 0.4f;
            const std::string w = kCelebWalk[std::clamp(type - 'A', 0, 10)];
            r.body = spriteFor(app, w + "_Walk.flc", false); r.shadow = spriteFor(app, w + "_WalkShadow.flc", true);
            if (r.body) app.props.push_back(r);
        }
    }
}

// ---- Building rules from the exe (src/buildings.cpp): footprints, levels, site work, connection to the clubhouse ----
static void bsysContext(const App& app, std::vector<sg::BuildTile>& tiles, sg::BuildContext& ctx) {
    namespace be = sg::buildings_exe;
    const Terrain& t = app.terrain;
    tiles.assign((size_t)sg::BuildingSystem::kMapTiles, sg::BuildTile());
    for (int y = 0; y < t.h && y < be::kMapSide; y++)
        for (int x = 0; x < t.w && x < be::kMapSide; x++) {
            sg::BuildTile& b = tiles[(size_t)(x * be::kMapSide + y)];
            b.id = t.type[(size_t)t.tileIndex(x, y)]; b.owned = isOwned(app, x, y);
            if (t.pathAt(x, y)) b.flags |= be::kFlagPath;
            if (b.id == TT_PuttingGreen || b.id == TT_TrickyGreen) b.flags |= be::kFlagGreenTee;
        }
    for (const App::Amen& m : app.amen) {
        if (m.tx < 0 || m.ty < 0 || m.tx >= be::kMapSide || m.ty >= be::kMapSide) continue;
        sg::BuildTile& b = tiles[(size_t)(m.tx * be::kMapSide + m.ty)];
        if (m.kind == 0) b.flags |= be::kFlagBench; else if (m.kind == 1) b.flags |= be::kFlagFlowers;
        else if (m.kind == 2) b.flags |= be::kFlagScenic; else if (m.kind == 3) b.flags |= be::kFlagScenic | be::kFlagPath;
    }
    for (const App::Home& h : app.homes) for (int dy = 0; dy < 2; dy++) for (int dx = 0; dx < 2; dx++) {
        const int x = h.x + dx, y = h.y + dy; if (x >= 0 && y >= 0 && x < be::kMapSide && y < be::kMapSide) tiles[(size_t)(x * be::kMapSide + y)].id = be::kTileBuildingHome;
    }
    ctx = sg::BuildContext();
    ctx.tiles = tiles.data(); ctx.theme = themeExe(app.theme); ctx.difficulty = app.difficulty; ctx.grade = sg::courseGrade((int)app.allHoles.size());
}
static void bsysRebuild(App& app) { std::vector<sg::BuildTile> tl; sg::BuildContext c; bsysContext(app, tl, c); app.bsys.rebuild(c); }

// ---- Home sites: lot value environment and the free-site counter ----
struct HomeLotEnv {
    std::vector<sg::BuildTile> tiles; sg::BuildContext bc; std::vector<sg::homes::HoleInfo> holes; sg::homes::LotContext c;
    int lot(int x, int y) const { return sg::homes::lotValue(c, x, y); }
};
static void makeHomeEnv(const App& app, HomeLotEnv& e) {
    bsysContext(app, e.tiles, e.bc);
    const Terrain& t = app.terrain;
    auto tileOfW = [&](float wx, float wz, int& tx, int& ty) { tx = (int)std::floor((wx + t.w * kTileSize * 0.5f) / kTileSize); ty = (int)std::floor((wz + t.h * kTileSize * 0.5f) / kTileSize); };
    e.holes.clear();
    for (size_t i = 0; i < app.holes.size(); i++) {
        sg::homes::HoleInfo h; h.exists = true;
        if (app.holeStats.size() == app.holes.size()) { h.plays = app.holeStats[i].plays; h.moodSum = app.holeStats[i].moodSum; }
        if (app.hstats.size() == app.holes.size()) { h.top100 = (app.hstats[i].flags & sg::kHoleTop100) != 0; h.top18 = (app.hstats[i].flags & sg::kHoleTop18) != 0; }
        tileOfW(app.holes[i].teeX, app.holes[i].teeZ, h.teeX, h.teeY); tileOfW(app.holes[i].greenX, app.holes[i].greenZ, h.greenX, h.greenY);
        e.holes.push_back(h);
    }
    e.c = sg::homes::LotContext(); e.c.tiles = e.tiles.data(); e.c.theme = themeExe(app.theme); e.c.difficulty = app.difficulty;
    e.c.marinaEffect = app.bsys.effectLevel(sg::buildings_exe::Marina); e.c.holes = e.holes.data(); e.c.holeCount = (int)e.holes.size();
}
static int homeFreeSites(const App& app) {
    int silver = 0;
    if (app.rosterReady) for (int i = 1; i <= sg::Roster::kPool; i++) if ((int)app.roster.e[i].tier >= (int)sg::Tier::Silver && !app.roster.e[i].resigned) silver++;
    return sg::homes::freeSites(silver, (int)app.homes.size());
}
static int defForType(int type) { for (int i = 0; i < kBuildCount; i++) if (kBuild[i].unlock == type) return i; return -1; }
// Mirrors the logic records into app.buildings (what the drawing, saving and visit code read).
static void syncBuildings(App& app) {
    app.buildings.clear();
    for (const sg::BuildingRecord& r : app.bsys.records()) { const int d = defForType(r.type); if (d >= 0) app.buildings.push_back({d, r.x, r.y, r.storedLevel + 1}); }
}
static void resetBsys(App& app) {
    app.bsys.clear();
    const Terrain& t = app.terrain;
    if (t.clubhouseX >= 0) { sg::BuildingRecord c; c.type = sg::buildings_exe::Clubhouse; c.x = t.clubhouseX - 1; c.y = t.clubhouseY - 1; c.side = 4; c.connected = true; app.bsys.addRecord(c); }
    for (const App::Placed& b : app.buildings) { sg::BuildingRecord r; r.type = kBuild[b.def].unlock; r.x = b.tx; r.y = b.ty; r.storedLevel = b.lvl - 1; r.side = sg::buildings_exe::kBaseSide[r.type] + r.storedLevel; app.bsys.addRecord(r); }
    for (const App::Home& h : app.homes) { sg::BuildingRecord r; r.type = sg::buildings_exe::HomeSite; r.x = h.x; r.y = h.y; r.side = 2; app.bsys.addRecord(r); }
    for (const App::Amen& m : app.amen) if (m.kind == 4 || m.kind == 5) { sg::BuildingRecord r; r.type = m.kind == 4 ? sg::buildings_exe::BallWasher : sg::buildings_exe::Landmark; r.x = m.tx; r.y = m.ty; r.side = 1; r.landmarkKind = m.var; app.bsys.addRecord(r); }
    bsysRebuild(app);
}

// Demo scenery: trees on Woods tiles and a clubhouse. File names per theme are the ones found in Flics/.
static void populateProps(App& app) {
    app.props.clear();
    if (app.celebs.empty()) sg::loadCelebrities(app.gameDir + "/Themes/Standard/celebrities.dta", app.celebs, nullptr);   // read from the disc at run time
    resetBsys(app);
    static const char* kClub[4][2] = {{"Bldgs/Park/clubL2", "Bldgs/Park/clubL2_base"}, {"Bldgs/links/clubL2", "Bldgs/links/clubL2_dirt"},
                                      {"Bldgs/Desert/DESclubL2", "Bldgs/Desert/DesClubL1base"}, {"Bldgs/Tropical/TROPclubL2", "Bldgs/Tropical/TROPclubL2_base"}};
    const Terrain& t = app.terrain;
    auto tileCentre = [&](int tx, int ty, float& x, float& z) { x = tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f; z = ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f; };
    addTrees(app);
    for (const App::Placed& b : app.buildings) addBuildingProp(app, b);
    addAmenityProps(app);
    addHomeProps(app);
    app.emps.clear();
    if (t.clubhouseX >= 0) {
        Prop base, body;
        tileCentre(t.clubhouseX, t.clubhouseY, body.x, body.z);
        base.x = body.x; base.z = body.z;
        std::string b = kClub[app.theme][0], g = kClub[app.theme][1];
        body.body = spriteFor(app, b + ".flc", false);
        body.shadow = spriteFor(app, b + "Shadow.flc", true);
        base.body = spriteFor(app, g + ".flc", false);
        base.shadow = spriteFor(app, g + "Shadow.flc", true);
        base.flat = true;
        if (base.body) app.props.push_back(base);
        if (body.body) app.props.push_back(body);
    }
    // The pool of golfers (their sprites differ for variety). Golfers arrive over time, see stepGame().
    app.walkers.clear();
    app.pathLen.clear();
    static const char* kLook[kLooks] = {"Male/MaleKLS", "Male/MalePLS", "Female/FemalePLS", "Female/FemaleSSS"};
    static const char* kAnim[6] = {"_NormalWalk", "_NormalAddress", "_PerfectSwing", "_PuttAddress", "_Putt", "_Happy"};
    for (int l = 0; l < kLooks; l++) {
        app.lookOk[l] = true;
        for (int i = 0; i < 6; i++) {
            const std::string anim = (i == 2 && l >= 2) ? "_NormalSwing" : kAnim[i];   // the women have no PerfectSwing clip
            app.lookBody[l][i] = spriteFor(app, std::string(kLook[l]) + anim + ".flc", false);
            app.lookShadow[l][i] = spriteFor(app, std::string(kLook[l]) + anim + "Shadow.flc", true);
            if (!app.lookBody[l][i]) app.lookOk[l] = false;
        }
    }
    refreshHoles(app);
    app.golfers.assign(kMaxGolfers, Golfer());
    app.econ.init(app.terrain);
    app.simTime = 0; app.spawnTimer = 1e9;
    for (int i = 0; i < kMaxGolfers; i++) {
        Prop p;
        p.golfer = i; p.hidden = true;
        p.body = app.lookBody[0][0]; p.shadow = app.lookShadow[0][0];
        app.props.push_back(p);
    }
}

static std::string money(long long v);
static void say(App& app, const std::string& m, double secs = 5);
static void openGolferCard(App& app, int gi);
static void top10Submit(App& app);
static void openTop10(App& app);
static void creditsOpen(App& app);
static void t2LoadArt(App& app);
static void t2ListSaves(App& app);
static void toastMsg(App& app, const std::string& m);
static bool openBuyLand(App& app);
static void refreshBuildingProps(App& app);
static void tileOf(const App& a, float wx, float wz, int& tx, int& ty);
static unsigned skillMaskOf(const sg::GolferSkills& k);
static int clubFun(const App& app);
static double clubSkill(const App& app);
static int golfersOnCourse(const App& app);
static void loadStory(App& app);
// Starts a golfer on the first hole. Skills vary from golfer to golfer (PLACEHOLDER spread of 4 to 11 out of 15).
static bool g_cardSkills = false;
static int g_cuHover = -1, g_cuFace = -1, g_cardHook = -1, g_cardHover = -1;   // test hooks --card N (open after the --time run), --cardhover B;    // test hooks --cuhover N, --cuface PAGE
static std::string g_saveChamp; static int g_champGo = -1;   // test hooks --savechamp NAME and --champgo PRO_INDEX
static int g_t2Hover = -1;   // test hook --t2hover N (hover or selection on the title side screens)
static int g_resetSeed = -1, g_hoverHook = -1, g_confirmHook = -1;   // test hooks --worldreset SEED, --worldhover N, --worldconfirm N
static int g_moodHook = -99;   // test hook --mood N: starting mood of every new golfer
static int g_needsHook[3] = {0, 0, 0};   // test hook --needs hunger,thirst,fatigue: starting counters of every new golfer
static sg::BodyLook chrLook(const App& app);
static void spawnGolfer(App& app, bool player = false, int proIdx = -1) {
    if (app.holes.empty()) return;
    if (!app.rosterReady) { app.roster.newGame(app.srng); app.rosterReady = true; }
    std::vector<int> onC; unsigned onBits = 0;
    for (const Golfer& o : app.golfers) if (o.active) { if (o.memberId) onC.push_back(o.memberId); onBits |= o.vis == sg::Visitor::Ceo ? sg::kCeoOnCourse : o.vis == sg::Visitor::Commissioner ? sg::kCommissionerOnCourse : o.vis == sg::Visitor::Heiress ? sg::kHeiressOnCourse : 0; }
    const int arrivalId = player || proIdx >= 0 ? 0 : app.roster.pickArrival(app.srng, onC);
    if (!player && proIdx < 0 && arrivalId < 0) { if (app.simTime - app.declineWarn > 60) { say(app, "Your membership is declining.", 8); app.declineWarn = (int)app.simTime; } return; }
    for (int i = 0; i < kMaxGolfers; i++) {
        if (app.golfers[(size_t)i].active) continue;
        if (app.matchOn && (i == app.matchPl || i == app.matchOp)) continue;   // the slots of a match keep their scores until it is settled
        Golfer& g = app.golfers[(size_t)i];
        uint32_t r = app.seed * 2654435761u + (uint32_t)(app.simTime * 1000) + (uint32_t)i * 97u + (uint32_t)app.roundsStarted * 7919u + 1u;
        auto next = [&]() { r ^= r << 13; r ^= r >> 17; r ^= r << 5; return r; };
        g = Golfer();
        g.active = true;
        g.look = (int)(next() % kLooks);
        if (!app.lookOk[g.look]) g.look = 0;
        for (int k = 0; k < 10; k++) g.sim.skills.v[k] = 4 + (int)(next() % 8);
        if (app.roundsStarted == 0 || player) g.sim.skills = app.skills;
        if (proIdx >= 0 && proIdx < (int)app.pros.size()) { g.proIdx = proIdx; for (int k = 0; k < 10; k++) g.sim.skills.v[k] = app.pros[(size_t)proIdx].skills[k]; }   // the first golfer is the one picked with --golfer
        g.route = app.holes[0].route;
        g.holeStart = app.simTime;
        g.memberId = arrivalId; g.skillMask = skillMaskOf(g.sim.skills);
        { sg::SpawnContext c; c.pairIndex = (app.roundsStarted / 2) % 6; c.holes = (int)app.holes.size(); c.difficulty = app.difficulty; c.funRating = clubFun(app); c.skillRating = (int)(clubSkill(app) * 100);
          c.sandbox = app.econ.sandbox; c.onCourse = onBits;
          g.vis = player || proIdx >= 0 ? sg::Visitor::None : sg::visitorFor(app.vstate, c); if (g.vis != sg::Visitor::None) { g.memberId = 0; say(app, g.vis == sg::Visitor::Ceo ? "A corporate CEO is playing your course today. If he likes it he may invest in a seat on your board!" : g.vis == sg::Visitor::Commissioner ? "A county commissioner is playing your course today." : "A wealthy heiress is playing your course today.", 9); } }
        // The exe starts each golfer's mood at 3 plus a random 0 to 2, or at 4 on the easiest difficulty.
        g.mood = app.bsys.arrivalMood((int)(next() % 3), app.difficulty == 0);
        if (g.memberId) { auto& en = app.roster.e[g.memberId]; en.rounds = (uint16_t)std::min(65535, en.rounds + 1); }   // Rnds counts rounds started (docs/DECODE_HOLE_STATS2.md section 5)
        g.lastTick = (unsigned)(app.simTime * sg::kSimTickHz);
        assignBody(app, g, (int)next());
        if (g_moodHook != -99) g.mood = g_moodHook;
        g.rx.hunger = g_needsHook[0]; g.rx.thirst = g_needsHook[1]; g.rx.fatigue = g_needsHook[2];
        g.sim.loop = false;
        applyFacilityEffects(app, g.sim);
        g.sim.setRoute(&g.route);
        g.sim.theme = app.theme; g.ordinal = app.roundsStarted;
        g.sim.init(app.terrain, next());
        if (player) { g.isPlayer = true; g.mood = 6; g.look = app.chr.female() ? 2 : 0; assignBody(app, g, 0); }
        else if (proIdx >= 0) { g.mood = 6; assignBody(app, g, 0); }
        else app.roundsStarted++;
        return;
    }
}

static int golfersOnCourse(const App& app) { int n = 0; for (const Golfer& g : app.golfers) n += g.active; return n; }

static void golferSounds(App& app, Golfer& g) {
    const char* ev = g.sim.event;
    if (!std::strcmp(ev, "drive")) snd(app, !std::strcmp(g.sim.club, "iron") ? (g.sim.planDist * 0.15f < 45.0f ? "Golf_Sfx/Chip.wav" : "Golf_Sfx/Iron.wav") : "Golf_Sfx/Drive With Ball.wav", 0.8f);   // a short approach is a chip (DECODE_VOICES.md section 5)
    else if (!std::strcmp(ev, "putt")) snd(app, "Golf_Sfx/Putt.wav", 0.8f);
    else if (!std::strcmp(ev, "on the course")) snd(app, "Golf_Sfx/Ball Drop Fairway.wav", 0.5f);
    else if (!std::strcmp(ev, "in the sand")) snd(app, "Golf_Sfx/Ball Drop Sand.wav", 0.6f);
    else if (!std::strncmp(ev, "splash", 6)) snd(app, "Golf_Sfx/Ball Water.wav", 0.8f);
    else if (!std::strncmp(ev, "out of bounds", 13)) snd(app, "Golf_Sfx/Ball Tree.wav", 0.8f);
    else if (!std::strcmp(ev, "holed")) {
        snd(app, "Golf_Sfx/Ball In Hole.wav", 0.8f);
        snd(app, g.sim.stroke <= 5 ? "ApplauseGood.wav" : "Applause.wav", 0.5f);
        snd(app, "Effects/cash.wav", 0.6f);
    }
}

// Advances the club by dt seconds: arrivals, every golfer's round, fees, wages and the board's messages.
static std::string money(long long v);
static void say(App& app, const std::string& m, double secs);
// ---- Club systems decoded from the publisher exe (docs/DECODE_*.md). Placeholders are marked where inputs are not available yet. ----
static void awardGoals(App& app, const std::vector<int>& v) {
    for (int g : v) {
        say(app, std::string("Accomplishment: ") + sg::goalName(g), 7); snd(app, "Effects/TaDa.wav", 0.7f);
        if (g >= 0 && g < 22 && app.miles[g].tick == 0) { app.miles[g].tick = (long)(app.econ.day - 1) * 1024 + 512; app.miles[g].course = app.courseName; app.snapPending = g; }
    }
}
static void goalEvent(App& app, sg::GoalEvent ev) { ev.holeCount = (int)app.holes.size(); awardGoals(app, app.tracker.onEvent(ev)); }
static double routeYardsRaw(const HoleRoute& r) { return r.length * 0.15; }   // PLACEHOLDER scale (see the report screen)
static int doglegOf(const HoleRoute& r) {
    if (r.route.size() < 6) return 0;
    const double gx = r.route[r.route.size() - 2], gz = r.route[r.route.size() - 1];
    const double a1 = std::atan2(gz - r.route[1], gx - r.route[0]), a2 = std::atan2(gz - r.route[3], gx - r.route[2]);
    double d = (a1 - a2) / (2 * M_PI); d -= std::floor(d + 0.5);
    return sg::doglegFromAngle((int32_t)std::llround(d * 4294967296.0));
}
static void checkGoals(App& app) {
    if (app.autoOpen) return;
    if (app.ratings.size() == app.holes.size())
        for (const HoleRating& r : app.ratings) { sg::GoalEvent e; e.kind = sg::GoalEvent::HoleTyped; e.type = r.typeIndex; goalEvent(app, e); }
}
static void logEv(App& app, int code, int arg) { app.evLog[(size_t)((app.econ.day - 1) % 500)] = (short)(code | (arg & 31)); }
static void addHighlight(App& app, const std::string& text, bool bad = false) {
    static const char* mo[8] = {"March", "April", "May", "June", "July", "August", "September", "October"};
    if (app.highlights.size() < 12) app.highlights.push_back({std::string(mo[(app.econ.day - 1) % 8]) + ": " + text, bad});
}
static void syncHoleStats(App& app) {
    if (app.hstats.size() == app.holes.size()) return;
    app.hstats.assign(app.holes.size(), sg::HoleStats());
    for (size_t i = 0; i < app.holes.size(); i++) {
        const HoleRoute& r = app.holes[i];
        const int yards = sg::measuredToYards((int)routeYardsRaw(r));
        app.hstats[i].reset(r.par, yards, 0);
    }
}
static int ownedFacilityKinds(const App& app) {
    if (const char* e = std::getenv("SG_TEST_FACILITIES")) return std::atoi(e);   // test hook
    std::set<int> k; for (const App::Placed& b : app.buildings) k.insert(kBuild[b.def].unlock);
    int n = 0; for (int t : k) if (t >= 6) n++;   // building types from the Putting Green up count as facilities
    return n;
}
// Course statistics for the SGA evaluation. Length, time and fun come from the exe style hole records once golfers have played;
// variety and scenic are PLACEHOLDER tests (the exe's come from golfer comments the port does not produce yet).
static sg::SgaInput sgaInput(App& app) {
    syncHoleStats(app);
    sg::SgaInput in;
    in.holes = (int)app.holes.size(); in.difficulty = app.difficulty;
    if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 20, app.difficulty)); }
    int played = 0, funSum = 0, minSum = 0;
    for (size_t i = 0; i < app.holes.size(); i++) {
        in.totalYards += app.hstats[i].yards;
        const HoleRating& r = app.ratings[i];
        in.lengthHoles += (r.typeIndex & 1) ? 1 : 0; in.accuracyHoles += (r.typeIndex & 2) ? 1 : 0; in.imaginationHoles += (r.typeIndex & 4) ? 1 : 0;
        if (app.hstats[i].rounds > 0) { played++; funSum += app.hstats[i].funPercent(); minSum += app.hstats[i].avgMinutes(); }
        else minSum += 13;   // PLACEHOLDER minutes for a hole nobody has played
        if (i == 0 || r.typeIndex != app.ratings[i - 1].typeIndex || app.holes[i].par != app.holes[i - 1].par) in.varietyHoles++;   // PLACEHOLDER variety
    }
    in.avgMinutes = minSum;
    in.funPercent = played ? funSum / played : 100;
    in.scenicHoles = 0;   // PLACEHOLDER: needs the "scenic view" comments
    in.facilityKinds = ownedFacilityKinds(app);
    return in;
}
static void julyCheck(App& app) {
    const int month = app.econ.day - 1;
    if (month == app.offerMonth) return;
    app.offerMonth = month;
    if (app.autoOpen || app.holes.empty() || app.econ.gameOver) return;
    sg::OfferContext oc; oc.sandbox = app.econ.sandbox; oc.inProgress = false; oc.matchPending = false;
    if (!sg::julyOfferDue((unsigned)month * 1024u, oc)) return;
    const sg::SgaInput in = sgaInput(app);
    const sg::SgaResult r = sg::evaluateCourse(in);
    { sg::GoalEvent e; e.kind = sg::GoalEvent::CourseRated; e.total = r.total; goalEvent(app, e); }
    if (app.tourney.evaluateOffer(in)) {
        char b[220]; std::snprintf(b, sizeof b, "The SGA is interested in holding the %s at your course, first prize %s,000. Press F7 to respond.", app.tourney.name(), std::to_string(app.tourney.firstPrizeThousands()).c_str());
        say(app, b, 14);
    }
}
static unsigned skillMaskOf(const sg::GolferSkills& k) {
    unsigned m = 0;   // PLACEHOLDER: skill groups for the hole statistics
    if (k.v[0] + k.v[1] >= 16) m |= 1;
    if (k.v[2] + k.v[3] >= 16) m |= 2;
    if (k.v[5] + k.v[6] + k.v[7] >= 24) m |= 4;
    return m ? m : 7;
}
static void visitors(App& app) { (void)app; }   // visitors are decided when a pair launches (see spawnGolfer)
static std::string memberName(App& app, int id);
static std::string memberName(App& app, int id);
// Which colours a golfer wears: a club member is a row of progolfers.dta (body, skin, hat, shirt, pants; the alternate skin is forced to 4 for pros), anyone else
// takes the exe's default rules (pants = id mod 10, shirt = 3 id mod 10 for men, 2 id mod 3 + 1 for women). PLACEHOLDER: female hair and the default skin, which come from a table not in the text.
static void assignBody(App& app, Golfer& g, int spin) {
    if (!app.prosTried) { app.prosTried = true; sg::loadTourPros(app.gameDir + "/Themes/Standard/progolfers.dta", app.pros, nullptr); }
    sg::BodyLook l; const bool fem = g.look >= 2;
    if ((g.memberId > 0 || g.proIdx >= 0) && !app.pros.empty() && !g.isPlayer) {
        const sg::TourPro& r = app.pros[(size_t)(g.proIdx >= 0 ? g.proIdx : (g.memberId - 1)) % app.pros.size()];
        l.female = r.female(); l.body = r.body; l.skin = std::clamp(r.skin, 0, 3); l.hat = r.hat; l.shirt = r.shirt; l.pants = r.pants; l.hair = r.hat % 6; l.altSkin = 4;
        g.look = l.female ? 2 : 0;
    } else {
        const int id = (spin & 0x7fffffff) % 75 + 1;
        l.female = fem; l.body = fem ? 4 + (id & 1) : id & 3; l.pants = id % 10; l.shirt = fem ? (id * 2) % 3 + 1 : (id * 3) % 10; l.skin = id % 4; l.hat = l.shirt; l.hair = id % 6; l.altSkin = 4;
    }
    { const sg::BodyLook use = g.isPlayer ? chrLook(app) : l; g.bodyCls = std::clamp(use.body, 0, 7); g.bset = bodySetFor(app, use); }
}
static void endRound(App& app, Golfer& g, bool finished) {
    const int holes = (int)app.holes.size();
    if (g.proIdx >= 0) { g.active = false; return; }   // the pro in a match is not a member of the club
    if (g.isPlayer) {   // a practice round: nothing is recorded in the club's books
        int tp = 0; for (const auto& hh : app.holes) tp += hh.par;
        if (finished) say(app, app.charName + " finished the practice round with " + std::to_string(g.strokesRound) + " strokes on a par " + std::to_string(tp) + " course.", 8);
        g.active = false; return;
    }
    if (finished && holes > 0 && g.strokesRound > 0) {   // Best N Hole Scores: a full round of every hole (docs/DECODE_TOP10_PAIR.md section 2)
        int tp = 0; for (const auto& hh : app.holes) tp += hh.par;
        const std::string who = g.memberId ? memberName(app, g.memberId) : std::string("Visiting golfer");   // PLACEHOLDER: visitors have no names yet
        const bool rec = app.best.setsRecord(g.strokesRound, holes, tp);
        app.best.insert(g.strokesRound, who);
        if (rec) say(app, who + " has just set a new course record of " + std::to_string(g.strokesRound) + " strokes!", 8);
    }
    if (g.vis != sg::Visitor::None) {
        const sg::Outcome o = sg::resolve(g.vis, app.vstate, finished, g.mood, holes, app.srng);
        if (o.cashUnits > 0) { app.econ.earn(o.cashUnits * 100.0); say(app, "The corporate CEO enjoyed your course and invested " + money((long long)o.cashUnits * 100) + " in the club", 9); }
        if (o.landmarkType >= 0) {   // the exe names the design, its value, where to place it and one sentence for the effect class (docs/DECODE_WORLD2.md 1.5); the wording is PLACEHOLDER paraphrase
            static const char* kEff[4] = {"Golfers will have happy thoughts near it.", "Golfers will find no dandelions near it.", "will improve their skills rapidly near it.", "Golfer stories will proceed happily near it."};
            const std::string eff = o.effect == 2 ? app.charName + " " + kEff[2] : kEff[o.effect & 3];
            snd(app, "Effects/Twinkle.wav", 0.7f);
            say(app, "The wealthy heiress donated a " + std::string(sg::landmarkKindName(o.landmarkType)) + " worth " + money(o.landmarkDollars) + ". Place it for free from Landmarks on the Amenities panel. " + eff, 12);
        }
        if (o.buyLand) { app.landOffer = true; say(app, "The county commissioner approves of your course and will let you buy more land", 9); openBuyLand(app); }
    } else if (g.memberId) {
        if (finished) {
            sg::RoundSummary r; r.likedSum = g.liked; r.mood = g.mood; r.difficulty = app.difficulty; r.yearIndex = (app.econ.day - 1) / 8;
            r.cashUnits = (int)(app.econ.cash / 100); r.memberCount = app.roster.memberCount(); r.likedNearEnd = g.likedLast; r.ordinary = true;
            int invited = -1;
            { auto& en = app.roster.e[g.memberId]; int tp = 0; for (const auto& hh : app.holes) tp += hh.par; const sg::RoundEnd re = sg::roundEnd(g.strokesRound, tp, (int)app.holes.size()); en.low = (uint8_t)sg::newLow(en.low, re.adjusted); en.hcp = (int8_t)sg::newHandicap(en.hcp, re.overPar); }   // Low and Hcp rules: docs/DECODE_HOLE_STATS2.md section 5
            if (app.roster.endRound(g.memberId, r, app.srng, &invited)) say(app, "A golfer has been promoted: membership is now " + std::to_string(app.roster.memberCount()) + " members", 6);
        } else if (app.roster.leave(g.memberId, g.hole + 1)) say(app, "A member resigned from the club after a bad round", 6);
    }
    g.active = false;
}
// ---- Golfer reactions (docs/DECODE_EVENTS_SHOTS.md, DECODE_EVENTS_NEEDS.md) ----
// The reaction engine and the needs clock are exact; the PLACEHOLDERS are the tick rate (sg::kSimTickHz), the lie penalty per terrain (the exe's table
// is not in the decompile), the plan hazard score formula, and the producers that need the exe's aim search or flight model (types 6, 8, 12, 32,
// 33, 10, 43, 16, 17), which do not exist yet.
static int penOf(int tt) {
    if (tt < 0) return 4;
    switch (tt) {
        case sg::TT_Tee: case sg::TT_PuttingGreen: case sg::TT_Fairway: case sg::TT_FirmFairway: case sg::TT_TrickyGreen: return 0;
        case sg::TT_Rough: case sg::TT_Marsh: case sg::TT_FlowerBed: return 1;
        case sg::TT_DeepRough: case sg::TT_Overgrowth: case sg::TT_Overgrowth2: case sg::TT_GrassySand: case sg::TT_PotSandBunker: case sg::TT_SandBunker1: case sg::TT_ZenSand: case sg::TT_GrassBunker: return 2;
        case sg::TT_Brush: case sg::TT_Rock: case sg::TT_Woods: case sg::TT_Cliff: case sg::TT_Ravine: return 3;
        default: return 4;   // water, buildings
    }
}
static bool isTreeTile(int tt) { return tt == sg::TT_Woods; }
static int exeTerrainId(int tt) {   // the id the comment text expects (docs/DECODE_COMMENTS.md section 3.1)
    switch (tt) {
        case sg::TT_Tee: return 0; case sg::TT_PuttingGreen: case sg::TT_TrickyGreen: return 1; case sg::TT_Fairway: return 2; case sg::TT_FirmFairway: return 3;
        case sg::TT_Rough: case sg::TT_FlowerBed: return 4; case sg::TT_DeepRough: return 5; case sg::TT_GrassySand: return 8; case sg::TT_PotSandBunker: return 9;
        case sg::TT_Overgrowth: case sg::TT_Ravine: return 10; case sg::TT_Brush: return 11; case sg::TT_Rock: case sg::TT_Cliff: return 12; case sg::TT_Woods: return 13;
        case sg::TT_WaterShallow: case sg::TT_WaterMiddle: case sg::TT_WaterDeep: case sg::TT_WaterShallowDesert: return 17; case sg::TT_Marsh: return 18; case sg::TT_Overgrowth2: return 19;
        case sg::TT_Building: return 22; case sg::TT_SandBunker1: case sg::TT_ZenSand: case sg::TT_GrassBunker: return 7; default: return tt < 0 ? 20 : 4;
    }
}
// Golfer voices (docs/DECODE_VOICES.md section 9). The exe picks a voice id by reaction type and gender; the id to file link is rebuilt from the type meaning,
// so it is keyed by clip name here. No timer, kind or probability test exists in the exe. Pan, pitch jitter and start delays are not modelled yet.
static const char* voiceStem(int type, bool female) {
    switch (type) {
        case 4: return "EASY0"; case 5: case 31: return "HARD0"; case 6: case 39: return "TRICKY0"; case 7: case 28: return "LOVELY0";
        case 9: case 35: return "MAD0"; case 10: case 43: return "WET0"; case 11: return "SENIC0"; case 12: case 13: return "OOPS0";   // PLACEHOLDER clip for 12/13 (id clash in the exe)
        case 14: return "THIRSTY"; case 15: return "HUNGRY"; case 20: return "UGLY0"; case 21: return "WAITING0"; case 22: return "CELEB0"; case 23: return "BADHOLE";
        case 24: return "CRAB0"; case 25: return "COKE0"; case 26: return "TIRED"; case 27: return "BENCH0"; case 29: return "VARIETY"; case 30: return "SAME";
        case 32: case 33: return "OPTIONS"; case 34: return female ? "STORYOK" : "STORYYES"; case 36: return "BAD0"; case 58: return female ? "STORYNO" : "STORYOK";
        default: return nullptr;
    }
}
static void speakVoice(App& app, const Golfer& g, int type) {
    const bool female = g.look >= 2;   // looks 2 and 3 use the female body sets (PLACEHOLDER until golfers carry a character template)
    if (type == 1 || type == 2 || type == 3 || type == 8) {   // emotion clips by body class (DECODE_VOICES.md 4.1; which bank is which kind is a weak reading)
        static const char* kM[4] = {"MalePLS", "MaleKLS", "MalePSS", "MaleSSS"}, *kF[4] = {"FemalePLS", "FemaleSSS", "FemalePSS", "FemaleSKTT"};
        const bool bad = type != 1, after = g.rx.polarity() == 2;
        const std::string kind = bad ? (after ? "Sad" : "Failure") : (after ? "Happy" : "Success");
        const char* cls = female ? kF[g.bodyCls & 3] : kM[g.bodyCls & 3];
        std::string rel = std::string("Emotion/") + cls + kind + (!female && (g.bodyCls & 3) == 1 && kind == "Sad" ? " mix" : "") + ".wav";
        snd(app, rel.c_str(), 0.55f);
        return;
    }
    const char* stem = voiceStem(type, female);
    if (!stem) return;
    std::string rel = std::string("simsfx/") + (female ? "female/f" : "male/m") + stem + ".wav";
    snd(app, rel.c_str(), 0.8f);
}
static void raiseReaction(App& app, size_t gi, int type, int loc) {
    Golfer& g = app.golfers[gi];
    sg::ReactIn in; in.type = type; in.loc = loc; in.difficulty = app.difficulty; in.kind = g.vis == sg::Visitor::None ? 0 : 0x60; in.strokes = g.sim.stroke; in.slot = (int)gi;
    Golfer* pr = (gi ^ 1) < app.golfers.size() ? &app.golfers[gi ^ 1] : nullptr;   // the exe's partner is the other golfer of the pair; the port pairs only slots that share a hole
    const bool partnerFree = pr && pr->active && pr->hole == g.hole && pr->rx.speech == 0 && pr->mood > 0;
    const sg::ReactOut o = sg::react(g.rx, g.mood, in, partnerFree);
    if (getenv("SG_REACTLOG")) std::fprintf(stderr, "[%.1fs] golfer %zu hole %d stroke %d reacts type %d loc %d: %s delta %d mood %d\n", app.simTime, gi, g.hole + 1, g.sim.stroke, type, loc & 0x3fff, o.ignored ? "ignored" : "applied", o.delta, g.mood);
    if (o.ignored) return;
    speakVoice(app, g, type);
    if (!g.leaving && g.hole >= 0 && g.hole < (int)app.hstats.size()) app.hstats[(size_t)g.hole].recordEvent(type, loc & 0x3fff, o.delta);   // a leaving golfer files into record 19, which no screen shows
    if (!g.leaving && g.hole >= 0 && g.hole < (int)app.holeStats.size()) app.holeStats[(size_t)g.hole].moodSum += o.delta;   // the hole's fun total
    if (o.delta > 0) { if (g.memberId) { app.roster.like(g.memberId, g.hole + 1); g.liked++; } g.likedLast = true; } else if (o.delta < 0) g.likedLast = false;
    if (o.partnerReacts) { raiseReaction(app, gi ^ 1, 47, 0x14); pr->rx.speech += 2; }
}
static float tileCornerH(const App& app, int tx, int ty) { return (tx < 0 || ty < 0 || tx > app.terrain.w || ty > app.terrain.h) ? 0.0f : (float)app.terrain.cornerAt(tx, ty); }
static const int kNbDx[8] = {1, 1, 0, -1, -1, -1, 0, 1}, kNbDy[8] = {0, 1, 1, 1, 0, -1, -1, -1};
// Walking glance (docs/DECODE_EVENTS_NEEDS.md section 3.6): one adjacent tile within 90 degrees of the facing, once per (mask + 1) ticks.
static void walkingGlance(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    int tx, ty; tileOf(app, g.sim.golferX, g.sim.golferZ, tx, ty);
    const int f = ((int)std::lround(g.sim.golferHeading / 45.0f) % 8 + 8) % 8;
    const int k = (f - 2 + (int)app.srng.below(5) + 8) & 7;
    const int a = tx + kNbDx[k], b = ty + kNbDy[k];
    for (const App::Amen& m : app.amen) if (m.tx == a && m.ty == b) {
        if (m.kind == 5) raiseReaction(app, gi, m.var >= 16 ? sg::kEvUglyView : sg::kEvScenicView, a + 50 * b);   // a landmark: kinds 16 and up are the ugly ones
        else if (m.kind == 1 && g.rx.polarity() == 2) raiseReaction(app, gi, sg::kEvScenicView, a + 50 * b);   // flower beds only please golfers whose last reaction was bad
        return;
    }
}
// Plan time scenic scan, then the reaction tree (docs/DECODE_EVENTS_SHOTS.md section 2.4, DECODE_EVENTS_NEEDS.md section 3.7).
static void onShotPlan(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    if (g.hole < 0 || g.hole >= (int)app.hstats.size()) return;
    sg::HoleStats& hs = app.hstats[(size_t)g.hole];
    const sg::ShotSim& sim = g.sim;
    const int d = app.difficulty, hole1 = g.hole + 1, strokes = sim.stroke;
    if (sim.planPutt) return;   // putts are not counted as plans and raise nothing here
    hs.recordShotPlan();
    int fx, fy; tileOf(app, sim.planFromX, sim.planFromZ, fx, fy);
    const int fromTile = app.terrain.typeAtWorld(sim.planFromX, sim.planFromZ);
    const float yards = sim.planDist / kTileSize * 25.0f, b0c = std::min(yards, 260.0f);
    const float ca = std::cos(sim.planAim * 3.14159265f / 180.0f), sa = std::sin(sim.planAim * 3.14159265f / 180.0f);
    const int n = (int)(b0c / 25.0f);
    // Hazard score, following the doc's shape (DECODE_EVENTS_SHOTS.md section 2.2): a slot term, the aim line weighted by distance, a second line rotated 15 degrees
    // either way, the eight tiles around the aim point, then divided by n + 3. The per terrain penalties are a PLACEHOLDER table (penOf).
    int acc = 4 - ((int)gi & 3), trees = 0;
    {
        const float rot = ((app.srng.below(2) ? 15.0f : -15.0f) + sim.planAim) * 3.14159265f / 180.0f, c2 = std::cos(rot), s2 = std::sin(rot);
        for (int k = 0; k < n; k++) {
            const int t1 = app.terrain.typeAtWorld(sim.planFromX + ca * kTileSize * (0.5f + k), sim.planFromZ + sa * kTileSize * (0.5f + k));
            const int t2 = app.terrain.typeAtWorld(sim.planFromX + c2 * kTileSize * (0.5f + k), sim.planFromZ + s2 * kTileSize * (0.5f + k));
            acc += k * penOf(t1) * 2 + penOf(t2) * (k + 1);
            if (isTreeTile(t1)) trees++;
        }
        const float axw = sim.planFromX + ca * kTileSize * b0c / 25.0f, azw = sim.planFromZ + sa * kTileSize * b0c / 25.0f;
        for (int q = 0; q < 8; q++) {
            const int tt = app.terrain.typeAtWorld(axw + kNbDx[q] * kTileSize, azw + kNbDy[q] * kTileSize);
            acc += tt < 0 ? 4 + 4 * n : penOf(tt) * (n + 1) / 2;
        }
    }
    int hz = acc / (n + 3);
    if (trees > 1) hz += 4;
    if (fromTile == sg::TT_PuttingGreen) hz = 0;
    if (b0c < 41 && hz > 10) hz = 10;
    // Scenic scan: samples around the landing point (more of them from high ground).
    const float elev = tileCornerH(app, fx, fy) + 3.0f;
    const int samples = std::max(1, (int)(std::min(15.0f, std::max(3.0f, elev)) * 16) / (d + 2));
    int lovely = -1, ugly = -1, celeb = -1;
    for (int i = 0; i < samples; i++) {
        const float hd = (sim.planAim + (12 - (int)app.srng.below(25)) * 1.40625f) * 3.14159265f / 180.0f;
        const float tiles = (yards - (int)app.srng.below(200) + 100) / 25.0f; if (tiles < 0) continue;
        int cx, cy; tileOf(app, sim.planFromX + std::cos(hd) * kTileSize * tiles, sim.planFromZ + std::sin(hd) * kTileSize * tiles, cx, cy);
        if (cx < 0 || cy < 0 || cx >= app.terrain.w || cy >= app.terrain.h) continue;
        if (tileCornerH(app, cx, cy) > tileCornerH(app, fx, fy) + 1) continue;
        if (app.terrain.type[(size_t)app.terrain.tileIndex(cx, cy)] == sg::TT_Overgrowth2) lovely = cx + 50 * cy;
        for (const App::Amen& m : app.amen) if (m.kind == 5 && m.tx == cx && m.ty == cy) lovely = cx + 50 * cy;
        for (const App::Home& h : app.homes) if (cx >= h.x && cx < h.x + 2 && cy >= h.y && cy < h.y + 2) { if (h.buyer > 0) celeb = h.buyer; else ugly = h.x + 50 * h.y; }
    }
    bool raised = false;
    auto say1 = [&](int type, int loc) { raiseReaction(app, gi, type, loc); raised = true; };
    if (celeb > 0) say1(sg::kEvNiceFeature, celeb - 1);
    else if (ugly >= 0) say1(sg::kEvUglyView, ugly);
    else {
        const int V = hs.variety;
        if (hole1 >= 2 && strokes == (int)(gi & 1) + 1 && V <= (int)app.srng.below(3)) say1(sg::kEvVariety, 0x14);
        else if (d > 0 && hole1 >= 2 && strokes == 1 && V > (int)app.srng.below(3) + 3) say1(sg::kEvSameAsLast, 0x14);
        else if (d > 0 && !((int)app.srng.below(4) + 2 * d < hz) && b0c >= 101 && penOf(fromTile) <= 0) { say1(sg::kEvEasyShotMiss, 0x14); hz = 0; }
        else if ((int)app.srng.below(5) + 6 + d < hz && b0c > 40) { say1(hz >= 24 ? 31 : 37, 0x14); hz = hz >= 24 ? 20 : 10; }   // delta 0 types: only speech and history
        else hz = 6;
    }
    if (!raised && g.rx.speech == 0 && lovely >= 0 && penOf(fromTile) <= 0) say1(sg::kEvLovelyView, lovely);
    g.planYards = b0c;
    if (!raised && g.rx.speech == 0 && hole1 >= 2) {   // first use of a club (exe tier 3): the club index is a PLACEHOLDER chosen from the distance (0 driver ... 12 wedge)
        const int club = std::max(0, std::min(12, (int)((260.0f - b0c) / 20.0f)));
        if (!(g.clubMask & (1 << club))) { g.clubMask |= 1 << club; say1(54, club); }
    }
    if (!raised && g.rx.speech == 0) {   // uphill and downhill remarks: aim corner against ball corner
        int ax, ay; tileOf(app, sim.planFromX + ca * kTileSize * b0c / 25.0f, sim.planFromZ + sa * kTileSize * b0c / 25.0f, ax, ay);
        const float ha = tileCornerH(app, ax, ay), hb = tileCornerH(app, fx, fy);
        if (ha < hb) say1(sg::kEvDownhillNice, 0x14); else if (ha > hb) say1(sg::kEvUphillTricky, 0x14);
    }
    g.hz = (float)hz;
}
// A full shot has come down (docs/DECODE_EVENTS_SHOTS.md section 3.3, 3.2).
static void onShotLanded(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    const sg::ShotSim& sim = g.sim; const int d = app.difficulty;
    const int T = sim.landType, P = sim.landFromType, penT = penOf(T), penP = penOf(P), idT = exeTerrainId(T);
    bool fired = false;
    if (!sim.landOut && penT <= 0) { if (g.hz > 8 && sim.landCloser) raiseReaction(app, gi, sg::kEvGoodShot, idT); }
    else {
        if (g.hz < d + 3) { raiseReaction(app, gi, sg::kEvBadShotPartner, idT); fired = true; }
        else if (penP > 0 && penP <= penT) { raiseReaction(app, gi, T == P ? sg::kEvBadShotPartner : sg::kEvBadShotSelf, idT); fired = true; }
        if (sim.landWater) raiseReaction(app, gi, sg::kEvWater, idT);
        else if (sim.landOut && !fired) raiseReaction(app, gi, sg::kEvBadShotPartner, 20);
    }
    // Hook and slice (exe 22155 to 22162): a silent speaker and a bad lie after a shot of more than 75 yards. The exe's curve is not decoded, so the sideways
    // error of the port's shot stands in for it (PLACEHOLDER threshold of 6 degrees).
    if (g.rx.speech == 0 && penT > 0 && g.planYards > 75.0f) { if (sim.landDev > 6.0f) raiseReaction(app, gi, sg::kEvSlice, 0x14); else if (sim.landDev < -6.0f) raiseReaction(app, gi, sg::kEvHook, 0x14); }
    // Near miss: a hard bounce close to a golfer on another hole.
    const float reach = (d + 2) * 0.5f * kTileSize;
    for (size_t k = 0; k < app.golfers.size(); k++) {
        const Golfer& o = app.golfers[k];
        if (k == gi || !o.active || o.hole == g.hole) continue;
        if (std::hypot(o.sim.golferX - sim.ballX, o.sim.golferZ - sim.ballZ) < reach) raiseReaction(app, k, sg::kEvBallNearlyHit, 0x14);
    }
}
// Tee visits (docs/DECODE_EVENTS_NEEDS.md sections 3.4, 3.5 and 3.14): before the first shot of a hole a golfer in need walks to a Snack Bar within 8 tiles or a bench
// within 2 tiles, and otherwise may use the Putting Green (imagination), Pro Shop (accuracy) or Driving Range (length) once a round. The port moves the golfer
// there instantly and holds it for the visit time; the exe walks. The queue term (golfers ahead on the tee) is taken as 0.
static const App::Placed* nearestBuilding(const App& app, const Golfer& g, int type, float maxTiles) {
    const App::Placed* best = nullptr; float bd = maxTiles * kTileSize;
    for (const App::Placed& b : app.buildings) {
        if (kBuild[b.def].unlock != type) continue;
        const float bx = b.tx * kTileSize - app.terrain.w * kTileSize * 0.5f + kTileSize * 0.5f, bz = b.ty * kTileSize - app.terrain.h * kTileSize * 0.5f + kTileSize * 0.5f;
        const float d = std::hypot(bx - g.sim.golferX, bz - g.sim.golferZ);
        if (d < bd) { bd = d; best = &b; }
    }
    return best;
}
static void teeVisits(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    if (g.vis != sg::Visitor::None || g.hole >= (int)app.holes.size()) return;   // kind 0 golfers only
    const int q = 0;
    const double hz13 = sg::kSimTickHz;
    auto income = [&](int type) { const int inc = app.bsys.incomePerVisit(type); app.econ.earn(inc * Economy::kUnit); app.econ.book(sg::costs::FoodDrink, inc * Economy::kUnit); };
    auto hold = [&](double ticks) { g.sim.hold = std::max(g.sim.hold, (float)(ticks / hz13)); };
    const bool needy = !((g.rx.hunger < 8 - 2 * q || g.rx.hunger < 2) && (g.rx.thirst < 12 - 2 * q || g.rx.thirst < 3));
    if (needy && app.bsys.isConnected(7) && nearestBuilding(app, g, 7, 8)) {
        raiseReaction(app, gi, sg::kEvSnack, 0x14); g.rx.thirst = 0; g.rx.hunger = 0; hold(96); income(7); return;
    }
    if (g.rx.fatigue != 0 && g.rx.fatigue / 20 >= 6 - q) {
        const App::Amen* bench = nullptr; float bd = 2.0f * kTileSize;
        for (const App::Amen& m : app.amen) {
            if (m.kind != 0) continue;
            const float bx = m.tx * kTileSize - app.terrain.w * kTileSize * 0.5f + kTileSize * 0.5f, bz = m.ty * kTileSize - app.terrain.h * kTileSize * 0.5f + kTileSize * 0.5f;
            const float d = std::hypot(bx - g.sim.golferX, bz - g.sim.golferZ); if (d < bd) { bd = d; bench = &m; }
        }
        if (bench) { raiseReaction(app, gi, 27, 0x14); hold(g.rx.fatigue * 8 / 20); g.rx.fatigue = 0; return; }
    }
    struct Fac { unsigned bit, attr; int type, reaction; float radius; };
    static const Fac kFac[3] = {{0x10, 4, 6, 53, 5}, {8, 2, 8, 52, 5}, {4, 1, 10, 51, 6}};
    for (const Fac& f : kFac) {
        if ((g.visits & f.bit) || !(g.skillMask & f.attr) || app.bsys.effectLevel(f.type) <= 0 || !nearestBuilding(app, g, f.type, f.radius)) continue;
        hold(16 + (int)app.srng.below(9)); g.visits |= f.bit; income(f.type); raiseReaction(app, gi, f.reaction, 0x14); return;
    }
}
// ---- Slow play, quitting and tantrums (docs/DECODE_EVENTS_NEEDS.md sections 3.8, 3.9, 6.2, 6.3) ----
// The tee scan, the queue count, the warning flag and the quit rule follow the exe. PLACEHOLDERS: golfers are not blocked by the exe's walking collision, so
// "blocked" means another earlier golfer stands within half a tile in front (or is the partner); the walk to the exit goes straight to the clubhouse; the
// gender of a snapped golfer is the look index parity.
static void clubhouseWorld(const App& app, float& x, float& z);
static void startLeaving(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    if (g.leaving) return;
    const int last = g.rx.hist[0];
    if (g.hole >= 0 && g.hole < (int)app.hstats.size()) app.hstats[(size_t)g.hole].recordQuit();
    const char* why = "gave up in disgust";
    switch (sg::quitReasonCategory(last)) {
        case 4: why = "wanted a tougher course"; break; case 8: case 23: case 30: why = "thinks the course needs work"; break; case 9: why = "got into a scrap with another golfer"; break;
        case 12: why = "took it out on a tree"; break; case 13: why = "threw clubs in the lake"; break; case 14: why = "was too thirsty to go on"; break;
        case 15: why = "was too hungry to go on"; break; case 21: why = "was fed up with slow play"; break; case 26: why = "was too tired to go on"; break; default: break;
    }
    std::printf("[%6.1fs] golfer %zu leaves after hole %d (%s)\n", app.simTime, gi, g.hole + 1, why);
    if (g.memberId) say(app, "A golfer " + std::string(why), 8);
    g.leaving = true; g.rx.leaving = true; g.rx.fatigue = 0; g.sim.hold = 1e9f;
}
static void leavingTick(App& app, size_t gi, unsigned t) {
    Golfer& g = app.golfers[gi];
    if ((t + 0x23u * (unsigned)gi) % 100u == 0) {   // a tantrum every 100 ticks; below -10 the golfer vanishes at once
        raiseReaction(app, gi, 35, 0x14); g.mood -= 1; g.leaveWait = 99; g.sim.anim = sg::GolferAnim::Happy; g.sim.animTime = 0;   // stop timer -99 and the throwing pose
        if (g.mood < -10) { endRound(app, g, false); return; }
    }
    if (g.leaveWait > 0) { g.leaveWait--; return; }
    g.sim.anim = sg::GolferAnim::Walk;
    float cx, cz; clubhouseWorld(app, cx, cz);
    const float dx = cx - g.sim.golferX, dz = cz - g.sim.golferZ, d = std::hypot(dx, dz), step = (float)(130.0 / sg::kSimTickHz);
    if (d < 60.0f) { endRound(app, g, false); return; }
    g.sim.golferX += dx / d * step; g.sim.golferZ += dz / d * step; g.sim.golferHeading = std::atan2(dz, dx) * 180.0f / 3.14159265f;
}
static void crowdTick(App& app, size_t gi) {
    Golfer& g = app.golfers[gi];
    // Another golfer snapped (36): a leaving golfer within 2 tiles, unless the last thing said was the same.
    for (size_t k = 0; k < app.golfers.size(); k++) {
        const Golfer& o = app.golfers[k];
        if (k == gi || !o.active || !o.leaving || g.rx.hist[0] == 36) continue;
        if (std::hypot(o.sim.golferX - g.sim.golferX, o.sim.golferZ - g.sim.golferZ) < 2.0f * kTileSize) { raiseReaction(app, gi, 36, o.look & 1); g.sim.hold = std::max(g.sim.hold, (float)((4 + (int)app.srng.below(4)) / sg::kSimTickHz)); return; }
    }
    if (g.sim.hold > 0 || g.sim.stroke != 0 || !g.sim.walking()) return;
    int q2 = 0; bool blocked = false;
    for (size_t k = 0; k < app.golfers.size(); k++) {
        const Golfer& o = app.golfers[k];
        if (k == gi || !o.active || o.leaving || o.ordinal > g.ordinal || o.hole > g.hole) continue;
        const float dx = o.sim.golferX - g.sim.golferX, dz = o.sim.golferZ - g.sim.golferZ, dd = std::hypot(dx, dz);
        if (dd < 0.5f * kTileSize) {
            float diff = std::fabs(std::fmod(std::atan2(dz, dx) * 180.0f / 3.14159265f - g.sim.golferHeading + 540.0f, 360.0f) - 180.0f);
            if (dd < 1.0f || diff <= 45.0f || k == (gi ^ 1)) blocked = true;
        }
        if (o.hole == g.hole && o.sim.stroke == 0) q2++;
    }
    if (!blocked) return;
    g.sim.hold = (float)((4 + (int)app.srng.below(4)) / sg::kSimTickHz);
    if (q2 <= 3 || g.hurried) return;
    if (!g.rx.slowWarned) g.rx.slowWarned = true;   // the first time only sets the warning flag
    else { raiseReaction(app, gi, sg::kEvSlowPlay, 0x14); g.rx.fatigue++; }
    g.sim.hold = (float)((0x20 + (int)app.srng.below(64)) / sg::kSimTickHz);
    if (app.difficulty == 0) { g.sim.hold = (float)((100 + (int)app.srng.below(28)) / sg::kSimTickHz); g.hurried = true; }
    if (g.mood < 0 && g.vis == sg::Visitor::None) {   // mood below zero: both partners walk off
        startLeaving(app, gi);
        if ((gi ^ 1) < app.golfers.size() && app.golfers[gi ^ 1].active && app.golfers[gi ^ 1].hole == g.hole) startLeaving(app, gi ^ 1);
    }
}
static void golferTick(App& app, size_t gi, unsigned t) {
    Golfer& g = app.golfers[gi];
    if (g.leaving) { leavingTick(app, gi, t); return; }
    crowdTick(app, gi);
    sg::speechTick(g.rx, t);
    sg::NeedsIn ni; ni.tick = t; ni.slot = (int)gi; ni.walking = g.sim.walking(); ni.moving = ni.walking; ni.hole = g.hole + 1;
    ni.theme = themeExe(app.theme); ni.kind = g.vis == sg::Visitor::None ? 0 : 0x60; ni.holeCount = (int)app.holes.size();
    { int tx, ty; tileOf(app, g.sim.golferX, g.sim.golferZ, tx, ty); ni.effort = app.terrain.pathAt(tx, ty) ? 1 : 1 + penOf(app.terrain.typeAtWorld(g.sim.golferX, g.sim.golferZ)) / 2 + 1; }   // PLACEHOLDER walking effort
    int out[3];
    const int n = sg::needsTick(g.rx, ni, [&](int k) { return (int)app.srng.below((uint32_t)std::max(1, k)); }, out);
    for (int i = 0; i < n; i++) raiseReaction(app, gi, out[i], out[i] == 26 ? (g.rx.relation == 3 ? 1 : 0) : 0x14);
    if (ni.walking && sg::glanceDue(g.rx, t, (int)gi, true) && g.rx.hist[0] != 11 && g.rx.hist[1] != 11) walkingGlance(app, gi);
}
// ---- Employees on the course (docs/DECODE_BUILDINGS.md section 5) ----
// Each hired employee walks to the nearest golfer that suits the job and acts for a few seconds. Needs on golfers are a PLACEHOLDER: only thirst exists,
// rising at 0.35 a second (the exe's thirst, hunger and fatigue rates are not decoded). The Groundskeeper has no weeds to remove yet and patrols.
static const char* kEmpWalk[8] = {"Employee/GreeterWalk", "Employee/RangerWalk", "Employee/GKWalk", "Employee/TrayGirl_Walk", "Employee/GolfCeleb_Walk", "Employee/Marshall_Walk", "Employee/LawnTech_Walk", "Employee/SodaVendorWalk"};
static const char* kEmpAct[8] = {"Employee/GreeterAction2", "Employee/RangerAction", "Employee/GKAction", "Employee/TrayGirl_Action", "Employee/GolfCeleb_Action", "Employee/Marshall_Action", "Employee/LawnTech_Action", "Employee/SodaVendorAction"};
static void clubhouseWorld(const App& app, float& x, float& z) {
    const Terrain& t = app.terrain;
    x = t.clubhouseX * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f + 150; z = t.clubhouseY * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f + 150;
}
static void refreshEmpProps(App& app) {
    app.props.erase(std::remove_if(app.props.begin(), app.props.end(), [](const Prop& p) { return p.emp >= 0; }), app.props.end());
    for (size_t i = 0; i < app.emps.size(); i++) { Prop p; p.emp = (int)i; p.x = app.emps[i].x; p.z = app.emps[i].z; app.props.push_back(p); }
}
static void syncEmployees(App& app) {
    int want[4][2];
    for (int k = 0; k < 4; k++) { want[k][1] = std::min(app.econ.skilled[k], app.econ.staff[k]); want[k][0] = app.econ.staff[k] - want[k][1]; }
    bool changed = false;
    for (int k = 0; k < 4; k++) for (int sk = 0; sk < 2; sk++) {
        int have = 0; for (const App::Emp& e : app.emps) if (e.kind == k && e.skilled == (sk != 0)) have++;
        while (have > want[k][sk]) { for (size_t i = app.emps.size(); i-- > 0;) if (app.emps[i].kind == k && app.emps[i].skilled == (sk != 0)) { app.emps.erase(app.emps.begin() + (long)i); break; } have--; changed = true; }
        while (have < want[k][sk]) { App::Emp e; e.kind = k; e.skilled = sk != 0; clubhouseWorld(app, e.x, e.z); e.wx = e.x; e.wz = e.z; e.hired = app.econ.day; app.emps.push_back(e); have++; changed = true; }
    }
    if (changed) refreshEmpProps(app);
}
// The roster row r (kind order, regular after skilled as in empRows) is the n-th walking employee of its kind and grade.
static int empForRow(const App& app, int kind, bool skilled, int nth) {
    int c = 0; for (size_t i = 0; i < app.emps.size(); i++) if (app.emps[i].kind == kind && app.emps[i].skilled == skilled) { if (c == nth) return (int)i; c++; }
    return -1;
}
static void stepEmployees(App& app, float dt) {
    syncEmployees(app);
    for (App::Emp& e : app.emps) {
        e.cool -= dt;
        if (e.phase == 1) {   // acting
            e.timer -= dt;
            if (e.timer <= 0) { e.phase = 0; e.cool = 2.0f; }
            continue;
        }
        // Choose a golfer that suits the job.
        int best = -1; float bd = 1e9f;
        if (e.cool <= 0 && e.kind != 2) for (size_t i = 0; i < app.golfers.size(); i++) {
            const Golfer& g = app.golfers[i];
            if (!g.active || g.sim.finished) continue;
            const bool ok = e.kind == 0 ? g.greetedHole != g.hole : e.kind == 1 ? !g.hurried : g.rx.thirst > (e.skilled ? 0 : 4);
            if (!ok) continue;
            const float d = std::hypot(g.sim.golferX - e.x, g.sim.golferZ - e.z);
            if (d < bd) { bd = d; best = (int)i; }
        }
        float tx = e.wx, tz = e.wz;
        if (best >= 0) { tx = app.golfers[(size_t)best].sim.golferX; tz = app.golfers[(size_t)best].sim.golferZ; }
        else if (e.kind == 2 && !app.allHoles.empty() && std::hypot(e.wx - e.x, e.wz - e.z) < 30.0f) {   // patrol: a random point along some hole
            const HoleRoute& r = app.allHoles[(size_t)(((int)(app.simTime * 7.0) + (int)(e.x)) % (int)app.allHoles.size())];
            if (r.route.size() >= 2) { const size_t k = (((size_t)(app.simTime * 13.0)) % (r.route.size() / 2)) * 2; e.wx = r.route[k]; e.wz = r.route[k + 1]; }
            tx = e.wx; tz = e.wz;
        }
        const float dx = tx - e.x, dz = tz - e.z, d = std::hypot(dx, dz);
        if (best >= 0 && d < 45.0f) {   // arrived: act
            Golfer& g = app.golfers[(size_t)best];
            e.phase = 1; e.heading = std::atan2(dz, dx) * 180.0f / 3.14159265f;
            if (getenv("SG_EMPLOG")) fprintf(stderr, "[%.1fs] employee kind %d%s acts on golfer %d (thirst %d)\n", app.simTime, e.kind, e.skilled ? " skilled" : "", best, g.rx.thirst);
            if (e.kind == 0) {          // Club Pro: stops the golfer 16 ticks; +1 mood (basic only when the golfer is not badly in need)
                e.timer = 16.0f / 13.0f; g.sim.hold = std::max(g.sim.hold, e.timer); g.greetedHole = g.hole;
                raiseReaction(app, (size_t)best, (e.skilled || (g.rx.thirst <= 16 && g.rx.hunger <= 16 && g.rx.fatigue <= 160)) ? 34 : 58, g.rx.polarity());   // a pleasant word, or just a chat when the golfer is in need
                app.empCount[0][e.skilled]++; e.n++;
            } else if (e.kind == 1) {   // Ranger: hurries the golfer
                e.timer = 28.0f / 13.0f; g.hurried = true; app.empCount[1][e.skilled]++; e.n++;
            } else {                    // Soda Vendor: stops the golfer 32 ticks, thirst to 0, pays 2, +1 mood if thirst was above 7 (always for the skilled)
                e.timer = 32.0f / 13.0f; g.sim.hold = std::max(g.sim.hold, e.timer);
                if (e.skilled) g.rx.thirst = 99;
                raiseReaction(app, (size_t)best, 25, 0x14);   // the drink pleases only a thirsty golfer (counter read before the reset)
                g.rx.thirst = 0; app.econ.earn(2 * Economy::kUnit); app.econ.book(sg::costs::FoodDrink, 2 * Economy::kUnit); app.empCount[3][e.skilled]++; e.n++;
            }
            continue;
        }
        if (d > 6.0f) { const float sp = std::min(d, 170.0f * dt); e.x += dx / d * sp; e.z += dz / d * sp; e.heading = std::atan2(dz, dx) * 180.0f / 3.14159265f; }
    }
}

// Monthly home pass (docs/DECODE_HOMES.md section 1.4): every home site updates its value once a month; a for-sale site whose value passes the bar is
// bought by a random celebrity from celebrities.dta, if the ticker is free. The sound id 0x33 is not mapped to a file, so a Buy sound stands in (PLACEHOLDER).
// ---- Hole analysis: Top 100 and Top 18 awards (docs/DECODE_HOLE_STATS2.md section 6) ----
// Absolute thresholds per hole, no ranking. The exe runs this whenever its rating panel is drawn; the port runs it every 2 seconds of play.
// PLACEHOLDER: a declined offer is asked again only after a minute (the exe's repeat rate is not known); the awards' sound and trophy art are stand-ins.
static void stepAwards(App& app) {
    if (app.screen != App::ScreenPlay || app.simTime - app.awardCheck < 2.0) return;
    app.awardCheck = app.simTime;
    {   // variety counter of every hole against the hole before it (docs/DECODE_HOLE_STATS.md); feeds the variety and repetition comments
        unsigned prevMask = 0xffffffffu; sg::HoleGeometry prev; bool havePrev = false;
        for (size_t i = 0; i < app.hstats.size() && i < app.holes.size(); i++) {
            sg::HoleStats& hs = app.hstats[i];
            sg::HoleGeometry geo; geo.par = hs.par; geo.flags = hs.flags;
            tileOf(app, app.holes[i].teeX, app.holes[i].teeZ, geo.teeX, geo.teeY); tileOf(app, app.holes[i].greenX, app.holes[i].greenZ, geo.greenX, geo.greenY);
            const unsigned mask = hs.typeMask(app.difficulty, false);
            hs.variety = hs.computeVariety((int)i + 1, geo, havePrev ? prev : geo, mask, prevMask, app.difficulty);
            prev = geo; prevMask = mask; havePrev = true;
        }
    }
    for (size_t i = 0; i < app.hstats.size() && i < 18; i++) {
        sg::HoleStats& hs = app.hstats[i];
        if (hs.par == 0 || hs.rounds == 0) continue;
        const unsigned keep = hs.flags & (sg::kHoleTop100 | sg::kHoleTop18 | sg::kHoleNamePending);
        hs.flags = (hs.analysisFlags(app.difficulty) & ~(unsigned)(sg::kHoleTop100 | sg::kHoleTop18 | sg::kHoleNamePending)) | keep;
        if (app.awardHole >= 0 || app.simTime < app.awardWait[i]) continue;
        const unsigned cand = hs.awardCandidate(app.difficulty);
        if ((cand & sg::kHoleTop100) && !(hs.flags & sg::kHoleTop100)) { hs.flags |= sg::kHoleNamePending; app.awardHole = (int)i; app.awardKind = 100; app.screen = App::ScreenAward; return; }
        if ((cand & sg::kHoleTop18) && !(hs.flags & sg::kHoleTop18)) { app.awardHole = (int)i; app.awardKind = 18; app.screen = App::ScreenAward; return; }
    }
}
static void answerAward(App& app, bool yes) {
    if (app.awardHole < 0 || app.awardHole >= (int)app.hstats.size()) { app.awardHole = -1; app.screen = App::ScreenPlay; return; }
    sg::HoleStats& hs = app.hstats[(size_t)app.awardHole];
    const int h = app.awardHole;
    if (app.awardKind == 100) {
        hs.flags &= ~(unsigned)sg::kHoleNamePending;
        if (yes) { hs.flags |= sg::kHoleTop100; logEv(app, 0x80, h + 1); snd(app, "Buy1Short.wav", 0.9f); addHighlight(app, "Hole " + std::to_string(h + 1) + " is named one of the best 100 holes in the country."); }
    } else if (yes) { hs.flags |= sg::kHoleTop18; logEv(app, 0xa0, h + 1); snd(app, "Buy1Short.wav", 0.9f); addHighlight(app, "Hole " + std::to_string(h + 1) + " is named one of the top 18 holes."); }
    if (!yes) app.awardWait[h] = app.simTime + 60;
    app.awardHole = -1; app.screen = App::ScreenPlay; app.hover = -1;
}
static void drawAward(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    ui::fillRect(190, 210, 420, 180, 0.12f, 0.2f, 0.12f, 0.96f);
    ui::fillRect(192, 212, 416, 176, 0.86f, 0.84f, 0.7f, 1.0f);
    const int h = app.awardHole + 1;
    const bool t100 = app.awardKind == 100;
    app.font.drawCentered(400, 240, t100 ? "TOP 100 HOLE" : "TOP 18 HOLE", 18, 0.15f, 0.12f, 0.3f);
    const std::string l1 = "Hole " + std::to_string(h) + " has been rated one of the " + (t100 ? "best 100 holes in the country by the Golf Enquirer." : "top 18 holes by Great Golf Holes magazine.");
    app.font.drawCentered(400, 280, l1, 13, 0.1f, 0.1f, 0.3f);
    app.font.drawCentered(400, 302, t100 ? "Accept the honour and give the hole a proper name?" : "Accept the honour?", 13, 0.1f, 0.1f, 0.3f);
    for (int b = 0; b < 2; b++) {
        const float x = b == 0 ? 260.0f : 440.0f; const bool hot = app.vmx >= x && app.vmx < x + 100 && app.vmy >= 336 && app.vmy < 366;
        ui::fillRect(x, 336, 100, 30, hot ? 0.95f : 0.75f, hot ? 0.9f : 0.72f, hot ? 0.6f : 0.5f, 1.0f);
        app.font.drawCentered(x + 50, 358, b == 0 ? "Yes" : "No", 14, 0.1f, 0.1f, 0.3f);
    }
}
static void loadPros(App& app);
static void matchFinish(App& app, bool canceled);
// A pro's challenge and the match itself (docs/DECODE_TOURNAMENTS.md section 7). The wager size, the strength window and the odds are PLACEHOLDER.
static void matchStep(App& app) {
    if (!app.matchOn) {
        if (app.matchPro >= 0 || app.autoOpen || app.holes.size() < 2 || app.econ.gameOver || app.tourney.state() == sg::Tournament::State::InProgress) { app.matchMonth = app.econ.day; return; }
        if (app.econ.day == app.matchMonth) return;
        app.matchMonth = app.econ.day;
        loadPros(app); if (app.pros.empty()) return;
        int mine = 0; for (int k = 0; k < 10; k++) mine += app.skills.v[k];
        std::vector<int> cand; for (size_t i = 1; i < app.pros.size(); i++) if (std::abs(app.pros[i].skillSum - mine) <= (int)app.pros.size() / 4) cand.push_back((int)i);
        if (cand.empty() || app.srng.below(3) != 0) return;
        app.matchPro = cand[(size_t)app.srng.below((int)cand.size())];
        app.matchWager = 10 * (1 + app.difficulty);
        say(app, app.pros[(size_t)app.matchPro].name + " challenges you to a match at your course with a wager of " + money((long long)app.matchWager * 100) + " per hole. Open the Player panel and press Play.", 12);
        return;
    }
    const Golfer& pl = app.golfers[(size_t)app.matchPl]; const Golfer& op = app.golfers[(size_t)app.matchOp];
    const int nh = (int)app.holes.size();
    for (int h = 0; h < nh && h < 18; h++) {
        if (app.matchDone[h] || pl.holeStrokes[h] <= 0 || op.holeStrokes[h] <= 0) continue;
        app.matchDone[h] = true; app.matchDoneN++;
        const int d = op.holeStrokes[h] - pl.holeStrokes[h];   // positive: the player won the hole
        app.matchLead += d > 0 ? 1 : d < 0 ? -1 : 0;
        const std::string who = d > 0 ? app.charName + " wins" : d < 0 ? app.pros[(size_t)op.proIdx].name + " wins" : "Tied on";
        const std::string st = app.matchLead > 0 ? app.charName + " leads by " + std::to_string(app.matchLead) : app.matchLead < 0 ? app.pros[(size_t)op.proIdx].name + " leads by " + std::to_string(-app.matchLead) : "all square";
        say(app, who + " hole " + std::to_string(h + 1) + ": " + st + ".", 5);
    }
    if (app.matchDoneN >= nh || (!pl.active && !op.active)) matchFinish(app, false);
}
static void matchFinish(App& app, bool canceled) {
    if (!app.matchOn) return;
    const std::string pro = app.golfers[(size_t)app.matchOp].proIdx >= 0 ? app.pros[(size_t)app.golfers[(size_t)app.matchOp].proIdx].name : std::string("The pro");
    if (canceled) { app.golfers[(size_t)app.matchPl].active = false; app.golfers[(size_t)app.matchOp].active = false; say(app, "Match canceled.", 5); }
    else if (app.matchLead > 0) {
        const long long amt = (long long)app.matchLead * app.matchWager * 100; app.econ.earn((double)amt);
        say(app, "You beat " + pro + " by " + std::to_string(app.matchLead) + (app.matchLead == 1 ? " hole" : " holes") + ". Collect " + money(amt) + ".", 10);
        { sg::GoalEvent e; e.kind = sg::GoalEvent::MatchWon; goalEvent(app, e); }
        addHighlight(app, "Won a match against " + pro);
    } else if (app.matchLead < 0) {
        const long long amt = (long long)(-app.matchLead) * app.matchWager * 100; app.econ.spend((double)amt);
        say(app, pro + " beat you by " + std::to_string(-app.matchLead) + (app.matchLead == -1 ? " hole" : " holes") + ". Pay " + money(amt) + ".", 10);
    } else say(app, "The match with " + pro + " ended all square. No money changes hands.", 8);
    app.matchOn = false; app.matchPro = -1; app.matchLead = 0; app.matchDoneN = 0; for (bool& b : app.matchDone) b = false;
    app.matchPl = app.matchOp = -1;
}
static void stepHomes(App& app) {
    if (app.homes.empty()) { app.homeMonth = app.econ.day; return; }
    if (app.econ.day == app.homeMonth) return;
    app.homeMonth = app.econ.day;
    HomeLotEnv env; makeHomeEnv(app, env);
    bool changed = false;
    for (App::Home& h : app.homes) {
        sg::homes::Site site; site.x = h.x; site.y = h.y; site.buyer = h.buyer; site.value = h.value;
        const int lot = env.lot(h.x, h.y);
        const bool tickerFree = SDL_GetTicks() / 1000.0 >= app.toastUntil;
        const int nCel = (int)app.celebs.size();
        const bool sold = nCel > 0 && sg::homes::monthlyPass(site, lot, app.homeSales, app.difficulty, tickerFree, [&] { return app.srng.below(nCel); });
        const int before = h.value; h.value = site.value; h.buyer = site.buyer;
        if (sg::homes::sizeClass(before) != sg::homes::sizeClass(h.value) || sold) changed = true;
        if (sold) {
            const sg::Celebrity& c = app.celebs[(size_t)(h.buyer - 1)];
            say(app, std::string("International ") + sg::homes::celebrityKindWord(c.type) + " " + c.name + " has purchased a vacation home at your golf course! Golfers enjoy seeing celebrities as they play.", 9);
            snd(app, "Buy1Short.wav", 0.8f);
            logEv(app, 0xc0, h.buyer - 1); addHighlight(app, c.name + " buys a home.");
        }
    }
    if (changed) refreshBuildingProps(app);
}

static bool g_practiceTest, g_matchTest; static int g_tutTest = -1;
static void stepGame(App& app, float dt) {
    if (g_tutTest >= 0) { app.tutPage = g_tutTest; g_tutTest = -1; }
    { static bool once3 = false; if (!once3 && g_matchTest && !app.holes.empty()) { once3 = true; loadPros(app); app.matchPro = 5; app.matchWager = 10; spawnGolfer(app, true); spawnGolfer(app, false, app.matchPro);
        for (size_t i = 0; i < app.golfers.size(); i++) { if (app.golfers[i].active && app.golfers[i].isPlayer) app.matchPl = (int)i; if (app.golfers[i].active && app.golfers[i].proIdx == app.matchPro) app.matchOp = (int)i; }
        app.matchOn = app.matchPl >= 0 && app.matchOp >= 0; } }
    { static bool once2 = false; if (!once2 && g_practiceTest && !app.holes.empty()) { once2 = true; spawnGolfer(app, true); } }
    const bool open = !app.holes.empty() && !app.econ.gameOver;
    // Arrivals (PLACEHOLDER rates): the first golfer comes at once, then one every 25 s when golfers are happy, slower when not;
    // the course takes at most two golfers per hole, up to the size of the pool.
    app.spawnTimer += dt;
    const double every = 25.0 / (0.5 + app.econ.fun / 100.0);
    const int capacity = std::min(kMaxGolfers, 2 * (int)app.holes.size());
    if (open && app.spawnTimer >= every && golfersOnCourse(app) < capacity) { spawnGolfer(app); app.spawnTimer = 0; }
    app.econ.difficulty = app.difficulty; app.econ.holes = (int)app.allHoles.size(); app.econ.updateUpkeep(app.terrain);
    app.econ.step(dt);
    stepEmployees(app, dt);
    stepHomes(app);
    julyCheck(app); matchStep(app);
    checkGoals(app);
    syncHoleStats(app);
    stepAwards(app);
    if (app.econ.day != app.hgLastDay) {   // one histograph sample per game month
        app.hgLastDay = app.econ.day; const size_t m = (size_t)((app.econ.day - 1) % 500);
        app.hgSkill[m] = (short)std::max(0.0, std::min(32000.0, clubSkill(app) * 100)); app.hgCash[m] = (short)std::max(-32000.0, std::min(32000.0, app.econ.cash / 100.0));
        app.hgFun[m] = (short)std::max(-32000, std::min(32000, clubFun(app))); int st = 0; for (int k = 0; k < Economy::StaffKinds; k++) st += app.econ.staff[k]; app.hgStaff[m] = (short)st;
    }
    if (app.econ.notice[0]) { app.eoyBoard = app.econ.notice; }
    {   // year rollover: record the year and show the End of Year screen when the course is on screen
        const int yr = (app.econ.day - 1) / Economy::kMonthsPerYear;
        if (yr > app.lastYear) {
            App::YearRec rec; rec.cash = (long long)app.econ.cash; rec.fun = clubFun(app); rec.skill = clubSkill(app); rec.members = app.roster.memberCount();
            app.yearHist.push_back(rec); app.eoyYear = app.lastYear; app.lastYear = yr;
            if (app.uiOk && app.screen == App::ScreenPlay) { top10Submit(app); app.screen = App::ScreenEoy; snd(app, "Buy2Short.wav", 0.8f); }   // the end of year sound is list entry 0x7f, buy2short (DECODE_VOICES.md)
        }
    }
    if (app.econ.notice[0]) { std::printf("[%6.1fs] board: %s\n", app.simTime, app.econ.notice); say(app, std::string("The board: ") + app.econ.notice, 12); app.econ.notice = ""; }
    for (size_t gi = 0; gi < app.golfers.size(); gi++) {
        Golfer& g = app.golfers[gi];
        if (!g.active) continue;
        { sg::ShotSim tmp; applyFacilityEffects(app, tmp); g.sim.paceScale = (g.hurried ? 1.15f : 1.0f) * tmp.paceScale; g.sim.spreadDivisor = tmp.spreadDivisor; g.sim.driveBonus = tmp.driveBonus; g.sim.washerAtTee = washerNearTee(app, g.route); }   // a Ranger speeds play up (PLACEHOLDER; the original works near one tee)
        { const unsigned nowTick = (unsigned)(app.simTime * sg::kSimTickHz); for (unsigned t = g.lastTick + 1; t <= nowTick && t <= g.lastTick + 60; ++t) golferTick(app, gi, t); g.lastTick = nowTick; }
        if (!g.active) continue;   // removed by a tick above
        if (g.leaving) continue;
        g.sim.shape = g.isPlayer ? app.shotShape : 0;
        g.sim.step(dt);
        if (g.sim.walking() && g.sim.stroke == 0 && g.visitedHole != g.hole) { g.visitedHole = g.hole; teeVisits(app, gi); }
        if (g.sim.planCount != g.seenPlans) { g.seenPlans = g.sim.planCount; onShotPlan(app, gi); }
        if (g.sim.obsCount != g.seenObs) { g.seenObs = g.sim.obsCount; raiseReaction(app, gi, sg::kEvSlicedTrouble, exeTerrainId(g.sim.obsType)); }   // a tree or building was hit in flight
        if (g.sim.landCount != g.seenLands) { g.seenLands = g.sim.landCount; onShotLanded(app, gi); }
        if (g.sim.event != g.lastEvent || g.sim.stroke != g.lastStroke) {
            g.lastEvent = g.sim.event; g.lastStroke = g.sim.stroke;
            golferSounds(app, g);
            if (!std::strcmp(g.sim.event, "holed")) {
                const int par = g.hole < (int)app.holes.size() ? app.holes[(size_t)g.hole].par : 4;
                const double before = app.econ.cash;
                // Green fee, from the exe's fee routine, in units of 100: the golfer's mood, plus 2 for a Creative class hole or 5 for a
                // Heroic, Strategic or Classic one (the exe adds 2 for the hole type values above 3, and 3 more unless the value is 4; here
                // the type index is L + 2A + 4I, which is an ASSUMPTION about the exe's numbering). The exe also adds 2 each for two hole
                // flags (probably Top 100 and Top 18) and an Airstrip bonus; none of those exist here yet.
                sg::HoleFeeInput fi; fi.golferMood = g.mood; fi.memberTier = g.memberId ? (int)app.roster.e[g.memberId].tier : 0;
                const unsigned hflags = g.hole < (int)app.hstats.size() ? app.hstats[(size_t)g.hole].flags : 0u;
                const int feeUnits = g.isPlayer || g.proIdx >= 0 ? 0 : std::max(0, sg::holeFee(fi, hflags) + app.bsys.airstripFeeBonus());   // the player pays no fee on a practice round
                app.econ.holeFinished(g.sim.stroke, par, feeUnits);
                // After the hole the exe lowers mood by (hole field + 6 + holes played) * (mood - 1 + difficulty) * (difficulty + 1) /
                // ((course factor * 5 + 15) * 8), integer division. The hole field and the course factor are not decoded; both are taken as 0.
                { const int d = app.difficulty; const int dec = ((6 + g.hole) * (g.mood - 1 + d) * (d + 1)) / app.bsys.moodDecayDivisor(); g.mood -= dec; }
                // The score call (type 19, turned into 23 on a hole flagged too hard or too easy) comes when the golfer is silent.
                if (g.rx.speech == 0 && g.hole < (int)app.hstats.size()) { const int st = app.hstats[(size_t)g.hole].scoreEventType(g.sim.stroke, false); if (st == sg::kEvTooHardEasy) raiseReaction(app, gi, st, g.mood & 0x3fff); }
                if (app.holeStats.size() != app.holes.size()) app.holeStats.assign(app.holes.size(), App::HoleStat());
                if (g.hole < (int)app.holeStats.size()) {
                    App::HoleStat& hs = app.holeStats[(size_t)g.hole];
                    hs.plays++; hs.strokes += g.sim.stroke; hs.seconds += app.simTime - g.holeStart; hs.revenue += app.econ.cash - before; hs.mood += app.econ.lastMood;
                    hs.hist[std::min(5, std::max(0, g.sim.stroke - 3))]++;
                }
                if (g.hole < (int)app.hstats.size()) {   // exe style record (docs/DECODE_HOLE_STATS.md); drive, ticks and reactions are PLACEHOLDER inputs
                    sg::HoleStats& xs = app.hstats[(size_t)g.hole];
                    sg::BallLandedInput bl; bl.strokesBefore = 0; bl.driveYards = (int)(std::min(900.0f, app.holes[(size_t)g.hole].length) * 0.15f); bl.fairwayOrBetter = true; xs.recordBallLanded(bl);
                    sg::HoleFinishInput fin; fin.strokes = g.sim.stroke; fin.skillMask = g.skillMask; fin.ordinaryGolfer = g.vis == sg::Visitor::None; fin.fee = feeUnits;
                    fin.elapsedTicks = (int)((app.simTime - g.holeStart) * 13.0); xs.recordHoleFinished(fin);
                }
                g.strokesRound += g.sim.stroke; if (g.hole >= 0 && g.hole < 18) g.holeStrokes[g.hole] = g.sim.stroke;
                std::printf("[%6.1fs] golfer %zu holed hole %d in %d (par %d), fee $%.0f, cash $%.0f, fun %.0f\n", app.simTime, gi, g.hole + 1, g.sim.stroke, par, app.econ.cash - before, app.econ.cash, app.econ.fun);
            }
        }
        if (!g.isPlayer && g.proIdx < 0 && ((g.sim.finished && g.hole + 1 < (int)app.holes.size() && sg::wantsToQuit(g.rx, g.mood, 0)) || (!g.sim.finished && g.sim.walking() && g.sim.stroke == 0 && sg::wantsToQuit(g.rx, g.mood, 0)))) {   // mood below zero and a silent speaker   // a golfer in a bad mood gives up (the exe's rule: mood below zero)
            startLeaving(app, gi);
        } else if (g.sim.finished) {
            if (++g.hole < (int)app.holes.size()) {
                g.route = app.holes[(size_t)g.hole].route; g.hurried = false; g.rx.hurried = false;
                g.holeStart = app.simTime;
                g.sim.init(app.terrain, g.sim.stroke * 7919u + (uint32_t)g.hole + (uint32_t)gi);
            } else {
                std::printf("[%6.1fs] golfer %zu finished the round in %d strokes\n", app.simTime, gi, g.strokesRound);
                endRound(app, g, true);
            }
        }
    }
}

// Advances animation: golfers play, sprite frames cycle at the file's frame time.
static void updateProps(App& app) {
    for (; app.simTime < app.time; app.simTime += 1.0 / 60.0) stepGame(app, 1.0f / 60.0f);
    if (app.time < app.simTime - 1.0) { app.simTime = 0; for (Golfer& g : app.golfers) g.active = false; app.spawnTimer = 1e9; }   // clock went backwards
    for (Prop& p : app.props) {
        if (p.emp >= 0) {
            if (p.emp >= (int)app.emps.size()) { p.hidden = true; continue; }
            const App::Emp& e = app.emps[(size_t)p.emp];
            const int idx = (e.skilled ? 4 : 0) + e.kind;
            const bool act = e.phase == 1;
            p.body = spriteFor(app, std::string(act ? kEmpAct[idx] : kEmpWalk[idx]) + ".flc", false);
            p.shadow = spriteFor(app, std::string(act ? kEmpAct[idx] : kEmpWalk[idx]) + "Shadow.flc", true);
            p.x = e.x; p.z = e.z; p.heading = e.heading; p.hidden = !p.body;
            if (p.body) p.frame = (int)((app.simTime * 1000.0) / std::max<uint32_t>(1, p.body->s.frameMs)) % std::max(1, (int)p.body->s.framesPerView);
            continue;
        }
        if (p.golfer < 0) continue;
        const Golfer& g = app.golfers[(size_t)p.golfer];
        p.hidden = !g.active;
        if (!g.active) continue;
        const ShotSim& s = g.sim;
        int a = (int)s.anim;
        if (g.bset) { p.body = g.bset->body[a]; p.shadow = g.bset->shadow[a]; }
        else { if (!app.lookBody[g.look][a]) { p.hidden = true; continue; } p.body = app.lookBody[g.look][a]; p.shadow = app.lookShadow[g.look][a]; }
        p.x = s.golferX; p.z = s.golferZ; p.heading = s.golferHeading;
        float fr = s.animTime * 1000.0f / (float)std::max<uint32_t>(1, p.body->s.frameMs);
        bool loop = s.anim == GolferAnim::Walk || s.anim == GolferAnim::Happy;
        int n = p.body->s.framesPerView;
        p.frame = loop ? (int)fr % n : std::min((int)fr, n - 1);
    }
}

static bool loadTheme(App& app, int theme) {
    std::string err, dir = app.gameDir + "/Data/Textures/" + kThemes[theme];
    TextureCatalog cat;
    if (!cat.load(dir, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return false; }
    for (auto& kv : app.textures) if (kv.second) glDeleteTextures(1, &kv.second);
    app.textures.clear();
    app.catalog = std::move(cat);
    app.theme = theme;
    {   static const char* kTb[4] = {"ParkLandTerrainButtons.pcx", "LinksTerrainButtons.pcx", "DesertTerrainButtons.pcx", "TropicalTerrainButtons.pcx"};
        app.terrBtns = ui::Image();
        ui::loadPcx(app.gameDir + "/Interface/" + kTb[theme], app.terrBtns, true);
        static const char* kLay[4] = {"parklands layout.pcx", "links layout.pcx", "desert layout.pcx", "trop layout.pcx"};
        app.layoutArt = ui::Image();
        ui::loadPcx(app.gameDir + "/Interface/" + kLay[theme], app.layoutArt, false, 0xF800F8);
    }
    app.sprites.clear();  // GL textures of sprites are leaked on theme change; they are small
    app.light = Lighting();
    loadLighting(app.gameDir + "/" + kThemes[theme] + "Lighting.txt", app.light);
    rebuildBatches(app);
    populateProps(app);
    return true;
}

// Draws props as camera-facing quads. Sprites are pre-rendered, so they ignore lighting and depth:
// ground overlays first, then shadows, then bodies far to near (painter's order).
static void drawProps(App& app) {
    if (!app.showProps || app.props.empty()) return;
    updateProps(app);
    if (app.followG >= 0 && app.followG < (int)app.golfers.size() && app.golfers[(size_t)app.followG].active) { const Golfer& g = app.golfers[(size_t)app.followG]; app.camX = g.sim.golferX; app.camZ = g.sim.golferZ; }
    else if (app.follow) for (const Golfer& g : app.golfers) if (g.active) { app.camX = g.sim.golferX; app.camZ = g.sim.golferZ; break; }
    GLdouble mv[16];
    glGetDoublev(GL_MODELVIEW_MATRIX, mv);
    const float rx = (float)mv[0], ry = (float)mv[4], rz = (float)mv[8];   // eye X axis in world space
    const float ux = (float)mv[1], uy = (float)mv[5], uz = (float)mv[9];   // eye Y axis in world space
    // The sprites were rendered at 4 camera yaws, 90 degrees apart; 8 view sprites (people) face 45 degree steps.
    const int quarter = ((int)std::lround(app.rot / 90.0f) % 4 + 4) % 4;
    const float s = kSpriteUnitsPerPixel;

    struct Item { float depth; const Prop* p; };
    std::vector<Item> bodies;
    for (const Prop& p : app.props) {
        if (p.hidden) continue;
        float y = app.terrain.heightAt(p.x, p.z);
        float depth = (float)(mv[2] * p.x + mv[6] * y + mv[10] * p.z);
        bodies.push_back({depth, &p});
    }
    std::sort(bodies.begin(), bodies.end(), [](const Item& a, const Item& b) { return a.depth < b.depth; });

    glDisable(GL_LIGHTING);
    glDisable(GL_DEPTH_TEST);
    glEnable(GL_TEXTURE_2D);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    auto quad = [&](GlSprite* g, const Prop& p) {
        if (!g) return;
        int views = g->s.views;
        int view = views >= 8 ? (quarter * 2) % 8 : (views >= 4 ? quarter % views : (views == 2 ? quarter % 2 : 0));
        if (views >= 8 && p.heading > -999) {
            // View 0 faces world -X (heading 180); each further view turns 45 degrees counter clockwise
            // on screen, i.e. heading - 45; a 90 degree camera turn adds 2 views.
            int k = (int)std::lround((180.0f - p.heading) / 45.0f);
            view = ((k + 2 * quarter) % 8 + 8) % 8;
        }
        glBindTexture(GL_TEXTURE_2D, spriteTexture(*g, view, p.frame));
        float y = app.terrain.heightAt(p.x, p.z);
        float l = -g->s.anchorX * s, r = ((float)g->s.w - g->s.anchorX) * s;
        float t = g->s.anchorY * s, b = -((float)g->s.h - g->s.anchorY) * s;  // up is positive
        auto corner = [&](float cx, float cy, float u, float v) {
            glTexCoord2f(u, v);
            glVertex3f(p.x + rx * cx + ux * cy, y + ry * cx + uy * cy, p.z + rz * cx + uz * cy);
        };
        glBegin(GL_QUADS);
        corner(l, t, 0, 0); corner(r, t, 1, 0); corner(r, b, 1, 1); corner(l, b, 0, 1);
        glEnd();
    };
    for (const Item& it : bodies) if (it.p->flat) { quad(it.p->body, *it.p); }
    for (const Item& it : bodies) { quad(it.p->shadow, *it.p); }
    for (const Item& it : bodies) if (!it.p->flat) quad(it.p->body, *it.p);
    // The ball: a small white disc with a dark disc on the ground below it.
    for (const Golfer& gl : app.golfers) {
        if (!gl.active || gl.sim.ballH < 0) continue;
        const ShotSim& sm = gl.sim;
        float gy = app.terrain.heightAt(sm.ballX, sm.ballZ);
        glDisable(GL_TEXTURE_2D);
        auto disc = [&](float x, float y, float z, float rpx, float rr, float gg, float bb, float aa, bool upright) {
            glColor4f(rr, gg, bb, aa);
            glBegin(GL_TRIANGLE_FAN);
            glVertex3f(x, y, z);
            for (int i = 0; i <= 12; i++) {
                float a = i * 6.2831853f / 12, cx = std::cos(a) * rpx, cy = std::sin(a) * rpx;
                if (upright) glVertex3f(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy);
                else glVertex3f(x + cx, y, z + cy * 0.8f);
            }
            glEnd();
        };
        disc(sm.ballX, gy + 1, sm.ballZ, 4.5f, 0, 0, 0, 0.45f, false);
        disc(sm.ballX, gy + sm.ballH + 4.0f, sm.ballZ, 5.0f, 1, 1, 1, 1, true);
        glColor4f(1, 1, 1, 1);
    }
    glDisable(GL_BLEND);
    glEnable(GL_DEPTH_TEST);
}

// Course Status Report (the original's F1): the hole's class and length, and how many paths are joined to the clubhouse.
static void reportCourse(App& app, bool force) {
    app.hole = analyzeHole(app.terrain);
    refreshHoles(app);
    if (app.holeStats.size() != app.holes.size()) app.holeStats.assign(app.holes.size(), App::HoleStat());
    int paths = 0, joined = 0;
    const std::vector<uint8_t> conn = pathsConnectedToClubhouse(app.terrain);
    for (size_t i = 0; i < app.terrain.pathKind.size(); i++) if (app.terrain.pathKind[i]) { paths++; joined += conn[i]; }
    char b[160];
    std::snprintf(b, sizeof b, "%zu holes (tee to green pairs); paths: %d tiles, %d joined to the clubhouse, %d shown as mud", app.holes.size(), paths, joined, paths - joined);
    std::string r = app.hole.report() + "; " + b;
    if (force || r != app.lastReport) { std::printf("course: %s\n", r.c_str()); app.lastReport = r; }
}

// ---- Screens: title menu, property chooser, heads-up display --------------------------------------------------------

struct Rect { float x, y, w, h; bool has(float px, float py) const { return px >= x && py >= y && px < x + w && py < y + h; } };

// Regions of the 800x600 title art (read off TitleBASE / TitleUnSel / TitleMO in the disc's Interface folder).
static const Rect kMenuBtn[6] = {{40, 25, 325, 120}, {415, 40, 360, 135}, {12, 395, 285, 100}, {508, 380, 270, 95}, {285, 478, 285, 105}, {718, 528, 64, 64}};
static const char* kThemePacks[5] = {"Standard", "Firaxis", "More Stories", "The Sims", "Championship"};   // the folders under Themes/
static const char* kMenuLabel[5] = {"Continue Saved Game", "Start New Game (Standard)", "Sandbox Mode", "Select A Theme", "Play a Championship"};
static const float kMenuLabelX[5] = {180, 595, 150, 650, 418}, kMenuLabelY[5] = {83, 112, 450, 427, 543};

static bool loadUi(App& app) {
    const std::string i = app.gameDir + "/Interface/";
    bool ok = app.font.load(app.gameDir + "/KLEPTO__.TTF");
    ok = ui::loadPcx(i + "TitleBASE.pcx", app.titleBase, false) && ok;
    ok = ui::loadPcx(i + "TitleUnSel.pcx", app.titleUn, true) && ok;
    ok = ui::loadPcx(i + "TitleMO.pcx", app.titleMo, true) && ok;
    ok = ui::loadPcx(i + "WorldBase.pcx", app.worldBase, false) && ok;
    ui::loadPcx(i + "WorldButton.pcx", app.worldBtn, true);   // optional: Reset, Save and Load button art
    ui::loadPcx(i + "infoscreens/coursereport.pcx", app.reportArt, true);   // optional
    ui::loadPcx(i + "infoscreens/shortcuts.pcx", app.keysArt, true);
    ui::loadPcx(i + "infoscreens/ENDoYEAR.pcx", app.eoyArt, true);
    ui::loadPcx(i + "infoscreens/PlayComt.pcx", app.comtArt, true);
    ui::loadPcx(i + "infoscreens/SGA.pcx", app.sgaArt, true);
    ui::loadPcx(i + "infoscreens/tournament result.pcx", app.tourArt, true);
    ui::loadPcx(i + "infoscreens/histograph.pcx", app.histArt, true);
    ui::loadPcx(i + "bulletinboard&mantlewood.pcx", app.boardArt, false);
    ui::loadPcx(i + "TROPHYparts.pcx", app.partsArt, false, -1, i + "TrophyParts_A.pcx");
    ui::loadPcx(i + "tacs&tees.pcx", app.tacsArt, false, -1, i + "tacs&tees_A.pcx");
    ui::loadPcx(i + "TacksandArrow.pcx", app.pinArt, false, -1, i + "TacksandArrow_A.pcx");
    ui::loadPcx(i + "infoscreens/HoleSTAT.pcx", app.holeArt, true);
    ui::loadPcx(i + "infoscreens/FINANCEreport.pcx", app.finArt, true);     // optional
    ui::loadPcx(i + "infoscreens/memberRoster.pcx", app.rosterArt, true);
    ui::loadPcx(i + "infoscreens/memberRoster_buttons.pcx", app.rosterBtn, true);
    ui::loadPcx(i + "infoscreens/memberRoster_scrollbar.pcx", app.rosterBar, true);
    ui::loadPcx(i + "infoscreens/OkStates.pcx", app.okArt, true);
    ui::loadPcxCircles(i + "infoscreens/OkStates.pcx", app.okRound, 23, 22, 45, 3, 20.5f);           // optional
    ui::loadPcx(i + "3mainLowerLeft.pcx", app.dockArt, false, 0xF800F8);               // optional: the lower left dock
    ui::loadPcx(i + "BaseTerrainPanel.pcx", app.terrPanel, true);
    ui::loadPcx(i + "AmenitiesPanel.pcx", app.amenArt, false, 0xF800F8);
    ui::loadPcx(i + "ElevationPanel.pcx", app.elevArt, true);
    ui::loadPcx(i + "BuildingPanel.pcx", app.bldgArt, true);
    ui::loadPcx(i + "EmployeePanel.pcx", app.empArt, true);
    ui::loadPcx(i + "MemberPanel.pcx", app.memberArt, true);
    ui::loadPcx(i + "JoeCoolPanel.pcx", app.joeArt, true);
    ui::loadPcxCircles(i + "MemberPanel.pcx", app.faceArt, 458, 108, 16, 10, 7.6f);   // the ten mood faces as round pieces without their square backdrop
    ui::loadPcx(i + "infoscreens/buy_land.pcx", app.landArt, true);
    ui::loadPcx(i + "infoscreens/route screens_course.pcx", app.ovHead[0], true);
    ui::loadPcx(i + "infoscreens/route screens_aura.pcx", app.ovHead[1], true);
    ui::loadPcx(i + "infoscreens/route screens_value.pcx", app.ovHead[2], true);
    ui::loadPcx(i + "infoscreens/route screens_employ.pcx", app.ovHead[3], true);
    ui::loadPcx(i + "infoscreens/route screens_bottom.pcx", app.ovBottom, false, 0xFF00FF);
    ui::loadPcx(i + "infoscreens/buy_land_buttons.pcx", app.landBtn, false, 0xFF00FF);
    ui::loadPcx(i + "infoscreens/hire.pcx", app.hireArt, true);
    static const char* kIcon[4] = {"ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"};
    for (int t = 0; t < 4; t++) ui::loadPcx(i + kIcon[t], app.themeIcons[t], false, 0xFF0000);   // icons are optional
    app.uiOk = ok;
    return ok;
}

static std::string money(long long v) {   // "§1,234,567" (the game's currency sign, UTF-8)
    std::string digits = std::to_string(v < 0 ? -v : v), out;
    for (size_t k = 0; k < digits.size(); k++) { if (k && (digits.size() - k) % 3 == 0) out += ','; out += digits[k]; }
    return std::string(v < 0 ? "-" : "") + "\xC2\xA7" + out;
}

static void drawMenu(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.titleBase, 0, 0);
    ui::drawImage(app.titleMo, 170, 190, 170, 190, 480, 185);       // the logo sits in the highlight layer
    ui::drawImage(app.titleUn, 0, 0);
    if (app.hover >= 0 && app.hover < 6) { const Rect& r = kMenuBtn[app.hover]; ui::drawImage(app.titleMo, r.x, r.y, r.x, r.y, r.w, r.h); }
    for (int b = 0; b < 5; b++) {
        std::string label = kMenuLabel[b];
        if (b == 1) { app.font.drawCentered(kMenuLabelX[b], kMenuLabelY[b] - 10, "Start New Game", 19, 0.12f, 0.12f, 0.38f); app.font.drawCentered(kMenuLabelX[b], kMenuLabelY[b] + 12, "(Standard)", 19, 0.12f, 0.12f, 0.38f); continue; }   // two lines in the real title screen
        app.font.drawCentered(kMenuLabelX[b], kMenuLabelY[b], label, 19, 0.12f, 0.12f, 0.38f);
    }
    if (app.themePack != 0) app.font.drawCentered(kMenuLabelX[3], kMenuLabelY[3] + 17, std::string("Theme: ") + kThemePacks[app.themePack], 13, 0.12f, 0.12f, 0.38f);
    if (!app.toast.empty() && SDL_GetTicks() / 1000.0 < app.toastUntil) {
        const float w = app.font.width(app.toast, 18) + 24;
        ui::fillRect(400 - w / 2, 560, w, 30, 0.1f, 0.1f, 0.3f, 0.9f);
        app.font.drawCentered(400, 581, app.toast, 18, 1, 1, 0.8f);
    }
    ui::endScreen();
}

// World map list (docs/UI_SCREENS2.md section 3, "List generation"): each site gets a price slot. The default is the arrangement of the screenshots the
// chooser was first built from (which is itself one generated list); Reset World rolls a new one by the exe's rules.
static void worldDefault(App& app) {
    for (int i = 0; i < 16; i++) { app.siteSlot[i] = 0; for (int k = 0; k < 16; k++) if (ui_screens2::worldmap::kSlotPriceUnits[k] * 100 == kProperties[i].price) app.siteSlot[i] = k; }
    app.worldInit = true;
}
static void worldReset(App& app, uint64_t seed) {
    sg::SocialRng rng(seed); bool used[16] = {}; int slotSite[16];
    bool typeUsed[4] = {};
    for (int sl = ui_screens2::worldmap::kTypeSlotsFirst; sl < 16; sl++) {   // the top four slots take one site of each course type
        for (;;) { const int c = rng.below(16); if (used[c] || typeUsed[ui_screens2::worldmap::kSites[c].type]) continue; used[c] = true; typeUsed[ui_screens2::worldmap::kSites[c].type] = true; slotSite[sl] = c; break; }
    }
    for (int sl = 0; sl < ui_screens2::worldmap::kTypeSlotsFirst; sl++) {   // the rest are random, slot 0 must be parkland
        for (;;) { const int c = rng.below(16); if (used[c] || (sl == 0 && ui_screens2::worldmap::kSites[c].type != 0)) continue; used[c] = true; slotSite[sl] = c; break; }
    }
    for (int sl = 0; sl < 16; sl++) app.siteSlot[slotSite[sl]] = sl;
    app.worldInit = true;
}
static int propPrice(App& app, int i) { if (!app.worldInit) worldDefault(app); return ui_screens2::worldmap::kSlotPriceUnits[app.siteSlot[i]] * 100; }
static int propAcres(App& app, int i) { if (!app.worldInit) worldDefault(app); return ui_screens2::worldmap::acres(app.siteSlot[i], ui_screens2::worldmap::kSites[i].lie); }
static int propAcresShown(App& app, int i) { return (app.sandboxChoice && !app.switchMode) || (app.switchMode && app.econ.sandbox) ? ui_screens2::worldmap::kUnlimitedAcres : propAcres(app, i); }

static Rect propertyCard(int i) {
    static const float lx[10] = {250, 301, 345, 383, 416, 437, 453, 464, 468, 462}, lw[10] = {175, 173, 174, 174, 173, 173, 174, 176, 173, 174};
    static const float ly[10] = {12, 64, 115, 167, 220, 277, 337, 398, 457, 520};
    static const float rx[6] = {465, 508, 545, 579, 605, 622}, ry[6] = {12, 64, 115, 167, 220, 277};
    const Property& p = kProperties[i];
    int idx = 0;
    for (int k = 0; k < i; k++) if (kProperties[k].column == p.column) idx++;
    return p.column == 0 ? Rect{lx[idx], ly[idx], lw[idx], 48} : Rect{rx[idx], ry[idx], 175, 48};
}

static bool canAfford(App& app, int i) {
    if (app.switchMode) return (app.careerOwned >> i & 1) || app.econ.sandbox || propPrice(app, i) <= app.econ.cash;
    return app.sandboxChoice || propPrice(app, i) <= kStartFunds;
}

// Tutorial pages (DECODE_SOCIAL.md section 4: pages 1 to 11 on fun, 21 to 29 on skill; any key advances, Escape stops). The exe text is not copied; these are my own words
// about the rules the port implements. The start trigger is not decoded, so Shift+F8 starts it (PLACEHOLDER).
static const char* const kTutorial[20] = {
    "Welcome to the tutorial. Every course is judged on two ratings: fun, shown in yellow at the top right, and skill, shown in light blue. Money is the green number above them.",
    "Fun comes from happy golfers. Golfers are happier when holes are varied, scenic and fair, and when their needs for food, drink and rest are met.",
    "Start with a tee and a green. Paint a tee, paint a putting green a good distance away, then press H to open the hole. Golfers only play open holes.",
    "Make holes different from each other. A long hole, a short one and a dogleg each feel new; a row of identical holes bores golfers and lowers fun.",
    "Hazards add excitement but they also add frustration. Water, sand and trees make a hole interesting when a good player can see a way round them.",
    "Trees, flowers, benches and landmarks make the course pretty. Scenic holes are rated higher by the magazines and by your visitors.",
    "Golfers do not live by golf alone. Put up a snack bar beside a path that joins the clubhouse, and add a pro shop and a swim club as your course grows.",
    "Benches let tired golfers rest, and a ball washer near a tee helps their next shot. Place them where golfers will actually walk past.",
    "Employees keep golfers content. The club pro greets them, the ranger keeps play moving, the groundskeeper tends the turf and the soda vendor sells drinks.",
    "Watch the comments golfers make. Praise tells you what works; complaints tell you what to fix. Press F2 for the Player Comments report.",
    "That covers fun. Next we look at skill, the rating that decides which tournaments and which visitors come to your course.",
    "Skill measures how hard and how interesting the shots are. Holes that test length, accuracy and imagination all raise it.",
    "Length: a hole long enough that a golfer needs a big drive rates well on length. Short holes are pleasant but easy.",
    "Accuracy: narrow fairways, hazards beside the line and small landing areas make golfers aim carefully. That raises the accuracy rating.",
    "Imagination: doglegs, carries over water and choices between a safe and a risky route reward golfers who think. That raises the imagination rating.",
    "Press F1 for the Course Report to see each hole's length, accuracy and imagination, and the type of hole it makes.",
    "Your own golfer matters too. Spend skill points on the Customise screen, and use the Player panel to play a practice round on your own course.",
    "Good scores and big prizes bring accomplishments. Each one earns a trophy and extra skill points to spend on your golfer.",
    "The SGA may offer a tournament once your course scores well. Press F7 to read its evaluation and respond to the offer.",
    "Now back to your course. Press Shift+F8 whenever you want to see the tutorial again."};
static void tutStart(App& app) { if (app.tutPage >= 0) return; app.tutPage = 0; app.tutWasPaused = app.paused; app.paused = true; }
static void tutNext(App& app, bool stop) {
    if (stop || ++app.tutPage >= 20) { app.tutPage = -1; app.paused = app.tutWasPaused; }
}
// Happy ending: the story's first letter picks the landmark design that is donated (DECODE_WORLD2 1.5), one heart is added, and it is logged as a highlight.
static void storyHappyEnding(App& app) {
    int kind;
    switch (app.storyLetter) {
        case 'C': kind = 0; break; case 'P': kind = 1; break; case 'A': kind = 2; break; case 'M': kind = 3; break; case 'L': kind = 4; break;
        case 'H': kind = 5; break; case 'G': kind = 7; break; case 'F': case 'R': kind = 8; break; case 'X': kind = 9; break; case 'S': kind = 11; break;
        default: kind = (int)(app.seed * 2654435761u >> 8) % 10 + (app.storiesDone % 10); kind %= 10; break;
    }
    app.vstate.availMask |= 1u << kind; app.vstate.landmarkMask |= 1u << kind;
    app.hearts++; app.storiesDone++;
    logEv(app, 0x120, kind);
    addHighlight(app, "Happy ending: " + app.storyTitle);
    snd(app, "Buy1Short.wav", 0.9f);
    say(app, "A love story has a happy ending. A new landmark design is donated to the club.");
    loadStory(app);
}

static std::vector<std::string> wrapText(const App& app, const std::string& s, float size, float maxW);
// ---- Customise Character (docs/DECODE_CUSTOMISE.md). Opened from the Golfers dock tab with the player's name on it. Art, cuts, controls and rules follow the
// exe; hit centres and the small label texts come from data tables the decompile does not show, so they are measured from the art (DERIVED) or marked PLACEHOLDER. ----
static const float kCuPos[20][2] = {{213, 153}, {213, 188}, {213, 223}, {95, 130}, {95, 158}, {95, 186}, {95, 214}, {95, 242}, {49, 22}, {259, 22}, {328, 106}, {454, 80},
                                    {328, 56}, {454, 180}, {454, 230}, {454, 130}, {454, 30}, {328, 156}, {762, 37}, {762, 218}};
static const char* kCuTip[20] = {"", "", "", "", "", "", "", "", "Load character", "Save character", "Body type", "Adult/child", "Select face", "Shirt Color", "Pants Color", "Hair Color", "Skin Tone", "Gender", "Cancel", "OK"};
// PLACEHOLDER label texts: the exe's tables for the five trait buttons and three toggles are not in the decompile (only the first toggle, "length").
static const char* kCuTrait[5] = {"Trait 1", "Trait 2", "Trait 3", "Trait 4", "Skirt or shorts"};
static const char* kCuToggle[3] = {"Length", "Toggle 2", "Toggle 3"};
// Dialogue rows: row 0 is the signature saying (exact); the others are named from the character file notes in docs/FORMATS.md (PLACEHOLDER wording).
static const char* kCuRow[20] = {"Signature saying", "Great shot", "Poor lie", "Rough lie", "Too easy", "Hazard ahead", "Uses a slope", "Slice or hook", "Bad design", "Near miss", "Water", "Lovely view",
                                 "Walk-through", "Crowding", "Thirsty", "Drink", "Hungry", "Snack", "Tired", "Rested"};
static const char* kCuSkill[10] = {"Power Hitter", "Long Driver", "Accurate Driver", "Accurate Irons", "Accurate Putter", "Draw Shot", "Fade Shot", "High Backspin Shot", "Recovery Skills", "Luck"};
static const char* kCuAge[3] = {"Young", "Middle Aged", "Mature"};
static const char* kCuMarital[4] = {"Single", "Married", "Divorced", "Widowed"};

static std::string charDir(const App& app) { return (std::filesystem::path(app.courseFile).parent_path() / "characters").string(); }
static void chrSync(App& app) { app.charName = app.chr.name; app.charFemale = app.chr.female(); }
static void chrInit(App& app) {
    if (app.chrReady) return;
    app.chrReady = true;
    const std::string pro = app.gameDir + "/Themes/Championship/Gary Golf.pro";   // the exe's default player record (title Golf Pro)
    if (!sg::charLoadFile(pro, app.chr)) {
        app.chr = sg::CharRec(); std::snprintf(app.chr.title, 16, "Golf Pro"); std::snprintf(app.chr.name, 16, "Gary Golf"); app.chr.flagB = 0x0a; app.chr.head = 8; app.chr.traits = 0x14;
    }
    chrSync(app);
}
static void loadCharArt(App& app) {
    const std::string i = app.gameDir + "/Interface/", h = app.gameDir + "/Heads/";
    ui::loadPcx(i + "TransPopups.pcx", app.ballArt, false, -1, i + "TransPopups_A.pcx");   // the golf ball piece (0,300,140,140) behind portraits
    ui::loadPcx(i + "CGButtons.pcx", app.cgBtn, true); ui::loadPcx(i + "CustGolfBckgrnd.pcx", app.custBg[0], false); ui::loadPcx(i + "CustGlfBckMale.pcx", app.custBg[1], false);
    ui::loadPcx(i + "HeadBodyBck.pcx", app.headWin, false); ui::loadPcx(i + "HeadSelect.pcx", app.headSel, false, 0xF800F8);   // this sheet keys on (248,0,248)
    ui::loadPcx(h + "sim_FEMALE_all_expressionsflat.pcx", app.headExp[0], true); ui::loadPcx(h + "sim_MALE_all_expressionsflat.pcx", app.headExp[1], true);
    ui::loadPcx(h + "golfballhalopage_female.pcx", app.headHalo[0], true); ui::loadPcx(h + "golfballhalopage_male .pcx", app.headHalo[1], true);
    for (int g = 0; g < 2; g++) {   // custom heads: every Heads/*.pcx whose name starts with F (female) or M (male), 140 x 420 with three 140 x 140 faces
        std::vector<std::string> names; std::error_code ec;
        for (const auto& e : std::filesystem::directory_iterator(h, ec)) {
            const std::string n = e.path().filename().string();
            if (n.size() > 4 && (n[0] == (g ? 'M' : 'F') || n[0] == (g ? 'm' : 'f')) && (n.substr(n.size() - 4) == ".pcx" || n.substr(n.size() - 4) == ".PCX") && n.find("expressions") == std::string::npos && n.find("halopage") == std::string::npos && n.find("Template") == std::string::npos) names.push_back(n);
        }
        std::sort(names.begin(), names.end());
        app.headCustom[g].assign(names.size(), ui::Image());
        for (size_t k = 0; k < names.size(); k++) ui::loadPcx(h + names[k], app.headCustom[g][k], true);
    }
}
// One face: expr 0 happy, 1 neutral, 2 angry (docs/DECODE_FACES.md section 1). Built-in heads come from the expression sheet (90 x 120 cells) or the halo page (140 x 140);
// custom heads are 140 x 140 cells of their own file. Returns false when the art is missing. halo selects the 140 x 140 version (centred on x, y given as its top left).
static bool drawFace(App& app, bool female, int head, int expr, float x, float y, bool halo) {
    const int g = female ? 0 : 1;
    if (head < 19) {
        const float col = (float)(head >> 1), band = (float)(head & 1);
        if (halo) { if (!app.headHalo[g].tex) return false; ui::drawImage(app.headHalo[g], x, y, col * 140, band * 420 + 140 * expr, 140, 140); }
        else { if (!app.headExp[g].tex) return false; ui::drawImage(app.headExp[g], x, y, col * 100, band * 372 + (expr == 0 ? 4 : expr == 1 ? 128 : 252), 90, 120); }
        return true;
    }
    const size_t k = (size_t)(head - 19);
    if (k >= app.headCustom[g].size() || !app.headCustom[g][k].tex) return false;
    ui::drawImage(app.headCustom[g][k], halo ? x : x - 25, halo ? y : y - 10, 0, 140.0f * expr, 140, 140);
    return true;
}
static int cuHeadCount(const App& app) { return 19 + (int)app.headCustom[app.chr.female() ? 0 : 1].size(); }
static void openCustomise(App& app) {
    chrInit(app);
    if (!app.cgBtn.tex) loadCharArt(app);
    if (!app.chr.flags) app.chr.flags = 7;   // an unedited record starts with the three clothing toggles on
    app.chrUndo = app.chr; app.cuHover = -1; app.cuFrames = 0; app.cuFace = false; app.cuEdit = -1; app.cuLoad = false; app.screen = App::ScreenCustomise; app.hover = -1;
}
static int cuHit(const App& app, float vx, float vy) {
    int best = -1; float bd = 1e9f;
    for (int i = 0; i < 20; i++) {
        const float dx = std::fabs(vx - kCuPos[i][0]), dy = std::fabs(vy - kCuPos[i][1]);
        const float a = i < 8 ? dx / 3 : dx, b = dy;
        const float d = b < a ? (b + 2 * a) / 2 : (a + 2 * b) / 2;   // the exe's distance metric
        if (d < 40 && d < bd) { bd = d; best = i; }
    }
    if (best == 15 && !app.chr.female()) return -1;   // hair colour cannot be changed for men
    return best;
}
static const float kCuNavy[3] = {0.06f, 0.06f, 0.26f};
static ui::Image* bodyPortrait(App& app, const sg::BodyLook& l);
static sg::BodyLook chrLook(const App& app);
static void drawBodyUi(GlSprite* sp, int view, int frame, float x, float y, float hgt);
static void drawCustomise(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    const sg::CharRec& c = app.chr;
    ui::drawImage(app.custBg[c.female() ? 0 : 1], 0, 0);
    const int hv = app.cuFace || app.cuEdit >= 0 || app.cuLoad ? -1 : app.cuHover;
    // Buttons that are on show their yellow cut; the hovered one too.
    auto yellow = [&](int idx) {
        if (idx < 3) ui::drawImage(app.cgBtn, 157, 136 + 35.0f * idx, 150, 300 + 50.0f * idx, 112, 35);
        else if (idx < 8) ui::drawImage(app.cgBtn, 39, 116 + 28.0f * (idx - 3), 150, 50 + 50.0f * (idx - 3), 112, 28);
        else if (idx == 8) ui::drawImage(app.cgBtn, 32, 4, 0, 0, 35, 37);
        else if (idx == 9) ui::drawImage(app.cgBtn, 242, 4, 100, 0, 35, 37);
        else if (idx == 12) ui::drawImage(app.cgBtn, 310, 38, 350, 250, 50, 50);
        else if (idx == 10) ui::drawImage(app.cgBtn, 310, 88, 350, 300, 50, 50);
        else if (idx == 17) ui::drawImage(app.cgBtn, 310, 138, 350, 350, 50, 50);
        else if (idx == 16) ui::drawImage(app.cgBtn, 436, 12, 350, 0, 50, 50);
        else if (idx == 11) ui::drawImage(app.cgBtn, 436, 62, 350, 50, 50, 50);
        else if (idx == 15) ui::drawImage(app.cgBtn, 436, 112, 350, 100, 50, 50);
        else if (idx == 13) ui::drawImage(app.cgBtn, 436, 162, 350, 150, 50, 50);
        else if (idx == 14) ui::drawImage(app.cgBtn, 436, 212, 350, 200, 50, 50);
        else if (idx == 18) ui::drawImage(app.cgBtn, 732, 7, 500, 0, 60, 61);
        else if (idx == 19) ui::drawImage(app.cgBtn, 732, 188, 500, 100, 60, 61);
    };
    // Preview window: sky and grass, then the head (PLACEHOLDER: the exe also draws two small walking bodies and a child sprite here). The window is wider than the
    // gap between the round buttons and the tabs, so the idle buttons are printed again from the background before the lit ones.
    ui::drawImage(app.headWin, 336, 20);
    { // The preview window holds the stack the exe draws: the body sheet at (355,116), the head cell at (336,20), and two small walking figures at (315,239) and (335,239) (frame 5 of the normal walk,
      // views 0 and 4; docs/DECODE_CUSTOMISE.md 3.3 and DECODE_BODIES.md). The figures are drawn at their own size, not zoomed (the exe's zoom variable is unknown).
      const sg::BodyLook l = chrLook(app);
      if (ui::Image* bi = bodyPortrait(app, l)) ui::drawImage(*bi, 355, 116, 0, 0, 60, 120);
      drawFace(app, c.female(), c.head, 1, 336, 20, false);
      if (const BodySet* bs = bodySetFor(app, l)) { drawBodyUi(bs->body[0], 0, 5, 315, 239, 35); drawBodyUi(bs->body[0], 4, 5, 335, 239, 35); } }
    for (int k = 0; k < 3; k++) ui::drawImage(app.custBg[c.female() ? 0 : 1], 310, 38 + 50.0f * k, 310, 38 + 50.0f * k, 50, 50);
    for (int k = 0; k < 5; k++) ui::drawImage(app.custBg[c.female() ? 0 : 1], 436, 12 + 50.0f * k, 436, 12 + 50.0f * k, 50, 50);
    for (int t = 0; t < 3; t++) if (c.flags >> t & 1) yellow(t);
    for (int t = 0; t < 5; t++) if (c.traits >> t & 1) yellow(3 + t);
    if (hv >= 0) yellow(hv);
    // Name box: name, title, marital status, age group (hover colours as in the exe).
    const int row = hv < 0 && !app.cuFace && app.cuEdit < 0 && !app.cuLoad && std::fabs(app.cuMx - 154) < 80 && std::fabs(app.cuMy - 64) < 40 ? (int)((app.cuMy - 24) / 20) : -1;
    auto col = [&](int r, float* rgb) { const bool h = row == r; rgb[0] = h && r == 0 ? 0.97f : h ? 1.0f : kCuNavy[0]; rgb[1] = h && r == 0 ? 0.8f : h ? 1.0f : kCuNavy[1]; rgb[2] = h && r == 0 ? 0.0f : h ? 1.0f : kCuNavy[2]; };
    float rgb[3];
    col(0, rgb); app.font.drawCentered(154, 37, std::string(c.name) + (c.flags & 8 ? " Jr." : ""), 15, rgb[0], rgb[1], rgb[2]);
    col(1, rgb); app.font.drawCentered(154, 61, c.title, 13, rgb[0], rgb[1], rgb[2]);
    col(2, rgb); app.font.drawCentered(154, 82, c.marital() >= 0 ? kCuMarital[c.marital()] : "Single", 13, rgb[0], rgb[1], rgb[2]);
    col(3, rgb); app.font.drawCentered(154, 103, c.age() >= 0 ? kCuAge[c.age()] : "Young", 13, rgb[0], rgb[1], rgb[2]);
    // Left and right button labels.
    for (int i = 0; i < 5; i++) { const bool on = c.traits >> i & 1; const float g = on ? 0.06f : 0.5f; app.font.drawCentered(93, 116 + 28.0f * i + 19, kCuTrait[i], 12, g, g, on ? 0.26f : 0.5f); }
    for (int i = 0; i < 3; i++) { const bool on = c.flags >> i & 1; const float g = on ? 0.06f : 0.5f; app.font.drawCentered(212, 136 + 35.0f * i + 23, kCuToggle[i], 12, g, g, on ? 0.26f : 0.5f); }
    // The skills panel (read only for the player): ten rows, value boxes and names. The values are the port's 0 to 15 skill levels shown as the exe's 0 to 100 scale (PLACEHOLDER mapping).
    app.font.drawCentered(585, 36, "Golf Skill Levels", 15, 0, 0, 0);
    for (int i = 0; i < 10; i++) {
        const float y = 61 + 16.0f * i; const int v = app.skills.v[i] * 100 / 15;
        ui::fillRect(498, y - 3, 42, 14, 0, 0, 0, 1);
        ui::fillRect(499, y - 2, 40, 12, v ? 0.97f : 0.13f, v ? 0.63f : 0.1f, v ? 0.25f : 0.2f, 1);
        if (v) app.font.draw(501, y + 8, std::to_string(v), 11, 0, 0, 0);
        app.font.draw(546, y + 9, kCuSkill[i], 12, v ? 0.0f : 0.5f, v ? 0.0f : 0.5f, v ? 0.0f : 0.5f);
    }
    // Dialogue table: label on the dark side, the saying on the cream side (grey when the slot is empty).
    for (int r = 0; r < 20; r++) {
        const float y = 271 + 16.0f * r;
        app.font.draw(52, y + 12, kCuRow[r], 12, 1, 1, 1);
        const bool has = c.dialogue[r][0] != 0;
        app.font.draw(412, y + 12, has ? c.dialogue[r] : "(default saying)", 12, has ? kCuNavy[0] : 0.5f, has ? kCuNavy[1] : 0.5f, has ? kCuNavy[2] : 0.5f);
    }
    // Tooltip after the pointer rests on a button.
    if (hv >= 0 && app.cuFrames > 10 && kCuTip[hv][0]) { const float w = app.font.width(kCuTip[hv], 12) + 12; ui::fillRect(app.cuMx + 12, app.cuMy + 8, w, 18, 1, 1, 0.8f, 0.95f); app.font.draw(app.cuMx + 18, app.cuMy + 21, kCuTip[hv], 12, 0, 0, 0); }
    // Face picker: ten halo balls per page over the HeadSelect panel.
    if (app.cuFace) {
        ui::drawImage(app.headSel, 0, 258, 0, 258, 800, 342);
        const int n = cuHeadCount(app);
        for (int s = 0; s < 10; s++) {
            const float bx = 100 + 120.0f * (s % 5) - 6, by = 268 + 142.0f * (s / 5);
            ui::drawImage(app.headSel, bx, by, app.cuFaceHover == s ? 150.0f : 0.0f, 0, 133, 137);
            if (app.cuFacePage + s < n) drawFace(app, c.female(), app.cuFacePage + s, 0, bx - 4, by - 1, true);
        }
        if (app.cuFacePage > 0) ui::drawImage(app.headSel, 10, 425, app.cuFaceHover == 11 ? 432.0f : 519.0f, 105, 87, 152);
        if (app.cuFacePage + 10 < n) ui::drawImage(app.headSel, 707, 284, app.cuFaceHover == 10 ? 614.0f : 707.0f, 0, 93, 152);
    }
    if (app.cuEdit >= 0) {   // single line text box (the exe's box is at (200,32); the port centres it)
        const int maxLen = app.cuEdit >= 100 ? 48 : 16;
        const float w = std::min(760.0f, maxLen * 12.0f + 32), x = app.cuEdit >= 100 ? 20.0f : 200.0f, y = app.cuEdit >= 100 ? 300.0f : 32.0f;
        ui::fillRect(x - 1, y - 1, w + 2, 50, 1, 1, 1, 1); ui::fillRect(x + 1, y + 1, w + 2, 50, 0, 0, 0, 1); ui::fillRect(x, y, w, 48, 0.42f, 0.7f, 0.9f, 1);
        app.font.draw(x + 4, y + 18, app.cuEdit == 0 ? "New name: " : app.cuEdit == 1 ? "Profession: " : "New text: ", 12, 1, 1, 1);
        ui::fillRect(x + 16, y + 23, w - 32, 22, 1, 1, 1, 1);
        app.font.draw(x + 20, y + 40, app.cuBuf + "_", 13, 0, 0, 0);
    }
    if (app.cuLoad) {   // "Pick one" list of character files
        ui::fillRect(90, 14, 420, 12.0f + 18.0f * (float)std::max<size_t>(1, std::min<size_t>(20, app.cuFiles.size())) + 22, 0.42f, 0.7f, 0.9f, 0.98f);
        app.font.draw(100, 34, app.cuFiles.empty() ? "No character files found." : "Pick one:", 13, 1, 1, 1);
        for (size_t k = 0; k < app.cuFiles.size() && k < 20; k++) app.font.draw(100, 54 + 18.0f * (float)k, std::filesystem::path(app.cuFiles[k]).stem().string(), 13, 1, 1, 1);
    }
    if (!app.toast.empty() && SDL_GetTicks() / 1000.0 < app.toastUntil) { const float w = app.font.width(app.toast, 14) + 24; ui::fillRect(400 - w / 2, 250, w, 26, 0.1f, 0.1f, 0.3f, 0.9f); app.font.drawCentered(400, 268, app.toast, 14, 1, 1, 0.8f); }
    ui::endScreen();
}
static void cuListFiles(App& app) {
    app.cuFiles.clear(); std::error_code ec;
    auto scan = [&](const std::filesystem::path& d) { for (const auto& e : std::filesystem::directory_iterator(d, ec)) { std::string x = e.path().extension().string(); for (char& ch : x) ch = (char)std::tolower((unsigned char)ch); if (x == ".pro" || x == ".chr" || x == ".glf") app.cuFiles.push_back(e.path().string()); } };
    scan(charDir(app));
    for (const auto& e : std::filesystem::directory_iterator(app.gameDir + "/Themes", ec)) if (e.is_directory()) scan(e.path());
    std::sort(app.cuFiles.begin(), app.cuFiles.end(), [](const std::string& a, const std::string& b) { return std::filesystem::path(a).stem().string() < std::filesystem::path(b).stem().string(); });
}
static void cuCommitEdit(App& app) {
    const int t = app.cuEdit; app.cuEdit = -1;
    if (t == 0 && !app.cuBuf.empty()) { std::snprintf(app.chr.name, 16, "%s", app.cuBuf.c_str()); chrSync(app); }
    else if (t == 1 && !app.cuBuf.empty()) std::snprintf(app.chr.title, 16, "%s", app.cuBuf.c_str());
    else if (t >= 100) std::snprintf(app.chr.dialogue[t - 100], 50, "%s", app.cuBuf.c_str());   // no emptiness check: an empty text clears the saying
}
static void customiseEvent(App& app, const SDL_Event& e) {
    sg::CharRec& c = app.chr;
    if (app.cuEdit >= 0) {
        if (e.type == SDL_TEXTINPUT) { const size_t mx = app.cuEdit >= 100 ? 48 : 15; for (const char* p = e.text.text; *p; p++) if ((unsigned char)*p >= 32 && (unsigned char)*p < 127 && app.cuBuf.size() < mx) app.cuBuf += *p; }
        else if (e.type == SDL_KEYDOWN) { const SDL_Keycode k = e.key.keysym.sym; if (k == SDLK_RETURN || k == SDLK_KP_ENTER) cuCommitEdit(app); else if (k == SDLK_ESCAPE) { app.cuBuf.clear(); cuCommitEdit(app); } else if (k == SDLK_BACKSPACE && !app.cuBuf.empty()) app.cuBuf.pop_back(); }
        return;
    }
    if (e.type != SDL_MOUSEMOTION && e.type != SDL_MOUSEBUTTONDOWN) return;
    const bool click = e.type == SDL_MOUSEBUTTONDOWN;
    const float vx = app.view.toVirtualX((click ? e.button.x : e.motion.x) * app.dpi), vy = app.view.toVirtualY((click ? e.button.y : e.motion.y) * app.dpi);
    app.cuMx = vx; app.cuMy = vy;
    const bool left = click && e.button.button == SDL_BUTTON_LEFT, any = click && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT);
    if (app.cuLoad) {
        if (left) {
            const int k = (int)((vy - 45) / 18);
            if (vx > 90 && vx < 510 && k >= 0 && k < (int)app.cuFiles.size() && k < 20) {
                sg::CharRec n;
                if (sg::charLoadFile(app.cuFiles[(size_t)k], n)) { if (n.head >= cuHeadCount(app)) n.head = 8; app.chr = n; chrSync(app); app.chrUndo = n; snd(app, "Interface/Button2.wav"); }
                else toastMsg(app, "That character file could not be read");
            }
            app.cuLoad = false;
        } else if (any) app.cuLoad = false;
        return;
    }
    if (app.cuFace) {
        int hit = 0, n = cuHeadCount(app);
        int slot = -1;
        for (int s = 0; s < 10; s++) { const float cx = 100 + 120.0f * (s % 5) - 6 + 60, cy = 268 + 142.0f * (s / 5) + 60; if (std::hypot(vx - cx, vy - cy) < 60) slot = s; }
        if (std::hypot(vx - 778, vy - 363) < 60) hit = 10; else if (std::hypot(vx - 18, vy - 495) < 60) hit = 11;
        app.cuFaceHover = slot >= 0 ? slot : hit >= 10 ? hit : -1;
        if (!any) return;
        if (vy < 258 || e.button.button != SDL_BUTTON_LEFT) { app.cuFace = false; return; }
        if (hit == 10) { if (app.cuFacePage < 70) app.cuFacePage += 10; app.cuFacePage = std::max(0, std::min(app.cuFacePage, n - 10)); }
        else if (hit == 11) { if (app.cuFacePage > 0) app.cuFacePage -= 10; app.cuFacePage = std::max(0, std::min(app.cuFacePage, n - 10)); }
        else if (slot >= 0 && app.cuFacePage + slot < n) { c.head = (uint8_t)(app.cuFacePage + slot); app.cuFace = false; snd(app, "Interface/Button2.wav"); }
        return;
    }
    const int hit = cuHit(app, vx, vy);
    if (hit != app.cuHover) { app.cuHover = hit; app.cuFrames = 0; }
    if (!any) return;
    const bool up = left;   // left click steps forward, right click back
    if (hit < 0) {   // the name box rows and the dialogue table
        if (std::fabs(vx - 154) < 80 && std::fabs(vy - 64) < 40) {
            const int row = (int)((vy - 24) / 20);
            if (row == 0 || row == 1) { app.cuEdit = row; app.cuBuf.clear(); }
            else if (row == 2) c.cycleMarital(up); else if (row == 3) c.cycleAge(up);
        } else if (std::fabs(vx - 578) < 178 && vy > 269) {
            const int row = (int)((vy - 270) / 16);
            if (row >= 0 && row < 20) { app.cuEdit = 100 + row; app.cuBuf = c.dialogue[row]; }
        }
        return;
    }
    snd(app, "Interface/Button2.wav");
    switch (hit) {
        case 0: case 1: case 2: c.flags ^= 1u << hit; break;
        case 3: case 4: case 5: case 6: case 7: c.traits ^= (uint8_t)(1u << (hit - 3)); break;
        case 8: cuListFiles(app); app.cuLoad = true; break;
        case 9: {
            if (!sg::charNameValid(c.name)) { toastMsg(app, "That is not a valid file name"); break; }
            std::error_code ec; std::filesystem::create_directories(charDir(app), ec);
            const std::string path = charDir(app) + "/" + c.name + ".pro";
            for (int k = 0; k < 10; k++) c.skills[k] = (uint8_t)std::min(10, app.skills.v[k] * 10 / 15);
            toastMsg(app, sg::charSaveFile(path, c) ? std::string("Character saved as ") + c.name + ".pro" : "The character could not be saved");
            break;
        }
        case 10: c.cycleBody(up); break;
        case 11: c.flags ^= 8; c.flags |= 0x80; break;
        case 12: app.cuFace = true; app.cuFacePage = (c.head / 10) * 10; app.cuFaceHover = -1; break;
        case 13: c.cycleShirt(up); break;
        case 14: c.cyclePants(up); break;
        case 15: c.cycleHair(up); break;
        case 16: c.cycleSkin(up); break;
        case 17: c.flagB ^= 0x80; c.flags |= 0x80; if (c.head >= cuHeadCount(app)) c.head = 0; chrSync(app); break;
        case 18: app.chr = app.chrUndo; chrSync(app); break;   // undo stays on the screen
        case 19: app.screen = App::ScreenPlay; app.hover = -1; chrSync(app); break;
    }
}

static void drawStatusDot(float cx, float cy, float r, int st) {
    const float c[3][3] = {{0.98f, 0.82f, 0.1f}, {0.5f, 0.5f, 0.48f}, {0.8f, 0.1f, 0.25f}};
    for (int j = -(int)r - 1; j <= (int)r + 1; j++) { const float hw = std::sqrt(std::max(0.0f, (r + 1) * (r + 1) - (float)(j * j))); ui::fillRect(cx - hw, cy + (float)j, 2 * hw, 1, 0.15f, 0.12f, 0.12f, 1); }
    for (int j = -(int)r; j <= (int)r; j++) { const float hw = std::sqrt(std::max(0.0f, r * r - (float)(j * j))); ui::fillRect(cx - hw, cy + (float)j, 2 * hw, 1, c[st][0], c[st][1], c[st][2], 1); }
    ui::fillRect(cx - 2, cy - 2, 2, 2, std::min(1.0f, c[st][0] + 0.35f), std::min(1.0f, c[st][1] + 0.35f), std::min(1.0f, c[st][2] + 0.35f), 1);
}
static void drawProperty(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.worldBase, 0, 0);
    app.font.draw(24, 36, app.switchMode ? "Which course will you play?" : "Where will you build your golf course?", 14, 0.1f, 0.1f, 0.35f);
    app.font.drawCentered(737, 32, app.switchMode ? (app.econ.sandbox ? std::string("Unlimited \xC2\xA7") : money((long long)app.econ.cash)) : app.sandboxChoice ? "Unlimited \xC2\xA7" : money(kStartFunds), 17, 0.1f, 0.1f, 0.35f);
    for (int i = 0; i < 16; i++) {
        const Property& p = kProperties[i];
        const Rect r = propertyCard(i);
        const bool ok = canAfford(app, i);
        const float a = ok ? 1.0f : 0.5f;
        ui::drawImage(app.themeIcons[p.theme], r.x + 1, r.y - 1, ok ? 200.0f : 0.0f, 0, 52, 52);
        const float cx = r.x + 62 + (r.w - 74) * 0.5f;
        app.font.drawCentered(cx, r.y + 15, p.name, 16, 0.08f, 0.08f, 0.3f, ok ? 1.0f : 0.55f);
        app.font.drawCentered(cx, r.y + 26, p.bonus, 11, 0.2f, 0.2f, 0.35f, ok ? 1.0f : 0.55f);
        char line[64]; std::snprintf(line, sizeof line, "%d acres: ", propAcresShown(app, i));
        if (app.switchMode && (app.careerOwned >> i & 1)) app.font.drawCentered(cx, r.y + 43, i == app.curProp ? "Playing now" : "Already purchased", 12, 0.1f, 0.35f, 0.15f);
        else if (!app.sandboxChoice && !(app.switchMode && app.econ.sandbox)) app.font.drawCentered(cx, r.y + 43, std::string(line) + money(propPrice(app, i)), 12, 0.25f, 0.18f, 0.1f, ok ? 1.0f : 0.55f);
        if (app.switchMode && i == app.curProp) { ui::fillRect(r.x, r.y, r.w, 2, 0.1f, 0.4f, 0.1f, 0.9f); ui::fillRect(r.x, r.y + r.h - 2, r.w, 2, 0.1f, 0.4f, 0.1f, 0.9f); }
        if (app.hover == i) { ui::fillRect(r.x, r.y, r.w, r.h, 1, 1, 0.4f, 0.22f); }
        // Status ball at the right end of the card and a pin on the globe: yellow available, grey insufficient funds, red already purchased (real screenshots; the sprites are not located, PLACEHOLDER discs)
        const int st = (app.switchMode && (app.careerOwned >> i & 1)) ? 2 : ok ? 0 : 1;
        drawStatusDot(r.x + r.w - 9, r.y + 14, 5.0f, st);
        drawStatusDot((float)ui_screens2::worldmap::kSites[i].pinX, (float)ui_screens2::worldmap::kSites[i].pinY, 5.0f, st);
    }
    if (app.hover < 0 || app.hover > 15) {   // the legend box, bottom left
        static const char* kLg[3] = {"Available", "Insufficient funds", "Already purchased"};
        static const float lx[3] = {26, 124, 270};
        for (int k = 0; k < 3; k++) { drawStatusDot(lx[k], 563, 5.0f, k == 0 ? 0 : k == 1 ? 1 : 2); app.font.draw(lx[k] + 12, 568, kLg[k], 11, 0.1f, 0.1f, 0.3f); }
    }
    // Reset World (before the game starts) or Save Game (in the course switch screen), and Load Game: strips cut from WorldButton.pcx (blue idle, yellow under the pointer).
    if (app.worldBtn.tex) {
        const auto cut = [&](float dx, float dy, float sx, float sy, float w, float h) { ui::drawImage(app.worldBtn, dx, dy, sx, sy, w, h); };
        const bool hotA = app.hover == 101, hotL = app.hover == 102;
        if (app.switchMode) cut(724, 458, hotA ? 400.0f : 550.0f, 458, 76, 64);
        else cut(724, 408, hotA ? 724.0f : 250.0f, 408, 76, 114);
        cut(724, 408, hotL ? 400.0f : 550.0f, 408, 76, 50);
    }
    if (app.hover == 101) app.font.draw(26, 568, app.switchMode ? "Save Game" : "Reset World: roll a new list of properties", 14, 0.1f, 0.1f, 0.3f);
    else if (app.hover == 102) app.font.draw(26, 568, "Load Game", 14, 0.1f, 0.1f, 0.3f);
    if (app.hover >= 0 && app.hover < 16) {
        const Property& p = kProperties[app.hover];
        char l[160]; std::snprintf(l, sizeof l, "%s, %d acres. Bonus: %s.", p.name, propAcresShown(app, app.hover), p.bonus);
        app.font.draw(26, 568, l, 14, 0.1f, 0.1f, 0.3f);
        if (!canAfford(app, app.hover)) app.font.draw(26, 586, "Not enough funds.", 12, 0.55f, 0.1f, 0.1f);
    }
    if (app.worldMsgUntil > SDL_GetTicks() / 1000.0 && !app.worldMsg.empty()) {   // the exe's dark red message box at (200,200)
        const std::vector<std::string> ln = wrapText(app, app.worldMsg, 15, 370);
        const float h = 20.0f + 20.0f * (float)ln.size();
        ui::fillRect(200, 200, 400, h, 0.45f, 0.05f, 0.05f, 0.96f);
        for (size_t i = 0; i < ln.size(); i++) app.font.drawCentered(400, 224 + (float)i * 20, ln[i], 15, 1, 1, 1);
    }
    if (app.confirmIdx >= 0) {   // purchase confirm (wording is the port's own; the exe's box is at (204,276,192,48))
        ui::fillRect(190, 250, 420, 100, 0.12f, 0.1f, 0.3f, 0.97f);
        const std::string q = (app.switchMode && app.econ.sandbox) || (!app.switchMode && app.sandboxChoice) ? "Build at " + std::string(kProperties[app.confirmIdx].name) + "?" : "Buy " + std::string(kProperties[app.confirmIdx].name) + " for " + money(propPrice(app, app.confirmIdx)) + "?";
        app.font.drawCentered(400, 290, q, 17, 1, 1, 0.8f);
        for (int b = 0; b < 2; b++) { const bool hot = app.hover == 110 + b; ui::fillRect(b ? 405.0f : 215.0f, 308, 180, 30, hot ? 0.45f : 0.25f, hot ? 0.4f : 0.22f, hot ? 0.7f : 0.5f, 0.97f); app.font.drawCentered(b ? 495.0f : 305.0f, 329, b ? "No" : "Yes", 16, 1, 1, 1); }
    }
    if (app.goTarget >= 0) {   // "We're off to X!" then "... one moment please ..." (white, centred near x 300)
        ui::fillRect(204, 276, 192, 48, 0.1f, 0.1f, 0.3f, 0.97f);
        app.font.drawCentered(300, 297, std::string("We're off to ") + kProperties[app.goTarget].name + "!", 14, 1, 1, 1);
        app.font.drawCentered(300, 315, "... one moment please ...", 12, 1, 1, 1);
    }
    ui::endScreen();
}

// The Course Report (the original's Information menu, Course Report). Drawn from the disc's coursereport.pcx pieces: a header band,
// repeated row strips, a total strip and a legend. Column layout read off that art. Yards use 0.15 yards per world unit (so a full
// drive of 900 units is 135 yards, close to the 128 yard average drive seen in a Hole Stats screenshot): a placeholder scale.
static std::string num(double v, int dec) {
    if (dec == 0) { std::string out; long long n = std::llround(v); std::string d = std::to_string(n < 0 ? -n : n); for (size_t k = 0; k < d.size(); k++) { if (k && (d.size() - k) % 3 == 0) out += ','; out += d[k]; } return (n < 0 ? "-" : "") + out; }
    char b[48]; std::snprintf(b, sizeof b, "%.*f", dec, v);
    return b;
}

static void drawReport(App& app) {
    if (!app.uiOk || !app.reportArt.tex) return;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    static const float cx[12] = {112, 157, 190, 233, 283, 330, 375, 422, 468, 571, 641, 716}, cw[12] = {41, 29, 39, 46, 43, 41, 43, 42, 99, 66, 71, 74};
    static const char* head[12] = {"Yds", "Par", "Avg", "Time", "Fun", "+Len", "+Acc", "+Img", "Type", "Avg.Fee", "Revenue", "Profit"};
    const int n = (int)app.holes.size();
    const int rowH = 22, top = 104, bodyY = 108;
    const float totalY = (float)(bodyY + n * rowH + 6);
    ui::fillRect(0, 100, 800, totalY - 100, 148 / 255.0f, 150 / 255.0f, 198 / 255.0f, 1);
    ui::drawImage(app.reportArt, 0, 0, 0, 0, 800, (float)top);
    app.font.drawCentered(323, 53, "COURSE REPORT", 26, 0.15f, 0.12f, 0.3f);
    for (int c = 0; c < 12; c++) app.font.drawCentered(cx[c] + cw[c] / 2, 89, head[c], 12, 0.1f, 0.1f, 0.3f);
    if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 40, app.difficulty)); }
    double tY = 0, tPar = 0, tStr = 0, tSec = 0, tMood = 0, tRev = 0, tPlays = 0, tProf = 0, tLen = 0, tAcc = 0, tImg = 0;
    const double upkeepShare = n ? (app.econ.upkeepPaid + app.econ.wagesPaid) / n : 0;
    for (int i = 0; i < n; i++) {
        const float y = (float)(bodyY + i * rowH);
        ui::drawImage(app.reportArt, 0, y, 0, i % 2 ? 168.0f : 128.0f, 800, 19);
        const HoleRoute& r = app.holes[(size_t)i];
        const HoleRating& rt = app.ratings[(size_t)i];
        const App::HoleStat hs = i < (int)app.holeStats.size() ? app.holeStats[(size_t)i] : App::HoleStat();
        const double yds = r.length * 0.15, avg = hs.plays ? hs.strokes / hs.plays : 0, mins = hs.plays ? hs.seconds / hs.plays * 0.35 : 0;   // 0.35 game minutes per sim second: placeholder
        const double fun = hs.plays ? hs.mood / hs.plays : 0, fee = hs.plays ? hs.revenue / hs.plays : 0, profit = hs.revenue - upkeepShare;
        tY += yds; tPar += r.par; tStr += avg; tSec += mins; tMood += fun; tRev += hs.revenue; tPlays += hs.plays; tProf += profit; tLen += rt.len; tAcc += rt.acc; tImg += rt.img;
        auto cell = [&](int c, const std::string& s, int tint) {   // tint: 0 none, 1 green, 2 red
            if (tint) ui::drawImage(app.reportArt, cx[c], y + 2, cx[c], tint == 2 ? 205.0f : 240.0f, cw[c], 15);
            app.font.drawCentered(cx[c] + cw[c] / 2, y + 14, s, 12, tint ? 1.0f : 0.1f, tint ? 1.0f : 0.1f, tint ? 1.0f : 0.3f);
        };
        app.font.draw(16, y + 14, "Hole " + std::to_string(i + 1), 12, 0.1f, 0.1f, 0.3f);
        cell(0, num(yds, 0), 0); cell(1, std::to_string(r.par), 0);
        cell(2, hs.plays ? num(avg, 2) : "-", 0); cell(3, hs.plays ? num(mins, 0) + "m" : "-", 0);
        cell(4, hs.plays ? num(fun, 0) + "%" : "-", hs.plays ? (fun >= 70 ? 1 : fun < 45 ? 2 : 0) : 0);
        cell(5, num(rt.len, 2), 0); cell(6, num(rt.acc, 2), 0); cell(7, num(rt.img, 2), 0);
        cell(8, rt.type, 0);
        cell(9, hs.plays ? num(fee, 0) : "-", 0); cell(10, num(hs.revenue, 0), 0); cell(11, num(profit, 0), profit < 0 ? 2 : (profit > 0 ? 1 : 0));
    }
    ui::fillRect(8, totalY, 784, 48, 148 / 255.0f, 150 / 255.0f, 198 / 255.0f, 1);   // the keyed gap under the total row is lavender in the real game
    ui::drawImage(app.reportArt, 0, totalY, 0, 420, 800, 92);
    ui::fillRect(0, totalY + 28, 24, 26, 148 / 255.0f, 150 / 255.0f, 198 / 255.0f, 1);   // the art carries some layout numbers in its margin
    {
        const float y = totalY + 8 + 14;
        auto cell = [&](int c, const std::string& s) { app.font.drawCentered(cx[c] + cw[c] / 2, y, s, 12, 0.1f, 0.1f, 0.3f); };
        app.font.draw(16, y, "Total", 12, 0.1f, 0.1f, 0.3f);
        const double k = n ? 1.0 / n : 0;
        cell(0, num(tY, 0)); cell(1, num(tPar, 0)); cell(2, tPlays ? num(tStr, 2) : "-"); cell(3, tPlays ? num(tSec, 0) + "m" : "-");
        cell(4, tPlays ? num(tMood * k, 0) + "%" : "-"); cell(5, num(tLen * k, 2)); cell(6, num(tAcc * k, 2)); cell(7, num(tImg * k, 2));
        cell(9, tPlays ? num(tRev / tPlays, 0) : "-"); cell(10, num(tRev, 0)); cell(11, num(tProf, 0));
        const float ly = totalY + 55 + 11;
        app.font.draw(138, ly, "Top 100 Hole", 11, 0.1f, 0.1f, 0.3f); app.font.draw(296, ly, "Top 18 Hole", 11, 0.1f, 0.1f, 0.3f); app.font.draw(448, ly, "Scenic Hole", 11, 0.1f, 0.1f, 0.3f);
    }
    ui::endScreen();
}



// ---- SGA evaluation and tournament screen (rules: docs/DECODE_TOURNAMENTS.md, sg/sga.h, sg/tournament.h) ----
static void loadPros(App& app) {
    if (app.prosTried) return;
    app.prosTried = true;
    sg::loadTourPros(app.gameDir + "/Themes/Standard/progolfers.dta", app.pros, nullptr);   // read from the disc at run time
}
static void openSgaScreen(App& app) {
    const sg::SgaInput in = sgaInput(app);
    app.sgaMode = 0;
    if (app.tourney.state() == sg::Tournament::State::Offered && app.tourney.reopenOffer(in)) app.sgaMode = 1;
    app.sgaIn = in; app.sgaLast = sg::evaluateCourse(in);
    { sg::GoalEvent e; e.kind = sg::GoalEvent::CourseRated; e.total = app.sgaLast.total; goalEvent(app, e); }
    app.screen = App::ScreenSga; app.hover = -1;
}
// Accepts the pending offer and plays the tournament out. Each entrant's strokes come from the shot simulation on this course (see playHole below);
// everything around it is the exe's.
static int g_revealForce = -1; static bool g_scriptedRun = false;   // test hooks: --reveal N shows N holes; scripted renders skip the reveal
static void runTournament(App& app) {
    loadPros(app);
    if (app.pros.empty()) { say(app, "progolfers.dta was not found on the disc folder"); return; }
    std::vector<int> pars; for (const HoleRoute& r : app.holes) pars.push_back(r.par);
    int skillSum = 0; for (int k = 0; k < 10; k++) skillSum += app.skills.v[k];
    sg::SimpleRng rng; rng.state = app.seed * 2654435761u + (uint32_t)app.econ.day * 40503u + 17u;
    const sg::RandFn fn = [&](int n) { return rng(n); };
    std::vector<int> acc;
    if (!app.tourney.accept(app.pros, pars, app.difficulty, (int)(app.econ.cash / 100), app.charName, skillSum, fn, &acc)) { say(app, "There is no tournament offer to accept"); return; }
    app.tName = app.tourney.name() ? app.tourney.name() : ""; app.tPrize = app.tourney.firstPrizeThousands(); app.tPars = pars;
    if (!app.champ) { sg::GoalEvent e; e.kind = sg::GoalEvent::TournamentAccepted; e.prizeThousands = app.tPrize; goalEvent(app, e); }   // championship play stamps nothing
    app.tStrokes.clear();
    const std::vector<sg::Entrant> fld = app.tourney.field();
    // Every entrant plays every hole of this course through the same shot simulation the golfers on the course use, with the pro's own ten skills
    // (the player uses the character's skills). The simulation's own physics are still the port's placeholder, so the scores are real for THIS course
    // and these skills but not the exe's numbers.
    auto playHole = [&](const sg::GolferSkills& sk, int hole, uint32_t seed, int par) {
        sg::ShotSim sim; sim.skills = sk; sim.loop = false; applyFacilityEffects(app, sim);
        const std::vector<float> route = app.holes[(size_t)hole].route;
        sim.setRoute(&route); sim.init(app.terrain, seed);
        for (int i = 0; i < 20000 && !sim.finished; i++) sim.step(0.05f);
        return sim.finished ? std::max(1, sim.stroke) : par + 6;
    };
    for (const sg::Entrant& en : fld) {
        std::vector<int> st;
        sg::GolferSkills sk = app.skills;
        if (!en.isPlayer && en.proIndex >= 0 && en.proIndex < (int)app.pros.size()) for (int k = 0; k < 10; k++) sk.v[k] = app.pros[(size_t)en.proIndex].skills[k];
        for (size_t h = 0; h < pars.size(); h++) {
            if (h < app.holes.size() && !app.holes[h].route.empty()) st.push_back(playHole(sk, (int)h, app.seed * 2246822519u + (uint32_t)en.slot * 3266489917u + (uint32_t)h * 668265263u + 1u, pars[h]));
            else st.push_back(sg::placeholderStrokes(pars[h], en.skillSum, fn));
        }
        app.tourney.submitRound(en.slot, st); app.tStrokes[en.slot] = st;
    }
    app.tField = app.tourney.field();
    sg::TournamentResult res;
    if (app.tourney.finish(themeExe(app.theme), res)) {
        app.tResult = res;
        app.econ.earn(res.cashDeltaUnits * 100.0); app.fame += res.fameDelta;
        sg::GoalEvent e; e.kind = sg::GoalEvent::TournamentResult; e.place = res.playerPlace; e.prizeThousands = app.tPrize; e.prizeDollars = res.playerPrizeThousands * 1000L; e.theme = themeExe(app.theme); goalEvent(app, e);
        app.sgaMode = 2; app.screen = App::ScreenSga; app.tRevealStart = g_scriptedRun ? -1e9 : SDL_GetTicks() / 1000.0;
        say(app, "You placed " + std::to_string(res.playerPlace) + " in the " + app.tName, 8);
        addHighlight(app, app.charName + " places " + std::to_string(res.playerPlace) + " in tournament"); snd(app, res.playerPlace <= 3 ? "ApplauseGood.wav" : res.playerPlace > 10 ? "ApplauseBad.wav" : "Applause.wav", 0.6f); logEv(app, 0xe0, res.playerPlace);
    }
}
struct SgaBtn { float x, y, w, h; };
static SgaBtn sgaBtn(int i) { return {170.0f + i * 240.0f, 440, 220, 36}; }

// ---- Financial Report (layout from docs/UI_SCREENS.md section 1) ----
static std::string finNum(int units) {
    long long v = (long long)units * 100; const bool neg = v < 0; if (neg) v = -v;
    std::string d = std::to_string(v), o;
    for (size_t i = 0; i < d.size(); i++) { if (i && (d.size() - i) % 3 == 0) o += ','; o += d[i]; }
    return neg ? "-" + o : o;
}
static void drawFinance(App& app) {
    namespace fin = sg::ui_screens::fin;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.85f);
    if (app.finArt.tex) ui::drawImage(app.finArt, (float)fin::art.x, (float)fin::art.y, (float)fin::art.x, (float)fin::art.y, (float)fin::art.w, (float)fin::art.h);
    else ui::fillRect((float)fin::art.x, (float)fin::art.y, (float)fin::art.w, (float)fin::art.h, 0.75f, 0.78f, 0.7f, 1);
    app.font.draw((float)fin::title.x, (float)fin::title.y + 22, "FINANCIAL REPORT", 24, 0.15f, 0.12f, 0.3f);
    const int n = std::min(99, std::max(0, (app.econ.day - 1) / Economy::kMonthsPerYear));
    const int first = std::max(0, n - 8);
    for (int r = 0; r < fin::kRows; r++) app.font.drawCentered((float)fin::kLabelCx, (float)fin::labelY(r) + 12, fin::kRowLabels[r], 13, 0.1f, 0.1f, 0.3f);
    for (int c = 0; c < fin::kColumns && first + c <= n; c++) {
        const int y = first + c;
        app.font.drawCentered((float)fin::headerCx(c), (float)fin::kHeaderY + 13, std::to_string(fin::kYearBase + y), 13, 0.1f, 0.1f, 0.3f);
        const sg::costs::LedgerMonth ys = app.econ.ledger.yearSum(y * Economy::kMonthsPerYear + 7);
        for (int r = 0; r < fin::kRows; r++) {
            const int v = r == 8 ? ys.total() : ys.row[r];
            const std::string t = finNum(v);
            const float w = app.font.width(t, 13);
            app.font.draw((float)fin::valueRightX(c) - w, (float)fin::valueY(r) + 12, t, 13, v < 0 ? 0.5f : 0.1f, v < 0 ? 0.0f : 0.1f, v < 0 ? 0.0f : 0.3f);
        }
    }
    const bool hot = app.vmx >= fin::okHit.x && app.vmx < fin::okHit.x + fin::okHit.w && app.vmy >= fin::okHit.y && app.vmy < fin::okHit.y + fin::okHit.h;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, (float)fin::okPos.x, (float)fin::okPos.y, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- Membership Roster (layout from docs/UI_SCREENS.md section 2) ----
// PLACEHOLDER: the exe's 75 member names live in data this port has not located, so the pros list lends the names for now.
static std::string memberName(App& app, int id) {
    if (!app.prosTried) { app.prosTried = true; sg::loadTourPros(app.gameDir + "/Themes/Standard/progolfers.dta", app.pros, nullptr); }
    if (!app.pros.empty()) return app.pros[(size_t)(id - 1) % app.pros.size()].name;
    return "Golfer " + std::to_string(id);
}
static std::vector<int> rosterRows(App& app) {
    std::vector<int> v;
    if (!app.rosterReady) { app.roster.newGame(app.srng); app.rosterReady = true; }
    for (int i = 1; i <= sg::Roster::kPool; i++) if (app.roster.e[i].tier != sg::Tier::None || app.roster.e[i].resigned) v.push_back(i);
    std::sort(v.begin(), v.end(), [&](int a, int b) { return memberName(app, a) < memberName(app, b); });
    return v;
}
static void drawRoster(App& app) {
    namespace ro = sg::ui_screens::roster;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    if (app.rosterArt.tex) ui::drawImage(app.rosterArt, 0, 0, 0, 0, 800, 600); else ui::fillRect(0, 0, 800, 600, 0.75f, 0.78f, 0.7f, 1);
    const float R = 0.1f, G = 0.1f, B = 0.3f;
    app.font.draw((float)ro::title.x, (float)ro::title.y + 22, "Membership Roster", 24, 0.15f, 0.12f, 0.3f);
    app.font.draw((float)ro::headMember.x, (float)ro::kHeadY + 13, "Member", 13, R, G, B);
    app.font.drawCentered((float)ro::headLow.x, (float)ro::kHeadY + 13, "Low", 13, R, G, B);
    app.font.drawCentered((float)ro::headHcp.x, (float)ro::kHeadY + 13, "Hcp", 13, R, G, B);
    app.font.drawCentered((float)ro::headRnds.x, (float)ro::kHeadY + 13, "Rnds", 13, R, G, B);
    app.font.drawCentered((float)ro::headStatus.x, (float)ro::kHeadY + 13, "Status", 13, R, G, B);
    for (int h = 0; h < ro::kHoles; h++) app.font.drawCentered((float)ro::holeX(h) + ro::kHoleCellW / 2.0f, (float)ro::kHeadY + 13, std::to_string(h + 1), 12, R, G, B);
    const std::vector<int> rows = rosterRows(app);
    const int n = (int)rows.size();
    app.rosterScroll = std::max(0, std::min(app.rosterScroll, std::max(0, n - ro::kVisibleRows)));
    auto cut = [&](const sg::ui_screens::Rect& c, float x, float y) { if (app.rosterBtn.tex) ui::drawImage(app.rosterBtn, x, y, (float)c.x, (float)c.y, (float)c.w, (float)c.h); };
    for (int r = 0; r < ro::kVisibleRows && app.rosterScroll + r < n; r++) {
        const int id = rows[(size_t)(app.rosterScroll + r)];
        const auto& e = app.roster.e[id];
        const float y = (float)ro::rowY(r) + 12;
        int code = e.resigned ? -1 : (int)e.tier;   // status code: 1 Visitor ... 4 Gold (Platinum has no text in the exe)
        if (code > 4) code = 4;
        app.font.draw((float)ro::colName.x, y, memberName(app, id), 13, R, G, B);
        if (code >= 2) cut(ro::tierBall[code - 2], (float)ro::tierBallPos.x, (float)(ro::rowY(r) + ro::tierBallPos.y));
        else if (code == -1) cut(ro::tierBall[3], (float)ro::tierBallPos.x, (float)(ro::rowY(r) + ro::tierBallPos.y));
        app.font.drawCentered((float)ro::colLow.x, y, e.low ? std::to_string((int)e.low) : "-", 13, R, G, B);
        app.font.draw((float)ro::colHcp.x, y, e.hcp > 0 ? std::to_string((int)e.hcp) : "-", 13, R, G, B);
        app.font.draw((float)ro::colRnds.x, y, e.rounds ? std::to_string((int)e.rounds) : "-", 13, R, G, B);
        app.font.drawCentered((float)ro::colStatus.x, y, sg::ui_screens::roster::statusText(code), 13, code == -1 ? 0.5f : R, code == -1 ? 0.0f : G, code == -1 ? 0.0f : B);
        for (int h = 1; h <= 18; h++) {
            const int f = e.holeFlags[h], v = ro::iconVariant(h);
            const float ix = (float)ro::holeX(h - 1), iy = (float)(ro::rowY(r) + ro::kIconDy);
            if (f & 1) cut(ro::iconCamera[v], ix, iy);
            if (f & 2) cut(ro::iconHeart[v], ix, iy);
            if (f & 4) cut(ro::iconResigned[v], ix, iy);
        }
    }
    for (int i = 0; i < 6; i++) app.font.drawCentered((float)ro::legend[i].x, (float)ro::legend[i].y + 12, ro::kLegendText[i], 12, R, G, B);
    if (n > ro::kVisibleRows) {
        if (app.rosterBar.tex) ui::drawImage(app.rosterBar, (float)ro::trackPos.x, (float)ro::trackPos.y, 0, 0, 18, 447);
        const int ty = ro::kThumbY0 + app.rosterScroll * ro::kTrackLen / n;
        const int th = std::max(2, std::min(ro::kTrackLen - (ty - ro::kThumbY0), 8008 / n));
        ui::fillRect((float)ro::kThumbX, (float)ty, (float)ro::kThumbW, (float)th, 1, 1, 1, 1);
    }
    const bool hot = app.vmx >= ro::okHit.x && app.vmx < ro::okHit.x + ro::okHit.w && app.vmy >= ro::okHit.y && app.vmy < ro::okHit.y + ro::okHit.h;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, (float)ro::okPos.x, (float)ro::okPos.y, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- Comment sentences (docs/DECODE_COMMENTS.md); wording is the port's own ----
static sg::CellInfo cellInfoFor(const App& app, int loc) {
    sg::CellInfo c; const int a = loc % 50, b = loc / 50;
    for (const App::Amen& m : app.amen) if (m.tx == a && m.ty == b) {
        c.tile = 22;
        if (m.kind == 1) c.flowerbedRecord = true; else c.landmarkKind = m.var % 16;
        return c;
    }
    for (const App::Home& h : app.homes) if (a >= h.x && a < h.x + 2 && b >= h.y && b < h.y + 2) { c.tile = 21; return c; }
    c.tile = 0;   // PLACEHOLDER: other cells read as plain ground (wildflower); the exe reads the terrain id of the cell
    return c;
}
static sg::CommentCtx commentCtxFor(const App& app, int holeIdx, int loc) {
    sg::CommentCtx x; x.theme = themeExe(app.theme);
    if (holeIdx >= 0 && holeIdx < (int)app.hstats.size()) {
        x.par = app.hstats[(size_t)holeIdx].par; x.flags = app.hstats[(size_t)holeIdx].flags;
        if (holeIdx > 0) { x.prevPar = app.hstats[(size_t)holeIdx - 1].par; x.prevFlags = app.hstats[(size_t)holeIdx - 1].flags; }
    }
    x.tick = (unsigned)(SDL_GetTicks() / 25);   // 40 ticks a second
    x.cell = cellInfoFor(app, loc);
    if (loc >= 0 && loc < (int)app.celebs.size()) x.celebrity = app.celebs[(size_t)loc].name;
    return x;
}
static void commentPen(sg::CommentColour c, float& r, float& g, float& b) {
    if (c == sg::CommentColour::Good) { r = 0.13f; g = 0.65f; b = 0.13f; } else if (c == sg::CommentColour::Bad) { r = 1.0f; g = 0.26f; b = 0.26f; } else { r = g = b = 0.0f; }
}

// ---- Hole Stats dialog (layout from docs/UI_SCREENS.md section 3, rules from docs/DECODE_HOLE_STATS2.md) ----
static std::string hsDec(int v100) { char b[24]; std::snprintf(b, sizeof b, "%s%d.%02d", v100 < 0 ? "-" : "", std::abs(v100) / 100, std::abs(v100) % 100); return b; }
static void drawHoleStat(App& app) {
    namespace hs = sg::ui_screens::holestat;
    if (app.hsHole < 0 || app.hsHole >= (int)app.hstats.size()) return;
    const sg::HoleStats& h = app.hstats[(size_t)app.hsHole];
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    auto piece = [&](const sg::ui_screens::Rect& r, float dy) { if (app.holeArt.tex) ui::drawImage(app.holeArt, 0, dy, (float)r.x, (float)r.y, (float)r.w, (float)r.h); };
    piece(hs::top, 0);
    int ev[5] = {}, pct[5] = {}; const int nc = h.topComments(ev, pct);
    const int rowsN = std::max(1, nc);
    for (int i = 0; i < rowsN; i++) piece(hs::rowStrip, (float)hs::commentY(i));
    const float after = (float)hs::commentY(rowsN);
    piece(hs::bottom, after);
    const float R = 0.1f, G = 0.1f, B = 0.3f;
    {   // title: a plain "Hole N" unless the hole is named (Top 100 or naming pending), then the name and "(N)"
        const bool named = (h.flags & 0x81) != 0;
        std::string t = "HOLE STATS for " + (named ? sg::defaultHoleName(h.par, app.hsHole + 1) : "Hole " + std::to_string(app.hsHole + 1));
        if (named) t += " (" + std::to_string(app.hsHole + 1) + ")";
        app.font.drawCentered((float)hs::title.x, (float)hs::title.y + 8, t, 18, 0.15f, 0.12f, 0.3f);
    }
    bool shown = false; const int fp = h.funPercentDialog(&shown);
    auto L = [&](float x, float y, const std::string& t) { app.font.draw(x, y + 12, t, 13, R, G, B); };
    auto C = [&](float x, float y, const std::string& t) { app.font.drawCentered(x, y + 12, t, 13, R, G, B); };
    auto diffRow = [&](int row, const char* label, unsigned skill) {
        const int v = h.dialogDiff100(skill);
        L((float)hs::kLeftLabelX, (float)hs::leftRowY(row), label);
        C((float)hs::kLeftValueCx, (float)hs::leftRowY(row), sg::signedHundredths(v) + " (" + sg::diffWord(v) + ")");
    };
    L((float)hs::kLeftLabelX, (float)hs::leftRowY(0), "Fun Factor");
    if (shown) C((float)hs::kLeftValueCx, (float)hs::leftRowY(0), std::to_string(fp) + "%-(" + sg::funWord(fp) + ")");   // the exe prints a percent sign then a hyphen
    diffRow(1, "Length", sg::kSkillLength); diffRow(2, "Accuracy", sg::kSkillAccuracy); diffRow(3, "Imagination", sg::kSkillImagination);
    L((float)hs::kRightLabelX, (float)hs::kYardsY, "Yards"); C((float)hs::kYardsValueCx, (float)hs::kYardsY, std::to_string(h.yards));
    if (h.rounds > 0) {   // rotating stat: advances by itself every 1.024 seconds
        static const char* rl[5] = {"Avg. Drive: ", "Longest Drive", "Fairways hit", "Greens in Reg", "Average Putts"};
        const int rot = (int)((SDL_GetTicks() >> 10) % 5);
        int onHole = 0; for (const Golfer& g : app.golfers) if (g.active && g.hole == app.hsHole && g.sim.stroke > 0) onHole++;
        const int rn = std::max(1, h.rounds - onHole);
        L((float)hs::kRightLabelX, (float)hs::kRotStatY, rl[rot]);
        const std::string rv = rot == 0 ? std::to_string(h.driveSum / h.rounds) + " yds" : rot == 1 ? std::to_string(h.longestDrive) + " yds" : rot == 2 ? std::to_string(h.fairways * 100 / h.rounds) + "%" : rot == 3 ? std::to_string(h.gir * 100 / rn) + "%" : hsDec(h.putts * 100 / rn);
        C((float)hs::kStatValueCx, (float)hs::kRotStatY, rv);
    }
    L((float)hs::kRightLabelX, (float)hs::kParY, "Par "); C((float)hs::kParValueCx, (float)hs::kParY, std::to_string(h.par));
    if (h.dialogVisibleCount() > 0) { L((float)hs::kRightLabelX, (float)hs::kStrokeAvgY, "Stroke average"); C((float)hs::kStatValueCx, (float)hs::kStrokeAvgY, hsDec(h.dialogAvg100())); }
    L((float)hs::histHead.x, (float)hs::histHead.y, "Average shots on this hole");
    const int first = h.dialogFirstBin();
    for (int k = 0; k < hs::kHistCols; k++) {
        const int st = first + k; int cnt = 0;
        for (int g = 0; g < 8; g++) for (int b = st; b <= (k == hs::kHistCols - 1 ? 9 : st); b++) if (b >= 1 && b <= 9) cnt += h.hist[g][b];
        C((float)hs::histCx(k), (float)hs::kHistLabelY, std::to_string(st) + (k == hs::kHistCols - 1 ? "+" : ""));
        if (cnt) C((float)hs::histCx(k), (float)hs::kHistCountY, std::to_string(cnt));
    }
    L((float)hs::commentsHead.x, (float)hs::commentsHead.y, "Comments");
    if (h.par == 0) { const bool on = (SDL_GetTicks() & 0x200) != 0; if (on) app.font.drawCentered((float)hs::kCommentCx, (float)hs::commentY(0) + 12, "Under Construction!", 13, 1.0f, 0.26f, 0.26f); }
    for (int i = 0; i < nc; i++) {
        const sg::CommentOut co = sg::commentText(ev[i], h.eventLoc[ev[i]] & 0x3fff, commentCtxFor(app, app.hsHole, h.eventLoc[ev[i]] & 0x3fff));
        float r, g, b; commentPen(co.colour, r, g, b);
        app.font.drawCentered((float)hs::kCommentCx, (float)hs::commentY(i) + 12, std::to_string(pct[i]) + "%   '" + co.text + "'", 12, r, g, b);
    }
    const float ox = (float)hs::kOkX, oy = after + hs::kOkDy;
    const bool hot = app.vmx >= ox && app.vmx < ox + hs::kOkW && app.vmy >= oy && app.vmy < oy + 44;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, ox, oy, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- Keyboard shortcuts (layout from docs/UI_SCREENS.md section 5). The list is the original's; not every key is wired in the port yet. ----
static void drawKeys(App& app) {
    namespace sc = sg::ui_screens::shortcuts;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    if (app.keysArt.tex) ui::drawImage(app.keysArt, 0, 0, 0, 0, (float)sc::art.w, (float)sc::art.h); else ui::fillRect(0, 0, 800, 409, 0.75f, 0.78f, 0.7f, 1);
    app.font.drawCentered((float)sc::title.x, (float)sc::title.y + 8, "KEYBOARD SHORTCUTS", 22, 0.15f, 0.12f, 0.3f);
    for (int i = 0; i < sc::kEntries; i++) {
        const auto& e = sc::kList[i];
        const bool hi = std::string(e.key) == "F8";
        const float r = hi ? 0.6f : 0.1f, g = hi ? 0.0f : 0.1f, b = hi ? 0.0f : 0.3f;
        app.font.drawCentered((float)(e.rightColumn ? sc::kRightKeyCx : sc::kLeftKeyCx), (float)e.y + 12, [&]{ std::string k = e.key; for (char& ch : k) if (ch == '+') ch = ' '; return k; }(), 13, r, g, b);
        app.font.draw((float)(e.rightColumn ? sc::kRightActionX : sc::kLeftActionX), (float)e.y + 12, e.action, 13, r, g, b);
    }
    const bool hot = app.vmx >= sc::okPos.x && app.vmx < sc::okPos.x + 44 && app.vmy >= sc::okPos.y && app.vmy < sc::okPos.y + 44;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, (float)sc::okPos.x, (float)sc::okPos.y, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- End of Year (layout from docs/UI_SCREENS.md section 7; sentence wording is the port's own) ----
static void drawEoy(App& app) {
    namespace ey = sg::ui_screens::endyear;
    const int yi = std::min((int)app.yearHist.size(), std::max(1, app.eoyYear + 1)) - 1;
    if (yi < 0 || yi >= (int)app.yearHist.size()) return;
    const App::YearRec cur = app.yearHist[(size_t)yi], prev = yi > 0 ? app.yearHist[(size_t)yi - 1] : App::YearRec();
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    auto piece = [&](const sg::ui_screens::Rect& r, float dy) { if (app.eoyArt.tex) ui::drawImage(app.eoyArt, (float)r.x, dy, (float)r.x, (float)r.y, (float)r.w, (float)r.h); };
    piece(ey::top, (float)ey::top.y);
    const int nh = (int)app.highlights.size();
    const float y0 = 217;
    for (int i = 0; i < nh; i++) piece(ey::strip, y0 + 15.0f * i);
    const float after = y0 + 15.0f * nh;
    piece(ey::bottom, after);
    app.font.drawCentered((float)ey::title.x, (float)ey::title.y + 8, "END of  YEAR: " + std::to_string(2001 + yi), 20, 0.15f, 0.12f, 0.3f);
    app.font.drawCentered((float)ey::headThis.x, (float)ey::headThis.y + 12, "This Year", 13, 0.1f, 0.1f, 0.3f);
    app.font.drawCentered((float)ey::headLast.x, (float)ey::headLast.y + 12, "Last Year", 13, 0.1f, 0.1f, 0.3f);
    auto verb = [](double a, double b) { return a > b ? "increased" : a < b ? "decreased" : "not changed"; };
    char sk[2][24]; std::snprintf(sk[0], 24, "%.2f", cur.skill); std::snprintf(sk[1], 24, "%.2f", prev.skill);
    const std::string lab[4] = {std::string("Cash reserves have ") + verb((double)cur.cash, (double)prev.cash), std::string("Your fun rating has ") + verb(cur.fun, prev.fun),
                                std::string("Your skill rating has ") + verb(cur.skill, prev.skill), std::string("Your membership has ") + verb(cur.members, prev.members)};
    const std::string a[4] = {money(cur.cash), std::to_string(cur.fun), sk[0], std::to_string(cur.members)};
    const std::string b[4] = {money(prev.cash), std::to_string(prev.fun), sk[1], std::to_string(prev.members)};
    for (int i = 0; i < 4; i++) {
        const float y = (float)ey::rowY(i) + 12;
        app.font.drawCentered((float)ey::kRowLabelCx, y, lab[i], 12, 0.1f, 0.1f, 0.3f);
        app.font.draw((float)ey::kThisRightX - app.font.width(a[i], 13), y, a[i], 13, 0.0f, 0.5f, 0.1f);
        app.font.draw((float)ey::kLastRightX - app.font.width(b[i], 13), y, b[i], 13, 0, 0, 0);
    }
    app.font.drawCentered((float)ey::highlights.x, (float)ey::highlights.y + 12, "Highlights", 14, 0.1f, 0.1f, 0.3f);
    for (int i = 0; i < nh; i++) { const auto& h = app.highlights[(size_t)i]; app.font.drawCentered((float)ey::kHighlightCx, y0 + 15.0f * i + 12, h.first, 12, h.second ? 0.8f : 0.0f, 0, 0); }
    const float ox = (float)ey::kOkX, oy = after;
    const bool hot = app.vmx >= ox && app.vmx < ox + 44 && app.vmy >= oy && app.vmy < oy + 44;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, ox, oy, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
    if (!app.eoyBoard.empty()) app.font.drawCentered(400, after + 70, "The board: " + app.eoyBoard, 12, 1, 0.8f, 0.6f);
}

// ---- Player Comments report (layout from docs/UI_SCREENS.md section 8, rules from docs/DECODE_COMMENTS.md section 6). Up to 20 types over all holes. ----
static void drawComments(App& app) {
    namespace cm = sg::ui_screens::comments;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    struct Row { int type, hole, count, rounds; };
    std::vector<Row> rows; int totalRounds = 0;
    for (const auto& h : app.hstats) totalRounds += h.rounds;
    int sum[64] = {};
    for (const auto& h : app.hstats) for (int t = 0; t < 64; t++) sum[t] += h.events[t];
    for (int k = 0; k < cm::kMaxRows; k++) {
        int best = 0, bt = 0;
        for (int t = 0; t < 50; t++) if (sum[t] > best) { best = sum[t]; bt = t; }
        if (!best || !totalRounds) { if (best) sum[bt] = 0; continue; }   // a pass that draws nothing still uses up its slot
        int bh = 0, bc = 0;
        for (size_t h = 0; h < app.hstats.size(); h++) if (app.hstats[h].events[bt] > bc) { bc = app.hstats[h].events[bt]; bh = (int)h; }
        rows.push_back({bt, bh, sum[bt], totalRounds}); sum[bt] = 0;
    }
    auto piece = [&](const sg::ui_screens::Rect& r, float dy) { if (app.comtArt.tex) ui::drawImage(app.comtArt, (float)r.x, dy, (float)r.x, (float)r.y, (float)r.w, (float)r.h); };
    piece(cm::top, (float)cm::top.y);
    const int n = (int)rows.size();
    for (int i = 0; i < n; i++) piece(cm::strip, (float)cm::stripY(i));
    const float after = (float)cm::stripY(n);
    piece(cm::bottom, after);
    app.font.drawCentered((float)cm::title.x, (float)cm::title.y + 8, "PLAYER COMMENTS REPORT", 20, 0.15f, 0.12f, 0.3f);
    app.font.draw((float)cm::headComments.x, (float)cm::headComments.y + 12, "Comments", 13, 0.1f, 0.1f, 0.3f);
    app.font.drawCentered((float)cm::headHole.x, (float)cm::headHole.y + 12, "Hole", 13, 0.1f, 0.1f, 0.3f);
    app.font.drawCentered((float)cm::headFrequency.x, (float)cm::headFrequency.y + 12, "Frequency", 13, 0.1f, 0.1f, 0.3f);
    for (int i = 0; i < n; i++) {
        const Row& rw = rows[(size_t)i];
        const int loc = app.hstats[(size_t)rw.hole].eventLoc[rw.type] & 0x3fff;
        // PLACEHOLDER: the exe builds the report sentence against hole record 0 (content unknown); the port uses the hole the row names.
        const sg::CommentOut co = sg::commentText(rw.type, loc, commentCtxFor(app, rw.hole, loc));
        float r, g, b; commentPen(co.colour, r, g, b);
        const float y = (float)cm::textY(i) + 11;
        app.font.draw((float)cm::kTextX, y, "'" + co.text + "'", 12, r, g, b);
        app.font.drawCentered((float)cm::kHoleCx, y, std::to_string(rw.hole + 1), 12, r, g, b);
        app.font.drawCentered((float)cm::kFreqCx, y, std::to_string(rw.count * 100 / std::max(1, rw.rounds)) + "%", 12, r, g, b);
    }
    if (!n) app.font.drawCentered(400, (float)cm::kStripY0 + 14, "No comments yet.", 13, 0.1f, 0.1f, 0.3f);
    const float ox = (float)cm::kOkX, oy = after;
    const bool hot = app.vmx >= ox && app.vmx < ox + 44 && app.vmy >= oy && app.vmy < oy + 44;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, ox, oy, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- Histograph (layout from docs/UI_SCREENS2.md section 1) ----
static int hgH(int v) { const int a = std::abs(v); return a < 501 ? a / 2 : v / 10 + 200; }
static void hgSeg(float x0, int y0, float x1, int y1, float r, float g, float b) {   // monotone in x; clipped to the plot box
    namespace hg = sg::ui_screens2::histograph;
    const int dx = std::max(1, (int)(x1 - x0));
    for (int i = 0; i < dx; i++) {
        const float x = x0 + i; if (x < hg::plotClip.x || x >= hg::plotClip.x + hg::plotClip.w) continue;
        const int ya = y0 + (y1 - y0) * i / dx, yb = y0 + (y1 - y0) * (i + 1) / dx; const int lo = std::min(ya, yb), hi = std::max(ya, yb);
        const int cl = std::max(lo, hg::plotClip.y), ch = std::min(hi + 1, hg::plotClip.y + hg::plotClip.h);
        if (ch > cl) ui::fillRect(x, (float)cl, 1, (float)(ch - cl), r, g, b, 1);
    }
}
static void drawHisto(App& app) {
    namespace hg = sg::ui_screens2::histograph;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
    if (app.histArt.tex) ui::drawImage(app.histArt, 0, 0, 0, 0, 800, 600); else ui::fillRect(0, 0, 800, 600, 0.8f, 0.8f, 0.7f, 1);
    app.font.drawCentered(400, 40, hg::kTitle, 22, 0.15f, 0.12f, 0.3f);
    for (int i = 0; i < 4; i++) app.font.drawCentered((float)hg::kLegendCx[i], (float)hg::kLegendY + 12, hg::kLegend[i], 13, 0, 0, 0);
    const int months = std::max(0, app.econ.day - 1);
    const int cur = (int)((app.econ.day - 1) % 500);
    const double cashNow = app.econ.cash / 100.0, skillNow = clubSkill(app) * 100;
    const int cs = cashNow > 100000 ? 80 : cashNow > 50000 ? 40 : cashNow > 25000 ? 20 : cashNow > 10000 ? 10 : cashNow > 5000 ? 4 : cashNow > 2500 ? 2 : 1;
    const int ss = skillNow > 2500 ? 2 : 1;
    for (int k = 0; k < hg::kTicks; k++) {
        const int lv = 100 * ss * (k <= 4 ? k : 4 + 5 * (k - 4)) / (k <= 4 ? 1 : 1);
        const int left = k <= 4 ? 100 * ss * k : 500 * ss * (k - 4), right = k <= 4 ? 10 * cs * k : 50 * cs * (k - 4); (void)lv;
        app.font.drawCentered((float)hg::kLeftLabelCx, (float)hg::tickY(k) + 4, std::to_string(left), 11, 0.5f, 0, 0.35f);
        app.font.drawCentered((float)hg::kRightLabelCx, (float)hg::tickY(k) + 4, "\xC2\xA7" + std::to_string(right) + "k", 11, 0, 0, 0);
    }
    (void)cur;
    const int step = hg::stepPx(months);
    int pS = 0, pC = 0, pF = 0, pT = 0; float px = (float)hg::kFirstX; int label = 0;
    for (int m = 1; m <= std::min(months, 499); m++) {
        const size_t i = (size_t)(m % 500); const float x = px + step;
        const int hs = hgH(app.hgSkill[i] / ss), hc = hgH(app.hgCash[i] / cs), hf = hgH(app.hgFun[i]), ht = app.hgStaff[i] * 4;
        const int by = hg::kBaselineY;
        hgSeg(px, by - pS, x, by - hs, 0.5f, 0.0f, 0.5f);
        hgSeg(px, by - pC, x, by - hc, app.hgCash[i] < 0 ? 0.8f : 0.0f, 0, 0);
        hgSeg(px, by - pF, x, by - hf, 0, 0.8f, 0);
        hgSeg(px, by - pT, x, by - ht, 0, 0.5f, 0.5f);
        const int ev = app.evLog[i];
        if (ev) {
            const int code = ev & ~31, arg = ev & 31; std::string t;
            if (code == 0x20) t = "Hole " + std::to_string(arg) + " opened.";
            else if (code == 0x40) t = std::string(arg < 15 ? kTypeNames[arg] : "Building") + " built.";
            else if (code == 0x80) t = "Hole " + std::to_string(arg) + " rated top 100.";
            else if (code == 0xa0) t = "Hole " + std::to_string(arg) + " rated top 18!";
            else if (code == 0xe0) t = app.charName + " places " + std::to_string(arg) + " in tournament.";
            else if (code == 0x120) t = "Happy Ending.";
            else if (code == 0x140) t = "Additional land purchased.";
            else if (code == 0x60) t = "Won match.";
            if (!t.empty()) {
                const int lh = (label % 450) + 10; label = lh;
                ui::fillRect(x + 10, (float)(by - hs), 2, 2, 0, 0.5f, 0.5f, 1);
                ui::fillRect(x + 11, (float)(by - lh), 1, (float)(lh - hs), 0.1f, 0.3f, 1, 1);
                ui::fillRect(x + 10, (float)(by - lh), 2, 2, 0, 0.5f, 0.5f, 1);
                if (by - lh > hg::plotClip.y) app.font.draw(x + 14, (float)(518 - lh) + 8, t, 11, 0, 0.5f, 0.5f);
            }
        }
        pS = hs; pC = hc; pF = hf; pT = ht; px = x;
    }
    const bool hot = app.vmx >= hg::okHit.x && app.vmx < hg::okHit.x + hg::okHit.w && app.vmy >= hg::okHit.y && app.vmy < hg::okHit.y + hg::okHit.h;
    if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, (float)hg::okPos.x, (float)hg::okPos.y, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
}

// ---- Accomplishments board (F10; layout from docs/UI_SCREENS2.md section 2). Photos are taken from the world view the frame after a milestone. ----
static void captureSnap(App& app) {   // 200x160 crop of the world view, centred (the exe crops around the event spot, which the port does not track)
    const int g = app.snapPending; app.snapPending = -1;
    if (g < 0 || g >= 22) return;
    const float sc = std::max(0.2f, std::min(app.drawW / 800.0f, app.drawH / 600.0f));
    const int w = std::min(app.drawW, (int)(200 * sc)), h = std::min(app.drawH, (int)(160 * sc));
    const int x0 = std::max(0, (app.drawW - w) / 2), y0 = std::max(0, (app.drawH - h) / 2);
    std::vector<unsigned char> px((size_t)w * h * 4);
    glReadPixels(x0, y0, w, h, GL_RGBA, GL_UNSIGNED_BYTE, px.data());
    if (!app.miles[g].snap) glGenTextures(1, &app.miles[g].snap);
    glBindTexture(GL_TEXTURE_2D, app.miles[g].snap);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, px.data());
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
}
static void drawSnap(GLuint tex, float x, float y, float w, float h) {
    glEnable(GL_TEXTURE_2D); glBindTexture(GL_TEXTURE_2D, tex); glColor4f(1, 1, 1, 1);
    glBegin(GL_QUADS); glTexCoord2f(0, 1); glVertex2f(x, y); glTexCoord2f(1, 1); glVertex2f(x + w, y); glTexCoord2f(1, 0); glVertex2f(x + w, y + h); glTexCoord2f(0, 0); glVertex2f(x, y + h); glEnd();
}
static void drawBoard(App& app) {
    namespace ac = sg::ui_screens2::accomplishments;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    if (app.boardArt.tex) ui::drawImage(app.boardArt, 0, 0, 0, 0, 800, 600); else ui::fillRect(0, 0, 800, 600, 0.35f, 0.22f, 0.12f, 1);
    auto parts = [&](const sg::ui_screens2::Rect& c, float dx, float dy) { if (app.partsArt.tex) ui::drawImage(app.partsArt, dx, dy, (float)c.x, (float)c.y, (float)c.w, (float)c.h); };
    auto tacs = [&](const sg::ui_screens2::Rect& c, float dx, float dy) { if (app.tacsArt.tex) ui::drawImage(app.tacsArt, dx, dy, (float)c.x, (float)c.y, (float)c.w, (float)c.h); };
    std::vector<int> earned; for (int c = 0; c < 22; c++) if (app.miles[c].tick) earned.push_back(c);
    std::sort(earned.begin(), earned.end(), [&](int a, int b) { return app.miles[a].tick < app.miles[b].tick; });
    const int n = (int)earned.size();
    parts(ac::kSignRight.cut, (float)ac::kSignRight.dst.x, (float)ac::kSignRight.dst.y); parts(ac::kSignLeft.cut, (float)ac::kSignLeft.dst.x, (float)ac::kSignLeft.dst.y);
    parts(ac::kTowerTop.cut, (float)ac::kTowerTop.dst.x, (float)ac::kTowerTopY(n));
    parts(ac::kTowerCapA.cut, (float)ac::kPlaqueX, (float)ac::kTowerCapY0(n)); parts(ac::kTowerCapA.cut, (float)ac::kPlaqueX, (float)ac::kTowerCapY1(n));
    { float y = (float)ac::kPlaqueY0(n);
      for (int i = n - 1; i >= 0; i--) { parts(ac::plaqueCut(earned[(size_t)i]), (float)ac::kPlaqueX, y); if (i == n - 1) parts(ac::kTowerCapB.cut, (float)ac::kPlaqueX, y + 14); y += (i == n - 1) ? 16.0f : 14.0f; } }
    parts(ac::kTowerBase.cut, (float)ac::kTowerBase.dst.x, (float)ac::kTowerBase.dst.y); parts(ac::kTowerFoot.cut, (float)ac::kTowerFoot.dst.x, (float)ac::kTowerFoot.dst.y);
    tacs(ac::kCornerLeft.cut, (float)ac::kCornerLeft.dst.x, (float)ac::kCornerLeft.dst.y); tacs(ac::kCornerRight.cut, (float)ac::kCornerRight.dst.x, (float)ac::kCornerRight.dst.y);
    for (int k = 0; k < n; k++) {
        const App::Mile& m = app.miles[earned[(size_t)k]];
        tacs(ac::kFrame.cut, (float)ac::framePos(k).x, (float)ac::framePos(k).y);
        const auto pd = ac::photoDst(k);
        if (m.snap) drawSnap(m.snap, (float)pd.x, (float)pd.y, 200, 160); else ui::fillRect((float)pd.x, (float)pd.y, 200, 160, 0.2f, 0.3f, 0.2f, 1);
        if (app.pinArt.tex) ui::drawImage(app.pinArt, (float)ac::pinPos(k).x, (float)ac::pinPos(k).y, 0, 0, 20, 24);
        static const char* mo[8] = {"March", "April", "May", "June", "July", "August", "September", "October"};
        const int day = (int)(((m.tick & 1023) * 30 >> 10) + 1);
        const std::string cap = m.course + "  " + std::to_string(day) + " " + mo[(m.tick >> 10) & 7] + " " + std::to_string(2001 + m.tick / 8192);
        app.font.draw((float)ac::polaroidX(k), (float)ac::polaroidY(k) + 10, cap, 11, 0.0f, 0.1f, 0.5f);
    }
    tacs(ac::kNoteCard.cut, (float)ac::kNoteCard.dst.x, (float)ac::kNoteCard.dst.y);
    { int off = 0, shown = 0; for (int c = 0; c < 22 && shown < ac::kTodoCount; c++) if (!app.miles[c].tick) { const auto& lc = ac::labelCut[c]; tacs(lc, (float)(ac::kTodoX0 + off / 2), (float)(ac::kTodoY0 + off)); off += lc.h - 4; shown++; } }
}
static void drawSga(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH, !((app.sgaMode == 0 && app.sgaArt.tex) || (app.sgaMode == 2 && app.tourArt.tex)));
    if (app.sgaMode == 2 && app.tourArt.tex) {   // real-art results (layout from docs/UI_SCREENS.md section 6; the per hole strokes are the placeholder generator's)
        namespace tn = sg::ui_screens::tournament;
        ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
        auto piece = [&](const sg::ui_screens::Rect& r, float dy) { ui::drawImage(app.tourArt, 0, dy, (float)r.x, (float)r.y, (float)r.w, (float)r.h); };
        piece(tn::headerPiece, 0);
        const int Hn = (int)app.tPars.size();
        const int shownHoles = g_revealForce >= 0 ? std::min(g_revealForce, Hn) : std::min(Hn, (int)((SDL_GetTicks() / 1000.0 - app.tRevealStart) / 0.7));
        const bool live = shownHoles < Hn;
        app.font.drawCentered((float)tn::title.x, (float)tn::title.y + 18, live ? "TOURNAMENT IN PROGRESS" : "TOURNAMENT RESULTS", 20, 0.15f, 0.12f, 0.3f);
        app.font.drawCentered(88, (float)tn::name.y + 12, app.tName.size() > 20 ? app.tName.substr(0, 20) : app.tName, 10, 0.1f, 0.1f, 0.3f);
        const int H = (int)app.tPars.size();
        for (int h = 0; h < 18 && h < H; h++) app.font.drawCentered((float)tn::holeCx(h), (float)tn::kHeadY + 12, std::to_string(h + 1), 11, 0.1f, 0.1f, 0.3f);
        app.font.drawCentered((float)tn::headFinal.x, (float)tn::headFinal.y + 12, "F", 12, 0.1f, 0.1f, 0.3f);
        app.font.drawCentered((float)tn::headPrize.x, (float)tn::headPrize.y + 12, "Prize", 12, 0.1f, 0.1f, 0.3f);
        float y = (float)tn::kRowY0;
        if (live) {   // running leaderboard: lowest total over the holes played so far leads
            std::vector<std::pair<int, int>> ord;   // (total, slot)
            for (const auto& kv : app.tStrokes) { int t = 0; for (int c = 0; c < shownHoles && c < (int)kv.second.size(); c++) t += kv.second[(size_t)c]; ord.push_back({t, kv.first}); }
            std::sort(ord.begin(), ord.end());
            std::vector<size_t> pick; for (size_t k = 0; k < ord.size() && k < 14; k++) pick.push_back(k);
            for (size_t k = 14; k < ord.size(); k++) if (ord[k].second == 1) pick.push_back(k);
            for (size_t q : pick) {
                piece(tn::rowMid, y); const float ty = y + 16, me = ord[q].second == 1 ? 1.0f : 0.0f;
                std::string nm = "?"; for (const sg::Entrant& e : app.tField) if (e.slot == ord[q].second) nm = e.name;
                app.font.draw(16, ty, std::to_string(q + 1) + ". " + nm, 11, 0, me * 0.5f, me ? 0.1f : 0.09f);
                const std::vector<int>& sv = app.tStrokes[ord[q].second];
                for (int c = 0; c < shownHoles && c < (int)sv.size() && c < 18; c++) app.font.drawCentered((float)tn::holeCx(c), ty, std::to_string(sv[(size_t)c]), 11, 0, me * 0.5f, me ? 0.1f : 0.09f);
                app.font.drawCentered((float)tn::kTotalCx, ty, std::to_string(ord[q].first), 11, 0, me * 0.5f, me ? 0.1f : 0.09f);
                y += 22;
            }
            app.font.drawCentered(400, y + 24, "Click or press a key to skip to the results", 12, 1, 1, 1);
            ui::endScreen(); return;
        }
        std::vector<const sg::Standing*> show; const sg::Standing* you = nullptr;
        for (const sg::Standing& st : app.tResult.standings) { if (st.slot == 1) you = &st; if ((st.paid && (int)show.size() < 16) || (!st.paid && 0)) show.push_back(&st); }
        if (you && std::find(show.begin(), show.end(), you) == show.end()) show.push_back(you);
        for (size_t k = 0; k < show.size(); k++) {
            const sg::Standing& st = *show[k];
            const bool tall = k < 3 && st.paid;
            piece(st.paid ? (tall ? tn::rowTall : tn::rowMid) : tn::rowThin, y);
            const float h = st.paid ? (tall ? 26.0f : 22.0f) : 18.0f, ty = y + h / 2 + 5;
            const bool me = st.slot == 1; const float r = me ? 0.0f : 0.0f, g = me ? 0.5f : 0.0f, b = me ? 0.1f : 0.09f;
            std::string nm = "?"; for (const sg::Entrant& e : app.tField) if (e.slot == st.slot) nm = e.name;
            app.font.draw(16, ty, std::to_string(st.place) + ". " + nm, 11, r, g, b);
            auto it = app.tStrokes.find(st.slot); int tot = 0;
            if (it != app.tStrokes.end()) for (size_t c = 0; c < it->second.size() && c < 18; c++) { tot += it->second[c]; app.font.drawCentered((float)tn::holeCx((int)c), ty, std::to_string(it->second[c]), 11, r, g, b); }
            app.font.drawCentered((float)tn::kTotalCx, ty, std::to_string(tot), 11, r, g, b);
            if (st.paid) app.font.drawCentered(740, ty, std::to_string(st.prizeThousands) + ",000", 11, r, g, b);
            y += h;
        }
        piece(tn::endA, y);
        const float ox = 732, oy = y;
        const bool hot = app.vmx >= ox && app.vmx < ox + 44 && app.vmy >= oy && app.vmy < oy + 44;
        if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, ox, oy, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
        ui::endScreen(); return;
    }
    if (app.sgaMode == 0 && app.sgaArt.tex) {   // the real-art report (layout from docs/UI_SCREENS.md section 4)
        namespace sa = sg::ui_screens::sga;
        const sg::SgaResult& r = app.sgaLast; const sg::SgaInput& in = app.sgaIn;
        ui::fillRect(0, 0, 800, 600, 0.05f, 0.07f, 0.05f, 0.6f);
        ui::drawImage(app.sgaArt, (float)sa::art.x, (float)sa::art.y, (float)sa::art.x, (float)sa::art.y, (float)sa::art.w, (float)sa::art.h);
        const float R = 0.1f, G = 0.1f, B = 0.3f;
        app.font.draw((float)sa::title.x, (float)sa::title.y + 18, "REPORT of the SIM GOLF ASSOCIATION", 20, 0.15f, 0.12f, 0.3f);
        const int gr = r.grade < 0 ? 3 : r.grade;
        app.font.draw((float)sa::selectionCriteria.x, (float)sa::selectionCriteria.y + 12, std::string("Selection Criteria: ") + sg::kGradeName[gr], 13, R, G, B);
        app.font.draw((float)sa::grade.x, (float)sa::grade.y + 12, r.grade < 0 ? "Grade: none" : std::string("Grade: ") + sg::kGradeName[gr], 12, R, G, B);
        app.font.draw((float)sa::idealHead.x, (float)sa::idealHead.y + 12, "Ideal  (Minimum)", 13, R, G, B);
        const int c8 = r.grade == 2 ? std::max(0, in.holes - 1) : r.requiredHoles;
        const std::string actual[10] = {std::to_string(in.totalYards) + " yds", std::to_string(in.holes), std::to_string(in.avgMinutes / 60) + "h " + std::to_string(in.avgMinutes % 60) + "m",
            std::to_string(in.funPercent) + "%", std::to_string(in.varietyHoles), std::to_string(in.scenicHoles), std::to_string(in.lengthHoles), std::to_string(in.accuracyHoles),
            std::to_string(in.imaginationHoles), std::to_string(in.facilityKinds)};
        const std::string ideal[10] = {std::to_string(r.idealYards) + " yds", std::to_string(r.requiredHoles), "235 min or less", "109% or more", std::to_string(c8), std::to_string(c8),
            std::to_string(c8), std::to_string(c8), std::to_string(c8), std::to_string(c8 / 2) + " or more"};
        for (int i = 0; i < sa::kRows; i++) {
            const float y = 104.0f + 17 * i + 12;
            app.font.drawCentered((float)sa::kLabelCx, y, sa::kRowLabels[i], 12, R, G, B);
            app.font.drawCentered((float)sa::kActualCx, y, actual[i], 12, R, G, B);
            if (r.scores[i] == 0) app.font.drawCentered((float)sa::kNotAcceptableCx, y, "- not acceptable -", 12, 0.8f, 0, 0);
            else for (int j = 0; j < r.scores[i] && j < sa::kMaxPips; j++) ui::fillRect((float)(sa::kPipX0 + sa::kPipPitch * j) - 4, y - 8, 10, 10, 0.1f, 0.55f, 0.2f, 1);
            app.font.draw((float)sa::kIdealX, y, ideal[i], 12, R, G, B);
        }
        app.font.draw((float)sa::recommendation.x, (float)sa::recommendation.y + 12, "Committee recommendation", 13, R, G, B);
        if (r.total < 1) {
            app.font.draw(438 - app.font.width("0/100", 13), (float)sa::zeroScore.y + 10, "0/100", 13, R, G, B);
            app.font.drawCentered((float)sa::verdictTitle.x, (float)sa::verdictTitle.y + 12, "Improvement Required.", 18, 0.6f, 0, 0);
        } else {
            const std::string t = std::to_string(r.total) + "/100";
            app.font.draw(438 - app.font.width(t, 13), (float)sa::totalScore.y + 12, t, 13, R, G, B);
            const char* n = sg::tournamentName(r.total, r.grade == 3);
            app.font.drawCentered((float)sa::verdictTitle.x, (float)sa::verdictTitle.y + 12, n ? n : "", 20, 0.15f, 0.12f, 0.3f);
            app.font.drawCentered((float)sa::prizeLine.x, (float)sa::prizeLine.y + 12, std::to_string(sg::tournamentPrizeThousands(r.total, gr, in.holes)) + ",000 first prize.", 15, R, G, B);
        }
        const bool hot = app.vmx >= sa::okPos.x && app.vmx < sa::okPos.x + 44 && app.vmy >= sa::okPos.y && app.vmy < sa::okPos.y + 44;
        if (app.okArt.tex && hot) { const auto& k = sg::ui_screens::kOkCut[1]; ui::drawImage(app.okArt, (float)sa::okPos.x, (float)sa::okPos.y, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
        ui::endScreen(); return;
    }
    ui::fillRect(0, 0, 800, 600, 0.05f, 0.06f, 0.16f, 0.94f);
    ui::fillRect(90, 50, 620, 500, 0.12f, 0.1f, 0.3f, 0.96f);
    if (app.sgaMode == 0) {
        const sg::SgaResult& r = app.sgaLast;
        app.font.drawCentered(400, 86, "REPORT of the SIM GOLF ASSOCIATION", 20, 1, 1, 0.7f);
        char b[160];
        std::snprintf(b, sizeof b, "Grade: %s   Holes: %zu   Ideal length %d yards", r.grade >= 0 ? sg::kGradeName[r.grade] : "too many holes", app.holes.size(), r.idealYards);
        app.font.drawCentered(400, 112, b, 14, 0.9f, 0.9f, 1);
        for (int i = 0; i < 10; i++) {
            app.font.draw(130, 150 + i * 25.0f, sg::kSgaCriterion[i], 15, 1, 1, 1);
            for (int k = 0; k < 10; k++) ui::fillRect(400 + k * 14.0f, 138 + i * 25.0f, 11, 14, k < r.scores[i] ? 0.4f : 0.2f, k < r.scores[i] ? 0.85f : 0.2f, k < r.scores[i] ? 0.4f : 0.3f, 1);
            app.font.draw(560, 150 + i * 25.0f, r.scores[i] == 0 ? "not acceptable" : std::to_string(r.scores[i]) + "/10", 14, r.scores[i] == 0 ? 1.0f : 0.8f, r.scores[i] == 0 ? 0.5f : 0.9f, r.scores[i] == 0 ? 0.5f : 0.8f);
        }
        app.font.drawCentered(400, 430, "Committee recommendation", 15, 0.8f, 0.9f, 1);
        if (r.total < 1) app.font.drawCentered(400, 462, "Improvement Required.", 22, 1, 0.5f, 0.5f);
        else { const char* n = sg::tournamentName(r.total, r.grade == 3); app.font.drawCentered(400, 462, std::to_string(r.total) + "/100  " + (n ? n : ""), 20, 1, 1, 0.7f); }
        app.font.drawCentered(400, 520, "Esc to close. A tournament offer arrives every July when the course scores above zero.", 12, 0.75f, 0.75f, 0.95f);
    } else if (app.sgaMode == 1) {
        app.font.drawCentered(400, 110, "A message from the SGA", 22, 1, 1, 0.7f);
        const std::string t = std::string("The SGA is interested in holding a tournament at your course. 'We'd like to schedule the ") + (app.tourney.name() ? app.tourney.name() : "") + " at your course with a first prize of \xC2\xA7" + std::to_string(app.tourney.firstPrizeThousands()) + ",000. Open Golf Tournament here as soon as possible.'";
        const std::vector<std::string> lines = wrapText(app, t, 17, 480);
        for (size_t i = 0; i < lines.size(); i++) app.font.drawCentered(400, 190 + 26.0f * i, lines[i], 17, 1, 1, 1);
        const char* lab[2] = {"Great, let the games begin.", "I think I need more practice."};
        for (int b = 0; b < 2; b++) { const SgaBtn r = sgaBtn(b); const bool hot = app.hover == b; ui::fillRect(r.x, r.y, r.w, r.h, hot ? 0.45f : 0.25f, hot ? 0.4f : 0.22f, hot ? 0.7f : 0.5f, 0.95f); app.font.drawCentered(r.x + r.w / 2, r.y + 24, lab[b], 14, 1, 1, 1); }
    } else {
        app.font.drawCentered(400, 86, "TOURNAMENT RESULTS", 22, 1, 1, 0.7f);
        app.font.drawCentered(400, 112, "LEADER BOARD of the " + app.tName, 15, 0.9f, 0.9f, 1);
        app.font.draw(130, 140, "Place", 13, 0.7f, 0.8f, 1); app.font.draw(190, 140, "Golfer", 13, 0.7f, 0.8f, 1); app.font.draw(470, 140, "To par", 13, 0.7f, 0.8f, 1); app.font.draw(560, 140, "Prize", 13, 0.7f, 0.8f, 1);
        int row = 0;
        for (const sg::Standing& st : app.tResult.standings) {
            const bool you = st.slot == 1;
            if (row >= 14 && !you) continue;
            if (row >= 14 && you) row = 14;
            const float y = 162 + row * 22.0f; row++;
            if (you) ui::fillRect(120, y - 14, 560, 19, 0.9f, 0.75f, 0.2f, 0.4f);
            std::string nm = "?"; for (const sg::Entrant& e : app.tField) if (e.slot == st.slot) nm = e.name;
            app.font.draw(138, y, std::to_string(st.place), 14, 1, 1, 1); app.font.draw(190, y, nm, 14, 1, 1, 1);
            app.font.draw(480, y, st.relToPar > 0 ? "+" + std::to_string(st.relToPar) : st.relToPar == 0 ? "E" : std::to_string(st.relToPar), 14, 1, 1, 1);
            if (st.paid) app.font.draw(560, y, "\xC2\xA7" + std::to_string(st.prizeThousands) + ",000", 14, 1, 1, 0.7f);
        }
        char b[160]; std::snprintf(b, sizeof b, "%s placed %d. Club cash %s%s.", app.charName.c_str(), app.tResult.playerPlace, app.tResult.playerPrizeThousands ? "rises by \xC2\xA7" : "is unchanged", app.tResult.playerPrizeThousands ? (std::to_string(app.tResult.playerPrizeThousands) + ",000").c_str() : "");
        app.font.drawCentered(400, 500, b, 15, 1, 1, 0.8f);
        app.font.drawCentered(400, 530, "Esc to close. (Strokes come from the port's shot simulation, which is still a placeholder.)", 12, 0.75f, 0.75f, 0.95f);
    }
    ui::endScreen();
}


// ---- Buy Land (docs/UI_SCREENS2.md section 4, art infoscreens/buy_land.pcx) ----
namespace bl2 = sg::ui_screens2::buyland2;
static bool tractForSale(const App& app, int t) { return app.landModel && !((app.ownMask >> t) & 1); }
static void rollTractPrices(App& app) {
    uint32_t r = app.seed * 2654435761u + (uint32_t)app.landBought * 40503u + (uint32_t)app.econ.day * 9973u + 77u;
    auto rnd3 = [&]() { r ^= r << 13; r ^= r >> 17; r ^= r << 5; return (int)(r % 3u); };
    const int n = std::min(app.landBought & 0x7f, 18);
    for (int t = 0; t < 9; t++) {
        long long f = bl2::kBaseF; int sale = 0;
        if (tractForSale(app, t)) for (int i = 0; i < bl2::kTractSide * bl2::kTractSide; i++) { sale++; if (rnd3() == 0) f += 1LL << n; }
        app.blSale[t] = sale;
        app.blPrice[t] = sale ? bl2::priceUnits((int)std::min<long long>(f, 100000000LL)) : 0;
    }
    app.blRolled = true;
}
static bool openBuyLand(App& app) {
    if (!app.landModel) { say(app, "This course has no land for sale", 4); return false; }
    if (!app.econ.sandbox && !app.landOffer) { say(app, "The county commissioner has not offered you any land yet", 5); return false; }
    app.blRolled = false; app.blSel = -1; app.blNote.clear(); app.screen = App::ScreenBuyLand; return true;
}
static int blTractAt(float vx, float vy) {
    for (int c = 0; c < 3; c++) for (int r = 0; r < 3; r++)
        if (vx >= bl2::kCardPickX[c] && vx < bl2::kCardPickX[c] + 245 && vy >= bl2::kCardPickY[r] && vy < bl2::kCardPickY[r] + 64) return c * 3 + r;
    const float dx = vx - 400, dy = vy - 440;
    const float u = (dx / 96.0f + dy / 47.5f) * 0.5f + 1.5f, v = (dy / 47.5f - dx / 96.0f) * 0.5f + 1.5f;
    if (u >= 0 && u < 3 && v >= 0 && v < 3) return (int)v * 3 + (int)u;
    return -1;
}
static void drawBuyLand(App& app) {
    namespace bl = sg::ui_screens::buyland;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0.04f, 0.05f, 0.1f, 1);
    if (app.landArt.tex) ui::drawImage(app.landArt, 0, 0, 0, 0, 800, 600);
    if (!app.blRolled) rollTractPrices(app);
    app.blSel = blTractAt(app.vmx, app.vmy);
    app.font.drawCentered(400, 34, "TRACTS FOR SALE", 18, 0.15f, 0.12f, 0.3f);
    app.font.draw(548, 273, "Cash Reserve", 11, 0.12f, 0.1f, 0.3f);
    app.font.drawCentered(720, 274, app.econ.sandbox ? "Unlimited \xC2\xA7" : money((long long)app.econ.cash), 12, 0.12f, 0.1f, 0.3f);
    for (int t = 0; t < 9; t++) {
        const int col = t / 3, row = t % 3;
        const float cx = (float)bl2::kCardColumnX[col] + 4, cy = (float)(bl2::kCardRowY0 + bl2::kCardRowPitch * row) + 2;
        const bool sale = tractForSale(app, t);
        if (!sale) {
            app.font.draw(cx, cy + 14, bl2::kLabelSold, 14, 0.13f, 0.13f, 0.4f);
        } else {
            app.font.draw(cx, cy + 14, std::string(bl2::kLabelBuy) + std::to_string(t + 1), 14, 0.13f, 0.13f, 0.4f);
            // The three most common terrain types of the tract.
            const int x0 = bl2::tractOriginX(t), y0 = bl2::tractOriginY(t);
            std::map<int, int> cnt;
            for (int y = y0; y < y0 + 16; y++) for (int x = x0; x < x0 + 16; x++) if (x < app.terrain.w && y < app.terrain.h) cnt[app.terrain.type[(size_t)app.terrain.tileIndex(x, y)]]++;
            std::vector<std::pair<int, int>> top; for (auto& kv : cnt) if (sg::tileTypeName(kv.first)) top.push_back({kv.second, kv.first});
            std::sort(top.begin(), top.end(), [](const std::pair<int, int>& a, const std::pair<int, int>& b) { return a.first != b.first ? a.first > b.first : a.second < b.second; });
            auto nm = [&](size_t i) { return std::string(i < top.size() ? sg::tileTypeName(top[i].second) : "land"); };
            std::string desc = std::to_string(bl2::acresShown(app.blSale[t])) + " acres of " + nm(0) + ", " + nm(1) + ", and " + nm(2);
            float y = cy + 28;
            for (const std::string& ln : wrapText(app, desc, 11, (float)bl2::kTextWrapWidth)) { app.font.draw(cx, y, ln, 11, 0.13f, 0.13f, 0.4f); y += 12; }
            const std::string pr = std::string(bl2::kPriceLabel) + money(app.blPrice[t] * 100LL);
            app.font.draw(cx + (float)bl2::kTextWrapWidth - app.font.width(pr, 11), cy + 54, pr, 11, 0.13f, 0.13f, 0.4f);
        }
        // The tract's number on its map tile (large digits), the diamond highlight under the pointer, and the yellow ball on its card.
        const float mx = 400.0f + ((t % 3) - (t / 3)) * 96.0f, my = 440.0f + ((t % 3) + (t / 3) - 2) * 47.5f;
        const bool hot = app.blSel == t;
        if (hot) {
            ui::drawImage(app.landBtn, mx - 96, my - 47, (float)bl::diamondHighlight.x, (float)bl::diamondHighlight.y, (float)bl::diamondHighlight.w, (float)bl::diamondHighlight.h);
            if (sale) ui::drawImage(app.landBtn, (float)bl::ballIdle[t].x - 2, (float)bl::ballIdle[t].y - 2, (float)bl::ballYellow(t).x, (float)bl::ballYellow(t).y, (float)bl::ballYellow(t).w, (float)bl::ballYellow(t).h);
        }
        app.font.drawCentered(mx, my + 10, std::to_string(t + 1), 30, t == 4 ? 1.0f : (hot ? 0.1f : 0.0f), t == 4 ? 1.0f : (hot ? 0.1f : 0.0f), t == 4 ? 1.0f : (hot ? 0.3f : 0.0f));
    }
    ui::drawImage(app.landBtn, (float)bl::okPos.x, (float)bl::okPos.y, (float)bl::okCut.x, (float)bl::okCut.y, (float)bl::okCut.w, (float)bl::okCut.h);
    if (app.blSel < 0) app.font.drawCentered(400, 590, "I don't think I'll buy any land. (Esc or the tick to leave)", 12, 0.9f, 0.9f, 1);
    if (!app.blNote.empty()) { ui::fillRect(200, 200, 400, 50, 0.45f, 0.05f, 0.05f, 0.95f); app.font.drawCentered(400, 230, app.blNote, 14, 1, 1, 1); }
    ui::endScreen();
}
static void buyTract(App& app, int t) {
    if (t < 0 || t > 8 || !tractForSale(app, t)) { snd(app, "Interface/Button1.wav"); return; }
    const int price = app.blPrice[t];
    if (sg::costs::purchaseNeedsConfirm(price, (int)(app.econ.cash / 100), app.econ.sandbox, (int)app.allHoles.size())) {
        app.blNote = "This change costs " + money(price * 100LL) + ". You have only " + money((long long)app.econ.cash) + "."; snd(app, "Interface/Button1.wav"); return;
    }
    app.econ.spend(price * 100.0); if (!app.econ.sandbox) app.econ.book(sg::costs::Other, -price * 100.0);
    app.ownMask |= 1 << t; app.landBought++; app.landOffer = false; app.blRolled = false; app.blNote.clear();
    app.vstate.commissionerCount = std::max(app.vstate.commissionerCount, 0);
    logEv(app, 0x140, t); addHighlight(app, "Additional land purchased");
    app.dirty = true; bsysRebuild(app);
    snd(app, "Interface/Building.wav", 0.8f);
}


// ---- Course overview, F5 (docs/UI_SCREENS2.md section 5 and UI_SCREENS.md section 9) ----
namespace ov2 = sg::ui_screens2::overview2;
static void ovTileXY(const App& app, float tx, float ty, float& sx, float& sy) {
    const float s = 312.0f / (float)std::max(app.terrain.w, app.terrain.h);
    sx = 400.0f + (tx - ty) * s; sy = 440.0f + (tx + ty - (float)(app.terrain.w + app.terrain.h) * 0.5f) * s * 0.5f;
}
static void ovComputeHeat(App& app) {
    const Terrain& t = app.terrain; const int w = t.w, h = t.h;
    app.ovAura.assign((size_t)w * h, 0.5f); app.ovValue.assign((size_t)w * h, 0.0f);
    const float ox = -w * kTileSize * 0.5f, oz = -h * kTileSize * 0.5f;
    std::vector<float> aura((size_t)w * h, 0.0f), wgt((size_t)w * h, 0.0f);
    for (size_t hi = 0; hi < app.allHoles.size(); hi++) {
        const HoleRoute& r = app.allHoles[hi];
        float mood = 0;
        if (hi < app.holeStats.size() && app.holeStats[hi].plays) mood = std::clamp((float)app.holeStats[hi].moodSum / (float)app.holeStats[hi].plays, -1.0f, 1.0f);
        for (size_t k = 0; k + 1 < r.route.size(); k += 2) {
            const int cx = (int)((r.route[k] - ox) / kTileSize), cy = (int)((r.route[k + 1] - oz) / kTileSize);
            for (int dy = -5; dy <= 5; dy++) for (int dx = -5; dx <= 5; dx++) {
                const int x = cx + dx, y = cy + dy; if (x < 0 || y < 0 || x >= w || y >= h) continue;
                const float wt = 1.0f / (1.0f + dx * dx + dy * dy); aura[(size_t)t.tileIndex(x, y)] += mood * wt; wgt[(size_t)t.tileIndex(x, y)] += wt;
            }
        }
    }
    for (size_t i = 0; i < aura.size(); i++) app.ovAura[i] = wgt[i] > 0 ? std::clamp(0.5f + 0.5f * aura[i] / wgt[i], 0.0f, 1.0f) : -1.0f;   // -1: no comments there
    // Home site value (docs/DECODE_HOMES.md section 5): per land tile, the quarter lot value minus the site work, scaled to a green channel; dark red where a
    // lot cannot be built and grey outside the course. Stored here as 0..1 green, -1 dark red, -2 grey.
    { HomeLotEnv env; makeHomeEnv(app, env); env.bc.lotValueUnits = 9999;
      for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
          const size_t i = (size_t)t.tileIndex(x, y);
          if (!isOwned(app, x, y)) { app.ovValue[i] = -2.0f; continue; }
          const sg::PlaceCheck pc = app.bsys.canPlace(sg::buildings_exe::HomeSite, x, y, env.bc);
          const int g = pc.ok ? sg::homes::mapGreen(env.lot(x, y), pc.siteUnits) : -1;
          app.ovValue[i] = g < 0 ? -1.0f : (float)g / 31.0f;
      } }
}
static void ovTileColour(const App& app, int tx, int ty, float& r, float& g, float& b) {
    const Terrain& t = app.terrain; const size_t i = (size_t)t.tileIndex(tx, ty); const int type = t.type[i];
    switch (type) {
        case 0: r = 0.45f; g = 0.85f; b = 0.55f; break;
        case 1: case 26: r = 0.55f; g = 0.95f; b = 0.4f; break;
        case 2: case 3: r = 0.35f; g = 0.72f; b = 0.3f; break;
        case 4: case 5: case 10: case 11: r = 0.24f; g = 0.5f; b = 0.24f; break;
        case 13: r = 0.1f; g = 0.34f; b = 0.14f; break;
        case 17: case 23: case 24: case 25: r = 0.2f; g = 0.4f; b = 0.8f; break;
        case 7: case 8: case 9: case 27: case 34: r = 0.88f; g = 0.82f; b = 0.55f; break;
        case 12: r = 0.55f; g = 0.55f; b = 0.55f; break;
        case 22: r = 0.6f; g = 0.4f; b = 0.25f; break;
        default: r = 0.3f; g = 0.5f; b = 0.3f; break;
    }
    if (t.pathAt(tx, ty)) { r = 0.8f; g = 0.72f; b = 0.5f; }
    if (!isOwned(app, tx, ty)) { r *= 0.4f; g *= 0.4f; b *= 0.45f; }
}
static void drawOverview(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0, 0, 0, 1);
    if (app.ovBottom.tex) ui::drawImage(app.ovBottom, 0, 253, 0, 253, 800, 347); else ui::fillRect(0, 253, 800, 347, 0.3f, 0.3f, 0.5f, 1);
    // The map: one diamond per tile through the port's own iso fit of the black window (the exe's transform is not read exactly).
    const Terrain& t = app.terrain;
    glDisable(GL_TEXTURE_2D); glDisable(GL_LIGHTING); glDisable(GL_DEPTH_TEST);
    const float s = 312.0f / (float)std::max(t.w, t.h), hx = s, hy = s * 0.5f;
    glBegin(GL_QUADS);
    for (int ty = 0; ty < t.h; ty++) for (int tx = 0; tx < t.w; tx++) {
        float r, g, b; ovTileColour(app, tx, ty, r, g, b);
        const size_t i = (size_t)t.tileIndex(tx, ty);
        if (app.ovMode == 1 && i < app.ovAura.size() && app.ovAura[i] >= 0) { const float a = app.ovAura[i]; r = r * 0.3f + (1 - a) * 0.8f; g = g * 0.3f + a * 0.8f; b = b * 0.3f; }
        else if (app.ovMode == 2 && i < app.ovValue.size()) { const float a = app.ovValue[i]; if (a <= -1.5f) { r = 0.13f; g = 0.13f; b = 0.16f; } else if (a < 0) { r = 0.28f; g = 0.03f; b = 0.03f; } else { r = 0.0f; g = a; b = 0.0f; } }
        float sx, sy; ovTileXY(app, (float)tx + 0.5f, (float)ty + 0.5f, sx, sy);
        glColor3f(r, g, b);
        glVertex2f(sx, sy - hy); glVertex2f(sx + hx, sy); glVertex2f(sx, sy + hy); glVertex2f(sx - hx, sy);
    }
    glEnd();
    // Hole routes: thin white lines, the selected one thick with a tee flag.
    for (size_t hi = 0; hi < app.holes.size(); hi++) {
        glLineWidth((int)hi == app.ovSel ? 3.0f : 1.0f); glColor3f(1, 1, (int)hi == app.ovSel ? 0.2f : 1);
        glBegin(GL_LINE_STRIP);
        const std::vector<float>& rt = app.holes[hi].route;
        for (size_t k = 0; k + 1 < rt.size(); k += 2) { float sx, sy; ovTileXY(app, (rt[k] + t.w * kTileSize * 0.5f) / kTileSize, (rt[k + 1] + t.h * kTileSize * 0.5f) / kTileSize, sx, sy); glVertex2f(sx, sy); }
        glEnd();
    }
    if (app.ovHead[app.ovMode == -1 ? 0 : app.ovMode == 1 ? 1 : app.ovMode == 2 ? 2 : 3].tex) ui::drawImage(app.ovHead[app.ovMode == -1 ? 0 : app.ovMode], 0, 0, 0, 0, 800, 253);
    // Selected mode button in yellow, over the idle one baked into the art.
    struct Btn { int mode; up::Rect cut; int dx, dy; };
    static const Btn btns[4] = {{3, {0, 114, 129, 113}, 60, 313}, {2, {130, 114, 130, 113}, 612, 313}, {-1, {261, 118, 134, 117}, 182, 254}, {1, {396, 118, 139, 117}, 483, 254}};
    for (const Btn& b : btns) if (b.mode == app.ovMode && app.ovBottom.tex) ui::drawImage(app.ovBottom, (float)b.dx, (float)b.dy, (float)b.cut.x, (float)b.cut.y, (float)b.cut.w, (float)b.cut.h);
    if (app.ovBottom.tex) ui::drawImage(app.ovBottom, 71, 533, 536, 0, 65, 65);
    const float k0 = 0, k1 = 0, k2 = 0;
    app.font.drawCentered(400, 33, app.ovMode == 3 ? "EMPLOYEES" : "ROUTING MAP", 20, 0.15f, 0.12f, 0.3f);
    app.font.drawCentered(400, app.ovSel >= 0 ? 68.0f : 58.0f, app.courseName, 15, k0, k1, k2);
    if (app.ovMode == -1) {
        app.font.drawCentered(400, 99, "COURSE ROUTING", 13, 0.15f, 0.12f, 0.3f);
        app.font.drawCentered(400, 120, "Left click to select hole.", 12, 0, 0, 0);
        app.font.drawCentered(400, 138, "Right click to swap holes.", 12, 0, 0, 0);
        static const char* heads[4] = {"Hole #", "PAR", "YDS", "Time"}; static const int hxs[4] = {71, 132, 182, 239};
        for (int side = 0; side < 2; side++) for (int c = 0; c < 4; c++) app.font.drawCentered((float)(hxs[c] + (side ? 0x1f2 : 0)), 97, heads[c], 11, 0.15f, 0.12f, 0.3f);
        for (int i = 1; i <= 18; i++) {
            const bool have = i <= (int)app.holes.size();
            const int y = ov2::rowY(i) + 11;
            if (have && app.ovSel == i - 1) { ui::fillRect((float)ov2::selOuter(i).x, (float)ov2::selOuter(i).y, (float)ov2::selOuter(i).w, (float)ov2::selOuter(i).h, 0.14f, 0.7f, 0.25f, 1); ui::fillRect((float)ov2::selInner(i).x, (float)ov2::selInner(i).y, (float)ov2::selInner(i).w, (float)ov2::selInner(i).h, 1, 1, 1, 1); }
            const float c = have ? 0.0f : 0.4f;
            const int sh = ov2::colShift(i);
            app.font.drawCentered((float)(i < 10 ? 71 : 569), (float)y, std::to_string(i), 12, c, c, c);
            if (have) {
                const int yds = (size_t)(i - 1) < app.hstats.size() ? app.hstats[(size_t)(i - 1)].yards : (int)(app.holes[(size_t)(i - 1)].length * 0.15f);
                app.font.drawCentered((float)(132 + sh), (float)y, std::to_string(app.holes[(size_t)(i - 1)].par), 12, 0, 0, 0);
                app.font.drawCentered((float)(182 + sh), (float)y, std::to_string(yds), 12, 0, 0, 0);
                if ((size_t)(i - 1) < app.holeStats.size() && app.holeStats[(size_t)(i - 1)].plays)
                    app.font.drawCentered((float)(239 + sh), (float)y, std::to_string((int)(app.holeStats[(size_t)(i - 1)].seconds / app.holeStats[(size_t)(i - 1)].plays * 13.0 / ov2::kTicksPerMinute)) + ov2::kTimeSuffix, 12, 0, 0, 0);
            }
        }
    } else if (app.ovMode == 1) {
        app.font.drawCentered(400, 99, "AURA", 13, 0.15f, 0.12f, 0.3f);
        { float y = 124; for (const std::string& ln : wrapText(app, ov2::kAuraHelp, 12, 300)) { app.font.draw(250, y, ln, 12, 0, 0, 0); y += 15; } }
        app.font.drawCentered(400, 202, "COURSE AURA", 13, 0.15f, 0.12f, 0.3f);
        app.font.draw(136, 220, "Unhappy", 12, 0, 0, 0); app.font.draw(624, 220, "Happy", 12, 0, 0, 0);
    } else if (app.ovMode == 2) {
        app.font.drawCentered(400, 202, "HOME SITE VALUE", 13, 0.15f, 0.12f, 0.3f);
        for (int i = 0; i < 5; i++) { app.font.draw((float)ov2::kValueLeftX, (float)ov2::kValueRowY[i] + 11, ov2::kValueLeft[i], 12, 0, 0, 0); app.font.draw((float)ov2::kValueRightX, (float)ov2::kValueRowY[i] + 11, ov2::kValueRight[i], 12, 0, 0, 0); }
        app.font.draw(142, 220, "Low", 12, 0, 0, 0); app.font.draw(631, 220, "High", 12, 0, 0, 0);
    } else {
        static const char* kind[4] = {"Club pros", "Rangers", "Groundskeepers", "Soda vendors"};
        for (int k = 0; k < 4; k++) app.font.draw(260, 112.0f + k * 17, std::string(kind[k]) + ": " + std::to_string(app.econ.staff[k]), 13, 0, 0, 0);
        app.font.draw(260, 112.0f + 4 * 17, "paid: " + money((long long)app.econ.dailyWages()) + " a day", 13, 0, 0, 0);
    }
    ui::endScreen();
}
// Routing map: right click on a second hole while one is selected swaps their places in the play order (the exe's way of renumbering holes).
static void swapHoles(App& app, int a, int b) {
    if (a == b || a < 0 || b < 0 || a >= (int)app.holes.size() || b >= (int)app.holes.size() || app.openKeys.size() != app.holes.size()) return;
    app.autoOpen = false;   // a chosen order is kept, so holes are no longer opened automatically in tile order
    std::swap(app.openKeys[(size_t)a], app.openKeys[(size_t)b]);
    refreshHoles(app);
    if (app.holeStats.size() == app.holes.size()) std::swap(app.holeStats[(size_t)a], app.holeStats[(size_t)b]);
    if (app.hstats.size() == app.holes.size()) std::swap(app.hstats[(size_t)a], app.hstats[(size_t)b]);
    if (app.ratings.size() == app.holes.size()) std::swap(app.ratings[(size_t)a], app.ratings[(size_t)b]);
    for (Golfer& g : app.golfers) if (g.active) { if (g.hole == a) g.hole = b; else if (g.hole == b) g.hole = a; }   // golfers keep playing the same physical hole
    app.ovSel = b; app.dirty = true;
    snd(app, "Interface/Button1.wav");
    char m[64]; std::snprintf(m, sizeof m, "Holes %d and %d swapped", a + 1, b + 1); toastMsg(app, m);
}
static void overviewClick(App& app, float vx, float vy, bool right) {
    if (right && app.ovMode == -1 && app.ovSel >= 0) for (int i = 1; i <= (int)app.holes.size(); i++) {
        const sg::ui_screens2::Rect o = ov2::selOuter(i);
        if (vx >= o.x && vx < o.x + o.w && vy >= o.y && vy < o.y + o.h) { swapHoles(app, app.ovSel, i - 1); return; }
    }
    if (right || (vx >= 71 && vx < 136 && vy >= 533 && vy < 598)) { app.screen = App::ScreenPlay; app.hover = -1; return; }
    struct Disc { float x, y; int mode; };
    static const Disc d[4] = {{100, 355, 3}, {700, 355, 2}, {225, 295, -1}, {582, 295, 1}};
    for (const Disc& b : d) if ((vx - b.x) * (vx - b.x) + (vy - b.y) * (vy - b.y) < 38 * 38) { app.ovMode = b.mode; if (b.mode == 1 || b.mode == 2) ovComputeHeat(app); return; }
    if (app.ovMode == -1) for (int i = 1; i <= (int)app.holes.size(); i++) {
        const sg::ui_screens2::Rect o = ov2::selOuter(i);
        if (vx >= o.x && vx < o.x + o.w && vy >= o.y && vy < o.y + o.h) { app.ovSel = i - 1; return; }
    }
}

// Saved games: the terrain section is written by Terrain::save; a GAME section follows it with the money, the date, the staff and the buildings.
// Terrain::load stops at the first word it does not know, so older course files (and the editor's F5/F9) still work.
static bool saveGame(App& app, const std::string& file, std::string& err) {
    if (!app.terrain.save(file, err)) return false;
    FILE* f = std::fopen(file.c_str(), "a");
    if (!f) { err = "cannot append to " + file; return false; }
    std::fprintf(f, "GAME 2\n%s\n", app.courseName.c_str());
    std::fprintf(f, "%.0f %.0f %d %d %d %d\n", app.econ.cash, app.econ.startCash, app.econ.sandbox ? 1 : 0, app.econ.day, app.theme, app.difficulty);
    std::fprintf(f, "%d %d %d %d\n", app.econ.staff[0], app.econ.staff[1], app.econ.staff[2], app.econ.staff[3]);
    std::fprintf(f, "%zu\n", app.buildings.size());
    for (const App::Placed& b : app.buildings) std::fprintf(f, "%d %d %d\n", b.def + 100 * (b.lvl - 1), b.tx, b.ty);
    std::fprintf(f, "%d %d %d %zu\n", app.unlockLevel, app.courseStage, app.autoOpen ? 1 : 0, app.openKeys.size());
    for (long k : app.openKeys) std::fprintf(f, "%ld\n", k);
    { std::vector<int> done; for (int g = 0; g < sg::kGoalCount; g++) if (app.tracker.done(g)) done.push_back(g);
      std::fprintf(f, "%zu %d %d\n", done.size(), app.vipDay, app.investors);
      for (int g : done) std::fprintf(f, "%d\n", g); }
    std::fprintf(f, "%d\n%s\n", app.charFemale ? 1 : 0, app.charName.c_str());
    // Club systems: visitor counters, the 75 member identities, the tournament offer month and fame.
    std::fprintf(f, "SOCIAL 1\n%d %d %d %u %d %d\n", app.vstate.ceoCount, app.vstate.commissionerCount, app.vstate.heiressVisits, app.vstate.landmarkMask, app.fame, app.rosterReady ? 1 : 0);
    if (app.rosterReady) { for (int i = 1; i <= sg::Roster::kPool; i++) std::fprintf(f, "%d %d ", (int)app.roster.e[i].tier, app.roster.e[i].resigned ? 1 : 0); std::fprintf(f, "\n"); }
    std::fprintf(f, "LAVAIL 1 %u\n", app.vstate.availMask);
    std::fprintf(f, "LAND 1 %d %d %d %d\n", app.landModel ? 1 : 0, app.ownMask, app.landBought, app.landOffer ? 1 : 0);
    std::fprintf(f, "CAREER 1 %d %u\n", app.curProp, app.careerOwned);
    std::fprintf(f, "HOMES 1 %zu %d\n", app.homes.size(), app.homeSales);
    for (const App::Home& h : app.homes) std::fprintf(f, "%d %d %d %d\n", h.x, h.y, h.buyer, h.value);
    std::fprintf(f, "AMEN 1 %zu\n", app.amen.size());
    for (const App::Amen& m : app.amen) std::fprintf(f, "%d %d %d %d\n", m.kind, m.tx, m.ty, m.var);
    // Progress records: skilled staff, the Financial Report ledger, year history, Histograph series, accomplishments and employee counters.
    std::fprintf(f, "STATE 1\n%d %d %d %d %d\n", app.econ.skilled[0], app.econ.skilled[1], app.econ.skilled[2], app.econ.skilled[3], app.econ.ledger.monthCounter);
    { int nz = 0; for (int m = 0; m < sg::costs::kLedgerMonths; m++) for (int r = 0; r < sg::costs::LedgerRows; r++) if (app.econ.ledger.month[m].row[r]) nz++;
      std::fprintf(f, "%d\n", nz);
      for (int m = 0; m < sg::costs::kLedgerMonths; m++) for (int r = 0; r < sg::costs::LedgerRows; r++) if (app.econ.ledger.month[m].row[r]) std::fprintf(f, "%d %d %d\n", m, r, (int)app.econ.ledger.month[m].row[r]); }
    std::fprintf(f, "%zu %d %d\n", app.yearHist.size(), app.eoyYear, app.lastYear);
    for (const App::YearRec& y : app.yearHist) std::fprintf(f, "%lld %d %.4f %d\n", y.cash, y.fun, y.skill, y.members);
    std::fprintf(f, "%d\n", app.hgLastDay);
    for (const std::vector<short>* v : {&app.hgSkill, &app.hgCash, &app.hgFun, &app.hgStaff, &app.evLog}) { for (short x : *v) std::fprintf(f, "%d ", (int)x); std::fprintf(f, "\n"); }
    for (int i = 0; i < 22; i++) std::fprintf(f, "%ld\n%s\n", app.miles[i].tick, app.miles[i].course.c_str());
    for (int k = 0; k < 4; k++) std::fprintf(f, "%d %d ", app.empCount[k][0], app.empCount[k][1]);
    std::fprintf(f, "\nBEST 1 %d\n", app.best.count);
    for (int i = 0; i < app.best.count; i++) std::fprintf(f, "%d %s\n", app.best.score[i], app.best.name[i]);
    if (app.holes.empty()) refreshHoles(app);
    if (app.hstats.size() != app.holes.size()) syncHoleStats(app);
    { int par = 0, yards = 0; for (size_t i = 0; i < app.holes.size() && i < app.hstats.size(); i++) { par += app.holes[i].par; yards += app.hstats[i].yards; }
      if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 20, app.difficulty)); }
      double lenS = 0, accS = 0, imgS = 0; if (app.ratings.size() == app.holes.size()) for (const HoleRating& r : app.ratings) { lenS += r.len; accS += r.acc; imgS += r.img; }
      std::fprintf(f, "INFO 1 %zu %d %d %.0f %d %.0f %.0f %.0f %d\n", app.holes.size(), par, yards, app.econ.cash, clubFun(app), lenS * 100, accS * 100, imgS * 100, app.themePack);
      for (size_t i = 0; i < app.holes.size() && i < app.hstats.size(); i++) std::fprintf(f, "%d %d\n", app.holes[i].par, app.hstats[i].yards);
      std::fprintf(f, "%s\n", app.charName.empty() ? "Gary Golf" : app.charName.c_str()); }
    std::fprintf(f, "END\n");
    std::fclose(f);
    app.thumbReq = file + ".thumb";   // the course view is captured on the next frame for the Load screen
    return true;
}
static bool loadGame(App& app, const std::string& file, std::string& err) {
    Terrain t;
    if (!Terrain::load(file, t, err)) return false;
    std::string name = "Saved Course"; double cash = 0, start = 0; int sandbox = 0, day = 1, theme = app.theme, diff = app.difficulty, st[4] = {}; std::vector<App::Placed> bl; int unl = 17, stage = 0; bool autoOpenLoaded = true; std::vector<long> keys; std::set<int> goalsLoaded; int vday = 0, inv = 0; std::string cname = app.charName; bool cfem = app.charFemale;
    bool haveGame = false;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            const int gver = std::strncmp(line, "GAME 2", 6) == 0 ? 2 : (std::strncmp(line, "GAME 1", 6) == 0 ? 1 : 0);
            if (!gver) continue;
            char nm[256] = {};
            if (!std::fgets(nm, sizeof nm, f)) break;
            nm[std::strcspn(nm, "\r\n")] = 0; name = nm;
            int n = 0;
            if (std::fscanf(f, "%lf %lf %d %d %d %d", &cash, &start, &sandbox, &day, &theme, &diff) != 6) break;
            if (std::fscanf(f, "%d %d %d %d", &st[0], &st[1], &st[2], &st[3]) != 4) break;
            if (std::fscanf(f, "%d", &n) != 1 || n < 0 || n > 500) break;
            for (int i = 0; i < n; i++) { int d, x, y; if (std::fscanf(f, "%d %d %d", &d, &x, &y) != 3) break; const int lv = d / 100 + 1; d %= 100; if (d >= 0 && d < kBuildCount) bl.push_back({d, x, y, std::min(lv, 2)}); }
            if (gver == 2) {
                int nOpen = 0, auto_ = 1;
                if (std::fscanf(f, "%d %d %d %d", &unl, &stage, &auto_, &nOpen) == 4 && nOpen >= 0 && nOpen <= 64) {
                    autoOpenLoaded = auto_ != 0;
                    for (int i = 0; i < nOpen; i++) { long k; if (std::fscanf(f, "%ld", &k) == 1) keys.push_back(k); }
                    int ng = 0; if (std::fscanf(f, "%d %d %d", &ng, &vday, &inv) == 3 && ng >= 0 && ng <= 64) for (int i = 0; i < ng; i++) { int g; if (std::fscanf(f, "%d", &g) == 1) goalsLoaded.insert(g); }
                    { int fem = 0; char nm2[64] = {}; if (std::fscanf(f, "%d", &fem) == 1 && std::fgets(nm2, sizeof nm2, f) && std::fgets(nm2, sizeof nm2, f)) { nm2[std::strcspn(nm2, "\r\n")] = 0; if (nm2[0]) { cname = nm2; cfem = fem != 0; } } }
                }
            }
            haveGame = true;
            break;
        }
        std::fclose(f);
    }
    sg::VisitorState vsLoaded; sg::Roster rosterLoaded; bool haveRoster = false; int fameLoaded = 0;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            if (std::strncmp(line, "SOCIAL 1", 8) != 0) continue;
            int rr = 0; unsigned lm = 0;
            if (std::fscanf(f, "%d %d %d %u %d %d", &vsLoaded.ceoCount, &vsLoaded.commissionerCount, &vsLoaded.heiressVisits, &lm, &fameLoaded, &rr) == 6) {
                vsLoaded.landmarkMask = lm;
                if (rr) { bool ok = true; for (int i = 1; i <= sg::Roster::kPool && ok; i++) { int tr = 0, rs = 0; if (std::fscanf(f, "%d %d", &tr, &rs) != 2) ok = false; else { rosterLoaded.e[i].tier = (sg::Tier)std::clamp(tr, 0, 5); rosterLoaded.e[i].resigned = rs != 0; } } haveRoster = ok; }
            }
            break;
        }
        std::fclose(f);
    }
    vsLoaded.availMask = 0x3fffu;   // older saves had every design on the strip
    if (FILE* f = std::fopen(file.c_str(), "r")) { char line[512]; while (std::fgets(line, sizeof line, f)) if (!std::strncmp(line, "LAVAIL 1", 8)) { unsigned am = 0x3fffu; if (std::sscanf(line + 8, "%u", &am) == 1) vsLoaded.availMask = am; break; } std::fclose(f); }
    int landModelL = 0, ownMaskL = 0x1ff, landBoughtL = 0, landOfferL = 0;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) if (!std::strncmp(line, "LAND 1", 6)) { std::sscanf(line + 6, "%d %d %d %d", &landModelL, &ownMaskL, &landBoughtL, &landOfferL); break; }
        std::fclose(f);
    }
    int curPropL = -1; unsigned careerL = 0;
    if (FILE* f = std::fopen(file.c_str(), "r")) { char line[512]; while (std::fgets(line, sizeof line, f)) if (!std::strncmp(line, "CAREER 1", 8)) { std::sscanf(line + 8, "%d %u", &curPropL, &careerL); break; } std::fclose(f); }
    std::vector<App::Home> homesLoaded; int salesLoaded = 0;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            if (std::strncmp(line, "HOMES 1", 7) != 0) continue;
            size_t n = 0; if (std::sscanf(line + 7, "%zu %d", &n, &salesLoaded) != 2) break;
            for (size_t i = 0; i < n && i < 256; i++) { App::Home h; if (std::fscanf(f, "%d %d %d %d", &h.x, &h.y, &h.buyer, &h.value) != 4) break; homesLoaded.push_back(h); }
            break;
        }
        std::fclose(f);
    }
    std::vector<App::Amen> amenLoaded;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            if (std::strncmp(line, "AMEN 1", 6) != 0) continue;
            size_t n = 0; if (std::sscanf(line + 6, "%zu", &n) != 1) break;
            for (size_t i = 0; i < n && i < 4000; i++) { App::Amen m; if (std::fscanf(f, "%d %d %d %d", &m.kind, &m.tx, &m.ty, &m.var) != 4) break; if (m.kind >= 0 && m.kind <= 5) amenLoaded.push_back(m); }
            break;
        }
        std::fclose(f);
    }
    sg::BestScores bestL;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            if (std::strncmp(line, "BEST 1", 6) != 0) continue;
            int n = 0; if (std::sscanf(line + 6, "%d", &n) != 1) break;
            for (int i = 0; i < n && i < sg::BestScores::kMax; i++) {
                if (!std::fgets(line, sizeof line, f)) break;
                int sc = 0, off = 0; if (std::sscanf(line, "%d %n", &sc, &off) < 1) break;
                std::string nm = line + off; while (!nm.empty() && (nm.back() == '\n' || nm.back() == '\r')) nm.pop_back();
                bestL.insert(sc, nm);
            }
            break;
        }
        std::fclose(f);
    }
    struct StateL { bool ok = false; int skilled[4] = {}; sg::costs::Ledger ledger; std::vector<App::YearRec> years; int eoy = 0, last = 0, hgDay = 1; std::vector<short> hg[5]; long tick[22] = {}; std::string course[22]; int emp[4][2] = {}; } stl;
    if (FILE* f = std::fopen(file.c_str(), "r")) {
        char line[512];
        while (std::fgets(line, sizeof line, f)) {
            if (std::strncmp(line, "STATE 1", 7) != 0) continue;
            bool ok = std::fscanf(f, "%d %d %d %d %d", &stl.skilled[0], &stl.skilled[1], &stl.skilled[2], &stl.skilled[3], &stl.ledger.monthCounter) == 5;
            int nz = 0; ok = ok && std::fscanf(f, "%d", &nz) == 1 && nz >= 0 && nz <= 100000;
            for (int i = 0; ok && i < nz; i++) { int m, r, v; if (std::fscanf(f, "%d %d %d", &m, &r, &v) != 3) { ok = false; break; } if (m >= 0 && m < sg::costs::kLedgerMonths && r >= 0 && r < sg::costs::LedgerRows) stl.ledger.month[m].row[r] = (int16_t)v; }
            size_t ny = 0; ok = ok && std::fscanf(f, "%zu %d %d", &ny, &stl.eoy, &stl.last) == 3 && ny <= 1000;
            for (size_t i = 0; ok && i < ny; i++) { App::YearRec y; if (std::fscanf(f, "%lld %d %lf %d", &y.cash, &y.fun, &y.skill, &y.members) != 4) { ok = false; break; } stl.years.push_back(y); }
            ok = ok && std::fscanf(f, "%d", &stl.hgDay) == 1;
            for (int a = 0; ok && a < 5; a++) { stl.hg[a].assign(500, 0); for (int i = 0; i < 500; i++) { int x; if (std::fscanf(f, "%d", &x) != 1) { ok = false; break; } stl.hg[a][(size_t)i] = (short)x; } }
            for (int i = 0; ok && i < 22; i++) { char nm[256] = {}; if (std::fscanf(f, "%ld", &stl.tick[i]) != 1 || !std::fgets(nm, sizeof nm, f) || !std::fgets(nm, sizeof nm, f)) { ok = false; break; } nm[std::strcspn(nm, "\r\n")] = 0; stl.course[i] = nm; }
            for (int k = 0; ok && k < 4; k++) if (std::fscanf(f, "%d %d", &stl.emp[k][0], &stl.emp[k][1]) != 2) ok = false;
            stl.ok = ok; break;
        }
        std::fclose(f);
    }
    app.terrain = std::move(t);
    app.best = haveGame ? bestL : sg::BestScores();
    if (haveGame) { app.curProp = curPropL; app.careerOwned = careerL | (curPropL >= 0 ? 1u << curPropL : 0u); } else { app.curProp = -1; app.careerOwned = 0; }
    app.amen = haveGame ? amenLoaded : std::vector<App::Amen>();
    app.homes = haveGame ? homesLoaded : std::vector<App::Home>(); app.homeSales = haveGame ? salesLoaded : 0; app.homeMonth = -1; app.homeConfirm = -1;
    app.landModel = haveGame && landModelL && app.terrain.w == 50 && app.terrain.h == 50; app.ownMask = app.landModel ? (ownMaskL & 0x1ff) : 0x1ff; app.landBought = landBoughtL; app.landOffer = landOfferL != 0;
    app.tracker = sg::GoalTracker(std::clamp(diff, 0, 3));
    for (int g : goalsLoaded) app.tracker.mark(g);
    if (app.econ.sandbox) vsLoaded.availMask |= 0x3fffu;
    app.vstate = vsLoaded; app.fame = fameLoaded; app.roster = rosterLoaded; app.rosterReady = haveRoster; app.tourney.cancel(); app.offerMonth = -1; app.hstats.clear();
    if (haveGame) {
        app.courseName = name; app.econ.sandbox = sandbox != 0; app.econ.startCash = start; app.econ.day = day; app.difficulty = std::clamp(diff, 0, 3);
        if (theme != app.theme && theme >= 0 && theme < 4) loadTheme(app, theme);
        app.buildings = bl;
        app.unlockLevel = unl; app.courseStage = stage; app.autoOpen = autoOpenLoaded; app.openKeys = keys; app.vipDay = vday; app.investors = inv; app.charName = cname; app.charFemale = cfem;
    } else { app.buildings.clear(); app.econ.sandbox = false; app.unlockLevel = 17; app.courseStage = 0; app.autoOpen = true; app.openKeys.clear(); app.vipDay = 0; app.investors = 0; }
    rebuildBatches(app); populateProps(app);
    if (haveGame) { app.econ.cash = cash; for (int i = 0; i < 4; i++) app.econ.staff[i] = st[i]; app.econ.version++; }
    if (haveGame && stl.ok) {
        for (int i = 0; i < 4; i++) app.econ.skilled[i] = stl.skilled[i];
        app.econ.ledger = stl.ledger; app.yearHist = stl.years; app.eoyYear = stl.eoy; app.lastYear = stl.last; app.hgLastDay = stl.hgDay;
        app.hgSkill = stl.hg[0]; app.hgCash = stl.hg[1]; app.hgFun = stl.hg[2]; app.hgStaff = stl.hg[3]; app.evLog = stl.hg[4];
        for (int i = 0; i < 22; i++) { app.miles[i].tick = stl.tick[i]; app.miles[i].course = stl.course[i]; }
        for (int k = 0; k < 4; k++) { app.empCount[k][0] = stl.emp[k][0]; app.empCount[k][1] = stl.emp[k][1]; }
    }
    app.dirty = true; app.panel = 0; app.edit = false; app.screen = App::ScreenPlay; app.resetClock = true;
    return true;
}

// World map course switching (F6). Every property you own keeps its own saved course next to the main save file; cash and the date carry across.
// PLACEHOLDER: the exe's handling of club-wide state across courses (goals, fame, roster) is not decoded; a bought course starts like a new game and keeps
// the cash and date, and a switched-to course restores what it had when you left.
static void startGame(App& app, int propIdx, bool sandbox);
static std::string careerSlot(const App& app, int i) { return app.courseFile + ".c" + std::to_string(i); }
static void switchCourse(App& app, int target) {
    const bool owned = app.careerOwned >> target & 1;
    const double cash = app.econ.cash; const int day = app.econ.day; const bool sb = app.econ.sandbox; std::string err;
    if (app.curProp >= 0) saveGame(app, careerSlot(app, app.curProp), err);
    unsigned mask = app.careerOwned;
    app.switchMode = false;
    if (owned) {
        if (!loadGame(app, careerSlot(app, target), err)) { app.toast = "That course could not be loaded"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; app.screen = App::ScreenPlay; return; }
        app.econ.cash = cash; app.econ.day = day; app.econ.version++;
    } else {
        startGame(app, target, sb);
        app.econ.cash = cash - (sb ? 0 : propPrice(app, target)); app.econ.day = day; app.econ.version++;
        mask |= 1u << target;
    }
    app.curProp = target; app.careerOwned = mask | (1u << target);
    char m[96]; std::snprintf(m, sizeof m, "We're off to %s!", kProperties[target].name); toastMsg(app, m);
}

static void loadStory(App& app);
static void startGame(App& app, int propIdx, bool sandbox) {
    app.curProp = propIdx; app.careerOwned |= 1u << propIdx; app.best = sg::BestScores();
    const Property& p = kProperties[propIdx];
    app.econ.sandbox = sandbox;
    const int price = propPrice(app, propIdx), acres = propAcres(app, propIdx);
    app.econ.startCash = sandbox ? kStartFunds : kStartFunds - price;   // the property is paid for out of the starting funds
    app.seed = app.seed * 1664525u + 1013904223u + (uint32_t)propIdx * 7919u;
    { const auto& site = ui_screens2::worldmap::kSites[propIdx]; if (!app.worldInit) worldDefault(app);
      app.terrain = Terrain::generate(50, 50, app.seed, p.theme, site.lie, site.terrain, app.siteSlot[propIdx], sandbox); }
    // The land you start with: whole tracts, beginning with the one that holds the clubhouse, enough for the property's acres (PLACEHOLDER: the exe's
    // starting ownership per property is not decoded).
    { int order[9]; for (int i = 0; i < 9; i++) order[i] = i;
      const int cx = app.terrain.clubhouseX, cy = app.terrain.clubhouseY;
      auto d2 = [&](int t) { const int tx = 1 + (t % 3) * 16 + 8, ty = 1 + (t / 3) * 16 + 8; return (tx - cx) * (tx - cx) + (ty - cy) * (ty - cy); };
      std::stable_sort(order, order + 9, [&](int a, int b) { return d2(a) < d2(b); });   // the tract with the clubhouse first, then its neighbours (PLACEHOLDER: the exe's starting ownership is not decoded)
      const int n = std::clamp((acres + 10) / 15, 4, 9);   // PLACEHOLDER: real starts show far more owned land than acres/25 gives
      app.landModel = true; app.ownMask = 0; for (int i = 0; i < n; i++) app.ownMask |= 1 << order[i];
      app.landBought = 0; app.landOffer = false; }
    app.courseName = std::string(p.name) + " GC";
    if (!std::strcmp(p.name, "Scotland")) snd(app, "World/Bagpipe.wav", 0.7f);
    loadTheme(app, p.theme);
    app.screen = App::ScreenPlay;
    app.hover = -1;
    app.edit = false;
    app.econ.day = 1;
    app.panel = 0; app.buildings.clear(); app.amen.clear(); app.amenCounter = 0; app.homes.clear(); app.homeSales = 0; app.homeMonth = -1; app.homeConfirm = -1;
    app.autoOpen = false; app.openKeys.clear(); app.unlockLevel = sandbox ? 17 : 6; app.courseStage = 0; app.vipDay = 0; app.investors = 0;
    app.tracker = sg::GoalTracker(app.difficulty); app.vstate = sg::VisitorState(); if (sandbox) app.vstate.availMask = 0x3fffu; app.srng = sg::SocialRng((uint64_t)app.seed); app.rosterReady = false; app.tourney.cancel(); app.offerMonth = -1; app.fame = 0; app.hstats.clear();   // the exe: a normal game starts with counter 6, sandbox with everything (17)
    app.econ.staff[0] = app.econ.staff[1] = app.econ.staff[2] = app.econ.staff[3] = 0;
    // Ugly landmarks: as many as the difficulty (exe: kind 16 on parkland, 18 on desert, 17 on tropical and links), on open rough ground away from the clubhouse.
    app.amen.clear();
    { const int kind = p.theme == 0 ? 16 : p.theme == 1 ? 18 : 17; sg::SocialRng pr((uint64_t)app.seed ^ 0x9e3779b97f4a7c15ull);
      for (int placed = 0, tries = 0; placed < app.difficulty && tries < 400; tries++) {
          const int x = 6 + (int)pr.below(38), y = 6 + (int)pr.below(38);
          if (app.terrain.type[(size_t)y * 50 + x] != sg::TT_Rough) continue;
          if (std::abs(x - app.terrain.clubhouseX) + std::abs(y - app.terrain.clubhouseY) < 10) continue;
          bool dup = false; for (const App::Amen& m : app.amen) if (std::abs(m.tx - x) + std::abs(m.ty - y) < 8) dup = true;
          if (dup) continue;
          app.amen.push_back({5, x, y, kind}); placed++; } }
    rebuildBatches(app); populateProps(app); app.dirty = true;
    loadStory(app);
    app.resetClock = true;
    std::printf("new game: %s, %d acres, price %d, cash left %.0f%s\n", p.name, acres, price, app.econ.startCash, sandbox ? " (sandbox)" : "");
}

// Club ratings as the exe computes them. Fun: each hole contributes 100 * (sum of mood changes) / (plays + 4), and the club's value is
// the sum over holes (the exe also divides by half of another per-hole counter that is not decoded; taken as 0). Skill: the sum over holes
// of the Length, Accuracy and Imagination differences in strokes (the exe keeps hundredths and prints with two decimals).
static int clubFun(const App& app) {
    long long s = 0;
    for (const App::HoleStat& h : app.holeStats) if (h.plays) s += (100LL * h.moodSum) / (h.plays + 4);
    return (int)s;
}
static double clubSkill(const App& app) {
    double s = 0;
    for (const HoleRating& r : app.ratings) s += r.len + r.acc + r.img;
    return s;
}

static const char* kMonths[12] = {"January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"};

// Heads-up display over the course: club name and date, and the money and fun counters. Layout is my own, from the screenshots.
// One economy day counts as one month (1024 ticks in the exe).

// ---- In-game interface: the lower left dock, its panels, the advisor and the golfer stories -------------------------------------------------
// The dock art is the disc's own Interface/3mainLowerLeft.pcx (the assembled dock sits in the bottom left of the sheet, with the highlighted
// versions of each button elsewhere on it). Button positions are measured from that art. The panels, advisor text and layout are my own.
struct DockBtn { float cx, cy, r; float sx, sy, sw, sh, hoverDx; };   // dock centre and radius; normal sprite on the sheet; offset to the highlighted one
static const DockBtn kDock[10] = {
    {43, 473, 33, 0, 0, 76, 80, 100},     // Build Course
    {117, 497, 30, 0, 100, 70, 78, 100},  // Add Buildings
    {177, 536, 26, 0, 200, 64, 76, 100},  // People
    {32, 543, 11, 598, 48, 30, 30, 50},   // zoom in
    {31, 585, 11, 598, 98, 30, 30, 50},   // zoom out
    {17, 565, 11, 598, 148, 30, 30, 50},  // rotate right
    {47, 565, 11, 598, 198, 30, 30, 50},  // rotate left
    {75, 552, 14, 598, 248, 34, 34, 50},  // information (course report)
    {107, 568, 14, 598, 298, 34, 34, 50}, // pause
    {133, 583, 13, 598, 348, 34, 34, 50}, // tools (save the course)
};
static const char* kDockHelp[10] = {"Build Course", "Add Buildings", "People", "Zoom in", "Zoom out", "Rotate right", "Rotate left", "Information", "Pause or Unpause", "System Functions"};

// Opens the next finished hole (the exe's H key). Golfers only play open holes. Each opened hole may unlock a building type (the exe unlocks
// one when the number of open holes is greater than the unlock counter minus 5), and holes 6, 10 and 18 upgrade the course.
static const char* kUnlockEffect[15] = {"", "", "", "", "", "", "Putting Green: golfers with imagination improve their putting", "Snack Bar: feeds hungry golfers", "Pro Shop: accurate golfers upgrade equipment",
                                        "Swim Club: golfers start in a better mood", "Driving Range: long hitters gain distance", "Cart Garage: faster play", "Marina: raises home values",
                                        "Resort Hotel: golfers tire less late in a round", "Airstrip: lets you charge higher green fees"};
static bool isRefusal(const std::string& m) {
    static const char* const kP[] = {"You do not own", "Not enough money", "Not available", "Out of bounds", "Invalid", "That is not", "Nothing to undo", "This building", "You need", "No ", "There is no", "Could not", "Finish or cancel", "This course has no", "The SGA has not", "The county commissioner has not", "A match or practice"};
    for (const char* p : kP) if (m.rfind(p, 0) == 0) return true;
    return false;
}
static void say(App& app, const std::string& m, double secs) { if (isRefusal(m)) snd(app, "Interface/Wrong.wav", 0.7f); app.lastMsg = m; app.toast = m; app.toastKind = 1; app.toastUntil = SDL_GetTicks() / 1000.0 + secs; std::printf("%s\n", m.c_str()); }
static void openHole(App& app) {
    refreshHoles(app);
    int pick = -1;
    for (size_t i = 0; i < app.allHoles.size() && pick < 0; i++)
        if (std::find(app.openKeys.begin(), app.openKeys.end(), holeKey(app, app.allHoles[i])) == app.openKeys.end()) pick = (int)i;
    if (app.autoOpen && pick < 0) { say(app, "All holes are already open"); return; }
    if (pick < 0) { say(app, app.teeBlobs > (int)app.allHoles.size() ? "This hole needs a green before it can open" : "Build a tee and a green first, then press H to open the hole"); return; }
    app.autoOpen = false;
    app.openKeys.push_back(holeKey(app, app.allHoles[(size_t)pick]));
    refreshHoles(app);
    const int n = (int)app.openKeys.size();
    { const HoleRoute& hr = app.holes.back(); sg::GoalEvent e; e.kind = sg::GoalEvent::HoleOpened; e.hole = n; e.doglegDir = doglegOf(hr); e.holeClass = sg::holeClassFromYards(routeYardsRaw(hr), e.doglegDir != 0, 0); app.pendingGoal = e; app.hasPendingGoal = true; }
    addHighlight(app, "Hole " + std::to_string(n) + " opened"); logEv(app, 0x20, n);
    std::string msg = "Hole " + std::to_string(n) + " is now open for play";
    if (n == 6 || n == 10 || n == 18) {
        app.courseStage = n == 6 ? 1 : n == 10 ? 2 : 3;
        msg += std::string(". Your course is now a ") + kCourseStage[app.courseStage];
    } else if (n > app.unlockLevel - 5 && app.unlockLevel <= 14) {
        const int b = app.unlockLevel++;
        msg += std::string(". You may now build: ") + (kUnlockEffect[b][0] ? kUnlockEffect[b] : kTypeNames[b]);
    }
    { const int dl = app.hasPendingGoal ? app.pendingGoal.doglegDir : 0; if (dl) msg += dl > 0 ? " It doglegs left." : " It doglegs right."; }
    msg += n % 2 ? " New players are flocking to your course." : " As your course grows you can add more to it.";   // the exe prints one of these by odd or even hole number (DECODE_WORLD2 5.4); wording PLACEHOLDER
    say(app, msg, 8);
    if (app.hasPendingGoal) { app.hasPendingGoal = false; goalEvent(app, app.pendingGoal); }
    app.dirty = true;
    app.econ.version++;
    snd(app, "Interface/Button1.wav");
}


// Build Course panel, decoded from the publisher exe: 16 button slots at fixed screen positions. Slots 0 to 12 are isometric tile buttons cut from the
// theme's TerrainButtons sheet (62x54, four states: normal, hover, selected, disabled; cell k sits at x = (k>6 ? 248 : 0) + state*62, y = (k%7)*54).
// Slots 13 to 15 are the three tree buttons (x = 520 + state*80 on the sheet, rows at y 0, 150 and 300, three states, sizes depend on the theme).
// Button ids in slot order are 0,1,7,4,9,10,17,2,3,5,8,11,12,13,14,15 (the exe's tile ids). `paint` is our kPaint index for that id.
struct TerrSlot { int x, y, id, paint; };
static const TerrSlot kTerrSlots[16] = {
    {270, 508, 0, 3}, {332, 508, 1, 2}, {394, 508, 7, 7}, {456, 508, 4, 4}, {518, 508, 9, 8}, {580, 508, 10, 21}, {642, 508, 17, 9},
    {301, 546, 2, 0}, {363, 546, 3, 1}, {425, 546, 5, 5}, {487, 546, 8, 20}, {549, 546, 11, 13}, {611, 546, 12, 12},
    {704, 477, 13, 6}, {673, 530, 14, 6}, {735, 511, 15, 6}};
static void terrSlotRect(const App& app, int i, float& x, float& y, float& w, float& h) {
    x = (float)kTerrSlots[i].x; y = (float)kTerrSlots[i].y;
    if (i < 13) { w = 62; h = 54; return; }
    static const int sz[4][3][2] = {{{0x41, 0x54}, {0x3e, 0x46}, {0x3e, 0x58}}, {{0x4b, 0x4a}, {0x3e, 0x43}, {0x41, 0x68}}, {{0x48, 0x4f}, {0x3e, 0x51}, {0x41, 0x68}}, {{0x43, 0x54}, {0x3e, 0x58}, {0x43, 0x4c}}};
    w = (float)sz[themeExe(app.theme)][i - 13][0]; h = (float)sz[themeExe(app.theme)][i - 13][1];
}

static bool terrArt(const App& app) { return app.uiOk && app.terrPanel.tex && app.terrBtns.tex; }
static int terrSlotAt(const App& app, float vx, float vy) {
    for (int i = 15; i >= 0; i--) {
        float x, y, w, h; terrSlotRect(app, i, x, y, w, h);
        if (vx < x || vx >= x + w || vy < y || vy >= y + h) continue;
        if (i < 13) { if (std::fabs(vx - (x + 31)) / 31.0f + std::fabs(vy - (y + 24)) / 24.0f > 1.0f) continue; }
        return i;
    }
    return -1;
}
static std::string terrSlotName(const App& app, int i) {
    const int id = kTerrSlots[i].id, t = themeExe(app.theme);
    if (t == 1) { if (id == 4) return "Desert"; if (id == 5) return "Rough"; if (id == 10) return "Ravine"; if (id == 13) return "Cactus"; if (id == 14) return "Joshua tree"; if (id == 15) return "Palm tree"; }
    if (t == 2 && id == 11) return "Tropical bush";
    if (t == 2 && id == 13) return "Tropical tree";
    if (t == 3 && id == 11) return "Gorse";
    static const char* nm[18] = {"Tees", "Green", "Fairway", "Firm fairway", "Rough", "Deep rough", "Mound", "Sand trap", "Waste bunker", "Pot bunker", "Stream", "Brush", "Rocks", "Tree", "Pine tree", "Palm tree", "Elm tree", "Water"};
    return nm[id];
}

// Items in the open panel, laid out in three columns from the panel's top left.
struct PanelItem { std::string label; int kind; int arg; };   // kind: 0 paint (arg kPaint index), 1 path, 2 raise, 3 lower, 4 building (arg kBuild index), 5 staff (arg Staff)
static std::vector<PanelItem> panelItems(const App& app) {
    std::vector<PanelItem> v;
    char b[96];
    if (app.panel == 1) {
        for (int i = 0; i < 15; i++) { std::snprintf(b, sizeof b, "%s  %s", kPaint[i].name, money(Economy::terrainCostUnits(kPaint[i].type) * 100LL).c_str()); v.push_back({b, 0, i}); }
        v.push_back({"Path, gravel", 1, 1}); v.push_back({"Path, paved", 1, 2}); v.push_back({"Raise ground", 2, 0}); v.push_back({"Lower ground", 3, 0}); v.push_back({"Open the hole (H)", 6, 0});
    } else if (app.panel == 2) {
        for (int i = 0; i < kBuildCount; i++) {
            if (!themeHas(app, i)) continue;
            if (buildUnlocked(app, i)) std::snprintf(b, sizeof b, "%s  %s", kBuild[i].name, money(kBuild[i].cost * 100LL).c_str());
            else std::snprintf(b, sizeof b, "%s  (not yet open)", kBuild[i].name);
            v.push_back({b, 4, i});
        }
    } else if (app.panel == 3) {
        for (int k = 0; k < Economy::StaffKinds; k++) { std::snprintf(b, sizeof b, "%s: %d  (click to hire, right click to fire)", Economy::staffName(k), app.econ.staff[k]); v.push_back({b, 5, k}); }
    }
    return v;
}
static void panelItemRect(int i, int cols, float& x, float& y, float& w, float& h) {
    const float cw = cols == 1 ? 540.0f : 182.0f;
    x = 232 + (i % cols) * cw; y = 462 + (i / cols) * 18.5f; w = cw - 4; h = 17;
}
static int panelCols(const App& app) { return app.panel == 3 ? 1 : 3; }

static void selectPanelItem(App& app, const PanelItem& it, bool rightClick) {
    switch (it.kind) {
        case 0: app.tool = 0; app.paintIdx = it.arg; app.edit = true; break;
        case 1: app.tool = 2; app.pathKind = it.arg; app.edit = true; break;
        case 2: app.tool = 1; app.raiseSign = 1; app.edit = true; break;
        case 3: app.tool = 1; app.raiseSign = -1; app.edit = true; break;
        case 4: if (!buildUnlocked(app, it.arg)) { say(app, "This building becomes available as you open more holes", 4); break; } app.tool = 4; app.buildIdx = it.arg; app.edit = true; break;
        case 6: openHole(app); break;
        case 5: if (rightClick) app.econ.fire(it.arg); else app.econ.hire(it.arg); break;
    }
}

static bool inDiscInt(const up::Disc& d, int px, int py) { return up::inDisc(d, px, py); }

// ---- Dock panels rebuilt from the decoded layouts (docs/UI_PANELS.md, include/sg/ui_panels.h) ----
static bool pnlArt(const App& a) { return a.uiOk && a.terrPanel.tex && a.terrBtns.tex && a.amenArt.tex && a.elevArt.tex && a.bldgArt.tex && a.empArt.tex && a.memberArt.tex; }
static void pnlBlit(const ui::Image& im, const up::Rect& c, int dx, int dy) { ui::drawImage(im, (float)dx, (float)dy, (float)c.x, (float)c.y, (float)c.w, (float)c.h); }
static int lotDef(int lot) { for (int i = 0; i < kBuildCount; i++) if (kBuild[i].unlock == lot + 6) return i; return -1; }   // our building for the exe's lot (-1: not implemented yet)
static bool lotCanBuild(const App& a, int lot) { return up::lotBuildable(lot + 6, a.bsys.level(lot + 6), a.unlockLevel, (int)a.allHoles.size() + 1); }
// Staff list: the exe keeps individual employees; the port keeps counts, so the roster is rebuilt from them (skilled first within a kind).
struct EmpRow { int kind; bool skilled; };
static std::vector<EmpRow> empRows(const App& a) {
    std::vector<EmpRow> v;
    for (int k = 0; k < Economy::StaffKinds; k++) for (int n = 0; n < a.econ.staff[k]; n++) v.push_back({k, n < a.econ.skilled[k]});
    return v;
}
static int rowEmp(const App& app, const std::vector<EmpRow>& rows, int r) {
    int nth = 0; for (int i = 0; i < r; i++) if (rows[(size_t)i].kind == rows[(size_t)r].kind && rows[(size_t)i].skilled == rows[(size_t)r].skilled) nth++;
    return empForRow(app, rows[(size_t)r].kind, rows[(size_t)r].skilled, nth);
}
static int pnlHit(const App& a, int px, int py) {
    if (a.panel == 1 && a.amenities) return up::amenitiesHit(px, py);
    if (a.panel == 2 && a.elevation) return up::elevHit(px, py);
    if (a.panel == 2) return up::buildingsHit(px, py);
    if (a.panel == 3 && a.playerPanel) { bool out = false; for (const Golfer& g : a.golfers) out = out || (g.active && g.isPlayer); return up::playerHit(out ? up::ModePlayerSelected : up::ModePlayer, px, py); }
    if (a.panel == 3 && a.golfersMode) return up::golfersHit(px, py);
    if (a.panel == 3) return up::employeeHit(px, py);
    return -1;
}
static void pnlTip(App& a, const std::string& text, float cx) {
    const float w = a.font.width(text, 13) + 14;
    const float x = std::min(std::max(cx - w * 0.5f, 3.0f), 797.0f - w);
    ui::fillRect(x, 462, w, 18, 0.12f, 0.1f, 0.3f, 0.92f);
    a.font.draw(x + 7, 475, text, 13, 1, 0.95f, 0.7f);
}
static std::string wageText(int kind, bool skilled) {
    char b[96]; std::snprintf(b, sizeof b, "%s: %s per week", skilled ? up::kStaffKinds[kind].skilledName : up::kStaffKinds[kind].name, money(up::kStaffKinds[kind].wageUnits[skilled] * 100LL).c_str());
    return b;
}

// ---- Variant strip above the Amenities panel (docs/UI_PANELS.md): the pointer picks a design, then a map click places it ----
// The strip background in the original is cut from AmenitiesPanel_A; the port draws a plain dark band in its place (presentation PLACEHOLDER).
static int stripIndexFor(int tool) { for (int i = 0; i < 5; i++) if (up::kVariantStrips[i].tool == tool) return i; return -1; }
static int stripVarSlot(int strip) { static const int m[5] = {0, 4, 3, 1, 2}; return m[strip]; }   // kVariantStrips order -> App::amenVar order
static void drawSpriteIcon(App& app, GlSprite* g, float cx, float cy, float box, int view = 0, int frame = 0) {
    if (!g) return;
    const Rgba& img = g->s.frame(view, frame);
    if (!img.w || !img.h) return;
    const float sc = std::min(box / (float)img.w, box / (float)img.h);
    const float w = img.w * sc, h = img.h * sc;
    glEnable(GL_TEXTURE_2D); glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glBindTexture(GL_TEXTURE_2D, spriteTexture(*g, view, frame));
    glColor4f(1, 1, 1, 1);
    glBegin(GL_QUADS);
    glTexCoord2f(0, 0); glVertex2f(cx - w * 0.5f, cy - h * 0.5f); glTexCoord2f(1, 0); glVertex2f(cx + w * 0.5f, cy - h * 0.5f);
    glTexCoord2f(1, 1); glVertex2f(cx + w * 0.5f, cy + h * 0.5f); glTexCoord2f(0, 1); glVertex2f(cx - w * 0.5f, cy + h * 0.5f);
    glEnd();
    glDisable(GL_TEXTURE_2D);
    (void)app;
}

// Animated staff portraits (the "SQ" clips): basic Greeter, Ranger, Groundskeeper, Tray Girl; skilled Celebrity, Marshall, Technician, Refresher.
static void drawStaffPortrait(App& app, int kind, bool skilled, float cx, float cy, float box) {
    static const char* sq[8] = {"Employee/GreeterSQ", "Employee/RangerSQ", "Employee/GKSQ", "Employee/TrayGirl_SQ", "Employee/GolfCeleb_SQ", "Employee/Marshall_SQ", "Employee/LawnTech_SQ", "Employee/SodaVendorSQ"};
    GlSprite* g = spriteFor(app, std::string(sq[(skilled ? 4 : 0) + kind]) + ".flc", false);
    if (!g) return;
    const int fr = g->s.frameMs ? (int)(app.time * 1000.0 / g->s.frameMs) % std::max(1, g->s.framesPerView) : 0;
    drawSpriteIcon(app, g, cx, cy, box, 0, fr);
}
static App::Amen stripEntryAmen(const App& app, int strip, int i) {
    const int tool = up::kVariantStrips[strip].tool;
    App::Amen m{tool == 1 ? 0 : tool == 2 ? 1 : tool == 16 ? 2 : tool == 19 ? 3 : 5, 0, 0, i};
    (void)app; return m;
}
// The Landmarks strip lists only the designs whose bit is set in the available mask (docs/DECODE_WORLD2.md 1.4); sandbox shows all 14.
static int stripCount(const App& app, int si) { return si == 1 ? __builtin_popcount(app.vstate.availMask & 0x3fffu) : up::kVariantStrips[si].count; }
static int stripKind(const App& app, int si, int i) {
    if (si != 1) return i;
    int n = 0; for (int k = 0; k < 14; k++) if (app.vstate.availMask >> k & 1u) { if (n++ == i) return k; }
    return 0;
}
static void drawAmenStrip(App& app, int px, int py) {
    if (app.tool != 5) return;
    const int si = stripIndexFor(app.amenTool);
    if (si < 0) return;
    const up::VariantStrip& vs = up::kVariantStrips[si];
    const int cnt = stripCount(app, si); if (cnt <= 0) return;
    const int x0 = up::stripX0(vs.anchor, cnt);
    const int w = (cnt / 2 + 2) * 47;
    ui::fillRect((float)x0, 477, (float)w, 79, 0.10f, 0.09f, 0.26f, 0.92f);
    const int sel = app.amenVar[stripVarSlot(si)];
    for (int i = 0; i < cnt; i++) {
        const float cx = (float)up::stripEntryCx(x0, i), cy = (float)up::stripEntryCy(i);
        const bool hov = up::stripEntryHit(x0, i, px, py);
        if (stripKind(app, si, i) == sel) ui::fillRect(cx - 21, cy - 21, 42, 42, 0.95f, 0.85f, 0.35f, 0.55f);
        else if (hov) ui::fillRect(cx - 21, cy - 21, 42, 42, 1, 1, 1, 0.25f);
        const App::Amen m = stripEntryAmen(app, si, stripKind(app, si, i));
        const std::string base = amenArt(m);
        drawSpriteIcon(app, spriteFor(app, base + ".flc", false), cx, cy, vs.picSize > 0 ? std::min(56.0f, (float)vs.picSize) : 50.0f, m.kind == 3 ? i % 8 : 0);
    }
}
static bool amenStripClick(App& app, int px, int py) {
    if (app.tool != 5) return false;
    const int si = stripIndexFor(app.amenTool);
    if (si < 0) return false;
    const up::VariantStrip& vs = up::kVariantStrips[si];
    const int cnt = stripCount(app, si);
    const int x0 = up::stripX0(vs.anchor, cnt);
    for (int i = 0; i < cnt; i++)
        if (up::stripEntryHit(x0, i, px, py)) { app.amenVar[stripVarSlot(si)] = stripKind(app, si, i); snd(app, "Interface/Button2.wav"); return true; }
    return false;
}
static std::string amenPriceText(const App& app, int tool) {
    namespace be = sg::buildings_exe;
    int units = 0;
    switch (tool) {
        case 3: units = be::kBaseCostUnits[be::BallWasher]; break; case 1: units = be::kBaseCostUnits[be::Benches]; break;
        case 2: units = be::kBaseCostUnits[be::FlowerBed]; break; case 16: units = be::kBaseCostUnits[be::WillowTree]; break;
        case 19: units = be::kBaseCostUnits[be::ScenicBridge]; break;
        case 4: if ((app.vstate.landmarkMask >> app.amenVar[4]) & 1u) return "FREE!"; units = be::landmarkCostUnits(app.amenVar[4], false); break;
        default: return "";
    }
    return "Cost: " + money(units * 100LL);
}

static void pnlDraw(App& app, float mx, float my) {
    const int px = (int)mx, py = (int)my;
    const int h = pnlHit(app, px, py);
    if (h != app.pHoverLast) { app.pHoverLast = h; app.pHoverFrames = 0; } else app.pHoverFrames++;
    const bool tip = app.pHoverFrames > up::kTipDelayFrames;
    if (app.panel == 1 && !app.amenities) {
        ui::drawImage(app.terrPanel, 216, 482, 216, 482, 584, 118);
        const int te = themeExe(app.theme);
        for (int i = 0; i < 16; i++) {
            float x, y, w, hh; terrSlotRect(app, i, x, y, w, hh);
            if (i >= 13) { x = (float)up::kTerrainTreePos[te][i - 13].x; y = (float)up::kTerrainTreePos[te][i - 13].y; }
            const bool sel = app.tool == 0 && app.terrSlot == i;
            const int st = sel ? 2 : (app.terrHover == i ? 1 : 0);
            if (i < 13) ui::drawImage(app.terrBtns, x, y, (float)((i > 6 ? 248 : 0) + st * 62), (float)((i % 7) * 54), 62, 54);
            else ui::drawImage(app.terrBtns, x, y, (float)(0x208 + st * 0x50), (float)((i - 13) == 0 ? 0 : (i - 13) == 1 ? 0x96 : 300), w, hh);
        }
        if (inDiscInt(up::kTerrainToAmenitiesHit, px, py)) pnlBlit(app.terrPanel, up::kTerrainToAmenitiesCut, up::kTerrainToAmenitiesDst.x, up::kTerrainToAmenitiesDst.y);
        if (inDiscInt(up::kTerrainUndoHit, px, py)) pnlBlit(app.terrPanel, up::kTerrainUndoCut, up::kTerrainUndoDst.x, up::kTerrainUndoDst.y);
        if (app.terrHover >= 0) {
            float x, y, w, hh; terrSlotRect(app, app.terrHover, x, y, w, hh);
            const float bx = std::min(std::max(x + 32.0f, 80.0f), 720.0f) - 80.0f;
            ui::fillRect(bx, 402, 160, 44, 0.12f, 0.1f, 0.3f, 0.9f);
            app.font.draw(bx + 8, 420, terrSlotName(app, app.terrHover), 14, 1, 0.95f, 0.7f);
            app.font.draw(bx + 8, 438, "Cost per tile: " + money(Economy::terrainCostUnits(kPaint[kTerrSlots[app.terrHover].paint].type) * 100LL), 13, 1, 1, 1);
        } else if (inDiscInt(up::kTerrainToAmenitiesHit, px, py)) pnlTip(app, "Amenities", 237);
        else if (inDiscInt(up::kTerrainUndoHit, px, py)) pnlTip(app, "Undo", 261);
    } else if (app.panel == 1) {   // Amenities
        ui::drawImage(app.amenArt, 214, 482, 214, 482, 586, 118);
        if (h == -2) pnlBlit(app.amenArt, up::kAmenBackCut, up::kAmenBackDst.x, up::kAmenBackDst.y);
        for (int i = 0; i < up::kAmenitySlotCount; i++) {
            if (i == 8) continue;
            const up::AmenitySlot& sl = up::kAmenitySlots[i];
            const bool selected = (app.tool == 2 && i == 1) || (app.tool == 6 && i == 7) || (app.tool == 5 && up::amenitySlotForTool(app.amenTool) == i);
            if (i == 2 && homeFreeSites(app) < 1) { pnlBlit(app.amenArt, up::amenityDisabledCut(i), sl.dst.x, sl.dst.y); continue; }   // no free home site: a Silver member is needed for each lot
            if (selected) pnlBlit(app.amenArt, up::amenitySelectedCut(i), sl.dst.x, sl.dst.y);
            else if (h == i) pnlBlit(app.amenArt, sl.hoverCut, sl.dst.x, sl.dst.y);
        }
        drawAmenStrip(app, px, py);
        if (tip) {
            if (h == -2) pnlTip(app, up::kAmenBackTip, mx);
            else if (h >= 0 && up::kAmenitySlots[h].tip) {
                std::string t = up::kAmenitySlots[h].tip;
                if (h == 1) t += app.pathKind == 2 ? ": paved, $100 per tile" : ": gravel, $100 per tile";
                else if (h == 7) t += ": click a tile, or right-click on the map";
                else if (h != 2) t += ", " + amenPriceText(app, up::kAmenitySlots[h].tool);
                else t += ", " + std::to_string(std::max(0, homeFreeSites(app))) + " free";
                pnlTip(app, t, mx);
            }
        }
    } else if (app.panel == 2 && app.elevation) {
        ui::drawImage(app.elevArt, 215, 482, 215, 482, 585, 118);
        if (h == -2) pnlBlit(app.elevArt, up::kElevBackCut, up::kElevBackDst.x, up::kElevBackDst.y);
        if (h == -3) pnlBlit(app.elevArt, up::kElevUndoHover, up::kElevUndoDst.x, up::kElevUndoDst.y);
        if (h >= 0 && h != app.elevTool) pnlBlit(app.elevArt, up::kElevTools[h].hover, up::kElevTools[h].dst.x, up::kElevTools[h].dst.y);
        pnlBlit(app.elevArt, up::kElevTools[app.elevTool].selected, up::kElevTools[app.elevTool].dst.x, up::kElevTools[app.elevTool].dst.y);
        if (tip) { if (h == -2) pnlTip(app, up::kElevBackTip, mx); else if (h == -3) pnlTip(app, up::kElevUndoTip, mx); else if (h >= 0) pnlTip(app, up::kElevTools[h].tip, mx); }
    } else if (app.panel == 2) {   // Add Buildings
        ui::drawImage(app.bldgArt, 216, 482, 216, 482, 584, 118);
        if (h == -2) pnlBlit(app.bldgArt, up::kBuildToElevationCut, up::kBuildToElevationDst.x, up::kBuildToElevationDst.y);
        if (h == -3) pnlBlit(app.bldgArt, up::kBuildUndoHover, up::kBuildUndoDst.x, up::kBuildUndoDst.y);
        for (int i = 0; i < up::kLotCount; i++) {
            const bool can = lotCanBuild(app, i);
            const int d = lotDef(i);
            const bool armed = app.tool == 4 && d >= 0 && d == app.buildIdx;
            if (can && h == i) pnlBlit(app.bldgArt, up::padHoverCut(i), up::kLots[i].pad.x, up::kLots[i].pad.y);
            if (armed) pnlBlit(app.bldgArt, up::padSelectedCut(i), up::kLots[i].pad.x, up::kLots[i].pad.y);
        }
        if (app.layoutArt.tex)
            for (int i = 0; i < up::kLotCount; i++) {
                const bool can = lotCanBuild(app, i);
                const up::Rect c = up::lotIconCut(i, app.bsys.level(i + 6) > 0, can);
                const up::XY d = up::lotIconDst(i, h == i && can);
                ui::drawImage(app.layoutArt, (float)d.x, (float)d.y, (float)c.x, (float)c.y, (float)c.w, (float)c.h);
            }
        if (tip) {
            if (h == -2) pnlTip(app, up::kBuildToElevationTip, mx);
            else if (h == -3) pnlTip(app, "Undo", mx);
            else if (h >= 0) {
                const float bx = (float)(up::lotInfoCentreX(h) - 80);
                ui::fillRect(bx, 460, 160, 44, 0.12f, 0.1f, 0.3f, 0.92f);
                app.font.drawCentered(bx + 80, 474, (app.bsys.level(h + 6) > 0 ? std::string("Upgraded ") : std::string()) + up::kLots[h].name, 13, 1, 0.95f, 0.7f);
                app.font.drawCentered(bx + 80, 488, "Cost: $" + std::to_string(up::lotInfoDollars(h, app.bsys.level(h + 6))), 12, 1, 1, 1);
                app.font.drawCentered(bx + 80, 500, up::kLots[h].tip, 11, 0.9f, 0.9f, 1);
            }
        }
    } else if (app.panel == 3 && app.playerPanel && app.joeArt.tex) {   // Player panel (docs/DECODE_GOLFERCARD.md section 4); scorecard and skill list placement are PLACEHOLDER
        const Golfer* pg = nullptr; for (const Golfer& g : app.golfers) if (g.active && g.isPlayer) pg = &g;
        ui::drawImage(app.joeArt, 214, 474, 214, 474, 586, 126);
        if (pg) {   // scorecard above the panel: hole numbers, par, strokes, and the score against par
            ui::drawImage(app.joeArt, 297, 366, 297, 300, 503, 108);
            int tot = 0, tpar = 0;
            for (int k = 0; k < 18; k++) {
                const float cxk = 297 + (439 + 18 * k - 297) + 6.5f, y0 = 366 - 300;
                if (k < (int)app.holes.size()) {
                    app.font.drawCentered(cxk, 335 + y0, std::to_string(k + 1), 10, 0.1f, 0.1f, 0.3f);
                    app.font.drawCentered(cxk, 350 + y0, std::to_string(app.holes[(size_t)k].par), 10, 0.1f, 0.1f, 0.3f);
                    if (pg->holeStrokes[k] > 0) { app.font.drawCentered(cxk, 369 + y0, std::to_string(pg->holeStrokes[k]), 10, 0.1f, 0.1f, 0.3f); tot += pg->holeStrokes[k]; tpar += app.holes[(size_t)k].par; }
                }
            }
            app.font.drawCentered(778, 350 + 66, std::to_string(tot), 11, 0.1f, 0.1f, 0.3f);
            app.font.draw(312, 418, app.charName, 11, 0.1f, 0.1f, 0.3f);
            const int d = tot - tpar; app.font.draw(312, 439, tpar ? (d == 0 ? std::string("Even par") : d > 0 ? "+" + std::to_string(d) : std::to_string(d)) : std::string("Teeing off"), 11, 0.1f, 0.1f, 0.3f);
            for (int b = 4; b < 9; b++) {   // shot shape ovals
                const up::PlayerButton& pb = up::kPlayerButtons[b];
                const int col = app.shotShape == b - 4 ? 100 : 0;
                ui::drawImage(app.joeArt, (float)pb.hoverDst.x, (float)pb.hoverDst.y, (float)col, (float)(1 + 50 * (b - 4)), 80, 50);
                if (h == b && app.shotShape != b - 4) ui::drawImage(app.joeArt, (float)pb.hoverDst.x, (float)pb.hoverDst.y, 300.0f, (float)(1 + 50 * (b - 4)), 80, 50);
            }
        }
        { // action buttons: Practice Round needs two open holes and nobody of ours out; Play and Begin Tournament are not available yet and stay dim
            const int openH = (int)app.holes.size();
            for (int b = 1; b <= 3; b++) {
                const up::PlayerButton& pb = up::kPlayerButtons[b];
                const bool ok = openH >= 2 && !pg && !app.matchOn && (b == 1 || (b == 2 && app.matchPro >= 0) || (b == 3 && app.tourney.state() == sg::Tournament::State::Offered));
                if (!ok) pnlBlit(app.joeArt, up::Rect{100, pb.hoverCut.y, pb.hoverCut.w, pb.hoverCut.h}, pb.hoverDst.x, pb.hoverDst.y);
                else if (h == b) pnlBlit(app.joeArt, up::Rect{50, pb.hoverCut.y, pb.hoverCut.w, pb.hoverCut.h}, pb.hoverDst.x, pb.hoverDst.y);
            }
        }
        if (h == 9) pnlBlit(app.joeArt, up::kPlayerButtons[9].hoverCut, up::kPlayerButtons[9].hoverDst.x, up::kPlayerButtons[9].hoverDst.y);
        if (h == 10) pnlBlit(app.joeArt, up::kPlayerButtons[10].hoverCut, up::kPlayerButtons[10].hoverDst.x, up::kPlayerButtons[10].hoverDst.y);
        { static const char* kSk[10] = {"Power Hitter", "Long Driver", "Accurate Driver", "Accurate Irons", "Accurate Putter", "Draw Shot", "Fade Shot", "High Backspin", "Recovery", "Luck"};
          for (int k = 0; k < 10; k++) { const int col = k < 4 ? 0 : k < 7 ? 1 : 2, row = k < 4 ? k : k < 7 ? k - 4 : k - 7; const float bx = col == 0 ? 346.0f : col == 1 ? 493.0f : 645.0f;
            app.font.draw(bx, 550.0f + row * 13, kSk[k], 11, 1, 1, 1); app.font.draw(bx + 108, 550.0f + row * 13, std::to_string(app.chr.skills[k]), 11, 1, 0.9f, 0.4f); } }
        if (tip && h > 0) { const char* t = up::kPlayerButtons[h].tip; if (h == 9) t = "Golfers"; if (t) pnlTip(app, t, mx); }
    } else if (app.panel == 3 && app.golfersMode) {   // Golfers (docs/DECODE_BUILDINGS.md section 6, docs/UI_PANELS.md section 7)
        ui::drawImage(app.memberArt, 215, 474, 215, 474, 585, 126);
        std::vector<int> list; for (size_t i = 0; i < app.golfers.size(); i++) if (app.golfers[i].active) list.push_back((int)i);
        const int total = (int)list.size();
        if (app.golfOff > 0 && app.golfOff >= total) app.golfOff = std::max(0, app.golfOff - 16);
        if (h == -3) pnlBlit(app.memberArt, up::kGolfersPlayerTabCut, up::kGolfersPlayerTabDst.x, up::kGolfersPlayerTabDst.y);
        if (h == -4) pnlBlit(app.memberArt, up::kGolfersEmployeesCut, up::kGolfersEmployeesDst.x, up::kGolfersEmployeesDst.y);
        if (h == -5 && app.golfOff > 0) pnlBlit(app.memberArt, up::kGolfersScrollLeftCut, up::kGolfersScrollLeftDst.x, up::kGolfersScrollLeftDst.y);
        if (h == -6 && total - app.golfOff > 16) pnlBlit(app.memberArt, up::kGolfersScrollRightCut, up::kGolfersScrollRightDst.x, up::kGolfersScrollRightDst.y);
        for (int c = 0; c < 16; c++) {
            const int idx = app.golfOff + c;
            if (idx >= total) break;
            const Golfer& g = app.golfers[(size_t)list[(size_t)idx]];
            const int col = c / 4, row = c % 4;
            const int cx = 310 + col * 121, cy = 498 + row * 21;
            if (!(row & 1)) ui::drawImage(app.memberArt, (float)cx, (float)cy, 0, 0, 121, 44);   // one plate under each pair of rows (PLACEHOLDER: drawn from the even row so a lone golfer has one too)
            if (list[(size_t)idx] == app.followG) ui::fillRect((float)cx, (float)cy, 121, 20, 1, 1, 0.6f, 0.25f);
            const std::string nm = g.isPlayer ? app.charName : g.memberId ? memberName(app, g.memberId) : std::string("Golfer");
            app.font.draw((float)(cx + 16), (float)(cy + 14), nm.substr(0, 11), 11, 0.1f, 0.1f, 0.25f);
            const int face = std::clamp(g.mood + 2, 1, 10) - 1;
            pnlBlit(app.faceArt.tex ? app.faceArt : app.memberArt, up::Rect{594 - 16 * face, 100, 16, 16}, cx + 102, cy + 2);   // the faces run right to left on the sheet (docs/DECODE_FACES.md)
            const int hole = g.hole + 1;
            if (!(row & 1)) app.font.draw((float)(cx + (hole >= 10 ? 5 : 8)), (float)(cy + 27), std::to_string(hole), 10, 0.1f, 0.1f, 0.25f);   // the round tab on the plate shows the pair's hole
        }
        if (total > 16) { const float bx = 351.0f + app.golfOff * 400.0f / total; ui::fillRect(bx, 590, 400.0f * 16 / total, 6, 0.45f, 0.45f, 0.75f, 0.9f); }
        if (tip) { if (h == -2) pnlTip(app, up::kGolfersTabTip[0], mx); else if (h == -3) pnlTip(app, app.charName, mx); else if (h == -4) pnlTip(app, up::kGolfersTabTip[2], mx); }
        if (total == 0) app.font.drawCentered(500, 540, "No golfers on the course right now", 13, 0.15f, 0.15f, 0.3f);
    } else if (app.panel == 3) {   // Employees
        ui::drawImage(app.empArt, 215, 474, 215, 474, 585, 126);
        const std::vector<EmpRow> rows = empRows(app);
        const int n = (int)rows.size();
        if (app.empOff > 0 && app.empOff > n - 7) app.empOff = std::max(0, app.empOff - 2);
        if (app.empOff > 0) pnlBlit(app.empArt, up::kEmpLeftEnabled, up::kEmpLeftDst.x, up::kEmpLeftDst.y);
        if (n - app.empOff > 8) pnlBlit(app.empArt, up::kEmpRightEnabled, up::kEmpRightDst.x, up::kEmpRightDst.y);
        if (h == 0) pnlBlit(app.empArt, up::kEmpHireHover, up::kEmpHireDst.x, up::kEmpHireDst.y);
        if (n >= up::kMaxEmployees) pnlBlit(app.empArt, up::kEmpHireDisabled, up::kEmpHireDst.x, up::kEmpHireDst.y);
        if (h == 2 && app.empOff > 0) pnlBlit(app.empArt, up::kEmpLeftHover, up::kEmpLeftDst.x, up::kEmpLeftDst.y);
        if (h == 3 && n - app.empOff > 8) pnlBlit(app.empArt, up::kEmpRightHover, up::kEmpRightDst.x, up::kEmpRightDst.y);
        for (int p = 0; p < up::kEmployeePortraits; p++) {
            const int idx = app.empOff + p;
            if (idx >= n) break;
            const bool sel = idx == app.empSel;
            if (sel) pnlBlit(app.empArt, up::empPortraitSelectedCut(p), up::kEmpPortraitDst[p].x, up::kEmpPortraitDst[p].y);
            else if (h == 8 + p) pnlBlit(app.empArt, up::empPortraitHoverCut(p), up::kEmpPortraitDst[p].x, up::kEmpPortraitDst[p].y);
            // The exe draws the employee's animated staff sprite here; the port writes the job instead (PLACEHOLDER).
            drawStaffPortrait(app, rows[idx].kind, rows[idx].skilled, (float)up::kEmpPortraitDst[p].x + 32, (float)up::kEmpPortraitDst[p].y + 16, 34);
        }
        for (int a = 0; a < 3; a++) {
            const bool has = app.empSel >= 0 && app.empSel < n;
            pnlBlit(app.empArt, up::empActionCut(a, has ? 3 : 2), up::kEmpActions[a].dst.x, up::kEmpActions[a].dst.y);
            if (has && h == 4 + a) pnlBlit(app.empArt, up::empActionCut(a, 0), up::kEmpActions[a].dst.x, up::kEmpActions[a].dst.y);
            if (a == 0 && has && app.empMoveArm == app.empSel) pnlBlit(app.empArt, up::empActionCut(0, 1), up::kEmpActions[0].dst.x, up::kEmpActions[0].dst.y);   // armed cut
        }
        if (app.empSel >= 0 && app.empSel < n) {
            const EmpRow& r = rows[app.empSel];
            const int ei = rowEmp(app, rows, app.empSel); const App::Emp* em = ei >= 0 ? &app.emps[(size_t)ei] : nullptr;
            const std::string job = r.skilled ? up::kStaffKinds[r.kind].skilledName : up::kStaffKinds[r.kind].name;
            app.font.drawCentered((float)up::kEmpInfoX, (float)up::kEmpInfoY[0], em && !em->name.empty() ? em->name : job, 12, 0.1f, 0.08f, 0.3f);
            if (em) { static const char* kMo[12] = {"Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"}; const int mi = em->hired - 1;
                      app.font.drawCentered((float)up::kEmpInfoX, (float)up::kEmpInfoY[1], std::string("Hired: ") + kMo[(2 + mi % 8) % 12] + " " + std::to_string(2001 + mi / 8), 12, 0.1f, 0.08f, 0.3f); }
            // "Paid" is the wage times the months employed (PLACEHOLDER: the exe books each payment on the employee record; the port pays the whole staff together).
            app.font.drawCentered((float)up::kEmpInfoX, (float)up::kEmpInfoY[2], "Paid: " + money(up::kStaffKinds[r.kind].wageUnits[r.skilled] * 100LL * (em ? std::max(0, app.econ.day - em->hired) : 0)), 12, 0.1f, 0.08f, 0.3f);
            app.font.drawCentered((float)up::kEmpInfoX, (float)up::kEmpInfoY[3], up::kStaffKinds[r.kind].counter[r.skilled], 11, 0.25f, 0.2f, 0.45f);
            app.font.drawCentered((float)up::kEmpInfoX, (float)up::kEmpInfoY[4], std::to_string(em ? em->n : app.empCount[r.kind][r.skilled ? 1 : 0]), 12, 0.1f, 0.08f, 0.3f);   // this employee's own count (the exe counts per employee); a loaded game restarts the individual counts
        }
        if (tip) {
            if (h == 0) pnlTip(app, up::kEmpHireTip, mx);
            else if (h >= 4 && h <= 6) pnlTip(app, up::kEmpActions[h - 4].tip, mx);
        }
    }
}

// The hire dialog is modal and drawn over everything (sheet infoscreens/hire).
static void drawHireDialog(App& app, float mx, float my) {
    if (!app.hireOpen || !app.hireArt.tex) return;
    ui::fillRect(0, 0, 800, 600, 0, 0, 0, 0.45f);
    ui::drawImage(app.hireArt, 0, 0);
    app.font.drawCentered((float)up::kHireTitle.x, (float)up::kHireTitle.y + 14, "HIRE AN EMPLOYEE", 15, 0.2f, 0.15f, 0.35f);
    const int ch = (mx >= 231 && mx < 522) ? up::hireChoiceAt((int)my) : -1;
    for (int k = 0; k < 4; k++) drawStaffPortrait(app, k, ch == 2 * k + 1, 596.0f, (float)up::kHirePortraitY[k] + 20, 40);
    for (int k = 0; k < 4; k++) {
        const int y = up::kHireBandY[k];
        for (int sk = 0; sk < 2; sk++) {
            const int c = 2 * k + sk;
            if (ch == c) ui::fillRect(231, (float)(y + sk * 20), 291, 16, 1, 0.85f, 0.4f, 0.55f);
            app.font.draw((float)up::kHireTextX, (float)(y + 13 + sk * 20), wageText(k, sk != 0), 13, 0.15f, 0.12f, 0.3f);
        }
    }
}

static bool pnlClick(App& app, float vx, float vy, bool rightClick) {
    if (!pnlArt(app) || !app.panel) return false;
    const int px = (int)vx, py = (int)vy;
    if (app.panel == 1 && !app.amenities) {
        if (inDiscInt(up::kTerrainToAmenitiesHit, px, py)) { app.amenities = true; snd(app, "Interface/Button2.wav"); return true; }
        if (inDiscInt(up::kTerrainUndoHit, px, py)) { app.tool = 6; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        const int sl = terrSlotAt(app, vx, vy);
        // Tree slots live at per-theme positions; hit them by their own rectangles.
        int hit = sl;
        { const int te = themeExe(app.theme);
          for (int i = 15; i >= 13; i--) { float x, y, w, hh; terrSlotRect(app, i, x, y, w, hh); x = (float)up::kTerrainTreePos[te][i - 13].x; y = (float)up::kTerrainTreePos[te][i - 13].y; if (vx >= x && vx < x + w && vy >= y && vy < y + hh) { hit = i; break; } } }
        if (hit >= 0) { app.terrSlot = hit; selectPanelItem(app, PanelItem{"", 0, kTerrSlots[hit].paint}, false); snd(app, "Interface/Button2.wav"); return true; }
        return vx >= 216 && vy >= 482;
    }
    const int h = pnlHit(app, px, py);
    if (app.panel == 1) {   // Amenities
        if (h == -2) { app.amenities = false; app.tool = 0; app.paintIdx = kTerrSlots[std::max(0, app.terrSlot)].paint; snd(app, "Interface/Button2.wav"); return true; }
        if (h == 1) { if (app.tool == 2) app.pathKind = 3 - app.pathKind; app.tool = 2; if (app.pathKind < 1) app.pathKind = 1; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        if (amenStripClick(app, px, py)) return true;
        if (h == 7) { app.tool = 6; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h == 2 && homeFreeSites(app) < 1) { say(app, "No home sites are free: each Silver (or better) member buys one lot.", 4); snd(app, "Interface/Button1.wav"); return true; }
        if (h >= 0) { app.amenTool = up::kAmenitySlots[h].tool; app.tool = 5; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        return vx >= 214 && vy >= 482;
    }
    if (app.panel == 2 && app.elevation) {
        if (h == -2) { app.elevation = false; app.tool = 4; snd(app, "Interface/Button2.wav"); return true; }
        if (h == -3) { app.tool = 6; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h >= 0 && h <= 2) { app.elevTool = h; app.tool = 1; app.raiseSign = 1; app.brush = h == 0 ? 0 : (h == 1 ? 1 : 3); app.edit = true; snd(app, "Interface/Button2.wav"); return true; }   // brush sizes are PLACEHOLDERS; left click raises, shift lowers
        if (h == 3) { app.anArm = !app.anArm; if (!app.anArm) app.anHole = -1; else say(app, "Click a hole to see how golfers will play it.", 4); snd(app, "Interface/Button2.wav"); return true; }
        return vx >= 215 && vy >= 482;
    }
    if (app.panel == 2) {
        if (h == -2) { app.elevation = true; app.tool = 1; app.elevTool = 0; app.brush = 0; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h == -3) { app.tool = 6; app.edit = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h >= 0) {
            const int id = h + 6;
            if (!lotCanBuild(app, h)) { say(app, "This building will become available as you open more holes", 4); snd(app, "Interface/Button1.wav"); return true; }
            const int d = lotDef(h);
            if (d < 0) { say(app, std::string(up::kLots[h].name) + " is not in the port yet", 3); return true; }
            (void)id; app.tool = 4; app.buildIdx = d; app.edit = true; snd(app, "Interface/Button2.wav"); return true;
        }
        return vx >= 216 && vy >= 482;
    }
    if (app.panel == 3 && app.playerPanel) {
        bool out = false; for (const Golfer& g : app.golfers) out = out || (g.active && g.isPlayer);
        if (h == 9) { app.playerPanel = false; app.golfersMode = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h == 10) { app.playerPanel = false; app.golfersMode = false; snd(app, "Interface/Button2.wav"); return true; }
        if (h == 0) { openCustomise(app); snd(app, "Interface/Button2.wav"); return true; }
        if (h == 1) {
            if (app.matchOn) say(app, "Finish or cancel the match first.", 4);
            else if ((int)app.holes.size() < 2) { say(app, "You need at least two open holes for a practice round.", 4); }
            else if (!out) { spawnGolfer(app, true); say(app, app.charName + " is out on a practice round.", 4); snd(app, "Interface/Button2.wav"); }
            return true;
        }
        if (h == 2) {
            if (app.matchOn || out) say(app, "A match or practice round is already under way.", 4);
            else if (app.matchPro < 0) say(app, "No pro has challenged you to a match yet.", 4);
            else if ((int)app.holes.size() < 2) say(app, "You need at least two open holes for a match.", 4);
            else {
                spawnGolfer(app, true); spawnGolfer(app, false, app.matchPro);
                for (size_t i = 0; i < app.golfers.size(); i++) { if (app.golfers[i].active && app.golfers[i].isPlayer) app.matchPl = (int)i; if (app.golfers[i].active && app.golfers[i].proIdx == app.matchPro) app.matchOp = (int)i; }
                if (app.matchPl >= 0 && app.matchOp >= 0) { app.matchOn = true; app.matchLead = 0; app.matchDoneN = 0; for (bool& b : app.matchDone) b = false; say(app, "I'm ready for a match with " + app.pros[(size_t)app.matchPro].name + ".", 5); snd(app, "Interface/Button2.wav"); }
                else say(app, "There is no room on the course for a match right now.", 4);
            }
            return true;
        }
        if (h == 3) {
            if (app.tourney.state() == sg::Tournament::State::Offered) { snd(app, "Interface/Button2.wav"); openSgaScreen(app); }
            else say(app, "The SGA has not offered to hold a tournament at your course.", 4);
            return true;
        }
        if (h >= 4 && h <= 8 && out) { app.shotShape = h - 4; snd(app, "Interface/Button2.wav"); return true; }
        return vx >= 214 && vy >= 474;
    }
    if (app.panel == 3 && app.golfersMode) {
        std::vector<int> list; for (size_t i = 0; i < app.golfers.size(); i++) if (app.golfers[i].active) list.push_back((int)i);
        const int total = (int)list.size();
        if (h == -4) { app.golfersMode = false; snd(app, "Interface/Button2.wav"); return true; }
        if (h == -3) { app.playerPanel = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h == -5) { if (app.golfOff > 0) app.golfOff = std::max(0, app.golfOff - 16); return true; }
        if (h == -6) { if (total - app.golfOff > 16) app.golfOff += 16; return true; }
        if (h >= 0 && app.golfOff + h < total) { const int pick = list[(size_t)(app.golfOff + h)]; const bool again = pick == app.followG; app.followG = pick; snd(app, "Interface/Button2.wav"); if (again) openGolferCard(app, pick); return true; }   // a second click on the followed golfer opens the info card
        return vx >= 215 && vy >= 474;
    }
    if (app.panel == 3) {
        const std::vector<EmpRow> rows = empRows(app);
        const int n = (int)rows.size();
        if (h == 0) { if (n >= up::kMaxEmployees) snd(app, "Interface/Button1.wav"); else app.hireOpen = true; return true; }
        if (h == 2 && app.empOff > 0) { app.empOff -= 2; return true; }
        if (h == 3) { if (n - app.empOff > 8) app.empOff += 2; return true; }
        if (h == -2) { app.golfersMode = true; snd(app, "Interface/Button2.wav"); return true; }
        if (h == 5 && app.empSel >= 0 && app.empSel < n) {
            const int ei = rowEmp(app, rows, app.empSel); if (ei >= 0) app.emps.erase(app.emps.begin() + ei);   // this very employee leaves, not just the last of the kind
            app.econ.fire(rows[app.empSel].kind, rows[app.empSel].skilled); app.empSel = -1; app.empMoveArm = -1; refreshEmpProps(app); snd(app, "Interface/Button2.wav"); return true; }
        if (h == 4 && app.empSel >= 0 && app.empSel < n) { app.empMoveArm = app.empMoveArm == app.empSel ? -1 : app.empSel; if (app.empMoveArm >= 0) say(app, "Click on the course where this employee should wait.", 4); return true; }
        if (h == 6 && app.empSel >= 0 && app.empSel < n) {
            const int ei = rowEmp(app, rows, app.empSel);
            if (ei >= 0) { app.empRenameRow = ei; app.popKind = 4; app.popPromptFor = 3; app.popHead = "Rename Employee..."; app.popBuf = app.emps[(size_t)ei].name; SDL_StartTextInput(); }
            return true; }
        if (h >= 8 && app.empOff + (h - 8) < n) { app.empSel = app.empOff + (h - 8); snd(app, "Interface/Button2.wav"); return true; }
        return vx >= 215 && vy >= 474;
    }
    return false;
}
static bool hireClick(App& app, float vx, float vy) {
    if (!app.hireOpen) return false;
    const int c = (vx >= 231 && vx < 522) ? up::hireChoiceAt((int)vy) : -1;
    app.hireOpen = false;
    if (c < 0) return true;
    const int kind = c / 2; const bool sk = (c & 1) != 0;
    if (sk && !sg::costs::skilledHireAllowed(sg::costs::courseGrade((int)app.allHoles.size()) < 0 ? 3 : sg::costs::courseGrade((int)app.allHoles.size()))) { say(app, up::kSkilledRefusal, 5); return true; }
    app.econ.hire(kind, sk); snd(app, "Interface/Button2.wav");
    return true;
}

static int dockHit(float vx, float vy) {
    for (int i = 0; i < 10; i++) { const float dx = vx - kDock[i].cx, dy = vy - kDock[i].cy; if (dx * dx + dy * dy <= kDock[i].r * kDock[i].r) return i; }
    return -1;
}

// ---- Popup menus (docs/DECODE_MENUS.md). The frame is the 9 piece panel cut from InfoButtons.pcx at (200,0), 16 px pieces on a 17 px pitch, over a lavender fill (exe colour 0x4e79).
// PLACEHOLDER: the radio ball beside each option (the exe's sprite is not located) and the System menu captions the exe keeps in its data.
static void ibLoad(App& app) { if (!app.ibArt.tex) { const std::string i = app.gameDir + "/Interface/"; ui::loadPcx(i + "InfoButtons.pcx", app.ibArt, false, -1, i + "InfoButtons_A.pcx"); } }
static void drawFrame9(App& app, float x, float y, float w, float h) {
    ibLoad(app);
    if (int r = (int)w & 15) { w += 15 - r; x -= (15 - r) / 2; }
    if (int r = (int)h & 15) { h += 15 - r; y -= (15 - r) / 2; }
    ui::fillRect(x + 4, y + 4, w - 8, h - 8, 152 / 255.0f, 152 / 255.0f, 200 / 255.0f, 1);
    auto piece = [&](int col, int row, float dx, float dy) { ui::drawImage(app.ibArt, dx, dy, 200.0f + 17 * col, 17.0f * row, 16, 16); };
    piece(0, 0, x, y);
    for (float t = x + 16; t < x + w - 16; t += 16) { piece(1, 0, t, y); piece(1, 2, t, y + h - 16); }
    for (float t = y + 16; t < y + h - 16; t += 16) { piece(0, 1, x, t); piece(2, 1, x + w - 16, t); }
    piece(2, 0, x + w - 16, y); piece(2, 2, x + w - 16, y + h - 16); piece(0, 2, x, y + h - 16);
}
// The game font has no plus sign, so "+N%" is drawn as a small cross and then the figure. centred true puts the whole thing around x.
static void drawPlusPct(App& app, float x, float y, int pct, float size, float r, float g, float b, bool centred) {
    const std::string t = std::to_string(pct) + "%"; const float tw = app.font.width(t, size), total = tw + 9.0f; const float x0 = centred ? x - total / 2 : x;
    ui::fillRect(x0, y - size * 0.45f, 7, 2, r, g, b, 1); ui::fillRect(x0 + 2.5f, y - size * 0.45f - 2.5f, 2, 7, r, g, b, 1);
    app.font.draw(x0 + 9.0f, y, t, size, r, g, b);
}
static const char* kInfoMenu[12] = {"Repeat Last Message", "Course Report", "Player Comments", "Routing Map", "Histogram", "SGA Evaluation", "Financial Report", "Membership Roster", "Professional Accomplishments", "World Map", "Best Scores", "Top 10 Designers"};
static void popOpen(App& app, int kind) {
    app.popKind = kind; app.popHover = -1; app.popLines.clear(); app.popHead.clear();
    if (kind == 1) for (const char* c : kInfoMenu) app.popLines.push_back(c);
    else if (kind == 2) {
        app.popLines = {"Save Game", "Load Game", "Cancel Match or Tournament", "Save " + app.charName + " for Championship", "Rename Your Course", "Preferences", "Save Course for Championship", "Quit"};
    } else if (kind == 5) {
        app.popLines = {"Yes, save over it.", "No, never mind."};
    } else if (kind == 3) {
        app.popLines = {"Display golfer names on screen", "Show advisor and first-time messages", "Show ambient animals"};   // bits 1, 4 and 0x20; the exe's other three labels are not in the text
    }
}
static void popGeom(const App& app, float& x, float& y, float& w, float& h, float& firstY) {
    float mw = 0; for (const std::string& l : app.popLines) mw = std::max(mw, app.font.width(l, 16));
    const float cx = app.popKind == 1 ? 200.0f : app.popKind == 2 ? 250.0f : 400.0f, top = app.popKind == 1 ? 250.0f : app.popKind == 2 ? 340.0f : app.popKind == 5 ? 140.0f : 200.0f;
    const int n = (int)app.popLines.size() + (app.popKind == 3 || app.popKind == 5 ? 1 : 0);
    w = (float)(((int)mw - 1) | 15) + 0x31; h = (float)((n * 3 + 3) * 8); x = cx - w / 2; y = top; firstY = y + 12.0f + (app.popKind == 3 || app.popKind == 5 ? 18.0f : 0.0f) + (app.popKind == 1 || app.popKind == 2 ? 12.0f : 0.0f);   // the exe's (n * 3 + 3) * 8 height holds a heading row on the menus
}
static int popArg = 0;
static void popChoose(App& app, int kind, int idx);
static void drawPopup(App& app) {
    static bool once = false; if (!once && popArg) { once = true; if (popArg == 6) popChoose(app, 2, 0); else popOpen(app, popArg); }
    if (!app.popKind) return;
    float x, y, w, h, fy; popGeom(app, x, y, w, h, fy);
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    if (app.popKind == 4) {
        const bool sv = app.popPromptFor == 2;   // the Save dialog's box is (54,80) 628 by 80 with the prompt at (384,88) and the field at (64,102) (FUN_00405b10)
        const float bw = sv ? 628.0f : 400.0f, bh = 80, bx = sv ? 54.0f : 200.0f, by = sv ? 80.0f : 240.0f; drawFrame9(app, bx, by, bw, bh);
        app.font.drawCentered(sv ? 384.0f : 400.0f, by + (sv ? 16.0f : 28.0f), app.popHead, 16, 1, 1, 1);
        ui::fillRect(sv ? 64.0f : bx + 20, sv ? 102.0f : by + 40, sv ? 604.0f : bw - 40, 24, 0.97f, 0.95f, 0.85f, 1);
        app.font.draw((sv ? 64.0f : bx + 20) + 6, (sv ? 102.0f : by + 40) + 18, app.popBuf + ((SDL_GetTicks() / 400) % 2 ? "_" : ""), 15, 0, 0, 0);
        ui::endScreen(); return;
    }
    const bool menu = app.popKind == 1 || app.popKind == 2;
    if (menu) {   // the real screenshots: slate panel, light heading, teal items, amber ball bullets that turn bright with white text under the cursor (colours measured by eye, PLACEHOLDER)
        ui::fillRect(x + 3, y + 3, w, h, 0, 0, 0, 0.35f); ui::fillRect(x, y, w, h, 0.30f, 0.30f, 0.50f, 1); ui::fillRect(x + 2, y + 2, w - 4, h - 4, 0.43f, 0.43f, 0.66f, 1);
        const std::string hd = app.popKind == 1 ? "Information..." : "System...";
        app.font.drawCentered(x + w / 2 + 1, y + 21, hd, 16, 0.2f, 0.2f, 0.3f); app.font.drawCentered(x + w / 2, y + 20, hd, 16, 0.92f, 0.92f, 0.95f);
    } else drawFrame9(app, x, y, w, h);
    if (app.popKind == 3 || app.popKind == 5) app.font.drawCentered(x + w / 2, y + 26, app.popKind == 3 ? "Preferences" : app.popHead, 16, 1, 1, 1);
    for (size_t i = 0; i < app.popLines.size(); i++) {
        const float ty = fy + 24.0f * (float)i; const bool hot = app.popHover == (int)i;
        const bool dis = app.popKind == 2 && ((i == 2 && !app.matchOn) || (i == 3 && false));   // nothing to cancel: the port plays a tournament out at once
        if (menu) {
            if (hot && !dis) app.font.draw(x + 0x24 + 1, ty + 17, app.popLines[i], 16, 0.25f, 0.25f, 0.3f);
            const float c[3] = {dis ? 0.55f : hot ? 0.97f : 0.08f, dis ? 0.55f : hot ? 0.97f : 0.45f, dis ? 0.62f : hot ? 0.9f : 0.48f};
            app.font.draw(x + 0x24, ty + 16, app.popLines[i], 16, c[0], c[1], c[2]);
            const float bx = x + 18, by = ty + 12, br = 6.5f;   // an amber ball with a highlight
            for (int j = -(int)br; j <= (int)br; j++) { const float hw = std::sqrt(std::max(0.0f, br * br - (float)(j * j))); ui::fillRect(bx - hw, by + (float)j, 2 * hw, 1, hot ? 0.95f : 0.45f, hot ? 0.8f : 0.33f, hot ? 0.1f : 0.05f, 1); }
            ui::fillRect(bx - 3, by - 3, 2, 2, hot ? 1.0f : 0.8f, hot ? 0.97f : 0.65f, hot ? 0.6f : 0.3f, 1);
            continue;
        }
        const float tb = hot ? 1.0f : 0.0f; const float tg = hot ? 1.0f : 0.5f;
        app.font.draw(x + 0x24, ty + 16, app.popLines[i], 16, dis ? 0.75f : tb, dis ? 0.75f : tg, dis ? 0.75f : tg);
        const bool on = app.popKind == 3 && (app.popPrefs & (i == 0 ? 1 : i == 1 ? 4 : 0x20));
        ui::fillRect(x + 12, ty + 6, 12, 12, 0.35f, 0.35f, 0.6f, 1);   // PLACEHOLDER ball / box
        if (app.popKind != 3 || on) ui::fillRect(x + 14, ty + 8, 8, 8, hot ? 1.0f : 0.1f, hot ? 1.0f : 0.8f, hot ? 1.0f : 0.8f, 1);
    }
    if (app.okArt.tex && !menu) { const auto& k = sg::ui_screens::kOkCut[(app.popHover == 100) ? 1 : 0]; ui::drawImage(app.okArt, x + w - 44, y + h - 42, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
    ui::endScreen();
}
static void pushKey(SDL_Keycode k) { SDL_Event ev; SDL_zero(ev); ev.type = SDL_KEYDOWN; ev.key.keysym.sym = k; ev.key.state = SDL_PRESSED; SDL_PushEvent(&ev); }
static void toastMsg(App& app, const std::string& m);
static bool champSave(App& app, const std::string& name, std::string& err);
static void t2ListSaves(App& app);
static void t2LoadArt(App& app);
static bool saveGame(App& app, const std::string& file, std::string& err);
static bool loadGame(App& app, const std::string& file, std::string& err);
static void openBest(App& app);
static void openTop10(App& app);
static std::string saveDirOf(const App& app) { return (std::filesystem::path(app.courseFile).has_parent_path() ? std::filesystem::path(app.courseFile).parent_path() : std::filesystem::path(".")).string(); }
static std::string browsePath(const App& app) { return (std::filesystem::path(app.courseFile).has_parent_path() ? std::filesystem::path(app.courseFile).parent_path() : std::filesystem::path(".")).string() + "/While Browsing.sgc"; }
static void popChoose(App& app, int kind, int idx) {
    app.popKind = 0; SDL_StopTextInput();
    if (kind == 1) {
        static const SDL_Keycode k[12] = {SDLK_UNKNOWN, SDLK_F1, SDLK_F2, SDLK_F5, SDLK_F3, SDLK_F7, SDLK_F4, SDLK_F9, SDLK_F10, SDLK_F6, SDLK_UNKNOWN, SDLK_UNKNOWN};
        if (idx == 0) { if (!app.lastMsg.empty()) { app.toast = app.lastMsg; app.toastUntil = SDL_GetTicks() / 1000.0 + 5; } }
        else if (idx == 10) openBest(app); else if (idx == 11) openTop10(app);
        else if (idx > 0 && idx < 10) pushKey(k[idx]);
    } else if (kind == 5) {
        if (idx == 0) { std::string err; toastMsg(app, saveGame(app, saveDirOf(app) + "/" + app.popBuf + ".sgc", err) ? "Game Saved." : "Invalid File Name"); }
    } else if (kind == 2) {
        std::string err;
        switch (idx) {
            case 0: {   // FUN_00405b10: the name is edited in a box; the default is the course name, the day and the month and year
                const int mi = app.econ.day - 1; static const char* mo[8] = {"March", "April", "May", "June", "July", "August", "September", "October"};
                const int dayN = (int)(app.econ.monthProgress() * 30.0) + 1;
                app.popKind = 4; app.popPromptFor = 2; app.popHead = "SAVE GAME: edit name then press Enter.";
                app.popBuf = (app.courseName.empty() ? std::string("Course") : app.courseName) + " " + std::to_string(dayN) + " " + mo[mi % 8] + " " + std::to_string(2001 + mi / 8);
                SDL_StartTextInput(); break; }
            case 1: {   // the exe saves "While Browsing" and Cancel restores it
                std::string e2; const std::string wb = browsePath(app);
                if (saveGame(app, wb, e2)) { app.browsing = true; t2LoadArt(app); app.t2Mode = 0; t2ListSaves(app); app.t2Files.erase(std::remove(app.t2Files.begin(), app.t2Files.end(), wb), app.t2Files.end()); app.t2Sel = -1; app.t2Scroll = 0; app.t2Confirm = 0; app.t2Hover = -1; app.screen = App::ScreenLoad; }
                break; }
            case 2: if (app.matchOn) matchFinish(app, true); else toastMsg(app, "There is no match or tournament to cancel"); break;
            case 3: { const std::string pf = app.gameDir + "/Themes/Championship/" + app.charName + ".pro"; toastMsg(app, sg::charSaveFile(pf, app.chr) ? app.charName + " saved for championship play." : "The player could not be saved"); break; }
            case 4: app.popKind = 4; app.popPromptFor = 1; app.popHead = "Rename Course..."; app.popBuf = app.courseName; SDL_StartTextInput(); break;
            case 5: popOpen(app, 3); break;
            case 6: toastMsg(app, champSave(app, app.courseName.empty() ? std::string("Course") : app.courseName, err) ? app.courseName + " saved for championship play." : "Could not save the course for championship play"); break;
            case 7: { SDL_Event q; SDL_zero(q); q.type = SDL_QUIT; SDL_PushEvent(&q); break; }
        }
    }
}
static bool popEvent(App& app, const SDL_Event& e) {
    if (!app.popKind) return false;
    float x, y, w, h, fy; popGeom(app, x, y, w, h, fy);
    const int n = (int)app.popLines.size();
    if (app.popKind == 4) {
        if (e.type == SDL_TEXTINPUT) { if (app.popBuf.size() < (app.popPromptFor == 2 ? 48u : app.popPromptFor == 3 ? 31u : 32u)) app.popBuf += e.text.text; return true; }
        if (e.type == SDL_KEYDOWN) {
            const SDL_Keycode k = e.key.keysym.sym;
            if (k == SDLK_ESCAPE) { app.popKind = 0; SDL_StopTextInput(); }
            else if (k == SDLK_BACKSPACE && !app.popBuf.empty()) app.popBuf.pop_back();
            else if (k == SDLK_RETURN && app.popPromptFor == 2) {
                std::string nm = app.popBuf; while (!nm.empty() && nm.back() == ' ') nm.pop_back();   // FUN_00405ac0: trim, non empty, no forbidden characters
                if (nm.empty() || nm.find_first_of("<>:\"/\\|?*") != std::string::npos) { app.popKind = 0; SDL_StopTextInput(); toastMsg(app, "Invalid File Name"); }
                else {
                    app.popBuf = nm; SDL_StopTextInput();
                    const std::string path = saveDirOf(app) + "/" + nm + ".sgc";
                    if (std::filesystem::exists(path)) { popOpen(app, 5); app.popHead = "A game called " + nm + " already exists."; }   // overwrite confirm (FUN_0046d6e0); the exact wording is UNKNOWN
                    else { std::string err; app.popKind = 0; toastMsg(app, saveGame(app, path, err) ? "Game Saved." : "Invalid File Name"); }
                }
            }
            else if (k == SDLK_RETURN && app.popPromptFor == 3) { if (app.empRenameRow >= 0 && app.empRenameRow < (int)app.emps.size() && !app.popBuf.empty()) app.emps[(size_t)app.empRenameRow].name = app.popBuf; app.popKind = 0; SDL_StopTextInput(); app.empRenameRow = -1; }
            else if (k == SDLK_RETURN && !app.popBuf.empty()) { app.courseName = app.popBuf; app.popKind = 0; SDL_StopTextInput(); toastMsg(app, "Course renamed"); }
            return true;
        }
        return e.type == SDL_MOUSEBUTTONDOWN || e.type == SDL_MOUSEMOTION;
    }
    float vx = 0, vy = 0; const bool mouse = e.type == SDL_MOUSEMOTION || e.type == SDL_MOUSEBUTTONDOWN;
    if (mouse) { vx = app.view.toVirtualX((e.type == SDL_MOUSEMOTION ? e.motion.x : e.button.x) * app.dpi); vy = app.view.toVirtualY((e.type == SDL_MOUSEMOTION ? e.motion.y : e.button.y) * app.dpi); }
    auto rowAt = [&]() { return (vx > x && vx < x + w - 0x30 && vy >= fy && vy < fy + 24.0f * n) ? (int)((vy - fy) / 24) : -1; };
    auto onOk = [&]() { return std::fabs(vx - (x + w - 22)) < 22 && std::fabs(vy - (y + h - 21)) < 22; };
    if (e.type == SDL_MOUSEMOTION) { app.popHover = onOk() ? 100 : rowAt(); return true; }
    if (e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT) {
        const int r = rowAt();
        if (app.popKind == 3) {
            if (r >= 0) app.popPrefs ^= (r == 0 ? 1 : r == 1 ? 4 : 0x20);
            else if (onOk()) { app.popKind = 0; app.showAdvisor = (app.popPrefs & 4) != 0; }
            return true;
        }
        if (r >= 0 && !(app.popKind == 2 && r == 2)) { popChoose(app, app.popKind, r); return true; }
        if (r < 0 && !onOk() && !(vx > x && vx < x + w && vy > y && vy < y + h)) app.popKind = 0;   // a click outside closes it
        return true;
    }
    if (e.type == SDL_KEYDOWN) {
        const SDL_Keycode k = e.key.keysym.sym;
        if (k == SDLK_ESCAPE) { if (app.popKind == 3) app.showAdvisor = (app.popPrefs & 4) != 0; app.popKind = 0; }
        else if (k == SDLK_UP) app.popHover = std::max(0, (app.popHover < 0 || app.popHover > 50 ? 1 : app.popHover) - 1);
        else if (k == SDLK_DOWN) app.popHover = std::min(n - 1, (app.popHover < 0 || app.popHover > 50 ? -1 : app.popHover) + 1);
        else if ((k == SDLK_RETURN || k == SDLK_SPACE) && app.popHover >= 0 && app.popHover < n) { if (app.popKind == 3) app.popPrefs ^= (app.popHover == 0 ? 1 : app.popHover == 1 ? 4 : 0x20); else popChoose(app, app.popKind, app.popHover); }
        return true;
    }
    return e.type == SDL_MOUSEBUTTONUP || e.type == SDL_MOUSEWHEEL;
}

// Returns true when the click was on the dock or an open panel.
static int stripHit(const App& app, float vx, float vy);
static void openGolferCard(App& app, int gi);
static bool dockClick(App& app, float vx, float vy, bool rightClick, bool& togglePause) {
    if (!rightClick && app.screen == App::ScreenPlay) { const int sg_ = stripHit(app, vx, vy); if (sg_ >= 0) { openGolferCard(app, sg_); return true; } }
    if (!app.uiOk || !app.dockArt.tex) return false;
    if (hireClick(app, vx, vy)) return true;
    const int d = dockHit(vx, vy);
    if (d >= 0) {
        if (d <= 2) {
            const int want = d + 1;
            app.panel = app.panel == want ? 0 : want;
            if (app.panel == 3) { app.golfersMode = true; app.playerPanel = false; }
            app.amenities = false; app.elevation = false; app.hireOpen = false;
            app.edit = app.panel == 1 || app.panel == 2;
            if (app.panel == 1) { app.tool = 0; if (app.terrSlot < 0) app.terrSlot = 0; app.paintIdx = kTerrSlots[app.terrSlot].paint; }
            if (app.panel == 2) { app.tool = 4; for (int k = 0; k < kBuildCount && !buildAvailable(app, app.buildIdx); k++) app.buildIdx = (app.buildIdx + 1) % kBuildCount; }
        } else if (d == 3) app.zoom *= 1.12f;
        else if (d == 4) app.zoom /= 1.12f;
        else if (d == 5) app.rot += 15;
        else if (d == 6) app.rot -= 15;
        else if (d == 7) popOpen(app, 1);   // Information
        else if (d == 8) togglePause = true;
        else if (d == 9) popOpen(app, 2);   // System Functions
        snd(app, "Interface/Button1.wav");
        return true;
    }
    if (pnlClick(app, vx, vy, rightClick)) return true;
    if (app.panel == 1 && terrArt(app)) {
        const int sl = terrSlotAt(app, vx, vy);
        if (sl >= 0) { app.terrSlot = sl; selectPanelItem(app, PanelItem{"", 0, kTerrSlots[sl].paint}, false); snd(app, "Interface/Button2.wav"); return true; }
        if (vx >= 216 && vy >= 482) return true;
    }
    if (app.panel) {
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            if (vx >= x && vx < x + w && vy >= y && vy < y + h) { selectPanelItem(app, items[i], rightClick); snd(app, "Interface/Button2.wav"); return true; }
        }
        if (vx >= 226 && vx < 796 && vy >= 452 && vy < 596) return true;   // the panel background swallows clicks
    }
    return false;
}

static void dockHoverUpdate(App& app, float vx, float vy) {
    app.vmx = vx; app.vmy = vy;
    app.dockHover = dockHit(vx, vy);
    app.panelHover = -1;
    app.terrHover = app.panel == 1 && terrArt(app) ? terrSlotAt(app, vx, vy) : -1;
    if (app.panel) {
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            if (vx >= x && vx < x + w && vy >= y && vy < y + h) app.panelHover = (int)i;
        }
    }
}

// Loads one story script from the disc's Themes folder. Format (read from the files): a title line, then blocks separated by blank lines; the first line of a
// block is one golfer's line, the lines after it that start with a space are the other golfer's possible replies. PARTNER stands for the other golfer.
// What decides which story plays, and when, is not decoded; this picks one by seed and shows it when two golfers are on the course.
static void loadStory(App& app) {
    app.storyLines.clear(); app.storyTitle.clear(); app.storyPos = 0; app.storyNext = 0;
    namespace fs = std::filesystem;
    std::vector<std::string> files;
    std::error_code ec;
    for (const fs::directory_entry& de : fs::directory_iterator(app.gameDir + "/Themes/Standard", ec))
        if (de.path().extension() == ".txt") files.push_back(de.path().string());
    for (const fs::directory_entry& de : fs::directory_iterator(app.gameDir + "/Themes/More_Stories", ec))
        if (de.path().extension() == ".txt") files.push_back(de.path().string());
    if (files.empty()) return;
    std::sort(files.begin(), files.end());
    // The first pair always gets the opening story (DECODE_WORLD2 5.1); later pairs are picked by seed and count.
    size_t pick = (size_t)((app.seed + (unsigned)app.storiesDone * 7u) % files.size());
    if (app.storiesDone == 0) for (size_t i = 0; i < files.size(); i++) if (files[i].find("OpeningDay") != std::string::npos) pick = i;
    app.storyLetter = std::filesystem::path(files[pick]).filename().string()[0];
    std::ifstream in(files[pick], std::ios::binary);
    std::string line; bool first = true, newBlock = true;
    while (std::getline(in, line)) {
        while (!line.empty() && (line.back() == '\r' || line.back() == '\n')) line.pop_back();
        if (first) { size_t a = line.find_first_not_of(' '); app.storyTitle = a == std::string::npos ? "" : line.substr(a); first = false; continue; }
        if (line.empty()) { newBlock = true; continue; }
        const bool indented = line[0] == ' ';
        size_t p;
        while ((p = line.find("PARTNER")) != std::string::npos) line.replace(p, 7, "pal");
        size_t a = line.find_first_not_of(' ');
        if (a == std::string::npos) continue;
        line = line.substr(a);
        if (!indented || newBlock) app.storyLines.push_back("A|" + line); else app.storyLines.push_back("B|" + line);
        newBlock = false;
    }
}

static std::vector<std::string> wrapText(const App& app, const std::string& s, float size, float maxW) {
    std::vector<std::string> out; std::string cur, word;
    auto flush = [&]() { if (!cur.empty()) { out.push_back(cur); cur.clear(); } };
    std::istringstream is(s);
    while (is >> word) {
        const std::string t = cur.empty() ? word : cur + " " + word;
        if (app.font.width(t, size) > maxW && !cur.empty()) { flush(); cur = word; } else cur = t;
    }
    flush();
    return out;
}

// Advisor text from what the club looks like now. These hints are my own words.
static std::string advisorText(const App& app) {
    if (!app.autoOpen) {
        const int pending = (int)app.allHoles.size() - (int)app.holes.size();
        if (pending > 0) return "The hole is built. Press H, or use Open the hole in the Build Course panel, to open it for play. Golfers only play open holes.";
        if (app.teeBlobs > (int)app.allHoles.size()) return "Now we need a green. Paint a putting green a good distance from the tee, then the hole can open.";
        if (app.holes.empty() && app.teeBlobs == 0) return "Let's build our first hole. Open Build Course (the big round button at the bottom left) and paint a tee, then a green. A hole needs only those two.";
    }
    if (app.holes.empty()) return "Welcome to your new club. Open Build Course (the big round button at the bottom left), then paint a tee and a green a good distance apart to make your first hole.";
    if (app.buildings.empty()) return "Golfers are on the course. Open Add Buildings and put up a snack bar: pick a building lot next to a path that joins the clubhouse. Visitors spend money there.";
    if (app.holes.size() < 3) return "More holes bring more golfers and more money. Build another tee and green, and make the holes different: long, narrow and tricky shots raise the club's skill rating.";
    { static const char* kAdvice[15] = {"", "", "", "A ball washer near the tee improves golfers' accuracy.", "", "A home site is a quick way to raise cash.", "A putting green helps golfers with imagination.",
          "A snack bar keeps hungry golfers happy: golfers do not live by golf alone.", "A pro shop lets accurate golfers upgrade their equipment.", "A swim club puts players in a better mood.",
          "A driving range helps long hitters.", "A cart garage speeds up play.", "A marina raises home values.", "A resort hotel means golfers tire less.", "An airstrip lets you charge higher green fees."};
      for (int b = 14; b >= 3; b--) if (b < app.unlockLevel && b != 4) { bool built = false; for (const App::Placed& pl : app.buildings) if (kBuild[pl.def].unlock == b) built = true; if (!built && kAdvice[b][0]) return std::string(kAdvice[b]) + " Open Add Buildings to build one."; } }
    if (app.econ.staffCount() == 0) return "Your course is growing. The People button lets you hire a club pro, ranger, groundskeeper or soda vendor, who cost wages but keep golfers happy.";
    return "Watch the fun and skill numbers at the top right. Press the information button for the course report, and keep cash above zero so the board stays calm.";
}

static void drawRoundBox(float x, float y, float w, float h, float r, float g, float b);
// The mood face strip along the bottom edge while no dock panel is open (seen in the real screenshots): one face per golfer on the course in hole order, the
// golfer's hole number under it; a click opens that golfer's card. The face art and the mood slot are exact (DECODE_FACES.md 1.5); the strip position is measured by eye (PLACEHOLDER).
static std::vector<int> stripGolfers(const App& app) {
    std::vector<int> v; for (size_t i = 0; i < app.golfers.size(); i++) if (app.golfers[i].active) v.push_back((int)i);
    std::stable_sort(v.begin(), v.end(), [&](int a, int b) { return app.golfers[(size_t)a].hole < app.golfers[(size_t)b].hole; });
    return v;
}
static int stripHit(const App& app, float vx, float vy) {
    if (app.panel || !app.memberArt.tex || vy < 574 || vy >= 600 || vx < 218) return -1;
    const int k = (int)((vx - 218) / 16); const std::vector<int> v = stripGolfers(app);
    return k >= 0 && k < (int)v.size() ? v[(size_t)k] : -1;
}
static void drawGolferStrip(App& app) {
    if (app.panel || !app.memberArt.tex) return;
    const std::vector<int> v = stripGolfers(app);
    for (size_t i = 0; i < v.size(); i++) {
        const Golfer& g = app.golfers[(size_t)v[i]];
        const int face = std::clamp(g.mood + 2, 1, 10) - 1;
        const float x = 218.0f + 16.0f * (float)i;
        ui::drawImage(app.faceArt.tex ? app.faceArt : app.memberArt, x, 574, (float)(594 - 16 * face), 100, 16, 16);
        ui::fillRect(x, 590, 16, 10, 0.96f, 0.95f, 0.9f, 1); ui::fillRect(x, 590, 1, 10, 0.25f, 0.25f, 0.4f, 1);
        app.font.drawCentered(x + 8, 599, std::to_string(g.hole + 1), 9, 0.1f, 0.1f, 0.25f);
    }
}
static void drawDockUi(App& app) {
    if (!app.uiOk) return;
    drawGolferStrip(app);
    if (app.testHx >= 0) dockHoverUpdate(app, app.testHx, app.testHy);
    if (app.dockArt.tex) {
        ui::drawImage(app.dockArt, 0, 430, 0, 430, 215, 170);
        if (app.dockHover >= 0) {
            const DockBtn& b = kDock[app.dockHover];
            const float dx = b.cx - (b.sx + b.sw * 0.5f), dy = b.cy - (b.sy + b.sh * 0.5f);
            ui::drawImage(app.dockArt, b.sx + dx, b.sy + dy, b.sx + b.hoverDx, b.sy, b.sw, b.sh);
        }
        if (app.dockHover >= 0) app.font.draw(228, 448, kDockHelp[app.dockHover], 14, 1, 1, 0.7f);
    }
    if (app.panel && pnlArt(app)) {
        pnlDraw(app, app.testHx >= 0 ? app.testHx : app.vmx, app.testHx >= 0 ? app.testHy : app.vmy);
    } else if (app.panel) {
        ui::fillRect(226, 452, 570, 144, 0.16f, 0.14f, 0.34f, 0.88f);
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            bool sel = false;
            if (items[i].kind == 0) sel = app.tool == 0 && app.paintIdx == items[i].arg;
            else if (items[i].kind == 1) sel = app.tool == 2 && app.pathKind == items[i].arg;
            else if (items[i].kind == 2) sel = app.tool == 1 && app.raiseSign > 0;
            else if (items[i].kind == 3) sel = app.tool == 1 && app.raiseSign < 0;
            else if (items[i].kind == 4) sel = app.tool == 4 && app.buildIdx == items[i].arg;
            if (sel) ui::fillRect(x, y, w, h, 0.9f, 0.75f, 0.2f, 0.55f);
            else if ((int)i == app.panelHover) ui::fillRect(x, y, w, h, 0.5f, 0.5f, 0.9f, 0.45f);
            app.font.draw(x + 4, y + 13, items[i].label, 13, 1, 1, 1);
        }
    }
    // Advisor and story, top centre.
    if (app.showAdvisor && app.screen != App::ScreenGolfer) {
        const std::vector<std::string> lines = wrapText(app, app.charName + ": " + advisorText(app), 14, 270);
        const float h = std::max(10 + 17.0f * lines.size(), 90.0f);
        ui::fillRect(244, 8, 306, h, 0.12f, 0.1f, 0.3f, 0.82f);
        for (size_t i = 0; i < lines.size(); i++) app.font.draw(282, 25 + 17.0f * i, lines[i], 14, 1, 0.95f, 0.7f);
        chrInit(app); if (!app.cgBtn.tex) loadCharArt(app);
        ui::drawImage(app.ballArt, 144, -2, 0, 300, 140, 140);   // the speaker's portrait: ball and halo head (docs/DECODE_FACES.md 2.1 row 5; position PLACEHOLDER, the exe's popup origin differs)
        drawFace(app, app.chr.female(), app.chr.head, 1, 144, -2, true);
        float y = 8 + h + 6;
        if (!app.storyLines.empty() && golfersOnCourse(app) >= 2) {
            const std::string& s = app.storyLines[app.storyPos % app.storyLines.size()];
            const bool a = s.size() > 2 && s[0] == 'A';
            const std::vector<std::string> sl = wrapText(app, s.substr(2), 14, 290);
            const float sh = 12 + 17.0f * (sl.size() + 1);
            ui::fillRect(244, y, 306, sh, a ? 0.25f : 0.12f, a ? 0.12f : 0.22f, a ? 0.12f : 0.12f, 0.82f);
            app.font.draw(252, y + 16, app.storyTitle + (a ? " (golfer one)" : " (golfer two)"), 12, 0.8f, 0.9f, 1);
            for (size_t i = 0; i < sl.size(); i++) app.font.draw(252, y + 33 + 17.0f * i, sl[i], 14, 1, 1, 1);
        }
    }
    if (app.tutPage >= 0) {
        const float nx = 160, ny = 150, nw = 480;
        const std::vector<std::string> ln = wrapText(app, kTutorial[app.tutPage], 15, nw - 40);
        const float nh = 78 + 20.0f * (float)ln.size();
        ui::fillRect(nx - 2, ny - 2, nw + 4, nh + 4, 0.62f, 0.6f, 0.86f, 0.97f); ui::fillRect(nx, ny, nw, nh, 0.1f, 0.2f, 0.1f, 0.94f);
        app.font.drawCentered(400, ny + 24, std::string(app.tutPage < 11 ? "Tutorial: Fun" : "Tutorial: Skill") + "  (" + std::to_string(app.tutPage + 1) + " of 20)", 16, 1, 0.9f, 0.4f);
        for (size_t i = 0; i < ln.size(); i++) app.font.draw(nx + 20, ny + 52 + 20.0f * (float)i, ln[i], 15, 0.98f, 0.97f, 0.9f);
        app.font.drawCentered(400, ny + nh - 12, "Press any key to continue, Escape to stop.", 12, 0.75f, 0.85f, 0.75f);
    }
    if (!app.toast.empty() && SDL_GetTicks() / 1000.0 < app.toastUntil) {
        if (app.toastKind == 1 && app.shArt.tex) {   // notice: dark green translucent panel, lavender edge, an icon plate on the left (SGA offers show the trophy); measured on the real screenshots (PLACEHOLDER geometry)
            const bool trophy = app.toast.rfind("The SGA", 0) == 0;
            const float nx = 190, nw = 420, tx0 = trophy ? nx + 96 : nx + 16, tw = nx + nw - 14 - tx0;
            const std::vector<std::string> ln = wrapText(app, app.toast, 14, tw);
            const float nh = std::max(14 + 18.0f * (float)ln.size(), trophy ? 88.0f : 40.0f), ny = app.showAdvisor ? 112.0f : 12.0f;
            ui::fillRect(nx - 2, ny - 2, nw + 4, nh + 4, 0.62f, 0.6f, 0.86f, 0.95f); ui::fillRect(nx, ny, nw, nh, 0.1f, 0.2f, 0.1f, 0.9f);
            if (trophy) { drawRoundBox(nx + 14, ny + nh / 2 - 36, 56, 72, 0.2f, 0.22f, 0.3f); ui::drawImageScaled(app.shArt, nx + 20, ny + nh / 2 - 30, 44, 55, 64, 0, 16, 20); }
            for (size_t i = 0; i < ln.size(); i++) app.font.draw(tx0, ny + 22 + 18.0f * (float)i, ln[i], 14, 0.98f, 0.97f, 0.9f);
        } else {
            const float w = app.font.width(app.toast, 16) + 24;
            ui::fillRect(400 - w / 2, 410, w, 28, 0.5f, 0.1f, 0.1f, 0.88f);
            app.font.drawCentered(400, 430, app.toast, 16, 1, 1, 1);
        }
    }
}

// Home site placement preview (docs/DECODE_HOMES.md section 2.1): the quarter lot value the club is paid, the site clearing and the fixed preparation cost.
static void drawHomePreview(App& app) {
    if (!(app.edit && app.tool == 5 && app.amenTool == 5 && app.hasHit)) return;
    int tx, ty; tileOf(app, app.hitX, app.hitZ, tx, ty);
    if (tx < 0 || ty < 0 || tx >= app.terrain.w || ty >= app.terrain.h) return;
    HomeLotEnv env; makeHomeEnv(app, env);
    const int lot = env.lot(tx, ty);
    env.bc.lotValueUnits = 9999;
    const sg::PlaceCheck pc = app.bsys.canPlace(sg::buildings_exe::HomeSite, tx, ty, env.bc);
    const int share = sg::homes::clubShare(lot);
    const int site = pc.ok ? pc.siteUnits : 0, prep = sg::buildings_exe::kBaseCostUnits[sg::buildings_exe::HomeSite];
    const int profit = share - prep - site;
    ui::fillRect(236, 408, 190, 66, 0.10f, 0.09f, 0.26f, 0.9f);
    auto line = [&](int i, const char* label, long long units, bool bad) {
        app.font.draw(246, 426.0f + 15.0f * i, label, 13, 0.85f, 0.85f, 1);
        const std::string v = money(units * 100);
        app.font.draw(416 - app.font.width(v, 13), 426.0f + 15.0f * i, v, 13, bad ? 1.0f : 0.85f, bad ? 0.45f : 1.0f, bad ? 0.45f : 0.85f);
    };
    line(0, "Lot value:", share, false); line(1, "Site clear:", site, false); line(2, "Site prep.:", prep, false); line(3, pc.ok ? "Profit:" : "Cannot build here", pc.ok ? profit : 0, profit <= 0);
}

// ---- Title side screens and the Top 10 (docs/DECODE_TITLE2.md, docs/DECODE_TOP10_PAIR.md). Art and positions follow the exe. PLACEHOLDER: the key that opens the Top 10 (F11) and the
// click that opens the credits (the exe's menu dispatch is not decoded), the Load screen's side panel, the delete confirmation box (the exe has a yes/no box), the scroll thumb.
static void t2LoadArt(App& app) {
    if (app.t2Ok) return;
    app.t2Ok = true;
    const std::string i = app.gameDir + "/Interface/";
    ui::loadPcx(i + "TitleSelDiffUnSel.pcx", app.t2Diff, false); ui::loadPcx(i + "TitleSelDiffMO.pcx", app.t2DiffMo, true);
    ui::loadPcx(i + "Title_ThemePacks.pcx", app.t2Theme, false); ui::loadPcx(i + "Title_ThemePacks_MO.pcx", app.t2ThemeMo, true);
    ui::loadPcx(i + "Title_Pickapro.pcx", app.t2Pro, false); ui::loadPcx(i + "infoscreens/lowscore.pcx", app.bestArt, true); ui::loadPcx(i + "Title_LoadGame.pcx", app.t2Load, false); ui::loadPcx(i + "Title_LoadGame_MO.pcx", app.t2LoadMo, true);
    ui::loadPcx(i + "Top10_Blank.pcx", app.t10Blank, false); ui::loadPcx(i + "Top10_Trophies.pcx", app.t10Troph, false);
    ui::loadPcx(app.gameDir + "/creditsbckgrd.pcx", app.creditsBg, false); ui::loadPcx(app.gameDir + "/bink64.pcx", app.creditsLogo, false);
}
static float tBase(float size) { return size * 0.82f; }   // the exe's text calls take the top of the text; the port's font takes the baseline
static void tText(App& app, float x, float y, const std::string& s, float size, float r, float g, float b, bool centre) {
    if (centre) app.font.drawCentered(x, y + tBase(size), s, size, r, g, b); else app.font.draw(x, y + tBase(size), s, size, r, g, b);
}
static void c15(unsigned v, float* o) { o[0] = ((v >> 10) & 31) / 31.0f; o[1] = ((v >> 5) & 31) / 31.0f; o[2] = (v & 31) / 31.0f; }   // 15 bit colours of the exe
static int octDist(float dx, float dy) { const int a = (int)std::fabs(dx), b = (int)std::fabs(dy); return b < a ? (b + 2 * a) / 2 : (a + 2 * b) / 2; }

// Difficulty select (FUN_0043a400)
static const int kDiffSrc[4][4] = {{0, 0, 338, 146}, {400, 0, 338, 146}, {0, 300, 332, 114}, {400, 300, 332, 130}}, kDiffDst[4][2] = {{193, 32}, {150, 161}, {161, 320}, {200, 436}};
static const int kDiffC[4][2] = {{348, 107}, {319, 218}, {320, 367}, {363, 495}}, kDiffLbl[4][2] = {{388, 113}, {360, 227}, {366, 346}, {402, 464}};
static const char* kDiffName[4] = {"Easy", "Moderate", "Difficult", "Impossible"};
static int diffHit(float x, float y) {
    for (int i = 0; i < 4; i++) if (octDist((x - kDiffC[i][0]) / 2, y - kDiffC[i][1]) < 80) return i;
    return std::hypot(x - 767, y - 557) < 25 ? 4 : -1;
}
static void drawDiff(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.t2Diff, 0, 0);
    if (app.t2Hover >= 0 && app.t2Hover < 4) { const int* s = kDiffSrc[app.t2Hover]; ui::drawImage(app.t2DiffMo, (float)kDiffDst[app.t2Hover][0], (float)kDiffDst[app.t2Hover][1], (float)s[0], (float)s[1], (float)s[2], (float)s[3]); }
    if (app.t2Hover == 4) ui::drawImage(app.t2DiffMo, 732, 532, 732, 532, 68, 68);
    tText(app, 602, 42, "Select Difficulty", 20, 0, 0, 0, true);
    for (int i = 0; i < 4; i++) { const bool h = app.t2Hover == i; float c[3]; c15(0x4210, c); tText(app, (float)kDiffLbl[i][0], (float)kDiffLbl[i][1] + (h ? 0 : 2), kDiffName[i], 20, h ? 0 : c[0], h ? 0 : c[1], h ? 0 : c[2], true); }
    ui::endScreen();
}

// Theme packs (FUN_004725b0)
static std::vector<std::string> themeFolders(const App& app) {
    std::vector<std::string> v;
    std::error_code ec;
    for (const auto& d : std::filesystem::directory_iterator(app.gameDir + "/Themes", ec)) {
        if (!d.is_directory()) continue;
        const std::string n = d.path().filename().string();
        if (n.find('.') != std::string::npos || n == "Championship") continue;
        v.push_back(n);
    }
    std::sort(v.begin(), v.end());
    return v;
}
static bool themeHas(const App& app, const std::string& folder, int col) {
    std::error_code ec; const std::filesystem::path dir = std::filesystem::path(app.gameDir) / "Themes" / folder;
    auto ext = [&](const char* e1, const char* e2) { for (const auto& f : std::filesystem::directory_iterator(dir, ec)) { std::string x = f.path().extension().string(); for (char& ch : x) ch = (char)std::tolower((unsigned char)ch); if (x == e1 || (e2 && x == e2)) return true; } return false; };
    switch (col) { case 0: return ext(".txt", nullptr); case 1: return ext(".glf", ".chr"); case 2: return std::filesystem::exists(dir / "celebrities.dta", ec); case 3: return std::filesystem::exists(dir / "progolfers.dta", ec); default: return ext(".cse", nullptr); }
}
static std::string themeLabel(std::string s) { for (char& c : s) if (c == '_') c = ' '; return s; }
static void drawThemes(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.t2Theme, 0, 0);
    tText(app, 78, 34, "Select a Theme Pack", 18, 0, 0, 0, false);
    static const char* kCol[5] = {"Stories", "Characters", "Celebrities", "Pro Golfers", "Courses"};
    float c[3]; c15(0x4210, c);
    for (int k = 0; k < 5; k++) tText(app, 346.0f + 90 * k, (k & 1) ? 37.0f : 62.0f, kCol[k], 11, 0, 0, 0, true);
    const std::vector<std::string> th = themeFolders(app);
    for (size_t i = 0; i < th.size() && i < 13; i++) {
        const float y = 92.0f + 32 * (float)i; const bool cur = themeLabel(th[i]) == kThemePacks[app.themePack];
        if (cur) ui::drawImage(app.t2ThemeMo, 72, y, 1, 33, 221, 31);
        tText(app, 73, y + 11, themeLabel(th[i]), 14, cur ? 0.0f : c[0], cur ? 0.0f : c[1], cur ? 0.0f : c[2], false);
        for (int k = 0; k < 5; k++) {
            const bool has = themeHas(app, th[i], k); const float x = 346.0f + 90 * k - 43;
            if (cur) ui::drawImage(app.t2ThemeMo, x, y, has ? 89.0f : 1.0f, 1, 87, 31); else if (has) ui::drawImage(app.t2ThemeMo, x, y, 177, 1, 87, 31);
        }
    }
    if (app.t2Hover == 0) ui::drawImage(app.t2ThemeMo, 581, 520, 265, 1, 71, 80);
    if (app.t2Hover == 1) ui::drawImage(app.t2ThemeMo, 726, 533, 337, 1, 48, 49);
    ui::endScreen();
}

// Load Previous Game (FUN_0043b610). The port keeps its saves as *.sgc files beside the course file.
static std::string t2RowName(const std::string& path) {   // names starting with '&' are autosaves (DECODE_TITLE2 section 1); the bracket punctuation is DERIVED
    std::string n = std::filesystem::path(path).stem().string();
    if (!n.empty() && n[0] == '&') n = "autosave (" + n.substr(1) + ")";
    return n;
}
static void t2ListFolder(App& app, const char* sub, const char* ext) {   // championship courses (.cse) and pros (.pro) live in Themes/Championship
    app.t2Files.clear();
    std::error_code ec;
    for (const auto& f : std::filesystem::directory_iterator(std::filesystem::path(app.gameDir) / "Themes" / sub, ec)) {
        std::string x = f.path().extension().string(); for (char& ch : x) ch = (char)std::tolower((unsigned char)ch);
        if (x == ext && f.path().filename().string().find("Shadow") == std::string::npos) app.t2Files.push_back(f.path().string());
    }
    std::sort(app.t2Files.begin(), app.t2Files.end());
}
static void t2ListSaves(App& app) {
    app.t2Files.clear();
    std::error_code ec; const std::filesystem::path dir = std::filesystem::path(app.courseFile).has_parent_path() ? std::filesystem::path(app.courseFile).parent_path() : std::filesystem::path(".");
    for (const auto& f : std::filesystem::directory_iterator(dir, ec)) if (f.path().extension() == ".sgc" && f.path().filename().string().find("Shadow") == std::string::npos && f.path().filename().string().find("While Browsing") == std::string::npos) app.t2Files.push_back(f.path().string());
    std::sort(app.t2Files.begin(), app.t2Files.end());
}
static int loadHit(float x, float y) {
    if (std::hypot(x - 633, y - 555) < 26) return 100;
    if (std::hypot(x - 769, y - 556) < 24) return 101;
    if (std::hypot(x - 784, y - 120) < 20) return 102;
    if (std::hypot(x - 784, y - 400) < 20) return 103;
    if (std::hypot(x - 704, y - 555) < 23) return 104;
    return -1;
}
// Load screen side panel (DECODE_TITLE2 section 1): the numbers come from the INFO section the port writes into its saves, the picture from a thumbnail
// captured when the game was saved. PLACEHOLDER: the exe's own save file layout is not used; the oval picture's source in the exe is unknown.
static void t2ReadInfo(App& app, const std::string& path) {
    if (app.t2InfoPath == path) return;
    app.t2InfoPath = path; app.t2Info = App::SaveInfo();
    if (app.t2Thumb) { glDeleteTextures(1, &app.t2Thumb); app.t2Thumb = 0; }
    FILE* f = std::fopen(path.c_str(), "r"); if (!f) return;
    char line[512];
    while (std::fgets(line, sizeof line, f)) {
        if (!std::strncmp(line, "GAME ", 5) && std::fgets(line, sizeof line, f)) { line[std::strcspn(line, "\r\n")] = 0; app.t2Info.designer = ""; app.t2Info.themeName = line; }   // the course name line
        else if (!std::strncmp(line, "BEST 1", 6)) { int n = 0; std::sscanf(line + 6, "%d", &n); if (n > 0 && std::fgets(line, sizeof line, f)) { int sc = 0, off = 0; if (std::sscanf(line, "%d %n", &sc, &off) >= 1) { app.t2Info.record = sc; app.t2Info.recordBy = line + off; while (!app.t2Info.recordBy.empty() && (app.t2Info.recordBy.back() == '\n' || app.t2Info.recordBy.back() == '\r')) app.t2Info.recordBy.pop_back(); } } }
        else if (!std::strncmp(line, "CAREER 1", 8)) { int cp = 0; unsigned co = 0; if (std::sscanf(line + 8, "%d %u", &cp, &co) >= 1) app.t2Info.prop = std::max(0, cp); }
        else if (!std::strncmp(line, "INFO 1", 6)) {
            size_t h = 0; double cash = 0, len = 0, acc = 0, img = 0;
            if (std::sscanf(line + 6, "%zu %d %d %lf %d %lf %lf %lf %d", &h, &app.t2Info.par, &app.t2Info.yards, &cash, &app.t2Info.fun, &len, &acc, &img, &app.t2Info.theme) == 9) {
                app.t2Info.holes = (int)h; app.t2Info.cash = cash; app.t2Info.len = (int)len; app.t2Info.acc = (int)acc; app.t2Info.img = (int)img;
                for (size_t i = 0; i < h && i < 18; i++) { int p = 0, y = 0; if (!std::fgets(line, sizeof line, f) || std::sscanf(line, "%d %d", &p, &y) != 2) break; app.t2Info.hole.push_back({p, y}); }
                if (std::fgets(line, sizeof line, f)) { line[std::strcspn(line, "\r\n")] = 0; app.t2Info.designer = line; }
                app.t2Info.ok = true;
            }
            break;
        }
    }
    std::fclose(f);
    if (FILE* t = std::fopen((path + ".thumb").c_str(), "rb")) {
        int w = 0, hh = 0; char hdr[64] = {}; if (std::fgets(hdr, sizeof hdr, t) && std::sscanf(hdr, "SGT1 %d %d", &w, &hh) == 2 && w > 0 && hh > 0 && w <= 512 && hh <= 512) {
            std::vector<unsigned char> px((size_t)w * hh * 3);
            if (std::fread(px.data(), 1, px.size(), t) == px.size()) { glGenTextures(1, &app.t2Thumb); glBindTexture(GL_TEXTURE_2D, app.t2Thumb); glPixelStorei(GL_UNPACK_ALIGNMENT, 1); glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, w, hh, 0, GL_RGB, GL_UNSIGNED_BYTE, px.data()); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR); }
        }
        std::fclose(t);
    }
}
static void captureThumb(App& app) {   // a 116 x 90 picture of the world view, taken the frame after a save
    const std::string path = app.thumbReq; app.thumbReq.clear();
    const int W = 116, H = 90;
    const float sc = std::min(app.drawW / 800.0f, app.drawH / 600.0f);
    const int cw = std::min(app.drawW, (int)(480 * sc)), ch = std::min(app.drawH, (int)(cw * H / (float)W));
    const int x0 = std::max(0, (app.drawW - cw) / 2), y0 = std::max(0, (app.drawH - ch) / 2);
    std::vector<unsigned char> px((size_t)cw * ch * 4); glPixelStorei(GL_PACK_ALIGNMENT, 1); glReadPixels(x0, y0, cw, ch, GL_RGBA, GL_UNSIGNED_BYTE, px.data());
    FILE* t = std::fopen(path.c_str(), "wb"); if (!t) return;
    std::fprintf(t, "SGT1 %d %d\n", W, H);
    for (int y = H - 1; y >= 0; y--) for (int x = 0; x < W; x++) {   // box average, top row first
        int r = 0, g = 0, b = 0, n = 0;
        for (int sy = y * ch / H; sy < std::max(y * ch / H + 1, (y + 1) * ch / H); sy++) for (int sx = x * cw / W; sx < std::max(x * cw / W + 1, (x + 1) * cw / W); sx++) { const unsigned char* q = &px[((size_t)sy * cw + sx) * 4]; r += q[0]; g += q[1]; b += q[2]; n++; }
        unsigned char o[3] = {(unsigned char)(r / n), (unsigned char)(g / n), (unsigned char)(b / n)}; std::fwrite(o, 1, 3, t);
    }
    std::fclose(t);
}
// Club emblem per property: sheet 0 = parklink.pcx (cuts at x=4+88*cut, y=396), sheet 1 = tropdesert.pcx (y=481). DERIVED from the art and the real screenshots
// (site table order Monterey, San Diego, Rocky, Las Vegas, Phoenix, Hawaii, Oahu, Nova Scotia, Northeast, Carolina, Ireland, Scotland, Wales, Spain, Florida, Jamaica).
static void drawEmblem(App& app, int prop, float x, float y, float size) {
    static const int kEmb[16][2] = {{0,1},{1,4},{0,3},{1,3},{1,0},{1,7},{1,6},{0,0},{0,7},{0,2},{0,6},{0,4},{0,5},{1,2},{1,5},{1,1}};
    if (prop < 0 || prop > 15) return;
    const int sheet = kEmb[prop][0], cut = kEmb[prop][1]; ui::Image& im = app.emblem[sheet];
    if (!im.tex) { const std::string i = app.gameDir + "/Interface/"; const char* nm = sheet ? "tropdesert" : "parklink"; ui::loadPcx(i + nm + ".pcx", im, false, -1, i + nm + "_A.pcx"); }
    if (im.tex) ui::drawImageScaled(im, x, y, size, size, 4.0f + 88 * cut, sheet ? 481.0f : 396.0f, 80, 80);
}

static void drawLoadPanel(App& app) {
    const bool sel = app.t2Sel >= 0 && app.t2Sel < (int)app.t2Files.size();
    if (!sel) return;
    t2ReadInfo(app, app.t2Files[(size_t)app.t2Sel]);
    const App::SaveInfo& in = app.t2Info;
    if (in.ok) {   // club emblem in the black oval (docs/DECODE_MENUS.md section 9: 8 cuts of 80 x 80 on parklink.pcx and tropdesert.pcx)
        drawEmblem(app, in.prop, 84, 32, 80);
    }
    if (in.ok) {
        float g[3]; c15(0x4210, g); float fg[3]; c15(0x1284, fg); float sk[3]; c15(0x0210, sk);
        static const char* kH[3] = {"Holes", "Par", "Yards"};
        const float cx[3] = {66, 125, 184};
        const std::string v[3] = {std::to_string(in.holes), std::to_string(in.par), std::to_string(in.yards)};
        for (int k = 0; k < 3; k++) { tText(app, cx[k], 124, kH[k], 11, g[0], g[1], g[2], true); tText(app, cx[k], 146, v[k], 12, 0, 0, 0, true); }
        tText(app, 75, 169, "Cash", 11, g[0], g[1], g[2], true); tText(app, 161, 169, money((long long)in.cash), 12, 0, 0, 0, true);
        static const char* kL[4] = {"Fun Rating", "Length Skill", "Accuracy Skill", "Imagination"};
        for (int k = 0; k < 4; k++) {
            tText(app, 48, 192.0f + 18 * k, kL[k], 11, g[0], g[1], g[2], false);
            char b[32]; const int val = k == 0 ? in.fun : k == 1 ? in.len : k == 2 ? in.acc : in.img;
            if (k == 0) std::snprintf(b, sizeof b, "%d", val); else std::snprintf(b, sizeof b, "%.2f", val / 100.0);
            tText(app, 184, 192.0f + 18 * k, b, 12, k == 0 ? fg[0] : sk[0], k == 0 ? fg[1] : sk[1], k == 0 ? fg[2] : sk[2], true);
        }
        static const char* kT[3] = {"Hole", "Par", "Yards"};
        for (int k = 0; k < 3; k++) tText(app, cx[k], 270, kT[k], 11, g[0], g[1], g[2], true);
        for (size_t h = 0; h < in.hole.size(); h++) {
            const float y = 286.0f + 17 * (float)h;
            tText(app, cx[0], y, std::to_string(h + 1), 12, 0, 0, 0, true); tText(app, cx[1], y, std::to_string(in.hole[h].first), 12, 0, 0, 0, true); tText(app, cx[2], y, std::to_string(in.hole[h].second), 12, 0, 0, 0, true);
        }
        tText(app, 388, 442, " (Theme: " + std::string(kThemePacks[std::clamp(in.theme, 0, 4)]) + ")", 12, 1, 1, 1, false);
        tText(app, 388, 458, "Designed by " + in.designer, 12, 1, 1, 1, false);
        if (in.record > 0) tText(app, 388, 474, "Course Record: " + std::to_string(in.record) + " by " + in.recordBy, 12, 1, 1, 1, false);
    }
}
static void t2Ok(App& app);
static void drawLoad(App& app) {
    const bool pro = app.screen == App::ScreenPro;   // Pick A Pro shares the layout and hit circles of the Load screen (DECODE_TITLE2 section 2)
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(pro ? app.t2Pro : app.t2Load, 0, 0);
    const bool sel = app.t2Sel >= 0 && app.t2Sel < (int)app.t2Files.size();
    if (!sel) { ui::drawImage(app.t2LoadMo, 600, 520, 600, 435, 70, 70); ui::drawImage(app.t2LoadMo, 670, 520, 670, 435, 70, 70); }
    else {
        if (app.t2Hover == 100) ui::drawImage(app.t2LoadMo, 600, 520, 600, 520, 70, 70);
        if (app.t2Hover == 104) ui::drawImage(app.t2LoadMo, 670, 520, 670, 520, 70, 70);
    }
    if (app.t2Hover == 101) ui::drawImage(app.t2LoadMo, 740, 530, 740, 530, 60, 60);
    if (app.t2Hover == 102) ui::drawImage(app.t2LoadMo, 778, 100, 778, 100, 22, 40);
    if (app.t2Hover == 103) ui::drawImage(app.t2LoadMo, 778, 380, 778, 380, 22, 40);
    tText(app, 504, 42, pro ? "Pick A Pro" : app.t2Mode == 1 ? "Select Championship Course" : "Load Previous Game", 18, 0, 0, 0, true);
    const int n = (int)app.t2Files.size();
    for (int r = 0; r < 16 && app.t2Scroll + r < n; r++) {
        const int idx = app.t2Scroll + r; const float y = 116.0f + 16 * r;
        const bool on = idx == app.t2Sel;
        if (on) { float c[3]; c15(pro ? 0x1284 : 0x7b20, c); ui::fillRect(310, y - 1, 456, 15, c[0], c[1], c[2], 1); }
        const float tc = on && pro ? 1.0f : 0.0f;
        tText(app, 320, y, t2RowName(app.t2Files[(size_t)idx]), 13, tc, tc, tc, false);
    }
    if (n > 16 && app.t2Scroll + 16 < n) tText(app, 320, 372, "(more...)", 13, 0, 0, 0, false);
    if (n == 0) tText(app, 320, 116, pro ? "No pros found." : app.t2Mode == 1 ? "No championship courses saved yet." : "No saved games found.", 13, 0.3f, 0.3f, 0.3f, false);
    {   // tooltips appear after the pointer rests on a button for 30 frames (about half a second)
        if (app.t2Hover != app.t2HoverPrev) { app.t2HoverPrev = app.t2Hover; app.t2HoverSince = SDL_GetTicks(); }
        if (app.t2Hover >= 100 && SDL_GetTicks() - app.t2HoverSince > 500 && !(app.t2Hover == 100 && !sel) && !(app.t2Hover == 104 && !sel)) {
            static const float cx[5] = {600, 740, 778, 778, 670}, cy[5] = {520, 530, 100, 380, 520};
            const int k = app.t2Hover - 100; static const char* tip[5] = {"OK", "Cancel", "", "", "Delete"};   // PLACEHOLDER: the OK word lives in a shared literal under five characters
            if (tip[k][0]) { ui::fillRect(cx[k] + 6, cy[k] + 14, 48, 16, 0.1f, 0.1f, 0.1f, 0.9f); tText(app, cx[k] + 30, cy[k] + 16, tip[k], 11, 1, 1, 1, true); }
        }
    }
    if (app.t2Go) { if (pro) tText(app, 388, 472, "Loading...", 13, 1, 1, 1, false); else { float c[3]; c15(0x7b20, c); tText(app, 550, 456, "Loading...", 13, c[0], c[1], c[2], false); } }
    if (!pro) drawLoadPanel(app);
    else if (sel) {   // Pick A Pro left panel
        if (!app.headHalo[0].tex && !app.headHalo[1].tex) loadCharArt(app);
        if (app.proPrevPath != app.t2Files[(size_t)app.t2Sel]) { app.proPrevPath = app.t2Files[(size_t)app.t2Sel]; app.proPrevOk = sg::charLoadFile(app.proPrevPath, app.proPrev); if (app.proPrevOk && app.proPrev.head >= cuHeadCount(app)) app.proPrev.head = 8; }
        if (app.proPrevOk) {
            const sg::CharRec& c = app.proPrev;
            drawFace(app, c.female(), c.head, 1, 69, 61, true);
            tText(app, 56, 255, std::string(c.name) + "'s skills", 13, 0, 0, 0, false);
            static const char* kSk[10] = {"Power Hitter", "Long Driver", "Accurate Driver", "Accurate Irons", "Accurate Putter", "Draw Shot (R to L)", "Fade Shot (L to R)", "High Backspin Shot", "Recovery Skills", "Luck"};
            for (int k = 0; k < 10; k++) {
                const float y = 279.0f + 24 * k;
                tText(app, 94, y, kSk[k], 12, 0, 0.5f, 0.5f, false);
                if (c.skills[k] > 0 && c.skills[k] < 10) drawPlusPct(app, 59, y + 12, c.skills[k] * 10, 12, 0, 0.5f, 0.5f, true); else tText(app, 59, y, std::to_string(c.skills[k] * 10) + "%", 12, 0, 0.5f, 0.5f, true);
            }
            tText(app, 56, 544, "Signature saying:", 13, 0, 0, 0, false);
            const std::string say = c.dialogue[0][0] ? c.dialogue[0] : "Fore!";   // PLACEHOLDER stock fallback; the exe takes dialogue event 0x3e
            tText(app, 36, 562, "\"" + say + "\"", 12, 1, 1, 1, false);
        }
    }
    if (app.t2Confirm && sel) {   // FUN_0046d6e0(400,100,1,1,0): heading with the file name and two options (PLACEHOLDER: the second option's wording beyond "No, never" is unknown)
        drawFrame9(app, 150, 80, 500, 120);
        tText(app, 400, 100, "Are you sure you want to delete " + t2RowName(app.t2Files[(size_t)app.t2Sel]) + "?", 13, 1, 1, 1, true);
        for (int k = 0; k < 2; k++) { const bool hot = app.t2ConfirmHover == k; tText(app, 400, 138.0f + 24 * k, k == 0 ? "Yes, delete this file." : "No, never mind.", 14, hot ? 1.0f : 0.0f, hot ? 1.0f : 0.5f, hot ? 1.0f : 0.5f, true); }
    }
    if (n > 16) { const float th = std::clamp(16.0f * 228 / n, 8.0f, 228.0f); ui::fillRect(781, 147 + app.t2Scroll * 228.0f / n, 6, th, 1, 1, 1, 1); }
    ui::endScreen();
    if (app.t2Go == 1) app.t2Go = 2; else if (app.t2Go == 2) { app.t2Go = 0; t2Ok(app); }
}

// Credits (FUN_0044b9c0)
static void creditsOpen(App& app) {
    t2LoadArt(app);
    app.creditLines.clear();
    std::ifstream f(app.gameDir + "/credits.txt"); std::string line; bool on = false;
    while (std::getline(f, line)) {
        while (!line.empty() && (line.back() == '\r' || line.back() == ' ')) line.pop_back();
        if (!on) { if (line == "#CREDITS") on = true; continue; }
        if (!line.empty() && line[0] == '#') break;
        if (app.creditLines.size() < 512) app.creditLines.push_back(line);
    }
    app.creditStart = SDL_GetTicks(); app.screen = App::ScreenCredits;
    snd(app, "credit.wav", 0.7f);   // DERIVED: the clip is registered by the exe; its trigger is not found
}
static void drawCredits(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.creditsBg, 0, 0);
    const float pitch = 24.0f; const float y0 = 600.0f - (SDL_GetTicks() - app.creditStart) / 30.0f;
    for (size_t i = 0; i < app.creditLines.size(); i++) {
        const float y = y0 + pitch * (float)i; if (y < -40 || y > 610) continue;
        if (app.creditLines[i] == "$bink") { ui::drawImage(app.creditsLogo, 368, y, 0, 0, 64, 64); continue; }
        tText(app, 52, y + 2, app.creditLines[i], 18, 0, 0, 0, false); tText(app, 50, y, app.creditLines[i], 18, 1, 1, 1, false);
    }
    if (y0 + pitch * (float)app.creditLines.size() < -150) { app.screen = App::ScreenMenu; app.hover = -1; }
    ui::endScreen();
}

// Top 10 Designers (FUN_00473470)
static std::string top10Path(const App& app) { return (std::filesystem::path(app.courseFile).has_parent_path() ? std::filesystem::path(app.courseFile).parent_path() : std::filesystem::path(".")).string() + "/top10.sve"; }
static void top10Ensure(App& app) {
    if (app.top10Ready) return;
    app.top10Ready = true;
    if (top10Load(app.top10, top10Path(app)) || top10Load(app.top10, app.gameDir + "/top10.sve")) return;
    sg::SocialRng r((uint64_t)SDL_GetTicks() + 1); app.top10.makeDefault(r);
}
static void top10Submit(App& app) {   // end of year: the club's fun, skill and cash score against the table
    top10Ensure(app);
    sg::DesignerEntry n; std::snprintf(n.name, sizeof n.name, "%s", app.charName.empty() ? "Gary Golf" : app.charName.c_str()); std::snprintf(n.course, sizeof n.course, "%s", app.courseName.c_str());
    n.fun = clubFun(app); n.skill = (int)(clubSkill(app) * 100); n.cash = (int)(app.econ.cash / 100.0); n.difficulty = (int16_t)app.difficulty; n.courseId = app.curProp;
    if (app.top10.insert(n) >= 0) { top10Save(app.top10, top10Path(app)); app.top10Show = true; }
}
static void openTop10(App& app) { if (!app.uiOk) return; t2LoadArt(app); top10Ensure(app); app.top10Return = app.screen; app.screen = App::ScreenTop10; }
static void drawTop10(App& app) {
    top10Ensure(app);
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.t10Blank, 0, 0);
    static const int kRankSlot[10] = {2, 1, 3, 0, 4, 7, 6, 8, 5, 9};
    for (int r = 0; r < 10; r++) {
        const sg::DesignerEntry& e = app.top10.e[r]; const int slot = kRankSlot[r], col = slot % 5, row = slot / 5;
        if (row == 0) ui::drawImage(app.t10Troph, 160.0f * col, 40, 160.0f * col, 40, 160, 304); else ui::drawImage(app.t10Troph, 160.0f * col, 344, 160.0f * col, 344, 160, 256);
        const float x0 = 160.0f * col + 80; const bool isNew = r == app.top10.lastRank;
        const float ny = row == 0 ? 321.0f : 574.0f; float o[3]; c15(0x4206, o);
        tText(app, x0, ny, e.name, 12, 0, 0, 0, true); tText(app, x0, ny + 2, e.name, 12, 0.97f, 0.97f, 0.5f, true);
        tText(app, x0, ny + 1, e.name, 12, isNew ? 1.0f : o[0], isNew ? 1.0f : o[1], isNew ? 1.0f : o[2], true);
        if (isNew) { const float w = app.font.width(e.name, 12) + 4; ui::fillRect(x0 - w / 2, ny + 15, w, 1, 0.97f, 0.97f, 0.1f, 1); ui::fillRect(x0 - w / 2, ny + 14, w, 1, 0.5f, 0.5f, 0.1f, 1); }
        const float p = row == 0 ? 12.0f : 10.0f; const float cy = row == 0 ? 164.0f + 10 * r : 443.0f + 4 * r;
        tText(app, x0, cy, "Cash: " + std::to_string(e.cash), 11, o[0], o[1], o[2], true);
        char sk[32]; std::snprintf(sk, sizeof sk, "Skill: %.2f", e.skill / 100.0); tText(app, x0, cy - p, sk, 11, o[0], o[1], o[2], true);
        const float fy = cy - 2 * p; tText(app, x0, fy, "Fun: " + std::to_string(e.fun), 11, o[0], o[1], o[2], true);
        const float ty = fy - (row == 0 ? 8 + p : 2 + p); float t[3]; c15(0x5288, t);
        tText(app, x0, ty - 1, std::to_string(e.score()), 16, 0, 0, 0, true); tText(app, x0, ty + 1, std::to_string(e.score()), 16, 0.97f, 0.97f, 0.5f, true); tText(app, x0, ty, std::to_string(e.score()), 16, t[0], t[1], t[2], true);
        tText(app, x0, ty - 13, "Total Score (x" + std::to_string(e.difficulty + 1) + ")", 10, o[0], o[1], o[2], true);
        if (r == 0) tText(app, x0, ty - 27, e.course, 10, o[0], o[1], o[2], true);
    }
    ui::endScreen();
}

// Championship play (FUN_0046ddd0): a course saved for championship play is loaded, Pick A Pro chooses the player's record, then the field is drawn and the
// tournament runs at once. Cash is reset to $100,000 and the clock to year 1, month 3 (tick 0x2c00); the difficulty chosen at the title is kept.
static bool champSave(App& app, const std::string& name, std::string& err) {   // "Save Course for Championship": Themes/Championship/<name>.cse
    std::error_code ec; const std::filesystem::path d = std::filesystem::path(app.gameDir) / "Themes" / "Championship"; std::filesystem::create_directories(d, ec);
    return saveGame(app, (d / (name + ".cse")).string(), err);
}
static void champStart(App& app) {
    app.screen = App::ScreenPlay; app.hover = -1; app.t2Hover = -1;
    refreshHoles(app); syncHoleStats(app);
    app.tourney.cancel(); app.tourney.setChampionship(true);
    if (app.holes.empty()) { toastMsg(app, "This course has no complete holes"); app.champ = false; app.screen = App::ScreenMenu; app.tourney.setChampionship(false); return; }
    app.tourney.forceOffer(sgaInput(app));
    runTournament(app);
    app.tourney.setChampionship(false);
}
// Best N Hole Scores (FUN_00455a30). PLACEHOLDER: the key that opens it (F12; the exe uses its Information menu) and the text colours (the exe's are not decoded).
static void openBest(App& app) { if (!app.uiOk) return; t2LoadArt(app); app.top10Return = app.screen; app.screen = App::ScreenBest; app.t2Hover = -1; }
static void drawBest(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::fillRect(0, 0, 800, 600, 0, 0, 0, 0.55f);
    ui::drawImage(app.bestArt, 195, 45, 195, 45, 411, 79);
    tText(app, 413, 59, "Best " + std::to_string(app.holes.size()) + " Hole Scores", 20, 0, 0, 0, true);
    tText(app, 236, 96, "Golfer", 13, 0, 0, 0, false); tText(app, 551, 96, "Score", 13, 0, 0, 0, true);
    const int n = app.best.count;
    for (int i = 0; i < n; i++) {
        const float y = 124.0f + 17 * i;
        ui::drawImage(app.bestArt, 195, y, 195, 224, 411, 17);
        tText(app, 246, y, app.best.name[i], 13, 0, 0, 0, false); tText(app, 551, y, std::to_string(app.best.score[i]), 13, 0, 0, 0, true);
    }
    const float ye = 124.0f + 17 * n;
    ui::drawImage(app.bestArt, 195, ye, 195, 321, 411, 61);
    if (app.t2Hover == 1) ui::drawImage(app.bestArt, 544, ye + 14, 593, 434, 44, 44);
    ui::endScreen();
}
static void t2Ok(App& app) {
    const int n = (int)app.t2Files.size(); const bool pro = app.screen == App::ScreenPro; const bool sel = app.t2Sel >= 0 && app.t2Sel < n;
    if (!sel) return;
    const int h = 100;
        if (h == 100 && sel && pro) {   // Pick A Pro: the chosen record becomes the player's character, then the tournament starts
            sg::CharRec c;
            if (!sg::charLoadFile(app.t2Files[(size_t)app.t2Sel], c)) { toastMsg(app, "Could not read that pro"); return; }
            if (c.head >= cuHeadCount(app)) c.head = 8;
            app.chr = c; app.chrUndo = c; chrSync(app); snd(app, "Interface/Button1.wav"); champStart(app); return;
        }
        if (h == 100 && sel && app.t2Mode == 1) {   // championship course chosen
            const int keep = app.difficulty; std::string err;
            if (!loadGame(app, app.t2Files[(size_t)app.t2Sel], err)) { toastMsg(app, "Could not load: " + err); return; }
            app.difficulty = keep; app.tracker = sg::GoalTracker(keep); app.champ = true; app.t2Mode = 0;
            app.econ.sandbox = false; app.econ.cash = 100000; app.econ.startCash = 100000; app.econ.day = 12; app.econ.version++;
            t2ListFolder(app, "Championship", ".pro"); app.t2Sel = -1; app.t2Scroll = 0; app.t2Confirm = 0; app.t2Hover = -1; app.screen = App::ScreenPro; return;
        }
        if (h == 100 && sel) { std::string err; if (loadGame(app, app.t2Files[(size_t)app.t2Sel], err)) { app.t2Hover = -1; if (app.browsing) { app.browsing = false; std::error_code ec; std::filesystem::remove(browsePath(app), ec); std::filesystem::remove(browsePath(app) + ".thumb", ec); } std::printf("loaded %s\n", app.t2Files[(size_t)app.t2Sel].c_str()); } else toastMsg(app, "Could not load: " + err); return; }
}
static void t2Event(App& app, const SDL_Event& e, bool& running) {
    const int scr = app.screen;
    const bool click = e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT, move = e.type == SDL_MOUSEMOTION;
    const bool key = e.type == SDL_KEYDOWN;
    float vx = 0, vy = 0; if (click || move) { vx = app.view.toVirtualX((click ? e.button.x : e.motion.x) * app.dpi); vy = app.view.toVirtualY((click ? e.button.y : e.motion.y) * app.dpi); }
    auto toMenu = [&] { app.screen = App::ScreenMenu; app.hover = -1; app.t2Hover = -1; };
    if (scr == App::ScreenBest) {
        if (move) { const float ye = 124.0f + 17 * app.best.count; app.t2Hover = vx >= 544 && vx <= 587 && vy >= ye + 14 && vy <= ye + 58 ? 1 : -1; }
        if (key || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = app.top10Return == App::ScreenBest ? App::ScreenPlay : app.top10Return; app.t2Hover = -1; }
        return;
    }
    if (scr == App::ScreenCredits) { if (key || e.type == SDL_MOUSEBUTTONDOWN) toMenu(); return; }
    if (scr == App::ScreenTop10) { if (key || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = app.top10Return == App::ScreenTop10 ? App::ScreenPlay : app.top10Return; app.top10.lastRank = -1; } return; }
    if (scr == App::ScreenDiff) {
        if (key) { toMenu(); return; }
        if (move) app.t2Hover = diffHit(vx, vy);
        if (click) { const int h = diffHit(vx, vy); if (h >= 0 && h < 4) { app.difficulty = h; app.sandboxChoice = app.diffPending == 2; app.screen = App::ScreenProperty; app.hover = -1; snd(app, "Interface/Button1.wav"); } else toMenu(); }
        return;
    }
    if (scr == App::ScreenThemes) {
        const std::vector<std::string> th = themeFolders(app);
        auto hit = [&]() { return std::hypot(vx - 615, vy - 555) < 26 ? 0 : std::hypot(vx - 749, vy - 556) < 24 ? 1 : -1; };
        if (key) { app.themePack = 0; toMenu(); return; }
        if (move) app.t2Hover = hit();
        if (click) {
            const int h = hit();
            if (h == 1) { app.themePack = 0; toMenu(); return; }
            if (h == 0) { toMenu(); return; }
            const int row = (int)((vy - 91) / 32);
            if (vx > 72 && vy >= 91 && row >= 0 && row < (int)th.size()) { const std::string lab = themeLabel(th[(size_t)row]); for (int k = 0; k < 5; k++) if (lab == kThemePacks[k]) app.themePack = k; snd(app, "Interface/Button2.wav"); }
        }
        return;
    }
    if (scr == App::ScreenLoad || scr == App::ScreenPro) {
        const int n = (int)app.t2Files.size(); const bool pro = scr == App::ScreenPro;
        auto leave = [&] {
            app.champ = false; app.t2Mode = 0;
            if (app.browsing) { std::string err; app.browsing = false; if (!loadGame(app, browsePath(app), err)) toastMsg(app, "Could not restore the game"); std::error_code ec; std::filesystem::remove(browsePath(app), ec); std::filesystem::remove(browsePath(app) + ".thumb", ec); app.screen = App::ScreenPlay; app.t2Hover = -1; return; }
            toMenu(); };
        if (app.t2Go) return;
        if (app.t2Confirm && (click || move)) {   // the delete box: option rows at y 138 and 162
            const int r = (vx > 250 && vx < 550 && vy >= 126 && vy < 174) ? (int)((vy - 126) / 24) : -1;
            if (move) app.t2ConfirmHover = r;
            if (click) {
                if (r == 0 && app.t2Sel >= 0 && app.t2Sel < n) { std::error_code ec; std::filesystem::remove(app.t2Files[(size_t)app.t2Sel], ec); std::filesystem::remove(app.t2Files[(size_t)app.t2Sel] + ".thumb", ec); app.t2Sel = -1; if (app.t2Mode == 1) t2ListFolder(app, "Championship", ".cse"); else t2ListSaves(app); }
                app.t2Confirm = 0; app.t2ConfirmHover = -1;
            }
            return;
        }
        if (key) { if (e.key.keysym.sym == SDLK_ESCAPE) { if (app.t2Confirm) app.t2Confirm = 0; else leave(); } return; }
        if (move) app.t2Hover = loadHit(vx, vy);
        if (!click) return;
        const int h = loadHit(vx, vy); const bool sel = app.t2Sel >= 0 && app.t2Sel < n;
        if (h == 101) { leave(); return; }
        if (h == 102) { app.t2Scroll = std::max(0, std::min(app.t2Scroll - 4, std::max(0, n - 16))); return; }
        if (h == 103) { app.t2Scroll = std::max(0, std::min(app.t2Scroll + 4, std::max(0, n - 16))); return; }
        if (h == 100 && sel) { app.t2Go = 1; return; }
        if (h == 104 && sel && pro) return;   // PLACEHOLDER: the exe's delete button on Pick A Pro is not decoded; the pros are disc files, so it does nothing here
        if (h == 104 && sel) { app.t2Confirm = 1; return; }
        if (vx > 320 && vy > 115) { const int row = (int)((vy - 116) / 16) + app.t2Scroll; app.t2Sel = row < n ? row : -1; snd(app, "Interface/Button2.wav"); }
        return;
    }
    (void)running;
}

// ---- Pair selection (FUN_00459850, docs/DECODE_TOP10_PAIR.md section 3). PLACEHOLDER: the exe's call site is unknown, so the screen opens from the ` key with six roster members;
// trait lines use made-up labels (the exe's five trait words are not in the text), the age is a stand-in, and the invalid click sound (id 0x18) is silent here.
static void pairOpen(App& app) {
    if (!app.uiOk) return;
    if (!app.pairBase.tex) { const std::string i = app.gameDir + "/Interface/"; ui::loadPcx(i + "PairBase.pcx", app.pairBase, false); ui::loadPcx(i + "PairButtons.pcx", app.pairBtn, true); }
    app.pairIds.clear(); app.pairSel = 0; app.pairHover = -1; app.pairMsg.clear();
    for (int id = 1; id <= 75 && app.pairIds.size() < 6; id++) {
        const int pick = (id * 7 + (int)(SDL_GetTicks() / 1000)) % 75 + 1; bool dup = false;
        for (int q : app.pairIds) dup |= q == pick;
        if (!dup && !app.roster.e[pick].resigned) app.pairIds.push_back(pick);
    }
    app.top10Return = app.screen; app.screen = App::ScreenPair;
}
static int pairCell(float vx, float vy, size_t n) { if (vx < 6 || vx >= 664 || vy < 50) return -1; const int c = (int)((vx - 6) / 334) + (int)((vy - 50) / 136) * 2; return c >= 0 && c < (int)n ? c : -1; }
static void drawPair(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.pairBase, 0, 0);
    app.font.drawCentered(338, 32, "SELECT THE NEXT PAIR OF GOLFERS", 22, 0, 0, 0);
    static const char* kMar[4] = {"Single", "Married", "Divorced", "Widowed"};
    for (size_t k = 0; k < app.pairIds.size(); k++) {
        const int id = app.pairIds[k]; const float x0 = 329.0f * (float)(k & 1), y = 50.0f + 136 * (float)(k / 2);
        const bool on = app.pairSel >> k & 1, hot = app.pairHover == (int)k;
        const float yy = hot && !on ? y - 1 : y;
        ui::drawImage(app.pairBtn, x0 + 6, yy, 0, on ? 272.0f : 0.0f, 329, 136);
        drawFace(app, id % 3 == 0, (id - 1) % 19, 1, x0, yy + 4, true);
        app.font.drawCentered(x0 + 214, yy + 23, memberName(app, id), 15, 0, 0, 0);
        app.font.drawCentered(x0 + 262, yy + 52, "Golf Pro", 13, 0, 0, 0);
        app.font.drawCentered(x0 + 262, yy + 84, std::to_string(20 + (id * 7) % 35) + " years old", 13, 0, 0, 0);
        app.font.drawCentered(x0 + 262, yy + 116, kMar[id % 4], 13, 0, 0, 0);
        const int tb = (id * 5 + 3) & 31; int cnt = 0; for (int t = 0; t < 5; t++) if (tb >> t & 1) cnt++;
        int line = 0; for (int t = 0; t < 5; t++) if (tb >> t & 1) app.font.draw(x0 + 146, yy + (5 - cnt) * 9 + 48 + 18.0f * line++, kCuTrait[t], 12, 0, 0, 0);
    }
    if (!app.pairMsg.empty()) app.font.drawCentered(338, 560, app.pairMsg, 14, 1, 1, 1);
    if (__builtin_popcount(app.pairSel) == 2) ui::drawImage(app.pairBtn, 693, 502, 693, 502, 75, 75);   // DERIVED: the lit OK shows once a pair is chosen
    ui::endScreen();
}
static void pairEvent(App& app, const SDL_Event& e) {
    float vx = 0, vy = 0; const bool mouse = e.type == SDL_MOUSEMOTION || e.type == SDL_MOUSEBUTTONDOWN;
    if (mouse) { vx = app.view.toVirtualX((e.type == SDL_MOUSEMOTION ? e.motion.x : e.button.x) * app.dpi); vy = app.view.toVirtualY((e.type == SDL_MOUSEMOTION ? e.motion.y : e.button.y) * app.dpi); }
    if (e.type == SDL_MOUSEMOTION) { app.pairHover = pairCell(vx, vy, app.pairIds.size()); return; }
    const bool click = e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT;
    if (!click && e.type != SDL_KEYDOWN) return;
    const int c = click ? pairCell(vx, vy, app.pairIds.size()) : -1;
    if (c >= 0) {
        const int have = __builtin_popcount(app.pairSel);
        if (have < 2 || (app.pairSel >> c & 1)) { app.pairSel ^= 1u << c; snd(app, "Interface/Button2.wav"); }
        return;
    }
    const int have = __builtin_popcount(app.pairSel);   // empty space or a key accepts
    if (have == 1) { app.pairMsg = "Choose two golfers to play together."; return; }
    if (have == 2) {   // PLACEHOLDER: group play is not built, so the pair is only announced
        std::string a, b; for (size_t k = 0; k < app.pairIds.size(); k++) if (app.pairSel >> k & 1) (a.empty() ? a : b) = memberName(app, app.pairIds[k]);
        say(app, a + " and " + b + " will play together", 6);
    }
    app.screen = app.top10Return == App::ScreenPair ? App::ScreenPlay : app.top10Return;
}

// ---- Golfer info card (docs/DECODE_GOLFERCARD.md section 2, layout S). Plate, ball and face, five meters, 18 hole scorecard and five round buttons follow the exe.
// PLACEHOLDER: the text lines (the exe's literals live in its data), the partner layout (needs group play), the shaded backdrop, the shadow sheet and the stats card.
static bool screenshot(const App& app, const char* file);
static bool golferFemale(const Golfer& g) { return g.look >= 2; }
static int golferHead(const Golfer& g) { return g.memberId ? (g.memberId - 1) % 19 : (g.look * 5 + 1) % 19; }   // PLACEHOLDER: the exe's head rule for members is not decoded
static int golferExpr(const Golfer& g) { const int p = g.rx.polarity(); return p == 0 ? 0 : p == 2 ? 2 : 1; }   // newest reaction good, none or bad (FUN_004675d0)
static void openGolferCard(App& app, int gi) {
    if (!app.uiOk || gi < 0 || gi >= (int)app.golfers.size()) return;
    if (!app.cgBtn.tex) loadCharArt(app);
    if (!app.cardArt.tex) { const std::string i = app.gameDir + "/Interface/"; ui::loadPcx(i + "GolferStats.pcx", app.cardArt, false, -1, i + "GolferStats_A.pcx"); }
    app.cardSkills = false; app.cardG = gi; app.cardHover = -1; app.cardFrames = 0; app.screen = App::ScreenGolfer; app.followG = gi;
}
static int pickGolfer(App& app, float wx, float wy) {   // window points; the body is drawn above the ground point
    int best = -1; float bd = 30;
    for (size_t i = 0; i < app.golfers.size(); i++) {
        const Golfer& g = app.golfers[i]; if (!g.active || g.leaving) continue;
        const float gx = g.sim.golferX, gz = g.sim.golferZ, y = app.terrain.heightAt(gx, gz);
        const double ex = app.mv[0] * gx + app.mv[4] * y + app.mv[8] * gz + app.mv[12], ey = app.mv[1] * gx + app.mv[5] * y + app.mv[9] * gz + app.mv[13];
        const float sx = (float)((ex / app.upp + app.drawW * 0.5) / app.dpi), sy = (float)((app.drawH * 0.5 - ey / app.upp) / app.dpi) - 22.0f;
        const float d = std::hypot(sx - wx, sy - wy); if (d < bd) { bd = d; best = (int)i; }
    }
    return best;
}
static const float kCardBtnX[5] = {0x125, 0x166, 0x1a7, 0x1e8, 0x23a}, kCardBtnCx[5] = {0x13e, 0x17f, 0x1c0, 0x201, 0x253};
static const int kCardBtnRow[5] = {0, 2, 3, 4, 5};   // person with list, runner, camera, book, tick on the GolferStats strips
static const char* kCardTip[5] = {"Customize", "Move/Eject Golfer", "Take Snapshot", "View Story", ""};
static int cardHit(float vx, float vy) { for (int b = 0; b < 5; b++) if (std::hypot(vx - kCardBtnCx[b], vy - 0x106) < 25) return b; return -1; }
static bool cardBtnLive(int b) { return b == 1 || b == 2 || b == 4; }   // Customize and View Story have no port action yet, so they are drawn in the pale strip
// Read only skills dialog (FUN_0045f0f0 with x offset -50): a 208 by 316 panel at (28,50), the title white and centred at x 160, ten rows from y 90 in steps of 24,
// names at x 88 (grey when the skill is 0, else black), values "+N%" at x 37 (docs/DECODE_CARDS2.md section 1). PLACEHOLDER: the row strips and panel art (TransPopups and the
// 0x4c1570 sheet cuts are not measured), so the panel is the popup frame and the rows are plain boxes; skills are the port's 0..15 mapped onto the exe's 0..10.
static bool g_skillsTest = false;
static void drawSkillsPanel(App& app, const Golfer& g, const std::string& who) {
    drawFrame9(app, 28, 50, 208, 316);
    app.font.drawCentered(132, 58 + 14, who + "'s skills", 14, 1, 1, 1);
    static const char* kSk[10] = {"Power Hitter", "Long Driver", "Accurate Driver", "Accurate Irons", "Accurate Putter", "Draw Shot (R to L)", "Fade Shot (L to R)", "High Backspin Shot", "Recovery Skills", "Luck"};
    for (int i = 0; i < 10; i++) {
        const float y = 90.0f + 24 * i; const int pts = (g.sim.skills.v[i] * 10 + 7) / 15;
        ui::fillRect(80, y + 2, 150, 20, 0.95f, 0.92f, 0.82f, 1);
        const float c = pts > 0 ? 0.0f : 0.5f;
        app.font.draw(88, y + 7 + 11, kSk[i], 12, c, c, c);
        if (pts > 0) { if (pts < 10) drawPlusPct(app, 37, y + 7 + 11, pts * 10, 12, 1, 1, 1, false); else app.font.draw(37, y + 7 + 11, "100%", 12, 1, 1, 1); }
    }
}
// ---- Editable skills dialog (docs/DECODE_CARDS2.md section 1, FUN_0045f0f0 with X = 200): the player's own skills, spend the points left ----
static const char* kSkName[10] = {"Power Hitter", "Long Driver", "Accurate Driver", "Accurate Irons", "Accurate Putter", "Draw Shot (R to L)", "Fade Shot (L to R)", "High Backspin Shot", "Recovery Skills", "Luck"};
static void skillsOpen(App& app, int points) {
    if (points <= 0) return;
    app.skOpen = true; app.skConfirm = false; app.skLeft = points; app.skHover = -1; app.skConfHover = -1;
    for (int i = 0; i < 10; i++) app.skStart[i] = app.chr.skills[i];
    app.skWasPaused = app.paused; app.paused = true;
}
static void skillsClose(App& app) {
    app.skOpen = false; app.skConfirm = false; app.paused = app.skWasPaused; app.chr.flags |= 0x80;
    for (int k = 0; k < 10; k++) app.skills.v[k] = std::min(15, (int)app.chr.skills[k] * 15 / 10);   // the player's tournament skills follow the points spent (a point is 10 percent, a skill value is 0..15)
}
static const float kSkX = 200.0f;
// Draws one frame of a golfer sprite on the 2D screen, ground point at (x, y), scaled so the frame is `hgt` pixels tall (docs/DECODE_BODIES.md; the dialog shows the standing pose).
static void drawBodyUi(GlSprite* sp, int view, int frame, float x, float y, float hgt) {
    if (!sp) return;
    const GLuint tex = spriteTexture(*sp, view, frame);
    const float sc = hgt / (float)sp->s.h, w = (float)sp->s.w * sc, h = (float)sp->s.h * sc;
    const float dx = x - (float)sp->s.anchorX * sc, dy = y - (float)sp->s.anchorY * sc;
    glEnable(GL_TEXTURE_2D); glBindTexture(GL_TEXTURE_2D, tex); glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE); glColor4f(1, 1, 1, 1);
    glBegin(GL_QUADS); glTexCoord2f(0, 0); glVertex2f(dx, dy); glTexCoord2f(1, 0); glVertex2f(dx + w, dy); glTexCoord2f(1, 1); glVertex2f(dx + w, dy + h); glTexCoord2f(0, 1); glVertex2f(dx, dy + h); glEnd();
}
// The standing portrait body (Bodies/<set>.pcx, 60 x 120, docs/DECODE_BODIES.md section 4): the sheet's own palette index of each pixel is looked up in the composed
// swap palette, so shirt, pants, skin and hat follow the character. Cached per look.
static ui::Image* bodyPortrait(App& app, const sg::BodyLook& l) {
    static std::map<std::string, ui::Image> cache;
    const std::string key = l.key(); auto it = cache.find(key);
    if (it != cache.end()) return it->second.tex ? &it->second : nullptr;
    ui::Image& im = cache[key];
    uint8_t pal[768], own[768]; sg::Bytes d; sg::Rgba rgb; std::string err;
    if (sg::composeBodyPalette(app.gameDir, l, pal) && sg::readFile(app.gameDir + "/Bodies/" + sg::bodySetName(l) + ".pcx", d) && sg::readPcxPalette(d, own) && sg::decodePcx(d, rgb, err)) {
        std::map<unsigned, int> idx; for (int i = 255; i >= 0; i--) idx[(unsigned)own[i * 3] << 16 | (unsigned)own[i * 3 + 1] << 8 | own[i * 3 + 2]] = i;
        std::vector<unsigned char> out(rgb.px.size());
        for (size_t i = 0; i + 3 < rgb.px.size(); i += 4) {
            const unsigned c = (unsigned)rgb.px[i] << 16 | (unsigned)rgb.px[i + 1] << 8 | rgb.px[i + 2]; const int k = idx.count(c) ? idx[c] : 255;
            if (k == 255 || c == 0xff00ff) { out[i] = out[i + 1] = out[i + 2] = out[i + 3] = 0; }
            else { out[i] = pal[k * 3]; out[i + 1] = pal[k * 3 + 1]; out[i + 2] = pal[k * 3 + 2]; out[i + 3] = 255; }
        }
        ui::uploadRgba(out.data(), (int)rgb.w, (int)rgb.h, im);
    }
    return im.tex ? &im : nullptr;
}
static sg::BodyLook chrLook(const App& app) {
    sg::BodyLook l; l.female = app.chr.female(); l.body = l.female ? 4 + app.chr.bodyType() : app.chr.bodyType();
    l.shirt = app.chr.shirt; l.pants = app.chr.pants; l.skin = app.chr.skin; l.hat = app.chr.bodyHat & 15; l.hair = app.chr.hair; l.altSkin = app.chr.altSkin;
    return l;
}
static void drawRoundBox(float x, float y, float w, float h, float r, float g, float b) {   // a pill, one scanline at a time so there are no seams
    const float rad = h * 0.5f;
    for (int j = 0; j < (int)h; j++) { const float dy = rad - (float)j - 0.5f, inset = rad - std::sqrt(std::max(0.0f, rad * rad - dy * dy)); ui::fillRect(x + inset, y + (float)j, w - 2 * inset, 1, r, g, b, 1); }
}
// The editable skills dialog as in the real screenshot: black panel with a lavender edge, white title, a yellow "Add N skill points." line, ten rows (lavender
// toggle oval, cream value oval with "+N0%", cream name box) and a standing portrait on the right. Row shapes and colours are measured by eye (PLACEHOLDER).
static void drawSkillsEdit(App& app) {
    if (!app.skOpen) return;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    const float X = kSkX, px = X + 0x2e, py = 50, pw = 320, ph = 316;
    ui::fillRect(px - 2, py - 2, pw + 4, ph + 4, 0.62f, 0.6f, 0.86f, 1); ui::fillRect(px, py, pw, ph, 0.03f, 0.03f, 0.05f, 1);
    app.font.drawCentered(X + 0xd2, 58 + 14, app.charName, 16, 1, 1, 1);
    { const std::string t = "Add " + std::to_string(app.skLeft) + " skill point" + (app.skLeft == 1 ? "" : "s") + "."; app.font.drawCentered(X + 0xd2, 80 + 10, t, 13, 1.0f, 0.95f, 0.35f); }
    // portrait: navy sky with a white oval, grass at the foot, the standing figure
    { const float qx = X + 0xf7 + 21, qy = 96, qw = 92, qh = 216;
      ui::fillRect(qx, qy, qw, qh, 0.1f, 0.1f, 0.33f, 1);
      { const float ecx = qx + qw * 0.5f, ecy = qy + qh * 0.5f - 16, erx = qw * 0.5f + 2, ery = qh * 0.5f - 12;   // the white oval behind the figure
        for (int j = 0; j < (int)(2 * ery); j++) { const float dy = ((float)j + 0.5f - ery) / ery, hw = erx * std::sqrt(std::max(0.0f, 1 - dy * dy)); ui::fillRect(std::max(qx, ecx - hw), ecy - ery + (float)j, std::min(qx + qw, ecx + hw) - std::max(qx, ecx - hw), 1, 0.97f, 0.97f, 0.98f, 1); } }
      ui::fillRect(qx, qy + qh - 36, qw, 36, 0.35f, 0.55f, 0.2f, 1);
      const sg::BodyLook l = chrLook(app); ui::Image* bi = bodyPortrait(app, l);
      const float hx = qx + qw * 0.5f - 45, hy = qy + 6;   // head cell 90 x 120, body 60 x 120 placed 19 px right and 96 px below the head origin (the Customise preview offsets)
      if (bi) ui::drawImage(*bi, hx + 19, hy + 96 - 18, 0, 0, 60, 120);
      drawFace(app, app.chr.female(), app.chr.head, 1, hx, hy - 18, false); }
    for (int i = 0; i < 10; i++) {
        const float y = 90.0f + 24 * i; const int v = app.chr.skills[i];
        const bool hu = app.skHover == i, hd = app.skHover == i + 10;
        drawRoundBox(X + 0x32, y + 1, 35, 21, 0.62f, 0.58f, 0.86f);
        // the toggle holds a small double arrow; the half under the cursor lights up
        const float tc = X + 0x32 + 17.5f;
        const float up[3] = {hu ? 1.0f : 0.85f, hu ? 1.0f : 0.78f, hu ? 0.4f : 0.3f}, dn[3] = {hd ? 1.0f : 0.85f, hd ? 1.0f : 0.78f, hd ? 0.4f : 0.3f};
        for (int k = 0; k < 4; k++) { ui::fillRect(tc - 1 - k, y + 4 + k, 2 + 2 * k, 1, up[0], up[1], up[2], 1); ui::fillRect(tc - 1 - (3 - k), y + 13 + k, 2 + 2 * (3 - k), 1, dn[0], dn[1], dn[2], 1); }
        drawRoundBox(X + 0x52 + 1, y + 1, 46, 21, 0.97f, 0.96f, 0.9f);
        if (v > 0) { if (v < 10) drawPlusPct(app, X + 0x57 + 1, y + 7 + 11, v * 10, 12, 0, 0, 0, false); else app.font.draw(X + 0x57 + 4, y + 7 + 11, "100%", 12, 0, 0, 0); }
        drawRoundBox(X + 0x8a - 5, y + 1, 139, 21, 0.97f, 0.96f, 0.9f);
        const float c = v > 0 ? 0.0f : 0.55f;
        app.font.draw(X + 0x8a, y + 7 + 11, kSkName[i], 12, c, c, c);
    }
    { ui::fillRect(193, 364, 414, 94, 0.62f, 0.6f, 0.86f, 0.9f); ui::fillRect(195, 366, 410, 90, 0.12f, 0.2f, 0.12f, 0.94f);   // the explanation box under the dialog
      static const char* kT = "Before you play your course you may customize your character by improving his or her golf skills. You can also win additional skill points for each accomplishment added to your trophy.";
      float ty = 384; for (const std::string& ln : wrapText(app, kT, 13, 392)) { app.font.draw(204, ty, ln, 13, 0.98f, 0.95f, 0.8f); ty += 17; } }
    if (app.okRound.tex) { const auto& k = sg::ui_screens::kOkCut[(app.skHover == 100) ? 1 : 0]; ui::drawImage(app.okRound, X + 0x146 + 4, 0x13e - 6, (float)k.x, (float)k.y, (float)k.w, (float)k.h); }
    if (app.skConfirm) {   // modal confirm (FUN_0046d6e0 400 x 200): points are still unspent
        drawFrame9(app, 200, 200, 400, 128);
        app.font.drawCentered(400, 232, "You haven't used all your skill points.", 14, 1, 1, 1);
        app.font.drawCentered(400, 252, "Do you want to go on without them?", 13, 1, 1, 1);
        static const char* kB[2] = {"Yea, I don't need no stinkin' skill points", "No, let me finish"};
        for (int i = 0; i < 2; i++) { const bool h = app.skConfHover == i; ui::fillRect(214, 264.0f + 28 * i, 372, 22, h ? 1.0f : 0.28f, h ? 1.0f : 0.25f, h ? 1.0f : 0.5f, 1); app.font.drawCentered(400, 264.0f + 28 * i + 16, kB[i], 12, h ? 0.1f : 1, h ? 0.1f : 1, h ? 0.3f : 1); }
    }
    ui::endScreen();
}
static bool skillsEvent(App& app, const SDL_Event& e) {
    if (!app.skOpen) return false;
    const bool mouse = e.type == SDL_MOUSEMOTION || e.type == SDL_MOUSEBUTTONDOWN;
    float vx = 0, vy = 0;
    if (mouse) { vx = app.view.toVirtualX((e.type == SDL_MOUSEMOTION ? e.motion.x : e.button.x) * app.dpi); vy = app.view.toVirtualY((e.type == SDL_MOUSEMOTION ? e.motion.y : e.button.y) * app.dpi); }
    const float X = kSkX;
    if (app.skConfirm) {
        int h = -1; for (int i = 0; i < 2; i++) if (vx >= 214 && vx < 586 && vy >= 264.0f + 28 * i && vy < 286.0f + 28 * i) h = i;
        if (e.type == SDL_MOUSEMOTION) { app.skConfHover = h; return true; }
        if (e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT) { if (h == 0) skillsClose(app); else if (h == 1) app.skConfirm = false; return true; }
        if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_ESCAPE) { app.skConfirm = false; return true; }
        return e.type == SDL_MOUSEBUTTONUP || e.type == SDL_KEYDOWN || e.type == SDL_TEXTINPUT;
    }
    auto hit = [&]() { if (std::fabs(vx - (X + 0x15e)) < 20 && std::fabs(vy - 0x14e) < 20) return 100;
        if (vx >= X + 0x32 && vx < X + 0x52) for (int r = 0; r < 10; r++) { const float y = 90.0f + 24 * r; if (vy > y && vy <= y + 12) return r; if (vy > y + 12 && vy <= y + 24) return r + 10; }
        return -1; };
    if (e.type == SDL_MOUSEMOTION) { app.skHover = hit(); return true; }
    if (e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT) {
        const int h = hit();
        if (h == 100) { if (app.skLeft > 0) { app.skConfirm = true; app.skConfHover = -1; } else skillsClose(app); snd(app, "Interface/Button1.wav"); }
        else if (h >= 0 && h < 10) {   // add: needs a free point and a skill below 10
            if (app.skLeft > 0 && app.chr.skills[h] < 10) { app.chr.skills[h]++; app.skLeft--; snd(app, "Interface/Button1.wav"); } else snd(app, "Interface/Button2.wav");
        } else if (h >= 10 && h < 20) {   // refund: only down to the value the dialog started with
            const int r = h - 10; if (app.chr.skills[r] > app.skStart[r]) { app.chr.skills[r]--; app.skLeft++; snd(app, "Interface/Button1.wav"); } else snd(app, "Interface/Button2.wav");
        }
        return true;
    }
    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_RETURN)) { if (app.skLeft > 0) app.skConfirm = true; else skillsClose(app); return true; }
    return e.type == SDL_MOUSEBUTTONUP || e.type == SDL_KEYDOWN || e.type == SDL_TEXTINPUT || e.type == SDL_MOUSEWHEEL;
}
static void drawGolferCard(App& app) {
    if (app.cardG < 0 || app.cardG >= (int)app.golfers.size()) { app.screen = App::ScreenPlay; return; }
    const Golfer& g = app.golfers[(size_t)app.cardG];
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    ui::drawImage(app.cardArt, 236, 26, 64, 33, 402, 244);   // the plate (sheet cut measured on the art)
    ui::drawImage(app.ballArt, 172, -2, 0, 300, 140, 140);
    drawFace(app, golferFemale(g), golferHead(g), golferExpr(g), 172, -2, true);
    if (app.cardSkills) drawSkillsPanel(app, g, g.memberId ? memberName(app, g.memberId) : std::string("Visiting golfer"));
    const float cx = 420;
    const std::string nm = g.memberId ? memberName(app, g.memberId) : std::string("Visiting golfer");
    app.font.drawCentered(cx, 0x28 + 6, nm, 17, 0.1f, 0.08f, 0.3f);
    static const char* kTier[6] = {"", "Visitor", "Member", "Silver member", "Gold member", "Platinum member"};
    const int tier = g.memberId ? (int)app.roster.e[g.memberId].tier : 1;
    app.font.drawCentered(cx, 0x3e + 6, kTier[std::clamp(tier, 0, 5)], 13, 0.1f, 0.08f, 0.3f);
    if (g.memberId) { const auto& en = app.roster.e[g.memberId]; app.font.drawCentered(cx, 0x4b + 6, "Rounds " + std::to_string(en.rounds) + "   Best " + (en.low ? std::to_string(en.low) : std::string("-")) + "   Hcp " + std::to_string(en.hcp), 13, 0.1f, 0.08f, 0.3f); }
    app.font.drawCentered(cx, 0x5b + 6, "Hole " + std::to_string(g.hole + 1) + "   Strokes " + std::to_string(g.sim.stroke), 13, 0.3f, 0.25f, 0.1f);
    if (g.lastEvent) app.font.drawCentered(cx, 0x69 + 6, g.lastEvent, 12, 0.1f, 0.08f, 0.3f);
    // Meters: a bright bar of the full width, then the dark remainder from the right edge (exe widths w, 0 to 80).
    auto cl = [](int v) { return std::clamp(v, 0, 0x50); };
    int good = 0, bad = 0; for (int k = 0; k < 5; k++) { const int l = g.rx.histLoc[k]; if (!g.rx.hist[k]) continue; if ((l & 0xc000) == 0xc000) bad++; else if (l & sg::kLocGood) good++; }
    const int w[5] = {cl((8 - g.mood) * 10), cl((4 - (good - bad)) * 10), cl(g.rx.fatigue / 4), cl(g.rx.hunger * 5 / 2), cl(g.rx.thirst * 5 / 2)};
    static const char* kMeter[5] = {"Mood", "Attitude", "Energy", "Hunger", "Thirst"};
    for (int m = 0; m < 5; m++) {
        const float y = (float)(0x36 + 0x10 * m); const bool hi = w[m] > 0x28;
        app.font.drawCentered(0x24a, y - 4, kMeter[m], 10, 0.1f, 0.08f, 0.3f);
        ui::fillRect(0x222, y, 80, 4, hi ? 1.0f : 0.14f, hi ? 0.26f : 0.9f, hi ? 0.26f : 0.14f, 1);
        ui::fillRect((float)(0x272 - w[m]), y, (float)w[m], 4, hi ? 0.77f : 0.13f, hi ? 0.0f : 0.65f, hi ? 0.0f : 0.13f, 1);
    }
    // Scorecard strip: hole numbers and strokes, par relative colours, running total.
    int total = 0;
    for (int i = 1; i <= 18; i++) {
        const float x = (float)(0x10a + 0x12 * (i - 1)) + 0.5f;
        app.font.drawCentered(x, 0x84 + 5, std::to_string(i), 10, 0, 0, 0);
        if (i > (int)app.holes.size()) continue;
        int s = g.holeStrokes[i - 1]; const int par = app.holes[(size_t)(i - 1)].par; float r = 0, gr = 0, b = 0;
        if (i - 1 == g.hole && !s && g.sim.stroke > 0) { s = g.sim.stroke; r = gr = b = 0.52f; }   // the hole in progress in grey
        else if (s) { if (s < par) { r = 0.97f; gr = 0.26f; b = 0.26f; } if (s < par - 1) { r = 1.0f; gr = 0.9f; b = 0.0f; } if (s > par) { r = 0.1f; gr = 0.4f; b = 1.0f; } if (s > par + 1) { r = 0.0f; gr = 0.0f; b = 0.5f; } }
        if (s) { app.font.drawCentered(x, 0x94 + 6, std::to_string(s), 14, r, gr, b); total += s; }
    }
    app.font.drawCentered(0x256 + 1, 0x94 + 6, std::to_string(total), 14, 0, 0, 0);
    // Round buttons: strip 0 normal, strip 1 pale (no action in the port), strip 2 hover.
    for (int b = 0; b < 5; b++) {
        const int strip = !cardBtnLive(b) ? 1 : app.cardHover == b ? 2 : 0;
        ui::drawImage(app.cardArt, kCardBtnX[b], 0xed, 498.0f + 100 * strip, 50.0f * kCardBtnRow[b], 60, 50);
    }
    if (app.cardHover >= 0 && app.cardFrames > 7 && kCardTip[app.cardHover][0]) {
        const float tw = app.font.width(kCardTip[app.cardHover], 12) + 12;
        ui::fillRect(app.cardMx + 12, app.cardMy + 8, tw, 18, 1, 1, 0.8f, 0.95f); app.font.draw(app.cardMx + 18, app.cardMy + 21, kCardTip[app.cardHover], 12, 0, 0, 0);
    }
    ui::endScreen();
}
static void golferCardEvent(App& app, const SDL_Event& e) {
    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_RETURN)) { app.screen = App::ScreenPlay; app.cardG = -1; app.cardSkills = false; return; }
    if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_s) { app.cardSkills = !app.cardSkills; return; }   // PLACEHOLDER trigger: the exe shows the dialog from the card when a slot flag is set
    if (e.type != SDL_MOUSEMOTION && e.type != SDL_MOUSEBUTTONDOWN) return;
    const bool click = e.type == SDL_MOUSEBUTTONDOWN;
    const float vx = app.view.toVirtualX((click ? e.button.x : e.motion.x) * app.dpi), vy = app.view.toVirtualY((click ? e.button.y : e.motion.y) * app.dpi);
    app.cardMx = vx; app.cardMy = vy;
    const int h = cardHit(vx, vy);
    if (h != app.cardHover) { app.cardHover = h; app.cardFrames = 0; }
    if (!click || e.button.button != SDL_BUTTON_LEFT) return;
    if (h < 0) { app.screen = App::ScreenPlay; app.cardG = -1; return; }   // a click elsewhere puts the card away
    if (!cardBtnLive(h)) return;
    snd(app, "Interface/Button1.wav");
    if (h == 2) { if (screenshot(app, "simgolf-shot.png")) toastMsg(app, "Snapshot saved"); }
    else if (h == 1) { const size_t gi = (size_t)app.cardG; app.screen = App::ScreenPlay; app.cardG = -1; if (app.golfers[gi].active) startLeaving(app, gi); }   // PLACEHOLDER: ejecting is filed like a quit
    else if (h == 4) { app.screen = App::ScreenPlay; app.cardG = -1; }
}
static void worldTag(App& app, float gx, float gz, const std::string& text) {
    const float y = app.terrain.heightAt(gx, gz);
    const double ex = app.mv[0] * gx + app.mv[4] * y + app.mv[8] * gz + app.mv[12], ey = app.mv[1] * gx + app.mv[5] * y + app.mv[9] * gz + app.mv[13];
    const float px = (float)(ex / app.upp + app.drawW * 0.5), py = (float)(app.drawH * 0.5 - ey / app.upp);   // drawable pixels
    const float vx = app.view.toVirtualX(px), vy = app.view.toVirtualY(py);
    if (vx < 0 || vx > 800 || vy < 20 || vy > 570) return;
    app.font.drawCentered(vx + 1, vy + 11, text, 11, 0, 0, 0);
    app.font.drawCentered(vx, vy + 10, text, 11, 1, 1, 1);
}
static void drawNameTags(App& app) {
    if (!app.showNames || app.screen != App::ScreenPlay) return;
    for (size_t i = 0; i < app.golfers.size(); i++) {
        const Golfer& g = app.golfers[i]; if (!g.active || g.leaving) continue;
        std::string nm = g.isPlayer ? app.charName : g.proIdx >= 0 ? app.pros[(size_t)g.proIdx].name : g.memberId ? memberName(app, g.memberId) : std::string();
        const size_t sp = nm.find(' '); if (!g.isPlayer && g.proIdx < 0 && sp != std::string::npos) nm = nm.substr(0, sp);   // members show a first name
        if (!nm.empty()) worldTag(app, g.sim.golferX, g.sim.golferZ, nm);
    }
    for (const App::Emp& e : app.emps) {
        const std::string job = e.skilled ? up::kStaffKinds[e.kind].skilledName : up::kStaffKinds[e.kind].name;
        worldTag(app, e.x, e.z, e.name.empty() ? job : e.name + " " + job);
    }
}
static void drawHud(App& app) {
    if (!app.uiOk) return;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    drawNameTags(app);
    // The exe's date stamp routine counts months in blocks of 1024 ticks and shows month numbers 3..10, so a year here is eight months,
    // March to October (medium confidence; the start year 2001 is a placeholder).
    const int mi = app.econ.day - 1;
    char date[48]; std::snprintf(date, sizeof date, "%s %d", kMonths[(2 + mi % 8) % 12], 2001 + mi / 8);
    // The four HUD pills are cuts of Interface/courseinfo.pcx (+ _A alpha), drawn at their own sheet positions (FUN_00442180 loader; DERIVED placement).
    if (!app.ciArt.tex) { const std::string i = app.gameDir + "/Interface/"; ui::loadPcx(i + "courseinfo.pcx", app.ciArt, false, -1, i + "courseinfo_A.pcx"); ui::loadShade(i + "s_courseinfo.pcx", app.ciShade, 0.55f); }
    if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 20, app.difficulty)); }
    ui::drawImage(app.ciShade, 48, 6, 48, 6, 183, 58); ui::drawImage(app.ciShade, 647, 13, 647, 13, 138, 36); ui::drawImage(app.ciShade, 675, 55, 675, 55, 110, 37); ui::drawImage(app.ciShade, 697, 99, 697, 99, 88, 36);
    ui::drawImage(app.ciArt, 48, 6, 48, 6, 183, 58); ui::drawImage(app.ciArt, 647, 13, 647, 13, 138, 36); ui::drawImage(app.ciArt, 675, 55, 675, 55, 110, 37); ui::drawImage(app.ciArt, 697, 99, 697, 99, 88, 36);
    const bool neg = app.econ.cash < 0 && !app.econ.sandbox;
    app.font.drawCentered(140, 23, app.courseName, 15, 1, 1, 1);
    app.font.drawCentered(140, 37, date, 12, 1, 1, 1);
    // Rating row and hole flags (measured on the real screenshots, DERIVED): golf ball dots, then one star per course grade step (grade + 1, from the exe's grade table), then a heart
    // per happy ending; the row is 146 px wide and spaced evenly (8 px for 18 slots, at most 12.4). The number of dots is a PLACEHOLDER (about 1.5 per hole, capped at 18 slots in all).
    // Below it one flag per hole up to six, otherwise a flag and "x N".
    if (!app.shArt.tex) { const std::string i = app.gameDir + "/Interface/"; ui::loadPcx(i + "StarsHeartsETC.pcx", app.shArt, false, 0x00ff00); }
    { const int nh = (int)app.holes.size(); const int g = sg::courseGrade(nh); const int stars = nh == 0 ? 0 : g < 0 ? 4 : g + 1; const int hearts = std::min(app.hearts, 3);
      const int balls = std::clamp(nh * 3 / 2, 0, 18 - stars - hearts), n = balls + stars + hearts;
      if (n > 0 && app.shArt.tex) {
          const float step = std::min(12.4f, 146.0f / (float)n);
          for (int k = 0; k < n; k++) { const int cell = k < balls ? 3 : k < balls + stars ? 2 : 0; ui::drawImage(app.shArt, 79.5f + step * (float)k - 8, 32, 16.0f * cell, 0, 16, 20); }
      }
      auto flag = [&](float fx, float fy) { ui::fillRect(fx, fy, 1, 9, 0.9f, 0.9f, 0.9f, 1); ui::fillRect(fx + 1, fy, 5, 4, 0.85f, 0.25f, 0.25f, 1); ui::fillRect(fx - 2, fy + 9, 5, 1, 0.9f, 0.9f, 0.9f, 1); };
      if (nh >= 1 && nh <= 6) { const float x0 = 139.5f - 6.0f * (float)nh + 3; for (int k = 0; k < nh; k++) flag(x0 + 12.0f * (float)k, 48); }
      else if (nh > 6) { const std::string t = "x" + std::to_string(nh); const float w = app.font.width(t, 12) + 14; flag(139.5f - w / 2 + 3, 48); app.font.draw(139.5f - w / 2 + 14, 58, t, 12, 1, 1, 1); } }
    if (app.paused && !app.skOpen) { app.font.drawCentered(401, 13, "Paused", 13, 0, 0, 0); app.font.drawCentered(400, 12, "Paused", 13, 1, 1, 0.9f); }
    { const std::string m = app.econ.sandbox ? "Sandbox" : money((long long)app.econ.cash); app.font.draw(750 - 8 - app.font.width(m, 17) - 26 + 18, 37, m, 17, neg ? 1.0f : 0.5f, neg ? 0.4f : 1.0f, neg ? 0.4f : 0.2f); }
    { char f[24]; std::snprintf(f, sizeof f, "%d", clubFun(app)); app.font.draw(762 - app.font.width(f, 16) - 22 + 6 - 6, 80, f, 16, 1, 0.92f, 0.3f); }
    { char f[24]; std::snprintf(f, sizeof f, "%.2f", clubSkill(app)); app.font.draw(783 - app.font.width(f, 16) - 22 - 6, 123, f, 16, 0.35f, 0.92f, 0.95f); }
    if (app.curProp >= 0) drawEmblem(app, app.curProp, 2, 0, 76);   // PLACEHOLDER placement: left of the course pill as in the real HUD screenshots
    drawHomePreview(app);
    drawDockUi(app);
    drawHireDialog(app, app.testHx >= 0 ? app.testHx : app.vmx, app.testHx >= 0 ? app.testHy : app.vmy);
    drawPopup(app);
    { static bool once = false; if (!once && g_skillsTest) { once = true; skillsOpen(app, 10); } }
    if (app.anHole >= 0 && app.anHole < (int)app.allHoles.size()) {   // the Shot Analysis panel: dark translucent, lavender edge (real screenshot)
        static const float col[4][3] = {{1.0f, 0.95f, 0.3f}, {0.4f, 1.0f, 0.45f}, {0.4f, 0.9f, 1.0f}, {1.0f, 1.0f, 0.9f}};
        static const char* kL[4] = {"golfers with ALL skills", "no Imagination skill", "no Accuracy skill", "no Length skill"};
        const float px = 232, py = 18, pw = 262, ph = 128;
        ui::fillRect(px - 2, py - 2, pw + 4, ph + 4, 0.62f, 0.6f, 0.86f, 0.9f); ui::fillRect(px, py, pw, ph, 0.08f, 0.14f, 0.12f, 0.88f);
        app.font.drawCentered(px + pw / 2, py + 20, "Shot Analysis:", 15, 0.4f, 0.95f, 0.9f);
        app.font.drawCentered(px + pw / 2, py + 35, "Golfers playing hole " + std::to_string(app.anHole + 1) + "...", 11, 0.4f, 0.95f, 0.9f);
        for (int v = 0; v < 4; v++) {
            const float y = py + 58 + 17.0f * (float)v;
            app.font.draw(px + 12, y, kL[v], 13, col[v][0], col[v][1], col[v][2]);
            if (v > 0) { char b[32]; std::snprintf(b, sizeof b, "%+d yds.", (int)std::lround(app.anYds[v])); app.font.draw(px + pw - 12 - app.font.width(b, 12), y, b, 12, 0.55f, 0.6f, 0.6f); }
        }
    }
    drawSkillsEdit(app);
    if (app.econ.gameOver) {
        ui::fillRect(200, 250, 400, 80, 0.5f, 0.05f, 0.05f, 0.9f);
        app.font.drawCentered(400, 300, "GAME OVER", 40, 1, 1, 1);
    }
    ui::endScreen();
}

// ---- Course editing ----------------------------------------------------------------------------------------

static void setTitle(App& app, SDL_Window* win) {
    char t[320];
    char cash[160]; std::snprintf(cash, sizeof cash, "%s, month %d%s", app.econ.sandbox ? "sandbox, unlimited funds" : (std::string("$") + std::to_string((long long)app.econ.cash)).c_str(), app.econ.day, app.econ.gameOver ? ", GAME OVER" : "");
    { char extra[96]; std::snprintf(extra, sizeof extra, " | golfers %d, holes %zu | fun %.0f (%s) | staff %d", golfersOnCourse(app), app.holes.size(), app.econ.fun, app.econ.attitude(), app.econ.staffCount()); std::strncat(cash, extra, sizeof cash - std::strlen(cash) - 1); }
    if (app.edit) std::snprintf(t, sizeof t, "SimGolf native: EDIT | %s | %s | brush %d | %s | Tab exit, T tool, [ ] type, , . brush, F5 save, F9 load, shift=lower",
                                app.tool == 0 ? "Paint" : app.tool == 1 ? "Raise/Lower" : app.tool == 2 ? "Path" : app.tool == 3 ? "Wall" : "Building", app.tool == 0 ? kPaint[app.paintIdx].name : app.tool == 2 ? (app.pathKind == 1 ? "gravel" : "paved") : app.tool == 3 ? "edge" : app.tool == 4 ? kBuild[app.buildIdx].name : "terrain", app.brush * 2 + 1, cash);
    else std::snprintf(t, sizeof t, "SimGolf native: %s | %s | Tab = edit course, M music, N mute", kThemes[app.theme], cash);
    SDL_SetWindowTitle(win, t);
}

// Ground point under a window position: ortho ray against the heightfield (a few fixed-point iterations).
static bool pickGround(App& app, int mx, int my, float dpi, float& wx, float& wz) {
    const GLdouble* m = app.mv;
    double ex = (mx * dpi - app.drawW * 0.5) * app.upp, ey = (app.drawH * 0.5 - my * dpi) * app.upp;
    double v[3] = {ex - m[12], ey - m[13], 0 - m[14]};
    double o[3], d[3];
    for (int j = 0; j < 3; j++) { o[j] = m[4 * j] * v[0] + m[4 * j + 1] * v[1] + m[4 * j + 2] * v[2]; d[j] = -m[4 * j + 2]; }
    if (std::fabs(d[1]) < 1e-6) return false;
    // March from above the highest possible ground down along the ray, then bisect the first crossing.
    const double top = Terrain::kMaxLevel * kHeightStep + 1.0;
    auto at = [&](double s, double& x, double& z) { x = o[0] + d[0] * s; z = o[2] + d[2] * s; return o[1] + d[1] * s - app.terrain.heightAt((float)x, (float)z); };
    double s0 = (top - o[1]) / d[1], s1 = s0, x = 0, z = 0;
    const double span = top / std::fabs(d[1]);
    const int steps = 120;
    double prev = at(s0, x, z);
    bool hit = false;
    for (int i = 1; i <= steps; i++) {
        s1 = s0 + span * i / steps;
        double v = at(s1, x, z);
        if (v <= 0) { hit = true; break; }
        s0 = s1; prev = v;
    }
    (void)prev;
    if (!hit) return false;
    for (int i = 0; i < 24; i++) { double m = 0.5 * (s0 + s1); if (at(m, x, z) > 0) s0 = m; else s1 = m; }
    at(s1, x, z);
    const float half = kTileSize;
    if (std::fabs(x) > app.terrain.w * half * 0.5 || std::fabs(z) > app.terrain.h * half * 0.5) return false;
    wx = (float)x; wz = (float)z;
    return true;
}

static void tileOf(const App& a, float wx, float wz, int& tx, int& ty) {
    tx = (int)std::floor((wx + a.terrain.w * kTileSize * 0.5f) / kTileSize);
    ty = (int)std::floor((wz + a.terrain.h * kTileSize * 0.5f) / kTileSize);
}
static void cornerOf(const App& a, float wx, float wz, int& cx, int& cy) {
    cx = (int)std::lround((wx + a.terrain.w * kTileSize * 0.5f) / kTileSize);
    cy = (int)std::lround((wz + a.terrain.h * kTileSize * 0.5f) / kTileSize);
}

static void editPaint(App& app, int tx, int ty) {
    const PaintEntry& pe = kPaint[app.paintIdx];
    const int r = app.brush;
    if (pe.type == TT_Tee && !app.autoOpen) {   // the exe: one hole at a time, finished and opened before the next, 18 holes at most
        bool nearTee = false;
        for (int dy = -3; dy <= 3; dy++) for (int dx = -3; dx <= 3; dx++) {
            const int x = tx + dx, y = ty + dy;
            if (x >= 0 && y >= 0 && x < app.terrain.w && y < app.terrain.h && app.terrain.type[(size_t)app.terrain.tileIndex(x, y)] == TT_Tee) nearTee = true;
        }
        if (!nearTee) {
            const int pending = (int)app.allHoles.size() - (int)app.holes.size();
            const char* why = (int)app.allHoles.size() >= 18 ? "You cannot build more than 18 holes"
                            : app.teeBlobs > (int)app.allHoles.size() ? "Next you need to add a green to the hole you started"
                            : pending > 0 ? "Open this hole (press H) before you build another one" : nullptr;
            if (why) { if (app.lastCell != -2) say(app, why, 4); return; }
        }
    }
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++) {
            if (dx * dx + dy * dy > r * r + r) continue;
            int x = tx + dx, y = ty + dy;
            if (!isOwned(app, x, y)) { if (app.lastCell != -2) toastMsg(app, "You do not own that land"); continue; }
            int vb = pe.type == 0 ? (int)(((uint32_t)(x * 7 + y * 13)) % 5) : pe.vbyte;  // tees use this byte as their look
            if (x >= 0 && y >= 0 && x < app.terrain.w && y < app.terrain.h && app.terrain.type[(size_t)app.terrain.tileIndex(x, y)] != pe.type) {
                app.econ.spend(Economy::terrainCostUnits(pe.type) * Economy::kUnit);
                if (!app.econ.sandbox) app.econ.book(sg::costs::BuildCourse, -Economy::terrainCostUnits(pe.type) * Economy::kUnit);
                // Undo slot: the previous tile id and what was paid (docs/DECODE_BUILDINGS.md section 7).
                app.bsys.recordUndo(x, y, app.terrain.type[(size_t)app.terrain.tileIndex(x, y)], std::min(127, Economy::terrainCostUnits(pe.type)));
            }
            app.terrain.paint(x, y, pe.type, vb);
            if (pe.type == 0 || pe.type == 1 || pe.type == 17 || pe.type == 22) app.terrain.flattenTile(x, y);
        }
    app.dirty = true;
    {
        const int ty = pe.type;
        const char* s = ty == 2 || ty == 3 ? "Interface/Place Fairway.wav" : ty == 0 || ty == 1 ? "Interface/Place GreenTee.wav"
                      : ty == 7 || ty == 9 || ty == 34 || ty == 35 ? "Interface/Place Bunker.wav" : ty == 17 ? "Interface/Place Water.wav"
                      : ty == 12 || ty == 31 || ty == 32 ? "Interface/Place Rocks.wav" : ty == 22 ? "Interface/Building.wav"
                      : ty == 33 ? "Effects/Flower Bed.wav" : "Interface/Place Rough.wav";
        snd(app, s, 0.7f);
    }
}

static void editPath(App& app, int tx, int ty, bool remove) {
    const int r = app.brush;
    if (app.terrain.pathKind.size() != app.terrain.type.size()) app.terrain.pathKind.assign(app.terrain.type.size(), 0);
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++) {
            int x = tx + dx, y = ty + dy;
            if (dx * dx + dy * dy > r * r + r || x < 0 || y < 0 || x >= app.terrain.w || y >= app.terrain.h) continue;
            if (!isOwned(app, x, y)) { toastMsg(app, "You do not own that land"); continue; }
            uint8_t& pk = app.terrain.pathKind[(size_t)app.terrain.tileIndex(x, y)];
            if (!remove && pk == 0) { app.econ.spend(Economy::kPathTileCost); if (!app.econ.sandbox) app.econ.book(sg::costs::Facilities, -Economy::kPathTileCost); app.bsys.recordUndo(x, y, sg::buildings_exe::kUndoPath, 1); }
            pk = remove ? 0 : (uint8_t)app.pathKind;
        }
    app.dirty = true;
    snd(app, "Interface/Path.wav", 0.7f);
}

// Amenity placement. Rules from the game text: buildings go on a building lot and need a path to the clubhouse. The lot must touch a path tile that
// is connected to the clubhouse. Removing refunds the cost (the "money will be refunded" undo text).
static void toastMsg(App& app, const std::string& m) { if (isRefusal(m)) snd(app, "Interface/Wrong.wav", 0.7f); app.toast = m; app.toastKind = 0; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; }
static void applyBuildChanges(App& app, const sg::PlaceResult& res) {
    for (const sg::TileChange& c : res.changes) if (c.newId >= 0) app.terrain.paint(c.tile / 50, c.tile % 50, c.newId == sg::buildings_exe::kTileBuildingHome ? (int)TT_Building : c.newId, 0);
    for (int tl : res.vacatedTiles) app.terrain.paint(tl / 50, tl % 50, TT_Rough, 0);   // PLACEHOLDER: what a vacated tile becomes is not recorded
    app.dirty = true;
}
static void refreshBuildingProps(App& app) {
    app.props.erase(std::remove_if(app.props.begin(), app.props.end(), [](const Prop& p) { return p.building; }), app.props.end());
    for (const App::Placed& b : app.buildings) addBuildingProp(app, b);
    addAmenityProps(app);
    addHomeProps(app);
}
// Placement and removal follow the exe: any owned tile with a valid footprint, a price of base * (level + 2) / 2 plus site work, one more tile per level,
// the old building rebuilt when upgrading, and no refund on demolition (docs/DECODE_BUILDINGS.md).
static void editBuilding(App& app, int tx, int ty, bool remove) {
    const Terrain& t = app.terrain;
    if (tx < 0 || ty < 0 || tx >= t.w || ty >= t.h) return;
    if (remove) {
        const int ri = app.bsys.recordAtTile(tx, ty);
        if (ri < 0 || app.bsys.records()[(size_t)ri].type == sg::buildings_exe::Clubhouse) return;
        const sg::BuildingRecord rec = app.bsys.records()[(size_t)ri];
        const int s = rec.side;
        app.bsys.demolish(ri);
        for (int y = rec.y; y < rec.y + s; y++) for (int x = rec.x; x < rec.x + s; x++) if (x >= 0 && y >= 0 && x < t.w && y < t.h && t.type[(size_t)t.tileIndex(x, y)] == TT_Building) app.terrain.paint(x, y, TT_Rough, 0);
        app.dirty = true; bsysRebuild(app); syncBuildings(app); refreshBuildingProps(app);
        toastMsg(app, "Building demolished (no refund)");
        snd(app, "Interface/Building.wav", 0.7f);
        return;
    }
    if (!themeHas(app, app.buildIdx)) { toastMsg(app, "Not available in this theme"); return; }
    if (!buildUnlocked(app, app.buildIdx)) { toastMsg(app, "This building becomes available as you open more holes"); return; }
    std::vector<sg::BuildTile> tl; sg::BuildContext ctx; bsysContext(app, tl, ctx);
    app.bsys.rebuild(ctx);
    const int type = kBuild[app.buildIdx].unlock;
    const sg::PlaceCheck chk = app.bsys.canPlace(type, tx, ty, ctx);
    if (!chk.ok) { toastMsg(app, chk.reason); return; }
    const double cost = chk.priceUnits * Economy::kUnit;
    if (!app.econ.sandbox && app.econ.cash < cost) { toastMsg(app, "Not enough money"); return; }
    const sg::PlaceResult res = app.bsys.place(type, tx, ty, ctx);
    if (!res.ok) { toastMsg(app, res.reason); return; }
    app.econ.spend(cost); if (!app.econ.sandbox) app.econ.book(sg::costs::Facilities, -cost);
    applyBuildChanges(app, res);
    bsysRebuild(app); syncBuildings(app); refreshBuildingProps(app);
    const bool upgrade = !res.demolished.empty();
    addHighlight(app, std::string(kBuild[app.buildIdx].name) + (upgrade ? " upgraded" : " built")); logEv(app, 0x40, type);
    if (!app.bsys.isConnected(type) && type != sg::buildings_exe::SnackBar) toastMsg(app, "Built, but it needs a path to the clubhouse to work");
    snd(app, "Interface/Building.wav", 0.7f);
}

// ---- Amenities on the Amenities panel and the one-slot-per-tile Undo (docs/DECODE_BUILDINGS.md sections 3 and 7) ----
static bool tileIsWater(int ty) { return ty == TT_WaterShallow || ty == TT_WaterMiddle || ty == TT_WaterDeep || ty == TT_WaterShallowDesert; }
static int amenIndex(const App& app, int tx, int ty, int kind) {
    for (size_t i = 0; i < app.amen.size(); i++) if (app.amen[i].tx == tx && app.amen[i].ty == ty && (kind < 0 || app.amen[i].kind == kind)) return (int)i;
    return -1;
}
// A neighbour that a golfer can stand on: land, no path or bridge on it (flag 0x120), not a building.
static bool benchNeighbourOk(const App& app, int x, int y) {
    const Terrain& t = app.terrain;
    if (x < 0 || y < 0 || x >= t.w || y >= t.h) return false;
    const int ty = t.type[(size_t)t.tileIndex(x, y)];
    if (tileIsWater(ty) || ty == TT_Building || ty == TT_Cliff || ty == TT_Ravine) return false;
    if (t.pathAt(x, y)) return false;
    const int b = amenIndex(app, x, y, 3); return b < 0;
}
// Refunds go back into cash (never in sandbox, where nothing was charged). The exe books them to Build course as well; the port books a refund to the
// ledger row that took the charge so the two stay in step.
static void refundUnits(App& app, int units, int row) {
    if (units <= 0 || app.econ.sandbox) return;
    app.econ.earn(units * Economy::kUnit); app.econ.book(row, units * Economy::kUnit);
}
static void removeAmen(App& app, int index) { if (index >= 0 && index < (int)app.amen.size()) app.amen.erase(app.amen.begin() + index); }
static void applyUndoAt(App& app, int tx, int ty) {
    namespace be = sg::buildings_exe;
    const Terrain& t = app.terrain;
    if (tx < 0 || ty < 0 || tx >= t.w || ty >= t.h) { toastMsg(app, "Out of bounds"); return; }
    // Ball washers and landmarks are building records, not undo slots: removing one refunds as in docs/EXE_COSTS.md.
    const int ri = app.bsys.recordAtTile(tx, ty);
    if (ri >= 0) {
        const sg::BuildingRecord rec = app.bsys.records()[(size_t)ri];
        if (rec.type == be::HomeSite) {   // demolition asks first (a second right click within five seconds) and charges lot / 2 plus buyer / 50, no refund
            int hi = -1; for (size_t i = 0; i < app.homes.size(); i++) if (app.homes[i].x == rec.x && app.homes[i].y == rec.y) hi = (int)i;
            HomeLotEnv env; makeHomeEnv(app, env);
            const int cost = sg::homes::removalCharge(env.lot(rec.x, rec.y), hi >= 0 ? app.homes[(size_t)hi].buyer : 0);
            const double now = SDL_GetTicks() / 1000.0;
            if (app.homeConfirm != hi || now > app.homeConfirmUntil) {
                app.homeConfirm = hi; app.homeConfirmUntil = now + 5;
                char m[140]; std::snprintf(m, sizeof m, "Demolishing this home site costs %s. Right click again to confirm.", money((long long)cost * 100).c_str()); toastMsg(app, m); return;
            }
            app.homeConfirm = -1;
            app.bsys.demolish(ri); if (hi >= 0) app.homes.erase(app.homes.begin() + hi);
            for (int y = rec.y; y < rec.y + 2; y++) for (int x = rec.x; x < rec.x + 2; x++) app.terrain.paint(x, y, TT_Rough, 0);
            if (!app.econ.sandbox) { app.econ.spend(cost * Economy::kUnit); app.econ.book(sg::costs::HomeSites, -(double)cost * Economy::kUnit); }
            app.dirty = true; bsysRebuild(app); refreshBuildingProps(app);
            toastMsg(app, "Home site demolished"); snd(app, "Interface/Building.wav", 0.7f);
            return;
        }
        if (rec.type == be::BallWasher || rec.type == be::Landmark) {
            const int refund = rec.type == be::BallWasher ? be::kBaseCostUnits[be::BallWasher] : be::landmarkRefundUnits(rec.landmarkKind);
            app.bsys.demolish(ri);
            for (int i = (int)app.amen.size() - 1; i >= 0; i--) if (app.amen[(size_t)i].tx == tx && app.amen[(size_t)i].ty == ty && app.amen[(size_t)i].kind >= 4) removeAmen(app, i);
            app.terrain.paint(tx, ty, TT_Rough, 0); app.dirty = true;
            refundUnits(app, refund, sg::costs::Facilities);
            bsysRebuild(app); refreshBuildingProps(app);
            toastMsg(app, rec.type == be::BallWasher ? "Ball washer removed" : "Landmark removed");
            snd(app, "Interface/Building.wav", 0.7f);
            return;
        }
    }
    const sg::UndoResult r = app.bsys.previewUndo(tx, ty);
    if (!r.ok) { toastMsg(app, "Nothing to undo here"); return; }
    const char* name = sg::BuildingSystem::undoName((uint8_t)r.code);
    app.bsys.applyUndo(tx, ty);
    int row = sg::costs::Facilities;
    switch (r.code) {
        case be::kUndoPath: if (!app.terrain.pathKind.empty()) app.terrain.pathKind[(size_t)t.tileIndex(tx, ty)] = 0; break;
        case be::kUndoBench: removeAmen(app, amenIndex(app, tx, ty, 0)); break;
        case be::kUndoFlower: removeAmen(app, amenIndex(app, tx, ty, 1)); break;
        case be::kUndoTree: removeAmen(app, amenIndex(app, tx, ty, 2)); break;
        case be::kUndoBridge: removeAmen(app, amenIndex(app, tx, ty, 3)); break;
        default: if (r.restoreTileId >= 0) { app.terrain.paint(tx, ty, r.restoreTileId, 0); row = sg::costs::BuildCourse; } break;
    }
    refundUnits(app, r.refundUnits, row);
    app.dirty = true; bsysRebuild(app); refreshBuildingProps(app);
    toastMsg(app, std::string(name) + (r.refundUnits > 0 && !app.econ.sandbox ? ", money refunded" : ""));
    snd(app, "Interface/Button2.wav", 0.7f);
}


// Home site placement (docs/DECODE_HOMES.md section 2): the full price is a Facilities charge, a quarter of the lot value comes back as Home Sites income.
static void placeHomeSite(App& app, int tx, int ty) {
    namespace be = sg::buildings_exe;
    auto fail = [&](const char* m) { toastMsg(app, m); snd(app, "Interface/Button1.wav"); };
    if (homeFreeSites(app) < 1) { app.tool = 0; app.amenities = false; fail("No home sites are free: each Silver (or better) member buys one."); return; }
    HomeLotEnv env; makeHomeEnv(app, env);
    const int lot = env.lot(tx, ty);
    env.bc.lotValueUnits = lot;
    app.bsys.rebuild(env.bc);
    const sg::PlaceCheck chk = app.bsys.canPlace(be::HomeSite, tx, ty, env.bc);
    if (!chk.ok) {
        if (lot < sg::homes::kMinLotValue && std::strcmp(chk.reason, "That is an unattractive lot") == 0) { say(app, sg::homes::unattractiveLotText(), 7); snd(app, "Interface/Button1.wav"); }
        else fail(chk.reason);
        return;
    }
    const int share = sg::homes::clubShare(lot), net = chk.priceUnits - share;
    if (!app.econ.sandbox && app.econ.cash < chk.priceUnits * Economy::kUnit) { fail("Not enough money"); return; }
    const sg::PlaceResult res = app.bsys.place(be::HomeSite, tx, ty, env.bc);
    if (!res.ok) { fail(res.reason); return; }
    if (!app.econ.sandbox) { app.econ.spend(net * Economy::kUnit); app.econ.book(sg::costs::Facilities, -(double)chk.priceUnits * Economy::kUnit); app.econ.book(sg::costs::HomeSites, (double)share * Economy::kUnit); }
    applyBuildChanges(app, res);
    app.homes.push_back({tx, ty, 0, 0});
    bsysRebuild(app); refreshBuildingProps(app);
    char m[96]; std::snprintf(m, sizeof m, "Lot sold to the club: %s", money((long long)share * 100).c_str()); toastMsg(app, m);
    snd(app, "Interface/Building.wav", 0.7f);
}

// Placement by the exe tool id: 1 bench, 2 flower bed, 3 ball washer, 4 landmark, 16 scenic tree, 19 scenic bridge. A right-click is the undo.
static void editAmenity(App& app, int tx, int ty, bool remove) {
    namespace be = sg::buildings_exe;
    const Terrain& t = app.terrain;
    if (tx < 0 || ty < 0 || tx >= t.w || ty >= t.h) return;
    if (remove) { applyUndoAt(app, tx, ty); return; }
    if (!isOwned(app, tx, ty)) { toastMsg(app, "You do not own that land"); snd(app, "Interface/Button1.wav"); return; }
    const int tt = t.type[(size_t)t.tileIndex(tx, ty)];
    const bool water = tileIsWater(tt);
    const int tool = app.amenTool;
    auto fail = [&](const char* m) { toastMsg(app, m); snd(app, "Interface/Button1.wav"); };
    if (tool == 5) { placeHomeSite(app, tx, ty); return; }
    if (tool == 3 || tool == 4) {   // ball washer and landmark are building records
        const int type = tool == 3 ? be::BallWasher : be::Landmark;
        std::vector<sg::BuildTile> tl; sg::BuildContext ctx; bsysContext(app, tl, ctx);
        ctx.landmarkKind = app.amenVar[4]; ctx.landmarkDonated = (app.vstate.landmarkMask >> app.amenVar[4]) & 1u;
        app.bsys.rebuild(ctx);
        const sg::PlaceCheck chk = app.bsys.canPlace(type, tx, ty, ctx);
        if (!chk.ok) { fail(chk.reason); return; }
        const double cost = chk.priceUnits * Economy::kUnit;
        if (!app.econ.sandbox && app.econ.cash < cost) { fail("Not enough money"); return; }
        const sg::PlaceResult res = app.bsys.place(type, tx, ty, ctx);
        if (!res.ok) { fail(res.reason); return; }
        app.econ.spend(cost); if (!app.econ.sandbox) app.econ.book(sg::costs::Facilities, -cost);
        applyBuildChanges(app, res);
        app.amen.push_back({tool == 3 ? 4 : 5, tx, ty, tool == 3 ? 0 : app.amenVar[4]});
        bsysRebuild(app); refreshBuildingProps(app);
        snd(app, "Interface/Building.wav", 0.7f);
        return;
    }
    if (tt == TT_Building || tt == TT_Tee || tt == TT_PuttingGreen || tt == TT_TrickyGreen) { fail("Can't build there."); return; }
    if (amenIndex(app, tx, ty, -1) >= 0 && !(tool == 2 && amenIndex(app, tx, ty, 1) < 0)) { fail("Something is already built there."); return; }
    int kind = 0, var = 0, units = 0; uint8_t code = 0; const char* snd_ = "Interface/Bench.wav";
    if (tool == 1) {
        if (water) { fail("Can't build bench there."); return; }
        if (!(benchNeighbourOk(app, tx - 1, ty) || benchNeighbourOk(app, tx + 1, ty) || benchNeighbourOk(app, tx, ty - 1) || benchNeighbourOk(app, tx, ty + 1))) { fail("Can't build bench there."); return; }
        kind = 0; var = app.amenVar[0]; units = be::kBaseCostUnits[be::Benches]; code = be::kUndoBench;
    } else if (tool == 2) {
        if (water) { fail("Can't build flowers there."); return; }
        kind = 1; var = app.amenVar[1]; units = be::kBaseCostUnits[be::FlowerBed]; code = be::kUndoFlower; snd_ = "Effects/Flower Bed.wav";
    } else if (tool == 16) {
        if (water) { fail("Can't build a tree there."); return; }
        kind = 2; var = app.amenVar[2]; units = be::kBaseCostUnits[be::WillowTree]; code = be::kUndoTree; snd_ = "Interface/Place Rough.wav";
    } else if (tool == 19) {
        if (tt != TT_WaterShallow && tt != TT_WaterShallowDesert) { fail("A bridge can only be built on shallow water."); return; }
        kind = 3; var = (int)(app.amenCounter % 8); units = be::kBaseCostUnits[be::ScenicBridge]; code = be::kUndoBridge; snd_ = "Interface/Bridge.wav";
    } else return;
    const double cost = units * Economy::kUnit;
    if (!app.econ.sandbox && app.econ.cash < cost) { fail("Not enough money"); return; }
    app.econ.spend(cost); if (!app.econ.sandbox) app.econ.book(sg::costs::Facilities, -cost);
    if (kind == 1 && tt != TT_Rough) app.terrain.paint(tx, ty, TT_Rough, 0);   // a flower bed turns the tile into rough
    app.amen.push_back({kind, tx, ty, var});
    app.bsys.recordUndo(tx, ty, code, std::min(units, 127));
    app.amenCounter++;
    app.dirty = true; bsysRebuild(app); refreshBuildingProps(app);
    snd(app, snd_, 0.7f);
}

static void editWall(App& app, bool remove) {
    // Nearest tile edge to the picked point.
    int tx, ty; tileOf(app, app.hitX, app.hitZ, tx, ty);
    float fx = (app.hitX + app.terrain.w * kTileSize * 0.5f) / kTileSize - tx, fz = (app.hitZ + app.terrain.h * kTileSize * 0.5f) / kTileSize - ty;
    float d[4] = {fz, 1 - fx, 1 - fz, fx};  // distances to N, E, S, W edges
    int dir = (int)(std::min_element(d, d + 4) - d);
    app.terrain.setWall(tx, ty, dir, !remove);
    app.dirty = true;
    snd(app, "Interface/Place Rocks Generic.wav", 0.7f);
}

static void editRaise(App& app, int cx, int cy, int delta) {
    const int r = app.brush;
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++)
            if (dx * dx + dy * dy <= r * r + r) app.terrain.raiseCorner(cx + dx, cy + dy, delta);
    app.dirty = true;
    snd(app, delta > 0 ? "Interface/Bass Up 2.wav" : "Interface/Bass Down 2.wav", 0.6f);
}

// Applies the current tool at the picked ground point. During a drag it only fires when the cell changes.
static void applyTool(App& app, bool lower, bool dragging) {
    if (!app.hasHit) return;
    int a, b;
    if (app.tool != 1) tileOf(app, app.hitX, app.hitZ, a, b); else cornerOf(app, app.hitX, app.hitZ, a, b);
    int cell = (b << 12) | a | (app.tool << 24);
    if (dragging && cell == app.lastCell) return;
    app.lastCell = cell;
    if (app.tool == 0) editPaint(app, a, b); else if (app.tool == 2) editPath(app, a, b, lower); else if (app.tool == 3) editWall(app, lower); else if (app.tool == 4) editBuilding(app, a, b, lower); else if (app.tool == 5) editAmenity(app, a, b, lower); else if (app.tool == 6) applyUndoAt(app, a, b); else editRaise(app, a, b, (lower != (app.raiseSign < 0)) ? -1 : 1);
}

// Shot path preview while editing: for every tee and green pair, a dotted line along the hole's route with a marker where each shot of a typical golfer
// would land, and a flag at the green. The line and the landing spacing (a drive of about 900 world units, the report's placeholder figure) are
// the port's own presentation; the exe's own preview is not decoded.
// Runs the analysis for one hole: 5 sample drives per skill set (all skills, no Imagination, no Accuracy, no Length), recording the first shot's flight. The yard figure is
// how much closer to the green the average first shot ends than the all-skills golfer's (negative when worse), 25 yards to a tile. The exe's own routine is not decoded
// (docs/SCREENSHOT_NOTES.md only shows the panel), so the sets and the figure are DERIVED from that screenshot.
static void shotAnalysisRun(App& app, int holeIdx) {
    if (holeIdx < 0 || holeIdx >= (int)app.allHoles.size()) return;
    const HoleRoute& r = app.allHoles[(size_t)holeIdx];
    if (r.route.size() < 4) return;
    app.anHole = holeIdx;
    double meanDist[4] = {};
    for (int v = 0; v < 4; v++) {
        sg::GolferSkills sk; for (int& x : sk.v) x = 15;
        if (v == 1) { sk.v[sg::GolferSkills::Draw] = 0; sk.v[sg::GolferSkills::Fade] = 0; sk.v[sg::GolferSkills::Backspin] = 0; sk.v[sg::GolferSkills::Recovery] = 0; }
        if (v == 2) { sk.v[sg::GolferSkills::AccDriver] = 0; sk.v[sg::GolferSkills::AccIrons] = 0; sk.v[sg::GolferSkills::AccPutter] = 0; }
        if (v == 3) { sk.v[sg::GolferSkills::Power] = 0; sk.v[sg::GolferSkills::LongDriver] = 0; }
        double dsum = 0;
        for (int i = 0; i < 5; i++) {
            sg::ShotSim sim; sim.skills = sk; sim.loop = false; sim.setRoute(&r.route); sim.theme = app.theme; sim.init(app.terrain, 4321u + (uint32_t)i * 977u);
            std::vector<float>& P = app.anPath[v][i]; P.clear();
            bool flew = false; int guard = 0;
            while (!sim.finished && guard++ < 30 * 120) {
                sim.step(1.0f / 30.0f);
                if (sim.stroke >= 1) {
                    P.push_back(sim.ballX); P.push_back(sim.ballZ); P.push_back(sim.ballH);
                    if (sim.ballH > 0) flew = true;
                    if (flew && sim.ballH <= 0) { for (int k = 0; k < 20 && !sim.finished; k++) { sim.step(1.0f / 30.0f); if (sim.stroke != 1) break; P.push_back(sim.ballX); P.push_back(sim.ballZ); P.push_back(sim.ballH); } break; }
                }
            }
            const float lx = P.size() >= 3 ? P[P.size() - 3] : r.route[0], lz = P.size() >= 3 ? P[P.size() - 2] : r.route[1];
            app.anLand[v][i][0] = lx; app.anLand[v][i][1] = lz;
            dsum += std::hypot(lx - r.greenX, lz - r.greenZ);
        }
        meanDist[v] = dsum / 5.0;
    }
    for (int v = 1; v < 4; v++) app.anYds[v] = (float)((meanDist[0] - meanDist[v]) * 25.0 / 1024.0);
    app.anYds[0] = 0;
}
static int holeNearGround(const App& app, float wx, float wz) {
    int best = -1; float bd = 1e9f;
    for (size_t h = 0; h < app.allHoles.size(); h++) { const std::vector<float>& P = app.allHoles[h].route; for (size_t i = 0; i + 1 < P.size(); i += 2) { const float d = std::hypot(P[i] - wx, P[i + 1] - wz); if (d < bd) { bd = d; best = (int)h; } } }
    return best;
}
static void drawShotAnalysisWorld(App& app) {
    if (app.anHole < 0 || app.anHole >= (int)app.allHoles.size()) return;
    static const float col[4][3] = {{1.0f, 0.95f, 0.3f}, {0.4f, 1.0f, 0.45f}, {0.4f, 0.9f, 1.0f}, {1.0f, 1.0f, 0.9f}};
    const Terrain& t = app.terrain;
    glDisable(GL_LIGHTING); glDisable(GL_TEXTURE_2D); glDisable(GL_DEPTH_TEST); glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glLineWidth(2.0f);
    for (int v = 3; v >= 0; v--) for (int i = 0; i < 5; i++) {
        const std::vector<float>& P = app.anPath[v][i]; if (P.size() < 6) continue;
        glColor4f(col[v][0], col[v][1], col[v][2], 0.9f);
        glBegin(GL_LINE_STRIP);
        for (size_t k = 0; k + 2 < P.size(); k += 3) glVertex3f(P[k], t.heightAt(P[k], P[k + 1]) + P[k + 2] + 4.0f, P[k + 1]);
        glEnd();
        const float lx = app.anLand[v][i][0], lz = app.anLand[v][i][1], y = t.heightAt(lx, lz) + 6.0f;   // a ring where the shot came down
        glBegin(GL_LINE_LOOP); for (int a = 0; a < 12; a++) { const float an = (float)a * 0.5236f; glVertex3f(lx + 14 * std::cos(an), y, lz + 14 * std::sin(an)); } glEnd();
    }
    glLineWidth(1.0f); glDisable(GL_BLEND);
}
static void drawShotPath(App& app) {
    const Terrain& t = app.terrain;
    glDisable(GL_LIGHTING); glDisable(GL_TEXTURE_2D); glDisable(GL_DEPTH_TEST);
    glLineWidth(2.0f);
    for (const HoleRoute& r : app.allHoles) {
        const bool open = std::find(app.openKeys.begin(), app.openKeys.end(), holeKey(app, r)) != app.openKeys.end() || app.autoOpen;
        if (open) glColor4f(1.0f, 1.0f, 1.0f, 0.85f); else glColor4f(1.0f, 0.9f, 0.2f, 0.95f);   // yellow until the hole is opened
        const std::vector<float>& P = r.route;
        float carried = 0;
        glBegin(GL_LINES);
        for (size_t i = 0; i + 3 < P.size(); i += 2) {
            const float x0 = P[i], z0 = P[i + 1], x1 = P[i + 2], z1 = P[i + 3];
            const float len = std::hypot(x1 - x0, z1 - z0);
            for (float d = 0; d + 30 < len; d += 60) {
                const float a = d / len, b = (d + 30) / len;
                const float ax = x0 + (x1 - x0) * a, az = z0 + (z1 - z0) * a, bx = x0 + (x1 - x0) * b, bz = z0 + (z1 - z0) * b;
                glVertex3f(ax, t.heightAt(ax, az) + 6.0f, az); glVertex3f(bx, t.heightAt(bx, bz) + 6.0f, bz);
            }
        }
        glEnd();
        // Landing markers every 900 units along the route.
        glBegin(GL_QUADS);
        float next = 900;
        for (size_t i = 0; i + 3 < P.size(); i += 2) {
            const float x0 = P[i], z0 = P[i + 1], x1 = P[i + 2], z1 = P[i + 3];
            const float len = std::hypot(x1 - x0, z1 - z0);
            while (next - carried <= len && next < r.length - 120.0f) {
                const float a = (next - carried) / len, mx = x0 + (x1 - x0) * a, mz = z0 + (z1 - z0) * a, y = t.heightAt(mx, mz) + 7.0f;
                glVertex3f(mx - 14, y, mz - 14); glVertex3f(mx + 14, y, mz - 14); glVertex3f(mx + 14, y, mz + 14); glVertex3f(mx - 14, y, mz + 14);
                next += 900;
            }
            carried += len;
        }
        glEnd();
        glColor3f(1.0f, 0.2f, 0.2f);   // the flag
        const float gy = t.heightAt(r.greenX, r.greenZ);
        glBegin(GL_LINES); glVertex3f(r.greenX, gy, r.greenZ); glVertex3f(r.greenX, gy + 90.0f, r.greenZ); glEnd();
    }
    glEnable(GL_DEPTH_TEST);
}

static void drawCursor(App& app) {
    const Terrain& t = app.terrain;
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    float x0, z0, x1, z1;
    const float r = (float)app.brush;
    if (app.tool != 1) {
        int tx, ty; tileOf(app, app.hitX, app.hitZ, tx, ty);
        x0 = ox + (tx - r) * kTileSize; z0 = oz + (ty - r) * kTileSize;
        x1 = ox + (tx + r + 1) * kTileSize; z1 = oz + (ty + r + 1) * kTileSize;
    } else {
        int cx, cy; cornerOf(app, app.hitX, app.hitZ, cx, cy);
        x0 = ox + (cx - r - 0.5f) * kTileSize; z0 = oz + (cy - r - 0.5f) * kTileSize;
        x1 = ox + (cx + r + 0.5f) * kTileSize; z1 = oz + (cy + r + 0.5f) * kTileSize;
    }
    glDisable(GL_LIGHTING); glDisable(GL_TEXTURE_2D); glDisable(GL_DEPTH_TEST);
    glLineWidth(2.0f);
    glColor3f(1.0f, 0.95f, 0.3f);
    glBegin(GL_LINE_LOOP);
    const int N = 8;
    auto pt = [&](float x, float z) { x = std::clamp(x, ox, -ox); z = std::clamp(z, oz, -oz); glVertex3f(x, t.heightAt(x, z) + 3.0f, z); };
    for (int i = 0; i < N; i++) pt(x0 + (x1 - x0) * i / N, z0);
    for (int i = 0; i < N; i++) pt(x1, z0 + (z1 - z0) * i / N);
    for (int i = 0; i < N; i++) pt(x1 - (x1 - x0) * i / N, z1);
    for (int i = 0; i < N; i++) pt(x0, z1 - (z1 - z0) * i / N);
    glEnd();
    glEnable(GL_DEPTH_TEST);
}


// Land you do not own is shaded dark (PLACEHOLDER look: the exe shows for-sale tiles with tile code 0x14, whose art is not identified).
static void drawUnowned(App& app) {
    if (!app.landModel || app.ownMask == 0x1ff) return;
    const Terrain& t = app.terrain;
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    glDisable(GL_LIGHTING); glDisable(GL_TEXTURE_2D); glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glDepthMask(GL_FALSE);
    glColor4f(0.0f, 0.0f, 0.0f, 0.82f);   // unowned land is shown black in the real game
    glBegin(GL_QUADS);
    for (int y = 0; y < t.h; y++) for (int x = 0; x < t.w; x++) {
        if (isOwned(app, x, y)) continue;
        const float x0 = ox + x * kTileSize, z0 = oz + y * kTileSize, x1 = x0 + kTileSize, z1 = z0 + kTileSize;
        glVertex3f(x0, t.heightAt(x0, z0) + 2, z0); glVertex3f(x1, t.heightAt(x1, z0) + 2, z0);
        glVertex3f(x1, t.heightAt(x1, z1) + 2, z1); glVertex3f(x0, t.heightAt(x0, z1) + 2, z1);
    }
    glEnd();
    glDepthMask(GL_TRUE); glDisable(GL_BLEND);
}

static void render(App& app) {
    glViewport(0, 0, app.drawW, app.drawH);
    glClearColor(0.04f, 0.06f, 0.09f, 1);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

    float upp = 2.0f / (app.zoom * app.dpi);  // world units per drawable pixel
    float hw = app.drawW * upp * 0.5f, hh = app.drawH * upp * 0.5f;
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glOrtho(-hw, hw, -hh, hh, -kDepthRange, kDepthRange);
    glMatrixMode(GL_MODELVIEW);
    glLoadIdentity();

    // Directional light fixed in eye space. APPROXIMATION: the original's light setup is not decoded.
    const GLfloat pos[4] = {-0.45f, 0.85f, 0.45f, 0.0f};
    glLightfv(GL_LIGHT0, GL_POSITION, pos);
    GLfloat amb[4] = {app.light.ambient[0] * 0.6f, app.light.ambient[1] * 0.6f, app.light.ambient[2] * 0.6f, 1};
    GLfloat dif[4] = {app.light.diffuse[0] * 0.55f, app.light.diffuse[1] * 0.55f, app.light.diffuse[2] * 0.55f, 1};
    glLightfv(GL_LIGHT0, GL_AMBIENT, amb);
    glLightfv(GL_LIGHT0, GL_DIFFUSE, dif);
    const GLfloat none[4] = {0, 0, 0, 1}, white[4] = {1, 1, 1, 1};
    glLightModelfv(GL_LIGHT_MODEL_AMBIENT, none);
    glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, white);

    glRotated(pitchFor(app.drawW, app.drawH), 1, 0, 0);
    glRotatef(45.0f + app.rot, 0, 1, 0);
    glTranslatef(-app.camX, 0, -app.camZ);

    glEnable(GL_DEPTH_TEST);
    glEnable(GL_LIGHTING);
    glEnable(GL_LIGHT0);
    glEnable(GL_NORMALIZE);
    glDisable(GL_CULL_FACE);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
    for (auto& kv : app.batches) {
        // APPROXIMATION: water shimmers by drifting its texture a pixel or so; the original's water animation is not decoded.
        const bool water = app.waterTex.count(kv.first) != 0;
        if (water) {
            glMatrixMode(GL_TEXTURE); glLoadIdentity();
            glTranslatef(0.014f * std::sin((float)app.time * 1.3f), 0.014f * std::cos((float)app.time * 0.9f), 0);
            glMatrixMode(GL_MODELVIEW);
        }
        if (kv.first) { glEnable(GL_TEXTURE_2D); glBindTexture(GL_TEXTURE_2D, kv.first); }
        else { glDisable(GL_TEXTURE_2D); glColor3f(0.4f, 0.45f, 0.4f); }
        glBegin(GL_TRIANGLES);
        for (const Vertex& v : kv.second) {
            glNormal3f(v.nx, v.ny, v.nz);
            glTexCoord2f(v.u, v.v);
            glVertex3f(v.x, v.y, v.z);
        }
        glEnd();
        if (water) { glMatrixMode(GL_TEXTURE); glLoadIdentity(); glMatrixMode(GL_MODELVIEW); }
    }
    glEnable(GL_TEXTURE_2D);
    for (auto& kv : app.wallBatches) {
        glBindTexture(GL_TEXTURE_2D, kv.first);
        glBegin(GL_TRIANGLES);
        for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
        glEnd();
    }
    if (!app.pathBatches.empty()) {
        glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_ALPHA_TEST); glAlphaFunc(GL_GREATER, 0.04f);
        glDepthMask(GL_FALSE);
        glEnable(GL_TEXTURE_2D);
        for (auto& kv : app.pathBatches) {
            glBindTexture(GL_TEXTURE_2D, kv.first);
            glBegin(GL_TRIANGLES);
            for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
            glEnd();
        }
        glDepthMask(GL_TRUE); glDisable(GL_ALPHA_TEST); glDisable(GL_BLEND);
    }
    if (!app.mudBatches.empty()) {   // unconnected paths: the same artwork in a muddy brown
        const GLfloat mud[4] = {0.45f, 0.32f, 0.22f, 1.0f};
        glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, mud);
        glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_ALPHA_TEST); glAlphaFunc(GL_GREATER, 0.04f);
        glDepthMask(GL_FALSE);
        glEnable(GL_TEXTURE_2D);
        for (auto& kv : app.mudBatches) {
            glBindTexture(GL_TEXTURE_2D, kv.first);
            glBegin(GL_TRIANGLES);
            for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
            glEnd();
        }
        glDepthMask(GL_TRUE); glDisable(GL_ALPHA_TEST); glDisable(GL_BLEND);
        const GLfloat white[4] = {1, 1, 1, 1};
        glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, white);
    }
    glGetDoublev(GL_MODELVIEW_MATRIX, app.mv);
    app.upp = upp;
    drawUnowned(app);
    drawProps(app);
    if (app.edit) drawShotPath(app);
    drawShotAnalysisWorld(app);
    if (app.edit && app.hasHit) drawCursor(app);
}

static bool screenshot(const App& app, const char* file) {
    Rgba img;
    img.w = (uint32_t)app.drawW; img.h = (uint32_t)app.drawH;
    Bytes raw((size_t)img.w * img.h * 4);
    glPixelStorei(GL_PACK_ALIGNMENT, 1);
    glReadPixels(0, 0, app.drawW, app.drawH, GL_RGBA, GL_UNSIGNED_BYTE, raw.data());
    img.px.resize(raw.size());
    for (uint32_t y = 0; y < img.h; y++)  // GL origin is bottom-left
        std::memcpy(&img.px[(size_t)y * img.w * 4], &raw[(size_t)(img.h - 1 - y) * img.w * 4], (size_t)img.w * 4);
    for (size_t i = 3; i < img.px.size(); i += 4) img.px[i] = 255;
    return writePng(file, img);
}

static void pan(App& app, float right, float up) {
    float yaw = (45.0f + app.rot) * 3.14159265f / 180.0f;
    app.camX += std::cos(yaw) * right + std::sin(yaw) * up;
    app.camZ += std::sin(yaw) * right - std::cos(yaw) * up;
}

int main(int argc, char** argv) {
    App app;
    app.gameDir = "game/Program_Files_(ENGLISH)";
    int winW = 1280, winH = 800;
    int centerX = -1, centerY = -1;  // optional start position in tile coordinates
    const char* pngOut = nullptr;
    std::string golfer;
    int sgaTest = -1;   // --sga N: 0 evaluation screen, 1 forced offer, 2 offer accepted and played
    std::string screenArg;   // menu, property or play (default: the menu unless the run is scripted)
    const char* loadFile = nullptr; const char* saveFile = nullptr; std::string editSpec; int newGameIdx = -1; int staffHook[4][2] = {}; bool staffSet = false; int openN = 0; const char* saveGameFile = nullptr; const char* loadGameFile = nullptr; const char* saveLateFile = nullptr; std::vector<int> switchTo; std::vector<std::array<int, 4>> evHook, hhHook; int swapA = -1, swapB = -1, silverN = 0, monthsN = 0, fakePlays = 0, fakeMood = 0; std::vector<std::pair<int, int>> homeAt, unhomeAt;
    for (int i = 1; i < argc; i++) {
        std::string a = argv[i];
        auto next = [&]() { return i + 1 < argc ? argv[++i] : (const char*)""; };
        if (a == "--game") app.gameDir = next();
        else if (a == "--theme") { std::string t = next(); for (int k = 0; k < 4; k++) if (strcasecmp(t.c_str(), kThemes[k]) == 0) app.theme = k; }
        else if (a == "--seed") app.seed = (uint32_t)std::strtoul(next(), nullptr, 10);
        else if (a == "--size") std::sscanf(next(), "%dx%d", &winW, &winH);
        else if (a == "--zoom") app.zoom = (float)std::atof(next());
        else if (a == "--rot") app.rot = (float)std::atof(next());
        else if (a == "--png") pngOut = next();
        else if (a == "--amen") app.amenities = true;
        else if (a == "--amentool") { app.amenities = true; app.panel = 1; app.edit = true; app.tool = 5; app.amenTool = std::atoi(next()); }   // test hook: arm an Amenities tool
        else if (a == "--elev") app.elevation = true;
        else if (a == "--hire") app.hireOpen = true;
        else if (a == "--staff" && i + 1 < argc) { int k = 0, n = 0, sk = 0; if (sscanf(argv[++i], "%d,%d,%d", &k, &n, &sk) >= 2 && k >= 0 && k < 4) { staffHook[k][0] = n; staffHook[k][1] = sk; staffSet = true; } }
        else if (a == "--hover") { std::sscanf(next(), "%f,%f", &app.testHx, &app.testHy); }
        else if (a == "--time") app.time = std::atof(next());
        else if (a == "--follow") app.follow = true;
        else if (a == "--golfer") golfer = next();
        else if (a == "--mute") app.mute = true;
        else if (a == "--sound-log") app.soundLog = true;
        else if (a == "--sandbox") app.econ.sandbox = true;
        else if (a == "--skills") g_skillsTest = true;
        else if (a == "--player") { app.panel = 3; app.golfersMode = true; app.playerPanel = true; app.dockHover = 2; }
        else if (a == "--practice") g_practiceTest = true;
        else if (a == "--tutorial") g_tutTest = std::atoi(next());
        else if (a == "--match") g_matchTest = true;
        else if (a == "--analyze") app.anHole = std::atoi(next()) + 1000000;   // hole index to analyse once the course is ready
        else if (a == "--say") { app.lastMsg = next(); app.toast = app.lastMsg; app.toastKind = 1; app.toastUntil = 1e12; app.showAdvisor = false; }
        else if (a == "--prop" && i + 1 < argc) app.curProp = std::atoi(argv[++i]);
        else if (a == "--screen") screenArg = next();
        else if (a == "--popup") popArg = std::atoi(next());
        else if (a == "--sga") sgaTest = std::atoi(next());
        else if (a == "--reveal") g_revealForce = std::atoi(next());   // test hook: show N holes of the tournament reveal
        else if (a == "--cash") app.econ.startCash = std::atof(next());   // test hook: starting cash
        else if (a == "--open") openN = std::atoi(next());                   // test hook: press H this many times after the edits
        else if (a == "--new") newGameIdx = std::atoi(next());               // test hook: start a new game on this property
        else if (a == "--savegame") saveGameFile = next();                    // test hook: write a full saved game (terrain, money, buildings) after any edits
        else if (a == "--switch") switchTo.push_back(std::atoi(next()));     // test hook: switch to (or buy) this property via the world map logic, in order
        else if (a == "--home") { int hx = 0, hy = 0; if (std::sscanf(next(), "%d,%d", &hx, &hy) == 2) homeAt.push_back({hx, hy}); }   // test hook: place a home site (after --open and --fake)
        else if (a == "--unhome") { int hx = 0, hy = 0; if (std::sscanf(next(), "%d,%d", &hx, &hy) == 2) unhomeAt.push_back({hx, hy}); }   // test hook: demolish the home site at x,y (two right clicks)
        else if (a == "--t2hover") g_t2Hover = std::atoi(next());
        else if (a == "--savechamp") g_saveChamp = next();
        else if (a == "--champgo") g_champGo = std::atoi(next());
        else if (a == "--card") g_cardHook = std::atoi(next());
        else if (a == "--cardhover") g_cardHover = std::atoi(next());
        else if (a == "--cardskills") g_cardSkills = true;
        else if (a == "--cuhover") g_cuHover = std::atoi(next());
        else if (a == "--cuface") g_cuFace = std::atoi(next());
        else if (a == "--worldreset") g_resetSeed = std::atoi(next());
        else if (a == "--worldhover") g_hoverHook = std::atoi(next());
        else if (a == "--worldconfirm") g_confirmHook = std::atoi(next());
        else if (a == "--mood") g_moodHook = std::atoi(next());
        else if (a == "--needs") std::sscanf(next(), "%d,%d,%d", &g_needsHook[0], &g_needsHook[1], &g_needsHook[2]);
        else if (a == "--ev") { std::array<int, 4> v = {0, 0, 0, 0}; std::sscanf(next(), "%d,%d,%d,%d", &v[0], &v[1], &v[2], &v[3]); evHook.push_back(v); }   // test hook: hole,type,count,location
        else if (a == "--hh") { std::array<int, 4> v = {0, 0, 0, 0}; std::sscanf(next(), "%d,%d,%d,%d", &v[0], &v[1], &v[2], &v[3]); hhHook.push_back(v); }   // test hook: hole,mask,bin,count of the stroke histogram
        else if (a == "--fake") std::sscanf(next(), "%d,%d", &fakePlays, &fakeMood);   // test hook: give every open hole this many plays and mood total
        else if (a == "--silver") silverN = std::atoi(next());               // test hook: make the first N roster members Silver
        else if (a == "--months") monthsN = std::atoi(next());               // test hook: run the monthly home pass N months
        else if (a == "--swap") { std::sscanf(next(), "%d,%d", &swapA, &swapB); }   // test hook: swap two holes (1 based) after the --open presses
        else if (a == "--savelate") saveLateFile = next();                    // test hook: write the saved game after the --time run, just before the screenshot
        else if (a == "--loadgame") loadGameFile = next();                    // test hook: load a full saved game
        else if (a == "--course") loadFile = next();
        else if (a == "--save") saveFile = next();
        else if (a == "--edit") editSpec = next();
        else if (a == "--panel") { app.panel = std::atoi(next()); app.dockHover = app.panel > 0 ? app.panel - 1 : -1; app.edit = app.panel == 1 || app.panel == 2; }
        else if (a == "--empsel") { app.panel = 3; app.golfersMode = false; app.empSel = std::atoi(next()); }
        else if (a == "--center") std::sscanf(next(), "%d,%d", &centerX, &centerY);
        else { std::fprintf(stderr, "usage: sgview --game DIR [--theme T] [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--png FILE]\n"); return 2; }
    }
    if (!golfer.empty()) {  // play as a golfer from progolfers.dta (case-insensitive name match)
        Bytes d; std::vector<ProGolfer> pros; std::string err;
        if (!readFile(app.gameDir + "/Themes/Standard/progolfers.dta", d) || !parseProGolfers(std::string(d.begin(), d.end()), pros, err)) {
            std::fprintf(stderr, "error: cannot read progolfers.dta (%s)\n", err.c_str()); return 1;
        }
        auto low = [](std::string s) { for (char& ch : s) ch = (char)std::tolower((unsigned char)ch); return s; };
        const ProGolfer* hit = nullptr;
        for (const auto& g : pros) if (low(g.name).find(low(golfer)) != std::string::npos) { hit = &g; break; }
        if (!hit) { std::fprintf(stderr, "no golfer matching '%s'\n", golfer.c_str()); return 1; }
        for (int k = 0; k < 10; k++) app.skills.v[k] = hit->skill[k];
        std::printf("golfer: %s, skills", hit->name.c_str());
        for (int k = 0; k < 10; k++) std::printf(" %X", hit->skill[k]);
        std::printf("\n");
    }
    if (SDL_Init(SDL_INIT_VIDEO) != 0) { std::fprintf(stderr, "SDL_Init: %s\n", SDL_GetError()); return 1; }
    SDL_StartTextInput();
    SDL_GL_SetAttribute(SDL_GL_DEPTH_SIZE, 24);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    SDL_Window* win = SDL_CreateWindow("SimGolf native: terrain viewer", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, winW, winH,
                                       SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI);
    if (!win) { std::fprintf(stderr, "SDL_CreateWindow: %s\n", SDL_GetError()); return 1; }
    SDL_GLContext ctx = SDL_GL_CreateContext(win);
    if (!ctx) { std::fprintf(stderr, "SDL_GL_CreateContext: %s\n", SDL_GetError()); return 1; }
    SDL_GL_SetSwapInterval(1);
    std::printf("GL: %s | %s\n", glGetString(GL_VERSION), glGetString(GL_RENDERER));

    app.mixer = std::make_unique<Mixer>(app.gameDir + "/Sounds");
    app.mixer->addFolder(app.gameDir + "/SimsFX/Male", "simsfx/male/");
    app.mixer->addFolder(app.gameDir + "/SimsFX/Female", "simsfx/female/");
    if (app.mixer->clipsLoaded() == 0 && app.mixer->list("").empty()) { std::fprintf(stderr, "sound: %s\n", app.mixer->lastError.c_str()); app.mixer.reset(); }
    else if (!app.mute && !pngOut) {
        if (app.audioDev.open(*app.mixer)) startAmbience(app);
        else std::fprintf(stderr, "sound: no audio device, running silent\n");
    } else if (app.soundLog) startAmbience(app);
    app.terrain = Terrain::demoCourse(40, 40, app.seed);
    if (!loadTheme(app, app.theme)) return 1;
    if (!loadUi(app)) std::fprintf(stderr, "ui: could not load the Interface art or KLEPTO__.TTF, starting on the course\n");
    loadStory(app);
    {
        g_scriptedRun = pngOut != nullptr;
        const bool scripted = pngOut || loadFile || newGameIdx >= 0 || loadGameFile || !editSpec.empty() || saveFile || !golfer.empty() || app.econ.sandbox || app.follow;
        std::string s = screenArg.empty() ? (scripted ? "play" : "menu") : screenArg;
        if (s == "report" && app.uiOk) app.screen = App::ScreenReport;
        if (s == "finance" && app.uiOk) app.screen = App::ScreenFinance;
        if (s == "roster" && app.uiOk) app.screen = App::ScreenRoster;
        if (s == "keys" && app.uiOk) app.screen = App::ScreenKeys;
        if (s == "histo" && app.uiOk) app.screen = App::ScreenHisto;
        if (s == "buyland" && app.uiOk) openBuyLand(app);
        if ((s == "overview" || s == "aura" || s == "value" || s == "employ") && app.uiOk) { app.ovMode = s == "aura" ? 1 : s == "value" ? 2 : s == "employ" ? 3 : -1; app.ovSel = 0; if (app.ovMode == 1 || app.ovMode == 2) ovComputeHeat(app); app.screen = App::ScreenOverview; }
        if (s == "comments" && app.uiOk) { syncHoleStats(app); app.screen = App::ScreenComments; }
        if (s == "holestat" && app.uiOk) { syncHoleStats(app); if (!app.hstats.empty()) { app.hsHole = 0; app.screen = App::ScreenHoleStat; } }
        if (app.uiOk && s == "menu") app.screen = App::ScreenMenu;
        else if (app.uiOk && s == "property") app.screen = App::ScreenProperty;
        else if (app.uiOk && s == "character") { openCustomise(app); }
    }
    if (newGameIdx >= 0 && newGameIdx < 16) { const int pn = app.panel; startGame(app, newGameIdx, app.econ.sandbox); app.panel = pn; app.edit = pn == 1 || pn == 2; }
    if (staffSet) for (int k = 0; k < 4; k++) if (staffHook[k][0]) { app.econ.staff[k] = staffHook[k][0]; app.econ.skilled[k] = staffHook[k][1]; }
    if (loadGameFile) { std::string err; if (!loadGame(app, loadGameFile, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; } std::printf("loaded game %s: cash %.0f, %zu buildings, day %d\n", loadGameFile, app.econ.cash, app.buildings.size(), app.econ.day); }
    for (int t : switchTo) if (t >= 0 && t < 16) switchCourse(app, t);
    if (screenArg == "world" && app.uiOk) { app.switchMode = true; app.screen = App::ScreenProperty; }
    if (screenArg == "character" && app.uiOk) { openCustomise(app); if (g_cuHover >= 0) { app.cuHover = g_cuHover; app.cuFrames = 20; } if (g_cuFace >= 0) { app.cuFace = true; app.cuFacePage = g_cuFace; } }
    if (screenArg == "diff" && app.uiOk) { t2LoadArt(app); app.diffPending = 1; app.t2Hover = g_t2Hover; app.screen = App::ScreenDiff; }
    if (!g_saveChamp.empty() && app.uiOk) { std::string err; std::printf("championship save %s: %s\n", g_saveChamp.c_str(), champSave(app, g_saveChamp, err) ? "ok" : err.c_str()); }
    if (screenArg == "champload" && app.uiOk) { t2LoadArt(app); app.t2Mode = 1; t2ListFolder(app, "Championship", ".cse"); app.t2Sel = g_t2Hover >= 0 && !app.t2Files.empty() ? 0 : -1; app.t2Hover = g_t2Hover; app.screen = App::ScreenLoad; }
    if (screenArg == "pair" && app.uiOk) { app.screen = App::ScreenPlay; loadCharArt(app); pairOpen(app); app.pairSel = 5; }
    if (screenArg == "pro" && app.uiOk) { t2LoadArt(app); app.champ = true; t2ListFolder(app, "Championship", ".pro"); app.t2Sel = g_t2Hover >= 0 && !app.t2Files.empty() ? 0 : -1; app.t2Hover = g_t2Hover; app.screen = App::ScreenPro; }
    if (g_champGo >= 0 && app.uiOk) {   // headless run of the whole flow: first championship course, then pro number g_champGo
        t2LoadArt(app); app.t2Mode = 1; t2ListFolder(app, "Championship", ".cse");
        if (app.t2Files.empty()) std::printf("no championship courses saved\n");
        else {
            const int keep = app.difficulty; std::string err;
            if (loadGame(app, app.t2Files[0], err)) {
                app.difficulty = keep; app.tracker = sg::GoalTracker(keep); app.champ = true; app.t2Mode = 0; app.econ.sandbox = false; app.econ.cash = 100000; app.econ.startCash = 100000; app.econ.day = 12; app.econ.version++;
                t2ListFolder(app, "Championship", ".pro"); sg::CharRec c;
                if (!app.t2Files.empty() && sg::charLoadFile(app.t2Files[(size_t)std::min<int>(g_champGo, (int)app.t2Files.size() - 1)], c)) { app.chr = c; chrSync(app); }
                champStart(app); std::printf("championship: player place %d of %zu, prize %d,000\n", app.tResult.playerPlace, app.tField.size(), app.tPrize);
            } else std::printf("load failed: %s\n", err.c_str());
        }
    }
    if (screenArg == "themes" && app.uiOk) { t2LoadArt(app); app.t2Hover = g_t2Hover; app.screen = App::ScreenThemes; }
    if (screenArg == "load" && app.uiOk) { t2LoadArt(app); t2ListSaves(app); app.t2Sel = g_t2Hover >= 0 && !app.t2Files.empty() ? 0 : -1; app.t2Hover = g_t2Hover; app.screen = App::ScreenLoad; }
    if (screenArg == "credits" && app.uiOk) { creditsOpen(app); app.creditStart = SDL_GetTicks() - 20000; }
    if (screenArg == "best" && app.uiOk) { refreshHoles(app); if (g_t2Hover >= 0) { const char* nm[6] = {"Avery Gale", "B. Houston", "Kelley Greens", "Joe Pro", "Ivana Richman", "J.P. Bigdome"}; for (int i = 0; i < 6; i++) app.best.insert(68 + i * 2 + (i == 3), nm[i]); } openBest(app); app.t2Hover = g_t2Hover; }
    if (screenArg == "top10" && app.uiOk) { openTop10(app); if (g_t2Hover >= 0) { sg::DesignerEntry n; std::snprintf(n.name, sizeof n.name, "Gary Golf"); std::snprintf(n.course, sizeof n.course, "Test Course"); n.fun = 700; n.skill = 600; n.cash = 5000; n.difficulty = 2; n.courseId = 3; app.top10.insert(n); } }
    if (screenArg == "property" && app.uiOk) { app.switchMode = false; app.screen = App::ScreenProperty; }
    if (g_resetSeed >= 0) worldReset(app, (uint64_t)g_resetSeed);
    if (g_hoverHook >= 0) app.hover = g_hoverHook;
    if (g_confirmHook >= 0) app.confirmIdx = g_confirmHook;
    if (screenArg == "finance" && app.uiOk) app.screen = App::ScreenFinance;
    if (screenArg == "roster" && app.uiOk) app.screen = App::ScreenRoster;
    if (screenArg == "sga" && app.uiOk) openSgaScreen(app);
    if (screenArg == "histo" && app.uiOk) app.screen = App::ScreenHisto;
    if (screenArg == "buyland" && app.uiOk) openBuyLand(app);
    if ((screenArg == "overview" || screenArg == "aura" || screenArg == "value" || screenArg == "employ") && app.uiOk) { app.ovMode = screenArg == "aura" ? 1 : screenArg == "value" ? 2 : screenArg == "employ" ? 3 : -1; app.ovSel = 0; if (app.ovMode == 1 || app.ovMode == 2) ovComputeHeat(app); app.screen = App::ScreenOverview; }
    if (screenArg == "board" && app.uiOk) app.screen = App::ScreenBoard;
    if (screenArg == "boardtest" && app.uiOk) { int ids[5] = {0, 2, 5, 6, 7}; for (int i = 0; i < 5; i++) { app.miles[ids[i]].tick = 1500 + 2100L * i; app.miles[ids[i]].course = app.courseName; } app.screen = App::ScreenBoard; }
    if (loadFile) {
        std::string err;
        if (!Terrain::load(loadFile, app.terrain, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        rebuildBatches(app); refreshTrees(app);
    }
    // Scripted edits for tests: "p:x,y,type[,vbyte,radius];r:cx,cy,delta[,radius];b:x,y,building"
    if (silverN > 0) { if (!app.rosterReady) { app.roster.newGame(app.srng); app.rosterReady = true; } for (int i = 1; i <= silverN && i <= sg::Roster::kPool; i++) { app.roster.e[i].tier = sg::Tier::Silver; app.roster.e[i].resigned = false; } }
    for (size_t pos = 0; pos < editSpec.size();) {
        size_t end = editSpec.find(';', pos);
        std::string item = editSpec.substr(pos, end == std::string::npos ? std::string::npos : end - pos);
        pos = end == std::string::npos ? editSpec.size() : end + 1;
        int a = 0, b = 0, c3 = 0, d = 0, r = 0;
        if (item.size() > 2 && item[0] == 'p' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d,%d", &a, &b, &c3, &d, &r) >= 3) {
            for (int dy = -r; dy <= r; dy++) for (int dx = -r; dx <= r; dx++)
                if (dx * dx + dy * dy <= r * r + r) {
                    app.terrain.paint(a + dx, b + dy, c3, d);
                    if (c3 == 0 || c3 == 1 || c3 == 17 || c3 == 22) app.terrain.flattenTile(a + dx, b + dy);
                }
        } else if (item.size() > 2 && item[0] == 'w' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d", &a, &b, &c3, &d) >= 3) {
            if (app.terrain.pathKind.size() != app.terrain.type.size()) app.terrain.pathKind.assign(app.terrain.type.size(), 0);
            for (int dy = -d; dy <= d; dy++) for (int dx = -d; dx <= d; dx++) {
                int x = a + dx, y = b + dy;
                if (dx * dx + dy * dy <= d * d + d && x >= 0 && y >= 0 && x < app.terrain.w && y < app.terrain.h) app.terrain.pathKind[(size_t)app.terrain.tileIndex(x, y)] = (uint8_t)c3;
            }
        } else if (item.size() > 2 && item[0] == 'b' && std::sscanf(item.c_str() + 2, "%d,%d,%d", &a, &b, &c3) == 3) {
            app.buildIdx = c3 % kBuildCount;
            editBuilding(app, a, b, false);
            std::printf("building %s at %d,%d: %zu placed, toast '%s' (clubhouse %d,%d)\n", kBuild[app.buildIdx].name, a, b, app.buildings.size(), app.toast.c_str(), app.terrain.clubhouseX, app.terrain.clubhouseY);
        } else if (item.size() > 2 && item[0] == 'a' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d", &a, &b, &c3, &d) >= 3) {   // amenity: x,y,exe tool id,design
            app.amenTool = c3; const int si = stripIndexFor(c3); if (si >= 0) app.amenVar[stripVarSlot(si)] = d; else if (c3 == 4) app.amenVar[4] = d;
            editAmenity(app, a, b, false);
            std::printf("amenity tool %d at %d,%d: %zu placed, toast '%s'\n", c3, a, b, app.amen.size(), app.toast.c_str());
        } else if (item.size() > 2 && item[0] == 'l' && std::sscanf(item.c_str() + 2, "%d", &a) == 1) {   // buy land tract a (0..8)
            app.blRolled = false; rollTractPrices(app); const int before = app.ownMask; const double c0 = app.econ.cash; buyTract(app, a);
            std::printf("buy tract %d: price %d units, owned mask %03x -> %03x, cash %.0f -> %.0f, note '%s'\n", a, app.blPrice[a], before, app.ownMask, c0, app.econ.cash, app.blNote.c_str());
        } else if (item.size() > 2 && item[0] == 'u' && std::sscanf(item.c_str() + 2, "%d,%d", &a, &b) == 2) {   // undo at x,y
            applyUndoAt(app, a, b);
            std::printf("undo at %d,%d: %zu amenities left, cash %.0f, toast '%s'\n", a, b, app.amen.size(), app.econ.cash, app.toast.c_str());
        } else if (item.size() > 2 && item[0] == 'k' && std::sscanf(item.c_str() + 2, "%d,%d,%d", &a, &b, &c3) == 3) {
            app.terrain.setWall(a, b, c3, true);
        } else if (item.size() > 2 && item[0] == 'r' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d", &a, &b, &c3, &d) >= 3) {
            for (int dy = -d; dy <= d; dy++) for (int dx = -d; dx <= d; dx++)
                if (dx * dx + dy * dy <= d * d + d) app.terrain.raiseCorner(a + dx, b + dy, c3);
        } else std::fprintf(stderr, "bad --edit item: %s\n", item.c_str());
    }
    if (!editSpec.empty()) { rebuildBatches(app); refreshTrees(app); app.econ.updateUpkeep(app.terrain); }
    if (app.anHole >= 1000000) { const int hh = app.anHole - 1000000; app.anHole = -1; refreshHoles(app); shotAnalysisRun(app, hh); }
    if (openN > 0) { refreshHoles(app); for (int i = 0; i < openN; i++) openHole(app); std::printf("open holes %zu of %zu, unlock level %d, stage %s\n", app.holes.size(), app.allHoles.size(), app.unlockLevel, kCourseStage[app.courseStage]); }
    if (swapA > 0) swapHoles(app, swapA - 1, swapB - 1);
    if (!evHook.empty() || !hhHook.empty()) {
        syncHoleStats(app);
        for (const auto& v : evHook) if (v[0] >= 1 && v[0] <= (int)app.hstats.size() && v[1] >= 0 && v[1] < 64) { sg::HoleStats& hs = app.hstats[(size_t)v[0] - 1]; hs.events[v[1]] = (int16_t)v[2]; hs.eventLoc[v[1]] = (int16_t)v[3]; if (hs.rounds < 100) hs.rounds = 100; }
        for (const auto& v : hhHook) if (v[0] >= 1 && v[0] <= (int)app.hstats.size() && v[1] >= 0 && v[1] < 8 && v[2] >= 1 && v[2] <= 9) { sg::HoleStats& hs = app.hstats[(size_t)v[0] - 1]; hs.hist[v[1]][v[2]] = (int16_t)v[3]; }
    }
    if (app.uiOk && screenArg == "holestat") { syncHoleStats(app); if (!app.hstats.empty()) { app.hsHole = 0; app.screen = App::ScreenHoleStat; } }   // after --open so the hole exists
    if (app.uiOk && screenArg == "comments") { syncHoleStats(app); app.screen = App::ScreenComments; }
    if (app.uiOk && screenArg == "award") { syncHoleStats(app); if (!app.hstats.empty()) { app.awardHole = 0; app.awardKind = 100; app.screen = App::ScreenAward; } }
    if (fakePlays > 0) { app.holeStats.assign(app.holes.size(), App::HoleStat()); for (App::HoleStat& hs : app.holeStats) { hs.plays = fakePlays; hs.moodSum = fakeMood; } }
    for (const auto& hp : homeAt) { app.amenTool = 5; editAmenity(app, hp.first, hp.second, false); std::printf("home site at %d,%d: %zu sites, toast '%s'\n", hp.first, hp.second, app.homes.size(), app.toast.c_str()); }
    for (const auto& hp : unhomeAt) { const double c0 = app.econ.cash; editAmenity(app, hp.first, hp.second, true); std::printf("first click: '%s'\n", app.toast.c_str()); editAmenity(app, hp.first, hp.second, true); std::printf("second click: '%s', sites %zu, cash %.0f -> %.0f\n", app.toast.c_str(), app.homes.size(), c0, app.econ.cash); }
    for (int m = 0; m < monthsN; m++) { app.econ.day++; app.toastUntil = 0; stepHomes(app); }
    if (saveGameFile) { std::string err; if (!saveGame(app, saveGameFile, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; } std::printf("saved game %s\n", saveGameFile); if (!pngOut) return 0; }
    if (saveFile) {
        std::string err;
        if (!app.terrain.save(saveFile, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        std::printf("saved course %s\n", saveFile);
    }
    if (centerX >= 0) {
        app.camX = centerX * kTileSize - app.terrain.w * kTileSize * 0.5f + kTileSize * 0.5f;
        app.camZ = centerY * kTileSize - app.terrain.h * kTileSize * 0.5f + kTileSize * 0.5f;
    }

    bool editing = false;
    if (sgaTest >= 0 && app.uiOk) {
        refreshHoles(app); syncHoleStats(app);
        if (sgaTest >= 1) app.tourney.evaluateOffer(sgaInput(app));
        if (sgaTest == 3) {   // test only: pretend the course has 18 copies of the first hole and an ideal evaluation
            while (app.holes.size() < 18 && !app.holes.empty()) app.holes.push_back(app.holes[0]);
            sg::SgaInput ideal; ideal.holes = 18; ideal.totalYards = 6500; ideal.avgMinutes = 200; ideal.funPercent = 150; ideal.varietyHoles = 18; ideal.scenicHoles = 18; ideal.lengthHoles = 18; ideal.accuracyHoles = 18; ideal.imaginationHoles = 18; ideal.facilityKinds = 9;
            app.tourney.evaluateOffer(ideal); app.tourney.reopenOffer(ideal); runTournament(app);
        } else
        if (sgaTest == 2) { app.tourney.reopenOffer(sgaInput(app)); runTournament(app); } else openSgaScreen(app);
    }
    unsigned shownVersion = 0;
    reportCourse(app, true);
    bool running = true, dragging = false, needShot = pngOut != nullptr;
    double pausedTotal = 0, pauseStart = 0, lastTick = SDL_GetTicks() / 1000.0;
    int frames = 0;
    while (running) {
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (app.skOpen && app.screen == App::ScreenPlay && e.type != SDL_QUIT && skillsEvent(app, e)) continue;
            if (app.popKind && app.screen == App::ScreenPlay && e.type != SDL_QUIT && popEvent(app, e)) continue;
            if (app.screen != App::ScreenPlay && app.uiOk) {   // title menu and property chooser
                if (e.type == SDL_QUIT) running = false;
                else if (app.screen == App::ScreenAward) {
                    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_y || e.key.keysym.sym == SDLK_RETURN)) answerAward(app, true);
                    else if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_n || e.key.keysym.sym == SDLK_ESCAPE)) answerAward(app, false);
                    else if (e.type == SDL_MOUSEBUTTONDOWN) { const float vx = app.view.toVirtualX(e.button.x * app.dpi), vy = app.view.toVirtualY(e.button.y * app.dpi); if (vy >= 336 && vy < 366) { if (vx >= 260 && vx < 360) answerAward(app, true); else if (vx >= 440 && vx < 540) answerAward(app, false); } }
                }
                else if (app.screen == App::ScreenHoleStat) { if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenReport; app.hover = -1; } }
                else if (app.screen == App::ScreenReport) {
                    if (e.type == SDL_MOUSEBUTTONDOWN) {
                        const float my = app.view.toVirtualY((float)e.button.y); const int row = (int)((my - 108) / 22);
                        if (my >= 108 && row >= 0 && row < (int)app.holes.size() && app.hstats.size() == app.holes.size()) { app.hsHole = row; app.hsRot = (int)(app.srng.next() % 5); app.screen = App::ScreenHoleStat; }
                        else { app.screen = App::ScreenPlay; app.hover = -1; }
                    } else if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F1)) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenRoster) {
                    namespace ro = sg::ui_screens::roster;
                    const int total = (int)rosterRows(app).size(), mx = (int)app.vmx, my = (int)app.vmy;
                    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F9 || e.key.keysym.sym == SDLK_RETURN)) { app.screen = App::ScreenPlay; app.hover = -1; }
                    else if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_UP) app.rosterScroll = std::max(0, app.rosterScroll - 1);
                    else if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_DOWN) app.rosterScroll = std::min(std::max(0, total - ro::kVisibleRows), app.rosterScroll + 1);
                    else if (e.type == SDL_MOUSEWHEEL) app.rosterScroll = std::max(0, std::min(std::max(0, total - ro::kVisibleRows), app.rosterScroll - e.wheel.y));
                    else if (e.type == SDL_MOUSEBUTTONDOWN) {
                        const bool inUp = mx >= ro::upZone.x && mx < ro::upZone.x + ro::upZone.w && my >= ro::upZone.y && my < ro::upZone.y + ro::upZone.h;
                        const bool inDn = mx >= ro::downZone.x && mx < ro::downZone.x + ro::downZone.w && my >= ro::downZone.y && my < ro::downZone.y + ro::downZone.h;
                        const int rem = std::max(0, total - ro::kVisibleRows);
                        if (total > ro::kVisibleRows && inUp) app.rosterScroll = std::max(0, app.rosterScroll - std::max(1, std::min(3, app.rosterScroll)));
                        else if (total > ro::kVisibleRows && inDn) app.rosterScroll = std::min(rem, app.rosterScroll + std::max(1, std::min(3, rem - app.rosterScroll)));
                        else if (!(total > ro::kVisibleRows && mx >= 767)) { app.screen = App::ScreenPlay; app.hover = -1; }
                    }
                }
                else if (app.screen == App::ScreenKeys) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F8 || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenEoy) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; app.highlights.clear(); app.eoyBoard.clear(); if (app.top10Show) { app.top10Show = false; openTop10(app); } }
                }
                else if (app.screen == App::ScreenComments) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F2 || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenHisto) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F3 || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenBuyLand) {
                    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_RETURN)) { app.screen = App::ScreenPlay; app.hover = -1; }
                    else if (e.type == SDL_MOUSEBUTTONDOWN) {
                        const float vx = app.view.toVirtualX(e.button.x * app.dpi), vy = app.view.toVirtualY(e.button.y * app.dpi);
                        if (e.button.button == SDL_BUTTON_RIGHT || (vx >= 662 && vx < 726 && vy >= 533 && vy < 597)) { app.screen = App::ScreenPlay; app.hover = -1; }
                        else buyTract(app, blTractAt(vx, vy));
                    }
                }
                else if (app.screen == App::ScreenOverview) {
                    if (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F5 || e.key.keysym.sym == SDLK_RETURN)) { app.screen = App::ScreenPlay; app.hover = -1; }
                    else if (e.type == SDL_MOUSEBUTTONDOWN) overviewClick(app, app.view.toVirtualX(e.button.x * app.dpi), app.view.toVirtualY(e.button.y * app.dpi), e.button.button == SDL_BUTTON_RIGHT);
                }
                else if (app.screen == App::ScreenBoard) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F10 || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenFinance) {
                    if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F4 || e.key.keysym.sym == SDLK_RETURN)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; }
                }
                else if (app.screen == App::ScreenSga && app.sgaMode == 2 && g_revealForce < 0 && (SDL_GetTicks() / 1000.0 - app.tRevealStart) / 0.7 < (double)app.tPars.size() && (e.type == SDL_KEYDOWN || e.type == SDL_MOUSEBUTTONDOWN)) app.tRevealStart = -1e9;
                else if (app.screen == App::ScreenSga) {
                    if (e.type == SDL_KEYDOWN) {
                        const SDL_Keycode k = e.key.keysym.sym;
                        if (app.sgaMode == 1 && (k == SDLK_y || k == SDLK_RETURN)) runTournament(app);
                        else if (app.sgaMode == 1 && k == SDLK_n) { app.tourney.decline(); app.screen = App::ScreenPlay; }
                        else if (k == SDLK_ESCAPE || (app.sgaMode != 1 && (k == SDLK_RETURN || k == SDLK_F7))) { if (app.sgaMode == 1) app.tourney.decline(); app.screen = app.champ && app.sgaMode == 2 ? App::ScreenMenu : App::ScreenPlay; if (app.champ && app.sgaMode == 2) app.champ = false; app.hover = -1; }   // championship play ends at the results
                    } else if (e.type == SDL_MOUSEMOTION && app.sgaMode == 1) {
                        const float vx = app.view.toVirtualX(e.motion.x * app.dpi), vy = app.view.toVirtualY(e.motion.y * app.dpi); app.hover = -1;
                        for (int b = 0; b < 2; b++) { const SgaBtn r = sgaBtn(b); if (vx >= r.x && vx < r.x + r.w && vy >= r.y && vy < r.y + r.h) app.hover = b; }
                    } else if (e.type == SDL_MOUSEBUTTONDOWN) {
                        const float vx = app.view.toVirtualX(e.button.x * app.dpi), vy = app.view.toVirtualY(e.button.y * app.dpi);
                        if (app.sgaMode == 1) { for (int b = 0; b < 2; b++) { const SgaBtn r = sgaBtn(b); if (vx >= r.x && vx < r.x + r.w && vy >= r.y && vy < r.y + r.h) { if (b == 0) runTournament(app); else { app.tourney.decline(); app.screen = App::ScreenPlay; } } } }
                        else { app.screen = App::ScreenPlay; app.hover = -1; }
                    }
                }
                else if (app.screen == App::ScreenCustomise) customiseEvent(app, e);
                else if (app.screen == App::ScreenGolfer) golferCardEvent(app, e);
                else if (app.screen == App::ScreenPair) pairEvent(app, e);
                else if (app.screen >= App::ScreenDiff) t2Event(app, e, running);
                else if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_ESCAPE) { if (app.switchMode) { app.switchMode = false; app.screen = App::ScreenPlay; } else if (app.screen == App::ScreenMenu) running = false; else app.screen = App::ScreenMenu; app.hover = -1; }
                else if (e.type == SDL_MOUSEMOTION || (e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT)) {
                    const bool click = e.type == SDL_MOUSEBUTTONDOWN;
                    const float vx = app.view.toVirtualX((click ? e.button.x : e.motion.x) * app.dpi), vy = app.view.toVirtualY((click ? e.button.y : e.motion.y) * app.dpi);
                    int hit = -1;
                    if (app.screen == App::ScreenMenu) { for (int b = 0; b < 6; b++) if (kMenuBtn[b].has(vx, vy)) hit = b; if (click && hit < 0 && Rect{170, 190, 480, 165}.has(vx, vy)) { creditsOpen(app); app.hover = -1; continue; } }   // PLACEHOLDER: the credits are opened by a click on the logo
                    else {
                        for (int p = 0; p < 16; p++) if (propertyCard(p).has(vx, vy)) hit = p;
                        if (std::hypot(vx - 768, vy - 557) < 25) hit = 100;   // Cancel, Reset or Save and Load: the exe's circular hit areas
                        else if (std::hypot(vx - 760, vy - 476) < 25) hit = 101;
                        else if (std::hypot(vx - 774, vy - 425) < 20) hit = 102;
                        if (app.confirmIdx >= 0) { hit = Rect{215, 308, 180, 30}.has(vx, vy) ? 110 : Rect{405, 308, 180, 30}.has(vx, vy) ? 111 : -1; }
                        if (app.goTarget >= 0) hit = -1;
                    }
                    app.hover = hit;
                    if (click && app.screen == App::ScreenProperty && app.goTarget < 0 && app.worldMsgUntil > SDL_GetTicks() / 1000.0) app.worldMsgUntil = 0;   // any click closes the red box
                    if (click && hit >= 0 && app.screen == App::ScreenProperty && (hit >= 101 || app.confirmIdx >= 0)) {
                        if (app.confirmIdx >= 0) {
                            if (hit == 110) { app.goTarget = app.confirmIdx; app.goFrames = 2; }
                            app.confirmIdx = -1; app.hover = -1;
                        } else if (hit == 101 && !app.switchMode) { worldReset(app, (uint64_t)SDL_GetTicks() * 2654435761ull + app.seed); snd(app, "Interface/Button1.wav"); }
                        else if (hit == 101) { std::string err; toastMsg(app, saveGame(app, app.courseFile, err) ? "Game Saved." : "Could not save the game"); snd(app, "Interface/Button1.wav"); }
                        else if (hit == 102) {
                            std::string err;
                            if (loadGame(app, app.courseFile, err)) { app.switchMode = false; std::printf("loaded %s\n", app.courseFile.c_str()); } else toastMsg(app, "No saved game found (course.sgc)");
                        }
                    } else if (click && hit >= 0) {
                        if (app.screen == App::ScreenMenu) {
                            if (hit == 0) { t2LoadArt(app); app.t2Mode = 0; t2ListSaves(app); app.t2Sel = -1; app.t2Scroll = 0; app.t2Confirm = 0; app.t2Hover = -1; app.screen = App::ScreenLoad; app.hover = -1; }
                            else if (hit == 1 || hit == 2) { t2LoadArt(app); app.diffPending = hit; app.t2Hover = -1; app.screen = App::ScreenDiff; app.hover = -1; }
                            else if (hit == 3) { t2LoadArt(app); app.t2Hover = -1; app.screen = App::ScreenThemes; app.hover = -1; }
                            else if (hit == 4) { t2LoadArt(app); app.t2Mode = 1; app.champ = false; t2ListFolder(app, "Championship", ".cse"); app.t2Sel = -1; app.t2Scroll = 0; app.t2Confirm = 0; app.t2Hover = -1; app.screen = App::ScreenLoad; app.hover = -1; }
                            else if (hit == 5) running = false;
                        } else {
                            if (app.switchMode) {
                                if (hit == 100 || hit == app.curProp) { app.switchMode = false; app.screen = App::ScreenPlay; app.hover = -1; }
                                else if (canAfford(app, hit)) { if (app.careerOwned >> hit & 1) { switchCourse(app, hit); app.hover = -1; } else app.confirmIdx = hit; }
                                else { app.worldMsg = "You need more money before you can purchase this property."; app.worldMsgUntil = SDL_GetTicks() / 1000.0 + 4; }
                            }
                            else if (hit == 100) { app.screen = App::ScreenMenu; app.hover = -1; }
                            else if (canAfford(app, hit)) app.confirmIdx = hit;
                            else { app.worldMsg = "You need more money before you can purchase this property."; app.worldMsgUntil = SDL_GetTicks() / 1000.0 + 4; }
                        }
                    }
                }
                continue;
            }
            if (e.type == SDL_QUIT) running = false;
            else if (e.type == SDL_KEYDOWN && app.tutPage >= 0) { tutNext(app, e.key.keysym.sym == SDLK_ESCAPE); }
            else if (e.type == SDL_KEYDOWN) {
                float step = 60.0f / app.zoom;
                // Hotkeys of the original (manual p. 3 and 4): Z / X zoom, Shift+S / Shift+L save and load, Shift+P pause,
                // Shift+T trees; in edit mode F fairway, G green/tee, R rough, S sandtrap, W water, P pathway, - lower, = raise.
                {
                    const SDL_Keycode k = e.key.keysym.sym;
                    const bool shift = (SDL_GetModState() & KMOD_SHIFT) != 0;
                    bool handled = true;
                    auto pick = [&](int type, int vb) { app.tool = 0; for (int i = 0; i < kPaintCount; i++) if (kPaint[i].type == type && kPaint[i].vbyte == vb) app.paintIdx = i; setTitle(app, win); };
                    if (shift && k == SDLK_s) { std::string err; if (saveGame(app, app.courseFile, err)) std::printf("saved %s\n", app.courseFile.c_str()); else std::fprintf(stderr, "error: %s\n", err.c_str()); }
                    else if (shift && k == SDLK_l) { std::string err; if (loadGame(app, app.courseFile, err)) { std::printf("loaded %s\n", app.courseFile.c_str()); } else std::fprintf(stderr, "error: %s\n", err.c_str()); }
                    else if (shift && k == SDLK_p) { app.paused = !app.paused; if (app.paused) pauseStart = SDL_GetTicks() / 1000.0; else pausedTotal += SDL_GetTicks() / 1000.0 - pauseStart; }
                    else if (shift && k == SDLK_t) app.showProps = !app.showProps;
                    else if (shift && k == SDLK_n) app.showNames = !app.showNames;
                    else if ((shift || (SDL_GetModState() & KMOD_CTRL)) && (k == SDLK_c || k == SDLK_r || k == SDLK_g || k == SDLK_v)) {
                        // Hire (Shift) or fire (Ctrl) a Club Pro, Ranger, Groundskeeper or Soda Vendor.
                        const int kind = k == SDLK_c ? Economy::ClubPro : k == SDLK_r ? Economy::Ranger : k == SDLK_g ? Economy::Groundskeeper : Economy::SodaVendor;
                        bool ok = shift ? app.econ.hire(kind) : app.econ.fire(kind);
                        std::printf("%s %s: %s (staff now %d, wages $%.0f a day)\n", shift ? "hire" : "fire", Economy::staffName(kind), ok ? "done" : "not possible", app.econ.staffCount(), app.econ.dailyWages());
                        snd(app, "Interface/Button2.wav"); setTitle(app, win);
                    }
                    else if (shift && k == SDLK_k && app.screen == App::ScreenPlay && !app.edit) { std::string err; const std::string nm = app.courseName.empty() ? std::string("Course") : app.courseName; toastMsg(app, champSave(app, nm, err) ? nm + " saved for championship play." : "Could not save the course for championship play"); }   // PLACEHOLDER key: the exe has a menu item
                    else if (k == SDLK_F1) { reportCourse(app, true); if (app.uiOk && app.reportArt.tex) { app.ratings.clear(); app.screen = App::ScreenReport; } }
                    else if (k == SDLK_F7 && app.uiOk) openSgaScreen(app);
                    else if (k == SDLK_F8 && shift && app.uiOk && app.screen == App::ScreenPlay) tutStart(app);
                    else if (k == SDLK_F6 && app.uiOk && app.screen == App::ScreenPlay && !app.edit) { app.switchMode = true; app.screen = App::ScreenProperty; app.hover = -1; }
                    else if ((k == SDLK_F5 || (k == SDLK_r && shift && !app.edit)) && app.uiOk && app.screen == App::ScreenPlay) { app.ovMode = -1; app.ovSel = -1; app.screen = App::ScreenOverview; }
                    else if (k == SDLK_b && shift && app.uiOk && app.screen == App::ScreenPlay) openBuyLand(app);
                    else if (k == SDLK_F10 && app.uiOk) app.screen = App::ScreenBoard;
                    else if (k == SDLK_BACKQUOTE && app.uiOk && app.screen == App::ScreenPlay) pairOpen(app);   // PLACEHOLDER key for the pair screen
                    else if (k == SDLK_F12 && app.uiOk && app.screen == App::ScreenPlay) openBest(app);   // PLACEHOLDER key: the exe reaches it from its Information menu
                    else if (k == SDLK_F11 && app.uiOk && app.screen == App::ScreenPlay) openTop10(app);   // PLACEHOLDER key: the exe reaches the Top 10 from its Information menu
                    else if (k == SDLK_F3 && app.uiOk && app.screen == App::ScreenPlay) app.screen = App::ScreenHisto;
                    else if (k == SDLK_F2 && app.uiOk) { syncHoleStats(app); app.screen = App::ScreenComments; }
                    else if (k == SDLK_F4 && app.uiOk) app.screen = App::ScreenFinance;
                    else if (k == SDLK_F8 && app.uiOk) app.screen = App::ScreenKeys;
                    else if (k == SDLK_F9 && app.uiOk) { app.rosterScroll = 0; app.screen = App::ScreenRoster; }
                    else if (k == SDLK_z) app.zoom *= 1.12f;
                    else if (k == SDLK_x) app.zoom /= 1.12f;
                    else if (app.edit && !shift && k == SDLK_f) pick(TT_Fairway, 0);
                    else if (app.edit && !shift && k == SDLK_g) pick(kPaint[app.paintIdx].type == TT_PuttingGreen && app.tool == 0 ? TT_Tee : TT_PuttingGreen, 0);
                    else if (app.edit && !shift && k == SDLK_r) pick(TT_Rough, 0);
                    else if (app.edit && !shift && k == SDLK_s) pick(7, 0);
                    else if (app.edit && !shift && k == SDLK_w) pick(TT_WaterShallow, 0);
                    else if (app.edit && !shift && k == SDLK_p) { app.tool = 2; setTitle(app, win); }
                    else if (app.edit && !shift && k == SDLK_MINUS) { app.tool = 1; app.raiseSign = -1; setTitle(app, win); }
                    else if (app.edit && !shift && k == SDLK_EQUALS) { app.tool = 1; app.raiseSign = 1; setTitle(app, win); }
                    else handled = false;
                    if (handled) continue;
                }
                switch (e.key.keysym.sym) {
                    case SDLK_ESCAPE: if (app.uiOk) { app.screen = App::ScreenMenu; app.hover = -1; } else running = false; break;
                    case SDLK_LEFT: case SDLK_a: pan(app, -step, 0); break;
                    case SDLK_RIGHT: case SDLK_d: pan(app, step, 0); break;
                    case SDLK_UP: case SDLK_w: pan(app, 0, step); break;
                    case SDLK_DOWN: case SDLK_s: pan(app, 0, -step); break;
                    case SDLK_q: app.rot -= 5; break;
                    case SDLK_e: app.rot += 5; break;
                    case SDLK_EQUALS: case SDLK_PLUS: case SDLK_KP_PLUS: app.zoom *= 1.12f; break;
                    case SDLK_MINUS: case SDLK_KP_MINUS: app.zoom /= 1.12f; break;
                    case SDLK_1: case SDLK_2: case SDLK_3: case SDLK_4: loadTheme(app, e.key.keysym.sym - SDLK_1); break;
                    case SDLK_r: app.seed = app.seed * 1664525u + 1013904223u; app.terrain = Terrain::demoCourse(40, 40, app.seed); rebuildBatches(app); populateProps(app); break;
                    case SDLK_TAB: app.edit = !app.edit; setTitle(app, win); break;
                    case SDLK_t: app.tool = (app.tool + 1) % 5; setTitle(app, win); break;
                    case SDLK_LEFTBRACKET: if (app.tool == 4) { for (int k = 0; k < kBuildCount; k++) { app.buildIdx = (app.buildIdx + kBuildCount - 1) % kBuildCount; if (themeHas(app, app.buildIdx)) break; } setTitle(app, win); break; } if (app.tool == 3) break; if (app.tool == 2) { app.pathKind = 3 - app.pathKind; setTitle(app, win); break; } app.paintIdx = (app.paintIdx + kPaintCount - 1) % kPaintCount; setTitle(app, win); break;
                    case SDLK_RIGHTBRACKET: if (app.tool == 4) { for (int k = 0; k < kBuildCount; k++) { app.buildIdx = (app.buildIdx + 1) % kBuildCount; if (themeHas(app, app.buildIdx)) break; } setTitle(app, win); break; } if (app.tool == 3) break; if (app.tool == 2) { app.pathKind = 3 - app.pathKind; setTitle(app, win); break; } app.paintIdx = (app.paintIdx + 1) % kPaintCount; setTitle(app, win); break;
                    case SDLK_COMMA: app.brush = std::max(0, app.brush - 1); setTitle(app, win); break;
                    case SDLK_PERIOD: app.brush = std::min(6, app.brush + 1); setTitle(app, win); break;
                    case SDLK_F5: { std::string err; if (app.terrain.save(app.courseFile, err)) { std::printf("saved %s\n", app.courseFile.c_str()); snd(app, "Interface/Button1.wav"); } else std::fprintf(stderr, "error: %s\n", err.c_str()); break; }
                    case SDLK_F9: { std::string err; if (Terrain::load(app.courseFile, app.terrain, err)) { app.dirty = true; std::printf("loaded %s\n", app.courseFile.c_str()); } else std::fprintf(stderr, "error: %s\n", err.c_str()); break; }
                    case SDLK_m: toggleMusic(app); break;
                    case SDLK_n:
                        app.mute = !app.mute;
                        if (app.mute) { if (app.mixer) app.mixer->stopAll(); app.ambience = app.music = -1; } else { startAmbience(app); if (app.musicOn) { app.musicOn = false; toggleMusic(app); } }
                        break;
                    case SDLK_p: app.showProps = !app.showProps; break;
                    case SDLK_h: openHole(app); break;
                    case SDLK_SLASH: { refreshHoles(app); int hi = 0; float gx, gz; int mx, my; SDL_GetMouseState(&mx, &my); if (pickGround(app, mx, my, app.dpi, gx, gz)) hi = std::max(0, holeNearGround(app, gx, gz)); if (app.anHole >= 0 && app.anHole == hi) app.anHole = -1; else shotAnalysisRun(app, hi); break; }   // instant shot analysis (docs/UI_SCREENS.md)
                    case SDLK_F1: app.showAdvisor = !app.showAdvisor; break;
                    case SDLK_f: app.follow = !app.follow; break;
                    case SDLK_F2: if (screenshot(app, "simgolf-shot.png")) std::printf("saved simgolf-shot.png\n"); break;
                    default: break;
                }
            } else if (e.type == SDL_MOUSEWHEEL) {
                app.zoom *= e.wheel.y > 0 ? 1.1f : (e.wheel.y < 0 ? 1 / 1.1f : 1.0f);
            } else if (e.type == SDL_MOUSEBUTTONDOWN && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)
                       && app.uiOk && dockClick(app, app.view.toVirtualX(e.button.x * app.dpi), app.view.toVirtualY(e.button.y * app.dpi), e.button.button == SDL_BUTTON_RIGHT, app.pauseToggle)) {
                setTitle(app, win);
            } else if (e.type == SDL_MOUSEBUTTONDOWN && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)) {
                if (app.anArm && e.button.button == SDL_BUTTON_LEFT && app.screen == App::ScreenPlay) {   // Analyze Golf Shot: the clicked ground picks the hole
                    float gx, gz; if (pickGround(app, e.button.x, e.button.y, app.dpi, gx, gz)) shotAnalysisRun(app, holeNearGround(app, gx, gz));
                    continue;
                }
                if (app.empMoveArm >= 0 && e.button.button == SDL_BUTTON_LEFT && app.screen == App::ScreenPlay) {   // Move: the next map click sends the employee there
                    float mx2, mz2; const std::vector<EmpRow> rows = empRows(app);
                    if (app.empMoveArm < (int)rows.size() && pickGround(app, e.button.x, e.button.y, app.dpi, mx2, mz2)) { const int ei = rowEmp(app, rows, app.empMoveArm); if (ei >= 0) { app.emps[(size_t)ei].wx = mx2; app.emps[(size_t)ei].wz = mz2; app.emps[(size_t)ei].x = mx2; app.emps[(size_t)ei].z = mz2; } }
                    app.empMoveArm = -1; continue;
                }
                int pg = -1; if (!app.edit && app.uiOk && app.screen == App::ScreenPlay && e.button.button == SDL_BUTTON_LEFT) pg = pickGolfer(app, (float)e.button.x, (float)e.button.y);
                if (pg >= 0) openGolferCard(app, pg);
                else if (app.edit && e.button.button == SDL_BUTTON_LEFT) { editing = true; app.lastCell = -1; applyTool(app, (SDL_GetModState() & KMOD_SHIFT) != 0, false); }
                else dragging = true;
            }
            else if (e.type == SDL_MOUSEBUTTONUP && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)) { dragging = false; editing = false; }
            else if (e.type == SDL_MOUSEMOTION && app.uiOk && (app.dockHover = -2, dockHoverUpdate(app, app.view.toVirtualX(e.motion.x * app.dpi), app.view.toVirtualY(e.motion.y * app.dpi)), false)) {}
            else if (e.type == SDL_MOUSEMOTION && editing) {
                app.hasHit = pickGround(app, e.motion.x, e.motion.y, app.dpi, app.hitX, app.hitZ);
                applyTool(app, (SDL_GetModState() & KMOD_SHIFT) != 0, true);
            }
            else if (e.type == SDL_MOUSEMOTION && dragging) {
                int ww, wh; SDL_GetWindowSize(win, &ww, &wh);
                (void)ww;
                float k = 2.0f / app.zoom;  // window points -> world units
                float sinP = (float)std::sin(pitchFor(app.drawW, app.drawH) * 3.14159265 / 180.0);
                pan(app, -e.motion.xrel * k, e.motion.yrel * k / sinP);
            }
        }
        if (app.pauseToggle) { app.pauseToggle = false; app.paused = !app.paused; if (app.paused) pauseStart = SDL_GetTicks() / 1000.0; else pausedTotal += SDL_GetTicks() / 1000.0 - pauseStart; }
        if (!app.storyLines.empty() && !app.paused && SDL_GetTicks() / 1000.0 > app.storyNext) { app.storyPos++; app.storyNext = SDL_GetTicks() / 1000.0 + 7.0; if (app.storyPos >= app.storyLines.size() && golfersOnCourse(app) >= 2) storyHappyEnding(app); }
        if (app.edit) { int mx, my; SDL_GetMouseState(&mx, &my); app.hasHit = pickGround(app, mx, my, app.dpi, app.hitX, app.hitZ); }
        if (app.dirty) { rebuildBatches(app); refreshTrees(app); app.econ.updateUpkeep(app.terrain); app.dirty = false; reportCourse(app, false); setTitle(app, win); }
        if (app.econ.version != shownVersion) { shownVersion = app.econ.version; setTitle(app, win); }
        {
            const double now = SDL_GetTicks() / 1000.0;
            if (app.resetClock) { pausedTotal = now; if (!pngOut) app.time = 0; app.resetClock = false; }
            if (app.screen != App::ScreenPlay && !app.paused) pausedTotal += now - lastTick;   // the club does not run while a menu is open
            lastTick = now;
            if (!pngOut && !app.paused && app.screen == App::ScreenPlay) app.time = now - pausedTotal;
        }
        SDL_GL_GetDrawableSize(win, &app.drawW, &app.drawH);
        { int ww, wh; SDL_GetWindowSize(win, &ww, &wh); app.dpi = ww > 0 ? (float)app.drawW / (float)ww : 1.0f; }
        if (app.goTarget >= 0 && app.screen == App::ScreenProperty && app.goFrames-- <= 0) {   // after the "off to" screen has been drawn
            const int t = app.goTarget; app.goTarget = -1; app.hover = -1;
            if (app.switchMode) switchCourse(app, t);
            else { chrInit(app); startGame(app, t, app.sandboxChoice); if (!app.sandboxChoice) { int sum = 0; for (int i = 0; i < 10; i++) sum += app.chr.skills[i]; skillsOpen(app, 10 - sum); } }
        }
        if (app.screen == App::ScreenMenu && app.uiOk) drawMenu(app);
        else if (app.screen == App::ScreenProperty && app.uiOk) drawProperty(app);
        else if (app.screen == App::ScreenCustomise && app.uiOk) { app.cuFrames++; drawCustomise(app); }
        else if (app.screen == App::ScreenDiff && app.uiOk) drawDiff(app);
        else if (app.screen == App::ScreenThemes && app.uiOk) drawThemes(app);
        else if ((app.screen == App::ScreenLoad || app.screen == App::ScreenPro) && app.uiOk) drawLoad(app);
        else if (app.screen == App::ScreenPair && app.uiOk) drawPair(app);
        else if (app.screen == App::ScreenCredits && app.uiOk) drawCredits(app);
        else if (app.screen == App::ScreenTop10 && app.uiOk) drawTop10(app);
        else { if (app.testBoard > 0 && --app.testBoard == 0) app.screen = App::ScreenBoard; render(app); if (app.snapPending >= 0) captureSnap(app); if (!app.thumbReq.empty()) captureThumb(app); drawHud(app); if (app.screen == App::ScreenReport) drawReport(app); if (app.screen == App::ScreenSga) drawSga(app); if (app.screen == App::ScreenBuyLand) drawBuyLand(app); if (app.screen == App::ScreenOverview) drawOverview(app); if (app.screen == App::ScreenFinance) drawFinance(app); if (app.screen == App::ScreenRoster) drawRoster(app); if (app.screen == App::ScreenHoleStat) drawHoleStat(app); if (app.screen == App::ScreenKeys) drawKeys(app); if (app.screen == App::ScreenEoy) drawEoy(app); if (app.screen == App::ScreenComments) drawComments(app); if (app.screen == App::ScreenHisto) drawHisto(app); if (app.screen == App::ScreenBoard) drawBoard(app); if (app.screen == App::ScreenAward) drawAward(app); if (app.screen == App::ScreenGolfer) { app.cardFrames++; drawGolferCard(app); } if (app.screen == App::ScreenBest) drawBest(app); }
        if (getenv("SG_PICKTEST") && frames == 1) {  // project known ground points to the screen and pick them back
            float worst = 0;
            for (float wx : {-600.f, 0.f, 750.f}) for (float wz : {-500.f, 100.f, 900.f}) {
                float y = app.terrain.heightAt(wx, wz);
                double ex = app.mv[0] * wx + app.mv[4] * y + app.mv[8] * wz + app.mv[12], ey = app.mv[1] * wx + app.mv[5] * y + app.mv[9] * wz + app.mv[13];
                int sx = (int)std::lround((ex / app.upp + app.drawW * 0.5) / app.dpi), sy = (int)std::lround((app.drawH * 0.5 - ey / app.upp) / app.dpi);
                float px, pz;
                bool ok = pickGround(app, sx, sy, app.dpi, px, pz);
                float err = ok ? std::hypot(px - wx, pz - wz) : 9999;
                worst = std::max(worst, err); std::printf("  (%.0f,%.0f) y=%.1f -> %.1f,%.1f err %.1f\n", wx, wz, y, px, pz, err);
            }
            std::printf("pick test: worst error %.2f world units (1 px = %.2f)\n", worst, app.upp);
        }
        if (needShot && ++frames >= 2) {  // let the first frame settle
            if (g_cardHook >= 0) { openGolferCard(app, g_cardHook); app.cardSkills = g_cardSkills; g_cardHook = -1; if (g_cardHover >= 0) { app.cardHover = g_cardHover; app.cardFrames = 20; app.cardMx = kCardBtnCx[g_cardHover]; app.cardMy = 0x106; } frames = 0; SDL_GL_SwapWindow(win); continue; }
            if (saveLateFile) { std::string err; if (!saveGame(app, saveLateFile, err)) std::fprintf(stderr, "error: %s\n", err.c_str()); else std::printf("saved game %s\n", saveLateFile); saveLateFile = nullptr; }
            if (!screenshot(app, pngOut)) { std::fprintf(stderr, "could not write %s\n", pngOut); return 1; }
            std::printf("saved %s (%dx%d)\n", pngOut, app.drawW, app.drawH);
            break;
        }
        SDL_GL_SwapWindow(win);
    }
    SDL_GL_DeleteContext(ctx);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}

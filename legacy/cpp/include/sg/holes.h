// SimGolf native port: course analysis from the original manual's rules (pathways, SGA hole classes).
//
// The manual (docs/MANUAL_NOTES.md) says: most buildings must be joined to the Clubhouse by an unbroken pathway, and the Sim Golf
// Association classes each hole as Breather, Freeway, Precise, Creative, Challenge, Heroic, Strategic or Classic by which of the three
// skills it exercises (length, accuracy, imagination). It does NOT say how a hole's demands are measured, so the measures below are
// PLACEHOLDERS of my own, chosen to be explainable: the real rules are in golf.exe, which this project does not read.
#pragma once
#include <string>
#include <vector>
#include "sg/terrain.h"

namespace sg {

// One flag per tile (w*h): 1 when a path tile is joined to the clubhouse lot by an unbroken chain of path tiles. A path tile touching a
// Building tile (4-neighbourhood) is the start of a chain. Paths that are not joined appear as mud tracks in the viewer (manual p. 18).
std::vector<uint8_t> pathsConnectedToClubhouse(const Terrain& t);

struct HoleInfo {
    bool valid = false;
    float teeX = 0, teeZ = 0, holeX = 0, holeZ = 0;
    float length = 0;            // world units, straight line tee to hole
    int par = 0;                 // PLACEHOLDER thresholds on length
    int hazardsOnLine = 0;       // distinct hazard stretches the straight line crosses
    int hazardsNearLine = 0;     // hazard tiles within two tiles of the line, not on it
    bool length_ = false, accuracy = false, imagination = false;   // the three demands
    const char* cls = "Breather";
    std::string report() const;  // one line, for the console
};

// Analyses the demo hole (the route stored in Terrain::path: first point the tee, last point the hole).
// PLACEHOLDER rules: length demand when the line is longer than 1200 units; accuracy demand when 8 or more hazard tiles lie
// within two tiles of the line; imagination demand when the line itself crosses a hazard (the golfer has to shape or flight the shot).
HoleInfo analyzeHole(const Terrain& t);

// The course's holes, found from the painted terrain: each cluster of Tee tiles is paired with the nearest unused cluster of Putting
// Green tiles, in reading order of the tees (top row first). The route is a straight line, except that when the course has a stored
// route (Terrain::path, the generated demo hole) that begins and ends at the same tee and green, that curved route is used.
// PLACEHOLDER rule: the original numbers holes with a tool, this one reads the terrain.
struct HoleRoute {
    float teeX = 0, teeZ = 0, greenX = 0, greenZ = 0;
    float length = 0;
    int par = 4;                      // PLACEHOLDER thresholds on length (3 under 1300 units, 4 under 2600, else 5)
    std::vector<float> route;         // x,z pairs from tee to green
};
std::vector<HoleRoute> findHoles(const Terrain& t);

// Course report ratings. The original's Shot Analysis (see docs/SCREENSHOT_NOTES.md) compares golfers with every skill against golfers
// missing one skill; this does the same with the placeholder shot model: `samples` simulated rounds of the hole per variant, and the
// score is how many strokes worse the golfer lacking the skill averages. A hole 'demands' a skill when its score reaches 0.30 strokes
// (PLACEHOLDER). Draw, fade and backspin are not modelled by the shot model, so Imagination is always 0.
struct HoleRating {
    float len = 0, acc = 0, img = 0;
    float avgStrokes = 0;          // for the full skill golfer
    float avgDrive = 0;            // world units carried by the first shot (placeholder model)
    const char* type = "Breather";
    int typeIndex = 0;             // L + 2A + 4I, the index into the class names
};
HoleRating rateHole(const Terrain& t, const HoleRoute& r, int samples = 40, int difficulty = 1);

}  // namespace sg

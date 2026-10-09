// SimGolf native port: a first slice of game logic, one golfer playing one hole.
//
// This is NOT a reimplementation of the original rules: those live in the protected golf.exe, which this
// project does not read. It is a self-contained demonstration of the loop (walk, address, swing, ball
// flight, lie, putt) driven by the sprite timings found in the data files, so the pieces the viewer needs
// (animation states, facing, ball position) exist. Distances, dispersion and penalties are placeholders.
#pragma once
#include <vector>
#include "sg/terrain.h"
#include "sg/lie.h"

namespace sg {

enum class GolferAnim { Walk, Address, Swing, PuttAddress, Putt, Happy };

// Sprite timings, from the FLC files (frames per view x 83 ms): Male*_PerfectSwing 20 frames, ball struck
// near frame 11 (club horizontal in front of the golfer); *_NormalAddress 18 frames; *_Putt 33 frames.
constexpr float kFrameSec = 0.083f;
constexpr float kSwingSec = 20 * kFrameSec, kSwingImpactSec = 11 * kFrameSec;
constexpr float kAddressSec = 18 * kFrameSec;
constexpr float kPuttSec = 33 * kFrameSec, kPuttImpactSec = 12 * kFrameSec;

// Skill levels 0..15 in the order of progolfers.dta. The NAMES are from the data file; what each level DOES here
// is a placeholder (see docs/GAMELOGIC.md). Draw, fade and backspin are not modelled yet.
struct GolferSkills {
    enum { Power, LongDriver, AccDriver, AccIrons, AccPutter, Draw, Fade, Backspin, Recovery, Luck };
    int v[10] = {7, 7, 7, 7, 7, 7, 7, 7, 7, 7};
};

struct ShotSim {
    float paceScale = 1.0f;             // walking speed multiplier (a Ranger speeds play up)
    float hold = 0;                     // seconds the golfer is held in place (an employee has stopped them)
    float spreadDivisor = 1.0f;         // Pro Shop: error spread of accurate golfers is divided by this (relative to the level 0 shop)
    bool washerAtTee = false;           // a ball washer stands within 3 tiles of this hole's tee (docs/DECODE_WORLD2.md 2.1); the viewer sets it per hole
    bool washed = false;                // the ball was washed: the next shot's sideways error loses a third; cleared when the ball stops anywhere but on fairway
    float driveBonus = 0.0f;            // Driving Range: extra carry (world units) for long hitters on drives
    GolferSkills skills;                // set before init(); init() keeps them
    int shape = 0;                      // the shot shape the player picked on the Player panel: 0 straight, 1 fade L to R, 2 draw R to L, 3 high backspin, 4 low punch (effects are PLACEHOLDER)
    // Outputs, read by the viewer every frame.
    GolferAnim anim = GolferAnim::Walk;
    float animTime = 0;                 // seconds since the animation started
    float golferX = 0, golferZ = 0;     // world position of the golfer's feet
    float golferHeading = 0;            // degrees, atan2(z, x), the direction the body faces
    float ballX = 0, ballZ = 0, ballH = 0;  // ballH = height above the ground
    int stroke = 0;
    const char* club = "";             // club of the latest stroke: "drive", "iron" or "putt"
    const char* event = "";             // short text of the latest event (for the log / window title)

    // Reaction hooks (read by the viewer): a plan is counted when the golfer settles at the ball, a landing when a full shot comes down.
    int planCount = 0; bool planPutt = false; float planFromX = 0, planFromZ = 0, planAim = 0, planDist = 0;   // aim in degrees, distance in world units
    int landCount = 0, landType = -1, landFromType = -1; bool landWater = false, landOut = false, landCloser = false;   // TileType values, -1 off the map
    // Flight hooks: a tree or building hit during flight (docs/DECODE_EVENTS_SHOTS.md 3.1) and the sideways error of the shot (PLACEHOLDER curve for hook and slice).
    int theme = 0;   // course theme 0..3, picks the tree bands
    int obsCount = 0, obsType = -1;   // TileType of the tile whose tree or building was hit
    float landDev = 0;   // degrees between the aim and the actual heading of the latest full shot
    bool walking() const { return phase == Phase::Walk; }

    // The route of the hole being played (x,z pairs, first the tee, last the green). Defaults to Terrain::path.
    void setRoute(const std::vector<float>* r) { route_ = r; }
    bool loop = true;                   // start over after the hole (the viewer turns this off to move to the next hole)
    bool finished = false;              // set after the celebration when loop is false
    void init(const Terrain& t, uint32_t seed);
    void step(float dt);                // advance the simulation

  private:
    enum class Phase { Walk, Address, Swing, Flight, Settle, PuttAddress, Putt, Roll, Celebrate, Pause } phase = Phase::Pause;
    const Terrain* t_ = nullptr;
    const std::vector<float>* route_ = nullptr;
    uint32_t rng_ = 1;
    float phaseTime_ = 0;
    float shotFromX_ = 0, shotFromZ_ = 0, landX_ = 0, landZ_ = 0, flightSec_ = 1, flightPeak_ = 0, aimHeading_ = 0;
    bool struck_ = false, missed_ = false, hitObstacle_ = false;
    float flightFromX_ = 0, flightFromZ_ = 0, fallFrom_ = 0, tickAcc_ = 0;
    std::vector<float> trajX_, trajZ_, trajH_;   // the ball's path tick by tick (docs/DECODE_PLAYCORE.md section 3)
    int trajHitTick_ = -1, trajHitType_ = -1, trajLandTile_ = -1, trajRestTile_ = -1; bool trajWater_ = false, trajOob_ = false, trajCounted_ = false;
    float rnd();                        // 0..1
    void setPhase(Phase p) { phase = p; phaseTime_ = 0; }
    void aimAtGreen();
    float holeX() const, holeZ() const;
    float distToHole() const;
};

}  // namespace sg

#include "sg/shot.h"
#include "sg/flight.h"
#include "sg/ballphys.h"
#include <algorithm>
#include <cmath>

namespace sg {

static const float kPi = 3.14159265f;
static const float kTileSizeWorld = kTileSize;   // world units per tile; the exe uses 1024
static const float kWalkSpeed = 130.0f;      // world units per second (placeholder)
static const float kDriveDist = 900.0f;     // placeholder carry for a full swing
static const float kPuttRange = 230.0f;     // switch to the putter inside this distance of the hole

float ShotSim::rnd() { rng_ ^= rng_ << 13; rng_ ^= rng_ >> 17; rng_ ^= rng_ << 5; return (rng_ & 0xFFFFFF) / float(0x1000000); }
float ShotSim::holeX() const { return (*route_)[route_->size() - 2]; }
float ShotSim::holeZ() const { return (*route_)[route_->size() - 1]; }
float ShotSim::distToHole() const { return std::hypot(holeX() - ballX, holeZ() - ballZ); }

void ShotSim::init(const Terrain& t, uint32_t seed) {
    t_ = &t;
    if (!route_) route_ = &t.path;
    finished = false;
    rng_ = seed ? seed : 1u;
    stroke = 0; washed = false;
    ballX = route_->size() >= 2 ? (*route_)[0] : 0; ballZ = route_->size() >= 2 ? (*route_)[1] : 0; ballH = 0;
    golferX = ballX; golferZ = ballZ;
    event = "on the tee";
    setPhase(Phase::Walk);
    anim = GolferAnim::Walk;
}

// Aim at the point on the course route about one drive further on than the ball.
void ShotSim::aimAtGreen() {
    const auto& P = *route_;
    size_t n = P.size() / 2, best = 0;
    float bd = 1e30f;
    for (size_t i = 0; i < n; i++) {
        float d = std::hypot(P[2 * i] - ballX, P[2 * i + 1] - ballZ);
        if (d < bd) { bd = d; best = i; }
    }
    float acc = 0;
    size_t i = best;
    while (i + 1 < n && acc < kDriveDist) { acc += std::hypot(P[2 * i + 2] - P[2 * i], P[2 * i + 3] - P[2 * i + 1]); i++; }
    float tx = P[2 * i], tz = P[2 * i + 1];
    if (distToHole() < kDriveDist) { tx = holeX(); tz = holeZ(); }
    aimHeading_ = std::atan2(tz - ballZ, tx - ballX) * 180.0f / kPi;
}

void ShotSim::step(float dt) {
    if (!t_ || !route_ || route_->size() < 4) return;
    if (hold > 0) { hold -= dt; return; }
    phaseTime_ += dt;
    animTime += dt;
    const float rad = kPi / 180.0f;
    // The golfer stands beside the ball, on the left of the line of play.
    auto standSpot = [&](float& sx, float& sz) {
        sx = ballX + std::cos((aimHeading_ - 90.0f) * rad) * 16.0f;
        sz = ballZ + std::sin((aimHeading_ - 90.0f) * rad) * 16.0f;
    };
    switch (phase) {
        case Phase::Pause:
            if (phaseTime_ > 1.5f) { if (loop) init(*t_, rng_); else finished = true; }
            break;
        case Phase::Walk: {
            aimAtGreen();
            float sx, sz; standSpot(sx, sz);
            float dx = sx - golferX, dz = sz - golferZ, d = std::hypot(dx, dz);
            if (anim != GolferAnim::Walk) { anim = GolferAnim::Walk; animTime = 0; }
            if (d > 2.0f) {
                float s = std::min(d, kWalkSpeed * paceScale * dt);
                golferX += dx / d * s; golferZ += dz / d * s;
                golferHeading = std::atan2(dz, dx) / rad;
            } else {
                golferX = sx; golferZ = sz;
                bool putt = distToHole() < kPuttRange;
                if (stroke == 0 && washerAtTee && !washed) { washed = true; hold = (12.0f + 8.0f * rnd()) / flight::kTicksPerSecond; }   // stop of 12 to 19 ticks at the washer
                ++planCount; planPutt = putt; planFromX = golferX; planFromZ = golferZ; planAim = aimHeading_; planDist = distToHole();
                anim = putt ? GolferAnim::PuttAddress : GolferAnim::Address; animTime = 0;
                setPhase(putt ? Phase::PuttAddress : Phase::Address);
            }
            break;
        }
        case Phase::Address:
        case Phase::PuttAddress:
            golferHeading = aimHeading_;
            if (phaseTime_ >= kAddressSec) {
                bool putt = phase == Phase::PuttAddress;
                anim = putt ? GolferAnim::Putt : GolferAnim::Swing; animTime = 0; struck_ = false;
                setPhase(putt ? Phase::Putt : Phase::Swing);
            }
            break;
        case Phase::Swing:
        case Phase::Putt: {
            golferHeading = aimHeading_;
            bool putt = phase == Phase::Putt;
            if (!struck_ && phaseTime_ >= (putt ? kPuttImpactSec : kSwingImpactSec)) {
                struck_ = true;
                stroke++;
                shotFromX_ = ballX; shotFromZ_ = ballZ;
                if (putt) {
                    // PLACEHOLDER: chance to hole out grows with the putting skill, and a little with luck.
                    float chance = 0.40f + 0.55f * skills.v[GolferSkills::AccPutter] / 15.0f + 0.05f * skills.v[GolferSkills::Luck] / 15.0f;
                    missed_ = distToHole() > 60.0f ? rnd() > chance : rnd() > std::min(1.0f, chance + 0.3f);
                    landX_ = holeX(); landZ_ = holeZ();
                    if (missed_) {
                        float a = rnd() * 2 * kPi, off = 20.0f + 35.0f * rnd();
                        landX_ += std::cos(a) * off; landZ_ += std::sin(a) * off;
                    }
                    flightSec_ = std::max(0.6f, distToHole() / 160.0f); flightPeak_ = 0;
                    event = "putt"; club = "putt";
                    setPhase(Phase::Roll); anim = GolferAnim::Putt;
                    animTime = kPuttImpactSec;  // keep the putt animation running through the roll
                } else {
                    using S = GolferSkills;
                    const bool approach = distToHole() < kDriveDist;
                    // PLACEHOLDERS: skill levels scale carry (power, long driver) and the dispersion cone (accuracy,
                    // recovery when starting from sand).
                    // Range and flight follow the exe's rules (include/sg/flight.h). The skill levels here are 0..15 placeholders mapped onto the
                    // exe's 0..9 digits; the base byte, the lie hazard and the range unit in yards are not decoded, so 3 and 0 are used.
                    const int here0 = t_->typeAtWorld(ballX, ballZ);
                    const bool onTee = here0 == TT_Tee;
                    const int lenDigit = skills.v[S::LongDriver] * 9 / 15, accDigit = skills.v[approach ? S::AccIrons : S::AccDriver] * 9 / 15;
                    const int maxR = flight::maxRange(1, 3 + skills.v[S::Power] * 6 / 15, lenDigit, accDigit, false, 0, onTee);
                    const float unitsPerRange = kTileSizeWorld / flight::kRangeUnitsPerTile;
                    float carry = maxR * unitsPerRange * 0.8f;
                    if (!approach && skills.v[S::LongDriver] >= 8) carry += driveBonus;   // long hitters only (the exe's attribute bit 1, mapped from skill >= 8 as a PLACEHOLDER)
                    float acc = skills.v[approach ? S::AccIrons : S::AccDriver] / 15.0f;
                    float spread = 24.0f - 20.0f * acc;
                    if (acc >= 8.0f / 15.0f) spread /= spreadDivisor;   // accurate golfers only (the exe's attribute bit 2, mapped from skill >= 8 as a PLACEHOLDER)
                    int here = t_->typeAtWorld(ballX, ballZ);
                    if (here == 7 || here == TT_PotSandBunker || here == TT_GrassySand || here >= TT_SandBunker1)
                        spread *= 1.6f - 0.8f * skills.v[S::Recovery] / 15.0f;
                    // The exe previews the flight along the aim, bounce and roll included, to pick the club (FUN_004226a0). The port does the same: it finds the launch range
                    // whose resting point is the intended distance, then adds a PLACEHOLDER spread of 6 percent either way on the range.
                    const float s = kTileSizeWorld / 1024.0f;
                    const float ah = aimHeading_ * rad;
                    auto tileAtRel = [&](float x, float z, float dx, float dz) { return t_->typeAtWorld(ballX + (dx * x - dz * z) * s, ballZ + (dz * x + dx * z) * s); };
                    auto previewRest = [&](int range) {
                        const flight::Launch l = flight::launchFor(range);
                        const float dx = std::cos(ah), dz = std::sin(ah);
                        auto r = ballphys::run(1, 0, l.speed, l.vertical, theme, false, [&](float x, float z) { return tileAtRel(x, z, dx, dz); }, [] { return 0.5f; });
                        return r.restX * s;   // distance along the aim, world units
                    };
                    const int rMax = std::max(1, maxR);
                    float want = distToHole();
                    const float reach = previewRest(rMax) + (!approach && skills.v[S::LongDriver] >= 8 ? driveBonus : 0.0f);
                    int range = rMax;
                    if (want < reach) { int lo = 1, hi = rMax; while (lo < hi) { const int mid = (lo + hi) / 2; if (previewRest(mid) < want) lo = mid + 1; else hi = mid; } range = lo; }
                    // Shot shapes (PLACEHOLDER numbers; the exe's effect is not decoded): a fade or draw bends the ball a few degrees and is only
                    // reliable with the matching skill, backspin shortens the roll and is tighter, a punch keeps the ball low and short.
                    float shapeBias = 0, rangeK = 1.0f;
                    if (shape == 1 || shape == 2) { const float k = skills.v[shape == 1 ? S::Fade : S::Draw] / 15.0f; shapeBias = (shape == 1 ? 1.0f : -1.0f) * (3.0f + 5.0f * k); spread *= 1.5f - 0.7f * k; }
                    else if (shape == 3) { const float k = skills.v[S::Backspin] / 15.0f; spread *= 1.3f - 0.5f * k; rangeK = 0.97f; }
                    else if (shape == 4) { spread *= 0.85f; rangeK = 0.88f; }
                    range = std::max(1, (int)std::lround(range * rangeK * (0.94f + 0.12f * rnd())));
                    float dev = (rnd() - 0.5f) * spread + shapeBias; if (washed) dev -= dev / 3.0f;   // EXACT: the direction error loses one third
                    float h = aimHeading_ + dev;
                    landDev = h - aimHeading_;
                    {
                        const flight::Launch l = flight::launchFor(range);
                        const float dx = std::cos(h * rad), dz = std::sin(h * rad);
                        const float fx = ballX, fz = ballZ;
                        auto rr = ballphys::run(dx, dz, l.speed, l.vertical, theme, true, [&](float x, float z) { return t_->typeAtWorld(fx + x * s, fz + z * s); }, [&] { return rnd(); });
                        trajX_.clear(); trajZ_.clear(); trajH_.clear();
                        for (const auto& p : rr.pts) { trajX_.push_back(fx + p.x * s); trajZ_.push_back(fz + p.z * s); trajH_.push_back(p.h * s); }
                        trajHitTick_ = rr.hitTick; trajHitType_ = rr.hitType; trajLandTile_ = rr.landTile; trajRestTile_ = rr.restTile; trajWater_ = rr.water; trajOob_ = rr.oob;
                        landX_ = trajX_.back(); landZ_ = trajZ_.back();
                        flightSec_ = std::max(0.5f, (float)rr.pts.size() / flight::kTicksPerSecond); trajCounted_ = false;
                    }
                    club = approach ? "iron" : "drive";
                    event = "drive";
                    flightFromX_ = ballX; flightFromZ_ = ballZ; fallFrom_ = 0; hitObstacle_ = false; tickAcc_ = 0;
                    setPhase(Phase::Flight); anim = GolferAnim::Swing;
                    animTime = kSwingImpactSec;
                }
            }
            break;
        }
        case Phase::Flight: {
            const size_t n = trajX_.size();
            const float pos = std::min((float)(n - 1), phaseTime_ * flight::kTicksPerSecond);
            const size_t i0 = (size_t)pos, i1 = std::min(n - 1, i0 + 1); const float fr = pos - (float)i0;
            ballX = trajX_[i0] + (trajX_[i1] - trajX_[i0]) * fr; ballZ = trajZ_[i0] + (trajZ_[i1] - trajZ_[i0]) * fr; ballH = trajH_[i0] + (trajH_[i1] - trajH_[i0]) * fr;
            if (!trajCounted_ && trajHitTick_ >= 0 && pos >= (float)trajHitTick_) { trajCounted_ = true; ++obsCount; obsType = trajHitType_; }
            if (animTime > kSwingSec) animTime = kSwingSec - 0.001f;  // hold the follow through
            if (pos >= (float)(n - 1)) {
                ballH = 0;
                int ty = t_->typeAtWorld(ballX, ballZ);
                bool water = trajWater_ && isWater(ty >= 0 ? ty : (int)TT_WaterDeep);
                ++landCount; landType = ty; landFromType = t_->typeAtWorld(shotFromX_, shotFromZ_); landWater = water; landOut = ty < 0;
                landCloser = std::hypot(ballX - holeX(), ballZ - holeZ()) < std::hypot(shotFromX_ - holeX(), shotFromZ_ - holeZ());
                if (ty < 0) { event = "out of bounds, replay"; stroke++; ballX = shotFromX_; ballZ = shotFromZ_; }
                else if (water) { event = "splash, replay with a penalty"; stroke++; ballX = shotFromX_; ballZ = shotFromZ_; }
                else if (ty == 7 || ty == TT_PotSandBunker || ty == TT_GrassySand || ty >= TT_SandBunker1) event = "in the sand";
                else event = "on the course";
                if (ty != 2) washed = false;   // EXACT: the flag clears when the ball stops, unless it rests on a fairway tile
                setPhase(Phase::Settle);
            }
            break;
        }
        case Phase::Settle:
            if (animTime > kSwingSec) animTime = kSwingSec - 0.001f;
            if (phaseTime_ > 0.8f) setPhase(Phase::Walk);
            break;
        case Phase::Roll: {
            float u = std::min(1.0f, phaseTime_ / flightSec_);
            ballX = shotFromX_ + (landX_ - shotFromX_) * u; ballZ = shotFromZ_ + (landZ_ - shotFromZ_) * u;
            if (u >= 1.0f && missed_) { event = "putt missed"; setPhase(Phase::Settle); }
            else if (u >= 1.0f) { ballH = -1; event = "holed"; anim = GolferAnim::Happy; animTime = 0; setPhase(Phase::Celebrate); }
            break;
        }
        case Phase::Celebrate:
            if (phaseTime_ > 3.0f) setPhase(Phase::Pause);
            break;
    }
}

}  // namespace sg

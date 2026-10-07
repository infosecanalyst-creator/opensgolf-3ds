#include "sg/shot.h"
#include "sg/flight.h"
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
    stroke = 0;
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
                    float acc = skills.v[approach ? S::AccIrons : S::AccDriver] / 15.0f;
                    float spread = 24.0f - 20.0f * acc;
                    int here = t_->typeAtWorld(ballX, ballZ);
                    if (here == 7 || here == TT_PotSandBunker || here == TT_GrassySand || here >= TT_SandBunker1)
                        spread *= 1.6f - 0.8f * skills.v[S::Recovery] / 15.0f;
                    float dist = std::min(carry, distToHole()) * (0.88f + 0.24f * rnd());
                    float h = aimHeading_ + (rnd() - 0.5f) * spread;
                    landX_ = ballX + std::cos(h * rad) * dist; landZ_ = ballZ + std::sin(h * rad) * dist;
                    {
                        const flight::Arc arc = flight::simulate(std::max(1, (int)std::lround(dist / (unitsPerRange * 0.8f))));
                        flightSec_ = std::max(0.5f, arc.ticks / flight::kTicksPerSecond);
                        flightPeak_ = arc.peak / 1024.0f * kTileSizeWorld;
                    }
                    club = approach ? "iron" : "drive";
                    event = "drive";
                    setPhase(Phase::Flight); anim = GolferAnim::Swing;
                    animTime = kSwingImpactSec;
                }
            }
            break;
        }
        case Phase::Flight: {
            float u = std::min(1.0f, phaseTime_ / flightSec_);
            ballX = shotFromX_ + (landX_ - shotFromX_) * u; ballZ = shotFromZ_ + (landZ_ - shotFromZ_) * u;
            ballH = 4.0f * flightPeak_ * u * (1 - u);
            if (animTime > kSwingSec) animTime = kSwingSec - 0.001f;  // hold the follow through
            if (u >= 1.0f) {
                ballH = 0;
                int ty = t_->typeAtWorld(ballX, ballZ);
                // The manual: firm fairway makes balls bounce higher and roll farther, and rocks deflect the ball at random.
                // The amounts are PLACEHOLDERS (a 12 percent run on, a 40 to 90 unit kick in a random direction).
                if (ty == TT_FirmFairway) {
                    float dx = landX_ - shotFromX_, dz = landZ_ - shotFromZ_;
                    ballX += dx * 0.12f; ballZ += dz * 0.12f;
                    ty = t_->typeAtWorld(ballX, ballZ);
                } else if (ty == TT_Rock) {
                    float a = rnd() * 2 * kPi, off = 40.0f + 50.0f * rnd();
                    ballX += std::cos(a) * off; ballZ += std::sin(a) * off;
                    ty = t_->typeAtWorld(ballX, ballZ);
                }
                bool water = ty == TT_WaterShallow || ty == TT_WaterMiddle || ty == TT_WaterDeep || ty == TT_WaterShallowDesert;
                if (ty < 0) { event = "out of bounds, replay"; stroke++; ballX = shotFromX_; ballZ = shotFromZ_; }
                else if (water) { event = "splash, replay with a penalty"; stroke++; ballX = shotFromX_; ballZ = shotFromZ_; }
                else if (ty == 7 || ty == TT_PotSandBunker || ty == TT_GrassySand || ty >= TT_SandBunker1) event = "in the sand";
                else event = "on the course";
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

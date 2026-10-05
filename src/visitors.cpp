#include <algorithm>
#include "sg/visitors.h"

namespace sg {

static int ratingFor(const SpawnContext& c) { return c.difficulty < 2 ? c.funRating : c.skillRating; }

int commissionerThreshold(const VisitorState& s, int d) { return (s.commissionerCount + 2) * (d + 2) * 50; }
int heiressThreshold(const VisitorState& s) { int h = s.heiressVisits + 1; return h * h * 25; }

Visitor visitorFor(const VisitorState& s, const SpawnContext& c) {
    if (c.sandbox || c.pairBlocked) return Visitor::None;
    int r = ratingFor(c);
    if (c.pairIndex == 1 && !(c.onCourse & kHeiressOnCourse) && s.ceoCount < 8 && c.holes >= 2 * (s.ceoCount + 1))
        return Visitor::Ceo;
    if (c.pairIndex == 3 && !(c.onCourse & kCommissionerOnCourse) && c.propertyKind != 2 && r > commissionerThreshold(s, c.difficulty))
        return Visitor::Commissioner;
    if (c.pairIndex == 5 && !(c.onCourse & kHeiressOnCourse) && heiressThreshold(s) < r && s.availMask < 0xffffu)
        return Visitor::Heiress;
    return Visitor::None;
}

Outcome resolve(Visitor v, VisitorState& s, bool finished, int mood, int holes, SocialRng& rng) {
    Outcome o;
    switch (v) {
    case Visitor::Ceo: {
        ++s.ceoCount;
        if (finished && mood > 2 && 2 * s.ceoCount < holes + 1) {
            o.accepted = true;
            o.cashUnits = (mood > 4 ? 2 : 1) * 50;
        } else --s.ceoCount;
        break;
    }
    case Visitor::Commissioner:
        if (finished && mood > 2) { o.accepted = true; o.buyLand = true; }
        break;
    case Visitor::Heiress:
        if (finished && mood >= 3) {
            // Start at the mood; draw a kind below it (clamped to 0..15); stop at the first design the strip does not have; otherwise widen the draw by one while below 25.
            // If every try hits a design already available, the last draw is donated anyway (EXACT).
            int i = mood, t = 0;
            do { t = std::clamp(rng.below(std::max(1, i)), 0, 15); if (!(s.availMask >> t & 1u)) break; ++i; } while (i < 25);
            {
                s.landmarkMask |= 1u << t; s.availMask |= 1u << t;
                ++s.heiressVisits;
                o.accepted = true;
                o.landmarkType = t;
                o.effect = t & 3;
                o.landmarkDollars = (long(t) * 5 + 5) * 200;
            }
        }
        break;
    default: break;
    }
    return o;
}

}

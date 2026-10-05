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
    if (c.pairIndex == 5 && !(c.onCourse & kHeiressOnCourse) && heiressThreshold(s) < r && s.landmarkMask < 0xffffu)
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
            int range = mood < 1 ? 1 : mood;
            int t = -1;
            for (; range <= 25 && t < 0; ++range) {
                int p = rng.below(range);
                if (p < 16 && !(s.landmarkMask & (1u << p))) t = p;
            }
            if (t < 0) for (int p = 0; p < 16; ++p) if (!(s.landmarkMask & (1u << p))) { t = p; break; }
            if (t >= 0) {
                s.landmarkMask |= 1u << t;
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

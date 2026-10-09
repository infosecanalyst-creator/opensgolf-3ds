#include "sg/membership.h"
#include <algorithm>

namespace sg {

void Roster::newGame(SocialRng& rng) {
    for (auto& x : e) x = Entry();
    int n = 0;
    for (int tries = 0; n < 12 && tries < 10000; ++tries) {
        int id = 1 + rng.below(kPool);
        if (e[id].tier == Tier::None) { e[id].tier = Tier::Visitor; ++n; }
    }
}

int Roster::promotionPoints(const RoundSummary& r) const {
    int pts = r.likedSum & 3;
    if (r.mood >= 2 * r.difficulty + 6) pts += 1;
    if (r.likedNearEnd || r.difficulty == 0 || r.yearIndex == 0 || (r.cashUnits < 200 && r.memberCount < 1)) pts += 1;
    return pts + r.improvementLevel;
}

bool Roster::endRound(int id, const RoundSummary& r, SocialRng& rng, int* invited) {
    if (invited) *invited = -1;
    if (id < 1 || id > kPool || e[id].resigned || !r.ordinary) return false;
    Entry& g = e[id];
    if (g.tier == Tier::None || g.tier == Tier::Platinum) return false;
    if (pointsToPromote(g.tier) > promotionPoints(r)) return false;
    g.tier = Tier(int(g.tier) + 1);
    int f = invite(rng);
    if (invited) *invited = f;
    return true;
}

void Roster::like(int id, int hole) {
    if (id < 1 || id > kPool || hole < 1 || hole > 18) return;
    uint8_t& c = e[id].liked[hole];
    if (c < 3) ++c;
}

bool Roster::leave(int id, int hole) {
    if (id < 1 || id > kPool) return false;
    Entry& g = e[id];
    if (g.tier < Tier::Member) return false;
    g.tier = Tier::None;
    g.resigned = true;
    g.blamedHole = uint8_t(hole);
    if (hole >= 1 && hole <= 18) g.holeFlags[hole] |= 4;
    return true;
}

int Roster::pickArrival(SocialRng& rng, const std::vector<int>& onCourse) const {
    for (int t = 0; t < 999; ++t) {
        int id = 1 + rng.below(kPool);
        const Entry& g = e[id];
        if (g.resigned || g.tier == Tier::None) continue;
        bool bad = false;
        for (int o : onCourse) if (o == id || o % 19 == id % 19) { bad = true; break; }
        if (!bad) return id;
    }
    return -1;
}

int Roster::invite(SocialRng& rng) {
    // PLACEHOLDER detail: resigned golfers are skipped here; the original's check is on the tier byte alone.
    for (int t = 0; t < 1000; ++t) {
        int id = 1 + rng.below(kPool);
        if (e[id].tier == Tier::None && !e[id].resigned) { e[id].tier = Tier::Visitor; return id; }
    }
    return -1;
}

int Roster::memberCount() const {
    int n = 0;
    for (int i = 1; i <= kPool; ++i) if (!e[i].resigned && e[i].tier >= Tier::Member) ++n;
    return n;
}
int Roster::count(Tier t) const {
    int n = 0;
    for (int i = 1; i <= kPool; ++i) if (!e[i].resigned && e[i].tier == t) ++n;
    return n;
}
int Roster::resignedCount() const {
    int n = 0;
    for (int i = 1; i <= kPool; ++i) if (e[i].resigned) ++n;
    return n;
}

const char* leaveReason(int ev) {
    switch (ev) {
        case 4: return "the course is too tough";
        case 8: case 0x17: case 0x1e: return "the course needs improvement";
        case 9: return "hit by a golfer";
        case 0xc: return "club wrapped round a tree";
        case 0xd: return "clubs thrown in a lake";
        case 0xe: return "thirst";
        case 0xf: return "hunger";
        case 0x15: return "insulted by a golfer";
        case 0x1a: return "tiredness";
        default: return "general disgust";
    }
}

}

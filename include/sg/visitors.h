// Special visitors: corporate CEO, county commissioner, wealthy heiress. Spawn and outcome rules read from the publisher's golf.exe
// (docs/DECODE_SOCIAL.md). EXACT: pair slot rule, thresholds, counts, payouts. The international celebrity is dead code in that build
// and is not implemented. PLACEHOLDER: the landmark pick distribution (shape known, exact draw loop approximate) and which rating the
// heiress test uses (the fun/skill choice is shared with the commissioner, probable).
#pragma once
#include "sg/rng.h"

namespace sg {

enum class Visitor { None, Ceo, Commissioner, Heiress };
enum VisitorBits { kCommissionerOnCourse = 2, kHeiressOnCourse = 4, kCeoOnCourse = 8 };

struct VisitorState {
    int ceoCount = 0;            // CEOs hosted so far (max 8)
    int commissionerCount = 0;   // land purchases accepted so far
    int heiressVisits = 0;       // donations so far
    unsigned landmarkMask = 0;   // 16 landmark types already donated
};

struct SpawnContext {
    int pairIndex = 0;           // (second slot / 2) % 6 of the pair being launched
    int holes = 0;
    int difficulty = 0;
    int funRating = 0;           // same scale as the original's fun rating
    int skillRating = 0;         // hundredths
    int propertyKind = 0;        // the commissioner never comes on kind 2
    unsigned onCourse = 0;       // VisitorBits already playing
    bool sandbox = false;        // visitors are off in sandbox
    bool pairBlocked = false;    // pairs flagged as tournament or match play
};

// Which visitor (if any) the pair about to launch becomes.
Visitor visitorFor(const VisitorState& s, const SpawnContext& c);
// The rating threshold the next commissioner / heiress visit needs.
int commissionerThreshold(const VisitorState& s, int difficulty);
int heiressThreshold(const VisitorState& s);

struct Outcome {
    bool accepted = false;
    int cashUnits = 0;           // units of $100 paid to the club (CEO)
    int landmarkType = -1;       // heiress: 0..15
    int effect = -1;             // heiress: 0 happy thoughts, 1 no dandelions, 2 faster player skills, 3 happy golfer stories
    long landmarkDollars = 0;    // heiress: landmark worth
    bool buyLand = false;        // commissioner: open the land purchase offer
};
// Called when the visitor finishes. holesPlayedAll: the visitor reached the end. mood: final mood.
Outcome resolve(Visitor v, VisitorState& s, bool finished, int mood, int holes, SocialRng& rng);

}

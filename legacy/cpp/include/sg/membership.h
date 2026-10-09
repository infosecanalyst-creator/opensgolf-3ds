// Club membership: a persistent pool of 75 golfer identities, each with a tier, that the club grows or loses through play.
// Rules read from the publisher's golf.exe (see docs/DECODE_SOCIAL.md). EXACT: tiers, promotion points and threshold, friend invite,
// resignation, fee bonus, member count, spawn eligibility. PLACEHOLDER: who the first twelve visitors are (archetypes not decoded),
// the roster "waiting list" definition, and anything the caller must supply (mood, per-hole liked counts).
#pragma once
#include <cstdint>
#include <string>
#include <vector>
#include "sg/rng.h"

namespace sg {

enum class Tier : uint8_t { None = 0, Visitor = 1, Member = 2, Silver = 3, Gold = 4, Platinum = 5 };

// Extra green fee per hole finished, in units of $100: Gold 2, Platinum 5. Silver buys home sites instead of paying more.
inline int memberFeeBonus(Tier t) { return t == Tier::Gold ? 2 : t == Tier::Platinum ? 5 : 0; }
// Points a golfer needs to move up from this tier: 2, 4, 8, 16 (1 << tier). Platinum is the top.
inline int pointsToPromote(Tier t) { return 1 << int(t); }

struct RoundSummary {          // everything the caller knows about a golfer who completed a round
    int likedSum = 0;          // sum of the per-hole 2-bit "liked" counters over holes played
    int mood = 0;              // final mood (the same 0.. scale as the fee)
    int difficulty = 0;        // 0..3
    int yearIndex = 0;         // 0 for the first year
    int cashUnits = 0;         // club cash in units of $100
    int memberCount = 0;       // current member count
    bool likedNearEnd = false; // a late hole has a nonzero liked counter
    int improvementLevel = 0;  // level of the "golfers become members" improvement (0 when absent)
    bool ordinary = true;      // false for visitors (CEO, commissioner, heiress) and tournament pros
};

struct Roster {
    static constexpr int kPool = 75;               // ids 1..75
    struct Entry { Tier tier = Tier::None; bool resigned = false; uint8_t liked[19] = {}; uint8_t blamedHole = 0;
                  uint16_t rounds = 0; uint8_t low = 0; int8_t hcp = 0; uint8_t holeFlags[19] = {}; };   // roster screen data: rounds finished, best round, last round over par (PLACEHOLDER rule for the handicap), per hole flag bytes (bit0 photo opp, bit1 happy ending, bit2 resigned here)
    Entry e[kPool + 1];

    void newGame(SocialRng& rng);                  // twelve golfers start as Visitors (PLACEHOLDER choice of which)
    int promotionPoints(const RoundSummary& r) const;
    // Applies a completed round to golfer id: returns true when the golfer moved up. A promotion also invites a friend.
    bool endRound(int id, const RoundSummary& r, SocialRng& rng, int* invitedId = nullptr);
    // Records that the golfer liked a hole (2-bit counter, saturating at 3).
    void like(int id, int hole);
    // A golfer quits mid-round (mood below zero). Members (tier 2+) resign for good; returns true when it was a resignation.
    bool leave(int id, int hole);
    // Picks a golfer id to arrive: not resigned, not on course, id % 19 unlike any golfer on course, tier above None. -1 means the
    // club's membership is declining (the original shows a warning after 999 failed picks).
    int pickArrival(SocialRng& rng, const std::vector<int>& onCourse) const;
    int invite(SocialRng& rng);                    // makes a random tier None golfer a Visitor; returns id or -1
    int memberCount() const;                       // tier 2 or more, not resigned
    int count(Tier t) const;
    int resignedCount() const;
};

const char* leaveReason(int eventId);              // short reason for an event id (own wording)

}

// Golfer reactions: the mood engine and the needs clock (docs/DECODE_EVENTS_SHOTS.md section 1, docs/DECODE_EVENTS_NEEDS.md sections 2, 3 and 6).
// Pure logic, integer maths as in the exe. The caller owns the hole statistics and the mood value; this file decides what a reaction does.
#pragma once
#include <cstdint>
#include <functional>

namespace sg {

constexpr double kSimTickHz = 13.0;   // PLACEHOLDER: ticks a second (the exe's tick length is not decoded; a year is 8192 ticks)

// History entry flags kept in the high bits of a stored location.
constexpr int kLocGood = 0x4000, kLocBaseBad = 0x8000;   // applied good; base was bad (0xc000 together: applied bad)

struct Reactor {
    int hist[10] = {};          // last reaction types, index 0 newest (0 = none)
    int histLoc[10] = {};       // locations with the polarity marks in the top bits
    int speech = 0;             // speech bubble timer: set to 7 by every reaction, minus 1 every 8 ticks
    int lastBubbleType = 0;
    int momentum = 0;           // only used by non ordinary golfers
    int hunger = 0, thirst = 0, fatigue = 0;   // needs counters, all start at 0
    int relation = 0;           // pair relationship state 0..3 (type 26 location is relation == 3)
    int stopTimer = 0;
    bool leaving = false, hurried = false, warned = false, slowWarned = false;
    int polarity() const { return (histLoc[0] & kLocBaseBad) ? 2 : (histLoc[0] & kLocGood) ? 0 : 1; }   // 0 good, 1 neutral, 2 bad after the newest reaction
};

struct ReactIn {
    int type = 0, loc = 0x14;
    int difficulty = 0;
    int kind = 0;               // golfer kind: 0 ordinary, 0x40 pro, 0x20 scripted pair, 0x60 CEO (kind & 0xe0 is what the rules read)
    int strokes = 0;            // strokes taken on the current hole
    int slot = 0;               // golfer slot (the partner cascade tests (strokes + slot) & 1)
    int mode543cf4 = 0;         // exe global with modes 1 (mute single complaints) and 2 (no gate); 0 in normal play
    bool disabled = false;      // game flag 0x2000000: reactions disabled
};

struct ReactOut {
    bool ignored = false;       // returned early: nothing is recorded
    int delta = 0;              // applied mood change (added to the hole fun total even when 0, unless ignored)
    bool counted = false;       // delta != 0: the hole's per type count and location are updated
    bool partnerReacts = false; // the partner should raise type 47 on itself (location 0x14, its speech timer +2)
    bool litterRoll = false;    // negative delta: the caller may roll the ground marker (probability (d + 1) / 6, always for leavers)
};

// Raise a reaction on a golfer. `mood` is the golfer's mood (clamped to -10..10 here). `partnerFree` is true when the partner is on a hole,
// silent and in a good mood (mood above 0); only then can the partner cascade fire.
ReactOut react(Reactor& r, int& mood, const ReactIn& in, bool partnerFree);

// Base mood change of a type before the gate and the scaling; hunger, thirst and fatigue are the counters at the moment of the call
// (read BEFORE the service event resets them).
int reactionBase(int type, int difficulty, const Reactor& r);

// ---- needs clock (per golfer, one call per game tick) ----
struct NeedsIn {
    unsigned tick = 0; int slot = 0;
    bool walking = false;        // an active walking segment
    int hole = 1;                // 1 based hole, 19 means finished or leaving (no needs then)
    int theme = 0;               // exe theme order: 0 parkland, 1 desert, 2 tropical, 3 links
    bool nearHot = false;        // a neighbouring tile with the heat flag (tile ids not known: false in the port)
    int kind = 0;
    int holeCount = 18;
    bool clubRemarkMade = false; // PLACEHOLDER for the exe's 0x579558 mask and the scorecard test of the type 63 chat
    bool moving = false;         // fatigue only grows while moving
    int effort = 2;              // tile walking effort byte (PLACEHOLDER by terrain), 1 on paths
};
// Fills `out` (room for 3) with the reaction types the tick raises (63, then 14 or 15, then 26) and returns how many. `rnd(n)` returns 0..n-1.
int needsTick(Reactor& r, const NeedsIn& in, const std::function<int(int)>& rnd, int out[3]);

// Mask of the walking scenic glance: periodic, shorter after good events, longer after filtered complaints.
int glanceMask(const Reactor& r, bool walking);
inline bool glanceDue(const Reactor& r, unsigned tick, int slot, bool walking) { return ((tick + 0x21u * (unsigned)slot) & (unsigned)glanceMask(r, walking)) == 0; }

// A golfer quits when mood is below 0 and the speech timer has run out.
inline bool wantsToQuit(const Reactor& r, int mood, int kind) { return mood < 0 && r.speech == 0 && !r.leaving && (kind & 0xe0) != 0x20; }
// Speech timer countdown: one step every 8 ticks.
inline void speechTick(Reactor& r, unsigned tick) { if (r.speech > 0 && (tick & 7u) == 0) --r.speech; }

// Quit reason by the type of the last reaction (which complaint ended the round): 0 generic.
int quitReasonCategory(int lastType);

}  // namespace sg

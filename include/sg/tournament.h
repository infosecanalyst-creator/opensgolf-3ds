// Tournament play for the native port: the pro golfer roster (progolfers.dta), the celebrity list (celebrities.dta), the SGA tournament
// offer and acceptance, the field draw, standings and the prize payout. Written from facts recorded in docs/DECODE_TOURNAMENTS.md; the
// per hole strokes are supplied by the caller (the real game simulates every golfer, the port's golfer simulation is not wired in yet).
//
// Exact (read from the publisher exe): file parsing rules, field size and slot layout, the field draw rule, the standings order and tie
// handling, the prize ladder and who is paid, the side effects (cash, fame, history code, accomplishment ids), the July offer rule.
// Placeholder: the demo stroke generator (placeholderStrokes) and anything marked PLACEHOLDER below.
#pragma once
#include <cstdint>
#include <functional>
#include <string>
#include <vector>
#include "sg/sga.h"

namespace sg {

// ---------------------------------------------------------------------------------------------------------------------------------
// Data files
// ---------------------------------------------------------------------------------------------------------------------------------

// One line of progolfers.dta: "name,body,skin,hat,shirt,pants,SSSSSSSSSS" (plus an ignored trailing number on most lines).
struct TourPro {
    std::string name;
    int body = 0;        // 0..7: long sleeves, knickers, short sleeves, short pants, then four female outfits (bit 2 set = female)
    int skin = 0;        // 0..3
    int hat = 0, shirt = 0, pants = 0;   // colour indexes 0..9
    int skills[10] = {}; // 0..15 each: Power Hitter, Long Driver, Accurate Driver, Accurate Irons, Accurate Putter, Draw, Fade, High Backspin,
                         // Recovery, Luck (the player has two more: Positive Attitude and Mental Toughness)
    int skillSum = 0;    // sum of the ten, kept by the exe and used by the field draw
    int noteRating = -1; // the loose number after the skill digits in the file; the exe ignores it (it roughly follows skillSum)
    bool female() const { return (body & 4) != 0; }
};

// One line of celebrities.dta: "name,type,skin,hair,shirt,pants". Celebrities are course visitors and home buyers, never tournament entrants.
struct Celebrity {
    std::string name;
    char type = 'A';     // A action star, B female pop star, C politician, D male comedian, E supermodel, F fitness female, G female comedian,
                         // H leading man, I female movie star, J rock and roller, K athlete
    int skin = 0, hair = 0, shirt = 0, pants = 0;
};

constexpr int kMaxPros = 100;          // the exe table holds 100 records
constexpr int kMaxCelebrities = 100;

// Parse rules copied from the exe: lines starting with '*' and lines of nine characters or fewer are skipped, fields are split on commas,
// the first character of each small field is the value, skills are one character each ('0'-'9', then 'A'.. = 10..). The table stops at 100.
// A line with a stray blank field (one line in the shipped file has one) is read here by dropping blank fields: the exe reads it by
// position and gets garbage. `warnings` (optional) collects one note per oddity.
bool parseTourPros(const std::string& text, std::vector<TourPro>& out, std::vector<std::string>* warnings = nullptr);
bool loadTourPros(const std::string& path, std::vector<TourPro>& out, std::vector<std::string>* warnings = nullptr);
bool parseCelebrities(const std::string& text, std::vector<Celebrity>& out, std::vector<std::string>* warnings = nullptr);
bool loadCelebrities(const std::string& path, std::vector<Celebrity>& out, std::vector<std::string>* warnings = nullptr);

// ---------------------------------------------------------------------------------------------------------------------------------
// Random numbers: rand(n) must return 0..n-1 (the exe's own generator is not reproduced).
// ---------------------------------------------------------------------------------------------------------------------------------
using RandFn = std::function<int(int)>;
struct SimpleRng {
    uint32_t state = 12345;
    int operator()(int n) { state = state * 1664525u + 1013904223u; return n <= 0 ? 0 : (int)((state >> 8) % (uint32_t)n); }
};

// ---------------------------------------------------------------------------------------------------------------------------------
// Offer rule
// ---------------------------------------------------------------------------------------------------------------------------------
struct OfferContext {
    bool matchPending = false;    // a match or practice round is waiting (exe: pending slot is not -1)
    bool sandbox = false;         // sandbox flag 0x1000000
    bool inProgress = false;      // tournament flag 0x200000
};
// True at the start of July (tick % 8192 == 4096) when no match is pending and neither sandbox nor a running tournament blocks it.
bool julyOfferDue(unsigned ticks, const OfferContext& ctx);

// ---------------------------------------------------------------------------------------------------------------------------------
// Field
// ---------------------------------------------------------------------------------------------------------------------------------
struct FieldParams {
    int holes = 18;               // open holes H: the field is 2 * H golfers, two per hole, a shotgun start (pair k starts at hole k+1)
    int difficulty = 1;           // 0..3
    int prizeThousands = 0;       // the offer's first prize (0 is replaced by the default, see defaultPrizeThousands)
    int cashUnits = 0;            // club cash in units of $100
    bool championshipMode = false;// "Save Course for Championship" play (flag 0x4000000): the strength score is randomised
};

struct Entrant {
    int slot = 0;                 // 0..2H-1. Slot 1 is the player; slot 0 is the player's pair partner.
    std::string name;
    bool isPlayer = false;
    int proIndex = -1;            // index into the roster (-1 for the player)
    int skillSum = 0;
    int startHole = 1;            // slot / 2 + 1
    int strokes[18] = {};         // strokes per hole, 0 = not played yet
    int currentHole = 0;          // 0 = off the course (finished)
    bool finished = false;
};

// Draw 2H - 1 pros for the slots other than the player's, using the exe's rule: a random record (not 0, not empty, not yet used unless
// 800 tries were spent) is accepted when its strength score x = (skillSum - 10)^2 / ((difficulty * 5 + 20) * 2) lies in a window around
// prize/10 + cash/200 that widens as tries accumulate (upper edge: tries/(4 - difficulty) + prize/10 + cash/200; lower edge:
// prize/10 - tries + cash/200). Slot 0 (the partner) gets x reduced by difficulty * x / 6 and a lower edge that widens four times slower.
// `playerName` and `playerSkillSum` fill slot 1. Returns the field ordered by slot.
std::vector<Entrant> selectField(const std::vector<TourPro>& pros, const FieldParams& p, const std::string& playerName, int playerSkillSum, const RandFn& rng);

// ---------------------------------------------------------------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------------------------------------------------------------
struct Standing {
    int place = 0;                // 1-based and always distinct: tied scores are ordered by slot number (lower slot first), no shared places
    int slot = 0;
    int relToPar = 0;             // sum of (strokes - par) over completed holes
    int prizeThousands = 0;       // 0 when unpaid
    bool paid = false;
};

// The prize ladder (thousands): place 1 gets `first`; each next place gets two thirds of the previous (integer division); places 1..H are
// paid, the rest get 0. A first prize of 0 is replaced by H * 20 ((H + 1) * 20 - 20, where the exe's counter is H + 1).
int defaultPrizeThousands(int holes);
std::vector<int> prizeLadder(int firstThousands, int holes);

// Order entrants by (relToPar, slot). `pars` lists the par of each open hole (hole 1 first). A hole counts only when it has strokes, par is
// open, and (while the golfer is still on the course) it is not the hole being played.
std::vector<Standing> rankEntrants(const std::vector<Entrant>& field, const std::vector<int>& pars, int holes);

struct TournamentResult {
    std::vector<Standing> standings;
    int playerPlace = 0;
    int playerPrizeThousands = 0;
    long cashDeltaUnits = 0;      // prize * 1000 / 100 = prize * 10 units of $100, player only
    long yearlyIncomeDeltaUnits = 0;  // same amount, added to the current year's record (field +0x0e, probably tournament winnings)
    int fameDelta = 0;            // fame counter: 4 - place for places 1..3, else 1; paid places only
    int historyCode = 0;          // 0xE0 | place, written to the 500 entry history ring for the current 1024 tick block; 0 when unpaid
    std::vector<int> accomplishments;  // ids to award (once each): see kAccomplishment*
};

// Accomplishment ids used by the tournament code (index into the trophy list: the 4th entry is "1st Tournament", and so on).
constexpr int kAccomplishmentFirstTournament = 3;          // when the tournament opens
constexpr int kAccomplishment500kTournament = 8;           // opens with a first prize of at least 500 (thousand)
constexpr int kAccomplishment1mTournament = 13;            // opens with a first prize of at least 1000
constexpr int kAccomplishmentWin9Holes = 11;               // win with 9 or more holes
constexpr int kAccomplishmentWin18Holes = 15;              // win with 18 holes
constexpr int kAccomplishmentGrandSlamFirst = 17;          // + theme (0..3), win with 18 holes and a first prize above 100000 (see notes: unreachable)

// ---------------------------------------------------------------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------------------------------------------------------------
class Tournament {
public:
    enum class State { Idle, Offered, InProgress };

    State state() const { return state_; }
    int holes() const { return holes_; }
    int firstPrizeThousands() const { return prize_; }
    const char* name() const { return name_; }
    const std::vector<Entrant>& field() const { return field_; }
    static constexpr int kRounds = 1;       // one pass over every open hole; there is no second round

    // The July evaluation (silent): runs the SGA evaluation, and when it yields a prize the offer becomes pending, otherwise any pending
    // offer is withdrawn. Returns true if an offer is pending afterwards.
    bool evaluateOffer(const SgaInput& in);
    // Pressing the tournament button with an offer pending: the course is evaluated again and the prize recomputed (it can change or drop
    // to zero, in which case the SGA evaluation screen is shown instead). Returns true if the offer dialog should be shown.
    bool reopenOffer(const SgaInput& in);
    // "I think I need more practice": the prize is cleared but the offer stays pending (the button re-evaluates next time).
    void decline();
    // "Great, let the games begin": draws the field and starts. Fails unless an offer with a prize is pending. `pars` has one entry per open hole.
    bool accept(const std::vector<TourPro>& pros, const std::vector<int>& pars, int difficulty, int cashUnits, const std::string& playerName,
                int playerSkillSum, const RandFn& rng, std::vector<int>* accomplishments = nullptr);
    // "Cancel match/tournament" confirmed: the running tournament and the pending offer are both dropped, no payout.
    void cancel();

    // Play. Strokes are supplied by the caller, one hole at a time or a whole round at once.
    bool recordHole(int slot, int hole, int strokes);   // marks the golfer as being on hole + 1 (0 after the last hole of his loop)
    bool submitRound(int slot, const std::vector<int>& strokesByHole);   // all holes of one golfer, marks him finished
    bool allFinished() const;
    std::vector<Standing> standings() const;              // live table (current hole excluded) or final order
    // Final: needs everyone finished. Computes payout and side effects, then the tournament is over (state Idle, offer consumed).
    bool finish(int theme, TournamentResult& out);

private:
    State state_ = State::Idle;
    int holes_ = 0, prize_ = 0;
    const char* name_ = nullptr;
    std::vector<int> pars_;
    std::vector<Entrant> field_;
    SgaResult eval_;
    void recompute(const SgaInput& in);
};

// PLACEHOLDER: stroke generator so a viewer can run a tournament without the golfer simulation. Better skills lower the score on average;
// the numbers are invented, not read from the exe.
int placeholderStrokes(int par, int skillSum, const RandFn& rng);

}  // namespace sg

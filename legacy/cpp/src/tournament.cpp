#include "sg/tournament.h"
#include <algorithm>
#include <cstdlib>
#include <fstream>
#include <sstream>

namespace sg {

// ---------------------------------------------------------------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------------------------------------------------------------
namespace {

std::vector<std::string> splitLines(const std::string& text) {
    std::vector<std::string> lines;
    std::string cur;
    for (char c : text) {
        if (c == '\n') { lines.push_back(cur); cur.clear(); }
        else cur += c;
    }
    if (!cur.empty()) lines.push_back(cur);
    for (auto& l : lines)
        while (!l.empty() && (l.back() == '\r' || l.back() == '\n')) l.pop_back();
    return lines;
}

std::vector<std::string> splitCommas(const std::string& s) {
    std::vector<std::string> out;
    std::string cur;
    for (char c : s) {
        if (c == ',') { out.push_back(cur); cur.clear(); }
        else cur += c;
    }
    out.push_back(cur);
    return out;
}

bool blank(const std::string& s) {
    return std::all_of(s.begin(), s.end(), [](char c) { return c == ' ' || c == '\t'; });
}

// The exe takes the first character of a small field and reduces (c - '0') with C remainder. Out of range values are clamped to 0 here.
int smallField(const std::string& tok, int mod) {
    if (tok.empty()) return 0;
    int v = (unsigned char)tok[0] - '0';
    v %= mod;
    return v < 0 ? 0 : v;
}

int skillChar(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'Z') return (c - 'A') % 10 + 10;
    if (c >= 'a' && c <= 'z') return (c - 'a') % 10 + 10;
    return 0;   // PLACEHOLDER for characters the exe would turn into garbage
}

bool readFile(const std::string& path, std::string& out) {
    std::ifstream f(path, std::ios::binary);
    if (!f) return false;
    std::ostringstream ss;
    ss << f.rdbuf();
    out = ss.str();
    return true;
}

// Fields after the name, with blank fields dropped when there are too many (see header).
std::vector<std::string> fieldsAfterName(const std::vector<std::string>& toks, size_t want, const std::string& name,
                                         std::vector<std::string>* warnings) {
    std::vector<std::string> rest(toks.begin() + 1, toks.end());
    if (rest.size() > want) {
        std::vector<std::string> kept;
        for (auto& t : rest) if (!blank(t)) kept.push_back(t);
        if (kept.size() != rest.size() && warnings) warnings->push_back(name + ": blank field dropped (the exe reads this line by position)");
        rest = kept;
    }
    return rest;
}

}  // namespace

bool parseTourPros(const std::string& text, std::vector<TourPro>& out, std::vector<std::string>* warnings) {
    out.clear();
    for (const std::string& line : splitLines(text)) {
        if (line.empty() || line[0] == '*' || line.size() <= 9) continue;
        if ((int)out.size() >= kMaxPros) break;
        auto toks = splitCommas(line);
        if (toks.size() < 7) {
            if (warnings) warnings->push_back("short line skipped: " + line);
            continue;
        }
        TourPro g;
        g.name = toks[0];
        auto rest = fieldsAfterName(toks, 6, g.name, warnings);
        if (rest.size() < 6) {
            if (warnings) warnings->push_back("short line skipped: " + line);
            continue;
        }
        g.body = smallField(rest[0], 8);
        g.skin = smallField(rest[1], 4);
        g.hat = smallField(rest[2], 10);
        g.shirt = smallField(rest[3], 10);
        g.pants = smallField(rest[4], 10);
        const std::string& sk = rest[5];
        for (int i = 0; i < 10; ++i) {
            g.skills[i] = i < (int)sk.size() ? skillChar(sk[i]) : 0;
            g.skillSum += g.skills[i];
        }
        if (sk.size() > 10) {
            std::string tail = sk.substr(10);
            size_t p = tail.find_first_of("0123456789");
            if (p != std::string::npos) g.noteRating = std::atoi(tail.c_str() + p);
        }
        out.push_back(g);
    }
    return !out.empty();
}

bool loadTourPros(const std::string& path, std::vector<TourPro>& out, std::vector<std::string>* warnings) {
    std::string t;
    if (!readFile(path, t)) return false;
    return parseTourPros(t, out, warnings);
}

bool parseCelebrities(const std::string& text, std::vector<Celebrity>& out, std::vector<std::string>* warnings) {
    out.clear();
    for (const std::string& line : splitLines(text)) {
        if (line.empty() || line[0] == '*' || line.size() <= 9) continue;
        if ((int)out.size() >= kMaxCelebrities) break;
        auto toks = splitCommas(line);
        if (toks.size() < 6) {
            if (warnings) warnings->push_back("short line skipped: " + line);
            continue;
        }
        Celebrity c;
        c.name = toks[0];
        c.type = toks[1].empty() ? 'A' : toks[1][0];
        c.skin = smallField(toks[2], 4);
        c.hair = smallField(toks[3], 10);
        c.shirt = smallField(toks[4], 10);
        c.pants = smallField(toks[5], 10);
        out.push_back(c);
    }
    return !out.empty();
}

bool loadCelebrities(const std::string& path, std::vector<Celebrity>& out, std::vector<std::string>* warnings) {
    std::string t;
    if (!readFile(path, t)) return false;
    return parseCelebrities(t, out, warnings);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Offer rule and field draw
// ---------------------------------------------------------------------------------------------------------------------------------
bool julyOfferDue(unsigned ticks, const OfferContext& ctx) {
    return (ticks % 8192u) == 4096u && !ctx.matchPending && !ctx.sandbox && !ctx.inProgress;
}

std::vector<Entrant> selectField(const std::vector<TourPro>& pros, const FieldParams& p, const std::string& playerName, int playerSkillSum,
                                 const RandFn& rng) {
    const int H = std::clamp(p.holes, 1, 18);
    const int slots = 2 * H;
    const int diff = std::clamp(p.difficulty, 0, 3);
    std::vector<Entrant> field(slots);
    bool used[kMaxPros] = {};
    // The draw uses the raw prize counter (the default replacement only happens later, on the results screen).
    const int centre = p.prizeThousands / 10 + p.cashUnits / 200;
    for (int s = 0; s < slots; ++s) {
        Entrant& e = field[s];
        e.slot = s;
        e.startHole = s / 2 + 1;
        e.currentHole = e.startHole;
        if (s == 1) {
            e.isPlayer = true;
            e.name = playerName;
            e.skillSum = playerSkillSum;
            continue;
        }
        int attempts = 0, idx = 0, x = 0;
        if (pros.size() < 2) { e.name = "(no pro)"; continue; }   // nothing but record 0 (never drawn) would loop forever
        for (int guard = 0; guard < 1000000; ++guard) {
            // inner draw: valid record, not index 0, not used unless 800 valid tries have been spent
            for (;;) {
                idx = rng(100);
                const bool valid = idx < (int)pros.size();
                if (!valid || idx == 0) continue;
                const int sum = pros[idx].skillSum;
                x = ((sum - 10) * (sum - 10)) / ((diff * 5 + 20) * 2);
                if (s == 0) {
                    long long prod = (long long)(diff * x) * -0x2aaaaaabLL;
                    int hi = (int)(prod >> 32);
                    x += hi - (hi >> 31);
                }
                if (p.championshipMode) x = x > 0 ? (rng(x) & 0xffff) : 0;
                ++attempts;
                if (attempts < 800 && used[idx]) continue;
                break;
            }
            if (attempts / (4 - diff) + centre < x) continue;                    // too strong for this prize and cash
            const int a2 = s == 0 ? attempts / 4 : attempts;
            if (x < (p.prizeThousands / 10 - a2) + p.cashUnits / 200) continue;   // too weak
            break;
        }
        used[idx] = true;
        e.proIndex = idx;
        e.name = pros[idx].name;
        e.skillSum = pros[idx].skillSum;
    }
    return field;
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Prizes and standings
// ---------------------------------------------------------------------------------------------------------------------------------
int defaultPrizeThousands(int holes) { return (holes + 1) * 20 - 20; }

std::vector<int> prizeLadder(int firstThousands, int holes) {
    std::vector<int> v;
    int p = firstThousands != 0 ? firstThousands : defaultPrizeThousands(holes);
    for (int i = 0; i < holes; ++i) { v.push_back(p); p = (p * 2) / 3; }
    return v;
}

std::vector<Standing> rankEntrants(const std::vector<Entrant>& field, const std::vector<int>& pars, int holes) {
    std::vector<Standing> rows;
    for (const Entrant& e : field) {
        Standing s;
        s.slot = e.slot;
        for (int h = 1; h <= holes && h <= 18 && h <= (int)pars.size(); ++h) {
            const int st = e.strokes[h - 1];
            if (pars[h - 1] <= 0 || st <= 0) continue;
            if (!e.finished && e.currentHole == h) continue;
            s.relToPar += st - pars[h - 1];
        }
        rows.push_back(s);
    }
    std::stable_sort(rows.begin(), rows.end(), [](const Standing& a, const Standing& b) {
        if (a.relToPar != b.relToPar) return a.relToPar < b.relToPar;
        return a.slot < b.slot;
    });
    for (size_t i = 0; i < rows.size(); ++i) rows[i].place = (int)i + 1;
    return rows;
}

// ---------------------------------------------------------------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------------------------------------------------------------
void Tournament::recompute(const SgaInput& in) {
    eval_ = evaluateCourse(in);
    holes_ = in.holes;
    prize_ = 0;
    name_ = nullptr;
    if (eval_.total < 1 || eval_.grade < 0) return;
    name_ = tournamentName(eval_.total, eval_.grade == 3);
    const int prize = tournamentPrizeThousands(eval_.total, eval_.grade, in.holes);
    // The exe treats only an exact zero as "no offer"; totals under 25 give a negative formula value, which is clamped to none here.
    prize_ = prize > 0 ? prize : 0;
}

bool Tournament::evaluateOffer(const SgaInput& in) {
    if (state_ == State::InProgress) return false;
    recompute(in);
    state_ = prize_ > 0 ? State::Offered : State::Idle;
    return state_ == State::Offered;
}

bool Tournament::forceOffer(const SgaInput& in) {
    if (state_ == State::InProgress) return false;
    recompute(in);
    if (prize_ <= 0) { prize_ = std::max(20, defaultPrizeThousands(in.holes)); name_ = tournamentName(std::max(1, eval_.total), false); }
    state_ = State::Offered;
    return true;
}

bool Tournament::reopenOffer(const SgaInput& in) {
    if (state_ != State::Offered) return false;
    recompute(in);
    return prize_ > 0;
}

void Tournament::decline() {
    if (state_ == State::Offered) prize_ = 0;
}

bool Tournament::accept(const std::vector<TourPro>& pros, const std::vector<int>& pars, int difficulty, int cashUnits,
                        const std::string& playerName, int playerSkillSum, const RandFn& rng, std::vector<int>* accomplishments) {
    if (state_ != State::Offered || prize_ <= 0 || pars.empty()) return false;
    pars_ = pars;
    holes_ = std::min<int>((int)pars.size(), 18);
    FieldParams fp;
    fp.holes = holes_;
    fp.difficulty = difficulty;
    fp.prizeThousands = prize_;
    fp.cashUnits = cashUnits;
    fp.championshipMode = champ_;
    field_ = selectField(pros, fp, playerName, playerSkillSum, rng);
    state_ = State::InProgress;
    if (accomplishments) {
        accomplishments->clear();
        if (champ_) return true;   // the exe's stamping routine does nothing in championship play
        accomplishments->push_back(kAccomplishmentFirstTournament);
        if (prize_ >= 500) accomplishments->push_back(kAccomplishment500kTournament);
        if (prize_ >= 1000) accomplishments->push_back(kAccomplishment1mTournament);
    }
    return true;
}

void Tournament::cancel() {
    state_ = State::Idle;
    prize_ = 0;
    field_.clear();
    pars_.clear();
}

bool Tournament::recordHole(int slot, int hole, int strokes) {
    if (state_ != State::InProgress || slot < 0 || slot >= (int)field_.size() || hole < 1 || hole > holes_ || strokes < 1) return false;
    Entrant& e = field_[slot];
    if (e.finished) return false;
    e.strokes[hole - 1] = strokes;
    int played = 0;
    for (int h = 0; h < holes_; ++h) if (e.strokes[h] > 0) ++played;
    if (played >= holes_) { e.finished = true; e.currentHole = 0; }
    else e.currentHole = hole % holes_ + 1;
    return true;
}

bool Tournament::submitRound(int slot, const std::vector<int>& strokesByHole) {
    if (state_ != State::InProgress || slot < 0 || slot >= (int)field_.size() || (int)strokesByHole.size() < holes_) return false;
    for (int h = 0; h < holes_; ++h) if (strokesByHole[h] < 1) return false;
    Entrant& e = field_[slot];
    for (int h = 0; h < holes_; ++h) e.strokes[h] = strokesByHole[h];
    e.finished = true;
    e.currentHole = 0;
    return true;
}

bool Tournament::allFinished() const {
    if (state_ != State::InProgress) return false;
    return std::all_of(field_.begin(), field_.end(), [](const Entrant& e) { return e.finished; });
}

std::vector<Standing> Tournament::standings() const {
    std::vector<Standing> rows = rankEntrants(field_, pars_, holes_);
    const std::vector<int> ladder = prizeLadder(prize_, holes_);
    for (auto& r : rows) {
        if (r.place <= (int)ladder.size()) { r.paid = true; r.prizeThousands = ladder[r.place - 1]; }
    }
    return rows;
}

bool Tournament::finish(int theme, TournamentResult& out) {
    if (!allFinished()) return false;
    out = TournamentResult();
    out.standings = standings();
    for (const Standing& s : out.standings) {
        if (s.slot != 1) continue;
        out.playerPlace = s.place;
        if (!s.paid) break;
        out.playerPrizeThousands = s.prizeThousands;
        out.cashDeltaUnits = (long)s.prizeThousands * 1000 / 100;
        out.yearlyIncomeDeltaUnits = out.cashDeltaUnits;
        out.fameDelta = s.place < 4 ? 4 - s.place : 1;
        out.historyCode = 0xE0 | s.place;
        if (s.place == 1) {
            if (holes_ >= 9) out.accomplishments.push_back(kAccomplishmentWin9Holes);
            if (holes_ >= 18) {
                out.accomplishments.push_back(kAccomplishmentWin18Holes);
                if (s.prizeThousands > 100000) out.accomplishments.push_back(kAccomplishmentGrandSlamFirst + std::clamp(theme, 0, 3));
            }
        }
        break;
    }
    state_ = State::Idle;
    prize_ = 0;
    return true;
}

int placeholderStrokes(int par, int skillSum, const RandFn& rng) {
    // PLACEHOLDER. Mean over par falls as skills rise: a skill sum of 10 is a hacker, 130 or more a top pro.
    const int r = rng(100);
    int over = 0;
    const int good = std::clamp(skillSum / 2, 0, 70);          // percent chance of par or better
    if (r < good / 5) over = -1;                               // birdie
    else if (r < good) over = 0;
    else if (r < 85) over = 1;
    else if (r < 96) over = 2;
    else over = 3;
    return std::max(1, par + over);
}

}  // namespace sg

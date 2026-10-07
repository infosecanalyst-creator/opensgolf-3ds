#include <cctype>
#include "sg/formats.h"
#include <cstring>

namespace sg {

static std::string cstr(const uint8_t* p, size_t max) {
    size_t n = 0;
    while (n < max && p[n]) n++;
    return std::string((const char*)p, n);
}
static uint32_t u32(const uint8_t* p) { return p[0] | (p[1] << 8) | (p[2] << 16) | ((uint32_t)p[3] << 24); }
static uint16_t u16(const uint8_t* p) { return (uint16_t)(p[0] | (p[1] << 8)); }

static const size_t kBaseSize = 1826, kSlotsAt = 0x230, kSlotSize = 50, kSlotCount = 25;

bool parseCharacter(const Bytes& d, Character& out, std::string& err) {
    if (d.size() < kBaseSize) { err = "character file too short"; return false; }
    out = Character();
    out.title = cstr(&d[0], 16);
    out.name = cstr(&d[16], 16);
    std::memcpy(out.attr, &d[0x20], 16);
    for (size_t i = 0; i < kSlotCount; i++) out.lines.push_back(cstr(&d[kSlotsAt + i * kSlotSize], kSlotSize));
    if (d.size() == kBaseSize) return true;

    if (d.size() < kBaseSize + 8 || std::memcmp(&d[kBaseSize], "*PCXFILE", 8) != 0) { err = "unknown data after base record"; return false; }
    Bytes pcxData(d.begin() + kBaseSize + 8, d.end());
    Rgba sheet;
    if (!decodePcx(pcxData, sheet, err)) return false;
    if (sheet.w != 140 || sheet.h != 420) { err = "unexpected portrait size"; return false; }
    for (uint32_t i = 0; i < 3; i++) {
        Rgba face;
        face.w = 140; face.h = 140;
        face.px.assign(sheet.px.begin() + (size_t)i * 140 * 140 * 4, sheet.px.begin() + (size_t)(i + 1) * 140 * 140 * 4);
        out.portraits.push_back(std::move(face));
    }
    return true;
}

bool parseTop10(const Bytes& d, std::vector<Top10Entry>& out, std::string& err) {
    const size_t R = 156;
    if (d.empty() || d.size() % R) { err = "top10 size is not a multiple of 156"; return false; }
    out.clear();
    for (size_t o = 0; o < d.size(); o += R) {
        Top10Entry e;
        e.player = cstr(&d[o], 64);
        e.course = cstr(&d[o + 64], 64);
        e.a = u32(&d[o + 128]); e.b = u32(&d[o + 132]); e.c = u32(&d[o + 136]);
        e.tier = u16(&d[o + 150]);
        out.push_back(std::move(e));
    }
    return true;
}

static std::string trim(const std::string& s) {
    size_t a = 0, b = s.size();
    while (a < b && (s[a] == ' ' || s[a] == '\t' || s[a] == '\r' || s[a] == '\n')) a++;
    while (b > a && (s[b - 1] == ' ' || s[b - 1] == '\t' || s[b - 1] == '\r' || s[b - 1] == '\n')) b--;
    return s.substr(a, b - a);
}

static std::vector<std::string> splitLines(const std::string& t) {
    std::vector<std::string> v;
    size_t pos = 0;
    while (pos <= t.size()) {
        size_t nl = t.find('\n', pos);
        if (nl == std::string::npos) nl = t.size();
        v.push_back(t.substr(pos, nl - pos));
        pos = nl + 1;
    }
    return v;
}

bool parseProGolfers(const std::string& text, std::vector<ProGolfer>& out, std::string& err) {
    DtaRows rows;
    if (!parseDta(text, rows, err)) return false;
    for (const auto& r : rows) {
        if (r.size() < 7) continue;  // header or oddly shaped row
        const std::string& s = r[6];
        // The skill field is ten hex digits, optionally followed by whitespace and a rating.
        size_t n = 0;
        while (n < s.size() && std::isxdigit((unsigned char)s[n])) n++;
        if (n < 10) continue;
        ProGolfer g;
        g.name = r[0];
        g.body = std::atoi(r[1].c_str()); g.skin = std::atoi(r[2].c_str()); g.hat = std::atoi(r[3].c_str());
        g.shirt = std::atoi(r[4].c_str()); g.pants = std::atoi(r[5].c_str());
        for (int k = 0; k < 10; k++) g.skill[k] = std::stoi(std::string(1, s[(size_t)k]), nullptr, 16);
        if (s.size() > 10) g.rating = std::atoi(s.c_str() + 10);
        out.push_back(g);
    }
    if (out.empty()) { err = "no golfers found"; return false; }
    return true;
}

bool parseDta(const std::string& text, DtaRows& out, std::string&) {
    out.clear();
    for (auto& raw : splitLines(text)) {
        std::string line = trim(raw);
        if (line.empty() || line[0] == '*') continue;
        std::vector<std::string> row;
        size_t pos = 0;
        while (true) {
            size_t c = line.find(',', pos);
            row.push_back(trim(line.substr(pos, c == std::string::npos ? c : c - pos)));
            if (c == std::string::npos) break;
            pos = c + 1;
        }
        out.push_back(std::move(row));
    }
    return true;
}

bool parseStory(const std::string& text, Story& out, std::string& err) {
    out = Story();
    auto lines = splitLines(text);
    size_t i = 0;
    while (i < lines.size() && trim(lines[i]).empty()) i++;
    if (i >= lines.size()) { err = "empty story"; return false; }
    out.title = trim(lines[i++]);
    StoryBlock cur;
    bool open = false;
    auto flush = [&] { if (open) out.blocks.push_back(std::move(cur)); cur = StoryBlock(); open = false; };
    for (; i < lines.size(); i++) {
        std::string t = trim(lines[i]);
        if (t.empty()) { flush(); continue; }
        if (!open) { cur.prompt = t; open = true; }
        else cur.replies.push_back(t);
    }
    flush();
    if (out.blocks.empty()) { err = "no dialogue blocks"; return false; }
    return true;
}

}  // namespace sg

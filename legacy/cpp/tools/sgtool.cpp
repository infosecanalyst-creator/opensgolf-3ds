// sgtool: inspect and convert original SimGolf assets.
//   sgtool check <dir>                 decode every known asset under dir, report failures
//   sgtool png   <in> <out.png> [n]    convert PCX/TGA/BMP (or frame n of an FLC) to PNG
//   sgtool sheet <in.flc> <out.png>    contact sheet of all FLC frames
//   sgtool chr   <file> [prefix]       dump a .chr/.glf/.pro character; writes prefix_{0,1,2}.png portraits
//   sgtool top10 <top10.sve>           dump the high score table
//   sgtool dta   <file.dta>            dump a .dta table
//   sgtool story <file.txt>            dump a conversation script
#include "sg/assets.h"
#include "sg/formats.h"
#include <algorithm>
#include <cstdio>
#include <filesystem>
#include <map>

namespace fs = std::filesystem;
using namespace sg;

static std::string ext(const fs::path& p) {
    std::string e = p.extension().string();
    std::transform(e.begin(), e.end(), e.begin(), [](unsigned char c) { return std::tolower(c); });
    return e;
}

static bool loadAny(const std::string& path, Rgba& out, std::string& err, size_t frame = 0) {
    Bytes d;
    if (!readFile(path, d)) { err = "cannot read"; return false; }
    std::string e = ext(path);
    if (e == ".pcx") return decodePcx(d, out, err);
    if (e == ".tga") return decodeTga(d, out, err);
    if (e == ".bmp") return decodeBmp(d, out, err);
    if (e == ".flc") {
        Flc f;
        if (!decodeFlc(d, f, err)) return false;
        if (frame >= f.frames.size()) frame = f.frames.size() - 1;
        out = toRgba(f.frames[frame]);
        return true;
    }
    err = "unknown type";
    return false;
}

int main(int argc, char** argv) {
    if (argc >= 3 && std::string(argv[1]) == "check") {
        std::map<std::string, std::pair<int, int>> stats;  // ext -> ok, fail
        for (auto& ent : fs::recursive_directory_iterator(argv[2])) {
            if (!ent.is_regular_file()) continue;
            std::string e = ext(ent.path());
            bool inThemes = ent.path().string().find("Themes") != std::string::npos;
            if (e != ".pcx" && e != ".tga" && e != ".bmp" && e != ".flc" && e != ".wav" && e != ".chr" && e != ".glf" &&
                e != ".pro" && e != ".sve" && e != ".dta" && !(e == ".txt" && inThemes)) continue;
            Bytes d; std::string err; bool ok;
            readFile(ent.path().string(), d);
            if (e == ".pcx") { Rgba r; ok = decodePcx(d, r, err); }
            else if (e == ".tga") { Rgba r; ok = decodeTga(d, r, err); }
            else if (e == ".bmp") { Rgba r; ok = decodeBmp(d, r, err); }
            else if (e == ".flc") { Flc f; ok = decodeFlc(d, f, err); }
            else if (e == ".chr" || e == ".glf" || e == ".pro") { Character c; ok = parseCharacter(d, c, err); }
            else if (e == ".sve") { std::vector<Top10Entry> t; ok = parseTop10(d, t, err); }
            else if (e == ".dta") { DtaRows r; ok = parseDta(std::string(d.begin(), d.end()), r, err); }
            else if (e == ".txt") { Story st; ok = parseStory(std::string(d.begin(), d.end()), st, err); }
            else { Wav w; ok = decodeWav(d, w, err); }
            (ok ? stats[e].first : stats[e].second)++;
            if (!ok) std::printf("FAIL %s: %s\n", ent.path().string().c_str(), err.c_str());
        }
        for (auto& [e, s] : stats) std::printf("%-5s ok=%d fail=%d\n", e.c_str(), s.first, s.second);
        return 0;
    }
    if (argc >= 4 && std::string(argv[1]) == "png") {
        Rgba img; std::string err;
        if (!loadAny(argv[2], img, err, argc > 4 ? std::stoul(argv[4]) : 0) || !writePng(argv[3], img)) {
            std::fprintf(stderr, "error: %s\n", err.c_str());
            return 1;
        }
        std::printf("%ux%u -> %s\n", img.w, img.h, argv[3]);
        return 0;
    }
    if (argc >= 4 && std::string(argv[1]) == "sheet") {
        Bytes d; Flc f; std::string err;
        if (!readFile(argv[2], d) || !decodeFlc(d, f, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        uint32_t cols = std::min<size_t>(f.frames.size(), 16), rows = (f.frames.size() + cols - 1) / cols;
        Rgba sheet; sheet.w = cols * f.w; sheet.h = rows * f.h; sheet.px.assign((size_t)sheet.w * sheet.h * 4, 0);
        for (size_t i = 0; i < f.frames.size(); i++) {
            Rgba fr = toRgba(f.frames[i]);
            for (uint32_t y = 0; y < f.h; y++)
                std::copy(&fr.px[(size_t)y * f.w * 4], &fr.px[(size_t)(y + 1) * f.w * 4],
                          &sheet.px[((size_t)((i / cols) * f.h + y) * sheet.w + (i % cols) * f.w) * 4]);
        }
        writePng(argv[3], sheet);
        std::printf("%zu frames, %ux%u each, %u ms/frame\n", f.frames.size(), f.w, f.h, f.frameMs);
        return 0;
    }
    if (argc >= 3 && std::string(argv[1]) == "chr") {
        Bytes d; Character c; std::string err;
        if (!readFile(argv[2], d) || !parseCharacter(d, c, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        std::printf("title: %s\nname:  %s\nattr: ", c.title.c_str(), c.name.c_str());
        for (int i = 0; i < 16; i++) std::printf(" %02x", c.attr[i]);
        std::printf("\n");
        for (size_t i = 0; i < c.lines.size(); i++)
            if (!c.lines[i].empty()) std::printf("  [%2zu] %s\n", i, c.lines[i].c_str());
        if (argc > 3)
            for (size_t i = 0; i < c.portraits.size(); i++) writePng(std::string(argv[3]) + "_" + std::to_string(i) + ".png", c.portraits[i]);
        std::printf("portraits: %zu\n", c.portraits.size());
        return 0;
    }
    if (argc >= 3 && std::string(argv[1]) == "top10") {
        Bytes d; std::vector<Top10Entry> t; std::string err;
        if (!readFile(argv[2], d) || !parseTop10(d, t, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        for (size_t i = 0; i < t.size(); i++)
            std::printf("%2zu. %-12s %-20s a=%u b=%u c=%u tier=%u\n", i + 1, t[i].player.c_str(), t[i].course.c_str(), t[i].a, t[i].b, t[i].c, t[i].tier);
        return 0;
    }
    if (argc >= 3 && std::string(argv[1]) == "dta") {
        Bytes d; DtaRows r; std::string err;
        if (!readFile(argv[2], d) || !parseDta(std::string(d.begin(), d.end()), r, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        for (auto& row : r) { for (size_t i = 0; i < row.size(); i++) std::printf("%s%s", i ? " | " : "", row[i].c_str()); std::printf("\n"); }
        return 0;
    }
    if (argc >= 3 && std::string(argv[1]) == "story") {
        Bytes d; Story st; std::string err;
        if (!readFile(argv[2], d) || !parseStory(std::string(d.begin(), d.end()), st, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        std::printf("== %s (%zu blocks)\n", st.title.c_str(), st.blocks.size());
        for (auto& b : st.blocks) { std::printf("- %s\n", b.prompt.c_str()); for (auto& r : b.replies) std::printf("    > %s\n", r.c_str()); }
        return 0;
    }
    std::fprintf(stderr, "usage: sgtool check|png|sheet|chr|top10|dta|story ...\n");
    return 2;
}

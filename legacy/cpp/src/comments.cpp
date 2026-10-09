// Comment sentence builder. See include/sg/comments.h. All wording is the port's own.
#include "sg/comments.h"
#include <cstring>

namespace sg {

namespace {
struct Terr { const char* sing; const char* plur; int cls; };
const Terr kTerr[23] = {
    {"tees", "tees", 0}, {"green", "green", 1}, {"fairway", "fairway", 2}, {"firm fairway", "firm fairway", 2},
    {"rough", "rough", 4}, {"deep rough", "deep rough", 4}, {"mound", "mound", 4}, {"sand trap", "sand trap", 7},
    {"waste bunker", "waste bunker", 8}, {"pot bunker", "pot bunker", 7}, {"ravine", "ravine", 4}, {"brush", "brush", 4},
    {"rocks", "rocks", 4}, {"tree", "trees", 13}, {"pine tree", "pine trees", 13}, {"palm tree", "palm trees", 13},
    {"elm tree", "elm trees", 13}, {"water", "water", 17}, {"wetlands", "wetlands", 4}, {"marsh", "marsh", 4},
    {"out of bounds", "out of bounds", 18}, {"building", "building", 18}, {"building", "building", 18}};
const char* const kLandmark[19] = {"sundial", "barn", "cannon", "stonehenge", "water mill", "rock face", "statue", "lighthouse", "Buddha",
                                   "windmill", "Easter Island head", "pagoda", "oriental house", "dinosaur tarpit", "fountain", "gazebo", "wishing well",
                                   "water tower", "oil pump"};
const char* const kAnimal[9] = {"elk", "crane", "flamingo", "sheep", "crocodile", "snake", "Gila monster", "road runner", "duck"};
// Own lists of flower beds per theme (five each).
const char* const kBeds[4][5] = {{"tulip bed", "daffodil bed", "pansy bed", "red rose bush", "lilac bush"},
                                 {"cactus flower bed", "desert marigold bed", "agave bed", "yucca bed", "red rose bush"},
                                 {"orchid bed", "hibiscus bed", "bird of paradise bed", "ginger flower bed", "plumeria bed"},
                                 {"heather bed", "gorse bush", "sea thrift bed", "red rose bush", "poppy bed"}};

void replaceFirst(std::string& s, const char* tok, const std::string& with) {
    const size_t p = s.find(tok);
    if (p != std::string::npos) s.replace(p, std::strlen(tok), with);
}
}  // namespace

const char* terrainName(int id, bool plural) { return id >= 0 && id < 23 ? (plural && kTerr[id].cls == 13 ? kTerr[id].plur : kTerr[id].sing) : ""; }
bool terrainIsTree(int id) { return id >= 0 && id < 23 && kTerr[id].cls == 13; }
const char* landmarkKindName(int k) { return k >= 0 && k < 19 ? kLandmark[k] : ""; }
const char* animalName(int i) { return i >= 0 && i < 9 ? kAnimal[i] : ""; }

std::string cellObjectName(const CellInfo& c, int theme) {
    const int id = c.tile;
    if (id == 21 || id == 22) {
        if (c.landmarkKind >= 0) return landmarkKindName(c.landmarkKind);
        if (c.flowerbedRecord) return "flower bed";
        return id == 22 ? "landmark" : "home site";
    }
    const int cls = id >= 0 && id < 23 ? kTerr[id].cls : -1;
    switch (cls) {
        case 4:
            if (id == 4) {
                if (!c.decoration) return theme == 1 ? "pile of bones" : "ornamental grass";
                if (theme < 0 || theme > 3) return "";
                return kBeds[theme][((c.objectByte % 5) + 5) % 5];
            }
            return "rose bush";
        case 7: return "rhododendron";
        case 13:
            if (id == 16) return "scenic tree";
            if (theme == 1 && id >= 13 && id <= 15) return "scenic cactus";
            if (id == 13 || id == 14) return "scenic tree";
            if (id == 15) return "statue";
            return "";
        case 17:
            if (c.bridge) return "scenic bridge";
            if (c.hasObjectByte) return "dolphin";
            return theme == 1 ? "rock formation" : "fountain";
        case 18: return "fountain";
        default: return "wildflower";
    }
}

CommentOut commentText(int type, int loc, const CommentCtx& x) {
    using C = CommentColour;
    CommentOut o;
    std::string s;
    C col = C::Neutral;
    auto good = [&](const char* t) { s = t; col = C::Good; };
    auto bad = [&](const char* t) { s = t; col = C::Bad; };
    std::string lead;   // leading score word (types 19 and 23)
    switch (type) {
        case 1: good("Did you see that one, PARTNER?"); break;
        case 2: bad("Ugh, I'm stuck DATA."); break;
        case 3: bad("Wonderful, now I'm DATA."); break;
        case 4: bad("That one should have been easy."); break;
        case 6: good("I can use this slope."); break;
        case 7: good("Love strolling across this bridge."); break;
        case 8: bad("Who laid out this course?! #@%&!"); break;
        case 9: bad("That ball nearly took my head off."); break;
        case 10: {
            const std::string n = terrainName(loc, false);
            const bool pl = !n.empty() && n.back() == 's';
            s = std::string("Do I really have to walk through ") + (pl ? "these " : "this ") + n + "?";
            col = C::Bad; break;
        }
        case 11: good("Take a look at this ADJ DATA!"); break;
        case 12: bad("Blasted DATA."); break;
        case 13: bad("My ball is sleeping with the fishes."); break;
        case 14: bad("I could use a drink."); break;
        case 15: bad("Time for some food."); break;
        case 18: good("Nothing beats a snack."); break;
        case 20: bad("What an eyesore, that DATA."); break;
        case 21: bad("Can we speed this group up?"); break;
        case 22: good("Look, that's DATA's place."); break;
        case 23: {
            const int diff = -x.par;
            switch (diff) {
                case 0: lead = "Par. "; break; case 1: case 2: case 3: lead = "Bogey. "; break;
                case -1: lead = "Birdie! "; break; case -2: lead = "Eagle! "; break; case -3: lead = "Double eagle! "; break;
                case -4: lead = "Triple eagle! "; break; default: lead = "Argh! "; break;
            }
            if (x.flags & 4) s = "This hole is too tough.";
            else if (x.flags & 8) s = "This hole is too easy.";
            else s = "This hole is either too tough or too easy.";
            col = C::Bad; break;
        }
        case 24: bad("Eww, the weeds are taking over."); break;
        case 25: good("Ah, just what I needed to drink."); break;
        case 26: s = "I'm getting worn out."; break;
        case 27: good("What a comfy bench."); break;
        case 28: good("I really like that DATA."); break;
        case 29: good("Good variety out here."); break;
        case 30:
            if ((x.flags & 0x20) && (x.prevFlags & 0x20)) s = "Another dogleg to the left, just like the last one.";
            else if ((x.flags & 0x40) && (x.prevFlags & 0x40)) s = "Another dogleg to the right, just like the last one.";
            else if (x.par == x.prevPar) s = "Yet another par " + std::to_string(x.par) + ".";
            else s = "This one feels like the last hole.";
            col = C::Bad; break;
        case 32: good("Should I go long or play safe?"); break;
        case 33: good("Left side or right side?"); break;
        case 34: {
            static const char* const r[3] = {"Fine, thanks.", "Doing great, thanks for asking.", "Never better, thanks a lot!"};
            if (loc >= 0 && loc < 3) s = r[loc];
            col = C::Good; break;
        }
        case 35: {
            static const char* const r[4] = {"I'm done with this game.", "I'll never play this again!", "Why do I even bother?", "This is the worst round ever."};
            s = r[((x.tick + 45u * 0x98u) / 80u) % 4u];
            col = C::Bad; break;
        }
        case 36: bad(loc == 0 ? "She has lost it completely." : "He has lost it completely."); break;
        case 39: good("Oops, I startled that DATA."); break;
        case 43: bad("Do I have to climb this hill?"); break;
        case 44: good("Never mind. (nice DATA)"); break;
        case 46: good("A friendly downhill lie."); break;
        case 47: bad("PARTNER, lighten up."); break;
        default: break;   // types without a case give an empty sentence
    }
    // DATA and the other tokens, each replaced once in a fixed order.
    std::string data;
    switch (type) {
        case 2: case 3: data = std::string(terrainIsTree(loc) ? "under the " : "in the ") + terrainName(loc, false); break;
        case 12: data = terrainName(loc, false); break;
        case 11: case 20: case 28: data = cellObjectName(x.cell, x.theme); break;
        case 22: data = x.celebrity; break;
        case 39: data = animalName(loc); break;
        case 44: data = terrainName(loc, false); break;
        default: break;
    }
    if (type == 11) replaceFirst(s, "ADJ", (loc & 0x100) ? "nice" : "magnificent");
    replaceFirst(s, "MYNAME", x.name);
    replaceFirst(s, "PARTNER", x.name);
    replaceFirst(s, "DATA", data);
    o.text = lead + s;
    o.colour = col;
    return o;
}

std::string defaultHoleName(int par, int n) {
    static const char* const p3[18] = {"Alexandra", "Belinda", "Carmen", "Daphne", "Elaine", "Fiona", "Gloria", "Helena", "Irene", "Josephine", "Kimberly", "Lucinda", "Marissa", "Natalie", "Ophelia", "Priscilla", "Rosalind", "Zelda"};
    static const char* const p4[18] = {"Olive", "Dogwood", "Peach", "Magnolia", "Juniper", "Azalea", "Cypress", "Hickory", "Laurel", "Maple", "Sycamore", "Camellia", "Hawthorn", "Myrtle", "Aspen", "Willow", "Cedar", "Holly"};
    static const char* const px[18] = {"Pride", "Inferno", "Fortress", "Torment", "Abyss", "Wrath", "Gehenna", "Perdition", "Brimstone", "Cauldron", "Dread", "Hades", "Scorch", "Blight", "Oblivion", "Anguish", "Doom", "Purgatory"};
    const int i = (n >= 1 && n <= 18) ? n - 1 : 0;
    return par == 3 ? p3[i] : par == 4 ? p4[i] : px[i];
}

}  // namespace sg

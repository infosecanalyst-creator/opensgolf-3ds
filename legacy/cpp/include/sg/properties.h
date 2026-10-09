// SimGolf native port: the sixteen properties of the property chooser.
// Names, sizes, prices and bonuses were read from screenshots of the original game (docs/SCREENSHOT_NOTES.md). The theme is inferred
// from the icon on each card, clearly visible in the sandbox screenshot (tree = Parkland, castle = Links, desert building = Desert,
// dolphin = Tropical). The money symbol of the game is the section sign.
#pragma once

namespace sg {

struct Property {
    const char* name;
    const char* bonus;
    int acres;
    int price;
    int theme;        // 0 Parkland, 1 Links, 2 Desert, 3 Tropical (index into the viewer's theme list)
    int column;       // 0 left, 1 right in the chooser
};

constexpr int kStartFunds = 100000;

// In the order the cards appear, top to bottom, left column first.
constexpr Property kProperties[16] = {
    {"Monterey", "Scenic Cypress", 110, 250000, 0, 0},
    {"San Diego", "Dolphins", 50, 60000, 0, 0},
    {"Rocky Mtns.", "Free Hotel", 180, 700000, 0, 0},
    {"Las Vegas", "Fun, Fun, Fun", 100, 120000, 2, 0},
    {"Phoenix", "Free Spa", 210, 1000000, 2, 0},
    {"Hawaii", "Scenic Waterfall", 70, 80000, 3, 0},
    {"Oahu", "Japanese Garden", 50, 70000, 3, 0},
    {"Nova Scotia", "Scenic Lighthouse", 170, 800000, 1, 0},
    {"Northeast", "Civil War Battlefield", 50, 50000, 0, 0},
    {"Carolina", "Free Putting Green", 160, 500000, 0, 0},
    {"Ireland", "Leprechauns", 170, 600000, 1, 1},
    {"Scotland", "Free Castle", 130, 400000, 1, 1},
    {"Wales", "Stonehenge", 80, 200000, 1, 1},
    {"Spain", "Scenic Vineyards", 120, 300000, 2, 1},
    {"Florida", "Free Pro Shop", 90, 150000, 3, 1},
    {"Jamaica", "Scenic Statues", 200, 900000, 3, 1},
};

}  // namespace sg

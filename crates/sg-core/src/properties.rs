//! The sixteen properties of the property chooser. Names, sizes, prices and bonuses were read from screenshots of the original game
//! (docs/SCREENSHOT_NOTES.md). The theme is inferred from the icon on each card (tree = Parkland, castle = Links, desert building =
//! Desert, dolphin = Tropical). The money symbol of the game is the section sign. The exe's price table (docs/PUBLISHER_EXE_NOTES.md)
//! holds the same sixteen prices.

#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub name: &'static str,
    pub bonus: &'static str,
    pub acres: i32,
    pub price: i32,
    /// 0 Parkland, 1 Links, 2 Desert, 3 Tropical.
    pub theme: usize,
    /// 0 left, 1 right in the chooser.
    pub column: i32,
}

pub const START_FUNDS: i32 = 100_000;

const fn p(name: &'static str, bonus: &'static str, acres: i32, price: i32, theme: usize, column: i32) -> Property {
    Property { name, bonus, acres, price, theme, column }
}

/// In the order the cards appear, top to bottom, left column first.
pub const PROPERTIES: [Property; 16] = [
    p("Monterey", "Scenic Cypress", 110, 250000, 0, 0),
    p("San Diego", "Dolphins", 50, 60000, 0, 0),
    p("Rocky Mtns.", "Free Hotel", 180, 700000, 0, 0),
    p("Las Vegas", "Fun, Fun, Fun", 100, 120000, 2, 0),
    p("Phoenix", "Free Spa", 210, 1000000, 2, 0),
    p("Hawaii", "Scenic Waterfall", 70, 80000, 3, 0),
    p("Oahu", "Japanese Garden", 50, 70000, 3, 0),
    p("Nova Scotia", "Scenic Lighthouse", 170, 800000, 1, 0),
    p("Northeast", "Civil War Battlefield", 50, 50000, 0, 0),
    p("Carolina", "Free Putting Green", 160, 500000, 0, 0),
    p("Ireland", "Leprechauns", 170, 600000, 1, 1),
    p("Scotland", "Free Castle", 130, 400000, 1, 1),
    p("Wales", "Stonehenge", 80, 200000, 1, 1),
    p("Spain", "Scenic Vineyards", 120, 300000, 2, 1),
    p("Florida", "Free Pro Shop", 90, 150000, 3, 1),
    p("Jamaica", "Scenic Statues", 200, 900000, 3, 1),
];

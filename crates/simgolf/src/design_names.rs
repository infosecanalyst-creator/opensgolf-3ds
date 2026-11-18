//! OpenSGolf extras: design names (extras.rs `design_names`; the port's own, docs/PLAYING.md). The original names a
//! landmark in its design strip but none of the benches, flower beds and scenic trees: their designs are pictures only,
//! and on the course nothing says what one is. With this extra the strip names the bench, flower bed or scenic tree design
//! under the pointer, and with no tool in hand the pointer on one on the course names it in a small tag.
//!
//! The names come from the art they name: the scenic trees' and benches' sprite files (sg_core::decor::sprite_file:
//! Plum, Dogwood, ScenicElm, JapaneseMaple, Cypress, Scenic_Tree, PeachTree; benW01, box bench, red bench, round wood
//! bench, backless bench, lovers bench), the flower beds' three shapes (plain, sinuous and walled pieces) and their five
//! colours per theme (the palettes, with the flowers the original's golfers name in their thoughts where they match:
//! sg_core::thoughts::feature). Purely a label: nothing of the game changes.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10};
use sg_core::land;
use sg_core::lang::{tr, trf};

/// The benches by design (sprites 0x208 on; the sixth, the lovers' bench, is the cut content's).
const BENCHES: [&str; 6] = [
    "STR_DESIGN_BENCH_PARK",
    "STR_DESIGN_BENCH_BOX",
    "STR_DESIGN_BENCH_RED",
    "STR_DESIGN_BENCH_ROUND",
    "STR_DESIGN_BENCH_BACKLESS",
    "STR_DESIGN_BENCH_LOVERS",
];

/// The scenic trees by the tile's variant (0 the willow, 1..7 sprites 0x12f on).
const TREES: [&str; 8] = [
    "STR_DESIGN_TREE_WILLOW",
    "STR_DESIGN_TREE_PLUM",
    "STR_DESIGN_TREE_DOGWOOD",
    "STR_DESIGN_TREE_ELM",
    "STR_DESIGN_TREE_MAPLE",
    "STR_DESIGN_TREE_CYPRESS",
    "STR_DESIGN_TREE_ORNAMENTAL",
    "STR_DESIGN_TREE_PEACH",
];

/// The flower beds' colours (palettes 0x2d..0x31) by theme: Parkland and Links share the first set, Desert and Tropical
/// have their own.
const FLOWERS: [[&str; 5]; 3] = [
    [
        "STR_DESIGN_FLOWERS_YELLOW",
        "STR_DESIGN_FLOWERS_ORANGE",
        "STR_DESIGN_FLOWERS_PURPLE",
        "STR_DESIGN_FLOWERS_WHITE",
        "STR_DESIGN_FLOWERS_RED",
    ],
    [
        "STR_DESIGN_FLOWERS_DESERT_RED",
        "STR_DESIGN_FLOWERS_DESERT_ORANGE",
        "STR_DESIGN_FLOWERS_DESERT_PURPLE",
        "STR_DESIGN_FLOWERS_DESERT_BLUE",
        "STR_DESIGN_FLOWERS_DESERT_PINK",
    ],
    [
        "STR_DESIGN_FLOWERS_TROPICAL_AQUA",
        "STR_DESIGN_FLOWERS_TROPICAL_ORANGE",
        "STR_DESIGN_FLOWERS_TROPICAL_PURPLE",
        "STR_DESIGN_FLOWERS_TROPICAL_BLUE",
        "STR_DESIGN_FLOWERS_TROPICAL_RED",
    ],
];

/// The flower beds' shapes (design / 5): the plain, sinuous and walled pieces.
const BEDS: [&str; 3] = ["STR_DESIGN_BED_PLAIN", "STR_DESIGN_BED_CURVED", "STR_DESIGN_BED_WALLED"];

/// The name of a tile item's design as the design strip numbers it (bench 0..4, flower bed 0..14, scenic tree 0..6), in
/// exe theme `theme`; None for the other kinds.
pub fn strip_name(kind: i32, design: i32, theme: u8) -> Option<String> {
    match kind {
        land::K_BENCH => bench_name(design as u8),
        land::K_FLOWERS => Some(bed_name(design as u8, theme)),
        land::K_WILLOW => tree_name((design + 1) as u8),
        _ => None,
    }
}

/// A bench by its tile's variant (as decor::benches reads it).
pub fn bench_name(var: u8) -> Option<String> {
    BENCHES.get((var % 7) as usize).map(|k| tr(k).to_string())
}

/// A scenic tree by its tile's variant (as decor::ornamental reads it).
pub fn tree_name(var: u8) -> Option<String> {
    Some(tr(TREES[(var & 7) as usize]).to_string())
}

/// A flower bed by its tile's design (shape var / 5, colour var % 5).
pub fn bed_name(var: u8, theme: u8) -> String {
    let set = match theme {
        1 => 1,
        2 => 2,
        _ => 0,
    };
    let v = var.min(14) as usize;
    trf("STR_DESIGN_FLOWER_BED", &[("FLOWERS", &tr(FLOWERS[set][v % 5])), ("BED", &tr(BEDS[v / 5]))])
}

impl App {
    /// The name of the bench, scenic tree or flower bed on tile (a, b), if there is one.
    pub fn design_at(&self, a: i32, b: i32) -> Option<String> {
        let land = self.land.as_ref()?;
        if !(0..land::mn()).contains(&a) || !(0..land::mn()).contains(&b) {
            return None;
        }
        let i = (a * land::mn() + b) as usize;
        let (f, ty, var) = (land.flags[i], land.ty[i], land.var[i]);
        if ty == land::T_OUT {
            return None;
        }
        // in the order the tile loop draws them (app.rs add_tile_items)
        if f & 0x200 != 0 {
            bench_name(var)
        } else if ty == land::T_ELM && f & 0x100 != 0 {
            tree_name(var)
        } else if f & 0x1000 != 0 {
            Some(bed_name(var, self.exe_theme()))
        } else {
            None
        }
    }

    /// The tag over the pointer naming the bench, scenic tree or flower bed under it (draw_hud, after the name tag): with
    /// the extra on, no tool in hand, and no golfer or employee tag showing.
    pub fn draw_design_tag(&self, g: &mut Gfx, s: &Ui) {
        if !crate::extras::design_names() || self.screen != Screen::Play || self.edit || self.xs.touched || self.card_ui.held.is_some() {
            return;
        }
        let (px, py) = self.info.pointer;
        if self.dock_hover >= 0 || (px < 230.0 && py > 430.0) || (self.panel != 0 && px > 220.0 && py > 446.0) {
            return;
        }
        if crate::extras::name_tags() && !self.xs.tags_hidden && self.name_tag_shown() {
            return;
        }
        let Some((a, b)) = self.hover_tile else { return };
        let Some(name) = self.design_at(a, b) else { return };
        let f = crate::textsize::big(F_ARIAL10);
        let k = crate::textsize::scale();
        let w = f.width(&name) + 10.0;
        let h = (15.0 * k).round();
        let top = (py - h - 14.0).floor().max(2.0);
        let left = (px - w / 2.0).floor().clamp(2.0, 798.0 - w);
        s.fill(g, left - 1.0, top - 1.0, w + 2.0, h + 2.0, rgba(0.85, 0.85, 1.0, 0.55));
        s.fill(g, left, top, w, h, rgba(0.0, 0.0, 0.1, 0.62));
        s.put_shadowed(g, f, left + 5.0, top + 2.0, &name, c15(0x7fff), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text that is a key, not words (a key missing from lang/en.txt comes back as itself).
    fn is_key(t: &str) -> bool {
        t.chars().all(|c| c.is_ascii_uppercase() || c == '_')
    }

    #[test]
    fn every_design_has_a_name() {
        for theme in 0..4 {
            for d in 0..15 {
                let n = strip_name(land::K_FLOWERS, d, theme).unwrap();
                assert!(!is_key(&n) && n.contains(','), "{n}");
            }
        }
        for d in 0..5 {
            assert!(!is_key(&strip_name(land::K_BENCH, d, 0).unwrap()));
        }
        for d in 0..7 {
            assert!(!is_key(&strip_name(land::K_WILLOW, d, 0).unwrap()));
        }
        assert_eq!(tree_name(4).as_deref(), Some("Japanese maple"));
        assert_eq!(strip_name(land::K_LANDMARK, 0, 0), None);
    }
}

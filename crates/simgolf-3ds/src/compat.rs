//! What the game's modules take from its desktop main.rs, for the 3DS build.
use crate::app::App;
use crate::gfx::Gfx;
use crate::{render, ui};

/// The desktop event loop's state, as far as photo.rs (photo mode) reaches into it. The 3DS frontend has its own loop;
/// photo mode is not offered there.
pub struct Stage {
    pub app: App,
    pub g: Gfx,
    pub dragging: bool,
    pub editing: bool,
    pub mouse: (f32, f32),
    pub shift: bool,
}

impl Stage {
    pub fn draw_frame(&mut self, _target: Option<miniquad::RenderPass>) {}
}

/// The title and interface art (main.rs load_art, unchanged).
pub fn load_art(app: &mut App, g: &mut Gfx) -> bool {
    let mut img = |rel: &str, magenta: bool, key: Option<u32>| ui::load_pcx(g, &app.game_path(&format!("Interface/{rel}")), magenta, key);
    let base = img("TitleBASE.pcx", false, None);
    let un = img("TitleUnSel.pcx", true, None);
    let mo = img("TitleMO.pcx", true, None);
    let world = img("WorldBase.pcx", false, None);
    let diff_base = img("TitleSelDiffUnSel.pcx", false, None);
    let diff_mo = img("TitleSelDiffMO.pcx", true, None);
    let report = img("infoscreens/coursereport.pcx", true, None); // optional
    let dock = img("3mainLowerLeft.pcx", false, Some(0xF800F8)); // optional: the lower left dock
    const ICON: [&str; 4] = ["ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"];
    let icons: Vec<Option<ui::Image>> = ICON.iter().map(|f| img(f, false, Some(0xFF0000))).collect(); // icons are optional
    let ok = base.is_some() && un.is_some() && mo.is_some() && world.is_some();
    app.title_base = base.unwrap_or_default();
    app.title_un = un.unwrap_or_default();
    app.title_mo = mo.unwrap_or_default();
    let mut rects: Vec<(f32, f32, f32, f32)> = render::MENU_BTN.iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
    rects.push((170.0, 190.0, 480.0, 165.0)); // the logo
    app.title_mo_parts = ui::split_overlay(g, &app.game_path("Interface/TitleMO.pcx"), &rects);
    app.world_base = world.unwrap_or_default();
    app.diff_base = diff_base.unwrap_or_default();
    app.hud_art =
        ui::load_pcx_alpha(g, &app.game_path("Interface/courseinfo.pcx"), &app.game_path("Interface/courseinfo_A.pcx")).unwrap_or_default();
    app.diff_mo = diff_mo.unwrap_or_default();
    app.report_art = report.unwrap_or_default();
    app.dock_art = dock.unwrap_or_default();
    for (t, i) in icons.into_iter().enumerate() {
        app.theme_icons[t] = i.unwrap_or_default();
    }
    // the terrain brushes' tile pictures (0x445a43): 57 x 40 cells, eight to a row
    for (t, f) in ["parkland", "desert", "tropical", "links"].iter().enumerate() {
        app.tool_tiles[t] = ui::load_pcx(g, &app.game_path(&format!("Data/{f}.pcx")), true, None).unwrap_or_default();
    }
    // the bank rocks: in HD with the classic picture's key (hd::load_pcx_classic_key, docs/FIXES.md 1)
    let cliffs = app.game_path("cliffs01.pcx");
    app.cliffs = crate::hd::active()
        .then(|| crate::hd::load_pcx_classic_key(g, &cliffs))
        .flatten()
        .or_else(|| ui::load_pcx(g, &cliffs, true, None))
        .unwrap_or_default();
    app.ui_ok = ok;
    app.art = crate::screens_ui::Art::load(g, app);
    app.register_portraits(g);
    app.reports = crate::reports_ui::ReportArt::load(g, app);
    app.panel_art = crate::panels_ui::PanelArt::load(g, app);
    app.title.art = crate::files_ui::TitleArt::load(g, app);
    app.hud = crate::hud_ui::HudArt::load(g, app);
    app.info.art = crate::info_ui::InfoArt::load(g, app);
    ok
}

/// Leading comma separated integers, like sscanf("%d,%d,...") (main.rs's, which stops at the first that does not parse).
pub fn ints(s: &str) -> Vec<i32> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let p = part.trim_start();
        let digits = p.strip_prefix('-').unwrap_or(p);
        if !digits.starts_with(|c: char| c.is_ascii_digit()) {
            break;
        }
        let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
        let n: i32 = digits[..end].parse().unwrap_or(0);
        v.push(if p.starts_with('-') { -n } else { n });
    }
    v
}

/// The desktop's scripted edits (main.rs's --editor-set, also replayed from a bug report's action log): not on the 3DS.
pub fn apply_edit_spec(_app: &mut App, _spec: &str) {}

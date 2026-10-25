//! The two screens and how the game is split across them.
//!
//! Top (400 x 240, stereoscopic): the course, `App::render_world`, at native resolution; the HUD's few in-world labels at
//! zero parallax. Bottom (320 x 240, touch): the dock, build palette, reports and panels, which the original lays out in an
//! 800 x 600 virtual screen. Each `*_ui.rs` screen is given a bottom-screen layout one at a time (the port's long tail); until
//! then `BOTTOM_VIRTUAL` scales the original's whole 800 x 600 onto the bottom screen so everything is at least reachable.
//!
//! The 3DS framebuffers are portrait (240 wide, 400/320 tall) and the GPU draws into them rotated; gfx.rs's projection
//! fix-up hides that, so everything here is in the landscape pixels the player sees.

pub const TOP_W: u32 = 400;
pub const TOP_H: u32 = 240;
pub const BOTTOM_W: u32 = 320;
pub const BOTTOM_H: u32 = 240;

/// The original's virtual screen the UI code positions itself in.
pub const VIRTUAL_W: f32 = 800.0;
pub const VIRTUAL_H: f32 = 600.0;

/// Which screen a draw lands on. The renderer draws the top screen twice (left and right eye) when the 3D slider is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Top,
    Bottom,
}

/// Scale and offset that fit the original 800 x 600 UI onto the bottom screen, letterboxed (0.4 x). The interim mapping;
/// readable layouts replace it screen by screen.
pub struct View {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
}

pub fn bottom_virtual() -> View {
    let scale = (BOTTOM_W as f32 / VIRTUAL_W).min(BOTTOM_H as f32 / VIRTUAL_H);
    View { scale, ox: (BOTTOM_W as f32 - VIRTUAL_W * scale) * 0.5, oy: (BOTTOM_H as f32 - VIRTUAL_H * scale) * 0.5 }
}

/// A touch at bottom-screen pixel (x, y) as a position in the original's 800 x 600 UI space.
pub fn touch_to_virtual(x: f32, y: f32) -> (f32, f32) {
    let v = bottom_virtual();
    ((x - v.ox) / v.scale, (y - v.oy) / v.scale)
}

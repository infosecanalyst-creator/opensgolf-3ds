//! The frame rate cap and v-sync (the port's own; Enhanced graphics > Display, enhanced.rs `frame_cap` and `vsync`; both
//! count in Classic too). Only how often the screen is drawn changes: the simulation runs on its fixed 87 ms ticks
//! whatever the frame rate (main.rs update).
//!
//! On the desktop the cap waits out the rest of each frame's time after the frame is shown; v-sync is the swap interval the
//! window is made with (miniquad sets it once, so a change applies at the next start; macOS and Android always pace to the
//! display). In a browser the page skips animation frames to keep under the cap (web/opensgolf.js), and v-sync is always on:
//! browsers draw on the display's refresh.

/// The cap's choices in frames a second, in the setting's order (0: uncapped).
pub const CAPS: [u32; 6] = [30, 60, 120, 144, 240, 0];
/// The setting's default: uncapped (what the game always did).
pub const UNCAPPED: u8 = 5;

/// The cap in use in frames a second (0: none).
pub fn cap() -> u32 {
    CAPS[crate::enhanced::settings().frame_cap.min(UNCAPPED) as usize]
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static NEXT: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

/// After a frame is shown: waits until the next frame is due under the cap.
#[cfg(not(target_arch = "wasm32"))]
pub fn pace() {
    use std::time::{Duration, Instant};
    let fps = cap();
    if fps == 0 {
        NEXT.with(|n| n.set(None));
        return;
    }
    let period = Duration::from_secs_f64(1.0 / fps as f64);
    let now = Instant::now();
    let due = NEXT.with(|n| n.get()).filter(|t| *t + period > now);
    let next = match due {
        Some(t) => {
            if t > now {
                std::thread::sleep(t - now);
            }
            t + period
        }
        // more than a frame late (or the first frame): the next one a whole period from now
        None => now + period,
    };
    NEXT.with(|n| n.set(Some(next)));
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn sg_frame_cap(fps: u32);
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static TOLD: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// In a browser: tells the page the cap when it changes (the page skips animation frames).
#[cfg(target_arch = "wasm32")]
pub fn pace() {
    let fps = cap();
    if TOLD.with(|t| t.replace(Some(fps))) != Some(fps) {
        // SAFETY: a plain call into the page's script with a number
        unsafe { sg_frame_cap(fps) };
    }
}

/// The swap interval the window is made with: 1 with v-sync (the default), 0 without.
pub fn swap_interval() -> i32 {
    crate::enhanced::saved().vsync as i32
}

//! Playing upright (the port's own; the interface extra `portrait`, extras.rs). SimGolf's screens are 800 x 600, so the
//! Android app is held landscape (Cargo.toml) and the browser asks an upright phone to turn sideways (web/files.js). With
//! the extra on, the device is told it may turn every way: Android's activity follows the sensor (MainActivity.setPortrait)
//! and the page lets the canvas fill the tall screen (web/opensgolf.js sg_portrait). The game itself needs nothing more:
//! it reads the window's size every frame, the course view fills it, and the interface is laid out at the screen's edges
//! (enhanced::widescreen), so the dock and build palette sit along the bottom and the money and ratings at the top
//! corners, with the boxes and reports across the width in the middle.

/// Tells the device whether it may be held upright (extras::set, at the start and on each change of the tick).
pub fn tell_device(on: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "env")]
        extern "C" {
            fn sg_portrait(on: u32);
        }
        // SAFETY: a plain call into the page's script with a number
        unsafe { sg_portrait(on as u32) };
    }
    #[cfg(target_os = "android")]
    crate::android::set_portrait(on);
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    let _ = on;
}

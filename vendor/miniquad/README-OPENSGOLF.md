# miniquad 0.4.11, patched for OpenSGolf

This is miniquad 0.4.11 from crates.io (MIT or Apache-2.0, see LICENSE-MIT and LICENSE-APACHE), used through
`[patch.crates-io]` in the workspace's Cargo.toml. Its `[profile]` and `[dev-dependencies]` sections were dropped (unused
here), and `src/lib.rs` allows rustc's warnings (as Cargo caps them for a crates.io dependency). One change, in `src/native/windows.rs` (marked "OpenSGolf"):

- `SetWindowLongPtrA(wnd, GWLP_USERDATA, ...)` and the fullscreen style change were compiled only for
  `target_arch = "x86_64"` and `"x86"`. On 64-bit ARM Windows (aarch64-pc-windows-msvc) the window procedure therefore
  never found the display: mouse, keyboard, resize and close messages all went to `DefWindowProcW`, so the game ignored
  clicks, and closing the window destroyed it while the program (and its music) kept running. They now follow
  `target_pointer_width`, as the matching `GetWindowLongPtrA` call already did.

Drop this copy when an upstream release carries the fix.

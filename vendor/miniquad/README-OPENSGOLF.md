# miniquad 0.4.11, patched for OpenSGolf

This is miniquad 0.4.11 from crates.io (MIT or Apache-2.0, see LICENSE-MIT and LICENSE-APACHE), used through
`[patch.crates-io]` in the workspace's Cargo.toml. Its `[profile]` and `[dev-dependencies]` sections were dropped (unused
here), and `src/lib.rs` allows rustc's warnings (as Cargo caps them for a crates.io dependency). Three changes (marked "OpenSGolf"). In `src/native/windows.rs`:

- `SetWindowLongPtrA(wnd, GWLP_USERDATA, ...)` and the fullscreen style change were compiled only for
  `target_arch = "x86_64"` and `"x86"`. On 64-bit ARM Windows (aarch64-pc-windows-msvc) the window procedure therefore
  never found the display: mouse, keyboard, resize and close messages all went to `DefWindowProcW`, so the game ignored
  clicks, and closing the window destroyed it while the program (and its music) kept running. They now follow
  `target_pointer_width`, as the matching `GetWindowLongPtrA` call already did.

In `src/native/egl.rs` and `src/native/android.rs` (Android only):

- The EGL setup (`create_egl_context_android`) asks for an OpenGL ES 3 config and context first, then ES 2, each with
  the upstream attributes, then without multisampling, then with any depth buffer. Upstream asserted that its one
  config request (ES 1 renderable type, at least `sample_count` samples) matched and made an ES 2 context, which closed
  the app at start on drivers without such a config, and gave an ES 2 context on drivers that keep to the version asked
  for. A failure (no context, no window surface, eglMakeCurrent) panics with what was tried and the EGL error.

In `java/MainActivity.java`: the activity loads `libopensgolf.so` by name. cargo-quad-apk 0.1.4 replaces `LIBRARY_NAME`
with the last part of the Android package name (`org.opensgolf.game` gives `game`), while the library it packs is named
after the crate (`libopensgolf.so`), so every build closed at start with `UnsatisfiedLinkError: dlopen failed: library
"libgame.so" not found` (tools/android_build.sh now checks the two match).

Also in `java/MainActivity.java` (OpenSGolf's own, not an upstream fix): `pickGameFiles`, `importState`,
`onActivityResult` and `restartApp` for "Use my SimGolf files..." on Android (crates/simgolf/src/android.rs): Android's
own picker for a .zip or a folder, the pick copied into the app's folders on a thread, and the app started again. A
newer upstream would need these carried over.

Drop this copy when an upstream release carries the fixes.

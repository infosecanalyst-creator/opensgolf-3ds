//! Android link fixes for the app's shared library (libopensgolf.so), which the phone loads when the app starts.
//!
//! * Sound goes through cpal's oboe output, whose C++ (oboe-sys) asks for the NDK's static C++ library (`c++_static`). Since
//!   NDK r22 that archive no longer holds the C++ ABI runtime (`__cxa_throw`, `__gxx_personality_v0`, `__cxa_pure_virtual`,
//!   std::runtime_error ...): those live in `libc++abi.a`, which `clang++` adds on its own but the plain `clang` Rust links
//!   with does not. Without it the library was built with those symbols unresolved and Android refused to load it ("cannot
//!   locate symbol"), so the app closed the moment it was opened (v0.1.0). The link arguments go after every library on the
//!   command line, so `-lc++abi` resolves what oboe's objects left open.
//! * `-z defs` makes any symbol still unresolved a link error, so a library the phone cannot load is never packed again
//!   (tools/android_build.sh then builds the app without sound instead).
//! * `-z max-page-size=16384` lays the library's segments out on 16 KB boundaries. Android 15 and newer phones may run
//!   with 16 KB memory pages, and their loader refuses a library aligned for 4 KB pages (NDK r25's lld lays out x86_64
//!   for 4 KB); 16 KB alignment also loads on 4 KB phones.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os != "android" {
        return;
    }
    if std::env::var_os("CARGO_FEATURE_AUDIO").is_some() {
        println!("cargo:rustc-link-arg=-lc++abi");
    }
    // two arguments, which both a linker driver (clang) and the linker itself (cargo-quad-apk calls lld directly) take
    println!("cargo:rustc-link-arg=-z");
    println!("cargo:rustc-link-arg=defs");
    println!("cargo:rustc-link-arg=-z");
    println!("cargo:rustc-link-arg=max-page-size=16384");
}

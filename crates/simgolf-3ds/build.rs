//! Compiles the PICA200 vertex shaders with devkitPro's `picasso` into OUT_DIR/*.shbin, which gfx.rs embeds with
//! include_bytes!. picasso ships with devkitARM ($DEVKITPRO/tools/bin/picasso).
use std::{env, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let picasso = env::var("DEVKITPRO")
        .ok()
        .map(|p| PathBuf::from(p).join("tools/bin/picasso"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("picasso"));
    for (src, bin) in [("shader/main.v.pica", "main.shbin"), ("shader/terrain.v.pica", "terrain.shbin")] {
        let src = PathBuf::from(src);
        println!("cargo:rerun-if-changed={}", src.display());
        let status = Command::new(&picasso)
            .arg("-o")
            .arg(out.join(bin))
            .arg(&src)
            .status()
            .unwrap_or_else(|e| panic!("could not run {} (is devkitARM installed and DEVKITPRO set?): {e}", picasso.display()));
        assert!(status.success(), "picasso failed on {}", src.display());
    }
    selftest(&out);
}

/// Hardware test hooks (3ds-test-suite hwtest/selftest): inactive unless the test agent starts the game with
/// --selftest=<script>. The C files are built into a static library; the linker wraps the HID and APT calls.
/// heap3ds.c sets up the heaps leaving a few MB uncommitted (the next program failed to load after SimGolf quit).
fn selftest(out: &PathBuf) {
    println!("cargo:rerun-if-changed=selftest3ds.c");
    println!("cargo:rerun-if-changed=heap3ds.c");
    let dkp = env::var("DEVKITPRO").unwrap_or_else(|_| "/opt/devkitpro".into());
    let arm = env::var("DEVKITARM").unwrap_or_else(|_| format!("{dkp}/devkitARM"));
    let mut objs = Vec::new();
    // heap3ds.c goes into the same object as the selftest hooks: that object is always linked (the wrapped
    // HID calls need it), so its strong __system_allocateHeaps replaces libctru's weak one.
    for src in ["selftest3ds.c"] {
        let obj = out.join(src.replace(".c", ".o"));
        let ok = Command::new(format!("{arm}/bin/arm-none-eabi-gcc"))
            .args(["-march=armv6k", "-mtune=mpcore", "-mfloat-abi=hard", "-mtp=soft", "-O2", "-D__3DS__"])
            .arg(format!("-I{dkp}/libctru/include"))
            .args(["-include", "heap3ds.c", "-c", src, "-o"])
            .arg(&obj)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "could not build {src}");
        objs.push(obj);
    }
    let lib = out.join("libselftest3ds.a");
    let _ = std::fs::remove_file(&lib);
    let ok = Command::new(format!("{arm}/bin/arm-none-eabi-ar")).arg("rcs").arg(&lib).args(&objs).status().map(|s| s.success()).unwrap_or(false);
    assert!(ok, "could not archive selftest3ds.o");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=selftest3ds");
    for f in ["hidScanInput", "hidKeysHeld", "hidKeysDown", "hidKeysDownRepeat", "hidKeysUp", "hidTouchRead", "hidCircleRead", "irrstCstickRead", "aptMainLoop"] {
        println!("cargo:rustc-link-arg=-Wl,--wrap={f}");
    }
}

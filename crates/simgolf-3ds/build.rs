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
}

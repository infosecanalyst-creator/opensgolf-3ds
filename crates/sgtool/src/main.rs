//! sgtool: inspect and convert the original SimGolf assets.
//!   sgtool check <dir>                 decode every known asset under dir, report failures
//!   sgtool png   <in> <out.png> [n]    convert PCX/TGA/BMP (or frame n of an FLC) to PNG
//!   sgtool sheet <in.flc> <out.png>    contact sheet of all FLC frames
//!   sgtool chr   <file> [prefix]       dump a .chr/.glf/.pro character; writes prefix_{0,1,2}.png portraits
//!   sgtool top10 <top10.sve>           dump the high score table
//!   sgtool dta   <file.dta>            dump a .dta table
//!   sgtool story <file.txt>            dump a conversation script
//!   sgtool fixture <dir> [font.ttf]    write a stand-in game folder with placeholder art (for tests)
mod fixture;

use sg_core::assets::{decode_bmp, decode_pcx, decode_tga, decode_wav, to_rgba, Rgba};
use sg_core::flc::decode_flc;
use sg_core::formats::*;
use sg_core::fsutil::{ext_lower, walk};
use sg_core::png::write_png;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::exit;

fn read(p: &str) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| {
        eprintln!("error: cannot read {p}: {e}");
        exit(1)
    })
}

fn fail(e: String) -> ! {
    eprintln!("error: {e}");
    exit(1)
}

fn load_any(path: &str, frame: usize) -> Result<Rgba, String> {
    let d = std::fs::read(path).map_err(|_| "cannot read".to_string())?;
    match ext_lower(Path::new(path)).as_str() {
        ".pcx" => decode_pcx(&d),
        ".tga" => decode_tga(&d),
        ".bmp" => decode_bmp(&d),
        ".flc" => {
            let f = decode_flc(&d)?;
            Ok(to_rgba(&f.frames[frame.min(f.frames.len() - 1)], None))
        }
        _ => Err("unknown type".into()),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let cmd = a.get(1).map(|s| s.as_str()).unwrap_or("");
    match (cmd, a.len()) {
        ("check", n) if n >= 3 => {
            let mut stats: BTreeMap<String, (i32, i32)> = BTreeMap::new();
            for p in walk(Path::new(&a[2])) {
                let e = ext_lower(&p);
                let in_themes = p.to_string_lossy().contains("Themes");
                let known = [".pcx", ".tga", ".bmp", ".flc", ".wav", ".chr", ".glf", ".pro", ".sve", ".dta"];
                if !known.contains(&e.as_str()) && !(e == ".txt" && in_themes) {
                    continue;
                }
                let d = std::fs::read(&p).unwrap_or_default();
                let r: Result<(), String> = match e.as_str() {
                    ".pcx" => decode_pcx(&d).map(|_| ()),
                    ".tga" => decode_tga(&d).map(|_| ()),
                    ".bmp" => decode_bmp(&d).map(|_| ()),
                    ".flc" => decode_flc(&d).map(|_| ()),
                    ".chr" | ".glf" | ".pro" => parse_character(&d).map(|_| ()),
                    ".sve" => parse_top10(&d).map(|_| ()),
                    ".dta" => Ok(()),
                    ".txt" => parse_story(&latin1(&d)).map(|_| ()),
                    _ => decode_wav(&d).map(|_| ()),
                };
                let s = stats.entry(e).or_default();
                match r {
                    Ok(()) => s.0 += 1,
                    Err(err) => {
                        s.1 += 1;
                        println!("FAIL {}: {err}", p.display());
                    }
                }
            }
            for (e, (ok, bad)) in stats {
                println!("{e:<5} ok={ok} fail={bad}");
            }
        }
        ("png", n) if n >= 4 => {
            let frame = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
            let img = load_any(&a[2], frame).unwrap_or_else(|e| fail(e));
            if !write_png(&a[3], &img) {
                fail(format!("cannot write {}", a[3]));
            }
            println!("{}x{} -> {}", img.w, img.h, a[3]);
        }
        ("sheet", n) if n >= 4 => {
            let f = decode_flc(&read(&a[2])).unwrap_or_else(|e| fail(e));
            let cols = f.frames.len().min(16) as u32;
            let rows = (f.frames.len() as u32).div_ceil(cols);
            let mut sheet = Rgba::new(cols * f.w, rows * f.h);
            for (i, fr) in f.frames.iter().enumerate() {
                let img = to_rgba(fr, None);
                let (cx, cy) = (i as u32 % cols, i as u32 / cols);
                for y in 0..f.h {
                    let src = (y * f.w * 4) as usize;
                    let dst = (((cy * f.h + y) * sheet.w + cx * f.w) * 4) as usize;
                    sheet.px[dst..dst + (f.w * 4) as usize].copy_from_slice(&img.px[src..src + (f.w * 4) as usize]);
                }
            }
            write_png(&a[3], &sheet);
            println!("{} frames, {}x{} each, {} ms/frame", f.frames.len(), f.w, f.h, f.frame_ms);
        }
        ("chr", n) if n >= 3 => {
            let c = parse_character(&read(&a[2])).unwrap_or_else(|e| fail(e));
            println!("title: {}\nname:  {}", c.title, c.name);
            println!("attr: {}", c.attr.iter().map(|b| format!(" {b:02x}")).collect::<String>());
            for (i, l) in c.lines.iter().enumerate().filter(|(_, l)| !l.is_empty()) {
                println!("  [{i:2}] {l}");
            }
            if let Some(prefix) = a.get(3) {
                for (i, p) in c.portraits.iter().enumerate() {
                    write_png(format!("{prefix}_{i}.png"), p);
                }
            }
            println!("portraits: {}", c.portraits.len());
        }
        ("top10", n) if n >= 3 => {
            let t = parse_top10(&read(&a[2])).unwrap_or_else(|e| fail(e));
            for (i, e) in t.iter().enumerate() {
                println!("{:2}. {:<12} {:<20} a={} b={} c={} tier={}", i + 1, e.player, e.course, e.a, e.b, e.c, e.tier);
            }
        }
        ("dta", n) if n >= 3 => {
            for row in parse_dta(&latin1(&read(&a[2]))) {
                println!("{}", row.join(" | "));
            }
        }
        ("story", n) if n >= 3 => {
            let st = parse_story(&latin1(&read(&a[2]))).unwrap_or_else(|e| fail(e));
            println!("== {} ({} blocks)", st.title, st.blocks.len());
            for b in &st.blocks {
                println!("- {}", b.prompt);
                for r in &b.replies {
                    println!("    > {r}");
                }
            }
        }
        ("fixture", n) if n >= 3 => {
            fixture::write_fixture(Path::new(&a[2]), a.get(3).map(Path::new));
            println!("wrote a placeholder game folder to {}", a[2]);
        }
        _ => {
            eprintln!("usage: sgtool check|png|sheet|chr|top10|dta|story|fixture ...");
            exit(2);
        }
    }
}

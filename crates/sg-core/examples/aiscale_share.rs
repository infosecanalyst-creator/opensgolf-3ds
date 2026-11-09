//! The share of the game's art the AI upscaler works out (hd_make.rs AI_WORKED_SHARE): `aiscale_share <game folder>`.
fn main() {
    let game = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let (mut total, mut worked) = (0f64, 0f64);
    for s in sg_core::hdgen::collect(&game, None) {
        let Ok(Some(p)) = sg_core::hdgen::prepare(&s) else { continue };
        for q in &p.pictures {
            let (w, h) = (q.img.w as usize, q.img.h as usize);
            total += (w * h) as f64;
            if q.mask {
                continue;
            }
            let need: Vec<bool> = q.img.px.chunks_exact(4).map(|c| q.wrap || c[3] != 0).collect();
            worked += sg_core::aiscale::worked_px(&need, w, h);
        }
    }
    println!("total {:.0} M px, worked {:.0} M px, share {:.3}", total / 1e6, worked / 1e6, worked / total);
}

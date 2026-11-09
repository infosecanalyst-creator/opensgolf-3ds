//! Times the built-in AI upscaler (sg_core::aiscale) on a PNG and writes its x4 output: `aiscale_try in.png out.png`.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let d = std::fs::read(&a[1]).unwrap();
    let img = sg_core::png::decode_png(&d).unwrap();
    let (w, h) = (img.w as usize, img.h as usize);
    let rgb: Vec<u8> = img.px.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let t = std::time::Instant::now();
    let out = sg_core::aiscale::upscale4(&rgb, w, h, None);
    let s = t.elapsed().as_secs_f64();
    println!(
        "{w}x{h} in {s:.2}s = {:.0} px/s = {:.1} GMAC/s",
        (w * h) as f64 / s,
        (w * h) as f64 * sg_core::aiscale::MACS_PER_PX / s / 1e9
    );
    let mut o = sg_core::assets::Rgba::new(w as u32 * 4, h as u32 * 4);
    for (i, q) in o.px.chunks_exact_mut(4).enumerate() {
        q.copy_from_slice(&[out[i * 3], out[i * 3 + 1], out[i * 3 + 2], 255]);
    }
    sg_core::png::write_png(&a[2], &o);
}

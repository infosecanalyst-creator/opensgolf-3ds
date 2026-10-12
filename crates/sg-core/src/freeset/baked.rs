//! Pre-rendered art of the free set: sprite sheets rendered offline from our own 3D models by tools/freeset_art (they
//! cannot be drawn at run time in reasonable time) and kept in packs/opensgolf-free/art/. Each sheet is an 8-bit palette
//! PNG holding all frames of one sprite file, a row per view; it becomes the FLC the engine reads, with the layout of the
//! manifest (frame size, views, frames, anchor). Sprites without a sheet are drawn at run time (sprite.rs).
use super::enc::{flc as write_flc, FlcHead};

include!("baked_list.rs");

/// The file of the art folder at `rel` (its path under art/, any case), if there is one.
pub fn get(rel: &str) -> Option<&'static [u8]> {
    let key = rel.to_lowercase().replace('\\', "/");
    FILES.binary_search_by(|(p, _)| p.cmp(&key.as_str())).ok().map(|i| FILES[i].1)
}

/// The FLC of a sprite (`path` as the manifest spells it, e.g. `Flics/Trees/TreeMapleLarge.flc`) from its sheet. Shadow
/// sheets keep their shading indices and take the shadow palette.
pub fn flc(path: &str, head: &FlcHead, shadow_pal: Option<&[[u8; 3]; 256]>) -> Option<Vec<u8>> {
    let stem = &path[..path.rfind('.').unwrap_or(path.len())];
    let png = get(&format!("{stem}.png"))?;
    let (w, h, idx, pal) = crate::png::decode_png_indexed(png)?;
    // a sheet may have a frame of its own (buildings drawn at the world's scale): "w h ax ay" in its "frame" text
    let mut head = *head;
    if let Some(t) = png_text(png, "frame") {
        let v: Vec<u32> = t.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        if let [fw, fh, ax, ay] = v[..] {
            if ax <= 240 && ay <= 240 && head.views > 0 {
                head.w = fw;
                head.h = fh;
                head.crop = (240 - ax, 240 - ay);
            }
        }
    }
    let head = &head;
    let views = head.views.max(1);
    let fpv = head.fpv.max(1);
    if w != head.w * fpv || h != head.h * views {
        return None;
    }
    let (fw, fh) = (head.w as usize, head.h as usize);
    let mut frames = Vec::with_capacity((views * fpv) as usize);
    for v in 0..views as usize {
        for f in 0..fpv as usize {
            let mut fr = Vec::with_capacity(fw * fh);
            for y in 0..fh {
                let row = (v * fh + y) * w as usize + f * fw;
                fr.extend_from_slice(&idx[row..row + fw]);
            }
            frames.push(fr);
        }
    }
    Some(write_flc(head, shadow_pal.unwrap_or(&pal), &frames))
}

/// The text of a PNG tEXt chunk by its keyword.
fn png_text(d: &[u8], key: &str) -> Option<String> {
    let mut p = 8;
    while p + 12 <= d.len() {
        let len = u32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]) as usize;
        let body = d.get(p + 8..p + 8 + len)?;
        if &d[p + 4..p + 8] == b"tEXt" {
            if let Some(z) = body.iter().position(|&b| b == 0) {
                if &body[..z] == key.as_bytes() {
                    return Some(String::from_utf8_lossy(&body[z + 1..]).into_owned());
                }
            }
        }
        p += 12 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_list_is_sorted_and_lower_case() {
        for w in super::FILES.windows(2) {
            assert!(w[0].0 < w[1].0, "{} {}", w[0].0, w[1].0);
        }
        for (p, d) in super::FILES {
            assert_eq!(*p, p.to_lowercase());
            assert!(!d.is_empty());
        }
    }
}

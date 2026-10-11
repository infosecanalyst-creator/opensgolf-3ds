//! The OpenSGolf application icon: a putting green under a blue sky, a red flag on its pin and a golf ball at the hole,
//! drawn with the free set's painter (our own art, nothing of the original's). `sgtool icon DIR` writes it at the sizes the
//! desktop and Android packages want; the game also gives it to the window (miniquad's window icon).
use super::paint::{mix, rgb, sd_ellipse, sd_poly, sd_rect, sd_segment, shade, Canvas};
use crate::assets::Rgba;

/// The icon at `size` x `size` pixels, with alpha (transparent outside its rounded square).
pub fn app_icon(size: usize) -> Rgba {
    let mut c = Canvas::new(size, size, [0.0; 4]);
    // drawn on a 256 x 256 grid
    let k = size as f32 / 256.0;
    let s = |v: f32| v * k;
    let full = (0.0, 0.0, size as f32, size as f32);
    let (sky_top, sky_low) = (rgb(70, 140, 220), rgb(170, 215, 245));
    let (grass, green) = (rgb(58, 140, 54), rgb(96, 186, 80));

    // the rounded square: sky fading to the horizon
    let corner = s(44.0);
    let square = |px: f32, py: f32| sd_rect(px, py, s(8.0), s(8.0), s(240.0), s(240.0), corner);
    c.fill(full, square, |_, py| mix(sky_top, sky_low, py / s(170.0)));
    // the fairway: a wide hill across the bottom, clipped to the square
    c.fill(
        full,
        |px, py| square(px, py).max(sd_ellipse(px, py, s(128.0), s(250.0), s(190.0), s(92.0))),
        |_, py| mix(shade(grass, 0.1), shade(grass, -0.25), (py - s(160.0)) / s(90.0)),
    );
    // the putting green, lighter, with its hole
    c.fill(
        full,
        |px, py| sd_ellipse(px, py, s(128.0), s(186.0), s(92.0), s(30.0)),
        |px, _| mix(shade(green, 0.15), green, (px - s(40.0)) / s(180.0)),
    );
    c.ellipse(s(150.0), s(186.0), s(13.0), s(5.0), rgb(30, 50, 25));

    // the pin, from the hole up, and its flag waving to the left
    let pole = |px: f32, py: f32| sd_segment(px, py, s(150.0), s(184.0), s(150.0), s(46.0)) - s(4.0);
    c.fill(full, pole, |px, _| if px < s(150.0) { rgb(250, 250, 245) } else { rgb(200, 200, 195) });
    let flag = [(s(147.0), s(44.0)), (s(66.0), s(70.0)), (s(147.0), s(98.0))];
    c.fill(full, |px, py| sd_poly(px, py, &flag), |_, py| mix(rgb(235, 50, 45), rgb(185, 25, 30), (py - s(44.0)) / s(54.0)));

    // the ball, resting on the green to the left of the hole: a soft shadow, the ball shaded from the top left, dimples
    let (bx, by, br) = (s(96.0), s(176.0), s(26.0));
    c.fill(full, |px, py| sd_ellipse(px, py, bx + s(6.0), by + br - s(2.0), br * 1.1, s(7.0)), |_, _| [20.0, 60.0, 20.0, 0.45]);
    c.fill(
        full,
        |px, py| sd_ellipse(px, py, bx, by, br, br),
        |px, py| {
            let d = ((px - (bx - br * 0.4)).powi(2) + (py - (by - br * 0.45)).powi(2)).sqrt() / (br * 1.6);
            mix(rgb(255, 255, 255), rgb(185, 195, 200), d)
        },
    );
    if size >= 48 {
        for (dx, dy) in [(-0.35, 0.1), (0.05, 0.3), (0.4, 0.05), (0.15, -0.25), (-0.15, 0.55), (0.45, 0.45), (-0.5, 0.45)] {
            c.ellipse(bx + dx * br, by + dy * br, br * 0.09, br * 0.08, [150.0, 160.0, 165.0, 0.6]);
        }
    }
    c.to_rgba()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes() {
        for n in [16, 32, 64, 256] {
            let i = app_icon(n);
            assert_eq!((i.w, i.h, i.px.len()), (n as u32, n as u32, n * n * 4));
            // the corners are clear, the middle is drawn
            assert_eq!(i.px[3], 0);
            let mid = (n / 2 * n + n / 2) * 4;
            assert_eq!(i.px[mid + 3], 255);
        }
    }
}

//! Home sites (object kind 5): a 2 x 2 lot sold the moment it is placed, worth what the views around it and the fun of the
//! nearest good hole make it (0x42ef40), and re-valued now and then so the house drawn on it grows with the course.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Home sites"), restated in our own words.

use crate::course::{inside, Course};
use crate::golfer::Hole;
use crate::land::{K_HOME_SITE, T_HOME_SITE, T_OUT, T_ROUGH, T_WATER};

/// The price byte of each type (+0x23).
const PRICE: [i32; 23] = [5, 10, 3, 3, 1, 2, 4, 6, 4, 8, 10, 4, 4, 10, 10, 10, 25, 50, 2, 6, 10, 0, 0];

/// What one tile around a lot adds to its view (0x42ee80).
fn view(c: &Course, a: i32, b: i32) -> i32 {
    if !inside(a, b) {
        return 0;
    }
    let t = c.ty[crate::course::idx(a, b)];
    let row = crate::course::TYPES[(t as usize).min(22)];
    if t == T_OUT {
        0
    } else if t == T_ROUGH {
        12
    } else if t == T_HOME_SITE {
        -16
    } else if row.hazard < 1 {
        -8
    } else if t == T_WATER {
        32
    } else if t == 18 && c.theme == 1 {
        16
    } else if row.class == 13 {
        PRICE[t as usize] * 5 / 2
    } else {
        PRICE[t as usize] * row.hazard as i32 / 2
    }
}

/// Distance in tiles, truncated (0x40acd0).
fn dist(dx: i32, dy: i32) -> i32 {
    (((dx * dx + dy * dy) as f64).sqrt()) as i32
}

/// The value of a lot anchored at (a, b): its ring of twelve neighbouring tiles times the best "fun over distance" of the
/// holes golfers have played, scaled down; a Marina in use raises it. `holes` are the hole records (index 1..18 used).
pub fn lot_value(c: &Course, holes: &[Hole], difficulty: i32, a: i32, b: i32) -> i32 {
    let mut ring = 0;
    for x in a - 1..=a + 2 {
        ring += view(c, x, b - 1) + view(c, x, b + 2);
    }
    for y in b..=b + 1 {
        ring += view(c, a - 1, y) + view(c, a + 2, y);
    }
    let mut best = 0;
    for h in holes.iter().skip(1).take(18) {
        if h.tee_shots == 0 {
            continue;
        }
        let mut fun = h.mood_sum * 1000 / (h.plans / 2 + 4 + h.tee_shots) + (3 - difficulty) * 100;
        if h.flags & 1 != 0 {
            fun += 100;
        }
        if h.flags & 2 != 0 {
            fun += 100;
        }
        let mut d = dist(a - h.back.0, b - h.back.1).min(dist(a - h.pin.0, b - h.pin.1));
        let (mx, my) = h.markers[2];
        if mx != -1 {
            d = d.min(dist(a - (mx >> 10), b - (my >> 10)));
        }
        best = best.max(fun / (d + 8));
    }
    let marina = c.oper[12];
    if marina != 0 {
        best += marina * best / 3;
    }
    best * ring / 40
}

/// The house size a home site's value draws (0x40e5b0): 0 a sign, 1 a house being built, 2 and 3 a house, 4 a house with a
/// fence round it.
pub fn house_size(val: i32) -> i32 {
    let n = val / 200;
    match n {
        0 => 0,
        1..=2 => 1,
        3..=5 => 2,
        6..=9 => 3,
        _ => 4,
    }
}

/// Re-values the home sites whose turn it is at a charge interval (main frame 0x417a08): every (difficulty + 2)th object,
/// starting from (tick / interval) % (difficulty + 2), keeps a smoothed value v = value + v - v / 12. Returns the objects
/// re-valued (each one also draws a celebrity, see the VIP code).
pub fn revalue(objects: &mut [crate::land::Object], c: &Course, holes: &[Hole], difficulty: i32, tick: u32) -> Vec<usize> {
    let stride = (difficulty + 2) as usize;
    let interval = 1024 / (difficulty as u32 + 2);
    let mut out = Vec::new();
    let mut i = ((tick / interval) as usize) % stride;
    while i < 256 && i < objects.len() {
        if objects[i].kind == K_HOME_SITE {
            let v = objects[i].val;
            objects[i].val = lot_value(c, holes, difficulty, objects[i].a, objects[i].b) + (v - v / 12);
            out.push(i);
        }
        i += stride;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::{idx, t};

    #[test]
    fn values() {
        let mut c = Course::default();
        let mut holes = vec![Hole::default(); 20];
        for h in holes.iter_mut() {
            h.markers = [(-1, -1); 3];
        }
        // a lot in plain rough with no played holes is worth nothing
        assert_eq!(lot_value(&c, &holes, 1, 10, 10), 0);
        holes[1].tee_shots = 10;
        holes[1].mood_sum = 30;
        holes[1].back = (10, 14);
        holes[1].pin = (20, 14);
        let rough = lot_value(&c, &holes, 1, 10, 10);
        // fun = 30*1000/14 + 200 = 2342; d = 4 → 195; ring = 12 * 12 = 144 → 195*144/40
        assert_eq!(rough, 195 * 144 / 40);
        for b in 9..13 {
            c.ty[idx(9, b)] = t::WATER;
        }
        assert!(lot_value(&c, &holes, 1, 10, 10) > rough);
        assert_eq!(house_size(0), 0);
        assert_eq!(house_size(450), 1);
        assert_eq!(house_size(2400), 4);
    }
}

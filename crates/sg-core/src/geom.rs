//! The exe's integer geometry, shared by the golfers, the shot planner and the ball: 1024 map units per tile, angles where a
//! full turn is 2^32 (0 = -y, a quarter turn = +x), and the "range units" of shot distances (25 per tile).
//!
//! Every routine here reproduces the integer behaviour of the matching exe routine, including its rounding and overflow,
//! because golfer decisions and the random number sequence depend on the exact values (docs/PUBLISHER_EXE_NOTES.md).

use std::sync::OnceLock;

/// Map units per tile.
pub const UNIT: i32 = 1024;
/// Direction table, 8 headings: 0 = -y, 2 = +x, 4 = +y, 6 = -x; odd headings are diagonals.
pub const DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
pub const DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// Quarter-wave sine table: 256 entries of sin(i * pi/2/255) * 65535, truncated (built at start-up by the exe).
fn sine_table() -> &'static [i32; 257] {
    static T: OnceLock<[i32; 257]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0i32; 257];
        for (i, v) in t.iter_mut().enumerate().take(256) {
            *v = ((i as f64 * 0.006159985596078431).sin() * 65535.0) as i32;
        }
        // The exe reads one entry past its table at the very end of a quarter turn; the table ends at the peak value.
        t[256] = 65535;
        t
    })
}

/// r * sin(a), the exe's table sine with linear interpolation and its three magnitude ranges (32-bit products wrap).
pub fn sinr(a: u32, r: i32) -> i32 {
    let (mut a, mut r) = (a, r);
    if a & 0x8000_0000 != 0 {
        r = r.wrapping_neg();
        a &= 0x7fff_ffff;
    }
    if a & 0x4000_0000 != 0 {
        a = 0x7fff_ffff - a;
    }
    let t = sine_table();
    let i = ((a & 0x3fff_ffff) >> 22) as usize;
    let f = (a & 0x3f_ffff) as i32;
    let v = t[i] + ((t[i + 1] - t[i]).wrapping_mul(f) >> 22);
    if r < 0xffff {
        v.wrapping_mul(r) >> 16
    } else if r < 0xff_ffff {
        v.wrapping_mul(r >> 8) >> 8
    } else {
        v.wrapping_mul(r >> 16)
    }
}

/// r * cos(a).
pub fn cosr(a: u32, r: i32) -> i32 {
    sinr(a.wrapping_add(0x4000_0000), r)
}

/// Angle of the vector (dx, dy): the exe's polynomial arctangent.
pub fn angle(dx: i32, dy: i32) -> u32 {
    crate::staff::angle(dx, dy)
}

/// Heading 0..7 nearest to an angle.
pub fn dir8(a: u32) -> i32 {
    ((((a as i32) >> 28) + 1) >> 1) & 7
}

/// Coarse heading of a vector without trigonometry (quadrant diagonal, or the axis when one leg is over twice the other).
pub fn cdir(dx: i32, dy: i32) -> i32 {
    crate::staff::octant(dx, dy)
}

/// Length of a vector, with the exe's overflow guard: a leg above 0x4000 is divided by 8 first (both large: factor 64).
pub fn len(dx: i32, dy: i32) -> i32 {
    crate::staff::true_dist(dx, dy)
}

/// Distance in range units (25 per tile) from a map point to the centre of tile (ta, tb).
pub fn tdist(px: i32, py: i32, ta: i32, tb: i32) -> i32 {
    let d = len(px - (ta * UNIT + 512), py - (tb * UNIT + 512));
    d.wrapping_mul(25) / 1024
}

/// The planner's distance in range units: each leg is scaled to range units first, then the length is truncated.
pub fn isqrt_ru(dx: i32, dy: i32) -> i32 {
    let a = (dx.wrapping_mul(25) >> 10) as f64;
    let b = (dy.wrapping_mul(25) >> 10) as f64;
    (a * a + b * b).sqrt() as i32
}

/// The exe's clamp: below lo gives lo; above hi gives hi only when lo <= hi.
pub fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    let v = if v < lo { lo } else { v };
    if v > hi && lo <= hi {
        hi
    } else {
        v
    }
}

/// Sign: 1, 0 or -1.
pub fn sign(v: i32) -> i32 {
    v.signum()
}

/// Signed difference of two angles.
pub fn adiff(a: u32, b: u32) -> i32 {
    a.wrapping_sub(b) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_quadrants() {
        assert_eq!(sinr(0, 1000), 0);
        assert!((sinr(0x4000_0000, 1000) - 1000).abs() <= 1);
        assert!((sinr(0x8000_0000, 1000)).abs() <= 1);
        assert!((sinr(0xc000_0000, 1000) + 1000).abs() <= 1);
        assert!((cosr(0, 1000) - 1000).abs() <= 1);
        // 30 degrees
        assert!((sinr(0x1555_5555, 1000) - 500).abs() <= 2);
    }

    #[test]
    fn headings() {
        assert_eq!(dir8(angle(0, -100)), 0);
        assert_eq!(dir8(angle(100, 0)), 2);
        assert_eq!(dir8(angle(0, 100)), 4);
        assert_eq!(dir8(angle(-100, 0)), 6);
        assert_eq!(dir8(angle(100, 100)), 3);
    }

    #[test]
    fn range_units() {
        assert_eq!(tdist(512, 512, 0, 0), 0);
        assert_eq!(tdist(512 + 1024 * 4, 512, 0, 0), 100);
        assert_eq!(clamp(5, 0, 3), 3);
        assert_eq!(clamp(5, 4, 3), 5);
    }
}

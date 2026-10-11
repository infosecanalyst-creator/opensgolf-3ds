//! Ball flight and range rules read from the publisher's golf.exe (see docs/PUBLISHER_EXE_NOTES.md, "Maximum range and ball flight").
//! All integer maths as in the exe: one tile is 1024 units, and a shot "range" of 25 is one tile. What a range unit is in yards is
//! NOT known; the game uses RANGE_UNITS_PER_TILE and a placeholder tick rate, both marked below.

/// From the exe: target distance in world units = range * 1024 / 25.
pub const RANGE_UNITS_PER_TILE: i32 = 25;
/// One game tick in milliseconds at normal speed. The exe's main loop waits until (175 + 75 if a game-state flag is set) / 2 ms
/// have passed since the frame started, and one tick is one frame: 87 ms (125 ms with the flag). See docs/PUBLISHER_EXE_NOTES.md.
pub const TICK_MS: f32 = 87.0;
/// Ticks per real second at normal speed (about 11.5).
pub const TICKS_PER_SECOND: f32 = 1000.0 / TICK_MS;
/// Ticks in one game month (the exe's date stamp counts months in blocks of 1024 ticks).
pub const TICKS_PER_MONTH: i32 = 1024;

/// Coarse range estimate the exe uses while searching for the launch speed (its routine at 0x4223f0): steps stand for two ticks.
pub fn coarse_distance(mut speed: i32, mut vertical: i32) -> i32 {
    let (mut dist, mut height) = (0i32, 0i32);
    loop {
        dist = dist.wrapping_add((speed + if speed < 0 { 7 } else { 0 }) >> 3);
        height = height.wrapping_add((vertical + if vertical < 0 { 15 } else { 0 }) >> 4);
        speed -= speed >> 4;
        vertical -= 128;
        if height <= 0 {
            break;
        }
    }
    dist
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Launch {
    pub speed: i32,
    pub vertical: i32,
}

/// The vertical launch speed for a shot of the given range, and the horizontal speed found by bisection so the carry equals 4/5 of
/// the range.
pub fn launch_for(range: i32) -> Launch {
    let u = range * 20 / 25;
    let est = (u * 33 - (u * u) / 48) + 64;
    let vertical = ((est + if est < 0 { 7 } else { 0 }) >> 3) + 512;
    let carry = range * 4 / 5;
    let target = (carry << 10) / 25;
    let (mut speed, mut step) = (est, est / 2);
    loop {
        let d = coarse_distance(speed, vertical);
        if target < d {
            speed -= step;
        }
        if d < target {
            speed += step;
        }
        step /= 2;
        if step <= 2 {
            break;
        }
    }
    Launch { speed, vertical }
}

/// Flight of one shot with the exe's per-tick rules.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arc {
    /// Ticks in the air.
    pub ticks: i32,
    /// Distance covered, 1024 units per tile.
    pub distance: f32,
    pub peak: f32,
}

pub fn simulate(range: i32) -> Arc {
    let l = launch_for(range);
    let (mut speed, mut vert, mut height) = (l.speed, l.vertical, 0i32);
    let mut dist: i64 = 0;
    let mut a = Arc::default();
    loop {
        dist += (speed >> 4) as i64;
        height += vert >> 5;
        speed -= speed >> 5;
        vert -= 64;
        a.peak = a.peak.max(height as f32);
        a.ticks += 1;
        if !(height > 0 && a.ticks < 4000) {
            break;
        }
    }
    a.distance = dist as f32;
    a
}

/// Maximum range of a shot before the cap of 330, from the exe's routine at 0x422530. Inputs follow the exe's meaning as far as it
/// is decoded: base_byte is the per-golfer byte used on difficulties above 0, length_digit and accuracy_digit are 0..9 skill digits
/// (-1 when the golfer lacks the skill flag), pro adds the pro bonuses, hazard is the terrain hazard severity under the ball (0..3
/// after clamping), on_tee is true for a tee shot.
pub fn max_range(difficulty: i32, base_byte: i32, length_digit: i32, accuracy_digit: i32, pro: bool, hazard: i32, on_tee: bool) -> i32 {
    let mut r = if difficulty < 1 {
        if pro {
            40
        } else {
            25
        }
    } else {
        (base_byte * 50) / 3
    };
    r += 150 + if pro { 50 } else { 0 };
    if length_digit >= 0 {
        r += -20 + length_digit * 4;
    }
    if accuracy_digit >= 0 && on_tee {
        r += (accuracy_digit - 5) * 6;
    }
    let hazard = hazard.clamp(0, 3);
    if hazard > 0 {
        r -= (hazard * r) / 8;
    }
    if !on_tee {
        r -= r / 5;
    }
    r.min(330)
}

/// How far up the screen a ball in the air is drawn above its ground point, in 800 x 600 pixels (0x415652..0x4157b5): the
/// height step times the zoom times the ball's height (+0xdc), divided by 5 and then by 16, each toward zero. With the step
/// at 5 that is height * zoom / 16. `zoom` is the exe's zoom step (1 far .. 4 near); between steps the port's continuous
/// zoom is used as it is.
pub fn ball_lift(height: i32, zoom: f32) -> f32 {
    let v = (crate::course::HEIGHT_STEP_PX as f32 * zoom * height as f32 / 5.0).trunc();
    (v / 16.0).trunc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ball_lift_is_height_times_zoom_over_16() {
        assert_eq!(ball_lift(0, 4.0), 0.0);
        assert_eq!(ball_lift(15, 1.0), 0.0);
        assert_eq!(ball_lift(16, 1.0), 1.0);
        assert_eq!(ball_lift(160, 4.0), 40.0);
        assert_eq!(ball_lift(161, 2.0), 20.0);
        assert_eq!(ball_lift(-17, 1.0), -1.0);
    }
}

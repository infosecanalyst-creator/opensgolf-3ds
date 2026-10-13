//! Course size: the edge of the square map in tiles. The original game's map is 50 x 50 tiles (51 x 51 corner heights, 25
//! yards and 1024 map units a tile); OpenSGolf can also start a game on a larger map (docs/PLAYING.md, "Course size").
//!
//! One map is in play at a time, so the size is a process-wide setting that the course, land and golfer code read through
//! `n()` instead of a constant. A game sets it when it starts or loads (`set`); a test (or any code that wants to look at a
//! map of another size on one thread) uses `scoped`, which overrides it for the current thread only.
//!
//! The Original size runs exactly the original rules. On the larger sizes the generator, the land deal and the tracts scale
//! with the map (see `scale`, `land::generate`, `land::slot_acres`, `tracts`), and one original distance bug is fixed
//! (`staff::true_dist`).

use std::cell::Cell;
use std::sync::atomic::{AtomicI32, Ordering};

/// The original map's edge in tiles.
pub const ORIGINAL: i32 = 50;

/// The largest edge the engine accepts (map coordinates, tile indices and the per-tile buffers all stay far inside their
/// types at this size; see docs/MODDING.md, "Course size").
pub const MAX: i32 = 200;

/// The sizes the New Game screen offers: (edge in tiles, name text key). The first is the original's.
pub const SIZES: [(i32, &str); 4] =
    [(ORIGINAL, "STR_COURSE_SIZE_ORIGINAL"), (74, "STR_COURSE_SIZE_LARGE"), (98, "STR_COURSE_SIZE_HUGE"), (128, "STR_COURSE_SIZE_GIANT")];

static GLOBAL: AtomicI32 = AtomicI32::new(ORIGINAL);

thread_local! {
    /// A per-thread override (0: none).
    static LOCAL: Cell<i32> = const { Cell::new(0) };
}

/// The map edge in tiles for the map in play.
#[inline]
pub fn n() -> i32 {
    let l = LOCAL.with(|c| c.get());
    if l != 0 {
        l
    } else {
        GLOBAL.load(Ordering::Relaxed)
    }
}

/// Tiles on the map (n x n).
#[inline]
pub fn tiles() -> usize {
    let n = n();
    (n * n) as usize
}

/// Corner heights on the map ((n + 1) x (n + 1)).
#[inline]
pub fn corners() -> usize {
    let n = n() + 1;
    (n * n) as usize
}

/// Whether the map in play is the original's size (the rules run 1:1).
#[inline]
pub fn original() -> bool {
    n() == ORIGINAL
}

/// A valid edge: a size outside ORIGINAL..=MAX falls back to the original's.
pub fn valid(n: i32) -> i32 {
    if (ORIGINAL..=MAX).contains(&n) {
        n
    } else {
        ORIGINAL
    }
}

/// Sets the size for the whole process (a game starting or loading). Clears this thread's override.
pub fn set(n: i32) {
    GLOBAL.store(valid(n), Ordering::Relaxed);
    LOCAL.with(|c| c.set(0));
}

/// Overrides the size on this thread until the guard is dropped.
pub fn scoped(n: i32) -> Scoped {
    let prev = LOCAL.with(|c| c.replace(valid(n)));
    Scoped { prev }
}

pub struct Scoped {
    prev: i32,
}

impl Drop for Scoped {
    fn drop(&mut self) {
        LOCAL.with(|c| c.set(self.prev));
    }
}

/// `v` scaled by the edge ratio n / 50, truncated (v itself on the original map).
#[inline]
pub fn scale(v: i32) -> i32 {
    v * n() / ORIGINAL
}

/// `v` scaled by the area ratio (n / 50)^2, rounded to nearest (v itself on the original map).
#[inline]
pub fn scale_area(v: i32) -> i32 {
    let n = n() as i64;
    ((v as i64 * n * n + 1250) / 2500) as i32
}

/// Money scaled by a course size's edge ratio n / 50 (the property prices and starting funds; v itself on the original map).
pub fn scale_money_at(n: i32, v: i64) -> i64 {
    v * valid(n) as i64 / ORIGINAL as i64
}

/// The path searches' cost cap (the exe keeps a step's cost in a byte: below 0x100). A larger map scales it with the edge, so
/// a walk across the map still fits.
#[inline]
pub fn route_cap() -> i32 {
    scale(0x100)
}

/// The path searches' ring buffer length (the exe's 1024 entries; a larger map gets twice its tile count, a power of two).
#[inline]
pub fn route_queue() -> usize {
    if original() {
        1024
    } else {
        (tiles() * 2).next_power_of_two()
    }
}

/// The size's name for an edge (the nearest offered size's, or "N x N").
pub fn name(n: i32) -> String {
    match SIZES.iter().find(|s| s.0 == n) {
        Some(s) => crate::lang::tr(s.1).to_string(),
        None => format!("{n} x {n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_override_and_scaling() {
        assert_eq!(n(), ORIGINAL);
        assert_eq!(scale(16), 16);
        assert_eq!(scale_area(250), 250);
        {
            let _g = scoped(100);
            assert_eq!(n(), 100);
            assert_eq!(tiles(), 10_000);
            assert_eq!(corners(), 101 * 101);
            assert_eq!(scale(16), 32);
            assert_eq!(scale_area(250), 1000);
        }
        assert_eq!(n(), ORIGINAL);
        let _g = scoped(7);
        assert_eq!(n(), ORIGINAL, "sizes below the original fall back to it");
    }
}

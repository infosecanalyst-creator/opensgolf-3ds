//! Golden values from the C++ port (src/*.cpp), so the Rust port keeps producing the same courses, shots, ratings and exe maths.
//! They were captured by running the same scenario through both implementations; the outputs matched line for line.
use sg_core::{economy::Economy, flight, holes::*, shot::ShotSim, terrain::*};

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// Runs one golfer through the demo hole like the parity harness did: returns (step, event, final stroke) at the end.
fn play(t: &Terrain, seed: u32) -> (i32, &'static str, i32, f32, f32) {
    let mut s = ShotSim::default();
    s.looping = false;
    s.init(t, seed);
    let mut i = 0;
    let mut last = (0, "", 0, 0.0, 0.0);
    while i < 30 * 120 && !s.finished {
        s.step(t, 1.0 / 30.0);
        if i % 37 == 0 {
            last = (i, s.event, s.stroke, s.ball_x, s.ball_z);
        }
        i += 1;
    }
    last
}

#[test]
fn demo_courses_match_cpp() {
    for (seed, hash, upkeep, near, last_step, strokes) in [
        (7u32, 0xcaad5ef2046ea4e9u64, "151.040000", 38, 1961, 8),
        (12345, 0x6c053b4f5dd1c717, "150.440000", 38, 1998, 8),
        (99, 0x811b607feb2ab7fd, "151.400000", 26, 1813, 7),
    ] {
        let t = Terrain::demo_course(40, 40, seed);
        assert_eq!(fnv(&t.to_course_text()), hash, "course text, seed {seed}");
        assert_eq!(format!("{:.6}", Economy::upkeep_for(&t)), upkeep);
        assert_eq!(analyze_hole(&t).hazards_near_line, near);
        let (step, event, stroke, bx, bz) = play(&t, seed);
        assert_eq!((step, event, stroke), (last_step, "holed", strokes), "shot trace, seed {seed}");
        assert_eq!(format!("{bx:.4} {bz:.4}"), "767.4089 -1400.0000");
    }
}

#[test]
fn hole_rating_matches_cpp() {
    let t = Terrain::demo_course(40, 40, 7);
    let h = &find_holes(&t)[0];
    let r = rate_hole(&t, h, 10, 1);
    assert_eq!(
        format!("{:.4} {} {:.4} {:.4} {:.4} {:.4} {}", h.length, h.par, r.len, r.acc, r.avg_strokes, r.avg_drive, r.type_index),
        "3623.5342 5 3.2000 0.3000 5.6000 1053.3409 1"
    );
}

#[test]
fn exe_flight_maths() {
    let want = [
        (1, 4, 520, 17, 0.0, 72.0),
        (50, 1521, 680, 22, 1525.0, 121.0),
        (125, 3180, 906, 29, 3822.0, 210.0),
        (200, 4634, 1113, 35, 6215.0, 306.0),
        (330, 6870, 1427, 45, 10447.0, 506.0),
    ];
    for (r, speed, vertical, ticks, dist, peak) in want {
        let l = flight::launch_for(r);
        let a = flight::simulate(r);
        assert_eq!((l.speed, l.vertical, a.ticks, a.distance, a.peak), (speed, vertical, ticks, dist, peak), "range {r}");
    }
    assert_eq!(flight::max_range(0, 3, 5, 5, true, 2, true), 180);
    assert_eq!(flight::max_range(2, 4, 9, -1, false, 1, false), 163);
    assert_eq!(flight::max_range(1, 7, 3, 8, false, 0, true), 276);
}

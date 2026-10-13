//! Course sizes (sg_core::mapsize): new-game land, the course built from it, tracts and saves at every offered size, and
//! (ignored by default: `cargo test --release -p sg-core --test course_sizes -- --ignored --nocapture`) the cost of a busy
//! 18-hole course's ticks and of the longest walk's path search at each size.

use sg_core::course::{f, idx, t, Course};
use sg_core::golfer::{Club, Hole, SLOTS};
use sg_core::land::{self, ExeRng, Land, Noise, Slot};
use sg_core::mapsize;

fn new_land(n: i32, property: usize, slot: usize, difficulty: i32, sandbox: bool, clock: u32) -> Land {
    let _size = mapsize::scoped(n);
    let mut rng = ExeRng::from_clock(clock);
    let noise = Noise::new(&mut rng);
    let acres = land::slot_acres(slot as i32, property, sandbox);
    land::generate(slot, Slot { property, acres }, difficulty, sandbox, &mut rng, &noise)
}

#[test]
fn every_size_generates_every_property() {
    for &(n, _) in mapsize::SIZES.iter() {
        let _size = mapsize::scoped(n);
        let (tiles, corners) = ((n * n) as usize, ((n + 1) * (n + 1)) as usize);
        for p in 0..16 {
            for d in [0, 3] {
                let l = new_land(n, p, 7, d, false, 1000 + p as u32 * 31 + d as u32);
                assert_eq!(l.n, n);
                assert_eq!((l.ty.len(), l.flags.len(), l.var.len(), l.height.len()), (tiles, tiles, tiles, corners));
                // the clubhouse in the middle of the map, on its own footprint
                let (a, b) = l.clubhouse;
                let mid = (n - mapsize::ORIGINAL) / 2;
                assert!((15 + mid..32 + mid).contains(&a) && (15 + mid..32 + mid).contains(&b), "size {n} property {p}: {a},{b}");
                assert_eq!(l.tile(a, b), land::T_BUILDING);
                assert!(l.height.iter().all(|&h| h <= 17));
                assert!(l.objects.len() < 0xff, "size {n}: {} objects", l.objects.len());
                // the owned land is in play, the corners of the map are not (inland and coast deals)
                assert!(l.in_play(a, b));
                if sg_core::data::records()[p].coast == 0 {
                    assert!(!l.in_play(0, 0) && !l.in_play(n - 1, n - 1));
                }
                let tr = l.to_terrain();
                assert_eq!((tr.w, tr.h, tr.ty.len(), tr.corner.len()), (n, n, tiles, corners));
                let mut c = Course::default();
                c.sync(&l, false);
                assert_eq!((c.ty.len(), c.height.len(), c.walls.len()), (tiles, corners, tiles));
                assert_eq!(c.ty_at(a, b), t::BUILDING);
                assert_eq!(c.ty_at(n, 0), t::OUT, "off the map reads as out of bounds");
            }
        }
    }
}

#[test]
fn deals_scale_with_the_map() {
    for &(n, _) in mapsize::SIZES.iter() {
        let _size = mapsize::scoped(n);
        let q = (n * n) as f64 / 2500.0;
        for slot in 0..16 {
            let a50 = {
                let _o = mapsize::scoped(mapsize::ORIGINAL);
                land::slot_acres(slot, 3, false)
            };
            assert_eq!(land::slot_acres(slot, 3, false), (a50 as f64 * q).round() as i32);
        }
        // the owned share of the map is about the same as on the original's for the same slot
        let share = |l: &Land| l.ty.iter().filter(|&&t| t != land::T_OUT).count() as f64 / l.ty.len() as f64;
        let here = share(&new_land(n, 9, 10, 1, false, 77));
        let orig = share(&new_land(mapsize::ORIGINAL, 9, 10, 1, false, 77));
        assert!((here - orig).abs() < 0.08, "size {n}: owned share {here:.2} against {orig:.2}");
        assert_eq!(mapsize::scale_money_at(n, 50_000), 50_000 * n as i64 / 50);
    }
}

#[test]
fn tracts_cover_the_map_at_every_size() {
    for &(n, _) in mapsize::SIZES.iter() {
        let mut l = new_land(n, 9, 4, 1, false, 4242);
        let _size = mapsize::scoped(n);
        let side = sg_core::tracts::side();
        assert_eq!(side, (n - 2) / 3);
        let (a8, b8) = sg_core::tracts::origin(8);
        assert!(a8 + side <= n && b8 + side <= n);
        let mut rng = ExeRng::from_clock(9);
        let tracts = sg_core::tracts::roll(&l, 0, &mut rng);
        let oob: i32 = tracts.iter().map(|t| t.oob).sum();
        assert!(oob > 0, "size {n}: land for sale");
        // the price per out-of-bounds tile is about the same at every size (the roll is per tile)
        for (i, tr) in tracts.iter().enumerate().filter(|(_, t)| t.oob > 200) {
            let per = tr.price as f64 / tr.oob as f64;
            assert!((0.4..1.3).contains(&per), "size {n} tract {i}: {} for {} tiles", tr.price, tr.oob);
        }
        let i = tracts.iter().position(|t| t.oob > 0).unwrap();
        sg_core::tracts::buy(&mut l, i);
        let after = sg_core::tracts::roll(&l, 1, &mut rng);
        assert_eq!(after[i].oob, 0);
    }
}

#[test]
fn saves_round_trip_and_old_saves_load_at_the_original_size() {
    for &(n, _) in mapsize::SIZES.iter() {
        let l = new_land(n, 5, 9, 2, false, 31337);
        let _size = mapsize::scoped(n);
        let mut c = Course::default();
        c.sync(&l, false);
        let json = serde_json::to_string(&l).unwrap();
        // the original's size writes no size field, so its saves stay as they were
        assert_eq!(json.contains("\"n\":"), n != mapsize::ORIGINAL);
        let back: Land = serde_json::from_str(&json).unwrap();
        assert_eq!(back.n, n);
        assert_eq!((back.ty, back.height, back.flags), (l.ty.clone(), l.height.clone(), l.flags.clone()));
        let cj = serde_json::to_string(&c).unwrap();
        let cb: Course = serde_json::from_str(&cj).unwrap();
        assert_eq!((cb.ty, cb.walls, cb.height), (c.ty.clone(), c.walls.clone(), c.height.clone()));
    }
    // a save from before course sizes: no size field, 50 x 50
    let l = new_land(mapsize::ORIGINAL, 5, 9, 2, false, 31337);
    let mut v: serde_json::Value = serde_json::to_value(&l).unwrap();
    v.as_object_mut().unwrap().remove("n");
    let old: Land = serde_json::from_value(v).unwrap();
    assert_eq!(old.n, mapsize::ORIGINAL);
}

#[test]
fn the_original_size_is_unchanged() {
    let l = new_land(mapsize::ORIGINAL, 9, 5, 1, false, 42);
    assert_eq!((l.ty.len(), l.height.len()), (2500, 51 * 51));
    assert_eq!(sg_core::tracts::side(), 16);
    // the exe's distance bug is kept on the original's map (both legs long: 8 times too long) and fixed on larger ones
    assert_eq!(sg_core::staff::true_dist(0x8000, 0x8000), 370727);
    let _big = mapsize::scoped(98);
    assert_eq!(sg_core::staff::true_dist(0x8000, 0x8000), 46340);
}

/// An 18-hole course snaking over the middle of the map (each green a few tiles from the next tee), on sandbox land.
fn eighteen(n: i32) -> (Course, Club) {
    let mut l = new_land(n, 9, 0, 0, true, 1234);
    let _size = mapsize::scoped(n);
    let length = if n >= 98 {
        20
    } else if n >= 70 {
        16
    } else {
        11
    };
    let pitch = if n >= 98 {
        10
    } else if n >= 70 {
        8
    } else {
        6
    };
    let (cols, rows, gap) = (3, 6, 3);
    let a_start = (n - cols * (length + gap)) / 2;
    let b_start = (n - (rows - 1) * pitch) / 2;
    let mut club = Club::default();
    let paint = |l: &mut Land, a: i32, b: i32, ty: u8| {
        let i = idx(a, b);
        l.ty[i] = ty;
        l.flags[i] &= !(f::LOW | f::CUP | f::FOOTPRINT | f::OBSTACLE);
        l.var[i] = 0;
    };
    let mut h = 1;
    for r in 0..rows {
        let b = b_start + r * pitch;
        for k in 0..cols {
            let c = if r % 2 == 0 { k } else { cols - 1 - k };
            let lo = a_start + c * (length + gap);
            let (a0, a1) = if r % 2 == 0 { (lo, lo + length) } else { (lo + length, lo) };
            let step = (a1 - a0).signum();
            let mut a = a0;
            while a != a1 + step {
                for db in -1..=1 {
                    paint(&mut l, a, b + db, t::FAIRWAY);
                }
                a += step;
            }
            for da in -1..=1 {
                for db in -1..=1 {
                    paint(&mut l, a0 + da, b + db, t::TEE);
                    paint(&mut l, a1 + da, b + db, t::GREEN);
                }
            }
            l.flags[idx(a1, b)] |= f::CUP | h as u16;
            l.flags[idx(a0, b)] |= h as u16;
            let facing = if step > 0 { 2 } else { 6 };
            club.holes[h] = Hole { par: 4, tee_facing: facing, back: (a0, b), fwd: (a0, b), pin: (a1, b), ..club.holes[h].clone() };
            h += 1;
        }
    }
    let door = l.clubhouse;
    for k in [19usize] {
        club.holes[k].back = door;
        club.holes[k].fwd = door;
        club.holes[k].pin = door;
    }
    club.next_hole = 19;
    let mut c = Course::default();
    c.sync(&l, true);
    (c, club)
}

#[test]
#[ignore]
fn bench_ticks_and_walks() {
    for &(n, _) in mapsize::SIZES.iter() {
        let (mut c, mut club) = eighteen(n);
        let _size = mapsize::scoped(n);
        let mut rng = ExeRng { state: 4242 };
        club.difficulty = 0;
        club.new_game(&mut rng);
        let ticks = 40_000u32;
        let (mut most, mut sum_playing) = (0, 0usize);
        let t0 = std::time::Instant::now();
        let mut slowest = 0.0f64;
        for tick in 1..=ticks {
            for g in club.g.iter_mut().take(SLOTS) {
                g.sx = 400;
                g.sy = 300;
            }
            let s = std::time::Instant::now();
            club.tick(&mut c, &mut rng, tick);
            slowest = slowest.max(s.elapsed().as_secs_f64());
            club.out.clear();
            let playing = club.g.iter().take(SLOTS).filter(|g| g.hole > 0).count();
            most = most.max(playing);
            sum_playing += playing;
        }
        let per = t0.elapsed().as_secs_f64() * 1000.0 / ticks as f64;
        // the longest walk: corner to corner over the whole map, golfer rules and staff rules
        let mut walk_ms = 0.0;
        let reps = 50;
        let w0 = std::time::Instant::now();
        for k in 0..reps {
            let far = (n - 1) << 10;
            club.path_dir(&c, &mut rng, 0, (far, far), (512, 512 + (k % 4) * 1024), k % 2 == 1);
        }
        walk_ms += w0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
        println!(
            "size {n:3}: {per:.4} ms a tick (slowest {:.2} ms), {:.1} golfers on the course on average, {most} at most; a corner-to-corner path search {walk_ms:.3} ms",
            slowest * 1000.0,
            sum_playing as f64 / ticks as f64
        );
    }
}

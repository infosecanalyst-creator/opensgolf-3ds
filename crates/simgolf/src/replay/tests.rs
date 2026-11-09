use super::*;
use crate::app::PAINT;
use sg_core::terrain::Terrain;
use std::path::PathBuf;

/// A game on a flat fairway with hole 1's tee at 10,10 and its cup green at 10,30, planned but not opened (app.rs's
/// building_game), its log begun.
fn game() -> App {
    let mut a = App::with_clock(PathBuf::new(), 4242);
    let n = sg_core::mapsize::ORIGINAL;
    let tiles = (n * n) as usize;
    a.terrain = Terrain { w: n, h: n, ty: vec![2; tiles], variation: vec![0; tiles], set: vec![0; tiles], ..Default::default() };
    a.terrain.corner = vec![1; ((n + 1) * (n + 1)) as usize];
    a.terrain.path_kind = vec![0; tiles];
    a.terrain.wall_mask = vec![0; tiles];
    a.terrain.paint(10, 10, 0, 0);
    a.terrain.paint(10, 30, 1, 0);
    let h = &mut a.club.holes[1];
    h.back = (10, 10);
    h.fwd = (10, 10);
    h.pin = (10, 30);
    a.sync_course();
    a.game_tick = 1;
    a.econ.cash = 1e7;
    // as a game loaded from its save is (the stories, the weather's place and the purchases as a load makes them)
    let bytes = a.save_bytes(1).unwrap();
    let mut b = App::with_clock(PathBuf::new(), 4242);
    b.load_save_into(None, read_save_bytes(&bytes, "test").unwrap(), false).unwrap();
    b.screen = Screen::Play;
    b.log_restart();
    b
}

/// Plays `n` game ticks.
fn play(a: &mut App, n: u32) {
    let end = a.game_tick + n;
    while a.game_tick < end {
        a.step_game(sg_core::flight::TICK_MS / 1000.0 + 1e-4);
    }
}

/// A session played through the interface's own calls: brushes, strokes, a path, staff, an undo, the hole opened.
fn session(a: &mut App) {
    let fairway = PAINT.iter().position(|p| p.ty == 4).unwrap();
    a.pick_brush(fairway);
    a.undo_press();
    a.paint_tiles(&[(20, 20), (21, 20), (22, 20)]);
    a.undo_release();
    play(a, 40);
    a.hire_employee(0, false);
    a.brush = 1;
    a.edit_path(15, 15, false);
    play(a, 300);
    let sand = PAINT.iter().position(|p| p.ty == 7).unwrap();
    a.pick_brush(sand);
    a.undo_press();
    a.paint_tiles(&[(30, 30)]);
    a.undo_release();
    play(a, 20);
    assert!(a.undo_step_headless());
    a.open_hole();
    a.step_speed(true);
    play(a, 400);
}

impl App {
    /// Ctrl+Z without the graphics (a click's step only).
    fn undo_step_headless(&mut self) -> bool {
        self.act(Action::Undo)
    }

    fn total(&self) -> u64 {
        self.replay_hash()
    }
}

/// Plays a replay file in a fresh game up to tick `to`.
fn replayed(file: &[u8], to: u32) -> App {
    let mut b = App::with_clock(PathBuf::new(), 7);
    b.start_replay(None, file).unwrap();
    b.screen = Screen::Play;
    let n = to - b.game_tick;
    play(&mut b, n);
    // the entries logged after the last tick are applied before the next one
    b.replay_due();
    b
}

#[test]
fn a_replayed_session_reaches_the_same_game() {
    let mut a = game();
    session(&mut a);
    let log = a.rec.log.clone().unwrap();
    assert!(log.player_actions() >= 10, "the session's actions are logged");
    assert!(log.entries.iter().any(|e| matches!(e.action, Action::Check(_))), "a hash every {CHECK_EVERY} ticks");
    assert!(log.entries.iter().any(|e| e.action == Action::Undo));
    let file = a.replay_file().unwrap();
    let b = replayed(&file, a.game_tick);
    assert_eq!(b.game_tick, a.game_tick);
    assert_eq!(b.replay_diverged(), None);
    assert_eq!(b.total(), a.total(), "{:?}", changed(&checked_parts(&a), &checked_parts(&b)));
    assert!(b.rec.replay.is_none(), "the log is played through");
    assert_eq!(b.club.next_hole, a.club.next_hole);
    assert_eq!(b.terrain.ty, a.terrain.ty);
}

#[test]
fn a_changed_log_is_reported_where_it_diverges() {
    let mut a = game();
    session(&mut a);
    let file = a.replay_file().unwrap();
    let (save, mut log) = decode_replay(&file).unwrap();
    // the second stroke one tile further on: the replay must say it stopped matching at the first hash after it
    let at = log.entries.iter().position(|e| matches!(&e.action, Action::Paint { tiles, .. } if tiles == &[(30, 30)])).unwrap();
    let tick = log.entries[at].tick;
    if let Action::Paint { tiles, .. } = &mut log.entries[at].action {
        tiles[0] = (31, 30);
    }
    let first_check = log.entries[at..].iter().find(|e| matches!(e.action, Action::Check(_))).map(|e| e.tick).unwrap();
    let b = replayed(&encode_replay(&save, &log).unwrap(), a.game_tick);
    let d = b.replay_diverged().expect("the changed log diverges");
    assert!(d.tick >= tick && d.tick == first_check, "diverged at {} (changed at {tick}, next hash at {first_check})", d.tick);
}

#[test]
fn actions_built_without_the_interface_play_as_the_interfaces_own() {
    // through the interface's calls
    let mut a = game();
    let fairway = PAINT.iter().position(|p| p.ty == 2 && p.vbyte == 0).unwrap();
    a.pick_brush(fairway);
    a.paint_tiles(&[(25, 25), (26, 25)]);
    a.hire_employee(1, false);
    a.edit_raise(5, 5, 1);
    play(&mut a, 50);
    a.open_hole();
    play(&mut a, 50);
    // the same as plain data on a fresh copy of the game
    let mut b = game();
    b.apply(&Action::PickBrush { ty: 2, vbyte: 0 });
    let turn = b.brush_turn();
    assert!(turn.is_some(), "picking a brush rolls its turn");
    b.apply(&Action::Paint { ty: 2, vbyte: 0, turn, look: None, tiles: vec![(25, 25), (26, 25)], tee: 0 });
    b.apply(&Action::Hire { kind: 1, skilled: false });
    b.apply(&Action::Raise { x: 5, y: 5, brush: 0, delta: 1 });
    play(&mut b, 50);
    assert!(b.apply(&Action::OpenHole));
    play(&mut b, 50);
    assert_eq!(a.total(), b.total(), "{:?}", changed(&checked_parts(&a), &checked_parts(&b)));
}

#[test]
fn a_replay_refuses_the_players_actions_until_its_log_is_played() {
    let mut a = game();
    session(&mut a);
    let file = a.replay_file().unwrap();
    let mut b = App::with_clock(PathBuf::new(), 7);
    b.start_replay(None, &file).unwrap();
    let cash = b.econ.cash;
    assert!(!b.act(Action::Hire { kind: 0, skilled: false }));
    assert_eq!(b.econ.cash, cash);
    // the speed keys only say how fast it goes by
    b.step_speed(true);
    assert_eq!(b.rec.sim_speed, 1);
    b.screen = Screen::Play;
    let n = a.game_tick - b.game_tick;
    play(&mut b, n);
    b.replay_due();
    assert!(b.rec.replay.is_none());
    assert!(b.act(Action::Hire { kind: 0, skilled: false }), "after the log the game is the player's again");
}

/// A hole from the tee at x, y to the green three tiles on, through the interface's own calls, opened.
fn build_hole(a: &mut App, x: i32, y: i32) {
    let tee = PAINT.iter().position(|p| p.ty == 0).unwrap();
    let green = PAINT.iter().position(|p| p.ty == 1 && p.vbyte == 0).unwrap();
    a.pick_brush(tee);
    a.paint_tiles(&[(x, y)]);
    a.pick_brush(green);
    a.paint_tiles(&[(x, y + 3)]);
    assert!(a.open_hole(), "the hole at {x},{y} opens");
}

#[test]
fn a_session_on_course_b_with_a_forward_tee_replays() {
    // the Second course and Multiple tees extras: Course A's 18 holes, the A/B switch, B1, then B1's forward tee
    let mut a = game();
    let mut e = a.club.extras;
    e.second_course = true;
    e.tees = true;
    a.set_extras(e);
    assert!(a.open_hole());
    for k in 0..17 {
        build_hole(&mut a, 14 + 2 * k, 40);
    }
    play(&mut a, 60);
    assert!(a.club.course_a_full());
    a.toggle_hole_course();
    assert!(a.hole_b(), "the switch starts Course B");
    build_hole(&mut a, 40, 10);
    assert_eq!(a.club.course_b_open(), 1);
    play(&mut a, 40);
    // the Tees tool stepped to a forward tee (Tab), a tile nearer the cup
    let tee = PAINT.iter().position(|p| p.ty == 0).unwrap();
    a.pick_brush(tee);
    a.tee_tab();
    assert_eq!(a.tee_tool_step(), 1);
    a.paint_tiles(&[(41, 11)]);
    assert!(a.club.course_holes(true)[1].extra_tees().next().is_some(), "B1 has its forward tee");
    a.toggle_hole_course();
    assert!(!a.hole_b());
    play(&mut a, 300);
    let file = a.replay_file().unwrap();
    let b = replayed(&file, a.game_tick);
    assert_eq!(b.replay_diverged(), None);
    assert_eq!(b.total(), a.total(), "{:?}", changed(&checked_parts(&a), &checked_parts(&b)));
    assert!(b.club.course_holes(true)[1].extra_tees().next().is_some());
}

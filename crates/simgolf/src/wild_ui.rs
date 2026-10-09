//! Ambient life and water in the app: animals, the fly-overs, the ball splash, and the water effects the exe draws on water
//! tiles (shore ripples, rocks and coral, dolphins and fountains on scenic water). The rules are in sg_core::wildlife; the
//! tile rules for the water effects follow the exe's main frame (docs/PUBLISHER_EXE_NOTES.md, "Wildlife and water").

use crate::app::*;
use sg_core::course::{idx, inside};
use sg_core::terrain::TILE_SIZE;
use sg_core::wildlife::{self, KINDS};

const WATER_PALS: [&str; 4] = ["ParkWater Palette", "DesWater Palette", "TropWater Palette", "LinksWater Palette"];
const RIPPLES: [[&str; 3]; 4] = [
    ["rippleParkA", "rippleParkB", "rippleParkC"],
    ["rippleDesA", "rippleDesB", "rippleDesC"],
    ["waveTropA", "waveTropA", "waveTropA"],
    ["waveLinksA", "waveLinksA", "waveLinksA"],
];
/// Above-surface rocks A, B and the three underwater pieces (the exe's above-surface index 2 lands on underwater A).
const ROCKS: [[&str; 5]; 4] = [
    ["ASparkrockA", "ASparkrockB", "UWparkrockA", "UWparkrockB", "UWparkrockC"],
    ["ASdesrockA", "ASdesrockB", "UWdesrockA", "UWdesrockB", "UWdesrockC"],
    ["AStropsandA", "AStropsandB", "UWtropcoralA", "UWtropcoralB", "UWtropcoralC"],
    ["ASlinksrockA", "ASlinksrockB", "UWlinksrockA", "UWlinksrockB", "UWlinksrockC"],
];
const DOLPHINS: [&str; 4] = ["dolphin", "dolphinDesert", "dolphinTropical", "dolphinLinks"];
const SPRAYS: [&str; 4] = ["waterspray", "Desert_waterspray", "Trop_waterspray", "Links_waterspray"];

/// World units per exe screen pixel at full zoom (an exe tile is 64 pixels wide on screen and 100 world units here).
const PX: f32 = TILE_SIZE / 64.0;

impl App {
    fn animal_sprite(&mut self, kind: i32, clip: &str) -> (Option<usize>, Option<usize>) {
        // each clip carries its own colours; the kind palettes the exe also loads are not needed for them
        let _ = kind;
        let body = self.sprite_for(&format!("Animals/{clip}.flc"), false, None);
        let shadow = self.sprite_for(&format!("Animals/{clip}Shadow.flc"), true, None);
        (body, shadow)
    }

    fn frames_of(&self, s: Option<usize>) -> i32 {
        s.map(|s| self.sprites[s].s.frames_per_view).unwrap_or(1)
    }

    /// After the club's tick: tiles retyped by the player may draw wildlife; the animals and the fly-overs move.
    pub fn wildlife_tick(&mut self) {
        let theme = self.exe_theme();
        for (a, b) in std::mem::take(&mut self.retyped) {
            self.club.wildlife.retyped(&self.course, &mut self.exe_rng, a, b, theme);
        }
        let mut table = [[1i32; 4]; 9];
        for (k, row) in table.iter_mut().enumerate() {
            for (s, n) in row.iter_mut().enumerate() {
                let clip = KINDS[k].clips[s];
                let (body, _) = self.animal_sprite(k as i32, clip);
                *n = self.frames_of(body);
            }
        }
        let frames = move |kind: i32, state: i32| {
            let s = match state {
                wildlife::WALK => 0,
                wildlife::DOWN => 2,
                wildlife::LOOP => 3,
                _ => 1,
            };
            table[kind.clamp(0, 8) as usize][s]
        };
        self.club.wildlife_tick(&mut self.course, &mut self.exe_rng, &frames);
        let tick = self.game_tick;
        self.club.flyers_tick(&self.course, &mut self.exe_rng, tick);
        for (slot, (x, y)) in std::mem::take(&mut self.club.wildlife.sounds) {
            let (wx, wz) = self.units_to_world(x, y);
            self.slot_sound(slot, wx, wz);
        }
        let new: Vec<String> = self.club.wildlife.labels.iter().filter(|l| l.3 == 47).map(|l| l.2.clone()).collect();
        for t in new {
            println!("[{:6.1}s] {t}", self.sim_time);
            self.show_toast(&t);
        }
    }

    /// Rebuilds the ambient props: animals, fly-overs, the splash, and the water effects (their frames follow the tick).
    pub fn update_ambient_props(&mut self) {
        self.props.retain(|p| !p.ambient);
        if self.land.is_none() {
            return;
        }
        let theme = self.exe_theme().min(3) as usize;
        let tick = self.game_tick as i32;
        let mut add: Vec<Prop> = Vec::new();
        // animals
        let animals = self.club.wildlife.animals.clone();
        if self.club.wildlife.enabled {
            for a in animals.iter().filter(|a| a.kind >= 0) {
                let (body, shadow) = self.animal_sprite(a.kind, a.clip());
                let n = self.frames_of(body);
                let (x, z) = self.units_to_world(a.x, a.y);
                let lift = if a.t < 0 { -(wildlife::DROP[(a.t + 20).clamp(0, 19) as usize] * 4) as f32 * PX } else { 0.0 };
                const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
                const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
                let k = (a.heading & 7) as usize;
                add.push(Prop {
                    x,
                    z,
                    body,
                    shadow,
                    frame: a.shown_frame(n),
                    heading: DY[k].atan2(DX[k]).to_degrees(),
                    facing: (a.heading / 2) & 3,
                    scale: a.scale() as f32 / 4.0,
                    lift,
                    ..Default::default()
                });
            }
        }
        // fly-overs
        for f in self.club.wildlife.flyers {
            if !f.active || self.course.oob(f.x >> 10, f.y >> 10) {
                continue;
            }
            let (x, z) = self.units_to_world(f.x, f.y);
            let (file, shadow, lift, frame) = match f.kind {
                wildlife::BALLOON => ("Scenic/hotair01".to_string(), Some("Scenic/hotair01Shadow"), (f.h - 22) as f32 * PX, 0),
                wildlife::BLIMP => ("Scenic/blimpFXS".to_string(), Some("Scenic/blimpFXSShadow"), (f.h - 32) as f32 * PX, 0),
                wildlife::SAILBOAT => (format!("Scenic/{}", wildlife::SAILBOATS[theme]), None, 0.0, tick / 2),
                _ => {
                    let bird = wildlife::BIRDS[theme];
                    let flap = tick % 48 < 2 * 8;
                    (format!("Scenic/{bird}_{}", if flap { "Fidget" } else { "SQ" }), None, 0.0, tick)
                }
            };
            let body = self.sprite_for(&format!("{file}.flc"), false, None);
            let shadow = shadow.and_then(|s| self.sprite_for(&format!("{s}.flc"), true, None));
            let n = self.frames_of(body);
            const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
            const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
            let k = (f.heading & 7) as usize;
            add.push(Prop {
                x,
                z,
                body,
                shadow,
                frame: frame.rem_euclid(n.max(1)),
                heading: DY[k].atan2(DX[k]).to_degrees(),
                facing: (f.heading / 2) & 3,
                lift,
                ..Default::default()
            });
        }
        // the splash
        if let Some((sx, sy, n)) = self.club.wildlife.splash {
            let body = self.sprite_for("Scenic/Splash.flc", false, None);
            let (x, z) = self.units_to_world(sx, sy);
            add.push(Prop { x, z, body, frame: n, ..Default::default() });
        }
        // water effects
        let depth = &self.water_depth;
        if depth.len() == (sg_core::course::N * sg_core::course::N) as usize {
            let water = |a: i32, b: i32| inside(a, b) && self.course.ty[idx(a, b)] == sg_core::course::t::WATER;
            let wpal = format!("Water/{}.pcx", WATER_PALS[theme]);
            let mut items: Vec<(i32, i32, String, i32, i32, bool)> = Vec::new();
            for a in 0..sg_core::course::N {
                for b in 0..sg_core::course::N {
                    let i = idx(a, b);
                    if depth[i] == 255 {
                        continue;
                    }
                    // shore ripples on the +b shore, two tiles in three
                    if water(a, b - 1)
                        && water(a - 1, b)
                        && water(a + 1, b)
                        && !water(a, b + 1)
                        && depth.get(idx(a, b - 1)).is_some_and(|&d| d != 0)
                        && (a + b) % 3 != 0
                    {
                        let f = RIPPLES[theme][((a + 2 * b) % 3) as usize];
                        let facing = if theme >= 2 { 1 } else { 3 };
                        items.push((a, b, format!("Water/{f}"), tick + 7 * a, facing, true));
                    }
                    // rocks and coral in shallow, flat water
                    if water(a + 1, b) && water(a, b + 1) && water(a + 1, b + 1) && depth[i] <= 1 {
                        let mut id = -1;
                        if ((a - b) * 3) & 7 == 0 {
                            id = (a + b) % 3;
                        }
                        if (b * b + 3 * a) % 13 == 0 {
                            id = 2 + (a + b) % 3;
                        }
                        if id >= 0 {
                            let f = ROCKS[theme][id as usize];
                            items.push((a, b, format!("Water/{f}"), tick, (a - b + 1).rem_euclid(4), true));
                        }
                    }
                    // scenic water: dolphins in open water, else a fountain (a rock in the desert)
                    if self.course.flags[i] & sg_core::course::f::SCENIC != 0 && self.course.flags[i] & sg_core::course::f::PATH == 0 {
                        let ty = self.course.ty[i];
                        if ty == sg_core::course::t::WATER && depth[i] != 0 {
                            let c = (tick + 7 * a + 3 * b) & 63;
                            if c < 15 {
                                items.push((
                                    a,
                                    b,
                                    format!("Scenic/{}", DOLPHINS[theme]),
                                    c,
                                    (((tick + 7 * a + 3 * b) >> 6) + a + b) & 3,
                                    false,
                                ));
                            }
                        } else if ty == sg_core::course::t::WATER {
                            if theme == 1 {
                                items.push((a, b, "Scenic/Desert Water Rock".to_string(), 0, 0, false));
                            } else {
                                items.push((a, b, format!("Scenic/{}", SPRAYS[theme]), tick, 0, false));
                            }
                        }
                    }
                }
            }
            for (a, b, file, frame, facing, flat) in items {
                let body = self.sprite_for(&format!("{file}.flc"), false, if flat { Some(&wpal) } else { None });
                let n = self.frames_of(body);
                let (x, z) = self.terrain.tile_centre(a, b);
                add.push(Prop { x, z, body, frame: frame.rem_euclid(n.max(1)), facing, flat, ..Default::default() });
            }
        }
        for mut p in add {
            p.ambient = true;
            if p.body.is_some() {
                self.props.push(p);
            }
        }
    }
}

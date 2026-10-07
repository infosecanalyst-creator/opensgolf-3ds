//! Course analysis: pathways, the holes found from the painted terrain, and the course report's hole ratings.
//!
//! The manual says most buildings must be joined to the Clubhouse by an unbroken pathway, and the Sim Golf Association classes
//! each hole by which of the three skills it exercises (length, accuracy, imagination). The rating method follows the exe's
//! course statistics routine as far as it is decoded (docs/PUBLISHER_EXE_NOTES.md); hole detection is our own rule.
use crate::shot::{GolferSkills, ShotSim};
use crate::terrain::*;
use std::collections::VecDeque;

pub const CLASS_NAMES: [&str; 8] = ["Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"];

/// One flag per tile (w*h): 1 when a path tile is joined to the clubhouse lot by an unbroken chain of path tiles. A path tile
/// touching a Building tile (4-neighbourhood) is the start of a chain. Paths that are not joined appear as mud tracks (manual p. 18).
pub fn paths_connected_to_clubhouse(t: &Terrain) -> Vec<u8> {
    let mut ok = vec![0u8; (t.w * t.h).max(0) as usize];
    if t.path_kind.is_empty() {
        return ok;
    }
    const DX: [i32; 4] = [0, 1, 0, -1];
    const DY: [i32; 4] = [-1, 0, 1, 0];
    let mut q = VecDeque::new();
    for y in 0..t.h {
        for x in 0..t.w {
            if t.path_at(x, y) == 0 {
                continue;
            }
            if (0..4).any(|k| t.type_at(x + DX[k], y + DY[k]) == TT_BUILDING as i32) {
                ok[t.tile_index(x, y)] = 1;
                q.push_back(t.tile_index(x, y));
            }
        }
    }
    while let Some(i) = q.pop_front() {
        let (x, y) = (i as i32 % t.w, i as i32 / t.w);
        for k in 0..4 {
            let (nx, ny) = (x + DX[k], y + DY[k]);
            if t.path_at(nx, ny) == 0 {
                continue;
            }
            let ni = t.tile_index(nx, ny);
            if ok[ni] == 0 {
                ok[ni] = 1;
                q.push_back(ni);
            }
        }
    }
    ok
}

fn is_hazard(ty: i32) -> bool {
    matches!(ty, 17 | 23 | 24 | 25 | 18 | 9 | 7 | 27..=30 | 13 | 12 | 11 | 32 | 35)
}

#[derive(Clone, Debug, Default)]
pub struct HoleInfo {
    pub valid: bool,
    pub tee_x: f32,
    pub tee_z: f32,
    pub hole_x: f32,
    pub hole_z: f32,
    /// World units, straight line tee to hole.
    pub length: f32,
    /// PLACEHOLDER thresholds on length.
    pub par: i32,
    /// Distinct hazard stretches the straight line crosses.
    pub hazards_on_line: i32,
    /// Hazard tiles within two tiles of the line, not on it.
    pub hazards_near_line: i32,
    pub length_demand: bool,
    pub accuracy: bool,
    pub imagination: bool,
    pub cls: &'static str,
}

impl HoleInfo {
    /// One line, for the console.
    pub fn report(&self) -> String {
        if !self.valid {
            return "no hole".into();
        }
        let yn = |b: bool| if b { "yes" } else { "no" };
        format!(
            "hole: {:.0} units, par {}, {} (length {}, accuracy {}, imagination {}; {} hazards on the line, {} near it)",
            self.length,
            self.par,
            self.cls,
            yn(self.length_demand),
            yn(self.accuracy),
            yn(self.imagination),
            self.hazards_on_line,
            self.hazards_near_line
        )
    }
}

pub fn par_for_length(len: f32) -> i32 {
    if len < 1300.0 {
        3
    } else if len < 2600.0 {
        4
    } else {
        5
    }
}

/// Analyses the demo hole (the route stored in Terrain::path: first point the tee, last point the hole).
/// PLACEHOLDER rules: length demand when the line is longer than 1200 units; accuracy demand when 8 or more hazard tiles lie within
/// two tiles of the line; imagination demand when the line itself crosses a hazard.
pub fn analyze_hole(t: &Terrain) -> HoleInfo {
    let mut h = HoleInfo { cls: "Breather", ..Default::default() };
    if t.path.len() < 4 {
        return h;
    }
    h.valid = true;
    h.tee_x = t.path[0];
    h.tee_z = t.path[1];
    h.hole_x = t.path[t.path.len() - 2];
    h.hole_z = t.path[t.path.len() - 1];
    let (dx, dz) = (h.hole_x - h.tee_x, h.hole_z - h.tee_z);
    h.length = dx.hypot(dz);
    h.par = par_for_length(h.length);
    let mut in_hazard = false;
    let steps = 1.max((h.length / 25.0) as i32);
    let mut on_line = vec![0u8; (t.w * t.h) as usize];
    for i in 0..=steps {
        let x = h.tee_x + dx * i as f32 / steps as f32;
        let z = h.tee_z + dz * i as f32 / steps as f32;
        let ty = t.type_at_world(x, z);
        let hz = ty >= 0 && is_hazard(ty);
        if hz && !in_hazard {
            h.hazards_on_line += 1;
        }
        in_hazard = hz;
        if ty >= 0 {
            let (tx, tz) = t.tile_of(x, z);
            on_line[t.tile_index(tx, tz)] = 1;
        }
    }
    for y in 0..t.h {
        for x in 0..t.w {
            if !is_hazard(t.ty[t.tile_index(x, y)] as i32) {
                continue;
            }
            let cx = -t.w as f32 * TILE_SIZE * 0.5 + (x as f32 + 0.5) * TILE_SIZE;
            let cz = -t.h as f32 * TILE_SIZE * 0.5 + (y as f32 + 0.5) * TILE_SIZE;
            // distance from the tile centre to the segment tee-hole
            let t0 = (((cx - h.tee_x) * dx + (cz - h.tee_z) * dz) / (h.length * h.length)).clamp(0.0, 1.0);
            let d = (cx - (h.tee_x + dx * t0)).hypot(cz - (h.tee_z + dz * t0));
            if d < 200.0 && on_line[t.tile_index(x, y)] == 0 {
                h.hazards_near_line += 1;
            }
        }
    }
    h.length_demand = h.length > 1200.0;
    h.accuracy = h.hazards_near_line >= 8;
    h.imagination = h.hazards_on_line >= 1;
    h.cls = CLASS_NAMES[h.length_demand as usize | (h.accuracy as usize) << 1 | (h.imagination as usize) << 2];
    h
}

#[derive(Clone, Debug, PartialEq)]
pub struct HoleRoute {
    pub tee_x: f32,
    pub tee_z: f32,
    pub green_x: f32,
    pub green_z: f32,
    pub length: f32,
    /// PLACEHOLDER thresholds on length (3 under 1300 units, 4 under 2600, else 5).
    pub par: i32,
    /// x,z pairs from tee to green.
    pub route: Vec<f32>,
}

/// The course's holes, found from the painted terrain: each cluster of Tee tiles is paired with the nearest unused cluster of
/// Putting Green tiles, in reading order of the tees (top row first). The route is a straight line, except that when the course has
/// a stored route (Terrain::path, the generated demo hole) that begins and ends at the same tee and green, that curved route is used.
/// PLACEHOLDER rule: the original numbers holes with a tool, this one reads the terrain.
pub fn find_holes(t: &Terrain) -> Vec<HoleRoute> {
    let mut out = Vec::new();
    let (w, h) = (t.w, t.h);
    if w <= 0 || h <= 0 {
        return out;
    }
    struct Blob {
        x: f32,
        z: f32,
        used: bool,
    }
    let centre_of = |tx: f32, ty: f32| {
        (tx * TILE_SIZE - w as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5, ty * TILE_SIZE - h as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5)
    };
    let blobs = |ty: u8| {
        let mut res = Vec::new();
        let mut seen = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = t.tile_index(x, y);
                if seen[i] || t.ty[i] != ty {
                    continue;
                }
                let mut stack = vec![i];
                seen[i] = true;
                let (mut sx, mut sy, mut n) = (0f64, 0f64, 0i32);
                while let Some(c) = stack.pop() {
                    let (cx, cy) = (c as i32 % w, c as i32 / w);
                    sx += cx as f64;
                    sy += cy as f64;
                    n += 1;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (nx, ny) = (cx + dx, cy + dy);
                            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                                continue;
                            }
                            let j = t.tile_index(nx, ny);
                            if !seen[j] && t.ty[j] == ty {
                                seen[j] = true;
                                stack.push(j);
                            }
                        }
                    }
                }
                if n < 2 {
                    continue; // ignore a single stray tile
                }
                let (bx, bz) = centre_of((sx / n as f64) as f32, (sy / n as f64) as f32);
                res.push(Blob { x: bx, z: bz, used: false });
            }
        }
        res
    };
    let tees = blobs(TT_TEE);
    let mut greens = blobs(TT_PUTTING_GREEN);
    for tb in &tees {
        let (mut best, mut bd) = (None, 1e30f32);
        for (g, gb) in greens.iter().enumerate() {
            if gb.used {
                continue;
            }
            let d = (gb.x - tb.x).hypot(gb.z - tb.z);
            if d < bd {
                bd = d;
                best = Some(g);
            }
        }
        let Some(best) = best else { continue };
        if bd < 250.0 {
            continue; // a tee right on top of its green
        }
        greens[best].used = true;
        let (gx, gz) = (greens[best].x, greens[best].z);
        let p = &t.path;
        let route = if p.len() >= 4 && (p[0] - tb.x).hypot(p[1] - tb.z) < 300.0 && (p[p.len() - 2] - gx).hypot(p[p.len() - 1] - gz) < 300.0
        {
            p.clone()
        } else {
            vec![tb.x, tb.z, gx, gz]
        };
        out.push(HoleRoute { tee_x: tb.x, tee_z: tb.z, green_x: gx, green_z: gz, length: bd, par: par_for_length(bd), route });
    }
    if out.is_empty() && t.path.len() >= 4 {
        // an old course with a stored route but no recognisable tee and green
        let p = &t.path;
        let (tx, tz, gx, gz) = (p[0], p[1], p[p.len() - 2], p[p.len() - 1]);
        let len = (gx - tx).hypot(gz - tz);
        out.push(HoleRoute { tee_x: tx, tee_z: tz, green_x: gx, green_z: gz, length: len, par: par_for_length(len), route: p.clone() });
    }
    out
}

/// Course report ratings. The original's Shot Analysis compares golfers with every skill against golfers missing one skill; this
/// does the same with the shot model: `samples` simulated rounds of the hole per variant, and the score is how many strokes worse the
/// golfer lacking the skill averages. Draw, fade and backspin are not modelled by the shot model, so Imagination is always 0.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleRating {
    pub len: f32,
    pub acc: f32,
    pub img: f32,
    /// For the full skill golfer.
    pub avg_strokes: f32,
    /// World units carried by the first shot.
    pub avg_drive: f32,
    pub ty: &'static str,
    /// L + 2A + 4I, the index into the class names.
    pub type_index: usize,
}

impl Default for HoleRating {
    fn default() -> Self {
        HoleRating { len: 0.0, acc: 0.0, img: 0.0, avg_strokes: 0.0, avg_drive: 0.0, ty: "Breather", type_index: 0 }
    }
}

pub fn rate_hole(t: &Terrain, r: &HoleRoute, samples: i32, difficulty: i32) -> HoleRating {
    let mut out = HoleRating::default();
    if r.route.len() < 4 || samples < 1 {
        return out;
    }
    let avg = |s: &GolferSkills, drive: Option<&mut f32>| {
        let (mut sum, mut drv) = (0f64, 0f64);
        for i in 0..samples {
            let mut sim = ShotSim::default();
            sim.skills = *s;
            sim.looping = false;
            sim.set_route(r.route.clone());
            sim.init(t, 1234u32.wrapping_add((i as u32).wrapping_mul(977)));
            let (mut guard, mut first_stroke) = (0, false);
            while !sim.finished && guard < 30 * 400 {
                guard += 1;
                sim.step(t, 1.0 / 30.0);
                if !first_stroke && sim.stroke >= 1 && sim.event != "drive" && sim.ball_h <= 0.0 {
                    first_stroke = true;
                    drv += (sim.ball_x - r.route[0]).hypot(sim.ball_z - r.route[1]) as f64;
                }
            }
            sum += sim.stroke.min(12) as f64;
        }
        if let Some(d) = drive {
            *d = (drv / samples as f64) as f32;
        }
        (sum / samples as f64) as f32
    };
    let full = GolferSkills { v: [15; 10] };
    let mut drive = 0.0;
    out.avg_strokes = avg(&full, Some(&mut drive));
    out.avg_drive = drive;
    let mut no_len = full;
    no_len.v[GolferSkills::POWER] = 0;
    no_len.v[GolferSkills::LONG_DRIVER] = 0;
    let mut no_acc = full;
    no_acc.v[GolferSkills::ACC_DRIVER] = 0;
    no_acc.v[GolferSkills::ACC_IRONS] = 0;
    out.len = (avg(&no_len, None) - out.avg_strokes).max(0.0);
    out.acc = (avg(&no_acc, None) - out.avg_strokes).max(0.0);
    out.img = 0.0;
    // The exe flags a skill as demanded when full-skill golfers beat golfers lacking it by 50 hundredths of a stroke or more (25 on
    // the easiest difficulty).
    let thr = if difficulty == 0 { 0.25 } else { 0.50 };
    out.type_index = (out.len >= thr) as usize | ((out.acc >= thr) as usize) << 1 | ((out.img >= thr) as usize) << 2;
    out.ty = CLASS_NAMES[out.type_index];
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_course_has_one_hole_with_curved_route() {
        let t = Terrain::demo_course(40, 40, 7);
        let holes = find_holes(&t);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0].route, t.path);
    }

    #[test]
    fn paths_join_through_building() {
        let mut t = Terrain::demo_course(40, 40, 7);
        let (cx, cy) = (t.clubhouse_x, t.clubhouse_y);
        let a = t.tile_index(cx + 3, cy);
        let b = t.tile_index(cx + 4, cy);
        let far = t.tile_index(1, 1);
        t.path_kind[a] = 1;
        t.path_kind[b] = 1;
        t.path_kind[far] = 1;
        let ok = paths_connected_to_clubhouse(&t);
        assert_eq!((ok[a], ok[b], ok[far]), (1, 1, 0));
    }

    #[test]
    fn rating_is_deterministic() {
        let t = Terrain::demo_course(40, 40, 7);
        let h = &find_holes(&t)[0];
        assert_eq!(rate_hole(&t, h, 4, 1), rate_hole(&t, h, 4, 1));
    }
}

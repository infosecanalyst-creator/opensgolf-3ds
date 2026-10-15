//! Undo for building and terrain (the port's own, docs/PLAYING.md "Undo"). Before a click on the course the whole game is
//! kept in memory as a save; when the button comes up and the click changed the course (its land, terrain or holes), that
//! save goes on the undo list. Ctrl+Z (Cmd+Z on a Mac) goes back to the last one: the course, the money and everything else
//! as they were just before that click, as a saved game would load. The list is not saved and starts empty with each game.

use crate::app::App;
use crate::gfx::Gfx;
use std::hash::{Hash, Hasher};

/// The most clicks the list keeps (the oldest go first).
pub const MAX_STEPS: usize = 30;

#[derive(Default)]
pub struct Undo {
    /// Saves from before each click that changed the course, oldest first.
    steps: Vec<Vec<u8>>,
    /// The save and course fingerprint taken at the button's press, until it comes up.
    pending: Option<(Vec<u8>, u64)>,
}

impl Undo {
    pub fn clear(&mut self) {
        self.steps.clear();
        self.pending = None;
    }
}

impl App {
    /// A fingerprint of the course: what building and terrain tools change (not what the simulation changes as time goes
    /// by: growth, weeds, the undo records).
    fn course_fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.terrain.ty.hash(&mut h);
        self.terrain.corner.hash(&mut h);
        self.terrain.path_kind.hash(&mut h);
        if let Some(l) = &self.land {
            l.ty.hash(&mut h);
            l.height.hash(&mut h);
            if let Ok(v) = serde_json::to_vec(&l.objects) {
                v.hash(&mut h);
            }
        }
        h.finish()
    }

    /// A press on the course: keeps the game as it is now, in case the click changes the course.
    pub fn undo_press(&mut self) {
        if self.land.is_none() {
            return;
        }
        if let Ok(b) = self.save_bytes(1) {
            self.undo.pending = Some((b, self.course_fingerprint()));
        }
    }

    /// The button came up: the kept game goes on the undo list when the course changed since the press.
    pub fn undo_release(&mut self) {
        let Some((b, f)) = self.undo.pending.take() else { return };
        if self.land.is_some() && self.course_fingerprint() != f {
            self.undo.steps.push(b);
            if self.undo.steps.len() > MAX_STEPS {
                self.undo.steps.remove(0);
            }
        }
    }

    /// Ctrl+Z: back to the game as it was before the last click that changed the course. Returns false with nothing to undo.
    pub fn undo_step(&mut self, g: &mut Gfx) -> bool {
        let Some(b) = self.undo.steps.pop() else { return false };
        let Ok(s) = crate::app::read_save_bytes(&b, "undo") else { return false };
        // what the player was doing stays as it was: the open panel, its tool and the view
        let steps = std::mem::take(&mut self.undo.steps);
        let (panel, alt, edit, tool) = (self.panel, self.pstate.alt, self.edit, self.tool);
        if self.load_save_with(g, s, true).is_err() {
            self.undo.steps = steps;
            return false;
        }
        self.undo.steps = steps;
        self.panel = panel;
        self.pstate.alt = alt;
        self.edit = edit;
        self.tool = tool;
        true
    }
}

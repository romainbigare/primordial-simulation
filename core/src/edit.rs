//! Edits of a world (handover.md 11: an action is a physics edit, made at a step boundary): the one way a run's plan
//! and the live page change a world from the outside. Each edit acts on the cell under its place, through the law it
//! belongs to (`World::clear`, `World::split`, `cover::place`, the flames' strikes), and is told to the shell as a raw
//! event (`EventKind::Edit`).

use crate::event::{Event, EventKind};
use crate::World;
use serde::{Deserialize, Serialize};

/// What an edit does to the cell under its place.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Act {
    /// The share of the cell made a new patch whose plants die above the ground (`World::clear`), about the cell's
    /// sample under the place (`World::sample_in`)
    Clear(f64),
    /// The share of the cell's first patch made a new patch alike to it, a control that changes nothing (`World::split`)
    Split(f64),
    /// `kg` per m² of the cover of genes `genes` and starting kind `root` placed in the cell's newest patch, the one of
    /// the least age (`cover::place`)
    Plant { genes: [f32; crate::cover::NG], root: u32, kg: f64 },
    /// One strike that lights the cell in the coming month (`World::strikes`, `flame::shares`), where its remains
    /// carry a flame (`flame::lights`)
    Strike,
}

/// An edit: what it does (`act`) to the cell under its place, in squares from the map's north-west corner (x east, y
/// south: 3.5, 2.5 is the middle of square 3, 2; `World::cell_under`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub place: [f64; 2],
    pub act: Act,
}

impl Edit {
    /// What the edit did to cell `i`, in a line; `done` false: the world could not make it.
    pub fn line(&self, i: usize, done: bool) -> String {
        let what = match self.act {
            Act::Clear(s) => format!("cleared {s} of the cover of cell {i} as a new patch"),
            Act::Split(s) => format!("split {s} of the cover of cell {i} off as a new patch alike to the rest"),
            Act::Plant { root, kg, .. } => format!("placed {kg} kg per m2 of starting kind {root} in the newest patch of cell {i}"),
            Act::Strike => format!("a strike lights cell {i} in the coming month"),
        };
        format!("{what}{}", if done { "" } else { ": nothing done" })
    }
}

impl World {
    /// Make the edits `edits` in their order, after the passes of a month's start (`month_start`), so each reads the
    /// world as the laws read it at a month's start; each is told as an event (`EventKind::Edit`, its cell the node,
    /// the value 1 where it was made, 0 where the world could not make it). Returns whether each was made.
    pub fn edit(&mut self, edits: &[Edit]) -> Vec<bool> {
        if edits.is_empty() {
            return Vec::new();
        }
        crate::month_start(self);
        edits
            .iter()
            .map(|e| {
                let i = self.cell_under(e.place);
                let done = self.make(i, e);
                self.events.push(Event { day: self.day(), kind: EventKind::Edit(*e), node: i as u32, value: if done { 1.0 } else { 0.0 } });
                done
            })
            .collect()
    }

    /// Make the edit `e` on cell `i`; whether it was made.
    fn make(&mut self, i: usize, e: &Edit) -> bool {
        match e.act {
            Act::Clear(s) => {
                let at = self.sample_in(i, e.place);
                self.clear(i, s, Some(at))
            }
            Act::Split(s) => self.split(i, s),
            Act::Plant { genes, root, kg } => {
                let c = self.life.cover(i);
                let newest = (0..c.patch_count()).rev().min_by(|&a, &b| c.patches[a].age.total_cmp(&c.patches[b].age)).unwrap_or(0);
                crate::cover::place(self, i, newest, genes, root, kg)
            }
            Act::Strike => {
                let lit = crate::flame::lights(self, i).is_ok();
                if lit {
                    self.strikes.push(i as u32);
                }
                lit
            }
        }
    }
}

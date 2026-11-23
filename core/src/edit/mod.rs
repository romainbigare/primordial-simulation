//! Edits of a world (handover.md 11: an action is a physics edit, made at a step boundary): the one way a run's plan
//! and the live page change a world from the outside. Each edit acts on the cell under its place, through the law it
//! belongs to (`World::clear`, `World::split`, `cover::place`, the flames' strikes), and is told to the shell as a raw
//! event (`EventKind::Edit`). A round brush finds the cells under it and the share of each it covers (`brush`), and
//! raises or lowers the ground under it (`shape`); what the edits start that takes no time runs at once, with no time
//! on the world's clock (`World::settle`).

mod brush;
mod shape;

pub use brush::{Brush, Under};

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
    /// The ground under a brush of radius `radius_m` m about the place raised by `height_m` m at its centre (lowered
    /// where under none), less toward its edge; a stroke that has raised `stroke_m` m there before lays the loose ground
    /// of its top (`Config::edit_top_m`) of the terrain's make `make` (`World::shape`)
    Ground { radius_m: f64, height_m: f64, stroke_m: f64, make: u8 },
}

/// An edit: what it does (`act`) to the cell under its place, in squares from the map's north-west corner (x east, y
/// south: 3.5, 2.5 is the middle of square 3, 2; `World::cell_under`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub place: [f64; 2],
    pub act: Act,
}

/// What settling the edits did (`World::settle`): whether the samples were built again over the ground edits shaped
/// (`ground`), and each cell the flames of the strikes of edits reached and when, s after its strike (`burnt`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Settled {
    pub ground: bool,
    pub burnt: Vec<(u32, f64)>,
}

impl Edit {
    /// What the edit did to cell `i`, in a line; `done` false: the world could not make it.
    pub fn line(&self, i: usize, done: bool) -> String {
        let what = match self.act {
            Act::Clear(s) => format!("cleared {s} of the cover of cell {i} as a new patch"),
            Act::Split(s) => format!("split {s} of the cover of cell {i} off as a new patch alike to the rest"),
            Act::Plant { root, kg, .. } => format!("placed {kg} kg per m2 of starting kind {root} in the newest patch of cell {i}"),
            Act::Strike => format!("a strike lights cell {i} in the coming month"),
            Act::Ground { radius_m, height_m, .. } => format!("the ground within {radius_m} m of a place in cell {i} raised by {height_m} m at its middle"),
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

    /// Run at once what the edits since the month began start that takes no time, with no time on the world's clock
    /// (laws.md 0): where edits shaped the ground (`Relief::due`), the samples are built again over it
    /// (`flow::build::reshape`), the passes of a month's start find the state of the ground from them, and the liquid
    /// settles over it (`flow::settle`); then the strikes of edits burn (`flame::now`). Returns what it did (`Settled`).
    pub fn settle(&mut self) -> Settled {
        let ground = self.relief.due;
        if ground {
            crate::flow::build::reshape(self);
            crate::month_start(self);
            crate::flow::settle(self);
        }
        let burnt = if self.strikes.is_empty() { Vec::new() } else { crate::flame::now(self) };
        Settled { ground, burnt }
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
            Act::Ground { radius_m, height_m, stroke_m, make } => self.shape(Brush { centre: e.place, radius_m }, (height_m, stroke_m), make as usize),
        }
    }
}

//! Edits of a world (handover.md 11: an action is a physics edit, made at a step boundary): the one way a run's plan
//! and the live page change a world from the outside. Each edit acts on the cell under its place, through the law it
//! belongs to (`World::clear`, `World::split`, `cover::place`, the flames' strikes), and is told to the shell as a raw
//! event (`EventKind::Edit`). A round brush finds the cells under it and the share of each it covers (`brush`), and
//! raises or lowers the ground under it (`shape`); what the edits start that takes no time runs at once, with no time
//! on the world's clock (`World::settle`).

mod brush;
mod heat;
mod basin;
mod shape;

pub use brush::{falloff, Brush, Under};

use crate::event::{Event, EventKind};
use crate::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What an edit does to the cell under its place.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Act {
    /// The share of the cell (above 0, at most 1) made a new patch whose plants die above the ground (`World::clear`),
    /// about the cell's sample under the place (`World::sample_in`)
    Clear(f64),
    /// The share of the cell's first patch made a new patch alike to it, a control that changes nothing (`World::split`)
    Split(f64),
    /// `kg` per m² of the cover of genes `genes` and starting kind `root` placed in the cell's newest patch, the one of
    /// the least age (`cover::place`)
    Plant { genes: [f32; crate::cover::NG], root: u32, kg: f64 },
    /// The grown stand of starting kind `root` the world holds (`cover::grown`) placed in the cell's newest patch, its
    /// amounts and plants per m² (`cover::place_grown`); where none stands, as `Plant`
    Grown { genes: [f32; crate::cover::NG], root: u32, kg: f64 },
    /// One strike that lights the cell in the coming month (`World::strikes`, `flame::shares`), where its remains
    /// carry a flame (`flame::lights`)
    Strike,
    /// `joules_m2` J a m² of heat added to the remains under a brush of radius `radius_m` m about the place at its
    /// centre, less toward its edge (`World::heat`): it dries and lights them when the edits settle (`flame::heated`)
    Heat { radius_m: f64, joules_m2: f64 },
    /// A basin of liquid `depth_m` m deep under a brush of radius `radius_m` m about the place: its ground cut to a
    /// floor that deep under its rim's lowest dry sample, or under the level of a pond it reaches, and the key liquid
    /// poured in to its pass (`World::basin`)
    Basin { radius_m: f64, depth_m: f64 },
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

impl Act {
    /// Whether it reads the world as the laws read it at a month's start (what stands on each cell, Flow's state, the
    /// wet ground): the edits of the cover and the strikes do; those that put matter or heat in the ground from outside
    /// read the store alone (a basin builds what it reads itself).
    pub fn reads_month(&self) -> bool {
        !matches!(self, Act::Ground { .. } | Act::Heat { .. } | Act::Basin { .. })
    }
}

impl Edit {
    /// What the edit did to cell `i`, in a line; `done` false: the world could not make it.
    pub fn line(&self, i: usize, done: bool) -> String {
        let what = match self.act {
            Act::Clear(s) => format!("cleared {s} of the cover of cell {i} as a new patch"),
            Act::Split(s) => format!("split {s} of the cover of cell {i} off as a new patch alike to the rest"),
            Act::Plant { root, kg, .. } => format!("placed {kg} kg per m2 of starting kind {root} in the newest patch of cell {i}"),
            Act::Grown { root, .. } => format!("placed a grown stand of starting kind {root} in the newest patch of cell {i}"),
            Act::Strike => format!("a strike lights cell {i} in the coming month"),
            Act::Basin { radius_m, depth_m } => format!("a basin of liquid {depth_m} m deep within {radius_m} m of a place in cell {i}"),
            Act::Heat { radius_m, joules_m2 } => format!("{joules_m2} J per m2 of heat added within {radius_m} m of a place in cell {i}"),
            Act::Ground { radius_m, height_m, .. } => format!("the ground within {radius_m} m of a place in cell {i} raised by {height_m} m at its middle"),
        };
        format!("{what}{}", if done { "" } else { ": nothing done" })
    }
}

impl World {
    /// Make the edits `edits` in their order, after the passes of a month's start (`month_start`) where one of them
    /// reads the world as the laws read it at a month's start (`Act::reads_month`); each is told as an event
    /// (`EventKind::Edit`, its cell the node, the value 1 where it was made, 0 where the world could not make it).
    /// The grown stand of a kind is found once for all of them, as the world stood before the first placed it: each
    /// cell takes the same stand. Returns whether each was made.
    pub fn edit(&mut self, edits: &[Edit]) -> Vec<bool> {
        if edits.iter().any(|e| e.act.reads_month()) {
            crate::month_start(self);
        }
        let mut grown = BTreeMap::new();
        edits
            .iter()
            .map(|e| {
                let i = self.cell_under(e.place);
                let done = self.make(i, e, &mut grown);
                self.events.push(Event { day: self.day(), kind: EventKind::Edit(*e), node: i as u32, value: if done { 1.0 } else { 0.0 } });
                done
            })
            .collect()
    }

    /// Run at once what the edits since the month began start that takes no time, with no time on the world's clock
    /// (laws.md 0): where edits shaped the ground (`Relief::due`), the samples are built again over it
    /// (`flow::build::reshape`), the passes of a month's start find the state of the ground from them, and the liquid
    /// settles over it (`flow::settle`); then the strikes of edits and the heat they added burn (`flame::now`). Returns
/// what it did (`Settled`).
    pub fn settle(&mut self) -> Settled {
        let ground = self.relief.due;
        if ground {
            crate::flow::build::reshape(self);
            crate::month_start(self);
            crate::flow::settle(self);
        }
        let burnt = if self.strikes.is_empty() && self.heated.is_empty() { Vec::new() } else { crate::flame::now(self) };
        Settled { ground, burnt }
    }

    /// The newest patch of cell `i`'s cover, the one of the least age (the last of those).
    fn newest(&self, i: usize) -> usize {
        let c = self.life.cover(i);
        (0..c.patch_count()).rev().min_by(|&a, &b| c.patches[a].age.total_cmp(&c.patches[b].age)).unwrap_or(0)
    }

    /// Make the edit `e` on cell `i`, the grown stands of the kinds found so far in `grown`; whether it was made.
    fn make(&mut self, i: usize, e: &Edit, grown: &mut BTreeMap<u32, Option<crate::cover::Grown>>) -> bool {
        match e.act {
            Act::Clear(s) => {
                let at = self.sample_in(i, e.place);
                self.clear(i, s, Some(at))
            }
            Act::Split(s) => self.split(i, s),
            Act::Plant { genes, root, kg } => crate::cover::place(self, i, self.newest(i), genes, root, kg),
            Act::Grown { genes, root, kg } => match grown.entry(root).or_insert_with(|| crate::cover::grown(self, root)).clone() {
                Some(g) => crate::cover::place_grown(self, i, self.newest(i), &g),
                None => crate::cover::place(self, i, self.newest(i), genes, root, kg),
            },
            Act::Strike => {
                let lit = crate::flame::lights(self, i).is_ok();
                if lit {
                    self.strikes.push(i as u32);
                }
                lit
            }
            Act::Basin { radius_m, depth_m } => self.basin(Brush { centre: e.place, radius_m }, depth_m),
            Act::Heat { radius_m, joules_m2 } => self.heat(Brush { centre: e.place, radius_m }, joules_m2),
            Act::Ground { radius_m, height_m, stroke_m, make } => self.shape(Brush { centre: e.place, radius_m }, (height_m, stroke_m), make as usize),
        }
    }
}

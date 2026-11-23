//! Edits of a world (handover.md 11: an action is a physics edit, made at a step boundary): the one way a run's plan
//! and the live page change a world from the outside. Each edit acts on the cell under its place, through the law it
//! belongs to (`World::clear`, `World::split`, `cover::place`, the flames' strikes), and is told to the shell as a raw
//! event (`EventKind::Edit`). A round brush finds the cells under it and the share of each it covers (`World::under`);
//! what the edits start that takes no time runs at once, with no time on the world's clock (`World::settle`).

use crate::event::{Event, EventKind};
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// A round brush: its centre, in squares from the map's north-west corner (as `Edit::place`), and its radius, m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brush {
    pub centre: [f64; 2],
    pub radius_m: f64,
}

/// A cell under a brush (`World::under`): the cell, the share of it the brush covers, and the place in it nearest the
/// brush's centre, in squares (the centre of its sample the brush covers nearest that centre).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Under {
    pub cell: usize,
    pub share: f64,
    pub place: [f64; 2],
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

    /// The cells under the brush `b`, in the order of the cells (`Under`). The brush covers the terrain's samples
    /// (`SAMPLE_DEPTH`, the finest the ground is held at) whose centres lie within its radius, measured in samples from
    /// north to south (`Square::length`), and the sample under its centre always (a brush finer than a sample covers
    /// it); a cell's share is the share of its samples it covers.
    pub fn under(&self, b: Brush) -> Vec<Under> {
        let map = &self.map;
        let per = (1u64 << SAMPLE_DEPTH.saturating_sub(map.square_depth())) as f64;
        let r = b.radius_m / map.length(SAMPLE_DEPTH);
        let [cx, cy] = b.centre.map(|v| v * per);
        let span = |c: f64, n: usize| {
            let most = n * per as usize;
            (((c - r).floor().max(0.0) as usize).min(most - 1), ((c + r).ceil().max(0.0) as usize).min(most))
        };
        let ((x0, x1), (y0, y1)) = (span(cx, map.w), span(cy, map.h));
        let (mid_x, mid_y) = (cx.floor(), cy.floor());
        // Of each cell: the samples covered, and the place of the one nearest the centre with its distance.
        let mut cells: BTreeMap<usize, (f64, f64, [f64; 2])> = BTreeMap::new();
        for sy in y0..y1.max(y0 + 1) {
            for sx in x0..x1.max(x0 + 1) {
                let (x, y) = (sx as f64 + 0.5, sy as f64 + 0.5);
                let d = (x - cx).hypot(y - cy);
                if d > r && !(sx as f64 == mid_x && sy as f64 == mid_y) {
                    continue;
                }
                let place = [x / per, y / per];
                let at = cells.entry(self.cell_under(place)).or_insert((0.0, f64::INFINITY, place));
                at.0 += 1.0;
                if d < at.1 {
                    (at.1, at.2) = (d, place);
                }
            }
        }
        cells.into_iter().map(|(cell, (count, _, place))| Under { cell, share: (count / (map.squares(cell) * per * per)).min(1.0), place }).collect()
    }

    /// Run at once what the edits since the month began start that takes no time, with no time on the world's clock
    /// (laws.md 0): the strikes of edits burn now (`flame::now`). Returns each cell their flames reached and when, s
    /// after its strike; none where no edit struck.
    pub fn settle(&mut self) -> Vec<(u32, f64)> {
        if self.strikes.is_empty() {
            return Vec::new();
        }
        crate::flame::now(self)
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

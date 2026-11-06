//! The interest of a cell (laws.md 0): a sum of four terms, each about 1 in a cell worth refining. The camera's, where
//! its window asks for finer cells (what is looked at); brains, the members of bodies that decide by a brain of hidden
//! units in the cell (where choices are made); its parts and bonds (what bodies made, which a join would lump); and
//! change, the share by which the cell's animal mass moved in the month (an arrival, a die-off). The ground's
//! temperature is no term: it moves by the season, faster on land than under liquid, so it marks the coasts each
//! season, not an event. These are settings for speed: no law reads them.
//!
//! Refinement settles: for the same things in a cell, no quarter scores more than the cell. Brains and parts are
//! counts against a unit, so a split shares them out and the quarters' terms sum to the cell's (a density would grow
//! four times at each split, and a cell of one part would never join); change is a share of a mass of at least
//! `MASS_FLOOR`, the same in quarters that changed alike and less in a quarter holding little; the camera's halves at
//! each depth. So refinement goes down until a cell holds about one unit and joins where the cell its quarters make
//! would hold much less (`joined`).

use super::camera::Shot;
use crate::remap::{sum4, Made};
use crate::space::tree::DEPTH_MOST;
use crate::World;
use serde::{Deserialize, Serialize};

/// A cell of this score or more splits: the terms are scaled so a cell worth refining scores about this.
pub(super) const SPLIT_AT: f64 = 1.0;
/// Four quarters join when the cell they make would score under this: a quarter of `SPLIT_AT`, so a cell does not flap.
pub(super) const JOIN_AT: f64 = 0.25;
/// Members of bodies with a brain of hidden units in a cell at which the brains term is 1.
const BRAINS_UNIT: f64 = 4e4;
/// Parts and bonds in a cell at which the parts term is 1.
pub const PARTS_UNIT: f64 = 40.0;
/// The share by which a cell's animal mass moved in a month at which the change term is 1, and the least animal mass,
/// kg, it is a share of (a cell of less does not change by as much).
const MASS_SHARE: f64 = 0.5;
const MASS_FLOOR: f64 = 4e3;

/// What refinement keeps of a cell from the month before for its change term: the animal mass on it, kg/m² (NaN: not
/// known yet); and the depth it was built at, the coarsest it joins back to.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Watch {
    pub mass: f32,
    pub built: u8,
}

/// A new cell: nothing known, and it never joins.
impl Default for Watch {
    fn default() -> Watch {
        Watch { mass: f32::NAN, built: DEPTH_MOST }
    }
}

impl Watch {
    /// A cell built at `depth`, nothing known of it yet.
    pub(super) fn built(depth: u8) -> Watch {
        Watch { built: depth, ..Watch::default() }
    }
    /// What a split or a join makes of the watches of its cells (`World::remap`): a quarter its parent's, a joined cell
    /// the mean by area of its quarters' mass and the coarsest depth they were built at.
    pub(crate) fn made(m: &Made, at: &dyn Fn(u32) -> Watch) -> Watch {
        let built = m.pick(|i| at(i).built, |b| b.iter().copied().min().expect("four quarters"));
        Watch { mass: m.mean(|i| at(i).mass as f64) as f32, built }
    }
}

/// A cell's interest: all its terms summed; the camera's alone; the brains and parts terms; and its animal mass now
/// and a month before, kg (NaN: not known), the change term's.
#[derive(Clone, Copy, Debug)]
pub(super) struct Score {
    pub all: f64,
    pub camera: f64,
    held: f64,
    mass: [f64; 2],
}

/// The score of each cell of `w` in order, with the camera's `shot` in force, and what each keeps for the next month.
pub(super) fn scores(w: &World, shot: Option<&Shot>) -> (Vec<Score>, Vec<Watch>) {
    let (cells, n) = (w.mesh.cells(), w.mesh.cells().n());
    let now: Vec<Watch> = (0..n).map(|i| Watch { mass: (animal_kg(w, i) / cells.area()[i]) as f32, built: w.watch[i].built }).collect();
    let score = (0..n)
        .map(|i| {
            let life = w.life.cell_at(i);
            let brains: f64 = life.bodies.iter().filter(|b| b.alive && w.life.brain(b.lineage).is_some_and(|l| !l.none())).map(|b| b.count).sum();
            let parts = (life.parts.len() + life.bonds.a.len()) as f64;
            let mass = [now[i].mass, w.watch[i].mass].map(|m| m as f64 * cells.area()[i]);
            let camera = shot.map_or(0.0, |s| camera(w, i, s));
            let held = brains / BRAINS_UNIT + parts / PARTS_UNIT;
            Score { all: camera + held + change(mass), camera, held, mass }
        })
        .collect();
    (score, now)
}

/// The score of the cell four quarters of scores `q` would join into: their brains and parts summed, the change of
/// their masses summed, and the camera's twice their highest (none where they are finer than it asks).
pub(super) fn joined(q: [&Score; 4]) -> f64 {
    let mass = [0, 1].map(|k| sum4(q.map(|s| s.mass[k])));
    let camera = q.iter().map(|s| s.camera).fold(0.0, f64::max);
    sum4(q.map(|s| s.held)) + change(mass) + 2.0 * camera
}

/// The change term of a cell of animal mass `now` that was `was` a month before, kg (NaN: not known, none): the share
/// by which it moved, of at least `MASS_FLOOR`, over `MASS_SHARE`.
fn change([now, was]: [f64; 2]) -> f64 {
    if was.is_nan() {
        return 0.0;
    }
    (now - was).abs() / now.max(was).max(MASS_FLOOR) / MASS_SHARE
}

/// The camera's term of cell `i` under `shot`: in its window, 2^(asked - depth - 1), so a cell coarser than asked
/// splits (1 one depth coarser, more the coarser) and one of the depth asked (no finer than the floor,
/// `Config::finest_depth`) scores 1/2 and stays; nothing outside the window or finer than asked.
fn camera(w: &World, i: usize, s: &Shot) -> f64 {
    let (asked, depth) = (s.depth.min(w.cfg.finest_depth), w.map.leaves()[i].depth);
    let wrap = if w.map.wrap { w.map.w as f64 } else { 0.0 };
    if depth > asked || !s.sees(w.map.bounds(i), wrap) {
        return 0.0;
    }
    0.5f64.powi(1 + depth as i32 - asked as i32)
}

/// The animal mass on cell `i`, kg: its living bodies' (each the mass of all its members).
fn animal_kg(w: &World, i: usize) -> f64 {
    w.life.cell_at(i).bodies.iter().filter(|b| b.alive).map(|b| b.mass(&w.chem)).sum()
}

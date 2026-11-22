//! The cover's budget (laws.md 3): what its parts hold as fuel (`Parts::content`), and each flow that changes that,
//! booked where the law moves it (`Counters::cover_budget`), so a year's change of the cover's content is the sum of its
//! flows.

use super::build::per_input;
use super::{Cover, Parts, FUEL, NP};
use crate::chem::Chemistry;

/// The flows of the budget, J, + into the cover: the light reaction's fuel; its upkeep, each part of it (basal,
/// capacity, firmness, capture area, conducting walls, the breathing of its fine roots, their turnover); the work of
/// building and fixing (its conversions keep the content); the leaf layers it sheds, what dies (unpaid, worn, frozen,
/// pushed out, emptied), the fuel its roots give its ground, its seeds out and in (they land on a ground, `lying`, part
/// of the cover's node), the seeds lying that die, what takers take or break of it (`contract`), what is placed from
/// outside, what rounding set from below zero to zero, and what flames burnt or killed (`flame`).
pub const BUDGET: [&str; 18] = ["stored", "basal", "capacity", "firmness", "leaves", "sapwood", "root_breath", "root_renew", "building", "shed", "died", "exuded", "seeds", "seeds_died", "takers", "placed", "rounded", "burnt"];
pub const STORED: usize = 0;
/// The first of the parts of the upkeep, in the order of `BUDGET`.
pub const UPKEEP: usize = 1;
/// The parts of the upkeep.
pub const UPKEEP_PARTS: usize = 7;
pub const BUILDING: usize = 8;
pub const SHED: usize = 9;
pub const DIED: usize = 10;
pub const EXUDED: usize = 11;
pub const SEEDS: usize = 12;
pub const SEEDS_DIED: usize = 13;
pub const TAKERS: usize = 14;
pub const PLACED: usize = 15;
pub const ROUNDED: usize = 16;
pub const BURNT: usize = 17;

impl Parts {
    /// What a mol of each part holds, J: the fuel what its burning (reaction 1) frees, each other part the fuel its
    /// conversion (`Chemistry::build`, `builds`) takes to make a mol of it.
    pub fn content(&self, chem: &Chemistry) -> [f64; NP] {
        let burn = &chem.reactions[self.burn];
        let fuel = -chem.dh(self.burn) / per_input(burn, self.sub[FUEL]).unwrap_or(1.0);
        let made = |k: usize| {
            let rx = chem.build.iter().chain(&chem.builds).find(|r| r.outputs[0].0 as usize == k);
            rx.map_or(0.0, |r| per_input(r, self.sub[FUEL]).unwrap_or(0.0) / r.outputs[0].1 as f64 * fuel)
        };
        std::array::from_fn(|q| if q == FUEL { fuel } else { made(self.sub[q]) })
    }
}

/// What amounts `m` (mol of each part) hold, J, at `e` per mol (`Parts::content`).
pub fn held(e: &[f64; NP], m: &[f64; NP]) -> f64 {
    (0..NP).map(|q| m[q] * e[q]).sum()
}

impl Cover {
    /// What all its slots and its seeds lying hold, J, at `e` per mol (`Parts::content`).
    pub fn content(&self, e: &[f64; NP]) -> f64 {
        self.all_mol().map(|m| held(e, m)).sum()
    }
}

/// What the cover of the world holds, J (`Parts::content`; none without a cover).
pub fn content(w: &crate::World) -> f64 {
    Parts::of(&w.chem).map_or(0.0, |p| {
        let e = p.content(&w.chem);
        w.life.cells.iter().map(|c| c.cover.content(&e)).sum()
    })
}

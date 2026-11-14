//! The takers: animals as a field (laws.md 3b). Each ground cell holds up to `FS` kinds of takers, each a slot of
//! members of one kind: their real count, the amounts of the fuel they store and their body matter (`life::Parts`),
//! their heat and a small vector of genes, as plain arrays per cell (`Fauna`). Once a month, right after the cover's
//! month on the same cell, each slot pays its upkeep and keeps its body's heat (`month`), and its members die by what
//! they could not pay, by wear, by frost and by thirst. A taker reads the land and moves matter only through the
//! contract (`contract`); its books are the life of the cell's (`life::book`).

mod body;
mod gut;
mod month;
mod place;
pub mod take;
mod thirst;

pub use body::BAND;
pub use month::step;
pub use place::place;

use crate::chem::Chemistry;
use crate::life::{Parts, FUEL, MATTER, NO_ROOT, WET_TISSUE};
use serde::{Deserialize, Serialize};

/// Kinds of takers a cell holds at most.
pub const FS: usize = 8;
/// The parts of a taker, by their place in `Fauna::mol`: the fuel it stores and its body matter (life's `FUEL` and
/// `MATTER`).
pub const NF: usize = 2;
pub const STORE: usize = 0;
pub const BODY: usize = 1;

// The genes of a kind of takers.
/// Living mass of an adult, log10 kg
pub const MASS: usize = 0;
/// Mass of a newborn, log10 share of the adult's
pub const CHILD: usize = 1;
/// Share of the month's surplus put into young
pub const YOUNG: usize = 2;
/// Capacity of the gut to split walls into fuel, log10 mol per kg living per s at `t_cat`
pub const ENZYME: usize = 3;
/// Middle of the band of heights it searches, log10 m
pub const REACH: usize = 4;
/// Body temperature it keeps by burning where its ground is colder, K
pub const KEEP: usize = 5;
/// Insulation of its coat, a share of `Physics::r_coat`
pub const COAT: usize = 6;
/// Repair, log10: its wear life is 10^REPAIR times the rate of living's (laws.md 3)
pub const REPAIR: usize = 7;
pub const NG: usize = 8;

/// Below this many members a slot's count is whole and changes by draws: the relative noise of a count of 64 is an
/// eighth, which a mean would hide (a rule of the model).
pub const N_WHOLE: f64 = 64.0;

/// The takers of one ground cell, in `FS` slots.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fauna {
    /// Per slot: its members, a real count (whole below `N_WHOLE`)
    pub adults: [f64; FS],
    /// Per slot: mol of each part (`STORE`, `BODY`)
    pub mol: [[f64; NF]; FS],
    /// Per slot: its heat, J (its members at their own temperature)
    pub e: [f64; FS],
    /// Per slot: its genes, and the starting kind it descends from (a label for the tape; `NO_ROOT` when empty)
    pub genes: [[f32; NG]; FS],
    pub root: [u32; FS],
}

impl Default for Fauna {
    /// No takers.
    fn default() -> Fauna {
        Fauna { adults: [0.0; FS], mol: [[0.0; NF]; FS], e: [0.0; FS], genes: [[0.0; NG]; FS], root: [NO_ROOT; FS] }
    }
}

impl Fauna {
    /// Whether it holds any takers.
    pub fn any(&self) -> bool {
        self.root.iter().any(|&r| r != NO_ROOT)
    }
    /// Its slots that hold takers.
    pub fn held(&self) -> impl Iterator<Item = usize> + '_ {
        (0..FS).filter(|&x| self.root[x] != NO_ROOT)
    }
    /// The living mass of slot `x`, kg: its body matter with the liquid of living tissue (`life::WET_TISSUE`).
    pub fn living(&self, mm: &[f64; NF], x: usize) -> f64 {
        self.mol[x][BODY] * mm[BODY] / (1.0 - WET_TISSUE)
    }
    /// The heat capacity of slot `x`, J/K.
    pub fn cap(&self, chem: &Chemistry, sub: &[usize; NF], x: usize) -> f64 {
        (0..NF).map(|q| self.mol[x][q] * chem.subs[sub[q]].cp).sum()
    }
    /// Its amounts of each part, slot by slot (mol).
    pub fn all_mol(&self) -> impl Iterator<Item = &[f64; NF]> + '_ {
        self.mol.iter()
    }
    /// Its heat, J.
    pub fn heat(&self) -> f64 {
        self.e.iter().sum()
    }
}

/// The substances of a taker's parts (`STORE`, `BODY`) among life's parts.
pub fn sub(p: &Parts) -> [usize; NF] {
    [p.sub[FUEL], p.sub[MATTER]]
}

/// The molar mass of a taker's parts, kg/mol.
pub fn mm(chem: &Chemistry, p: &Parts) -> [f64; NF] {
    sub(p).map(|k| chem.subs[k].molar_mass)
}

/// The living mass of an adult of genes `g`, kg (its `MASS` gene).
pub fn adult(g: &[f32; NG]) -> f64 {
    crate::math::dec(g[MASS])
}

/// The takers of the world by starting kind (index: the root), in the order of the cells and their slots: members and
/// living mass, kg.
pub fn sums(w: &crate::World) -> Vec<[f64; 2]> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mm = mm(&w.chem, &p);
    let mut out: Vec<[f64; 2]> = Vec::new();
    for f in w.life.cells.iter().map(|c| &c.fauna) {
        for x in f.held() {
            let r = f.root[x] as usize;
            if r >= out.len() {
                out.resize(r + 1, [0.0; 2]);
            }
            out[r] = [out[r][0] + f.adults[x], out[r][1] + f.living(&mm, x)];
        }
    }
    out
}

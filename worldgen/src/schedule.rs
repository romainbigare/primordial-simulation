//! The plan of a run: what is placed in a world, and when (the soil and the starting kinds once life starts). It is an
//! input of the run, reported with every result. A snapshot carries the schedule, so a resumed run places what is
//! still due as the unbroken run does.

use crate::life::{self, Seeding, KINDS};
use pcore::World;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// The year life starts: the soil gets its mineral (`life::soil`); none: no life
    pub life: Option<u64>,
    /// The starting kinds (rows of `KINDS`) and the year each is placed
    pub kinds: Vec<(usize, u64)>,
    /// How the kinds are placed
    pub seeding: Seeding,
}

impl Schedule {
    /// Life from year `life` on: the soil, then the starting kinds `rows` (rows of `KINDS`: `life::TWO`, `life::ROSTER`,
    /// or the cover of either, `life::plants`), each at its `Kind::year` after the first life.
    pub fn new(life: Option<u64>, rows: &[usize], seeding: Seeding) -> Self {
        let kinds = life.map_or(vec![], |y| rows.iter().map(|&k| (k, y + KINDS[k].year)).collect());
        Schedule { life, kinds, seeding }
    }

    /// Place what is due in the world's year now (call at the start of each year): the soil, then the kinds in their
    /// order. Returns the lines to print.
    pub fn place(&self, w: &mut World) -> Vec<String> {
        let year = w.cfg.calendar().years(w.time);
        let mut said = vec![];
        if self.life == Some(year) {
            said.push(life::soil(w, &crate::chemistry0().1.soil));
        }
        for &(kind, at) in &self.kinds {
            if at == year {
                said.push(life::seed_with(w, kind, &self.seeding));
            }
        }
        said
    }
}

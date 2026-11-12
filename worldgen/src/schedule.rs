//! The plan of a run: what is placed in a world, and when (the soil and the starting kinds once life starts). It is an
//! input of the run, reported with every result. A snapshot carries the schedule, so a resumed run places what is
//! still due as the unbroken run does.

use crate::life::{self, Place, Seeding, KINDS};
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
    /// Life from year `life` on: the soil and the cover the nursery sorts (`life::nursery`), then the bodies of the
    /// starting kinds `rows` (rows of `KINDS`: `life::TWO`, `life::ROSTER`, or none), each at its `Kind::year` after the
    /// first life; the rows of cover among them the nursery knows, and they are not placed.
    pub fn new(life: Option<u64>, rows: &[usize], seeding: Seeding) -> Self {
        let bodies = rows.iter().copied().filter(|&k| matches!(KINDS[k].place, Place::Bodies(_)));
        let kinds = life.map_or(vec![], |y| bodies.map(|k| (k, y + KINDS[k].year)).collect());
        Schedule { life, kinds, seeding }
    }

    /// Place what is due in the world's year now (call at the start of each year): the soil and the sorted cover, then
    /// the kinds in their order. Returns the lines to print.
    pub fn place(&self, w: &mut World) -> Vec<String> {
        let year = w.cfg.calendar().years(w.time);
        let mut said = vec![];
        if self.life == Some(year) {
            said.push(life::soil(w, &crate::chemistry0().1.soil));
            said.push(life::nursery::start(w));
        }
        for &(kind, at) in &self.kinds {
            if at == year {
                said.push(life::seed_with(w, kind, &self.seeding));
            }
        }
        said
    }
}

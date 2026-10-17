//! What is placed in a world, and when: the soil and the starting kinds once life starts, and the big-brain test.
//! These are initial conditions, reported with every result. A snapshot carries the schedule, so a resumed run
//! places what is still due as the unbroken run does.

use crate::life::{self, Seeding, KINDS, ROSTER};
use pcore::World;
use serde::{Deserialize, Serialize};

/// The kinds `ew run` places without `--roster` (rows of `KINDS`): light-eaters, then eaters.
const TWO: [usize; 2] = [0, 1];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// The year life starts: the soil gets its mineral (`life::soil`); none: no life
    pub life: Option<u64>,
    /// The starting kinds (rows of `KINDS`) and the year each is placed
    pub kinds: Vec<(usize, u64)>,
    /// The year of the big-brain test (`life::seed_clever`) and how many eaters it copies
    pub clever: Option<(u64, usize)>,
    /// How the kinds are placed
    pub seeding: Seeding,
}

impl Schedule {
    /// Life from year `life` on: the soil, then the two starting kinds, or with `roster` the kinds of `ROSTER`, each at
    /// its `Kind::year` after the first life.
    pub fn new(life: Option<u64>, roster: bool, clever: Option<(u64, usize)>, seeding: Seeding) -> Self {
        let rows: &[usize] = if roster { &ROSTER } else { &TWO };
        let kinds = life.map_or(vec![], |y| rows.iter().map(|&k| (k, y + KINDS[k].year)).collect());
        Schedule { life, kinds, clever, seeding }
    }

    /// Place what is due in the world's year now (call at the start of each year): the soil, the kinds in their order,
    /// the big-brain test. Returns the lines to print.
    pub fn place(&self, w: &mut World) -> Vec<String> {
        let year = w.day / w.days_per_year();
        let mut said = vec![];
        if self.life == Some(year) {
            said.push(life::soil(w, &crate::chemistry0().1.soil));
        }
        for &(kind, at) in &self.kinds {
            if at == year {
                said.push(life::seed_with(w, kind, &self.seeding));
            }
        }
        if let Some((_, n)) = self.clever.filter(|c| c.0 == year) {
            said.push(life::seed_clever(w, n));
        }
        said
    }
}

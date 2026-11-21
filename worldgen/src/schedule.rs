//! The plan of a run: what is placed in a world, and when (the soil and the starting kinds once life starts), and the
//! edits made to it (handover.md 11: an action is a physics edit). It is an input of the run, reported with every
//! result. A snapshot carries the schedule, so a resumed run places what is still due as the unbroken run does.

use crate::life::{self, Seeding, KINDS};
use pcore::edit::{Act, Edit};
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
    /// The year the starting kinds of takers are placed (`life::place_takers`); none: no takers
    pub takers: Option<u64>,
    /// The edits, each at the start of its year (`Due`)
    pub edits: Vec<Due>,
}

/// An edit due at the start of a year (`World::edit`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Due {
    pub year: u64,
    pub edit: Edit,
}

/// The edit that places the cover of the starting kind of row `row` of `KINDS`, its genes as they start and `Kind::kg`
/// per m² (`Act::Plant`).
pub fn plant(row: usize) -> Act {
    let kind = &KINDS[row];
    Act::Plant { genes: kind.genes, root: row as u32, kg: kind.kg }
}

impl Schedule {
    /// Life from year `life` on: the soil, then the starting kinds `rows` (rows of `KINDS`: `life::ONE` or `life::ROSTER`),
    /// each at its `Kind::year` after the first life; no takers (a run that wants them sets `takers`).
    pub fn new(life: Option<u64>, rows: &[usize], seeding: Seeding) -> Self {
        let kinds = life.map_or(vec![], |y| rows.iter().map(|&k| (k, y + KINDS[k].year)).collect());
        Schedule { life, kinds, seeding, takers: None, edits: Vec::new() }
    }

    /// Place what is due in the world's year now (call at the start of each year): the soil, then the kinds in their
    /// order, then the edits in theirs. Returns the lines to print.
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
        if self.life.is_some() && self.takers == Some(year) {
            said.extend(life::place_takers(w));
        }
        let due: Vec<Edit> = self.edits.iter().filter(|e| e.year == year).map(|e| e.edit).collect();
        let done = w.edit(&due);
        said.extend(due.iter().zip(done).map(|(e, done)| format!("year {:.1}: {}", w.year(), e.line(w.cell_under(e.place), done))));
        said
    }
}

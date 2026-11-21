//! The plan of a run: what is placed in a world, and when (the soil and the starting kinds once life starts), and the
//! edits made to it (handover.md 11: an action is a physics edit). It is an input of the run, reported with every
//! result. A snapshot carries the schedule, so a resumed run places what is still due as the unbroken run does.

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
    /// The year the starting kinds of takers are placed (`life::place_takers`); none: no takers
    pub takers: Option<u64>,
    /// The edits, each at the start of its year (`Due`)
    pub edits: Vec<Due>,
}

/// An edit due in a year: the place it is made at, in squares from the map's north-west corner (x east, y south: 3.5,
/// 2.5 is the middle of square 3, 2), whose cell it acts on (`World::cell_under`), and what it does.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Due {
    pub year: u64,
    pub place: [f64; 2],
    pub edit: Edit,
}

/// An edit of a cell's cover (`Schedule::edits`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Edit {
    /// The share of the cell made a new patch whose plants die above the ground (`World::clear`), about the cell's
    /// sample under the edit's place (`World::sample_in`)
    Clear(f64),
    /// The share of the cell's first patch made a new patch alike to it, a control that changes nothing (`World::split`)
    Split(f64),
    /// The cover of a starting kind (a row of `KINDS`), its genes as they start and `Kind::kg` per m², placed in the
    /// cell's newest patch (`cover::place`)
    Plant(usize),
}

impl Edit {
    /// Make the edit on the cell `i` of `w` at its sample `at`; returns what it did, in a line.
    fn make(&self, w: &mut World, i: usize, at: usize) -> String {
        let c = w.life.cover(i);
        let newest = (0..c.patch_count()).rev().min_by(|&a, &b| c.patches[a].age.total_cmp(&c.patches[b].age)).unwrap_or(0);
        let (done, what) = match *self {
            Edit::Clear(s) => (w.clear(i, s, Some(at)), format!("cleared {s} of the cover of cell {i} as a new patch")),
            Edit::Split(s) => (w.split(i, s), format!("split {s} of the cover of cell {i} off as a new patch alike to the rest")),
            Edit::Plant(row) => {
                let kind = &KINDS[row];
                (pcore::cover::place(w, i, newest, kind.genes, row as u32, kind.kg), format!("placed {} kg per m2 of {} in patch {newest} of cell {i}", kind.kg, kind.name))
            }
        };
        format!("year {:.1}: {what}{}", w.year(), if done { "" } else { ": nothing done" })
    }
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
        let due: Vec<&Due> = self.edits.iter().filter(|e| e.year == year).collect();
        if !due.is_empty() {
            pcore::month_start(w);
        }
        for d in due {
            let i = w.cell_under(d.place);
            said.push(d.edit.make(w, i, w.sample_in(i, d.place)));
        }
        said
    }
}

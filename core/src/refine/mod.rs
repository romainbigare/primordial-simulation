//! Refinement (laws.md 0): at a month's start the cells of the highest interest (`score`) split and the quartets of
//! the lowest join, under a budget of cells (`Config::cell_budget`) and no finer than a floor
//! (`Config::finest_depth`), the camera (`camera`) an input of the run. A setting for speed, not a law: no law reads
//! the score, and with no budget the cells stay as built.

mod camera;
mod score;

pub use camera::{Camera, Shot};
pub use score::Watch;

use crate::event::{Event, EventKind};
use crate::remap::Change;
use crate::space::Leaf;
use crate::World;
use score::{Score, JOIN_AT, SPLIT_AT};

/// The most splits and joins in a month (a split the 2:1 rule forces counts as one), so a month's refinement takes a
/// bounded time; what is left waits for the next month.
const MOST_CHANGES: usize = 32;

/// A cell that wants to split: its score, its serial (ties go to the least), its leaf, and whether the camera asks
/// for it (its term alone splits it).
struct Want {
    score: f64,
    serial: u32,
    leaf: Leaf,
    asked: bool,
}

/// Four quarters that may join into `parent`: the highest of their scores, their least serial (ties go to the least),
/// and whether the camera sees any of them or a cell beside them (a split there would split their join again, 2:1).
struct Quartet {
    top: f64,
    serial: u32,
    parent: Leaf,
    seen: bool,
}

/// The month's refinement as it goes: the cells now, the budget, the changes made, the events of the joins' merges
/// with the leaf of the cell each names (`World::join_quartet`), and the change of the cells they make, one for the
/// month (`Change`).
struct Month {
    cells: usize,
    budget: usize,
    made: usize,
    held: Vec<(Event, Leaf)>,
    change: Change,
}

impl World {
    /// Refine the cells at a month's start, before its survey, once the shell took the events (laws.md 0): score each
    /// cell (`score`), with the shot of `camera` in force this month; then, the quietest first, join each quartet whose
    /// four quarters all score under `JOIN_AT`, or while the cells are over the budget; then, those the camera asks for
    /// first, then the highest score first (ties by serial), split each cell scoring `SPLIT_AT` or more above the floor
    /// whose split (with those the 2:1 rule forces) keeps the cells within the budget. A split the camera asks for may
    /// first join the quietest quartets it does not see, nor a cell beside them, to make room: what is looked at comes
    /// first, and what it took room from does not take it back while the camera asks. A cell joins no coarser than it
    /// was built. At most
    /// `MOST_CHANGES` a month. The events of a join's merge name their cell by index, which the next change renumbers:
    /// they wait with their cell's leaf and come back at the end naming the cell that then holds its place.
    /// Each change is an event (`World::split`, `World::join`). Returns the splits and joins made; none with no budget.
    pub fn refine(&mut self, camera: &Camera) -> usize {
        if self.cfg.cell_budget == 0 {
            return 0;
        }
        let n = self.mesh.cells().n();
        if self.watch.len() != n {
            self.watch = self.map.leaves().iter().map(|l| Watch::built(l.depth)).collect();
        }
        let (score, now) = score::scores(self, camera.at(self.month()));
        self.watch = now;
        let (wants, quartets) = (self.wants(&score), self.quartets(&score));
        let mut m = Month { cells: n, budget: self.cfg.cell_budget as usize, made: 0, held: Vec::new(), change: self.change() };
        let mut next = 0;
        while let Some(q) = quartets.get(next).filter(|q| m.made < MOST_CHANGES && (m.cells > m.budget || q.top < JOIN_AT)) {
            next += 1;
            self.join_quartet(q, &mut m);
        }
        let mut room = quartets[next..].iter().filter(|q| !q.seen);
        for s in &wants {
            let Ok(mut forced) = self.map.splits(s.leaf) else { continue };
            while s.asked && m.cells + 3 * forced.len() > m.budget && m.made + forced.len() < MOST_CHANGES {
                let Some(q) = room.next() else { break };
                self.join_quartet(q, &mut m);
                forced = self.map.splits(s.leaf).expect("a cell the camera sees joins with no quartet it does not see");
            }
            if m.made + forced.len() > MOST_CHANGES {
                break;
            }
            if m.cells + 3 * forced.len() <= m.budget {
                let made = self.split_in(s.leaf, &mut m.change).expect("a split the tree allows").len();
                (m.cells, m.made) = (m.cells + 3 * made, m.made + made);
            }
        }
        self.changed(m.change);
        let tree = self.mesh.tree();
        let back: Vec<Event> = m.held.into_iter().map(|(e, l)| Event { node: tree.cell_at(l.corner()) as u32, ..e }).collect();
        self.events.extend(back);
        m.made
    }

    /// The cells that want to split, those the camera asks for first, then the highest score first: those of `SPLIT_AT`
    /// or more above the floor.
    fn wants(&self, score: &[Score]) -> Vec<Want> {
        let floor = self.cfg.finest_depth;
        let leaves = self.map.leaves();
        let mut wants: Vec<Want> = (score.iter().enumerate())
            .filter(|(i, s)| s.all >= SPLIT_AT && leaves[*i].depth < floor)
            .map(|(i, s)| Want { score: s.all, serial: self.life.serial(i), leaf: leaves[i], asked: s.camera >= SPLIT_AT })
            .collect();
        wants.sort_by(|a, b| b.asked.cmp(&a.asked).then(b.score.total_cmp(&a.score)).then(a.serial.cmp(&b.serial)));
        wants
    }

    /// The quartets that may join, the quietest first: four cells that are the quarters of one leaf, built no coarser
    /// than it, none with a cell across a side finer than itself (the joined cell would border one two depths finer).
    fn quartets(&self, score: &[Score]) -> Vec<Quartet> {
        let (cells, leaves) = (self.mesh.cells(), self.map.leaves());
        let beside = |i: usize| {
            let (links, other) = cells.around(i);
            links.iter().zip(other).filter(|(&l, _)| cells.width()[l as usize] > 0.0).map(|(_, &j)| j as usize)
        };
        let fine_beside = |i: usize| beside(i).any(|j| leaves[j].depth > leaves[i].depth);
        let seen = |i: usize| score[i].camera > 0.0 || beside(i).any(|j| score[j].camera > 0.0);
        let mut quartets: Vec<Quartet> = (0..leaves.len())
            .filter_map(|i| {
                let parent = leaves[i].parent().filter(|p| p.quarters()[0] == leaves[i])?;
                let kids = parent.quarters().map(|q| self.map.cell(q));
                let kids = [kids[0]?, kids[1]?, kids[2]?, kids[3]?];
                let joins = kids.iter().all(|&k| self.watch[k].built <= parent.depth && !fine_beside(k));
                let top = kids.iter().map(|&k| score[k].all).fold(f64::NEG_INFINITY, f64::max);
                let serial = kids.iter().map(|&k| self.life.serial(k)).min().expect("four quarters");
                joins.then_some(Quartet { top, serial, parent, seen: kids.iter().any(|&k| seen(k)) })
            })
            .collect();
        quartets.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.serial.cmp(&b.serial)));
        quartets
    }

    /// Join quartet `q` if the tree allows it, counted in `m`; the events of its merge (each naming a cell by its
    /// index) wait in `m` with their cell's leaf, as a remap takes only splits and joins.
    fn join_quartet(&mut self, q: &Quartet, m: &mut Month) {
        if self.join_in(q.parent, &mut m.change).is_ok() {
            (m.cells, m.made) = (m.cells - 3, m.made + 1);
        }
        let (kept, merged): (Vec<Event>, Vec<Event>) = std::mem::take(&mut self.events).into_iter().partition(|e| matches!(e.kind, EventKind::Split | EventKind::Join));
        self.events = kept;
        m.held.extend(merged.into_iter().map(|e| (e, self.map.leaves()[e.node as usize])));
    }
}

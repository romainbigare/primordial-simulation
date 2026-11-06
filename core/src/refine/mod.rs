//! Refinement (laws.md 0): at a month's start the cells of the highest interest (`score`) split and the quartets of
//! the lowest join, under a budget of cells (`Config::cell_budget`, or the cells as built while the camera rests) and no finer than a floor
//! (`Config::finest_depth`), the camera (`camera`) an input of the run. A setting for speed, not a law: no law reads
//! the score, and with no budget the cells stay as built.

mod camera;
mod join;
mod score;

pub use camera::{Camera, Shot};
pub use score::{Watch, PARTS_UNIT};

use crate::event::Event;
use crate::remap::Change;
use crate::space::tree::SIDES;
use crate::space::Leaf;
use crate::World;
use score::{Score, SPLIT_AT};
use std::collections::BTreeMap;

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

/// Four quarters that may join into `parent`: the score of the cell they would make (`score::merged`), their least
/// serial (ties go to the least), and whether the camera sees any of them or a cell beside them (a split there would
/// split their join again, 2:1).
#[derive(Clone)]
struct Quartet {
    score: f64,
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
    /// cell (`score`), with the shot of `camera` in force this month; then, the quietest first, join each quartet that
    /// would make a cell scoring under `JOIN_AT`, or while the cells are over the budget; then, those the camera asks
    /// for first, then the highest score first (ties by serial), split each cell scoring `SPLIT_AT` or more above the
    /// floor whose split (with those the 2:1 rule forces) keeps the cells within the budget, until no split can (the
    /// least makes 3 cells more); a split is worked out only where the least it makes fits (`least_splits`). A split
    /// the camera asks for may first join the quietest quartets it does not see, nor a cell beside them, to make room:
    /// what is looked at comes first, and what it took room from does not take it back while the camera asks. The
    /// quarters of a split the camera asks for that it asks for in turn split next, before any other (`asked_quarters`),
    /// so a zoom goes down several depths in a month; and the joins go on with the quartets of the cells joins made
    /// and those 2:1 refused until a finer cell beside them joined (`join_quiet`), so the cells of a window let go join
    /// back up several depths. A cell joins no coarser than it was built. At most
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
        let shot = camera.at(self.month());
        let (score, now) = score::scores(self, shot);
        self.watch = now;
        let (wants, quartets) = (self.wants(&score), self.quartets(&score));
        let mut wants: std::collections::VecDeque<Want> = wants.into();
        let budget = if camera.rests(self.month()) { self.cells_as_built().min(self.cfg.cell_budget as usize) } else { self.cfg.cell_budget as usize };
        let mut m = Month { cells: n, budget, made: 0, held: Vec::new(), change: self.change() };
        let mut by_leaf: BTreeMap<Leaf, Score> = self.map.leaves().iter().copied().zip(score).collect();
        let next = self.join_quiet(&quartets, &mut by_leaf, &mut m);
        let mut room = quartets[next..].iter().filter(|q| !q.seen).peekable();
        while let Some(s) = wants.pop_front() {
            if m.cells + 3 > m.budget && (!s.asked || room.peek().is_none()) {
                break;
            }
            let Some(least) = self.least_splits(s.leaf) else { continue };
            if m.made + least > MOST_CHANGES {
                break;
            }
            // (A split that cannot fit is not worked out.)
            if !s.asked && m.cells + 3 * least > m.budget {
                continue;
            }
            let Ok(mut forced) = self.map.splits(s.leaf) else { continue };
            while s.asked && m.cells + 3 * forced.len() > m.budget && m.made + forced.len() < MOST_CHANGES {
                let Some(q) = room.next() else { break };
                _ = self.join_quartet(q, &mut m);
                forced = self.map.splits(s.leaf).expect("a cell the camera sees joins with no quartet it does not see");
            }
            if m.made + forced.len() > MOST_CHANGES {
                break;
            }
            if m.cells + 3 * forced.len() <= m.budget {
                let made = self.split_in(s.leaf, &mut m.change).expect("a split the tree allows").len();
                (m.cells, m.made) = (m.cells + 3 * made, m.made + made);
                if let Some(shot) = shot.filter(|_| s.asked) {
                    self.asked_quarters(s.leaf, shot).into_iter().rev().for_each(|q| wants.push_front(q));
                }
            }
        }
        self.changed(m.change);
        let tree = self.mesh.tree();
        let back: Vec<Event> = m.held.into_iter().map(|(e, l)| Event { node: tree.cell_at(l.corner()) as u32, ..e }).collect();
        self.events.extend(back);
        m.made
    }

    /// The cells the mesh would have with every cell joined back as far as it was built: each cell counts the share
    /// 4^(built − depth) of a built cell (`Watch::built`), so the sum is a whole number.
    fn cells_as_built(&self) -> usize {
        let share = |(l, w): (&Leaf, &Watch)| 0.25f64.powi(l.depth.saturating_sub(w.built) as i32);
        self.map.leaves().iter().zip(&self.watch).map(share).sum::<f64>().round() as usize
    }

    /// The quarters of `leaf`, split just now, that the camera's `shot` asks to split in turn (its term alone 1 or more),
    /// in order, the highest first.
    fn asked_quarters(&self, leaf: Leaf, shot: &Shot) -> Vec<Want> {
        let mut asked: Vec<Want> = (leaf.quarters().iter().filter_map(|&q| self.map.cell(q)))
            .map(|i| (i, score::camera(self, i, shot)))
            .filter(|&(_, c)| c >= SPLIT_AT)
            .map(|(i, score)| Want { score, serial: self.life.serial(i), leaf: self.map.leaves()[i], asked: true })
            .collect();
        asked.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.serial.cmp(&b.serial)));
        asked
    }

    /// The least splits a split of cell `leaf` makes (`Square::splits`), if it is a cell: itself, and each coarser
    /// cell across one of its sides, which the 2:1 rule splits first, by the tree of the cells as they are now.
    fn least_splits(&self, leaf: Leaf) -> Option<usize> {
        let (tree, leaves) = (self.mesh.tree(), self.map.leaves());
        self.map.cell(leaf)?;
        let mut coarser: Vec<usize> = SIDES.iter().flat_map(|&way| tree.beside(&leaf, way)).filter(|&c| leaves[c].depth < leaf.depth).collect();
        coarser.sort_unstable();
        coarser.dedup();
        Some(1 + coarser.len())
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

    /// The quartets that may join, the one that would make the quietest cell first: four cells that are the quarters of
    /// one leaf, built no coarser than it (one with a cell across a side finer than itself joins once that cell joined,
    /// 2:1, `join_quiet`).
    fn quartets(&self, score: &[Score]) -> Vec<Quartet> {
        let (cells, leaves) = (self.mesh.cells(), self.map.leaves());
        let beside = |i: usize| {
            let (links, other) = cells.around(i);
            links.iter().zip(other).filter(|(&l, _)| cells.width()[l as usize] > 0.0).map(|(_, &j)| j as usize)
        };
        let seen = |i: usize| score[i].camera > 0.0 || beside(i).any(|j| score[j].camera > 0.0);
        let mut quartets: Vec<Quartet> = (0..leaves.len())
            .filter_map(|i| {
                let parent = leaves[i].parent().filter(|p| p.quarters()[0] == leaves[i])?;
                let kids = parent.quarters().map(|q| self.map.cell(q));
                let kids = [kids[0]?, kids[1]?, kids[2]?, kids[3]?];
                let joins = kids.iter().all(|&k| self.watch[k].built <= parent.depth);
                let joined = score::merged(kids.map(|k| &score[k])).all;
                let serial = kids.iter().map(|&k| self.life.serial(k)).min().expect("four quarters");
                joins.then_some(Quartet { score: joined, serial, parent, seen: kids.iter().any(|&k| seen(k)) })
            })
            .collect();
        quartets.sort_by(|a, b| a.score.total_cmp(&b.score).then(a.serial.cmp(&b.serial)));
        quartets
    }
}

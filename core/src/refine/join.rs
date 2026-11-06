//! The joins of a month's refinement (laws.md 0): the quietest quartets first, then again those the tree refused for
//! 2:1 until a finer cell beside them joined, and the quartets of the cells the joins made, while any joins, so the
//! cells of a window the camera let go join back up several depths in a month.

use super::score::{self, Score, JOIN_AT};
use super::{Month, Quartet, MOST_CHANGES};
use crate::event::{Event, EventKind};
use crate::space::tree::Refused;
use crate::space::Leaf;
use crate::World;
use std::collections::BTreeMap;

impl World {
    /// Join the quartets of `quartets` (the quietest first) while they join (`Month::joins`), then those that wait
    /// (`join_or_wait`), the quietest first, round after round while any joins. `by_leaf`: the score of each cell of the
    /// month by its leaf. Returns how many of `quartets` were tried: the rest wait for room for the camera's splits.
    pub(super) fn join_quiet(&mut self, quartets: &[Quartet], by_leaf: &mut BTreeMap<Leaf, Score>, m: &mut Month) -> usize {
        let (mut next, mut again) = (0, Vec::new());
        while let Some(q) = quartets.get(next).filter(|q| m.joins(q.score)) {
            next += 1;
            self.join_or_wait(q, by_leaf, m, &mut again);
        }
        loop {
            let (made, mut tries) = (m.made, std::mem::take(&mut again));
            tries.sort_by(|a, b| a.score.total_cmp(&b.score).then(a.serial.cmp(&b.serial)));
            for q in &tries {
                if m.joins(q.score) {
                    self.join_or_wait(q, by_leaf, m, &mut again);
                }
            }
            if m.made == made {
                return next;
            }
        }
    }

    /// Join quartet `q`. If it joins, its cell scores its quarters' merged (`score::merged`), and the quartet of that
    /// cell, if its four are cells built no coarser than their parent, waits in `again`; if the tree refuses it for 2:1,
    /// it waits there itself.
    fn join_or_wait(&mut self, q: &Quartet, by_leaf: &mut BTreeMap<Leaf, Score>, m: &mut Month, again: &mut Vec<Quartet>) {
        let scores = |by_leaf: &BTreeMap<Leaf, Score>, l: Leaf| l.quarters().map(|k| by_leaf.get(&k).copied());
        match self.join_quartet(q, m) {
            Err(Refused::Unbalanced) => again.push(q.clone()),
            Err(_) => {}
            Ok(()) => {
                let [Some(a), Some(b), Some(c), Some(d)] = scores(by_leaf, q.parent) else { return };
                by_leaf.insert(q.parent, score::merged([&a, &b, &c, &d]));
                let Some(parent) = q.parent.parent() else { return };
                let cells = parent.quarters().map(|k| self.map.cell(k).filter(|&c| self.watch[c].built <= parent.depth));
                let ([Some(a), Some(b), Some(c), Some(d)], [Some(i), Some(j), Some(k), Some(l)]) = (scores(by_leaf, parent), cells) else { return };
                let serial = [i, j, k, l].map(|x| self.life.serial(x)).into_iter().min().expect("four quarters");
                again.push(Quartet { score: score::merged([&a, &b, &c, &d]).all, serial, parent, seen: false });
            }
        }
    }

    /// Join quartet `q` if the tree allows it, counted in `m`; the events of its merge (each naming a cell by its
    /// index) wait in `m` with their cell's leaf, as a remap takes only splits and joins.
    pub(super) fn join_quartet(&mut self, q: &Quartet, m: &mut Month) -> Result<(), Refused> {
        let joined = self.join_in(q.parent, &mut m.change);
        if joined.is_ok() {
            (m.cells, m.made) = (m.cells - 3, m.made + 1);
        }
        let (kept, merged): (Vec<Event>, Vec<Event>) = std::mem::take(&mut self.events).into_iter().partition(|e| matches!(e.kind, EventKind::Split | EventKind::Join));
        self.events = kept;
        m.held.extend(merged.into_iter().map(|e| (e, self.map.leaves()[e.node as usize])));
        joined
    }
}

impl Month {
    /// Whether four quarters whose cell would score `score` join now: while the month may change more, if the cells are
    /// over the budget or that cell would score under `JOIN_AT`.
    pub(super) fn joins(&self, score: f64) -> bool {
        self.made < MOST_CHANGES && (self.cells > self.budget || score < JOIN_AT)
    }
}

//! The lineages and the born rows of `Life`: their registries, the holders counted into them, and the provisional ids
//! of those body code makes, kept by their domain until the world settles and makes them global (laws.md 0).

use super::{free_place, Domain, Life};
use crate::body::Body;
use crate::brain::Shape;
use crate::lineage::Lineage;
use crate::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The bit that marks a provisional id of a lineage or of born rows, made by body code (laws.md 0): its domain lies above
/// `PROV_SHIFT`, its place in that domain's list below.
const PROVISIONAL: u32 = 1 << 31;
/// The bit of a provisional lineage's id that says it has no brain (`Lineage::none`), so `Life::brainless` reads it
/// through the world alone (`body::upkeep`) while its domain is out.
const NONE: u32 = 1 << 30;
/// The bits of a provisional id below its domain: room for 65,536 made in one domain between two settlings, and 16,384
/// domains.
const PROV_SHIFT: u32 = 16;

/// What a count of holders is of: a lineage (its living bodies) or born rows (the bodies born with them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Held {
    Lineage,
    Rows,
}

/// Born brain rows and how many hold them: the living bodies born with them (most children share their parent's, `copy`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub refs: u32,
}

impl Life {
    pub fn lineages(&self) -> usize {
        self.members.iter().filter(|&&m| m > 0).count()
    }
    /// A new lineage, of no bodies yet (body code makes provisional ones, `Domain::new_lineage`).
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        self.brains.push(Some(l));
        self.members.push(0);
        (self.brains.len() - 1) as u32
    }
    /// The brain of `lineage` (none once it died out; a provisional one is read through its domain, `Domain::brain`).
    pub fn brain(&self, lineage: u32) -> Option<&Lineage> {
        self.brains.get(lineage as usize)?.as_ref()
    }
    /// Whether `lineage` has a brain (none once it died out); of a provisional one, from its id.
    pub(crate) fn has_brain(&self, lineage: u32) -> Option<bool> {
        provisional(lineage).map_or_else(|| self.brain(lineage).map(|l| !l.none()), |_| Some(lineage & NONE == 0))
    }
    /// New born rows, not yet given to a body (body code makes provisional ones, `Domain::new_genome`).
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        let i = free_place(&mut self.genomes, &mut self.free_genomes);
        self.genomes[i] = Genome { rows, refs: 0 };
        i as u32
    }
    /// Body `b` has no brain (`Lineage::none`).
    pub(crate) fn brainless(&self, b: &Body) -> bool {
        self.has_brain(b.lineage) == Some(false)
    }
    /// The rows body `b` was born with (provisional ones are read through their domain, `Domain::born`).
    pub fn born(&self, b: &Body) -> &[f32] {
        &self.genomes[b.genome as usize].rows
    }
    /// The holders of lineage or born rows `x`.
    fn holders(&mut self, (what, x): (Held, u32)) -> &mut u32 {
        match what {
            Held::Rows => &mut self.genomes[x as usize].refs,
            Held::Lineage => &mut self.members[x as usize],
        }
    }
    /// Lineage or born rows `x` has no holder: it dies out, or its rows are freed.
    fn gone(&mut self, (what, x): (Held, u32)) {
        match what {
            Held::Rows => {
                self.genomes[x as usize].rows = Vec::new();
                self.free_genomes.push(x);
            }
            Held::Lineage => self.brains[x as usize] = None,
        }
    }
    /// The lineages and born rows the domains made get global ids, domain by domain in the order made, and the bodies,
    /// the outbox and the changes of holders of each domain name those (laws.md 0).
    pub(crate) fn remap(&mut self, ds: std::ops::Range<usize>) {
        let made: Vec<usize> = ds.filter(|&d| !self.domains[d].brains.is_empty() || !self.domains[d].genomes.is_empty()).collect();
        for d in made {
            let lin: Vec<u32> = std::mem::take(&mut self.domains[d].brains).into_iter().map(|l| self.new_lineage(l)).collect();
            let gen: Vec<u32> = std::mem::take(&mut self.domains[d].genomes).into_iter().map(|g| self.new_genome(g)).collect();
            let to = |x: u32, what: Held| provisional(x).map_or(x, |(_, k)| [&lin, &gen][what as usize][k]);
            let x = &mut self.domains[d];
            x.bodies.iter_mut().for_each(|b| (b.lineage, b.genome) = (to(b.lineage, Held::Lineage), to(b.genome, Held::Rows)));
            x.out.iter_mut().for_each(|e| e.remap(&to));
            x.counted.iter_mut().for_each(|c| c.1 = to(c.1, c.0));
        }
    }
    /// The changes of holders the domains made are added, domain by domain in the order made; then, in the order of
    /// their ids, the lineages changed that have no body die out and the born rows changed with no holder are freed.
    pub(crate) fn count_all(&mut self, ds: std::ops::Range<usize>) {
        let counted: Vec<(Held, u32, i32)> = self.domains[ds].iter_mut().flat_map(|d| std::mem::take(&mut d.counted)).collect();
        let changed: BTreeSet<(Held, u32)> = counted.iter().map(|c| (c.0, c.1)).collect();
        for (what, x, by) in counted {
            let n = self.holders((what, x));
            *n = n.wrapping_add_signed(by);
        }
        for x in changed {
            if *self.holders(x) == 0 {
                self.gone(x);
            }
        }
    }
}

impl Domain {
    /// The living lineages of starting kind `root` this domain sees, with their ids: the world's that have members, then
    /// its own provisional ones.
    pub(crate) fn lineages_of<'a>(&'a self, w: &'a World, root: u32) -> impl Iterator<Item = (u32, &'a Lineage)> + 'a {
        let settled = w.life.brains.iter().zip(&w.life.members).enumerate().filter_map(|(k, (l, &m))| Some((k as u32, l.as_ref().filter(|_| m > 0)?)));
        let own = self.brains.iter().enumerate().map(|(k, l)| (provisional_id(self.index as usize, k) | if l.none() { NONE } else { 0 }, l));
        settled.chain(own).filter(move |(_, l)| l.root == root)
    }
    /// A new lineage, of no bodies yet: a provisional one of this domain.
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        let none = if l.none() { NONE } else { 0 };
        self.brains.push(l);
        provisional_id(self.index as usize, self.brains.len() - 1) | none
    }
    /// New born rows, not yet given to a body: provisional ones of this domain.
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        self.genomes.push(rows);
        provisional_id(self.index as usize, self.genomes.len() - 1)
    }
    /// The brain of `lineage`, provisional (of this domain) or not (none once it died out).
    pub(crate) fn brain<'a>(&'a self, w: &'a World, lineage: u32) -> Option<&'a Lineage> {
        provisional(lineage).map_or_else(|| w.life.brain(lineage), |(_, k)| Some(&self.brains[k]))
    }
    /// The brain shape of `lineage` (none once it died out).
    pub(crate) fn shape_of(&self, w: &World, lineage: u32) -> Option<Shape> {
        self.brain(w, lineage).map(|l| l.shape)
    }
    /// The starting kind body `b` descends from (`Lineage::root`); `u32::MAX` if its lineage is gone.
    pub(crate) fn root(&self, w: &World, b: &Body) -> u32 {
        self.brain(w, b.lineage).map_or(u32::MAX, |l| l.root)
    }
    /// The rows body `b` was born with (provisional, of this domain, or not).
    pub(crate) fn born<'a>(&'a self, w: &'a World, b: &Body) -> &'a [f32] {
        provisional(b.genome).map_or_else(|| w.life.born(b), |(_, k)| &self.genomes[k])
    }
    /// The rows of a body born with `born` from body `b`: its born rows plus what `b` learned, where the brains have one
    /// shape; else its born rows.
    pub(crate) fn inherit(&self, w: &World, born: &[f32], lineage: u32, b: &Body) -> Vec<f32> {
        let (from, rows) = (self.born(w, b), &b.rows);
        if self.shape_of(w, lineage) != self.shape_of(w, b.lineage) || born.len() != rows.len() || from.len() != rows.len() {
            return born.to_vec();
        }
        born.iter().zip(rows).zip(from).map(|((x, r), f)| x + (r - f)).collect()
    }
    /// Count body `b` into its lineage and its born rows.
    pub(super) fn enter(&mut self, b: &Body) {
        self.counted.push((Held::Lineage, b.lineage, 1));
        self.counted.push((Held::Rows, b.genome, 1));
    }
    /// Count body `b` out of its born rows and its lineage.
    pub(super) fn leave(&mut self, b: &Body) {
        self.counted.push((Held::Rows, b.genome, -1));
        self.counted.push((Held::Lineage, b.lineage, -1));
    }
    /// Body `b` (taken out of its slot) is now born with `rows` (its own rows are the caller's).
    pub(crate) fn reborn(&mut self, w: &World, b: &mut Body, rows: Vec<f32>) {
        if rows == self.born(w, b) {
            return;
        }
        let g = self.new_genome(rows);
        self.counted.push((Held::Rows, g, 1));
        self.counted.push((Held::Rows, b.genome, -1));
        b.genome = g;
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, rows: Vec<f32>) {
        self.leave(b);
        (b.lineage, b.genome) = (lineage, self.new_genome(rows));
        self.enter(b);
    }
}

/// The provisional id of the `k`-th lineage or born rows made in domain `d` since the world last settled.
fn provisional_id(d: usize, k: usize) -> u32 {
    assert!(k < 1 << PROV_SHIFT && d < 1 << (30 - PROV_SHIFT), "provisional id {k} of domain {d} out of its bits");
    PROVISIONAL | (d as u32) << PROV_SHIFT | k as u32
}

/// The domain and the place in its list of provisional id `x`; none for a global id.
fn provisional(x: u32) -> Option<(usize, usize)> {
    (x & PROVISIONAL != 0).then_some((((x & !(PROVISIONAL | NONE)) >> PROV_SHIFT) as usize, (x & ((1 << PROV_SHIFT) - 1)) as usize))
}

//! The lineages and the born rows of `Life`: their registries, the holders counted into them, and the provisional ids
//! of those made in the parallel phase of a batch, kept by their domain until the serial phase makes them global
//! (plan 3.3).

use super::{free_place, Life};
use crate::body::Body;
use crate::brain::Shape;
use crate::lineage::Lineage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The bit that marks a provisional id of a lineage or of born rows, made in the parallel phase of a batch (plan 3.3):
/// its domain lies above `PROV_SHIFT`, its place in that domain's list below.
const PROVISIONAL: u32 = 1 << 31;
const PROV_SHIFT: u32 = 16;

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
    /// A new lineage, of no bodies yet; in the parallel phase of a batch a provisional one of the domain running.
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        if let Some(d) = self.parallel {
            let x = &mut self.domains[d].brains;
            x.push(l);
            return provisional_id(d, x.len() - 1);
        }
        self.brains.push(Some(l));
        self.members.push(0);
        (self.brains.len() - 1) as u32
    }
    /// The brain of `lineage`, provisional or not (none once it died out).
    pub fn brain(&self, lineage: u32) -> Option<&Lineage> {
        match provisional(lineage) {
            Some((d, k)) => Some(&self.domains[d].brains[k]),
            None => self.brains.get(lineage as usize)?.as_ref(),
        }
    }
    /// The brain shape of `lineage` (none once it died out).
    pub(crate) fn shape_of(&self, lineage: u32) -> Option<Shape> {
        self.brain(lineage).map(|l| l.shape)
    }
    /// The rows of a body born with `born` from body `b`: its born rows plus what `b` learned, where the brains have one
    /// shape; else its born rows.
    pub(crate) fn inherit(&self, born: &[f32], lineage: u32, b: &Body) -> Vec<f32> {
        let (from, rows) = (self.born(b), &b.rows);
        if self.shape_of(lineage) != self.shape_of(b.lineage) || born.len() != rows.len() || from.len() != rows.len() {
            return born.to_vec();
        }
        born.iter().zip(rows).zip(from).map(|((x, r), f)| x + (r - f)).collect()
    }
    /// New born rows (not yet given to a body); in the parallel phase of a batch provisional ones of the domain running.
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        if let Some(d) = self.parallel {
            let x = &mut self.domains[d].genomes;
            x.push(rows);
            return provisional_id(d, x.len() - 1);
        }
        let i = free_place(&mut self.genomes, &mut self.free_genomes);
        self.genomes[i] = Genome { rows, refs: 0 };
        i as u32
    }
    /// Body `b` has no brain (`Lineage::none`).
    pub(crate) fn brainless(&self, b: &Body) -> bool {
        self.brain(b.lineage).is_some_and(|l| l.none())
    }
    /// The starting kind body `b` descends from (`Lineage::root`); `u32::MAX` if its lineage is gone.
    pub fn root(&self, b: &Body) -> u32 {
        self.brain(b.lineage).map_or(u32::MAX, |l| l.root)
    }
    /// The rows body `b` was born with (provisional or not).
    pub fn born(&self, b: &Body) -> &[f32] {
        provisional(b.genome).map_or_else(|| &self.genomes[b.genome as usize].rows, |(d, k)| &self.domains[d].genomes[k])
    }
    /// Count body `b` into its lineage and its born rows.
    pub(super) fn enter(&mut self, b: &Body) {
        self.count(b.cell, (false, b.lineage), 1);
        self.count(b.cell, (true, b.genome), 1);
    }
    /// Count body `b` out of its born rows and its lineage.
    pub(super) fn leave(&mut self, b: &Body) {
        self.count(b.cell, (true, b.genome), -1);
        self.count(b.cell, (false, b.lineage), -1);
    }
    /// The holders of lineage (`rows` false) or born rows (true) `x` change by `by`, for a body of cell `cell`. In a
    /// batch the change waits in that cell's domain until the batch ends (`settle`), so what its bodies hold or made
    /// stays readable to its end; outside, it is at once, and a lineage left with no body dies out, born rows with no
    /// holder are freed.
    fn count(&mut self, cell: u32, (rows, x): (bool, u32), by: i32) {
        if self.batch {
            return self.domains[self.cell_in[cell as usize].0 as usize].counted.push((rows, x, by));
        }
        let n = self.holders((rows, x));
        *n = n.wrapping_add_signed(by);
        if *n == 0 {
            self.gone((rows, x));
        }
    }
    /// The holders of lineage (`rows` false) or born rows (true) `x`.
    fn holders(&mut self, (rows, x): (bool, u32)) -> &mut u32 {
        if rows { &mut self.genomes[x as usize].refs } else { &mut self.members[x as usize] }
    }
    /// Lineage (`rows` false) or born rows (true) `x` has no holder: it dies out, or its rows are freed.
    fn gone(&mut self, (rows, x): (bool, u32)) {
        if rows {
            self.genomes[x as usize].rows = Vec::new();
            self.free_genomes.push(x);
        } else {
            self.brains[x as usize] = None;
        }
    }
    /// The serial phase of a batch (plan 3.3): the lineages and born rows made in its parallel phase get global ids,
    /// domain by domain in the order made, and the bodies, the outboxes and the changes of holders held by the domains
    /// name those.
    pub(crate) fn remap(&mut self) {
        let mut map: Vec<[Vec<u32>; 2]> = Vec::with_capacity(self.domains.len());
        for d in 0..self.domains.len() {
            let lin = std::mem::take(&mut self.domains[d].brains).into_iter().map(|l| self.new_lineage(l)).collect();
            let gen = std::mem::take(&mut self.domains[d].genomes).into_iter().map(|g| self.new_genome(g)).collect();
            map.push([lin, gen]);
        }
        let to = |x: u32, rows: bool| provisional(x).map_or(x, |(d, k)| map[d][rows as usize][k]);
        for d in &mut self.domains {
            d.bodies.iter_mut().for_each(|b| (b.lineage, b.genome) = (to(b.lineage, false), to(b.genome, true)));
            d.out.iter_mut().for_each(|e| e.remap(&to));
            d.counted.iter_mut().for_each(|c| c.1 = to(c.1, c.0));
        }
    }
    /// The end of a batch: the changes of holders are added, domain by domain in the order made; then, in the order
    /// of their ids, the lineages changed that have no body die out and the born rows changed with no holder are freed.
    pub(crate) fn settle(&mut self) {
        self.batch = false;
        let counted: Vec<(bool, u32, i32)> = self.domains.iter_mut().flat_map(|d| std::mem::take(&mut d.counted)).collect();
        let changed: BTreeSet<(bool, u32)> = counted.iter().map(|c| (c.0, c.1)).collect();
        for (rows, x, by) in counted {
            let n = self.holders((rows, x));
            *n = n.wrapping_add_signed(by);
        }
        for x in changed {
            if *self.holders(x) == 0 {
                self.gone(x);
            }
        }
    }
    /// Body `b` (taken out of its slot) is now born with `rows` (its own rows are the caller's).
    pub(crate) fn reborn(&mut self, b: &mut Body, rows: Vec<f32>) {
        if rows == self.born(b) {
            return;
        }
        let g = self.new_genome(rows);
        self.count(b.cell, (true, g), 1);
        self.count(b.cell, (true, b.genome), -1);
        b.genome = g;
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, rows: Vec<f32>) {
        self.leave(b);
        (b.lineage, b.genome) = (lineage, self.new_genome(rows));
        self.enter(b);
    }
}

/// The provisional id of the `k`-th lineage or born rows made in domain `d` in the batch running.
fn provisional_id(d: usize, k: usize) -> u32 {
    assert!(k < 1 << PROV_SHIFT && d < 1 << (31 - PROV_SHIFT), "provisional id {k} of domain {d} out of its bits");
    PROVISIONAL | (d as u32) << PROV_SHIFT | k as u32
}

/// The domain and the place in its list of provisional id `x`; none for a global id.
fn provisional(x: u32) -> Option<(usize, usize)> {
    (x & PROVISIONAL != 0).then_some((((x & !PROVISIONAL) >> PROV_SHIFT) as usize, (x & ((1 << PROV_SHIFT) - 1)) as usize))
}

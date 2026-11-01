//! The lineages and the born rows of `Life`: their registries, their holders counted from the bodies after each pass
//! (`Life::recount`), and the provisional ids of those body code makes, which the chunk keeps until the world names
//! them when the chunk ends (laws.md 0).

use super::{free_place, Chunk, Entry, Life};
use crate::body::Body;
use crate::brain::Shape;
use crate::lineage::Lineage;
use crate::World;
use serde::{Deserialize, Serialize};

/// The bit that marks a provisional id of a lineage or of born rows, made by body code in a chunk (laws.md 0): its place
/// in that chunk's list lies below `NONE`.
const PROVISIONAL: u32 = 1 << 31;
/// The bit of a provisional lineage's id that says it has no brain (`Lineage::none`), so `Life::brainless` reads it
/// through the world alone (`body::upkeep`) while its chunk runs.
const NONE: u32 = 1 << 30;

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
    /// The living lineages of starting kind `root`, with their ids, as the world held them when the pass began: the
    /// provisional ones of a chunk are not among them, so what a body chooses does not depend on who shares its chunk.
    pub(crate) fn lineages_of(&self, root: u32) -> impl Iterator<Item = (u32, &Lineage)> + '_ {
        let living = self.brains.iter().zip(&self.members).enumerate().filter_map(|(k, (l, &m))| Some((k as u32, l.as_ref().filter(|_| m > 0)?)));
        living.filter(move |(_, l)| l.root == root)
    }
    /// A new lineage, of no bodies yet (body code makes provisional ones, `Chunk::new_lineage`).
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        self.brains.push(Some(l));
        self.members.push(0);
        (self.brains.len() - 1) as u32
    }
    /// The brain of `lineage` (none once it died out; a provisional one is read through its chunk, `Chunk::brain`).
    pub fn brain(&self, lineage: u32) -> Option<&Lineage> {
        self.brains.get(lineage as usize)?.as_ref()
    }
    /// New born rows, not yet given to a body (body code makes provisional ones, `Chunk::new_genome`).
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        let i = free_place(&mut self.genomes, &mut self.free_genomes);
        self.genomes[i] = Genome { rows, refs: 0 };
        i as u32
    }
    /// Whether `lineage` has a brain (none once it died out); of a provisional one, from its id.
    pub(crate) fn has_brain(&self, lineage: u32) -> Option<bool> {
        provisional(lineage).map_or_else(|| self.brain(lineage).map(|l| !l.none()), |_| Some(lineage & NONE == 0))
    }
    /// Body `b` has no brain (`Lineage::none`).
    pub(crate) fn brainless(&self, b: &Body) -> bool {
        self.has_brain(b.lineage) == Some(false)
    }
    /// The rows body `b` was born with (provisional ones are read through their chunk, `Chunk::born`).
    pub fn born(&self, b: &Body) -> &[f32] {
        &self.genomes[b.genome as usize].rows
    }
    /// The holders of each lineage and born rows are counted from the living bodies; then, in the order of their ids,
    /// a lineage with none dies out and born rows with none are freed.
    pub(crate) fn recount(&mut self) {
        self.members.iter_mut().for_each(|m| *m = 0);
        self.genomes.iter_mut().for_each(|g| g.refs = 0);
        for b in self.cells.iter().flat_map(|c| &c.bodies).filter(|b| b.alive) {
            self.members[b.lineage as usize] += 1;
            self.genomes[b.genome as usize].refs += 1;
        }
        for (l, &m) in self.brains.iter_mut().zip(&self.members) {
            if m == 0 {
                *l = None;
            }
        }
        let mut free = vec![false; self.genomes.len()];
        self.free_genomes.iter().for_each(|&x| free[x as usize] = true);
        for (x, g) in self.genomes.iter_mut().enumerate().filter(|(x, g)| g.refs == 0 && !free[*x]) {
            g.rows = Vec::new();
            self.free_genomes.push(x as u32);
        }
    }
    /// The lineages and born rows a chunk of `cells` made get ids of the world, in the order made, and its bodies and
    /// the entries it queued (`out`) name those (laws.md 0).
    pub(super) fn name(&mut self, cells: std::ops::Range<usize>, (brains, genomes): (Vec<Lineage>, Vec<Vec<f32>>), out: &mut [Entry]) {
        let lin: Vec<u32> = brains.into_iter().map(|l| self.new_lineage(l)).collect();
        let gen: Vec<u32> = genomes.into_iter().map(|g| self.new_genome(g)).collect();
        let to = |x: u32, rows: bool| provisional(x).map_or(x, |k| if rows { gen[k] } else { lin[k] });
        let body = |b: &mut Body| (b.lineage, b.genome) = (to(b.lineage, false), to(b.genome, true));
        self.cells[cells].iter_mut().flat_map(|c| &mut c.bodies).for_each(body);
        for e in out {
            match e {
                Entry::Litter(x) | Entry::Back(x) => body(&mut x.c),
                Entry::Watch { lineage, .. } => *lineage = to(*lineage, false),
                _ => {}
            }
        }
    }
}

impl Chunk<'_> {
    /// A new lineage, of no bodies yet: a provisional one of this chunk.
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        let none = if l.none() { NONE } else { 0 };
        self.brains.push(l);
        provisional_id(self.brains.len() - 1) | none
    }
    /// New born rows, not yet given to a body: provisional ones of this chunk.
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        self.genomes.push(rows);
        provisional_id(self.genomes.len() - 1)
    }
    /// The brain of `lineage`, provisional (of this chunk) or not (none once it died out).
    pub(crate) fn brain<'a>(&'a self, w: &'a World, lineage: u32) -> Option<&'a Lineage> {
        provisional(lineage).map_or_else(|| w.life.brain(lineage), |k| Some(&self.brains[k]))
    }
    /// The brain shape of `lineage` (none once it died out).
    pub(crate) fn shape_of(&self, w: &World, lineage: u32) -> Option<Shape> {
        self.brain(w, lineage).map(|l| l.shape)
    }
    /// The starting kind body `b` descends from (`Lineage::root`); `u32::MAX` if its lineage is gone.
    pub(crate) fn root(&self, w: &World, b: &Body) -> u32 {
        self.brain(w, b.lineage).map_or(u32::MAX, |l| l.root)
    }
    /// The rows body `b` was born with (provisional, of this chunk, or not).
    pub(crate) fn born<'a>(&'a self, w: &'a World, b: &Body) -> &'a [f32] {
        provisional(b.genome).map_or_else(|| w.life.born(b), |k| &self.genomes[k])
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
    /// Body `b` (taken out of its slot) is now born with `rows` (its own rows are the caller's).
    pub(crate) fn reborn(&mut self, w: &World, b: &mut Body, rows: Vec<f32>) {
        if rows != self.born(w, b) {
            b.genome = self.new_genome(rows);
        }
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, rows: Vec<f32>) {
        (b.lineage, b.genome) = (lineage, self.new_genome(rows));
    }
}

/// The provisional id of the `k`-th lineage or born rows a chunk made.
fn provisional_id(k: usize) -> u32 {
    assert!(k < NONE as usize, "provisional id {k} out of its bits");
    PROVISIONAL | k as u32
}

/// The place in its chunk's list of provisional id `x`; none for an id of the world.
fn provisional(x: u32) -> Option<usize> {
    (x & PROVISIONAL != 0).then_some((x & !(PROVISIONAL | NONE)) as usize)
}

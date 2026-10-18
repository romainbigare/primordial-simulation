//! Lineages: what the bodies of a lineage share (`Lineage`), the brain shape the sizes of brain and senses give
//! (`shape`), and a body moving to a lineage of new sizes (`resize`).

use crate::body::{Body, BRAIN_SIZES, HIDDEN, MEMORY, SEEN, SIGHT};
use crate::brain::{self, Shape};
use crate::rng::{Purpose, Rng};
use crate::{bond, choose, World};
use serde::{Deserialize, Serialize};

/// What the bodies of a lineage share: the sizes of their brain and senses, the base of the brain (its hidden layer),
/// and where it came from.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lineage {
    pub shape: Shape,
    pub seen: usize,
    pub sight: usize,
    pub base: Vec<f32>,
    /// The starting kind this lineage descends from (inherited by every lineage made from it): a label for tallies
    pub root: u32,
}

impl Lineage {
    /// A brain of no hidden units is no brain: its bodies take no decisions and pay nothing for a brain; their update is
    /// their physics only (`survey::day`).
    pub fn none(&self) -> bool {
        self.shape.h == 0
    }
    /// Energy one decision takes, J per kg of a member of `Physics::m_ref` kg (`Physics::think`); none without a brain.
    /// It goes with the body's own size genes, not the lineage's rounded sizes, so a smaller brain pays less at once and
    /// selection can lead a node's sizes down, to none.
    pub fn cost(&self, p: &crate::Physics, genes: &[f32]) -> f64 {
        if self.none() {
            return 0.0;
        }
        let g = |k: usize| genes[k].max(0.0) as f64;
        p.c_hidden * g(HIDDEN) + p.c_memory * g(MEMORY) + p.c_seen * g(SEEN) + p.c_sight * g(SIGHT).powi(2)
    }
}


/// The brain's sizes for `ns` substances, `h` hidden units and `mem` memory numbers.
pub fn shape(ns: usize, h: usize, mem: usize) -> Shape {
    Shape { h, mem, ni: 2 * ns + choose::INPUTS + mem, nf: choose::NF, nt: choose::NT, no: bond::NO }
}

/// The sizes the genes give: hidden units, memory numbers, bodies perceived, sight (cells).
pub fn sizes(genes: &[f32]) -> [usize; 4] {
    BRAIN_SIZES.map(|g| genes[g].round().max(0.0) as usize)
}

/// The sizes of a lineage.
pub fn sizes_of(l: &Lineage) -> [usize; 4] {
    [l.shape.h, l.shape.mem, l.seen, l.sight]
}

/// A lineage with the sizes of brain and senses that `genes` ask for, if one of them is a whole unit away from lineage
/// `lin`'s (the others kept), its base carried over where the sizes overlap (`brain::rebase`, new weights of spread
/// `noise`).
pub(crate) fn resized(lin: &Lineage, genes: &[f32], ns: usize, r: &mut Rng, noise: f32) -> Option<Lineage> {
    let (old, want) = (sizes_of(lin), sizes(genes));
    let gene = BRAIN_SIZES.map(|g| genes[g]);
    let new: [usize; 4] = std::array::from_fn(|k| if (gene[k] - old[k] as f32).abs() >= 1.0 { want[k] } else { old[k] });
    let to = shape(ns, new[0], new[1]);
    (new != old).then(|| Lineage { shape: to, seen: new[2], sight: new[3], base: brain::rebase(&lin.base, lin.shape, to, r, noise), root: lin.root })
}

/// At its update, a body whose brain-size genes (a node's: their mean) are a whole unit away from its lineage's sizes
/// moves itself to a new lineage with those sizes (`resized`; its born rows by `brain::rerow`), its own rows carried
/// over with the same new weights (so what it learned goes with it; a body that had no brain starts from its new born
/// rows, one that has none after has no rows). An individual's child does this at birth (`copy::copy`); a node's
/// children start no lineage. Moves no matter and no energy.
pub fn resize(w: &mut World, slot: usize, b: &mut Body) {
    let (ns, noise, id) = (w.chem.ns(), w.cfg.new_noise(), b.id(slot));
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Resize);
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let Some(l) = resized(lin, &b.genes, ns, &mut r, noise) else { return };
    let from = lin.shape;
    // The born rows and its own get the same new weights (two streams of one key, the body's id turned over).
    let rerow = |x: &[f32]| brain::rerow(x, from, l.shape, &mut Rng::new(w.seed, w.day(), !id, Purpose::Resize), noise);
    let born = rerow(w.life.born(b));
    b.rows = if b.rows.is_empty() { born.clone() } else { rerow(&b.rows) };
    let lineage = w.life.new_lineage(l);
    w.life.move_to(b, lineage, born);
    // With no brain it has no decision to learn from, no memory and no signal.
    if w.life.brainless(b) {
        (b.trace, b.mem, b.sig, b.wb, b.var, b.ms) = (Vec::new(), Vec::new(), Default::default(), 0.0, 0.0, Vec::new());
    }
}

//! Lineages: what their bodies share (`Lineage`), the brain shape the size genes give, and moving to new sizes (`resize`).

use crate::body::{Body, BRAIN_SIZES, HIDDEN, MEMORY, SEEN, SIGHT};
use crate::brain::{self, Shape};
use crate::rng::{Purpose, Rng};
use crate::{bond, choose, life::Chunk, World};
use serde::{Deserialize, Serialize};

/// What the bodies of a lineage share: the sizes of brain and senses, the brain's base, and where it came from.
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
    /// A brain of no hidden units is no brain: its bodies take no decisions and pay for none.
    pub fn none(&self) -> bool {
        self.shape.h == 0
    }
    /// Energy one decision of a lineage with a brain (`brain`) or none takes, J per kg of a member of `m_ref` kg: by the
    /// body's own size genes, not the rounded sizes, so a smaller brain pays less at once and selection can lead a node's
    /// sizes down to none.
    pub fn cost(brain: bool, p: &crate::Physics, genes: &[f32]) -> f64 {
        if !brain {
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

/// The sizes `genes` ask of lineage `lin`: where a size gene is a whole unit away from `lin`'s size, the gene's; else
/// `lin`'s (all of `lin`'s: the genes fit it).
pub(crate) fn wanted(lin: &Lineage, genes: &[f32]) -> [usize; 4] {
    let (old, want) = (sizes_of(lin), sizes(genes));
    std::array::from_fn(|k| if (genes[BRAIN_SIZES[k]] - old[k] as f32).abs() >= 1.0 { want[k] } else { old[k] })
}

/// A lineage with the sizes `genes` ask for, if one is a whole unit away from `lin`'s, its base carried over
/// (`brain::rebase`).
pub(crate) fn resized(lin: &Lineage, genes: &[f32], ns: usize, r: &mut Rng, noise: f32) -> Option<Lineage> {
    let (old, new) = (sizes_of(lin), wanted(lin, genes));
    let to = shape(ns, new[0], new[1]);
    (new != old).then(|| Lineage { shape: to, seen: new[2], sight: new[3], base: brain::rebase(&lin.base, lin.shape, to, r, noise), root: lin.root })
}

/// Living lineages a starting kind keeps at most: past them a body that changes sizes joins the nearest (a speed
/// setting).
const MAX_LINEAGES: usize = 8;

/// At its update, a body whose brain-size genes are a whole unit away from its lineage's moves to a lineage of those
/// sizes, its born rows and its own rows carried over with the same new weights: a living one of its starting kind
/// with those sizes if there is one (it takes that one's base); else, if its kind has `MAX_LINEAGES` living lineages,
/// the one nearest in sizes (none if that is its own); else a new one. Living: as the world held them when the pass
/// began (`Life::lineages_of`), so two bodies that move to new sizes in one pass make two lineages. Moves no matter and
/// no energy.
pub fn resize(w: &World, d: &mut Chunk, b: &mut Body) {
    let (ns, noise, id) = (w.chem.ns(), w.cfg.new_noise(), b.id);
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Resize);
    let Some(lin) = d.brain(w, b.lineage) else { return };
    let Some(l) = resized(lin, &b.genes, ns, &mut r, noise) else { return };
    let from = lin.shape;
    // The living lineage of its kind nearest in sizes (the sum of the size differences; the first of equals), and how
    // many its kind has.
    let sizes = sizes_of(&l);
    let (mut count, mut near) = (0, None::<(u32, usize, brain::Shape)>);
    for (k, x) in w.life.lineages_of(l.root) {
        let gap = sizes_of(x).iter().zip(&sizes).map(|(a, b)| a.abs_diff(*b)).sum::<usize>();
        count += 1;
        if near.is_none_or(|n| gap < n.1) {
            near = Some((k, gap, x.shape));
        }
    }
    let to = near.filter(|n| n.1 == 0 || count >= MAX_LINEAGES);
    if to.is_some_and(|n| n.0 == b.lineage) {
        return;
    }
    let shape = to.map_or(l.shape, |n| n.2);
    // The born rows and its own get the same new weights (two streams of one key, the body's id turned over).
    let rerow = |x: &[f32]| brain::rerow(x, from, shape, &mut Rng::new(w.seed, w.day(), !id, Purpose::Resize), noise);
    let born = rerow(d.born(w, b));
    let rows = match d.rows(w, b) {
        [] => born.clone(),
        own => rerow(own),
    };
    // Its brain moves to a block of the new shape, with what of its learning and its last trace fits it.
    let brains = d.brains(b.cell as usize);
    let new = brains.alloc(shape, &rows);
    b.traced &= brains.carry(b.brain, from, new, shape);
    brains.free(std::mem::replace(&mut b.brain, new));
    let lineage = match to {
        Some(n) => n.0,
        None => d.new_lineage(l),
    };
    d.move_to(b, lineage, born);
    // With no brain it has no decision to learn from and no memory.
    if w.life.brainless(b) {
        d.brains(b.cell as usize).free(std::mem::take(&mut b.brain));
        (b.traced, b.mem, b.wb, b.var) = (false, Default::default(), 0.0, 0.0);
    }
}

//! The genes: what each scalar gene means, its step and range, where each kind starts (`Layout`), likeness (`alike`,
//! `within`) and mutation (`vary`). Laws.md 4.1.

use super::Body;
use crate::chem::Chemistry;
use crate::rng::Rng;
use std::f32::consts::{FRAC_PI_8, LOG10_2};

// Scalar genes, then one per reaction (a diet: on above 0, it burns or digests what that reaction takes) and three per
// substance (safe minimum as a share of mass; take selectivity; most share of mass kept): `Layout`. Below zero a gene
// is off.
/// Heat conductance through its coat, log10 W/(K kg) for a member of `Physics::m_ref` kg; with mass^(2/3)
pub const HEAT: usize = 0;
/// Strength, 0..1
pub const STRENGTH: usize = 1;
/// A child's body, log10 of its share of its parent's copy mass (`COPY_MASS`), at most `LOG_HALF`
pub const CHILD: usize = 2;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 3;
/// Mutation size, log10
pub const MUT: usize = 4;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 5;
/// Sizes of the brain and the senses (each rounded): hidden units (0..64; 0 is no brain), memory numbers (0..12),
/// bodies perceived (1..6), sight in cells (1..4) (`lineage::resize`).
pub const HIDDEN: usize = 6;
pub const MEMORY: usize = 7;
pub const SEEN: usize = 8;
pub const SIGHT: usize = 9;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 10;
/// Repair, log10: a body wears out at `Physics::e_life` x 10^gene per kg; each unit doubles the repair share of upkeep
pub const REPAIR: usize = 11;
/// The child's key liquid, log10 of the factor on its provisions: below 0, dry and nearly dormant children
pub const WET: usize = 12;
/// Share of what a mature body builds that goes to its brood (`Body::brood`), 0..1
pub const ALLOC: usize = 13;
/// The temperature below which the body burns on demand to keep warm, K (`live::keep_warm`)
pub const KEEP_T: usize = 14;
/// The temperature above which the body evaporates its key liquid to keep cool, K (`live::keep_warm`); between the two
/// it does neither
pub const COOL_T: usize = 15;
pub const NSCALAR: usize = 16;
/// A rate gene (`LEARN`) at or below this is off: the lower end of its range, -4, less a margin for rounding.
const GENE_OFF: f32 = -3.9;
/// The genes of the sizes of brain and senses.
pub const BRAIN_SIZES: [usize; 4] = [HIDDEN, MEMORY, SEEN, SIGHT];

/// log10 of one half: the largest child, a share of its parent's copy mass (`CHILD`).
pub(crate) const LOG_HALF: f32 = -LOG10_2;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums and uptake are off
/// below 0, with 20 steps of room there, so an off gene wanders without meeting a wall whose clamp would push it up.
pub(crate) fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        g if g < NSCALAR => SCALARS[g],
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 6.0),
    }
}

/// `range` of the scalar genes as a table.
const SCALARS: [(f32, f32, f32); NSCALAR] = {
    let (mut t, mut g) = ([(0.0, 0.0, 0.0); NSCALAR], 0);
    while g < NSCALAR {
        t[g] = scalar(g);
        g += 1;
    }
    t
};

const fn scalar(g: usize) -> (f32, f32, f32) {
    match g {
        HEAT => (0.3, -2.5, 3.5),
        STRENGTH => (0.1, 0.0, 1.0),
        CHILD => (0.1, -6.0, LOG_HALF),
        WET => (0.05, -3.0, 0.5),
        ALLOC => (0.05, 0.0, 1.0),
        COPY_MASS => (0.2, -3.0, 6.0),
        MUT => (0.2, -3.0, 0.0),
        LEARN => (0.2, -4.0, 0.0),
        // Down to no brain: below 0.5 it rounds to 0 hidden units; the room below 0 lets a node's mean reach it.
        HIDDEN => (0.3, -1.0, 64.0),
        MEMORY => (0.15, 0.0, 12.0),
        SEEN => (0.1, 1.0, 6.0),
        SIGHT => (0.07, 1.0, 4.0),
        DISPERSE => (0.05, 0.0, 1.0),
        REPAIR => (0.2, -3.0, 3.0),
        // Up to the warmest that real bodies keep (about 315 K), and far below the coldest ground.
        KEEP_T | COOL_T => (5.0, 200.0, 320.0),
        _ => (0.3, -6.0, 18.0),
    }
}

/// Where the genes of each kind start, for a chemistry.
#[derive(Clone, Copy)]
pub struct Layout {
    pub cat: usize,
    pub min: usize,
    pub upt: usize,
    pub keep: usize,
    pub len: usize,
}

impl Layout {
    pub fn of(chem: &Chemistry) -> Self {
        let (nr, ns) = (chem.reactions.len(), chem.ns());
        let cat = NSCALAR;
        Layout { cat, min: cat + nr, upt: cat + nr + ns, keep: cat + nr + 2 * ns, len: cat + nr + 3 * ns }
    }
}

impl Body {
    /// Learning rate of its brain within its life: 10 to the power of its gene, 0 when it is off (at the lower end of
    /// its range).
    pub fn learn_rate(&self) -> f32 {
        if self.genes[LEARN] <= GENE_OFF { 0.0 } else { 10f32.powf(self.genes[LEARN]) }
    }
    /// Scalar gene `g` kept within its range.
    pub(crate) fn gene_in_range(&self, g: usize) -> f64 {
        let (_, lo, hi) = SCALARS[g];
        self.gene(g).clamp(lo as f64, hi as f64)
    }
}

/// The step gene `g` is measured in for likeness: its mutation step, but a whole unit for the sizes of brain and senses
/// (in their small steps a split's halves would be unlike; not counted, small brains would join a big-brained kind).
fn like(lay: &Layout, g: usize) -> f32 {
    if (HIDDEN..=SIGHT).contains(&g) { 1.0 } else { range(lay, g).0 }
}

/// Gene `g` of value `x` in steps of `like` (a gene that is off counts as 0).
fn in_steps(lay: &Layout, g: usize, x: f32, like: f32) -> f32 {
    (if g < lay.cat { x } else { x.max(0.0) }) / like
}

/// Gene `g` of value `x` in steps of its own `like`.
fn step(lay: &Layout, g: usize, x: f32) -> f32 {
    in_steps(lay, g, x, like(lay, g))
}

/// Genes in steps of `like`.
pub fn steps(lay: &Layout, a: &[f32]) -> Vec<f32> {
    steps_by(lay, &likes(lay, a.len()), a)
}

/// `steps` with the step of each gene for likeness (`likes`) made once for many bodies.
pub(crate) fn steps_by(lay: &Layout, likes: &[f32], a: &[f32]) -> Vec<f32> {
    a.iter().zip(likes).enumerate().map(|(g, (&x, &like))| in_steps(lay, g, x, like)).collect()
}

/// The step for likeness of each of the first `n` genes (`like`).
pub(crate) fn likes(lay: &Layout, n: usize) -> Vec<f32> {
    (0..n).map(|g| like(lay, g)).collect()
}

/// The variance of gene `g` in a node's spread (0 past its end).
pub(crate) fn spread_at(spread: &[f32], g: usize) -> f32 {
    spread.get(g).map_or(0.0, |&v| v.max(0.0))
}

/// The variance `v` a node keeps along gene `g` of mean `x`: none for a gene that is off, where differences do nothing.
pub(crate) fn spread_kept(lay: &Layout, g: usize, x: f32, v: f32) -> f32 {
    if g >= lay.cat && x < 0.0 { 0.0 } else { v }
}

/// How far apart genes `a` and `b` are: the sum of their squared gaps in steps (`step`), if in every gene `g` the gap
/// is at most `reach(g)` steps; else none.
pub(crate) fn distance(lay: &Layout, a: &[f32], b: &[f32], reach: impl Fn(usize) -> f32) -> Option<f32> {
    let mut sum = 0.0;
    for (g, (&x, &y)) in a.iter().zip(b).enumerate() {
        let d = step(lay, g, x) - step(lay, g, y);
        if d.abs() <= reach(g) {
            sum += d * d;
        } else {
            return None;
        }
    }
    Some(sum)
}

/// Genes no more than `limit` steps apart in every gene: who mixes and which nodes merge (laws.md 4.1).
pub fn alike(lay: &Layout, limit: f32, a: &[f32], b: &[f32]) -> bool {
    distance(lay, a, b, |_| limit).is_some()
}

/// Reach of `noise` (a sum of four uniforms, centred and scaled): 2 sqrt(3).
pub(crate) const REACH: f32 = 3.464_101_6;
/// The number of values of a 16-bit uniform.
const U16_SPAN: f32 = 65536.0;

/// Noise near a standard normal, within `REACH`: a sum of four 16-bit uniforms from one draw, centred and scaled.
pub(crate) fn noise(r: &mut Rng) -> f32 {
    let u = r.u64();
    (((u & 0xffff) + (u >> 16 & 0xffff) + (u >> 32 & 0xffff) + (u >> 48)) as f32 - 2.0 * (U16_SPAN - 1.0)) * (3f32.sqrt() / U16_SPAN)
}

/// How far `x` lies inside the walls `lo` and `hi` (0 outside).
pub(crate) fn wall_room(x: f32, lo: f32, hi: f32) -> f32 {
    (x - lo).min(hi - x).max(0.0)
}

/// The mutation size of genes `genes`: 10 to the power of their `MUT` gene.
pub fn mutation_size(genes: &[f32]) -> f32 {
    10f32.powf(genes[MUT])
}

/// Most variance a node keeps along a gene (or its rows) of mutation step `step`: the split threshold, where its halves
/// (means 2 sqrt(2/π) standard deviations apart) are `alike` steps apart, plus one step². Without it nothing would take
/// variance out of a node that cannot split along the gene.
pub(crate) fn most(step: f32, alike: f32) -> f32 {
    step * step * (alike * alike * FRAC_PI_8 + 1.0)
}

/// The genes of `m` members drawn from a node of mean `genes` and variance `spread`, changed by a generation's
/// mutation: their mean (plus noise of that variance over `m`) and the variance left among them ((1 - 1/m) of it, up to
/// `most`). The spread's share of the noise shrinks near a wall, so the mean is never clamped: a clamp would be a drift
/// no selection made. A body with no `brain` does not mutate its brain-size genes (`BRAIN_SIZES`): they do nothing
/// without one, so a brain arises only in a lineage founded with one ("no brain where nothing is decided").
pub fn vary(lay: &Layout, genes: &[f32], spread: &[f32], m: f64, alike: f32, brain: bool, r: &mut Rng)
    -> (Vec<f32>, Vec<f32>) {
    let (size, m, mut left) = (mutation_size(genes), m.max(1.0) as f32, Vec::with_capacity(genes.len()));
    let genes = (genes.iter().enumerate())
        .map(|(g, &v)| {
            let (step, lo, hi) = range(lay, g);
            let held = !brain && BRAIN_SIZES.contains(&g);
            let (u2, s2) = (if held { 0.0 } else { (step * size).powi(2) }, spread_at(spread, g));
            let room = wall_room(v, lo, hi) / REACH;
            let x = (v + ((u2 + s2.min((room * room * m - u2).max(0.0))) / m).sqrt() * noise(r)).clamp(lo, hi);
            if m > 1.0 {
                left.push(spread_kept(lay, g, x, ((u2 + s2) * (1.0 - 1.0 / m)).min(most(step, alike))));
            }
            x
        })
        .collect();
    (genes, left)
}

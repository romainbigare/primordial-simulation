//! The genes: what each scalar gene means, its step and range, where each kind starts (`Layout`), likeness (`alike`,
//! `within`) and mutation (`vary`). Laws.md 4.1.

use super::Body;
use crate::chem::Chemistry;
use crate::rng::Rng;
use std::f32::consts::{FRAC_PI_8, LOG10_2};

// Scalar genes, then one per reaction (catalyst, log10 speed-up) and three per substance (safe minimum as a share of
// mass; take selectivity; most share of mass kept): `Layout`. Below zero a gene is off.
/// Gas exchange, log10 mol of air per m² of open surface (light-capture area and skin) per day: its pores' conductance
pub const GAS: usize = 0;
/// Liquid exchange with the ground, log10 per day
pub const LIQ: usize = 1;
/// Heat conductance through its coat, log10 W/(K kg) for a member of `Physics::m_ref` kg; with mass^(2/3)
pub const HEAT: usize = 2;
/// Light-capture area, log10 m² per kg-equivalent of its body (`Body::leaf_area`)
pub const LEAF: usize = 3;
/// Strength, 0..1
pub const STRENGTH: usize = 4;
/// A child's body, log10 of its share of its parent's copy mass (`COPY_MASS`), at most `LOG_HALF`
pub const CHILD: usize = 5;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 6;
/// Days between updates, log10 (kept between `MIN_INTERVAL` and `MAX_INTERVAL`)
pub const INTERVAL: usize = 7;
/// Mutation size, log10
pub const MUT: usize = 8;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 9;
/// Sizes of the brain and the senses (each rounded): hidden units (0..64; 0 is no brain), memory numbers (0..12),
/// bodies perceived (1..6), sight in cells (1..4) (`lineage::resize`).
pub const HIDDEN: usize = 10;
pub const MEMORY: usize = 11;
pub const SEEN: usize = 12;
pub const SIGHT: usize = 13;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 14;
/// Temperature above which its catalysts lose their shape, K (`change::denature`)
pub const DENATURE: usize = 15;
/// Repair, log10: a body wears out at `Physics::e_life` x 10^gene per kg; each unit doubles the repair share of upkeep
pub const REPAIR: usize = 16;
/// The child's key liquid, log10 of the factor on its provisions: below 0, dry and nearly dormant children
pub const WET: usize = 17;
/// Share of what a mature body builds that goes to its brood (`Body::brood`), 0..1
pub const ALLOC: usize = 18;
/// The temperature below which the body burns on demand to keep warm, K (`live::keep_warm`)
pub const KEEP_T: usize = 19;
/// The temperature above which the body evaporates to keep cool, K (`physics::evaporate`); between the two it does
/// neither
pub const COOL_T: usize = 20;
pub const NSCALAR: usize = 21;
/// A rate gene (`LEARN`) at or below this is off: the lower end of its range, -4, less a margin for rounding.
const GENE_OFF: f32 = -3.9;
/// The genes of the sizes of brain and senses.
pub const BRAIN_SIZES: [usize; 4] = [HIDDEN, MEMORY, SEEN, SIGHT];

/// Shortest (a speed setting) and longest interval between updates, days.
pub(crate) const MIN_INTERVAL: f64 = 5.0;
pub(crate) const MAX_INTERVAL: f64 = 360.0;
/// log10 of `MIN_INTERVAL` (log10 5 = 1 - log10 2) and of `MAX_INTERVAL` (log10 360): the range of the `INTERVAL` gene.
const LOG_MIN_INTERVAL: f32 = 1.0 - LOG10_2;
const LOG_MAX_INTERVAL: f32 = 2.556_302_5;

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
        GAS => (0.3, -1.0, 6.0),
        LIQ => (0.3, -4.0, 1.0),
        HEAT => (0.3, -2.5, 3.5),
        LEAF => (0.3, -4.0, 1.0),
        STRENGTH => (0.1, 0.0, 1.0),
        CHILD => (0.1, -6.0, LOG_HALF),
        WET => (0.05, -3.0, 0.5),
        ALLOC => (0.05, 0.0, 1.0),
        COPY_MASS => (0.2, -3.0, 6.0),
        INTERVAL => (0.2, LOG_MIN_INTERVAL, LOG_MAX_INTERVAL),
        MUT => (0.2, -3.0, 0.0),
        LEARN => (0.2, -4.0, 0.0),
        // Down to no brain: below 0.5 it rounds to 0 hidden units; the room below 0 lets a node's mean reach it.
        HIDDEN => (0.3, -1.0, 64.0),
        MEMORY => (0.15, 0.0, 12.0),
        SEEN => (0.1, 1.0, 6.0),
        SIGHT => (0.07, 1.0, 4.0),
        DISPERSE => (0.05, 0.0, 1.0),
        DENATURE => (5.0, 250.0, 450.0),
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

/// The highest speed-up (log10) of any reaction driven by light in the genes (0 if the chemistry has none).
pub fn light_cat(chem: &Chemistry, genes: &[f32]) -> f32 {
    chem.reactions.iter().enumerate().filter(|(_, rx)| rx.light).map(|(r, _)| genes[NSCALAR + r]).fold(0.0, f32::max)
}

/// A body whose light reactions are sped up at least this much (log10) is a light-eater, for the tape; no law reads it.
pub const LIGHT_EATER: f32 = 5.0;

/// Does a body of these genes live on light (`LIGHT_EATER`)?
pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    light_cat(chem, genes) >= LIGHT_EATER
}

/// The step gene `g` is measured in for likeness: its mutation step, but a whole unit for the sizes of brain and senses
/// (in their small steps a split's halves would be unlike; not counted, small brains would join a big-brained kind).
fn like(lay: &Layout, g: usize) -> f32 {
    if (HIDDEN..=SIGHT).contains(&g) { 1.0 } else { range(lay, g).0 }
}

/// Gene `g` of value `x` in steps of `like` (a gene that is off counts as 0).
fn step(lay: &Layout, g: usize, x: f32) -> f32 {
    (if g < lay.cat { x } else { x.max(0.0) }) / like(lay, g)
}

/// Genes in steps of `like`.
pub fn steps(lay: &Layout, a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(g, &x)| step(lay, g, x)).collect()
}

/// The variance of gene `g` in a node's spread (0 past its end).
pub(crate) fn spread_at(spread: &[f32], g: usize) -> f32 {
    spread.get(g).map_or(0.0, |&v| v.max(0.0))
}

/// The variance `v` a node keeps along gene `g` of mean `x`: none for a gene that is off, where differences do nothing.
pub(crate) fn spread_kept(lay: &Layout, g: usize, x: f32, v: f32) -> f32 {
    if g >= lay.cat && x < 0.0 { 0.0 } else { v }
}

/// A body's spread per gene (`spread_at`).
pub(crate) fn spreads(b: &Body) -> Vec<f32> {
    (0..b.genes.len()).map(|g| spread_at(&b.spread, g)).collect()
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

/// Standard deviations of the pooled spread, beyond `Physics::alike` steps, within which children join a node or nodes
/// merge: small, so a node never mixes genes that individuals of two kinds would not.
const WITHIN: f32 = 0.5;

/// The most steps gene `g` of two bodies may be apart for one to belong to the other: `limit` plus `WITHIN` standard
/// deviations of the sum of their variances `var`.
pub(crate) fn within_steps(lay: &Layout, limit: f32, var: f32, g: usize) -> f32 {
    limit + WITHIN * var.sqrt() / like(lay, g)
}

/// Do children of genes `genes` and spread `spread` belong to node `o` (`within_steps` in each gene alone, so kinds
/// that differ in a few genes stay apart)?
pub(crate) fn within(lay: &Layout, limit: f32, o: &Body, genes: &[f32], spread: &[f32]) -> bool {
    let reach = |g: usize| within_steps(lay, limit, spread_at(&o.spread, g) + spread_at(spread, g), g);
    distance(lay, &o.genes, genes, reach).is_some()
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
/// no selection made.
pub(crate) fn vary(lay: &Layout, genes: &[f32], spread: &[f32], m: f64, alike: f32, r: &mut Rng) -> (Vec<f32>, Vec<f32>) {
    let (size, m, mut left) = (mutation_size(genes), m.max(1.0) as f32, Vec::with_capacity(genes.len()));
    let genes = (genes.iter().enumerate())
        .map(|(g, &v)| {
            let (step, lo, hi) = range(lay, g);
            let (u2, s2) = ((step * size).powi(2), spread_at(spread, g));
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

/// Parent genes plus a generation's noise, kept in range.
pub fn mutate(lay: &Layout, genes: &[f32], r: &mut Rng) -> Vec<f32> {
    vary(lay, genes, &[], 1.0, 0.0, r).0
}

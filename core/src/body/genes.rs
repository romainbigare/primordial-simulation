//! The genes: what each scalar gene means, its mutation step and range, where the genes of each kind start
//! (`Layout`), how alike two sets of genes are (in steps, `alike`, `within`, size classes), and how a generation
//! changes them (`vary`).

use super::Body;
use crate::chem::Chemistry;
use crate::rng::Rng;
use std::f32::consts::{FRAC_PI_8, LOG10_2};

// Scalar genes, then one per reaction (catalyst, log10 speed-up) and three per substance (safe minimum as a share of
// mass; take selectivity; most share of mass kept): `Layout`. Below zero a gene is off.
/// Gas exchange, log10 mol of air per m² of the surface open to the air (`Body::open_area`) per day: the conductance
/// of its pores, in series with the boundary layer (`exchange::breath`). Its vapour leaves through them too.
pub const GAS: usize = 0;
/// Liquid exchange with the ground, log10 per day
pub const LIQ: usize = 1;
/// Heat conductance to the surface through the body's surface, log10 W/(K kg) for a member of
/// `Physics::m_ref` kg; it goes with mass^(2/3) (`Body::conductance`)
pub const HEAT: usize = 2;
/// Light-capture area, log10 m² per kg-equivalent of its body (`Body::leaf_area`)
pub const LEAF: usize = 3;
/// Strength, 0..1
pub const STRENGTH: usize = 4;
/// A child's body, log10 of its share of its parent's copy mass (`COPY_MASS`), at most `LOG_HALF`. A child is made
/// whenever the brood (`Body::brood`) covers its matter
pub const CHILD: usize = 5;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 6;
/// Days between updates, log10 (kept between `MIN_INTERVAL` and `MAX_INTERVAL`)
pub const INTERVAL: usize = 7;
/// Mutation size, log10
pub const MUT: usize = 8;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 9;
/// Share of what its group learned that fades each month, log10 (off at the lower end): the group forgets at the mean
/// rate of its members
pub(crate) const FORGET: usize = 10;
/// Sizes of the brain and the senses (each rounded): hidden units (0..64; 0 is no brain, `Lineage::none`), memory
/// numbers (0..12), bodies perceived (1..6), sight in cells (1..4). A body whose size genes are a whole unit away from
/// its lineage's sizes starts a new lineage with those sizes (`lineage::resize`).
pub const HIDDEN: usize = 11;
pub const MEMORY: usize = 12;
pub const SEEN: usize = 13;
pub const SIGHT: usize = 14;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 15;
/// Temperature above which its catalysts lose their shape, K (`change::denature`). A firmer catalyst costs more upkeep
/// (`Physics::t_firm`)
pub const DENATURE: usize = 16;
/// Repair, log10: a body wears out when the energy its upkeep turned over reaches `Physics::e_life` x 10^gene per kg;
/// each unit doubles the repair share `Physics::c_repair` of upkeep (`upkeep`)
pub const REPAIR: usize = 17;
/// The child's key liquid, log10 of the factor on its provisions (`copy::copy`): below 0 a body makes dry children,
/// nearly dormant (`Body::activity`) until they soak liquid up
pub const WET: usize = 18;
/// Share of what a mature body builds that goes to offspring (the brood, `Body::brood`), 0..1; the rest is its own
/// growth
pub const ALLOC: usize = 19;
/// The temperature the body keeps, K: when its update would leave it colder, it burns on demand the heat that holds it
/// there, as far as its reactions can release it (`live::keep_warm`)
pub const KEEP_T: usize = 20;
/// The temperature above which the body opens its surface to evaporation, K, as far as that holds it there
/// (`physics::evaporate`), the mirror of `KEEP_T`. Two levels: between them it neither burns nor evaporates, so a body
/// can keep no temperature
pub const COOL_T: usize = 21;
pub const NSCALAR: usize = 22;
/// A rate gene (`LEARN`, `FORGET`) at or below this is off: the lower end of its range, -4, less a margin for rounding.
const GENE_OFF: f32 = -3.9;
/// The genes of the sizes of brain and senses.
pub const BRAIN_SIZES: [usize; 4] = [HIDDEN, MEMORY, SEEN, SIGHT];

/// Shortest and longest interval between updates, days (the shortest is a speed safeguard).
pub(crate) const MIN_INTERVAL: f64 = 5.0;
pub(crate) const MAX_INTERVAL: f64 = 360.0;
/// log10 of `MIN_INTERVAL` (log10 5 = 1 - log10 2) and of `MAX_INTERVAL` (log10 360): the range of the `INTERVAL` gene.
const LOG_MIN_INTERVAL: f32 = 1.0 - LOG10_2;
const LOG_MAX_INTERVAL: f32 = 2.556_302_5;

/// log10 of one half: the largest child, a share of its parent's copy mass (`CHILD`).
pub(crate) const LOG_HALF: f32 = -LOG10_2;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums and uptake are off
/// below 0 (they cost and do nothing there), and their ranges go 20 steps below 0, so an off gene wanders as far as the
/// noise of a node's spread reaches (`most`) without meeting a wall, whose clamp would push it up.
pub(crate) fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        g if g < NSCALAR => SCALARS[g],
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 6.0),
    }
}

/// `range` of the scalar genes as a table (it is asked for every gene of every likeness test and child).
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
        FORGET => (0.2, -4.0, -1.7),
        // Down to no brain: below 0.5 it rounds to 0 hidden units; the room below 0 lets a node's mean reach it.
        HIDDEN => (0.3, -1.0, 64.0),
        MEMORY => (0.15, 0.0, 12.0),
        SEEN => (0.1, 1.0, 6.0),
        SIGHT => (0.07, 1.0, 4.0),
        DISPERSE => (0.05, 0.0, 1.0),
        DENATURE => (5.0, 250.0, 450.0),
        REPAIR => (0.2, -3.0, 3.0),
        // Up to the warmest that real bodies keep (about 315 K), and far enough below the coldest ground for an unused level
        // to wander.
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
    /// Learning rate of its brain within its life (0 when off).
    pub fn learn_rate(&self) -> f32 {
        self.rate_gene(LEARN)
    }
    /// Share of what its group learned that it would let fade each month (0 when off).
    pub(crate) fn forget(&self) -> f32 {
        self.rate_gene(FORGET)
    }
    /// 10 to the power of gene `g`, 0 when it is off (at the lower end of its range).
    fn rate_gene(&self, g: usize) -> f32 {
        if self.genes[g] <= GENE_OFF { 0.0 } else { 10f32.powf(self.genes[g]) }
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

/// A body whose light reactions are sped up at least this much (log10) is a light-eater; the others are eaters. The tape
/// and the reports count by this; no law reads it.
pub const LIGHT_EATER: f32 = 5.0;

/// Does a body of these genes live on light (`LIGHT_EATER`)?
pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    light_cat(chem, genes) >= LIGHT_EATER
}

/// The step gene `g` is measured in for likeness: its mutation step, but a whole unit for the sizes of brain and senses.
/// In their mutation steps (0.07 to 0.3) a split's halves (1.3 units apart) would be unlike; not counted at all, the
/// children of small brains would join a big-brained kind and average its brain down.
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

/// Genes no more than `limit` steps apart in every gene (`distance`). Nodes merge each month only if their means are
/// alike (`nodes::crowd`), and bodies mix their genes only if they are (a partner, `copy::copy`).
pub fn alike(lay: &Layout, limit: f32, a: &[f32], b: &[f32]) -> bool {
    distance(lay, a, b, |_| limit).is_some()
}

/// Standard deviations of the pooled spread, beyond `Physics::alike` steps, within which children join a node, or the
/// nodes of a full cell merge (`within`, `nodes::crowd`). Kept small so kinds stay apart: a node only saves time, so it
/// must not mix genes that individuals of two kinds would never mix.
const WITHIN: f32 = 0.5;

/// The most steps gene `g` of two bodies may be apart for one to belong to the other (`within`): `limit`, plus `WITHIN`
/// standard deviations of their spreads together (`var`, the sum of their variances).
pub(crate) fn within_steps(lay: &Layout, limit: f32, var: f32, g: usize) -> f32 {
    limit + WITHIN * var.sqrt() / like(lay, g)
}

/// Children `c` (one, or a group drawn from a node with the spread left among them) belong to node `o`: in each gene
/// their mean is within `limit` steps of its mean plus `WITHIN` standard deviations of both spreads together. Each
/// gene alone, so kinds that differ in a few genes stay apart; the spreads count, as a few children drawn from a node
/// differ from its mean by its spread over their number.
pub(crate) fn within(lay: &Layout, limit: f32, o: &Body, c: &Body) -> bool {
    let reach = |g: usize| within_steps(lay, limit, spread_at(&o.spread, g) + spread_at(&c.spread, g), g);
    distance(lay, &o.genes, &c.genes, reach).is_some()
}

/// Bodies are compared in log10 of their mass from this many kg up.
const LEAST_KG: f64 = 1e-30;
/// Slack of a size-class bound for rounding, decades.
const CLASS_SLACK: f64 = 1e-9;

/// The step of the copy-mass gene that sizes are alike in (`alike_size`).
pub(crate) fn size_step(lay: &Layout) -> f32 {
    like(lay, COPY_MASS)
}

/// A body's size class: log10 of its member's body (`Body::member_body`) and whether it is mature (`Body::mature`).
/// Two bodies are in one class when they are alike in size (`alike_log`) and both mature or both not: a node's mean
/// must stay at or past its copy mass for its members to reproduce, so the young join and merge with the young until
/// they have grown.
pub fn class(chem: &Chemistry, b: &Body) -> (f64, bool) {
    let x = b.member_body(chem);
    (x.max(LEAST_KG).log10(), b.mature(x))
}

/// Two member bodies (kg) are alike in size: within `alike` steps of the copy-mass gene of each other in log10 (0.2
/// decades a step: with `alike` 4 about a factor 6.3).
pub fn alike_size(lay: &Layout, alike: f32, x: f64, y: f64) -> bool {
    alike_log(lay, alike, x.max(LEAST_KG).log10(), y.max(LEAST_KG).log10())
}

/// `alike_size` of two bodies given as log10 (`class`).
pub fn alike_log(lay: &Layout, alike: f32, x: f64, y: f64) -> bool {
    (x - y).abs() <= (alike * size_step(lay)) as f64 + CLASS_SLACK
}

/// Reach of `noise` (a sum of four uniforms, centred and scaled): 2 sqrt(3).
pub(crate) const REACH: f32 = 3.464_101_6;
/// The number of values of a 16-bit uniform.
const U16_SPAN: f32 = 65536.0;

/// Noise near a standard normal: a sum of four 16-bit uniforms from one draw (mean 2 (`U16_SPAN` - 1), variance about
/// `U16_SPAN`²/3), centred and scaled; within `REACH`. One draw instead of four, as a node's dispersing children draw
/// it for every gene.
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

/// Most variance a node keeps along a gene (or its rows) of mutation step `step`, in its units squared: the split
/// threshold, where its halves (means `2 sqrt(2/π)` standard deviations apart) are `alike` steps apart,
/// `(alike step)² π / 8`, plus one step². A node held there is past it and splits (`nodes::split`), while one that
/// cannot split along it (a gene that is off, or near a wall) widens no further. Nothing else takes variance out of a
/// node, so without it the spread would grow with every generation and merge.
pub(crate) fn most(step: f32, alike: f32) -> f32 {
    step * step * (alike * alike * FRAC_PI_8 + 1.0)
}

/// The genes of `m` members drawn from a node of mean `genes` and variance `spread` per gene (empty: none), each changed
/// by a generation's mutation: their mean (the node's, plus noise of that variance over `m`), and the variance left
/// among them ((1 - 1/m) of it, up to `most` for `alike`; none for one). The spread's share of the noise is shrunk
/// where it could cross a wall, so the mean drawn is never clamped: a clamp would move the mean of a node's children
/// away from the wall, a drift no selection made. A generation's own change may still be clamped (one small step).
pub(crate) fn vary(lay: &Layout, genes: &[f32], spread: &[f32], m: f64, alike: f32, r: &mut Rng) -> (Vec<f32>, Vec<f32>) {
    let (size, m, mut left) = (mutation_size(genes), m.max(1.0) as f32, Vec::with_capacity(genes.len()));
    let genes = (genes.iter().enumerate())
        .map(|(g, &v)| {
            let (step, lo, hi) = range(lay, g);
            let (u2, s2) = ((step * size).powi(2), spread_at(spread, g));
            if m > 1.0 {
                left.push(((u2 + s2) * (1.0 - 1.0 / m)).min(most(step, alike)));
            }
            let room = wall_room(v, lo, hi) / REACH;
            let s2 = s2.min((room * room * m - u2).max(0.0));
            (v + ((u2 + s2) / m).sqrt() * noise(r)).clamp(lo, hi)
        })
        .collect();
    (genes, left)
}

/// Parent genes plus a generation's noise, kept in range.
pub fn mutate(lay: &Layout, genes: &[f32], r: &mut Rng) -> Vec<f32> {
    vary(lay, genes, &[], 1.0, 0.0, r).0
}

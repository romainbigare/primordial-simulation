//! The genes: what each scalar gene means, its mutation step and range, and where the genes of each kind start.

use super::Body;
use crate::chem::Chemistry;
use std::f32::consts::LOG10_2;

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
/// its lineage's sizes starts a new lineage with those sizes (`copy::resize`).
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

/// log10 of one half: the largest child, a share of its parent's copy mass (`CHILD`).
pub(crate) const LOG_HALF: f32 = -LOG10_2;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums and uptake are off
/// below 0 (they cost and do nothing there), and their ranges go 20 steps below 0, so an off gene wanders as far as the
/// noise of a node's spread reaches (`copy::most`) without meeting a wall, whose clamp would push it up.
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
        // (log10 of `MIN_INTERVAL` and `MAX_INTERVAL`, rounded.)
        INTERVAL => (0.2, 0.7, 2.556),
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
}

/// The highest speed-up (log10) of any reaction driven by light in the genes (0 if the chemistry has none).
pub fn light_cat(chem: &Chemistry, genes: &[f32]) -> f32 {
    chem.reactions.iter().enumerate().filter(|(_, rx)| rx.light).map(|(r, _)| genes[NSCALAR + r]).fold(0.0, f32::max)
}

/// A body whose light reactions are sped up at least this much (log10) is a light-eater; the others are eaters. The tape
/// counts by this, and so does `Config::nodes`.
pub const LIGHT_EATER: f32 = 5.0;

/// Does a body of these genes live on light (`LIGHT_EATER`)?
pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    light_cat(chem, genes) >= LIGHT_EATER
}

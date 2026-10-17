//! The genes: where each kind starts, and what the scalar ones mean.

use super::Body;
use crate::chem::Chemistry;
use std::f32::consts::LOG10_2;

// Scalar genes, then one per reaction (catalyst, log10 speed-up) and three per substance
// (safe minimum as a share of mass; take selectivity; most share of mass kept). Below zero a
// gene is off.
/// Gas exchange, log10 mol of air per m² of the surface open to the air (`Body::open_area`) per
/// day: the conductance of its pores (a leaf's stomata), which open as far as it is wet and are in series with the
/// boundary layer (`breath`); its vapour leaves through them too. Through the surface, capture area pays twice (light and gas), so a plant that grows its
/// area gains on both and a taller plant that shades the rest gains more.
pub const GAS: usize = 0;
/// Liquid exchange with the ground, log10 per day
pub const LIQ: usize = 1;
/// Heat conductance to the surface through the body's surface, log10 W/(K kg) for a member of
/// `Physics::m_ref` kg; it goes with mass^(2/3) (`Body::conductance`)
pub const HEAT: usize = 2;
/// Light-capture area, log10 m² per kg-equivalent of its body (`Body::size`, `Body::equiv`:
/// mass^0.75 from `Physics::m_ref`)
pub const LEAF: usize = 3;
/// Strength, 0..1
pub const STRENGTH: usize = 4;
/// A child's body, log10 of its share of its parent's copy mass (`COPY_MASS`): seeds and eggs at
/// -6, calves at -0.3. A child is made whenever the brood (`Body::brood`) covers its matter
pub const CHILD: usize = 5;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 6;
/// Days between updates, log10 (kept between 5 and 360 days)
pub const INTERVAL: usize = 7;
/// Mutation size, log10
pub const MUT: usize = 8;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 9;
/// Share of what its group learned that fades each month, log10 (off at the lower end): the
/// group forgets at the mean rate of its members
pub(crate) const FORGET: usize = 10;
/// Sizes of the brain and the senses (each rounded): hidden units (0..64; 0 is no brain, see
/// `Lineage::none`), memory numbers (0..12), bodies perceived (1..6), sight in cells (1..4). A
/// body whose size genes are a whole unit away from its lineage's sizes starts a new lineage with
/// those sizes.
pub const HIDDEN: usize = 11;
pub const MEMORY: usize = 12;
pub const SEEN: usize = 13;
pub const SIGHT: usize = 14;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 15;
/// Temperature above which its catalysts lose their shape, K: their speed-up falls off within a
/// few K (`change::denature`). A firmer catalyst costs more upkeep (`Physics::t_firm`)
pub const DENATURE: usize = 16;
/// Repair, log10: a body wears out when the energy its upkeep turned over reaches
/// `Physics::e_life` x 10^gene per kg; each unit doubles the repair share `Physics::c_repair` of upkeep (`upkeep`)
pub const REPAIR: usize = 17;
/// The child's share of the key liquid, log10 of the factor on its provisions: a child gets
/// provisions of every store but body matter in the parent's proportion to its matter, and of the
/// key liquid 10^`WET` times that (at most what the parent holds), so a body can make dry children
/// (below 0) that are nearly dormant (`Body::activity`) and wait until they soak liquid up
pub const WET: usize = 18;
/// Share of what a mature body builds in an update that goes to offspring (the brood), 0..1; the rest
/// is its own growth. With 1 it stops growing at maturity
pub const ALLOC: usize = 19;
/// The temperature the body keeps, K, as it keeps levels of its stores: when its update would leave it colder, it burns on
/// demand, beside its upkeep, the heat that holds it there, as far as its reactions can release it (`live`). Above it,
/// nothing more: it cools by physics. Below its surroundings it does nothing
pub const KEEP_T: usize = 20;
/// The temperature above which the body opens its surface to evaporation (pores, sweat, panting), K, as far as that holds it
/// there (`live`), the mirror of `KEEP_T`. Two levels, not one: between them it neither burns nor evaporates, as a body that
/// keeps no temperature must be able to (with one level every body would always burn or always evaporate)
pub const COOL_T: usize = 21;
pub const NSCALAR: usize = 22;
/// The genes of the sizes of brain and senses.
pub const BRAIN_SIZES: [usize; 4] = [HIDDEN, MEMORY, SEEN, SIGHT];

/// Shortest and longest interval between updates, days (the shortest is a speed safeguard; the
/// longest is one year, so a body that has nothing to decide can update once a year).
pub(crate) const MIN_INTERVAL: f64 = 5.0;
pub(crate) const MAX_INTERVAL: f64 = 360.0;

/// log10 of one half: the largest child, a share of its parent's copy mass (`CHILD`).
pub(crate) const LOG_HALF: f32 = -LOG10_2;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums
/// and uptake are off below 0 (they cost nothing and do nothing there), and their ranges go 20
/// steps below 0, so an off gene wanders as far as the noise of a node's spread reaches (about 10
/// steps, `most`) without meeting a wall, whose clamp would push it up. A catalyst
/// step is 0.3 decades, so nodes alike within `phys.alike` steps differ by a factor 16 at most.
pub(crate) fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        g if g < NSCALAR => SCALARS[g],
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 6.0),
    }
}

/// `range` of the scalar genes as a table: it is asked a hundred times in each likeness test and
/// each draw of children, and a look-up is cheaper than the match.
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
        INTERVAL => (0.2, 0.7, 2.556),
        MUT => (0.2, -3.0, 0.0),
        LEARN => (0.2, -4.0, 0.0),
        FORGET => (0.2, -4.0, -1.7),
        // Down to no brain: below 0.5 it rounds to 0 hidden units; the room below 0 lets a node's
        // mean reach it without meeting the wall.
        HIDDEN => (0.3, -1.0, 64.0),
        MEMORY => (0.15, 0.0, 12.0),
        SEEN => (0.1, 1.0, 6.0),
        SIGHT => (0.07, 1.0, 4.0),
        DISPERSE => (0.05, 0.0, 1.0),
        DENATURE => (5.0, 250.0, 450.0),
        REPAIR => (0.2, -3.0, 3.0),
        // Up to the warmest real bodies keep (birds about 315 K) and down far enough below the coldest ground for an unused
        // level to wander without meeting the wall.
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
        if self.genes[LEARN] <= -3.9 { 0.0 } else { 10f32.powf(self.genes[LEARN]) }
    }
    /// Share of what its group learned that it would let fade each month (0 when off).
    pub(crate) fn forget(&self) -> f32 {
        if self.genes[FORGET] <= -3.9 { 0.0 } else { 10f32.powf(self.genes[FORGET]) }
    }
}

/// The highest speed-up (log10) of any reaction driven by light in the genes (a raw number; 0
/// if the chemistry has none).
pub fn light_cat(chem: &Chemistry, genes: &[f32]) -> f32 {
    chem.reactions.iter().enumerate().filter(|(_, rx)| rx.light).map(|(r, _)| genes[NSCALAR + r]).fold(0.0, f32::max)
}

/// A body whose light reactions are sped up at least this much (log10) is a light-eater. The
/// tape counts by this, and so does `Config::nodes`: the others are the animals.
pub const LIGHT_EATER: f32 = 5.0;

/// Does a body of these genes live on light (the rest are animals, `Config::nodes`)?
pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    light_cat(chem, genes) >= LIGHT_EATER
}

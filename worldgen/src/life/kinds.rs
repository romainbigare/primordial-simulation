//! The table of starting kinds: the genes each starts with, and where, when and how many are placed.

use pcore::body::{self, COOL_T, COPY_MASS, DISPERSE, GAS, HIDDEN, INTERVAL, KEEP_T, LEAF, LEARN, LIQ, MEMORY, NSCALAR, REPAIR, SEEN, SIGHT, STRENGTH};
use pcore::brain;

/// One row of the table of starting kinds: the genes it starts with and where and how many are placed.
pub struct Kind {
    /// What the seeding line calls it (physical terms only)
    pub name: &'static str,
    /// Years after the first life that it is placed: light-eaters first, the others once what they take has spread
    pub year: u64,
    /// Placed on any land cell (else only where bodies already are)
    pub(super) anywhere: bool,
    pub(super) scalars: [f32; NSCALAR],
    /// log10 speed-up of the light reaction and of the burning of the energy-rich solid
    pub(super) cat: [f32; 2],
    /// Uptake selectivity of the liquid and the energy-rich solid
    pub(super) upt: [f32; 2],
    /// Cover of a node, kg/m² (0: individuals); bodies placed per cell; chance per land cell
    pub(super) cover: f64,
    pub(super) per: usize,
    pub(super) chance: f64,
    /// Share of a member's mass placed as the energy-rich solid, its fuel at the start (the rest is its body, `split`)
    pub(super) store: f64,
    /// Hand-set brain biases (`Seeding::hand`; none for a kind without a brain)
    pub(super) hand: Option<Hand>,
}

/// Hand-set brain biases: the fixed actions (`brain::FIXED`), moving, then the verbs toward a body and a part.
pub(super) type Hand = ([f32; brain::FIXED], f32, [f32; brain::VERBS + brain::OVERBS]);
const H_EATER: Hand = ([0.0, -3.0, 0.5, -2.5, -2.0, -2.5, -3.0], -1.0, [2.0, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0]);

/// `s` with the genes of `d` (gene, value) set.
const fn with(mut s: [f32; NSCALAR], d: &[(usize, f32)]) -> [f32; NSCALAR] {
    let mut i = 0;
    while i < d.len() {
        s[d[i].0] = d[i].1;
        i += 1;
    }
    s
}

/// The genes every kind starts with unless its row says otherwise: HEAT 3.5 (no coat), CHILD -0.523 (a child 0.3 of
/// the copy mass), MUT -1.2, HIDDEN 12, MEMORY 6, SEEN 3, SIGHT 1, DENATURE 320 K (about where enzymes fail), ALLOC
/// 0.8, KEEP_T 250 K and COOL_T 320 K (outside what the island has: they do nothing at the start).
const BASE: [f32; NSCALAR] = [0.0, 0.0, 3.5, 0.0, 0.0, -0.523, 0.0, 0.0, -1.2, 0.0, 12.0, 6.0, 3.0, 1.0, 0.0, 320.0, 0.0, 0.0, 0.8, 250.0, 320.0];

/// Light-eaters: strong light reaction, slow burning, a leaf's open pores (`GAS` 4.334, about 0.25 mol/(m² s): Körner,
/// Scheel and Bauer 1979), long intervals, no brain (what they do is their body's physics); one node per land cell.
const LIGHT: Kind = Kind {
    name: "light-eaters",
    year: 0,
    anywhere: true,
    scalars: with(BASE, &[(GAS, 4.334), (LEAF, 0.416), (STRENGTH, 0.05), (COPY_MASS, 0.145), (INTERVAL, 1.3), (LEARN, -4.0), (DISPERSE, 0.1), (REPAIR, -1.0), (HIDDEN, 0.0), (MEMORY, 0.0)]),
    cat: [9.0, 2.0],
    upt: [0.0, 0.0],
    cover: 0.1,
    per: 1,
    chance: 1.0,
    store: 0.3,
    hand: None,
};

/// The temperatures the eaters' founders keep their bodies between, K: the melting point of chemistry 0's key liquid
/// (273 K) plus, and their catalysts' unfolding (`DENATURE`, 320 K) less, the margin within which a body's well-being
/// falls for nearness to a limit (5 K, `body::PHASE_MARGIN`): their liquid does not freeze in a winter on the hills, and
/// they sweat before their catalysts unfold in a summer; light-eaters keep none (`BASE`).
const EATER_KEEP_T: f32 = 278.0;
const EATER_COOL_T: f32 = 315.0;

/// Eaters: fast burning, take the energy-rich solid from bodies and the ground, short intervals, learning on, keep
/// warm and cool (`EATER_KEEP_T`, `EATER_COOL_T`); individuals, two in some cells where bodies already are.
const EATER: Kind = Kind {
    name: "eaters",
    year: 20,
    anywhere: false,
    scalars: with(BASE, &[(GAS, 2.304), (LIQ, -0.5), (LEAF, -2.884), (STRENGTH, 0.6), (COPY_MASS, 1.645), (INTERVAL, 1.0), (LEARN, -1.0), (DISPERSE, 0.1), (REPAIR, -0.4), (KEEP_T, EATER_KEEP_T), (COOL_T, EATER_COOL_T)]),
    cat: [-1.0, 2.6],
    upt: [0.3, 1.0],
    cover: 0.0,
    per: 2,
    chance: 0.3,
    store: 0.3,
    hand: Some(H_EATER),
};

/// Starting kinds; the row is the root label of the lineages a kind starts (the tape's `root_<k>_*` columns). Rows 0
/// and 1 are the two kinds of `ew run`, rows 2 to 9 with them the roster. A row sets only what the laws read (genes,
/// catalysts, uptake, and where, when and how many): kinds are initial conditions, never rules. The roster's rungs
/// scale the first two rows; eaters of bodies are stronger than what they eat, eaters of remains have no strength, and
/// the two big slow eater kinds start with 60% of their mass as fuel, not 30%, as a neutral brain must find what to take
/// within the few decisions its store lasts.
pub const KINDS: [Kind; 10] = [
    LIGHT,
    EATER,
    // Light-eaters, small and fast, children spread wide; big and slow, less capture area per kg, children stay.
    Kind { name: "small fast light-eaters", scalars: with(LIGHT.scalars, &[(LEAF, 0.716), (COPY_MASS, -1.155), (INTERVAL, 1.0), (DISPERSE, 0.3), (REPAIR, -1.2)]), cover: 0.05, chance: 0.6, ..LIGHT },
    Kind { name: "big slow light-eaters", scalars: with(LIGHT.scalars, &[(LEAF, -0.184), (COPY_MASS, 1.445), (INTERVAL, 1.78), (DISPERSE, 0.05), (REPAIR, -0.5)]), cover: 0.3, chance: 0.4, ..LIGHT },
    // Eaters of remains on the ground, small and big: no strength, the solid first.
    Kind { name: "small eaters of remains", year: 10, scalars: with(EATER.scalars, &[(GAS, 1.815), (STRENGTH, 0.0), (COPY_MASS, 0.245), (LEARN, -1.5), (DISPERSE, 0.3), (REPAIR, -1.48)]), upt: [0.2, 1.0], ..EATER },
    Kind { name: "big eaters of remains", year: 10, scalars: with(EATER.scalars, &[(GAS, 2.205), (STRENGTH, 0.0), (COPY_MASS, 1.325), (INTERVAL, 1.3), (LEARN, -1.5), (DISPERSE, 0.2), (REPAIR, -0.89)]), per: 1, chance: 0.2, ..EATER },
    // Eaters of light-eaters, small and fast, and big and slow.
    Kind { name: "small fast eaters of light-eaters", scalars: with(EATER.scalars, &[(GAS, 2.273), (STRENGTH, 0.3), (COPY_MASS, 0.625), (INTERVAL, 0.85), (DISPERSE, 0.2), (REPAIR, -1.06)]), ..EATER },
    Kind { name: "big slow eaters of light-eaters", scalars: with(EATER.scalars, &[(GAS, 2.536), (LEAF, -2.702), (STRENGTH, 0.5), (COPY_MASS, 2.082), (INTERVAL, 1.2), (DISPERSE, 0.05), (REPAIR, -0.2)]), per: 1, chance: 0.15, store: 0.6, ..EATER },
    // Eaters of eaters, placed once the eaters have spread, with a bigger brain (16 hidden, 8 memory, 4 seen, sight 2).
    Kind { name: "strong fast eaters of eaters", year: 24, scalars: with(EATER.scalars, &[(GAS, 2.478), (COPY_MASS, 2.145), (INTERVAL, 0.85), (LEARN, -0.7), (HIDDEN, 16.0), (MEMORY, 8.0), (SEEN, 4.0), (SIGHT, 2.0), (REPAIR, -0.5)]), per: 1, chance: 0.15, ..EATER },
    Kind { name: "strong big slow eaters of eaters", year: 24, scalars: with(EATER.scalars, &[(GAS, 2.736), (LEAF, -2.702), (STRENGTH, 0.5), (COPY_MASS, 2.682), (INTERVAL, 1.15), (LEARN, -0.7), (HIDDEN, 16.0), (MEMORY, 8.0), (SEEN, 4.0), (SIGHT, 2.0), (DISPERSE, 0.05), (REPAIR, 0.0)]), per: 1, chance: 0.15, store: 0.6, ..EATER },
];

/// The roster of `ew run --roster`: the ten rows of `KINDS`, each placed at its `year`.
pub const ROSTER: [usize; 10] = [2, 0, 3, 4, 5, 6, 1, 7, 8, 9];

/// Whether a kind is a light-eater (its light catalyst at or above `body::LIGHT_EATER`).
pub(super) fn light(kind: &Kind) -> bool {
    kind.cat[0] >= body::LIGHT_EATER
}

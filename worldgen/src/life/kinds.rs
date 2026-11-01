//! The table of starting kinds: the genes each starts with, and where, when and how many are placed.

use pcore::body::{COOL_T, COPY_MASS, DISPERSE, GAS, HIDDEN, INTERVAL, KEEP_T, LEAF, LEARN, LIQ, MEMORY, NSCALAR, REPAIR, SEEN, SIGHT, STRENGTH};
use pcore::{brain, cover};

/// One row of the table of starting kinds: its name, when it is placed, and what is placed.
pub struct Kind {
    /// What the seeding line calls it (physical terms only)
    pub name: &'static str,
    /// Years after the first life that it is placed: the cover first, the others once what they take has spread
    pub year: u64,
    pub place: Place,
}

/// What a starting kind places: cover (`pcore::cover`) or bodies.
pub enum Place {
    Field(Field),
    Bodies(Bodies),
}

/// Cover placed on land cells: its genes (`pcore::cover::NG`), kg per m², and the chance per land cell.
pub struct Field {
    pub genes: [f32; cover::NG],
    pub kg: f64,
    pub chance: f64,
}

/// Bodies placed: the genes they start with and where and how many.
pub struct Bodies {
    /// Placed on any land cell (else only where bodies or cover already are)
    pub(super) anywhere: bool,
    pub(super) scalars: [f32; NSCALAR],
    /// log10 speed-up of the light reaction and of the burning of the energy-rich solid
    pub(super) cat: [f32; 2],
    /// Uptake selectivity of the liquid and the energy-rich solid
    pub(super) upt: [f32; 2],
    /// Bodies placed per cell; chance per land cell
    pub(super) per: usize,
    pub(super) chance: f64,
    /// Share of a member's mass placed as the energy-rich solid, its fuel at the start (the rest is its body, `split`)
    pub(super) store: f64,
    /// Hand-set brain biases (`Seeding::hand`; none for a kind without a brain)
    pub(super) hand: Option<Hand>,
}

/// Hand-set brain biases: the fixed actions (`brain::FIXED`), moving, the weight a move gives the depth of standing
/// liquid where it leads (`choose::WET`), then the verbs toward a body and a part.
pub(super) type Hand = ([f32; brain::FIXED], f32, f32, [f32; brain::VERBS + brain::OVERBS]);
/// The eaters' founders take from the place (its ground and its cover, 1) before they bite a body (0.5), and keep out
/// of deep standing liquid, as a land animal that sinks does: the weight -1 on ln(1 + depth / m) makes a move's odds
/// fall as 1 / (1 + depth / m).
const H_EATER: Hand = ([0.0, -3.0, 1.0, -2.5, -2.0, -2.5, -3.0], -1.0, -1.0, [0.5, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0]);

/// `s` with the genes of `d` (gene, value) set.
const fn with<const N: usize>(mut s: [f32; N], d: &[(usize, f32)]) -> [f32; N] {
    let mut i = 0;
    while i < d.len() {
        s[d[i].0] = d[i].1;
        i += 1;
    }
    s
}

/// The genes every kind of bodies starts with unless its row says otherwise: HEAT 3.5 (no coat), CHILD -0.523 (a
/// child 0.3 of the copy mass), MUT -1.2, HIDDEN 12, MEMORY 6, SEEN 3, SIGHT 1, DENATURE 320 K (about where enzymes
/// fail), ALLOC 0.8, KEEP_T 250 K and COOL_T 320 K (outside what the island has: they do nothing at the start).
const BASE: [f32; NSCALAR] = [0.0, 0.0, 3.5, 0.0, 0.0, -0.523, 0.0, 0.0, -1.2, 0.0, 12.0, 6.0, 3.0, 1.0, 0.0, 320.0, 0.0, 0.0, 0.8, 250.0, 320.0];

/// The cover of the starting light-eaters: a light capacity that a full summer's light about fills at a capture area of
/// 5 m² per kg (a herb's 20 m² per kg of leaf, a quarter of it leaf), walls 4 times its matter (herbs' cell walls), a
/// tenth of its built cover kept as fuel, frozen below 268 K, fully active at half the ground's field capacity, a
/// tenth of what it builds to seeds, the fixing reaction off (`FIXERS` of the cells turn it on); 0.1 kg per m² on every
/// land cell.
const COVER_GENES: [f32; cover::NG] = [-4.7, 0.7, 0.6, -1.0, 268.0, -0.3, 0.1, -1.0, -11.0];
const LIGHT: Kind = Kind { name: "light-eaters", year: 0, place: Place::Field(Field { genes: COVER_GENES, kg: 0.1, chance: 1.0 }) };

/// The temperatures the eaters' founders keep their bodies between, K: the melting point of chemistry 0's key liquid
/// (273 K) plus, and their catalysts' unfolding (`DENATURE`, 320 K) less, the margin within which a body's well-being
/// falls for nearness to a limit (5 K, `body::PHASE_MARGIN`): their liquid does not freeze in a winter on the hills, and
/// they sweat before their catalysts unfold in a summer; light-eaters keep none (`BASE`).
const EATER_KEEP_T: f32 = 278.0;
const EATER_COOL_T: f32 = 315.0;

/// Eaters: fast burning, take the energy-rich solid from the ground, the cover and bodies, short intervals, learning
/// on, keep warm and cool (`EATER_KEEP_T`, `EATER_COOL_T`); individuals, two in some cells where life already is.
const EATER_BODIES: Bodies = Bodies {
    anywhere: false,
    scalars: with(BASE, &[(GAS, 2.304), (LIQ, -0.5), (LEAF, -2.884), (STRENGTH, 0.6), (COPY_MASS, 1.645), (INTERVAL, 1.0), (LEARN, -1.0), (DISPERSE, 0.1), (REPAIR, -0.4), (KEEP_T, EATER_KEEP_T), (COOL_T, EATER_COOL_T)]),
    cat: [-1.0, 2.6],
    upt: [0.3, 1.0],
    per: 2,
    chance: 0.3,
    store: 0.3,
    hand: Some(H_EATER),
};
const EATER: Kind = eaters("eaters", 20, EATER_BODIES);

/// A kind of eaters placed `year` years after the first life.
const fn eaters(name: &'static str, year: u64, b: Bodies) -> Kind {
    Kind { name, year, place: Place::Bodies(b) }
}

/// Starting kinds; the row is the root label of the lineages a kind starts (the tape's `root_<k>_*` columns). Rows 0
/// and 1 are the two kinds of `ew run`, rows 2 to 9 with them the roster. A row sets only what the laws read (genes,
/// catalysts, uptake, and where, when and how many): kinds are initial conditions, never rules. The roster's rungs
/// scale the first two rows; eaters of bodies are stronger than what they eat, eaters of remains have no strength, and
/// the two big slow eater kinds start with 60% of their mass as fuel, not 30%, as a neutral brain must find what to take
/// within the few decisions its store lasts.
pub const KINDS: [Kind; 10] = [
    LIGHT,
    EATER,
    // Cover, small and fast, seeds spread wide; big and slow, less capture area per kg and tall walls, seeds stay.
    Kind { name: "small fast light-eaters", year: 0, place: Place::Field(Field { genes: with(COVER_GENES, &[(cover::LIGHT, -4.4), (cover::LEAF, 1.0), (cover::WALLS, 0.3), (cover::SEED, 0.3), (cover::REPAIR, -1.2)]), kg: 0.05, chance: 0.6 }) },
    Kind { name: "big slow light-eaters", year: 0, place: Place::Field(Field { genes: with(COVER_GENES, &[(cover::LIGHT, -5.0), (cover::LEAF, 0.4), (cover::WALLS, 1.0), (cover::STORE, -0.7), (cover::SEED, 0.05), (cover::REPAIR, -0.5)]), kg: 0.3, chance: 0.4 }) },
    // Eaters of remains on the ground, small and big: no strength, the solid first.
    eaters("small eaters of remains", 10, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 1.815), (STRENGTH, 0.0), (COPY_MASS, 0.245), (LEARN, -1.5), (DISPERSE, 0.3), (REPAIR, -1.48)]), upt: [0.2, 1.0], ..EATER_BODIES }),
    eaters("big eaters of remains", 10, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 2.205), (STRENGTH, 0.0), (COPY_MASS, 1.325), (INTERVAL, 1.3), (LEARN, -1.5), (DISPERSE, 0.2), (REPAIR, -0.89)]), per: 1, chance: 0.2, ..EATER_BODIES }),
    // Eaters of the cover, small and fast, and big and slow.
    eaters("small fast eaters of light-eaters", 20, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 2.273), (STRENGTH, 0.3), (COPY_MASS, 0.625), (INTERVAL, 0.85), (DISPERSE, 0.2), (REPAIR, -1.06)]), ..EATER_BODIES }),
    eaters("big slow eaters of light-eaters", 20, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 2.536), (LEAF, -2.702), (STRENGTH, 0.5), (COPY_MASS, 2.082), (INTERVAL, 1.2), (DISPERSE, 0.05), (REPAIR, -0.2)]), per: 1, chance: 0.15, store: 0.6, ..EATER_BODIES }),
    // Eaters of eaters, placed once the eaters have spread, with a bigger brain (16 hidden, 8 memory, 4 seen, sight 2).
    eaters("strong fast eaters of eaters", 24, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 2.478), (COPY_MASS, 2.145), (INTERVAL, 0.85), (LEARN, -0.7), (HIDDEN, 16.0), (MEMORY, 8.0), (SEEN, 4.0), (SIGHT, 2.0), (REPAIR, -0.5)]), per: 1, chance: 0.15, ..EATER_BODIES }),
    eaters("strong big slow eaters of eaters", 24, Bodies { scalars: with(EATER_BODIES.scalars, &[(GAS, 2.736), (LEAF, -2.702), (STRENGTH, 0.5), (COPY_MASS, 2.682), (INTERVAL, 1.15), (LEARN, -0.7), (HIDDEN, 16.0), (MEMORY, 8.0), (SEEN, 4.0), (SIGHT, 2.0), (DISPERSE, 0.05), (REPAIR, 0.0)]), per: 1, chance: 0.15, store: 0.6, ..EATER_BODIES }),
];

/// The roster of `ew run --roster`: the ten rows of `KINDS`, each placed at its `year`.
pub const ROSTER: [usize; 10] = [2, 0, 3, 4, 5, 6, 1, 7, 8, 9];


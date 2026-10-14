//! First life: bodies placed on land at the start. These are initial conditions only;
//! afterwards everything follows the laws. The reactions they speed up are found from the
//! chemistry's numbers: the light reaction, the energy-rich solid it makes, the liquid it
//! uses, and the reaction that burns that solid with a gas.

use pcore::body::{self, Body, Layout, Lineage};
use pcore::chem::{Chemistry, Phase};
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World, T_SURF};

/// One row of the table of starting kinds: the genes it starts with and where and how many are placed.
pub struct Kind {
    /// What the seeding line calls it (physical terms only)
    pub name: &'static str,
    /// Years after the first life that it is placed (`--life-year`): light-eaters first, the others once what they take has spread
    pub year: i64,
    /// Placed on any land cell (else only where bodies already are)
    anywhere: bool,
    scalars: [f32; body::NSCALAR],
    /// log10 speed-up of the light reaction and of the burning of the energy-rich solid
    cat: [f32; 2],
    /// Uptake selectivity of the liquid and the energy-rich solid
    upt: [f32; 2],
    /// Mass of a member, kg; cover of a node, kg/m² (0: individuals); bodies per cell placed
    mass: f64,
    cover: f64,
    per: usize,
    /// Chance per land cell
    chance: f64,
    /// Share of a member's mass placed as the energy-rich solid: its fuel at the start, which lasts
    /// 85 days x 0.6 / strength at 0.3 (more at a higher share). The rest is its body (`split`): body
    /// matter and the liquid it keeps with it
    store: f64,
    /// Hand-set brain biases (used for the animal kinds by `ew run` since 14 Oct 2026, and for
    /// the plants with `--plant-bias`, through `Seeding::hand`; `Seeding::default` keeps only the
    /// copy logit)
    hand: Hand,
}

/// Hand-set brain biases: rest, copy (its own chance, as a logit), put, take from the node, form a
/// part, let go, join to the ground, break the ground; move; take from, put to, watch a body;
/// hold, take from, put to, join to, break, enter a part
type Hand = ([f32; brain::FIXED], f32, [f32; brain::VERBS + brain::OVERBS]);
const H_LIGHT: Hand = ([2.0, -1.0, -3.0, -2.0, -6.0, -6.0, -6.0, -6.0], -4.0, [-4.0, -4.0, -4.0, -6.0, -6.0, -6.0, -6.0, -6.0, -6.0]);
const H_EATER: Hand = ([0.0, -1.6, -3.0, 0.5, -2.5, -2.0, -2.5, -3.0], -1.0, [2.0, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0]);

/// Light reaction, energy-rich solid, liquid, burning reaction.
pub fn roles(chem: &Chemistry) -> Option<(usize, usize, usize, usize)> {
    let at = |k: u16| chem.phase(k as usize, T_SURF);
    let r_light = (0..chem.reactions.len()).find(|&r| chem.reactions[r].light && chem.dh(r) > 0.0)?;
    let rx = &chem.reactions[r_light];
    let rich = rx.outputs.iter().find(|p| at(p.0) == Phase::Solid)?.0 as usize;
    let liquid = rx.inputs.iter().find(|p| at(p.0) == Phase::Liquid)?.0 as usize;
    let burn = (0..chem.reactions.len()).find(|&r| {
        let i = &chem.reactions[r].inputs;
        chem.dh(r) < 0.0 && i.iter().any(|p| p.0 as usize == rich) && i.iter().any(|p| at(p.0) == Phase::Gas)
    })?;
    Some((r_light, rich, liquid, burn))
}

/// Body matter and the reaction that burns it with a gas (none where bodies do not grow).
pub fn matter_roles(chem: &Chemistry) -> Option<(usize, usize)> {
    let m = chem.matter()?;
    let burn = (0..chem.reactions.len()).find(|&r| {
        let i = &chem.reactions[r].inputs;
        chem.dh(r) < 0.0 && i.iter().any(|p| p.0 as usize == m) && i.iter().any(|p| chem.phase(p.0 as usize, T_SURF) == Phase::Gas)
    })?;
    Some((m, burn))
}

/// How a placed member's mass is shared: (fuel, liquid, body matter), shares of the mass, for a
/// fuel share `store` and the keep gene `h` of the liquid. The body (the rest) is liquid in the
/// share `h` and matter in 1 - `h` (`Body::size`: matter over 1 - `h`). Without body matter in the
/// chemistry the rest is liquid.
pub fn split(store: f64, h: f64, matter: bool) -> (f64, f64, f64) {
    let body = 1.0 - store;
    if !matter {
        return (store, body, 0.0);
    }
    let liquid = h.clamp(0.0, 0.95) * body;
    (store, liquid, body - liquid)
}

/// Starting kinds, rows 0 to 2 the old ones and rows 3 to 10 more for the roster (`ROSTER`,
/// `ew run --roster`). Light-eaters: strong light reaction, slow burning, wide gas exchange and
/// capture area, long intervals, resting, learning off; one node per land cell, members of 1 kg
/// (copying at 2 kg), 0.1 kg/m². Eaters: fast burning, take the energy-rich solid from bodies and
/// the ground, short intervals, learning on at rate 10^-1 (the `LEARN` gene, -1.0: the rate the
/// Choose test passed with, ten times slower than before; it evolves); individuals of 30 kg
/// (copying at 63 kg), two in some cells. Both
/// start with a mutation size of 10^-1.2 per generation (generations are short at these sizes),
/// and catalysts that lose their shape above 320 K (about where life's enzymes fail).
/// The third row is the old clever lineage of the experiment `--clever-year`: an eater with a big
/// brain (48 hidden units, 12 memory numbers, 6 bodies perceived, sight of 3 cells) and learning
/// on (rate 10^-1.5), one per cell in 30% of the cells with bodies, always with the hand-set eater
/// biases. That test was confounded (review 6, finding 3: the hand-set start alone gave its
/// effect, a control with it did as well), so `--clever-year` no longer places this row but
/// `seed_clever`: copies of living eaters, with and without the big brain. The row stays so the
/// rows keep their numbers (`seed_with(2)` still places it).
///
/// A row sets only what the laws read: the genes (mass of a member and where it copies, strength,
/// interval, the four exchange genes, mutation, learning, brain sizes, dispersal, the temperature
/// its catalysts hold to, repair), the catalysts (what it digests and how fast), the uptake (what
/// it takes), and when, where and how many are placed. There is no height, lifespan, speed or
/// "intelligence" field, because the laws have none: size is the mass, speed is the interval
/// (a move is a decision), mind is the brain's sizes and learning rate, and a life lasts until its
/// upkeep has turned over `Physics::e_life` x 10^`REPAIR` per kg (14 Oct 2026). Kinds are initial
/// conditions, not laws; a bigger table is a bigger start, never a rule.
///
/// The `GAS` genes (the first scalar) are mol of air per m² of open surface per day since 14 Oct
/// 2026 (`Body::open_area`: capture area plus skin). Each is the old per-kg gene (2.5 for
/// light-eaters, 0.6 for the others, 0.9 for the small fast eaters of light-eaters) plus
/// log10(member mass / area per member at the start), so a kind breathes at its start what it did
/// per kg. Reset a second time the same day when leaf went with mass^0.75 like intake (review 5,
/// row 3; `Body::equiv`): new = old + log10(old area / new area), with area = (min(m, 2 x copy
/// mass) / 1 kg)^0.75 x 10^LEAF + 6 (m / 1000)^(2/3), m² (old: min(m, 2 x copy mass) x 10^LEAF
/// + the same skin). Light-eaters 1 kg: 1.995 + 0.060 = 2.055 either way, gene 2.187; small fast
/// 0.05 kg: leaf 0.199 -> 0.421 (0.05^0.75 x 10^0.6), area 0.207 -> 0.429, 1.883 -> 1.567; big
/// slow 20 kg: leaf 10.02 -> 4.74 (20^0.75 = 9.46 x 10^-0.3), area 10.47 -> 5.18, 2.781 -> 3.086;
/// eaters 30 kg: 0.030 -> 0.013, area 0.609 -> 0.592, 2.292 -> 2.304; eaters of remains 1 kg:
/// 0.061, 1.815 (no change); 15 kg: 0.380 -> 0.373, 2.196 -> 2.205; eaters of light-eaters 3 kg:
/// 0.128 -> 0.127, 2.271 -> 2.273; 150 kg: 1.844 -> 1.737, 2.510 -> 2.536; eaters of eaters 100 kg:
/// 1.393 -> 1.324, 2.456 -> 2.478; 600 kg: 4.868 -> 4.390, 2.691 -> 2.736.
///
/// The `HEAT` genes (the third scalar) are W per K and kg of a member of `Physics::m_ref` = 1 kg
/// since 14 Oct 2026 (before, of 30 kg: conductance = gene x 30^(1/3) x m^(2/3) per member). The
/// same conductance at any mass needs a gene 30^(1/3) = 3.107 times higher, log10 = +0.492, in
/// every kind (a member of m kg: old g x 30^(1/3) x m^(2/3) = new g x 1 x m^(2/3)): light-eaters,
/// small fast and big slow 2.2 to 2.692; eaters and clever eaters, small fast eaters of
/// light-eaters 0.0 to 0.492; small eaters of remains 1.0 to 1.492; big eaters of remains 0.3 to
/// 0.792; big slow eaters of light-eaters and strong fast eaters of eaters -0.3 to 0.192; strong
/// big slow eaters of eaters -0.5 to -0.008. The gene's range moved by 0.5 with it (-2.5 to 3.5).
///
/// The `REPAIR` genes (the last scalar) are set so the kinds start with a range of lives, short
/// for small fast ones and long for big slow ones (years, at the starting mass and interval, by
/// the upkeep per kg). These were set before basal upkeep (review 5) and the costs by mass^0.75
/// (review 6), which raised the upkeep per kg of small bodies, so measured one year after
/// placement (tape `root_<k>_life`, 64x64 roster, seeds 1 to 3, 14 Oct 2026) the lives were:
/// light-eaters 5.9 to 7.7 (1 kg; 18 was meant) and 39 to 52 (20 kg; 97); eaters 19; eaters of
/// remains 0.65 to 0.77 (1 kg; 2.5) and 1.2 to 1.3 (15 kg; 17); small fast eaters of light-eaters
/// 1.0 to 1.4 (3 kg; 2.6), 150 kg 37 to 42, eaters of eaters 18 (100 kg) and 77 to 80 (600 kg). Age
/// was then 49 to 65% of the deaths of the three small animal kinds. Their genes are set again
/// (new = old + log10(meant / measured): -2.1 -> -1.48, -2.1 -> -0.89, -1.5 -> -1.06, for 3, 21 and 3
/// years), which took age to 10 to 25% of their deaths (their real lives hardly moved: they are
/// held by what they find to eat). At gene 0 they would live 25 to 1,700 years: the strength gene
/// sets most of an eater's upkeep and a small member's brain costs it much per kg, so the eaters
/// of remains (strength 0) would live longest. A gene below 0 costs less upkeep
/// (`Physics::c_repair`): evolution sets it from there.
///
/// The roster has rungs from the first day, each derived from the two old rows by scaling their
/// numbers and keeping what lets a kind live: light-eaters keep the light catalyst (12) and the
/// exchange genes, the others the burning catalyst (2.6: catalysts cost by capacity, an eater's about a tenth of its upkeep, 14 Oct 2026) and an uptake of the liquid and
/// the energy-rich solid. Small ones copy at about twice their mass at short intervals, big ones
/// at long intervals; the exchange genes are per kg and the heat gene goes with mass^(2/3), so a
/// small body loses heat faster per kg and a big one slower (`Body::conductance`). What one body
/// can take from another is set by its force against the other's (`choose::take_body`: the
/// strength gene times the mass), and upkeep charges the gene by the kg-equivalent (mass^0.75, since 14 Oct 2026): the "eaters of
/// light-eaters" have genes of 0.3 to 0.5 (the light-eaters have 0.05), the "eaters of eaters" 0.6
/// to 0.6 and 100 to 600 kg (the eaters have 0.6 and 30 kg: three to twenty times their force), and the "eaters of
/// remains" none (0: they lose every fight, pay no upkeep for it, and work the ground). The gene
/// need not go higher with the mass, and cannot with the interval: a body starts with 30% of its
/// mass as the energy-rich solid, which at a strength gene of s lasts 85 days x 0.6 / s, and a
/// neutral brain must find what to take within the few decisions that fit (a kind with long
/// intervals and a high gene is out of store before it has decided twice: 600 kg members at 40
/// days and strength 0.95 were all dead within 6 months). Eaters of eaters and big eaters are
/// placed in hundreds (chance 0.15 of a cell) for that reason: most of a neutral start starves.
/// The two big slow eater kinds (150 kg every 16 days, 600 kg every 14 days; strength 0.5) start
/// with 60% of their mass as the energy-rich solid, not 30% (`Kind::store`: an initial condition,
/// the seeding line says it): 100 days of store held 6 or 7 decisions of a neutral brain and they
/// were all gone by year 30 in a 64x64 roster run (seed 1: of 209 of the 150 kg kind placed at
/// year 22, 3 bodies left at year 24 and none at 30; of 220 of the 600 kg kind placed at year 26,
/// 1 left at 28 and none at 35); with 200 days, the 150 kg kind had 7 bodies at year 24, 73 at 30
/// and 92 at 40 (1.5 million members in nodes), and the 600 kg kind kept 3 to year 40.
///
/// **Body and store (14 Oct 2026, decision after watch report 7).** A member is placed as
/// fuel (`Kind::store` of its mass, the energy-rich solid) and a body, which is body matter and the
/// liquid it keeps with it in the share of its liquid keep gene (0.75, `Body::size`): the body is
/// matter over 1 - 0.75, so of the 70% that is not fuel, 52.5% is liquid and 17.5% matter (`split`);
/// of the 40% that is not fuel in the two big slow kinds (store 60%), 30% liquid and 10% matter.
/// Copy mass and leaf now go by the body, so the genes are set so that each kind copies at the same
/// total mass and has the same leaf area at placement as before. With b the body's share of the mass
/// at placement (0.7; 0.4 for the store-60% kinds): copy mass new = old x b, so the gene rises by
/// log10 b (-0.155; -0.398); leaf area per member = (body / 1 kg)^0.75 x 10^LEAF, so LEAF new = old -
/// 0.75 log10 b (+0.116; +0.299). The area is the same at placement, so the `GAS` genes (per m² of the
/// area and the skin, which goes by the whole mass) stay. Copy mass, gene old -> new (kg old ->
/// new): light-eaters 0.3 -> 0.145 (2.0 -> 1.4), small fast -1.0 -> -1.155 (0.1 -> 0.07), big slow
/// 1.6 -> 1.445 (40 -> 28); eaters 1.8 -> 1.645 (63 -> 44); eaters of remains 0.4 -> 0.245 (2.5 ->
/// 1.8) and 1.48 -> 1.325 (30 -> 21); eaters of light-eaters 0.78 -> 0.625 (6.0 -> 4.2) and 2.48 ->
/// 2.082 (302 -> 121); eaters of eaters 2.3 -> 2.145 (200 -> 140) and 3.08 -> 2.682 (1,202 -> 481).
/// Leaf: light-eaters 0.3 -> 0.416, small fast 0.6 -> 0.716, big slow -0.3 -> -0.184, animals -3.0
/// -> -2.884 (store 30%) or -2.702 (store 60%).
///
/// **What a body carries is bounded by its body** (`Body::level`, the keep genes, and the safe
/// minimums, the min genes, are kg per kg of body, not of the whole mass). At placement a 30% kind
/// carries 0.3 / 0.7 = 0.43 kg of the solid per kg of body, under the keep gene 0.9, and its liquid
/// is the share 0.75 of the body (3 x its matter), exactly its keep; a 60% kind carries 0.6 / 0.4 =
/// 1.5, so its keep gene for the solid is 1.1 x 1.5 = 1.65 (the solid's gene is `max(0.9, 1.1 x
/// store / (1 - store))`). Minimums: the liquid 0.3 and the solid 0.25 of the body (the liquid is
/// 0.75 of the body, the solid 0.43). The solid's minimum is a reserve, not a death line: a body
/// burns the solid, so it dies of hunger when it cannot pay its upkeep (`body::SPENT`), and the
/// minimum is the floor growth and giving keep (`Body::burns`). 0.25 of the body is about three
/// updates of upkeep (a 1 kg light-eater: 0.25 x 15.6 MJ = 3.9 MJ against 1.1 MJ an update of 20
/// days; a 35 kg eater: 0.25 x 35 x 15.6 = 136 MJ against 46 MJ an update of 10 days). It was 0.02,
/// and growth spent the rest, so a body held one update of upkeep it could release (review 6,
/// review of lives, 14 Oct 2026). An initial condition: the gene evolves.
///
/// **Eaters digest body matter.** An animal kind (its light gene below `body::LIGHT_EATER`) gets the
/// catalyst gene of the reaction that burns body matter equal to its gene for the energy-rich solid,
/// and an uptake of the matter equal to its uptake of the solid (`matter_roles`); light-eaters have
/// neither (catalysts cost by capacity, so a plant does not carry it). Both evolve from there. A body
/// without the catalyst takes no matter, and what it takes is broken into the solid as it comes in
/// (`Body::digest`).
pub const KINDS: [Kind; 11] = [
    Kind { name: "light-eaters", year: 0, anywhere: true, scalars: [2.187, 0.0, 2.692, 0.416, 0.05, 0.3, 0.145, 1.3, -1.2, -4.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1, 320.0, -1.0], cat: [9.0, 2.0], upt: [0.0, 0.0], mass: 1.0, cover: 0.1, per: 1, chance: 1.0, store: 0.3, hand: H_LIGHT },
    Kind { name: "eaters", year: 20, anywhere: false, scalars: [2.304, -0.5, 0.492, -2.884, 0.6, 0.3, 1.645, 1.0, -1.2, -1.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1, 320.0, -0.4], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 30.0, cover: 0.0, per: 2, chance: 0.3, store: 0.3, hand: H_EATER },
    Kind { name: "clever eaters (48 hidden, 12 memory, 6 seen, sight 3, learning on)", year: 0, anywhere: false, scalars: [2.304, -0.5, 0.492, -2.884, 0.6, 0.3, 1.645, 1.0, -1.2, -1.5, -4.0, 48.0, 12.0, 6.0, 3.0, 0.1, 320.0, -0.4], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 30.0, cover: 0.0, per: 1, chance: 0.3, store: 0.3, hand: H_EATER },
    // Light-eaters, small and fast: members of 0.05 kg copying at 0.1 kg every 10 days, children spread wide.
    Kind { name: "small fast light-eaters", year: 0, anywhere: true, scalars: [1.567, 0.0, 2.692, 0.716, 0.05, 0.3, -1.155, 1.0, -1.2, -4.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.3, 320.0, -1.2], cat: [9.0, 2.0], upt: [0.0, 0.0], mass: 0.05, cover: 0.05, per: 1, chance: 0.6, store: 0.3, hand: H_LIGHT },
    // Light-eaters, big and slow: members of 20 kg copying at 40 kg every 60 days, less capture area per kg, children stay.
    Kind { name: "big slow light-eaters", year: 0, anywhere: true, scalars: [3.086, 0.0, 2.692, -0.184, 0.05, 0.3, 1.445, 1.78, -1.2, -4.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.05, 320.0, -0.5], cat: [9.0, 2.0], upt: [0.0, 0.0], mass: 20.0, cover: 0.3, per: 1, chance: 0.4, store: 0.3, hand: H_LIGHT },
    // Eaters of remains on the ground: weak (0.05 and 0.1) (the burning catalyst of the others: a quicker one costs too much), the solid first.
    Kind { name: "small eaters of remains", year: 10, anywhere: false, scalars: [1.815, -0.5, 1.492, -2.884, 0.0, 0.3, 0.245, 1.0, -1.2, -1.5, -4.0, 12.0, 6.0, 3.0, 1.0, 0.3, 320.0, -1.48], cat: [-1.0, 2.6], upt: [0.2, 1.0], mass: 1.0, cover: 0.0, per: 2, chance: 0.3, store: 0.3, hand: H_EATER },
    Kind { name: "big eaters of remains", year: 10, anywhere: false, scalars: [2.205, -0.5, 0.792, -2.884, 0.0, 0.3, 1.325, 1.3, -1.2, -1.5, -4.0, 12.0, 6.0, 3.0, 1.0, 0.2, 320.0, -0.89], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 15.0, cover: 0.0, per: 1, chance: 0.2, store: 0.3, hand: H_EATER },
    // Eaters of light-eaters, small and fast: 3 kg copying at 6 kg every 7 days, strength 0.3 (the light-eaters have 0.05, the eaters 0.6).
    Kind { name: "small fast eaters of light-eaters", year: 20, anywhere: false, scalars: [2.273, -0.5, 0.492, -2.884, 0.3, 0.3, 0.625, 0.85, -1.2, -1.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.2, 320.0, -1.06], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 3.0, cover: 0.0, per: 2, chance: 0.3, store: 0.3, hand: H_EATER },
    // Eaters of light-eaters, big and slow: 150 kg copying at 300 kg every 16 days, strength 0.5.
    Kind { name: "big slow eaters of light-eaters", year: 20, anywhere: false, scalars: [2.536, -0.5, 0.192, -2.702, 0.5, 0.3, 2.082, 1.2, -1.2, -1.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.05, 320.0, -0.2], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 150.0, cover: 0.0, per: 1, chance: 0.15, store: 0.6, hand: H_EATER },
    // Eaters of eaters (stronger in force than the eaters, whose strength gene is 0.6: the force is the gene times the mass), placed once the eaters have spread:
    // 100 kg copying at 200 kg every 7 days, strength 0.6; 600 kg copying at 1200 kg every 14 days, strength 0.5. A bigger brain (16 hidden, 8 memory, 4 seen, sight 2), learning on.
    Kind { name: "strong fast eaters of eaters", year: 24, anywhere: false, scalars: [2.478, -0.5, 0.192, -2.884, 0.6, 0.3, 2.145, 0.85, -1.2, -0.7, -4.0, 16.0, 8.0, 4.0, 2.0, 0.1, 320.0, -0.5], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 100.0, cover: 0.0, per: 1, chance: 0.15, store: 0.3, hand: H_EATER },
    Kind { name: "strong big slow eaters of eaters", year: 24, anywhere: false, scalars: [2.736, -0.5, -0.008, -2.702, 0.5, 0.3, 2.682, 1.15, -1.2, -0.7, -4.0, 16.0, 8.0, 4.0, 2.0, 0.05, 320.0, 0.0], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 600.0, cover: 0.0, per: 1, chance: 0.15, store: 0.6, hand: H_EATER },
];

/// The roster of `ew run --roster`: ten kinds with rungs from the first day (rows of `KINDS`,
/// each placed at its `year`): light-eaters of three sizes, eaters of remains of two, eaters of
/// light-eaters of three and eaters of eaters of two.
pub const ROSTER: [usize; 10] = [3, 0, 4, 5, 6, 7, 1, 8, 9, 10];

/// How the first bodies are placed. What is placed, and when, is initial condition and must be
/// reported with every result (`seed` returns the line to print).
///
/// By default each kind starts as a simple ancestor with a NEUTRAL brain: every action bias is 0
/// (resting, putting, taking, moving, the verbs on bodies and parts) and the weights are small
/// random numbers, so nothing says "eat" or "move when hungry"; selection has to find that. The
/// one bias kept from the old hand-set brains is the copy logit (`Kind::fixed[1]`, the body's own
/// chance to copy): copying is already gated by physics (a member must hold its copy mass and pay
/// for it), and a neutral 0 would make every body that holds the mass copy on half of its
/// decisions, a rate nobody chose; the old value keeps the pace of generations, not a behaviour.
/// Then every body gets `rounds` rounds of mutation (genes by `copy::mutate`, born brain rows by
/// `copy::shake` as large as a copy that changes rows), each body its own random walk and, for
/// the rows, its own genome, so the first generation is already a spray of variants. A neutral
/// start gets more than one generation's step, since that step is too small to give selection
/// anything to pick from in the time available (review 2, problem 2): the bias of every row (the
/// action biases among them) gets rounds of `size`, so after 20 rounds they span about
/// sqrt(20) × 0.2 = 0.9; the other entries of the rows keep the step of one generation, so the
/// features do not swamp the brain; the genes get `genes` times the step of one generation.
/// Hand-set starts (`hand`) keep the step of one generation for everything.
#[derive(Clone, Copy, Debug)]
pub struct Seeding {
    /// Hand-set brain biases, per kind (light-eaters, animals). `Seeding::default` is neutral for
    /// both (the tests of the laws place bodies that way); `ew run` sets the animals' since 14 Oct
    /// 2026 (review 6, finding 1: 97 to 99% of neutral-brained founder eaters die in their first
    /// year, so neutral animal founders are a lottery; `--neutral-eaters` for the old start).
    pub hand: [bool; 2],
    /// Rounds of mutation given to every body placed (0: all bodies of a kind are the same).
    pub rounds: u32,
    /// Step of one round on the bias of every row (neutral starts)
    pub size: f32,
    /// How many times the step of one generation the genes get in all (neutral starts)
    pub genes: f32,
}

impl Seeding {
    /// The old start: hand-set brain biases and no mutation (for tests of the laws that need the
    /// same body every time).
    pub fn old() -> Self {
        Seeding { hand: [true; 2], rounds: 0, ..Default::default() }
    }
}

impl Default for Seeding {
    fn default() -> Self {
        Seeding { hand: [false; 2], rounds: 20, size: 0.2, genes: 3.0 }
    }
}

/// Place bodies of starting kind `which` (a row of `KINDS`: 0 light-eaters on land, 1 eaters where bodies already
/// are, 2 the clever lineage where bodies already are, 3 and on the roster) as a neutral ancestor sprayed with 20 rounds of mutation (`Seeding::default`).
pub fn seed(w: &mut World, which: usize) -> String {
    seed_with(w, which, &Seeding::default())
}

/// `seed` with the choices of `how`. Returns what was placed, in one line.
pub fn seed_with(w: &mut World, which: usize, how: &Seeding) -> String {
    let Some(kind) = KINDS.get(which) else { return String::new() };
    if roles(&w.chem).is_none() {
        return String::new();
    }
    body::survey(w);
    let [h, mem, seen, sight] = choose::sizes(&kind.scalars);
    let shape = choose::shape(w.chem.ns(), h, mem);
    let mut r = Rng::new(w.seed, w.day, which as u64, Purpose::Body);
    let hand = how.hand[(!kind.anywhere) as usize] || which == 2;
    let (fixed, dir, verbs) = if hand {
        (kind.hand.0, kind.hand.1, kind.hand.2)
    } else {
        let mut fixed = [0.0; brain::FIXED];
        fixed[1] = kind.hand.0[1];
        (fixed, 0.0, [0.0; brain::VERBS + brain::OVERBS])
    };
    let mut base = brain::random(shape, &mut r, 0.1, fixed, dir, verbs);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen, sight, base, root: which as u32 });
    // One genome for all when nothing is sprayed; else one per body.
    let shared = (how.rounds == 0).then(|| w.life.new_genome(rows.clone()));
    let (mut placed, mut members, mut kg) = (0usize, 0.0, 0.0);
    let (mut cat, mut cmass) = ([(f32::MAX, f32::MIN); 2], (f32::MAX, f32::MIN));
    // One action bias, "take from a body": its range and spread over the bodies placed.
    let (at, mut tb) = (shape.verb(0) * shape.r() + shape.h, (f32::MAX, f32::MIN, 0.0f64, 0.0f64));
    let (light, burn) = roles(&w.chem).map_or((0, 0), |(l, _, _, b)| (l, b));
    for i in 0..w.grid.n() {
        let [_, depth, z] = w.life.env[i];
        if z <= 0.0 || depth > 0.5 || r.f64() >= kind.chance || (!kind.anywhere && w.life.at[i].is_empty()) {
            continue;
        }
        for k in 0..kind.per {
            let id = (1 + which as u64) << 48 | (i as u64) << 8 | k as u64;
            let genome = if let Some(g) = shared {
                g
            } else {
                let mut v = rows.clone();
                let mut rr = Rng::new(w.seed, w.day, id, Purpose::Mutate);
                let size = w.cfg.phys.row_noise * 10f32.powf(kind.scalars[body::MUT]) / (w.cfg.phys.row_chance as f32).sqrt();
                for _ in 0..how.rounds {
                    pcore::copy::shake(&mut v, &mut rr, size);
                }
                // The bias of each row (its last entry) as far as `rounds` rounds of `size` go.
                if !hand {
                    let (r, extra) = (shape.r(), how.size * (how.rounds as f32).sqrt());
                    v.chunks_exact_mut(r).for_each(|row| pcore::copy::shake(&mut row[shape.h..], &mut rr, extra));
                }
                w.life.new_genome(v)
            };
            let Some(b) = make_with(w, which, i, lineage, genome, how.rounds, if hand { 1.0 } else { how.genes }, id) else { return String::new() };
            let lay = Layout::of(&w.chem);
            for (c, g) in cat.iter_mut().zip([lay.cat + light, lay.cat + burn]) {
                *c = (c.0.min(b.genes[g]), c.1.max(b.genes[g]));
            }
            cmass = (cmass.0.min(b.genes[body::COPY_MASS]), cmass.1.max(b.genes[body::COPY_MASS]));
            let x = w.life.born(&b)[at];
            tb = (tb.0.min(x), tb.1.max(x), tb.2 + x as f64, tb.3 + (x * x) as f64);
            (placed, members, kg) = (placed + 1, members + b.count, kg + b.mass(&w.chem));
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day, delay);
        }
    }
    let sc = &kind.scalars;
    let (fuel, liq, mat) = split(kind.store, 0.75, matter_roles(&w.chem).is_some());
    let name = format!("{} (members of {} kg copying at {:.3} kg, every {:.0} days, strength {}, repair {}, starting mass {:.1}% energy-rich solid, {:.1}% liquid, {:.1}% body matter)", kind.name, kind.mass, 10f32.powf(sc[body::COPY_MASS]), 10f32.powf(sc[body::INTERVAL]), sc[body::STRENGTH], sc[body::REPAIR], 100.0 * fuel, 100.0 * liq, 100.0 * mat);
    let brain = if hand { "hand-set brain biases (initial condition)" } else { "neutral brain (action biases 0, copy logit kept)" };
    let (m, n) = (tb.2 / placed.max(1) as f64, placed.max(1) as f64);
    let spray = if how.rounds == 0 {
        String::new()
    } else if hand {
        ", row and gene steps of one generation".to_string()
    } else {
        format!(", row biases {} x {} (spread {:.2}), other row entries and genes at one generation's step (genes x{})", how.rounds, how.size, how.size * (how.rounds as f32).sqrt(), how.genes)
    };
    format!(
        "year {:.1}: placed {placed} {name} ({members:.3e} members, {kg:.3e} kg) with {brain}, {} rounds of mutation each{spray}; light gene {:.2}..{:.2}, burn gene {:.2}..{:.2}, copy-mass gene {:.2}..{:.2}; bias of taking from a body {:.2}..{:.2} (mean {m:.2}, sd {:.2})",
        w.year(),
        how.rounds,
        cat[0].0,
        cat[0].1,
        cat[1].0,
        cat[1].1,
        cmass.0,
        cmass.1,
        tb.0,
        tb.1,
        (tb.3 / n - m * m).max(0.0).sqrt()
    )
}

/// The root label of the big-brain lineage `seed_clever` places (the row of the old "clever" kind)
/// and of its control, past the rows of `KINDS`: each is its own kind on the tape.
pub const CLEVER_ROOT: u32 = 2;
pub const CONTROL_ROOT: u32 = KINDS.len() as u32;
/// The brain sizes of the big-brain lineage: hidden units, memory numbers, bodies perceived, sight.
pub const CLEVER_SIZES: [usize; 4] = [48, 12, 6, 3];

/// A fair test of a big brain (`--clever-year`, review 6, finding 3): copy `n` living ordinary
/// eaters (an even sample of the bodies of the eaters' root, one member of each; if there are
/// fewer, some are copied again) twice, into the same cells with the same stores per member.
/// One set gets the big brain (`CLEVER_SIZES`; the lineage's base and the born rows carried over
/// by `brain::rebase` and `brain::rerow`, as `copy::resize` does), a new lineage of root
/// `CLEVER_ROOT`; the other keeps its sizes, a new lineage of root `CONTROL_ROOT`. Genes and born
/// rows are the source's, no brain bias is set by hand, and the sets differ in brain size only
/// (and what that costs). The copies are new matter, booked like an edit. Returns what was placed,
/// in one line.
pub fn seed_clever(w: &mut World, n: usize) -> String {
    let src: Vec<usize> = (0..w.life.bodies.len()).filter(|&s| {
        let b = &w.life.bodies[s];
        b.alive && w.life.root(b) == 1 && !body::light_eater(&w.chem, &b.genes)
    }).collect();
    if src.is_empty() || n == 0 {
        return format!("year {:.1}: no living ordinary eaters to copy for the big-brain test", w.year());
    }
    let (ns, noise) = (w.chem.ns(), w.cfg.phys.new_noise);
    let big = choose::shape(ns, CLEVER_SIZES[0], CLEVER_SIZES[1]);
    // The two new lineages made from each source lineage: (source, big, control).
    let mut made: Vec<(u32, u32, u32)> = Vec::new();
    let (mut members, mut kg, mut sources) = (0.0f64, 0.0f64, 0usize);
    let mut last = usize::MAX;
    for j in 0..n {
        let s = src[j * src.len() / n];
        sources += (s != last) as usize;
        last = s;
        let b = &w.life.bodies[s];
        let (cell, lin, m) = (b.cell as usize, b.lineage, b.n());
        let (genes, born, wear, wet) = (b.genes.clone(), w.life.born(b).to_vec(), if m > 1.0 { 0.0 } else { b.wear }, b.wet);
        let (store, e): (Vec<f64>, f64) = (b.store.iter().map(|a| a / m).collect(), b.e / m);
        let src_lin = w.life.brains[lin as usize].clone().expect("lineage");
        if !made.iter().any(|x| x.0 == lin) {
            let mut r = Rng::new(w.seed, w.day, 1 << 40 | lin as u64, Purpose::Body);
            let base = brain::rebase(&src_lin.base, src_lin.shape, big, &mut r, noise);
            let a = w.life.new_lineage(Lineage { shape: big, seen: CLEVER_SIZES[2], sight: CLEVER_SIZES[3], base, root: CLEVER_ROOT });
            let c = w.life.new_lineage(Lineage { root: CONTROL_ROOT, ..src_lin.clone() });
            made.push((lin, a, c));
        }
        let (_, a, c) = *made.iter().find(|x| x.0 == lin).expect("made");
        let mut r = Rng::new(w.seed, w.day, 2 << 40 | j as u64, Purpose::Body);
        for (lineage, is_big) in [(a, true), (c, false)] {
            let (mut g, mut rows) = (genes.clone(), born.clone());
            if is_big {
                for (k, gene) in [body::HIDDEN, body::MEMORY, body::SEEN, body::SIGHT].into_iter().enumerate() {
                    g[gene] = CLEVER_SIZES[k] as f32;
                }
                rows = brain::rerow(&born, src_lin.shape, big, &mut Rng::new(w.seed, w.day, 3 << 40 | j as u64, Purpose::Body), noise);
            }
            let group = w.life.new_group(lineage, rows.clone());
            let genome = w.life.new_genome(rows);
            let mut b = Body { cell: cell as u32, count: 1.0, lineage, group, near: u32::MAX, mark: w.light[cell], e, store: store.clone(), genes: g, genome, wear, wet, born: w.day, ..Default::default() };
            b.vis = b.visible(&w.chem, &w.cfg.phys);
            // New matter and energy, booked like an edit (as `make_with`).
            w.ledger.imposed_energy += b.e;
            for (k, &x) in b.store.iter().enumerate() {
                w.chem.add_elems(k, x, &mut w.ledger.imposed_elems);
                w.ledger.imposed_energy += x * w.chem.subs[k].hf;
            }
            if is_big {
                (members, kg) = (members + 1.0, kg + b.mass(&w.chem));
            }
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day, delay);
        }
    }
    format!(
        "year {:.1}: placed {n} copies of living ordinary eaters (from {sources} bodies of the eaters' root) twice, in the same cells with the same stores ({:.1} kg a member, {kg:.3e} kg for each set), same genes and born rows, no hand-set biases: as a big-brain lineage (root {CLEVER_ROOT}: {} hidden, {} memory, {} seen, sight {}; rows carried over by rerow) and as its control (root {CONTROL_ROOT}: brain sizes unchanged)",
        w.year(),
        kg / members.max(1.0),
        CLEVER_SIZES[0],
        CLEVER_SIZES[1],
        CLEVER_SIZES[2],
        CLEVER_SIZES[3]
    )
}

/// A body of starting kind `which` in cell `i`, of `lineage`, born with brain rows `genome`, in
/// a new group of it, at the cell's temperature; its matter and energy are booked like an edit.
/// Not yet placed.
pub fn make(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32) -> Option<Body> {
    make_with(w, which, i, lineage, genome, 0, 1.0, 0)
}

/// `make` after `rounds` rounds of mutation of its genes, `gmul` times a generation's step (a
/// random walk keyed by `id`).
fn make_with(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32, rounds: u32, gmul: f32, id: u64) -> Option<Body> {
    let ((r_light, rich, liquid, burn), kind) = (roles(&w.chem)?, KINDS.get(which)?);
    let group = w.life.new_group(lineage, w.life.genomes[genome as usize].rows.clone());
    let (chem, ledger, t) = (&w.chem, &mut w.ledger, w.life.env[i][0]);
    let (lay, ns) = (Layout::of(chem), chem.ns());
    // Genes that are off start below zero, so small changes keep them off.
    let mut genes = vec![-0.1f32; lay.len];
    genes[lay.cat..lay.min].fill(-1.0);
    genes[..body::NSCALAR].copy_from_slice(&kind.scalars);
    (genes[lay.cat + r_light], genes[lay.cat + burn]) = (kind.cat[0], kind.cat[1]);
    (genes[lay.min + liquid], genes[lay.min + rich]) = (0.3, 0.25);
    (genes[lay.upt + liquid], genes[lay.upt + rich]) = (kind.upt[0], kind.upt[1]);
    // Animals digest body matter as they digest the solid, and take it (14 Oct 2026): light-eaters do not.
    let matter = matter_roles(chem);
    if let Some((m, r)) = matter.filter(|_| kind.cat[0] < body::LIGHT_EATER) {
        (genes[lay.cat + r], genes[lay.upt + m]) = (kind.cat[1], kind.upt[1]);
    }
    // What a body carries of each store is kg per kg of its body (`Body::level`, 14 Oct 2026): 0.9 of
    // the solid, or what it is placed with and a tenth more if that is more (a kind of 60% fuel and a
    // body of 40% carries 1.5 kg of the solid per kg of body: keep 1.65); the liquid is the share 0.75.
    genes[lay.keep..].fill(0.9);
    genes[lay.keep + liquid] = 0.75;
    genes[lay.keep + rich] = 0.9f32.max((1.1 * kind.store / (1.0 - kind.store)) as f32);
    let mut rg = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    // Its mutation size is raised by `gmul` for the rounds (and taken back, whatever the walk made of it).
    genes[body::MUT] += gmul.log10();
    for _ in 0..rounds {
        genes = pcore::copy::mutate(&lay, &genes, &mut rg);
    }
    genes[body::MUT] -= gmul.log10();
    let count = if kind.cover > 0.0 { kind.cover * w.grid.area() / kind.mass } else { 1.0 };
    // Fuel, and a body of liquid and matter in the share its keep gene sets (`split`).
    let mut store = vec![0.0; ns];
    let (fuel, liq, mat) = split(kind.store, genes[lay.keep + liquid] as f64, matter.is_some());
    store[rich] = fuel * count * kind.mass / chem.subs[rich].molar_mass;
    store[liquid] = liq * count * kind.mass / chem.subs[liquid].molar_mass;
    if let Some((m, _)) = matter {
        store[m] = mat * count * kind.mass / chem.subs[m].molar_mass;
    }
    let mut b = Body { cell: i as u32, count, lineage, group, near: u32::MAX, mark: w.light[i], store, genes, genome, ..Default::default() };
    b.e = b.heat_cap(chem) * t;
    ledger.imposed_energy += b.e;
    for (k, &a) in b.store.iter().enumerate() {
        chem.add_elems(k, a, &mut ledger.imposed_elems);
        ledger.imposed_energy += a * chem.subs[k].hf;
    }
    // Its key liquids first: `visible` reads the body (`Body::size`) through them.
    b.wet = b.key_liquids(chem);
    b.vis = b.visible(chem, &w.cfg.phys);
    Some(b)
}

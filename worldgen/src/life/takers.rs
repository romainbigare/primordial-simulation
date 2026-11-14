//! The starting kinds of takers and their placing: initial conditions only (laws.md 5). Each kind is placed on every
//! open land cell at a third of the line of members per area against body mass that real plant-eaters follow (Damuth
//! 1981), all adults, with a store of fuel.

use super::open_land;
use pcore::fauna::{self, CHILD, COAT, ENZYME, KEEP, MASS, NG, N_WHOLE, REACH, REPAIR, YOUNG};
use pcore::rng::{Purpose, Rng};
use pcore::World;

/// One starting kind of takers: what the placing line calls it (physical terms only), the key of its random draws
/// (fixed with the kind, so its placing does not move when the table changes) and its genes (`pcore::fauna::NG`). Its row
/// is its label (`Fauna::root`).
pub struct TakerKind {
    pub name: &'static str,
    pub key: u32,
    pub genes: [f32; NG],
}

/// The starting kinds, each at the size of a real kind of plant-eater of its mass (the genes in the order of
/// `pcore::fauna`'s: log10 mass, log10 newborn share, share of a surplus to young, log10 gut capacity for walls, log10
/// middle of the heights searched, temperature kept, coat, repair): 30 g, 3 kg, 60 kg and 400 kg kinds that keep a
/// mammal's heat, and a 2 kg kind that keeps none of its own. Each gut's capacity splits the walls a member of its kind
/// takes a day on a cell of good low cover in its warm month, and no more (its capacity is paid for whether used or
/// not): walls w kg a day over their molar mass (0.162 kg/mol) over m × pace × a day's s, at the pace of its body's
/// temperature (5.6 at 310 K; 0.83 for the cold kind at 288 K): 1.2 g, 40 g, 0.39 kg, 1.6 kg and 24 g of walls a day
/// give 10^-6.3, 10^-6.8, 10^-7.1, 10^-7.3 and 10^-6.0 mol a kg a s.
pub const TAKER_KINDS: [TakerKind; 5] = [
    TakerKind { name: "small low takers", key: 101, genes: [-1.5, -1.2, 0.8, -6.3, -1.52, 310.0, 0.2, -0.5] },
    TakerKind { name: "small takers", key: 102, genes: [0.477, -1.5, 0.6, -6.8, -0.824, 311.0, 0.4, 0.0] },
    TakerKind { name: "high takers", key: 103, genes: [1.778, -1.2, 0.4, -7.1, 0.0, 311.0, 0.4, 0.0] },
    TakerKind { name: "large low takers", key: 104, genes: [2.602, -1.3, 0.3, -7.3, -0.523, 311.0, 0.8, 0.0] },
    TakerKind { name: "cold low takers", key: 105, genes: [0.301, -1.7, 0.3, -6.0, -1.0, 250.0, 0.0, 0.5] },
];

/// Damuth's line of plant-eating mammals: log10 of the members per km² = `DAMUTH_A` − `DAMUTH_EXP` × log10 of the body
/// mass in g (Damuth 1981, Nature 290: 699, 307 species).
const DAMUTH_A: f64 = 4.23;
const DAMUTH_EXP: f64 = 0.75;
/// The takers start at this share of Damuth's line, so they grow into their range rather than fall into it.
const START_SHARE: f64 = 1.0 / 3.0;
/// The fuel a member starts with, kg per kg of its living mass: the low end of the stores of mammals before a winter,
/// 20 to 40% of lean mass (from memory).
pub const START_STORE: f64 = 0.1;
/// A founder slot's id (the key of its draw): its kind's key above `KIND_BIT`, its cell below.
const KIND_BIT: u32 = 40;

/// The members of a kind of living mass `m` kg in a cell of `area` m² at the start: `START_SHARE` of Damuth's line.
pub fn start_members(m: f64, area: f64) -> f64 {
    START_SHARE * 10f64.powf(DAMUTH_A - DAMUTH_EXP * (m * 1e3).log10()) * area / 1e6
}

/// Place every starting kind of takers (`TAKER_KINDS`) on the open land, each in every open land cell with its genes, its
/// members `start_members` on the cell's land (`open_land`) (whole below `N_WHOLE` by a draw keyed by the cell that keeps the mean). Returns a line per
/// kind.
pub fn place_takers(w: &mut World) -> Vec<String> {
    pcore::month_start(w);
    let mut said = Vec::new();
    let open = open_land(w);
    for (row, kind) in TAKER_KINDS.iter().enumerate() {
        let (mut cells, mut members, m) = (0, 0.0, fauna::adult(&kind.genes));
        for &(i, m2) in &open {
            let mut n = start_members(m, m2);
            if n < N_WHOLE {
                let id = (kind.key as u64) << KIND_BIT | i as u64;
                n = n.floor() + (Rng::new(w.seed, w.day(), id, Purpose::Place).f64() < n.fract()) as u8 as f64;
            }
            if fauna::place(w, i, (kind.genes, row as u32), (n, START_STORE * m)) {
                (cells, members) = (cells + 1, members + n);
            }
        }
        let g = &kind.genes;
        said.push(format!(
            "year {:.1}: placed the takers {} on {cells} cells ({members:.0} members of 10^{} kg, at {START_SHARE:.2} of Damuth's line, a store of {START_STORE} of their mass; newborn 10^{} of it, {:.0}% of a surplus to young, gut 10^{} mol per kg per s, reach 10^{} m, keeps {} K, coat {}, repair {})",
            w.year(), kind.name, g[MASS], g[CHILD], 100.0 * g[YOUNG], g[ENZYME], g[REACH], g[KEEP], g[COAT], g[REPAIR]
        ));
    }
    said
}

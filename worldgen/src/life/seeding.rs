//! Placing the cover of a starting kind: a spray of variants, the fixers among it, and the line that reports it.

use super::kinds::KINDS;
use super::open_land;
use pcore::rng::{Purpose, Rng};
use pcore::{cover, World};
use serde::{Deserialize, Serialize};

/// The share of the cells of the starting cover that carry the gene of the fixing reaction: one in ten, as fixing plants
/// are near a tenth of flowering species (docs/numbers.md). The gene then evolves as any other.
pub const FIXERS: f64 = 0.1;
/// The fixing gene of that cover (`cover::FIX`), log10 mol per kg of living cover per s: 10^-8.3, about 10 g of the
/// fixed element per m² a year for a kg of the starting cover per m² (2 kg living), what real fixing stands fix
/// (Cleveland et al. 1999).
pub const FIX_GENE: f32 = -8.3;
/// A founder's id (the key of its random streams): 1 + its kind's key above `KIND_BIT`, its cell above `CELL_BIT`.
const KIND_BIT: u32 = 48;
const CELL_BIT: u32 = 8;

fn founder_id(key: u32, i: usize) -> u64 {
    (1 + key as u64) << KIND_BIT | (i as u64) << CELL_BIT
}

/// How the first cover is placed: an initial condition, reported with every result (`seed_with` returns the line).
/// Every cell's cover gets `rounds` rounds of mutation, so the first generation is a spray of variants.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Seeding {
    /// Rounds of mutation given to the cover of every cell placed (0: all cells of a kind are the same)
    pub rounds: u32,
    /// The share of the cells of cover placed that carry the gene of the fixing reaction (`FIXERS`)
    pub fixers: f64,
}

impl Default for Seeding {
    fn default() -> Self {
        Seeding { rounds: 20, fixers: FIXERS }
    }
}

/// Place the cover of starting kind `which` (a row of `KINDS`) as `ew run` does (`Seeding::default`).
pub fn seed(w: &mut World, which: usize) -> String {
    seed_with(w, which, &Seeding::default())
}

/// Place the cover of starting kind `which` on the open land, its `kg` per m² of each cell's land (`open_land`), with
/// the choices of `how`, each cell by its chance: its
/// genes after `rounds` rounds of mutation (a random walk keyed by the cell), the fixing gene on in the share `fixers`
/// of the cells. Returns what was placed, in one line.
pub fn seed_with(w: &mut World, which: usize, how: &Seeding) -> String {
    let Some(kind) = KINDS.get(which) else { return String::new() };
    pcore::month_start(w);
    let mut r = Rng::new(w.seed, w.day(), kind.key as u64, Purpose::Place);
    let (mut cells, mut fixers, mut kg) = (0, 0, 0.0);
    for (i, m2) in open_land(w) {
        if r.f64() >= kind.chance {
            continue;
        }
        let id = founder_id(kind.key, i);
        let mut genes = kind.genes;
        let mut rg = Rng::new(w.seed, w.day(), id, Purpose::Make);
        for _ in 0..how.rounds {
            genes = pcore::life::kin::mutate(&genes, &cover::RANGE, w.cfg.phys.mut_size, &mut rg);
        }
        let fixes = Rng::new(w.seed, w.day(), id, Purpose::Fixer).f64() < how.fixers;
        if fixes {
            genes[cover::FIX] = FIX_GENE;
        }
        if pcore::cover::place(w, i, 0, genes, which as u32, kind.kg) {
            (cells, fixers, kg) = (cells + 1, fixers + fixes as usize, kg + kind.kg * m2);
        }
    }
    let g = &kind.genes;
    format!(
        "year {:.1}: placed the cover of {} on {cells} cells ({kg:.3e} kg, {:.2} kg per m2; light capacity 10^{} mol per kg living per s, capture area 10^{} m2 per kg living, walls 10^{} of its matter, fuel kept 10^{}, frost at {} K, active at moisture 10^{}, {:.0}% to seeds, repair {}), {} rounds of mutation each; {fixers} cells ({:.1}%) carry the fixing gene 10^{FIX_GENE} (share {} chosen before any run)",
        w.year(), kind.name, kind.kg, g[cover::LIGHT], g[cover::LEAF], g[cover::WALLS], g[cover::STORE], g[cover::FROST], g[cover::MOIST], 100.0 * g[cover::SEED], g[cover::REPAIR], how.rounds, 100.0 * fixers as f64 / cells.max(1) as f64, how.fixers
    )
}

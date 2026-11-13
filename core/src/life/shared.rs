//! What life shares: the causes it dies of (the cover's books count what it loses by them, `Counters::cover_lost`), the
//! liquid share of living tissue, and what a living thing is built of against breaking (laws.md 3).

use crate::chem::Chemistry;

/// Causes of death, the places of `Counters::cover_lost`: a store below its safe minimum; a key liquid froze; upkeep not
/// paid; taken by a taker (`contract::take`).
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Broken loose by work pressed on its cell (`contract::press`).
pub const BROKEN: u8 = 4;
/// Worn out by living (its wear, `cover::live`).
pub const AGE: u8 = 5;
/// The liquid share of living tissue: a kg of matter makes 1 / (1 - this) kg of living tissue (`Cover::living`).
pub const WET_TISSUE: f64 = 0.75;

/// What a living thing of `built` kg of soft tissue and `mol(k)` of each solid it builds is built of: [volume m³,
/// Σ volume × breaking stress × `cut(k)`], its soft tissue at `rho_body` (as substance `chem.matter()`). `cut(k)`: the
/// share of the energy to break substance `k` that is spent (1 blunt).
pub fn solids_of(chem: &Chemistry, cfg: &crate::Config, built: f64, mol: impl Fn(usize) -> f64, cut: impl Fn(usize) -> f64) -> [f64; 2] {
    let Some(m) = chem.matter() else { return [0.0; 2] };
    let soft = built.max(0.0) / cfg.phys.rho_body;
    let of = |v: f64, k: usize| [v, v * chem.subs[k].rupture * cut(k)];
    chem.materials().map(|k| of(mol(k).max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], k)).fold(of(soft, m), |a, x| [a[0] + x[0], a[1] + x[1]])
}

//! What all life shares, the cover and bodies alike: the causes it dies of, the liquid share of living tissue, and what
//! a living thing is built of against breaking (laws.md 4.5, 6.1, 6.8).

use crate::chem::Chemistry;

/// Death causes: a store below its safe minimum; a key liquid froze or boiled; upkeep not paid; taken by a body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Bonds broken by bodies moving through its cell.
pub const BROKEN: u8 = 4;
/// Worn out by living (its wear, `body::live`, `cover::live`).
pub const AGE: u8 = 5;
/// The liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of working body (`Body::built`,
/// `Cover::living`).
pub const WET_TISSUE: f64 = 0.75;

/// What a living thing of `built` kg of soft tissue (`Body::built`) and `mol(k)` of each solid it builds is built of:
/// [volume m³, Σ volume × breaking stress × `cut(k)`], its soft tissue at `rho_body` (as substance `chem.matter()`).
/// `cut(k)`: the share of the energy to break substance `k` that is spent (1 blunt).
pub fn solids_of(chem: &Chemistry, cfg: &crate::Config, built: f64, mol: impl Fn(usize) -> f64, cut: impl Fn(usize) -> f64) -> [f64; 2] {
    let Some(m) = chem.matter() else { return [0.0; 2] };
    let soft = built.max(0.0) / cfg.phys.rho_body;
    let of = |v: f64, k: usize| [v, v * chem.subs[k].rupture * cut(k)];
    chem.materials().map(|k| of(mol(k).max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], k)).fold(of(soft, m), |a, x| [a[0] + x[0], a[1] + x[1]])
}

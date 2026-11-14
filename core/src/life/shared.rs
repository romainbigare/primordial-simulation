//! What life shares: the causes it dies of (the cover's books count what it loses by them, `Counters::cover_lost`), the
//! liquid share of living tissue and what it takes to break it (laws.md 3).

/// Causes of death, the places of `Counters::cover_lost` and `Counters::takers_died`: a store below its safe minimum; a key liquid froze; upkeep not
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

/// The energy to break amounts `a` of living things (mol of each substance), J: each part's volume times its breaking
/// stress, the body matter as living tissue (soft, with its liquid, `WET_TISSUE`) at `rho_body`, and the solids life
/// builds (`Chemistry::materials`) at their own density; fuel is held inside the tissue and adds nothing.
pub fn breaking(chem: &crate::chem::Chemistry, ph: &crate::Physics, a: &[f64; crate::units::MAX_SUBSTANCES]) -> f64 {
    let Some(m) = chem.matter() else { return 0.0 };
    let stress = |v: f64, k: usize| v * chem.subs[k].rupture;
    let soft = stress((a[m] * chem.subs[m].molar_mass / (1.0 - WET_TISSUE)).max(0.0) / ph.rho_body, m);
    chem.materials().map(|k| stress(a[k].max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], k)).fold(soft, |s, x| s + x)
}

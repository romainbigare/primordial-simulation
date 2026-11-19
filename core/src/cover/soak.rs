//! How the cover's roots meet the liquid of their ground (laws.md 3): the liquid they can draw (`soak`, `Physics::available`)
//! and the oxidising gas their burning needs. Roots in pores full of liquid get that gas only down their own gas spaces,
//! from the air over the ground or over the liquid standing on it (Armstrong 1979), so roots past that length do not
//! work (`working`). A kind's gas gene (`GAS`) is the share of its roots' volume that is gas space.

use super::Parts;
use crate::chem::Phase;
use crate::units::{DAY, GAS_DIFFUSIVITY, GAS_DIFFUSIVITY_MM, R_GAS};

/// How the roots of a cell's cover meet its ground's liquid in a month (`soak`): the share of what its ground holds
/// for roots that it holds (`Physics::available`); the share of its pores its liquid fills (past 1 where liquid stands
/// over them); the depth of its loose ground, m; and the oxidising gas's spread through a root's gas spaces against
/// what the root's living tissue burns, m² (`breath`): a root of gas share φ keeps (2 breath φ / (1 − φ))^½ m of its
/// length in that gas (`aired`); and the key liquid that is liquid in the cell's ground outside its land, mol: what the
/// roots of plants standing under the liquid reach (`footing`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Soak {
    pub moist: f64,
    pub filled: f64,
    pub loose: f64,
    pub breath: f64,
    pub pond: f64,
}

/// Of `held` mol of the key liquid in the ground of cell `i`: what is liquid that its cover reaches, mol, and how its
/// roots meet it (`Soak`), its pores' space read from Flow (`Scratch::land_pores`; one store over the depth of its loose
/// ground). Of a cell of parts with land, its land part's own (`flow::parts`) over that part's area, no more than the
/// cell holds; else the cell's over its area.
pub fn soak(w: &crate::World, p: &Parts, i: usize, held: f64) -> (f64, Soak) {
    let (t, plateau) = w.thaw_last(i);
    let molten = w.molten(p.liquid, i, t, plateau);
    let area = w.mesh.cells().area()[i];
    let (liquid, over) = match w.scratch.land_part(i, p.liquid) {
        Some((mol, share)) => (mol.min(held) * molten, area * share),
        None => (held * molten, area),
    };
    let ph = &w.cfg.phys;
    let depth = liquid * w.chem.molar_volume(p.liquid, Phase::Liquid) / over;
    let pores = w.scratch.land_pores(i);
    let filled = depth / pores.max(1e-9);
    let loose = if ph.porosity > 0.0 { pores / ph.porosity } else { 0.0 };
    let pond = (held * molten - liquid).max(0.0);
    (liquid, Soak { moist: ph.available(filled), filled, loose, breath: breath(w, p, i, t), pond })
}

/// The oxidising gas's spread through a root's gas spaces against what the root's living tissue burns, at the ground
/// of cell `i` at `t` K, m²: D C / M of Armstrong (1979)'s longest root, L = (2 D φ C / M)^½, with D the gas's
/// diffusivity in the air (`units::GAS_DIFFUSIVITY` by Graham's law, as `pores` scales it), C its concentration in the
/// air at the ground (`World::near_ground` over R T), M what a m³ of living root burns of it a second: fine roots
/// breathe `c_basal` a kg a day at the burning's pace (`upkeep`), at `rho_body` kg a m³, over the energy the burning
/// frees per mol of that gas. None where the air holds none or the burning takes no gas.
pub fn breath(w: &crate::World, p: &Parts, i: usize, t: f64) -> f64 {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let burn = &chem.reactions[p.burn];
    let Some(&(ox, n)) = burn.inputs.iter().find(|q| chem.phase(q.0 as usize, t) == Phase::Gas) else { return 0.0 };
    let ox = ox as usize;
    let Some(p_ox) = w.near_ground(i, ox) else { return 0.0 };
    let d = GAS_DIFFUSIVITY * (GAS_DIFFUSIVITY_MM / chem.subs[ox].molar_mass).sqrt();
    let c = p_ox / (R_GAS * t.max(1.0));
    let pace = burn.arrhenius(t.max(1.0)) / burn.arrhenius(ph.t_cat);
    let per_mol = -chem.dh(p.burn) / n as f64;
    let m = ph.c_basal / DAY * pace * ph.rho_body / per_mol.max(1e-300);
    d * c / m.max(1e-300)
}

/// The length of a root of gas share `phi` that its gas spaces keep in the oxidising gas past where the pores fill, m
/// (Armstrong 1979, `Soak::breath`): its gas spaces carry the gas, its living share (1 − φ) burns it.
pub fn aired(breath: f64, phi: f64) -> f64 {
    (2.0 * breath * phi / (1.0 - phi).max(1e-9)).max(0.0).sqrt()
}

/// The share of the roots of a kind of gas gene `phi`, rooted `need` m deep, that work on ground `s`: its roots stand in
/// its loose ground (no deeper than it). Up to field capacity (`Physics::field`) its liquid is held in the small pores
/// all through it and the large ones hold the gas, so every root breathes; what it holds over that fills the rest of
/// its pores from the bottom up (Flow's one store over the depth of its loose ground), and over its top it stands. The
/// roots above that table breathe the gas of the open pores, and of those below it the length the gas spaces keep in
/// the gas (`aired`), less the depth of liquid standing over the ground, which that gas crosses first (Armstrong 1979;
/// Feddes, Kowalik and Zaradny 1978: none at saturation for a plant with no gas spaces).
pub fn working(phi: f64, need: f64, s: &Soak, ph: &crate::Physics) -> f64 {
    let roots = need.min(s.loose).max(1e-9);
    let full = ((s.filled - ph.field) / (1.0 - ph.field).max(1e-9)).clamp(0.0, 1.0);
    let open = (1.0 - full) * s.loose;
    let standing = (s.filled - 1.0).max(0.0) * s.loose * ph.porosity;
    ((open + aired(s.breath, phi) - standing) / roots).clamp(0.0, 1.0)
}

/// Where a kind of cover stands in a cell (`footing`): the share of the cell's ground (`cover::ground`) it stands on;
/// the mean share of its roots that work there (`working`); and the share of that ground that lies under the liquid.
#[derive(Clone, Copy, Debug, Default)]
pub struct Footing {
    pub share: f64,
    pub work: f64,
    pub wet: f64,
}

/// Where a kind of gas gene `phi`, rooted `need` m deep, stands in cell `i` of `w`, whose land meets its roots as `land`
/// (`soak`): on its land, and on each band of its ground under the liquid (`contract::wet`) where its roots work, its
/// pores full and the band's mean depth standing over them (`working`; its loose ground taken as its land's, and none
/// where that holds none): one rule for where a plant stands, on land or in
/// the liquid. A kind of few gas spaces stands on land alone; one of many stands out into the liquid as far as its
/// gas reaches down its roots (Armstrong 1979), as swamp trees and emergent herbs stand in the liquid to a depth their kind
/// tolerates (Kozlowski 1997).
pub fn footing(w: &crate::World, i: usize, phi: f64, need: f64, land: &Soak) -> Footing {
    let ph = &w.cfg.phys;
    let l = crate::contract::land(w, i);
    let (mut on, mut work, mut wet, mut all) = (l, l * working(phi, need, land, ph), 0.0, l);
    for &[area, depth] in crate::contract::wet(w, i) {
        all += area;
        let under = Soak { moist: 1.0, filled: 1.0 + depth / (land.loose * ph.porosity).max(1e-9), ..*land };
        let works = working(phi, need, &under, ph);
        if works > 0.0 && land.loose > 0.0 {
            (on, work, wet) = (on + area, work + area * works, wet + area);
        }
    }
    if on <= 0.0 {
        return Footing::default();
    }
    Footing { share: on / all, work: work / on, wet: wet / on }
}

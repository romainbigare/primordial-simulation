//! How the cover's roots meet the liquid of their ground (laws.md 3): the liquid they can draw (`soak`, `Pores::available`)
//! and the oxidising gas their burning needs. Roots in pores full of liquid get that gas only down their own gas spaces,
//! from the air over the ground or over the liquid standing on it (Armstrong 1979), so roots past that length do not
//! work (`working`). A kind's gas gene (`GAS`) is the share of its roots' volume that is gas space.

use super::Parts;
use crate::chem::Phase;
use crate::units::{DAY, GAS_DIFFUSIVITY, GAS_DIFFUSIVITY_MM, R_GAS};

/// How the roots of a cell's cover meet its ground's liquid in a month (`soak`): the share of what its ground holds
/// for roots that it holds (`Pores::available`); the share of its pores its liquid fills (past 1 where liquid stands
/// over them); the share of its land saturated (`inside::saturated_share`); the depth of its loose ground, m, and its
/// pores (`World::pores`); and the oxidising gas's spread through
/// a root's gas spaces against what the root's living tissue burns, m² (`breath`): a root of gas share φ keeps
/// (2 breath φ / (1 − φ))^½ m of its length in that gas (`aired`); and the key liquid that is liquid in the cell's
/// ground outside its land, mol: what the roots of plants standing under the liquid reach (`footing`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Soak {
    pub moist: f64,
    pub filled: f64,
    pub saturated: f64,
    pub loose: f64,
    pub pores: crate::soil::Pores,
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
    let (soil, depth) = (*w.pores(i), liquid * w.chem.molar_volume(p.liquid, Phase::Liquid) / over);
    let pores = w.scratch.land_pores(i);
    let filled = depth / pores.max(1e-9);
    let loose = if soil.porosity > 0.0 { pores / soil.porosity } else { 0.0 };
    let pond = (held * molten - liquid).max(0.0);
    let saturated = crate::inside::saturated_share(w, i);
    (liquid, Soak { moist: soil.available(filled), filled, saturated, loose, pores: soil, breath: breath(w, p, i, t), pond })
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
    let pace = burn.relative_rate(t, ph.t_cat);
    let per_mol = -chem.dh(p.burn) / n as f64;
    let m = ph.c_basal / DAY * pace * ph.rho_body / per_mol.max(1e-300);
    d * c / m.max(1e-300)
}

/// The length of a root of gas share `phi` that its gas spaces keep in the oxidising gas past where the pores fill, m
/// (Armstrong 1979, `Soak::breath`): its gas spaces carry the gas, its living share (1 − φ) burns it.
pub fn aired(breath: f64, phi: f64) -> f64 {
    (2.0 * breath * phi / (1.0 - phi).max(1e-9)).max(0.0).sqrt()
}

/// The share of the roots of a kind of gas gene `phi`, rooted `need` m deep, that work on ground `s`, on its land's
/// saturated strips or on its drier rest (`strips`): its roots stand in its loose ground (no deeper than it). The share
/// of that ground whose pores hold no gas is where its table lies (`inside::airless`: all of the strips; of the rest,
/// what it holds over field capacity, which fills its pores from the bottom up, `inside::rest`), and over its top the
/// liquid stands. The roots above that table breathe the gas of the open pores, and of those below it the length the
/// gas spaces keep in the gas (`aired`), less the depth of liquid standing over the ground, which that gas crosses
/// first (Armstrong 1979; Feddes, Kowalik and Zaradny 1978: none at saturation for a plant with no gas spaces). A
/// land with no strips saturated is all rest.
pub fn working(phi: f64, need: f64, s: &Soak, strips: bool) -> f64 {
    let strips = strips && s.saturated > 0.0;
    let roots = need.min(s.loose).max(1e-9);
    let full = if strips { 1.0 } else { s.pores.full(crate::inside::rest(s.saturated, s.filled)) };
    let open = (1.0 - full) * s.loose;
    let standing = (s.filled - 1.0).max(0.0) * s.loose * s.pores.porosity;
    ((open + aired(s.breath, phi) - standing) / roots).clamp(0.0, 1.0)
}

/// The share of what its ground holds for roots that the strips (`strips`) or the drier rest of land `s` hold
/// (`Pores::available`): the strips' pores are full.
pub fn moist(s: &Soak, strips: bool) -> f64 {
    if strips && s.saturated > 0.0 { 1.0 } else { s.pores.available(crate::inside::rest(s.saturated, s.filled)) }
}

/// How a kind of genes `genes`, rooted `need` m deep, gains on the drier rest and on the saturated strips of land `s`,
/// [rest, strips]: the share of its roots that work there (`working`) and its activity there (`cover::activity`, by
/// `moist`); and its activity on each. Its light reaction runs as both let it (laws.md 3).
pub fn gains(genes: &[f32; super::NG], need: f64, s: &Soak) -> ([f64; 2], [f64; 2]) {
    let phi = genes[super::GAS] as f64;
    let act = [false, true].map(|strips| super::activity(genes, moist(s, strips)));
    let work = [false, true].map(|strips| working(phi, need, s, strips));
    ([work[0] * act[0], work[1] * act[1]], act)
}

/// The share of a kind's plants on a land's saturated strips, of the share `on` of them there before, where on each
/// ground they gain `gain` [rest, strips] a plant (`gains`) and the strips are the share `saturated` of the land: of
/// plants that land evenly over it (`on` the land's share in the strips), those that take hold, as they gain; of plants
/// that grew, those that grew, as they gain. The share stays where neither gains.
pub fn strips_share(on: f64, gain: [f64; 2]) -> f64 {
    let all = (1.0 - on) * gain[0] + on * gain[1];
    if all > 0.0 { on * gain[1] / all } else { on }
}

/// Where a kind of cover stands in a cell (`footing`): its line, m over the cell's lowest ground (`Cover::line`), and the
/// share of the cell's ground (`cover::ground`) above it; the mean share of its roots that work there (`working`); the
/// share of its plants on its land that stand on the land's saturated strips (`Cover::strip`), what a plant gains on its
/// land's rest and strips (`gains`) and its activity on its land, their mean by its plants; the
/// share of that ground that lies under the liquid; the share of its plants on ground under the liquid past its roots'
/// reach (`aired`), lost where its line rises to `rise`, m, over which none lies; and the ground below its line where
/// its roots get the oxidising gas, m², which only its seedlings take (`lying`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Footing {
    pub line: f64,
    pub share: f64,
    pub work: f64,
    pub strip: f64,
    pub gain: [f64; 2],
    pub act: f64,
    pub wet: f64,
    pub airless: f64,
    pub rise: f64,
    pub free: f64,
}

/// Where a kind of genes `genes`, rooted `need` m deep, stands in cell `i` of `w`, whose land meets its roots as `land`
/// (`soak`): on the cell's ground above its line, with the share of its plants on its land in the saturated strips,
/// `held` (none: the line `found` of all the ground it may hold, where its seeds found a stand, and of seeds landing
/// evenly over the land those that take hold, `strips_share`), no more than the strips' share of the land: its plants
/// stand over all its land (`Cover::stands`), no denser on the strips than on the land as a whole. Its
/// roots work on its land as the land's liquid lets them (`working`, on each ground by its plants there), and on its ground
/// under the liquid, its pores full, as far as their gas spaces reach down them past the depth of the liquid standing
/// over it (Armstrong 1979; its loose ground taken as its land's, and none where that holds none): one rule for where a
/// plant's roots work, on land or in the liquid, as swamp trees and emergent herbs stand in the liquid to a depth their
/// kind tolerates (Kozlowski 1997). Plants do not move: its line holds where they stand while the level moves, so a
/// rising level stands them in deeper liquid, where its roots past their reach work no more (`Cover::airless`); where
/// no ground lies above its line, it holds the cell's highest ground (`Shore::holding`). Before the month's start found
/// the cell's levels it stands on its land alone.
pub fn footing(w: &crate::World, i: usize, genes: &[f32; super::NG], need: f64, land: &Soak, held: Option<(f64, f64)>) -> Footing {
    let phi = genes[super::GAS] as f64;
    let (gain, act) = gains(genes, need, land);
    let strip = held.map_or_else(|| strips_share(land.saturated, gain), |h| h.1).min(land.saturated);
    let work = [false, true].map(|strips| working(phi, need, land, strips));
    let on_land = (1.0 - strip) * work[0] + strip * work[1];
    let mine = Footing { strip, gain, act: (1.0 - strip) * act[0] + strip * act[1], ..Footing::default() };
    let line = held.map(|h| h.0);
    let levels = crate::contract::levels(w, i);
    if levels.is_empty() {
        let l = crate::contract::land(w, i);
        return if l > 0.0 { Footing { share: 1.0, work: on_land, ..mine } } else { Footing::default() };
    }
    let s = crate::flow::shore(w, i, levels);
    let reach = if land.loose > 0.0 { aired(land.breath, phi) } else { 0.0 };
    let h = line.map_or_else(|| s.found(reach), |l| s.holding(l));
    let f = s.feet(h, reach, need.min(land.loose).max(1e-9));
    let rest = if f.airless > 0.0 { 1.0 - s.above(f.rise) / f.ground } else { 0.0 };
    Footing { line: h, share: f.ground / s.area(), work: (f.land * on_land + f.work) / f.ground, wet: f.wet / f.ground, airless: rest.clamp(0.0, 1.0), rise: f.rise, free: f.free, ..mine }
}

/// The line of a slot of cover `c` (`Cover::line`) whose seedlings of `n` plants of `mass` kg each took hold on the
/// ground its kind stands on, `aired` m² of cell `i`, of which `free` lies below its line (`Footing::free`): the seeds
/// land evenly over it, and those below its line take the ground they hold as dense as the self-thinning law lets
/// them stand (`stand::plant_ground`), no more than lies there; the line falls to hold that ground too.
pub(super) fn lowered(w: &crate::World, i: usize, line: f64, (n, mass): (f64, f64), (aired, free): (f64, f64)) -> f64 {
    let levels = crate::contract::levels(w, i);
    if levels.is_empty() || line < 0.0 || free <= 0.0 || aired <= 0.0 {
        return line;
    }
    let s = crate::flow::shore(w, i, levels);
    let taken = (n * free / aired * super::stand::plant_ground(&w.cfg.phys, mass)).min(free);
    s.line_of(s.above(line) + taken).min(line)
}

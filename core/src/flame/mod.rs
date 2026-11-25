//! Flames in the remains lying on the ground (laws.md 3c): once a month, after the cover's month, the remains of life
//! on a land cell carry a flame where they cover its ground without gaps and are dry enough that the heat a burning kg
//! sends ahead dries and lights the next kg (Rothermel 1972); a strike lights the joined remains of a region, and the
//! flame runs over them. What burns goes through the chemistry's own burning reactions; the speed comes from the heat
//! balance, not from the reactions' rates (theirs are those of rot). Three stages, each number with one writer: each
//! cell finds the bed of each patch of its cover (`bed`, in parallel), the cells that carry a flame join into regions in
//! the order of the cells (`join`, serial), and each cell burns its own share on each patch (`burn`, one task per chunk
//! of cells).

mod bed;
mod burn;
mod join;

pub use bed::{beds, heated, Bed};
pub use burn::{burn, burnings, reach, Burning, Reach};
pub use join::{shares, Lit};

use crate::cover::Parts;
use crate::store::{Move, Node};
use crate::World;
use serde::{Deserialize, Serialize};

/// Remains that cover more than this share of a cell's ground join across it: the site-percolation threshold of the
/// square lattice, 0.592746 (Newman and Ziff 2000; Stauffer and Aharony 1994), a constant of mathematics.
pub const CARRY_SHARE: f64 = 0.592_746;
/// The temperature the sorption curve's numbers are given at (`Physics::sorb_scale`, `sorb_power`): that of the fit
/// they are taken from, 26.7 degrees over the key liquid's melting point (Nelson 2000).
pub const SORB_T: f64 = 299.85;
/// The relative vapour the sorption curve reads at most: at saturation its moisture has no bound (Nelson 2000 sets 0.99).
pub const SORB_HIGHEST: f64 = 0.99;
/// How long a fine particle of the remains flames, s per m of its width: 8 minutes an inch (Anderson 1969).
pub const RESIDENCE_PER_M: f64 = 8.0 * 60.0 / 0.0254;
/// How long a fine particle of the remains flames, s: `RESIDENCE_PER_M` × `fine_d` (Anderson 1969).
pub fn residence(ph: &crate::Physics) -> f64 {
    RESIDENCE_PER_M * ph.fine_d
}

/// Rothermel's (1972) wind factor φ_w = C U^B (β / β_op)^−E, U in feet a minute and the particles' surface to volume
/// ratio σ in 1/ft: C = `WIND_C` e^(`WIND_C_EXP` σ^`WIND_C_POW`), B = `WIND_B` σ^`WIND_B_POW`, E = `WIND_E`
/// e^(`WIND_E_EXP` σ), β_op = `PACK_OP` σ^`PACK_OP_POW`.
const WIND_C: f64 = 7.47;
const WIND_C_EXP: f64 = -0.133;
const WIND_C_POW: f64 = 0.55;
const WIND_B: f64 = 0.02526;
const WIND_B_POW: f64 = 0.54;
const WIND_E: f64 = 0.715;
const WIND_E_EXP: f64 = -3.59e-4;
const PACK_OP: f64 = 3.348;
const PACK_OP_POW: f64 = -0.8189;
/// Metres in a foot, and seconds in a minute, for Rothermel's fits.
const M_PER_FT: f64 = 0.3048;
const S_PER_MIN: f64 = 60.0;

/// How many times its speed with no wind a flame through `bed` moves along the unit way `way` [east, south] under the
/// wind `wind`, m/s [east, south]: 1 + φ_w times the share of the wind along the way (none against it; Rothermel 1972,
/// the factor of the wind along the way), φ_w of the wind's speed along it, the particles' σ = 4 / `fine_d` (round
/// fine particles) and the packing of what flames, its bulk density (`Bed::flames` over `Bed::depth`) over the
/// particles' (the frame's density).
pub fn downwind(ph: &crate::Physics, bed: &Bed, wind: [f64; 2], way: [f64; 2]) -> f64 {
    let along = wind[0] * way[0] + wind[1] * way[1];
    if along <= 0.0 || bed.depth(ph) <= 0.0 {
        return 1.0;
    }
    let sigma = 4.0 / ph.fine_d * M_PER_FT;
    let packing = bed.flames() / bed.depth(ph) / bed.particle_rho;
    let c = WIND_C * (WIND_C_EXP * sigma.powf(WIND_C_POW)).exp();
    let (b, e) = (WIND_B * sigma.powf(WIND_B_POW), WIND_E * (WIND_E_EXP * sigma).exp());
    let u = along / M_PER_FT * S_PER_MIN;
    1.0 + c * u.powf(b) * (packing / (PACK_OP * sigma.powf(PACK_OP_POW))).powf(-e)
}

/// Byram's (1959) flame: L = `FLAME_K` I^`FLAME_EXP` m, I the intensity in kW a m of front.
pub const FLAME_K: f64 = 0.0775;
pub const FLAME_EXP: f64 = 0.46;
/// Van Wagner's (1973) scorch height: `SCORCH_K` I^`SCORCH_EXP` m, I in kW a m (his 13 burns of 70 to 1,250 kW/m).
pub const SCORCH_K: f64 = 0.1483;
pub const SCORCH_EXP: f64 = 2.0 / 3.0;
/// W in a kW, for the fits in kW a m.
pub const W_PER_KW: f64 = 1e3;
/// Work a cell's bed takes to find, in instructions, for the pool (`par::map_n`).
const BED_WORK: usize = 2_000;

/// What flames burnt and killed in the cover (`Counters::flame`), kg (plants: a count): the remains on the ground that
/// flamed and those that smouldered after them; the cured leaves standing (last, `CURED`); the
/// soft part of the cover that stood in the flame; the leaves under the scorch height that died; what else died of the
/// plants whose whole crown stood under it (their walls and fuel); of that their walls; and those plants.
pub const FLOWS: [&str; 8] = ["remains", "smouldered", "living", "scorched", "killed", "killed_walls", "plants", "cured"];
pub const REMAINS: usize = 0;
pub const SMOULDERED: usize = 1;
pub const LIVING: usize = 2;
pub const SCORCHED: usize = 3;
pub const KILLED: usize = 4;
pub const KILLED_WALLS: usize = 5;
pub const PLANTS: usize = 6;
pub const CURED: usize = 7;

/// What flames did on a cell in a month, for the tape: the share of its land they burnt; their mean length, m; the
/// solids their burning left on its ground and the gas it gave the air, kg a m² of its land; and the share of the soft
/// tissue of the cover standing on the land they burnt that they burnt or killed.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Flamed {
    pub share: f32,
    pub length: f32,
    pub ash: f32,
    pub air: f32,
    pub crown: f32,
}

/// The most months since a cell last burnt that the tape tells apart (`since`): a cell that never burnt counts as this
/// old.
pub const AGE_MOST: f32 = 120.0;

/// Why a cell's remains carry no flame (`lights`, `bed`, `Bed::unlit`): it holds no land; the chemistry has no
/// cover; nothing lies on it that burns; its remains cover too little of its ground to join across it (`CARRY_SHARE`);
/// they are too few to hold a flame's heat even dry (`Bed::least`); snow lies over them; the air near its ground is
/// not read yet; its ground is too wet, so much of its land saturated that its remains on the rest no longer join;
/// or they hold so much liquid that too little of them is dry enough to flame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unlit {
    NoLand,
    Lifeless,
    Bare,
    Sparse,
    Thin,
    Snow,
    Unread,
    Wet,
    Damp,
}

/// Whether a strike on cell `i` lights it, by the test the flames' month uses (`beds`, `Bed::carries`): where the bed
/// of some patch of its cover carries a flame; else why the largest patch's does not.
pub fn lights(w: &World, i: usize) -> Result<(), Unlit> {
    let p = Parts::of(&w.chem).ok_or(Unlit::Lifeless)?;
    if crate::contract::land(w, i) <= 0.0 {
        return Err(Unlit::NoLand);
    }
    let beds = beds(w, &p, i);
    if beds.iter().flatten().any(Bed::carries) {
        return Ok(());
    }
    let c = w.life.cover(i);
    let largest = (0..beds.len()).max_by(|&a, &b| c.patches[a].share.total_cmp(&c.patches[b].share)).ok_or(Unlit::Bare)?;
    Err(beds[largest].map_or_else(|why| why, |b| b.unlit().unwrap_or(Unlit::Damp)))
}

/// The flames' month, after the cover's (`life::month`): the flames of the month (`flames`), and what each cell's flames
/// did kept for the tape (`tally`).
pub fn month(w: &mut World) {
    if flames(w, w.cfg.calendar().month as f64).is_some() {
        tally(w, w.mesh.cells().n());
    }
}

/// The flames of the strikes of edits and of the heat they added alone, burnt at once with no time on the world's clock
/// (`World::settle`, laws.md 0 and 3c): over no time no strike is drawn. They burn in the coming month: what they did is
/// shown as its own now (`show`) and kept in it (`Counters::flamed`, which its tally takes). Returns each cell their
/// flames reached and when, s after its strike or its heat.
pub fn now(w: &mut World) -> Vec<(u32, f64)> {
    let reached = flames(w, 0.0).unwrap_or_default();
    let flamed = w.counts.flamed.clone();
    let (n, (months, of_year)) = (w.mesh.cells().n(), of_year(w));
    sized(&mut w.counts, n, months);
    show(&mut w.counts, &flamed, (months, of_year));
    reached
}

/// The heat that lights the remains of land cell `i` over the share `over` of it where edits add heat (`heated`): what
/// warms and boils off the liquid they hold and warms them to `t_light` on every patch, J; or why none lights: no
/// land, no cover in the chemistry, nothing lying that burns, snow over it, or the air near the ground not read yet.
pub fn needed(w: &World, i: usize, over: f64) -> Result<f64, Unlit> {
    let p = Parts::of(&w.chem).ok_or(Unlit::Lifeless)?;
    if crate::contract::land(w, i) <= 0.0 {
        return Err(Unlit::NoLand);
    }
    let beds = heated(w, &p, i, (f64::INFINITY, over));
    let all: f64 = beds.iter().flatten().map(|b| b.1).sum();
    match beds.iter().find_map(|b| b.as_ref().err()) {
        Some(&why) if all <= 0.0 => Err(why),
        _ => Ok(all),
    }
}

/// The flames over `span` s: the bed of each patch of each land cell (`beds`), of a cell edits added heat to its beds
/// as that heat dries and lights them (`heated`, `World::heated`, which it then forgets); the share of each cell that
/// burns (`shares`, the strikes of edits and the heated cells with the strikes drawn over the span, which it then
/// forgets), the flames on its patches (`burnings`, serial) and the burning of them (`burn`), on the chunks of `life`
/// only where some burns; last, the heat the heated beds took leaves their ground for the air over them, as the heat
/// of their burning does. Returns each cell the flames reached and when, s after its strike or its heat (`Lit::at`);
/// none where the chemistry has no cover.
fn flames(w: &mut World, span: f64) -> Option<Vec<(u32, f64)>> {
    let p = Parts::of(&w.chem)?;
    let n = w.mesh.cells().n();
    let mut beds: Vec<Vec<Option<Bed>>> = crate::par::map_n(n, n * BED_WORK, |i| if crate::contract::land(w, i) > 0.0 { beds(w, &p, i).into_iter().map(Result::ok).collect() } else { Vec::new() });
    let (mut lit_by_heat, mut took) = (Vec::new(), Vec::new());
    for (i, [e, over]) in std::mem::take(&mut w.heated) {
        let (i, land) = (i as usize, crate::contract::land(w, i as usize));
        if land <= 0.0 || over <= 0.0 {
            continue;
        }
        let hot = heated(w, &p, i, (e / (land * over), over));
        if hot.iter().any(Result::is_ok) {
            let [heat, liquid] = hot.iter().flatten().fold([0.0; 2], |a, b| [a[0] + b.1, a[1] + b.2]);
            took.push((i, heat.min(e), liquid));
            beds[i] = hot.into_iter().map(|b| b.ok().map(|b| b.0)).collect();
            lit_by_heat.push(i);
        }
    }
    let lit = shares(w, &beds, span, &lit_by_heat);
    w.strikes.clear();
    let on: Vec<Vec<Burning>> = (0..n).map(|i| if lit.share[i] > 0.0 { burnings(w, i, &beds[i]) } else { Vec::new() }).collect();
    let load = on.iter().filter(|f| !f.is_empty()).count();
    if load > 0 {
        crate::life::run(w, load, |w, d| {
            for i in d.cells().filter(|&i| !on[i].is_empty()) {
                burn(w, d, &p, i, &on[i]);
            }
        });
    }
    if !took.is_empty() {
        boiled(w, p.liquid, &took);
    }
    Some(lit.at.iter().enumerate().filter(|p| p.1.is_finite()).map(|(i, &t)| (i as u32, t)).collect())
}

/// The heat the heated beds took and the liquid they boiled off (`took`: each cell, J, kg) leave their ground: the
/// liquid as vapour into the cell's aside node, which the air's step lets into the air (laws.md 1.5), carrying its heat
/// at the ground's temperature and its latent heat, as the ground's evaporation does; the rest of the heat into the air
/// over the cell, as the heat of the burning does. No more liquid leaves than the ground holds.
fn boiled(w: &mut World, liquid: usize, took: &[(usize, f64, f64)]) {
    use crate::chem::Heat;
    let s = &w.chem.subs[liquid];
    let moves: Vec<Move> = took.iter().flat_map(|&(i, heat, kg)| {
        let (t, molten) = (w.thaw_last(i).0, w.molten_last(liquid, i));
        let mol = (kg / s.molar_mass).min(w.g(liquid, i).max(0.0));
        // The vapour takes its own heat from the ground, and the latent heat that lifts it out of its phase from the heat added.
        let vapour = Move { from: Node::ground(i), to: Node::aside(i), k: liquid as u16, mol, heat: mol * (s.cp() * t + s.latent_gas()) };
        let rest = (heat - mol * (s.latent_gas() - s.lf() * molten)).max(0.0);
        [vapour, Move { from: Node::ground(i), to: Node::air(w.mesh.up(i, 0, crate::AIR)), k: 0, mol: 0.0, heat: rest }]
    }).collect();
    crate::store::apply(w, &[], &moves);
}

/// The months of a year and this month's place in it.
fn of_year(w: &World) -> (usize, usize) {
    let months = w.cfg.months_per_year as usize;
    (months, (w.month() % months as u64) as usize)
}

/// The tape's records of the flames sized for `n` cells and `months` a year: a cell new to them never burnt.
fn sized(c: &mut crate::Counters, n: usize, months: usize) {
    c.flame_month.resize(n, Flamed::default());
    c.burnt_months.resize(n * months, 0.0);
    c.burnt_age.resize(n, AGE_MOST);
    c.burnt_last.resize(n, [0.0; 2]);
}

/// What each cell's flames did in the month (`Counters::flamed`) becomes its last month's (`Counters::flame_month`,
/// one a cell of `n`) and the share it burnt that month of the year's (`burnt_months`); its months since it last burnt
/// (`burnt_age`) grow by the month, at most `AGE_MOST`, and start again from none where it burns. For the tape.
fn tally(w: &mut World, n: usize) {
    let (months, of_year) = of_year(w);
    let c = &mut w.counts;
    c.flame_month.clear();
    sized(c, n, months);
    for i in 0..n {
        c.burnt_months[i * months + of_year] = 0.0;
        c.burnt_age[i] = (c.burnt_age[i] + 1.0).min(AGE_MOST);
    }
    let flamed = std::mem::take(&mut c.flamed);
    show(c, &flamed, (months, of_year));
}

/// What the flames `flamed` did on each cell shown as this month's: its last month's (`Counters::flame_month`), the
/// share it burnt this month of the year's (`burnt_months`, of `months` a cell, this month `of_year`), none since it
/// last burnt (`burnt_age`), and its last burn's share and the share of the soft tissue on it killed (`burnt_last`).
fn show(c: &mut crate::Counters, flamed: &[(u32, Flamed)], (months, of_year): (usize, usize)) {
    for &(i, f) in flamed {
        let i = i as usize;
        (c.flame_month[i], c.burnt_months[i * months + of_year]) = (f, f.share);
        c.burnt_age[i] = 0.0;
        c.burnt_last[i] = [f.share, f.crown];
    }
}

/// The cured leaves standing on cell `i` (`Cover::cured`), kg a m² of its land (none where it holds no land).
pub fn cured(w: &World, i: usize) -> f64 {
    per_land(w, i, &w.life.cover(i).cured)
}

/// The dead walls flames left standing on cell `i` (`Patch::snag`, of all its patches), kg a m² of its land (none where
/// it holds no land).
pub fn snag(w: &World, i: usize) -> f64 {
    let mut all = [0.0; crate::cover::NP];
    w.life.cover(i).patches.iter().for_each(|p| all.iter_mut().zip(&p.snag).for_each(|(a, m)| *a += m));
    per_land(w, i, &all)
}

/// Of each patch of cell `i`'s cover, the dead walls flames left standing on it (`Patch::snag`), kg a m² of its own
/// land, and the height they stood to, m (0 and 0 for a patch not in use, or where the cell holds no land).
pub fn snag_of_patches(w: &World, i: usize) -> [(f64, f64); crate::cover::PATCHES] {
    let c = w.life.cover(i);
    std::array::from_fn(|k| {
        let p = &c.patches[k];
        if p.share <= 0.0 {
            return (0.0, 0.0);
        }
        (per_land(w, i, &p.snag) / p.share, p.snag_h as f64)
    })
}

/// The mass of the amounts `mol` of the cover's parts on cell `i`, kg a m² of its land (none where it holds no land).
fn per_land(w: &World, i: usize, mol: &[f64; crate::cover::NP]) -> f64 {
    let (land, p) = (crate::contract::land(w, i), Parts::of(&w.chem));
    p.filter(|_| land > 0.0).map_or(0.0, |p| mol.iter().zip(p.mm(&w.chem)).map(|(m, kg)| m * kg).sum::<f64>() / land)
}

/// The share of cell `i`'s land flames burnt in the last year's months (a share burnt twice counts twice).
pub fn burnt_year(w: &World, i: usize) -> f64 {
    let m = w.cfg.months_per_year as usize;
    w.counts.burnt_months.get(i * m..(i + 1) * m).map_or(0.0, |v| v.iter().map(|&f| f as f64).sum())
}

/// The whole months since cell `i` last burnt, at most `AGE_MOST` (a cell that never burnt counts as that old).
pub fn since(w: &World, i: usize) -> f64 {
    w.counts.burnt_age.get(i).copied().unwrap_or(AGE_MOST) as f64
}

/// Of cell `i`'s last burn: the share of its land it burnt, and the share of the soft tissue standing there it burnt
/// or killed (both 0 where it never burnt).
pub fn last_burn(w: &World, i: usize) -> [f64; 2] {
    w.counts.burnt_last.get(i).copied().unwrap_or_default().map(f64::from)
}

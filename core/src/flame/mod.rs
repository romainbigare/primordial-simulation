//! Flames in the remains lying on the ground (laws.md 3c): once a month, after the cover's month, the remains of life
//! on a land cell carry a flame where they cover its ground without gaps and are dry enough that the heat a burning kg
//! sends ahead dries and lights the next kg (Rothermel 1972); a strike lights the joined remains of a region, and the
//! flame runs over them. What burns goes through the chemistry's own burning reactions; the speed comes from the heat
//! balance, not from the reactions' rates (theirs are those of rot). Three stages, each number with one writer: each
//! cell finds its own bed (`bed`, in parallel), the cells that carry a flame join into regions in the order of the cells
//! (`join`, serial), and each cell burns its own share (`burn`, one task per chunk of cells).

mod bed;
mod burn;
mod join;

pub use bed::{bed, Bed};
pub use burn::{burn, reach, Reach};
pub use join::shares;

use crate::cover::Parts;
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
/// solids their burning left on its ground and the gas it gave the air, kg a m² of its land.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Flamed {
    pub share: f32,
    pub length: f32,
    pub ash: f32,
    pub air: f32,
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

/// Whether a strike on cell `i` lights it, by the test the flames' month uses (`bed`, `Bed::carries`); else why not.
pub fn lights(w: &World, i: usize) -> Result<(), Unlit> {
    let p = Parts::of(&w.chem).ok_or(Unlit::Lifeless)?;
    if crate::contract::land(w, i) <= 0.0 {
        return Err(Unlit::NoLand);
    }
    bed(w, &p, i)?.unlit().map_or(Ok(()), Err)
}

/// The flames' month, after the cover's (`life::month`): each land cell's bed (`bed`), the share of each cell that burns
/// (`shares`, the strikes of edits with the drawn ones, which it then forgets) and the burning of it (`burn`), on the
/// chunks of `life` only where some burns; what each cell's flames did is kept for the tape (`tally`).
pub fn month(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let n = w.mesh.cells().n();
    let beds = crate::par::map_n(n, n * BED_WORK, |i| if crate::contract::land(w, i) > 0.0 { bed(w, &p, i).ok() } else { None });
    let share = shares(w, &beds, w.cfg.calendar().month as f64);
    w.strikes.clear();
    let load = share.iter().filter(|&&f| f > 0.0).count();
    if load > 0 {
        crate::life::run(w, load, |w, d| {
            for i in d.cells() {
                if let Some(b) = beds[i].filter(|_| share[i] > 0.0) {
                    burn(w, d, &p, i, (share[i], &b));
                }
            }
        });
    }
    tally(w, n);
}

/// What each cell's flames did in the month (`Counters::flamed`) becomes its last month's (`Counters::flame_month`,
/// one a cell of `n`) and the share it burnt that month of the year's (`burnt_months`); its months since it last burnt
/// (`burnt_age`) grow by the month, at most `AGE_MOST`, and start again from none where it burns. For the tape.
fn tally(w: &mut World, n: usize) {
    let months = w.cfg.months_per_year as usize;
    let of_year = (w.month() % months as u64) as usize;
    let c = &mut w.counts;
    c.flame_month.clear();
    c.flame_month.resize(n, Flamed::default());
    c.burnt_months.resize(n * months, 0.0);
    c.burnt_age.resize(n, AGE_MOST);
    for i in 0..n {
        c.burnt_months[i * months + of_year] = 0.0;
        c.burnt_age[i] = (c.burnt_age[i] + 1.0).min(AGE_MOST);
    }
    for (i, f) in std::mem::take(&mut c.flamed) {
        let i = i as usize;
        (c.flame_month[i], c.burnt_months[i * months + of_year]) = (f, f.share);
        c.burnt_age[i] = 0.0;
    }
}

/// The cured leaves standing on cell `i` (`Cover::cured`), kg a m² of its land (none where it holds no land).
pub fn cured(w: &World, i: usize) -> f64 {
    per_land(w, i, &w.life.cover(i).cured)
}

/// The dead walls flames left standing on cell `i` (`Cover::snag`), kg a m² of its land (none where it holds no land).
pub fn snag(w: &World, i: usize) -> f64 {
    per_land(w, i, &w.life.cover(i).snag)
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

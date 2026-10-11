//! The world core: one graph, five laws. No IO, no clocks, no Earth words.

pub mod change;
pub mod check;
pub mod chem;
pub mod event;
pub mod flow;
pub mod grid;
pub mod rng;

use chem::Chemistry;
use event::Event;
use grid::Grid;
use serde::{Deserialize, Serialize};

/// Planet settings and free physics numbers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Sun strength at the top of the air, W/m²
    pub sun: f64,
    /// Axial tilt, degrees
    pub tilt: f64,
    /// Latitude of the map's south and north edges, degrees
    pub lat_south: f64,
    pub lat_north: f64,
    /// Strength of the deflection of moving air by planet rotation (0 = none)
    pub rotation: f64,
    pub days_per_month: u32,
    pub months_per_year: u32,
    /// Ground and air physics run every `phys_days` days
    pub phys_days: u32,
    /// Liquid, air and rain move every `flow_days` days (radiation every `phys_days`)
    pub flow_days: u32,
    /// Depth of ground that exchanges heat with the surface, m
    pub skin_m: f64,
    /// Height of the air column, m
    pub air_top_m: f64,
    /// Gravity, m/s²
    pub gravity: f64,
    /// Heat exchange between ground and the air touching it, W/(m² K)
    pub sensible: f64,
    /// Transfer speed of evaporation, m/s
    pub evap: f64,
    /// Overturning air speed per K of warmth difference between neighbouring air nodes, m/s
    pub wind: f64,
    /// Mixing of neighbouring air columns by eddies, m²/s
    pub mix: f64,
    /// Condensation in the air at which its droplets cover half the sky, mol/(m² s)
    pub cloud: f64,
    /// Loose solid volume moving liquid can carry per J it dissipates, m³/J
    pub carry: f64,
    /// Bonded volume broken per J dissipated, times (1 / bond strength), m³/J
    pub wear: f64,
    /// Speed of loose solids creeping down a unit slope, times (1 / hardness), m/s
    pub creep: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sun: 1361.0,
            tilt: 23.4,
            lat_south: 5.0,
            lat_north: 55.0,
            rotation: 1.0,
            days_per_month: 30,
            months_per_year: 12,
            phys_days: 3,
            flow_days: 15,
            skin_m: 2.0,
            air_top_m: 10_000.0,
            gravity: 9.81,
            sensible: 10.0,
            evap: 0.006,
            wind: 0.25,
            mix: 2e4,
            cloud: 1.2e-2,
            carry: 1e-7,
            wear: 3e-9,
            creep: 1e-6,
        }
    }
}

/// Energy and matter that crossed the world's boundary, for conservation checks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// Sunlight absorbed, J
    pub sun_in: f64,
    /// Radiation lost to space, J
    pub space_out: f64,
    /// Potential energy turned into heat (negative when matter is lifted), J
    pub gravity: f64,
    /// Energy added by player edits, J
    pub imposed_energy: f64,
    /// Elements added by player edits, mol
    pub imposed_elems: Vec<f64>,
    /// Tiny negative amounts clamped to zero, mol
    pub clamped: f64,
}

/// Per-cell slow averages, useful to see the world (not read by the laws).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Diag {
    /// Liquid moved out of each ground cell, mol/s (moving average)
    pub liquid_flux: Vec<f32>,
    /// Liquid falling on each ground cell, mol/(m² s) (moving average)
    pub rain: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub cfg: Config,
    pub chem: Chemistry,
    pub grid: Grid,
    pub seed: u64,
    /// World clock, days since the start
    pub day: u64,
    /// Loose (unbonded) amounts in ground nodes, [k * n + i], mol
    pub ground: Vec<f64>,
    /// Bonded ground amounts, [k * n + i], mol
    pub bed: Vec<f64>,
    /// Thermal energy of each ground node's active layer, J
    pub e_ground: Vec<f64>,
    /// Amounts in air nodes, [k * na + a], mol
    pub air: Vec<f64>,
    /// Thermal energy of each air node, J
    pub e_air: Vec<f64>,
    /// Elevation of the bottom of the ground column, m
    pub base_z: f64,
    pub diag: Diag,
    pub ledger: Ledger,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Working memory of the laws, rebuilt when missing (saved, so a resumed run goes on the same)
    pub scratch: flow::Scratch,
}

impl World {
    pub fn days_per_year(&self) -> u64 {
        (self.cfg.days_per_month * self.cfg.months_per_year) as u64
    }
    pub fn year(&self) -> f64 {
        self.day as f64 / self.days_per_year() as f64
    }
    /// Fraction of the year, 0..1
    pub fn season(&self) -> f64 {
        (self.day % self.days_per_year()) as f64 / self.days_per_year() as f64
    }
    #[inline]
    pub fn g(&self, k: usize, i: usize) -> f64 {
        self.ground[k * self.grid.n() + i]
    }
    #[inline]
    pub fn b(&self, k: usize, i: usize) -> f64 {
        self.bed[k * self.grid.n() + i]
    }
    #[inline]
    pub fn a(&self, k: usize, j: usize) -> f64 {
        self.air[k * self.grid.na() + j]
    }

    /// Heat capacity of a ground node's active layer, J/K.
    pub fn heat_cap_ground(&self, i: usize) -> f64 {
        let ns = self.chem.ns();
        let (mut c, mut bc, mut bv) = (0.0, 0.0, 0.0);
        for k in 0..ns {
            let s = &self.chem.subs[k];
            c += self.g(k, i) * s.cp;
            let b = self.b(k, i);
            bc += b * s.cp;
            bv += b * s.molar_mass / s.density[0];
        }
        if bv > 0.0 {
            c += bc / bv * self.cfg.skin_m * self.grid.area();
        }
        c
    }
    /// Temperature of a ground node, and the substance held at its melting point with its
    /// liquid share, if any. Molten amounts hold their fusion enthalpy in `e_ground`, so a node
    /// freezes or melts at a plateau.
    pub fn thaw(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        let c = self.heat_cap_ground(i).max(1.0);
        let mut ks: Vec<usize> = (0..self.chem.ns()).filter(|&k| self.g(k, i) > 0.0).collect();
        ks.sort_by(|&a, &b| self.chem.subs[a].melt.total_cmp(&self.chem.subs[b].melt));
        let (e, mut held) = (self.e_ground[i], 0.0);
        for k in ks {
            let (s, lat) = (&self.chem.subs[k], self.g(k, i) * self.chem.subs[k].lf);
            let lo = c * s.melt + held;
            if e < lo {
                break;
            }
            if e < lo + lat {
                return (s.melt, Some((k, (e - lo) / lat)));
            }
            held += lat;
        }
        ((e - held) / c, None)
    }
    pub fn t_ground(&self, i: usize) -> f64 {
        self.thaw(i).0
    }
    /// Thermal energy of a ground node at temperature `t` (with the fusion enthalpy of what is molten).
    pub fn energy_ground(&self, i: usize, t: f64) -> f64 {
        let held: f64 = (0..self.chem.ns()).filter(|&k| self.chem.subs[k].melt < t).map(|k| self.g(k, i) * self.chem.subs[k].lf).sum();
        self.heat_cap_ground(i) * t + held
    }
    /// Molten share of substance `k` in ground node `i` at temperature `t` (from `thaw`).
    pub fn molten(&self, k: usize, t: f64, plateau: Option<(usize, f64)>) -> f64 {
        match plateau {
            Some((pk, f)) if pk == k => f,
            _ => (t >= self.chem.subs[k].melt) as u8 as f64,
        }
    }
    pub fn heat_cap_air(&self, j: usize) -> f64 {
        (0..self.chem.ns()).map(|k| self.a(k, j) * self.chem.subs[k].cp).sum()
    }
    pub fn t_air(&self, j: usize) -> f64 {
        self.e_air[j] / self.heat_cap_air(j).max(1.0)
    }

    /// Height of the solid surface (bonded + loose solids), m.
    pub fn surface_z(&self, i: usize) -> f64 {
        let (t, pl) = self.thaw(i);
        let mut v = 0.0;
        for k in 0..self.chem.ns() {
            let mv = self.chem.molar_volume(k, chem::Phase::Solid);
            v += (self.b(k, i) + self.g(k, i) * (1.0 - self.molten(k, t, pl))) * mv;
        }
        self.base_z + v / self.grid.area()
    }
    /// Depth of liquid standing on a ground cell, m.
    pub fn liquid_depth(&self, i: usize) -> f64 {
        let (t, pl) = self.thaw(i);
        let mut v = 0.0;
        for k in 0..self.chem.ns() {
            if t < self.chem.subs[k].boil {
                v += self.g(k, i) * self.molten(k, t, pl) * self.chem.molar_volume(k, chem::Phase::Liquid);
            }
        }
        v / self.grid.area()
    }
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    let dpm = w.cfg.days_per_month;
    for d in 0..dpm {
        if d % w.cfg.phys_days == 0 {
            let dt = w.cfg.phys_days.min(dpm - d) as f64 * 86_400.0;
            flow::step(w, dt);
            change::step(w, dt);
        }
        w.day += 1;
    }
}

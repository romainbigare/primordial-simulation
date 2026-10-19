//! The settings of a world (`Config`) and the free numbers of its laws (`Physics`). What each
//! number is anchored to is in `docs/numbers.md`, one row per field.

use crate::clock::Calendar;
use crate::units::{DAY_S, T_SURF};
use serde::{Deserialize, Serialize};

/// The settings of a world: the planet, the calendar and time steps, the physics of its ground
/// and air, the run switches, and the free numbers of the laws (`phys`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    // Planet
    /// Sun strength at the top of the air, W/m²
    pub sun: f64,
    /// Axial tilt, degrees
    pub tilt: f64,
    /// Latitude of the map's south edge, degrees
    pub lat_south: f64,
    /// Latitude of the map's north edge, degrees
    pub lat_north: f64,
    /// Strength of the deflection of moving air by the planet's rotation (0 = none)
    pub rotation: f64,
    /// Gravity, m/s²
    pub gravity: f64,

    // Calendar and time steps
    /// Days in a month
    pub days_per_month: u32,
    /// Months in a year
    pub months_per_year: u32,
    /// Ground and air physics (and radiation) run every this many days
    pub phys_days: u32,
    /// Gas moves between air nodes (wind) every this many days
    pub air_days: u32,
    /// Evaporation and condensation run every this many days (a multiple of `phys_days`)
    pub exchange_days: u32,
    /// Liquid moves every this many days (a multiple of `phys_days`)
    pub flow_days: u32,
    /// Speed shortcut: past this many body nodes in a cell at a month's start the most alike merge (`nodes::crowd`)
    pub crowd: usize,
    /// Speed setting: a sleeping node that has gained over this share of an amount it held when it fell asleep wakes
    pub wake_share: f64,
    /// Speed setting: the longest a ground or air node's reactions sleep, days (0: every node reacts every step)
    pub sleep_days: u32,
    /// Speed setting: a population node (more than one member) updates at most every this many days (`Body::interval`)
    pub node_days: u32,
    /// Speed setting: a node without a brain runs its sample member on one update in this many, drawn (`copy::select`)
    pub sample_every: u32,

    // Ground and air physics
    /// Depth of ground that exchanges heat with the surface, m
    pub skin_m: f64,
    /// Height of the air column, m
    pub air_top_m: f64,
    /// Heat exchange between ground and the air touching it, W/(m² K)
    pub sensible: f64,
    /// Transfer speed of evaporation on the air's side of a surface, m/s
    pub evap: f64,
    /// Transfer speed of a dissolving gas on the liquid's side of its surface (in series with `evap`), m/s
    pub dissolve: f64,
    /// Overturning air speed per K of warmth difference between neighbouring air nodes, m/s per K
    pub wind: f64,
    /// Mixing of neighbouring air columns by eddies, m²/s
    pub mix: f64,
    /// Condensation in the air at which its droplets cover half the sky, mol/(m² s)
    pub cloud: f64,
    /// Loose solid volume moving liquid carries off per J it dissipates, m³/J
    pub carry: f64,
    /// Bonded volume broken per J dissipated, times 1 / bond strength, m³/J
    pub wear: f64,
    /// Speed at which dense cover spreads back into ground cleared along an edge, from each side, m/s
    pub regrow: f64,
    /// Speed of loose solids creeping down a unit slope, times 1 / hardness, m/s
    pub creep: f64,

    // Run switches (control runs to compare with)
    /// Bodies with a brain may be population nodes (off, `--individuals`: they are always individuals)
    pub nodes: bool,
    /// A mature body's children may have a second, alike parent (off, `--no-mix`: one parent)
    pub two_parents: bool,
    /// A null run: no learning, no imitation, born rows and brain bases never change at a copy or a merge
    pub null: bool,

    /// The free numbers of the laws
    pub phys: Physics,
}

/// Free numbers of the laws of bodies and of the ground's cover and hydrology, grouped by the
/// law that reads them; none is a rule, all are sizes of costs, strengths, spreads and flows.
/// Numerical safeguards, speed schedules and fixed sizes stay consts in the code.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Physics {
    // Change and Flow, as bodies feel them
    /// Liquid thickness over a body that holds back 63% of the light reaching it, m
    pub light_depth: f64,
    /// Share of the light a light-driven reaction takes that it stores; the rest becomes heat
    pub light_yield: f64,
    /// Liquid thickness over a body that cuts its gas exchange by a factor e, m
    pub gas_depth: f64,
    /// Depth of liquid, standing or in the ground's pores, at which the ground counts as half wet, m
    pub wet_depth: f64,
    /// Heat a cover of m kg/m² passes to a body: `shelter / m` per m² of its surface, W kg/(m⁴ K)
    pub shelter: f64,
    /// Upkeep of catalysts, J per kg of body per day per unit of capacity (1/s) (`body::upkeep`)
    pub c_cat: f64,
    /// Temperature at which upkeep measures catalyst capacity and the costs are anchored, K
    pub t_cat: f64,
    /// Width over which catalysts lose their shape above their `DENATURE` gene, K
    pub unfold: f64,
    /// Catalysts that keep their shape up to this temperature cost no more, K
    pub t_firm: f64,
    /// Each this many K of the `DENATURE` gene over `t_firm` adds the catalysts' upkeep once more, K
    pub c_firm: f64,
    /// Upkeep of strength, J per kg of body per day at strength 1
    pub c_strength: f64,
    /// Upkeep every body pays whatever its genes, J per kg of body per day
    pub c_basal: f64,
    /// Upkeep of light-capture area, J per m² per day
    pub c_leaf: f64,
    /// Cost of one decision of the starting brain, J per kg of a member of `m_ref` kg (`Physics::think`)
    pub c_brain: f64,
    /// The one reference mass at which a per-kg number is per kg, kg
    pub m_ref: f64,
    /// Cost of one decision per hidden unit, J per kg of a member of `m_ref` kg
    pub c_hidden: f64,
    /// Cost of one decision per memory number, J per kg of a member of `m_ref` kg
    pub c_memory: f64,
    /// Cost of one decision per body perceived, J per kg of a member of `m_ref` kg
    pub c_seen: f64,
    /// Cost of one decision per square cell of sight, J per kg of a member of `m_ref` kg
    pub c_sight: f64,
    /// Upkeep of learning, J per kg of body per day at learning rate 1
    pub c_learn: f64,
    /// Cost of signalling, J per kg of body per day at signal size 1
    pub c_signal: f64,
    /// Energy a kg of body turns over in its upkeep before it is worn out, at a `REPAIR` gene of 0, J/kg
    pub e_life: f64,
    /// Share of upkeep that is repair at a `REPAIR` gene of 0
    pub c_repair: f64,
    /// Exponent of a body's activity in its key liquid over the level it keeps (`Body::activity`)
    pub dormancy: f64,

    // Choose
    /// Energy to move one kg one metre on the flat for a member of `MOVE_REF` kg, J (`Physics::walk`)
    pub c_move: f64,
    /// Body mass per area that doubles the cost of entering a cell and of handling what is taken there, kg/m²
    pub rho_pass: f64,
    /// Width of the passage a mover clears through bodies, m
    pub lane: f64,
    /// The bodies of a cell must fit in a layer this thick over it, m
    pub layer: f64,
    /// Density of bodies, kg/m³
    pub rho_body: f64,
    /// The wind at a body, which the boundary layer of its surface goes by, m/s (`exchange::film`)
    pub wind_ground: f64,
    /// Forced convection through a surface's boundary layer, mol/(m² s) per (m/s / m)^0.5
    pub forced: f64,
    /// The length of the boundary layer of a body's capture area (a leaf), m
    pub leaf_size: f64,
    /// Most a body takes per day of its interval, as a share of the mass it takes by (`Body::supply`)
    pub take: f64,
    /// Exponent of a member's body mass that its supply (intake, building, leaf) goes with (`Body::equiv`)
    pub intake_exp: f64,
    /// Area a body works per day per kg of its working mass, m²/(kg day)
    pub sweep: f64,
    /// Energy to handle one kg taken or put, J
    pub c_handle: f64,
    /// Spread of the shape of a part made around its mean
    pub shape_noise: f32,
    /// Spread of the signal around its mean
    pub signal_noise: f32,
    /// Weight of well-being one decision ahead against well-being now
    pub discount: f32,
    /// Share of the way a watcher's rows move toward the watched one's
    pub blend: f32,
    /// Weight of the newest step in a learned row's mean square of steps
    pub learn_beta: f32,
    /// Most a row moves in one step, in units of the learning rate
    pub learn_most: f32,

    // Bond
    /// Tensile strength of a solid of bond strength 1, Pa
    pub tensile: f64,
    /// Share of the contact area a joint pressed by a body holds
    pub joint: f64,
    /// Energy to shape one kg into a part, J
    pub c_form: f64,
    /// Energy to make a joint, J per m² of contact
    pub c_join: f64,
    /// Heat passed through an opening, W/(m² K)
    pub leak: f64,
    /// Most of its mass a body puts into one part
    pub form: f64,
    /// Walls lighter than this fall apart (so a part is made of at least this), kg
    pub min_kg: f64,
    /// A held part multiplies a body's force by 1 + lever · hardness · m / (m + 0.05 body mass)
    pub lever: f64,
    /// Force of a body of strength 1, as a share of its weight
    pub grip: f64,
    /// Push of a mover on each structure of a cell, and the push its own force must give to move, share of its weight
    pub bump: f64,
    /// Most bonded ground a body breaks per day of its interval, as a share of its mass
    pub dig: f64,
    /// Liquid deeper than this is crossed afloat by a body holding a structure the liquid holds up, m
    pub deep: f64,
    /// Cost of crossing afloat, as a share of the cost of walking
    pub afloat: f64,

    // Copy
    /// Work to make one kg of body matter, J (`copy::grow`)
    pub c_copy: f64,
    /// Most work a member of `m_ref` kg puts into building body matter per day, J (`Body::supply`)
    pub c_build: f64,
    /// Chance that a child starts a new lineage with a changed brain base
    pub new_lineage: f64,
    /// Size of that change of the base
    pub base_noise: f32,
    /// Chance that a child's born brain rows change
    pub row_chance: f64,
    /// Step of that change per generation, times the body's mutation size
    pub row_noise: f32,
    /// Spread of the weights a bigger brain adds
    pub new_noise: f32,
    /// Nodes of a cell whose genes differ by at most this many mutation steps merge (`body::alike`)
    pub alike: f32,

    // Flow, as ground cover and moving liquid feel it
    /// Thickness of a layer that hides half of what lies below it: [soft solid, liquid, loose], m
    pub hide: [f64; 3],
    /// A solid warmer than this share of its melting point is soft cover lying on top
    pub soft: f64,
    /// Liquid leaving a node goes only to neighbours at least this share as steep as the steepest
    pub split: f64,
    /// Liquid does not move for a smaller head difference, m
    pub still: f64,
    /// Body mass per area that halves what moving liquid carries off a cell and how fast its loose solids creep, kg/m²
    pub hold: f64,
    /// Share of the volume of the loose ground that is pore space (`flow::liquid`)
    pub porosity: f64,
    /// Hydraulic conductivity of the loose ground (Darcy's law), m/s
    pub seep: f64,
    /// Share of its pores the loose ground holds against gravity, its field capacity (`flow::drain`, `flow::liquid`)
    pub field: f64,
    /// Depth of the store under the loose ground (the weathered zone of the bed), m
    pub bed_depth: f64,
    /// Share of that store that is pore space that drains (its specific yield)
    pub bed_porosity: f64,
    /// Hydraulic conductivity of that store (Darcy's law), m/s
    pub bed_seep: f64,
    /// Half the spacing of the channels inside a cell (the length of a hillslope), m
    pub hill: f64,
}

/// The mass at which `Physics::c_move` is the cost of moving a kg a metre, kg.
const MOVE_REF: f64 = 30.0;
/// Exponent of the cost of transport per kg and m in the mover's mass (Taylor et al. 1982).
const MOVE_EXP: f64 = -0.32;

impl Physics {
    /// Energy of one decision of a brain whose cost is `c` J per kg of a member of `m_ref` kg
    /// (`Lineage::cost`), for a member of `m` kg, J: weakly more for a bigger body. Paid per
    /// decision, not per day, so it pushes bodies toward long intervals.
    pub(crate) fn think(&self, c: f64, m: f64) -> f64 {
        c * self.m_ref * (m / self.m_ref).powf(0.25)
    }
    /// Energy to move one kg one metre on the flat for a member of `m1` kg, J: `c_move` at
    /// `MOVE_REF`, times (m1 / `MOVE_REF`)^`MOVE_EXP` otherwise, the real cost of transport
    /// (Taylor et al. 1982, 10.7 m1^-0.32 J per kg per m). Climbing is not in it: that is the
    /// weight lifted (`Config::gravity`).
    pub fn walk(&self, m1: f64) -> f64 {
        self.c_move * (m1 / MOVE_REF).powf(MOVE_EXP)
    }
    /// Check the numbers the laws need in their range: `dormancy` above 1 (a drying body's pores close as u^n, solved
    /// with n - 1 in a divisor: `body::exchange::closing`); `light_yield` above 0 and at most 1 (the light a light
    /// reaction takes is what it stores over this share).
    pub fn validate(&self) -> Result<(), String> {
        if self.dormancy.is_nan() || self.dormancy <= 1.0 {
            return Err(format!("dormancy ({}) must be above 1", self.dormancy));
        }
        if !(self.light_yield > 0.0 && self.light_yield <= 1.0) {
            return Err(format!("light_yield ({}) must be above 0 and at most 1", self.light_yield));
        }
        Ok(())
    }
}

impl Default for Physics {
    fn default() -> Self {
        Physics {
            light_depth: 10.0,
            light_yield: 0.05,
            gas_depth: 1.0,
            wet_depth: 0.01,
            shelter: 220.0,
            c_cat: 2.6306e10,
            t_cat: T_SURF,
            unfold: 3.0,
            t_firm: 320.0,
            c_firm: 20.0,
            c_strength: 3.683e5,
            c_basal: 2e4,
            c_leaf: 2e3,
            c_brain: 1e5,
            m_ref: 1.0,
            c_hidden: 5e3,
            c_memory: 5e3 / 3.0,
            c_seen: 5e3,
            c_sight: 1.5e4,
            c_learn: 1.1049e5,
            c_signal: 736.6,
            e_life: 1e9,
            c_repair: 0.25,
            dormancy: 5.7,
            c_move: 3.0,
            rho_pass: 5.0,
            lane: 1.0,
            layer: 10.0,
            rho_body: 1e3,
            wind_ground: 1.7,
            forced: 0.189,
            leaf_size: 0.05,
            take: 0.05,
            sweep: 1.0,
            intake_exp: 0.75,
            c_handle: 1e3,
            shape_noise: 0.5,
            signal_noise: 0.3,
            discount: 0.9,
            blend: 0.3,
            learn_beta: 0.1,
            learn_most: 0.5,
            tensile: 1e7,
            joint: 0.05,
            c_form: 1e5,
            c_join: 1e7,
            leak: 25.0,
            form: 0.1,
            min_kg: 1.0,
            lever: 3.0,
            grip: 0.5,
            bump: 0.05,
            dig: 0.01,
            deep: 0.5,
            afloat: 0.2,
            c_copy: 1e7,
            c_build: 1e4,
            new_lineage: 0.001,
            base_noise: 0.03,
            row_chance: 0.3,
            row_noise: 0.3,
            new_noise: 0.1,
            alike: 4.0,
            hide: [0.2, 0.5, 0.02],
            soft: 0.75,
            split: 0.2,
            still: 0.01,
            hold: 2.0,
            porosity: 0.4,
            seep: 3.6e-6,
            field: 0.6,
            bed_depth: 30.0,
            bed_porosity: 0.02,
            bed_seep: 3e-6,
            hill: 250.0,
        }
    }
}

impl Config {
    /// The world's calendar: its day, month and year, s.
    pub fn calendar(&self) -> Calendar {
        let month = self.days_per_month as u64 * DAY_S;
        Calendar { day: DAY_S, month, year: month * self.months_per_year as u64 }
    }
    /// Check a run's settings before it starts: the time steps fit together as the clock runs them (`schedule`): the
    /// physics runs every `phys_days` from the month's start, and the wind, evaporation and liquid every `air_days`,
    /// `exchange_days` and `flow_days` after the state of the ground and the air at a physics step, so each must be a
    /// whole multiple of `phys_days`; and the physics (`Physics::validate`).
    pub fn validate(&self) -> Result<(), String> {
        if self.phys_days == 0 {
            return Err("phys_days must be at least 1".into());
        }
        for (name, x) in [("air_days", self.air_days), ("exchange_days", self.exchange_days), ("flow_days", self.flow_days)] {
            if x == 0 || x % self.phys_days != 0 {
                return Err(format!("{name} ({x}) must be a whole multiple of phys_days ({})", self.phys_days));
            }
        }
        if self.node_days as f64 > crate::body::MAX_INTERVAL {
            return Err(format!("node_days ({}) must be at most {} days, the longest interval", self.node_days, crate::body::MAX_INTERVAL));
        }
        if self.sample_every == 0 {
            return Err("sample_every must be at least 1 (a sample on one update in that many)".to_string());
        }
        if !(self.wake_share > 0.0 && self.wake_share.is_finite()) {
            return Err(format!("wake_share ({}) must be above 0", self.wake_share));
        }
        self.phys.validate()
    }
    /// Spread of the new weights of a brain carried over to other sizes (`Physics::new_noise`; none in a null run, whose
    /// brains never change at a copy).
    pub(crate) fn new_noise(&self) -> f32 {
        if self.null { 0.0 } else { self.phys.new_noise }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sun: 1361.0,
            tilt: 23.4,
            lat_south: 31.0,
            lat_north: 33.3,
            rotation: 1.0,
            gravity: 9.81,
            days_per_month: 30,
            months_per_year: 12,
            phys_days: 3,
            air_days: 3,
            exchange_days: 3,
            flow_days: 30,
            crowd: 4,
            wake_share: 0.05,
            sleep_days: 90,
            node_days: 36,
            sample_every: 2,
            skin_m: 2.0,
            air_top_m: 10_000.0,
            sensible: 10.0,
            evap: 0.006,
            dissolve: 5e-5,
            wind: 0.25,
            mix: 2e4,
            cloud: 1.2e-2,
            carry: 2e-11,
            wear: 1.4e-11,
            regrow: 1e-9,
            creep: 6.3e-11,
            nodes: true,
            two_parents: true,
            null: false,
            phys: Physics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Physics};

    #[test]
    fn the_default_settings_pass_and_a_step_off_its_base_or_a_dormancy_of_1_is_refused() {
        assert_eq!(Config::default().validate(), Ok(()));
        let dormant = Config { phys: Physics { dormancy: 1.0, ..Default::default() }, ..Default::default() };
        for c in [Config { flow_days: 4, ..Default::default() }, Config { exchange_days: 4, ..Default::default() }, Config { air_days: 2, ..Default::default() }, Config { phys_days: 0, ..Default::default() }, dormant] {
            assert!(c.validate().is_err(), "{} {} {} {} {}", c.phys_days, c.air_days, c.exchange_days, c.flow_days, c.phys.dormancy);
        }
    }
}

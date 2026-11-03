//! The settings of a world (`Config`) and the free numbers of its laws (`Physics`). What each
//! number is anchored to is in `docs/numbers.md`, one row per field.

use crate::{clock::Calendar, units::{DAY_S, T_SURF}};
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
    /// Speed setting: a sleeping node that has gained over this share of an amount it held when it fell asleep wakes
    pub wake_share: f64,
    /// Speed setting: the longest a ground or air node's reactions sleep, days (0: every node reacts every step)
    pub sleep_days: u32,
    /// Speed setting: the cells are cut into this many runs for the parallel passes of bodies (any count gives one world;
    /// a setting of the run, not saved)
    #[serde(skip, default = "default_chunks")]
    pub chunks: u32,

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
    /// Condensation in the air at which its cloud reflects half of what its droplets can, mol/(m² s)
    pub cloud: f64,
    /// Optical depth to heat radiation of a cloud at the condensation `cloud`
    pub cloud_ir: f64,
    /// Time in which vapour over saturation rains out, s
    pub rain_time: f64,
    /// Loose solid volume moving liquid carries off per J it dissipates, m³/J
    pub carry: f64,
    /// Bonded volume broken per J dissipated, times 1 / bond strength, m³/J
    pub wear: f64,
    /// Speed at which dense cover spreads back into ground cleared along an edge, from each side, m/s
    pub regrow: f64,
    /// Speed of loose solids creeping down a unit slope, times 1 / hardness, m/s
    pub creep: f64,
    /// Added to a solid's hardness before its softness, 1 / (hardness + this), is taken
    pub hard_min: f64,

    // Run switches (control runs to compare with)

    /// A null run: no learning, no imitation, born rows and brain bases never change at a copy or a merge
    pub null: bool,

    /// The free numbers of the laws
    pub phys: Physics,
}

/// Free numbers of the laws of bodies and of the ground's cover and hydrology, grouped by the law that reads them:
/// sizes of costs, strengths, spreads and flows, never rules.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Physics {
    // Change and Flow, as bodies feel them
    /// Liquid thickness over a body that holds back 63% of the light reaching it, m
    pub light_depth: f64,
    /// Share of the light a light-driven reaction takes that it stores; the rest becomes heat
    pub light_yield: f64,
    /// Mole fraction of a light reaction's gas in the air at which it stores nothing (its compensation point)
    pub gas_floor: f64,
    /// Mole fraction of a light reaction's gas in the air at which it stores `light_yield`
    pub gas_ref: f64,
    /// Depth of liquid, standing or in the ground's pores, at which the ground counts as half wet, m
    pub wet_depth: f64,
    /// Temperature at which upkeep measures catalyst capacity and the costs are anchored, K
    pub t_cat: f64,
    /// Spread of a cell's microclimate: below a freezing point the frozen share of a body's or the cover's members grows from 0 to 1 over this, K
    pub frost_k: f64,
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
    /// Upkeep of learning, J per kg of body per day at learning rate 1
    pub c_learn: f64,
    /// Energy a kg of body turns over in its upkeep before it is worn out, at a `REPAIR` gene of 0, J/kg
    pub e_life: f64,
    /// Share of upkeep that is repair at a `REPAIR` gene of 0
    pub c_repair: f64,
    /// Exponent of a body's activity in its key liquid over the level it keeps (`Body::activity`)
    pub dormancy: f64,
    /// Least activity a dormant body or slot of cover wears out at, a share of the active rate (`body::live`, `cover::grow`)
    pub dormant: f64,
    /// Share of its body whose loss kills a member: a hit that tears it kills; starving to it, even chances
    pub lethal: f64,

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
    /// Most a body takes per day, as a share of the mass it takes by (`Body::supply`)
    pub take: f64,
    /// Exponent of a member's body mass that its supply (intake, building, leaf) goes with (`Body::equiv`)
    pub intake_exp: f64,
    /// Area a body works per day per kg of its working mass, m²/(kg day)
    pub sweep: f64,
    /// Energy to handle one kg taken or put, J
    pub c_handle: f64,
    /// A body keeps this many times the safe minimum of each store it gives away or puts out
    pub keep_min: f64,
    /// Spread of the shape of a part made around its mean
    pub shape_noise: f32,
    /// Weight of well-being one month (one update) ahead against well-being now
    pub discount: f32,
    /// Weight in a move's score of the critic's gain from stepping into a cell (`choose::plan`), per unit of well-being
    pub plan_weight: f32,
    /// Share of the way a watcher's rows move toward the watched one's
    pub blend: f32,
    /// Weight of the newest value in a learner's running mean squares: of a row's steps, and of the critic's errors
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
    /// Work to take hold of a part, as a share of the work of handling its mass (`c_handle`)
    pub grab: f64,
    /// Force of a body of strength 1, as a share of its weight
    pub grip: f64,
    /// Push of a mover on each structure of a cell, and the push its own force must give to move, share of its weight
    pub bump: f64,
    /// Liquid deeper than this is crossed afloat by a body holding a structure the liquid holds up, m
    pub deep: f64,

    // Copy
    /// Work to make one kg of body matter, J (`copy::grow`)
    pub c_copy: f64,
    /// Most work a member of `m_ref` kg puts into building body matter per day, J (`Body::supply`)
    pub c_build: f64,
    /// Chance that a child's born brain rows change
    pub row_chance: f64,
    /// Step of that change per generation, times the body's mutation size
    pub row_noise: f32,
    /// Spread of the weights a bigger brain adds
    pub new_noise: f32,
    /// Nodes of a cell whose genes differ by at most this many mutation steps merge (`body::alike`)
    pub alike: f32,

    // The cover (`cover`)
    /// Upkeep of the cover's light capacity, as a share of what that capacity could store in the time
    pub c_light: f64,
    /// Mutation size of the cover's genes per generation, times each gene's step
    pub cover_mut: f32,
    /// The solid the cover builds of a mineral (`cover::HARD`), kg per kg of its body matter
    pub cover_hard: f64,

    // Flow, as ground cover and moving liquid feel it
    /// Thickness of a layer that hides half of what lies below it: [soft solid, liquid, loose], m
    pub hide: [f64; 3],
    /// A solid warmer than this share of its melting point is soft cover lying on top
    pub soft: f64,
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
    /// Energy of one decision of a brain of cost `c` (`Lineage::cost`) for a member of `m` kg, J: weakly more for a
    /// bigger body; paid per decision, once a month.
    pub(crate) fn think(&self, c: f64, m: f64) -> f64 {
        c * self.m_ref * (m / self.m_ref).sqrt().sqrt()
    }
    /// Energy to move one kg one metre on the flat for a member of `m1` kg, J: the real cost of transport, 10.7
    /// m1^-0.32 J per kg per m (Taylor et al. 1982), scaled to `c_move` at `MOVE_REF`. Climbing is the weight lifted.
    pub fn walk(&self, m1: f64) -> f64 {
        self.c_move * (m1 / MOVE_REF).powf(MOVE_EXP)
    }
    /// The factor on upkeep of a repair gene (log10): its repair share `c_repair` doubles for each unit.
    pub fn repair(&self, gene: f64) -> f64 {
        1.0 - self.c_repair + self.c_repair * 2f64.powf(gene)
    }
    /// What a light reaction stores in air that holds the mole fraction `x` of its gas, as a share of what it stores at
    /// `gas_ref`: the light-limited rate (x - floor) / (x + 2 floor) of Farquhar, von Caemmerer and Berry (1980), none
    /// at or below the floor.
    pub fn gas_factor(&self, x: f64) -> f64 {
        let f = |x: f64| ((x - self.gas_floor) / (x + 2.0 * self.gas_floor)).max(0.0);
        f(x) / f(self.gas_ref)
    }
    /// How wet ground with `depth` m of liquid (in its pores and standing) is, 0..1: half at `wet_depth`.
    pub fn wetness(&self, depth: f64) -> f64 {
        depth / (depth + self.wet_depth)
    }
    /// The activity of a living thing that holds the share `x` of the liquid it keeps: x^`dormancy`, 1 at or above it.
    pub fn activity(&self, x: f64) -> f64 {
        if x >= 1.0 { 1.0 } else { x.max(0.0).powf(self.dormancy) }
    }
}

impl Default for Physics {
    fn default() -> Self {
        Physics {
            light_depth: 10.0,
            light_yield: 0.05,
            gas_floor: 4.3e-5,
            gas_ref: 3.8e-4,
            wet_depth: 0.01,
            t_cat: T_SURF,
            frost_k: 3.0,
            c_firm: 20.0,
            c_strength: 1.31e4,
            c_basal: 6.3e4,
            c_leaf: 2e3,
            c_brain: 8.5e4,
            m_ref: 1.0,
            c_hidden: 5e3,
            c_memory: 5e3 / 3.0,
            c_seen: 5e3,
            c_learn: 1.1049e5,
            e_life: 1e9,
            c_repair: 0.25,
            dormancy: 5.7,
            dormant: 0.02,
            lethal: 0.5,
            c_move: 3.0,
            rho_pass: 5.0,
            lane: 1.0,
            layer: 10.0,
            rho_body: 1e3,
            take: 0.01042,
            sweep: 1.0,
            intake_exp: 0.75,
            c_handle: 1e3,
            keep_min: 2.0,
            shape_noise: 0.5,
            discount: 0.9,
            plan_weight: 1.0,
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
            grab: 0.1,
            grip: 0.5,
            bump: 0.05,
            deep: 0.5,
            c_copy: 1e7,
            c_build: 1e4,
            row_chance: 0.3,
            row_noise: 0.3,
            new_noise: 0.1,
            alike: 4.0,
            c_light: 0.1,
            cover_mut: 0.063,
            cover_hard: 0.02,
            hide: [0.2, 0.5, 0.02],
            soft: 0.75,
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

/// Ok if `ok`, else the error `why` writes: one check of a world's inputs.
pub(crate) fn require(ok: bool, why: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why()) }
}

impl Config {
    /// The world's calendar: its day, month and year, s.
    pub fn calendar(&self) -> Calendar {
        let month = self.days_per_month as u64 * DAY_S;
        Calendar { day: DAY_S, month, year: month * self.months_per_year as u64 }
    }
    /// Days between a body's updates: every body updates once a month, at its start (`survey::batch`).
    pub fn update_days(&self) -> f64 {
        self.days_per_month as f64
    }
    /// Check a run's settings before it starts: the time steps, and the numbers of the physics the laws need in range.
    pub fn validate(&self) -> Result<(), String> {
        let (m, p) = (self.days_per_month, &self.phys);
        // The month holds a whole number of the air's steps (`schedule::AIR_STEP_DAYS`).
        let air = crate::schedule::AIR_STEP_DAYS;
        require(m > 0 && m.is_multiple_of(air), || format!("days_per_month ({m}) must be a whole number of the air's steps ({air} days)"))?;
        require(self.chunks > 0, || "chunks must be at least 1".to_string())?;
        let wake = self.wake_share;
        require(wake > 0.0 && wake.is_finite(), || format!("wake_share ({wake}) must be above 0"))?;
        // `dormancy` above 1 (`body::exchange::closing` divides by n - 1); `light_yield` in (0, 1] (a light reaction takes
        // what it stores over this share).
        require(p.dormancy > 1.0, || format!("dormancy ({}) must be above 1", p.dormancy))?;
        // The bed and the skin of its top are layers of matter: a thickness of zero would give a node no heat capacity.
        require(self.skin_m > 0.0 && p.bed_depth > 0.0, || format!("skin_m ({}) and bed_depth ({}) must be above 0", self.skin_m, p.bed_depth))?;
        let light = p.light_yield;
        require(light > 0.0 && light <= 1.0, || format!("light_yield ({light}) must be above 0 and at most 1"))
    }
    /// Spread of the new weights of a brain carried over to other sizes (none in a null run).
    pub(crate) fn new_noise(&self) -> f32 {
        if self.null { 0.0 } else { self.phys.new_noise }
    }
}

/// The count of chunks of a run that sets none (`Config::chunks`; `docs/numbers.md`).
fn default_chunks() -> u32 {
    64
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sun: 1361.0,
            tilt: 23.4,
            lat_south: 40.0,
            lat_north: 42.3,
            rotation: 1.0,
            gravity: 9.81,
            days_per_month: 30,
            months_per_year: 12,
            wake_share: 0.05,
            sleep_days: 90,
            chunks: default_chunks(),
            skin_m: 2.0,
            air_top_m: 10_000.0,
            sensible: 10.0,
            evap: 0.006,
            dissolve: 5e-5,
            wind: 0.25,
            mix: 1e6,
            cloud: 1.2e-2,
            cloud_ir: 6.7,
            rain_time: 86400.0,
            carry: 2e-11,
            wear: 1.4e-11,
            regrow: 1e-9,
            creep: 6.3e-11,
            hard_min: 0.05,

            null: false,
            phys: Physics::default(),
        }
    }
}

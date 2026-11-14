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
    /// Radius of the planet, m
    pub radius_m: f64,
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
    /// Depth of the layer that exchanges with the surface within a year (heat, pores), m
    pub skin_m: f64,
    /// Height of the air column, m
    pub air_top_m: f64,
    /// Heat exchange between ground and the air touching it, W/(m² K)
    pub sensible: f64,
    /// Transfer speed of evaporation on the air's side of a surface, m/s
    pub evap: f64,
    /// Transfer speed of a dissolving gas on the liquid's side of its surface (in series with `evap`), m/s
    pub dissolve: f64,
    /// Overturning air speed per K/m of warmth gradient, m²/(s K)
    pub wind: f64,
    /// Mixing of neighbouring air columns by eddies, m²/s
    pub mix: f64,
    /// Condensation in the air at which its cloud reflects half of what its droplets can, mol/(m² s)
    pub cloud: f64,
    /// Optical depth to heat radiation of a cloud at the condensation `cloud`
    pub cloud_ir: f64,
    /// Time in which vapour over saturation rains out, s
    pub rain_time: f64,
    /// Distance the wind carries condensed rain before it lands, m
    pub rain_drift_m: f64,
    /// Loose solid volume moving liquid carries off per J it dissipates, m³/J
    pub carry: f64,
    /// Bonded volume broken per J dissipated, times 1 / bond strength, m³/J
    pub wear: f64,
    /// Speed of loose solids creeping down a unit slope, times 1 / hardness, m/s
    pub creep: f64,
    /// Added to a solid's hardness before its softness, 1 / (hardness + this), is taken
    pub hard_min: f64,
    /// Speed at which the mixing of the loose ground buries what lies on it alone, m/s
    pub bury: f64,


    /// The free numbers of the laws
    pub phys: Physics,
}

/// Free numbers of the laws of the cover and of the ground's hydrology, grouped by the law that reads them:
/// sizes of costs, strengths, spreads and flows, never rules.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Physics {
    // Change and Flow, as life feels them
    /// Liquid thickness over the cover that holds back 63% of the light reaching it, m
    pub light_depth: f64,
    /// Share of the light a light-driven reaction takes that it stores; the rest becomes heat
    pub light_yield: f64,
    /// Mole fraction of a light reaction's gas in the air at which it stores nothing (its compensation point)
    pub gas_floor: f64,
    /// Mole fraction of a light reaction's gas in the air at which it stores `light_yield`
    pub gas_ref: f64,
    /// Depth of liquid, standing or in the ground's pores, at which the ground counts as half wet, m
    pub wet_depth: f64,
    /// Liquid a point of a cell's soil holds over the cell's mean per unit of its wetness index over the cell's, m (`inside`)
    pub soil_m: f64,
    /// Temperature at which upkeep measures catalyst capacity and the costs are anchored, K
    pub t_cat: f64,
    /// Spread of a cell's microclimate: below a freezing point the frozen share of the cover grows from 0 to 1 over this, K
    pub frost_k: f64,
    /// Depth to which dormancy lowers the lethal temperature of living tissue (cold hardening), K
    pub harden: f64,
    /// K of the cover's frost tolerance below its key liquid's melting point that add its light capacity's upkeep again
    pub c_firm: f64,
    /// Basal upkeep of living tissue, J per kg per day
    pub c_basal: f64,
    /// Upkeep of light-capture area, J per m² per day
    pub c_leaf: f64,
    /// Energy a kg of living tissue turns over in its upkeep before it is worn out, at a `REPAIR` gene of 0, J/kg
    pub e_life: f64,
    /// Share of upkeep that is repair at a `REPAIR` gene of 0
    pub c_repair: f64,
    /// Exponent of the activity of living tissue in its key liquid over the level it keeps (`Physics::activity`)
    pub dormancy: f64,
    /// Least activity a dormant slot of cover wears out at, a share of the active rate (`cover::grow`)
    pub dormant: f64,

    // Dispersal
    /// Standard deviation of the near scale of the dispersal kernel, m (`life::lot`)
    pub kernel_near: f64,
    /// Standard deviation of the far scale of the dispersal kernel, m
    pub kernel_far: f64,
    /// Share of what disperses that lands by the far scale
    pub kernel_far_share: f64,

    // Growth and likeness
    /// Density of living tissue, kg/m³
    pub rho_body: f64,
    /// Work to make one kg of body matter, J (`cover::build`)
    pub c_copy: f64,
    /// Genes that differ by at most this many mutation steps are alike (`cover::APART`)
    pub alike: f32,

    // The cover (`cover`)
    /// Upkeep of a catalyst's capacity (the light reaction, a gut), a share of what it could make in the time
    pub c_capacity: f64,
    /// Mutation size of a living kind's genes per generation, times each gene's step
    pub mut_size: f32,
    /// Built cover of one plant a seed founds, kg
    pub seed_kg: f64,
    /// Share of the air's gas a leaf draws it down by inside, where the light reaction takes it
    pub drawdown: f64,
    /// Conducting cross-section of stems per m² of the capture area they feed, m² per m²
    pub huber: f64,
    /// Upkeep of conducting walls, J per kg per day at `t_cat`
    pub c_sapwood: f64,
    /// The key liquid a kg of living fine roots takes up, mol per s
    pub root_uptake: f64,
    /// Fine roots built anew per year
    pub root_turnover: f64,
    /// Built cover of a plant that builds half the walls its `WALLS` gene sets, kg
    pub wall_kg: f64,
    /// The self-thinning law: mean plant mass = this × (plants per m²)^(-3/2), kg (per m² to the 1/2)
    pub thin_kg: f64,
    /// A plant's height over its stem's width to the 2/3 (the buckling limit), m^(1/3)
    pub tall: f64,
    /// A stem's volume over its cylinder's, and the share of a stem that is its walls' solid
    pub stem_form: f64,
    pub stem_solid: f64,
    /// Loose ground a plant roots in per m of its height, m per m, and at least, m
    pub root_per_height: f64,
    pub root_least: f64,
    /// Share of its roots' need of loose ground below which a point holds none of a kind
    pub rooted_from: f64,

    // The takers (`fauna`)
    /// Upkeep of a taker's resting muscle, J per kg^`upkeep_exp` per day at `t_cat`, on top of `c_basal`
    pub c_muscle: f64,
    /// Power of a taker's living mass, kg, its upkeep grows as (Kleiber's law)
    pub upkeep_exp: f64,
    /// A taker's surface over its living mass to the 2/3, m² per kg^(2/3)
    pub meeh: f64,
    /// Insulation of the still air at a taker's surface, and of its coat at a `COAT` gene of 1, m² K/W
    pub r_air: f64,
    pub r_coat: f64,
    /// The least key liquid a taker of 1 kg living passes through in a day besides its cooling, kg, and the power of its
    /// living mass that grows as
    pub drink_least: f64,
    pub drink_exp: f64,
    /// A taker's walking speed over that of a pendulum of its length, squared (its Froude number)
    pub walk_froude: f64,
    /// Work a taker of `move_ref` kg spends walking a m per kg of its living mass, J/(kg m), the reference mass, kg, and
    /// the power of its mass over the reference that the work per kg and m goes as
    pub c_move: f64,
    pub move_ref: f64,
    pub move_exp: f64,
    /// A taker's chewing power over its living mass to `upkeep_exp`, J per kg^`upkeep_exp` per day at `t_cat`
    pub chew: f64,
    /// Share of the day a taker searches and takes
    pub busy: f64,
    /// Least height a taker's bite reaches down to, a share of its body's length
    pub bite_floor: f64,
    /// Highest a taker reaches, body lengths
    pub reach_len: f64,
    /// Most fuel a taker stores, kg per kg of its living mass
    pub store_most: f64,

    // Flow, as ground cover and moving liquid feel it
    /// Thickness of a layer that hides half of what lies below it: [soft solid, liquid, loose], m
    pub hide: [f64; 3],
    /// A solid warmer than this share of its melting point is soft cover lying on top
    pub soft: f64,
    /// Liquid does not move for a smaller head difference, m
    pub still: f64,
    /// Mass standing per area that halves what moving liquid carries off a cell and how fast its loose solids creep, kg/m²
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
    /// Width of a channel that carries 1 m³/s, m, and the power of the discharge its width grows as
    pub channel_width: f64,
    pub channel_width_exp: f64,
    /// Depth of a channel that carries 1 m³/s, m, and the power of the discharge its depth grows as
    pub channel_depth: f64,
    pub channel_depth_exp: f64,
    /// Depth of a standing liquid's top layer, which the air mixes (its mixed layer), m
    pub h_mix: f64,
    /// Eddy diffusivity through the thermocline under it, m²/s
    pub thermocline: f64,
}

impl Physics {
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
    /// The share of members that freeze in a month at `t` K when their lethal limit is `limit` K at full activity `act`
    /// = 1: dormancy hardens them, lowering the limit by `depth` K × (1 - `act`) (the cover's own gene, `cover::frozen`),
    /// and their microclimates spread over `frost_k` below it (laws.md 3).
    pub fn frozen(&self, (limit, depth): (f64, f64), t: f64, act: f64) -> f64 {
        ((limit - depth * (1.0 - act) - t) / self.frost_k).clamp(0.0, 1.0)
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
            soil_m: 0.03,
            t_cat: T_SURF,
            frost_k: 3.0,
            harden: 20.0,
            c_firm: 20.0,
            c_basal: 4.22e4,
            c_leaf: 2e3,
            e_life: 1e9,
            c_repair: 0.25,
            dormancy: 5.7,
            dormant: 0.02,
            kernel_near: 40.0,
            kernel_far: 800.0,
            kernel_far_share: 0.1,
            rho_body: 1e3,
            c_copy: 1e7,
            alike: 4.0,
            c_capacity: 0.1,
            mut_size: 0.063,
            seed_kg: 1e-5,
            drawdown: 0.3,
            huber: 2e-4,
            c_sapwood: 1600.0,
            root_uptake: 4e-4,
            root_turnover: 0.6,
            wall_kg: 0.05,
            thin_kg: 5.0,
            tall: 35.0,
            stem_form: 0.5,
            stem_solid: 0.4,
            root_per_height: 0.1,
            root_least: 0.02,
            rooted_from: 0.25,
            c_muscle: 1.12e4,
            upkeep_exp: 0.75,
            meeh: 0.1,
            r_air: 0.12,
            r_coat: 0.9,
            drink_least: 0.0928,
            drink_exp: 0.818,
            walk_froude: 0.25,
            c_move: 3.0,
            move_ref: 30.0,
            move_exp: -0.32,
            chew: 2.9e3,
            busy: 0.5,
            bite_floor: 0.1,
            reach_len: 5.0,
            store_most: 0.3,
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
            channel_width: 3.0,
            channel_width_exp: 0.5,
            channel_depth: 0.3,
            channel_depth_exp: 0.4,
            h_mix: 50.0,
            thermocline: 1e-4,
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
    /// Check a run's settings before it starts: the time steps, and the numbers of the physics the laws need in range.
    pub fn validate(&self) -> Result<(), String> {
        let (m, p) = (self.days_per_month, &self.phys);
        // The month holds a whole number of the air's steps (`schedule::AIR_STEP_DAYS`).
        let air = crate::schedule::AIR_STEP_DAYS;
        require(m > 0 && m.is_multiple_of(air), || format!("days_per_month ({m}) must be a whole number of the air's steps ({air} days)"))?;
        require(self.chunks > 0, || "chunks must be at least 1".to_string())?;
        let (south, north) = (self.lat_south, self.lat_north);
        require(-90.0 <= south && south < north && north <= 90.0, || format!("lat_south ({south}) must lie below lat_north ({north}), both from -90 to 90"))?;
        // The air's sideways laws divide by the stretched lengths (`flow::air::Stretch`), a globe's squares by it.
        require(self.radius_m > 0.0 && self.radius_m.is_finite(), || format!("radius_m ({}) must be above 0", self.radius_m))?;
        let wake = self.wake_share;
        require(wake > 0.0 && wake.is_finite(), || format!("wake_share ({wake}) must be above 0"))?;
        // `dormancy` above 1 (the activity falls as a power of the liquid held, `Physics::activity`); `light_yield` in
        // (0, 1] (a light reaction takes what it stores over this share).
        require(p.dormancy > 1.0, || format!("dormancy ({}) must be above 1", p.dormancy))?;
        // The bed and the skin of its top are layers of matter: a thickness of zero would give a node no heat capacity.
        require(p.h_mix > 0.0 && p.thermocline >= 0.0, || format!("h_mix ({}) must be above 0 and thermocline ({}) not below", p.h_mix, p.thermocline))?;
        require(self.skin_m > 0.0 && p.bed_depth > 0.0, || format!("skin_m ({}) and bed_depth ({}) must be above 0", self.skin_m, p.bed_depth))?;
        // Rain lands over a width the drift sets (`seen::Seen::landing` divides by it).
        require(self.rain_drift_m > 0.0, || format!("rain_drift_m ({}) must be above 0", self.rain_drift_m))?;
        // Hardening lowers a lethal limit, never raises it.
        require(p.harden >= 0.0 && p.harden.is_finite(), || format!("harden ({}) must be at least 0", p.harden))?;
        // A taker's coat divides by its insulation (`fauna`).
        require(p.r_air > 0.0 && p.r_coat >= 0.0, || format!("r_air ({}) must be above 0 and r_coat ({}) not below", p.r_air, p.r_coat))?;
        // The soil's shares inside a cell (`inside`) divide by it.
        require(p.soil_m > 0.0, || format!("soil_m ({}) must be above 0", p.soil_m))?;
        // A channel's speed is its discharge over its width times its depth (`Physics::channel`).
        require(p.channel_width > 0.0 && p.channel_depth > 0.0, || format!("channel_width ({}) and channel_depth ({}) must be above 0", p.channel_width, p.channel_depth))?;
        // The dispersal kernel's scales are lengths and its far share a share (`life::lot`).
        require(p.kernel_near > 0.0 && p.kernel_far > 0.0 && (0.0..=1.0).contains(&p.kernel_far_share), || format!("kernel_near ({}) and kernel_far ({}) must be above 0, kernel_far_share ({}) from 0 to 1", p.kernel_near, p.kernel_far, p.kernel_far_share))?;
        let light = p.light_yield;
        require(light > 0.0 && light <= 1.0, || format!("light_yield ({light}) must be above 0 and at most 1"))
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
            radius_m: 6.371e6,
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
            wind: 4e5,
            mix: 1.3e6,
            cloud: 1.2e-2,
            cloud_ir: 6.7,
            rain_time: 86400.0,
            rain_drift_m: 2e4,
            carry: 2e-11,
            wear: 1.4e-11,
            creep: 6.3e-11,
            hard_min: 0.05,
            bury: 1.6e-10,

            phys: Physics::default(),
        }
    }
}

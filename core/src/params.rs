//! The settings of a world (`Config`) and the free numbers of its laws (`Physics`). What each
//! number is anchored to is in `docs/numbers.md`, one row per field.

use crate::{clock::Calendar, math::Span, units::{DAY, DAY_S, T_SURF}};
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
    /// How many times a band's mean optical depth its core of strong lines has, its wings of weak lines 1 / that
    pub line_spread: f64,
    /// Time in which a saturated part of an air node's air rains out what the node holds over its saturation, s
    pub rain_time: f64,
    /// Least ratio of an air node's vapour to what saturates it at which its moistest air saturates
    pub moist_min: f64,
    /// Distance the wind carries condensed rain before it lands, m
    pub rain_drift_m: f64,
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
    /// Speed at which the mixing of the loose ground buries what lies on it alone, m/s
    pub bury: f64,

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
    /// Liquid a point of a cell's soil holds over the cell's mean per unit of its wetness index over the cell's, m (`inside`)
    pub soil_m: f64,
    /// Temperature at which upkeep measures catalyst capacity and the costs are anchored, K
    pub t_cat: f64,
    /// Spread of a cell's microclimate: below a freezing point the frozen share of a body's or the cover's members grows from 0 to 1 over this, K
    pub frost_k: f64,
    /// Depth to which dormancy lowers the lethal temperature of living tissue (cold hardening), K
    pub harden: f64,
    /// K of the cover's frost tolerance below its key liquid's melting point that add its light capacity's upkeep again
    pub c_firm: f64,
    /// Upkeep of strength, J per kg of body per day at strength 1
    pub c_strength: f64,
    /// Upkeep every body pays whatever its genes, J per kg of body per day
    pub c_basal: f64,
    /// Upkeep of light-capture area, J per m² per day
    pub c_leaf: f64,
    /// Cost of a month of thinking of the starting brain, J per kg of a member of `m_ref` kg (`Physics::think`)
    pub c_brain: f64,
    /// The one reference mass at which a per-kg number is per kg, kg
    pub m_ref: f64,
    /// Cost of a month of thinking per hidden unit, J per kg of a member of `m_ref` kg
    pub c_hidden: f64,
    /// Cost of a month of thinking per memory number, J per kg of a member of `m_ref` kg
    pub c_memory: f64,
    /// Cost of a month of thinking per body perceived, J per kg of a member of `m_ref` kg
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
    /// Standard deviation of the near scale of the dispersal kernel, m (`copy::lot`)
    pub kernel_near: f64,
    /// Standard deviation of the far scale of the dispersal kernel, m
    pub kernel_far: f64,
    /// Share of what disperses that lands by the far scale
    pub kernel_far_share: f64,
    /// The bodies of a cell must fit in a layer this thick over it, m
    pub layer: f64,
    /// Density of bodies, kg/m³
    pub rho_body: f64,
    /// Surface of a body per kg^(2/3) of a member (Meeh), m²/kg^(2/3)
    pub meeh: f64,
    /// Most heat insulation of a body's coat (fur and the tissue under it), over the still air at its surface, m² K/W
    pub r_coat: f64,
    /// Heat insulation of the still air at a surface, m² K/W
    pub r_air: f64,
    /// Heat insulation of the still liquid at a surface (its convective film), m² K/W
    pub r_liquid: f64,
    /// Share of a coat's insulation it keeps in liquid, its still air displaced
    pub wet_coat: f64,
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
    /// Weight of well-being one month ahead against well-being now, per month
    pub discount: f32,
    /// Weight in a move's score of the critic's gain from stepping into a cell (`choose::plan`), per unit of well-being
    pub plan_weight: f32,
    /// Share of the way a watcher's rows move toward the watched one's, per month of watching
    pub blend: f32,
    /// Weight per month of the newest value in a learner's running mean squares (of a row's steps, of its errors)
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
    /// Archard's wear coefficient of an edge: volume worn times hardness over the work it does
    pub abrade: f64,

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
    /// A body under this share of the members of the largest of its lineage, stage and room in its cell joins it (`cap`)
    pub node_floor: f64,

    // The cover (`cover`)
    /// Upkeep of the cover's light capacity, as a share of what that capacity could store in the time
    pub c_light: f64,
    /// Mutation size of the cover's genes per generation, times each gene's step
    pub cover_mut: f32,
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

/// The mass at which `Physics::c_move` is the cost of moving a kg a metre, kg.
const MOVE_REF: f64 = 30.0;
/// Exponent of the cost of transport per kg and m in the mover's mass (Taylor et al. 1982).
const MOVE_EXP: f64 = -0.32;
/// The Froude number v² / (g L) a body walks at (Alexander 1989: mammals of any size walk near 0.25, run past 0.5).
const WALK_FROUDE: f64 = 0.25;
/// The drag coefficient of a body pushed through liquid on its face: a cube face on, 1.05 (Hoerner 1965).
const DRAG: f64 = 1.05;
/// The share of the energy its muscles burn that a body's work through liquid gets (muscle's efficiency about 0.25:
/// Smith, Barclay and Loiselle 2005).
const MUSCLE: f64 = 0.25;

impl Physics {
    /// Energy of a month of thinking of a brain of cost `c` (`Lineage::cost`) for a member of `m` kg, J: weakly more for
    /// a bigger body; paid by the time (`body::upkeep`), whatever the pace of its decisions.
    pub(crate) fn think(&self, c: f64, m: f64) -> f64 {
        c * self.m_ref * (m / self.m_ref).sqrt().sqrt()
    }
    /// Energy to move one kg one metre on the flat for a member of `m1` kg, J: the real cost of transport, 10.7
    /// m1^-0.32 J per kg per m (Taylor et al. 1982), scaled to `c_move` at `MOVE_REF`. Climbing is the weight lifted.
    pub fn walk(&self, m1: f64) -> f64 {
        self.c_move * (m1 / MOVE_REF).powf(MOVE_EXP)
    }
    /// The speed a member of `m1` kg walks at under `gravity` m/s², m/s: bodies of every size move alike at one Froude
    /// number v² / (g L), `WALK_FROUDE` (the dynamic similarity of Alexander and Jayes 1983), its length L the side of
    /// a cube of its volume (`rho_body`): 0.9 m/s for 30 kg, 1.6 m/s for a tonne.
    pub fn speed(&self, m1: f64, gravity: f64) -> f64 {
        (WALK_FROUDE * gravity * self.side(m1)).sqrt()
    }
    /// Energy to move one kg one metre through liquid of density `rho` kg/m³ at `v` m/s for a member of `m1` kg, the
    /// share `wet` of its height under it, J: the drag on the face it pushes, ½ `rho` `DRAG` (`wet` L²) v², L its size
    /// (`side`), over the share `MUSCLE` of its burning that its muscles turn into that work. At its walking speed
    /// (`speed`), one Froude number, the same for every size: 5.2 J at `wet` 1 in a liquid as dense as bodies, against
    /// the walk's 3.0 at 30 kg and 1.0 at a tonne; a body not built to swim pays several times on land's cost to cross
    /// liquid (Williams 1999).
    pub fn drag(&self, m1: f64, wet: f64, rho: f64, v: f64) -> f64 {
        let side = self.side(m1);
        0.5 * rho * DRAG * wet * side * side * v * v / (m1.max(1e-9) * MUSCLE)
    }
    /// The size of a member of `m1` kg, m: the side of a cube of its volume (`rho_body`). It is how high it stands, how
    /// far it reaches, and the length it walks by (`speed`).
    pub fn side(&self, m1: f64) -> f64 {
        (m1.max(0.0) / self.rho_body).cbrt()
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
    /// The share of members that freeze in a month at `t` K when their lethal limit is `limit` K at full activity `act`
    /// = 1: dormancy hardens them, lowering the limit by `depth` K × (1 - `act`) (a body's `harden`, the cover's own
    /// gene, `cover::frozen`), and their microclimates spread over `frost_k` below it (laws.md 6.5).
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
            c_strength: 1.12e4,
            c_basal: 4.22e4,
            c_leaf: 2e3,
            c_brain: 8.5e4,
            m_ref: 1.0,
            c_hidden: 5e3,
            c_memory: 5e3 / 3.0,
            c_seen: 5e3,
            c_learn: 1.1049e4,
            e_life: 1e9,
            c_repair: 0.25,
            dormancy: 5.7,
            dormant: 0.02,
            lethal: 0.5,
            c_move: 3.0,
            rho_pass: 5.0,
            lane: 1.0,
            kernel_near: 40.0,
            kernel_far: 800.0,
            kernel_far_share: 0.1,
            layer: 10.0,
            rho_body: 1e3,
            meeh: 0.1,
            r_coat: 0.9,
            r_air: 0.12,
            r_liquid: 0.005,
            wet_coat: 0.2,
            take: 0.0089,
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
            abrade: 1e-5,
            c_copy: 1e7,
            c_build: 1e4,
            row_chance: 0.3,
            row_noise: 0.3,
            new_noise: 0.1,
            alike: 4.0,
            node_floor: 0.01,
            c_light: 0.1,
            cover_mut: 0.063,
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
    /// `days` d in the world's months (`calendar`): the time the rates of a body's brain are anchored at, a month
    /// (laws.md 5.4), over which a rate goes as a share, a weight or an amount (`Span`).
    pub(crate) fn span(&self, days: f64) -> Span {
        Span(days * DAY / self.calendar().month as f64)
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
        // A band's core holds at least its mean depth (`flow::air::Heat`).
        require(self.line_spread >= 1.0 && self.line_spread.is_finite(), || format!("line_spread ({}) must be at least 1", self.line_spread))?;
        // `dormancy` above 1 (`body::exchange::closing` divides by n - 1); `light_yield` in (0, 1] (a light reaction takes
        // what it stores over this share).
        require(p.dormancy > 1.0, || format!("dormancy ({}) must be above 1", p.dormancy))?;
        // The bed and the skin of its top are layers of matter: a thickness of zero would give a node no heat capacity.
        require(p.h_mix > 0.0 && p.thermocline >= 0.0, || format!("h_mix ({}) must be above 0 and thermocline ({}) not below", p.h_mix, p.thermocline))?;
        require(self.skin_m > 0.0 && p.bed_depth > 0.0, || format!("skin_m ({}) and bed_depth ({}) must be above 0", self.skin_m, p.bed_depth))?;
        // The moistest air of a node saturates before the node does, and not when it holds nothing (`seen::saturated`).
        require(self.moist_min > 0.0 && self.moist_min < 1.0, || format!("moist_min ({}) must be above 0 and below 1", self.moist_min))?;
        // Rain lands over a width the drift sets (`seen::Seen::landing` divides by it).
        require(self.rain_drift_m > 0.0, || format!("rain_drift_m ({}) must be above 0", self.rain_drift_m))?;
        // Hardening lowers a lethal limit, never raises it.
        require(p.harden >= 0.0 && p.harden.is_finite(), || format!("harden ({}) must be at least 0", p.harden))?;
        // A body's surface and the still air at it give its bare conductance (`Body::bare`): both above 0; a coat adds
        // insulation, never less than none.
        let (meeh, air, coat, film, wet) = (p.meeh, p.r_air, p.r_coat, p.r_liquid, p.wet_coat);
        require(meeh > 0.0 && air > 0.0 && coat >= 0.0 && coat.is_finite(), || format!("meeh ({meeh}) and r_air ({air}) must be above 0, r_coat ({coat}) at least 0"))?;
        // In liquid the same (`Body::conductance`): the still liquid's film above 0, a wet coat a share of a dry one.
        require(film > 0.0 && (0.0..=1.0).contains(&wet), || format!("r_liquid ({film}) must be above 0, wet_coat ({wet}) from 0 to 1"))?;
        // The soil's shares inside a cell (`inside`) divide by it.
        require(p.soil_m > 0.0, || format!("soil_m ({}) must be above 0", p.soil_m))?;
        // A channel's speed is its discharge over its width times its depth (`Physics::channel`).
        require(p.channel_width > 0.0 && p.channel_depth > 0.0, || format!("channel_width ({}) and channel_depth ({}) must be above 0", p.channel_width, p.channel_depth))?;
        let floor = p.node_floor;
        require((0.0..1.0).contains(&floor), || format!("node_floor ({floor}) must be at least 0 and below 1"))?;
        // The dispersal kernel's scales are lengths and its far share a share (`copy::lot`).
        require(p.kernel_near > 0.0 && p.kernel_far > 0.0 && (0.0..=1.0).contains(&p.kernel_far_share), || format!("kernel_near ({}) and kernel_far ({}) must be above 0, kernel_far_share ({}) from 0 to 1", p.kernel_near, p.kernel_far, p.kernel_far_share))?;
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
            line_spread: 4.0,
            rain_time: 86400.0,
            moist_min: 0.7,
            rain_drift_m: 2e4,
            carry: 2e-11,
            wear: 1.4e-11,
            regrow: 1e-9,
            creep: 6.3e-11,
            hard_min: 0.05,
            bury: 1.6e-10,

            null: false,
            phys: Physics::default(),
        }
    }
}

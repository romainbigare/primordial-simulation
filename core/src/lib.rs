//! The world core: one graph, five laws. No IO, no clocks, no Earth words.

pub mod body;
pub mod bond;
pub mod brain;
pub mod change;
pub mod check;
pub mod chem;
pub mod choose;
pub mod copy;
pub mod event;
pub mod flow;
pub mod grid;
pub mod par;
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
    /// Latitude of the map's south and north edges, degrees. The island (128 cells of 2 km) spans
    /// what 256 km spans on the planet, 2.3°, so its climate bands come from height and the
    /// liquid, not latitude. Centred at 32°, where the yearly sunlight keeps the starting
    /// ground near 287 K.
    pub lat_south: f64,
    pub lat_north: f64,
    /// Strength of the deflection of moving air by planet rotation (0 = none)
    pub rotation: f64,
    pub days_per_month: u32,
    pub months_per_year: u32,
    /// Ground and air physics run every `phys_days` days
    pub phys_days: u32,
    /// Gas moves between air nodes (wind) every `air_days` days (radiation every `phys_days`).
    /// How often the moisture path runs sets how much rain reaches land, hence the flowing channels
    /// (seed 5, 50 years, 128x128, no life; channel cells = dry land with liquid flux over 9e3 mol/s):
    /// air/exchange/flow 15/15/15: 240 channel cells; 3/9/15: 484; 3/6/15: 776; 3/3/15: 1405 (+400 ms
    /// a year); 3/3/3: 1369 (+850 ms).
    pub air_days: u32,
    /// Evaporation and condensation (rain) run every `exchange_days` days (a multiple of `air_days`).
    pub exchange_days: u32,
    /// Liquid and creep move every `flow_days` days (a multiple of `air_days`)
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
    /// Transfer speed of a dissolving gas on the liquid's side of its surface (in series with
    /// `evap` on the air's side), m/s. 5e-5 (4 m a day) is the order measured for slightly
    /// soluble gases over open liquid in wind; a column 800 m deep (all mixed as one layer)
    /// then follows the air in about half a year.
    #[serde(default = "dissolve")]
    pub dissolve: f64,
    /// Overturning air speed per K of warmth difference between neighbouring air nodes, m/s
    pub wind: f64,
    /// Mixing of neighbouring air columns by eddies, m²/s
    pub mix: f64,
    /// Condensation in the air at which its droplets cover half the sky, mol/(m² s)
    pub cloud: f64,
    /// Loose solid volume moving liquid carries off per J it dissipates, m³/J (anchored: the sediment of running liquid per J of its runoff, `docs/numbers.md`)
    pub carry: f64,
    /// Bonded volume broken per J dissipated, times (1 / bond strength), m³/J (anchored: the same measured ratio, `docs/numbers.md`)
    pub wear: f64,
    /// Speed at which bodies spread back into ground cleared along an edge, from each side, when
    /// the cover is dense (scaled down by `cover / phys.rho_pass` below that), m/s.
    #[serde(default = "regrow")]
    pub regrow: f64,
    /// Speed of loose solids creeping down a unit slope, times (1 / hardness), m/s (anchored: a hillslope diffusivity, `docs/numbers.md`)
    pub creep: f64,
    /// A speed shortcut, not a law (plan 7.1): past this many body nodes (individuals or
    /// population nodes) in a cell at a month's start, its most alike pairs merge (`body::crowd`),
    /// so a cell may keep more nodes of unlike kinds, up to twice this (past it the most alike
    /// merge however unlike, a safeguard); a node splits only while its cell has fewer than this
    /// minus one (`copy::split`). It never bounds how many members there are.
    #[serde(default = "four")]
    pub crowd: usize,
    /// Bodies that stay near each other share one brain's rows (for speed). When false each
    /// body keeps its own rows: a child starts with its parent's and no body joins another's group.
    #[serde(default = "yes")]
    pub share_brains: bool,
    /// A node also splits along its born rows (`copy::split`), an experiment. Off by default:
    /// the next month's merge takes the halves back (measured 2 to 2.5 times slower at 64x64).
    #[serde(default)]
    pub split_rows: bool,
    /// Whether animals (bodies that are not light-eaters, `body::light_eater`) may be population
    /// nodes. False (`ew run --individuals`): an animal's children are placed as individuals and
    /// never join a node (`copy::copy`), `body::crowd` merges no animal and none splits
    /// (`copy::split`). The detailed form the nodes are checked against.
    #[serde(default = "yes")]
    pub nodes: bool,
    /// Two-parent copying (`copy::partner`). True: a mature body's children have a second parent
    /// when a mature, alike body is in its cell, each gene from one of the two. False
    /// (`ew run --no-mix`): every child is a copy of one parent.
    #[serde(default = "yes")]
    pub two_parents: bool,
    /// Satiety (`choose::sate`; `ew run --sate-all`): a body takes no more of what it burns than its
    /// upkeep plus what it can build and the fuel it converts to do it, whatever its size. False
    /// (the default): a body takes as its intake allows (`Physics::take`) and what it carries is
    /// bounded by its body (`Body::level`), so a full store is the satiety.
    #[serde(default)]
    pub sate_all: bool,
    /// A null run, to be compared with the real run of the same seed: no learning, no imitation
    /// (a watch is drawn and counted but moves no rows), and the brains' born rows and bases never
    /// change at a copy (`copy_phys`); genes still mutate and evolve.
    #[serde(default)]
    pub null: bool,
    /// The tuned numbers of the laws of bodies (M7's world search varies them).
    #[serde(default)]
    pub phys: Physics,
}

/// Free physics numbers of the laws of bodies, grouped by the law that reads them; none is a
/// rule, all are sizes of costs, strengths and spreads. Numerical safeguards, speed schedules and
/// fixed sizes stay consts in the code.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Physics {
    // Change and Flow, as bodies feel them
    /// Liquid thickness over a body that holds back 63% of the light reaching it, m
    pub light_depth: f64,
    /// Liquid thickness over a body that cuts its gas exchange by a factor e, m
    pub gas_depth: f64,
    /// Depth of liquid, standing or in the pores of the ground, at which the ground counts as half wet, m
    pub wet_depth: f64,
    /// Heat the cover of other bodies passes to a body: a layer of `m` kg/m² over it passes
    /// `shelter / m` W per K and m² of its own surface (not of its light-capture area, which is
    /// open to the air above), W kg/(m⁴ K); that part of the body's heat to its surface goes
    /// through it in series. Thicker cover insulates more; bare ground adds nothing. 220 gives a
    /// 30 kg member (52 W of upkeep, 30 W/K) under 10 kg/m² of cover 3.4 K more warmth than bare
    /// ground (thick walls give 3.3 K), and a 1 kg plant member (capture area 2 m²) almost none.
    pub shelter: f64,
    /// Upkeep of catalysts, J per kg of body (`Body::size`) per day per unit of capacity (1/s)
    /// (`body::upkeep`). A body pays for the amount of its catalysts, which is what they can do:
    /// their capacity, the catalysed rate per unit of input at `t_cat`, `c_cat` x the sum over its
    /// catalyst genes that are on of 10^gene x k x exp(-Ea / `t_cat`). One constant serves every
    /// reaction. A light reaction counts the same way: the catalyst is kept whether light comes or
    /// not, so a light-eater is pushed to keep only what its light and gas can feed. Keeping
    /// enzymes costs a resting animal about 10 to 25% of its upkeep in life: the starting eaters'
    /// burn gene (2.6; k 4,300, Ea 7,760 K: 4.1 x 10^-6 /s) costs 6,200 J/kg/day, a tenth of their
    /// upkeep. The capacity counts every gas of a reaction at its share of the open air to its
    /// order (mass action). The five costs of tissue are per kg of body of a kind that matures at
    /// `m_ref`, times (copy mass / `m_ref`)^-0.25 (Kleiber); this one and `c_strength`, `c_learn`,
    /// `c_signal` carry a factor 44.16^0.25 = 2.578 so the eater founder, maturing at 44.16 kg,
    /// pays 2.63 x 10^10.
    pub c_cat: f64,
    /// Temperature at which upkeep measures catalyst capacity, K (a typical surface temperature), and
    /// where the costs are anchored: a body's upkeep is what the costs say at this temperature and goes
    /// by the rate law of its main burning reaction from it, exp(-Ea (1/T - 1/`t_cat`)) (`Body::pace`)
    pub t_cat: f64,
    /// Width over which catalysts lose their shape above their `DENATURE` gene, K: life's enzymes
    /// unfold within a few K; smooth, so the reaction solver never meets a jump (`change::denature`)
    pub unfold: f64,
    /// Catalysts that keep their shape up to `t_firm` K cost no more; each `c_firm` K more of the
    /// `DENATURE` gene adds their upkeep once more (a firmer shape needs more and stronger bonds:
    /// so a catalyst firm when hot costs more). 320 K is about where life's enzymes fail; at 360 K
    /// catalysts cost three times as much
    pub t_firm: f64,
    pub c_firm: f64,
    /// Upkeep of strength, J per kg of body (`Body::size`) per day at strength 1 (`body::upkeep`).
    /// Anchored at the 30 kg founders: 10^5 per kg of the whole there (about 1 W/kg), their body 70%
    /// of their mass: 1.43 x 10^5 per kg of body; x 2.578 (see `c_cat`): 3.68 x 10^5
    pub c_strength: f64,
    /// Upkeep every body pays whatever its genes, J per kg of body per day (`body::upkeep`): the
    /// cost of keeping tissue alive, resting metabolism (Kleiber, about 3.4 m^0.75 W). Per kg of
    /// body of a kind that matures at `m_ref`, times (copy mass / `m_ref`)^-0.25, so an adult pays
    /// 2 x 10^4 x m^0.75, Kleiber's shape (1/15 of his whole value; the rest is strength and
    /// catalysts here). The eater founder pays 7,758 per kg of body.
    pub c_basal: f64,
    /// Upkeep of light-capture area, J per m² per day
    pub c_leaf: f64,
    /// Cost of one decision of the starting brain (12 hidden, 6 memory, 3 seen, sight 1), J per kg
    /// of a member of `m_ref` kg: a member of `m` kg pays `c_brain` x `m_ref` x (`m` / `m_ref`)^0.25
    /// J (`Physics::think`)
    pub c_brain: f64,
    /// The one reference mass, kg: the member at which a per-kg number is per kg. The brain's cost
    /// is listed per kg of a member of this mass (the cost of a decision goes with the brain and
    /// only weakly with the body: a 30 kg member pays 2.3 times what a 1 kg member pays for the
    /// same brain, 13 times less per kg, not 30 times); `take` and `sweep` are per kg of a member
    /// of this mass (above it a member takes less per kg, `Body::intake`); the heat gene is the
    /// conductance per kg of a member of this mass (`Body::conductance`). Three reference masses
    /// for one idea were made one
    pub m_ref: f64,
    /// Cost of one decision per hidden unit, per memory number, per body perceived and per square
    /// cell of sight, J per kg of a member of `m_ref` kg
    pub c_hidden: f64,
    pub c_memory: f64,
    pub c_seen: f64,
    pub c_sight: f64,
    /// Upkeep of learning, J per kg of body per day at learning rate 1 (`body::upkeep`; anchored
    /// at the 30 kg founders, 70% body: 3 x 10^4 per kg there / 0.7 = 4.3 x 10^4; x 2.578, see
    /// `c_cat`: 1.10 x 10^5)
    pub c_learn: f64,
    /// Cost of signalling, J per kg of body per day at signal size 1 (as `c_learn`; anchored at
    /// the 30 kg founders: 200 per kg there / 0.7 = 286; x 2.578: 737)
    pub c_signal: f64,
    /// Energy a kg of body can turn over in its upkeep before it is worn out, at a `REPAIR` gene of
    /// 0, J/kg (`body::live`): it lives 10^gene times this over its upkeep per kg. 10^9 is the
    /// rate-of-living number of real life (Rubner, 1908): many kinds that move turn over about
    /// 1,000 kJ per gram of body in a life, whatever their size, as small ones live fast and short
    /// and big ones slow and long. A starting eater (30 kg, 64 kJ/kg/day) lives 43 years at gene 0, 19 at its -0.4
    pub e_life: f64,
    /// Share of upkeep that is repair at `REPAIR` 0; each unit of the gene (ten times the energy per
    /// kg before wearing out) doubles that share (upkeep x (1 - c + c 2^gene)), so at gene 0 a life
    /// 8 times longer costs 25% more upkeep, and less repair saves upkeep but never more than this
    /// share: remaking a body's own matter (mostly its catalysts) takes 20 to 30% of resting upkeep
    /// in real life. Every value of the gene has a cost (with no saving below 0, selection would
    /// lift a short life to gene 0 for free).
    pub c_repair: f64,
    /// How steeply activity falls as a body dries: a body's chemistry runs in its liquid, so its
    /// activity is
    /// min(1, l / l*)^`dormancy` with l the key liquid it holds and l* the level it keeps
    /// (`Body::activity`). Anchored to the longevity of seeds (Harrington's rule, from memory: seed
    /// life about doubles for each 1% less moisture between about 14% and 5%): a 3-fold drop in
    /// moisture gives about 2^9 = 512 times less activity, so n = ln 512 / ln 3 = 6.24 / 1.10 = 5.7
    /// (14/5 = 2.8 gives 6.1). Upkeep, the speed-up of catalysts, growth and the cost of a decision
    /// go with it (`body::upkeep`, `body::plan`, `copy::grow`, `body::live`)
    pub dormancy: f64,
    // Choose
    /// Energy to move one kg one metre on the flat, J, for a member of `MOVE_REF` (30 kg); other
    /// members pay it times (m1 / 30 kg)^`MOVE_EXP` (`Physics::walk`). The real cost of transport
    /// is about 10.7 m^-0.32 J per kg per m (Taylor et al. 1982, running, walking and flying
    /// animals alike): 3.6 at 30 kg, so 3 fits a founder; 18.5 at 0.18 kg, 1.4 at 600 kg
    pub c_move: f64,
    /// Body mass per area that doubles the cost of entering a cell, kg/m² (and of handling what is
    /// taken from a body there)
    pub rho_pass: f64,
    /// Width of the passage a mover clears through bodies, m
    pub lane: f64,
    /// The bodies of a cell must fit in a layer this thick over it, m, at this density, kg/m³
    pub layer: f64,
    pub rho_body: f64,
    /// Body mass broken per J a mover dissipates against it, times (strength + 0.1), kg/J: the share of a body's
    /// matter broken and of the stores that matter held (`choose::wear`)
    pub walk_wear: f64,
    /// Most a body can take per day of its interval, as a share of the mass it takes by (`Body::supply`:
    /// per member its body, `Body::built`, to the power `intake_exp`), and no more than it can use (`choose::sate`)
    pub take: f64,
    /// What a member takes, sweeps and receives as a gift, what it can build in a day (`c_build`)
    /// and the leaf it spreads (the supply) go with the mass of its body (`Body::supply`) to this
    /// power, about the 0.75 of intake and metabolism in life (`Body::equiv`). The cost of keeping
    /// tissue goes with the body's mass^1.0 (`body::upkeep`), so growth stops where the two meet
    /// (with 1 nothing scaled against size and selection pushed the copy-mass gene to its wall).
    /// A free number chosen for that reason, not a rule.
    pub intake_exp: f64,
    /// Area a body works per day per kg of its mass that works, m²/(kg day): it takes no more per day than
    /// the mass per m² of what it takes there times this times its mass (a 30 kg body works 300
    /// m² in 10 days), so thin cover is hard to find
    pub sweep: f64,
    /// Energy to handle one kg taken or put, J
    pub c_handle: f64,
    /// Spread of the shape of a part made, and of the signal, around their means
    pub shape_noise: f32,
    pub signal_noise: f32,
    /// Weight of well-being one decision ahead against well-being now
    pub discount: f32,
    /// Share of the way a watcher's rows move toward the watched one's (split over its group)
    pub blend: f32,
    /// A body further than `leave` cells from its group's centre leaves it; one within `join`
    /// cells of a bigger group's centre for `stay` days joins it
    pub leave: f32,
    pub join: usize,
    pub stay: u64,
    /// Weight of the newest step in a learned row's mean square of steps, and the most a row
    /// moves in one step in units of the learning rate
    pub learn_beta: f32,
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
    /// Walls lighter than this fall apart, kg (so a part is made of at least this)
    pub min_kg: f64,
    /// A held part multiplies a body's force by 1 + lever · hardness · m / (m + 0.05 body mass)
    pub lever: f64,
    /// Force of a body of strength 1, as a share of its weight
    pub grip: f64,
    /// Push of a body moving through a cell on each structure there, as a share of its weight; its own force must give it to move
    pub bump: f64,
    /// Most bonded ground a body breaks per day of its interval, as a share of its mass
    pub dig: f64,
    /// Liquid deeper than this is crossed afloat by a body holding a structure the liquid holds
    /// up, m, at this share of the cost of walking
    pub deep: f64,
    pub afloat: f64,
    // Copy
    /// Energy to make one kg of body matter, J: the work of building. A member makes matter of the
    /// energy-rich solid and pays this in work per kg made, through `body::pay` (`copy::grow`);
    /// storing fuel costs none, and a child costs its parent no work: it is given matter its parent
    /// built (its brood). Growth in life keeps 10 to 20% of the energy it eats, so a kg of body
    /// costs about 10^7 J; with the 1.11 kg of the solid converted besides (6 x 30 g for 162 g,
    /// 17.3 MJ) a kg of matter takes 27.3 MJ of fuel in all, 62% efficient.
    pub c_copy: f64,
    /// Most work a body can put into building body matter per day, J, for a member of `m_ref` (1 kg);
    /// a member of body `m` kg pays at most this x (`m` / `m_ref`)^`intake_exp` x days (`Body::supply`,
    /// `copy::grow`): building a body takes time, small ones grow fast and big ones slowly. Anchored
    /// to the growth of young in life: a 1 kg animal makes about 1 kg of dry young a year (2.7 x 10^4
    /// J a day at `c_copy`), a 50 kg one about 1 kg a year (1,450 per kg^0.75), a 1 t one about 10 kg
    /// (1,500): anchored to 1,500 to 30,000. Charged on body matter only (17% of a plant's mass) it
    /// lets a 1 kg plant make 0.37 kg of matter, about 1.5 kg of body, a year. A grown body also
    /// takes in no more than its reactions can use, its upkeep plus this and the fuel converted
    /// (`choose::sate`). Marked tuned in `docs/numbers.md`: chosen within its range by what runs did
    pub c_build: f64,
    /// Chance that a child starts a new lineage with a changed base, and the size of that change
    pub new_lineage: f64,
    pub base_noise: f32,
    /// Chance that a child's born brain rows change, and the step of that change per generation
    /// (times the body's mutation size, bigger by 1 / sqrt(chance))
    pub row_chance: f64,
    pub row_noise: f32,
    /// Spread of the weights a bigger brain adds
    pub new_noise: f32,
    /// Nodes in one cell whose genes differ by at most this many mutation steps in each merge
    /// (`copy::alike`). A node splits when its two halves would differ by more, in a gene or in
    /// its born rows (in row steps, `copy::split`). Children join a node within this many steps
    /// plus two standard deviations of its spread and theirs (`copy::within`)
    pub alike: f32,
    // Flow, as ground cover and moving liquid feel it
    /// Thickness of a layer that hides half of what lies below it, m: [soft solid, liquid, loose]
    pub hide: [f64; 3],
    /// A solid warmer than this share of its melting point is soft cover lying on top
    pub soft: f64,
    /// Liquid leaving a node goes only to neighbours at least this share as steep as the steepest
    pub split: f64,
    /// Liquid does not move for a smaller head difference, m
    pub still: f64,
    /// Body mass per area that halves what moving liquid can carry off a cell and how fast its
    /// loose solids creep, kg/m²
    pub hold: f64,
    /// Share of the volume of the loose ground that is pore space: the liquid of a cell soaks into the
    /// pores up to this share of the volume of its loose solids, and only the liquid over that stands and runs off
    /// (`flow::liquid`). Grains packed at random leave 0.36 (the random close packing of spheres, Scott 1960) to 0.44 (the
    /// loose packing); sands 0.35 to 0.45, loams 0.4 to 0.5. 0.4 (from memory, unchecked)
    pub porosity: f64,
    /// Hydraulic conductivity of the loose ground, m/s: Darcy's law, the pore liquid of a cell flows to a
    /// neighbour at q = `seep` x the slope of the liquid head, through the thickness of ground it fills (`flow::liquid`).
    /// A loam passes 1.3 cm an hour, 3.6e-6 m/s (Rawls et al. 1992 teaching table; sand 5.8e-5, clay about 2e-7)
    pub seep: f64,
    /// Drainage (`flow::drain`): share of its pores the loose ground holds against gravity (its field
    /// capacity); the rest sinks, at `seep` at the most, into the store of the bed below. A loam holds 0.25 to 0.32 of
    /// its volume at field capacity in 0.47 of pores (Saxton and Rawls 2006, as tabled by the Minnesota stormwater
    /// manual): 0.6
    pub field: f64,
    /// The store under the loose ground: the weathered and fractured zone of the bed, `bed_depth` m deep, of which
    /// `bed_porosity` is pore space that drains (its specific yield), and through which its liquid flows by Darcy's law
    /// at `bed_seep`, m/s, through the depth it fills. Weathered basement: 10 to 60 m deep, yield 0.01 to 0.05 (from
    /// memory), transmissivity 4 to 50 m2 a day (saprolite, Western Australia): 30 m, 0.02, 3e-6 (7.8 m2 a day full)
    pub bed_depth: f64,
    pub bed_porosity: f64,
    pub bed_seep: f64,
    /// Half the spacing of the channels inside a cell, m: the length of a hillslope, L = 1 / (2 x the drainage density)
    /// (Horton 1945). The liquid of the loose ground over its field capacity and of the bed's store flows down the
    /// cell's slope to them by Darcy's law and runs off (`flow::liquid`). Drainage densities of 1 to 5 km per km2
    /// (Strahler 1957, Horton 1945; from memory) give 100 to 500 m: 2 km per km2, 250 m
    pub hill: f64,
}

impl Physics {
    /// Energy of one decision of a brain whose cost is `c` J per kg of a member of `m_ref` kg
    /// (`body::Lineage::cost`), for a member of `m` kg, J: weakly more for a bigger body. Paid per
    /// decision, not per day (a speed incentive kept on purpose: it pushes bodies toward long
    /// intervals, plan 7.3; handover 6.5).
    pub fn think(&self, c: f64, m: f64) -> f64 {
        c * self.m_ref * (m / self.m_ref).powf(0.25)
    }
    /// Energy to move one kg one metre on the flat for a member of `m1` kg, J: `c_move` at 30 kg,
    /// and (m1 / 30)^-0.32 times that otherwise, the real cost of transport (Taylor et al. 1982,
    /// 10.7 m1^-0.32 J per kg per m), so what a body pays to move goes with its mass^0.68.
    /// Climbing is not in it: that is the weight lifted (`Config::gravity`).
    pub fn walk(&self, m1: f64) -> f64 {
        self.c_move * (m1 / MOVE_REF).powf(MOVE_EXP)
    }
}

/// The mass at which `Physics::c_move` is the cost of moving a kg a metre, kg: the 30 kg founders
/// (10.7 x 30^-0.32 = 3.6 J, near `c_move` 3).
pub const MOVE_REF: f64 = 30.0;
/// The exponent of the cost of transport per kg and m against the mover's mass (Taylor et al. 1982)
pub const MOVE_EXP: f64 = -0.32;

impl Default for Physics {
    fn default() -> Self {
        Physics {
            light_depth: 10.0,
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
            walk_wear: 3e-5,
            take: 0.05,
            sweep: 1.0,
            intake_exp: 0.75,
            c_handle: 1e3,
            shape_noise: 0.5,
            signal_noise: 0.3,
            discount: 0.9,
            blend: 0.3,
            leave: 2.5,
            join: 2,
            stay: 360,
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

fn yes() -> bool {
    true
}
fn dissolve() -> f64 {
    5e-5
}
fn regrow() -> f64 {
    1e-9
}
fn four() -> usize {
    4
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sun: 1361.0,
            tilt: 23.4,
            lat_south: 31.0,
            lat_north: 33.3,
            rotation: 1.0,
            days_per_month: 30,
            months_per_year: 12,
            phys_days: 3,
            air_days: 3,
            exchange_days: 3,
            flow_days: 15,
            skin_m: 2.0,
            air_top_m: 10_000.0,
            gravity: 9.81,
            sensible: 10.0,
            evap: 0.006,
            dissolve: 5e-5,
            wind: 0.25,
            mix: 2e4,
            cloud: 1.2e-2,
            carry: 2e-11,
            wear: 1.4e-11,
            creep: 6.3e-11,
            regrow: 1e-9,
            crowd: 4,
            share_brains: true,
            split_rows: false,
            nodes: true,
            two_parents: true,
            sate_all: false,
            null: false,
            phys: Physics::default(),
        }
    }
}

impl Config {
    /// The numbers the copy law reads: in a null run the born rows and the brain bases do not mutate.
    pub fn copy_phys(&self) -> Physics {
        if self.null { Physics { row_noise: 0.0, base_noise: 0.0, ..self.phys } } else { self.phys }
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
    /// Heat a ground node could not give (it would have fallen below `T_FLOOR`), J: whoever asked
    /// received that much less, so this is not a loss of energy.
    #[serde(default)]
    pub clamped_heat: f64,
}

/// A typical surface temperature, K: where the upkeep of catalysts is measured (`Physics::t_cat`)
/// and where the starting kinds and the richness checks look at the phase of a substance.
pub const T_SURF: f64 = 290.0;

/// The coldest a ground node may be taken by drawing heat from it, K.
pub const T_FLOOR: f64 = 3.0;

/// `World::take_heat` for callers that hold other parts of the world borrowed: take up to `q` from
/// a node with energy `e` and heat capacity `cap` (at most what it holds above `T_FLOOR`), count
/// the shortfall, return what was taken.
#[inline]
pub fn draw(e: &mut f64, ledger: &mut Ledger, cap: f64, q: f64) -> f64 {
    let took = if q > 0.0 { q.min(heat_above_floor(*e, cap)) } else { q };
    *e -= took;
    ledger.clamped_heat += q - took;
    took
}

/// The share of a wanted amount that was really given (exactly 1 when all of it was).
#[inline]
pub fn share(took: f64, want: f64) -> f64 {
    if took >= want { 1.0 } else { took / want }
}

/// Heat capacity of a ground node's active layer, J/K (from the fields of the world).
pub fn cap_ground(chem: &Chemistry, grid: &Grid, cfg: &Config, ground: &[f64], bed: &[f64], i: usize) -> f64 {
    let n = grid.n();
    let (mut c, mut bc, mut bv) = (0.0, 0.0, 0.0);
    for k in 0..chem.ns() {
        let s = &chem.subs[k];
        c += ground[k * n + i] * s.cp;
        let b = bed[k * n + i];
        bc += b * s.cp;
        bv += b * s.molar_mass / s.density[0];
    }
    if bv > 0.0 {
        c += bc / bv * cfg.skin_m * grid.area();
    }
    c
}

/// Heat a node with energy `e` and heat capacity `cap` can give before it falls to `T_FLOOR`, J.
#[inline]
pub fn heat_above_floor(e: f64, cap: f64) -> f64 {
    (e - cap.max(1.0) * T_FLOOR).max(0.0)
}

/// Per-cell slow averages, useful to see the world (not read by the laws).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Diag {
    /// Liquid moved out of each ground cell, mol/s (moving average)
    pub liquid_flux: Vec<f32>,
    /// Liquid falling on each ground cell, mol/(m² s) (moving average)
    pub rain: Vec<f32>,
    /// Members moving into each ground cell, per year (moving average over about a year), and
    /// since the last month
    pub traffic: Vec<f32>,
    pub crossings: Vec<f32>,
    /// Decisions drawn since the shell last read them, and of them those where the action drawn
    /// had the highest score (not saved)
    #[serde(skip)]
    pub decisions: f64,
    #[serde(skip)]
    pub greedy: f64,
    /// Population nodes split since the start: in two (`copy::split`), and one member off to act
    /// alone (`copy::one`)
    #[serde(default)]
    pub splits: [f64; 2],
    /// Members merged by the monthly merge however unlike (the safeguard past twice `Config::crowd`
    /// nodes in a cell, `body::crowd`), since the shell last read them. A count only, not saved.
    #[serde(skip)]
    pub unlike: f64,
    /// Members that joined a node (the monthly merge, `body::crowd`, and a child's join, `copy::copy`)
    /// whose means differ from the node's by more than `Physics::alike` steps in some gene
    /// (`copy::alike`), since the shell last read them. A count only, not saved.
    #[serde(skip)]
    pub wide: f64,
    /// Members that moved to a neighbour cell since the shell last read them, by starting kind
    /// (`Lineage::root`): a node moving counts all its members (`choose::go`). A count only, not saved.
    #[serde(skip)]
    pub moves: Vec<f64>,
    /// Body matter, kg, that hits tore off members who lived on (bites that left them smaller, bodies
    /// trampled, `body::tear`), since the shell last read it, by starting kind. A count only, not saved.
    #[serde(skip)]
    pub hurt: Vec<f64>,
    /// Members that could build body matter in an update, by starting kind, as [tried, held back by any mineral, by
    /// the first, by the second...]: held back are those whose store of a mineral (`Chemistry::minerals`) bound what they built,
    /// below what their fuels, their allowance and their spare allowed (`copy::grow`). A count only, not saved.
    #[serde(skip)]
    pub nlim: Vec<Vec<f64>>,
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
    /// Bodies and made parts (the sparse graph)
    pub life: body::Life,
    #[serde(default)]
    pub parts: bond::Parts,
    /// Light-capture area of the bodies of each cell per ground area (they intercept 1 - exp(-a))
    pub canopy: Vec<f32>,
    /// Light intercepted in each cell per m² of light-capture area since the start (the mean of
    /// its bodies: taller members catch more, `body::shade`), J/m²
    pub light: Vec<f64>,
    /// Ground cleared of bodies along each edge, m²: edge `4i + f` joins cell `i` to its
    /// neighbour `FORWARD[f]` (each edge once, used both ways). Movers pushing through bodies raise
    /// it (`choose::go`), cover growing back lowers it (`choose::regrow`). Empty until first used.
    #[serde(default)]
    pub pass: Vec<f32>,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Working memory of the laws, rebuilt when missing (saved, so a resumed run goes on the same)
    pub scratch: flow::Scratch,
    // Fields added later go last: a snapshot without them loads them empty (`tape::load_snapshot`).
    /// Light set aside in each cell for what its bodies' light reactions will store, J: put
    /// aside as it falls (`flow::air::radiation`), not as heat; a body takes its share at its
    /// update (`body::live`), what no body can claim goes to the ground's heat (`body::survey`).
    #[serde(default)]
    pub light_pool: Vec<f64>,
    /// Per body slot: the share of the light it caught that its light reactions stored at its
    /// last update (J per J), with the slot's generation (`World::keep_of`). Its cell sets that
    /// share of the light it absorbs aside.
    #[serde(default)]
    pub light_keep: Vec<(u32, f32)>,
    /// Liquid (with what is dissolved in it) in the store of the bed under each ground node, [k * n + i], mol, and its
    /// thermal energy with the fusion enthalpy of what is molten, J (`flow::drain`; empty until first used)
    #[serde(default)]
    pub deep: Vec<f64>,
    #[serde(default)]
    pub e_deep: Vec<f64>,
}

impl World {
    /// The share of the light it caught that the body in `slot` (of generation `gen`) stored at
    /// its last update (none before its first).
    pub fn keep_of(&self, slot: usize, gen: u32) -> f32 {
        self.light_keep.get(slot).filter(|k| k.0 == gen).map_or(0.0, |k| k.1)
    }
    pub fn set_keep(&mut self, slot: usize, gen: u32, k: f32) {
        if self.light_keep.len() <= slot {
            self.light_keep.resize(slot + 1, (u32::MAX, 0.0));
        }
        self.light_keep[slot] = (gen, k);
    }
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
        cap_ground(&self.chem, &self.grid, &self.cfg, &self.ground, &self.bed, i)
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
    /// Take up to `q` J from ground node `i`, never more than it holds above `T_FLOOR`, and return what was
    /// really taken. The shortfall is counted in `ledger.clamped_heat`. Whoever wanted `q` must
    /// pass on only the returned amount (evaporate less, carry less), so energy stays exact.
    /// A negative `q` gives heat to the node and is returned whole.
    pub fn take_heat(&mut self, i: usize, q: f64) -> f64 {
        self.take_heat_with(i, q, 0.0)
    }
    /// `take_heat` for heat that leaves with matter of heat capacity `c` J/K (still counted in
    /// the node): only the heat capacity that stays must be left above `T_FLOOR`.
    pub fn take_heat_with(&mut self, i: usize, q: f64, c: f64) -> f64 {
        // Only a take needs the capacity (a walk over the node's substances): a body taking from
        // the ground asks once per substance, mostly for nothing.
        let cap = if q > 0.0 { self.heat_cap_ground(i) - c } else { 0.0 };
        draw(&mut self.e_ground[i], &mut self.ledger, cap, q)
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
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    step_marked(w, &mut |_| {});
}

/// Debug builds only: stop at the first law part that leaves the world broken (the yearly check of
/// the shell comes too late to say which part did it). Release builds skip this.
#[inline]
fn audit(w: &World, part: &str) {
    if cfg!(debug_assertions) {
        if let Some(e) = check::broken(w) {
            panic!("broken after {part} (day {}): {e}", w.day);
        }
    }
}

/// `step`, calling `mark(part)` after each part (0 flow, 1 change, 2 bodies, 3 parts), e.g. to
/// time them.
pub fn step_marked(w: &mut World, mark: &mut dyn FnMut(usize)) {
    let dpm = w.cfg.days_per_month;
    for d in 0..dpm {
        if d % w.cfg.phys_days == 0 {
            let dt = w.cfg.phys_days.min(dpm - d) as f64 * 86_400.0;
            body::survey(w);
            mark(2);
            audit(w, "survey");
            flow::step(w, dt);
            mark(0);
            audit(w, "flow");
            change::step(w, dt);
            mark(1);
            audit(w, "change");
        }
        if d == 0 {
            bond::step(w);
            mark(3);
            audit(w, "bond");
        }
        body::day(w);
        w.day += 1;
        mark(2);
        audit(w, "body day");
    }
}

//! The world's state: the fields of ground and air, the bodies and parts, the nodes beyond its
//! boundary that book what crossed it, and the counters the shell reads.

use crate::chem::Chemistry;
use crate::event::Event;
use crate::params::Config;
use crate::space::square::Square;
use crate::space::Mesh;
use crate::store::BOUNDARIES;
use crate::{bond, flow};
use serde::{Deserialize, Serialize};

/// Counts and slow averages for the shell: the laws never read them. The fields not saved (`serde(skip)`) count since
/// the shell last read them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    /// Members moving into each ground cell, per year (moving average over about a year), and since the last month
    pub traffic: Vec<f32>,
    pub crossings: Vec<f32>,
    /// Decisions drawn, and of them those where the action drawn had the highest score
    #[serde(skip)]
    pub decisions: f64,
    #[serde(skip)]
    pub greedy: f64,
    /// Population nodes split since the start: in two (`nodes::split`), and one member off to act alone (`nodes::one`)
    pub splits: [f64; 2],
    /// Body updates since the start
    pub updates: u64,
    /// Heat a ground node could not give (it would have fallen below `T_FLOOR`), J: whoever asked received that much
    /// less, so this is not a loss of energy
    pub clamped_heat: f64,
    /// What bodies did with parts
    #[serde(skip)]
    pub parts: bond::Tally,
    /// What `store::apply` set from just below zero (rounding) to zero: mol, and J
    #[serde(skip)]
    pub rounded: [f64; 2],
}

/// One number per body slot, kept with the generation of the body that set it: a body reads only its own (0 for a new
/// body in a reused slot, and before its first set).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PerSlot(Vec<(u32, f32)>);

impl PerSlot {
    /// The number set by the body in `slot` of generation `gen` (0 if it set none).
    pub fn get(&self, slot: usize, gen: u32) -> f32 {
        self.0.get(slot).filter(|k| k.0 == gen).map_or(0.0, |k| k.1)
    }
    pub fn set(&mut self, slot: usize, gen: u32, v: f32) {
        if self.0.len() <= slot {
            self.0.resize(slot + 1, (u32::MAX, 0.0));
        }
        self.0[slot] = (gen, v);
    }
    /// Every slot's (generation, number); `u32::MAX` for a slot never set.
    pub fn entries(&self) -> &[(u32, f32)] {
        &self.0
    }
}

/// The whole state of a world: what `step` advances, and what a snapshot saves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub cfg: Config,
    pub chem: Chemistry,
    /// The map of squares the world is laid out on, and the ground cells along a side of the block under each air node
    pub map: Square,
    pub air_scale: usize,
    /// The shape of the world (`World::mesh_of`): its cells are the ground nodes, the cells of its level `AIR` the air
    /// nodes. Built from `map` and `air_scale`, so not saved (a loaded world rebuilds it).
    #[serde(skip)]
    pub mesh: Mesh,
    /// The id, class, parent and cell of every node (`store::Nodes`): the mesh's, and the parts', which come and go
    pub table: crate::store::Nodes,
    /// Seed of every keyed random draw (`rng::Rng`)
    pub seed: u64,
    /// World clock, seconds since the start (`clock`)
    pub time: u64,
    /// The alarms of the nodes' own processes, and when each was last brought up to date (`clock::Sleep`)
    pub sleep: crate::clock::Sleep,
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
    /// Liquid falling on each ground cell, mol/(m² s), a moving average over about a year (`flow::air::exchange`); bodies
    /// catch it (`body::live`)
    pub rain: Vec<f32>,
    /// Liquid moved out of each ground cell, mol/s, a moving average over about a year (`flow::liquid`); it pushes parts
    /// (`bond::load`)
    pub liquid_flux: Vec<f32>,
    pub counts: Counters,
    /// Amounts in the boundary nodes (`store::Boundary`: sun, space, outside), [k * BOUNDARIES + b], mol, and their heat,
    /// J: what crossed the world's boundary, as a node holds it (below 0 for what it gave; `World::cross`)
    pub boundary: Vec<f64>,
    pub e_boundary: [f64; BOUNDARIES],
    /// Bodies, and the parts they made: the sparse graph
    pub life: crate::life::Life,
    pub parts: bond::Parts,
    /// Light-capture area of the bodies of each cell per ground area (they intercept 1 - exp(-a))
    pub canopy: Vec<f32>,
    /// Light intercepted in each cell per m² of light-capture area since the start (the mean of
    /// its bodies: taller members catch more, `survey::shade`), J/m²
    pub light: Vec<f64>,
    /// What the links of the cell level remember (`store::Links`)
    pub links: crate::store::Links,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Working memory of the laws, rebuilt when missing (saved, so a resumed run goes on the same)
    pub scratch: flow::Scratch,
    /// What Change books at a step, kept between steps (not saved)
    #[serde(skip)]
    pub batches: crate::change::Batches,
    /// Light set aside in each cell for what its bodies' light reactions will store, J: put
    /// aside as it falls (`flow::air::radiation`), not as heat; a body takes its share at its
    /// update (`body::live`), what no body can claim goes to the ground's heat (`survey::survey`).
    pub light_pool: Vec<f64>,
    /// Per body slot: the share of the light it caught that its light reactions stored at its
    /// last update (J per J). Its cell sets that share of the light it absorbs aside.
    pub light_keep: PerSlot,
    /// Liquid (with what is dissolved in it) in the store of the bed under each ground node, [k * n + i], mol, and its
    /// thermal energy with the fusion enthalpy of what is molten, J (`flow::drain`; empty until first used)
    pub deep: Vec<f64>,
    pub e_deep: Vec<f64>,
    /// Potential energy the world's matter has gained since the start, J: what lifting it took from heat (gas into the
    /// air and between air nodes) less what falls gave back (rain, liquid and solids downhill); the heat of the store's
    /// potential node (`store::Class::Potential`)
    pub potential: f64,
    /// Heat set aside in each cell for what its bodies draw from the ground between their updates, J (below 0: given to the
    /// ground ahead): each survey holds here what each body drew a second at its last update times the time since
    /// (`survey::survey`), so the ground pays it as it goes and a body's update settles only the difference (`body::live`).
    pub heat_pool: Vec<f64>,
    /// Per body slot: the heat it drew from the ground at its last update, W.
    pub heat_keep: PerSlot,
}

/// The level of the world's mesh its air nodes live on.
pub const AIR: usize = 1;

impl World {
    /// The mesh of `map`, with air nodes over blocks of `air_scale`² squares, its rows spread between the latitudes of
    /// `cfg`.
    pub fn mesh_of(map: &Square, air_scale: usize, cfg: &Config) -> Mesh {
        map.mesh(&[air_scale], cfg.lat_south, cfg.lat_north)
    }
    pub fn days_per_year(&self) -> u64 {
        let cal = self.cfg.calendar();
        cal.days(cal.year)
    }
    /// Whole days since the start: the day it is
    pub fn day(&self) -> u64 {
        self.cfg.calendar().days(self.time)
    }
    /// Whole months since the start
    pub fn month(&self) -> u64 {
        self.cfg.calendar().months(self.time)
    }
    /// Years since the start, with their fraction
    pub fn year(&self) -> f64 {
        self.time as f64 / self.cfg.calendar().year as f64
    }
    /// Fraction of the year, 0..1
    pub fn season(&self) -> f64 {
        self.cfg.calendar().season(self.time)
    }
    /// Whether ground node `i` is land: its solid surface above 0 m (`surface_z`).
    pub fn is_land(&self, i: usize) -> bool {
        self.surface_z(i) > 0.0
    }
    /// Loose amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn g(&self, k: usize, i: usize) -> f64 {
        self.ground[k * self.mesh.cells().n() + i]
    }
    /// Bonded amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn b(&self, k: usize, i: usize) -> f64 {
        self.bed[k * self.mesh.cells().n() + i]
    }
    /// Amount of substance `k` in air node `j`, mol.
    #[inline]
    pub fn a(&self, k: usize, j: usize) -> f64 {
        self.air[k * self.mesh.level(AIR).n() + j]
    }
}

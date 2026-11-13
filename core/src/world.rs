//! The world's state: the store's nodes (ground, air, bed, deep stores, what life sets aside, the nodes beyond its
//! boundary that book what crossed it), the cover, and the counters the shell reads.

use crate::{chem::Chemistry, event::Event, flow, params::Config, space::{square::Square, Mesh}, store::{Class, Node, Store}};
use serde::{Deserialize, Serialize};

/// Counts and slow averages for the shell; the laws never read them. The fields not saved count since the shell last read them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    /// Heat a ground node could not give (below `T_FLOOR`), J: whoever asked got that much less, so no energy is lost
    pub clamped_heat: f64,
    /// What `store::apply` and the booking by array (`book_by_array`, `add_rounded`) set from just below zero (rounding)
    /// to zero: mol, and J
    #[serde(skip)]
    pub rounded: [f64; 2],
    /// Cover that died, kg, by cause (`life::LACK` to `AGE`)
    #[serde(skip)]
    pub cover_lost: [f64; 6],
    /// The flows of the cover's budget, J, + into the cover (`cover::BUDGET`)
    #[serde(skip)]
    pub cover_budget: [f64; crate::cover::BUDGET.len()],
    /// Heat that crossed into the map at its open north, south, east and west edges since the start, J: sensible and
    /// latent
    pub edges: [f64; 4],
}

impl Counters {
    /// The counts `c` of a chunk are added (`World::take_chunks`), and start again. Every field is named, so a new one
    /// cannot be left out.
    pub(crate) fn add(&mut self, c: &mut Counters) {
        let Counters { clamped_heat, rounded, cover_lost, cover_budget, edges } = std::mem::take(c);
        self.clamped_heat += clamped_heat;
        self.add_rounded(rounded);
        self.cover_lost.iter_mut().zip(cover_lost).for_each(|(a, b)| *a += b);
        self.cover_budget.iter_mut().zip(cover_budget).for_each(|(a, b)| *a += b);
        self.edges.iter_mut().zip(edges).for_each(|(a, b)| *a += b);
    }
    /// What a booking set from just below zero to zero is added (`Counters::rounded`): mol, and J.
    pub(crate) fn add_rounded(&mut self, [mol, heat]: [f64; 2]) {
        (self.rounded[0], self.rounded[1]) = (self.rounded[0] + mol, self.rounded[1] + heat);
    }
}

/// The counts the shell takes once a year (`World::take_counters`).
#[derive(Debug, Default)]
pub struct Taken {
    pub cover_lost: [f64; 6],
    pub cover_budget: [f64; crate::cover::BUDGET.len()],
}

/// The whole state of a world: what `step` advances, and what a snapshot saves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub cfg: Config,
    pub chem: Chemistry,
    /// The map of squares the world is laid out on, its air blocks and the tree of its cells under them
    pub map: Square,
    /// The shape of the world: its cells are the ground nodes, those of level `AIR` the air nodes (built from `map`, not saved)
    #[serde(skip)]
    pub mesh: Mesh,
    /// Seed of every keyed random draw (`rng::Rng`)
    pub seed: u64,
    /// World clock, seconds since the start (`clock`)
    pub time: u64,
    /// The alarms of the nodes' own processes, and when each was last brought up to date (`clock::Sleep`)
    pub sleep: crate::clock::Sleep,
    /// The amounts and heat of the store's fixed nodes, which only `store::apply` and `store::book_by_array` write
    pub store: Store,
    /// Elevation of the bottom of the ground column, m
    pub base_z: f64,
    /// Liquid falling on each ground cell, mol/(m² s), a moving average over about a year, for the tape
    pub rain: Vec<f32>,
    /// The liquid that runs through each of the terrain's samples down its way, mol/s (`flow::runoff`), and of each
    /// ground cell the most through one of its samples, its main channel: moving averages over about a year
    pub discharge: Vec<f32>,
    pub liquid_flux: Vec<f32>,
    /// What rises from each ground cell into the air less what condenses on it, mol/(m² s), and what runs over the
    /// surface of each land cell into cells that are not land (`is_land`), mol/s: moving averages over about a year,
    /// for the tape
    pub evap: Vec<f32>,
    pub off_land: Vec<f32>,
    pub counts: Counters,
    /// The life of each cell: its cover (`life`)
    pub life: crate::life::Life,
    /// Light intercepted in each cell per m² of light-capture area since the start (its cover's mean), J/m²
    pub light: Vec<f64>,
    /// The terrain the world was built on (`space::terrain`)
    pub terrain: crate::space::terrain::Terrain,
    /// The mean rise of the ground of each cell toward the south, m per m, which the sun on it reads (`flow::on_slope`):
    /// the terrain's, found when the world is built (`World::rise_at_build`)
    pub rise_south: Vec<f64>,
    /// The loose ground of each cell, the share of its area in each bin of depth, which its cover's roots read
    /// (`cover::loose_bins`): the terrain's samples', found when the world is built
    pub loose_bins: Vec<[f32; crate::cover::BINS]>,
    /// The planet's air beyond the map's edges by row of latitude and month (`flow::Planet`), where they are open
    pub planet: Option<crate::flow::Planet>,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Flow's working memory: its state (saved) and its caches (rebuilt when missing, `flow::Scratch`)
    pub scratch: flow::Scratch,
}

/// The level of the world's mesh its air nodes live on.
pub const AIR: usize = 1;

impl World {
    /// The mesh of `map`, an air node over each of its blocks, its rows between the latitudes of `cfg`.
    pub fn mesh_of(map: &Square, cfg: &Config) -> Mesh {
        map.mesh(cfg.lat_south, cfg.lat_north)
    }
    /// The rise toward the south of each cell of a world as built, m per m: the terrain's inside it
    /// (`Terrain::rise_south`).
    pub fn rise_at_build(&self) -> Vec<f64> {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        crate::par::map_n(self.mesh.cells().n(), crate::par::UNCOUNTED, |i| self.terrain.rise_south(self.terrain.seed, &self.map, self.map.leaves()[i], lat))
    }
    /// Hand the shell the raw events since it last asked, and start the list again.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// Hand the shell the yearly counts (`Counters::cover_lost`, `cover_budget`), and start them again.
    pub fn take_counters(&mut self) -> Taken {
        let c = &mut self.counts;
        Taken { cover_lost: std::mem::take(&mut c.cover_lost), cover_budget: std::mem::take(&mut c.cover_budget) }
    }
    pub fn days_per_year(&self) -> u64 {
        self.cfg.calendar().days(self.cfg.calendar().year)
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
    /// The amounts of node `x` of the store (a ground, bed, bodies' or air node): of substance `k`, mol, at `k`.
    #[inline]
    pub(crate) fn stock(&self, x: Node) -> impl Fn(usize) -> f64 + '_ {
        let (a, n) = (self.store.amounts(x.class), if x.class == Class::Air { self.mesh.level(AIR).n() } else { self.mesh.cells().n() });
        move |k| a[k * n + x.index]
    }
    /// Loose amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn g(&self, k: usize, i: usize) -> f64 {
        self.stock(Node::ground(i))(k)
    }
    /// Bonded amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn b(&self, k: usize, i: usize) -> f64 {
        self.stock(Node::new(Class::Bed, i))(k)
    }
    /// Amount of substance `k` in air node `j`, mol.
    #[inline]
    pub fn a(&self, k: usize, j: usize) -> f64 {
        self.stock(Node::air(j))(k)
    }
    /// Amount of substance `k` that the cover sees in air node `j`, mol: what it holds (`a`) and what the aside nodes
    /// under it hold until the air's step takes it up.
    pub fn a_seen(&self, k: usize, j: usize) -> f64 {
        let kids = self.mesh.level(AIR).children(j);
        self.a(k, j) + kids.iter().map(|&i| self.stock(Node::aside(i as usize))(k)).sum::<f64>()
    }
}

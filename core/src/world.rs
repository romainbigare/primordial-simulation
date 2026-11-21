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
    /// Cover that died, kg, by cause (`life::LACK` to `FLAME`)
    #[serde(skip)]
    pub cover_lost: [f64; crate::life::DEATHS],
    /// The flows of the cover's budget, J, + into the cover (`cover::BUDGET`)
    #[serde(skip)]
    pub cover_budget: [f64; crate::cover::BUDGET.len()],
    /// The flows of the seeds, kg (`cover::SEED_FLOWS`)
    #[serde(skip)]
    pub seeds: [f64; crate::cover::SEED_FLOWS.len()],
    /// Takers that died, members, by cause (`life::LACK` to `DROWNED`)
    #[serde(skip)]
    pub takers_died: [f64; crate::life::DEATHS],
    /// What the cover of a cell built in a month, kg of dry matter (cell, kg), until its month ends (`cover::landed`)
    #[serde(skip)]
    pub grown: Vec<(u32, f64)>,
    /// Per cell, what its cover built in the year so far, and in the last whole year, kg of dry matter
    pub grown_year: Vec<f64>,
    pub grown_last: Vec<f64>,
    /// What flames did on a cell in a month (cell, `flame::Flamed`), until the month's flames end (`flame::month`)
    #[serde(skip)]
    pub flamed: Vec<(u32, crate::flame::Flamed)>,
    /// Per cell, what flames did on it in the last month (`flame::Flamed`)
    pub flame_month: Vec<crate::flame::Flamed>,
    /// Per cell, the share of its land flames burnt in each of the last months of a year, by the month of the year
    /// (`Config::months_per_year` a cell), and the months since it last burnt (`flame::since`)
    pub burnt_months: Vec<f32>,
    pub burnt_age: Vec<f32>,
    /// What flames burnt and killed, kg (`flame::FLOWS`), and the mol of each substance they gave the air
    #[serde(skip)]
    pub flame: [f64; crate::flame::FLOWS.len()],
    #[serde(skip)]
    pub flame_air: Vec<f64>,
    /// Heat that crossed into the map at its open north, south, east and west edges since the start, J: sensible and
    /// latent
    pub edges: [f64; 4],
    /// Per cell, the load the liquid in its main channel carried in the last month, g per m³ of that liquid, and the
    /// depth of load that settled on its ground in the month, m (`flow::runoff`): what a shell draws turbid channels
    /// and fresh deposits by
    #[serde(skip)]
    pub load_month: Vec<[f32; 2]>,
    /// The samples where load came into a body of standing liquid in the last month, each with how much, kg/s (a
    /// shell's plumes)
    #[serde(skip)]
    pub plumes: Vec<(u32, f32)>,
    /// The liquid that crossed into the map's open low ground at its open edges since the start (`flow::runoff`): m³,
    /// and its heat, J
    pub edge_liquid: [f64; 2],
    /// The load of the running liquid since the start (`flow::runoff::load`), m³: picked up, carried into the open low
    /// ground, carried into the other bodies, settled on land, and the bed worn into loose ground
    pub load: [f64; 5],
}

impl Counters {
    /// The counts `c` of a chunk are added (`World::take_chunks`), and start again. Every field is named, so a new one
    /// cannot be left out.
    pub(crate) fn add(&mut self, c: &mut Counters) {
        let Counters { clamped_heat, rounded, cover_lost, cover_budget, seeds, takers_died, grown, grown_year, grown_last, flamed, flame_month, burnt_months, burnt_age, flame, flame_air, edges, edge_liquid, load_month, plumes, load } = std::mem::take(c);
        self.clamped_heat += clamped_heat;
        self.add_rounded(rounded);
        self.cover_lost.iter_mut().zip(cover_lost).for_each(|(a, b)| *a += b);
        self.cover_budget.iter_mut().zip(cover_budget).for_each(|(a, b)| *a += b);
        self.seeds.iter_mut().zip(seeds).for_each(|(a, b)| *a += b);
        self.takers_died.iter_mut().zip(takers_died).for_each(|(a, b)| *a += b);
        self.grown.extend(grown);
        // (A chunk's counts hold no year: the world's are its own.)
        debug_assert!(grown_year.is_empty() && grown_last.is_empty() && flame_month.is_empty() && burnt_months.is_empty() && burnt_age.is_empty(), "a chunk counted a year");
        debug_assert!(load_month.is_empty() && plumes.is_empty(), "a chunk counted the month's load");
        self.flamed.extend(flamed);
        self.flame.iter_mut().zip(flame).for_each(|(a, b)| *a += b);
        if self.flame_air.len() < flame_air.len() {
            self.flame_air.resize(flame_air.len(), 0.0);
        }
        self.flame_air.iter_mut().zip(flame_air).for_each(|(a, b)| *a += b);
        self.edges.iter_mut().zip(edges).for_each(|(a, b)| *a += b);
        self.edge_liquid.iter_mut().zip(edge_liquid).for_each(|(a, b)| *a += b);
        self.load.iter_mut().zip(load).for_each(|(a, b)| *a += b);
    }
    /// What a booking set from just below zero to zero is added (`Counters::rounded`): mol, and J.
    pub(crate) fn add_rounded(&mut self, [mol, heat]: [f64; 2]) {
        (self.rounded[0], self.rounded[1]) = (self.rounded[0] + mol, self.rounded[1] + heat);
    }
}

/// The counts the shell takes once a year (`World::take_counters`).
#[derive(Debug, Default)]
pub struct Taken {
    pub cover_lost: [f64; crate::life::DEATHS],
    pub cover_budget: [f64; crate::cover::BUDGET.len()],
    pub seeds: [f64; crate::cover::SEED_FLOWS.len()],
    pub flame: [f64; crate::flame::FLOWS.len()],
    pub flame_air: Vec<f64>,
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
    /// What ran through each ground cell's main channel in the month's runoff pass, its own shed among it, mol/s (the
    /// month's `liquid_flux`, `flow::runoff`): what a taker can drink of the liquid running over the cell
    /// (`contract::liquid`). Not saved: the runoff's pass writes it each month before life reads it
    #[serde(skip)]
    pub liquid_month: Vec<f32>,
    /// What rises from each ground cell into the air less what condenses on it, mol/(m² s), and what leaves each land
    /// cell for the cells that are not land (`is_land`), over its surface and under it (`flow::drain`), mol/s: moving averages over about a
    /// year, for the tape
    pub evap: Vec<f32>,
    pub off_land: Vec<f32>,
    pub counts: Counters,
    /// Each air node's low wind at the air's last step, m/s [east, south] (`flow::air`, `rise::low_wind`), which a
    /// flame's run follows (`flame::run`)
    pub low_wind: Vec<[f32; 2]>,
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
    /// The soil of each cell (`soil::Soil`): its texture, the terrain's samples', and its pores, found when the world is
    /// built (`soil::of_world`)
    pub soil: Vec<crate::soil::Soil>,
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
    /// The pores of the loose ground of ground node `i` (`soil::Pores`).
    #[inline]
    pub fn pores(&self, i: usize) -> &crate::soil::Pores {
        &self.soil[i].pores
    }
    /// Hand the shell the raw events since it last asked, and start the list again.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// Hand the shell the yearly counts (`Counters::cover_lost`, `cover_budget`, `seeds`, `flame`, `flame_air`), and
    /// start them again.
    pub fn take_counters(&mut self) -> Taken {
        let c = &mut self.counts;
        Taken { cover_lost: std::mem::take(&mut c.cover_lost), cover_budget: std::mem::take(&mut c.cover_budget), seeds: std::mem::take(&mut c.seeds), flame: std::mem::take(&mut c.flame), flame_air: std::mem::take(&mut c.flame_air) }
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
    /// Whether ground node `i` is land against the open low liquid: its mean solid surface (`surface_z`) above that
    /// liquid's level (`STAND_M`), as Flow asks it (`flow::on_land`). What drains from it reaches the open low liquid,
    /// and the tape counts its area as land. The land out of the liquid standing on a cell, which life lives and walks on, is
    /// `contract::land`.
    pub fn is_land(&self, i: usize) -> bool {
        self.surface_z(i) > crate::space::wear::STAND_M
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

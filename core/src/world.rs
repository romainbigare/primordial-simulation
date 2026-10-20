//! The world's state: the store's nodes (ground, air, bed, deep stores, bodies' nodes, the nodes beyond its boundary
//! that book what crossed it, the potential), the bodies and parts, and the counters the shell reads.

use crate::{bond, chem::Chemistry, event::Event, flow, params::Config, space::{square::Square, Mesh}, store::{Class, Node, Store}};
use serde::{Deserialize, Serialize};

/// Counts and slow averages for the shell; the laws never read them. The fields not saved count since the shell last read them.
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
    /// Population nodes split in two since the start (`nodes::split`)
    pub splits: f64,
    /// Body updates since the start
    pub updates: u64,
    /// Heat a ground node could not give (below `T_FLOOR`), J: whoever asked got that much less, so no energy is lost
    pub clamped_heat: f64,
    /// What bodies did with parts
    #[serde(skip)]
    pub parts: bond::Tally,
    /// What `store::apply` set from just below zero (rounding) to zero: mol, and J
    #[serde(skip)]
    pub rounded: [f64; 2],
}

impl Counters {
    /// The counts that add up over the blocks of a wave (`waves`), taken out: they start again from zero.
    pub(crate) fn take_sums(&mut self) -> Counters {
        let mut s = Counters::default();
        s.add_sums(self);
        let (traffic, crossings) = (std::mem::take(&mut self.traffic), std::mem::take(&mut self.crossings));
        *self = Counters { traffic, crossings, ..Default::default() };
        s
    }
    /// Add the sums `s` (`take_sums`).
    pub(crate) fn add_sums(&mut self, s: &Counters) {
        let ((p, q), [r0, r1]) = ((&mut self.parts, &s.parts), &mut self.rounded);
        let sums = [
            (&mut self.decisions, s.decisions),
            (&mut self.greedy, s.greedy),
            (&mut self.splits, s.splits),
            (&mut self.clamped_heat, s.clamped_heat),
            (r0, s.rounded[0]),
            (r1, s.rounded[1]),
            (&mut p.afloat, q.afloat),
            (&mut p.formed, q.formed),
            (&mut p.to_parts, q.to_parts),
            (&mut p.from_parts, q.from_parts),
        ];
        sums.into_iter().for_each(|(x, y)| *x += y);
        self.updates += s.updates;
    }
}

/// The counts the shell takes once a year (`World::take_counters`).
#[derive(Debug, Default)]
pub struct Taken {
    pub decisions: f64,
    pub greedy: f64,
    pub parts: bond::Tally,
}

/// The whole state of a world: what `step` advances, and what a snapshot saves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub cfg: Config,
    pub chem: Chemistry,
    /// The map of squares the world is laid out on, and the ground cells along a side of the block under each air node
    pub map: Square,
    pub air_scale: usize,
    /// The shape of the world: its cells are the ground nodes, those of level `AIR` the air nodes (built from `map`, not saved)
    #[serde(skip)]
    pub mesh: Mesh,
    /// Seed of every keyed random draw (`rng::Rng`)
    pub seed: u64,
    /// World clock, seconds since the start (`clock`)
    pub time: u64,
    /// The alarms of the nodes' own processes, and when each was last brought up to date (`clock::Sleep`)
    pub sleep: crate::clock::Sleep,
    /// The amounts and heat of the store's fixed nodes, which only `store::apply` writes
    pub store: Store,
    /// Elevation of the bottom of the ground column, m
    pub base_z: f64,
    /// Liquid falling on each ground cell, mol/(m² s), a moving average over about a year, for the tape
    pub rain: Vec<f32>,
    /// Liquid moved out of each ground cell, mol/s, a moving average over about a year; it pushes parts (`bond::load`)
    pub liquid_flux: Vec<f32>,
    pub counts: Counters,
    /// Bodies, and the parts they made: the sparse graph
    pub life: crate::life::Life,
    pub parts: bond::Parts,
    /// Light-capture area of the bodies of each cell per ground area (they intercept 1 - exp(-a))
    pub canopy: Vec<f32>,
    /// Light intercepted in each cell per m² of light-capture area since the start (its bodies' mean), J/m²
    pub light: Vec<f64>,
    /// Rain fallen on each cell since the start, mol/m² (its bodies catch what fell since their last update)
    pub rained: Vec<f64>,
    /// What the links of the cell level remember (`store::Links`)
    pub links: crate::store::Links,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Flow's working memory: its state (saved) and its caches (rebuilt when missing, `flow::Scratch`)
    pub scratch: flow::Scratch,
    /// What Change books at a step, kept between steps (not saved)
    #[serde(skip)]
    pub batches: crate::change::Batches,
    /// The moves of the update of a body or a part running, and its cell's nodes as they leave them (not saved)
    #[serde(skip)]
    pub moves: crate::store::Moves,
    /// What runs the bodies' day in waves, and the worlds of the threads (not saved)
    #[serde(skip)]
    pub crew: crate::waves::Crew,
}

/// The level of the world's mesh its air nodes live on.
pub const AIR: usize = 1;

impl World {
    /// The mesh of `map`, air nodes over blocks of `air_scale`² squares, its rows between the latitudes of `cfg`.
    pub fn mesh_of(map: &Square, air_scale: usize, cfg: &Config) -> Mesh {
        map.mesh(&[air_scale], cfg.lat_south, cfg.lat_north)
    }
    /// Hand the shell the raw events since it last asked, and start the list again.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// Hand the shell the yearly counts (`Counters::decisions`, `greedy`, `parts`), and start them again.
    pub fn take_counters(&mut self) -> Taken {
        let c = &mut self.counts;
        Taken { decisions: std::mem::take(&mut c.decisions), greedy: std::mem::take(&mut c.greedy), parts: std::mem::take(&mut c.parts) }
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
    /// Loose amount of substance `k` in ground node `i`, mol, as the moves of the update running leave it.
    #[inline]
    pub fn g(&self, k: usize, i: usize) -> f64 {
        self.moves.amount(Node::ground(i), k).unwrap_or_else(|| self.store.amounts(Class::Ground)[k * self.mesh.cells().n() + i])
    }
    /// Bonded amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn b(&self, k: usize, i: usize) -> f64 {
        self.store.amounts(Class::Bed)[k * self.mesh.cells().n() + i]
    }
    /// Amount of substance `k` in air node `j`, mol, as the moves of the update running leave it.
    #[inline]
    pub fn a(&self, k: usize, j: usize) -> f64 {
        self.moves.amount(Node::air(j), k).unwrap_or_else(|| self.store.amounts(Class::Air)[k * self.mesh.level(AIR).n() + j])
    }
    /// Amount of substance `k` that bodies and parts see in air node `j`, mol: what it holds (`a`) and what the bodies'
    /// nodes under it hold until the air's step takes it up, as the moves of the update running leave them.
    pub fn a_seen(&self, k: usize, j: usize) -> f64 {
        if let Some(x) = self.seen_now(k, j) {
            return x;
        }
        let (n, kids) = (self.mesh.cells().n(), self.mesh.level(AIR).children(j));
        self.a(k, j) + kids.iter().map(|&i| self.moves.amount(Node::bodies(i as usize), k).unwrap_or_else(|| self.store.amounts(Class::Bodies)[k * n + i as usize])).sum::<f64>()
    }
}

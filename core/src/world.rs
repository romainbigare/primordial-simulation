//! The world's state: the store's nodes (ground, air, bed, deep stores, bodies' nodes, the nodes beyond its boundary
//! that book what crossed it), the bodies and parts, and the counters the shell reads.

use crate::{bond, chem::Chemistry, event::Event, flow, params::Config, space::{square::Square, Mesh}, store::{Class, Node, Store}};
use serde::{Deserialize, Serialize};

/// Counts and slow averages for the shell; the laws never read them. The fields not saved count since the shell last read them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    /// Members moving into each ground cell, per year (moving average over about a year)
    pub traffic: Vec<f32>,
    /// Decisions drawn, and of them those where the action drawn had the highest score
    #[serde(skip)]
    pub decisions: f64,
    #[serde(skip)]
    pub greedy: f64,
    /// Batches of bodies' updates, and the busiest chunk's share of each one's updates summed (`survey::batch`)
    #[serde(skip)]
    pub body_batches: f64,
    #[serde(skip)]
    pub busiest_share: f64,
    /// Body updates since the start
    pub updates: u64,
    /// Heat a ground node could not give (below `T_FLOOR`), J: whoever asked got that much less, so no energy is lost
    pub clamped_heat: f64,
    /// What bodies did with parts
    #[serde(skip)]
    pub parts: bond::Tally,
    /// What `store::apply` and the booking by array (`book_by_array`, `add_rounded`) set from just below zero (rounding)
    /// to zero: mol, and J
    #[serde(skip)]
    pub rounded: [f64; 2],
    /// Cover that died, kg, by the cause a body would die of (`body::LACK` to `AGE`)
    #[serde(skip)]
    pub cover_lost: [f64; 6],
}

impl Counters {
    /// The counts `c` of a chunk are added (`World::settle`), and start again. Every field is named, so a new one
    /// cannot be left out. A chunk counts per cell in its cells (`Cell::crossings`, `Cell::pushed`), so its `traffic`
    /// stays empty (checked in debug builds).
    pub(crate) fn add(&mut self, c: &mut Counters) {
        let Counters { traffic, decisions, greedy, body_batches, busiest_share, updates, clamped_heat, parts, rounded, cover_lost } = std::mem::take(c);
        debug_assert!(traffic.is_empty(), "a chunk counted per cell");
        (self.updates, self.decisions, self.greedy) = (self.updates + updates, self.decisions + decisions, self.greedy + greedy);
        (self.body_batches, self.busiest_share, self.clamped_heat) = (self.body_batches + body_batches, self.busiest_share + busiest_share, self.clamped_heat + clamped_heat);
        self.add_rounded(rounded);
        self.cover_lost.iter_mut().zip(cover_lost).for_each(|(a, b)| *a += b);
        let p = &mut self.parts;
        (p.afloat, p.formed, p.to_parts, p.from_parts) = (p.afloat + parts.afloat, p.formed + parts.formed, p.to_parts + parts.to_parts, p.from_parts + parts.from_parts);
    }
    /// What a booking set from just below zero to zero is added (`Counters::rounded`): mol, and J.
    pub(crate) fn add_rounded(&mut self, [mol, heat]: [f64; 2]) {
        (self.rounded[0], self.rounded[1]) = (self.rounded[0] + mol, self.rounded[1] + heat);
    }
}

/// The counts the shell takes once a year (`World::take_counters`).
#[derive(Debug, Default)]
pub struct Taken {
    pub decisions: f64,
    pub greedy: f64,
    pub body_batches: f64,
    pub busiest_share: f64,
    pub parts: bond::Tally,
    pub cover_lost: [f64; 6],
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
    /// The amounts and heat of the store's fixed nodes, which only `store::apply` and `store::book_by_array` write
    pub store: Store,
    /// Elevation of the bottom of the ground column, m
    pub base_z: f64,
    /// Liquid falling on each ground cell, mol/(m² s), a moving average over about a year, for the tape
    pub rain: Vec<f32>,
    /// Liquid moved out of each ground cell, mol/s, a moving average over about a year; it pushes parts (`bond::load`)
    pub liquid_flux: Vec<f32>,
    /// What rises from each ground cell into the air less what condenses on it, mol/(m² s), and what runs over the
    /// surface of each land cell into cells that are not land (`is_land`), mol/s: moving averages over about a year,
    /// for the tape
    pub evap: Vec<f32>,
    pub off_land: Vec<f32>,
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
    /// Hand the shell the yearly counts (`Counters::decisions`, `greedy`, `body_batches`, `busiest_share`, `parts`, `cover_lost`),
    /// and start them again.
    pub fn take_counters(&mut self) -> Taken {
        let c = &mut self.counts;
        let (decisions, greedy, body_batches, busiest_share) = (std::mem::take(&mut c.decisions), std::mem::take(&mut c.greedy), std::mem::take(&mut c.body_batches), std::mem::take(&mut c.busiest_share));
        Taken { decisions, greedy, body_batches, busiest_share, parts: std::mem::take(&mut c.parts), cover_lost: std::mem::take(&mut c.cover_lost) }
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
    /// Amount of substance `k` that bodies and parts see in air node `j`, mol: what it holds (`a`) and what the bodies'
    /// nodes under it hold until the air's step takes it up.
    pub fn a_seen(&self, k: usize, j: usize) -> f64 {
        let kids = self.mesh.level(AIR).children(j);
        self.a(k, j) + kids.iter().map(|&i| self.stock(Node::bodies(i as usize))(k)).sum::<f64>()
    }
}

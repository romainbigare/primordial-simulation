//! The second stage of the flames' month (laws.md 3c): the cells whose remains carry a flame join across the edges they
//! share into regions, in the order of the cells; strikes drawn from the world's seed light some of their cells, and
//! each flame runs from cell to cell over its region for as long as a flame front lives.

use super::Bed;
use crate::rng::{Purpose, Rng};
use crate::World;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The root of cell `i` in the forest `up` (each cell's parent; a root is its own), the way to it shortened as it goes.
fn root(up: &mut [u32], mut i: u32) -> u32 {
    while up[i as usize] != i {
        let next = up[up[i as usize] as usize];
        up[i as usize] = next;
        i = next;
    }
    i
}

/// Of the cells whose remains carry a flame (`beds`, by cell; `Bed::carries`): the share of each cell's land that burns
/// this month of `month` s. Cells that share an edge (a link of some width) join into a region (union by the lower
/// cell, in the order of the cells), of A m² covered by its remains (each cell's land times the share its remains
/// cover). Each region expects `strikes` × A × the month's share of a year of strikes that light it; their count is a
/// Poisson draw, and each lights a cell drawn by its covered ground, from the world's seed by the month and the
/// region's first cell (`Purpose::Flame`), so the draws do not depend on the threads. Each flame runs over its region
/// for `front_life` (`run`), and every cell it reaches burns all of its land its remains cover. A cell that burnt keeps
/// no remains to carry a flame until its cover's leaves build them again.
pub fn shares(w: &World, beds: &[Option<Bed>], month: f64) -> Vec<f64> {
    let (cells, n) = (w.mesh.cells(), beds.len());
    let carries = |i: usize| beds[i].is_some_and(|b| b.carries());
    let mut up: Vec<u32> = (0..n as u32).collect();
    for i in (0..n).filter(|&i| carries(i)) {
        let (links, others) = cells.around(i);
        for (&l, &j) in links.iter().zip(others) {
            if (j as usize) < i && cells.width()[l as usize] > 0.0 && carries(j as usize) {
                let (a, b) = (root(&mut up, i as u32), root(&mut up, j));
                up[a.max(b) as usize] = a.min(b);
            }
        }
    }
    let covered = |i: usize| beds[i].map_or(0.0, |b| b.cover) * crate::contract::land(w, i);
    // Each region's cells in the order of the cells, by its root (its first cell).
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); n];
    for i in (0..n).filter(|&i| carries(i)) {
        members[root(&mut up, i as u32) as usize].push(i as u32);
    }
    let ph = &w.cfg.phys;
    let strikes = ph.strikes * month / w.cfg.calendar().year as f64;
    let mut share = vec![0.0; n];
    for (r, region) in members.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
        let area: f64 = region.iter().map(|&i| covered(i as usize)).sum();
        let mut draw = Rng::new(w.seed, w.month(), r as u64, Purpose::Flame);
        for _ in 0..draw.poisson(strikes * area) {
            let mut at = draw.f64() * area;
            let lit = region.iter().copied().find(|&i| {
                at -= covered(i as usize);
                at < 0.0
            });
            for i in run(w, beds, lit.unwrap_or(region[region.len() - 1]) as usize) {
                share[i] = beds[i].map_or(0.0, |b| b.cover);
            }
        }
    }
    share
}

/// A place a flame reaches, and when, s (`run`); the heap pops the earliest.
#[derive(PartialEq)]
struct Reached(f64, usize);

impl Eq for Reached {}
impl Ord for Reached {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}
impl PartialOrd for Reached {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The cells a flame lit in cell `from` reaches within `front_life`: it crosses half the way between two cells' centres
/// at the speed of each one's bed (`reach`) along the way between them, the push of its wind at mid-flame added
/// (`Bed::waf`, `super::downwind`), through
/// cells whose remains carry a flame, the earliest first (Dijkstra's way over the times). Returns them, the lit one
/// first.
fn run(w: &World, beds: &[Option<Bed>], from: usize) -> Vec<usize> {
    let (cells, ph) = (w.mesh.cells(), &w.cfg.phys);
    let speed = |i: usize, way: [f64; 2]| {
        let wind = w.low_wind.get(w.mesh.up(i, 0, crate::AIR)).map_or([0.0; 2], |v| v.map(|x| x as f64));
        beds[i].filter(|b| b.carries()).map(|b| super::reach(ph, &b).speed * super::downwind(ph, &b, wind.map(|v| v * b.waf), way))
    };
    let mut when: Vec<(usize, f64)> = vec![(from, 0.0)];
    let mut heap = BinaryHeap::from([Reached(0.0, from)]);
    let mut out = Vec::new();
    while let Some(Reached(t, i)) = heap.pop() {
        if when.iter().any(|&(c, s)| c == i && s < t) || out.contains(&i) {
            continue;
        }
        out.push(i);
        let (links, others) = cells.around(i);
        for (&l, &j) in links.iter().zip(others) {
            let j = j as usize;
            let [x, y] = cells.offset(i, j);
            let half = x.hypot(y) / 2.0;
            let way = [x / (2.0 * half).max(f64::MIN_POSITIVE), y / (2.0 * half).max(f64::MIN_POSITIVE)];
            let Some(here) = speed(i, way) else { continue };
            let Some(there) = speed(j, way).filter(|_| cells.width()[l as usize] > 0.0) else { continue };
            let t = t + half / here.max(f64::MIN_POSITIVE) + half / there.max(f64::MIN_POSITIVE);
            if t <= ph.front_life && !when.iter().any(|&(c, s)| c == j && s <= t) {
                when.push((j, t));
                heap.push(Reached(t, j));
            }
        }
    }
    out
}

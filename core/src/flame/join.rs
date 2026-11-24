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

/// What the flames of a span do to each cell (`shares`): the share of its land that burns, and when the first flame
/// reaches it, s after the strike that lit that flame (infinite where none does).
pub struct Lit {
    pub share: Vec<f64>,
    pub at: Vec<f64>,
}

/// Of the cells whose remains carry a flame on some patch of their cover (`beds`, by cell and patch; `Bed::carries`):
/// what the flames of a span of `span` s do to each (`Lit`). Cells that share an edge (a link of some width)
/// join into a region (union by the lower cell, in the order of the cells), of A m² covered by its remains (each cell's
/// land times the share its remains cover on the patches that carry a flame, `covered`). Each region expects `strikes`
/// × A × the span's share of a year of strikes that light it; their count is a Poisson draw, and each lights a cell
/// drawn by its covered ground, from the world's seed by the month and the region's first cell (`Purpose::Flame`), so
/// the draws do not depend on the threads. Each flame runs over its region for `front_life` (`run`), and every cell it
/// reaches burns all of the ground its remains cover on each patch that carries a flame. Each strike of an edit
/// (`World::strikes`) lights its cell too, after them, where its remains carry a flame; and each cell of `heated`
/// burns where the heat edits added keeps a flame (`Bed::kept`), its flame running on where its beds carry one. A
/// patch that burnt keeps no remains to carry a flame until its cover's leaves build them again.
pub fn shares(w: &World, beds: &[Vec<Option<Bed>>], span: f64, heated: &[usize]) -> Lit {
    let (cells, n) = (w.mesh.cells(), beds.len());
    let carries = |i: usize| beds[i].iter().flatten().any(|b| b.carries());
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
    let area_of = |i: usize| covered(&beds[i]) * crate::contract::land(w, i);
    // Each region's cells in the order of the cells, by its root (its first cell).
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); n];
    for i in (0..n).filter(|&i| carries(i)) {
        members[root(&mut up, i as u32) as usize].push(i as u32);
    }
    let ph = &w.cfg.phys;
    let strikes = ph.strikes * span / w.cfg.calendar().year as f64;
    let mut lit = Lit { share: vec![0.0; n], at: vec![f64::INFINITY; n] };
    let reach = |from: usize, lit: &mut Lit| {
        for (i, t) in run(w, beds, from) {
            (lit.share[i], lit.at[i]) = (covered(&beds[i]), lit.at[i].min(t));
        }
    };
    for (r, region) in members.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
        let area: f64 = region.iter().map(|&i| area_of(i as usize)).sum();
        let mut draw = Rng::new(w.seed, w.month(), r as u64, Purpose::Flame);
        for _ in 0..draw.poisson(strikes * area) {
            let mut at = draw.f64() * area;
            let at_cell = region.iter().copied().find(|&i| {
                at -= area_of(i as usize);
                at < 0.0
            });
            reach(at_cell.unwrap_or(region[region.len() - 1]) as usize, &mut lit);
        }
    }
    for &struck in w.strikes.iter().filter(|&&i| carries(i as usize)) {
        reach(struck as usize, &mut lit);
    }
    for &i in heated {
        (lit.share[i], lit.at[i]) = (covered(&beds[i]), 0.0);
        if carries(i) {
            reach(i, &mut lit);
        }
    }
    lit
}

/// The share of a cell's ground the remains cover on the patches of its cover whose beds `beds` carry a flame or whose
/// flame added heat keeps (`Bed::kept`).
fn covered(beds: &[Option<Bed>]) -> f64 {
    beds.iter().flatten().filter(|b| b.carries() || b.kept).map(|b| b.cover * b.share).sum()
}

/// The bed a flame crosses a cell by, of its patches' `beds`: of those that carry a flame, the one whose remains cover
/// the most of the cell's ground.
fn lead(beds: &[Option<Bed>]) -> Option<&Bed> {
    beds.iter().flatten().filter(|b| b.carries()).max_by(|a, b| (a.cover * a.share).total_cmp(&(b.cover * b.share)))
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
/// at the speed of each one's bed (`lead`, `reach`) along the way between them, the push of its wind at mid-flame added
/// (`Bed::waf`, `super::downwind`), through
/// cells whose remains carry a flame, the earliest first (Dijkstra's way over the times). Returns them with when the
/// flame reaches each, s, the lit one first.
fn run(w: &World, beds: &[Vec<Option<Bed>>], from: usize) -> Vec<(usize, f64)> {
    let (cells, ph) = (w.mesh.cells(), &w.cfg.phys);
    let speed = |i: usize, way: [f64; 2]| {
        let wind = w.low_wind.get(w.mesh.up(i, 0, crate::AIR)).map_or([0.0; 2], |v| v.map(|x| x as f64));
        lead(&beds[i]).map(|b| super::reach(ph, b).speed * super::downwind(ph, b, wind.map(|v| v * b.waf), way))
    };
    let mut when: Vec<(usize, f64)> = vec![(from, 0.0)];
    let mut heap = BinaryHeap::from([Reached(0.0, from)]);
    let mut out = Vec::new();
    while let Some(Reached(t, i)) = heap.pop() {
        if when.iter().any(|&(c, s)| c == i && s < t) || out.iter().any(|&(c, _)| c == i) {
            continue;
        }
        out.push((i, t));
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

//! The second stage of the flames' month (laws.md 3c): the cells whose remains carry a flame join across the edges they
//! share into regions, in the order of the cells, and each region's strikes give each of its cells the share that
//! burns.

use super::Bed;
use crate::World;

/// The root of cell `i` in the forest `up` (each cell's parent; a root is its own), the way to it shortened as it goes.
fn root(up: &mut [u32], mut i: u32) -> u32 {
    while up[i as usize] != i {
        let next = up[up[i as usize] as usize];
        up[i as usize] = next;
        i = next;
    }
    i
}

/// Of the cells whose remains carry a flame (`beds`, by cell; `Bed::carries`): the share of each cell's land that
/// burns this month of `month` s. A flame lit runs for `front_life` at its bed's speed (`reach`), so it covers a disc of
/// radius speed × life, no larger than the remains joined to it: cells that share an edge (a link of some width) join
/// into a region (union by the lower cell, in the order of the cells), of A m² covered by its remains (each cell's land
/// times the share its remains cover). A point of a cell is reached by the flames lit within that disc, so it expects
/// λ = `strikes` × the disc's area × the month's share of a year of them, and with no draw the cell burns the share
/// (1 − e^−λ) of its land where its remains lie, (1 − e^−λ) × its cover.
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
    let mut area = vec![0.0; n];
    for i in (0..n).filter(|&i| carries(i)) {
        area[root(&mut up, i as u32) as usize] += covered(i);
    }
    let ph = &w.cfg.phys;
    let strikes = ph.strikes * month / w.cfg.calendar().year as f64;
    let mut share = vec![0.0; n];
    for i in (0..n).filter(|&i| carries(i)) {
        let b = beds[i].expect("a bed that carries");
        let radius = super::reach(ph, &b).speed * ph.front_life;
        let disc = (std::f64::consts::PI * radius * radius).min(area[root(&mut up, i as u32) as usize]);
        share[i] = (1.0 - (-strikes * disc).exp()) * b.cover;
    }
    share
}

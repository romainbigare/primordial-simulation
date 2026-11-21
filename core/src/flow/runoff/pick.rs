//! What the runoff picks up and the bed it wears (laws.md 1.3), each month before the load goes down its ways
//! (`runoff::load`).

use super::super::{bare_share, land::{lift, solid}, on_land};
use super::{carriage::sample_area, Bodies, Scratch};
use crate::{space::network::NONE, store::{Class, Move, Node}, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;

/// The work of the month's runoff, J: of each pair of `Carriage`, on the samples above its key sample and on the key
/// sample's own reach, and of each cell; and of each sample that holds relief, its cell and work.
pub(super) struct Work {
    pub(super) pairs: Vec<[f64; 2]>,
    pub(super) cells: Vec<f64>,
    on_relief: Vec<(u32, u32, f64)>,
}

/// What each land cell picks up (laws.md 1.3): the head the liquid loses down the samples' ways in it does work, g
/// times what runs through each of its samples in no pond, kg, times the fall to the sample it runs to; it picks up
/// loose solids up to `carry` m³ per J times its loose ground's erodibility (`Texture::erodibility`) and its bare share
/// (`bare_share`), at most all that lies loose, with their heat at its temperature; the work its load leaves free (the
/// cover effect, Sklar and Dietrich 2004) breaks bonded ground into loose (`wear`). Returns the work (`Work`), what each
/// cell picked up (rows of `sol`, mol, then the heat, J, into `picked`), the moves of the bed into loose ground, and
/// what the pick-up took from the samples' relief (`taken`).
pub(super) fn pick_up(w: &World, s: &mut Scratch, b: &Bodies, (flux, sol): (&[[f64; 2]], &[usize]), picked: &mut Vec<f64>) -> (Work, Vec<Move>, Vec<(u32, f64)>) {
    let (samples, n, width, c) = (w.terrain_samples(), w.mesh.cells().n(), sol.len() + 1, &b.carriage);
    let mut work = Work { pairs: vec![[0.0; 2]; c.pairs.len()], cells: vec![0.0; n], on_relief: Vec::new() };
    for &k in b.land.iter().filter(|&&k| flux[k as usize][1] > 0.0) {
        let (k, f) = (k as usize, flux[k as usize]);
        let (i, j) = (b.cell_of[k] as usize, samples.to[k]);
        if j != NONE && on_land(s, i) {
            let done = w.cfg.gravity * f[1] * (samples.h[k] - samples.h[j as usize]).max(0.0);
            work.cells[i] += done;
            debug_assert!(c.pair_of[k] != NONE, "a land sample whose way reaches no key sample");
            work.pairs[c.pair_of[k] as usize][usize::from(c.place[k] != NONE)] += done;
            if done > 0.0 && w.relief.at(k) > 0.0 {
                work.on_relief.push((k as u32, i as u32, done));
            }
        }
    }
    picked.clear();
    picked.resize(n * width, 0.0);
    let mut broke = Vec::new();
    let mut volume = vec![0.0; n];
    for i in (0..n).filter(|&i| work.cells[i] > 0.0) {
        let cap = w.cfg.carry * work.cells[i] * bare_share(w, i) * w.soil[i].texture.erodibility();
        let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let f = if avail > 0.0 { (cap / avail).min(1.0) } else { 0.0 };
        picked[i * width..(i + 1) * width].copy_from_slice(&loose_share(w, s, (sol, |_| true), i, f));
        volume[i] = f * avail;
        let worn = work.cells[i] * (1.0 - f * avail / cap).max(0.0);
        if worn > 0.0 {
            wear(w, s, i, worn, &mut broke);
        }
    }
    let taken = taken(w, &work, &volume);
    (work, broke, taken)
}

/// The share `share` of the loose solids `sol` of ground node `i` that `take` takes: mol of each (none of the others),
/// then their heat at the temperature of its land (its land part's in a cell of parts), J.
pub(in crate::flow) fn loose_share(w: &World, s: &Scratch, (sol, take): (&[usize], impl Fn(usize) -> bool), i: usize, share: f64) -> Vec<f64> {
    let t = land_t(s, i);
    let mut row: Vec<f64> = sol.iter().map(|&k| if take(k) { solid(w, s, k, i) * share } else { 0.0 }).collect();
    let heat = sol.iter().zip(&row).fold(0.0, |h, (&k, m)| h + m * s.sp[k].cp * t);
    row.push(heat);
    row
}

/// The temperature of the land of ground node `i`, K: its land part's in a cell of parts.
pub(in crate::flow) fn land_t(s: &Scratch, i: usize) -> f64 {
    s.mixed(i).filter(|_| super::super::parts::has(s, i)).map_or(s.state.tg[i], |m| m.piece[super::super::parts::LAND].tg)
}

/// What the pick-up of each cell, `volume` m³, takes from its samples' relief (`Relief`): each sample that does work
/// (`Work::on_relief`) gives its share of the cell's pick-up by its work, at most the relief it holds, m; the rest
/// comes from the cell's loose ground at large.
fn taken(w: &World, work: &Work, volume: &[f64]) -> Vec<(u32, f64)> {
    let samples = w.terrain_samples();
    work.on_relief.iter().map(|&(k, i, done)| {
        let a = sample_area(w, &samples, k as usize);
        (k, (volume[i as usize] * done / work.cells[i as usize] / a).min(w.relief.at(k as usize)))
    }).collect()
}

/// Break bonded ground of node `i` into loose by `bedwork` over bond strength (`wear`), each substance by its share of
/// the bed's volume, at most all of it: moves from `Class::Bed` gathered in `broke`. Its heat was already the node's.
fn wear(w: &World, s: &mut Scratch, i: usize, bedwork: f64, broke: &mut Vec<Move>) {
    let n = w.mesh.cells().n();
    for bi in 0..s.state.bedk.len() {
        let (k, bv) = (s.state.bedk[bi], s.state.bed[i].vol);
        let b = w.store.amounts(Class::Bed)[k * n + i];
        if b > 0.0 && bv > 0.0 {
            let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(STRENGTH_MIN);
            let go = Move { from: Node::new(Class::Bed, i), to: Node::ground(i), k: k as u16, mol: (v / s.sp[k].v[0]).min(b), heat: 0.0 };
            let area = w.mesh.cells().area()[i];
            let (m, dz) = (go.mol, go.mol * s.sp[k].v[0] / area);
            broke.push(go);
            s.state.bed[i].vol -= m * s.sp[k].v[0];
            s.add_loose(k, false);
            s.note(k, i, m, area);
            lift(s, i, [-dz, -dz]);
        }
    }
}

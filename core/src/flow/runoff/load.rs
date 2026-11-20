//! The load the running liquid carries (laws.md 1.3). Each month each land cell picks up loose solids from the work its
//! runoff does there (`pick_up`), and the load goes down the samples' ways. It is followed only at the **key samples**
//! (`Carriage`): each land cell's main channel and the samples where the ways end (in a body, or on land where none
//! runs on); between two key samples it only passes. Each key sample holds a **pool** this month: what came in from the
//! key samples above it and what was picked up on the ways to it (mol of each solid substance and its heat). One serial
//! pass over the key samples, each before the one its way leads to, moves each pool on; where a way ends, all of it
//! settles in the cell that holds that sample.
//!
//! The books: a move goes from the cell it leaves to the cell of the key sample it reaches; what settles stays in the
//! node it reached. The moves between two cells are summed in the order each pair first comes and booked once.

use super::{bare_share, solid, wear, Bodies, Crossings, Scratch};
use crate::{space::network::NONE, store::Move, units::G_PER_KG, World};
use std::collections::HashMap;

/// The key samples of the terrain's ways (`Carriage::of`): the key samples, those whose liquid runs on in the order of
/// `Bodies::down`, then the ends; of each sample its place among them (`NONE` for none); of each its next key sample
/// down its way (`NONE` for an end); of each land sample its pair (`NONE` for none); the pairs, each a cell with the
/// first key sample below one of its samples, in the order of the land samples; of each cell its main channel's key
/// sample (`NONE` for none).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Carriage {
    keys: Vec<u32>,
    place: Vec<u32>,
    next: Vec<u32>,
    pair_of: Vec<u32>,
    pairs: Vec<(u32, u32)>,
    main: Vec<u32>,
}

impl Carriage {
    /// The carriage of the bodies `b` of world `w` (`Carriage`): a cell's main channel is its land sample the most
    /// drains through (`Samples::up`); a way ends where it enters a body, or at a land sample whose liquid runs to none.
    pub(in crate::flow) fn of(w: &World, b: &Bodies) -> Carriage {
        let s = w.terrain_samples();
        let mut main = vec![NONE; b.of.len()];
        for &k in &b.land {
            let m = &mut main[b.cell_of[k as usize] as usize];
            if *m == NONE || s.up[k as usize] > s.up[*m as usize] {
                *m = k;
            }
        }
        let mut c = Carriage { place: vec![NONE; s.h.len()], ..Carriage::default() };
        let key = |c: &mut Carriage, k: u32| {
            if c.place[k as usize] == NONE {
                c.place[k as usize] = c.keys.len() as u32;
                c.keys.push(k);
            }
        };
        b.down.iter().filter(|&&k| main[b.cell_of[k as usize] as usize] == k).for_each(|&k| key(&mut c, k));
        for &k in &b.land {
            let j = s.to[k as usize];
            if j == NONE {
                key(&mut c, k);
            } else if b.at[j as usize] != NONE {
                key(&mut c, j);
            }
        }
        // Of each sample, the first key sample at it or below it: found from the ends up.
        let mut first = c.place.clone();
        for &k in b.down.iter().rev() {
            if first[k as usize] == NONE {
                first[k as usize] = first[s.to[k as usize] as usize];
            }
        }
        let below = |k: u32| if s.to[k as usize] == NONE { c.place[k as usize] } else { first[s.to[k as usize] as usize] };
        let next = c.keys.iter().map(|&k| if b.at[k as usize] != NONE || s.to[k as usize] == NONE { NONE } else { below(k) }).collect();
        let mut index: HashMap<(u32, u32), u32> = HashMap::new();
        let (mut pairs, mut pair_of) = (Vec::new(), vec![NONE; s.h.len()]);
        for &k in &b.land {
            let key = below(k);
            if key != NONE {
                let pair = (b.cell_of[k as usize], key);
                pair_of[k as usize] = *index.entry(pair).or_insert_with(|| {
                    pairs.push(pair);
                    (pairs.len() - 1) as u32
                });
            }
        }
        c.main = main.iter().map(|&k| if k == NONE { NONE } else { c.place[k as usize] }).collect();
        (c.next, c.pair_of, c.pairs) = (next, pair_of, pairs);
        c
    }
}

/// What the load did in a month: the moves of the bed into loose ground (`wear`); of each cell what its main channel
/// carried, g per m³ of its liquid, and the depth that settled on it, m; the plumes: each sample where a way entered a
/// body, with the load that came in there, kg/s.
pub(super) struct Carried {
    pub(super) broke: Vec<Move>,
    pub(super) loads: Vec<[f32; 2]>,
    pub(super) plumes: Vec<(u32, f32)>,
}

/// The moves of the month between pairs of cells (mol of each solid substance, then the heat), summed in the order each
/// pair first comes.
#[derive(Default)]
struct Moves {
    index: HashMap<(u32, u32), usize>,
    list: Vec<((u32, u32), Vec<f64>)>,
}

impl Moves {
    /// `row` moved from cell `from` to cell `to` (no move where they are one).
    fn add(&mut self, (from, to): (u32, u32), row: &[f64]) {
        if from == to {
            return;
        }
        let list = &mut self.list;
        let at = *self.index.entry((from, to)).or_insert_with(|| {
            list.push(((from, to), vec![0.0; row.len()]));
            list.len() - 1
        });
        list[at].1.iter_mut().zip(row).for_each(|(a, b)| *a += b);
    }
}

/// One month of the load (the module's doc) over the bodies `b`, what ran through each sample `flux` (mol and kg), the
/// solid substances `sol`, in a month of `dt` s: each cell's pick-up, its share to each of its pairs, the serial pass
/// over the key samples, the ends. Notes the crossings in `x`. Returns `Carried`.
pub(super) fn carry(w: &World, s: &mut Scratch, b: &Bodies, (flux, sol): (&[[f64; 2]], &[usize]), x: &mut Crossings, dt: f64) -> Carried {
    let (c, n, width) = (&b.carriage, w.mesh.cells().n(), sol.len() + 1);
    let (work, picked, broke) = pick_up(w, s, b, flux, sol);
    let sol_v: Vec<f64> = sol.iter().map(|&k| s.sp[k].v[0]).collect();
    let mm: Vec<f64> = sol.iter().map(|&k| s.sp[k].mm).collect();
    let volume = |row: &[f64]| row.iter().zip(&sol_v).map(|(m, v)| m * v).sum::<f64>();
    let kg = |row: &[f64]| row.iter().zip(&mm).map(|(m, k)| m * k).sum::<f64>();
    // The share of each cell's pick-up each pair takes: its samples' share of the cell's area (`Bodies::share`); what
    // the cell's samples in a pond take stays on its ground.
    let mut area = vec![0.0; c.pairs.len()];
    b.land.iter().filter(|&&k| c.pair_of[k as usize] != NONE).for_each(|&k| area[c.pair_of[k as usize] as usize] += b.share[k as usize]);
    let (mut pools, mut moves) = (vec![0.0; c.keys.len() * width], Moves::default());
    let (mut settled, mut sent) = (vec![0.0; n], vec![0.0; n]);
    for (p, &(i, key)) in c.pairs.iter().enumerate() {
        let (i, q) = (i as usize, key as usize);
        if work[i] > 0.0 && area[p] > 0.0 {
            let row: Vec<f64> = picked[i * width..(i + 1) * width].iter().map(|m| m * area[p]).collect();
            pools[q * width..(q + 1) * width].iter_mut().zip(&row).for_each(|(a, b)| *a += b);
            moves.add((i as u32, b.cell_of[c.keys[q] as usize]), &row);
            sent[i] += area[p];
        }
    }
    for i in (0..n).filter(|&i| work[i] > 0.0) {
        settled[i] += volume(&picked[i * width..(i + 1) * width - 1]) * (1.0 - sent[i]).max(0.0);
    }
    let mut loads = vec![[0.0f32; 2]; n];
    let mut plumes = Vec::new();
    // The serial pass: each key sample's pool goes on to the next, or settles where its way ends.
    for (q, &k) in c.keys.iter().enumerate() {
        let i = b.cell_of[k as usize] as usize;
        let row = pools[q * width..(q + 1) * width].to_vec();
        let mass = kg(&row[..width - 1]);
        if c.main[i] == q as u32 {
            loads[i][0] = main_load(w, i, mass, flux[k as usize][0]);
        }
        let next = c.next[q] as usize;
        if c.next[q] == NONE {
            settled[i] += volume(&row[..width - 1]);
            if b.at[k as usize] != NONE && mass > 0.0 {
                plumes.push((k, (mass / dt) as f32));
            }
        } else {
            pools[next * width..(next + 1) * width].iter_mut().zip(&row).for_each(|(a, b)| *a += b);
            moves.add((i as u32, b.cell_of[c.keys[next] as usize]), &row);
        }
    }
    for ((from, to), row) in moves.list {
        x.push(from, to, row[..width - 1].iter().copied().enumerate(), row[width - 1]);
    }
    let area = w.mesh.cells().area();
    loads.iter_mut().zip(&settled).zip(area).for_each(|((l, v), a)| l[1] = (v / a) as f32);
    Carried { broke, loads, plumes }
}

/// The load in cell `i`'s main channel, g per m³ of its liquid, from the load that passes there, `kg`, and the liquid,
/// `mol`, in the month.
fn main_load(w: &World, i: usize, kg: f64, mol: f64) -> f32 {
    let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
    if per_m3 > 0.0 && mol > 0.0 { (G_PER_KG * kg / (mol / per_m3)) as f32 } else { 0.0 }
}

/// What each land cell picks up (laws.md 1.3): the head the liquid loses down the samples' ways in it does work, g
/// times what runs through each of its samples in no pond, kg, times the fall to the sample it runs to; it picks up
/// loose solids up to `carry` m³ per J times its loose ground's erodibility (`Texture::erodibility`) and its bare share
/// (`bare_share`), at most all that lies loose, with their heat at its temperature; the work its load leaves free (the
/// cover effect, Sklar and Dietrich 2004) breaks bonded ground into loose (`wear`). Returns the work of each cell, J,
/// what each picked up (rows of `sol`, mol, then the heat, J), and the moves of the bed into loose ground.
fn pick_up(w: &World, s: &mut Scratch, b: &Bodies, flux: &[[f64; 2]], sol: &[usize]) -> (Vec<f64>, Vec<f64>, Vec<Move>) {
    let (samples, n, width) = (w.terrain_samples(), w.mesh.cells().n(), sol.len() + 1);
    let mut work = vec![0.0; n];
    for &k in b.land.iter().filter(|&&k| flux[k as usize][1] > 0.0) {
        let (k, f) = (k as usize, flux[k as usize]);
        let (i, j) = (b.cell_of[k] as usize, samples.to[k]);
        if j != NONE && s.z[i] > 0.0 {
            work[i] += w.cfg.gravity * f[1] * (samples.h[k] - samples.h[j as usize]).max(0.0);
        }
    }
    let (mut picked, mut broke) = (vec![0.0; n * width], Vec::new());
    for i in (0..n).filter(|&i| work[i] > 0.0) {
        let cap = w.cfg.carry * work[i] * bare_share(w, i) * w.soil[i].texture.erodibility();
        let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let f = if avail > 0.0 { (cap / avail).min(1.0) } else { 0.0 };
        for (r, &k) in sol.iter().enumerate() {
            picked[i * width + r] = solid(w, s, k, i) * f;
            picked[(i + 1) * width - 1] += solid(w, s, k, i) * f * s.sp[k].cp * s.state.tg[i];
        }
        let worn = work[i] * (1.0 - f * avail / cap).max(0.0);
        if worn > 0.0 {
            wear(w, s, i, worn, &mut broke);
        }
    }
    (work, picked, broke)
}

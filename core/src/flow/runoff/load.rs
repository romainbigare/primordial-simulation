//! The load the running liquid carries (laws.md 1.3). Each month each land cell picks up loose solids from the work its
//! runoff does there (`pick_up`), and the load goes down the samples' ways, in three particle classes (fine, middle,
//! coarse). It is followed only at the **key samples** (`Carriage`): those where some of a class settles, each land
//! cell's main channel, and the samples where the ways end (in a body, or on land where none runs on); between two key
//! samples it only passes. Each key sample holds a **pool** this month: what came in from the key samples above it and
//! what was picked up on the ways to it (mol of each solid substance and its heat). One serial pass over the key
//! samples, each before the one its way leads to, settles its share of each class (`soil::passes`) and moves the rest
//! on; where a way ends, all of it settles in the cell that holds that sample.
//!
//! The books: a move goes from the cell it leaves to the cell of the key sample it reaches; what settles stays in the
//! node it reached. The moves between two cells are summed in the order each pair first comes and booked once. **Known
//! limit:** in a pool the substances and the particle classes are independent: what leaves takes the pool's mixture of
//! substances by the share of its volume that leaves.

use super::carriage::{Shares, CLASS_COUNT};
use super::pick::pick_up;
use super::super::land::solid;
use super::{Bodies, Crossings, Scratch};
use crate::{space::network::NONE, store::Move, units::G_PER_KG, World};
use std::collections::HashMap;

/// What the load did in a month: the moves of the bed into loose ground (`wear`); of each cell what its main channel
/// carried, g per m³ of its liquid, and the depth that settled on it, m; the plumes: each sample where a way entered a
/// body, with the load that came in there, kg/s; the volumes of the month (`Volumes`); and of each cell the volume of
/// each class that settled on it, m³ (`mix`).
pub(super) struct Carried {
    pub(super) broke: Vec<Move>,
    pub(super) loads: Vec<[f32; 2]>,
    pub(super) plumes: Vec<(u32, f32)>,
    pub(super) volumes: Volumes,
    pub(super) settled: Vec<[f64; CLASS_COUNT]>,
}

/// The volumes of the load in a month, m³: picked up, carried into the open low ground, carried into the other bodies,
/// settled on land, and of the bed worn into loose ground (`Counters::load`).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Volumes {
    picked: f64,
    open: f64,
    bodies: f64,
    land: f64,
    worn: f64,
}

impl Volumes {
    /// In the order of `Counters::load`.
    pub(super) fn counts(&self) -> [f64; 5] {
        [self.picked, self.open, self.bodies, self.land, self.worn]
    }
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

/// The pools of the key samples this month: of each its row (mol of each solid substance, then the heat, J; `width`
/// numbers a key), and the volume of each class that came in from above and that was picked up on the ways to it, m³.
struct Pools {
    rows: Vec<f64>,
    came: Vec<[f64; CLASS_COUNT]>,
    picked: Vec<[f64; CLASS_COUNT]>,
    width: usize,
}

impl Pools {
    /// Row `row` into key `q`'s pool.
    fn add(&mut self, q: usize, row: &[f64]) {
        self.rows[q * self.width..(q + 1) * self.width].iter_mut().zip(row).for_each(|(a, b)| *a += b);
    }
}

/// One month of the load (the module's doc) over the bodies `b`, what ran through each sample `flux` (mol and kg), the
/// solid substances `sol`, in a month of `dt` s: each cell's pick-up, its share to each of its pairs by their work,
/// the serial pass over the key samples, the ends. Notes the crossings in `x`. Returns `Carried`.
pub(super) fn carry(w: &World, s: &mut Scratch, b: &Bodies, (flux, sol): (&[[f64; 2]], &[usize]), x: &mut Crossings, dt: f64) -> Carried {
    let (c, n, width) = (&b.carriage, w.mesh.cells().n(), sol.len() + 1);
    let mut picked = std::mem::take(&mut s.spare.picked);
    let (work, broke) = pick_up(w, s, b, (flux, sol), &mut picked);
    let sol_v: Vec<f64> = sol.iter().map(|&k| s.sp[k].v[0]).collect();
    let mut volumes = Volumes { worn: broke.iter().map(|m| m.mol * s.sp[m.k as usize].v[0]).sum(), ..Volumes::default() };
    let mm: Vec<f64> = sol.iter().map(|&k| s.sp[k].mm).collect();
    let volume = |row: &[f64]| row.iter().zip(&sol_v).map(|(m, v)| m * v).sum::<f64>();
    let kg = |row: &[f64]| row.iter().zip(&mm).map(|(m, k)| m * k).sum::<f64>();
    let texture = |i: usize| {
        let t = &w.soil[i].texture;
        [t.fine, t.middle(), t.coarse]
    };
    let mut pools = Pools { rows: vec![0.0; c.keys.len() * width], came: vec![[0.0; CLASS_COUNT]; c.keys.len()], picked: vec![[0.0; CLASS_COUNT]; c.keys.len()], width };
    let mut moves = Moves::default();
    let mut settled = vec![[0.0; CLASS_COUNT]; n];
    // Each cell's pick-up goes to its pairs by their share of its work; what no pair takes stays on its ground.
    let mut sent = vec![0.0; n];
    for (p, &(i, key)) in c.pairs.iter().enumerate() {
        let (i, q) = (i as usize, key as usize);
        if work.pairs[p] > 0.0 {
            let share = work.pairs[p] / work.cells[i];
            let row: Vec<f64> = picked[i * width..(i + 1) * width].iter().map(|m| m * share).collect();
            let v = volume(&row[..width - 1]);
            pools.add(q, &row);
            pools.picked[q].iter_mut().zip(texture(i)).for_each(|(e, t)| *e += v * t);
            moves.add((i as u32, b.cell_of[c.keys[q] as usize]), &row);
            sent[i] += share;
        }
    }
    for i in (0..n).filter(|&i| work.cells[i] > 0.0 && sent[i] < 1.0) {
        let v = volume(&picked[i * width..(i + 1) * width - 1]) * (1.0 - sent[i]);
        settled[i].iter_mut().zip(texture(i)).for_each(|(d, t)| *d += v * t);
        volumes.land += v;
    }
    volumes.picked = (0..n).filter(|&i| work.cells[i] > 0.0).map(|i| volume(&picked[i * width..(i + 1) * width - 1])).sum();
    let mut loads = vec![[0.0f32; 2]; n];
    let mut plumes = Vec::new();
    // The serial pass: each key sample settles its share of each class, and its pool's rest goes on to the next; where
    // a way ends, all of it settles.
    let mut row = vec![0.0; width];
    for (q, &k) in c.keys.iter().enumerate() {
        let i = b.cell_of[k as usize] as usize;
        row.copy_from_slice(&pools.rows[q * width..(q + 1) * width]);
        if c.main[i] == q as u32 {
            loads[i][0] = main_load(w, i, kg(&row[..width - 1]), flux[k as usize][0]);
        }
        let (came, got, sh) = (pools.came[q], pools.picked[q], c.shares[q]);
        let out = step(came, got, &sh);
        let (all, on) = (came.iter().chain(&got).sum::<f64>(), out.iter().sum::<f64>());
        (0..CLASS_COUNT).for_each(|l| settled[i][l] += came[l] + got[l] - out[l]);
        let body = b.at[k as usize];
        let gone = all - on;
        match (c.next[q] == NONE, body) {
            (true, body) if body != NONE && b.list[body as usize].pass == NONE => volumes.open += gone,
            (true, body) if body != NONE => volumes.bodies += gone,
            _ => volumes.land += gone,
        }
        if c.next[q] == NONE {
            let mass = kg(&row[..width - 1]);
            if body != NONE && mass > 0.0 {
                plumes.push((k, (mass / dt) as f32));
            }
        } else if on > 0.0 {
            let next = c.next[q] as usize;
            row.iter_mut().for_each(|m| *m *= (on / all).min(1.0));
            pools.add(next, &row);
            pools.came[next].iter_mut().zip(out).for_each(|(a, o)| *a += o);
            moves.add((i as u32, b.cell_of[c.keys[next] as usize]), &row);
        }
    }
    for ((from, to), row) in moves.list {
        x.push(from, to, row[..width - 1].iter().copied().enumerate(), row[width - 1]);
    }
    let area = w.mesh.cells().area();
    loads.iter_mut().zip(&settled).zip(area).for_each(|((l, v), a)| l[1] = (v.iter().sum::<f64>() / a) as f32);
    s.spare.picked = picked;
    Carried { broke, loads, plumes, volumes, settled }
}

/// What settled on each cell, `settled` (m³ of each class, `Carried`), mixes into its texture by volume over its loose
/// ground (the solids `sol`) up to `skin_m` deep, as the texture of a cell is found (`soil::of_world`): the load booked,
/// what it held before is what it holds less what settled. Its pores are found again at the month's start where they
/// moved (`soil::month`).
pub(super) fn mix(w: &mut World, s: &Scratch, settled: &[[f64; CLASS_COUNT]], sol: &[usize]) {
    let area = w.mesh.cells().area();
    for (i, v) in settled.iter().enumerate().filter(|(_, v)| v.iter().sum::<f64>() > 0.0) {
        let came: f64 = v.iter().sum();
        let loose: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let held = (loose - came).clamp(0.0, w.cfg.skin_m * area[i]);
        let t = &mut w.soil[i].texture;
        (t.fine, t.coarse) = ((t.fine * held + v[0]) / (held + came), (t.coarse * held + v[2]) / (held + came));
    }
}

/// What of a pool leaves a key sample of shares `sh`, m³ of each class: of what came in from the key samples above,
/// `came`, the share that passes, and of what was picked up on the ways to it, `got`, the share of that (`Shares`).
fn step(came: [f64; CLASS_COUNT], got: [f64; CLASS_COUNT], sh: &Shares) -> [f64; CLASS_COUNT] {
    std::array::from_fn(|l| came[l] * f64::from(sh[0][l]) + got[l] * f64::from(sh[1][l]))
}

/// The load in cell `i`'s main channel, g per m³ of its liquid, from the load that passes there, `kg`, and the liquid,
/// `mol`, in the month.
fn main_load(w: &World, i: usize, kg: f64, mol: f64) -> f32 {
    let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
    if per_m3 > 0.0 && mol > 0.0 { (G_PER_KG * kg / (mol / per_m3)) as f32 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{step, Shares, CLASS_COUNT};
    use crate::soil::{passes, CLASSES};

    /// A reach of a channel in which a flood of `q` m³/s a m of width runs `depth` m deep down `slope`, `run` m long,
    /// over particles 1.65 times the liquid's weight (a common rock's particles), at 9.81 m/s²: its shares.
    fn reach(slope: f64) -> Shares {
        let (q, depth, run, g) = (1.4, 1.9, 125.0, 9.81);
        passes((g * depth * slope).sqrt(), q, run, 1.65, g).map(|r| r.map(|x| x as f32))
    }

    /// A load of each class carried down a chain of reaches of `slopes`, each picking up `got` m³ of each class: what
    /// settles in each, and what leaves the last.
    fn down(slopes: &[f64], load: [f64; CLASS_COUNT], got: [f64; CLASS_COUNT]) -> (Vec<[f64; CLASS_COUNT]>, [f64; CLASS_COUNT]) {
        let mut came = load;
        let settled = slopes.iter().map(|&s| {
            let out = step(came, got, &reach(s));
            let left = std::array::from_fn(|l| came[l] + got[l] - out[l]);
            came = out;
            left
        }).collect();
        (settled, came)
    }

    /// A load carried down a steep reach and then a flat plain: the coarse class passes the steep reaches and settles
    /// on the plain, the fine passes both; what came in and was picked up is what settled and what left.
    #[test]
    fn a_load_settles_where_its_flow_slows() {
        let slopes: Vec<f64> = (0..20).map(|r| if r < 10 { 0.02 } else { 1e-4 }).collect();
        let (load, got) = ([1.0; CLASS_COUNT], [0.01; CLASS_COUNT]);
        let (settled, out) = down(&slopes, load, got);
        let (steep, flat) = settled.split_at(10);
        let sum = |part: &[[f64; CLASS_COUNT]], l: usize| part.iter().map(|x| x[l]).sum::<f64>();
        assert!(sum(steep, 2) < 1e-3 * sum(flat, 2), "coarse settles on the plain, not the steep reaches: {} {}", sum(steep, 2), sum(flat, 2));
        assert!(sum(flat, 2) > 0.5 && out[0] > 0.9, "most coarse settles on the plain ({}), most fine passes ({})", sum(flat, 2), out[0]);
        for l in 0..CLASS_COUNT {
            let came = load[l] + got[l] * slopes.len() as f64;
            assert!((came - sum(&settled, l) - out[l]).abs() < 1e-12, "class {l} is kept");
        }
    }

    /// Down a channel that grows ever flatter the coarse class settles first: the mean size of what settles falls
    /// along it.
    #[test]
    fn coarse_settles_before_fine() {
        let slopes: Vec<f64> = (0..20).map(|r| 1e-2 * 10f64.powf(-0.3 * r as f64)).collect();
        let (settled, _) = down(&slopes, [1.0; CLASS_COUNT], [0.0; CLASS_COUNT]);
        // The mean log-size of each class.
        let size = CLASSES.map(|[a, b]| (a * b).sqrt().ln());
        let mean: Vec<f64> = settled.iter().filter(|x| x.iter().sum::<f64>() > 0.0).map(|x| x.iter().zip(size).map(|(v, s)| v * s).sum::<f64>() / x.iter().sum::<f64>()).collect();
        let falls = mean.windows(2).all(|m| m[1] <= m[0] + 1e-9) && mean[mean.len() - 1] < mean[0] - 1.0;
        assert!(falls, "the mean size of what settles falls: {mean:?}");
    }
}

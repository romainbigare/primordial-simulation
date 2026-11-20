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

use super::{bare_share, solid, wear, Bodies, Crossings, Scratch};
use crate::soil::{flood_shear, passes};
use crate::space::{delta::FLOOD_RUNOFF, network::NONE, samples::Samples};
use crate::{store::Move, units::G_PER_KG, World};
use std::collections::HashMap;

/// The particle classes of the load: fine, middle, coarse (`soil::texture`).
const CLASSES: usize = 3;
/// A sample that lets each class pass to within this share is no key sample: what would settle there is left to the
/// key samples below.
const PASS_TOLERANCE: f64 = 1e-4;

/// Of each particle class, the share of a load that passes a key sample and the share of what was picked up on the ways
/// to it that does (`soil::passes`; f32: shares hold no matter).
type Shares = [[f32; CLASSES]; 2];

/// The key samples of the terrain's ways (`Carriage::of`): the key samples, those whose liquid runs on in the order of
/// `Bodies::down`, then the ends; of each sample its place among them (`NONE` for none); of each its next key sample
/// down its way (`NONE` for an end) and its shares (none pass an end); of each land sample its pair (`NONE` for none);
/// the pairs, each a cell with the first key sample below one of its samples, in the order of the land samples; of each
/// cell its main channel's key sample (`NONE` for none).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Carriage {
    keys: Vec<u32>,
    place: Vec<u32>,
    next: Vec<u32>,
    shares: Vec<Shares>,
    pair_of: Vec<u32>,
    pairs: Vec<(u32, u32)>,
    main: Vec<u32>,
}

impl Carriage {
    /// The carriage of the bodies `b` of world `w` (`Carriage`): a sample whose liquid runs on is a key where some class
    /// settles (`shares`), or where it is its cell's main channel, its land sample the most drains through
    /// (`Samples::up`); a way ends where it enters a body, or at a land sample whose liquid runs to none.
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
        let key = |c: &mut Carriage, k: u32, shares: Shares| {
            if c.place[k as usize] == NONE {
                c.place[k as usize] = c.keys.len() as u32;
                c.keys.push(k);
                c.shares.push(shares);
            }
        };
        let of_down = crate::par::map_n(b.down.len(), crate::par::UNCOUNTED, |q| shares(w, &s, b.down[q] as usize));
        for (&k, sh) in b.down.iter().zip(of_down) {
            let settles = sh[0].iter().any(|&p| f64::from(p) < 1.0 - PASS_TOLERANCE);
            if settles || main[b.cell_of[k as usize] as usize] == k {
                key(&mut c, k, if settles { sh } else { [[1.0; CLASSES]; 2] });
            }
        }
        for &k in &b.land {
            let j = s.to[k as usize];
            if j == NONE {
                key(&mut c, k, [[0.0; CLASSES]; 2]);
            } else if b.at[j as usize] != NONE {
                key(&mut c, j, [[0.0; CLASSES]; 2]);
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

/// The shares of sample `k` of samples `s` in world `w` (`Shares`, `soil::passes`): under the flood (`FLOOD_RUNOFF`
/// from the area that drains through it; `soil::flood_shear`) in a channel as wide as the law `channel_width` makes it
/// (Leopold and Maddock 1953), over the run to the sample its liquid runs to, of particles of its bed's make.
fn shares(w: &World, s: &Samples, k: usize) -> Shares {
    let (ph, g) = (&w.cfg.phys, w.cfg.gravity);
    let flood = FLOOD_RUNOFF * s.up[k];
    let wide = ph.channel_width * flood.powf(ph.channel_width_exp);
    let buoyant = w.terrain.makes.get(s.make[k] as usize).copied().unwrap_or_default().buoyant;
    passes(flood_shear(s, k, g), flood / wide, s.run[k], buoyant, g).map(|r| r.map(|x| x as f32))
}

/// What the load did in a month: the moves of the bed into loose ground (`wear`); of each cell what its main channel
/// carried, g per m³ of its liquid, and the depth that settled on it, m; the plumes: each sample where a way entered a
/// body, with the load that came in there, kg/s; and the volumes, m³, picked up, carried into the open low ground and
/// into the other bodies, settled on land, and of the bed worn into loose ground (`Counters::load`).
pub(super) struct Carried {
    pub(super) broke: Vec<Move>,
    pub(super) loads: Vec<[f32; 2]>,
    pub(super) plumes: Vec<(u32, f32)>,
    pub(super) volumes: [f64; 5],
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
    came: Vec<[f64; CLASSES]>,
    picked: Vec<[f64; CLASSES]>,
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
    let (work, picked, broke) = pick_up(w, s, b, flux, sol);
    let sol_v: Vec<f64> = sol.iter().map(|&k| s.sp[k].v[0]).collect();
    let mut volumes = [0.0; 5];
    volumes[4] = broke.iter().map(|m| m.mol * s.sp[m.k as usize].v[0]).sum();
    let mm: Vec<f64> = sol.iter().map(|&k| s.sp[k].mm).collect();
    let volume = |row: &[f64]| row.iter().zip(&sol_v).map(|(m, v)| m * v).sum::<f64>();
    let kg = |row: &[f64]| row.iter().zip(&mm).map(|(m, k)| m * k).sum::<f64>();
    let texture = |i: usize| {
        let t = &w.soil[i].texture;
        [t.fine, t.middle(), t.coarse]
    };
    let mut pools = Pools { rows: vec![0.0; c.keys.len() * width], came: vec![[0.0; CLASSES]; c.keys.len()], picked: vec![[0.0; CLASSES]; c.keys.len()], width };
    let mut moves = Moves::default();
    let mut settled = vec![[0.0; CLASSES]; n];
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
        volumes[3] += v;
    }
    volumes[0] = (0..n).filter(|&i| work.cells[i] > 0.0).map(|i| volume(&picked[i * width..(i + 1) * width - 1])).sum();
    let mut loads = vec![[0.0f32; 2]; n];
    let mut plumes = Vec::new();
    // The serial pass: each key sample settles its share of each class, and its pool's rest goes on to the next; where
    // a way ends, all of it settles.
    for (q, &k) in c.keys.iter().enumerate() {
        let i = b.cell_of[k as usize] as usize;
        let row = pools.rows[q * width..(q + 1) * width].to_vec();
        if c.main[i] == q as u32 {
            loads[i][0] = main_load(w, i, kg(&row[..width - 1]), flux[k as usize][0]);
        }
        let (came, got, sh) = (pools.came[q], pools.picked[q], c.shares[q]);
        let out = step(came, got, &sh);
        let (all, on) = (came.iter().chain(&got).sum::<f64>(), out.iter().sum::<f64>());
        (0..CLASSES).for_each(|l| settled[i][l] += came[l] + got[l] - out[l]);
        let body = b.at[k as usize];
        let into = if body == NONE { 3 } else if b.list[body as usize].pass == NONE { 1 } else { 2 };
        volumes[if c.next[q] == NONE { into } else { 3 }] += all - on;
        if c.next[q] == NONE {
            let mass = kg(&row[..width - 1]);
            if body != NONE && mass > 0.0 {
                plumes.push((k, (mass / dt) as f32));
            }
        } else if on > 0.0 {
            let next = c.next[q] as usize;
            let leaves: Vec<f64> = row.iter().map(|m| m * (on / all).min(1.0)).collect();
            pools.add(next, &leaves);
            pools.came[next].iter_mut().zip(out).for_each(|(a, o)| *a += o);
            moves.add((i as u32, b.cell_of[c.keys[next] as usize]), &leaves);
        }
    }
    for ((from, to), row) in moves.list {
        x.push(from, to, row[..width - 1].iter().copied().enumerate(), row[width - 1]);
    }
    let area = w.mesh.cells().area();
    loads.iter_mut().zip(&settled).zip(area).for_each(|((l, v), a)| l[1] = (v.iter().sum::<f64>() / a) as f32);
    Carried { broke, loads, plumes, volumes }
}

/// What of a pool leaves a key sample of shares `sh`, m³ of each class: of what came in from the key samples above,
/// `came`, the share that passes, and of what was picked up on the ways to it, `got`, the share of that (`Shares`).
fn step(came: [f64; CLASSES], got: [f64; CLASSES], sh: &Shares) -> [f64; CLASSES] {
    std::array::from_fn(|l| came[l] * f64::from(sh[0][l]) + got[l] * f64::from(sh[1][l]))
}

/// The load in cell `i`'s main channel, g per m³ of its liquid, from the load that passes there, `kg`, and the liquid,
/// `mol`, in the month.
fn main_load(w: &World, i: usize, kg: f64, mol: f64) -> f32 {
    let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
    if per_m3 > 0.0 && mol > 0.0 { (G_PER_KG * kg / (mol / per_m3)) as f32 } else { 0.0 }
}

/// The work of the month's runoff, J: of each pair of `Carriage`, and of each cell.
struct Work {
    pairs: Vec<f64>,
    cells: Vec<f64>,
}

/// What each land cell picks up (laws.md 1.3): the head the liquid loses down the samples' ways in it does work, g
/// times what runs through each of its samples in no pond, kg, times the fall to the sample it runs to; it picks up
/// loose solids up to `carry` m³ per J times its loose ground's erodibility (`Texture::erodibility`) and its bare share
/// (`bare_share`), at most all that lies loose, with their heat at its temperature; the work its load leaves free (the
/// cover effect, Sklar and Dietrich 2004) breaks bonded ground into loose (`wear`). Returns the work (`Work`), what each
/// cell picked up (rows of `sol`, mol, then the heat, J), and the moves of the bed into loose ground.
fn pick_up(w: &World, s: &mut Scratch, b: &Bodies, flux: &[[f64; 2]], sol: &[usize]) -> (Work, Vec<f64>, Vec<Move>) {
    let (samples, n, width, c) = (w.terrain_samples(), w.mesh.cells().n(), sol.len() + 1, &b.carriage);
    let mut work = Work { pairs: vec![0.0; c.pairs.len()], cells: vec![0.0; n] };
    for &k in b.land.iter().filter(|&&k| flux[k as usize][1] > 0.0) {
        let (k, f) = (k as usize, flux[k as usize]);
        let (i, j) = (b.cell_of[k] as usize, samples.to[k]);
        if j != NONE && s.z[i] > 0.0 {
            let done = w.cfg.gravity * f[1] * (samples.h[k] - samples.h[j as usize]).max(0.0);
            work.cells[i] += done;
            if c.pair_of[k] != NONE {
                work.pairs[c.pair_of[k] as usize] += done;
            }
        }
    }
    let (mut picked, mut broke) = (vec![0.0; n * width], Vec::new());
    for i in (0..n).filter(|&i| work.cells[i] > 0.0) {
        let cap = w.cfg.carry * work.cells[i] * bare_share(w, i) * w.soil[i].texture.erodibility();
        let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let f = if avail > 0.0 { (cap / avail).min(1.0) } else { 0.0 };
        for (r, &k) in sol.iter().enumerate() {
            picked[i * width + r] = solid(w, s, k, i) * f;
            picked[(i + 1) * width - 1] += solid(w, s, k, i) * f * s.sp[k].cp * s.state.tg[i];
        }
        let worn = work.cells[i] * (1.0 - f * avail / cap).max(0.0);
        if worn > 0.0 {
            wear(w, s, i, worn, &mut broke);
        }
    }
    (work, picked, broke)
}

#[cfg(test)]
mod tests {
    use super::{step, Shares, CLASSES};
    use crate::soil::passes;

    /// A reach of a channel in which a flood of `q` m³/s a m of width runs `depth` m deep down `slope`, `run` m long,
    /// over particles 1.65 times the liquid's weight (a common rock's particles), at 9.81 m/s²: its shares.
    fn reach(slope: f64) -> Shares {
        let (q, depth, run, g) = (1.4, 1.9, 125.0, 9.81);
        passes((g * depth * slope).sqrt(), q, run, 1.65, g).map(|r| r.map(|x| x as f32))
    }

    /// A load of each class carried down a chain of reaches of `slopes`, each picking up `got` m³ of each class: what
    /// settles in each, and what leaves the last.
    fn down(slopes: &[f64], load: [f64; CLASSES], got: [f64; CLASSES]) -> (Vec<[f64; CLASSES]>, [f64; CLASSES]) {
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
        let (load, got) = ([1.0; CLASSES], [0.01; CLASSES]);
        let (settled, out) = down(&slopes, load, got);
        let (steep, flat) = settled.split_at(10);
        let sum = |part: &[[f64; CLASSES]], l: usize| part.iter().map(|x| x[l]).sum::<f64>();
        assert!(sum(steep, 2) < 1e-3 * sum(flat, 2), "coarse settles on the plain, not the steep reaches: {} {}", sum(steep, 2), sum(flat, 2));
        assert!(sum(flat, 2) > 0.5 && out[0] > 0.9, "most coarse settles on the plain ({}), most fine passes ({})", sum(flat, 2), out[0]);
        for l in 0..CLASSES {
            let came = load[l] + got[l] * slopes.len() as f64;
            assert!((came - sum(&settled, l) - out[l]).abs() < 1e-12, "class {l} is kept");
        }
    }

    /// Down a channel that grows ever flatter the coarse class settles first: the mean size of what settles falls
    /// along it.
    #[test]
    fn coarse_settles_before_fine() {
        let slopes: Vec<f64> = (0..20).map(|r| 1e-2 * 10f64.powf(-0.3 * r as f64)).collect();
        let (settled, _) = down(&slopes, [1.0; CLASSES], [0.0; CLASSES]);
        // The mean log-size of each class (`soil::texture`'s classes): 0.63 µm, 10 µm, 0.32 mm.
        let size = [-14.27, -11.51, -8.05];
        let mean: Vec<f64> = settled.iter().filter(|x| x.iter().sum::<f64>() > 0.0).map(|x| x.iter().zip(size).map(|(v, s)| v * s).sum::<f64>() / x.iter().sum::<f64>()).collect();
        let falls = mean.windows(2).all(|m| m[1] <= m[0] + 1e-9) && mean[mean.len() - 1] < mean[0] - 1.0;
        assert!(falls, "the mean size of what settles falls: {mean:?}");
    }
}

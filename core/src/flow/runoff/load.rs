//! The load the running liquid carries (laws.md 1.3). Each month each land cell picks up loose solids from the work its
//! runoff does there (`pick_up`), and the load goes down the samples' ways, in three particle classes (fine, middle,
//! coarse). It is followed only at the **key samples** (`Carriage`): those where some of a class settles, each land
//! cell's main channel, and the samples where the ways end (in a body, or on land where none runs on). Each key sample
//! holds a **pool** this month (`Pools`): what came in at the top of its reach, from the key samples above and what was
//! picked up above it, and what was picked up along its own reach (mol of each solid substance and its heat). One serial
//! pass over the key samples, each before the next, settles its share of each class (`soil::passes`) and moves the rest
//! on; where a way ends, all of it settles in the cell that holds that sample.
//!
//! The books: a move goes from the cell it leaves to the cell of the key sample it reaches; what settles stays in the
//! node it reached. The moves over each link between two cells (`Carriage::links`) are summed and booked once.
//! **Known limit:** in a pool the substances and the particle classes are independent: what leaves takes the pool's
//! mixture of substances by the share of its volume that leaves.

use super::carriage::{End, Shares, CLASS_COUNT};
use super::pick::{pick_up, Work};
use super::super::land::solid;
use super::{Bodies, Crossings, Scratch};
use crate::{space::network::NONE, store::Move, units::G_PER_KG, World};

/// What the load did in a month: the moves of the bed into loose ground (`wear`); of each cell what its main channel
/// carried, g per m³ of its liquid, and the depth that settled on it, m; the plumes: each sample where a way entered a
/// body, with the load that came in there, kg/s; the volumes of the month (`Volumes`); of each cell the volume of
/// each class that settled on it, m³ (`mix`); and what the month laid on the samples' relief (taken where under none),
/// m (`Relief`): what settles at a key sample whose way runs on or ends on land lies on it; what reaches a body lies
/// over its entry cell's ground at large.
pub(super) struct Carried {
    pub(super) broke: Vec<Move>,
    pub(super) loads: Vec<[f32; 2]>,
    pub(super) plumes: Vec<(u32, f32)>,
    pub(super) volumes: Volumes,
    pub(super) settled: Vec<[f64; CLASS_COUNT]>,
    pub(super) relief: Vec<(u32, f64)>,
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

/// The pools of the key samples this month: of each its row (mol of each solid substance, then the heat, J; `width`
/// numbers a key); and of each class the volume that came in at the top of its reach (from the key samples above and
/// what was picked up above it), that was picked up along its own reach, and that settled on the samples between it
/// and the key sample above (`Carriage::skip`), m³. The pass takes each pool once and leaves it none, so the room is
/// kept from month to month all none (`Spare::pools`).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Pools {
    rows: Vec<f64>,
    came: Vec<[f64; CLASS_COUNT]>,
    own: Vec<[f64; CLASS_COUNT]>,
    dropped: Vec<[f64; CLASS_COUNT]>,
    width: usize,
}

impl Pools {
    /// Room for `keys` pools of rows `width` long, all none.
    fn resize(&mut self, keys: usize, width: usize) {
        if self.width != width || self.came.len() != keys {
            *self = Pools { rows: vec![0.0; keys * width], came: vec![[0.0; CLASS_COUNT]; keys], own: vec![[0.0; CLASS_COUNT]; keys], dropped: vec![[0.0; CLASS_COUNT]; keys], width };
        }
    }
    /// Row `row` into key `q`'s pool.
    fn add(&mut self, q: usize, row: &[f64]) {
        self.rows[q * self.width..(q + 1) * self.width].iter_mut().zip(row).for_each(|(a, b)| *a += b);
    }
    /// Key `q`'s pool, taken: its row into `row`, and what came in, what its own reach picked up and what settled above.
    fn take(&mut self, q: usize, row: &mut [f64]) -> [[f64; CLASS_COUNT]; 3] {
        let at = &mut self.rows[q * self.width..(q + 1) * self.width];
        row.copy_from_slice(at);
        at.fill(0.0);
        [std::mem::take(&mut self.came[q]), std::mem::take(&mut self.own[q]), std::mem::take(&mut self.dropped[q])]
    }
}

/// The rows the load moves over each link of `Carriage::links` in a month, summed (`width` numbers a link).
struct Links {
    rows: Vec<f64>,
    width: usize,
}

impl Links {
    /// `row` moved over link `l` (none for `NONE`, a move within a cell).
    fn add(&mut self, l: u32, row: &[f64]) {
        if l != NONE {
            let at = l as usize * self.width;
            self.rows[at..at + self.width].iter_mut().zip(row).for_each(|(a, b)| *a += b);
        }
    }
}

/// One month of the load (the module's doc) over the bodies `b`, what ran through each sample `flux` (mol and kg), the
/// solid substances `sol`, in a month of `dt` s: each cell's pick-up, its share to each of its pairs by their work,
/// the serial pass over the key samples, the ends. Notes the crossings in `x`. Returns `Carried`.
pub(super) fn carry(w: &World, s: &mut Scratch, b: &Bodies, (flux, sol): (&[[f64; 2]], &[usize]), x: &mut Crossings, dt: f64) -> Carried {
    let (c, n, width) = (&b.carriage, w.mesh.cells().n(), sol.len() + 1);
    let mut picked = std::mem::take(&mut s.spare.picked);
    let (work, broke, taken) = pick_up(w, s, b, (flux, sol), &mut picked);
    let mut relief: Vec<(u32, f64)> = taken.into_iter().map(|(k, m)| (k, -m)).collect();
    let sol_v: Vec<f64> = sol.iter().map(|&k| s.sp[k].v[0]).collect();
    let mut volumes = Volumes { worn: broke.iter().map(|m| m.mol * s.sp[m.k as usize].v[0]).sum(), ..Volumes::default() };
    let mm: Vec<f64> = sol.iter().map(|&k| s.sp[k].mm).collect();
    let volume = |row: &[f64]| row.iter().zip(&sol_v).map(|(m, v)| m * v).sum::<f64>();
    let kg = |row: &[f64]| row.iter().zip(&mm).map(|(m, k)| m * k).sum::<f64>();
    let keys = c.keys.len();
    let mut pools = std::mem::take(&mut s.spare.pools);
    pools.resize(keys, width);
    let mut links = Links { rows: vec![0.0; c.links.len() * width], width };
    gather(w, (c, &work), (&picked, &sol_v), (&mut pools, &mut links));
    volumes.picked = (0..n).filter(|&i| work.cells[i] > 0.0).map(|i| volume(&picked[i * width..(i + 1) * width - 1])).sum();
    let (mut settled, mut loads, mut plumes) = (vec![[0.0; CLASS_COUNT]; n], vec![[0.0f32; 2]; n], Vec::new());
    // The serial pass: each key sample settles its share of each class, and its pool's rest goes on to the next; where
    // a way ends, all of it settles.
    let mut row = vec![0.0; width];
    for (q, &k) in c.keys.iter().enumerate() {
        let i = c.cell[q] as usize;
        let [came, own, dropped] = pools.take(q, &mut row);
        let a = b.share[k as usize] * w.mesh.cells().area()[i];
        if c.main[i] == q as u32 {
            loads[i][0] = main_load(w, i, kg(&row[..width - 1]), flux[k as usize][0]);
        }
        if came.iter().chain(&own).chain(&dropped).all(|&v| v == 0.0) {
            continue;
        }
        let mut out = step(came, own, &c.shares[q]);
        if c.end[q] == End::On {
            fill_room(&mut out, [came, own, dropped], c.room[q] - w.relief.since(k as usize) * a);
        }
        let all: f64 = came.iter().chain(&own).chain(&dropped).sum();
        let on: f64 = out.iter().sum();
        (0..CLASS_COUNT).for_each(|l| settled[i][l] += came[l] + own[l] + dropped[l] - out[l]);
        if matches!(c.end[q], End::On | End::Land) && all > on {
            relief.push((k, (all - on) / a));
        }
        let gone = all - on;
        match c.end[q] {
            End::Open => volumes.open += gone,
            End::Body => volumes.bodies += gone,
            End::On | End::Land => volumes.land += gone,
        }
        if matches!(c.end[q], End::Open | End::Body) {
            plumes.push((k, (kg(&row[..width - 1]) / dt) as f32));
        } else if c.end[q] == End::On && on > 0.0 {
            // All that leaves goes to the next key sample's cell; what the samples between take settles there.
            let next = c.next[q] as usize;
            row.iter_mut().for_each(|m| *m *= (on / all).min(1.0));
            pools.add(next, &row);
            for (l, &o) in out.iter().enumerate() {
                let arrives = o * f64::from(c.skip[q][l]);
                (pools.came[next][l], pools.dropped[next][l]) = (pools.came[next][l] + arrives, pools.dropped[next][l] + o - arrives);
            }
            links.add(c.link[q], &row);
        }
    }
    for (l, &(from, to)) in c.links.iter().enumerate() {
        let r = &links.rows[l * width..(l + 1) * width];
        if r.iter().any(|&m| m != 0.0) {
            x.push(from, to, r[..width - 1].iter().copied().enumerate(), r[width - 1]);
        }
    }
    let area = w.mesh.cells().area();
    loads.iter_mut().zip(&settled).zip(area).for_each(|((l, v), a)| l[1] = (v.iter().sum::<f64>() / a) as f32);
    (s.spare.picked, s.spare.pools) = (picked, pools);
    Carried { broke, loads, plumes, volumes, settled, relief }
}

/// Each cell's pick-up (`picked`, rows of the solids of molar volumes `sol_v`, then the heat) to its pairs by their
/// share of its work (`Work`), each class by the cell's texture: what its key sample's own reach picked up passes it by
/// R, what was picked up above it comes in at the top of the reach (`Pools`).
fn gather(w: &World, (c, work): (&super::Carriage, &Work), (picked, sol_v): (&[f64], &[f64]), (pools, links): (&mut Pools, &mut Links)) {
    let width = sol_v.len() + 1;
    let mut row = vec![0.0; width];
    for (p, pair) in c.pairs.iter().enumerate() {
        let (i, q) = (pair.cell as usize, pair.key as usize);
        let [above, own] = work.pairs[p];
        if above + own > 0.0 {
            let share = (above + own) / work.cells[i];
            row.iter_mut().zip(&picked[i * width..(i + 1) * width]).for_each(|(r, m)| *r = m * share);
            let v: f64 = row.iter().zip(sol_v).map(|(m, v)| m * v).sum();
            let t = &w.soil[i].texture;
            for (l, part) in [t.fine, t.middle(), t.coarse].into_iter().enumerate() {
                pools.came[q][l] += v * part * above / (above + own);
                pools.own[q][l] += v * part * own / (above + own);
            }
            pools.add(q, &row);
            links.add(pair.link, &row);
        }
    }
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

/// What leaves a key sample, `out` m³ of each class, when what would settle there fills more than its room `room`, m³
/// (`Carriage::room`), of what came in, was picked up along its reach and settled from above (`dropped`): every class
/// settles by the share of what it would that the room takes, the rest passes on (the threshold channel, Parker 1978:
/// a sample rises no steeper than the slope at which nothing settles).
fn fill_room(out: &mut [f64; CLASS_COUNT], [came, own, dropped]: [[f64; CLASS_COUNT]; 3], room: f64) {
    let wanted: [f64; CLASS_COUNT] = std::array::from_fn(|l| came[l] + own[l] + dropped[l] - out[l]);
    let all: f64 = wanted.iter().sum();
    if all > room.max(0.0) {
        let share = room.max(0.0) / all;
        out.iter_mut().zip(wanted).for_each(|(o, x)| *o += x * (1.0 - share));
    }
}

/// What of a pool leaves a key sample of shares `sh`, m³ of each class: of what came in at the top of its reach,
/// `came`, the share that passes, and of what was picked up along it, `own`, the share of that (`Shares`).
fn step(came: [f64; CLASS_COUNT], own: [f64; CLASS_COUNT], sh: &Shares) -> [f64; CLASS_COUNT] {
    std::array::from_fn(|l| came[l] * f64::from(sh[0][l]) + own[l] * f64::from(sh[1][l]))
}

/// The load in cell `i`'s main channel, g per m³ of its liquid, from the load that passes there, `kg`, and the liquid,
/// `mol`, in the month.
fn main_load(w: &World, i: usize, kg: f64, mol: f64) -> f32 {
    let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
    if per_m3 > 0.0 && mol > 0.0 { (G_PER_KG * kg / (mol / per_m3)) as f32 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{fill_room, step, Shares, CLASS_COUNT};
    use crate::soil::{passes, threshold_slope, CLASSES};

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

    /// The threshold slope (`soil::threshold_slope`) is where `passes` lets every class pass: the room and the shares
    /// read one rule.
    #[test]
    fn nothing_settles_down_the_threshold_slope() {
        let (depth, q, run, buoyant, g) = (1.9, 1.4, 125.0, 1.65, 9.81);
        let u = (g * depth * threshold_slope(depth, buoyant, g)).sqrt();
        let shares = passes(u, q, run, buoyant, g).map(|r| r.map(|x| x as f32));
        assert_eq!(shares, [[1.0; CLASS_COUNT]; 2], "at the threshold slope");
    }

    /// A load comes month after month into a plain of samples, each with the same room: each settles no more than its
    /// room, and the deposit grows forward: the first samples full, what lies below them less the farther down.
    #[test]
    fn a_sample_holds_load_up_to_its_room() {
        let (rooms, months) = ([1.0; 10], 12);
        let mut laid = [0.0; 10];
        for _ in 0..months {
            let mut came = [0.0, 0.2, 0.3];
            for (k, &room) in rooms.iter().enumerate() {
                let mut out = step(came, [0.0; CLASS_COUNT], &reach(1e-5));
                fill_room(&mut out, [came, [0.0; CLASS_COUNT], [0.0; CLASS_COUNT]], room - laid[k]);
                laid[k] += came.iter().sum::<f64>() - out.iter().sum::<f64>();
                came = out;
            }
        }
        assert!(laid.iter().all(|&x| x <= 1.0 + 1e-12), "a sample over its room: {laid:?}");
        let full = laid.iter().take_while(|&&x| x > 1.0 - 1e-12).count();
        let falls = laid.windows(2).all(|p| p[1] <= p[0]);
        assert!(full >= 3 && falls, "the deposit grows forward from full samples: {laid:?}");
    }
}

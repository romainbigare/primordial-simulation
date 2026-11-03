//! The air level day by day over an air step (laws.md 1.4 to 1.6): each node's exchange with the ground under it, its
//! rain and its heat radiation to space, then what eddies even out within a day (`blocks`) and the wind across the links
//! between blocks. Explicit, on plain arrays of the level.

use super::super::Scratch;
use super::rain::{rain_air, Pool, Wet};
use super::wind::{even, lines, pushes, wind, Implicit, Links};
use super::{air_temp, at_height, radiation::{glow, glow_slope}, step::Active};
use crate::{math::mean_passed, store::Class, units::{DAY, R_GAS}, World, AIR};
use std::{collections::BTreeMap, ops::Range};

/// The time in which eddies even out a block of air, s: the air's day, so a block's side is the distance a diffusion of
/// `mix` spreads in it, sqrt(`mix` × this) (Taylor 1922), about 340 km (`blocks`).
const BLOCK_DAY: f64 = DAY;

/// What the ground under each air node gives it over a step, as its cells' one law (`cells::Response::ground`): per
/// node and output (what its air gains, J, then what evaporates of each active gas, mol, `[j * (1 + nx) + o]`) its
/// value at the air of the step's start (`t0`, K, per node; `x0`, mol, `[y * na + j]` per active gas), and per node
/// its cells' slopes with the air of each node they see, entries `at[j]..at[j + 1]`, node `near[e]`, slope of output
/// `o` with that node's temperature (`v` 0) or gas `y` (`v` 1 + y) at `slope[e * (1 + nx)² + o * (1 + nx) + v]`; and
/// where it rains (`Wet`).
pub(super) struct Ground {
    nx: usize,
    pub(super) base: Vec<f64>,
    pub(super) at: Vec<usize>,
    pub(super) near: Vec<u32>,
    pub(super) slope: Vec<f64>,
    t0: Vec<f64>,
    x0: Vec<f64>,
    wet: Wet,
}

impl Ground {
    pub(super) fn new(na: usize, nx: usize, (t0, x0): (Vec<f64>, Vec<f64>), wet: Wet) -> Ground {
        Ground { nx, base: vec![0.0; na * (1 + nx)], at: vec![0], near: Vec::new(), slope: Vec::new(), t0, x0, wet }
    }
    /// Where its rain falls.
    pub(super) fn wet(&self) -> &Wet {
        &self.wet
    }
    /// How far the air `t` (per node, K) and `x` (per gas of the air and node, `[gi * na + j]`, mol) of `act` is from the
    /// air of the step's start, per node its temperature then each active gas (`[q * (1 + nx) + v]`, as `gives` reads it).
    fn moved(&self, act: &[Active], (t, x): (&[f64], &[f64])) -> Vec<f64> {
        let (outs, na) = (1 + self.nx, t.len());
        let mut d = vec![0.0; na * outs];
        for (q, dq) in d.chunks_exact_mut(outs).enumerate() {
            dq[0] = t[q] - self.t0[q];
            act.iter().enumerate().for_each(|(y, ac)| dq[1 + y] = x[ac.gi * na + q] - self.x0[y * na + q]);
        }
        d
    }
    /// What each node gets over the whole step of the outputs `os` under the air that is `d` from the step's start
    /// (`moved`), into `got` (`[j * os.len() + o - os.start]`): per node, on the pool.
    fn gives(&self, d: &[f64], os: Range<usize>, got: &mut [f64]) {
        let outs = 1 + self.nx;
        crate::par::rows(got, os.len(), || (), |_, j, row| {
            row.copy_from_slice(&self.base[j * outs + os.start..j * outs + os.end]);
            for e in self.at[j]..self.at[j + 1] {
                let (q, r) = (self.near[e] as usize * outs, e * outs * outs);
                for (g, o) in row.iter_mut().zip(os.clone()) {
                    let r = r + o * outs;
                    *g += self.slope[r] * d[q] + (1..outs).map(|v| self.slope[r + v] * d[q + v]).sum::<f64>();
                }
            }
        });
    }
}

/// What the air level does over a step: per node its mean temperature, K, and the heat it sends to space, J; per gas
/// of the air and node (`[gi * na + j]`) its mean amount, mol; per active gas and node what rains, mol, the fusion
/// enthalpy it keeps as it falls liquid, J, and the saturation ratio it rains at, a mean by rain; and what moves between
/// nodes (`Moved`).
pub(super) struct Mean {
    pub(super) t: Vec<f64>,
    pub(super) space: Vec<f64>,
    pub(super) x: Vec<f64>,
    pub(super) rain: Vec<f64>,
    pub(super) rain_fusion: Vec<f64>,
    pub(super) ratio: Vec<f64>,
    pub(super) moved: Moved,
}

/// What moves between pairs of air nodes over a step (the links between blocks, then each block's other nodes from its
/// hub): per gas of the air and pair (`[gi * pairs + p]`) mol from `a` to `b`, and per pair the heat, J.
pub(super) struct Moved {
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
    pub(super) mol: Vec<f64>,
    pub(super) heat: Vec<f64>,
}

/// The air nodes in blocks that eddies even out within a day: squares on the map of side
/// sqrt(`mix` × `BLOCK_DAY`), in whole nodes, at least one. Each block's nodes, its hub first, in the order of their
/// squares. An island smaller than a block is one block.
pub(in crate::flow) fn blocks(w: &World) -> Vec<Vec<u32>> {
    let (air, map) = (w.mesh.level(AIR), &w.map);
    let spacing = (air.area().iter().sum::<f64>() / air.n() as f64).sqrt();
    let side = w.air_scale * ((w.cfg.mix * BLOCK_DAY).sqrt() / spacing).max(1.0) as usize;
    let mut of: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
    for j in 0..air.n() {
        let (x, y) = map.xy(w.mesh.corner(air.children(j)[0] as usize));
        of.entry((y / side, x / side)).or_default().push(j as u32);
    }
    of.into_values().collect()
}

/// The air level over a step of `dt` seconds, in days: each node's exchange with its ground and its rain (`exchange`, by
/// the law `ground` of its cells), then each block evened out (`even`) and the wind across the links between blocks
/// (`wind`).
pub(super) fn run(w: &World, s: &Scratch, act: &[Active], ground: &Ground, dt: f64) -> Mean {
    let (air, ng, nx) = (w.mesh.level(AIR), s.state.gas.len(), act.len());
    let na = air.n();
    let days = (dt / DAY).ceil().max(1.0);
    let dd = dt / days;
    let (links, blocks) = (Links::of(air), blocks(w));
    let push = pushes(&w.cfg, s, air, &links);
    let mut block = vec![0; na];
    blocks.iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let one = Together::of(s, ground, (&blocks, &block), nx);
    let border: Vec<usize> = (0..links.a.len()).filter(|&l| block[links.a[l] as usize] != block[links.b[l] as usize]).collect();
    let (mut a, mut b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (links.a[l], links.b[l])).unzip();
    for nodes in &blocks {
        a.extend(std::iter::repeat_n(nodes[0], nodes.len() - 1));
        b.extend_from_slice(&nodes[1..]);
    }
    let pairs = a.len();
    let lines = lines(air, (&links, &border), (&a, &b));
    let moved = Moved { a, b, mol: vec![0.0; ng * pairs], heat: vec![0.0; pairs] };
    let v = |len: usize| vec![0.0; len];
    let mut out = Mean { t: v(na), space: v(na), x: v(ng * na), rain: v(nx * na), rain_fusion: v(nx * na), ratio: v(nx * na), moved };
    let mut x: Vec<f64> = s.state.gas.iter().flat_map(|&k| w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied()).collect();
    let mut e = w.store.heat(Class::Air).to_vec();
    let imp = Implicit::of(s, (&push, &border, &lines), (&out.moved.a, &out.moved.b), &x, dd);
    for _ in 0..days as usize {
        exchange(w, s, (act, ground), (&blocks, &one), (&mut x, &mut e), (dd, dt), &mut out);
        even(w, s, &blocks, (&mut x, &mut e), border.len(), &mut out.moved);
        wind(w, s, (&push, &border, &lines, &imp), (&mut x, &mut e), dd, &mut out.moved);
    }
    out.ratio.iter_mut().zip(&out.rain).filter(|e| *e.1 > 0.0).for_each(|(r, m)| *r /= m);
    out
}

/// The temperature of each air node with gases `x` (`[gi * na + j]`) and heat `e`, K.
pub(super) fn temps(s: &Scratch, x: &[f64], e: &[f64]) -> Vec<f64> {
    let na = e.len();
    (0..na).map(|j| air_temp(e[j], s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum())).collect()
}

/// How each block's air moves as one through a day, as the eddies even it (`exchange`): per node its share of a change
/// of its block's vapour (`Pool::v`, by volume); per active gas and block (`[y * blocks + b]`) the slope of what the
/// block's ground gives of the gas with its vapour so shared (`Pool::s`), mol/mol; and per block the slope of the heat
/// its ground gives it with its temperature, all its nodes warming alike, J/K.
struct Together {
    v: Vec<f64>,
    gas: Vec<f64>,
    heat: Vec<f64>,
}

impl Together {
    fn of(s: &Scratch, ground: &Ground, (blocks, block): (&[Vec<u32>], &[usize]), nx: usize) -> Together {
        let (na, outs, nb) = (block.len(), 1 + nx, blocks.len());
        let mut v = vec![0.0; na];
        for nodes in blocks {
            let vol: f64 = nodes.iter().map(|&j| s.vol[j as usize]).sum();
            nodes.iter().for_each(|&j| v[j as usize] = s.vol[j as usize] / vol);
        }
        let (mut gas, mut heat) = (vec![0.0; nx * nb], vec![0.0; nb]);
        for j in 0..na {
            for e in (ground.at[j]..ground.at[j + 1]).filter(|&e| block[ground.near[e] as usize] == block[j]) {
                let q = ground.near[e] as usize;
                (0..nx).for_each(|y| gas[y * nb + block[j]] += ground.slope[e * outs * outs + (1 + y) * outs + 1 + y] * v[q]);
                heat[block[j]] += ground.slope[e * outs * outs];
            }
        }
        Together { v, gas, heat }
    }
}

/// One day of `dd` seconds of each node's exchange with its ground, its heat radiation to space and its rain, its means
/// over the step of `dt` added to `out`. Each block's heat (`warmth`), then its vapour with its rain (`vapour`), follow
/// it exactly through the day, as the eddies even it, the air outside it as at the day's start. The day's air is its
/// mean over the day, and each node gets what its cells' law gives at it (`Ground::gives`: their heat radiation,
/// sensible heat and the gas they evaporate), so the book, which takes each cell's law at the step's mean air
/// (`cells::Response::booked`), gives the air just what its days did. Rain frees its vaporisation enthalpy (and its fusion
/// enthalpy where the air at the node's mean ground is below the gas's melting point: snow); the node's layers send
/// heat to space (`glow`).
fn exchange(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (blocks, one): (&[Vec<u32>], &Together), (x, e): (&mut [f64], &mut [f64]), (dd, dt): (f64, f64), out: &mut Mean) {
    let (air, na, nx, share) = (w.mesh.level(AIR), e.len(), act.len(), dd / dt);
    let t = warmth(w, s, (act, ground), (blocks, one), (x, e), (dd, dt));
    let ts: Vec<f64> = (0..na).map(|j| at_height(t[j], s.gam[j], s.za[j], s.zair[j])).collect();
    let [mean, rain, ratio] = vapour(w, s, (act, ground), (blocks, one), (x, &t, &ts), (dd, dt));
    (0..s.state.gas.len() * na).for_each(|at| out.x[at] += mean[at] * share);
    let got = given(ground, act, (&t, &mean), 0..1 + nx);
    // The air each node's rain meets (`rain_air`), per active gas and node (`[y * na + j]`), on the pool, from the air
    // touching each place it falls on.
    let touched = ground.wet().touched(s, &t, &rain);
    let met = crate::par::map_n(nx * na, |i| (rain[i] > 0.0).then(|| rain_air(ground.wet(), (i / na, i % na, ratio[i]), &touched)).flatten());
    // What each node's layers send to space over the day, J, on the pool.
    let sky = crate::par::map_n(na, |j| glow(s.absorbs[j], ts[j], &s.lay[j][1]) * air.area()[j] * dd);
    for j in 0..na {
        out.t[j] += t[j] * share;
        let (sky, day) = (sky[j], &got[j * (1 + nx)..(j + 1) * (1 + nx)]);
        e[j] += day[0] * share - sky;
        out.space[j] += sky;
        for (y, ac) in act.iter().enumerate() {
            let (at, sp, i, m) = (ac.gi * na + j, s.sp[ac.k], y * na + j, rain[y * na + j]);
            x[at] += day[1 + y] * share - m;
            if m > 0.0 {
                // The rain leaves the air frozen or liquid as the air at the node's mean ground is, at the temperature of
                // the air touching the cells it falls on (`rain_air`, as the book lands it), the air paying what warms
                // it on the way down.
                let liquid = if ts[j] < sp.melt { 0.0 } else { sp.lf };
                let tr = met[i].unwrap_or(ts[j]);
                e[j] += m * (sp.lv + sp.lf - liquid - sp.cp * tr);
                (out.rain[i], out.rain_fusion[i], out.ratio[i]) = (out.rain[i] + m, out.rain_fusion[i] + m * liquid, out.ratio[i] + ratio[i]);
            }
        }
    }
}

/// What each node's ground gives it over the step under the air `t`, K, and `x` (`Ground::gives`), of the outputs `os`
/// only (0 its heat, 1 + y its active gas `y`), `[j * os.len() + o - os.start]`.
fn given(ground: &Ground, act: &[Active], (t, x): (&[f64], &[f64]), os: Range<usize>) -> Vec<f64> {
    let (mut got, d) = (vec![0.0; t.len() * os.len()], ground.moved(act, (t, x)));
    ground.gives(&d, os, &mut got);
    got
}

/// Each node's mean temperature over a day of `dd` seconds, K, from its air `x` (`[gi * na + j]`) and heat `e`: its
/// block's heat follows what its ground gives it (`Ground::gives`) and what its layers send to space (`glow`), each
/// linear in the block's temperature through the day (`Together::heat`, `glow_slope`), all its nodes warming alike.
fn warmth(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (blocks, one): (&[Vec<u32>], &Together), (x, e): (&[f64], &[f64]), (dd, dt): (f64, f64)) -> Vec<f64> {
    let (air, na) = (w.mesh.level(AIR), e.len());
    let start = temps(s, x, e);
    let (mut t, got) = (start.clone(), given(ground, act, (&start, x), 0..1));
    // Per node what its layers send to space, W, and its slope with the temperature, W/K, on the pool.
    let sky = crate::par::map_n(na, |j| {
        let (f, lay, area, ts) = (s.absorbs[j], &s.lay[j][1], air.area()[j], at_height(start[j], s.gam[j], s.za[j], s.zair[j]));
        [glow(f, ts, lay) * area, glow_slope(f, ts, lay) * area]
    });
    for (b, nodes) in blocks.iter().enumerate() {
        let (mut rate, mut slope, mut capa) = (0.0, -one.heat[b] / dt, 0.0);
        for &j in nodes {
            let j = j as usize;
            rate += got[j] / dt - sky[j][0];
            slope += sky[j][1];
            capa += s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum::<f64>();
        }
        let per = 1.0 / capa.max(f64::MIN_POSITIVE);
        let warm = rate * per * dd * mean_passed(slope * per * dd);
        nodes.iter().for_each(|&j| t[j as usize] += warm);
    }
    t
}

/// The mean air of a day of `dd` seconds from the air `x` at its start (`[gi * na + j]`), at the day's mean
/// temperatures `t` (per node, K; `ts` at its mean ground): per block and active gas what its ground gives and what its
/// nodes rain, of what each holds over what saturates the cells that see it in `rain_time` (`Wet`), follow the block's
/// vapour exactly through the day (`Pool`). With per active gas and node (`[y * na + j]`) what it rains over the day,
/// mol, and that times the ratio it rains at.
fn vapour(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (blocks, one): (&[Vec<u32>], &Together), (x, t, ts): (&[f64], &[f64], &[f64]), (dd, dt): (f64, f64)) -> [Vec<f64>; 3] {
    let (air, na, nx, nb) = (w.mesh.level(AIR), t.len(), act.len(), blocks.len());
    let got = given(ground, act, (t, x), 1..1 + nx);
    // Per active gas and block (`[y * nb + b]`), each on the pool: the block's vapour at the day's start and its mean
    // over the day, mol, and per node of the block what it rains and that times its ratio (`Pool::day`).
    let mut days: Vec<(usize, [f64; 2], [Vec<f64>; 2])> = (0..nx * nb).map(|i| (i, [0.0; 2], Default::default())).collect();
    crate::par::each_init(&mut days, nx * na, || (), |_, (i, sums, [fell, at])| {
        let (y, b) = (*i / nb, *i % nb);
        let (ac, nodes) = (&act[y], &blocks[b]);
        let full = |j: usize| if ac.cond { (s.sat[ac.k].at(ts[j]) * air.area()[j] * s.hk[ac.gi * na + j] / (R_GAS * ts[j])).max(0.0) } else { 0.0 };
        let held = |j: u32| x[ac.gi * na + j as usize];
        let all: f64 = nodes.iter().map(|&j| held(j)).sum();
        let gives: f64 = nodes.iter().map(|&j| got[j as usize * nx + y]).sum();
        let slope = one.gas[y * nb + b];
        let v: Vec<f64> = nodes.iter().map(|&j| one.v[j as usize]).collect();
        let off = nodes.iter().zip(&v).map(|(&j, v)| held(j) - v * all).collect();
        let pool = Pool { c: gives - slope * all, s: slope, v, off, full: nodes.iter().map(|&j| full(j as usize)).collect() };
        (*fell, *at) = (vec![0.0; nodes.len()], vec![0.0; nodes.len()]);
        *sums = [all, pool.day((&ground.wet, y, nodes), all, (dd, dt, w.cfg.rain_time), (fell, at))];
    });
    let (mut mean, mut rain, mut ratio) = (x.to_vec(), vec![0.0; nx * na], vec![0.0; nx * na]);
    for (i, [all, mid], [fell, at]) in &days {
        let (y, nodes) = (i / nb, &blocks[i % nb]);
        for (q, &j) in nodes.iter().enumerate() {
            let (j, at_x) = (j as usize, act[y].gi * na + j as usize);
            mean[at_x] = x[at_x] + one.v[j] * (mid - all);
            (rain[y * na + j], ratio[y * na + j]) = (fell[q], at[q]);
        }
    }
    [mean, rain, ratio]
}

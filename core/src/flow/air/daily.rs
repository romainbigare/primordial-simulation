//! The air level day by day over an air step (laws.md 1.4 to 1.6): each node's exchange with the ground under it, its
//! rain and its heat radiation to space, then what eddies even out within a day (`blocks`) and the wind across the links
//! between blocks. Explicit, on plain arrays of the level.

use super::super::Scratch;
use super::rain::{Pool, Room, Wet};
use super::soak::{land, Soak, Soaks};
use super::stretch::Stretch;
use super::together::Together;
use super::cell::Cell;
use super::rise::{rise, saturates};
use super::edge::Edges;
use super::weather::Weather;
use super::wind::{ends, even, lines, pushes, wind, Implicit, Links};
use super::{air_temp, at_height, radiation::UP, step::Active};
use crate::{math::mean_passed, store::Class, units::{DAY, R_GAS}, World, AIR};
use std::collections::BTreeMap;

/// The time in which eddies even out a block of air, s: the air's day, so a block's side is the distance a diffusion of
/// `mix` spreads in it, sqrt(`mix` × this) (Taylor 1922), about 300 km (`blocks`).
const BLOCK_DAY: f64 = DAY;

/// Instructions of a multiply-add of `Ground::gives_of`, its products unrolled (`par::pays`).
const GIVE_WORK: usize = 7;
/// Instructions of a multiply-add of `Ground::gives_any`, its products in a loop (`par::pays`).
const GIVE_ANY_WORK: usize = 17;
/// Instructions of a node's heat radiation to space (`Skies::glow`, `par::pays`).
const GLOW_WORK: usize = 480;
/// Instructions of the vapour's day (`Pool::day`, `par::pays`) per node and active gas (its saturation and sums), about
/// 760 a raining node-day in all with the next two on a settled island.
const DAY_NODE_WORK: usize = 100;
/// Instructions of the vapour's day per node of a gas that may condense (its piece of entries, rate and rain).
const DAY_RAIN_WORK: usize = 500;
/// Instructions of the vapour's day per ratio it crosses (`rain::Break`, kept in a heap), as its last day crossed.
const DAY_EVENT_WORK: usize = 300;

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
    pub(super) soaks: Vec<Soak>,
}

impl Ground {
    pub(super) fn new(na: usize, nx: usize, (t0, x0): (Vec<f64>, Vec<f64>), wet: Wet) -> Ground {
        Ground { nx, base: vec![0.0; na * (1 + nx)], at: vec![0], near: Vec::new(), slope: Vec::new(), t0, x0, wet, soaks: Vec::new() }
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
    /// What each node gets over the whole step under the air that is `d` from the step's start (`moved`), into `got`
    /// (`[j * (1 + nx) + o]`): per node, on the pool where it pays.
    fn gives(&self, d: &[f64], got: &mut [f64]) {
        match 1 + self.nx {
            1 => self.gives_of::<1>(d, got),
            2 => self.gives_of::<2>(d, got),
            3 => self.gives_of::<3>(d, got),
            4 => self.gives_of::<4>(d, got),
            n => self.gives_any(n, d, got),
        }
    }
    /// What the other threads could take of `gives` (`par::shareable`), at `cost` instructions a multiply-add of `n`
    /// outputs per node.
    fn shareable(&self, n: usize, cost: usize) -> usize {
        crate::par::shareable(self.at.windows(2).map(|e| (e[1] - e[0]) * n * n * cost))
    }
    /// `gives` for `N` outputs per node (`1 + nx`): each node's slopes with each node it sees, and that node's air, are
    /// arrays of a size fixed at compile time, so the sums unroll (a third of the instructions of `gives_any`).
    fn gives_of<const N: usize>(&self, d: &[f64], got: &mut [f64]) {
        crate::par::rows(got, N, self.shareable(N, GIVE_WORK), || (), |_, j, row| {
            row.copy_from_slice(&self.base[j * N..j * N + N]);
            let (near, slope) = (&self.near[self.at[j]..self.at[j + 1]], &self.slope[self.at[j] * N * N..self.at[j + 1] * N * N]);
            for (&q, by) in near.iter().zip(slope.chunks_exact(N * N)) {
                let dq: &[f64; N] = d[q as usize * N..q as usize * N + N].try_into().expect("N numbers");
                for (g, s) in row.iter_mut().zip(by.chunks_exact(N)) {
                    let s: &[f64; N] = s.try_into().expect("N numbers");
                    *g += s[0] * dq[0] + (1..N).map(|v| s[v] * dq[v]).sum::<f64>();
                }
            }
        });
    }
    /// `gives` for `n` outputs per node, any count, the sums in the same order.
    fn gives_any(&self, n: usize, d: &[f64], got: &mut [f64]) {
        crate::par::rows(got, n, self.shareable(n, GIVE_ANY_WORK), || (), |_, j, row| {
            row.copy_from_slice(&self.base[j * n..j * n + n]);
            for e in self.at[j]..self.at[j + 1] {
                let (q, r) = (self.near[e] as usize * n, e * n * n);
                for (g, o) in row.iter_mut().zip(0..n) {
                    let r = r + o * n;
                    *g += self.slope[r] * d[q] + (1..n).map(|v| self.slope[r + v] * d[q + v]).sum::<f64>();
                }
            }
        });
    }
}

/// What the air level does over a step: per node its mean temperature, K, and the heat it sends to space, J; per gas
/// of the air and node (`[gi * na + j]`) its mean amount, mol; per active gas and node what rains, mol, the fusion
/// enthalpy it keeps as it falls liquid, J, and what of it lands on each place that sees the node, mol, per entry of the
/// node (`Seen::cell`; empty where it rained none); per bucket of `Ground::soaks` what the rain adds to what it
/// evaporates, mol; per node the share of its sky its clouds cover, a mean over the step; what moves between nodes
/// (`Moved`); and what crosses the map's open edges (`Edges`).
pub(super) struct Mean {
    pub(super) t: Vec<f64>,
    pub(super) space: Vec<f64>,
    pub(super) x: Vec<f64>,
    pub(super) rain: Vec<f64>,
    pub(super) rain_fusion: Vec<f64>,
    pub(super) landed: Vec<Vec<f64>>,
    pub(super) soaked: Vec<f64>,
    pub(super) cloud: Vec<f64>,
    pub(super) moved: Moved,
    pub(super) edges: Edges,
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
/// sqrt(`mix` × `BLOCK_DAY`) over the nodes' spacing as the stretched air takes it (`Stretch`), in whole nodes, at least
/// one. Each block's nodes, its hub first, in the order of their squares. An island smaller than a block is one block.
pub(in crate::flow) fn blocks(w: &World) -> Vec<Vec<u32>> {
    let air = w.mesh.level(AIR);
    let spacing = Stretch::of(w).length((air.area().iter().sum::<f64>() / air.n() as f64).sqrt());
    let side = ((w.cfg.mix * BLOCK_DAY).sqrt() / spacing).max(1.0) as usize;
    let mut of: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
    for j in 0..air.n() {
        let (x, y) = w.mesh.tree().block_xy(j);
        of.entry((y / side, x / side)).or_default().push(j as u32);
    }
    of.into_values().collect()
}

/// The air level over a step of `dt` seconds, in days: each node's exchange with its ground and its rain (`exchange`, by
/// the law `ground` of its cells), then each block evened out (`even`) and the wind across the links between blocks
/// (`wind`).
pub(super) fn run(w: &World, s: &Scratch, (act, ground, low): (&[Active], &Ground, &[[f64; 2]]), dt: f64) -> Mean {
    let (air, ng, nx) = (w.mesh.level(AIR), s.state.gas.len(), act.len());
    let na = air.n();
    let days = (dt / DAY).ceil().max(1.0);
    let dd = dt / days;
    let (links, blocks) = (Links::of(air), blocks(w));
    let (ends, cell) = (ends(w, s), Cell::of(w));
    let push = pushes(&w.cfg, Stretch::of(w), &ends, air, (&links, &cell));
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
    let edges = Edges::of(w, s, &ends, &cell);
    let rise = rise(w, s, low);
    let mut out = Mean { t: v(na), space: v(na), x: v(ng * na), rain: v(nx * na), rain_fusion: v(nx * na), landed: vec![Vec::new(); nx * na], soaked: v(ground.soaks.len()), cloud: v(na), moved, edges };
    let mut x: Vec<f64> = s.state.gas.iter().flat_map(|&k| w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied()).collect();
    let mut e = w.store.heat(Class::Air).to_vec();
    let imp = Implicit::of(s, (&push, &border, &lines), (&out.moved.a, &out.moved.b), &x, dd);
    // What each active gas's vapour in each block keeps from day to day (`[y * nb + b]`, `rain::Room`).
    let mut rooms: Vec<Room> = (0..nx * blocks.len()).map(|_| Room::default()).collect();
    // The buckets the step's rain wets as it falls (`Soak`).
    let mut soaks = Soaks::new(ground.soaks.clone(), ground.wet(), nx);
    // The weather of the step: each day it lifts the nodes' air and moves where the edges' air comes from (`Weather`).
    let weather = Weather::of(w, s, &ends, days as usize);
    for d in 0..days as usize {
        let today = weather.as_ref().map(|wx| wx.rise(w, &rise, d));
        exchange(w, s, (act, ground), (&blocks, &one, today.as_deref().unwrap_or(&rise)), (&mut x, &mut e), (dd, dt), (&mut out, &mut rooms, &mut soaks));
        even(w, s, &blocks, (&mut x, &mut e), border.len(), &mut out.moved);
        wind(w, s, (&push, &border, &lines, &imp), (&mut x, &mut e), dd, &mut out.moved);
        if let Some(wx) = &weather {
            out.edges.weather(w, s, (&ends, &cell), |xy| wx.shift(xy, d));
        }
        out.edges.day(s, (&mut x, &mut e), dd, w.cfg.gravity);
    }
    out
}

/// The temperature of each air node with gases `x` (`[gi * na + j]`) and heat `e`, K.
pub(super) fn temps(s: &Scratch, x: &[f64], e: &[f64]) -> Vec<f64> {
    let na = e.len();
    (0..na).map(|j| air_temp(e[j], s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum())).collect()
}

/// One day of `dd` seconds of each node's exchange with its ground, its heat radiation to space and its rain, its means
/// over the step of `dt` added to `out`. Each block's heat (`warmth`), then its vapour with its rain (`vapour`), follow
/// it exactly through the day, as the eddies even it, the air outside it as at the day's start. The day's air is its
/// mean over the day, and each node gets what its cells' law gives at it (`Ground::gives`: their heat radiation,
/// sensible heat and the gas they evaporate), so the book, which takes each cell's law at the step's mean air
/// (`cells::Response::booked`), gives the air just what its days did. Rain frees its vaporisation enthalpy (and its fusion
/// enthalpy where the air at the node's mean ground is below the gas's melting point: snow); the node's layers send
/// heat to space (`Heat::glow`).
fn exchange(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (blocks, one, rise): (&[Vec<u32>], &Together, &[f64]), (x, e): (&mut [f64], &mut [f64]), (dd, dt): (f64, f64), (out, rooms, soaks): (&mut Mean, &mut [Room], &mut Soaks)) {
    let (air, na, nx, share) = (w.mesh.level(AIR), e.len(), act.len(), dd / dt);
    // What the ground gives at the day's start (`given`); at the day's warmth (`Together::warmed`) as the vapour reads
    // it; at the day's mean vapour too (`Together::shared`), as the day books it: the ground's law is linear in the air.
    let start = temps(s, x, e);
    let mut got = given(ground, act, (&start, x));
    let (t, warm) = warmth(w, s, blocks, one, (x, &start, &got), (dd, dt));
    one.warmed(&mut got, &warm);
    let ts: Vec<f64> = (0..na).map(|j| at_height(t[j], s.gam[j], s.za[j], s.zair[j])).collect();
    let [mean, rain, ratio, dx] = vapour(w, s, (act, ground), (blocks, one, rise), (x, &got, &ts), (dd, dt), rooms);
    one.shared(&mut got, &dx);
    (0..s.state.gas.len() * na).for_each(|at| out.x[at] += mean[at] * share);
    // The share of each node's sky its clouds cover at the day's mean vapour (`Seen::clouded`), the most of its gases'.
    for (j, c) in out.cloud.iter_mut().enumerate() {
        let of = |(y, ac): (usize, &Active)| {
            let full = saturating(w, s, ac, j, ts[j]);
            if full > 0.0 { ground.wet().nodes[j].clouded(y, mean[ac.gi * na + j] / full, w.cfg.moist_min) } else { 0.0 }
        };
        *c += act.iter().enumerate().filter(|p| p.1.cond).map(of).fold(0.0, f64::max) * share;
    }
    // Where each node's rain lands and the air it meets there, per active gas and node (`[y * na + j]`), and what it
    // adds to what its cells evaporate (`soak::land`).
    let touched = ground.wet().touched(s, &t, &rain);
    let met = land(ground.wet(), act, (&rain, &ratio, &touched), (soaks, dd), (x, e, out));
    // What each node's layers send to space over the day, J, on the pool.
    let sky = crate::par::map_n(na, na.saturating_sub(1) * GLOW_WORK, |j| s.heat[j].sends(&w.chem, ts[j], UP) * air.area()[j] * dd);
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
                (out.rain[i], out.rain_fusion[i]) = (out.rain[i] + m, out.rain_fusion[i] + m * liquid);
            }
        }
    }
}

/// What each node's ground gives it over the step under the air `t`, K, and `x` (`Ground::gives`): per node its heat,
/// then each active gas, `[j * (1 + nx) + o]`.
fn given(ground: &Ground, act: &[Active], (t, x): (&[f64], &[f64])) -> Vec<f64> {
    let (mut got, d) = (vec![0.0; t.len() * (1 + act.len())], ground.moved(act, (t, x)));
    ground.gives(&d, &mut got);
    got
}

/// Each node's mean temperature over a day of `dd` seconds, K, from its air `x` (`[gi * na + j]`) at `start` K, with
/// what its ground gives it there (`got`, `[j * (1 + nx) + o]`), and its block's warming, K: its block's heat follows
/// what its ground gives it (`Ground::gives`) and what its layers send to space (`Heat::glow`, with its slope), each
/// linear in the block's temperature through the day (`Together::heat`), all its nodes warming alike.
fn warmth(w: &World, s: &Scratch, blocks: &[Vec<u32>], one: &Together, (x, start, got): (&[f64], &[f64], &[f64]), (dd, dt): (f64, f64)) -> (Vec<f64>, Vec<f64>) {
    let (air, na) = (w.mesh.level(AIR), start.len());
    let outs = got.len() / na.max(1);
    let (mut t, mut warms) = (start.to_vec(), Vec::with_capacity(blocks.len()));
    // Per node what its layers send to space, W, and its slope with the temperature, W/K, on the pool.
    let sky = crate::par::map_n(na, na.saturating_sub(1) * GLOW_WORK, |j| {
        let (area, ts) = (air.area()[j], at_height(start[j], s.gam[j], s.za[j], s.zair[j]));
        s.heat[j].glow(&w.chem, ts, UP).map(|x| x * area)
    });
    for (b, nodes) in blocks.iter().enumerate() {
        let (mut rate, mut slope, mut capa) = (0.0, -one.heat[b] / dt, 0.0);
        for &j in nodes {
            let j = j as usize;
            rate += got[j * outs] / dt - sky[j][0];
            slope += sky[j][1];
            capa += s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum::<f64>();
        }
        let per = 1.0 / capa.max(f64::MIN_POSITIVE);
        let warm = rate * per * dd * mean_passed(slope * per * dd);
        nodes.iter().for_each(|&j| t[j as usize] += warm);
        warms.push(warm);
    }
    (t, warms)
}

/// What saturates air node `j` of active gas `ac` at `ts` K at its mean ground, mol: the gas at its saturation pressure
/// there filling the height its amount fills (`Scratch::hk`).
fn saturating(w: &World, s: &Scratch, ac: &Active, j: usize, ts: f64) -> f64 {
    let na = w.mesh.level(AIR).n();
    (s.sat[ac.k].at(ts) * w.mesh.level(AIR).area()[j] * s.hk[ac.gi * na + j] / (R_GAS * ts)).max(0.0)
}

/// The mean air of a day of `dd` seconds from the air `x` at its start (`[gi * na + j]`), at the day's mean
/// temperatures `t` (per node, K; `ts` at its mean ground): per block and active gas what its ground gives and what its
/// nodes rain, of what each holds over what saturates the cells that see it in `rain_time` (`Wet`), follow the block's
/// vapour exactly through the day (`Pool`), from what its ground gives at the day's warmth (`got`, `[j * (1 + nx) +
/// o]`). With per active gas and node (`[y * na + j]`) what it rains over the day, mol, and that times the ratio it rains
/// at; and per active gas and block (`[y * nb + b]`) the change of its vapour from the day's start to its mean, mol.
fn vapour(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (blocks, one, rise): (&[Vec<u32>], &Together, &[f64]), (x, got, ts): (&[f64], &[f64], &[f64]), (dd, dt): (f64, f64), rooms: &mut [Room]) -> [Vec<f64>; 4] {
    let (na, nx, nb) = (ts.len(), act.len(), blocks.len());
    // Per active gas and block (`[y * nb + b]`), each on the pool: the block's vapour at the day's start and its mean
    // over the day, mol, and per node of the block what it rains and that times its ratio (`Pool::day`).
    let mut days: Vec<(usize, [f64; 2], &mut Room)> = rooms.iter_mut().enumerate().map(|(i, room)| (i, [0.0; 2], room)).collect();
    // What the other threads could take of them: per node, per node of a gas that may condense, and per ratio crossed.
    let shareable = crate::par::shareable(days.iter().map(|(i, _, room)| {
        let nodes = blocks[i % nb].len();
        nodes * DAY_NODE_WORK + if act[i / nb].cond { nodes * DAY_RAIN_WORK + room.events * DAY_EVENT_WORK } else { 0 }
    }));
    let day = |(i, sums, room): &mut (usize, [f64; 2], &mut Room)| {
        let (y, b) = (*i / nb, *i % nb);
        let (ac, nodes) = (&act[y], &blocks[b]);
        // What saturates each node as the ground lifts its air or lets it sink (`rise::saturates`), its lower half from
        // its air now.
        let lower = |j: usize| (0..s.state.gas.len()).map(|q| (1.0 - s.split[q * na + j]) * x[q * na + j]).sum::<f64>();
        let full = |j: usize| if ac.cond { saturates(saturating(w, s, ac, j, ts[j]), rise[j], lower(j), s.split[ac.gi * na + j], w.cfg.rain_time) } else { 0.0 };
        let held = |j: u32| x[ac.gi * na + j as usize];
        let all: f64 = nodes.iter().map(|&j| held(j)).sum();
        let gives: f64 = nodes.iter().map(|&j| got[j as usize * (1 + nx) + 1 + y]).sum();
        let slope = one.gas[y * nb + b];
        let v: Vec<f64> = nodes.iter().map(|&j| one.v[j as usize]).collect();
        let off = nodes.iter().zip(&v).map(|(&j, v)| held(j) - v * all).collect();
        let pool = Pool { c: gives - slope * all, s: slope, v, off, full: nodes.iter().map(|&j| full(j as usize)).collect() };
        *sums = [all, pool.day((&ground.wet, y, nodes), all, (dd, dt, w.cfg.rain_time), room)];
    };
    if crate::par::pays(shareable) {
        crate::par::each_init(&mut days, nx * na, || (), |_, d| day(d));
    } else {
        days.iter_mut().for_each(day);
    }
    let (mut mean, mut rain, mut ratio, mut dx) = (x.to_vec(), vec![0.0; nx * na], vec![0.0; nx * na], vec![0.0; nx * nb]);
    for (i, [all, mid], room) in &days {
        dx[*i] = mid - all;
        let (y, nodes) = (i / nb, &blocks[i % nb]);
        for (q, &j) in nodes.iter().enumerate() {
            let (j, at_x) = (j as usize, act[y].gi * na + j as usize);
            mean[at_x] = x[at_x] + one.v[j] * (mid - all);
            (rain[y * na + j], ratio[y * na + j]) = (room.rain[q], room.ratio[q]);
        }
    }
    [mean, rain, ratio, dx]
}

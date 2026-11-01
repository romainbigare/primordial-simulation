//! The air level day by day over an air step (laws.md 1.4 to 1.6): each node's column (heat radiation, the exchange
//! with the ground under it, rain), then what eddies even out within a day (`blocks`) and the wind across the links
//! between blocks. Explicit, on plain arrays of the level.

use super::super::Scratch;
use super::{air_temp, radiation::glow, step::Active, wind::{pushes, Links, Pushes}};
use crate::{math::one_minus_exp, store::Class, units::{DAY, R_GAS}, World, AIR};
use std::collections::BTreeMap;

/// What the ground under each air node gives it over a step, from its cells at the step's start (`cells::forcing`):
/// per node the heat radiation of the ground its air absorbs, W, and the sensible heat `warm - touch × t`, `t` the
/// temperature of its air at its mean ground, W and W/K; per active gas and node (`[x * na + j]`) what evaporates,
/// `open - close × t × amount`, mol/s and 1/(K s), and the heat a mol of it brings the air, J/mol (its heat at the
/// ground's temperature less its lift); and where the node's cells rain (`Wet`).
pub(super) struct Forcing {
    pub(super) up: Vec<f64>,
    pub(super) warm: Vec<f64>,
    pub(super) touch: Vec<f64>,
    pub(super) open: Vec<f64>,
    pub(super) close: Vec<f64>,
    pub(super) brings: Vec<f64>,
    pub(super) wet: Wet,
}

/// The saturation ratio of an air node (its amount of a gas over what saturates it at its mean ground) at which each
/// of its cells rains, as of the step's start: per active gas and cell (`[x * n + i]`) that ratio; and per active gas
/// and node its cells `at[j]..at[j + 1]` (`[x * n + ..]`) in rising order of it, each `[ratio, its share of the node's
/// area and those before, Σ share × ratio so far]`.
pub(super) struct Wet {
    pub(super) ratio: Vec<f64>,
    pub(super) at: Vec<usize>,
    pub(super) steps: Vec<[f64; 3]>,
}

impl Wet {
    /// What node `j` of `n` cells holds of active gas `x` over what saturates its cells, at its saturation ratio `r`,
    /// as a share of what saturates it at its mean ground (the per-cell rain of laws.md 1.5, summed).
    fn over(&self, (x, n): (usize, usize), j: usize, r: f64) -> f64 {
        let cells = &self.steps[x * n + self.at[j]..x * n + self.at[j + 1]];
        cells.iter().take_while(|c| c[0] < r).last().map_or(0.0, |c| r * c[1] - c[2])
    }
}

/// What the air level does over a step: per node its mean temperature, K, and the heat it sends to space, J; per gas
/// of the air and node (`[gi * na + j]`) its mean amount, mol; per active gas and node what rains, mol, and the
/// saturation ratio it rains at, a mean by rain; and what moves between nodes (`Moved`).
pub(super) struct Mean {
    pub(super) t: Vec<f64>,
    pub(super) space: Vec<f64>,
    pub(super) x: Vec<f64>,
    pub(super) rain: Vec<f64>,
    pub(super) ratio: Vec<f64>,
    pub(super) moved: Moved,
}

/// What moves between pairs of air nodes over a step (the links between blocks, then each block's other nodes from its
/// hub): per gas of the air and pair (`[gi * pairs + p]`) mol from `a` to `b`, and per pair the heat, J; and per node
/// the lift its heat pays for the mass that enters it, J (a fall gives it back).
pub(super) struct Moved {
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
    pub(super) mol: Vec<f64>,
    pub(super) heat: Vec<f64>,
    pub(super) paid: Vec<f64>,
}

/// The air nodes in blocks that eddies even out within a day of `dd` seconds: squares on the map of the distance a
/// diffusion of `mix` spreads in that time, sqrt(`mix` × `dd`), in whole nodes, at least one (Taylor 1922). Each block's
/// nodes, its hub first, in the order of their squares.
fn blocks(w: &World, dd: f64) -> Vec<Vec<u32>> {
    let (air, map) = (w.mesh.level(AIR), &w.map);
    let spacing = (air.area().iter().sum::<f64>() / air.n() as f64).sqrt();
    let side = w.air_scale * ((w.cfg.mix * dd).sqrt() / spacing).max(1.0) as usize;
    let mut of: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
    for j in 0..air.n() {
        let (x, y) = map.xy(map.square_of(air.children(j)[0] as usize));
        of.entry((y / side, x / side)).or_default().push(j as u32);
    }
    of.into_values().collect()
}

/// The air level over a step of `dt` seconds, in days: each node's exchange with its ground (`exchange`) under the
/// ground's forcing `f`, then each block evened out (`even`), the wind across the links between blocks (`wind`), and
/// the rain (`rain`).
pub(super) fn run(w: &World, s: &Scratch, act: &[Active], f: &Forcing, dt: f64) -> Mean {
    let (air, ng, nx) = (w.mesh.level(AIR), s.state.gas.len(), act.len());
    let na = air.n();
    let days = (dt / DAY).ceil().max(1.0);
    let dd = dt / days;
    let (links, blocks) = (Links::of(air), blocks(w, dd));
    let push = pushes(&w.cfg, s, air, &links);
    let mut block = vec![0; na];
    blocks.iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let border: Vec<usize> = (0..links.a.len()).filter(|&l| block[links.a[l] as usize] != block[links.b[l] as usize]).collect();
    let (mut a, mut b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (links.a[l], links.b[l])).unzip();
    for nodes in &blocks {
        nodes[1..].iter().for_each(|&m| (a.push(nodes[0]), b.push(m)).1);
    }
    let pairs = a.len();
    let moved = Moved { a, b, mol: vec![0.0; ng * pairs], heat: vec![0.0; pairs], paid: vec![0.0; na] };
    let mut out = Mean { t: vec![0.0; na], space: vec![0.0; na], x: vec![0.0; ng * na], rain: vec![0.0; nx * na], ratio: vec![0.0; nx * na], moved };
    let mut x: Vec<f64> = s.state.gas.iter().flat_map(|&k| w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied()).collect();
    let mut e = w.store.heat(Class::Air).to_vec();
    for _ in 0..days as usize {
        exchange(w, s, (act, f), (&mut x, &mut e), (dd, dt), &mut out);
        even(w, s, &blocks, (&mut x, &mut e), border.len(), &mut out.moved);
        wind(w, s, (&push, &links, &border), (&mut x, &mut e), dd, &mut out.moved);
        rain(w, s, (act, f), (&mut x, &mut e), dd, &mut out);
    }
    out.ratio.iter_mut().zip(&out.rain).filter(|e| *e.1 > 0.0).for_each(|(r, m)| *r /= m);
    out
}

/// The temperature of each air node with gases `x` (`[gi * na + j]`) and heat `e`, K.
fn temps(s: &Scratch, x: &[f64], e: &[f64]) -> Vec<f64> {
    let na = e.len();
    (0..na).map(|j| air_temp(e[j], s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum())).collect()
}

/// One day of `dd` seconds of each node's exchange with its ground, its means over the step of `dt` added to `out`:
/// the heat radiation its ground sends up that it absorbs (`Forcing::up`), less what its layers send down and to space
/// (`glow`), with the sensible heat of its ground; and what evaporates, toward the balance with its ground.
fn exchange(w: &World, s: &Scratch, (act, f): (&[Active], &Forcing), (x, e): (&mut [f64], &mut [f64]), (dd, dt): (f64, f64), out: &mut Mean) {
    let (air, na) = (w.mesh.level(AIR), e.len());
    let t = temps(s, x, e);
    for j in 0..na {
        out.t[j] += t[j] * dd / dt;
        (0..s.state.gas.len()).for_each(|q| out.x[q * na + j] += x[q * na + j] * dd / dt);
        let (area, ts) = (air.area()[j], surface_air(s, t[j], j));
        let (down, sky) = (glow(s.absorbs[j], ts, &s.lay[j][0]) * area, glow(s.absorbs[j], ts, &s.lay[j][1]) * area);
        e[j] += dd * (f.up[j] + f.warm[j] - f.touch[j] * ts - down - sky);
        out.space[j] += dd * sky;
        for (q, ac) in act.iter().enumerate().filter(|a| a.1.up) {
            let (at, y) = (ac.gi * na + j, q * na + j);
            let (open, close) = (f.open[y], f.close[y] * ts);
            let m = if close > 0.0 { (open / close - x[at]) * one_minus_exp(dd * close) } else { open * dd };
            (x[at], e[j]) = (x[at] + m, e[j] + m * f.brings[y]);
        }
    }
}

/// The temperature of the air of node `j` at its mean ground, its air at `t`, K.
fn surface_air(s: &Scratch, t: f64, j: usize) -> f64 {
    (t - s.gam[j] * (s.za[j] - s.zair[j])).max(1.0)
}

/// One day of `dd` seconds of rain: what each node holds of a gas over what saturates its cells rains out in
/// `rain_time` (`Wet::over`), freeing its vaporisation enthalpy (and its fusion enthalpy where the air at its ground is
/// below the gas's melting point: snow), its fall heating the air. It comes after the day's wind, as the eddies carry
/// vapour across a block in hours and rain takes a day.
fn rain(w: &World, s: &Scratch, (act, f): (&[Active], &Forcing), (x, e): (&mut [f64], &mut [f64]), dd: f64, out: &mut Mean) {
    let (air, n, na, g) = (w.mesh.level(AIR), w.mesh.cells().n(), e.len(), w.cfg.gravity);
    let (rains, t) = (one_minus_exp(dd / w.cfg.rain_time), temps(s, x, e));
    for j in 0..na {
        let ts = surface_air(s, t[j], j);
        for (q, ac) in act.iter().enumerate().filter(|a| a.1.cond) {
            let (at, sp, y) = (ac.gi * na + j, s.sp[ac.k], q * na + j);
            let full = s.sat[ac.k].at(ts) * air.area()[j] * s.hk[at] / (R_GAS * ts);
            if full <= 0.0 {
                continue;
            }
            let r = x[at] / full;
            let m = (rains * full * f.wet.over((q, n), j, r)).min(x[at]);
            if m > 0.0 {
                let snow = if ts < sp.melt { sp.lf } else { 0.0 };
                (x[at], e[j]) = (x[at] - m, e[j] + m * (sp.lv + snow - sp.cp * t[j] + sp.mm * g * (s.zair[j] - s.za[j])));
                (out.rain[y], out.ratio[y]) = (out.rain[y] + m, out.ratio[y] + m * r);
            }
        }
    }
}

/// Each block's air evened out: every gas and the heat of each node in proportion to its volume, so its pressure,
/// shares and temperature are the block's. What moves goes from the block's hub to each other node, pairs `first`
/// on (`Moved`); the node the mass enters pays its lift.
fn even(w: &World, s: &Scratch, blocks: &[Vec<u32>], (x, e): (&mut [f64], &mut [f64]), first: usize, moved: &mut Moved) {
    let (na, ng, pairs, g) = (e.len(), s.state.gas.len(), moved.heat.len(), w.cfg.gravity);
    let mut p = first;
    for nodes in blocks.iter().filter(|b| b.len() > 1) {
        let (hub, vol) = (nodes[0] as usize, nodes.iter().map(|&j| s.vol[j as usize]).sum::<f64>());
        let gas: Vec<f64> = (0..ng).map(|q| nodes.iter().map(|&j| x[q * na + j as usize]).sum()).collect();
        let heat: f64 = nodes.iter().map(|&j| e[j as usize]).sum();
        for &m in &nodes[1..] {
            let (j, share) = (m as usize, s.vol[m as usize] / vol);
            let mut kg = 0.0;
            for (q, &k) in s.state.gas.iter().enumerate() {
                let d = gas[q] * share - x[q * na + j];
                (x[q * na + j], x[q * na + hub], moved.mol[q * pairs + p], kg) = (x[q * na + j] + d, x[q * na + hub] - d, moved.mol[q * pairs + p] + d, kg + d * s.sp[k].mm);
            }
            let d = heat * share - e[j];
            (e[j], e[hub], moved.heat[p]) = (e[j] + d, e[hub] - d, moved.heat[p] + d);
            lift((hub, j), kg, (s, g), e, &mut moved.paid);
            p += 1;
        }
    }
}

/// `kg` of mass moves from node `a` to node `b` (back where negative): the node it enters pays its lift from its heat.
fn lift((a, b): (usize, usize), kg: f64, (s, g): (&Scratch, f64), e: &mut [f64], paid: &mut [f64]) {
    let (from, to) = if kg > 0.0 { (a, b) } else { (b, a) };
    let q = g * kg.abs() * (s.zair[to] - s.zair[from]);
    (e[to], paid[to]) = (e[to] - q, paid[to] + q);
}

/// One day of `dd` seconds of the wind over the links between blocks (`border`), each from the air at the day's start:
/// the evening of pressure, the eddies and the overturning, each at most what evens its link in the day, and together
/// at most what a node holds (each link's share cut by the busier of its nodes), so the day is stable for any rates.
/// What crosses is the sender's air (by halves for the overturning: the lower half to the warmer node, the upper half
/// back, each with its node's heat per mol); the node the mass enters pays its lift.
fn wind(w: &World, s: &Scratch, (push, links, border): (&Pushes, &Links, &[usize]), (x, e): (&mut [f64], &mut [f64]), dd: f64, moved: &mut Moved) {
    let (na, ng, pairs, g) = (e.len(), s.state.gas.len(), moved.heat.len(), w.cfg.gravity);
    if border.is_empty() {
        return;
    }
    let t = temps(s, x, e);
    let sum = |f: &dyn Fn(usize, usize) -> f64, j: usize| (0..ng).map(|q| f(q, j)).sum::<f64>();
    let nt: Vec<f64> = (0..na).map(|j| sum(&|q, j| x[q * na + j], j)).collect();
    let upper: Vec<f64> = (0..na).map(|j| sum(&|q, j| x[q * na + j] * s.split[q * na + j], j)).collect();
    let lower: Vec<f64> = (0..na).map(|j| nt[j] - upper[j]).collect();
    let c: Vec<f64> = (0..na).map(|j| R_GAS * t[j] / s.vol[j]).collect();
    // Each link's share of what evens it in the day, by pressure, eddies and overturning, and each node's sum of them.
    let mut busy = vec![0.0; na];
    let shares: Vec<[f64; 3]> = border
        .iter()
        .map(|&l| {
            let (ia, ib, o) = (links.a[l] as usize, links.b[l] as usize, push.over[l]);
            let (from, to) = if o > 0.0 { (ia, ib) } else { (ib, ia) };
            let even = (dd * push.even[l] * (c[ia] + c[ib])).min(1.0);
            let mix = (dd * push.mix[l] * (1.0 / nt[ia] + 1.0 / nt[ib])).min(1.0);
            let over = (dd * o.abs() / lower[from].min(upper[to]).max(f64::MIN_POSITIVE)).min(1.0);
            [ia, ib].iter().for_each(|&j| busy[j] += even + mix + over);
            [even, mix, over]
        })
        .collect();
    let (x0, e0, mut d) = (x.to_vec(), e.to_vec(), vec![0.0; ng + 1]);
    for (p, (&l, [even, mix, over])) in border.iter().zip(shares).enumerate() {
        let (ia, ib, o) = (links.a[l] as usize, links.b[l] as usize, push.over[l]);
        let cut = 1.0 / busy[ia].max(busy[ib]).max(1.0);
        let (from, to, sign) = if o > 0.0 { (ia, ib, 1.0) } else { (ib, ia, -1.0) };
        // The air that evens the pressure (from the higher), what the eddies swap each way, and each half overturned.
        let push_mol = cut * even * (nt[ia] * c[ia] - nt[ib] * c[ib]) / (c[ia] + c[ib]);
        let sender = if push_mol > 0.0 { ia } else { ib };
        let (swap, turned) = (cut * mix / (1.0 / nt[ia] + 1.0 / nt[ib]), sign * cut * over * lower[from].min(upper[to]));
        let mut kg = 0.0;
        for (q, &k) in s.state.gas.iter().enumerate() {
            let at = |j: usize| x0[q * na + j];
            let halves = at(from) * (1.0 - s.split[q * na + from]) / lower[from] - at(to) * s.split[q * na + to] / upper[to];
            d[q] = push_mol * at(sender) / nt[sender] + swap * (at(ia) / nt[ia] - at(ib) / nt[ib]) + turned * halves;
            kg += d[q] * s.sp[k].mm;
        }
        let per = |j: usize| e0[j] / nt[j];
        d[ng] = push_mol * per(sender) + swap * (per(ia) - per(ib)) + turned * (per(from) - per(to));
        for q in 0..ng {
            (x[q * na + ia], x[q * na + ib], moved.mol[q * pairs + p]) = (x[q * na + ia] - d[q], x[q * na + ib] + d[q], moved.mol[q * pairs + p] + d[q]);
        }
        (e[ia], e[ib], moved.heat[p]) = (e[ia] - d[ng], e[ib] + d[ng], moved.heat[p] + d[ng]);
        lift((ia, ib), kg, (s, g), e, &mut moved.paid);
    }
}

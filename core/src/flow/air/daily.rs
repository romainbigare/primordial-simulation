//! The air level day by day over an air step (laws.md 1.4 to 1.6): each node's exchange with the ground under it and its
//! heat radiation to space, then what eddies even out within a day (`blocks`), the wind across the links between
//! blocks, and rain. Explicit, on plain arrays of the level.

use super::super::Scratch;
use super::{air_temp, at_height, radiation::glow, step::Active, wind::{pushes, Links, Pushes}};
use crate::{math::one_minus_exp, solve::sweep::{within, HALF_WAY}, store::Class, units::{DAY, R_GAS}, World, AIR};
use std::collections::BTreeMap;

/// The time in which eddies even out a block of air, s: the air's day, so a block's side is the distance a diffusion of
/// `mix` spreads in it, sqrt(`mix` × this) (Taylor 1922), about 300 km (`blocks`).
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
    /// What node `j` gets over the whole step under the air `t` (per node, K) and `x` (per gas of the air and node,
    /// `[gi * na + j]`, mol) of `act`, into `out` (`[o]`, as `base`).
    fn gives(&self, j: usize, act: &[Active], (t, x): (&[f64], &[f64]), out: &mut [f64]) {
        let (outs, na) = (1 + self.nx, t.len());
        out.copy_from_slice(&self.base[j * outs..(j + 1) * outs]);
        for e in self.at[j]..self.at[j + 1] {
            let q = self.near[e] as usize;
            let slope = &self.slope[e * outs * outs..(e + 1) * outs * outs];
            let dt = t[q] - self.t0[q];
            for (o, v) in out.iter_mut().enumerate() {
                *v += slope[o * outs] * dt + act.iter().enumerate().map(|(y, ac)| slope[o * outs + 1 + y] * (x[ac.gi * na + q] - self.x0[y * na + q])).sum::<f64>();
            }
        }
    }
}

/// Where each air node's rain falls, as of the step's start (`cells::wet`): per node its entries `at[j]..at[j + 1]`,
/// each a cell that sees it, the share of the node's air over that cell, and per active gas the node's saturation ratio
/// (its amount over what saturates it at its mean ground) past which the cell rains; and per active gas and node the
/// entries in rising order of that ratio, each `[ratio, Σ share so far, Σ share × ratio so far]`.
pub(super) struct Wet {
    pub(super) at: Vec<usize>,
    pub(super) cell: Vec<u32>,
    pub(super) share: Vec<f64>,
    pub(super) ratio: Vec<Vec<f64>>,
    steps: Vec<Vec<[f64; 3]>>,
}

impl Wet {
    /// From each node's entries: the cell, its share, and its ratio per active gas (`nx` of them).
    pub(super) fn new(nx: usize, by: Vec<Vec<(u32, f64, Vec<f64>)>>) -> Wet {
        let mut w = Wet { at: vec![0], cell: Vec::new(), share: Vec::new(), ratio: vec![Vec::new(); nx], steps: vec![Vec::new(); nx] };
        for node in by {
            for (y, steps) in w.steps.iter_mut().enumerate() {
                let mut order: Vec<(f64, f64)> = node.iter().map(|e| (e.2[y], e.1)).collect();
                order.sort_by(|p, q| p.0.total_cmp(&q.0));
                let mut sum = [0.0; 2];
                for (r, share) in order {
                    sum = [sum[0] + share, sum[1] + share * r];
                    steps.push([r, sum[0], sum[1]]);
                }
            }
            for (cell, share, ratio) in node {
                w.cell.push(cell);
                w.share.push(share);
                ratio.into_iter().enumerate().for_each(|(y, r)| w.ratio[y].push(r));
            }
            w.at.push(w.cell.len());
        }
        w
    }
    /// What node `j` holds of active gas `x` over what saturates the cells that see it, at its saturation ratio `r`, as a
    /// share of what saturates it at its mean ground (the per-cell rain of laws.md 1.5, summed).
    fn over(&self, x: usize, j: usize, r: f64) -> f64 {
        let steps = &self.steps[x][self.at[j]..self.at[j + 1]];
        match steps.partition_point(|c| c[0] < r) {
            0 => 0.0,
            k => r * steps[k - 1][1] - steps[k - 1][2],
        }
    }
}

/// What the air level does over a step: per node its mean temperature, K, and the heat it sends to space, J; per gas
/// of the air and node (`[gi * na + j]`) its mean amount, mol; per active gas and node what rains, mol, the heat the
/// rain brings the ground, J, and the saturation ratio it rains at, a mean by rain; per node the energy rain's fall
/// gives its air, J; and what moves between nodes (`Moved`).
pub(super) struct Mean {
    pub(super) t: Vec<f64>,
    pub(super) space: Vec<f64>,
    pub(super) x: Vec<f64>,
    pub(super) rain: Vec<f64>,
    pub(super) rain_heat: Vec<f64>,
    pub(super) ratio: Vec<f64>,
    pub(super) fall: Vec<f64>,
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

/// The air nodes in blocks that eddies even out within a day: squares on the map of side
/// sqrt(`mix` × `BLOCK_DAY`), in whole nodes, at least one. Each block's nodes, its hub first, in the order of their
/// squares. An island smaller than a block is one block.
fn blocks(w: &World) -> Vec<Vec<u32>> {
    let (air, map) = (w.mesh.level(AIR), &w.map);
    let spacing = (air.area().iter().sum::<f64>() / air.n() as f64).sqrt();
    let side = w.air_scale * ((w.cfg.mix * BLOCK_DAY).sqrt() / spacing).max(1.0) as usize;
    let mut of: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
    for j in 0..air.n() {
        let (x, y) = map.xy(map.square_of(air.children(j)[0] as usize));
        of.entry((y / side, x / side)).or_default().push(j as u32);
    }
    of.into_values().collect()
}

/// The air level over a step of `dt` seconds, in days: each node's exchange with its ground (`exchange`, by the law
/// `ground` of its cells), then each block evened out (`even`), the wind across the links between blocks (`wind`), and
/// the rain (`rain`).
pub(super) fn run(w: &World, s: &Scratch, act: &[Active], ground: &Ground, dt: f64) -> Mean {
    let (air, ng, nx) = (w.mesh.level(AIR), s.state.gas.len(), act.len());
    let na = air.n();
    let days = (dt / DAY).ceil().max(1.0);
    let dd = dt / days;
    let (links, blocks) = (Links::of(air), blocks(w));
    let push = pushes(&w.cfg, s, air, &links);
    let mut block = vec![0; na];
    blocks.iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let border: Vec<usize> = (0..links.a.len()).filter(|&l| block[links.a[l] as usize] != block[links.b[l] as usize]).collect();
    let (mut a, mut b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (links.a[l], links.b[l])).unzip();
    for nodes in &blocks {
        a.extend(std::iter::repeat_n(nodes[0], nodes.len() - 1));
        b.extend_from_slice(&nodes[1..]);
    }
    let pairs = a.len();
    let moved = Moved { a, b, mol: vec![0.0; ng * pairs], heat: vec![0.0; pairs], paid: vec![0.0; na] };
    let v = |len: usize| vec![0.0; len];
    let mut out = Mean { t: v(na), space: v(na), x: v(ng * na), rain: v(nx * na), rain_heat: v(nx * na), ratio: v(nx * na), fall: v(na), moved };
    let mut x: Vec<f64> = s.state.gas.iter().flat_map(|&k| w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied()).collect();
    let mut e = w.store.heat(Class::Air).to_vec();
    for _ in 0..days as usize {
        exchange(w, s, (act, ground), (&mut x, &mut e), (dd, dt), &mut out);
        even(w, s, &blocks, (&mut x, &mut e), border.len(), &mut out.moved);
        wind(w, s, (&push, &links, &border), (&mut x, &mut e), dd, &mut out.moved);
        rain(w, s, (act, &ground.wet), (&mut x, &mut e), dd, &mut out);
    }
    out.ratio.iter_mut().zip(&out.rain).filter(|e| *e.1 > 0.0).for_each(|(r, m)| *r /= m);
    out
}

/// The temperature of each air node with gases `x` (`[gi * na + j]`) and heat `e`, K.
fn temps(s: &Scratch, x: &[f64], e: &[f64]) -> Vec<f64> {
    let na = e.len();
    (0..na).map(|j| air_temp(e[j], s.state.gas.iter().enumerate().map(|(q, &k)| x[q * na + j] * s.sp[k].cp).sum())).collect()
}

/// One day of `dd` seconds of each node's exchange with its ground, its means over the step of `dt` added to `out`: the
/// day's share of what its cells' law gives it at the day's air (`Ground::gives`: their heat radiation, sensible heat
/// and the gas they evaporate), and the heat its layers send to space (`glow`). The day's air is the mean's, so the
/// book, which takes each cell's law at the mean air (`cells::Response::at`), gives the air just what its days did.
fn exchange(w: &World, s: &Scratch, (act, ground): (&[Active], &Ground), (x, e): (&mut [f64], &mut [f64]), (dd, dt): (f64, f64), out: &mut Mean) {
    let (air, na, share) = (w.mesh.level(AIR), e.len(), dd / dt);
    let t = temps(s, x, e);
    (0..s.state.gas.len() * na).for_each(|at| out.x[at] += x[at] * share);
    let (x0, mut got) = (x.to_vec(), vec![0.0; 1 + act.len()]);
    for j in 0..na {
        out.t[j] += t[j] * share;
        let ts = at_height(t[j], s.gam[j], s.za[j], s.zair[j]);
        let sky = glow(s.absorbs[j], ts, &s.lay[j][1]) * air.area()[j] * dd;
        ground.gives(j, act, (&t, &x0), &mut got);
        e[j] += got[0] * share - sky;
        out.space[j] += sky;
        act.iter().enumerate().for_each(|(y, ac)| x[ac.gi * na + j] += got[1 + y] * share);
    }
}

/// One day of `dd` seconds of rain: what each node holds of a gas over what saturates the cells that see it rains out
/// in `rain_time` (`Wet::over`), freeing its vaporisation enthalpy (and its fusion enthalpy where the air at its mean
/// ground is below the gas's melting point: snow), its fall heating the air. It comes after the day's wind, as the
/// eddies carry vapour across a block in hours and rain takes a day.
fn rain(w: &World, s: &Scratch, (act, wet): (&[Active], &Wet), (x, e): (&mut [f64], &mut [f64]), dd: f64, out: &mut Mean) {
    let (air, na, g) = (w.mesh.level(AIR), e.len(), w.cfg.gravity);
    let (rains, t) = (one_minus_exp(dd / w.cfg.rain_time), temps(s, x, e));
    for j in 0..na {
        let ts = at_height(t[j], s.gam[j], s.za[j], s.zair[j]);
        for (y, ac) in act.iter().enumerate().filter(|a| a.1.cond) {
            let (at, sp, i) = (ac.gi * na + j, s.sp[ac.k], y * na + j);
            let full = s.sat[ac.k].at(ts) * air.area()[j] * s.hk[at] / (R_GAS * ts);
            if full <= 0.0 {
                continue;
            }
            let r = x[at] / full;
            let m = (rains * full * wet.over(y, j, r)).min(x[at]);
            if m > 0.0 {
                // The rain leaves the air at its temperature, frozen or liquid, and its fall heats the air.
                let (liquid, fall) = (if ts < sp.melt { 0.0 } else { sp.lf }, m * sp.mm * g * (s.zair[j] - s.za[j]));
                x[at] -= m;
                e[j] += m * (sp.lv + sp.lf - liquid - sp.cp * t[j]) + fall;
                (out.rain[i], out.rain_heat[i], out.ratio[i]) = (out.rain[i] + m, out.rain_heat[i] + m * (sp.cp * t[j] + liquid), out.ratio[i] + m * r);
                out.fall[j] += fall;
            }
        }
    }
}

/// Each block's air evened out: every gas of each node in proportion to its volume, and the heat so that the warmth of
/// each (its temperature at its mean height, raised to a common height by the block's drop with height, as the wind's
/// overturning reads it, `wind::pushes`) is the block's; so its pressure, shares and warmth are the block's. What
/// moves goes from the block's hub to each other node, pairs `first` on (`Moved`); the node the mass enters pays its
/// lift.
fn even(w: &World, s: &Scratch, blocks: &[Vec<u32>], (x, e): (&mut [f64], &mut [f64]), first: usize, moved: &mut Moved) {
    let (na, ng, pairs, g) = (e.len(), s.state.gas.len(), moved.heat.len(), w.cfg.gravity);
    let mut p = first;
    for nodes in blocks.iter().filter(|b| b.len() > 1) {
        let (hub, vol) = (nodes[0] as usize, nodes.iter().map(|&j| s.vol[j as usize]).sum::<f64>());
        let gas: Vec<f64> = (0..ng).map(|q| nodes.iter().map(|&j| x[q * na + j as usize]).sum()).collect();
        let heat: f64 = nodes.iter().map(|&j| e[j as usize]).sum();
        // Each node's heat capacity once evened, J/K; the block's drop with height, K/m, and its warmth, K: its heat with
        // what that drop takes from its mean height, over its capacity.
        let capa = |j: usize| s.state.gas.iter().enumerate().map(|(q, &k)| gas[q] * s.vol[j] / vol * s.sp[k].cp).sum::<f64>();
        let all = nodes.iter().map(|&j| capa(j as usize)).sum::<f64>().max(f64::MIN_POSITIVE);
        let gam = nodes.iter().map(|&j| capa(j as usize) * s.gam[j as usize]).sum::<f64>() / all;
        let warmth = (heat + nodes.iter().map(|&j| capa(j as usize) * gam * s.zair[j as usize]).sum::<f64>()) / all;
        for &m in &nodes[1..] {
            let (j, share) = (m as usize, s.vol[m as usize] / vol);
            let mut kg = 0.0;
            for (q, &k) in s.state.gas.iter().enumerate() {
                let d = gas[q] * share - x[q * na + j];
                x[q * na + j] += d;
                x[q * na + hub] -= d;
                moved.mol[q * pairs + p] += d;
                kg += d * s.sp[k].mm;
            }
            let d = capa(j) * (warmth - gam * s.zair[j]) - e[j];
            e[j] += d;
            e[hub] -= d;
            moved.heat[p] += d;
            lift((hub, j), kg, (s, g), e, &mut moved.paid);
            p += 1;
        }
    }
}

/// `kg` of mass moves from node `a` to node `b` (back where negative): the node it enters pays its lift from its heat.
fn lift((a, b): (usize, usize), kg: f64, (s, g): (&Scratch, f64), e: &mut [f64], paid: &mut [f64]) {
    let (from, to) = if kg > 0.0 { (a, b) } else { (b, a) };
    let q = g * kg.abs() * (s.zair[to] - s.zair[from]);
    e[to] -= q;
    paid[to] += q;
}

/// One day of `dd` seconds of the wind over the links between blocks (`border`), each from the air at the day's start:
/// the evening of pressure, the eddies and the overturning, each at most what evens its link in the day; each link's
/// pressure at most half of that, and its eddies with its overturning at most all of it, over the links of the busier
/// of its nodes, so the day is stable for any rates while a link far from that limit keeps its own; and no node gives
/// more of a gas than it holds (`solve::sweep::within`).
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
    // Each link's share of what evens it in the day, by pressure, eddies and overturning; and each node's links.
    let mut links_of = vec![0usize; na];
    border.iter().for_each(|&l| [links.a[l], links.b[l]].iter().for_each(|&j| links_of[j as usize] += 1));
    let shares: Vec<[f64; 3]> = border
        .iter()
        .map(|&l| {
            let (ia, ib, o) = (links.a[l] as usize, links.b[l] as usize, push.over[l]);
            let (from, to) = if o > 0.0 { (ia, ib) } else { (ib, ia) };
            let even = (dd * push.even[l] * (c[ia] + c[ib])).min(1.0);
            let mix = (dd * push.mix[l] * (1.0 / nt[ia] + 1.0 / nt[ib])).min(1.0);
            let over = (dd * o.abs() / lower[from].min(upper[to]).max(f64::MIN_POSITIVE)).min(1.0);
            [even, mix, over]
        })
        .collect();
    let (x0, e0, width) = (x.to_vec(), e.to_vec(), ng + 1);
    // What crosses each link, of each gas and the heat (`[p * width + q]`), mol and J, + from its `a` to its `b`.
    let mut flow = vec![0.0; border.len() * width];
    for (p, (&l, [even, mix, over])) in border.iter().zip(shares).enumerate() {
        let (ia, ib, o) = (links.a[l] as usize, links.b[l] as usize, push.over[l]);
        let busy = links_of[ia].max(links_of[ib]) as f64;
        let (most, carry) = (HALF_WAY / busy, 1.0 / busy);
        let d = &mut flow[p * width..(p + 1) * width];
        let (even, other) = (even.min(most), if mix + over > carry { carry / (mix + over) } else { 1.0 });
        let (from, to, sign) = if o > 0.0 { (ia, ib, 1.0) } else { (ib, ia, -1.0) };
        // The air that evens the pressure (from the higher), what the eddies swap each way, and each half overturned.
        let push_mol = even * (nt[ia] * c[ia] - nt[ib] * c[ib]) / (c[ia] + c[ib]);
        let sender = if push_mol > 0.0 { ia } else { ib };
        let (swap, turned) = (other * mix / (1.0 / nt[ia] + 1.0 / nt[ib]), sign * other * over * lower[from].min(upper[to]));
        for q in 0..ng {
            let at = |j: usize| x0[q * na + j];
            let halves = at(from) * (1.0 - s.split[q * na + from]) / lower[from] - at(to) * s.split[q * na + to] / upper[to];
            d[q] = push_mol * at(sender) / nt[sender] + swap * (at(ia) / nt[ia] - at(ib) / nt[ib]) + turned * halves;
        }
        let per = |j: usize| e0[j] / nt[j];
        d[ng] = push_mol * per(sender) + swap * (per(ia) - per(ib)) + turned * (per(from) - per(to));
    }
    // No node gives more of a gas than it holds (`solve::sweep::within`).
    let (a, b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (links.a[l], links.b[l])).unzip();
    for q in 0..ng {
        let mut of: Vec<f64> = (0..border.len()).map(|p| flow[p * width + q]).collect();
        within(&a, &b, &mut of, &x0[q * na..(q + 1) * na]);
        of.into_iter().enumerate().for_each(|(p, m)| flow[p * width + q] = m);
    }
    for (p, (&ia, &ib)) in a.iter().zip(&b).enumerate() {
        let (ia, ib, d) = (ia as usize, ib as usize, &flow[p * width..(p + 1) * width]);
        let kg: f64 = s.state.gas.iter().enumerate().map(|(q, &k)| d[q] * s.sp[k].mm).sum();
        for q in 0..ng {
            x[q * na + ia] -= d[q];
            x[q * na + ib] += d[q];
            moved.mol[q * pairs + p] += d[q];
        }
        e[ia] -= d[ng];
        e[ib] += d[ng];
        moved.heat[p] += d[ng];
        lift((ia, ib), kg, (s, g), e, &mut moved.paid);
    }
}

#[cfg(test)]
mod tests {
    use super::Wet;

    #[test]
    fn a_node_rains_what_its_cells_hold_over_their_saturation() {
        // Two cells sharing a node's air in halves, one saturating at the node's ratio 1, one (higher, colder) at 0.8.
        let wet = Wet::new(1, vec![vec![(0, 0.5, vec![1.0]), (1, 0.5, vec![0.8])]]);
        for (r, want) in [(0.7, 0.0), (0.9, 0.5 * 0.1), (1.2, 0.5 * 0.2 + 0.5 * 0.4)] {
            assert!((wet.over(0, 0, r) - want).abs() < 1e-12, "at {r}: {} against {want}", wet.over(0, 0, r));
        }
    }
}

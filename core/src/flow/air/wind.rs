//! Wind: what moves air between neighbouring air nodes (laws.md 1.6): the three pushes on each link as rates, from the
//! air of the step's start (`pushes`), and what the air's days move by them (`daily`): each block evened (`even`), then
//! the wind across the links between blocks (`wind`).

use super::super::Scratch;
use super::daily::{temps, Moved};
use crate::{solve::sweep::{within, HALF_WAY}, space::Level, units::R_GAS, Config, World};

/// A fit of a gradient whose neighbours nearly line up (the determinant below this share of the product of its
/// diagonal) is made along each axis alone.
const SINGULAR: f64 = 1e-9;

/// The links of the air level that share a border (width above 0), with their ends.
pub(super) struct Links {
    pub(super) id: Vec<usize>,
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
}

impl Links {
    pub(super) fn of(air: &Level) -> Links {
        let id: Vec<usize> = (0..air.a().len()).filter(|&l| air.width()[l] > 0.0).collect();
        let (a, b) = id.iter().map(|&l| (air.a()[l], air.b()[l])).unzip();
        Links { id, a, b }
    }
}

/// The pushes on each link (`Links`): mixing by eddies, mol/s each way; overturning, mol/s of the lower half from its
/// `a` to its `b` (the upper half comes back); and the conductance of the evening of pressure, mol/(Pa s).
pub(super) struct Pushes {
    pub(super) mix: Vec<f64>,
    pub(super) over: Vec<f64>,
    pub(super) even: Vec<f64>,
}

/// Three pushes on each link: the column's pressure, evened in the time sound takes to cross it (Lamb waves, Lindzen
/// 1990, 9.2); overturning by the warmth difference, the lower half toward the warmer node at `wind` per K, turned by
/// the rotation at the link's latitude; and eddy mixing (`mix` × width / length). From the air of each node as the
/// last state left it (`Scratch::nt`, `capa`, `mass`, `ta`).
pub(super) fn pushes(cfg: &Config, s: &Scratch, air: &Level, links: &Links) -> Pushes {
    let na = air.n();
    let warmth: Vec<f64> = (0..na).map(|j| s.ta[j] + s.gam[j] * s.zair[j]).collect();
    let grad = gradient(air, &warmth);
    let mut p = Pushes { mix: Vec::with_capacity(links.id.len()), over: Vec::with_capacity(links.id.len()), even: Vec::with_capacity(links.id.len()) };
    for &l in &links.id {
        let (a, b, wd, d) = (air.a()[l] as usize, air.b()[l] as usize, air.width()[l], air.dist()[l]);
        let (rho, depth) = ((s.nt[a] / s.vol[a] + s.nt[b] / s.vol[b]) / 2.0, cfg.air_top_m - (s.za[a] + s.za[b]) / 2.0);
        let turn = cfg.rotation * ((air.lat()[a] + air.lat()[b]) / 2.0).to_radians().sin();
        let (ab, ga, gb) = (air.offset(a, b), grad[a], grad[b]);
        let across = ((ga[0] + gb[0]) * ab[1] - (ga[1] + gb[1]) * ab[0]) / 2.0;
        p.mix.push(cfg.mix * rho * depth * wd / d);
        p.over.push(cfg.wind * (warmth[b] - warmth[a] + turn * across) * rho * wd * depth / 2.0);
        // p = c n with c = RT/V; sound crosses the link in d / sqrt(gamma R T / M), gamma = cp / (cp - R) per mol.
        let c = |j: usize| R_GAS * s.ta[j] / s.vol[j];
        let sound = |j: usize| {
            let cp = s.capa[j] / s.nt[j];
            (cp / (cp - R_GAS) * R_GAS * s.ta[j] * s.nt[j] / s.mass[j]).sqrt()
        };
        p.even.push((sound(a) + sound(b)) / 2.0 / (d * (c(a) + c(b))));
    }
    p
}

/// The gradient of `v` at each node [east, south], per m: the least-squares fit of its differences to its bordering
/// nodes (0 along an axis with none).
fn gradient(air: &Level, v: &[f64]) -> Vec<[f64; 2]> {
    (0..air.n())
        .map(|j| {
            let (mut m, mut r) = ([0.0; 3], [0.0; 2]);
            for (&l, &k) in air.around(j).0.iter().zip(air.around(j).1) {
                if air.width()[l as usize] > 0.0 {
                    let (d, dv) = (air.offset(j, k as usize), v[k as usize] - v[j]);
                    m = [m[0] + d[0] * d[0], m[1] + d[0] * d[1], m[2] + d[1] * d[1]];
                    r = [r[0] + d[0] * dv, r[1] + d[1] * dv];
                }
            }
            let det = m[0] * m[2] - m[1] * m[1];
            if det > SINGULAR * m[0] * m[2] {
                [(m[2] * r[0] - m[1] * r[1]) / det, (m[0] * r[1] - m[1] * r[0]) / det]
            } else {
                [r[0] / m[0].max(f64::MIN_POSITIVE), r[1] / m[2].max(f64::MIN_POSITIVE)]
            }
        })
        .collect()
}

/// Each block's air evened out: every gas of each node in proportion to its volume, and the heat so that the warmth of
/// each (its temperature at its mean height, raised to a common height by the block's drop with height, as the wind's
/// overturning reads it, `wind::pushes`) is the block's; so its pressure, shares and warmth are the block's. What
/// moves goes from the block's hub to each other node, pairs `first` on (`Moved`); the node the mass enters pays its
/// lift.
pub(super) fn even(w: &World, s: &Scratch, blocks: &[Vec<u32>], (x, e): (&mut [f64], &mut [f64]), first: usize, moved: &mut Moved) {
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
pub(super) fn wind(w: &World, s: &Scratch, (push, links, border): (&Pushes, &Links, &[usize]), (x, e): (&mut [f64], &mut [f64]), dd: f64, moved: &mut Moved) {
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

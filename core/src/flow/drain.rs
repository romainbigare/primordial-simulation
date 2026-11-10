//! Liquid under the surface: the pores of the loose ground and the store of the bed below (`Class::Deep`), two layers
//! of each cell (laws.md 1.2). Liquid over field capacity sinks from the pores into the store under them (`sink`, after
//! each of the air's steps); Darcy's law moves it between cells in each layer, an explicit `solve::sweep` (the store's
//! sub-stepped where a cell is too small for the step, `deep`), and a full store gives back what it cannot hold
//! (`under`, every liquid step). Under a body of standing liquid (`basin`) the pores are
//! full and the store is not drained: none of this runs there.

use super::{land::{bottom, filled, t_now, table, wet}, liquid::{book, mobile}, range, Scratch};
use crate::{math::one_minus_exp, solve::sweep::{sweep, Net}, store::{Class, LinkFlux}, World};

/// Liquid over field capacity sinks into the store, at most `seep` a second (Darcy's law under its own weight) and no
/// more than its room, with its heat, outside the bodies of standing liquid; its fall heats the store (`potential`). It runs after each of the air's steps, which
/// bring the rain: the rain would otherwise wait in the pores, darkening the ground (`land::reflectance`), until the
/// month's liquid step.
pub(super) fn sink(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, p, area) = (w.mesh.cells().n(), w.cfg.phys, w.mesh.cells().area());
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let (mob, held) = (mobile(w, s, &liq, tmin, |i| !s.basin.is[i]), stores(w, s, tmin));
    // The cells where liquid sinks, and the share of their mobile liquid that does.
    let mut sinks = Vec::new();
    for i in (0..n).filter(|&i| !s.basin.is[i]) {
        let room = p.bed_porosity * p.bed_depth * area[i] - held[i];
        let sink = (mob.vol[i] - p.field * s.pd[i] * area[i]).min(room).min(p.seep * area[i] * dt);
        if sink > 0.0 {
            sinks.push((i as u32, sink / mob.vol[i]));
        }
    }
    let down: Vec<u32> = sinks.iter().map(|c| c.0).collect();
    let sunk = carry(&sinks, &liq, |q, i| mob.mol[q * n + i], |i, _, mol| liquid_heat(w, s, &liq, i, mol));
    let liq16: Vec<u16> = liq.iter().map(|&k| k as u16).collect();
    book(w, s, &[LinkFlux { class: [Class::Ground, Class::Deep], a: &down, b: &down, ks: &liq16, mol: &sunk.0, heat: &sunk.1 }]);
}

/// The heat of `mol` of the liquids `liq` leaving ground node `i`, at the node's temperature now, J (`liquid_heat_at`).
pub(super) fn liquid_heat(w: &World, s: &Scratch, liq: &[usize], i: usize, mol: &[f64]) -> f64 {
    liquid_heat_at(s, liq, t_now(w, s, i), mol)
}

/// The heat of `mol` of the liquids `liq` at `t` K, J: each its heat capacity times `t` and the fusion enthalpy it
/// holds as a liquid.
pub(super) fn liquid_heat_at(s: &Scratch, liq: &[usize], t: f64, mol: &[f64]) -> f64 {
    liq.iter().zip(mol).map(|(&k, m)| m * (s.sp[k].cp * t + s.sp[k].lf)).sum()
}

/// In each layer liquid flows between cells by Darcy's law, across the borders off the bodies of standing liquid (`pore_flow`,
/// `deep::flow`); what a store holds over its room comes up (`come_up`). A move takes its share of every liquid with
/// what is dissolved and its heat; its fall heats where it goes (`potential`).
pub(super) fn under(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64, dt: f64) {
    let (n, p, cells) = (w.mesh.cells().n(), w.cfg.phys, w.mesh.cells());
    // A body moves no liquid under its surface: its mobile liquid is not needed.
    let (mob, held) = (mobile(w, s, liq, tmin, |i| !s.basin.is[i]), stores(w, s, tmin));
    let off_basin = |l: usize| !s.basin.is[cells.a()[l] as usize] && !s.basin.is[cells.b()[l] as usize];
    let border: Vec<usize> = (0..cells.a().len()).filter(|&l| cells.width()[l] > 0.0 && off_basin(l)).collect();
    let (a, b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (cells.a()[l], cells.b()[l])).unzip();
    // What the volumes carry: the share of the sender's mobile liquid, at its temperature, or of its store.
    let deep: Vec<usize> = (0..w.chem.ns()).filter(|&k| w.store.amounts(Class::Deep)[k * n..(k + 1) * n].iter().any(|&m| m > 0.0)).collect();
    let in_stores = super::deep::flow(w, s, (&border, &a, &b), (&deep, &held), dt);
    let pores = pore_flow(w, s, (&border, &a, &b), (liq, &mob), dt);
    let (liq16, deep16): (Vec<u16>, Vec<u16>) = (liq.iter().map(|&k| k as u16).collect(), deep.iter().map(|&k| k as u16).collect());
    let fluxes = [
        LinkFlux { class: [Class::Ground; 2], a: &pores.a, b: &pores.b, ks: &liq16, mol: &pores.mol, heat: &pores.heat },
        LinkFlux { class: [Class::Deep; 2], a: &in_stores.a, b: &in_stores.b, ks: &deep16, mol: &in_stores.mol, heat: &in_stores.heat },
    ];
    book(w, s, &fluxes);
    let (held, area) = (stores(w, s, tmin), w.mesh.cells().area());
    let full = |i: usize| p.bed_porosity * p.bed_depth * area[i];
    let over: Vec<f64> = held.iter().enumerate().map(|(i, &v)| if v > full(i) { (v - full(i)) / v } else { 0.0 }).collect();
    come_up(w, s, tmin, &over);
}

/// Darcy's law through the pores of the loose ground by an explicit sweep over the links `border` (their ends `a` and
/// `b`, `sweep::sweep`): the potential is the liquid's table,
/// the capacity its volume per metre of head, and a link passes `seep` x the saturated thickness of its higher end x
/// width / distance (`pore_net`); over cells of kilometres a liquid step moves a table a thousandth of the way to its
/// neighbours'. No pores give more than they hold over field capacity. A move takes its share of the mobile liquid
/// `mob` (of the liquids `liq`) with its heat at the node's temperature. Returns what crosses.
fn pore_flow(w: &World, s: &Scratch, links: (&[usize], &[u32], &[u32]), (liq, mob): (&[usize], &super::liquid::Mobile), dt: f64) -> Carried {
    let (p, cells, (_, a, b)) = (w.cfg.phys, w.mesh.cells(), links);
    let (n, area) = (cells.n(), cells.area());
    let x: Vec<f64> = (0..n).map(|i| table(s, i, p.porosity)).collect();
    let cap: Vec<f64> = (0..n).map(|i| area[i] * if filled(s, i, p.porosity) { 1.0 } else { p.porosity }).collect();
    let g = pore_net(w, s, links, &x, &mob.vol);
    let free: Vec<f64> = (0..n).map(|i| mob.vol[i] - p.field * s.pd[i] * area[i]).collect();
    let flux = sweep(&Net { a, b, g: &g, cap: &cap }, &x, dt, &free);
    carried((a, b), &flux, (liq, |q, i| mob.mol[q * n + i]), &mob.vol, |i, _, mol| liquid_heat(w, s, liq, i, mol))
}

/// The conductance of each link of `links` through the pores, m²/s, as the tables stand at `x` and the mobile liquid
/// is `vol` m³: `seep` × the saturated thickness of its higher end × width / distance; none for pores at field capacity
/// (held against gravity).
fn pore_net(w: &World, s: &Scratch, (border, a, b): (&[usize], &[u32], &[u32]), x: &[f64], vol: &[f64]) -> Vec<f64> {
    let (p, cells) = (w.cfg.phys, w.mesh.cells());
    let area = cells.area();
    let free = |i: usize| vol[i].min(s.pd[i] * area[i]) > p.field * s.pd[i] * area[i];
    (border.iter().zip(a.iter().zip(b)))
        .map(|(&l, (&i, &j))| {
            let (i, j) = (i as usize, j as usize);
            let up = if x[i] >= x[j] { i } else { j };
            let thick = if free(up) { p.seep * (x[up].min(s.z[up] - s.cover[up][2]) - bottom(s, i, p.porosity).max(bottom(s, j, p.porosity))).max(0.0) } else { 0.0 };
            thick * cells.width()[l] / cells.dist()[l]
        })
        .collect()
}

/// What crosses the links that carry something: their ends, mol of each substance (`[q * links + l]`) and J per link.
pub(super) struct Carried {
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
    pub(super) mol: Vec<f64>,
    pub(super) heat: Vec<f64>,
}

/// What volumes `flux` (m³, + from `a` to `b`) carry: of each substance of `ks` the sender's share that leaves times its
/// amount there, and its heat (`carry`).
pub(super) fn carried(
    (a, b): (&[u32], &[u32]),
    flux: &[f64],
    (ks, amount): (&[usize], impl Fn(usize, usize) -> f64),
    vol: &[f64],
    heat: impl Fn(usize, f64, &[f64]) -> f64,
) -> Carried {
    let from = |l: usize| if flux[l] > 0.0 { a[l] as usize } else { b[l] as usize };
    let used: Vec<usize> = (0..flux.len()).filter(|&l| flux[l] != 0.0 && vol[from(l)] > 0.0).collect();
    let senders: Vec<(u32, f64)> = used.iter().map(|&l| (from(l) as u32, flux[l] / vol[from(l)])).collect();
    let (mol, heat) = carry(&senders, ks, amount, heat);
    let ends = |v: &[u32]| used.iter().map(|&l| v[l]).collect();
    Carried { a: ends(a), b: ends(b), mol, heat }
}

/// What leaves each sender of `from` (its node and the share that leaves, negative from `b` to `a`): of each substance
/// of `ks` the share times its amount, mol per link with the share's sign, and its heat, J per link.
fn carry(from: &[(u32, f64)], ks: &[usize], amount: impl Fn(usize, usize) -> f64, heat: impl Fn(usize, f64, &[f64]) -> f64) -> (Vec<f64>, Vec<f64>) {
    let (links, mut m) = (from.len(), vec![0.0; ks.len()]);
    let (mut mol, mut e) = (vec![0.0; ks.len() * links], vec![0.0; links]);
    for (l, &(i, f)) in from.iter().enumerate() {
        let (i, share) = (i as usize, f.abs());
        for (q, mq) in m.iter_mut().enumerate() {
            *mq = amount(q, i) * share;
            mol[q * links + l] = f.signum() * *mq;
        }
        e[l] = f.signum() * heat(i, share, &m);
    }
    (mol, e)
}

/// A share `share[i]` of the store under each ground node comes up into it with its heat, the node paying its rise
/// (`potential`). Returns the volume each node got, m³ (dissolved gas adds none).
fn come_up(w: &mut World, s: &mut Scratch, tmin: f64, share: &[f64]) -> Vec<f64> {
    let (n, ns) = (w.mesh.cells().n(), w.chem.ns());
    let mut got = vec![0.0; n];
    let holds = |i: usize| (0..ns).any(|k| w.store.amounts(Class::Deep)[k * n + i] > 0.0);
    let ups: Vec<(u32, f64)> = share.iter().enumerate().filter(|p| *p.1 > 0.0 && holds(p.0)).map(|(i, &f)| (i as u32, f)).collect();
    let ks: Vec<usize> = (0..ns).filter(|&k| ups.iter().any(|c| w.store.amounts(Class::Deep)[k * n + c.0 as usize] > 0.0)).collect();
    let (up, ks16): (Vec<u32>, Vec<u16>) = (ups.iter().map(|c| c.0).collect(), ks.iter().map(|&k| k as u16).collect());
    let (mol, heat) = carry(&ups, &ks, |q, i| w.store.amounts(Class::Deep)[ks[q] * n + i], |i, f, _| w.store.heat(Class::Deep)[i] * f);
    for (l, &i) in up.iter().enumerate() {
        let (i, links) = (i as usize, up.len());
        for (q, &k) in ks.iter().enumerate() {
            let (m, sp) = (mol[q * links + l], s.sp[k]);
            got[i] += if sp.boil > tmin { m * sp.v[1] * wet(s, k, i) } else { 0.0 };
        }
    }
    book(w, s, &[LinkFlux { class: [Class::Deep, Class::Ground], a: &up, b: &up, ks: &ks16, mol: &mol, heat: &heat }]);
    got
}

/// The stores drain to the channels inside each cell outside the bodies, `hill` from the divides, by Darcy's law
/// down its slope: a share 1 - exp(-`bed_seep` × slope × time / (`bed_porosity` × `hill`)) comes up. Returns the
/// volume each node got, m³.
pub(super) fn springs(w: &mut World, s: &mut Scratch, tmin: f64, slope: &[f64], dt: f64) -> Vec<f64> {
    let p = w.cfg.phys;
    let drain = |(&q, &basin): (&f64, &bool)| if basin { 0.0 } else { one_minus_exp(p.bed_seep * q * dt / (p.bed_porosity * p.hill)) };
    let share: Vec<f64> = slope.iter().zip(&s.basin.is).map(drain).collect();
    come_up(w, s, tmin, &share)
}

/// Volume of the liquid in the store under each ground node, m³ (dissolved gas adds none).
fn stores(w: &World, s: &Scratch, tmin: f64) -> Vec<f64> {
    let n = w.mesh.cells().n();
    let mut vol = vec![0.0; n];
    for k in (0..w.chem.ns()).filter(|&k| s.sp[k].boil > tmin) {
        // (A substance no store holds adds nothing.)
        let held = &w.store.amounts(Class::Deep)[k * n..(k + 1) * n];
        if held.iter().any(|&m| m != 0.0) {
            vol.iter_mut().zip(held).for_each(|(v, &m)| *v += m * s.sp[k].v[1]);
        }
    }
    vol
}

/// Height of the table of the store under ground node `i` holding `vol` m³, m: the bottom of its zone, `bed_depth`
/// under the loose ground, plus the depth its liquid fills.
pub(super) fn store_table(w: &World, s: &Scratch, i: usize, vol: f64) -> f64 {
    bottom(s, i, w.cfg.phys.porosity) - w.cfg.phys.bed_depth + vol / (w.cfg.phys.bed_porosity * w.mesh.cells().area()[i])
}

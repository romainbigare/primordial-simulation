//! Liquid under the surface: the pores of the loose ground and the store of the bed below (`Class::Deep`), two layers
//! of each cell's stack joined by a vertical link (laws.md 1.2): the rates along that link, and Darcy's law between
//! cells in each layer, which `solve::spread` solves.

use super::land::{t_now, table, wet};
use super::liquid::{book, mobile};
use super::Scratch;
use crate::math::one_minus_exp;
use crate::solve::spread::{within, Net, Spread};
use crate::space::{Layer, Stack};
use crate::store::{Batch, Class, LinkFlux};
use crate::World;

/// The layers of each cell's stack under its surface, bottom up: the bed's store and the loose ground's pores.
const DEEP: usize = 0;
const PORES: usize = 1;

/// The stack of the two layers: the store `bed_depth` deep under the active layer of the ground (`skin_m`).
fn stack(w: &World) -> Stack {
    let layer = |depth| Layer { level: 0, depth };
    Stack::new(&w.mesh, &[layer(w.cfg.phys.bed_depth), layer(w.cfg.skin_m)])
}

/// Liquid over field capacity sinks into the store, at most `seep` a second (Darcy's law under its own weight) and no
/// more than its room; in each layer liquid flows between cells (`flows`); what a store holds over its room comes up
/// (`come_up`). A move takes its share of every liquid with what is dissolved and its heat; lost head heats where it goes.
pub(super) fn under(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64, dt: f64) {
    let (n, p, g) = (w.mesh.cells().n(), w.cfg.phys, w.cfg.gravity);
    let (mob, stack, held) = (mobile(w, s, liq, tmin), stack(w), stores(w, s, tmin));
    // The heat each node of the stack gains from the head lost on the way into it, J.
    let mut fallen = vec![0.0; stack.nodes()];
    // The cells where liquid sinks down the vertical link, and the share of their mobile liquid that does.
    let mut sinks = Vec::new();
    for (v, &area) in stack.area().iter().enumerate() {
        let (lo, i) = (stack.lo()[v] as usize, stack.hi()[v] as usize - stack.node(PORES, 0));
        let room = p.bed_porosity * p.bed_depth * area - held[i];
        let sink = (mob.vol[i] - p.field * s.pd[i] * area).min(room).min(p.seep * area * dt);
        if sink > 0.0 {
            let (share, kg) = (sink / mob.vol[i], (0..liq.len()).map(|q| mob.mol[q * n + i] * s.sp[liq[q]].mm).sum::<f64>());
            fallen[lo] += share * kg * g * (s.head[i] - store_table(w, s, i, held[i]));
            sinks.push((i as u32, share));
        }
    }
    let down: Vec<u32> = sinks.iter().map(|c| c.0).collect();
    let sunk = carry(&sinks, liq, |q, i| mob.mol[q * n + i], |i, _, mol| {
        let t = t_now(w, s, i);
        liq.iter().zip(mol).map(|(&k, m)| m * (s.sp[k].cp * t + s.sp[k].lf)).sum()
    });
    let mut spread = std::mem::take(&mut s.spread);
    let (a, b, flux, x) = flows(w, s, &mut spread, &stack, (&mob, &held), dt);
    s.spread = spread;
    let links = a.len();
    // What the volumes carry: the share of the sender's mobile liquid, at its temperature, or of its store.
    let deep: Vec<usize> = (0..w.chem.ns()).filter(|&k| w.store.amounts(Class::Deep)[k * n..(k + 1) * n].iter().any(|&m| m > 0.0)).collect();
    let pores = carried((&a, &b), &flux[links..], (liq, |q, i| mob.mol[q * n + i]), &mob.vol, |i, _, mol| {
        let t = t_now(w, s, i);
        liq.iter().zip(mol).map(|(&k, m)| m * (s.sp[k].cp * t + s.sp[k].lf)).sum()
    });
    let in_stores = carried((&a, &b), &flux[..links], (&deep, |q, i| w.store.amounts(Class::Deep)[deep[q] * n + i]), &held, |i, share, _| w.store.heat(Class::Deep)[i] * share);
    for (layer, (ks, c)) in [(DEEP, (&deep[..], &in_stores)), (PORES, (liq, &pores))] {
        let links = c.a.len();
        for l in 0..links {
            let kg: f64 = ks.iter().enumerate().map(|(q, &k)| c.mol[q * links + l] * s.sp[k].mm).sum();
            let (from, to) = if kg > 0.0 { (c.a[l], c.b[l]) } else { (c.b[l], c.a[l]) };
            let (from, to) = (stack.node(layer, from as usize), stack.node(layer, to as usize));
            fallen[to] += kg.abs() * g * (x[from] - x[to]);
        }
    }
    let (liq16, deep16): (Vec<u16>, Vec<u16>) = (liq.iter().map(|&k| k as u16).collect(), deep.iter().map(|&k| k as u16).collect());
    let (mut into_deep, mut into_pores) = (Batch::falls(Class::Deep), Batch::falls(Class::Ground));
    for i in 0..n {
        into_deep.fall(i, fallen[stack.node(DEEP, i)]);
        into_pores.fall(i, fallen[stack.node(PORES, i)]);
    }
    let fluxes = [
        LinkFlux { class: [Class::Ground; 2], a: &pores.a, b: &pores.b, ks: &liq16, mol: &pores.mol, heat: &pores.heat },
        LinkFlux { class: [Class::Deep; 2], a: &in_stores.a, b: &in_stores.b, ks: &deep16, mol: &in_stores.mol, heat: &in_stores.heat },
        LinkFlux { class: [Class::Ground, Class::Deep], a: &down, b: &down, ks: &liq16, mol: &sunk.0, heat: &sunk.1 },
        into_deep.flux(),
        into_pores.flux(),
    ];
    book(w, s, &fluxes);
    let (held, area) = (stores(w, s, tmin), w.mesh.cells().area());
    let full = |i: usize| p.bed_porosity * p.bed_depth * area[i];
    let over: Vec<f64> = held.iter().enumerate().map(|(i, &v)| if v > full(i) { (v - full(i)) / v } else { 0.0 }).collect();
    come_up(w, s, tmin, (&over, &held));
}

/// Darcy's law between cells in each layer by one `spread` over the stack: the potential is the liquid's table, the
/// capacity its volume per metre of head, and a link passes the conductivity (`bed_seep`, `seep`) × the saturated
/// thickness of its higher end × width / distance (none for pores at field capacity, held against gravity). No node
/// gives more than it holds (`spread::within`). Returns the links, the volume through each (stores', then pores'), m³,
/// and the heads before the step, m.
fn flows(w: &World, s: &Scratch, spread: &mut Spread, stack: &Stack, (mob, held): (&super::liquid::Mobile, &[f64]), dt: f64) -> (Vec<u32>, Vec<u32>, Vec<f64>, Vec<f64>) {
    let (p, cells) = (w.cfg.phys, w.mesh.cells());
    let area = cells.area();
    let (mut x, mut cap) = (vec![0.0; stack.nodes()], vec![0.0; stack.nodes()]);
    for (i, &vol) in held.iter().enumerate() {
        let (d, q) = (stack.node(DEEP, i), stack.node(PORES, i));
        (x[d], cap[d]) = (store_table(w, s, i, vol), p.bed_porosity * area[i]);
        (x[q], cap[q]) = (table(s, i, p.porosity), area[i] * if s.head[i] - s.z[i] >= s.pd[i] { 1.0 } else { p.porosity });
    }
    let bottom = |i: usize| s.z[i] - if p.porosity > 0.0 { s.pd[i] / p.porosity } else { 0.0 };
    let free = |i: usize| mob.vol[i].min(s.pd[i] * area[i]) > p.field * s.pd[i] * area[i];
    let border: Vec<usize> = (0..cells.a().len()).filter(|&l| cells.width()[l] > 0.0).collect();
    let (a, b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (cells.a()[l], cells.b()[l])).unzip();
    let (mut na, mut nb, mut gl) = (Vec::new(), Vec::new(), Vec::new());
    for layer in [DEEP, PORES] {
        for (&l, (&i, &j)) in border.iter().zip(a.iter().zip(&b)) {
            let (i, j) = (i as usize, j as usize);
            let (ni, nj) = (stack.node(layer, i), stack.node(layer, j));
            let up = if x[ni] >= x[nj] { i } else { j };
            let thick = match layer {
                DEEP => p.bed_seep * held[up] / (p.bed_porosity * area[up]),
                _ if free(up) => p.seep * (x[stack.node(PORES, up)].min(s.z[up]) - bottom(i).max(bottom(j))).max(0.0),
                _ => 0.0,
            };
            na.push(ni as u32);
            nb.push(nj as u32);
            gl.push(thick * cells.width()[l] / cells.dist()[l]);
        }
    }
    let mut flux = vec![0.0; gl.len()];
    let x0 = x.clone();
    // The network is far from stiff (dt g / capacity about 1e-3): the solve converges in a few iterations.
    spread.step(&Net { a: &na, b: &nb, g: &gl, cap: &cap }, &mut x, dt, cap.len(), &mut flux);
    // No store and no pores give more than they hold.
    let mut have = vec![0.0; stack.nodes()];
    for (i, &vol) in held.iter().enumerate() {
        (have[stack.node(DEEP, i)], have[stack.node(PORES, i)]) = (vol, mob.vol[i]);
    }
    within(&na, &nb, &mut flux, &have);
    (a, b, flux, x0)
}

/// What crosses the links that carry something: their ends, mol of each substance (`[q * links + l]`) and J per link.
struct Carried {
    a: Vec<u32>,
    b: Vec<u32>,
    mol: Vec<f64>,
    heat: Vec<f64>,
}

/// What volumes `flux` (m³, + from `a` to `b`) carry: of each substance of `ks` the sender's share that leaves times its
/// amount there, and its heat (`carry`).
fn carried(
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

/// A share `share[i]` of the store under each ground node comes up into it with its heat, paying its rise into the
/// potential energy. Returns the volume each node got, m³ (dissolved gas adds none).
fn come_up(w: &mut World, s: &mut Scratch, tmin: f64, (share, held): (&[f64], &[f64])) -> Vec<f64> {
    let (n, g, ns) = (w.mesh.cells().n(), w.cfg.gravity, w.chem.ns());
    let (mut got, mut rise) = (vec![0.0; n], Batch::falls(Class::Ground));
    let holds = |i: usize| (0..ns).any(|k| w.store.amounts(Class::Deep)[k * n + i] > 0.0);
    let ups: Vec<(u32, f64)> = share.iter().enumerate().filter(|p| *p.1 > 0.0 && holds(p.0)).map(|(i, &f)| (i as u32, f)).collect();
    let ks: Vec<usize> = (0..ns).filter(|&k| ups.iter().any(|c| w.store.amounts(Class::Deep)[k * n + c.0 as usize] > 0.0)).collect();
    if ks.is_empty() {
        return got;
    }
    let (up, ks16): (Vec<u32>, Vec<u16>) = (ups.iter().map(|c| c.0).collect(), ks.iter().map(|&k| k as u16).collect());
    let (mol, heat) = carry(&ups, &ks, |q, i| w.store.amounts(Class::Deep)[ks[q] * n + i], |i, f, _| w.store.heat(Class::Deep)[i] * f);
    for (l, &i) in up.iter().enumerate() {
        let (i, links) = (i as usize, up.len());
        let (lift, mut r) = (g * (s.head[i] - store_table(w, s, i, held[i])), 0.0);
        for (q, &k) in ks.iter().enumerate() {
            let (m, sp) = (mol[q * links + l], s.sp[k]);
            r += m * sp.mm * lift;
            got[i] += if sp.boil > tmin { m * sp.v[1] * wet(s, k, i) } else { 0.0 };
        }
        rise.fall(i, -r);
    }
    book(w, s, &[LinkFlux { class: [Class::Deep, Class::Ground], a: &up, b: &up, ks: &ks16, mol: &mol, heat: &heat }, rise.flux()]);
    got
}

/// The stores drain to the channels inside each cell, `hill` from the divides, by Darcy's law down its slope: a share
/// 1 - exp(-`bed_seep` × slope × time / (`bed_porosity` × `hill`)) comes up. Returns the volume each node got, m³.
pub(super) fn springs(w: &mut World, s: &mut Scratch, tmin: f64, slope: &[f64], dt: f64) -> Vec<f64> {
    let p = w.cfg.phys;
    let share: Vec<f64> = slope.iter().map(|&q| one_minus_exp(p.bed_seep * q * dt / (p.bed_porosity * p.hill))).collect();
    come_up(w, s, tmin, (&share, &stores(w, s, tmin)))
}

/// Volume of the liquid in the store under each ground node, m³ (dissolved gas adds none).
fn stores(w: &World, s: &Scratch, tmin: f64) -> Vec<f64> {
    let n = w.mesh.cells().n();
    let mut vol = vec![0.0; n];
    for k in (0..w.chem.ns()).filter(|&k| s.sp[k].boil > tmin) {
        vol.iter_mut().zip(&w.store.amounts(Class::Deep)[k * n..(k + 1) * n]).for_each(|(v, &m)| *v += m * s.sp[k].v[1]);
    }
    vol
}

/// Height of the table of the store under ground node `i` holding `vol` m³, m: the bottom of its zone, `bed_depth`
/// under the loose ground, plus the depth its liquid fills.
fn store_table(w: &World, s: &Scratch, i: usize, vol: f64) -> f64 {
    let p = w.cfg.phys;
    let loose = if p.porosity > 0.0 { s.pd[i] / p.porosity } else { 0.0 };
    s.z[i] - loose - p.bed_depth + vol / (p.bed_porosity * w.mesh.cells().area()[i])
}

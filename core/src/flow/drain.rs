//! Liquid under the surface: the pores of the loose ground and the store of the bed below (`World::deep`), two layers
//! of each cell's stack (`space::Stack`) joined by a vertical link. This file gives their physics: the rates down and
//! up the vertical link, and the conductances and capacities of Darcy's law between cells in each layer, which
//! `solve::spread` solves; the store books every move (`store::apply`).

use super::land::{t_now, table, wet};
use super::liquid::{book, mobile};
use super::{Falls, Scratch};
use crate::math::one_minus_exp;
use crate::solve::spread::{Net, Spread};
use crate::space::{Layer, Stack};
use crate::store::{Class, LinkFlux};
use crate::World;

/// The layers of each cell's stack under its surface, bottom up: the store of the bed, and the pores of the loose
/// ground (the ground node).
const DEEP: usize = 0;
const PORES: usize = 1;

/// The stack of the two layers: the store `bed_depth` deep under the active layer of the ground (`skin_m`).
fn stack(w: &World) -> Stack {
    let layer = |depth| Layer { level: 0, depth };
    Stack::new(w.grid.mesh(), &[layer(w.cfg.phys.bed_depth), layer(w.cfg.skin_m)])
}

/// Down the vertical link, liquid the loose ground holds over its field capacity (a share `field` of its pores) sinks
/// into the store, `seep` a second at most (Darcy's law under its own weight), no more than the store has room for
/// (`bed_porosity` x `bed_depth`). In each layer the liquid flows between cells across their shared borders by Darcy's
/// law (`flows`). What a store then holds over its room comes up into its ground node (`come_up`). Each move takes its
/// share of every liquid and what is dissolved in it, with its heat; the head it loses heats where it goes, from the
/// world's potential energy.
pub(super) fn under(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64, dt: f64) {
    let (n, p, g) = (w.grid.n(), w.cfg.phys, w.cfg.gravity);
    let (mob, stack, held) = (mobile(w, s, liq, tmin), stack(w), stores(w, s, tmin));
    // The heat each node of the stack gains from the head lost on the way into it, J.
    let mut fallen = vec![0.0; stack.nodes()];
    // Each vertical link joins the store of a cell (below) to its ground node, across the cell's area: the cells where
    // liquid sinks, and the share of their mobile liquid that does.
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
    let sunk = carried_by(&sinks, liq, |q, i| mob.mol[q * n + i], |i, mol| {
        let t = t_now(w, s, i);
        liq.iter().zip(mol).map(|(&k, m)| m * (s.sp[k].cp * t + s.sp[k].lf)).sum()
    });
    let (a, b, flux, x) = flows(w, s, &stack, (&mob, &held), dt);
    let links = a.len();
    // What the volumes carry: the share of the sender's mobile liquid, at its temperature, or of its store.
    let deep: Vec<usize> = (0..w.chem.ns()).filter(|&k| w.deep[k * n..(k + 1) * n].iter().any(|&m| m > 0.0)).collect();
    let pores = carried((&a, &b), &flux[links..], (liq, |q, i| mob.mol[q * n + i]), &mob.vol, |i, _, mol| {
        let t = t_now(w, s, i);
        liq.iter().zip(mol).map(|(&k, m)| m * (s.sp[k].cp * t + s.sp[k].lf)).sum()
    });
    let in_stores = carried((&a, &b), &flux[..links], (&deep, |q, i| w.deep[deep[q] * n + i]), &held, |i, share, _| w.e_deep[i] * share);
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
    let (mut into_deep, mut into_pores) = (Falls::new(Class::Deep), Falls::new(Class::Ground));
    for i in 0..n {
        into_deep.add(i, fallen[stack.node(DEEP, i)]);
        into_pores.add(i, fallen[stack.node(PORES, i)]);
    }
    let fluxes = [
        LinkFlux { class: [Class::Ground; 2], a: &pores.a, b: &pores.b, ks: &liq16, mol: &pores.mol, heat: &pores.heat },
        LinkFlux { class: [Class::Deep; 2], a: &in_stores.a, b: &in_stores.b, ks: &deep16, mol: &in_stores.mol, heat: &in_stores.heat },
        LinkFlux { class: [Class::Ground, Class::Deep], a: &down, b: &down, ks: &liq16, mol: &sunk.0, heat: &sunk.1 },
        into_deep.flux(),
        into_pores.flux(),
    ];
    book(w, s, &fluxes);
    let full = p.bed_porosity * p.bed_depth * s.area;
    let held = stores(w, s, tmin);
    let over: Vec<f64> = held.iter().map(|&v| if v > full { (v - full) / v } else { 0.0 }).collect();
    come_up(w, s, tmin, (&over, &held));
}

/// Darcy's law between cells in each layer, across their shared borders, by one `spread` over the stack: the potential
/// is the head of the liquid (the store's table, the pores' liquid table, `land::table`), the capacity the volume per
/// metre of head (`bed_porosity` x area; `porosity` x area, or the area where the pores are full), and a link passes
/// the conductivity (`bed_seep`, `seep`) x the saturated thickness of its higher end x the border's width / the
/// distance. A store's saturated thickness is the depth it fills; the pores' is the depth of the higher table over
/// the higher of the two bottoms of the loose ground, none where they hold no more than their field capacity (they
/// hold that against gravity). Returns the links (the cells' links with a border), the volume through each, m³ (all
/// the stores' links, then all the pores'), and the heads before the step, m, by node of the stack.
fn flows(w: &World, s: &Scratch, stack: &Stack, (mob, held): (&super::liquid::Mobile, &[f64]), dt: f64) -> (Vec<u32>, Vec<u32>, Vec<f64>, Vec<f64>) {
    let (p, cells) = (w.cfg.phys, w.grid.mesh().cells());
    let (mut x, mut cap) = (vec![0.0; stack.nodes()], vec![0.0; stack.nodes()]);
    for (i, &vol) in held.iter().enumerate() {
        let (d, q) = (stack.node(DEEP, i), stack.node(PORES, i));
        (x[d], cap[d]) = (store_table(w, s, i, vol), p.bed_porosity * s.area);
        (x[q], cap[q]) = (table(s, i, p.porosity), s.area * if s.head[i] - s.z[i] >= s.pd[i] { 1.0 } else { p.porosity });
    }
    let bottom = |i: usize| s.z[i] - if p.porosity > 0.0 { s.pd[i] / p.porosity } else { 0.0 };
    let free = |i: usize| mob.vol[i].min(s.pd[i] * s.area) > p.field * s.pd[i] * s.area;
    let border: Vec<usize> = (0..cells.a().len()).filter(|&l| cells.width()[l] > 0.0).collect();
    let (a, b): (Vec<u32>, Vec<u32>) = border.iter().map(|&l| (cells.a()[l], cells.b()[l])).unzip();
    let (mut na, mut nb, mut gl) = (Vec::new(), Vec::new(), Vec::new());
    for layer in [DEEP, PORES] {
        for (&l, (&i, &j)) in border.iter().zip(a.iter().zip(&b)) {
            let (i, j) = (i as usize, j as usize);
            let (ni, nj) = (stack.node(layer, i), stack.node(layer, j));
            let up = if x[ni] >= x[nj] { i } else { j };
            let thick = match layer {
                DEEP => p.bed_seep * held[up] / (p.bed_porosity * s.area),
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
    // The network is far from stiff (dt g / capacity is about 1e-3 on the island): the solve converges in a few
    // iterations; the cap of one per node is never reached.
    Spread::default().step(&Net { a: &na, b: &nb, g: &gl, cap: &cap }, &mut x, dt, cap.len(), &mut flux);
    (a, b, flux, x0)
}

/// What crosses the links that carry something: their ends, and of each substance mol per link (`[q * links + l]`), and
/// J per link.
struct Carried {
    a: Vec<u32>,
    b: Vec<u32>,
    mol: Vec<f64>,
    heat: Vec<f64>,
}

/// What volumes `flux` (m³, + from `a` to `b`) carry, on the links where something leaves: of each substance of `ks`,
/// the share of its sender's volume `vol` that leaves times its amount there (`amount(q, i)`), and the heat of what
/// leaves (`heat(i, share, mol)`).
fn carried(
    (a, b): (&[u32], &[u32]),
    flux: &[f64],
    (ks, amount): (&[usize], impl Fn(usize, usize) -> f64),
    vol: &[f64],
    heat: impl Fn(usize, f64, &[f64]) -> f64,
) -> Carried {
    let from = |l: usize| if flux[l] > 0.0 { a[l] as usize } else { b[l] as usize };
    let used: Vec<usize> = (0..flux.len()).filter(|&l| flux[l] != 0.0 && vol[from(l)] > 0.0).collect();
    let (links, mut m) = (used.len(), vec![0.0; ks.len()]);
    let ends = |v: &[u32]| used.iter().map(|&l| v[l]).collect();
    let mut c = Carried { a: ends(a), b: ends(b), mol: vec![0.0; ks.len() * links], heat: vec![0.0; links] };
    for (u, &l) in used.iter().enumerate() {
        let (i, sign, share) = (from(l), flux[l].signum(), (flux[l].abs() / vol[from(l)]).min(1.0));
        for (q, mq) in m.iter_mut().enumerate() {
            *mq = amount(q, i) * share;
            c.mol[q * links + u] = sign * *mq;
        }
        c.heat[u] = sign * heat(i, share, &m);
    }
    c
}

/// What the vertical links of `cells` (each with the share of its sender that leaves) carry: of each substance of
/// `ks`, the share times its amount there (`amount(q, i)`), mol per link (`[q * links + l]`), and the heat of what
/// leaves (`heat(i, mol)`), J per link.
fn carried_by(cells: &[(u32, f64)], ks: &[usize], amount: impl Fn(usize, usize) -> f64, heat: impl Fn(usize, &[f64]) -> f64) -> (Vec<f64>, Vec<f64>) {
    let (links, mut m) = (cells.len(), vec![0.0; ks.len()]);
    let (mut mol, mut e) = (vec![0.0; ks.len() * links], vec![0.0; links]);
    for (l, &(i, share)) in cells.iter().enumerate() {
        for (q, mq) in m.iter_mut().enumerate() {
            *mq = amount(q, i as usize) * share;
            mol[q * links + l] = *mq;
        }
        e[l] = heat(i as usize, &m);
    }
    (mol, e)
}

/// A share `share[i]` of the store under each ground node `i` (holding `held[i]` m³, `stores`) comes up into it with
/// its heat, and pays its rise from the store's table to the node's head into the world's potential energy. Returns the volume of liquid each node got,
/// m³ (what boils below the coldest node, `tmin`, is dissolved gas and adds none).
fn come_up(w: &mut World, s: &mut Scratch, tmin: f64, (share, held): (&[f64], &[f64])) -> Vec<f64> {
    let (n, g, ns) = (w.grid.n(), w.cfg.gravity, w.chem.ns());
    let (mut got, mut rise) = (vec![0.0; n], Falls::new(Class::Ground));
    let holds = |i: usize| (0..ns).any(|k| w.deep[k * n + i] > 0.0);
    let ups: Vec<(u32, f64)> = share.iter().enumerate().filter(|p| *p.1 > 0.0 && holds(p.0)).map(|(i, &f)| (i as u32, f)).collect();
    let ks: Vec<usize> = (0..ns).filter(|&k| ups.iter().any(|c| w.deep[k * n + c.0 as usize] > 0.0)).collect();
    if ks.is_empty() {
        return got;
    }
    let (up, ks16): (Vec<u32>, Vec<u16>) = (ups.iter().map(|c| c.0).collect(), ks.iter().map(|&k| k as u16).collect());
    let (mol, heat) = carried_by(&ups, &ks, |q, i| w.deep[ks[q] * n + i], |i, _| w.e_deep[i] * share[i]);
    for (l, &i) in up.iter().enumerate() {
        let (i, links) = (i as usize, up.len());
        let (lift, mut r) = (g * (s.head[i] - store_table(w, s, i, held[i])), 0.0);
        for (q, &k) in ks.iter().enumerate() {
            let (m, sp) = (mol[q * links + l], s.sp[k]);
            r += m * sp.mm * lift;
            got[i] += if sp.boil > tmin { m * sp.v[1] * wet(s, k, i) } else { 0.0 };
        }
        rise.add(i, -r);
    }
    book(w, s, &[LinkFlux { class: [Class::Deep, Class::Ground], a: &up, b: &up, ks: &ks16, mol: &mol, heat: &heat }, rise.flux()]);
    got
}

/// The stores drain up the vertical link to the channels inside each cell, `hill` from the divides, by Darcy's law
/// down the slope of its surface (`slope`, to the steepest lower neighbour): a share 1 - exp(-`bed_seep` x slope x
/// time / (`bed_porosity` x `hill`)) of what they hold comes up (`come_up`). Returns the volume each node got, m³,
/// which runs off.
pub(super) fn springs(w: &mut World, s: &mut Scratch, tmin: f64, slope: &[f64], dt: f64) -> Vec<f64> {
    w.fit_deep();
    let p = w.cfg.phys;
    let share: Vec<f64> = slope.iter().map(|&q| one_minus_exp(p.bed_seep * q * dt / (p.bed_porosity * p.hill))).collect();
    come_up(w, s, tmin, (&share, &stores(w, s, tmin)))
}

/// Volume of the liquid in the store under each ground node, m³ (what boils below the coldest node, `tmin`, is
/// dissolved gas and adds none).
fn stores(w: &World, s: &Scratch, tmin: f64) -> Vec<f64> {
    let n = w.grid.n();
    let mut vol = vec![0.0; n];
    for k in (0..w.chem.ns()).filter(|&k| s.sp[k].boil > tmin) {
        vol.iter_mut().zip(&w.deep[k * n..(k + 1) * n]).for_each(|(v, &m)| *v += m * s.sp[k].v[1]);
    }
    vol
}

/// Height of the table of the store under ground node `i` holding `vol` m³, m: the bottom of its zone, `bed_depth`
/// under the loose ground (whose depth its pores tell, none without pores), plus the depth its liquid fills.
fn store_table(w: &World, s: &Scratch, i: usize, vol: f64) -> f64 {
    let p = w.cfg.phys;
    let loose = if p.porosity > 0.0 { s.pd[i] / p.porosity } else { 0.0 };
    s.z[i] - loose - p.bed_depth + vol / (p.bed_porosity * s.area)
}

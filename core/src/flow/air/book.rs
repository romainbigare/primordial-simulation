//! What an air step moves over its span (`Pending`): worked out at once by the step (`record`), and booked a share at
//! a time as the world's clock goes through the span (`dispense`, at each physics step for the physics step ahead).
//! So the ground, the air, the light on bodies and the rain change through the span as the other laws read them, as
//! they would with short steps, not all at its start; the step's solve costs the same.

use super::super::{follow, year_weight, Scratch};
use super::exchange::Crossing;
use super::newton::Wind;
use super::step::{Active, Iterate, FIXED, PER_GAS};
use super::wind::Links;
use super::{lerp, sums};
use crate::store::{apply, Batch, Boundary, Class, LinkFlux};
use crate::{World, AIR};
use serde::{Deserialize, Serialize};

/// What the last air step moves over its span, in the store's terms (each node by its index), and how far it is booked.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(in crate::flow) struct Pending {
    /// The span the step covers, from `from` for `span` seconds, and the time up to which it is booked, s.
    from: u64,
    span: u64,
    upto: u64,
    /// The gases the ground exchanges with the air (substances), and the gases of the air (`Scratch::gas`).
    ks: Vec<usize>,
    gas: Vec<u16>,
    /// Per ground node: the sunlight it absorbs, J; the heat it gives its air node (and what leaves to space through
    /// it), J; the light each unit of capture area of its bodies gets, J/m²; the light set aside for them, J; what rains
    /// on it, mol.
    sun: Vec<f64>,
    heat: Vec<f64>,
    light: Vec<f64>,
    kept: Vec<f64>,
    rain: Vec<f64>,
    /// Per gas of `ks` and ground node (`[x * cells + i]`): the net amount that goes up into its air node, mol, and the
    /// heat that leaves the ground with it, J.
    up: Vec<f64>,
    up_heat: Vec<f64>,
    /// Per air node: the heat that leaves it to space, J, and the energy it pays to lift the gas that enters it from its
    /// neighbours, J (`Crossing` books the lift of the gas from the ground).
    out: Vec<f64>,
    lift: Vec<f64>,
    /// Per gas of `gas` and link of the air level (`[gas * links + link]`, `Links`) what the wind carries, mol, and per
    /// link the heat, J.
    wind: Vec<f64>,
    wind_heat: Vec<f64>,
}

/// What the step that starts now and lasts `dt` seconds moves, from the rows of its last round (`step::cell`), the
/// wind of its last solve and its air `it`: the ground pays the heat of the gas that leaves it and its fusion and
/// vaporisation enthalpy, rain brings its heat at the air's temperature and its fusion enthalpy; a ground node gives no
/// more than it holds and gets. The clouds the step leaves, which the next step starts from.
pub(super) fn record(w: &World, s: &mut Scratch, act: &[Active], (rows, width): (&[f64], usize), wind: &Wind, it: &Iterate, dt: f64) {
    let (n, na, cells) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells());
    let v = |len: usize| vec![0.0; len];
    let mut p = Pending { from: w.time, span: dt as u64, upto: w.time, ks: act.iter().map(|a| a.k).collect(), gas: s.state.gas.iter().map(|&k| k as u16).collect(), sun: v(n), heat: v(n), light: v(n), kept: v(n), rain: v(n), up: v(act.len() * n), up_heat: v(act.len() * n), out: v(na), lift: wind.paid.clone(), wind: wind.mol.clone(), wind_heat: wind.heat.clone() };
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (j, ts, area) = (cells.parent()[i] as usize, row[4], cells.area()[i]);
        (p.sun[i], p.heat[i], p.light[i], p.kept[i]) = (row[3], row[0] + row[2], row[5], row[6] * area * dt);
        p.out[j] += row[2];
        for (x, ac) in act.iter().enumerate() {
            let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
            let r = row[o + 3];
            let m = row[o].min(w.store.amounts(Class::Ground)[ac.k * n + i] + r);
            (p.up[x * n + i], p.up_heat[x * n + i]) = (m - r, m * (sp.cp * ts + sp.lf + sp.lv) - r * (sp.cp * it.t[j] + sp.lf));
            p.rain[i] += r;
        }
    }
    for (x, ac) in act.iter().enumerate() {
        if p.up[x * n..(x + 1) * n].iter().any(|&u| u < 0.0) {
            s.add_loose(ac.k, false);
        }
    }
    for i in 0..n {
        s.state.cloud[i] = lerp(s, &it.refl, i);
    }
    s.state.pending = p;
}

/// Book what the last air step moves from the time it is booked up to `to` (no further than its span), as that share
/// of its span, in one call through the store (only what each node holds at the end counts): the sunlight, the light
/// set aside in the bodies' node of each cell, the heat to the air and to space, the gas of the ground and its heat, the
/// wind, the lifts; the light on bodies, and the yearly average of the rain. A ground node gives no more of a gas than it holds now, and an air node no more than it
/// holds and gets (`hold_air`): the other laws change both between shares.
pub(in crate::flow) fn dispense(w: &mut World, s: &mut Scratch, to: u64) {
    let p = std::mem::take(&mut s.state.pending);
    let upto = to.min(p.from + p.span).max(p.upto);
    if upto == p.upto || p.span == 0 {
        s.state.pending = p;
        return;
    }
    let (secs, n, na) = ((upto - p.upto) as f64, w.mesh.cells().n(), w.mesh.level(AIR).n());
    let f = secs / p.span as f64;
    let links = Links::of(w.mesh.level(AIR));
    let nl = links.a.len();
    let (parent, area) = (w.mesh.cells().parent().to_vec(), w.mesh.cells().area().to_vec());
    let scaled = |v: &[f64]| v.iter().map(|&q| f * q).collect::<Vec<f64>>();
    // The share of each ground node's gas, cut where it gives more than it holds; then the wind and the rain of each gas,
    // cut where an air node would.
    let mut up: Vec<f64> = p.up.iter().enumerate().map(|(q, &u)| if u > 0.0 { (f * u).min(w.store.amounts(Class::Ground)[p.ks[q / n] * n + q % n]) } else { f * u }).collect();
    let mut wind = scaled(&p.wind);
    for (gi, &k) in p.gas.iter().enumerate() {
        let k = k as usize;
        let rain: &mut [f64] = match p.ks.iter().position(|&q| q == k) {
            Some(x) => &mut up[x * n..(x + 1) * n],
            None => &mut [],
        };
        hold_air(&w.store.amounts(Class::Air)[k * na..(k + 1) * na], (&links.a, &links.b), &mut wind[gi * nl..(gi + 1) * nl], (&parent, rain));
    }
    let World { light, life, .. } = w;
    for (i, (l, &got)) in light.iter_mut().zip(&p.light).enumerate() {
        *l += f * got;
        for &b in life.at.get(i).into_iter().flatten() {
            life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&sh| sh) - 1.0) * f * got;
        }
    }
    let mut cross = Crossing::new(w, p.ks.clone());
    for (x, _) in p.ks.iter().enumerate() {
        for i in (0..n).filter(|&i| up[x * n + i] != 0.0) {
            let heat = p.up_heat[x * n + i] * up[x * n + i] / p.up[x * n + i];
            cross.cross(s, x, (i, parent[i] as usize, area[i]), (up[x * n + i], heat), false);
        }
    }
    let (sun, heat, out, wind_heat, kept) = (scaled(&p.sun), scaled(&p.heat), scaled(&p.out), scaled(&p.wind_heat), scaled(&p.kept));
    let ids: Vec<u32> = (0..n as u32).collect();
    let (to_sun, space) = (vec![Boundary::Sun as u32; n], vec![Boundary::Space as u32; na]);
    let mut lift = Batch::falls(Class::Air);
    p.lift.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -f * q));
    let (mut links_x, mut lift_x) = cross.batches();
    let fluxes = [
        LinkFlux { class: [Class::Boundary, Class::Ground], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &sun },
        LinkFlux { class: [Class::Boundary, Class::Bodies], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &kept },
        LinkFlux { class: [Class::Ground, Class::Air], a: &ids, b: &parent, ks: &[], mol: &[], heat: &heat },
        LinkFlux { class: [Class::Air, Class::Boundary], a: &ids[..na], b: &space, ks: &[], mol: &[], heat: &out },
        links_x.flux(),
        lift_x.flux(),
        LinkFlux { class: [Class::Air; 2], a: &links.a, b: &links.b, ks: &p.gas, mol: &wind, heat: &wind_heat },
        lift.flux(),
    ];
    apply(w, &fluxes, &[], &[]);
    let ema = year_weight(w, secs);
    for ((r, &got), &a) in w.rain.iter_mut().zip(&p.rain).zip(&area) {
        follow(r, (got / (a * p.span as f64)) as f32, ema);
    }
    sums(w, s);
    s.state.pending = Pending { upto, ..p };
}

/// Passes of `hold_air` before every air node gives at most what it holds.
const CUT_PASSES: usize = 4;

/// Cuts what leaves each air node of one gas where the node, which holds `held`, would end with less than nothing (the
/// other laws took from it since the step): the wind out of it (`wind`, + from `a` to `b`, per link) and the rain of
/// its ground nodes (the negative `up`, per ground node, whose air node is `parent`), each by the share that leaves it
/// nothing. That takes from its neighbours what they would have got, so it is done again, `CUT_PASSES` times at most;
/// then every node gives at most what it holds (as `solve::spread::within`).
fn hold_air(held: &[f64], (a, b): (&[u32], &[u32]), wind: &mut [f64], (parent, up): (&[u32], &mut [f64])) {
    for pass in 0..=CUT_PASSES {
        let (mut end, mut out) = (held.to_vec(), vec![0.0; held.len()]);
        for ((&i, &j), &m) in a.iter().zip(b).zip(wind.iter()) {
            let (from, to) = if m > 0.0 { (i as usize, j as usize) } else { (j as usize, i as usize) };
            (end[from], end[to], out[from]) = (end[from] - m.abs(), end[to] + m.abs(), out[from] + m.abs());
        }
        for (&j, &u) in parent.iter().zip(up.iter()) {
            (end[j as usize], out[j as usize]) = (end[j as usize] + u, out[j as usize] + (-u).max(0.0));
        }
        if end.iter().all(|&e| e >= 0.0) {
            return;
        }
        let keep: Vec<f64> = (0..held.len())
            .map(|j| match (end[j] < 0.0 && out[j] > 0.0, pass == CUT_PASSES) {
                (_, true) if out[j] > 0.0 => (held[j] / out[j]).min(1.0),
                (true, false) => ((end[j] + out[j]) / out[j]).max(0.0),
                _ => 1.0,
            })
            .collect();
        for ((&i, &j), m) in a.iter().zip(b).zip(wind.iter_mut()) {
            *m *= keep[if *m > 0.0 { i } else { j } as usize];
        }
        for (&j, u) in parent.iter().zip(up.iter_mut()).filter(|(_, u)| **u < 0.0) {
            *u *= keep[j as usize];
        }
    }
}

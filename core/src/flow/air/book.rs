//! What an air step moves over its span, booked at once as it is solved: backward Euler knows it at the start, and the
//! other laws read what it leaves until the next step.

use super::super::{follow, year_weight, Scratch};
use super::exchange::cloud;
use super::newton::Wind;
use super::step::{Active, Iterate, FIXED, PER_GAS};
use super::wind::Links;
use super::{lerp, sums};
use crate::store::{apply, Batch, Boundary, Class, LinkFlux};
use crate::{World, AIR};

/// Book what the step of `dt` seconds moves, from its last round's rows, wind and air, in one call: sunlight, light set
/// aside for bodies, heat to the air and to space, the ground's gas with its heat, the wind and the lifts; then the
/// light on bodies, the rain and the clouds of what condensed in each air node (`rained`). No node gives more of a gas
/// than it holds and gets (`hold_air`).
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, rained): (&[Active], &[[f64; 2]]), (rows, width): (&[f64], usize), wind: &Wind, it: &Iterate, dt: f64) {
    let (n, na, cells) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells());
    let (parent, area) = (cells.parent().to_vec(), cells.area().to_vec());
    let v = |len: usize| vec![0.0; len];
    let (mut sun, mut heat, mut light, mut kept, mut rain, mut out) = (v(n), v(n), v(n), v(n), v(n), v(na));
    // Per gas and ground node (`[x * n + i]`): what goes up into its air node, mol, and the heat leaving with it, J.
    let (mut up, mut up_heat) = (v(act.len() * n), v(act.len() * n));
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (j, ts) = (parent[i] as usize, row[4]);
        (sun[i], heat[i], light[i], kept[i]) = (row[3], row[0] + row[2], row[5], row[6] * area[i] * dt);
        out[j] += row[2];
        for (x, ac) in act.iter().enumerate() {
            let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
            let r = row[o + 3];
            let m = row[o].min(w.store.amounts(Class::Ground)[ac.k * n + i] + r);
            (up[x * n + i], up_heat[x * n + i]) = (m - r, m * (sp.cp * ts + sp.lf + sp.lv) - r * (sp.cp * it.t[j] + sp.lf));
            rain[i] += r;
        }
    }
    for (x, ac) in act.iter().enumerate() {
        if up[x * n..(x + 1) * n].iter().any(|&u| u < 0.0) {
            s.add_loose(ac.k, false);
        }
    }
    // The clouds of what condensed, which the next step's sunlight passes.
    let refl: Vec<f64> = rained.iter().zip(w.mesh.level(AIR).area()).map(|(&r, &a)| cloud(w.cfg.cloud, (r[0], r[1]), a, dt)).collect();
    s.state.cloud = (0..n).map(|i| lerp(s, &refl, i)).collect();
    // The wind and the rain of each gas, cut where an air node would give more than it holds.
    let links = Links::of(w.mesh.level(AIR));
    let (nl, mut wind_mol, cut) = (links.a.len(), wind.mol.clone(), up.clone());
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let rain: &mut [f64] = match act.iter().position(|a| a.k == k) {
            Some(x) => &mut up[x * n..(x + 1) * n],
            None => &mut [],
        };
        hold_air(&w.store.amounts(Class::Air)[k * na..(k + 1) * na], (&links.a, &links.b), &mut wind_mol[gi * nl..(gi + 1) * nl], (&parent, rain));
    }
    let World { light: lit, life, .. } = w;
    for (i, (l, &got)) in lit.iter_mut().zip(&light).enumerate() {
        *l += got;
        for &b in life.at.get(i).into_iter().flatten() {
            life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&sh| sh) - 1.0) * got;
        }
    }
    // Each ground node's gas crosses the link to its air node, one column per gas, with its heat; the air pays the lift.
    let (g, view) = (w.cfg.gravity, s.view());
    let mut paid = wind.paid.clone();
    for (x, ac) in act.iter().enumerate() {
        for i in (0..n).filter(|&i| up[x * n + i] != 0.0) {
            let (m, j) = (up[x * n + i], parent[i] as usize);
            heat[i] += up_heat[x * n + i] * m / cut[x * n + i];
            paid[j] += m * s.sp[ac.k].mm * view.lift(i, j, g);
        }
    }
    for (x, ac) in act.iter().enumerate() {
        (0..n).filter(|&i| up[x * n + i] != 0.0).for_each(|i| s.note(ac.k, i, -up[x * n + i], area[i]));
    }
    let ids: Vec<u32> = (0..n as u32).collect();
    let (to_sun, space) = (vec![Boundary::Sun as u32; n], vec![Boundary::Space as u32; na]);
    let mut lift = Batch::falls(Class::Air);
    paid.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -q));
    let (ks, gas): (Vec<u16>, Vec<u16>) = (act.iter().map(|a| a.k as u16).collect(), s.state.gas.iter().map(|&k| k as u16).collect());
    let fluxes = [
        LinkFlux { class: [Class::Boundary, Class::Ground], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &sun },
        LinkFlux { class: [Class::Boundary, Class::Bodies], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &kept },
        LinkFlux { class: [Class::Ground, Class::Air], a: &ids, b: &parent, ks: &ks, mol: &up, heat: &heat },
        LinkFlux { class: [Class::Air, Class::Boundary], a: &ids[..na], b: &space, ks: &[], mol: &[], heat: &out },
        LinkFlux { class: [Class::Air; 2], a: &links.a, b: &links.b, ks: &gas, mol: &wind_mol, heat: &wind.heat },
        lift.flux(),
    ];
    apply(w, &fluxes, &[], &[]);
    let ema = year_weight(w, dt);
    for (((r, sum), &got), &a) in w.rain.iter_mut().zip(&mut w.rained).zip(&rain).zip(&area) {
        follow(r, (got / (a * dt)) as f32, ema);
        *sum += got / a;
    }
    sums(w, s);
}

/// Passes of `hold_air` before every air node gives at most what it holds.
const CUT_PASSES: usize = 4;

/// Cuts the wind and the rain out of each air node of one gas where it would end below zero, by the share that leaves
/// it nothing; as that takes from its neighbours, again, `CUT_PASSES` times at most (as `solve::spread::within`).
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

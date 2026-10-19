//! What an air step moves over its span, booked at once as it is solved: backward Euler knows it at the start, and the
//! other laws read what it leaves until the next step.

use super::super::{follow, year_weight, Scratch};
use super::exchange::cloud;
use super::newton::Wind;
use super::step::{Active, Iterate, Start, FIXED, PER_GAS};
use super::wind::Links;
use super::{lerp, shares, sums};
use crate::store::{apply, Batch, Boundary, Class, LinkFlux};
use crate::{World, AIR};

/// Book what the step of `dt` seconds moves, from its last round's rows, wind and air, in one call: sunlight, light set
/// aside for bodies, the heat each ground node gives its bodies' node, heat to the air and to space, the
/// ground's gas with its heat, the wind and the lifts; then the light on bodies, the rain and the clouds of what condensed in each air node (`rained`). No node gives more of a gas
/// than it holds and gets (`hold_air`).
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, rained): (&[Active], &[[f64; 2]]), (rows, width): (&[f64], usize), wind: &Wind, (it, start): (&Iterate, &Start), dt: f64) {
    let (n, na, cells) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells());
    let (parent, area) = (cells.parent().to_vec(), cells.area().to_vec());
    // The links of each ground node to the air (`[e * n + i]`): to its own air node (`e` 0), and to the four it sees.
    let ends: Vec<u32> = parent.iter().copied().chain((0..4).flat_map(|c| s.near.iter().map(move |j| j[c]))).collect();
    let (from, ne): (Vec<u32>, usize) = ((0..ends.len()).map(|e| (e % n) as u32).collect(), ends.len());
    let v = |len: usize| vec![0.0; len];
    let (mut sun, mut heat, mut light, mut kept, mut drawn, mut rain, mut out) = (v(n), v(ne), v(n), v(n), v(n), v(n), v(na));
    // Per gas and link (`[x * ne + e]`): what goes up, mol (what evaporates into its own air node, what rains out of each
    // node it sees by its share), and the heat that leaves the ground with it, J.
    let (mut up, mut up_heat) = (v(act.len() * ne), v(act.len() * ne));
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (j, ts) = (parent[i] as usize, row[4]);
        (sun[i], heat[i], light[i], kept[i], drawn[i]) = (row[3], row[0] + row[2], row[5], row[6] * area[i] * dt, row[7]);
        out[j] += row[2];
        for (x, ac) in act.iter().enumerate() {
            let (o, sp, e) = (FIXED + PER_GAS * x, s.sp[ac.k], x * ne);
            let r = row[o + 3];
            let m = row[o].min(w.store.amounts(Class::Ground)[ac.k * n + i] + r);
            (up[e + i], up_heat[e + i]) = (m, m * (sp.cp * ts + sp.lf + sp.lv));
            for (c, (q, share)) in shares(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i).into_iter().enumerate() {
                (up[e + (c + 1) * n + i], up_heat[e + (c + 1) * n + i]) = (-r * share, -r * share * (sp.cp * it.t[q] + sp.lf));
            }
            rain[i] += r;
        }
    }
    // The clouds of what condensed, which the next step's sunlight passes.
    let (refl, depth): (Vec<f64>, Vec<f64>) = rained.iter().zip(w.mesh.level(AIR).area()).map(|(&r, &a)| cloud((w.cfg.cloud, w.cfg.cloud_ir), (r[0], r[1]), a, dt)).map(|[r, d]| (r, d)).unzip();
    s.state.cloud = (0..n).map(|i| lerp(s, &refl, i)).collect();
    s.state.cloud_depth = (0..n).map(|i| lerp(s, &depth, i)).collect();
    // The wind and the rain of each gas, cut where an air node would give more than it holds.
    let links = Links::of(w.mesh.level(AIR));
    let (nl, mut wind_mol, cut) = (links.a.len(), wind.mol.clone(), up.clone());
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let rain = act.iter().position(|a| a.k == k).map_or(&mut [][..], |x| &mut up[x * ne..(x + 1) * ne]);
        hold_air(&start.x[gi * na..(gi + 1) * na], (&links.a, &links.b), &mut wind_mol[gi * nl..(gi + 1) * nl], (&ends, rain));
    }
    let World { light: lit, life, .. } = w;
    for (i, (l, &got)) in lit.iter_mut().zip(&light).enumerate() {
        *l += got;
        for &b in life.at.get(i).into_iter().flatten() {
            life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&sh| sh) - 1.0) * got;
        }
    }
    // Each ground node's gas crosses its links to the air, one column per gas, with its heat; the air pays the lift.
    let (g, view) = (w.cfg.gravity, s.view());
    let (mut paid, mut net) = (wind.paid.iter().zip(&start.lift).map(|(a, b)| a + b).collect::<Vec<f64>>(), v(act.len() * n));
    for (x, ac) in act.iter().enumerate() {
        for e in (0..ne).filter(|&e| up[x * ne + e] != 0.0) {
            let (m, i, j) = (up[x * ne + e], e % n, ends[e] as usize);
            heat[e] += up_heat[x * ne + e] * m / cut[x * ne + e];
            paid[j] += m * s.sp[ac.k].mm * view.lift(i, j, g);
            net[x * n + i] += m;
        }
    }
    for (x, ac) in act.iter().enumerate() {
        net[x * n..(x + 1) * n].iter().any(|&u| u < 0.0).then(|| s.add_loose(ac.k, false));
        (0..n).filter(|&i| net[x * n + i] != 0.0).for_each(|i| s.note(ac.k, i, -net[x * n + i], area[i]));
    }
    let ids: Vec<u32> = (0..n as u32).collect();
    // The gas of the bodies' nodes comes up whole, with the latent heat it holds; the air paid its lift (`Start`).
    let (all, bodies_gas): (Vec<u16>, Vec<f64>) = ((0..w.chem.ns() as u16).collect(), w.store.amounts(Class::Bodies).to_vec());
    let (to_sun, space) = (vec![Boundary::Sun as u32; n], vec![Boundary::Space as u32; na]);
    let mut lift = Batch::fallen(Class::Air, paid.iter().map(|q| -q));
    let (ks, gas): (Vec<u16>, Vec<u16>) = (act.iter().map(|a| a.k as u16).collect(), s.state.gas.iter().map(|&k| k as u16).collect());
    let fluxes = [
        LinkFlux { class: [Class::Boundary, Class::Ground], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &sun },
        LinkFlux { class: [Class::Boundary, Class::Bodies], a: &to_sun, b: &ids, ks: &[], mol: &[], heat: &kept },
        LinkFlux { class: [Class::Ground, Class::Bodies], a: &ids, b: &ids, ks: &[], mol: &[], heat: &drawn },
        LinkFlux { class: [Class::Ground, Class::Air], a: &from, b: &ends, ks: &ks, mol: &up, heat: &heat },
        LinkFlux { class: [Class::Bodies, Class::Air], a: &ids, b: &parent, ks: &all, mol: &bodies_gas, heat: &[] },
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
/// it nothing; as that takes from its neighbours, again, `CUT_PASSES` times at most (as `solve::spread::within`). `up`
/// is what comes up into air node `ends[e]` over each link `e` with the ground.
fn hold_air(held: &[f64], (a, b): (&[u32], &[u32]), wind: &mut [f64], (ends, up): (&[u32], &mut [f64])) {
    for pass in 0..=CUT_PASSES {
        let (mut end, mut out) = (held.to_vec(), vec![0.0; held.len()]);
        for ((&i, &j), &m) in a.iter().zip(b).zip(wind.iter()) {
            let (from, to) = if m > 0.0 { (i as usize, j as usize) } else { (j as usize, i as usize) };
            (end[from], end[to], out[from]) = (end[from] - m.abs(), end[to] + m.abs(), out[from] + m.abs());
        }
        for (&j, &u) in ends.iter().zip(up.iter()) {
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
        for (&j, u) in ends.iter().zip(up.iter_mut()).filter(|(_, u)| **u < 0.0) {
            *u *= keep[j as usize];
        }
    }
}

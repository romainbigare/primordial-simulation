//! What an air step moves, booked at once (laws.md 0, "The books"): the bodies' gas coming up (`up_from_bodies`), then
//! each cell's exchange with its air node at the step's mean air, the rain, the sunlight and the heat to space, the wind
//! and the evening of blocks, the lifts and falls, and the light of the cover (`book`).

use super::super::{follow, year_weight, Scratch};
use super::super::potential::apply;
use super::cells::{Response, KEPT, LIGHT, LINK, SPACE, SUN, UP};
use super::daily::Mean;
use super::rain::Landed;
use super::exchange::cloud;
use super::step::Active;
use super::{lerp, sums, touching};
use crate::store::{Batch, Boundary, Class, LinkFlux};
use crate::{World, AIR};

/// The gas the bodies' nodes hold comes up whole into their cells' air nodes, holding its latent heat as the air does;
/// the air pays its lift (laws.md 1.5, `potential`). The bodies' nodes hold only gases of the air.
pub(super) fn up_from_bodies(w: &mut World, s: &mut Scratch) {
    let n = w.mesh.cells().n();
    let held = w.store.amounts(Class::Bodies);
    let ks: Vec<u16> = s.state.gas.iter().copied().filter(|&k| held[k * n..(k + 1) * n].iter().any(|&m| m != 0.0)).map(|k| k as u16).collect();
    if ks.is_empty() {
        return;
    }
    let (cells, parent): (Vec<u32>, Vec<u32>) = ((0..n as u32).collect(), w.mesh.cells().parent().to_vec());
    let mol: Vec<f64> = ks.iter().flat_map(|&k| held[k as usize * n..(k as usize + 1) * n].iter().copied()).collect();
    let up = LinkFlux { class: [Class::Bodies, Class::Air], a: &cells, b: &parent, ks: &ks, mol: &mol, heat: &[] };
    apply(w, s, &[up], &[]);
}

/// Book what the step of `dt` seconds moves: over each cell's link to its air node what evaporates and its heat, at the
/// step's mean air (`Response::at`, the law the air's days summed); over the links to each node it sees its rain
/// (`rain`, per active gas (cell, node, mol)), reaching it at the temperature of the air touching it (`touching`), the
/// air paying what warms it on the way down, frozen or liquid as the node's days rained it; the sunlight it absorbs and keeps
/// for its cover, the heat it and its air send to space; and the wind and the evening of blocks, with their lifts and
/// falls (`potential`). Then the light on the cover, the clouds of what rained (which the next step's sunlight
/// passes), and the tape's rain and evaporation.
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, response, rain): (&[Active], &Response, &[Landed]), mean: &Mean, dt: f64) {
    let (n, na, nx, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), act.len(), w.mesh.cells().area().to_vec());
    // Each cell's outputs at the mean air it saw.
    let sr = &*s;
    let mut seen = vec![0.0; n * (1 + nx)];
    crate::par::rows(&mut seen, 1 + nx, || (), |_, i, row| {
        row[0] = lerp(sr, &mean.t, i);
        act.iter().enumerate().for_each(|(y, ac)| row[1 + y] = lerp(sr, &mean.x[ac.gi * na..(ac.gi + 1) * na], i));
    });
    let at = |i: usize, o: usize| response.at(i, o, seen[i * (1 + nx)], |y| seen[i * (1 + nx) + 1 + y]);
    let (cells, parent): (Vec<u32>, Vec<u32>) = ((0..n as u32).collect(), w.mesh.cells().parent().to_vec());
    let (ks, gases): (Vec<u16>, Vec<u16>) = (act.iter().map(|a| a.k as u16).collect(), s.state.gas.iter().map(|&k| k as u16).collect());
    let mol = crate::par::map_n(nx * n, |x| at(x % n, UP + x / n));
    let (link, to_space) = (crate::par::map_n(n, |i| at(i, LINK)), crate::par::map_n(n, |i| at(i, SPACE)));
    // The rain, at the air touching each cell, with the fusion enthalpy each mol of its node's rain keeps.
    let mut rained = Batch::new([Class::Ground, Class::Air], &ks);
    let mut fell = vec![0.0; nx * n];
    for (y, list) in rain.iter().enumerate() {
        for &(i, j, m) in list {
            let (at, sp) = (y * na + j as usize, s.sp[ks[y] as usize]);
            let per = sp.cp * touching(sr, &mean.t, i as usize) + mean.rain_fusion[at] / mean.rain[at];
            rained.add((i as usize, j as usize), ks[y], (-m, -m * per));
            fell[y * n + i as usize] += m;
        }
    }
    let (sun, space) = (Boundary::Sun as usize, Boundary::Space as usize);
    let (mut lit, mut kept, mut cooled) = (Batch::new([Class::Boundary, Class::Ground], &[]), Batch::new([Class::Boundary, Class::Bodies], &[]), Batch::new([Class::Ground, Class::Boundary], &[]));
    for (i, &out) in to_space.iter().enumerate() {
        lit.link((sun, i), response.fixed(i, SUN));
        cooled.link((i, space), out);
        if response.fixed(i, KEPT) > 0.0 {
            kept.link((sun, i), response.fixed(i, KEPT));
        }
    }
    let mut sky = Batch::new([Class::Air, Class::Boundary], &[]);
    for (j, &q) in mean.space.iter().enumerate() {
        sky.link((j, space), q);
    }
    let up = LinkFlux { class: [Class::Ground, Class::Air], a: &cells, b: &parent, ks: &ks, mol: &mol, heat: &link };
    let m = &mean.moved;
    let wind = LinkFlux { class: [Class::Air, Class::Air], a: &m.a, b: &m.b, ks: &gases, mol: &m.mol, heat: &m.heat };
    apply(w, s, &[up, rained.flux(), wind, lit.flux(), kept.flux(), cooled.flux(), sky.flux()], &[]);
    for (y, ac) in act.iter().enumerate() {
        let net = |i: usize| mol[y * n + i] - fell[y * n + i];
        (0..n).any(|i| net(i) < 0.0).then(|| s.add_loose(ac.k, false));
        (0..n).filter(|&i| net(i) != 0.0).for_each(|i| s.note(ac.k, i, -net(i), area[i]));
    }
    w.light.iter_mut().enumerate().for_each(|(i, l)| *l += response.fixed(i, LIGHT));
    s.state.cloud = (0..na)
        .map(|j| {
            let (m, mr) = act.iter().enumerate().map(|(y, ac)| (mean.rain[y * na + j], mean.rain[y * na + j] * s.sp[ac.k].refl[0])).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));
            cloud((w.cfg.cloud, w.cfg.cloud_ir), (m, mr), w.mesh.level(AIR).area()[j], dt)
        })
        .collect();
    let ema = year_weight(w, dt);
    for (i, ((r, sum), e)) in w.rain.iter_mut().zip(&mut w.rained).zip(&mut w.evap).enumerate() {
        let (got, up) = ((0..nx).map(|y| fell[y * n + i]).sum::<f64>(), (0..nx).map(|y| mol[y * n + i]).sum::<f64>());
        follow(r, (got / (area[i] * dt)) as f32, ema);
        follow(e, (up / (area[i] * dt)) as f32, ema);
        *sum += got / area[i];
    }
    sums(w, s);
}

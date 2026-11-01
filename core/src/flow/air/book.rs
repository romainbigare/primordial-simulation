//! What an air step moves, booked at once (laws.md 0, "The books"): the bodies' gas coming up (`up_from_bodies`), then
//! each cell's exchange with its air node at the step's mean air, the rain, the sunlight and the heat to space, the wind
//! and the evening of blocks, the lifts and falls, and the light of the cover (`book`).

use super::super::{follow, year_weight, Scratch};
use super::cells::{Response, KEPT, LIFT, LIGHT, LINK, SPACE, SUN, UP};
use super::daily::Mean;
use super::rain::Landed;
use super::exchange::cloud;
use super::step::Active;
use super::{lerp, sums};
use crate::store::{apply, Batch, Boundary, Class, LinkFlux};
use crate::{World, AIR};

/// The gas the bodies' nodes hold comes up whole into their cells' air nodes, holding its latent heat as the air does;
/// the air pays its lift (laws.md 1.5). The bodies' nodes hold only gases of the air.
pub(super) fn up_from_bodies(w: &mut World, s: &Scratch) {
    let (n, g) = (w.mesh.cells().n(), w.cfg.gravity);
    let held = w.store.amounts(Class::Bodies);
    let ks: Vec<u16> = s.state.gas.iter().copied().filter(|&k| held[k * n..(k + 1) * n].iter().any(|&m| m != 0.0)).map(|k| k as u16).collect();
    if ks.is_empty() {
        return;
    }
    let (cells, parent): (Vec<u32>, Vec<u32>) = ((0..n as u32).collect(), w.mesh.cells().parent().to_vec());
    let mol: Vec<f64> = ks.iter().flat_map(|&k| held[k as usize * n..(k as usize + 1) * n].iter().copied()).collect();
    let mut lift = vec![0.0; w.mesh.level(AIR).n()];
    for (q, &k) in ks.iter().enumerate() {
        for (i, &j) in parent.iter().enumerate() {
            lift[j as usize] += mol[q * n + i] * s.sp[k as usize].mm * s.view().lift(i, j as usize, g);
        }
    }
    let up = LinkFlux { class: [Class::Bodies, Class::Air], a: &cells, b: &parent, ks: &ks, mol: &mol, heat: &[] };
    apply(w, &[up, Batch::fallen(Class::Air, lift.iter().map(|q| -q)).flux()], &[]);
}

/// Book what the step of `dt` seconds moves: over each cell's link to its air node what evaporates and its heat, at the
/// step's mean air (`Response::at`, the law the air's days summed); over the links to each node it sees its rain
/// (`rain`, per active gas (cell, node, mol)) with the heat the node's days gave it; the sunlight it absorbs and keeps
/// for its cover, the heat it and its air send to space; the wind and the evening of blocks; and the lifts the air pays
/// and the falls that heat it. Then the light on the cover, the clouds of what rained (which the next step's sunlight
/// passes), and the tape's rain and evaporation.
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, response, rain): (&[Active], &Response, &[Landed]), mean: &Mean, dt: f64) {
    let (n, na, nx, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), act.len(), w.mesh.cells().area().to_vec());
    // Each cell's outputs at the mean air it saw.
    let sr = &*s;
    let seen: Vec<f64> = (0..n).flat_map(|i| std::iter::once(lerp(sr, &mean.t, i)).chain(act.iter().map(move |ac| lerp(sr, &mean.x[ac.gi * na..(ac.gi + 1) * na], i)))).collect();
    let at = |i: usize, o: usize| response.at(i, o, seen[i * (1 + nx)], |y| seen[i * (1 + nx) + 1 + y]);
    let (cells, parent): (Vec<u32>, Vec<u32>) = ((0..n as u32).collect(), w.mesh.cells().parent().to_vec());
    let (ks, gases): (Vec<u16>, Vec<u16>) = (act.iter().map(|a| a.k as u16).collect(), s.state.gas.iter().map(|&k| k as u16).collect());
    let mol: Vec<f64> = (0..nx).flat_map(|y| (0..n).map(move |i| (y, i))).map(|(y, i)| at(i, UP + y)).collect();
    let link: Vec<f64> = (0..n).map(|i| at(i, LINK)).collect();
    let mut paid = mean.moved.paid.clone();
    (0..n).for_each(|i| paid[parent[i] as usize] += at(i, LIFT));
    mean.fall.iter().zip(&mut paid).for_each(|(f, p)| *p -= f);
    // The rain, with the heat its nodes' days gave it, as each mol of a node's rain brings it.
    let mut rained = Batch::new([Class::Ground, Class::Air], &ks);
    let mut fell = vec![0.0; nx * n];
    for (y, list) in rain.iter().enumerate() {
        for &(i, j, m) in list {
            let per = mean.rain_heat[y * na + j as usize] / mean.rain[y * na + j as usize];
            rained.add((i as usize, j as usize), ks[y], (-m, -m * per));
            fell[y * n + i as usize] += m;
        }
    }
    let (sun, space) = (Boundary::Sun as usize, Boundary::Space as usize);
    let (mut lit, mut kept, mut cooled) = (Batch::new([Class::Boundary, Class::Ground], &[]), Batch::new([Class::Boundary, Class::Bodies], &[]), Batch::new([Class::Ground, Class::Boundary], &[]));
    for i in 0..n {
        lit.link((sun, i), response.fixed(i, SUN));
        cooled.link((i, space), at(i, SPACE));
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
    let mut lifts = Batch::fallen(Class::Air, paid.iter().map(|q| -q));
    apply(w, &[up, rained.flux(), wind, lit.flux(), kept.flux(), cooled.flux(), sky.flux(), lifts.flux()], &[]);
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

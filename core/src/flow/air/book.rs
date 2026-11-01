//! What an air step moves, booked at once (laws.md 0, "The books"): the bodies' gas coming up (`up_from_bodies`), then
//! each cell's exchange with its air node, the sunlight and the heat to space, the wind and the evening of blocks, the
//! lifts, and the light and heat of the bodies (`book`).

use super::super::{follow, year_weight, Scratch};
use super::cells::{EVAPORATED, GIVEN, KEPT, LIFT, LIGHT, LINK, RAINED, ROW, SPACE, SUN, TS};
use super::daily::Mean;
use super::exchange::cloud;
use super::step::Active;
use super::sums;
use crate::store::{apply, Batch, Boundary, Class, LinkFlux};
use crate::{survey::Draws, World, AIR};

/// The gas the bodies' nodes hold comes up whole into their cells' air nodes, holding its latent heat as the air does;
/// the air pays its lift (laws.md 1.5).
pub(super) fn up_from_bodies(w: &mut World, s: &Scratch) {
    let (n, g) = (w.mesh.cells().n(), w.cfg.gravity);
    let held = w.store.amounts(Class::Bodies);
    let ks: Vec<u16> = (0..w.chem.ns()).filter(|&k| held[k * n..(k + 1) * n].iter().any(|&m| m != 0.0)).map(|k| k as u16).collect();
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

/// Book what the step of `dt` seconds moves, from the cells' rows (`cells::pass`) and the air level's days (`Mean`):
/// over each cell's link to its air node its gas and heat; the sunlight it absorbs and keeps for its bodies, the heat it
/// and its air send to space and the heat its bodies draw; the wind and the evening of blocks; and the lifts the air
/// pays. Then the light on the bodies, the clouds of what rained (which the next step's sunlight passes), the tape's
/// rain and evaporation, and each body's share of what its node got (`Draws::owe`).
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, rows): (&[Active], &[f64]), mean: &Mean, draws: &Draws, dt: f64) {
    let (n, na, width, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), ROW + act.len(), w.mesh.cells().area().to_vec());
    let col = |c: usize| -> Vec<f64> { rows.iter().skip(c).step_by(width).copied().collect() };
    let (cells, parent): (Vec<u32>, Vec<u32>) = ((0..n as u32).collect(), w.mesh.cells().parent().to_vec());
    let (ks, gases): (Vec<u16>, Vec<u16>) = (act.iter().map(|a| a.k as u16).collect(), s.state.gas.iter().map(|&k| k as u16).collect());
    let (mol, link) = ((0..act.len()).flat_map(|x| col(ROW + x)).collect::<Vec<f64>>(), col(LINK));
    let mut paid = mean.moved.paid.clone();
    parent.iter().zip(col(LIFT)).for_each(|(&j, q)| paid[j as usize] += q);
    let (sun, space) = (Boundary::Sun as usize, Boundary::Space as usize);
    let mut lit = Batch::new([Class::Boundary, Class::Ground], &[]);
    let mut kept = Batch::new([Class::Boundary, Class::Bodies], &[]);
    let (mut cooled, mut drawn) = (Batch::new([Class::Ground, Class::Boundary], &[]), Batch::new([Class::Ground, Class::Bodies], &[]));
    for (i, row) in rows.chunks_exact(width).enumerate() {
        lit.link((sun, i), row[SUN]);
        cooled.link((i, space), row[SPACE]);
        if row[KEPT] > 0.0 {
            kept.link((sun, i), row[KEPT]);
        }
        if row[GIVEN] > 0.0 {
            drawn.link((i, i), row[GIVEN]);
        }
    }
    let mut sky = Batch::new([Class::Air, Class::Boundary], &[]);
    mean.space.iter().enumerate().for_each(|(j, &q)| (sky.link((j, space), q), ()).1);
    let up = LinkFlux { class: [Class::Ground, Class::Air], a: &cells, b: &parent, ks: &ks, mol: &mol, heat: &link };
    let m = &mean.moved;
    let wind = LinkFlux { class: [Class::Air, Class::Air], a: &m.a, b: &m.b, ks: &gases, mol: &m.mol, heat: &m.heat };
    let mut lifts = Batch::fallen(Class::Air, paid.iter().map(|q| -q));
    apply(w, &[up, wind, lit.flux(), kept.flux(), cooled.flux(), drawn.flux(), sky.flux(), lifts.flux()], &[]);
    for (x, ac) in act.iter().enumerate() {
        mol[x * n..(x + 1) * n].iter().any(|&u| u < 0.0).then(|| s.add_loose(ac.k, false));
        (0..n).filter(|&i| mol[x * n + i] != 0.0).for_each(|i| s.note(ac.k, i, -mol[x * n + i], area[i]));
    }
    let light = col(LIGHT);
    w.light.iter_mut().zip(&light).for_each(|(l, got)| *l += got);
    w.life.each_mut(|b| b.mark -= (b.shade - 1.0) * light[b.cell as usize]);
    s.state.cloud = (0..na)
        .map(|j| {
            let (m, mr) = act.iter().enumerate().map(|(x, ac)| (mean.rain[x * na + j], mean.rain[x * na + j] * s.sp[ac.k].refl[0])).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));
            cloud((w.cfg.cloud, w.cfg.cloud_ir), (m, mr), w.mesh.level(AIR).area()[j], dt)
        })
        .collect();
    let (ema, rained, evaporated) = (year_weight(w, dt), col(RAINED), col(EVAPORATED));
    for (i, ((r, sum), e)) in w.rain.iter_mut().zip(&mut w.rained).zip(&mut w.evap).enumerate() {
        follow(r, (rained[i] / (area[i] * dt)) as f32, ema);
        follow(e, (evaporated[i] / (area[i] * dt)) as f32, ema);
        *sum += rained[i] / area[i];
    }
    let (ts, given) = (col(TS), col(GIVEN));
    draws.owe(w, |i| ts[i], &given, dt);
    sums(w, s);
}

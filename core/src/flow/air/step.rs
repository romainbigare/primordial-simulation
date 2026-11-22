//! One step of the air (laws.md 1.4): the gas of the aside nodes comes up; each cell's response to
//! the air it sees is found (`cells::respond`); the air level runs day by day under its cells' law summed per node
//! (`daily::run`); and all of it is booked whole, each cell's exchange at the step's mean air (`book`).

use super::super::scratch::may_evaporate;
use super::super::{range, Scratch};
use super::book::{book, up_from_bodies};
use super::air_temp;
use super::cells::{respond, Ctx};
use super::rain::{landed, wet};
use super::rise::low_wind;
use super::daily::run;
use super::surf::patches;
use super::radiation::{declination, sun_height, Cloud, Column, Light};
use crate::{store::Class, units::DAY, World, AIR};

/// The saturation tables reach this far above the warmest node at a step's start, K: no node warms more in a step.
const SAT_REACH: f64 = 60.0;

/// A gas that may leave the ground or condense: its index in `Scratch::gas`, its substance, and whether it may
/// evaporate or condense.
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) gi: usize,
    pub(super) k: usize,
    pub(super) up: bool,
    pub(super) cond: bool,
}

/// The air's step of `dt` seconds.
pub(crate) fn step(w: &mut World, s: &mut Scratch, dt: f64) {
    up_from_bodies(w, s);
    // The gases that dissolve (at the liquid step) do not leave the ground.
    let henry = s.henry.clone();
    let act = active(w, s, &henry);
    // A body under each air node faces the air as one surface (`surf::patches`).
    let patches = patches(w, s);
    // The sun's declination at the middle of each day of the step.
    let days = (dt / DAY).ceil().max(1.0) as usize;
    let decl: Vec<f64> = (0..days).map(|d| declination(w.cfg.tilt, w.cfg.calendar().season(w.time + ((d as f64 + 0.5) * dt / days as f64) as u64))).collect();
    // The air at the step's start, after the bodies' gas: per node its temperature, K, and per active
    // gas and node its amount, mol.
    let na = w.mesh.level(AIR).n();
    let t0: Vec<f64> = (0..na).map(|j| air_temp(w.store.heat(Class::Air)[j], s.state.gas.iter().map(|&k| w.store.amounts(Class::Air)[k * na + j] * s.sp[k].cp).sum())).collect();
    let x0: Vec<f64> = act.iter().flat_map(|ac| w.store.amounts(Class::Air)[ac.k * na..(ac.k + 1) * na].iter().copied()).collect();
    let room = std::mem::take(&mut s.spare.response);
    let light = lights(w, s, &decl);
    let cx = Ctx { w, s, act: &act, t0: &t0, x0: &x0, dt, decl: &decl, light: &light };
    let response = respond(&cx, &patches, room);
    // The low wind over each node, which lifts the air over the ground and carries the drifting rain (`rise::low_wind`).
    let wind = low_wind(w, s);
    let ground = response.ground(&cx, wet(w, s, (&act, &wind), &t0, &patches));
    let mut draws = std::mem::take(&mut s.draws);
    let mean = run(w, s, (&act, &ground, &wind), dt, &mut draws);
    s.draws = draws;
    let rain = landed(&act, ground.wet(), &mean);
    book(w, s, (&act, &response, &rain, &ground.soaks), &mean, dt);
    s.spare.response = response.rows();
    w.low_wind = wind.iter().map(|v| v.map(|x| x as f32)).collect();
}

/// What the air of each node does to sunlight over the step (`Light`): its gases that absorb it (`Chemistry::sun_depth`)
/// or scatter it (`Substance::scatter`), per m² of the node, under the sun's mean height at its latitude
/// (`sun_height`); and over the share of its sky the cloud of the step before covers, that cloud (`Cloud`).
fn lights(w: &World, s: &Scratch, decl: &[f64]) -> Vec<Light> {
    let (air, chem) = (w.mesh.level(AIR), &w.chem);
    let na = air.n();
    let lit: Vec<usize> = s.state.gas.iter().copied().filter(|&k| chem.subs[k].sw_abs > 0.0 || chem.subs[k].scatter > 0.0).collect();
    (0..na)
        .map(|j| {
            let cols: Vec<(usize, f64)> = lit.iter().map(|&k| (k, w.store.amounts(Class::Air)[k * na + j] / air.area()[j])).collect();
            let depths = |slant: f64| cols.iter().fold([0.0; 2], |[a, b], &(k, c)| [a + chem.sun_depth(k, c * slant), b + chem.subs[k].scatter * c * slant]);
            let (mu, [share, depth]) = (sun_height(w.cfg.sun, air.lat()[j].to_radians(), decl), s.state.cloud[j]);
            Light::of(Column::of(depths, mu), (share > 0.0).then(|| (share, Cloud::of(depth, mu, w.cfg.drop_forward, w.cfg.drop_dark))))
        })
        .collect()
}

/// The gases that leave the ground or may condense (not those of `henry`, which dissolve), with their saturation
/// pressures up to the warmest ground or air node plus `SAT_REACH` (`Scratch::sat`).
fn active(w: &World, s: &mut Scratch, henry: &[(usize, usize)]) -> Vec<Active> {
    let tmax = range(&s.state.tg).1;
    let hi = tmax.max(range(&s.ta).1) + SAT_REACH;
    s.sat.resize(w.chem.ns(), Default::default());
    let mut act = Vec::new();
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let up = s.state.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        if up || s.cond[gi] {
            s.sat[k].fit(&w.chem, k, hi);
            act.push(Active { gi, k, up, cond: s.cond[gi] });
        }
    }
    act
}

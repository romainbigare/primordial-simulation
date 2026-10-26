//! Law 1: Flow. Stuff moves along links from high push to low push, with loss (laws.md 1). `scratch`: the law's
//! working memory; `land`: the state and surface of ground nodes; `liquid`, `drain`, `creep`; `air`: radiation,
//! exchange and wind in one step.

pub(crate) mod air;
mod creep;
mod drain;
mod land;
mod liquid;
mod scratch;
pub(crate) use air::air_temp;
pub use air::{column, declination, glow, insolation, scale_height};
pub use scratch::Scratch;
pub(crate) use scratch::FlowView;

use crate::{store::Class, AIR, World};
use land::ground_state;
use scratch::tables;

/// The slow-changing ground and the substances present are rechecked this often, months (`schedule`).
pub(crate) const REFRESH_MONTHS: u64 = 3;
/// Heat conducted sideways and creep (with `lanes::regrow`) run once in this many years, for all that time: on
/// cells of kilometres both are slow, and the implicit spread holds for any step.
pub(crate) const CREEP_YEARS: u64 = 1;

/// Each part of the law (`state`, `air`, `liquid`, `creep`, `refresh`) is a process of the clock (`schedule`). This one:
/// the state of ground and air nodes from the world now, which the other laws read until the next physics step (in full
/// at each liquid step, so a world resumed at a month's start goes on as an unbroken one).
pub fn state(w: &mut World, full: bool) {
    with(w, |w, s| {
        ground_now(w, s, full);
        air::sums(w, s);
        (0..w.mesh.level(AIR).n()).for_each(|j| s.ta[j] = air_temp(w.store.heat(Class::Air)[j], s.capa[j]));
    });
}

/// The state of ground nodes (`ground_state`): only what changed since it was last rebuilt, or every node with `full` or
/// when a refresh changed what is present.
fn ground_now(w: &World, s: &mut Scratch, full: bool) {
    let full = full || std::mem::take(&mut s.stale) || s.lay.len() != w.mesh.cells().n();
    ground_state(w, s, full);
}

/// The air's step for `dt` seconds (`air::step`), booked whole, from the state of ground and air nodes, after the
/// bodies' nodes settle what no body claims (`survey::Draws::settle`).
pub fn air(w: &mut World, dt: f64) {
    with(w, |w, s| {
        let draws = crate::survey::Draws::settle(w, dt);
        ground_now(w, s, false);
        air::state(w, s);
        air::step(w, s, dt, &draws);
    });
}

/// Liquid with its drainage and its work on the bed for `dt` seconds (`liquid::liquid`).
pub fn liquid(w: &mut World, dt: f64) {
    with(w, |w, s| liquid::liquid(w, s, dt));
}

/// Heat conducted sideways through the ground (`land::conduct`), then creep (`creep::creep`), for `dt` seconds.
pub fn creep(w: &mut World, dt: f64) {
    with(w, |w, s| {
        land::conduct(w, s, dt);
        creep::creep(w, s, dt);
    });
}

/// The slow ground and the substances present are rechecked (`scratch::refresh`).
pub fn refresh(w: &mut World) {
    with(w, |w, s| scratch::refresh(w, s));
}

/// Build the working memory of the law where it is missing (`with`).
pub(crate) fn ensure(w: &mut World) {
    with(w, |_, _| {});
}

/// Run `f` on the world and the law's working memory, built first where it is missing (a new or loaded world): its
/// tables, a new world's state, and its caches from the world now, as at a liquid step.
fn with(w: &mut World, f: impl FnOnce(&mut World, &mut Scratch)) {
    let n = w.mesh.cells().n();
    let mut s = std::mem::take(&mut w.scratch);
    if s.near.len() != n {
        tables(w, &mut s);
    }
    if s.state.tg.len() != n {
        scratch::refresh(w, &mut s);
        // A new world's ground nodes, before its first step: no plateau, no clouds.
        let st = &mut s.state;
        (st.tg, st.pk, st.phi, st.cloud, st.cloud_depth) = (vec![0.0; n], vec![land::NONE; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    }
    if s.cap.len() != n {
        scratch::room(w, &mut s);
        ground_state(w, &mut s, true);
        air::state(w, &mut s);
    }
    f(w, &mut s);
    w.scratch = s;
}

/// Lowest and highest of `v`.
fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

/// Share of carrying and creep the bodies on cell `i` leave there (1 when bare).
fn bare_share(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + w.life.sum(i)[1] / w.mesh.cells().area()[i] / w.cfg.phys.hold)
}

/// Weight of a step of `dt` seconds in a moving average over about a year.
fn year_weight(w: &World, dt: f64) -> f32 {
    (dt / w.cfg.calendar().year as f64) as f32
}

/// Move the moving average `avg` toward `x` by the weight `a` (`year_weight`).
#[inline]
fn follow(avg: &mut f32, x: f32, a: f32) {
    *avg += (x - *avg) * a;
}

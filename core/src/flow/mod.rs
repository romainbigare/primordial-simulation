//! Law 1: Flow. Stuff moves along edges from high push to low push, with loss.
//!
//! Ground nodes: liquid soaks into the pores of the loose ground (porosity times the volume of the
//! loose solids); what is over that stands and runs down the cells in the order of their heads in
//! one pass (`liquid`, by `solve::downhill`); the pore liquid and the store of the bed below, the
//! layers of each cell's stack, flow by Darcy's law (`drain`, by `solve::spread`), and drain to the
//! channels inside a cell (`Physics::hill`); the head lost heats the ground and does work, carrying
//! loose solids and breaking weak bonds; loose solids creep downhill; heat conducts sideways
//! through the ground's active layer. Air nodes
//! (`air`): radiation, contact heat, evaporation and condensation, and gas moved by pressure,
//! overturning and mixing, solved together over a step. Every move is booked by the store (`store::apply`): it takes from one
//! node exactly what it gives to another, with its heat; lifting matter takes heat and a fall gives
//! it back, the difference held by the store's potential node (`store::Batch::falls`, `World::potential`), so
//! elements and energy are kept.
//!
//! Files: `scratch` (the law's working memory and its tables), `land` (state, phases and surface of
//! ground nodes), `liquid`, `drain`, `creep`, and `air` (`state`, `step`, `radiation`, `exchange`, `wind`).

pub(crate) mod air;
mod creep;
mod drain;
mod land;
mod liquid;
mod scratch;
pub(crate) use air::air_temp;
pub use air::{column, glow, scale_height};
pub use scratch::Scratch;
pub(crate) use scratch::FlowView;

use crate::{World, AIR};
use land::ground_state;
use scratch::tables;

/// The slow-changing ground and the substances present are rechecked this often, months (`schedule`).
pub(crate) const REFRESH_MONTHS: u64 = 3;
/// Heat conducted sideways through the ground, and creep (with the regrowth of bodies), run once in this many years,
/// for all that time (`schedule`): on cells of kilometres both are slow, and the implicit spread holds for any step.
pub(crate) const CREEP_YEARS: u64 = 1;

/// Each part of the law runs on its own period, as the world's clock calls it (`schedule`, which writes their order
/// once): `state`, `air`, `liquid`, `creep` and `refresh`.
///
/// The state of ground nodes (`ground_state`: only what changed since it was last rebuilt, or every node with `full`, at
/// each liquid step, so a world resumed at a month's start goes on as an unbroken one, or when a refresh changed what
/// is present) and the temperature of air nodes, from the world now: what the other laws and the rest of Flow read
/// until the next physics step (`World::t_ground_last`). Then what the last air step moves over the next `ahead`
/// seconds booked (`air::dispense`), as a short step of the air would book it.
pub(crate) fn state(w: &mut World, ahead: u64, full: bool) {
    with(w, |w, s| {
        ground_now(w, s, full);
        air::sums(w, s);
        for j in 0..w.mesh.level(AIR).n() {
            s.ta[j] = air_temp(w.e_air[j], s.capa[j]);
        }
        air::dispense(w, s, w.time + ahead);
    });
}

/// The state of ground nodes (`ground_state`): only what changed since it was last rebuilt, or every node with `full` or
/// when a refresh changed what is present.
fn ground_now(w: &World, s: &mut Scratch, full: bool) {
    let full = full || std::mem::take(&mut s.stale) || s.lay.len() != w.mesh.cells().n();
    ground_state(w, s, full);
}

/// What the last air step moves up to now booked (`air::dispense`); the state of ground nodes (`ground_now`: at a
/// physics step `state` has just set it, and nothing has changed it since) and of air nodes (`air::state`); then the
/// air's step for `dt` seconds (`air::step`: radiation, gases dissolving, evaporation and rain, and the wind, together),
/// its first `ahead` seconds booked at once and the rest a physics step at a time (`state`).
pub(crate) fn air(w: &mut World, dt: f64, ahead: u64) {
    with(w, |w, s| {
        air::dispense(w, s, w.time);
        ground_now(w, s, false);
        air::state(w, s);
        air::step(w, s, dt);
        air::dispense(w, s, w.time + ahead);
    });
}

/// Liquid with its drainage and its work on the bed for `dt` seconds (`liquid::liquid`): it moves the rain that fell
/// since the last liquid step.
pub(crate) fn liquid(w: &mut World, dt: f64) {
    with(w, |w, s| liquid::liquid(w, s, dt));
}

/// Heat conducted sideways through the ground (`land::conduct`), then creep (`creep::creep`), for `dt` seconds.
pub(crate) fn creep(w: &mut World, dt: f64) {
    with(w, |w, s| {
        land::conduct(w, s, dt);
        creep::creep(w, s, dt);
    });
}

/// The slow ground and the substances present are rechecked (`scratch::refresh`).
pub(crate) fn refresh(w: &mut World) {
    with(w, |w, s| scratch::refresh(w, s));
}

/// Every part of the law but creep once, for `dt` seconds, in the clock's order: the state rebuilt in full, the air's
/// step booked whole, liquid, and the refresh; then the state the step leaves. A law of Flow on its own, for tests.
pub fn step(w: &mut World, dt: f64) {
    state(w, 0, true);
    air(w, dt, dt as u64);
    liquid(w, dt);
    refresh(w);
    state(w, 0, false);
}

/// Heat conducts sideways through the ground for `dt` seconds (`land::conduct`) from the world as it is, and nothing
/// else of Flow moves.
pub fn conduct(w: &mut World, dt: f64) {
    with(w, |w, s| {
        ground_state(w, s, false);
        land::conduct(w, s, dt);
    });
}

/// Run `f` on the world and the law's working memory (`ensure`d).
fn with(w: &mut World, f: impl FnOnce(&mut World, &mut Scratch)) {
    ensure(w);
    let mut s = std::mem::take(&mut w.scratch);
    f(w, &mut s);
    w.scratch = s;
}

/// Build the working memory of the law where it is missing (a new world, a loaded one, a test that calls another law
/// alone): its tables; for a new world its state (`scratch::State`); and its caches, from the world now as at a liquid
/// step. Every law then reads the same state of the last flow step.
pub(crate) fn ensure(w: &mut World) {
    let n = w.mesh.cells().n();
    let mut s = std::mem::take(&mut w.scratch);
    if s.near.len() != n {
        tables(w, &mut s);
    }
    if s.state.tg.len() != n {
        scratch::refresh(w, &mut s);
        scratch::fresh(w, &mut s);
    }
    if s.cap.len() != n {
        scratch::room(w, &mut s);
        ground_state(w, &mut s, true);
        air::state(w, &mut s);
    }
    w.scratch = s;
}

/// Lowest and highest of `v`.
fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

/// Share of carrying and creep the bodies on cell `i` leave there (1 when bare).
fn bare_share(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + w.life.sum.get(i).map_or(0.0, |s| s[1]) / w.mesh.cells().area()[i] / w.cfg.phys.hold)
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

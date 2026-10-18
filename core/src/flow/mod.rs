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
//! overturning and mixing. Every move is booked by the store (`store::apply`): it takes from one
//! node exactly what it gives to another, with its heat; lifting matter takes heat and a fall gives
//! it back, the difference held by the store's potential node (`store::Batch::falls`, `World::potential`), so
//! elements and energy are kept.
//!
//! Files: `scratch` (the law's working memory and its tables), `land` (state, phases and surface of
//! ground nodes), `liquid`, `drain`, `creep`, and `air` (`state`, `radiation`, `exchange`, `wind`).

pub(crate) mod air;
mod creep;
mod drain;
mod land;
mod liquid;
mod scratch;
pub(crate) use air::{air_temp, Sky};
pub use air::{column, glow, scale_height, wind_on};
pub use scratch::Scratch;
pub(crate) use scratch::FlowView;

use crate::units::DAY;
use crate::World;
use creep::creep;
use land::ground_state;
use liquid::liquid;
use scratch::{refresh, tables};

// The cadences of the law (the order of its parts is in `step`).
/// The slow-changing ground and the substances present are rechecked this often, months.
const REFRESH_MONTHS: u64 = 3;
/// A trace gas (`air::exchange`) evaporates and condenses only at every this many exchanges,
/// for all the time since.
const TRACE_TURN: u32 = 4;
/// Heat conducted sideways through the ground, and creep (with the regrowth of bodies), run once in
/// this many years, at the first liquid step of the year, for all that time: on cells of kilometres
/// both are slow, and the implicit spread holds for any step.
const CREEP_YEARS: u64 = 1;
/// A part of the law is due this many seconds before its period is full (so that a period made
/// of whole steps is not missed by rounding).
const DUE_SLACK: f64 = 1.0;

/// Advance flow by `dt` seconds. At every call: the state of ground nodes (`ground_state`) and of
/// air nodes (`air::state`), then radiation (`air::radiation`). Each other part runs once its own
/// period has passed, for all the time since: the exchange of the ground with the air
/// (`air::exchange`, every `cfg.exchange_days`), the wind (`air::wind`, every `cfg.air_days`),
/// liquid with its drainage and its work on the bed (`liquid`) every `cfg.flow_days`, and heat
/// conducted sideways through the ground (`land::conduct`) and creep every `CREEP_YEARS`
/// (`creep`); every `REFRESH_MONTHS` the slow
/// ground is rechecked (`refresh`).
pub fn step(w: &mut World, dt: f64) {
    ensure(w);
    let mut s = std::mem::take(&mut w.scratch);
    let cfg = &w.cfg;
    // The first step runs every part.
    let (first, month) = (s.month.is_none(), w.month());
    let monthly = s.month != Some(month);
    (s.pending_exc, s.pending, s.pending_liq) = (s.pending_exc + dt, s.pending + dt, s.pending_liq + dt);
    let exc_due = first || due(s.pending_exc, cfg.exchange_days as f64);
    let (air_due, liq_due) = (first || due(s.pending, cfg.air_days as f64), first || due(s.pending_liq, cfg.flow_days as f64));
    // The state of ground and air is rebuilt at liquid steps and when a refresh changed what is present.
    let full = liq_due || std::mem::take(&mut s.stale) || s.lay.len() != w.mesh.cells().n();
    ground_state(w, &mut s, full);
    air::state(w, &mut s, full);
    air::radiation(w, &mut s, dt);
    if exc_due {
        let dt = std::mem::take(&mut s.pending_exc);
        air::exchange(w, &mut s, dt);
    }
    if air_due {
        let dt = std::mem::take(&mut s.pending);
        air::wind(w, &mut s, dt);
    }
    if liq_due {
        let dt = std::mem::take(&mut s.pending_liq);
        liquid(w, &mut s, dt);
        let period = CREEP_YEARS * w.cfg.calendar().year;
        let year = w.time / period;
        if s.year != Some(year) {
            if s.year.is_some() {
                land::conduct(w, &mut s, period as f64);
                creep(w, &mut s, period as f64);
            }
            s.year = Some(year);
        }
    }
    if monthly && month % REFRESH_MONTHS == 0 {
        refresh(w, &mut s);
    }
    s.month = Some(month);
    w.scratch = s;
}

/// Heat conducts sideways through the ground for `dt` seconds (`land::conduct`) from the world as it is, and nothing
/// else of Flow moves.
pub fn conduct(w: &mut World, dt: f64) {
    ensure(w);
    let mut s = std::mem::take(&mut w.scratch);
    ground_state(w, &mut s, false);
    land::conduct(w, &mut s, dt);
    w.scratch = s;
}

/// Build the working memory of the law where it is missing (a new world, a resumed one, a test
/// that calls another law alone): its tables and, where it has none, the state of every ground and
/// air node from the world now. Every law then reads the same state of the last flow step.
pub(crate) fn ensure(w: &mut World) {
    let n = w.mesh.cells().n();
    let mut s = std::mem::take(&mut w.scratch);
    if s.near.len() != n {
        tables(w, &mut s);
    }
    let new = s.cap.len() != n;
    if new {
        refresh(w, &mut s);
    }
    if new {
        ground_state(w, &mut s, true);
        air::state(w, &mut s, true);
    }
    w.scratch = s;
}

/// Is a part with a period of `days` due, `pending` seconds after it last ran?
#[inline]
fn due(pending: f64, days: f64) -> bool {
    pending >= days * DAY - DUE_SLACK
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

//! Law 1: Flow. Stuff moves along edges from high push to low push, with loss.
//!
//! Ground nodes: liquid soaks into the pores of the loose ground (porosity times the volume of the
//! loose solids); what is over that stands and runs down its head in one ordered sweep; the pore
//! liquid flows by Darcy's law, and drains to the channels inside a cell (`Physics::hill`) and into
//! the store of the bed below (`drain`); the head lost heats the ground and does work, carrying
//! loose solids and breaking weak bonds; loose solids creep downhill. Air nodes
//! (`air`): radiation, contact heat, evaporation and condensation, and gas moved by pressure,
//! overturning and mixing. Every move takes from one node exactly what it gives to another, with
//! its heat, and books matter lifted or dropped in the gravity ledger, so elements and energy are kept.
//!
//! Files: `scratch` (the law's working memory and its tables), `land` (state, phases and surface of
//! ground nodes), `liquid`, `drain`, `creep`, and `air` (`state`, `radiation`, `exchange`, `wind`).

pub(crate) mod air;
mod creep;
mod drain;
mod land;
mod liquid;
mod scratch;
pub use air::{column, glow};
pub use scratch::Scratch;

use crate::units::DAY;
use crate::World;
use creep::creep;
use drain::drain;
use land::ground_state;
use liquid::liquid;
use scratch::{refresh, space, tables};

// The cadences of the law (the order of its parts is in `step`).
/// Moving liquid does work on its bed (carrying, wear) once in this many months (seven,
/// so the season sampled turns over the years).
const WORK_MONTHS: u64 = 7;
/// The slow-changing ground and the substances present are rechecked this often, months.
const REFRESH_MONTHS: u64 = 3;
/// A trace gas (`air::exchange`) evaporates and condenses only at every this many exchanges,
/// for all the time since.
const TRACE_TURN: u32 = 4;
/// Creep (with the regrowth of bodies) runs once in this many years, at
/// the first liquid step of the year, for all that time.
const CREEP_YEARS: u64 = 1;
/// A part of the law is due this many seconds before its period is full (so that a period made
/// of whole steps is not missed by rounding).
const DUE_SLACK: f64 = 1.0;

/// Advance flow by `dt` seconds. At every call: the state of ground nodes (`ground_state`) and of
/// air nodes (`air::state`), then radiation (`air::radiation`). Once `cfg.air_days` have passed,
/// for all the time since: the exchange of the ground with the air (`air::exchange`, every
/// `cfg.exchange_days`) and the wind (`air::wind`); once `cfg.flow_days` have passed, liquid
/// (`liquid`, its work on the bed every `WORK_MONTHS`) and drainage (`drain`), and creep every
/// `CREEP_YEARS` (`creep`); every `REFRESH_MONTHS` the slow ground is rechecked (`refresh`).
pub fn step(w: &mut World, dt: f64) {
    let mut s = std::mem::take(&mut w.scratch);
    if s.air_of.len() != w.grid.n() {
        tables(w, &mut s);
    }
    let first = s.cap.len() != w.grid.n();
    if first {
        refresh(w, &mut s);
    }
    if s.done.len() != w.grid.n() {
        space(w, &mut s);
    }
    (s.pending, s.pending_liq) = (s.pending + dt, s.pending_liq + dt);
    let (air_due, liq_due) = (first || due(s.pending, w.cfg.air_days as f64), first || due(s.pending_liq, w.cfg.flow_days as f64));
    // The state of ground and air is rebuilt at liquid steps and when a refresh changed what is present.
    let full = liq_due || std::mem::take(&mut s.stale) || s.lay.len() != w.grid.n();
    ground_state(w, &mut s, full);
    air::state(w, &mut s, full);
    air::radiation(w, &mut s, dt);
    if !air_due {
        w.scratch = s;
        return;
    }
    let (dt, month) = (std::mem::take(&mut s.pending), w.day / w.cfg.days_per_month as u64);
    let monthly = s.month != Some(month);
    s.pending_exc += dt;
    if first || due(s.pending_exc, w.cfg.exchange_days as f64) {
        let dt = std::mem::take(&mut s.pending_exc);
        air::exchange(w, &mut s, dt);
    }
    air::wind(w, &mut s, dt);
    s.owe_work |= monthly && month % WORK_MONTHS == 0;
    if liq_due {
        let dt = std::mem::take(&mut s.pending_liq);
        let work = if std::mem::take(&mut s.owe_work) { (WORK_MONTHS * w.cfg.days_per_month as u64) as f64 * DAY / dt } else { 0.0 };
        liquid(w, &mut s, dt, work);
        drain(w, &mut s, dt);
        let period = CREEP_YEARS * w.days_per_year();
        let year = w.day / period;
        if s.year != Some(year) {
            if s.year.is_some() {
                creep(w, &mut s, period as f64 * DAY);
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
    1.0 / (1.0 + w.life.sum.get(i).map_or(0.0, |s| s[1]) / w.grid.area() / w.cfg.phys.hold)
}

/// Weight of a step of `dt` seconds in a moving average over about a year.
fn year_weight(w: &World, dt: f64) -> f32 {
    (dt / (w.days_per_year() as f64 * DAY)) as f32
}

/// Move the moving average `avg` toward `x` by the weight `a` (`year_weight`).
#[inline]
fn follow(avg: &mut f32, x: f32, a: f32) {
    *avg += (x - *avg) * a;
}

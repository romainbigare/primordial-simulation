//! Law 1: Flow. Stuff moves along edges from high push to low push, with loss.
//!
//! Ground nodes (here): liquid soaks into the pores of the loose ground (porosity times the volume
//! of the loose solids); what is over that stands and runs down its head in one ordered sweep; the
//! pore liquid flows by Darcy's law, and drains to the channels inside a cell (`Physics::hill`) and into the
//! store of the bed below (`drain`); the head lost heats the ground and does work, carrying loose
//! solids and breaking weak bonds; loose solids creep downhill and heat conducts. Air nodes (`air`):
//! radiation, contact heat, evaporation and condensation, and gas moved by pressure, overturning
//! and mixing. Every move takes from one node exactly what it gives to another, with its heat, and
//! books matter lifted or dropped in the gravity ledger, so elements and energy are kept.

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

/// Moving liquid does work on its bed (carrying, wear) once in this many months (seven,
/// so the season sampled turns over the years).
const WORK_MONTHS: u64 = 7;
/// The slow-changing ground and the substances present are rechecked this often, months.
const REFRESH_MONTHS: u64 = 3;

/// What the bodies on cell `i` leave of carrying and creep there (1 when bare).
pub(super) fn held(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + w.life.sum.get(i).map_or(0.0, |s| s[1]) / w.grid.area() / w.cfg.phys.hold)
}

/// Advance flow by `dt` seconds: radiation every call; air and rain once `cfg.air_days` have
/// passed, liquid and creep once `cfg.flow_days` have, each for all the time since it last moved.
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
    let (air_due, liq_due) = (first || s.pending >= w.cfg.air_days as f64 * DAY - 1.0, first || s.pending_liq >= w.cfg.flow_days as f64 * DAY - 1.0);
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
    if first || s.pending_exc >= w.cfg.exchange_days as f64 * DAY - 1.0 {
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
        let year = w.day / w.days_per_year();
        if s.year != Some(year) {
            if s.year.is_some() {
                creep(w, &mut s, w.days_per_year() as f64 * DAY);
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

pub(super) fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

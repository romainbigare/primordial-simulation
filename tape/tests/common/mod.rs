//! What the tape's tests share: giving the recorder a month as the shell does.

use pcore::World;
use tape::{Recorder, Spent};

/// Record the month that has just ended: the world's events, at a year's end its counts, and no time spent.
pub fn record(rec: &mut Recorder, w: &mut World) {
    let events = w.drain_events();
    let taken = Recorder::wants_counters(w).then(|| w.take_counters());
    rec.record_month(w, events, taken, &Spent::default());
}

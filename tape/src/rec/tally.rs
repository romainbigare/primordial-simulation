//! What the recorder counts between its yearly rows (started again at each): the land's mean ground temperature at each
//! month's end, and the core's tallies of the cover taken at the yearly row.

use super::Recorder;
use pcore::event::{Event, EventKind};

/// The sums of the year so far.
#[derive(Default)]
pub(super) struct YearTally {
    /// The land's mean ground temperature at each month end, K, and the land cells it is of (listed at the first
    /// month of the year, `World::is_land`; none until then)
    pub t_land: Vec<f64>,
    pub land: Option<Vec<u32>>,
    /// Cover that died, kg, by cause (`Counters::cover_lost`)
    pub cover_lost: [f64; 6],
    /// The flows of the cover's budget, J (`Counters::cover_budget`)
    pub cover_budget: [f64; pcore::cover::BUDGET.len()],
    /// The flows of the seeds, kg (`Counters::seeds`)
    pub seeds: [f64; pcore::cover::SEED_FLOWS.len()],
}

impl Recorder {
    /// Count one event: the viewer's log shows the first appearance of a substance.
    pub(super) fn count(&mut self, e: &Event) {
        match e.kind {
            EventKind::FirstSubstance(k) => self.files.log.push((e.day, format!("substance {k} first appears"))),
        }
    }
}

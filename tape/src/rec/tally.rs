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
    pub cover_lost: [f64; pcore::life::DEATHS],
    /// The flows of the cover's budget, J (`Counters::cover_budget`)
    pub cover_budget: [f64; pcore::cover::BUDGET.len()],
    /// The flows of the seeds, kg (`Counters::seeds`)
    pub seeds: [f64; pcore::cover::SEED_FLOWS.len()],
    /// What flames burnt and killed, kg (`Counters::flame`), and the mol of each substance they gave the air
    pub flame: [f64; pcore::flame::FLOWS.len()],
    pub flame_air: Vec<f64>,
}

impl Recorder {
    /// Count one event: the viewer's log shows the first appearance of a substance and each edit (`World::edit`).
    pub(super) fn count(&mut self, e: &Event) {
        let text = match e.kind {
            EventKind::FirstSubstance(k) => format!("substance {k} first appears"),
            EventKind::Edit(edit) => format!("edit: {}", edit.line(e.node as usize, e.value > 0.0)),
        };
        self.files.log.push((e.day, text));
    }
}

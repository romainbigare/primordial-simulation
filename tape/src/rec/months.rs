//! The month: a row of the viewer's file each month (its year and month, which a resumed run checks its tape by), and
//! what the year counts at month ends: the land's mean ground temperature.

use crate::measure::area_mean;
use super::Recorder;
use pcore::{par, World};
use serde::{Deserialize, Serialize};

/// One row of the months the tape holds: the year and the month.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MonthRow {
    pub year: u64,
    pub month: u64,
}

/// The world's year and month of the year now.
pub(super) fn year_month(w: &World) -> (u64, u64) {
    (w.cfg.calendar().years(w.time), w.month() % w.cfg.months_per_year as u64)
}

impl Recorder {
    /// The month: its row, and the land's mean ground temperature as life feels it (`World::t_ground_last`), which the
    /// year's row reads (the land is listed again each year).
    pub(super) fn month(&mut self, w: &World) {
        let cells = w.mesh.cells().n();
        let land = self.year.land.get_or_insert_with(|| {
            let land = par::map_n(cells, par::UNCOUNTED, |i| w.is_land(i));
            (0..cells).filter(|&i| land[i]).map(|i| i as u32).collect()
        });
        self.year.t_land.push(area_mean(land.iter().map(|&i| (w.t_ground_last(i as usize), w.mesh.cells().area()[i as usize]))));
        let (year, month) = year_month(w);
        self.files.months.push(MonthRow { year, month });
    }
}

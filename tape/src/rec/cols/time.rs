//! The time the shell spent in each process of the year (`Spent`).

use super::Row;
use crate::rec::{Recorder, Spent, TIMED};

impl Recorder {
    /// Milliseconds a year in each process since the last yearly row; NaN where no time was handed in (the first row,
    /// a run with no clock).
    pub(super) fn time_cols(&mut self, spent: &Spent, row: &mut Row) {
        let ms: Vec<f64> = spent.iter().zip(&self.spent).map(|(now, before)| now.saturating_sub(*before).as_secs_f64() * 1e3).collect();
        let timed = ms.iter().any(|&x| x > 0.0);
        for (name, x) in TIMED.iter().zip(ms) {
            row.push(format!("ms_{name}"), "ms/yr", if timed { x } else { f64::NAN });
        }
        self.spent = *spent;
    }
}

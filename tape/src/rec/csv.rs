//! The two CSV files of a tape, `series.csv` and `months.csv` (the second only when the recorder is asked for it). Their
//! rows are only ever added, so a file the recorder wrote before gets the new rows at its end; it is written again whole
//! only when its header line changed (a column or a kind that is new) or when its writer has not written it yet (a
//! resumed run's first write cuts the file back to the rows the recorder holds).

use super::{months, TapeFiles};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// What the two files hold after their writer's last write: each file's header line and the number of rows under it.
#[derive(Default)]
pub struct Written {
    series: Option<(String, usize)>,
    months: Option<(String, usize)>,
}

/// Bring the file at `path` to `rows` rows under the `head` line, where `row(i)` is the text of row `i`. `held` says
/// what the file holds from this recorder's last write.
fn sync(path: &Path, head: String, rows: usize, row: impl Fn(usize) -> String, held: &mut Option<(String, usize)>) -> io::Result<()> {
    let from = held.as_ref().filter(|(h, n)| *h == head && *n <= rows).map_or(0, |(_, n)| *n);
    let (mut file, mut text) = match from {
        0 => (File::create(path)?, head.clone() + "\n"),
        _ => (OpenOptions::new().append(true).open(path)?, String::new()),
    };
    for i in from..rows {
        text += &row(i);
        text.push('\n');
    }
    file.write_all(text.as_bytes())?;
    *held = Some((head, rows));
    Ok(())
}

impl TapeFiles {
    /// Bring `series.csv` (one row a year) and, if the recorder keeps it, `months.csv` (one row a month) in `dir` up to
    /// the rows held.
    pub(super) fn write_csv(&self, dir: &Path, written: &mut Written) -> io::Result<()> {
        let series = &self.series;
        let head = series.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(",");
        let row = |i: usize| series.iter().map(|c| c.values[i].to_string()).collect::<Vec<_>>().join(",");
        sync(&dir.join("series.csv"), head, series.first().map_or(0, |c| c.values.len()), row, &mut written.series)?;
        if !self.months_csv {
            return Ok(());
        }
        let (rows, nk) = (&self.months, months::kinds(&self.months));
        sync(&dir.join("months.csv"), months::head(nk), rows.len(), |i| months::line(&rows[i], nk), &mut written.months)
    }
}

//! The yearly row of the series, by subject: the world (`world`), each starting kind (`kinds`), brains (`brains`),
//! what bodies do with and to each other (`social`), and made objects (`parts`). Units are SI unless named.

mod brains;
mod kinds;
mod parts;
mod social;
mod world;

use super::{Column, Recorder};
use crate::layers::Ctx;

/// The unit of the mineral columns counted in mol: 10^9 mol.
const GIGA: f64 = 1e9;

/// One yearly row: (name, unit, value) of each column, in the order they are written.
#[derive(Default)]
struct Row(Vec<(String, &'static str, f64)>);

impl Row {
    fn push(&mut self, name: impl Into<String>, unit: &'static str, value: f64) {
        self.0.push((name.into(), unit, value));
    }
}

/// `a` over `b`, NaN (a mean of nothing) where `b` is not above 0.
fn ratio(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a / b
    } else {
        f64::NAN
    }
}

impl Recorder {
    /// Write the yearly row, then start the sums of the year again.
    pub(super) fn push_series(&mut self, c: &Ctx) {
        // The land: solid surface above 0 m, as everywhere on the tape.
        let land: Vec<usize> = (0..c.w.grid.n()).filter(|&i| c.f.z[i] > 0.0).collect();
        let mut row = Row::default();
        self.world_cols(c, &land, &mut row);
        self.kind_cols(c, &mut row);
        self.brain_cols(c.w, &mut row);
        self.social_cols(c, &land, &mut row);
        self.parts_cols(c.w, &mut row);
        self.year = Default::default();
        self.updates = c.w.life.updates;
        self.append(row);
    }

    /// Add a row; a column new this year reads NaN in the years before, and one the row does not have is dropped.
    fn append(&mut self, row: Row) {
        if self.series.iter().map(|c| &c.name).ne(row.0.iter().map(|r| &r.0)) {
            let rows = self.series.first().map_or(0, |c| c.values.len());
            let mut old = std::mem::take(&mut self.series);
            self.series = row
                .0
                .iter()
                .map(|r| match old.iter().position(|c| c.name == r.0) {
                    Some(at) => old.swap_remove(at),
                    None => Column { name: r.0.clone(), unit: String::new(), values: vec![f64::NAN; rows] },
                })
                .collect();
        }
        for (col, (_, unit, v)) in self.series.iter_mut().zip(row.0) {
            if col.unit != unit {
                col.unit = unit.to_string();
            }
            col.values.push(v);
        }
    }
}

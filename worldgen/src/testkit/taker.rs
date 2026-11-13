//! A plain taker on the contract (`pcore::contract`), for the tests of the natural world: it stands for no animal and
//! uses no body code, as a later taker would.

use pcore::contract::{self, Layer};
use pcore::units::MAX_SUBSTANCES;
use pcore::World;

/// Each month (`month`) it takes the share `share` of what `layer` of cell `cell` holds of each substance
/// (`contract::take`) and passes all it took to the cell's ground with its heat (`contract::give`), as a gut passes
/// what it does not use. It counts what it took, mol of each substance, and the heat, J.
pub struct Taker {
    pub cell: usize,
    pub layer: Layer,
    pub share: f64,
    pub took: [f64; MAX_SUBSTANCES],
    pub heat: f64,
}

impl Taker {
    /// A taker of the share `share` of `layer` of cell `cell`, that took nothing yet.
    pub fn new(cell: usize, layer: Layer, share: f64) -> Taker {
        Taker { cell, layer, share, took: [0.0; MAX_SUBSTANCES], heat: 0.0 }
    }
    /// Its month: what it takes and passes on, as a serial step on its cell (`World::in_cell`). Returns what it took
    /// this month, mol of each substance.
    pub fn month(&mut self, w: &mut World) -> [f64; MAX_SUBSTANCES] {
        let (i, layer, share) = (self.cell, self.layer, self.share);
        let (took, heat) = w.in_cell(i, |w, d| {
            let (mut took, mut heat) = ([0.0; MAX_SUBSTANCES], 0.0);
            let Some(shown) = contract::shown(w, i, d.at(i), layer) else { return (took, heat) };
            for (k, &held) in shown.mol.iter().enumerate().filter(|e| *e.1 > 0.0) {
                let (got, q) = contract::take(w, d, i, layer, (k, share * held));
                (took[k], heat) = (got, heat + q);
            }
            contract::give(w, d, i, took.iter().copied().enumerate().filter(|e| e.1 > 0.0), heat);
            (took, heat)
        });
        self.took.iter_mut().zip(&took).for_each(|(a, b)| *a += b);
        self.heat += heat;
        took
    }
}

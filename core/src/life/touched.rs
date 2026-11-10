//! The cells the passes held (laws.md 0): a chunk's (`life::run`), a serial step's (`World::in_cell`) or an entry's of
//! the serial pass (`cross::apply`). Only a pass that holds a cell changes its life, so what follows a pass works on
//! the cells it held, not on every cell: the compaction (`Life::compact`) and the bodies' visible traits after a batch
//! (`survey::batch`).

use super::chunk::Of;

/// What the passes held: whether `log` lists the cells held since the last compaction, with repeats (not once a pass
/// held every cell).
#[derive(Clone, Debug, Default)]
pub(crate) struct Touched {
    listed: bool,
    log: Vec<u32>,
}

impl Touched {
    /// A pass holds the cells `of`.
    pub(crate) fn held(&mut self, of: Of) {
        match of {
            Of::All => self.listed = false,
            Of::Only(cells) => self.log.extend_from_slice(cells),
        }
        if !self.listed {
            self.log.clear();
        }
    }
    /// The cells held since the last compaction, in order, each once; none: every cell.
    pub(crate) fn since_compact(&self) -> Option<Vec<u32>> {
        self.listed.then(|| sorted(self.log.clone()))
    }
    /// The compaction ran: what it held starts again.
    pub(super) fn compacted(&mut self) {
        (self.listed, self.log) = (true, Vec::new());
    }
}

/// `v` in order, each once.
fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v.dedup();
    v
}

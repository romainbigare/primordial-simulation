//! The cells the passes held (laws.md 0): a chunk's (`life::run`), a serial step's (`World::in_cell`) or an entry's of
//! the serial pass (`cross::apply`). Only a pass that holds a cell changes its life, so what follows a pass works on
//! the cells it held, not on every cell: the compaction (`Life::compact`), the physics of the bodies that landed
//! (`survey::batch_of`) and the survey of a sub-step (`survey::substeps`).

use super::chunk::Of;

/// What the passes held: the count of passes and the pass that last held each cell; whether `log` lists the cells held
/// since the last compaction, with repeats (not once a pass held every cell or the cells changed); and the cells bodies
/// landed in or acts on parts ran in (`cross::land`), or whose physics left a body behind (`Chunk::behind`), since the
/// last pass that ran their physics on, with repeats: only there may a body's physics be behind its cell's step.
#[derive(Clone, Debug, Default)]
pub(crate) struct Touched {
    pub(crate) pass: u64,
    pub(crate) at: Vec<u64>,
    listed: bool,
    log: Vec<u32>,
    landed: Vec<u32>,
}

impl Touched {
    /// A pass holds the cells `of` of a world of `n` cells. A count of cells that changed counts as every cell held.
    pub(crate) fn held(&mut self, n: usize, of: Of) {
        self.pass += 1;
        if self.at.len() != n {
            (self.at, self.listed) = (vec![self.pass; n], false);
        }
        match of {
            Of::All => {
                self.at.fill(self.pass);
                self.listed = false;
            }
            Of::Only(cells) => {
                cells.iter().for_each(|&i| self.at[i as usize] = self.pass);
                self.log.extend_from_slice(cells);
            }
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
    /// Bodies landed in cell `i`, or physics left one there behind, or an act on parts ran there.
    pub(super) fn landed(&mut self, i: u32) {
        self.landed.push(i);
    }
    /// The cells bodies landed in or acts ran in since the last call, in order, each once (`every`: every cell's
    /// physics runs on, and none is left).
    pub(crate) fn take_landed(&mut self, every: bool) -> Vec<u32> {
        let landed = std::mem::take(&mut self.landed);
        if every { Vec::new() } else { sorted(landed) }
    }
}

/// `v` in order, each once.
fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v.dedup();
    v
}

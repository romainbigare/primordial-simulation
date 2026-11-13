//! What a snapshot keeps of `Life`: the cover of each cell, in the order of the cells. What stands on each cell and its
//! place are built again before any law reads them (`contract::stands`, `contract::surroundings`).

use super::{Cell, Life};
use crate::cover::Cover;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Life {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        to.collect_seq(self.cells.iter().map(|c| &c.cover))
    }
}

/// A loaded world's life: its cells' covers; what is built again comes back empty.
impl<'de> Deserialize<'de> for Life {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let cells: Vec<Cell> = Vec::<Cover>::deserialize(from)?.into_iter().map(|cover| Cell { cover }).collect();
        let stands = vec![Default::default(); cells.len()];
        Ok(Life { cells, stands, ..Default::default() })
    }
}

//! What a snapshot keeps of `Life` (`Saved`) and how it is written and read back.

use super::{Cell, Genome, Life};
use crate::body::Body;
use crate::lineage::Lineage;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// What a snapshot keeps of `Life`: per cell what `SavedCell` keeps; the lineages and born rows. Where bodies that moved
/// are follows from the bodies, and the survey's caches are rebuilt (`Life::sum`). Written from the world's own
/// (borrowed), read back as new (owned).
#[derive(Serialize, Deserialize)]
struct Saved<'a> {
    cells: Vec<SavedCell<'a>>,
    brains: Cow<'a, [Option<Lineage>]>,
    members: Cow<'a, [u32]>,
    genomes: Cow<'a, [Genome]>,
    free_genomes: Cow<'a, [u32]>,
}

/// A cell as a snapshot keeps it: its count of bodies placed, the energy movers spent pushing through its bodies since
/// the last survey and the members that moved into it since the last month, its cover, its living bodies in order and
/// their brains, its parts in order with their bonds, and its count of parts made.
#[derive(Serialize, Deserialize)]
struct SavedCell<'a> {
    placed: u64,
    pushed: f64,
    crossings: f32,
    cover: Cow<'a, crate::cover::Cover>,
    bodies: Vec<Cow<'a, Body>>,
    brains: Cow<'a, super::Brains>,
    parts: Cow<'a, [crate::bond::Part]>,
    bonds: Cow<'a, crate::bond::Bonds>,
    parts_made: u64,
}

impl Serialize for Life {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        let cells = self.cells.iter().map(|c| SavedCell { placed: c.placed, pushed: c.pushed, crossings: c.crossings, cover: Cow::Borrowed(&c.cover), bodies: c.bodies.iter().filter(|b| b.alive).map(Cow::Borrowed).collect(), brains: Cow::Borrowed(&c.brains), parts: Cow::Borrowed(&c.parts), bonds: Cow::Borrowed(&c.bonds), parts_made: c.parts_made }).collect();
        let (brains, members) = ((&self.brains).into(), (&self.members).into());
        Saved { cells, brains, members, genomes: (&self.genomes).into(), free_genomes: (&self.free_genomes).into() }.serialize(to)
    }
}

/// A loaded world stores its bodies anew, in the order saved; the caches come back empty.
impl<'de> Deserialize<'de> for Life {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let x = Saved::deserialize(from)?;
        let (brains, members) = (x.brains.into_owned(), x.members.into_owned());
        let mut life = Life { brains, members, genomes: x.genomes.into_owned(), free_genomes: x.free_genomes.into_owned(), ..Default::default() };
        for (i, c) in x.cells.into_iter().enumerate() {
            let bodies: Vec<Body> = c.bodies.into_iter().map(Cow::into_owned).collect();
            bodies.iter().for_each(|b| life.note_at(b.id, i as u32));
            let parts = c.parts.into_owned();
            parts.iter().for_each(|p| life.note_part_at(p.id, i as u32));
            let (cover, brains, bonds) = (c.cover.into_owned(), c.brains.into_owned(), c.bonds.into_owned());
            life.cells.push(Cell { bodies, placed: c.placed, pushed: c.pushed, crossings: c.crossings, cover, brains, parts, bonds, parts_made: c.parts_made });
        }
        life.sums = vec![[0.0; crate::survey::ROW]; life.cells.len()];
        Ok(life)
    }
}

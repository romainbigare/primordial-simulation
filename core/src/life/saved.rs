//! What a snapshot keeps of `Life` (`Saving`, read back as `Saved`) and how it is written and read back.

use super::{Brains, Cell, Genome, Life};
use crate::body::Body;
use crate::lineage::Lineage;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// What a snapshot keeps of `Life`: per cell what `SavedCell` keeps; the serial the next cell made takes; the lineages
/// and born rows. The index of each serial and where bodies that moved are follow from the cells (`Life::index`), and
/// the survey's caches are rebuilt (`Life::sum`). Written from the world's own
/// (borrowed, each cell made as it is written, `Cells`), read back as new (owned, `Saved`).
#[derive(Serialize)]
struct Saving<'a> {
    cells: Cells<'a>,
    next_serial: u32,
    brains: &'a [Option<Lineage>],
    members: &'a [u32],
    genomes: &'a [Genome],
    free_genomes: &'a [u32],
}

/// `Saving` as it is read back: the same fields in the same order.
#[derive(Deserialize)]
struct Saved {
    cells: Vec<SavedCell<'static>>,
    next_serial: u32,
    brains: Vec<Option<Lineage>>,
    members: Vec<u32>,
    genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
}

/// The cells of a world, each made as `SavedCell` keeps it while the snapshot is written, so that only one cell's
/// packed brains (`Brains::packed`) are held at a time.
struct Cells<'a>(&'a [Cell]);

impl Serialize for Cells<'_> {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        to.collect_seq(self.0.iter().map(SavedCell::of))
    }
}

/// A cell as a snapshot keeps it: its serial, its count of bodies placed, the energy movers spent pushing through its bodies since
/// the last survey and the members that moved into it since the last month, its cover, its living bodies in order and
/// their brains (their blocks one after another, none free: `Brains::packed`), its parts in order with their bonds, and
/// its count of parts made.
#[derive(Serialize, Deserialize)]
struct SavedCell<'a> {
    serial: u32,
    placed: u64,
    pushed: f64,
    crossings: f32,
    cover: Cow<'a, crate::cover::Cover>,
    bodies: Vec<Cow<'a, Body>>,
    brains: Cow<'a, [f32]>,
    parts: Cow<'a, [crate::bond::Part]>,
    bonds: Cow<'a, crate::bond::Bonds>,
    parts_made: u64,
}

impl<'a> SavedCell<'a> {
    /// Cell `c` as a snapshot keeps it: its own numbers where its brains lie packed already, else its bodies with their
    /// blocks where packing puts them.
    fn of(c: &'a Cell) -> SavedCell<'a> {
        let living: Vec<&Body> = c.bodies.iter().filter(|b| b.alive).collect();
        let held: Vec<_> = living.iter().map(|b| b.brain).collect();
        let (bodies, brains) = match c.brains.packed(&held) {
            None => (living.into_iter().map(Cow::Borrowed).collect(), Cow::Borrowed(c.brains.numbers())),
            Some((v, blocks)) => (living.into_iter().zip(blocks).map(|(b, k)| Cow::Owned(Body { brain: k, ..b.clone() })).collect(), Cow::Owned(v)),
        };
        SavedCell { serial: c.serial, placed: c.placed, pushed: c.pushed, crossings: c.crossings, cover: Cow::Borrowed(&c.cover), bodies, brains, parts: Cow::Borrowed(&c.parts), bonds: Cow::Borrowed(&c.bonds), parts_made: c.parts_made }
    }
}

impl Serialize for Life {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        Saving { cells: Cells(&self.cells), next_serial: self.next_serial, brains: &self.brains, members: &self.members, genomes: &self.genomes, free_genomes: &self.free_genomes }.serialize(to)
    }
}

/// A loaded world stores its bodies anew, in the order saved; the caches come back empty.
impl<'de> Deserialize<'de> for Life {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let x = Saved::deserialize(from)?;
        let mut life = Life { next_serial: x.next_serial, brains: x.brains, members: x.members, genomes: x.genomes, free_genomes: x.free_genomes, ..Default::default() };
        for c in x.cells {
            let (bodies, parts) = (c.bodies.into_iter().map(Cow::into_owned).collect(), c.parts.into_owned());
            let (cover, brains, bonds) = (c.cover.into_owned(), Brains::from_packed(c.brains.into_owned()), c.bonds.into_owned());
            life.cells.push(Cell { serial: c.serial, bodies, placed: c.placed, pushed: c.pushed, crossings: c.crossings, cover, brains, parts, bonds, parts_made: c.parts_made });
        }
        life.index();
        (life.stands, life.sums) = (vec![Default::default(); life.cells.len()], vec![[0.0; crate::survey::ROW]; life.cells.len()]);
        Ok(life)
    }
}

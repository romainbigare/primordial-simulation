//! Split and join of cells between steps, at a month's start (laws.md 0): the tree's change (`Square::splits`,
//! `Square::joins`), one remap of the cell table each (`World::remap`), the relief a join flattens kept and a split's
//! given back, and an event each.

use super::{Joined, Made, Of, Remap, Renumber, Source};
use crate::event::{Event, EventKind};
use crate::space::{square::Square, tree, Leaf, Mesh};
use crate::store::{fall_heat, Class};
use crate::World;

/// Why a split or a join is refused, changing nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The tree refuses it (`space::tree::Refused`)
    Shape(tree::Refused),
    /// The cell holds life (bodies, cover or parts), which does not follow a split or a join yet
    Life(usize),
}

impl World {
    /// Split cell `leaf` into its quarters (`Square::splits`): first each cell across a side that would border them two
    /// depths coarser, by its own splits, each split a remap of its own (`split_one`). Returns the splits made, in
    /// order. Refused where the tree refuses it or a cell to split holds life.
    pub fn split(&mut self, leaf: Leaf) -> Result<Vec<Leaf>, Refused> {
        let splits = self.map.splits(leaf).map_err(Refused::Shape)?;
        self.lifeless(splits.iter().filter_map(|&l| self.map.cell(l)))?;
        splits.iter().for_each(|&l| self.split_one(l));
        Ok(splits)
    }

    /// Join the quarters of `parent` into it (`Square::joins`), one remap: it holds their sum, and keeps each one's
    /// shares and, as its relief, what the join flattens (the potential energy of their stacks less its own) with the
    /// relief they kept from their own joins (`Joined`). Refused where the tree refuses it or a quarter holds life.
    pub fn join(&mut self, parent: Leaf) -> Result<(), Refused> {
        let kids = self.map.joins(parent).map_err(Refused::Shape)?;
        self.lifeless(kids)?;
        let air = self.mesh.cells().parent()[kids[0]] as usize;
        let before = crate::flow::potential_of(self, &kids, air);
        let kept: f64 = kids.iter().filter_map(|&i| self.joined[i].as_ref()).map(|r| r.relief).sum();
        let map = self.map.join(parent);
        let mesh = World::mesh_of(&map, &self.cfg);
        let j = map.cell(parent).expect("the joined cell");
        let area = kids.map(|i| self.mesh.cells().area()[i] / mesh.cells().area()[j]);
        let mut record = Joined::of(self, kids, area);
        let made = Source::Made(Made::Join { kids: kids.map(|i| i as u32), area });
        let from = map.leaves().iter().map(|&l| if l == parent { made } else { Source::Old(self.map.cell(l).expect("a cell") as u32) }).collect();
        self.renew(map, mesh, from);
        record.relief = kept + before - crate::flow::potential_of(self, &[j], air);
        self.joined[j] = Some(record);
        self.events.push(Event { day: self.day(), kind: EventKind::Join, node: self.life.cells[j].serial, value: parent.depth as f64 });
        Ok(())
    }

    /// Split cell `leaf` into its quarters, one remap: each holds its share of what the cell held, by the cell's record
    /// where a join made it (`Joined`), else by area (flat quarters: the potential energy of the stacks does not
    /// change). A record's split gives back its relief less what the quarters' stacks hold over the cell's, to the
    /// quarters' ground heat by their shares of it (`store::fall_heat`, which books it against `Store::lifted`).
    fn split_one(&mut self, leaf: Leaf) {
        let i = self.map.cell(leaf).expect("a cell");
        let (air, record, serial) = (self.mesh.cells().parent()[i] as usize, self.joined[i].clone(), self.life.cells[i].serial);
        let before = record.as_ref().map(|_| crate::flow::potential_of(self, &[i], air));
        let (map, quarters) = (self.map.split(leaf), leaf.quarters());
        let mesh = World::mesh_of(&map, &self.cfg);
        let from = (map.leaves().iter().enumerate())
            .map(|(j, &l)| match quarters.iter().position(|&q| q == l) {
                Some(q) => Source::Made(Made::Quarter { parent: i as u32, q: q as u8, area: mesh.cells().area()[j] / self.mesh.cells().area()[i] }),
                None => Source::Old(self.map.cell(l).expect("a cell") as u32),
            })
            .collect();
        let kids = quarters.map(|q| map.cell(q).expect("a quarter"));
        self.renew(map, mesh, from);
        if let (Some(r), Some(before)) = (record, before) {
            let give = r.relief - (crate::flow::potential_of(self, &kids, air) - before);
            let mut q = vec![0.0; self.mesh.cells().n()];
            kids.iter().zip(0..).for_each(|(&c, k)| q[c] = give * r.share(Of::Heat(Class::Ground), k));
            fall_heat(self, Class::Ground, &q);
        }
        self.events.push(Event { day: self.day(), kind: EventKind::Split, node: serial, value: leaf.depth as f64 });
    }

    /// The world on `map` and its `mesh`, its cells from the old ones by `from` (`Renumber`), each link between two old
    /// cells keeping what it remembers and a new one remembering nothing (`World::remap`).
    fn renew(&mut self, map: Square, mesh: Mesh, from: Vec<Source>) {
        let (old, cells) = (self.mesh.cells(), mesh.cells());
        let was = |c: u32| if let Source::Old(i) = from[c as usize] { Some(i as usize) } else { None };
        let links = (0..cells.a().len()).map(|l| was(cells.a()[l]).zip(was(cells.b()[l])).and_then(|(a, b)| old.link(a, b)).map(|x| x as u32)).collect();
        let r = Remap { links: Renumber::new(old.a().len(), links), cells: Renumber::of(old.n(), from) };
        (self.map, self.mesh) = (map, mesh);
        self.remap(&r);
    }

    /// Refused where one of `cells` holds life: a living body, cover or a part.
    fn lifeless(&self, cells: impl IntoIterator<Item = usize>) -> Result<(), Refused> {
        let lives = |i: &usize| {
            let c = self.life.cell_at(*i);
            c.bodies.iter().any(|b| b.alive) || c.cover.any() || !c.parts.is_empty()
        };
        cells.into_iter().find(lives).map_or(Ok(()), |i| Err(Refused::Life(i)))
    }
}

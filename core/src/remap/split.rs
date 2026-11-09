//! Split and join of cells between steps, at a month's start (laws.md 0): the tree's change (`Square::splits`,
//! `Square::joins`), one remap of the cell table each (`World::remap_cells`, which shares out and gathers the life of
//! the cells, `Life::remap`), the relief a join flattens kept and a split's spent on its quarters' terrain or given
//! back, and an event each. A joined cell's cover lands by the cover's rule and its bodies merge as the monthly merge
//! does. The splits and joins of one change (`Change`: one split or join, or a month's refinement) read the cells'
//! places between them, not their links: the mesh is linked, and the links take their new numbers, once at its end
//! (`World::changed`).

use super::{sum4, Joined, Made, Of, Renumber, Source};
use crate::event::{Event, EventKind};
use crate::space::{square::Square, tree::Refused, Leaf, Mesh};
use crate::store::{fall_heat, Class};
use crate::World;
use std::collections::HashMap;

/// A change of the cells under way (`World::change`): the count of links it found, and of each link now the link it
/// was then, where each split and join between kept it (a link between two cells that both stay keeps what it
/// remembers, a new one nothing). Meanwhile the world's mesh has its cells' places and the ends of their links, not
/// linked (`Square::mesh_over`); the change links it at its end (`World::changed`).
pub(crate) struct Change {
    links: usize,
    was: Option<Vec<Option<u32>>>,
}

impl World {
    /// A change of the cells, from the cells now, on Flow's state of the ground (built here where it is not yet, as in
    /// a world as built: a split stands its quarters on the terrain inside it, as far as the stacks' potential energy,
    /// which the state's phases give, lets the cell's relief pay for it).
    pub(crate) fn change(&mut self) -> Change {
        if !crate::flow::stated(self) {
            crate::flow::state(self, true);
        }
        Change { links: self.mesh.cells().a().len(), was: None }
    }

    /// The end of change `c`: the cells linked (`Square::mesh_over`), and each link takes its number (`store::Links`).
    pub(crate) fn changed(&mut self, c: Change) {
        let Some(was) = c.was else { return };
        let mesh = self.mesh_for(&self.map, true);
        assert!(mesh.cells().a() == self.mesh.cells().a() && mesh.cells().b() == self.mesh.cells().b(), "the links the change found");
        self.links.remap(&Renumber::new(c.links, was));
        self.mesh = mesh;
    }

    /// Split cell `leaf` into its quarters (`Square::splits`): first each cell across a side that would border them two
    /// depths coarser, by its own splits, each split a remap of its own (`split_one`), one change. Returns the splits
    /// made, in order. Refused where the tree refuses it.
    pub fn split(&mut self, leaf: Leaf) -> Result<Vec<Leaf>, Refused> {
        let mut c = self.change();
        let made = self.split_in(leaf, &mut c);
        self.changed(c);
        made
    }

    /// `split`, a part of change `c`.
    pub(crate) fn split_in(&mut self, leaf: Leaf, c: &mut Change) -> Result<Vec<Leaf>, Refused> {
        let splits = self.map.splits(leaf)?;
        splits.iter().for_each(|&l| self.split_one(l, c));
        Ok(splits)
    }

    /// Join the quarters of `parent` into it (`Square::joins`), one remap: it holds their sum, and keeps each one's
    /// shares and relief (`Joined`), and passes on what leaves it of what they pass on (`join_flux`); its relief is theirs and what the join flattens (the potential energy of their
    /// stacks less its own). Their life comes together (`Life::remap`); then the slots of their cover land in its cover
    /// as seeds do (`cover::land`: what finds no slot goes to its ground as remains) and its bodies merge (`merge_in`). A
    /// merge that lets go of a part records that in an event naming the cell by its index, so the shell takes the events
    /// before the next split or join. Refused where the tree refuses it. One change.
    pub fn join(&mut self, parent: Leaf) -> Result<(), Refused> {
        let mut c = self.change();
        let made = self.join_in(parent, &mut c);
        self.changed(c);
        made
    }

    /// `join`, a part of change `c`.
    pub(crate) fn join_in(&mut self, parent: Leaf, c: &mut Change) -> Result<(), Refused> {
        let kids = self.map.joins(parent)?;
        let air = self.mesh.cells().parent()[kids[0]] as usize;
        let before = crate::flow::potential_of(self, &kids, air);
        let map = self.map.join(parent);
        let mesh = self.mesh_for(&map, false);
        let j = map.cell(parent).expect("the joined cell");
        let area = kids.map(|i| self.mesh.cells().area()[i] / mesh.cells().area()[j]);
        let (record, flux) = (Joined::of(self, kids, area), self.join_flux(kids, parent));
        let made = Source::Made(Made::Join { kids: kids.map(|i| i as u32), area });
        let from = map.leaves().iter().map(|&l| if l == parent { made } else { Source::Old(self.map.cell(l).expect("a cell") as u32) }).collect();
        self.renew((map, mesh), from, c);
        self.relief[j] = sum4(record.kept) + before - crate::flow::potential_of(self, &[j], air);
        self.joined[j] = Some(record);
        self.liquid_flux[j] = flux as f32;
        self.events.push(Event { day: self.day(), kind: EventKind::Join, node: self.life.cells[j].serial, value: parent.depth as f64 });
        crate::cover::land(self);
        self.merge_in(j);
        self.seen(&[j]);
        Ok(())
    }

    /// Split cell `leaf` into its quarters, one remap: each holds its share of what the cell held (`Made::held`), and
    /// its share of the cell's life (`Life::remap`), and passes on what the terrain's channels lead through it of what
    /// the cell passes on (`split_flux`); the cell's relief pays what the quarters' stacks hold over the
    /// cell's and the quarters' reliefs. Where a join made the cell, by its record (`Joined`), and each quarter takes back
    /// its relief; what is left (or short) goes to the quarters' ground heat by their shares (`store::fall_heat`, which
    /// books it against `Store::lifted`). Else by area but the bed, which stands as the terrain inside the cell does,
    /// drained (`World::uneven`), and the liquid that stood over the cell's pores stands at one level over the quarters'
    /// floors (`flow::stand_level`); each quarter with the relief of the terrain inside it and its share by area of what
    /// is left: the potential energy of the pits the drainage filled and what the liquid gave up as it found its level,
    /// which the ground finer than the quarters still holds. Each quarter faces as its ground stands (`World::rise_south`):
    /// as it was joined, or the cell's rise plus the terrain's departure inside it at the split's scale, so their mean by
    /// area is the cell's.
    fn split_one(&mut self, leaf: Leaf, c: &mut Change) {
        let i = self.map.cell(leaf).expect("a cell");
        let (air, record, serial) = (self.mesh.cells().parent()[i] as usize, self.joined[i].clone(), self.life.cells[i].serial);
        let (relief, before, rise) = (self.relief[i], crate::flow::potential_of(self, &[i], air), self.rise_south[i]);
        let flux = self.split_flux(i, leaf);
        let (map, quarters) = (self.map.split(leaf), leaf.quarters());
        let mesh = self.mesh_for(&map, false);
        let kids = quarters.map(|q| map.cell(q).expect("a quarter"));
        let share = kids.map(|j| mesh.cells().area()[j] / self.mesh.cells().area()[i]);
        let uneven = record.is_none().then(|| self.uneven((i, leaf), (mesh.cells(), kids)));
        let bed = uneven.as_ref().map_or(share, |u| u.bed);
        // The shares of the cell's own ground, as the split stands it (`inside`).
        let means = crate::flow::means(self, &[i]).map_or(Default::default(), |m| m[0]);
        let (ground, inside) = self.inside(i, uneven.as_ref().map_or(1.0, |u| u.scale), means);
        let cover = self.life.cells[i].cover.parts(&ground, &inside, &self.cfg.phys);
        let from = (map.leaves().iter())
            .map(|&l| match quarters.iter().position(|&q| q == l) {
                Some(q) => Source::Made(Made::Quarter { parent: i as u32, q: q as u8, area: share[q], bed: bed[q], cover: cover[q] }),
                None => Source::Old(self.map.cell(l).expect("a cell") as u32),
            })
            .collect();
        self.renew((map, mesh), from, c);
        kids.iter().zip(flux).for_each(|(&j, f)| self.liquid_flux[j] = f as f32);
        if uneven.is_some() {
            crate::flow::share_ground(self, &kids, ground.quarters(&inside.soil), uneven.as_ref().map_or(1.0, |u| u.scale));
        }
        // The terrain each quarter stands: what its join kept, or the split's scale.
        let scales = record.as_ref().map(|r| r.scales).unwrap_or([uneven.as_ref().map_or(0.0, |u| u.scale); 4]);
        kids.iter().zip(scales).for_each(|(&j, s)| self.relief_scale[j] = s);
        // Each quarter faces as the ground it stands does: as it was joined, or the terrain inside it at the scale the
        // split stands it about the cell's own (`uneven`); else flat, as the cell.
        let rises = record.as_ref().map(|r| r.rise).or(uneven.as_ref().map(|u| {
            let whole = self.terrain_rise(leaf);
            quarters.map(|q| rise + u.scale * (self.terrain_rise(q) - whole))
        }));
        if let Some(rises) = rises {
            kids.iter().zip(rises).for_each(|(&j, r)| self.rise_south[j] = r);
        }
        let kept = record.as_ref().map(|r| r.kept).or(uneven.as_ref().map(|u| u.relief));
        if let Some(kept) = kept {
            let left = relief - (crate::flow::potential_of(self, &kids, air) - before) - sum4(kept);
            if let Some(r) = &record {
                kids.iter().zip(kept).for_each(|(&j, k)| self.relief[j] = k);
                fall_heat(self, Class::Ground, kids.iter().zip(0..).map(|(&j, q)| (j, left * r.share(Of::Heat(Class::Ground), q))));
            } else {
                kids.iter().zip(kept).zip(share).for_each(|((&j, k), a)| self.relief[j] = k + left * a);
            }
        }
        self.events.push(Event { day: self.day(), kind: EventKind::Split, node: serial, value: leaf.depth as f64 });
        self.seen(&kids);
    }

    /// The world on `map` and its `mesh` (its cells not linked yet), its cells from the old ones by `from`
    /// (`Renumber`, `World::remap_cells`), a part of change `c`: each link between two old cells is the link it was, a
    /// new one none.
    fn renew(&mut self, (map, mesh): (Square, Mesh), from: Vec<Source>, c: &mut Change) {
        let (old, cells) = (self.mesh.cells(), mesh.cells());
        let at: HashMap<(u32, u32), usize> = old.a().iter().zip(old.b()).enumerate().map(|(l, (&a, &b))| ((a, b), l)).collect();
        let was_cell = |x: u32| if let Source::Old(i) = from[x as usize] { Some(i) } else { None };
        let before = |l: usize| c.was.as_ref().map_or(Some(l as u32), |was| was[l]);
        let links = (0..cells.a().len())
            .map(|l| was_cell(cells.a()[l]).zip(was_cell(cells.b()[l])).and_then(|(a, b)| at.get(&(a.min(b), a.max(b)))).and_then(|&l| before(l)))
            .collect();
        c.was = Some(links);
        let r = Renumber::of(old.n(), from);
        (self.map, self.mesh) = (map, mesh);
        self.remap_cells(&r);
    }

    /// The bodies of joined cell `i` merge as the monthly merge leaves a cell (`nodes::crowd`), so it holds no more
    /// nodes than a cell of its kind would; the parts they let go of are let go of (`life::land`).
    fn merge_in(&mut self, i: usize) {
        let lay = crate::body::Layout::of(&self.chem);
        self.in_cell(i, |w, d| {
            // (Fewer than two bodies have nothing to merge.)
            if d.cell(i).nth(1).is_some() {
                crate::nodes::crowd(w, d, &lay, i);
            }
        });
        crate::life::land(self);
    }

    /// After a split or a join, as after the month's batch (`survey::batch`): the holders of each lineage and born rows
    /// are counted from the bodies, and the visible traits of the bodies of `cells` worked out from what they hold
    /// (`Chunk::see_changed`), so a world saved now goes on as the unbroken one.
    fn seen(&mut self, cells: &[usize]) {
        self.life.compact();
        self.life.recount();
        cells.iter().for_each(|&i| self.in_cell(i, |w, d| d.see_changed(w)));
    }
}

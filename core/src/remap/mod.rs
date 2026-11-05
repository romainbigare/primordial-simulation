//! The cell table (laws.md 0): what the saved world keeps per cell of the finest level, and per link of it, follows a
//! change of the cells in one place (`World::remap`): cells renumbered, a cell split into its quarters, quarters joined
//! into their parent (`split`). Per cell: `World::rain`, `liquid_flux`, `evap`, `off_land`, `canopy`, `light`, `rained`,
//! `joined`, what refinement keeps (`watch`); the store's nodes of each cell (`Class::CELLS`) and their sleep; the life
//! of each cell and the survey's caches of it (`life::Cell`, `Life::sum`); Flow's state of each ground node; the traffic
//! (`Counters::traffic`). Per link: what the links remember (`store::Links`). A remap only moves numbers, and shares or
//! sums them for the cells a split or a join made (`made`; the bodies, parts and cover of a cell as wholes,
//! `Life::remap`): what a new cell holds is its law's to give.

mod made;
mod split;

pub use made::{Joined, Made};
pub(crate) use split::Change;
pub(crate) use made::{mean_by, Of, FLOW_HELD};

use crate::event::EventKind;
use crate::World;

/// What new item `j` holds: what an old item held, nothing (a new item), or what a split or a join made of old items
/// that are gone (`Made`).
#[derive(Clone, Copy, Debug)]
pub enum Source {
    Old(u32),
    New,
    Made(Made),
}

/// How the items of one kind, the cells of the finest level or its links, take new numbers (`Source`); an old item that
/// no new one holds is gone.
#[derive(Clone, Debug)]
pub struct Renumber {
    /// The count of old items
    old: usize,
    from: Vec<Source>,
}

impl Renumber {
    /// New items from old ones (none: a new item), of `old` old items.
    pub fn new(old: usize, from: Vec<Option<u32>>) -> Renumber {
        Renumber::of(old, from.into_iter().map(|f| f.map_or(Source::New, Source::Old)).collect())
    }
    /// New items from old ones (`Source`), of `old` old items. Each old item goes to one new item at most, so nothing is
    /// held twice, and an item a split or a join made reads only old items no new item holds.
    pub fn of(old: usize, from: Vec<Source>) -> Renumber {
        let mut taken = vec![false; old];
        for &i in from.iter().filter_map(|f| if let Source::Old(i) = f { Some(i) } else { None }) {
            assert!((i as usize) < old && !taken[i as usize], "old item {i} is one of {old} and goes to one new item at most");
            taken[i as usize] = true;
        }
        let mut read = from.iter().filter_map(|f| if let Source::Made(m) = f { Some(m.old()) } else { None }).flatten();
        assert!(read.all(|i| (i as usize) < old && !taken[i as usize]), "a made item reads old items that are gone");
        Renumber { old, from }
    }
    /// What each new item holds, in order.
    pub(crate) fn sources(&self) -> &[Source] {
        &self.from
    }
    /// `v`, in blocks of one number per old item (`[block * old + item]`; with no old items, one block), as blocks of one
    /// per new item: each number moves with its item, and a new item, or one a split or a join made, takes `empty()`.
    /// An empty `v` stays empty.
    pub(crate) fn apply<T>(&self, v: Vec<T>, mut empty: impl FnMut() -> T) -> Vec<T> {
        self.fill(v, |_, _, _| empty())
    }
    /// `apply`, where an item a split or a join made takes `made(block, how, at)`, `at(i)` the number of old item `i` in
    /// the block.
    pub(crate) fn apply_made<T: Copy>(&self, v: Vec<T>, mut empty: impl FnMut() -> T, mut made: impl FnMut(usize, &Made, &dyn Fn(u32) -> T) -> T) -> Vec<T> {
        self.fill(v, |block, how, old| match how {
            Some(m) => made(block, m, &|i| old[i as usize].expect("a made item reads an old item that is gone")),
            None => empty(),
        })
    }
    /// `v` as blocks of one number per new item (`apply`), an item that is not an old one's taking `fill(block, how it was
    /// made, the old block)`.
    fn fill<T>(&self, v: Vec<T>, mut fill: impl FnMut(usize, Option<&Made>, &[Option<T>]) -> T) -> Vec<T> {
        let blocks = v.len().checked_div(self.old).unwrap_or(1);
        assert_eq!(v.len(), blocks * self.old, "one number per old item in each block");
        let mut held: Vec<Option<T>> = v.into_iter().map(Some).collect();
        let mut out = Vec::with_capacity(blocks * self.from.len());
        for block in 0..blocks {
            let old = &mut held[block * self.old..(block + 1) * self.old];
            for f in &self.from {
                out.push(match f {
                    Source::Old(i) => old[*i as usize].take().expect("an old item goes to one new item"),
                    Source::New => fill(block, None, old),
                    Source::Made(m) => fill(block, Some(m), old),
                });
            }
        }
        out
    }
}

/// A change of the cells of the finest level and of its links (`World::remap`).
#[derive(Clone, Debug)]
pub struct Remap {
    pub cells: Renumber,
    pub links: Renumber,
}

impl World {
    /// The cells and links of the finest level take their new numbers (`Remap`): every number the world keeps per cell
    /// or per link (the cell table, `remap`) moves with its cell or link; a new cell holds nothing, its life none, with
    /// the next serial (laws.md 0); a cell a split or a join made (`Made`) holds its share or its sum of what the cells it
    /// came from held, a number per area theirs or their mean, and their life shared out or gathered (`Life::remap`),
    /// with the next serial. Every field of the world is named here, so a new one cannot be left out. It runs between
    /// steps, once the shell took the events (but splits and joins, which name their cell by its serial); the caller
    /// gives the world the map and the mesh of its new cells. Flow's caches are dropped, as a loaded world's, and rebuilt
    /// at the next step.
    pub fn remap(&mut self, r: &Remap) {
        self.remap_cells(&r.cells);
        self.links.remap(&r.links);
    }

    /// `remap` of the cells alone (`Renumber`): what the links remember stays as it is, for the links to take their new
    /// numbers once the change of the cells under way has its mesh (`World::split`).
    pub(crate) fn remap_cells(&mut self, c: &Renumber) {
        let World { cfg: _, chem, map: _, mesh: _, seed, time, sleep, store, base_z: _, rain, liquid_flux, evap, off_land, counts, life, canopy, light, rained, links: _, joined, terrain: _, watch, events, scratch } = self;
        assert!(events.iter().all(|e| matches!(e.kind, EventKind::Split | EventKind::Join)), "a remap runs between steps, once the shell took the events");
        // Per m² of a cell: its parent's in each quarter, the quarters' mean by area in a joined cell.
        for v in [rain, evap, canopy] {
            *v = c.apply_made(std::mem::take(v), || 0.0, |_, m, at| m.mean(|i| at(i) as f64) as f32);
        }
        for v in [light, rained] {
            *v = c.apply_made(std::mem::take(v), || 0.0, |_, m, at| m.mean(at));
        }
        // Per cell: shared by area, summed in a joined cell.
        for v in [liquid_flux, off_land, &mut counts.traffic] {
            *v = c.apply_made(std::mem::take(v), || 0.0, |_, m, at| m.held(joined, None, |i| at(i) as f64) as f32);
        }
        store.remap(c, joined);
        sleep.remap(c, *time, joined);
        life.remap(c, *seed, chem);
        scratch.remap(c, joined);
        *watch = c.apply_made(std::mem::take(watch), Default::default, |_, m, at| crate::refine::Watch::made(m, at));
        *joined = c.apply(std::mem::take(joined), || None);
        let made: Vec<usize> = c.sources().iter().enumerate().filter(|s| matches!(s.1, Source::Made(_))).map(|s| s.0).collect();
        self.thawed(&made);
    }
}

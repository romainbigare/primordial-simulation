//! The cell table (laws.md 0): what the saved world keeps per cell of the finest level, and per link of it, follows a
//! change of the cells in one place (`World::remap`): cells renumbered, a cell split into children, children joined
//! into a parent. Per cell: `World::rain`, `liquid_flux`, `evap`, `off_land`, `canopy`, `light`, `rained`; the store's
//! nodes of each cell (`Class::CELLS`) and their sleep; the life of each cell and the survey's caches of it
//! (`life::Cell`, `Life::sum`); Flow's state of each ground node; the traffic (`Counters::traffic`). Per link: what the
//! links remember (`store::Links`). A remap only moves numbers: what a new cell holds is its law's to give.

use crate::World;

/// How the items of one kind, the cells of the finest level or its links, take new numbers: new item `j` holds what old
/// item `from[j]` held (none: a new item, empty), and an old item that no new one names is gone.
#[derive(Clone, Debug)]
pub struct Renumber {
    /// The count of old items
    old: usize,
    from: Vec<Option<u32>>,
}

impl Renumber {
    /// New items from old ones (`Renumber`), of `old` old items. Each old item goes to one new item at most, so nothing is
    /// held twice.
    pub fn new(old: usize, from: Vec<Option<u32>>) -> Renumber {
        let mut taken = vec![false; old];
        for &i in from.iter().flatten() {
            assert!((i as usize) < old && !taken[i as usize], "old item {i} is one of {old} and goes to one new item at most");
            taken[i as usize] = true;
        }
        Renumber { old, from }
    }
    /// `v`, in blocks of one number per old item (`[block * old + item]`; with no old items, one block), as blocks of one
    /// per new item: each number moves with its item, and a new item takes `empty()`. An empty `v` stays empty.
    pub(crate) fn apply<T>(&self, v: Vec<T>, mut empty: impl FnMut() -> T) -> Vec<T> {
        let blocks = v.len().checked_div(self.old).unwrap_or(1);
        assert_eq!(v.len(), blocks * self.old, "one number per old item in each block");
        let mut held: Vec<Option<T>> = v.into_iter().map(Some).collect();
        let mut out = Vec::with_capacity(blocks * self.from.len());
        for block in 0..blocks {
            let old = &mut held[block * self.old..(block + 1) * self.old];
            out.extend(self.from.iter().map(|f| f.map_or_else(&mut empty, |i| old[i as usize].take().expect("an old item goes to one new item"))));
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
    /// or per link (the cell table, `remap`) moves with its cell or link, and a new cell holds nothing, its life none,
    /// with the next serial (laws.md 0). Every field of the world is named here, so a new one cannot be left out. It
    /// runs between steps, once the shell took the events; the caller gives the world the map and the mesh of its new
    /// cells. Flow's caches are dropped, as a loaded world's, and rebuilt at the next step.
    pub fn remap(&mut self, r: &Remap) {
        let World { cfg: _, chem: _, map: _, mesh: _, seed: _, time, sleep, store, base_z: _, rain, liquid_flux, evap, off_land, counts, life, canopy, light, rained, links, events, scratch } = self;
        assert!(events.is_empty(), "a remap runs between steps, once the shell took the events");
        for v in [rain, liquid_flux, evap, off_land, canopy, &mut counts.traffic] {
            *v = r.cells.apply(std::mem::take(v), || 0.0);
        }
        for v in [light, rained] {
            *v = r.cells.apply(std::mem::take(v), || 0.0);
        }
        store.remap(&r.cells);
        sleep.remap(&r.cells, *time);
        life.remap(&r.cells);
        scratch.remap(&r.cells);
        links.remap(&r.links);
    }
}

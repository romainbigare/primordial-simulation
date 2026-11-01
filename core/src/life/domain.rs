//! A domain (laws.md 0): the cells under one air node and all that its bodies' updates write. Body code runs on a domain
//! taken out of the world (`(&World, &mut Domain)`), in the parallel phase of a batch and in every serial step
//! (`World::in_domain`) alike: it writes its bodies, their lists, the store's view (`store::Moves`), its
//! events and counters, its outbox, and the lineages and born rows it makes (provisional, `registry`) and the changes of
//! their holders; the world settles them when the domain is back (`World::settle`). What lands in its cells
//! (`Life::cross`) is its own too.

use super::{free_place, Handle, Held, ID_SHIFT};
use crate::{body::Body, event::Event, lineage::Lineage, outbox::Entry, store::Moves, Counters, World};
use std::collections::BTreeMap;

/// The cells under one air node, how many bodies were placed in it, the bodies of its cells (their slots, the last freed
/// taken first, and each cell's list), and what its bodies' updates wrote.
#[derive(Clone, Debug, Default)]
pub struct Domain {
    /// Its place among the domains, and its cells in order (`Level::children` of its air node)
    pub(super) index: u32,
    pub(crate) cells: Vec<u32>,
    /// Bodies placed in it so far (children, split halves, parts of nodes, members drawn, founders)
    pub(super) placed: u64,
    pub(super) bodies: Vec<Body>,
    pub(super) free: Vec<u32>,
    /// The bodies of each of its cells (by their order in `cells`), in the order they came
    pub(crate) at: Vec<Vec<Handle>>,
    /// The slot of each body stored in it, by id
    pub(super) table: BTreeMap<u64, u32>,
    /// Per cell (by its order in `cells`): the energy movers spent pushing through its bodies since the last survey,
    /// which then breaks them, J (`choose::wear`)
    pub pushed: Vec<f64>,
    /// Per cell (by its order in `cells`): the members that moved into it since the last month (`Counters::traffic`)
    pub(crate) crossings: Vec<f32>,
    /// What its bodies' updates do outside them, in the order made (`outbox`)
    pub out: Vec<Entry>,
    /// The moves and litters that land in its cells, in the order of the domains they come from, then the order made
    /// (`outbox::land`)
    pub(crate) inbox: Vec<Entry>,
    /// The ids of the bodies that moved into it since the world last settled (`World::settle` notes where they are)
    pub(super) arrived: Vec<u64>,
    /// The lineages and the born rows its bodies made since the world last settled, by provisional id (`PROVISIONAL`),
    /// and the changes of the holders they made: of a lineage or born rows, its id, the change (`World::settle`)
    pub(super) brains: Vec<Lineage>,
    pub(super) genomes: Vec<Vec<f32>>,
    pub(super) counted: Vec<(Held, u32, i32)>,
    /// The cover of its cells (`cover`)
    pub cover: crate::cover::Cover,
    /// The moves its bodies gathered and the view of its nodes as they leave them
    pub moves: Moves,
    /// The events and counters of its bodies since the world last settled
    pub(crate) events: Vec<Event>,
    pub(crate) counts: Counters,
}

impl Domain {
    /// The order of cell `i` among its cells.
    pub fn slot(&self, i: usize) -> usize {
        self.cells.iter().position(|&c| c as usize == i).expect("a cell of the domain")
    }
    /// How many bodies are stored in it.
    pub(crate) fn count(&self) -> usize {
        self.table.len()
    }
    /// Whether it holds bodies or cover.
    pub(crate) fn lives(&self) -> bool {
        self.count() > 0 || self.cover.any()
    }
    /// The handles of the bodies of its cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = Handle> + '_ {
        self.at[self.slot(i)].iter().copied()
    }

    /// Where the body with id `id` is stored, if it lives and is stored in this domain.
    pub fn find(&self, id: u64) -> Option<Handle> {
        self.table.get(&id).map(|&slot| Handle { domain: self.index, slot })
    }
    /// Body `b` (taken out of its slot) now has `count` members.
    pub(crate) fn set_count(&mut self, b: &mut Body, count: f64) {
        b.count = count;
    }
    /// Place a new body in its cell on `day`; it first updates at the next month's start.
    pub(crate) fn add(&mut self, mut b: Body, day: u64) -> Handle {
        self.placed += 1;
        assert!(self.placed < 1 << ID_SHIFT, "domain {} placed 2^{ID_SHIFT} bodies: its ids would run into the next domain's", self.index);
        (b.id, b.alive, b.last, b.count) = ((self.index as u64) << ID_SHIFT | self.placed, true, day as f64, b.n());
        self.enter(&b);
        self.store(b)
    }
    /// Store body `b` (counted in its lineage and born rows, its id set): in a free slot, its
    /// cell's list and the id table.
    pub(super) fn store(&mut self, b: Body) -> Handle {
        let h = Handle { domain: self.index, slot: free_place(&mut self.bodies, &mut self.free) as u32 };
        let k = self.slot(b.cell as usize);
        self.at[k].push(h);
        self.table.insert(b.id, h.slot);
        self[h] = b;
        h
    }
    /// Body `b`, taken out of `h`, leaves the world: its slot is free.
    pub(crate) fn remove(&mut self, h: Handle, b: &Body) {
        self.leave(b);
        self.unstore(h, b.id, b.cell as usize);
    }
    /// Body `id`, taken out of `h` in cell `i`, is no longer stored here: it leaves the cell's list and the id table, and
    /// its slot is free.
    pub(super) fn unstore(&mut self, h: Handle, id: u64, i: usize) {
        let k = self.slot(i);
        self.at[k].retain(|&x| x != h);
        self.table.remove(&id);
        self.bodies[h.slot as usize] = Body::default();
        self.free.push(h.slot);
    }
    /// Body `b`, which moved into its cell from another, is stored here.
    pub(crate) fn arrive(&mut self, b: Body) -> Handle {
        self.arrived.push(b.id);
        self.store(b)
    }
    /// At the end of its task: each body with a move queued (`Entry::Go`) leaves its storage, and goes with
    /// its move; a move whose body died or joined another since is dropped.
    pub(crate) fn post(&mut self) {
        let mut out = std::mem::take(&mut self.out);
        out.retain_mut(|e| {
            let Entry::Go { id, body: body @ None, .. } = e else { return true };
            let Some(h) = self.find(*id) else { return false };
            let b = std::mem::take(&mut self[h]);
            self.depart(h, &b, b.cell as usize);
            *body = Some(Box::new(b));
            true
        });
        self.out = out;
    }
    /// `members` members entered its `k`th cell in a move that spent `pushed` J pushing through its bodies
    /// (`choose::land_move`).
    pub fn cross(&mut self, k: usize, members: f32, pushed: f64) {
        (self.crossings[k], self.pushed[k]) = (self.crossings[k] + members, self.pushed[k] + pushed);
    }
    /// Body `b`, taken out of `h` in cell `i`, leaves its storage, to be stored again elsewhere (`arrive`).
    pub(super) fn depart(&mut self, h: Handle, b: &Body, i: usize) {
        self.unstore(h, b.id, i);
    }
    /// Place body `c` made by a law on the world's day, its first update at the next month's start; its visible traits are
    /// worked out from what it holds now, so others see it before its first update.
    pub(crate) fn place(&mut self, w: &World, mut c: Body) -> Handle {
        c.prepare(&w.chem, &w.cfg);
        self.place_part(w, c)
    }
    /// `place` for a part of a node with the node's genes (`nodes::part`), whose constants (`Body::k`) follow from them
    /// already.
    pub(crate) fn place_part(&mut self, w: &World, mut c: Body) -> Handle {
        c.stale = true;
        self.add(c, w.day())
    }
    /// The visible traits of its living bodies that changed since the last survey are worked out again
    /// (`Body::visible`, `Body::stale`): once before what reads them, the cells' sums and the batch's faces.
    pub(crate) fn see_changed(&mut self, w: &World) {
        for b in self.bodies.iter_mut().filter(|b| b.alive && b.stale) {
            (b.vis, b.stale) = (b.visible(&w.chem, &w.cfg), false);
        }
    }
}

impl std::ops::Index<Handle> for Domain {
    type Output = Body;
    fn index(&self, h: Handle) -> &Body {
        &self.bodies[h.slot as usize]
    }
}

impl std::ops::IndexMut<Handle> for Domain {
    fn index_mut(&mut self, h: Handle) -> &mut Body {
        &mut self.bodies[h.slot as usize]
    }
}

impl World {
    /// A serial step on the domain of cell `i`: it is taken out of the world, `f` runs on it as body code runs in the
    /// parallel phase, it is put back, and the world settles (`settle`).
    pub fn in_domain<R>(&mut self, i: usize, f: impl FnOnce(&World, &mut Domain) -> R) -> R {
        let k = self.life.domain_of(i);
        self.life.mark(k);
        let mut d = std::mem::take(&mut self.life.domains[k]);
        let r = f(self, &mut d);
        self.life.domains[k] = d;
        self.settle(k..k + 1);
        r
    }
    /// The world takes what domains `ds` wrote since it last settled, domain by domain in order: their events and
    /// counters, and where the bodies that moved into them are stored (`Life::find`); the lineages and born rows they
    /// made get global ids (`Life::remap`); outside a batch the changes of holders are added (`Life::count_all`), in a
    /// batch at its end.
    pub(crate) fn settle(&mut self, ds: std::ops::Range<usize>) {
        let World { life, events, counts, .. } = self;
        for d in &mut life.domains[ds.clone()] {
            events.append(&mut d.events);
            counts.add(&mut d.counts);
        }
        ds.clone().for_each(|k| life.note_arrived(k));
        life.remap(ds.clone());
        if !life.batch {
            life.count_all(ds);
        }
    }
}

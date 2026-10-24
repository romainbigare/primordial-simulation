//! A domain (plan 3.2): the cells under one air node and all that its bodies' updates write. Body code runs on a domain
//! taken out of the world (`(&World, &mut Domain)`), in the parallel phase of a batch and in every serial step
//! (`World::in_domain`) alike: it writes its bodies, their lists, its ring, the store's view (`store::Moves`), its
//! events and counters, its outbox, and the lineages and born rows it makes (provisional, `registry`) and the changes of
//! their holders; the world settles them when the domain is back (`World::settle`, S72).

use super::{free_place, Handle, Held, ID_SHIFT, RING};
use crate::{body::Body, event::Event, lineage::Lineage, outbox::Entry, store::Moves, Counters, World};
use std::collections::BTreeMap;

/// The cells under one air node, how many bodies were placed in it, the bodies of its cells (their slots, the last freed
/// taken first, each cell's list and the ring of their updates), and what its bodies' updates wrote.
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
    /// Updates due, by batch modulo RING: body ids (`Body::due`)
    pub(super) ring: Vec<Vec<u64>>,
    /// Per cell (by its order in `cells`): whether its bodies changed since the survey last shared out its light
    pub(crate) stale: Vec<bool>,
    /// What its bodies' updates do outside them, in the order made (`outbox`)
    pub out: Vec<Entry>,
    /// The length of `out` when the update running began (`queued`)
    pub(crate) began: usize,
    /// The lineages and the born rows its bodies made since the world last settled, by provisional id (`PROVISIONAL`),
    /// and the changes of the holders they made: of a lineage or born rows, its id, the change (`World::settle`)
    pub(super) brains: Vec<Lineage>,
    pub(super) genomes: Vec<Vec<f32>>,
    pub(super) counted: Vec<(Held, u32, i32)>,
    /// The moves its bodies gathered and the view of its nodes as they leave them
    pub moves: Moves,
    /// The events and counters of its bodies since the world last settled
    pub(crate) events: Vec<Event>,
    pub(crate) counts: Counters,
}

impl Domain {
    /// The order of cell `i` among its cells.
    pub(super) fn slot(&self, i: usize) -> usize {
        self.cells.iter().position(|&c| c as usize == i).expect("a cell of the domain")
    }
    /// The handles of the bodies of its cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = Handle> + '_ {
        self.at[self.slot(i)].iter().copied()
    }

    /// Where the body with id `id` is stored, if it lives and is stored in this domain.
    pub fn find(&self, id: u64) -> Option<Handle> {
        self.table.get(&id).map(|&slot| Handle { domain: self.index, slot })
    }
    /// The bodies of its cell `i` changed: the survey shares out its light again (`survey::shade`).
    pub(crate) fn stir(&mut self, i: u32) {
        let k = self.slot(i as usize);
        self.stale[k] = true;
    }
    /// Body `b` (taken out of its slot) now has `count` members.
    pub(crate) fn set_count(&mut self, b: &mut Body, count: f64) {
        b.count = count;
        self.stir(b.cell);
    }
    /// Place a new body in its cell on `day`; it first updates in batch `due`, and has its cell's mean light until the
    /// survey shares it out again (`survey::shade`).
    pub(crate) fn add(&mut self, mut b: Body, day: u64, due: u64) -> Handle {
        self.placed += 1;
        assert!(self.placed < 1 << ID_SHIFT, "domain {} placed 2^{ID_SHIFT} bodies: its ids would run into the next domain's", self.index);
        (b.id, b.alive, b.last, b.count, b.shade) = ((self.index as u64) << ID_SHIFT | self.placed, true, day as f64, b.n(), 1.0);
        self.schedule(due, &mut b);
        self.enter(&b);
        self.store(b)
    }
    /// Store body `b` (counted in its lineage and born rows, its id set, its next update filed): in a free slot, its
    /// cell's list and the id table.
    pub(super) fn store(&mut self, b: Body) -> Handle {
        let h = Handle { domain: self.index, slot: free_place(&mut self.bodies, &mut self.free) as u32 };
        let k = self.slot(b.cell as usize);
        self.at[k].push(h);
        self.stale[k] = true;
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
        self.stale[k] = true;
        self.table.remove(&id);
        self.bodies[h.slot as usize] = Body::default();
        self.free.push(h.slot);
    }
    /// File the next update of body `b` in batch `due`.
    pub(crate) fn schedule(&mut self, due: u64, b: &mut Body) {
        b.due = due;
        self.ring[(due % RING) as usize].push(b.id);
    }
    /// Body `b` queued a part act of its own but a let-go or an anchor (`bond::LET_GO`, `TORN`, `ANCHORS`) in the update
    /// running: it does not split in it (`nodes::split`; a queued move leaves it `Body::leaving`).
    pub(crate) fn queued(&self, b: &Body) -> bool {
        let mine = |e: &Entry| matches!(e, Entry::Act { id, on, .. } if *id == b.id && on.as_ref().map_or(true, |x| x.0 < crate::bond::LET_GO));
        self.out[self.began.min(self.out.len())..].iter().any(mine)
    }
    /// Place body `c` made by a law on the world's day, its first update after its interval; its visible traits are
    /// worked out from what it holds now, so others see it before its first update.
    pub(crate) fn place(&mut self, w: &World, mut c: Body) -> Handle {
        c.prepare(&w.chem, &w.cfg);
        self.see(w, &mut c);
        let due = w.due_in(c.interval(w));
        self.add(c, w.day(), due)
    }
    /// What others see of body `b` is worked out again (`Body::visible`), and its cell's light shared out again.
    pub(crate) fn see(&mut self, w: &World, b: &mut Body) {
        b.vis = b.visible(&w.chem, &w.cfg);
        self.stir(b.cell);
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
        let mut d = std::mem::take(&mut self.life.domains[k]);
        let r = f(self, &mut d);
        self.life.domains[k] = d;
        self.settle(k..k + 1);
        r
    }
    /// The world takes what domains `ds` wrote since it last settled (S72), domain by domain in order: their events and
    /// counters; the lineages and born rows they made get global ids (`Life::remap`); outside a batch the changes of
    /// holders are added (`Life::count_all`), in a batch at its end.
    pub(crate) fn settle(&mut self, ds: std::ops::Range<usize>) {
        let World { life, events, counts, .. } = self;
        for d in &mut life.domains[ds.clone()] {
            events.append(&mut d.events);
            counts.add(&mut d.counts);
        }
        life.remap(ds.clone());
        if !life.batch {
            life.count_all(ds);
        }
    }
}

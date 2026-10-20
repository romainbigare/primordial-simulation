//! The bodies' day in waves (laws.md 0, "The order"). The cells under one air node make a block: its bodies breathe
//! from that node. What the update of a body reads and changes lies in its cell and the cells one link away (it sees,
//! bites, moves and has children there), so the updates due today in a block reach its cells that hold them and their
//! neighbours. Each block, those with the most updates first, joins the first wave in which no block reaches a cell its
//! own updates reach, and the waves run in turn from one that moves on each day, so no block comes first every day.
//! The blocks of a wave share nothing their bodies change: each runs its updates apart, in the order of its cells, its
//! new bodies, born rows, lineages and parts taken from places of its own (`life::Pool`), and what it did is put
//! together block by block in the order of the blocks. With threads the blocks of a wave run at once, each on a world
//! of a thread's own (`Crew`), so the history is the same with any number of threads and without them.

use crate::life::{Pool, Pools};
use crate::{World, AIR};
#[cfg(feature = "threads")]
use crate::{body::Body, store::{Class, Node}};

/// The worlds of the threads, one for each thread (none without threads), which hold what the updates of a block read
/// besides the bodies and lists of its reach, which move in before its updates and back after.
#[derive(Default)]
pub struct Crew {
    /// The worlds of the threads, each with how far it has read `log`
    #[cfg(feature = "threads")]
    worlds: Vec<std::sync::Mutex<(World, usize)>>,
    /// What changed since the worlds of the threads last copied all they read, which each copies before a block
    #[cfg(feature = "threads")]
    log: Vec<Note>,
    /// Whether processes other than the bodies' day ran since the worlds of the threads copied what they read
    pub(crate) stale: bool,
}

/// A copy of a world starts with no crew: it makes its own when its bodies first run.
impl Clone for Crew {
    fn clone(&self) -> Self {
        Crew::default()
    }
}

impl std::fmt::Debug for Crew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Crew")
    }
}

/// A change a world of a thread copies (`Crew::log`): the numbers of a node of the store that bodies booked moves of,
/// born rows written, or a lineage made.
#[cfg(feature = "threads")]
#[derive(Clone, Copy)]
enum Note {
    Node(Node),
    Genome(u32),
    Lineage(u32),
}

/// The updates due today in a block: (cell, slot, generation), in that order, and the cells they reach, in order.
struct Task {
    block: usize,
    due: Vec<(u32, u32, u32)>,
    reach: Vec<u32>,
}

/// Update the bodies due today, in waves of blocks.
pub fn day(w: &mut World) {
    let due = w.life.take_due(w.day());
    if due.is_empty() {
        return;
    }
    let (bodies, cells) = (&w.life.bodies, w.mesh.cells());
    let mut list: Vec<(usize, u32, u32, u32)> = due
        .iter()
        .filter(|&&(s, g)| bodies[s as usize].alive && bodies[s as usize].gen == g)
        .map(|&(s, g)| (w.mesh.up(bodies[s as usize].cell as usize, 0, AIR), bodies[s as usize].cell, s, g))
        .collect();
    list.sort_unstable();
    let mut tasks: Vec<Task> = Vec::new();
    for &(b, cell, slot, gen) in &list {
        match tasks.last_mut() {
            Some(t) if t.block == b => t.due.push((cell, slot, gen)),
            _ => tasks.push(Task { block: b, due: vec![(cell, slot, gen)], reach: Vec::new() }),
        }
    }
    let near = |&(c, _, _): &(u32, u32, u32)| std::iter::once(c).chain(cells.around(c as usize).1.iter().copied());
    for t in &mut tasks {
        t.reach = t.due.iter().flat_map(near).collect();
        t.reach.sort_unstable();
        t.reach.dedup();
    }
    // The blocks with the most updates first, each joins the first wave none of whose blocks reaches a cell of its
    // reach, so the blocks that take longest run together.
    tasks.sort_by_key(|t| std::cmp::Reverse(t.due.len()));
    let (mut waves, mut claimed): (Vec<Vec<Task>>, Vec<Vec<usize>>) = (Vec::new(), vec![Vec::new(); cells.n()]);
    for t in tasks {
        let wave = (0..).find(|&v| t.reach.iter().all(|&c| !claimed[c as usize].contains(&v))).expect("a wave");
        t.reach.iter().for_each(|&c| claimed[c as usize].push(wave));
        if wave == waves.len() {
            waves.push(Vec::new());
        }
        waves[wave].push(t);
    }
    let first = w.day() as usize % waves.len().max(1);
    waves.rotate_left(first);
    let mut crew = std::mem::take(&mut w.crew);
    for mut tasks in waves {
        tasks.sort_unstable_by_key(|t| t.block);
        wave(w, &mut crew, tasks);
    }
    w.crew = crew;
}

/// One wave. Blocks that reach a part run on the world itself after the others: the bonds of all parts are one list.
fn wave(w: &mut World, crew: &mut Crew, tasks: Vec<Task>) {
    crew.copy(w);
    let (held, free): (Vec<Task>, Vec<Task>) = tasks.into_iter().partition(|t| reaches_parts(w, &t.reach));
    let n = free.len();
    // The lists of bodies, born rows, lineages (which free none) and parts.
    let lens = [w.life.bodies.len(), w.life.genomes.len(), w.life.brains.len(), w.parts.parts.len()];
    let mut pools: Vec<(Pools, Pool)> = (0..n).map(|_| Default::default()).collect();
    for (x, free) in free_lists(w, &mut Vec::new()).into_iter().enumerate() {
        for ((life, parts), pool) in pools.iter_mut().zip(hand_out(free, lens[x], n)) {
            *[&mut life.bodies, &mut life.genomes, &mut life.lineages, parts][x] = pool;
        }
    }
    let taken = crew.run(w, free, pools);
    // The places from the end of a list that no pool took are free; a lineage's stays empty.
    let ends = [w.life.bodies.len(), w.life.genomes.len(), 0, w.parts.parts.len()];
    for (x, free) in free_lists(w, &mut Vec::new()).into_iter().enumerate() {
        let len = lens[x];
        free.extend((len..ends[x]).filter(|&i| ((i - len) / n) as u32 >= taken[(i - len) % n][x]).map(|i| i as u32));
    }
    for t in held {
        crate::survey::run(w, &t.due);
    }
}

/// The free list of each list a block takes places from: of bodies, born rows, lineages (`none`: they are not freed)
/// and parts.
fn free_lists<'a>(w: &'a mut World, none: &'a mut Vec<u32>) -> [&'a mut Vec<u32>; 4] {
    [&mut w.life.free, &mut w.life.free_genomes, none, &mut w.parts.free]
}

/// Whether a part lies in `reach`, or a body there holds one or is inside one.
fn reaches_parts(w: &World, reach: &[u32]) -> bool {
    reach.iter().any(|&c| {
        let (parts, at) = (w.parts.at.get(c as usize).is_some_and(|p| !p.is_empty()), &w.life.at[c as usize]);
        parts || at.iter().map(|&s| &w.life.bodies[s as usize]).any(|b| b.hold.is_some() || b.inside.is_some())
    })
}

/// The pools of the `n` blocks of a wave for a list of `len` entries with the free places `free`: each takes an even
/// share of the free places, the last freed first, and every `n`-th place from `len` on.
fn hand_out(free: &mut Vec<u32>, len: usize, n: usize) -> Vec<Pool> {
    let share = free.len() / n.max(1);
    let pool = |r: usize, free: Vec<u32>| Pool { free, next: (len + r) as u32, stride: n as u32, ..Default::default() };
    (0..n).map(|r| pool(r, free.split_off(free.len() - share))).collect()
}

/// The places a block's pools did not use go back to the free lists.
fn give_back(w: &mut World, (life, parts): &(Pools, Pool)) {
    w.life.free.extend(&life.bodies.free);
    w.life.free_genomes.extend(&life.genomes.free);
    w.parts.free.extend(&parts.free);
}

/// How many places from the end of each list (bodies, born rows, lineages, parts) a block's pools took.
fn taken((life, parts): &(Pools, Pool)) -> [u32; 4] {
    [life.bodies.taken, life.genomes.taken, life.lineages.taken, parts.taken]
}

/// Run the updates of a block on `w` itself with its pools; the counts that add up and the numbers of the nodes beyond
/// the world start from zero, so they add up block by block as on the worlds of the threads.
fn here(w: &mut World, t: &Task, (life, parts): (Pools, Pool)) -> [u32; 4] {
    let (sums, far) = (w.counts.take_sums(), w.store.take_far());
    (w.life.pools, w.parts.pool) = (Some(life), Some(parts));
    crate::survey::run(w, &t.due);
    let pools = (w.life.pools.take().expect("its pools"), w.parts.pool.take().expect("its pool"));
    give_back(w, &pools);
    let (s, f) = (w.counts.take_sums(), w.store.take_far());
    w.counts.add_sums(&sums);
    w.counts.add_sums(&s);
    w.store.add_far(&far);
    w.store.add_far(&f);
    taken(&pools)
}

#[cfg(not(feature = "threads"))]
impl Crew {
    /// Run the free blocks of a wave with their pools; returns what each block's pools took.
    fn run(&mut self, w: &mut World, free: Vec<Task>, pools: Vec<(Pools, Pool)>) -> Vec<[u32; 4]> {
        free.iter().zip(pools).map(|(t, p)| here(w, t, p)).collect()
    }
    /// Before a wave, the worlds of the threads copy what changed (none without threads).
    fn copy(&mut self, w: &mut World) {
        w.life.written = Default::default();
    }
}

/// A block of a wave as it leaves the world for the world of a thread: its updates, its pools, and the cell lists of
/// its reach with the boxes of their bodies and of its free slots, taken out.
#[cfg(feature = "threads")]
struct Load {
    task: Task,
    pools: (Pools, Pool),
    at: Vec<Vec<u32>>,
    boxes: Vec<(u32, Option<Box<Body>>)>,
}

/// What a block did on the world of a thread (`worker`), for the world to take in its turn: its reach, the slots it
/// brought and its pools, whose entries stay there until then, what it recorded, and the nodes it booked moves of.
#[cfg(feature = "threads")]
struct Done {
    block: usize,
    reach: Vec<u32>,
    worker: usize,
    slots: Vec<u32>,
    pools: (Pools, Pool),
    journal: Vec<crate::life::Op>,
    events: Vec<crate::event::Event>,
    sums: crate::Counters,
    far: [Vec<f64>; 3],
    bonds: crate::bond::Bonds,
    touched: Vec<Node>,
}

/// Put `x` at place `i` of `v`, which grows to hold it.
#[cfg(feature = "threads")]
fn put<T: Default>(v: &mut Vec<T>, i: u32, x: T) {
    let i = i as usize;
    if i >= v.len() {
        v.resize_with(i + 1, T::default);
    }
    v[i] = x;
}

/// Copy the numbers of node `x` from `from` to `to`.
#[cfg(feature = "threads")]
fn copy_node(to: &mut World, from: &World, x: Node) {
    let len = if x.class == Class::Air { from.mesh.level(AIR).n() } else { from.mesh.cells().n() };
    to.store.copy_node(&from.store, x, len);
}

/// Bring `r`, the world of a thread that read `log` as far as `upto`, up to date with `w`.
#[cfg(feature = "threads")]
fn catch_up(r: &mut World, upto: &mut usize, w: &World, log: &[Note]) {
    for &note in &log[*upto..] {
        match note {
            Note::Node(x) => copy_node(r, w, x),
            Note::Genome(g) => put(&mut r.life.genomes, g, w.life.genomes[g as usize].clone()),
            Note::Lineage(l) => put(&mut r.life.brains, l, w.life.brains[l as usize].clone()),
        }
    }
    *upto = log.len();
}

/// Copy the cells' marks for the survey and crossings and the lanes of their links in `reach`, from `from` to `to`.
#[cfg(feature = "threads")]
fn copy_cells(to: &mut World, from: &World, reach: &[u32]) {
    for c in reach.iter().map(|&c| c as usize) {
        (to.life.stale[c], to.counts.crossings[c]) = (from.life.stale[c], from.counts.crossings[c]);
        for &l in from.mesh.cells().around(c).0 {
            to.links.cleared[l as usize] = from.links.cleared[l as usize];
        }
    }
}

#[cfg(feature = "threads")]
impl Load {
    /// Take the cell lists of the block's reach and the boxes of their bodies and of its free slots out of `w`.
    fn take(w: &mut World, task: Task, pools: (Pools, Pool)) -> Load {
        let at: Vec<Vec<u32>> = task.reach.iter().map(|&c| std::mem::take(&mut w.life.at[c as usize])).collect();
        let slots = at.iter().flatten().chain(&pools.0.bodies.free);
        let boxes = slots.map(|&s| (s, w.life.bodies.take(s as usize))).collect();
        Load { task, pools, at, boxes }
    }
    /// Run the block on `r`, the world of thread `worker`: it copies what changed (`log`) and its reach moves in from
    /// `w`, then its updates run with its pools, recording what they do to the counts and the ring.
    fn run(self, w: &World, log: &[Note], (r, upto): &mut (World, usize), worker: usize) -> Done {
        catch_up(r, upto, w, log);
        let (task, (life, parts)) = (self.task, self.pools);
        r.time = w.time;
        let slots = self.boxes.iter().map(|x| x.0).collect();
        self.boxes.into_iter().for_each(|(s, x)| r.life.bodies.put(s as usize, x));
        for (&c, list) in task.reach.iter().zip(self.at) {
            r.life.at[c as usize] = list;
            r.parts.at[c as usize].clear();
        }
        copy_cells(r, w, &task.reach);
        // A free place of a part keeps its generation, which the next part there takes on.
        r.parts.parts.resize_with(w.parts.parts.len().max(r.parts.parts.len()), Default::default);
        parts.free.iter().for_each(|&p| r.parts.parts[p as usize].gen = w.parts.parts[p as usize].gen);
        // What the block makes of the counts, the numbers beyond the world, events and bonds starts from nothing.
        r.counts.take_sums();
        r.store.take_far();
        r.events.clear();
        r.parts.bonds = Default::default();
        r.moves.booked = Some(Vec::new());
        (r.life.pools, r.parts.pool, r.life.journal) = (Some(life), Some(parts), Some(Vec::new()));
        crate::survey::run(r, &task.due);
        let pools = (r.life.pools.take().expect("its pools"), r.parts.pool.take().expect("its pool"));
        let (journal, events) = (r.life.journal.take().unwrap_or_default(), std::mem::take(&mut r.events));
        let (sums, far, bonds) = (r.counts.take_sums(), r.store.take_far(), std::mem::take(&mut r.parts.bonds));
        let mut touched = r.moves.booked.take().unwrap_or_default();
        touched.sort_unstable();
        touched.dedup();
        Done { block: task.block, reach: task.reach, worker, slots, pools, journal, events, sums, far, bonds, touched }
    }
}

#[cfg(feature = "threads")]
impl Done {
    /// Take in what the block did, from `r`, the world it ran on; returns what its pools took.
    fn merge(self, w: &mut World, r: &mut World) -> [u32; 4] {
        let (life, parts) = &self.pools;
        let mut slots = self.slots;
        slots.extend(&life.bodies.used);
        slots.sort_unstable();
        slots.dedup();
        slots.into_iter().for_each(|s| w.life.bodies.put(s as usize, r.life.bodies.take(s as usize)));
        for c in self.reach.iter().map(|&c| c as usize) {
            w.life.at[c] = std::mem::take(&mut r.life.at[c]);
            w.parts.at[c] = std::mem::take(&mut r.parts.at[c]);
        }
        copy_cells(w, r, &self.reach);
        self.touched.iter().for_each(|&x| copy_node(w, r, x));
        for &g in &life.genomes.used {
            put(&mut w.life.genomes, g, std::mem::take(&mut r.life.genomes[g as usize]));
        }
        for &l in &life.lineages.used {
            put(&mut w.life.brains, l, r.life.brains[l as usize].take());
        }
        w.life.members.resize(w.life.brains.len(), 0);
        w.life.written.0.extend(&life.genomes.used);
        w.life.written.1.extend(&life.lineages.used);
        let mut made = parts.used.clone();
        made.sort_unstable();
        made.dedup();
        made.into_iter().for_each(|p| put(&mut w.parts.parts, p, std::mem::take(&mut r.parts.parts[p as usize])));
        let (to, from) = (&mut w.parts.bonds, self.bonds);
        to.a.extend(from.a);
        to.b.extend(from.b);
        to.strength.extend(from.strength);
        self.journal.into_iter().for_each(|op| w.life.count(op));
        w.events.extend(self.events);
        w.counts.add_sums(&self.sums);
        w.store.add_far(&self.far);
        give_back(w, &self.pools);
        taken(&self.pools)
    }
}

#[cfg(feature = "threads")]
impl Crew {
    /// Run the free blocks of a wave with their pools, at once on the threads (one block, or all with one thread, on
    /// the world itself); returns what each block's pools took.
    fn run(&mut self, w: &mut World, free: Vec<Task>, pools: Vec<(Pools, Pool)>) -> Vec<[u32; 4]> {
        if free.len() <= 1 || self.worlds.len() <= 1 {
            return free.iter().zip(pools).map(|(t, p)| here(w, t, p)).collect();
        }
        let mut loads: Vec<Load> = free.into_iter().zip(pools).map(|(t, p)| Load::take(w, t, p)).collect();
        loads.sort_by_key(|l| std::cmp::Reverse(l.task.due.len()));
        let (shared, log) = (&*w, &self.log);
        let mut done = crate::par::apart(loads, &self.worlds, |r, worker, load| load.run(shared, log, r, worker));
        done.sort_unstable_by_key(|d| d.block);
        let mut taken = Vec::new();
        for d in done {
            self.log.extend(d.touched.iter().map(|&x| Note::Node(x)));
            let r = &mut self.worlds[d.worker].lock().expect("a world of the threads").0;
            taken.push(d.merge(w, r));
        }
        taken
    }
    /// Before a wave, what the world changed outside the blocks of the threads joins the log: the nodes bodies booked
    /// moves of and the born rows and lineages written (`Life::written`). If other processes ran (`stale`), the worlds
    /// of the threads copy all they read and the log starts again. Made at the first wave, they copy all but the
    /// bodies; with one thread there are none.
    fn copy(&mut self, w: &mut World) {
        use rayon::prelude::*;
        let threads = rayon::current_num_threads();
        if threads <= 1 {
            (w.life.written, w.moves.booked) = Default::default();
            return;
        }
        if self.worlds.len() < threads {
            let bodies = std::mem::take(&mut w.life.bodies);
            self.worlds = (0..threads).map(|_| std::sync::Mutex::new((w.clone(), 0))).collect();
            (w.life.bodies, self.stale, self.log) = (bodies, true, Vec::new());
        }
        let (genomes, lineages) = std::mem::take(&mut w.life.written);
        let mut booked = w.moves.booked.replace(Vec::new()).unwrap_or_default();
        booked.sort_unstable();
        booked.dedup();
        self.log.extend(booked.iter().map(|&x| Note::Node(x)));
        self.log.extend(genomes.into_iter().map(Note::Genome).chain(lineages.into_iter().map(Note::Lineage)));
        if std::mem::take(&mut self.stale) {
            let (shared, log) = (&*w, &self.log);
            self.worlds.par_iter().for_each(|r| {
                let (r, upto) = &mut *r.lock().expect("a world of the threads");
                copy_read(r, shared);
                catch_up(r, upto, shared, log);
                *upto = 0;
            });
            self.log.clear();
        }
    }
}

/// What the updates of a block read besides its reach, the born rows and the lineages, copied from `w` to `r`, the
/// world of a thread. Every field is named, so a new one must be placed here.
#[cfg(feature = "threads")]
fn copy_read(r: &mut World, w: &World) {
    let World {
        cfg, chem, map, air_scale, seed, base_z, rain, liquid_flux, life, canopy, light, rained, scratch, store,
        mesh: _, time: _, sleep: _, counts: _, parts: _, links: _, events: _, batches: _, moves: _, crew: _,
    } = w;
    r.cfg.clone_from(cfg);
    r.chem.clone_from(chem);
    r.map.clone_from(map);
    r.scratch.clone_from(scratch);
    r.store.clone_from(store);
    (r.air_scale, r.seed, r.base_z) = (*air_scale, *seed, *base_z);
    for (to, from) in [(&mut r.rain, rain), (&mut r.liquid_flux, liquid_flux), (&mut r.canopy, canopy)] {
        to.clone_from(from);
    }
    r.light.clone_from(light);
    r.rained.clone_from(rained);
    r.life.copy_caches(life);
}

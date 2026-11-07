//! The life of the cells a split or a join makes (laws.md 0, 4.4): one life model at every cell size, so only how the
//! nodes, the parts and the cover are shared out and gathered. A split's quarters are flat copies of their parent, so a
//! node is shared out by area (`nodes::part`), each share a node in its quarter with a brain block of the node's, and a
//! lot of lone parts by count (`Part::split`); what is not cut goes whole to one quarter, drawn by area: an
//! individual, a node of fewer members than one per quarter's share, a lot of fewer parts, and a structure with the
//! bodies that hold it or are inside it (`share`). A join gathers its quarters' life in
//! their order (`gather`); its bodies merge after (`World::join`). A body or a part is a record that holds its own
//! amounts, so it moves between cells with nothing to book.

use super::{new_id, Cell, Life};
use crate::bond::NOBODY;
use crate::chem::Chemistry;
use crate::cover::Cover;
use crate::remap::{Made, Renumber, Source};
use crate::rng::{Purpose, Rng};
use crate::survey::ROW;
use std::collections::{BTreeMap, BTreeSet};

impl Life {
    /// The life of each cell and the survey's caches of it follow their cells (`World::remap`): an old cell's moves with
    /// it; a new cell, or one a split or a join made, takes the next serial, in the order of the new cells; a split's
    /// quarters share out their parent's life (`share`, its draws keyed by `seed`) and a joined cell gathers its
    /// quarters' (`gather`). Each living body and part takes its cell's new index, and the ids find their cells again
    /// (`index`). It runs between passes.
    pub(crate) fn remap(&mut self, r: &Renumber, seed: u64, chem: &Chemistry) {
        let Life { cells, next_serial, at_serial: _, moved: _, parts_moved: _, queued, sums, absorb, keep, env, feat, brains: _, members: _, genomes: _, free_genomes: _, air_seen, touched } = self;
        assert!(queued.is_empty() && air_seen.is_empty(), "a remap runs between passes");
        // (The cells change: every one counts as held.)
        *touched = Default::default();
        let mut old: Vec<Option<Cell>> = std::mem::take(cells).into_iter().map(Some).collect();
        let mut gone = |i: u32| old[i as usize].take().expect("an old cell goes to one new cell");
        *cells = (r.sources().iter())
            .map(|f| match *f {
                Source::Old(i) => gone(i),
                _ => {
                    *next_serial += 1;
                    Cell { serial: *next_serial - 1, ..Default::default() }
                }
            })
            .collect();
        // The quarters of each split cell, row by row, each with its share of the cell's area.
        let mut splits: BTreeMap<u32, [(usize, f64); 4]> = BTreeMap::new();
        for (j, f) in r.sources().iter().enumerate() {
            match *f {
                Source::Made(Made::Quarter { parent, q, area, .. }) => splits.entry(parent).or_default()[q as usize] = (j, area),
                Source::Made(Made::Join { kids, area }) => gather(&mut cells[j], kids.map(&mut gone), area, (chem, j)),
                _ => {}
            }
        }
        for (parent, quarters) in splits {
            let mut kids = quarters.map(|(j, _)| std::mem::take(&mut cells[j]));
            share(gone(parent), &mut kids, quarters.map(|x| x.1), seed);
            quarters.iter().zip(kids).for_each(|(&(j, _), k)| cells[j] = k);
        }
        for (i, c) in cells.iter_mut().enumerate() {
            c.bodies.iter_mut().filter(|b| b.alive).for_each(|b| b.cell = i as u32);
            c.parts.iter_mut().for_each(|p| p.cell = i as u32);
        }
        *sums = r.apply(std::mem::take(sums), || [0.0; ROW]);
        // A quarter reads its parent's ground temperature, liquid depth and height (flat quarters), a joined cell their
        // mean by area, until the next survey reads them from Flow's state (`survey::env_of`).
        *env = r.apply_made(std::mem::take(env), || [0.0; 3], |_, m, at| std::array::from_fn(|x| m.mean(|i| at(i)[x])));
        (*absorb, *keep) = (r.apply(std::mem::take(absorb), || 0.0), r.apply(std::mem::take(keep), || 0.0));
        *feat = r.apply(std::mem::take(feat), || [0.0; crate::choose::NCELL]);
        self.index();
    }
}

/// Cell `c`'s life shared out to its quarters `kids` (row by row, each `area[q]` of its area), each body and part in
/// the order it lies there: each structure whole, with its bonds and the bodies that hold a part of it or are inside it,
/// to one quarter (`structures`), but a lot that lies alone of at least one part per quarter's share leaves in each other
/// quarter a lot of that share of its parts, amounts and heat (`Part::split`, an id of that quarter); each other body
/// whole to the quarter drawn by its id (`draw`), but a node of at least one member per quarter's share leaves in each
/// other quarter a node of that share of its members, stores, heat and brood (`nodes::part`, its genes, spreads,
/// lineage and born rows; an id of that quarter, and a block there holding the node's brain, rows, moving averages and
/// trace). Each slot of the cover and its heat by area (`Cover::quarter`), and
/// what the movers did there by area.
fn share(mut c: Cell, kids: &mut [Cell; 4], area: [f64; 4], seed: u64) {
    let key = (seed, c.serial);
    let (root, go) = structures(&c, area, key);
    let bonds = std::mem::take(&mut c.bonds);
    for l in 0..bonds.count() {
        kids[go[&root[&bonds.a[l]]]].bonds.add(bonds.a[l], bonds.b[l], bonds.strength[l]);
    }
    let fewest = area.iter().copied().fold(f64::INFINITY, f64::min);
    let tied: BTreeSet<u64> = bonds.a.iter().chain(&bonds.b).chain(c.bodies.iter().filter(|b| b.alive).flat_map(|b| &b.inside)).copied().collect();
    for mut p in std::mem::take(&mut c.parts) {
        let q = go[&p.root];
        if p.held == NOBODY && !tied.contains(&p.id) && p.count * fewest >= 1.0 {
            let mut left = 1.0;
            for o in (0..4).filter(|&o| o != q) {
                let mut k = p.split(p.count * area[o] / left);
                left -= area[o];
                let kid = &mut kids[o];
                kid.parts_made += 1;
                (k.id, k.root) = (new_id(kid.serial, kid.parts_made), new_id(kid.serial, kid.parts_made));
                kid.parts.push(k);
            }
        }
        kids[q].parts.push(p);
    }
    for mut b in std::mem::take(&mut c.bodies).into_iter().filter(|b| b.alive) {
        let tied = [b.hold, b.inside].into_iter().flatten().find_map(|id| root.get(&id)).map(|r| go[r]);
        let q = tied.unwrap_or_else(|| draw(area, key, b.id));
        let all = c.brains.all(b.brain);
        if tied.is_none() && b.count * fewest >= 1.0 {
            let mut left = 1.0;
            for o in (0..4).filter(|&o| o != q) {
                let mut k = crate::nodes::part(&mut b, area[o] / left, area[o] / left);
                left -= area[o];
                let kid = &mut kids[o];
                kid.placed += 1;
                (k.id, k.brain, k.traced, k.stale) = (new_id(kid.serial, kid.placed), kid.brains.take_in(all), b.traced, true);
                kid.bodies.push(k);
            }
            b.stale = true;
        }
        b.brain = kids[q].brains.take_in(all);
        kids[q].bodies.push(b);
    }
    for (k, &a) in kids.iter_mut().zip(&area) {
        (k.cover, k.pushed, k.crossings) = (c.cover.quarter(a), c.pushed * a, c.crossings * a as f32);
    }
}

/// Of the parts of cell `c`: the root of each, by its id, and the quarter each structure goes to whole (`draw`), by its
/// root: the structures a body holds a part of and is inside of go together, drawn by the least of their roots.
fn structures(c: &Cell, area: [f64; 4], key: (u64, u32)) -> (BTreeMap<u64, u64>, BTreeMap<u64, usize>) {
    let root: BTreeMap<u64, u64> = c.parts.iter().map(|p| (p.id, p.root)).collect();
    // Each root's link toward the least root of those it goes with (itself where it goes alone).
    let mut with: BTreeMap<u64, u64> = root.values().map(|&r| (r, r)).collect();
    let least = |with: &BTreeMap<u64, u64>, mut r: u64| {
        while with[&r] != r {
            r = with[&r];
        }
        r
    };
    for b in c.bodies.iter().filter(|b| b.alive) {
        if let (Some(x), Some(y)) = (b.hold.and_then(|id| root.get(&id)), b.inside.and_then(|id| root.get(&id))) {
            let (x, y) = (least(&with, *x), least(&with, *y));
            with.insert(x.max(y), x.min(y));
        }
    }
    let go = with.keys().map(|&r| (r, draw(area, key, least(&with, r)))).collect();
    (root, go)
}

/// The quarter drawn for what is named `id` (a body, or structures by their least root) when cell `key.1` (a serial)
/// splits, in a world of seed `key.0`: each quarter with the chance of its share of the area. A cell splits once (its
/// quarters are new cells), so no two splits share a draw.
fn draw(area: [f64; 4], (seed, serial): (u64, u32), id: u64) -> usize {
    let x = Rng::new(seed, serial as u64, id, Purpose::Quarter).f64() * area.iter().sum::<f64>();
    let mut below = 0.0;
    (0..3).find(|&q| {
        below += area[q];
        x < below
    })
    .unwrap_or(3)
}

/// Quarters `kids` (row by row, each `area[q]` of the joined cell's area) gathered into cell `c`, cell `to` of `chem`:
/// their bodies and parts in their order, quarter by quarter, each body with its brain block (`Brains::take_in`), and
/// their bonds; the slots of their cover as seeds to land (`Cover::gathered`); what the movers did there summed.
fn gather(c: &mut Cell, kids: [Cell; 4], area: [f64; 4], (chem, to): (&Chemistry, usize)) {
    c.cover = Cover::gathered(kids.each_ref().map(|k| &k.cover), area, chem, to);
    for k in kids {
        for mut b in k.bodies.into_iter().filter(|b| b.alive) {
            b.brain = c.brains.take_in(k.brains.all(b.brain));
            c.bodies.push(b);
        }
        c.parts.extend(k.parts);
        c.bonds.extend(k.bonds);
        (c.pushed, c.crossings) = (c.pushed + k.pushed, c.crossings + k.crossings);
    }
}

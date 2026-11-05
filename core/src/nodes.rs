//! Population nodes: the monthly merge in each cell to one node per kind and stage (`crowd`), a body joining another
//! (`merge`, `add`), and a part of a node coming back to it (`rejoin`). Members, stores and heat are kept exactly in
//! every join.

use crate::bond::{held, let_go, room};
use crate::body::{self, alike, most, range, spread_at, spread_kept, Body, Layout, BRAIN_SIZES};
use crate::brain::row_step;
use crate::life::Block;
use crate::chem::Stores;
use crate::lineage::sizes_of;
use crate::life::{Cell, Handle};
use crate::{life::Chunk, World};

/// The monthly merge of the bodies of cell `i` of chunk `d` (laws.md 4.4): one node per kind and stage. Each body
/// joins the largest of its `Kin` (`join_kin`); then a node joins a larger one of another lineage of its stage, room
/// and part whose genes are alike (`body::alike`, on the means) and whose brain has the same sizes, taking its brain and
/// lineage (`join_other`); then the small bodies of each lineage, stage and room join its largest, or a cell over
/// `MAX_NODES` bodies keeps one young and one mature node per kind and room (`cap`).
pub fn crowd(w: &World, d: &mut Chunk, lay: &Layout, i: usize) {
    let heads = join_kin(w, d, i);
    join_other(w, d, lay, heads);
    cap(w, d, i);
}

/// Bodies a cell keeps at most after its monthly merge; past them each starting kind keeps one young and one mature
/// node in each room, and past them again across its rooms (`across_rooms`; a speed setting).
pub const MAX_NODES: usize = 8;

/// In a cell `i` of more than `MAX_NODES` bodies, the bodies of each starting kind (`Lineage::root`), stage and room (the
/// structure whose room they are in, or none) join the largest of them (equals: the first in the cell's order), whatever
/// their lineage, brain sizes and part, taking its brain, born rows and lineage when that is another (`merge`; the part a
/// joiner held stays in the cell, held by none): the cell then holds at most a young and a mature node of each kind
/// present in each room, and if that is still more than `MAX_NODES` bodies, the rooms are bounded too (`across_rooms`).
/// In any other cell a body of one lineage, stage and room with fewer members than
/// `Physics::node_floor` of the largest of them joins it, unless it may mate (`copy::mates`).
pub fn cap(w: &World, d: &mut Chunk, i: usize) {
    let mut left: Vec<Handle> = d.cell(i).collect();
    let all = left.len() > MAX_NODES;
    // (A stable sort: equals keep the cell's order.)
    left.sort_by(|&s, &t| d[t].n().total_cmp(&d[s].n()));
    let mut heads: Vec<(_, Handle)> = Vec::new();
    for s in left {
        let b = &d[s];
        let kind = (d.root(w, b), b.adult(&w.chem), room(d.at(i), b).map(|r| r.0), (!all).then_some(b.lineage));
        match heads.iter().find(|h| h.0 == kind) {
            Some(&(_, t)) if all || (b.count < w.cfg.phys.node_floor * d[t].count && !crate::copy::mates(w, d, b)) => merge(w, d, s, t),
            Some(_) => {}
            None => heads.push((kind, s)),
        }
    }
    if all {
        across_rooms(w, d, i);
    }
}

/// A cell `i` that still holds more than `MAX_NODES` bodies after `cap` (bodies of a kind and stage in the rooms of many
/// structures): its `MAX_NODES` largest stay, and each body past them joins the largest of its starting kind and stage
/// there, whatever their rooms (`merge`, as in `cap`), or stays if it is the largest of its kind and stage. The joiners'
/// members leave their room for that body's, where they stay while they fit (`bond::leave_outgrown`).
fn across_rooms(w: &World, d: &mut Chunk, i: usize) {
    let mut left: Vec<Handle> = d.cell(i).collect();
    if left.len() <= MAX_NODES {
        return;
    }
    // (A stable sort: equals keep the cell's order.)
    left.sort_by(|&s, &t| d[t].n().total_cmp(&d[s].n()));
    let mut heads: Vec<((u32, bool), Handle)> = Vec::new();
    for (k, s) in left.into_iter().enumerate() {
        let kind = (d.root(w, &d[s]), d[s].adult(&w.chem));
        match heads.iter().find(|h| h.0 == kind) {
            Some(&(_, t)) if k >= MAX_NODES => merge(w, d, s, t),
            Some(_) => {}
            None => heads.push((kind, s)),
        }
    }
}

/// What a node shares with those it joins (`join_kin`): its lineage (a node has one brain), its stage (both mature or
/// both young: a young node joins its adult kin at the merge once it matures), the structure whose room it is in (its
/// root, as the room law counts those inside, `bond::room`) and the structure it holds (its root).
type Kin = (u32, bool, Option<u64>, Option<u64>);

/// The `Kin` of body `o` of cell `c`.
fn kin_of(w: &World, c: &Cell, o: &Body) -> Kin {
    (o.lineage, o.adult(&w.chem), room(c, o).map(|r| r.0), held(c, o).map(|p| c.parts[p].root))
}

/// Individuals of one `Kin` a cell keeps apart where they may mate (a pair, `copy::mates`); past this many, with no node
/// of theirs, they merge into one (a speed setting).
const PAIR: usize = 2;

/// Each body of cell `i`, largest first (equals in the cell's order), joins the largest of its `Kin`, where that is a
/// node, its kind has more than `PAIR` bodies there or its individuals may not mate (`copy::mates`). Returns the bodies
/// left, largest first.
fn join_kin(w: &World, d: &mut Chunk, i: usize) -> Vec<Handle> {
    let b = &*d;
    let mut list: Vec<(u64, Handle)> = b.cell(i).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
    list.sort_by_key(|x| x.0);
    let kind: Vec<Kin> = list.iter().map(|&(_, s)| kin_of(w, b.at(i), &b[s])).collect();
    let many = |x: usize| !crate::copy::mates(w, b, &b[list[x].1]) || kind.iter().filter(|&&k| k == kind[x]).count() > PAIR;
    let head: Vec<usize> = (0..list.len()).map(|x| (0..x).find(|&y| kind[y] == kind[x] && many(y)).unwrap_or(x)).collect();
    for x in (0..list.len()).filter(|&x| head[x] != x) {
        merge(w, d, list[x].1, list[head[x]].1);
    }
    (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect()
}

/// Each node of `heads` (one per `Kin`, largest first) joins the first larger node of another lineage of its starting
/// kind and the same stage, room and part, alike in genes, whose brain has the same sizes, taking its brain, born rows and
/// lineage (`merge`). Individuals stay apart.
fn join_other(w: &World, d: &mut Chunk, lay: &Layout, heads: Vec<Handle>) {
    let heads: Vec<Handle> = heads.into_iter().filter(|&s| d[s].n() > 1.0).collect();
    let mut gone = vec![false; heads.len()];
    for x in 1..heads.len() {
        let c = d.at(heads[x].cell as usize);
        let (a, ka) = (&d[heads[x]], kin_of(w, c, &d[heads[x]]));
        let sizes = |k: u32| d.brain(w, k).map(sizes_of);
        let to = (0..x).filter(|&y| !gone[y]).find(|&y| {
            let (t, kt) = (&d[heads[y]], kin_of(w, c, &d[heads[y]]));
            kt.0 != ka.0 && d.root(w, t) == d.root(w, a) && (kt.1, kt.2, kt.3) == (ka.1, ka.2, ka.3) && sizes(kt.0) == sizes(ka.0) && alike(lay, w.cfg.phys.alike, &t.genes, &a.genes)
        });
        if let Some(y) = to {
            merge(w, d, heads[x], heads[y]);
            gone[x] = true;
        }
    }
}

/// The body at `s` joins the body at `t` (`add`), letting go of what it holds; of another lineage (of its starting kind),
/// its members take `t`'s brain, born rows and lineage.
pub fn merge(w: &World, d: &mut Chunk, s: Handle, t: Handle) {
    debug_assert_eq!(d.root(w, &d[s]), d.root(w, &d[t]), "a merge across starting kinds");
    let mut c = std::mem::take(&mut d[s]);
    let_go(d, &mut c, false);
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread, c.body_spread), (c.lineage, c.genome, Some(Own::Block(c.brain)), c.row_spread));
    d.remove(s, &c);
}

/// What splits and joins as an amount when members move between nodes, split by shares with the remainder kept so the
/// parts add up: count, mol of each store, heat (J), brood (mol), their last day and their share that is busy
/// (`Body::busy`).
pub(crate) struct Lot {
    pub count: f64,
    pub store: Stores,
    pub e: f64,
    pub brood: f64,
    pub last: f64,
    pub busy: f32,
}

impl Lot {
    /// The share `f` of node `b`'s members and the share `fa` of its stores, heat and brood leave it; it keeps the rest
    /// (the remainder, so the two add up to what it held to rounding). The same share of both is members alike to the
    /// others; a larger share of the amounts is members that hold more than the mean.
    pub(crate) fn split(b: &mut Body, f: f64, fa: f64) -> Lot {
        let store = b.store.map(|a| fa * a);
        b.store.iter_mut().zip(&store).for_each(|(a, x)| *a -= x);
        let c = Lot { count: f * b.count, store, e: fa * b.e, brood: fa * b.brood, last: b.last, busy: b.busy };
        (b.e, b.brood) = (b.e - c.e, b.brood - c.brood);
        b.count -= c.count;
        c
    }
    /// All that body `c` holds (its stores taken out of it).
    pub(crate) fn of(c: &mut Body) -> Lot {
        Lot { count: c.count, store: std::mem::take(&mut c.store), e: c.e, brood: c.brood, last: c.last, busy: c.busy }
    }
}

/// The own rows of members who join a node (`add`): their block in the node's cell's brains, or the numbers of a block
/// they carried from another cell (`Brains::all`, its rows first).
pub(crate) enum Own<'a> {
    Block(Block),
    Carried(&'a [f32]),
}

/// Members `c` join the body at `t` with genes and spread `genes`, the spread of their size `sizes` (`Body::body_spread`)
/// and rows `rows` (their lineage, the genome of their born rows, their own rows, none: `t`'s, and their row spread):
/// amounts add up (`rejoin`), genes, sizes and own rows become the means by members and the spreads pool around them; `t` keeps its born rows, the joiners' gap to them pooled
/// into its row spread (laws.md 4.4). Members of another lineage (of its starting kind) take `t`'s brain, born rows,
/// lineage and brain-size genes (`BRAIN_SIZES`, with their spread): its own rows become the mean by members where the
/// brains have one shape, else they stay `t`'s.
pub(crate) fn add(w: &World, d: &mut Chunk, t: Handle, c: Lot, (genes, spread, sizes): (&[f32], &[f32], f32), (lineage, genome, own, row_spread): (u32, u32, Option<Own>, f32)) {
    let mut o = std::mem::take(&mut d[t]);
    let (other, theirs) = (lineage != o.lineage, genome);
    let (genome, row_spread) = if other { (o.genome, o.row_spread) } else { (genome, row_spread) };
    let own = own.filter(|_| !other || d.shape_of(w, lineage) == d.shape_of(w, o.lineage));
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.count / (o.count + c.count)) as f32);
    let mix = |p: f32, q: f32| p + a * (q - p);
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    // The rows first: their spread's step reads the genes as they were. The node keeps its born rows: the joiners' gap
    // to them goes into its spread around them. (Born rows of one genome have no gap.)
    let gap = if o.genome == genome { 0.0 } else {
        let (was, born) = (d.born_of(w, o.genome), d.born_of(w, genome));
        crate::brain::square_gap(was, born) / was.len().max(1) as f32
    };
    let step = row_step(p, body::mutation_size(&o.genes));
    o.row_spread = ((1.0 - a) * o.row_spread + a * (row_spread + gap)).min(most(step, p.alike));
    // Each gene's mean and the spread pooled around it, in place.
    let mut pooled = std::mem::take(&mut o.spread);
    pooled.resize(o.genes.len(), 0.0);
    for (g, (x, v)) in o.genes.iter_mut().zip(pooled.iter_mut()).enumerate() {
        let (y, u) = if other && BRAIN_SIZES.contains(&g) { (*x, v.max(0.0)) } else { (genes[g], spread_at(spread, g)) };
        let mean = mix(*x, y);
        *v = spread_kept(&lay, g, mean, pool(v.max(0.0), u, (y - *x).powi(2), range(&lay, g).0));
        *x = mean;
    }
    o.spread = pooled;
    o.prepare(&w.chem, &w.cfg);
    // Its own rows: its born rows and the mean by members of what each learned (`Chunk::blend_learned`).
    if let (Some(own), Some(s)) = (own, d.shape_of(w, o.lineage)) {
        d.blend_learned(w, (o.cell as usize, o.brain), own, (o.genome, theirs), s, a);
    }
    // The spread of their sizes pools with the gap of their mean member to the node's: the living body the join adds is
    // theirs.
    let (before, members, joiners) = (o.built(&w.chem), o.count.max(1e-300), c.count.max(1e-300));
    rejoin(&mut o, c);
    let gap = ((o.built(&w.chem) - before) / joiners - before / members) as f32;
    o.body_spread = (1.0 - a) * o.body_spread + a * sizes + a * (1.0 - a) * gap * gap;
    o.stale = true;
    d[t] = o;
}

/// Members `c` with the genes and spreads of body `o` join it (taken out of its slot): the members of a node who waited
/// aside while the others acted, a part of a node that moved or bonded and came back to its cell (both
/// `choose::spread`), or joiners after `add` pooled their genes, sizes and rows. Amounts add up; the last day and the
/// share that is busy become the means by members; the spread of sizes stays (members who came back built nothing
/// apart: they differ from the others only by what they ate, not by their body). `o`'s visible traits are the caller's to work out again (`Chunk::see`).
pub(crate) fn rejoin(o: &mut Body, c: Lot) {
    let share = c.count / (o.count + c.count);
    o.last += share * (c.last - o.last);
    o.busy += share as f32 * (c.busy - o.busy);
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    o.brood += c.brood;
    o.e += c.e;
    o.count += c.count;
}

/// Share `f` of node `b`'s members and share `fa` of each of its stores, of its brood and of its heat as a new body
/// (not placed, `Lot::split`); lineage, genome, genes and their constants, link and memory as the node's; no brain block
/// of its own (one acting on the node's decision gets a copy of the node's brain if it leaves, `choose::spread`; a
/// node's share in a quarter of its split cell gets one there, `life::tree`); holding nothing, inside nothing.
pub(crate) fn part(b: &mut Body, f: f64, fa: f64) -> Body {
    let lot = Lot::split(b, f, fa);
    Body { count: lot.count, store: lot.store, e: lot.e, brood: lot.brood, hold: None, inside: None, leaving: false, brain: Block::default(), traced: false, ..b.clone() }
}

/// A count this close to a whole number is that number.
pub(crate) const COUNT_SLACK: f64 = 1e-9;

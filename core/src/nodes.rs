//! Population nodes: the monthly merge in each cell to one node per kind and stage (`crowd`), a body joining another
//! (`merge`, `add`), and a part of a node coming back to it (`rejoin`). Members, stores and heat are kept exactly in
//! every join.

use crate::bond::{held, let_go, room};
use crate::body::{self, alike, distance, most, range, spread_at, spread_kept, Body, Layout};
use crate::brain::{blend, row_step};
use crate::chem::Stores;
use crate::lineage::sizes_of;
use crate::life::Handle;
use crate::{life::Chunk, World};

/// The monthly merge of the bodies of cell `i` of chunk `d` (laws.md 4.4): one node per kind and stage. Each body
/// joins the largest of its `Kin` (`join_kin`); then a node joins a larger one of another lineage of its stage, room
/// and part whose genes are alike (`body::alike`, on the means) and whose brain has the same sizes, taking its brain and
/// lineage (`join_other`); then the cell keeps at most `MAX_NODES` bodies (`cap`).
pub(crate) fn crowd(w: &World, d: &mut Chunk, lay: &Layout, i: usize) {
    let heads = join_kin(w, d, i);
    join_other(w, d, lay, heads);
    cap(w, d, lay, i);
}

/// Bodies a cell keeps at most after its monthly merge; past them the most alike pair of one stage, starting kind and
/// brain sizes merges (a speed setting).
pub const MAX_NODES: usize = 8;

/// While cell `i` holds more than `MAX_NODES` bodies that may be nodes, the pair of one stage, one starting kind
/// (`Lineage::root`) and one brain's sizes nearest in genes (in mutation steps, `body::distance`) merges, whatever its
/// lineage, room and part: the smaller joins the larger, taking its brain, born rows and lineage when that is another
/// (`merge_taking`). Where no such pair is left the cell stays over the cap.
pub fn cap(w: &World, d: &mut Chunk, lay: &Layout, i: usize) {
    let left: Vec<Handle> = d.cell(i).collect();
    if left.len() <= MAX_NODES {
        return;
    }
    let m = left.len();
    let kind = |d: &Chunk, s: Handle| {
        let b = &d[s];
        (b.adult(&w.chem), d.brain(w, b.lineage).map(|l| (l.root, sizes_of(l))))
    };
    let gap = |d: &Chunk, s: Handle, t: Handle| {
        let (ks, kt) = (kind(d, s), kind(d, t));
        if ks.1.is_none() || ks != kt { f32::INFINITY } else { distance(lay, &d[s].genes, &d[t].genes, |_| f32::INFINITY).unwrap_or(f32::INFINITY) }
    };
    // The gap of each pair `p` < `q` lies at `p * m + q`.
    let mut far = vec![f32::INFINITY; m * m];
    for p in 0..m {
        for q in p + 1..m {
            far[p * m + q] = gap(d, left[p], left[q]);
        }
    }
    let mut gone = vec![false; m];
    for _ in MAX_NODES..m {
        let k = (0..m * m).fold(0, |k, x| if far[x] < far[k] { x } else { k });
        if far[k] == f32::INFINITY {
            break;
        }
        let (p, q) = (k / m, k % m);
        let (s, t) = if d[left[q]].n() > d[left[p]].n() { (p, q) } else { (q, p) };
        if d[left[s]].lineage == d[left[t]].lineage {
            merge(w, d, left[s], left[t]);
        } else {
            merge_taking(w, d, left[s], left[t]);
        }
        gone[s] = true;
        // The joiner's pairs are gone; the node it joined has new means.
        for (o, &out) in gone.iter().enumerate() {
            let (lo, hi) = (o.min(s), o.max(s));
            far[lo * m + hi] = f32::INFINITY;
            if o != t && o != s && !out {
                let (lo, hi) = (o.min(t), o.max(t));
                far[lo * m + hi] = gap(d, left[lo], left[hi]);
            }
        }
    }
}

/// What a node shares with those it joins (`join_kin`): its lineage (a node has one brain), its stage (both mature or
/// both young: a young node joins its adult kin at the merge once it matures), the structure whose room it is in (its
/// root, as the room law counts those inside, `bond::room`) and the structure it holds (its root).
type Kin = (u32, bool, Option<usize>, Option<usize>);

/// The `Kin` of body `o`.
fn kin_of(w: &World, o: &Body) -> Kin {
    (o.lineage, o.adult(&w.chem), room(w, o).map(|r| r.0), held(w, o).map(|p| w.parts.parts[p].root as usize))
}

/// Individuals of one `Kin` a cell keeps apart (a pair may mate, `copy::partner`); past this many, with no node of
/// theirs, they merge into one (a speed setting).
const PAIR: usize = 2;

/// Each body of cell `i` that may be a node, largest first (equals in the cell's order), joins the largest of its
/// `Kin`, where that is a node or its kind has more than `PAIR` bodies there. Returns the bodies left, largest first.
fn join_kin(w: &World, d: &mut Chunk, i: usize) -> Vec<Handle> {
    let b = &*d;
    let mut list: Vec<(u64, Handle)> = b.cell(i).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
    list.sort_by_key(|x| x.0);
    let kind: Vec<Kin> = list.iter().map(|&(_, s)| kin_of(w, &b[s])).collect();
    let many = |x: usize| b[list[x].1].n() > 1.0 || kind.iter().filter(|&&k| k == kind[x]).count() > PAIR;
    let head: Vec<usize> = (0..list.len()).map(|x| (0..x).find(|&y| kind[y] == kind[x] && many(y)).unwrap_or(x)).collect();
    for x in (0..list.len()).filter(|&x| head[x] != x) {
        merge(w, d, list[x].1, list[head[x]].1);
    }
    (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect()
}

/// Each node of `heads` (one per `Kin`, largest first) joins the first larger node of another lineage of its starting
/// kind and the same stage, room and part, alike in genes, whose brain has the same sizes (`merge_taking`). Individuals stay apart.
fn join_other(w: &World, d: &mut Chunk, lay: &Layout, heads: Vec<Handle>) {
    let heads: Vec<Handle> = heads.into_iter().filter(|&s| d[s].n() > 1.0).collect();
    let mut gone = vec![false; heads.len()];
    for x in 1..heads.len() {
        let (a, ka) = (&d[heads[x]], kin_of(w, &d[heads[x]]));
        let sizes = |k: u32| d.brain(w, k).map(sizes_of);
        let to = (0..x).filter(|&y| !gone[y]).find(|&y| {
            let (t, kt) = (&d[heads[y]], kin_of(w, &d[heads[y]]));
            kt.0 != ka.0 && d.root(w, t) == d.root(w, a) && (kt.1, kt.2, kt.3) == (ka.1, ka.2, ka.3) && sizes(kt.0) == sizes(ka.0) && alike(lay, w.cfg.phys.alike, &t.genes, &a.genes)
        });
        if let Some(y) = to {
            merge_taking(w, d, heads[x], heads[y]);
            gone[x] = true;
        }
    }
}

/// The body at `s` joins the body at `t` of another lineage of its starting kind whose brain has its sizes: its members
/// take `t`'s brain, born rows and lineage (the means of `t`'s rows stay `t`'s), letting go of what they hold.
pub fn merge_taking(w: &World, d: &mut Chunk, s: Handle, t: Handle) {
    debug_assert_eq!(d.root(w, &d[s]), d.root(w, &d[t]), "a merge across starting kinds");
    let (born, rows, spread) = (d.born(w, &d[t]).to_vec(), d[t].rows.clone(), d[t].row_spread);
    let mut c = std::mem::take(&mut d[s]);
    let_go(w, d, &mut c, false);
    d.remove(s, &c);
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread), (&born, &rows, spread));
}

/// The body at `s` joins the body at `t`, of its lineage (`add`), letting go of what it holds.
pub fn merge(w: &World, d: &mut Chunk, s: Handle, t: Handle) {
    let (born, mut c) = (d.born(w, &d[s]).to_vec(), std::mem::take(&mut d[s]));
    let_go(w, d, &mut c, false);
    d.remove(s, &c);
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread), (&born, &c.rows, c.row_spread));
}

/// What splits and joins as an amount when members move between nodes, split by shares with the remainder kept so the
/// parts add up: count, mol of each store, heat (J), brood (mol), and their last day.
pub(crate) struct Lot {
    pub count: f64,
    pub store: Stores,
    pub e: f64,
    pub brood: f64,
    pub last: f64,
}

impl Lot {
    /// The share `f` of node `b`'s members and the share `fa` of its stores, heat and brood leave it; it keeps the rest
    /// (the remainder, so the two add up to what it held to rounding). The same share of both is members alike to the
    /// others; a larger share of the amounts is members that hold more than the mean.
    pub(crate) fn split(b: &mut Body, f: f64, fa: f64) -> Lot {
        let store = b.store.map(|a| fa * a);
        b.store.iter_mut().zip(&store).for_each(|(a, x)| *a -= x);
        let c = Lot { count: f * b.count, store, e: fa * b.e, brood: fa * b.brood, last: b.last };
        (b.e, b.brood) = (b.e - c.e, b.brood - c.brood);
        b.count -= c.count;
        c
    }
    /// All that body `c` holds (its stores taken out of it).
    pub(crate) fn of(c: &mut Body) -> Lot {
        Lot { count: c.count, store: std::mem::take(&mut c.store), e: c.e, brood: c.brood, last: c.last }
    }
}

/// Members `c` join the body at `t` with genes and spread `genes` and rows `rows` (born rows, rows, row spread):
/// amounts add up (`rejoin`), genes and rows become the means by members and the spreads pool around them (laws.md 4.4).
pub(crate) fn add(w: &World, d: &mut Chunk, t: Handle, c: Lot, (genes, spread): (&[f32], &[f32]), (born, own, row_spread): (&[f32], &[f32], f32)) {
    let mut o = std::mem::take(&mut d[t]);
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.count / (o.count + c.count)) as f32);
    let mix = |p: f32, q: f32| p + a * (q - p);
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    // The rows first: their spread's step reads the genes as they were.
    let was = d.born(w, &o);
    let gap = was.iter().zip(born).map(|(p, q)| (q - p) * (q - p)).sum::<f32>() / was.len().max(1) as f32;
    o.row_spread = pool(o.row_spread, row_spread, gap, row_step(p, body::mutation_size(&o.genes)));
    d.reborn(w, &mut o, was.iter().zip(born).map(|(&p, &q)| mix(p, q)).collect());
    // Each gene's mean and the spread pooled around it, in place.
    let mut pooled = std::mem::take(&mut o.spread);
    pooled.resize(o.genes.len(), 0.0);
    for (g, (x, v)) in o.genes.iter_mut().zip(&mut pooled).enumerate() {
        let mean = mix(*x, genes[g]);
        *v = spread_kept(&lay, g, mean, pool(v.max(0.0), spread_at(spread, g), (genes[g] - *x).powi(2), range(&lay, g).0));
        *x = mean;
    }
    o.spread = pooled;
    o.prepare(&w.chem, &w.cfg);
    rejoin(&mut o, c, Some(own));
    o.stale = true;
    d[t] = o;
}

/// Members `c` with the genes and spreads of body `o` join it (taken out of its slot): the members of a node who waited
/// aside while the others acted, a part of a node that moved or bonded and came back to its cell (both
/// `choose::spread`), or joiners after `add` pooled their genes. Amounts add up; the last
/// day becomes the mean by members; `rows`: the rows of `c` where they differ from `o`'s, which `o`'s
/// become the mean by members of. `o`'s visible traits are the caller's to work out again (`Chunk::see`).
pub(crate) fn rejoin(o: &mut Body, c: Lot, rows: Option<&[f32]>) {
    let share = c.count / (o.count + c.count);
    o.last += share * (c.last - o.last);
    let a = share as f32;
    if let Some(rows) = rows {
        blend(&mut o.rows, rows, a);
    }
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    o.brood += c.brood;
    o.e += c.e;
    o.count += c.count;
}

/// Share `f` of node `b`'s members and share `fa` of each of its stores, of its brood and of its heat as a new body
/// (not placed, `Lot::split`); lineage, genome, genes and their constants, brain rows, link and memory as the node's;
/// holding nothing, inside nothing.
pub(crate) fn part(b: &mut Body, f: f64, fa: f64) -> Body {
    let lot = Lot::split(b, f, fa);
    // (The trace is not the part's: set aside, not cloned.)
    let trace = std::mem::take(&mut b.trace);
    let c = Body { count: lot.count, store: lot.store, e: lot.e, brood: lot.brood, hold: None, inside: None, leaving: false, ..b.clone() };
    b.trace = trace;
    c
}

/// A count this close to a whole number is that number.
pub(crate) const COUNT_SLACK: f64 = 1e-9;

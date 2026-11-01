//! Population nodes: the monthly merge of alike nodes in each cell (`crowd`), a body joining another (`merge`,
//! `add`), a part of a node coming back to it (`rejoin`), a node splitting in two (`split`), and members drawn from a
//! node as individuals (`open`). Members, stores and heat are kept exactly in every join and split.

use crate::bond::{held, let_go, room};
use crate::body::{self, alike, likes, most, noise, range, spread_at, spread_kept, spreads, steps_by, wall_room, within_steps, Body, Layout, REACH};
use crate::brain::{blend, row_step, shake};
use crate::rng::{Purpose, Rng};
use crate::life::Handle;
use crate::{life::Domain, World};
use std::f32::consts::FRAC_2_PI;

/// The monthly merge of nodes in cell `i` of domain `d`, a speed setting, not a law: a node joins the first larger one
/// alike to it (`join_alike`), then past `Config::crowd` the most alike pairs merge (`merge_closest`); laws.md 4.4.
pub(crate) fn crowd(w: &World, d: &mut Domain, lay: &Layout, i: usize) {
    let (left, max) = (join_alike(w, d, lay, i), w.cfg.crowd.max(1));
    if left.len() > max {
        merge_closest(w, d, lay, left, max);
    }
}

/// What a node shares with those it joins (`join_alike`): its lineage (a node has one brain), its stage (both mature or
/// both young: the young join their adult node member by member as they mature, `copy::young`), the structure whose
/// room it is in (its root, as the room law counts those inside, `bond::room`) and the structure it holds (its root).
type Kin = (u32, bool, Option<usize>, Option<usize>);

/// Each body of cell `i`, largest first (equals in the cell's order), however few its members, joins the first larger node (more than one member,
/// as in `copy::kin`) alike in genes (`body::alike`, on the means only, so a split's halves stay apart for selection)
/// of its `Kin`. Returns the bodies left.
fn join_alike(w: &World, d: &mut Domain, lay: &Layout, i: usize) -> Vec<Handle> {
    let b = &*d;
    let mut list: Vec<(u64, Handle)> = b.cell(i).filter(|&s| may_be_node(w, &b[s])).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
    list.sort_by_key(|x| x.0);
    let mut head: Vec<usize> = (0..list.len()).collect();
    // Each one's kin once, tested before the genes.
    let kin_key = |o: &Body| (o.lineage, o.adult(&w.chem), room(w, o).map(|r| r.0), held(w, o).map(|p| w.parts.parts[p].root as usize));
    let kind: Vec<Kin> = list.iter().map(|&(_, s)| kin_key(&b[s])).collect();
    for x in 0..list.len() {
        let a = &b[list[x].1];
        let joins = |y: usize| head[y] == y && b[list[y].1].n() > 1.0 && kind[y] == kind[x];
        head[x] = (0..x).find(|&y| joins(y) && alike(lay, w.cfg.phys.alike, &b[list[y].1].genes, &a.genes)).unwrap_or(x);
    }
    for x in (0..list.len()).filter(|&x| head[x] != x) {
        merge(w, d, list[x].1, list[head[x]].1);
    }
    (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect()
}

/// A body's genes in steps (`body::steps_by`), its spread per gene (`body::spreads`), and its lineage and stage
/// (`merge_closest`).
type Likeness = (Vec<f32>, Vec<f32>, (u32, bool));

/// The most alike pair of `left` merges first, the smaller into the larger (ties: the earlier, larger ones), until `max`
/// are left or no pair is alike: here each within the other's spread, as children join a node (`body::within`), of one
/// lineage and stage; unlike nodes never merge, however full the cell. What a body holds and the room it is in do not
/// keep it apart in a full cell: they are states of its members, not a likeness; the one that joins lets go of its
/// part and takes the other's room (`merge`). A pair's key: the squared distance of their genes in steps (infinite if
/// unlike).
fn merge_closest(w: &World, d: &mut Domain, lay: &Layout, left: Vec<Handle>, max: usize) {
    let (m, lim) = (left.len(), w.cfg.phys.alike);
    let units = likes(lay, lay.len);
    let steps = |d: &Domain, s: Handle| {
        let b = &d[s];
        (steps_by(lay, &units, &b.genes), spreads(&b.spread, b.genes.len()), (b.lineage, b.adult(&w.chem)))
    };
    let mut v: Vec<Likeness> = left.iter().map(|&s| steps(d, s)).collect();
    let dist = |p: &Likeness, q: &Likeness| {
        let gap = |g: usize| (p.0[g] - q.0[g]).abs() > within_steps(lim, p.1[g] + q.1[g], units[g]);
        if p.2 != q.2 || (0..p.0.len()).any(gap) { f32::INFINITY } else { p.0.iter().zip(&q.0).map(|(x, y)| (x - y) * (x - y)).sum() }
    };
    // The key of each pair `p`, `q` lies at `at(p, q)`, the smaller first.
    let (mut far, mut gone, at) = (vec![f32::INFINITY; m * m], vec![false; m], |p: usize, q: usize| p.min(q) * m + p.max(q));
    for p in 0..m {
        for q in p + 1..m {
            far[at(p, q)] = dist(&v[p], &v[q]);
        }
    }
    for _ in max..m {
        let k = (0..m * m).fold(0, |k, x| if far[x] < far[k] { x } else { k });
        if far[k] == f32::INFINITY {
            break;
        }
        let (p, q) = (k / m, k % m);
        let (s, t) = if d[left[q]].n() > d[left[p]].n() { (p, q) } else { (q, p) };
        merge(w, d, left[s], left[t]);
        (gone[s], v[t]) = (true, steps(d, left[t]));
        for o in 0..m {
            far[at(o, s)] = f32::INFINITY;
            if o != t && !gone[o] {
                far[at(o, t)] = dist(&v[o], &v[t]);
            }
        }
    }
}

/// Body `b` may be or join a population node: any body with `Config::nodes`, else only one without a brain (the check
/// of nodes against individuals compares brains in nodes and alone; a body with no brain decides nothing).
pub(crate) fn may_be_node(w: &World, b: &Body) -> bool {
    w.cfg.nodes || w.life.brainless(b)
}

/// The body at `s` joins the body at `t`, of its lineage (`add`), letting go of what it holds.
pub fn merge(w: &World, d: &mut Domain, s: Handle, t: Handle) {
    let (born, mut c) = (d.born(w, &d[s]).to_vec(), std::mem::take(&mut d[s]));
    let_go(w, d, &mut c, false);
    d.remove(s, &c);
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread), (&born, &c.rows, c.row_spread));
}

/// What splits and joins as an amount when members move between nodes, split by shares with the remainder kept so the
/// parts add up: count, mol of each store, heat (J), brood (mol), and their last day.
pub(crate) struct Lot {
    pub count: f64,
    pub store: Vec<f64>,
    pub e: f64,
    pub brood: f64,
    pub last: f64,
}

impl Lot {
    /// The share `f` of node `b`'s members and the share `fa` of its stores, heat and brood leave it; it keeps the rest
    /// (the remainder, so the two add up to what it held to rounding). The same share of both is members alike to the
    /// others; a larger share of the amounts is members that hold more than the mean.
    pub(crate) fn split(d: &mut Domain, b: &mut Body, f: f64, fa: f64) -> Lot {
        let store: Vec<f64> = b.store.iter().map(|a| fa * a).collect();
        b.store.iter_mut().zip(&store).for_each(|(a, x)| *a -= x);
        let c = Lot { count: f * b.count, store, e: fa * b.e, brood: fa * b.brood, last: b.last };
        (b.e, b.brood) = (b.e - c.e, b.brood - c.brood);
        d.set_count(b, b.count - c.count);
        c
    }
    /// All that body `c` holds (its stores taken out of it).
    pub(crate) fn of(c: &mut Body) -> Lot {
        Lot { count: c.count, store: std::mem::take(&mut c.store), e: c.e, brood: c.brood, last: c.last }
    }
}

/// Members `c` join the body at `t` with genes and spread `genes` and rows `rows` (born rows, rows, row spread):
/// amounts add up (`rejoin`), genes and rows become the means by members and the spreads pool around them (laws.md 4.4).
pub(crate) fn add(w: &World, d: &mut Domain, t: Handle, c: Lot, (genes, spread): (&[f32], &[f32]), (born, own, row_spread): (&[f32], &[f32], f32)) {
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
    rejoin(d, &mut o, c, Some(own));
    d.see(w, &mut o);
    d[t] = o;
}

/// Members `c` with the genes and spreads of body `o` join it (taken out of its slot): the members of a node who waited
/// aside while the others acted, a part of a node that moved or bonded and came back to its cell (both
/// `choose::spread`), or joiners after `add` pooled their genes. Amounts add up; the last
/// day becomes the mean by members; `rows`: the rows of `c` where they differ from `o`'s, which `o`'s
/// become the mean by members of. `o`'s visible traits are the caller's to work out again (`Domain::see`).
pub(crate) fn rejoin(d: &mut Domain, o: &mut Body, c: Lot, rows: Option<&[f32]>) {
    let share = c.count / (o.count + c.count);
    o.last += share * (c.last - o.last);
    let a = share as f32;
    if let Some(rows) = rows {
        blend(&mut o.rows, rows, a);
    }
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    o.brood += c.brood;
    o.e += c.e;
    d.set_count(o, o.count + c.count);
}

/// Share `f` of node `b`'s members and share `fa` of each of its stores, of its brood and of its heat as a new body
/// (not placed, `Lot::split`); lineage, genome, genes and their constants, brain rows, link and memory as the node's;
/// holding nothing, inside nothing.
pub(crate) fn part(d: &mut Domain, b: &mut Body, f: f64, fa: f64) -> Body {
    let lot = Lot::split(d, b, f, fa);
    // (The trace and the stores are not the part's: set aside, not cloned.)
    let (trace, store) = (std::mem::take(&mut b.trace), std::mem::take(&mut b.store));
    let c = Body { count: lot.count, store: lot.store, e: lot.e, brood: lot.brood, hold: None, inside: None, leaving: false, ..b.clone() };
    (b.trace, b.store) = (trace, store);
    c
}

/// Genes as wide as the widest but for this share (rounding) tie for the split.
const TIE: f32 = 1e-4;

/// At its update a node whose two halves would not be alike splits in two along its widest gene, the mean and the
/// spread of both kept, while its cell has room (fewer than `Config::crowd` - 1 bodies) and it is not leaving it nor has
/// a part act queued in this update (`Domain::queued`: its act runs on it whole); laws.md 4.4.
pub fn split(w: &World, d: &mut Domain, b: &mut Body) {
    if b.count < 2.0 || b.leaving || b.spread.is_empty() || d.cell(b.cell as usize).count() + 1 >= w.cfg.crowd || !may_be_node(w, b) || d.queued(b) {
        return;
    }
    let (lay, p) = (Layout::of(&w.chem), &w.cfg.phys);
    // Per gene the halves' shift from the mean (within the walls) and their gap in steps (one look-up of the range:
    // most updates do not split).
    let half = |g: usize| {
        let ((u, lo, hi), m) = (range(&lay, g), b.genes[g]);
        let d = (FRAC_2_PI * b.spread[g].max(0.0)).sqrt().min(wall_room(m, lo, hi));
        let (x, y) = if g < lay.cat { (m + d, m - d) } else { ((m + d).max(0.0), (m - d).max(0.0)) };
        (d, x / u - y / u)
    };
    let top = (0..b.spread.len()).fold(0.0f32, |a, g| a.max(half(g).1));
    if top <= p.alike {
        return;
    }
    let mut gap: Vec<(usize, f32)> = (0..b.spread.len()).map(|g| (g, half(g).1)).collect();
    gap.retain(|x| x.1 >= top * (1.0 - TIE));
    let mut r = Rng::new(w.seed, w.day(), b.id, Purpose::Split);
    let g = gap[r.below(gap.len())].0;
    let shift = half(g).0;
    let mut c = part(d, b, 0.5, 0.5);
    // Which half stays the body (keeps its id) is drawn.
    let shift = if r.below(2) == 0 { shift } else { -shift };
    (b.genes[g], c.genes[g]) = (b.genes[g] + shift, b.genes[g] - shift);
    b.spread[g] = (b.spread[g] - shift * shift).max(0.0);
    c.spread[g] = b.spread[g];
    b.prepare(&w.chem, &w.cfg);
    d.counts.splits += 1.0;
    d.place(w, c);
}

/// A count this close to a whole number is that number.
pub(crate) const COUNT_SLACK: f64 = 1e-9;

/// `n` members of the node at `h` as individuals, each drawn with genes and born rows of its own (`draw`),
/// drawn as its genes are from their spread (`Body::row_spread`), placed in the node's cell: the close view of a node,
/// and what the check of nodes against individuals compares. Each draw leaves the others' genes and rows, so the mean
/// of all is kept. The node keeps at least one member (draws stop when it has fewer than two), seen as what is left of
/// it. Returns where the members are stored.
pub fn open(w: &World, d: &mut Domain, h: Handle, n: usize) -> Vec<Handle> {
    let mut b = std::mem::take(&mut d[h]);
    let mut r = Rng::new(w.seed, w.day(), b.id, Purpose::Open);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let whole = b.count.round();
        if (b.count - whole).abs() < COUNT_SLACK {
            d.set_count(&mut b, whole);
        }
        if b.count < 2.0 {
            break;
        }
        out.push(draw(w, d, &mut b, &mut r));
    }
    d.see(w, &mut b);
    d[h] = b;
    out
}

/// One member drawn from node `b`, placed with the node's light-keep share: its genes the node's plus noise of their
/// spread (shrunk near a wall, so it is never clamped, as in `body::vary`), a share 1/count of its stores and heat
/// (`part`), and born rows of its own (`member_rows`), keeping what the node learned on top; the node's genes, rows and
/// spreads become those of the others.
fn draw(w: &World, d: &mut Domain, b: &mut Body, r: &mut Rng) -> Handle {
    let (lay, n) = (Layout::of(&w.chem), b.count as f32);
    // (The member keeps no spread: it is left out of the clone.)
    let spread = std::mem::take(&mut b.spread);
    let mut c = part(d, b, 1.0 / b.count, 1.0 / b.count);
    b.spread = spread;
    for (g, x) in c.genes.iter_mut().enumerate() {
        let ((_, lo, hi), v) = (range(&lay, g), spread_at(&b.spread, g));
        *x += v.sqrt().min(wall_room(*x, lo, hi) / REACH) * noise(r);
        let d = *x - b.genes[g];
        b.genes[g] = (b.genes[g] - d / (n - 1.0)).clamp(lo, hi);
        if let Some(s) = b.spread.get_mut(g) {
            *s = ((n * v - d * d * n / (n - 1.0)) / (n - 1.0)).max(0.0);
        }
    }
    if b.row_spread > 0.0 {
        let (mine, left, spread) = member_rows(d.born(w, b), b.row_spread, n, r);
        let shift = |rows: &mut [f32], to: &[f32], from: &[f32]| rows.iter_mut().zip(to).zip(from).for_each(|((x, t), f)| *x += t - f);
        let born = d.born(w, b);
        shift(&mut c.rows, &mine, born);
        shift(&mut b.rows, &left, born);
        (c.genome, b.row_spread) = (d.new_genome(mine), spread);
        d.reborn(w, b, left);
    }
    b.prepare(&w.chem, &w.cfg);
    (c.count, c.spread, c.row_spread) = (1.0, Vec::new(), 0.0);
    d.place(w, c)
}

/// The born rows of one member drawn from `n` members whose rows have mean `rows` and variance `v` (`brain::shake`),
/// the mean of the others' rows, and the variance left among them, as `draw` draws genes.
fn member_rows(rows: &[f32], v: f32, n: f32, r: &mut Rng) -> (Vec<f32>, Vec<f32>, f32) {
    let mut mine = rows.to_vec();
    shake(&mut mine, r, v.sqrt());
    let d2 = mine.iter().zip(rows).map(|(x, m)| (x - m) * (x - m)).sum::<f32>() / rows.len().max(1) as f32;
    let left = rows.iter().zip(&mine).map(|(m, x)| m - (x - m) / (n - 1.0)).collect();
    (mine, left, ((n * v - d2 * n / (n - 1.0)) / (n - 1.0)).max(0.0))
}

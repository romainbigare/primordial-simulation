//! Population nodes: the monthly merge of alike nodes in each cell (`crowd`), a body joining another (`merge`,
//! `add`), a part of a node coming back to it (`rejoin`), a node splitting in two (`split`), and members drawn from a
//! node as individuals (`open`). Members, stores and heat are kept exactly in every join and split.

use crate::bond::{held, let_go, room};
use crate::body::{self, alike, most, noise, range, spread_at, spread_kept, spreads, steps, wall_room, within_steps, Body, Layout, REACH};
use crate::brain::{blend, row_step, shake};
use crate::rng::{Purpose, Rng};
use crate::life::{Handle, Life};
use crate::World;
use std::f32::consts::FRAC_2_PI;

/// The monthly merge of nodes, a speed setting, not a law: in each cell a node joins the first larger one alike to it
/// (`join_alike`), then past `Config::crowd` the most alike pairs merge (`merge_closest`); laws.md 4.4.
pub(crate) fn crowd(w: &mut World) {
    let (max, lay) = (w.cfg.crowd.max(1), Layout::of(&w.chem));
    for i in 0..w.mesh.cells().n() {
        if w.life.in_cell(i) <= 1 {
            continue;
        }
        let left = join_alike(w, &lay, i);
        if left.len() > max {
            merge_closest(w, &lay, left, max);
        }
    }
}

/// What a node shares with those it joins (`join_alike`): its lineage (a node has one brain), its stage (both mature or
/// both young: the young join their adult node member by member as they mature, `copy::young`), the structure whose
/// room it is in (its root, as the room law counts those inside, `bond::room`) and the structure it holds (its root).
type Kin = (u32, bool, Option<usize>, Option<usize>);

/// The `Kin` of body `b`.
fn kin_of(w: &World, b: &Body) -> Kin {
    (b.lineage, b.adult(&w.chem), room(w, b).map(|r| r.0), held(w, b).map(|p| w.parts.parts[p].root as usize))
}

/// Each body of cell `i`, largest first (equals in the cell's order), however few its members, joins the first larger node (more than one member,
/// as in `copy::kin`) alike in genes (`body::alike`, on the means only, so a split's halves stay apart for selection)
/// of its `Kin`. Returns the bodies left.
fn join_alike(w: &mut World, lay: &Layout, i: usize) -> Vec<Handle> {
    let b = &w.life;
    let mut list: Vec<(u64, Handle)> = b.cell(i).filter(|&s| may_be_node(w, &b[s])).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
    list.sort_by_key(|x| x.0);
    let mut head: Vec<usize> = (0..list.len()).collect();
    // Each one's kin once, tested before the genes.
    let kind: Vec<Kin> = list.iter().map(|&(_, s)| kin_of(w, &b[s])).collect();
    for x in 0..list.len() {
        let a = &b[list[x].1];
        let joins = |y: usize| head[y] == y && b[list[y].1].n() > 1.0 && kind[y] == kind[x];
        head[x] = (0..x).find(|&y| joins(y) && alike(lay, w.cfg.phys.alike, &b[list[y].1].genes, &a.genes)).unwrap_or(x);
    }
    for x in (0..list.len()).filter(|&x| head[x] != x) {
        merge(w, list[x].1, list[head[x]].1);
    }
    (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect()
}

/// A body's genes in steps (`body::steps`), its spread (`body::spreads`), and its lineage and stage
/// (`merge_closest`).
type Likeness = (Vec<f32>, Vec<f32>, (u32, bool));

/// The most alike pair of `left` merges first, the smaller into the larger (ties: the earlier, larger ones), until `max`
/// are left or no pair is alike: here each within the other's spread, as children join a node (`body::within`), of one
/// lineage and stage; unlike nodes never merge, however full the cell. What a body holds and the room it is in do not
/// keep it apart in a full cell: they are states of its members, not a likeness; the one that joins lets go of its
/// part and takes the other's room (`merge`). A pair's key: the squared distance of their genes in steps (infinite if
/// unlike).
fn merge_closest(w: &mut World, lay: &Layout, left: Vec<Handle>, max: usize) {
    let (m, lim) = (left.len(), w.cfg.phys.alike);
    let steps = |w: &World, s: Handle| {
        let b = &w.life[s];
        (steps(lay, &b.genes), spreads(b), (b.lineage, b.adult(&w.chem)))
    };
    let mut v: Vec<Likeness> = left.iter().map(|&s| steps(w, s)).collect();
    let dist = |p: &Likeness, q: &Likeness| {
        let gap = |g: usize| (p.0[g] - q.0[g]).abs() > within_steps(lay, lim, p.1[g] + q.1[g], g);
        if p.2 != q.2 || (0..p.0.len()).any(gap) { f32::INFINITY } else { p.0.iter().zip(&q.0).map(|(x, y)| (x - y) * (x - y)).sum() }
    };
    let (mut d, mut gone) = (vec![f32::INFINITY; m * m], vec![false; m]);
    for p in 0..m {
        for q in p + 1..m {
            d[p * m + q] = dist(&v[p], &v[q]);
        }
    }
    for _ in max..m {
        let k = (0..m * m).fold(0, |k, x| if d[x] < d[k] { x } else { k });
        if d[k] == f32::INFINITY {
            break;
        }
        let (p, q) = (k / m, k % m);
        let (s, t) = if w.life[left[q]].n() > w.life[left[p]].n() { (p, q) } else { (q, p) };
        merge(w, left[s], left[t]);
        (gone[s], v[t]) = (true, steps(w, left[t]));
        for o in 0..m {
            let (lo, hi) = (o.min(s), o.max(s));
            d[lo * m + hi] = f32::INFINITY;
            if o != t && !gone[o] {
                let (lo, hi) = (o.min(t), o.max(t));
                d[lo * m + hi] = dist(&v[o], &v[t]);
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
pub fn merge(w: &mut World, s: Handle, t: Handle) {
    let (born, mut c) = (w.life.born(&w.life[s]).to_vec(), std::mem::take(&mut w.life[s]));
    let_go(w, &mut c, false);
    w.life.remove(s, &c);
    let size = c.size_moments(&w.chem);
    add(w, t, Lot::of(&mut c), (&c.genes, &c.spread), size, (&born, &c.rows, c.row_spread));
}

/// What splits and joins as an amount when members move between nodes, split by shares with the remainder kept so the
/// parts add up: count, mol of each store, heat (J), brood (mol), the heat its bodies' node holds for them (J,
/// `survey::Draw::owed`), and their last day and marks of light and rain.
pub(crate) struct Lot {
    pub count: f64,
    pub store: Vec<f64>,
    pub e: f64,
    pub brood: f64,
    pub owed: f64,
    pub last: f64,
    pub mark: f64,
    pub rained: f64,
}

impl Lot {
    /// The share `f` of node `b`'s members and the share `fa` of its stores, heat and brood leave it; it keeps the rest
    /// (the remainder, so the two add up to what it held to rounding). The same share of both is members alike to the
    /// others; a larger share of the amounts is members that hold more than the mean.
    pub(crate) fn split(life: &mut Life, b: &mut Body, f: f64, fa: f64) -> Lot {
        let store: Vec<f64> = b.store.iter().map(|a| fa * a).collect();
        b.store.iter_mut().zip(&store).for_each(|(a, x)| *a -= x);
        let c = Lot { count: f * b.count, store, e: fa * b.e, brood: fa * b.brood, owed: fa * b.draw.owed, last: b.last, mark: b.mark, rained: b.rained };
        (b.e, b.brood, b.draw.owed) = (b.e - c.e, b.brood - c.brood, b.draw.owed - c.owed);
        life.set_count(b, b.count - c.count);
        c
    }
    /// All that body `c` holds (its stores taken out of it).
    pub(crate) fn of(c: &mut Body) -> Lot {
        Lot { count: c.count, store: std::mem::take(&mut c.store), e: c.e, brood: c.brood, owed: c.draw.owed, last: c.last, mark: c.mark, rained: c.rained }
    }
}

/// Members `c` join the body at `t` with genes and spread `genes`, size moments `size` and rows `rows` (born rows,
/// rows, row spread): amounts add up (`rejoin`), genes, sizes and rows become the means by members and the spreads pool
/// around them (laws.md 4.4).
pub(crate) fn add(w: &mut World, t: Handle, c: Lot, (genes, spread): (&[f32], &[f32]), size: (f32, f32), (born, own, row_spread): (&[f32], &[f32], f32)) {
    let mut o = std::mem::take(&mut w.life[t]);
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.count / (o.count + c.count)) as f32);
    let mix = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p + a * (q - p)).collect::<Vec<f32>>();
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    let mean = mix(&o.genes, genes);
    let v = |g| pool(spread_at(&o.spread, g), spread_at(spread, g), (genes[g] - o.genes[g]).powi(2), range(&lay, g).0);
    o.spread = (0..o.genes.len()).map(|g| spread_kept(&lay, g, mean[g], v(g))).collect();
    let was = w.life.born(&o);
    let gap = was.iter().zip(born).map(|(p, q)| (q - p) * (q - p)).sum::<f32>() / was.len().max(1) as f32;
    o.row_spread = pool(o.row_spread, row_spread, gap, row_step(p, body::mutation_size(&o.genes)));
    w.life.reborn(&mut o, mix(was, born));
    o.set_genes(mean, &w.chem, &w.cfg.phys);
    rejoin(w, &mut o, c, size, Some(own));
    w.see(&mut o);
    w.life[t] = o;
}

/// Members `c` with the genes and spreads of body `o` join it (taken out of its slot): the members of a node who waited
/// aside while the others acted, a part of a node that moved or bonded and came back to its cell (both
/// `choose::spread`), or joiners after `add` pooled their genes. Amounts add up; the last
/// day, the marks and the sizes (moments `size`, `Body::size_moments`) become the means by members, the sizes' variance
/// with the gap between the two means (members who fed are larger than those who did not); `rows`: the rows of `c`
/// where they differ from `o`'s, which `o`'s become the mean by members of. `o`'s visible traits are the caller's to
/// work out again (`World::see`).
pub(crate) fn rejoin(w: &mut World, o: &mut Body, c: Lot, size: (f32, f32), rows: Option<&[f32]>) {
    let share = c.count / (o.count + c.count);
    (o.last, o.mark, o.rained) = (o.last + share * (c.last - o.last), o.mark + share * (c.mark - o.mark), o.rained + share * (c.rained - o.rained));
    let ((mo, vo), a) = (o.size_moments(&w.chem), share as f32);
    let gap = size.0 - mo;
    let (size_mean, size_var) = (mo + a * gap, (1.0 - a) * vo + a * size.1 + a * (1.0 - a) * gap * gap);
    if let Some(rows) = rows {
        blend(&mut o.rows, rows, a);
    }
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    o.brood += c.brood;
    (o.e, o.draw.owed) = (o.e + c.e, o.draw.owed + c.owed);
    w.life.set_count(o, o.count + c.count);
    o.set_size(&w.chem, size_mean as f64, size_var as f64);
}

/// Share `f` of node `b`'s members and share `fa` of each of its stores, of its brood and of its heat as a new body
/// (not placed, `Lot::split`); lineage, genome, genes and their constants, brain rows, link and memory as the node's;
/// holding nothing, inside nothing.
pub(crate) fn part(life: &mut Life, b: &mut Body, f: f64, fa: f64) -> Body {
    let lot = Lot::split(life, b, f, fa);
    // (The trace and the stores are not the part's: set aside, not cloned.)
    let (trace, store) = (std::mem::take(&mut b.trace), std::mem::take(&mut b.store));
    let draw = crate::survey::Draw { owed: lot.owed, ..Default::default() };
    let c = Body { count: lot.count, store: lot.store, e: lot.e, brood: lot.brood, draw, hold: None, inside: None, ..b.clone() };
    (b.trace, b.store) = (trace, store);
    c
}

/// Genes as wide as the widest but for this share (rounding) tie for the split.
const TIE: f32 = 1e-4;

/// At its update a node whose two halves would not be alike splits in two along its widest gene, the mean and the
/// spread of both kept, while its cell has room (fewer than `Config::crowd` - 1 bodies); laws.md 4.4.
pub fn split(w: &mut World, b: &mut Body) {
    if b.count < 2.0 || b.spread.is_empty() || w.life.in_cell(b.cell as usize) + 1 >= w.cfg.crowd || !may_be_node(w, b) {
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
    let d = half(g).0;
    let mut c = part(&mut w.life, b, 0.5, 0.5);
    let mut genes = b.genes.clone();
    // Which half stays the body (keeps its id) is drawn.
    let d = if r.below(2) == 0 { d } else { -d };
    (genes[g], c.genes[g]) = (genes[g] + d, genes[g] - d);
    b.spread[g] = (b.spread[g] - d * d).max(0.0);
    c.spread[g] = b.spread[g];
    b.set_genes(genes, &w.chem, &w.cfg.phys);
    w.counts.splits += 1.0;
    w.place(c);
}

/// A count this close to a whole number is that number.
pub(crate) const COUNT_SLACK: f64 = 1e-9;

/// `n` members of the node at `h` as individuals, each drawn with genes and born rows of its own (`draw`),
/// drawn as its genes are from their spread (`Body::row_spread`), placed in the node's cell: the close view of a node,
/// and what the check of nodes against individuals compares. Each draw leaves the others' genes and rows, so the mean
/// of all is kept. The node keeps at least one member (draws stop when it has fewer than two), seen as what is left of
/// it. Returns where the members are stored.
pub fn open(w: &mut World, h: Handle, n: usize) -> Vec<Handle> {
    let mut b = std::mem::take(&mut w.life[h]);
    let mut r = Rng::new(w.seed, w.day(), b.id, Purpose::Open);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let whole = b.count.round();
        if (b.count - whole).abs() < COUNT_SLACK {
            w.life.set_count(&mut b, whole);
        }
        if b.count < 2.0 {
            break;
        }
        out.push(draw(w, &mut b, &mut r));
    }
    w.see(&mut b);
    w.life[h] = b;
    out
}

/// One member drawn from node `b`, placed with the node's light-keep share: its genes the node's plus noise of their
/// spread (shrunk near a wall, so it is never clamped, as in `body::vary`), a share 1/count of its stores and heat
/// (`part`), and born rows of its own (`member_rows`), keeping what the node learned on top; the node's genes, rows and
/// spreads become those of the others.
fn draw(w: &mut World, b: &mut Body, r: &mut Rng) -> Handle {
    let (lay, n) = (Layout::of(&w.chem), b.count as f32);
    // (The member keeps no spread: it is left out of the clone.)
    let spread = std::mem::take(&mut b.spread);
    let (mut c, mut genes) = (part(&mut w.life, b, 1.0 / b.count, 1.0 / b.count), b.genes.clone());
    b.spread = spread;
    for (g, x) in c.genes.iter_mut().enumerate() {
        let ((_, lo, hi), v) = (range(&lay, g), spread_at(&b.spread, g));
        *x += v.sqrt().min(wall_room(*x, lo, hi) / REACH) * noise(r);
        let d = *x - genes[g];
        genes[g] = (genes[g] - d / (n - 1.0)).clamp(lo, hi);
        if let Some(s) = b.spread.get_mut(g) {
            *s = ((n * v - d * d * n / (n - 1.0)) / (n - 1.0)).max(0.0);
        }
    }
    if b.row_spread > 0.0 {
        let (mine, left, spread) = member_rows(w.life.born(b), b.row_spread, n, r);
        let shift = |rows: &mut [f32], to: &[f32], from: &[f32]| rows.iter_mut().zip(to).zip(from).for_each(|((x, t), f)| *x += t - f);
        let born = &w.life.genomes[b.genome as usize].rows;
        shift(&mut c.rows, &mine, born);
        shift(&mut b.rows, &left, born);
        (c.genome, b.row_spread) = (w.life.new_genome(mine), spread);
        w.life.reborn(b, left);
    }
    b.set_genes(genes, &w.chem, &w.cfg.phys);
    (c.count, c.spread, c.row_spread, c.size_spread, c.size_lag) = (1.0, Vec::new(), 0.0, 0.0, 0.0);
    w.place(c)
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

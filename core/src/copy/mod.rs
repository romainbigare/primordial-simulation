//! Law 4: Copy. Reproduction is a share of a surplus. A mature body puts the share `ALLOC` of what it builds (`grow`)
//! into a brood, and whenever the brood covers the matter of a child, a child is made (`copy`) at the body's update,
//! whatever its brain does. A child's genes are its parent's plus small noise, or with two parents each gene is one
//! parent's (`mix`). A population node keeps how its members differ (`Body::spread`, `Body::row_spread`,
//! `Body::size_spread`); its children are booked on the young node of their kind where they start, or start one, and
//! grow up into its adult node (`young`). Only individuals' children start new lineages. With `Config::nodes` off
//! bodies with a brain are never nodes: their children start as individuals and join nothing.

mod grow;
mod mix;
mod select;
mod young;

pub use grow::grow;
pub use grow::buildable;
pub(crate) use select::{sample, select};
pub use young::{mature, Grown};

use crate::body::{most, mutation_size, vary, within, Body, Layout, CHILD, COPY_MASS, DISPERSE, WET, WET_TISSUE};
use crate::brain::{self, row_step, shake};
use crate::chem::Chemistry;
use crate::event::{Event, EventKind};
use crate::lineage::resized;
use crate::nodes::{add, may_be_node, open, Lot, COUNT_SLACK};
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::World;
use mix::{mix, mix_rows, partner};

/// The brood a node holds besides what makes children, in children per member. Its members each fill their own brood and
/// make a child when it covers one, so their broods lie evenly between none and one child (the renewal of each one's
/// brood): half a child each on average. A member who dies loses the brood it held.
pub const BROOD_HELD: f64 = 0.5;

/// What the children of one `copy` share: each one's body, kg, its matter, mol (its body is that over 1 -
/// `WET_TISSUE`), and its provisions, mol of each store (none of the matter: `provisions`); whether the parent is a
/// node; whether they join no node (`nodes::may_be_node`); the partner; and the parent's id.
struct Litter {
    body: f64,
    matter: f64,
    prov: Vec<f64>,
    node: bool,
    alone: bool,
    mate: Option<usize>,
    parent: u64,
}

/// A mature body makes children whenever its brood covers one: an individual whole children of one parent or two, a
/// node fractional members of one parent beyond `BROOD_HELD`; a share `DISPERSE` starts in a neighbour cell (laws.md
/// 4.2-4.4). Returns whether any were made.
pub fn copy(w: &mut World, slot: usize, b: &mut Body) -> bool {
    let (chem, n, mc) = (&w.chem, b.n(), b.dec(COPY_MASS));
    let Some(mi) = chem.matter() else { return false };
    if !b.adult(chem) {
        return false;
    }
    let mm = chem.subs[mi].molar_mass;
    b.brood = b.brooded(chem);
    // A child's body (kg) and its matter (mol); the births the brood covers: an individual's whole children, a node's what
    // its brood holds beyond `BROOD_HELD`.
    let body = 10f64.powf(b.gene_in_range(CHILD)) * mc;
    let matter = body * (1.0 - WET_TISSUE) / mm;
    let k = b.brood / matter;
    let node = n > 1.0;
    let births = if node { k - BROOD_HELD * n } else { (k + COUNT_SLACK).floor() };
    if births <= 0.0 {
        return false;
    }
    let id = b.id(slot);
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Mutate);
    let i = b.cell as usize;
    let mate = if node { None } else { partner(w, slot, b, &mut r) };
    // Children that start in the neighbour cell of a compass direction drawn (no link that way: none): of a node, the
    // share `DISPERSE` of its births, however few (less than one member with no node to join there waits in the brood,
    // as the stayers do, `emit`); an individual's go together, by that chance.
    let d = b.gene_in_range(DISPERSE);
    let prov = provisions(&w.chem, b, mi, matter, births);
    let litter = Litter { body, matter, prov, node, alone: !may_be_node(w, b), mate, parent: id };
    let m = if node { d * births } else { (r.f64() < d) as u8 as f64 };
    let cells = w.mesh.cells();
    let to = if m > 0.0 { cells.way(i, r.below(COMPASS)).map(|e| cells.adj_cell()[e] as usize) } else { None };
    let (cell, m) = match to {
        _ if !node => (to.unwrap_or(i), births),
        Some(j) => (j, m),
        None => (i, 0.0),
    };
    let mut made = false;
    if m > 0.0 {
        made |= emit(w, slot, b, &litter, cell, m, &mut r);
    }
    // A node's children who stay: a group of their own (less than one member waits in the brood unless there is a node
    // of theirs to join).
    if node && births - m > 0.0 {
        made |= emit(w, slot, b, &litter, i, births - m, &mut r);
    }
    made
}

/// `m` children of `b` start in `cell`: their genes, stores, heat and rows, booked on the young node of their kind and
/// lineage there (`kin`), or a body of their own (`place_child`). Less than one member of a node's children with no
/// node to join waits in the brood (false).
fn emit(w: &mut World, slot: usize, b: &mut Body, l: &Litter, cell: usize, m: f64, r: &mut Rng) -> bool {
    let mixed = l.mate.map(|t| mix(b, &w.life.bodies[t], m, r));
    let (from, pooled) = mixed.as_ref().map_or((&b.genes, &b.spread), |x| (&x.0, &x.1));
    let (genes, spread) = vary(&Layout::of(&w.chem), from, pooled, m, w.cfg.phys.alike, r);
    let size = mutation_size(from);
    // The young node of their kind they join there: for a node's children who stay, its young node (`young::linked`);
    // else the first young node there of their genes and spread (`kin`). Nothing between here and the join changes the
    // nodes of the cell or the children's genes, so the search made now holds for the join.
    let hit = match () {
        _ if l.alone => None,
        _ if l.node && cell == b.cell as usize => young::linked(w, slot, b, false),
        _ => kin(w, cell, b.lineage, (&genes, &spread), false),
    };
    if l.node && m < 1.0 && hit.is_none() {
        return false;
    }
    let (store, e) = give(w, b, l, cell, m);
    let (mut rows, row_spread) = child_rows(w, b, l, m, size, r);
    // A node's children start no new lineage: a change of the brain's base in one child is lost in the mean of many; a
    // node whose mean sizes moved moves itself (`lineage::resize`).
    let fresh = if l.node { None } else { child_lineage(w, b, &genes, &mut rows, r) };
    let lineage = fresh.unwrap_or(b.lineage);
    // Their brain: their born rows plus what their parent learned.
    let own = w.life.inherit(rows.as_deref().unwrap_or(w.life.born(b)), lineage, b);
    let lot = Lot { count: m, store, e, brood: 0.0, owed: 0.0, last: w.day() as f64, mark: w.light[cell], rained: w.rained[cell] };
    if fresh.is_none() {
        if let Some(t) = hit {
            let born = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            let mut o = std::mem::take(&mut w.life.bodies[t]);
            add(w, &mut o, lot, &genes, &spread, (l.body.log10() as f32, 0.0), Some((&born, &own, row_spread)));
            w.life.bodies[t] = o;
            return true;
        }
    }
    let c = Body {
        cell: cell as u32,
        count: m,
        lineage,
        parent: l.parent,
        born: w.day(),
        mark: w.light[cell],
        rained: w.rained[cell],
        keep: b.keep,
        store: lot.store,
        e,
        genes,
        spread,
        row_spread,
        genome: b.genome,
        rows: own,
        wet: b.wet,
        ..Default::default()
    };
    let s = place_child(w, b, l, c, rows);
    if l.node && cell == b.cell as usize {
        young::link(w, slot, b, s);
    }
    true
}

/// The stores and heat of `m` children of `b` starting in `cell`, taken from it, and their birth recorded: the matter
/// of all `m` from the brood and their provisions (`Litter::prov`), each at most what it holds; their heat that of
/// its temperature on the heat capacity they got, with the fusion enthalpy of what they got molten (`Body::heat_with`),
/// so they start as warm as their parent (a dry child would else be much hotter, a wet one colder).
fn give(w: &mut World, b: &mut Body, l: &Litter, cell: usize, m: f64) -> (Vec<f64>, f64) {
    w.events.push(Event { day: w.day(), kind: EventKind::Born { root: w.life.root(b), mixed: l.mate.is_some() }, node: cell as u32, value: m });
    let mi = w.chem.matter().unwrap_or(0);
    let take = (m * l.matter).min(b.store[mi]);
    let store: Vec<f64> = b.store.iter().zip(&l.prov).enumerate().map(|(k, (a, p))| if k == mi { take } else { (p * m).min(*a) }).collect();
    let e = b.heat_with(&w.chem, &b.thermal(&w.chem), &store);
    b.e -= e;
    for (a, x) in b.store.iter_mut().zip(&store) {
        *a -= x;
    }
    b.brood = (b.brood - take).max(0.0);
    (store, e)
}

/// What one of the `births` children of `b` takes of each store but the matter (`mi`), mol: of the parent's stores
/// when the litter starts, the share the litter's `matter` is of the parent's body matter (of the key liquid 10^`WET`
/// times that: a child of a parent whose `WET` is below 0 is dry, and nearly dormant), at most all, split over the
/// births, so children who leave and children who stay get alike.
fn provisions(chem: &Chemistry, b: &Body, mi: usize, matter: f64, births: f64) -> Vec<f64> {
    let share = (births * matter).min(b.store[mi]) / (b.store[mi] - b.brooded(chem)).max(1e-300);
    let wet = share * 10f64.powf(b.gene(WET));
    b.store.iter().enumerate().map(|(k, a)| if k == mi { 0.0 } else { a * (if b.wet >> k & 1 == 1 { wet } else { share }).min(1.0) / births }).collect()
}

/// The first node of `cell` of `lineage` (a node has one brain), mature or young as `mature` asks, and of the kind of
/// genes `genes` and spread `spread` (`body::within`, tested last: it reads every gene): the young node children join
/// (they are below their parents' copy mass, and a mature node must keep its mean there), the adult node young members
/// mature into, or the node members who moved join.
pub(crate) fn kin(w: &World, cell: usize, lineage: u32, (genes, spread): (&[f32], &[f32]), mature: bool) -> Option<usize> {
    let (lay, alike) = (Layout::of(&w.chem), w.cfg.phys.alike);
    w.life.at[cell].iter().map(|&s| s as usize).find(|&s| {
        let t = &w.life.bodies[s];
        t.alive && t.n() > 1.0 && t.lineage == lineage && t.adult(&w.chem) == mature && within(&lay, alike, t, genes, spread)
    })
}

/// The born rows of `m` children of `b` (none: they share their parent's), and the spread left among them. A share
/// `Physics::row_chance` of children change theirs by a step (`brain::row_step`), else share their parent's. Children
/// drawn from a node whose rows differ, or a group of them, get the mean of the group drawn (noise of that variance and
/// the generation's change, over `m`) and keep the variance left among them (up to `body::most`, as a node keeps). With
/// a partner each row is one parent's (`mix::mix_rows`), the group's spread pooled, then the same change. In a null run
/// the rows do not change.
fn child_rows(w: &World, b: &Body, l: &Litter, m: f64, size: f32, r: &mut Rng) -> (Option<Vec<f32>>, f32) {
    let p = &w.cfg.phys;
    let mm = m.max(1.0) as f32;
    let row_noise = if w.cfg.null { 0.0 } else { p.row_noise };
    let (mut pre, mut spread) = (None, b.row_spread);
    if let Some(t) = l.mate.filter(|_| !w.cfg.null) {
        if let Some((v, s)) = mix_rows(w, b, &w.life.bodies[t], m, r) {
            // (Rows the same as the parent's are shared, not copied.)
            (pre, spread) = ((v != w.life.born(b)).then_some(v), s);
        }
    }
    let (var, changed) = (spread + (row_noise * size).powi(2), r.f64() < p.row_chance);
    let sd = if w.cfg.null {
        0.0
    } else if spread > 0.0 || mm > 1.0 {
        (var / mm).sqrt()
    } else if changed {
        row_step(p, size)
    } else {
        0.0
    };
    let left = (var * (1.0 - 1.0 / mm)).min(most(row_step(p, size), p.alike));
    match (pre, sd > 0.0) {
        (v, false) => (v, left),
        (v, true) => {
            let mut v = v.unwrap_or_else(|| w.life.born(b).to_vec());
            shake(&mut v, r, sd);
            (Some(v), left)
        }
    }
}

/// An individual's child whose brain-size genes are a whole unit away from its lineage's sizes starts a new lineage
/// with the new sizes, its brain carried over where they overlap (`lineage::resized`, its rows by `brain::rerow`);
/// rarely (`Physics::new_lineage`) the base of its lineage's brain is copied with small noise, which starts a new
/// lineage too (in a null run neither adds noise, `Config::new_noise`). Returns the new lineage (none: the child is of
/// its parent's).
fn child_lineage(w: &mut World, b: &Body, genes: &[f32], rows: &mut Option<Vec<f32>>, r: &mut Rng) -> Option<u32> {
    let (noise, base_noise) = (w.cfg.new_noise(), if w.cfg.null { 0.0 } else { w.cfg.phys.base_noise });
    let lin = w.life.brains[b.lineage as usize].as_ref()?;
    let fresh = if let Some(l) = resized(lin, genes, w.chem.ns(), r, noise) {
        *rows = Some(brain::rerow(rows.as_deref().unwrap_or(w.life.born(b)), lin.shape, l.shape, r, noise));
        Some(l)
    } else if r.f64() < w.cfg.phys.new_lineage {
        let mut l = lin.clone();
        shake(&mut l.base, r, base_noise);
        Some(l)
    } else {
        None
    };
    Some(w.life.new_lineage(fresh?))
}

/// Children `c` of `b` that join no node start a body of their own, born with `rows` (none: its parent's). Several
/// children together that join no node at all (`alone`) are drawn from it as individuals (`nodes::open`: each with
/// genes and born rows of its own, drawn from their spread), the last one the body itself, with the
/// genes and rows the draws left. Returns the body's slot.
fn place_child(w: &mut World, b: &Body, l: &Litter, mut c: Body, rows: Option<Vec<f32>>) -> usize {
    if let Some(v) = rows {
        c.genome = w.life.new_genome(v);
    }
    // Its key liquids first: `place` works out its visible traits, which read the body (`Body::size`) through them. (A
    // parent that survived a freezing is still wet in the sense of its key liquids: its child's are kept though it is
    // born frozen.)
    c.wet = c.key_liquids(&w.chem) | (b.wet & c.key_mask(&w.chem));
    let s = w.place(c);
    if l.alone && w.life.bodies[s].count > 1.5 {
        let k = w.life.bodies[s].count.round() as usize - 1;
        open(w, s, k);
        let mut c = std::mem::take(&mut w.life.bodies[s]);
        (c.spread, c.row_spread) = (Vec::new(), 0.0);
        w.life.set_count(&mut c, 1.0);
        // (The members drawn took their stores: what is seen of it is what is left.)
        w.see(&mut c);
        w.life.bodies[s] = c;
    }
    s
}

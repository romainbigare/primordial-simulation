//! Law 4: Copy. Reproduction is a share of a surplus. A mature body puts the share `ALLOC` of what it builds (`grow`)
//! into a brood, and whenever the brood covers the matter of a child, a child is made (`copy`) at the body's update,
//! whatever its brain does. A child's genes are its parent's plus small noise, or with two parents each gene is one
//! parent's (`mix`). A population node keeps how its members differ (`Body::spread`, `Body::row_spread`); its children
//! join an immature node of their kind and size class where they start, or start one (`nodes`). Only individuals'
//! children start new lineages. With `Config::nodes` off the animals (bodies that are not light-eaters) are never
//! nodes: their children start as individuals and join nothing.

mod grow;
mod mix;

pub use grow::grow;

use crate::body::{alike_size, most, mutation_size, vary, within, Body, Layout, CHILD, COPY_MASS, DISPERSE, WET, WET_TISSUE};
use crate::brain::{self, row_step, shake};
use crate::chem::Chemistry;
use crate::event::{Event, EventKind};
use crate::space::square::DIRS;
use crate::lineage::{own, resized};
use crate::nodes::{join, may_be_node, open, COUNT_SLACK};
use crate::rng::{Purpose, Rng};
use crate::World;
use mix::{mix, mix_rows, partner};

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

/// A mature body makes offspring from its brood. A child has a body of 10^`CHILD` of its parent's copy mass: matter of
/// 1 - `WET_TISSUE` of that, taken from the brood, and provisions of every other store (`provisions`), with its share of
/// the heat. A child is made whenever the brood covers one: an individual has whole children, a node (members who all
/// do the same) fractional members, as many as the brood makes. Their genes and born rows are one parent's or a
/// partner's (`mix::partner`; the same partner for all the children of the call). A body that is not mature
/// (`Body::adult`) makes none. A share of a node's children (the `DISPERSE` gene) starts in a random neighbour cell;
/// an individual's children stay together, in a neighbour cell with that chance.
pub fn copy(w: &mut World, slot: usize, b: &mut Body) -> bool {
    let (chem, n, mc) = (&w.chem, b.n(), b.dec(COPY_MASS));
    let Some(mi) = chem.matter() else { return false };
    if !b.adult(chem) {
        return false;
    }
    let mm = chem.subs[mi].molar_mass;
    b.brood = b.brooded(chem);
    // A child's body (kg) and its matter (mol); the births the brood covers.
    let body = 10f64.powf(b.gene_in_range(CHILD)) * mc;
    let matter = body * (1.0 - WET_TISSUE) / mm;
    let k = b.brood / matter;
    let node = n > 1.0;
    let births = if node { k } else { (k + COUNT_SLACK).floor() };
    if births <= 0.0 {
        return false;
    }
    let id = b.id(slot);
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Mutate);
    let i = b.cell as usize;
    let mate = partner(w, slot, b, &mut r);
    // Children that start in a neighbour cell (off the map: none): of a node, a share (none if less than one member); an
    // individual's go together, by that chance.
    let d = b.gene_in_range(DISPERSE);
    let prov = provisions(&w.chem, b, mi, matter, births);
    let litter = Litter { body, matter, prov, node, alone: !may_be_node(w, &b.genes), mate, parent: id };
    let m = if node { if d * births >= 1.0 { d * births } else { 0.0 } } else { (r.f64() < d) as u8 as f64 };
    let to = if m > 0.0 { w.map.step(i, DIRS[r.below(8)]) } else { None };
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

/// `m` children of `b` start in `cell` (`provision`, `child_rows`, `child_lineage`): they join the first node there
/// they belong to (in genes, in size, and immature as they are, `target`), their rows carried over to its brain's sizes,
/// or start a body (`place_child`). Children that started a new lineage stay apart. Less than one member of a node's
/// children with no node to join waits in the brood (nothing is made; false).
fn emit(w: &mut World, slot: usize, b: &mut Body, l: &Litter, cell: usize, m: f64, r: &mut Rng) -> bool {
    let (mut c, size, take) = provision(w, b, l, cell, m, r);
    // Nothing between here and the join changes the nodes of the cell or the child's genes, so a search made now holds
    // for the join.
    let mut hit = None;
    if l.node && m < 1.0 {
        hit = if l.alone { None } else { target(w, l, &c) };
        if hit.is_none() {
            return false;
        }
    }
    w.events.push(Event { day: w.day(), kind: EventKind::Born { root: w.life.root(b), mixed: l.mate.is_some() }, node: cell as u32, value: m });
    // Its heat is that of its parent's temperature on the heat capacity it got, with the fusion enthalpy of what it got
    // molten (`Body::heat_with`), so it starts as warm as its parent (a dry child would else be much hotter, a wet one
    // colder); the parent keeps the rest.
    c.e = b.heat_with(&w.chem, &c.store);
    b.e -= c.e;
    for (a, x) in b.store.iter_mut().zip(&c.store) {
        *a -= x;
    }
    b.brood = (b.brood - take).max(0.0);
    let mut rows = child_rows(w, b, l, &mut c, m, size, r);
    // A node's children start no new lineage: a change of the brain's base in one child is lost in the mean of many; a
    // node whose mean sizes moved moves itself (`lineage::resize`).
    if !l.node {
        child_lineage(w, b, &mut c, &mut rows, r);
    }
    if c.lineage == b.lineage && !l.alone {
        if let Some(t) = hit.or_else(|| target(w, l, &c)) {
            let rows = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            let rows = w.life.carry_rows(rows, c.lineage, w.life.bodies[t].lineage, r, w.cfg.new_noise());
            join(w, t, c, &rows);
            return true;
        }
    }
    place_child(w, slot, b, l, c, rows);
    true
}

/// The body of `m` children of `b` in `cell` (not placed): their genes (`mix` with a partner, then `body::vary`), the
/// matter of all `m` from the brood and their provisions (`Litter::prov`). Returns it, the mutation size of the genes
/// it came from and the matter taken, mol.
fn provision(w: &World, b: &Body, l: &Litter, cell: usize, m: f64, r: &mut Rng) -> (Body, f32, f64) {
    let lay = Layout::of(&w.chem);
    let mixed = l.mate.map(|t| mix(b, &w.life.bodies[t], m, r));
    let (from, pooled) = mixed.as_ref().map_or((&b.genes, &b.spread), |x| (&x.0, &x.1));
    let (genes, spread) = vary(&lay, from, pooled, m, w.cfg.phys.alike, r);
    let size = mutation_size(from);
    let mi = w.chem.matter().unwrap_or(0);
    let take = (m * l.matter).min(b.store[mi]);
    let c = Body {
        cell: cell as u32,
        count: m,
        lineage: b.lineage,
        parent: l.parent,
        born: w.day(),
        mark: w.light[cell],
        store: b.store.iter().zip(&l.prov).enumerate().map(|(k, (a, p))| if k == mi { take } else { (p * m).min(*a) }).collect(),
        genes,
        spread,
        genome: b.genome,
        group: b.group,
        near: u32::MAX,
        wet: b.wet,
        ..Default::default()
    };
    (c, size, take)
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

/// The node children `c` join in their cell: of their size and immature (children are below their parents' copy mass,
/// and a mature node must keep its mean there), and of their genes (`body::within`, tested last: it reads every gene).
fn target(w: &World, l: &Litter, c: &Body) -> Option<usize> {
    let (lay, alike) = (Layout::of(&w.chem), w.cfg.phys.alike);
    w.life.at[c.cell as usize].iter().map(|&s| s as usize).find(|&s| {
        let t = &w.life.bodies[s];
        if !t.alive || t.n() <= 1.0 {
            return false;
        }
        let x = t.member_body(&w.chem);
        !t.mature(x) && alike_size(&lay, alike, l.body, x) && within(&lay, alike, t, c)
    })
}

/// The born rows of children `c` of `b` (none: they share their parent's), and the spread left among them. A share
/// `Physics::row_chance` of children change theirs by a step (`brain::row_step`), else share their parent's. Children
/// drawn from a node whose rows differ, or a group of them, get the mean of the group drawn (noise of that variance and
/// the generation's change, over `m`) and keep the variance left among them (up to `body::most`, as a node keeps). With
/// a partner each row is one parent's (`mix::mix_rows`), the group's spread pooled, then the same change. In a null run
/// the rows do not change.
fn child_rows(w: &World, b: &Body, l: &Litter, c: &mut Body, m: f64, size: f32, r: &mut Rng) -> Option<Vec<f32>> {
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
    c.row_spread = (var * (1.0 - 1.0 / mm)).min(most(row_step(p, size), p.alike));
    match (pre, sd > 0.0) {
        (v, false) => v,
        (v, true) => {
            let mut v = v.unwrap_or_else(|| w.life.born(b).to_vec());
            shake(&mut v, r, sd);
            Some(v)
        }
    }
}

/// An individual's child whose brain-size genes are a whole unit away from its lineage's sizes starts a new lineage
/// with the new sizes, its brain carried over where they overlap (`lineage::resized`, its rows by `brain::rerow`);
/// rarely (`Physics::new_lineage`) the base of its lineage's brain is copied with small noise, which starts a new
/// lineage too (in a null run neither adds noise, `Config::new_noise`). A new lineage starts a new group with the
/// child's rows.
fn child_lineage(w: &mut World, b: &Body, c: &mut Body, rows: &mut Option<Vec<f32>>, r: &mut Rng) {
    let (noise, base_noise) = (w.cfg.new_noise(), if w.cfg.null { 0.0 } else { w.cfg.phys.base_noise });
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let fresh = if let Some(l) = resized(lin, &c.genes, w.chem.ns(), r, noise) {
        *rows = Some(brain::rerow(rows.as_deref().unwrap_or(w.life.born(b)), lin.shape, l.shape, r, noise));
        Some(l)
    } else if r.f64() < w.cfg.phys.new_lineage {
        let mut l = lin.clone();
        shake(&mut l.base, r, base_noise);
        Some(l)
    } else {
        None
    };
    if let Some(l) = fresh {
        c.lineage = w.life.new_lineage(l);
        c.group = w.life.new_group(c.lineage, rows.clone().unwrap_or_else(|| w.life.born(b).to_vec()));
    }
}

/// Children `c` of `b` that join no node start a body of their own, born with `rows` (none: its parent's), in a group of
/// its own if brains are not shared (`lineage::own`). Several children together that join no node at all (`alone`) are
/// drawn from it as individuals (`nodes::open`: each with genes and born rows of its own, drawn from their spread),
/// newborn (wear 0), the last one the body itself, with the genes and rows the draws left.
fn place_child(w: &mut World, slot: usize, b: &Body, l: &Litter, mut c: Body, rows: Option<Vec<f32>>) {
    if let Some(v) = rows {
        c.genome = w.life.new_genome(v);
    }
    own(w, b, &mut c);
    // Its key liquids first: `visible` reads the body (`Body::size`) through them. (A parent that survived a freezing is
    // still wet in the sense of its key liquids: its child's are kept though it is born frozen.)
    c.wet = c.key_liquids(&w.chem) | (b.wet & c.key_mask(&w.chem));
    c.vis = c.visible(&w.chem, &w.cfg);
    let s = w.place(c, Some(w.light_keep.get(slot, b.gen)));
    if l.alone && w.life.bodies[s].count > 1.5 {
        let k = w.life.bodies[s].count.round() as usize - 1;
        for m in open(w, s, k) {
            w.life.bodies[m].wear = 0.0;
        }
        let mut c = std::mem::take(&mut w.life.bodies[s]);
        (c.spread, c.row_spread) = (Vec::new(), 0.0);
        w.life.set_count(&mut c, 1.0);
        // (The members drawn took their stores: its height is that of what is left.)
        c.vis[6] = c.height(&w.chem, &w.cfg) as f32;
        w.life.bodies[s] = c;
    }
}

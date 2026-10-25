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

use crate::body::{most, mutation_size, remains, vary, within, Body, Layout, CHILD, DISPERSE, WET, WET_TISSUE};
use crate::brain::{self, row_step, shake};
use crate::chem::Chemistry;
use crate::event::{Event, EventKind};
use crate::lineage::{resized, sizes_of, wanted};
use crate::nodes::{add, may_be_node, open, Lot, COUNT_SLACK};
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::outbox::Entry;
use crate::store::{Class, Node};
use crate::{life::{Domain, Handle}, World};
use mix::{mix, mix_rows, partner};

/// The brood a node holds besides what makes children, in children per member. Its members each fill their own brood and
/// make a child when it covers one, so their broods lie evenly between none and one child (the renewal of each one's
/// brood): half a child each on average. A member who dies loses the brood it held.
pub const BROOD_HELD: f64 = 0.5;

/// The node in whose name the remains of a litter whose parent died go to the ground (`land_litter`): no body holds
/// them now, and the store books no body's side of a move (`store::apply`), so a body node of any slot serves.
const NO_BODY: Node = Node { class: Class::Body, index: 0 };

/// What the children of one `copy` share: each one's body, kg, its matter, mol (its body is that over 1 -
/// `WET_TISSUE`), and its provisions, mol of each store (none of the matter: `provisions`); whether the parent is a
/// node; whether they join no node (`nodes::may_be_node`); the partner; and the parent's id.
struct Births {
    body: f64,
    matter: f64,
    prov: Vec<f64>,
    node: bool,
    alone: bool,
    mate: Option<Handle>,
    parent: u64,
}

/// A mature body makes children whenever its brood covers one: an individual whole children of one parent or two, a
/// node fractional members of one parent beyond `BROOD_HELD`; a share `DISPERSE` starts in a neighbour cell (laws.md
/// 4.2-4.4). Returns whether any were made.
pub fn copy(w: &World, d: &mut Domain, b: &mut Body) -> bool {
    let (chem, n, mc) = (&w.chem, b.n(), b.k.copy_mass);
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
    let id = b.id;
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Mutate);
    let i = b.cell as usize;
    let mate = if node { None } else { partner(w, d, b, &mut r) };
    // Children that start in the neighbour cell of a compass direction drawn (no link that way: none): of a node, the
    // share `DISPERSE` of its births, however few (less than one member with no node to join there waits in the brood,
    // as the stayers do, `emit`); an individual's go together, by that chance.
    let disperse = b.gene_in_range(DISPERSE);
    let prov = provisions(&w.chem, b, mi, matter, births);
    let litter = Births { body, matter, prov, node, alone: !may_be_node(w, b), mate, parent: id };
    let m = if node { disperse * births } else { (r.f64() < disperse) as u8 as f64 };
    let cells = w.mesh.cells();
    let to = if m > 0.0 { cells.way(i, r.below(COMPASS)).map(|e| cells.adj_cell()[e] as usize) } else { None };
    let (cell, m) = match to {
        _ if !node => (to.unwrap_or(i), births),
        Some(j) => (j, m),
        None => (i, 0.0),
    };
    let mut made = false;
    if m > 0.0 {
        made |= emit(w, d, b, &litter, cell, m, &mut r);
    }
    // A node's children who stay: a group of their own (less than one member waits in the brood unless there is a node
    // of theirs to join).
    if node && births - m > 0.0 {
        made |= emit(w, d, b, &litter, i, births - m, &mut r);
    }
    made
}

/// Children that start in another cell than their parent's (`emit`), in the outbox until the batch ends (`land_litter`): their
/// body (in the cell they start in), their born rows (none: their parent's), the log10 of their bodies' mass, whether
/// they join no node (`Births::alone`), the cell their parent was in, and their birth.
#[derive(Clone, Debug)]
pub struct Litter {
    pub(crate) c: Body,
    rows: Option<Vec<f32>>,
    size: f32,
    alone: bool,
    from: u32,
    born: Event,
}

/// `m` children of `b` start in `cell`: their genes, stores, heat and rows, booked on the young node of their kind and
/// lineage there (`kin`), or a body of their own (`place_child`). Less than one member of a node's children with no
/// node to join waits in the brood (false). Children who start in another cell are a litter that lands when the batch
/// ends (`land_litter`).
fn emit(w: &World, d: &mut Domain, b: &mut Body, l: &Births, cell: usize, m: f64, r: &mut Rng) -> bool {
    let mixed = l.mate.map(|t| mix(b, &d[t], m, r));
    let (from, pooled) = mixed.as_ref().map_or((&b.genes, &b.spread), |x| (&x.0, &x.1));
    let (genes, spread) = vary(&Layout::of(&w.chem), from, pooled, m, w.cfg.phys.alike, !w.life.brainless(b), r);
    let size = mutation_size(from);
    // The young node of their kind they join in their parent's cell: for a node's children, its young node
    // (`young::linked`); else the first young node there of their genes and spread (`kin`). Nothing between here and
    // the join changes the nodes of the cell or the children's genes, so the search made now holds for the join.
    let here = cell == b.cell as usize;
    let hit = match () {
        _ if l.alone || !here => None,
        _ if l.node => young::linked(w, d, b, false),
        _ => kin(w, d, cell, b.lineage, (&genes, &spread), false),
    };
    if l.node && here && m < 1.0 && hit.is_none() {
        return false;
    }
    let (store, e) = give(w, b, l, m);
    let (mut rows, row_spread) = child_rows(w, d, b, l, m, size, r);
    // A node's children stay in its lineage: a change of the brain's base in one child is lost in the mean of many; a
    // node whose mean sizes moved moves itself (`lineage::resize`). In another lineage they join its young node there.
    let lineage = if l.node { b.lineage } else { child_lineage(w, d, b, l, &genes, &mut rows, r) };
    let hit = if lineage == b.lineage || l.alone || !here { hit } else { kin(w, d, cell, lineage, (&genes, &spread), false) };
    // Their brain: their born rows plus what their parent learned.
    let own = d.inherit(w, rows.as_deref().unwrap_or(d.born(w, b)), lineage, b);
    let c = Body {
        cell: cell as u32,
        count: m,
        lineage,
        parent: l.parent,
        born: w.day(),
        last: w.day() as f64,
        mark: w.light[cell],
        rained: w.rained[cell],
        keep: b.keep,
        store,
        e,
        genes,
        spread,
        row_spread,
        genome: b.genome,
        rows: own,
        wet: b.wet,
        ..Default::default()
    };
    let (born, size) = (Event { day: w.day(), kind: EventKind::Born { root: d.root(w, b), mixed: l.mate.is_some() }, node: cell as u32, value: m }, l.body.log10() as f32);
    if !here {
        let x = Litter { c, rows, size, alone: l.alone, from: b.cell, born };
        d.out.push(Entry::Litter(Box::new(x)));
        return true;
    }
    d.events.push(born);
    if let (Some(s), true) = (start(w, d, c, rows, size, l.alone, hit), l.node) {
        young::link(d, b, s);
    }
    true
}

/// Children `c` (a body not placed), born with `rows` (none: their parent's) and of bodies of log10 mass `size`, join
/// the node at `hit` (`nodes::add`), or start a body of their own (`place_child`), which is returned.
fn start(w: &World, d: &mut Domain, mut c: Body, rows: Option<Vec<f32>>, size: f32, alone: bool, hit: Option<Handle>) -> Option<Handle> {
    let Some(t) = hit else { return Some(place_child(w, d, alone, c, rows)) };
    let born = rows.unwrap_or_else(|| d.born(w, &c).to_vec());
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread), (size, 0.0), (&born, &c.rows, c.row_spread));
    None
}

/// A litter (`emit`) lands when its batch ends (plan 3.6, S25): it joins the young node of its kind and lineage there
/// (`kin`) or starts a body of its own (`start`), and its birth is recorded. Less than one member (of a node's children:
/// an individual's are whole) with no node to join goes back into its parent's brood if the parent lives; else what
/// they hold goes to the ground of the cell their parent was in, as remains do (`body::remains`).
pub(crate) fn land_litter(w: &mut World, x: Litter) {
    let Litter { mut c, rows, size, alone, from, born } = x;
    let hit = if alone { None } else { w.in_domain(c.cell as usize, |w, d| kin(w, d, c.cell as usize, c.lineage, (&c.genes, &c.spread), false)) };
    if hit.is_some() || c.count >= 1.0 {
        w.events.push(born);
        w.in_domain(c.cell as usize, |w, d| start(w, d, c, rows, size, alone, hit));
    } else if let Some(p) = w.life.find(c.parent) {
        let (mi, p) = (w.chem.matter().unwrap_or(0), &mut w.life[p]);
        p.store.iter_mut().zip(&c.store).for_each(|(a, x)| *a += x);
        (p.e, p.brood) = (p.e + c.e, p.brood + c.store[mi]);
    } else {
        c.cell = from;
        w.in_domain(from as usize, |w, d| {
            d.moves.open(w, from as usize, NO_BODY);
            remains(w, d, &c);
        });
    }
}

/// The stores and heat of `m` children of `b`, taken from it: the matter
/// of all `m` from the brood and their provisions (`Births::prov`), each at most what it holds; their heat that of
/// its temperature on the heat capacity they got, with the fusion enthalpy of what they got molten (`Body::heat_with`),
/// so they start as warm as their parent (a dry child would else be much hotter, a wet one colder).
fn give(w: &World, b: &mut Body, l: &Births, m: f64) -> (Vec<f64>, f64) {
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
    let wet = share * b.dec(WET);
    b.store.iter().enumerate().map(|(k, a)| if k == mi { 0.0 } else { a * (if b.wet >> k & 1 == 1 { wet } else { share }).min(1.0) / births }).collect()
}

/// The first node of `cell` of `lineage` (a node has one brain), mature or young as `mature` asks, and of the kind of
/// genes `genes` and spread `spread` (`body::within`, tested last: it reads every gene): the young node children join
/// (they are below their parents' copy mass, and a mature node must keep its mean there), the adult node young members
/// mature into, or the node members who moved join.
pub(crate) fn kin(w: &World, d: &Domain, cell: usize, lineage: u32, (genes, spread): (&[f32], &[f32]), mature: bool) -> Option<Handle> {
    let (lay, alike) = (Layout::of(&w.chem), w.cfg.phys.alike);
    d.cell(cell).find(|&s| {
        let t = &d[s];
        t.alive && !t.leaving && t.n() > 1.0 && t.lineage == lineage && t.adult(&w.chem) == mature && within(&lay, alike, t, genes, spread)
    })
}

/// The born rows of `m` children of `b` (none: they share their parent's), and the spread left among them. A share
/// `Physics::row_chance` of children change theirs by a step (`brain::row_step`), else share their parent's. Children
/// drawn from a node whose rows differ, or a group of them, get the mean of the group drawn (noise of that variance and
/// the generation's change, over `m`) and keep the variance left among them (up to `body::most`, as a node keeps). With
/// a partner each row is one parent's (`mix::mix_rows`), the group's spread pooled, then the same change. In a null run
/// the rows do not change.
fn child_rows(w: &World, d: &Domain, b: &Body, l: &Births, m: f64, size: f32, r: &mut Rng) -> (Option<Vec<f32>>, f32) {
    let p = &w.cfg.phys;
    let mm = m.max(1.0) as f32;
    let row_noise = if w.cfg.null { 0.0 } else { p.row_noise };
    let (mut pre, mut spread) = (None, b.row_spread);
    if let Some(t) = l.mate.filter(|_| !w.cfg.null) {
        if let Some((v, s)) = mix_rows(w, d, b, &d[t], m, r) {
            // (Rows the same as the parent's are shared, not copied.)
            (pre, spread) = ((v != d.born(w, b)).then_some(v), s);
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
    if sd > 0.0 {
        let mut v = pre.unwrap_or_else(|| d.born(w, b).to_vec());
        shake(&mut v, r, sd);
        return (Some(v), left);
    }
    (pre, left)
}

/// The lineage of an individual's child: its parent's if its brain-size genes fit it (no size a whole unit away,
/// `lineage::wanted`), else its partner's (`Births::mate`) if they fit that, its rows carried over to that brain as
/// `mix_rows` carries them, else a new lineage with the new sizes, its brain carried over where they overlap
/// (`lineage::resized`, its rows by `brain::rerow`). So unlike lineages, which never merge, start only where neither
/// parent's brain fits. Rarely (`Physics::new_lineage`) a child that fits its parent's lineage copies its brain's base
/// with small noise, which starts a new lineage too (in a null run neither adds noise, `Config::new_noise`).
fn child_lineage(w: &World, d: &mut Domain, b: &Body, l: &Births, genes: &[f32], rows: &mut Option<Vec<f32>>, r: &mut Rng) -> u32 {
    let (noise, base_noise) = (w.cfg.new_noise(), if w.cfg.null { 0.0 } else { w.cfg.phys.base_noise });
    let fits = |k: u32| d.brain(w, k).filter(|x| wanted(x, genes) == sizes_of(x));
    let Some(lin) = d.brain(w, b.lineage) else { return b.lineage };
    let fresh = if fits(b.lineage).is_some() {
        if r.f64() >= w.cfg.phys.new_lineage {
            return b.lineage;
        }
        let mut shaken = lin.clone();
        shake(&mut shaken.base, r, base_noise);
        shaken
    } else {
        let born = rows.as_deref().unwrap_or(d.born(w, b));
        if let Some((k, to)) = l.mate.map(|t| d[t].lineage).and_then(|k| Some((k, fits(k)?.shape))) {
            *rows = Some(brain::rerow(born, lin.shape, to, r, noise));
            return k;
        }
        let Some(sized) = resized(lin, genes, w.chem.ns(), r, noise) else { return b.lineage };
        *rows = Some(brain::rerow(born, lin.shape, sized.shape, r, noise));
        sized
    };
    d.new_lineage(fresh)
}

/// Children `c` of `b` that join no node start a body of their own, born with `rows` (none: its parent's). Several
/// children together that join no node at all (`alone`) are drawn from it as individuals (`nodes::open`: each with
/// genes and born rows of its own, drawn from their spread), the last one the body itself, with the
/// genes and rows the draws left. Returns where the body is stored.
fn place_child(w: &World, d: &mut Domain, alone: bool, mut c: Body, rows: Option<Vec<f32>>) -> Handle {
    if let Some(v) = rows {
        c.genome = d.new_genome(v);
    }
    // Its key liquids first: `place` works out its visible traits, which read the body (`Body::size`) through them. (A
    // parent that survived a freezing is still wet in the sense of its key liquids: its child's are kept though it is
    // born frozen.)
    c.wet = c.key_liquids(&w.chem) | (c.wet & c.key_mask(&w.chem));
    let s = d.place(w, c);
    if alone && d[s].count > 1.5 {
        let k = d[s].count.round() as usize - 1;
        open(w, d, s, k);
        let mut c = std::mem::take(&mut d[s]);
        (c.spread, c.row_spread) = (Vec::new(), 0.0);
        d.set_count(&mut c, 1.0);
        // (The members drawn took their stores: what is seen of it is what is left.)
        d.see(w, &mut c);
        d[s] = c;
    }
    s
}

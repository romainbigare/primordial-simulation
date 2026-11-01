//! Law 4: Copy. Reproduction is a share of a surplus. A mature body puts the share `ALLOC` of what it builds (`grow`)
//! into a brood, and whenever the brood covers the matter of a child, a child is made (`copy`) at the body's update,
//! whatever its brain does. A child's genes are its parent's plus
//! small noise, or with two parents each gene is one parent's (`mix`). A population node keeps how its members differ
//! (`Body::spread`, `Body::row_spread`); its children join a young node of their kind where they start, or start one,
//! which grows up as a whole (`kin`). A lineage starts only where a body's brain-size genes move a whole unit from its
//! own (`lineage::resize`).

mod grow;
mod mix;

pub use grow::grow;

use crate::body::{most, mutation_size, remains, vary, Body, Layout, CHILD, DISPERSE, WET, WET_TISSUE};
use crate::brain::{self, row_step, shake};
use crate::chem::{Chemistry, Stores};
use crate::event::{Event, EventKind};
use crate::lineage::{sizes_of, wanted};
use crate::nodes::{add, Lot, Own, COUNT_SLACK, MAX_NODES};
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::life::Entry;
use crate::{life::{Chunk, Handle}, World};
pub use mix::MATE_HIDDEN;
pub(crate) use mix::mates;
use mix::{mix, mix_rows, partner};

/// The brood a node holds besides what makes children, in children per member. Its members each fill their own brood and
/// make a child when it covers one, so their broods lie evenly between none and one child (the renewal of each one's
/// brood): half a child each on average. A member who dies loses the brood it held.
pub const BROOD_HELD: f64 = 0.5;


/// What the children of one `copy` share: each one's matter, mol (its body is that over 1 - `WET_TISSUE`), and its
/// provisions, mol of each store (none of the matter: `provisions`); whether the parent is a
/// node; the partner; and the parent's id.
struct Births {
    matter: f64,
    prov: Stores,
    node: bool,
    mate: Option<Handle>,
    parent: u64,
}

/// A mature body makes children whenever its brood covers one: an individual whole children of one parent or two, a
/// node fractional members of one parent beyond `BROOD_HELD`; a share
/// `DISPERSE` starts in a neighbour cell (laws.md 4.2-4.4). Returns whether any were made.
pub fn copy(w: &World, d: &mut Chunk, b: &mut Body) -> bool {
    let (chem, n, mc) = (&w.chem, b.n(), b.k.copy_mass);
    let Some(mi) = Body::matter_at(chem) else { return false };
    let node = n > 1.0;
    if !b.adult(chem) {
        return false;
    }
    let mm = chem.body().mm[mi];
    b.brood = b.brooded(chem);
    // A child's body (kg) and its matter (mol); the births the brood covers: an individual's whole children, a node's what
    // its brood holds beyond `BROOD_HELD`.
    let body = 10f64.powf(b.gene_in_range(CHILD)) * mc;
    let matter = body * (1.0 - WET_TISSUE) / mm;
    let k = b.brood / matter;
    let births = if node { k - BROOD_HELD * n } else { (k + COUNT_SLACK).floor() };
    if births <= 0.0 {
        return false;
    }
    let id = b.id;
    let mut r = Rng::new(w.seed, w.day(), id, Purpose::Mutate);
    let i = b.cell as usize;
    let mate = if mates(w, d, b) { partner(w, d, b, &mut r) } else { None };
    // Children that start in the neighbour cell of a compass direction drawn (no link that way: none): of a node, the
    // share `DISPERSE` of its births, however few (less than one member with no node to join there waits in the brood,
    // as the stayers do, `emit`), all of one update the same way, as the children of one call cost one `emit`; an
    // individual's go together, by that chance.
    let disperse = b.gene_in_range(DISPERSE);
    let prov = provisions(&w.chem, b, mi, matter, births);
    let litter = Births { matter, prov, node, mate, parent: id };
    let m = if node { disperse * births } else { (r.f64() < disperse) as u8 as f64 };
    let cells = w.mesh.cells();
    let to = if m > 0.0 { cells.way(i, r.below(COMPASS)).map(|e| cells.adj_cell()[e] as usize) } else { None };
    let (cell, m) = match to {
        _ if !node => (to.unwrap_or(i), births),
        Some(j) => (j, m),
        None => (i, 0.0),
    };
    // A node all of whose children would wait (under one member, all staying, no young node to join) makes none now.
    if node && m == 0.0 && births < 1.0 && kin(w, d, i, b.lineage, false).is_none() {
        return false;
    }
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

/// Children that start in another cell than their parent's (`emit`), queued until the serial pass (`land_litter`): their
/// body (in the cell they start in), their born rows (none: their parent's), their brain (`life::Brains::all`, to take
/// into their cell's brains), the cell their parent was in, and their birth.
#[derive(Clone, Debug)]
pub struct Litter {
    pub(crate) c: Body,
    rows: Option<Vec<f32>>,
    brain: Vec<f32>,
    from: u32,
    born: Event,
}

/// `m` children of `b` start in `cell`: their genes, stores, heat and rows, booked on the young node of their kind and
/// lineage there (`kin`), or a body of their own (`place_child`). Less than one member of a node's children with no
/// node to join waits in the brood (false). Children who start in another cell are a litter that lands when the batch
/// ends (`land_litter`).
fn emit(w: &World, d: &mut Chunk, b: &mut Body, l: &Births, cell: usize, m: f64, r: &mut Rng) -> bool {
    let mixed = l.mate.map(|t| mix(b, &d[t], m, r));
    let (from, pooled) = mixed.as_ref().map_or((&b.genes, &b.spread), |x| (&x.0, &x.1));
    let (genes, spread) = vary(&Layout::of(&w.chem), from, pooled, m, w.cfg.phys.alike, !w.life.brainless(b), r);
    let size = mutation_size(from);
    // The young node of their kind they join in their parent's cell: the first young node there of their genes and
    // spread (`kin`). Nothing between here and the join changes the nodes of the cell or the children's genes, so the
    // search made now holds for the join.
    let here = cell == b.cell as usize;
    let hit = if !here { None } else { kin(w, d, cell, b.lineage, false) };
    if l.node && here && m < 1.0 && hit.is_none() {
        return false;
    }
    let (store, e) = give(w, b, l, m);
    let born = Event { day: w.day(), kind: EventKind::Born { root: d.root(w, b), mixed: l.mate.is_some() }, node: cell as u32, value: m };
    // A node's children who stay join its young node there as amounts and means (`nodes::add`): their born rows are
    // their parent's, the generation's change of them its variance (`row_change`), their own rows their parent's; the
    // young node keeps its born rows, so no child body, no draw of every row and no new born rows are made.
    if let (true, Some(t)) = (l.node, hit) {
        // (The noise their mean would take is their spread: `add` pools it, to `body::most`.)
        let (_, sd, left) = row_change(w, b.row_spread, m, size, r);
        let row_spread = left + sd * sd;
        d.events.push(born);
        add(w, d, t, Lot { count: m, store, e, brood: 0.0, last: w.day() as f64 }, (&genes, &spread), (b.lineage, b.genome, Some(Own::Block(b.brain)), row_spread));
        return true;
    }
    let (mut rows, row_spread) = child_rows(w, d, b, l, m, size, r);
    // A node's children stay in its lineage: a change of the brain's base in one child is lost in the mean of many; a
    // node whose mean sizes moved moves itself (`lineage::resize`). In another lineage they join its young node there.
    let lineage = if l.node { b.lineage } else { child_lineage(w, d, b, l, &genes, &mut rows, r) };
    let hit = if lineage == b.lineage || !here { hit } else { kin(w, d, cell, lineage, false) };
    // Their brain: their born rows plus what their parent learned.
    let brain = d.inherit(w, rows.as_deref(), lineage, b);
    let mut c = Body {
        cell: cell as u32,
        count: m,
        lineage,
        parent: l.parent,
        born: w.day(),
        last: w.day() as f64,
        store,
        e,
        genes,
        spread,
        row_spread,
        genome: b.genome,
        brain,
        wet: b.wet,
        ..Default::default()
    };
    if !here {
        let brains = d.brains(b.cell as usize);
        let x = brains.all(c.brain).to_vec();
        brains.free(std::mem::take(&mut c.brain));
        d.out.push(Entry::Litter(Box::new(Litter { c, rows, brain: x, from: b.cell, born })));
        return true;
    }
    d.events.push(born);
    start(w, d, c, rows, hit);
    true
}

/// Children `c` (a body not placed, its brain in its cell's brains), born with `rows` (none: their parent's), join the
/// node at `hit` (`nodes::add`), or start a body of their own (`place_child`).
fn start(w: &World, d: &mut Chunk, mut c: Body, rows: Option<Vec<f32>>, hit: Option<Handle>) {
    if let Some(v) = rows {
        c.genome = d.new_genome(v);
    }
    let Some(t) = hit else {
        place_child(w, d, c);
        return;
    };
    add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread), (c.lineage, c.genome, Some(Own::Block(c.brain)), c.row_spread));
    d.brains(c.cell as usize).free(c.brain);
}

/// A litter (`emit`) lands in the serial pass (laws.md 0) in `d`, the chunk of its cell: it joins the young
/// node of its kind and lineage there (`kin`) or starts a body of its own (`start`), and its birth is recorded. Less
/// than one member (of a node's children: an individual's are whole) with no node to join goes back (`back`).
pub(crate) fn land_litter(w: &World, d: &mut Chunk, x: Litter) {
    let i = x.c.cell as usize;
    let hit = kin(w, d, i, x.c.lineage, false);
    if hit.is_none() && x.c.count < 1.0 {
        return d.out.push(Entry::Back(Box::new(x)));
    }
    let Litter { mut c, rows, brain, born, .. } = x;
    c.brain = d.brains(i).take_in(&brain);
    d.events.push(born);
    start(w, d, c, rows, hit);
}

/// Children of a litter who found no node to join (`land_litter`) go back into their parent's brood if the parent
/// lives; else what they hold goes to the ground of the cell their parent was in, as remains do (`body::remains`).
pub(crate) fn back(w: &mut World, x: Litter) {
    let Litter { mut c, from, .. } = x;
    if let Some(p) = w.life.find(c.parent) {
        let (mi, p) = (Body::matter_at(&w.chem).unwrap_or(0), &mut w.life[p]);
        p.store.iter_mut().zip(&c.store).for_each(|(a, x)| *a += x);
        (p.e, p.brood) = (p.e + c.e, p.brood + c.store[mi]);
    } else {
        c.cell = from;
        w.in_cell(from as usize, |w, d| remains(w, d, &c));
    }
}

/// The stores and heat of `m` children of `b`, taken from it: the matter
/// of all `m` from the brood and their provisions (`Births::prov`), each at most what it holds; their heat that of
/// its temperature on the heat capacity they got, with the fusion enthalpy of what they got molten (`Body::heat_with`),
/// so they start as warm as their parent (a dry child would else be much hotter, a wet one colder).
fn give(w: &World, b: &mut Body, l: &Births, m: f64) -> (Stores, f64) {
    let mi = Body::matter_at(&w.chem).unwrap_or(0);
    let take = (m * l.matter).min(b.store[mi]);
    let store: Stores = std::array::from_fn(|s| if s == mi { take } else { (l.prov[s] * m).min(b.store[s]) });
    let e = b.heat_with(&w.chem, &b.thermal(&w.chem), &store);
    b.e -= e;
    for (a, x) in b.store.iter_mut().zip(&store) {
        *a -= x;
    }
    b.brood = (b.brood - take).max(0.0);
    (store, e)
}

/// What one of the `births` children of `b` takes of each store but the matter (at `mi`), mol: of the parent's stores
/// when the litter starts, the share the litter's `matter` is of the parent's body matter (of the key liquid 10^`WET`
/// times that: a child of a parent whose `WET` is below 0 is dry, and nearly dormant), at most all, split over the
/// births, so children who leave and children who stay get alike.
fn provisions(chem: &Chemistry, b: &Body, mi: usize, matter: f64, births: f64) -> Stores {
    let share = (births * matter).min(b.store[mi]) / (b.store[mi] - b.brooded(chem)).max(1e-300);
    let wet = share * b.dec(WET);
    std::array::from_fn(|s| if s == mi { 0.0 } else { b.store[s] * (if b.wet >> s & 1 == 1 { wet } else { share }).min(1.0) / births })
}

/// The node of `cell` of `lineage` (a node has one brain), mature or young as `mature` asks: the one node of its kind
/// and stage the cell holds (`nodes::crowd`), which children join (the young node: they are below their parents' copy
/// mass, and a mature node must keep its mean there) and members who moved join. In a cell that holds `MAX_NODES`
/// bodies or more, with none of `lineage`, the first of its starting kind and stage (`nodes::cap`), whose brain, born
/// rows and lineage they take (`nodes::add`).
pub(crate) fn kin(w: &World, d: &Chunk, cell: usize, lineage: u32, mature: bool) -> Option<Handle> {
    let root = d.brain(w, lineage).map(|l| l.root);
    let node = |s: &Handle| {
        let t = &d[*s];
        t.alive && !t.leaving && t.n() > 1.0 && t.adult(&w.chem) == mature
    };
    let theirs = d.cell(cell).filter(node).find(|&s| d[s].lineage == lineage);
    let crowded = || d.cell(cell).nth(MAX_NODES - 1).is_some();
    theirs.or_else(|| crowded().then(|| d.cell(cell).filter(node).find(|&s| d.brain(w, d[s].lineage).map(|l| l.root) == root)).flatten())
}

/// The born rows of `m` children of `b` (none: they share their parent's), and the spread left among them. A share
/// `Physics::row_chance` of children change theirs by a step (`brain::row_step`), else share their parent's. Children
/// drawn from a node whose rows differ, or a group of them, get the mean of the group drawn (noise of that variance and
/// the generation's change, over `m`) and keep the variance left among them (up to `body::most`, as a node keeps). With
/// a partner each row is one parent's (`mix::mix_rows`), the group's spread pooled, then the same change. In a null run
/// the rows do not change.
fn child_rows(w: &World, d: &Chunk, b: &Body, l: &Births, m: f64, size: f32, r: &mut Rng) -> (Option<Vec<f32>>, f32) {
    let (mut pre, mut spread) = (None, b.row_spread);
    if let Some(t) = l.mate.filter(|_| !w.cfg.null) {
        if let Some((v, s)) = mix_rows(w, d, b, &d[t], m, r) {
            // (Rows the same as the parent's are shared, not copied.)
            (pre, spread) = ((v != d.born(w, b)).then_some(v), s);
        }
    }
    let (_, sd, left) = row_change(w, spread, m, size, r);
    if sd > 0.0 {
        let mut v = pre.unwrap_or_else(|| d.born(w, b).to_vec());
        shake(&mut v, r, sd);
        return (Some(v), left);
    }
    (pre, left)
}

/// The lineage of an individual's child: its parent's if its brain-size genes fit it (no size a whole unit away,
/// `lineage::wanted`) or fit neither parent's, else its partner's (`Births::mate`), its rows carried over to that brain
/// as `mix_rows` carries them. A lineage starts only where a body's brain-size genes move a whole unit from its own
/// (`lineage::resize`, at its update).
fn child_lineage(w: &World, d: &Chunk, b: &Body, l: &Births, genes: &[f32], rows: &mut Option<Vec<f32>>, r: &mut Rng) -> u32 {
    let fits = |k: u32| d.brain(w, k).filter(|x| wanted(x, genes) == sizes_of(x));
    let Some(lin) = d.brain(w, b.lineage) else { return b.lineage };
    if fits(b.lineage).is_some() {
        return b.lineage;
    }
    let born = rows.as_deref().unwrap_or(d.born(w, b));
    match l.mate.map(|t| d[t].lineage).and_then(|k| Some((k, fits(k)?.shape))) {
        Some((k, to)) => {
            *rows = Some(brain::rerow(born, lin.shape, to, r, w.cfg.new_noise()));
            k
        }
        None => b.lineage,
    }
}

/// The generation's change of the born rows of `m` children of a body whose rows spread `spread`, at mutation size
/// `size`: their variance (the spread and the step of change), the noise of their mean (sd; one draw of
/// `Physics::row_chance` decides it for one child of rows that do not spread) and the variance left among them (up to
/// `body::most`). In a null run nothing changes.
fn row_change(w: &World, spread: f32, m: f64, size: f32, r: &mut Rng) -> (f32, f32, f32) {
    let (p, mm) = (&w.cfg.phys, m.max(1.0) as f32);
    let row_noise = if w.cfg.null { 0.0 } else { p.row_noise };
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
    (var, sd, (var * (1.0 - 1.0 / mm)).min(most(row_step(p, size), p.alike)))
}

/// Children `c` that join no node start a body of their own.
fn place_child(w: &World, d: &mut Chunk, mut c: Body) {
    // Its key liquids first: `place` works out its visible traits, which read the body (`Body::size`) through them. (A
    // parent that survived a freezing is still wet in the sense of its key liquids: its child's are kept though it is
    // born frozen.)
    c.wet = c.key_liquids(&w.chem, c.temp(&w.chem)) | (c.wet & c.key_mask(&w.chem));
    d.place(w, c);
}

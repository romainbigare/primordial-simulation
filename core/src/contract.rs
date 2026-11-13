//! The contract between the natural world (the ground, the air and the cover) and what takes from it (laws.md 0, "The
//! contract"): a taker reads a cell and moves matter only through what is here, and Flow reads what stands on a cell
//! only as a `Stand`. It is plain data and functions over the one source and holds no copy: what a cell shows is read
//! from its cover when a taker asks. Its taker today is the plain taker of the tests (`worldgen::testkit::Taker`).
//!
//! - **What a cell shows**: its cover as a whole and slot by slot by the height its plants stand at (`shown`,
//!   `slots`), the seeds lying on its ground, and its place (`place`).
//! - **Take** (`take`), **give** (`give`) and **press** (`press`): each booked on both sides, never more than is there.
//! - **What stands on a cell** (`Stand`, `stand`): what life there weighs and catches of the light; Flow reads it.
//!
//! When: the place is as the month's start left it (`surroundings`), and so is what stands; the cover is as it is now.
//! Takes land during the takers' month, and the cover feels them in its own next month.

use crate::cover::{Parts, SLOTS, TAKEN_LYING, TAKERS};
use crate::life::{Cell, Chunk, BROKEN, TAKEN};
use crate::store::Node;
use crate::units::MAX_SUBSTANCES;
use crate::World;

/// What stands on a cell: the cover's part, written at the month's start (`stands`); Flow reads it (`stand`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Stand {
    /// Its mass, kg
    pub mass: f64,
    /// Its capture area, m²
    pub leaf: f64,
    /// Its capture area times the share of the light it absorbs, m², and that times the share set aside for its light
    /// reaction, m²
    pub absorbed: f64,
    pub kept: f64,
}

impl Stand {
    /// Its capture area per m² of a cell of `area` m² (in single precision): the shade it casts, as a layer stops
    /// 1 - e^-this of the light.
    pub fn canopy(&self, area: f64) -> f64 {
        (self.leaf / area) as f32 as f64
    }
    /// The share of the light reaching its capture area that it absorbs (a share, in single precision).
    pub fn absorb(&self) -> f64 {
        if self.leaf > 0.0 { (self.absorbed / self.leaf) as f32 as f64 } else { 0.0 }
    }
    /// The share of the light it absorbs that is set aside for its light reaction (a share, in single precision).
    pub fn keep(&self) -> f64 {
        if self.absorbed > 0.0 { (self.kept / self.absorbed) as f32 as f64 } else { 0.0 }
    }
}

/// What stands on cell `i` as the month's start found it.
pub fn stand(w: &World, i: usize) -> Stand {
    w.life.stands.get(i).copied().unwrap_or_default()
}

/// What stands on each cell (`Cover::stand`), on the pool.
pub fn stands(w: &mut World) {
    let n = w.mesh.cells().n();
    w.life.stands = match Parts::of(&w.chem) {
        Some(p) => crate::par::map_n(n, crate::par::UNCOUNTED, |i| w.life.cells[i].cover.stand(&w.chem, &p)),
        None => vec![Stand::default(); n],
    };
}

/// Where a taker stands: the ground's temperature, K, the depth of the liquid standing on it, m, and its surface's
/// height, m, as the month's start left them (`surroundings`).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub t: f64,
    pub liquid: f64,
    pub z: f64,
}

/// The place of cell `i` (`Place`).
pub fn place(w: &World, i: usize) -> Place {
    let [t, liquid, z] = w.life.env.get(i).copied().unwrap_or([0.0; 3]);
    Place { t, liquid, z }
}

/// The ground's temperature, the depth of the liquid standing on it and its surface's height (a cell of parts' land,
/// `flow::parts`) of cell `i`, as Flow's last step left them (once Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.standing_land(i);
    [w.t_ground_last(i), d, z]
}

/// At a month's start (and when the places were never found): Flow's state rebuilt in full, so a resumed run reads what
/// an unbroken one does, and the place of each cell from it (`place`).
pub fn surroundings(w: &mut World) {
    let n = w.mesh.cells().n();
    if w.time.is_multiple_of(w.cfg.calendar().month) || w.life.env.len() != n {
        crate::flow::state(w, true);
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
    }
}

/// A layer of a cell's cover that a taker may take from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// All the cover, each slot giving in proportion to what it holds of what is taken (a bite of the field)
    Cover,
    /// The plants of one slot of the cover
    Slot(usize),
    /// The seeds lying on the ground, each lot giving in proportion to what it holds
    Seeds,
}

impl Layer {
    /// Whether this layer holds slot `x` of the cover.
    fn holds(self, x: usize) -> bool {
        match self {
            Layer::Cover => true,
            Layer::Slot(y) => x == y,
            Layer::Seeds => false,
        }
    }
}

/// A layer of a cell's cover as a taker sees it: its mass, kg, and what it holds of each substance, mol.
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    pub kg: f64,
    pub mol: [f64; MAX_SUBSTANCES],
}

/// What `layer` of the cover of `cell` shows (`Cover::shown`, `Cover::shown_lying`); none with no cover in the
/// chemistry.
pub fn shown(w: &World, cell: &Cell, layer: Layer) -> Option<Shown> {
    let p = Parts::of(&w.chem)?;
    Some(match layer {
        Layer::Seeds => cell.cover.shown_lying(&w.chem, &p),
        _ => cell.cover.shown(&w.chem, &p, |x| layer.holds(x)),
    })
}

/// The slots of the cover of `cell` that hold plants, each with the height they stand at, m, from the lowest.
pub fn slots(w: &World, cell: &Cell) -> Vec<(Layer, f64)> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mut v: Vec<(Layer, f64)> = cell.cover.held().map(|x| (Layer::Slot(x), cell.cover.stands_at(&w.chem, &w.cfg.phys, &p, x))).collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

/// A taker asks `layer` of the cover of cell `i` of chunk `d` for `mol` of substance `k`: it gets what is granted, mol
/// (no more than the layer holds), and the heat that goes with it, J. The cover books its side (`Cover::take`,
/// `Cover::take_lying`), what it loses counted as taken (`Counters::cover_lost` or `Counters::seeds`, `cover::TAKERS` of
/// its budget); the taker books its own.
pub fn take(w: &World, d: &mut Chunk, i: usize, layer: Layer, (k, mol): (usize, f64)) -> (f64, f64) {
    let Some(p) = Parts::of(&w.chem) else { return (0.0, 0.0) };
    let e = p.content(&w.chem);
    let held = d.cover(i).content(&e);
    let (got, heat) = match layer {
        Layer::Seeds => d.cover(i).take_lying(&w.chem, &p, (k, mol)),
        _ => d.cover(i).take(&w.chem, &p, (k, mol), |x| layer.holds(x)),
    };
    let kg = got * w.chem.subs[k].molar_mass;
    match layer {
        Layer::Seeds => d.counts.seeds[TAKEN_LYING] += kg,
        _ => d.counts.cover_lost[TAKEN as usize] += kg,
    }
    d.counts.cover_budget[TAKERS] += d.cover(i).content(&e) - held;
    (got, heat)
}

/// A taker gives `amounts` (substance, mol) and `heat` J to the ground of cell `i` of chunk `d` (remains, what passes
/// through it, what it sheds), through the cell's view of the store.
pub fn give(w: &World, d: &mut Chunk, i: usize, amounts: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
    d.moves.give_all(w, Node::ground(i), amounts, heat);
}

/// Work `energy` J spent pressing on the cover of cell `i` of chunk `d` breaks it: the share `energy` over the energy
/// to break all of it (`Cover::breaking`), at most all, goes to the ground as remains with its heat (`cover::lose`),
/// through the chunk's moves in the name of the cover's node.
pub fn press(w: &World, d: &mut Chunk, i: usize, energy: f64) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (c, mm) = (&d.at(i).cover, p.mm(&w.chem));
    let kg: f64 = (0..SLOTS).map(|x| c.kg(&mm, x)).sum();
    if kg <= 0.0 {
        return;
    }
    let f = (energy / c.breaking(&w.chem, &w.cfg, &p).max(1e-30)).min(1.0);
    d.counts.cover_lost[BROKEN as usize] += f * kg;
    let e = p.content(&w.chem);
    let held = d.cover(i).content(&e);
    d.moves.open(w, i, Node::new(crate::store::Class::Cover, i));
    crate::cover::lose(w, d, &p, i, f);
    d.counts.cover_budget[TAKERS] += d.cover(i).content(&e) - held;
}

//! The contract between the natural world (the ground, the air and the cover) and what takes from it (laws.md 0, "The
//! contract"): a taker reads a cell and moves matter only through what is here, and Flow reads what stands on a cell
//! only as a `Stand`. It is plain data and functions over the one source and holds no copy: what a cell shows is read
//! from its cover when a taker asks. Its takers are the takers' field (`fauna`) and the plain taker of the tests
//! (`worldgen::testkit::Taker`).
//!
//! - **What a cell shows**: its cover as a whole and slot by slot by the height its plants stand at (`shown`,
//!   `slots`), with the energy to break it, the seeds lying on its ground, and its place (`place`).
//! - **Take** (`take`, a bite of all a layer's parts in their proportions), **give** (`give`) and **press** (`press`):
//!   each booked on both sides, never more than is there.
//! - **The key liquid** of a cell (`liquid`: open, in the pores, frozen) and a taker's **vapour** (`vapour`).
//! - A taker's moves (`open`), the gas its air offers it (`air`), and life placed from outside (`placed`).
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

/// Where a taker stands: the ground's temperature, K, the depth of the liquid standing on it, m, its surface's height,
/// m, and the depth of the soft solid lying on its land, m (at the solid's density), as the month's start left them
/// (`surroundings`).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub t: f64,
    pub liquid: f64,
    pub z: f64,
    pub snow: f64,
}

/// The place of cell `i` (`Place`).
pub fn place(w: &World, i: usize) -> Place {
    let [t, liquid, z, snow] = w.life.env.get(i).copied().unwrap_or([0.0; 4]);
    Place { t, liquid, z, snow }
}

/// The ground's temperature, the depth of the liquid standing on it, its surface's height and the depth of the soft
/// solid lying on it (a cell of parts' land, `flow::parts`) of cell `i`, as Flow's last step left them (once Flow's
/// working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 4] {
    let (d, z) = w.scratch.standing_land(i);
    [w.t_ground_last(i), d, z, w.scratch.soft_land(i)]
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

/// What lies on the ground (the seeds lying, height 0) stands this high to a mouth, m: a mouth whose floor is under it
/// and whose band reaches under it takes what lies there (a rule of the model: about a seed's size).
pub const GROUND_M: f64 = 0.01;

/// The share of a plant `height` m tall that a mouth reaches between the heights `band` m, above `floor` m (the snow
/// lying, and the least height its bite reaches down to): its matter is spread evenly over its height (the stand's law
/// draws a plant as a stem of its height, `cover::stem`), so the share is the length of its stem within the band and
/// over the floor over its height. What lies on the ground (height 0) is reached whole where the floor and the band's
/// bottom are under `GROUND_M`, else not at all.
pub fn reached((low, high): (f64, f64), floor: f64, height: f64) -> f64 {
    if height <= 0.0 {
        return if floor < GROUND_M && low < GROUND_M { 1.0 } else { 0.0 };
    }
    ((high.min(height) - low.max(floor)) / height).clamp(0.0, 1.0)
}

/// The share of a plant `height` m tall that stands above `snow` m of soft solid (`reached` with no band and the snow
/// as the floor).
pub fn above(snow: f64, height: f64) -> f64 {
    reached((0.0, f64::INFINITY), snow, height)
}

/// The share of each part of `layer` of cell `i` a taker can reach (`above`): of each slot of its cover the share of its
/// plants above the snow, of the seeds lying all where no snow lies, none where it lies.
fn reach<'a>(w: &'a World, cell: &'a Cell, p: &'a Parts, layer: Layer, snow: f64) -> impl Fn(usize) -> f64 + 'a {
    move |x| match layer {
        Layer::Seeds => above(snow, 0.0),
        _ if layer.holds(x) => above(snow, cell.cover.stands_at(&w.chem, &w.cfg.phys, p, x)),
        _ => 0.0,
    }
}

/// A layer of a cell's cover as a taker sees it: its mass, kg, what it holds of each substance, mol, and the energy to
/// break all of it, J (`life::breaking`).
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    pub kg: f64,
    pub mol: [f64; MAX_SUBSTANCES],
    pub breaking: f64,
}

/// What `layer` of the cover of cell `i` shows a taker (`Cover::shown`, `Cover::shown_lying`): what it can reach above
/// the snow of its place (`reach`); none with no cover in the chemistry.
pub fn shown(w: &World, i: usize, cell: &Cell, layer: Layer) -> Option<Shown> {
    let p = Parts::of(&w.chem)?;
    let of = reach(w, cell, &p, layer, place(w, i).snow);
    Some(match layer {
        Layer::Seeds => cell.cover.shown_lying(&w.chem, &w.cfg.phys, &p, of(0)),
        _ => cell.cover.shown(&w.chem, &w.cfg.phys, &p, of),
    })
}

/// The slots of the cover of `cell` that hold plants, each with the height they stand at, m, from the lowest.
pub fn slots(w: &World, cell: &Cell) -> Vec<(Layer, f64)> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mut v: Vec<(Layer, f64)> = cell.cover.held().map(|x| (Layer::Slot(x), cell.cover.stands_at(&w.chem, &w.cfg.phys, &p, x))).collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

/// What a taker took in one bite (`take`): its mass, kg, what it holds of each substance, mol, and its heat, J.
#[derive(Clone, Copy, Debug)]
pub struct Bite {
    pub kg: f64,
    pub mol: [f64; MAX_SUBSTANCES],
    pub heat: f64,
}

/// A taker bites `want` kg of `layer` of the cover of cell `i` of chunk `d`: a bite takes the parts of what it reaches whole,
/// each in its own proportion, as a mouth takes a piece of a plant (no more than the layer shows within its reach above
/// the snow, `reach`). It gets what is granted (`Bite`). The cover books its side (`Cover::take`, `Cover::take_lying`),
/// what it loses counted as taken (`Counters::cover_lost` or `Counters::seeds`, `cover::TAKERS` of its budget); the taker
/// books its own.
pub fn take(w: &World, d: &mut Chunk, i: usize, layer: Layer, want: f64) -> Bite {
    let none = Bite { kg: 0.0, mol: [0.0; MAX_SUBSTANCES], heat: 0.0 };
    let Some(p) = Parts::of(&w.chem) else { return none };
    let Some(seen) = shown(w, i, d.at(i), layer).filter(|s| s.kg > 0.0 && want > 0.0) else { return none };
    let (e, g) = (p.content(&w.chem), (want / seen.kg).min(1.0));
    let held = d.cover(i).content(&e);
    let of: Vec<f64> = {
        let f = reach(w, d.at(i), &p, layer, place(w, i).snow);
        (0..SLOTS).map(f).collect()
    };
    let (mol, heat) = match layer {
        Layer::Seeds => d.cover(i).take_lying(&w.chem, &p, g, of[0]),
        _ => d.cover(i).take(&w.chem, &p, g, |x| of[x]),
    };
    let kg: f64 = mol.iter().zip(&w.chem.subs).map(|(m, s)| m * s.molar_mass).sum();
    match layer {
        Layer::Seeds => d.counts.seeds[TAKEN_LYING] += kg,
        _ => d.counts.cover_lost[TAKEN as usize] += kg,
    }
    d.counts.cover_budget[TAKERS] += d.cover(i).content(&e) - held;
    Bite { kg, mol, heat }
}

/// A taker gives `amounts` (substance, mol) and `heat` J to the ground of cell `i` of chunk `d` (remains, what passes
/// through it, what it sheds), through the cell's view of the store.
pub fn give(w: &World, d: &mut Chunk, i: usize, amounts: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
    d.moves.give_all(w, Node::ground(i), amounts, heat);
}

/// The key liquid of a cell as a taker finds it, mol: open (standing on its ground, which a mouth reaches), what ran
/// over it in the month (its main channel's, `World::liquid_month`: a mouth reaches it too, but it has run on, so it is
/// no amount of the cell's), in the pores of its ground (the cover's roots reach it, a mouth does not), frozen and
/// lying on its ground as the soft solid (a mouth reaches it, paying its heat of fusion), and frozen in its pores.
#[derive(Clone, Copy, Debug, Default)]
pub struct Liquid {
    pub open: f64,
    pub running: f64,
    pub pores: f64,
    pub soft: f64,
    pub frozen: f64,
}

/// The key liquid of cell `i` as the cell's view in chunk `d` holds it now (`Liquid`): of what its ground holds, the
/// share molten at its temperature (`World::molten`) is liquid, open as far as the depth standing on it at the month's
/// start (`place`) holds, the rest in its pores; of the frozen, the soft solid lying on it as far as its depth at the
/// month's start holds, the rest in its pores; what ran through its main channel in the month's runoff pass over the
/// month. None without a key liquid in the chemistry.
pub fn liquid(w: &World, d: &Chunk, i: usize) -> Liquid {
    let Some(p) = Parts::of(&w.chem) else { return Liquid::default() };
    let (k, area, at) = (p.liquid, w.mesh.cells().area()[i], place(w, i));
    let held = d.moves.amount(w, Node::ground(i), k).max(0.0);
    let (t, plateau) = w.thaw_last(i);
    let molten = held * w.molten(k, i, t, plateau);
    let open = molten.min(at.liquid * area / w.chem.molar_volume(k, crate::chem::Phase::Liquid));
    let soft = (held - molten).min(at.snow * area / w.chem.molar_volume(k, crate::chem::Phase::Solid));
    let running = w.liquid_month.get(i).map_or(0.0, |&f| f as f64 * w.cfg.calendar().month as f64);
    Liquid { open, running, pores: molten - open, soft, frozen: held - molten - soft }
}

/// A taker of cell `i` of chunk `d` sheds heat as vapour of the key liquid: `[liquid, solid]` mol of it, taken from the
/// liquid and the soft solid of its ground (`liquid`: what it drank of the open or the running liquid leaves the cell's
/// liquid, which the running liquid joins; the caller asks no more than its ground holds), go to the cell's aside
/// node as vapour with their latent heat, as the cover's books give a gas (`life::book`). Returns the heat the taker pays
/// for it, J: the vapour's heat and latent heat less the heat the liquid and the solid held in the ground (the solid
/// holds no heat of fusion, so the taker pays it).
pub fn vapour(w: &World, d: &mut Chunk, i: usize, [liquid, solid]: [f64; 2]) -> f64 {
    let Some(p) = Parts::of(&w.chem) else { return 0.0 };
    let (k, mol) = (p.liquid, liquid + solid);
    if mol <= 0.0 {
        return 0.0;
    }
    let (s, t, ground) = (&w.chem.subs[k], w.t_ground_last(i), Node::ground(i));
    let cap = d.moves.heat_cap_ground(w, i);
    d.moves.take(w, ground, k, mol, 0.0);
    let got = d.moves.take_from_ground(w, i, (0, 0.0, 0.0), mol * s.cp * t + liquid * s.lf, cap).1;
    d.moves.gas(w, k, mol, w.lift(i));
    d.moves.give(w, Node::aside(i), 0, 0.0, mol * s.cp * t);
    mol * (s.cp * t + s.latent_gas()) - got
}

/// A taker's moves on cell `i` of chunk `d` open, in the name of the life of the cell: what it takes and gives from here
/// is booked through the cell's view.
pub fn open(w: &World, d: &mut Chunk, i: usize) {
    d.moves.open(w, i, Node::new(crate::store::Class::Life, i));
}

/// What the cell's share of its air node offers a taker of cell `i` of chunk `d` as the cell's view holds it now, mol
/// of each substance that is a gas at the ground's temperature (infinite for the rest: the ground's are limited where
/// they are read), for its books (`life::Books`).
pub fn air(w: &World, d: &Chunk, i: usize) -> [f64; MAX_SUBSTANCES] {
    let (seen, t, chem) = (d.moves.seen(w, w.mesh.up(i, 0, crate::AIR)), place(w, i).t, &w.chem);
    std::array::from_fn(|k| if k < chem.ns() && chem.phase(k, t) == crate::chem::Phase::Gas { seen(k) } else { f64::INFINITY })
}

/// Life placed on cell `i` from outside the world (an edit): `amounts` (substance, mol) and `heat` J come from the
/// outside node into the node of the cell's life (`World::cross`).
pub fn placed(w: &mut World, i: usize, amounts: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
    w.cross((crate::store::Boundary::Outside, Node::new(crate::store::Class::Life, i)), amounts, heat);
}

/// Work `energy` J spent pressing on the cover of cell `i` of chunk `d` breaks it: the share `energy` over the energy
/// to break all of it (`life::breaking` of all its slots), at most all, goes to the ground as remains with its heat
/// (`cover::lose`), through the chunk's moves in the name of the node of the cell's life.
pub fn press(w: &World, d: &mut Chunk, i: usize, energy: f64) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let all = d.at(i).cover.shown(&w.chem, &w.cfg.phys, &p, |_| 1.0);
    if all.kg <= 0.0 {
        return;
    }
    let f = (energy / all.breaking.max(1e-30)).min(1.0);
    d.counts.cover_lost[BROKEN as usize] += f * all.kg;
    let e = p.content(&w.chem);
    let held = d.cover(i).content(&e);
    d.moves.open(w, i, Node::new(crate::store::Class::Life, i));
    crate::cover::lose(w, d, &p, i, f);
    d.counts.cover_budget[TAKERS] += d.cover(i).content(&e) - held;
}

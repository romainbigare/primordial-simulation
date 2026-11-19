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
//! - A taker's moves (`open`), the gas its air offers it (`air`), life placed from outside (`placed`), and the way to
//!   the nearest open liquid (`to_liquid`, `ways_to_liquid`).
//! - **What stands on a cell** (`Stand`, `stand`): what life there weighs and catches of the light; Flow reads it.
//!
//! When: the place is as the month's start left it (`surroundings`), and so is what stands; the cover is as it is now.
//! Takes land during the takers' month, and the cover feels them in its own next month.

use crate::cover::{Parts, NP, SLOTS, TAKEN_LYING, TAKERS};
use crate::life::{Cell, Chunk, BROKEN, TAKEN};
use crate::store::Node;
use crate::units::MAX_SUBSTANCES;
use crate::World;

mod liquid;
pub use liquid::{fetched, liquid, to_liquid, vapour, ways_to_liquid, Liquid, LEVELS};

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
    if w.time.is_multiple_of(w.cfg.calendar().month) || w.life.env.len() != n || w.life.land.len() != n {
        crate::flow::state(w, true);
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
        (w.life.land, w.life.levels) = grounds_now(w);
    }
}

/// The land of every cell, m², and the levels of its parts (`flow::shores`), by which its ground under the liquid is
/// found by height (`flow::Shore`); before Flow first built its state, the land as `lands_now` gives it and no levels.
fn grounds_now(w: &World) -> (Vec<f64>, Vec<Vec<f64>>) {
    let (all, area): (Vec<usize>, _) = ((0..w.mesh.cells().n()).collect(), w.mesh.cells().area());
    let Some(shores) = crate::flow::shores(w, &all) else { return (lands_now(w), Vec::new()) };
    let land = all.iter().map(|&i| area[i] * shores[i].0).collect();
    (land, shores.into_iter().map(|s| s.1).collect())
}

/// The levels of the parts of cell `i` over its mean surface, m, as the month's start found them (`surroundings`,
/// `flow::shores`); none where they were not found.
pub fn levels(w: &World, i: usize) -> &[f64] {
    w.life.levels.get(i).map_or(&[], |l| &l[..])
}

/// The land of every cell, m²: its area times the share of it out of the liquid that stands over its pores
/// (`flow::land_shares`), by Flow's state now; before Flow first built its state, all of a cell whose mean ground
/// stands above the level and none of the others.
fn lands_now(w: &World) -> Vec<f64> {
    let (all, area): (Vec<usize>, _) = ((0..w.mesh.cells().n()).collect(), w.mesh.cells().area());
    let share = crate::flow::land_shares(w, &all).unwrap_or_else(|| all.iter().map(|&i| w.is_land(i) as u8 as f64).collect());
    all.iter().map(|&i| area[i] * share[i]).collect()
}

/// The land of cell `i`, m², as the month's start found it (`surroundings`, `lands_now`): what the land offers life.
/// The cover lives on it.
pub fn land(w: &World, i: usize) -> f64 {
    w.life.land.get(i).copied().unwrap_or(0.0)
}

/// The land of every cell, m² (`land`): as the month's start found it, or where the places are not found yet (a world
/// just loaded), by Flow's state now.
pub fn lands(w: &World) -> Vec<f64> {
    if w.life.land.len() == w.mesh.cells().n() { w.life.land.clone() } else { lands_now(w) }
}

/// A layer of a cell's cover that a taker may take from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// All the cover, each slot giving in proportion to what it holds of what is taken (a bite of the field)
    Cover,
    /// The soft part of the plants of one slot: its living matter (the leaves that capture light and the living cells of
    /// its sapwood) and its fuel
    Soft(usize),
    /// The frame of the plants of one slot: its walls (its stems and branches, and the walls of its leaves)
    Frame(usize),
    /// The seeds lying on the ground, each lot giving in proportion to what it holds
    Seeds,
}

impl Layer {
    /// The share of each part of slot `x` of the cover this layer holds (`life::Parts`: fuel, matter, walls).
    fn parts(self, x: usize) -> [f64; NP] {
        match self {
            Layer::Cover => [1.0; NP],
            Layer::Soft(y) if x == y => [1.0, 1.0, 0.0],
            Layer::Frame(y) if x == y => [0.0, 0.0, 1.0],
            _ => [0.0; NP],
        }
    }
}

/// What lies on the ground (the seeds lying, height 0) stands this high to a mouth, m: a mouth whose floor is under it
/// and whose band reaches under it takes what lies there (a rule of the model: about a seed's size).
pub const GROUND_M: f64 = 0.01;

/// The share of what stands from `base` to `top` m that a mouth reaches between the heights `band` m, above `floor` m
/// (the snow lying, and the least height its bite reaches down to): it is spread evenly over that span (a slot's soft
/// part over its crown, `crown`), so the share is the length of the span within the band and over the floor over its
/// length; a span of no length is reached whole where it lies within the band and over the floor. What lies on the
/// ground (a top of 0) is reached whole where the floor and the band's bottom are under `GROUND_M`, else not at all.
pub fn reached((low, high): (f64, f64), floor: f64, (base, top): (f64, f64)) -> f64 {
    if top <= 0.0 {
        return if floor < GROUND_M && low < GROUND_M { 1.0 } else { 0.0 };
    }
    if top <= base {
        return if low.max(floor) <= top && top <= high { 1.0 } else { 0.0 };
    }
    ((high.min(top) - low.max(floor).max(base)) / (top - base)).clamp(0.0, 1.0)
}

/// The share of what stands from `base` to `top` m that stands above `snow` m of soft solid (`reached` with no band and
/// the snow as the floor).
pub fn above(snow: f64, span: (f64, f64)) -> f64 {
    reached((0.0, f64::INFINITY), snow, span)
}

/// The crown of the plants of slot `x` of the cover of `cell` (cell `i`), from its base to its top, m (`Cover::crown` on
/// the cell's land, `land`).
pub fn crown(w: &World, i: usize, cell: &Cell, p: &Parts, x: usize) -> (f64, f64) {
    let (ph, mm) = (&w.cfg.phys, p.mm(&w.chem));
    let top = cell.cover.height(ph, &mm, w.chem.subs[p.sub[crate::life::WALL]].density[0], x);
    cell.cover.crown(ph, &mm, land(w, i), x, top)
}

/// The share of each part of each slot of `layer` of cell `i` a taker can reach (`above`): of each part the layer holds
/// of a slot of its cover the share of its crown above the snow (`crown`); of the seeds lying all where no snow lies,
/// none where it lies.
fn reach<'a>(w: &'a World, i: usize, cell: &'a Cell, p: &'a Parts, layer: Layer, snow: f64) -> impl Fn(usize) -> [f64; NP] + 'a {
    move |x| match layer {
        Layer::Seeds => [above(snow, (0.0, 0.0)); NP],
        _ => {
            let parts = layer.parts(x);
            if parts == [0.0; NP] { parts } else { parts.map(|f| f * above(snow, crown(w, i, cell, p, x))) }
        }
    }
}

/// A layer of a cell's cover as a taker sees it: its mass, kg, what it holds of each substance, mol, the energy to
/// break all of it, J (`life::breaking`), and of a frame the width of its plants' main stem, m (`Cover::width`; 0 for
/// any other layer).
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    pub kg: f64,
    pub mol: [f64; MAX_SUBSTANCES],
    pub breaking: f64,
    pub width: f64,
}

/// What `layer` of the cover of cell `i` shows a taker (`Cover::shown`, `Cover::shown_lying`): what it can reach above
/// the snow of its place (`reach`); none with no cover in the chemistry.
pub fn shown(w: &World, i: usize, cell: &Cell, layer: Layer) -> Option<Shown> {
    let p = Parts::of(&w.chem)?;
    let of = reach(w, i, cell, &p, layer, place(w, i).snow);
    Some(match layer {
        Layer::Seeds => cell.cover.shown_lying(&w.chem, &w.cfg.phys, &p, of(0)[0]),
        Layer::Frame(x) => {
            let rho = w.chem.subs[p.sub[crate::life::WALL]].density[0];
            Shown { width: cell.cover.width(&w.cfg.phys, &p.mm(&w.chem), rho, x), ..cell.cover.shown(&w.chem, &w.cfg.phys, &p, of) }
        }
        _ => cell.cover.shown(&w.chem, &w.cfg.phys, &p, of),
    })
}

/// The share of a frame of main stem `width` m that a jaw closing with `force` N can cut: it cuts a stem of width d
/// where the force is at least the frame's stress for a cut (`Substance::shear`) over the solid wall of the stem's
/// section (`stem_solid` of π d² / 4); a stand whose branches keep their section from stem to twigs (Shinozaki and
/// others 1964) with lengths as the radius to 2/3 (West, Brown and Enquist 1999) holds the share (d / width)^`thin_exp`
/// of its frame in stems thinner than d, at most all.
pub fn cut(w: &World, width: f64, force: f64) -> f64 {
    let Some(p) = Parts::of(&w.chem) else { return 0.0 };
    let (ph, shear) = (&w.cfg.phys, w.chem.subs[p.sub[crate::life::WALL]].shear);
    let d = (4.0 * force.max(0.0) / (std::f64::consts::PI * shear * ph.stem_solid).max(1e-300)).sqrt();
    if width <= d { 1.0 } else { (d / width).powf(ph.thin_exp) }
}

/// The slots of the cover of `cell` (cell `i`) that hold plants, each as its soft part and its frame, with the span of
/// their crown from its base to its top, m (`crown`: the frame a mouth can cut is the thin stems that carry the leaves;
/// its thick stems stand on the ground, and no jaw cuts them), from the lowest top.
pub fn slots(w: &World, i: usize, cell: &Cell) -> Vec<(Layer, (f64, f64))> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mut v: Vec<(Layer, (f64, f64))> = cell.cover.held().flat_map(|x| [(Layer::Soft(x), crown(w, i, cell, &p, x)), (Layer::Frame(x), crown(w, i, cell, &p, x))]).collect();
    v.sort_by(|a, b| a.1 .1.total_cmp(&b.1 .1));
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
    let of: Vec<[f64; NP]> = {
        let f = reach(w, i, d.at(i), &p, layer, place(w, i).snow);
        (0..SLOTS).map(f).collect()
    };
    let (mol, heat) = match layer {
        Layer::Seeds => d.cover(i).take_lying(&w.chem, &p, g, of[0][0]),
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

/// What lies on the ground of cell `i` of chunk `d` (the remains of life in its ground node: its dead body matter and
/// walls), kg a m² of the cell's land (`land`; none where it holds no land), as the cell's view holds it now.
pub fn remains(w: &World, d: &Chunk, i: usize) -> f64 {
    let Some(p) = Parts::of(&w.chem).filter(|_| land(w, i) > 0.0) else { return 0.0 };
    let kg = |q: usize| d.moves.amount(w, Node::ground(i), p.sub[q]).max(0.0) * w.chem.subs[p.sub[q]].molar_mass;
    (kg(crate::life::MATTER) + kg(crate::life::WALL)) / land(w, i)
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
    let all = d.at(i).cover.shown(&w.chem, &w.cfg.phys, &p, |_| [1.0; NP]);
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

/// Life that moved between cells after the chunks ran (the young of takers that disperse, `fauna::land`): per cell the
/// kg that left its head and the kg that came to it, `sent`; the potential energy it lost or gained is booked
/// (`store::life_sent`, as for the cover's seeds).
pub fn travelled(w: &mut World, sent: impl IntoIterator<Item = (usize, [f64; 2])>) {
    crate::store::life_sent(w, sent);
}

/// Life that came to cell `i` after the chunks ran and dies there: `amounts` (substance, mol) and `heat` J go from the
/// node of the cell's life to its ground, as remains.
pub fn died_at(w: &mut World, i: usize, amounts: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
    let (from, to) = (Node::new(crate::store::Class::Life, i), Node::ground(i));
    let mut moves: Vec<crate::store::Move> = amounts.into_iter().map(|(k, mol)| crate::store::Move { from, to, k: k as u16, mol, heat: 0.0 }).collect();
    moves.push(crate::store::Move { from, to, k: 0, mol: 0.0, heat });
    crate::store::apply(w, &[], &moves);
}

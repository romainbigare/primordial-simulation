//! The key liquid of a cell as the contract shows it to a taker (`liquid`), a taker's vapour (`vapour`), and the way
//! to the nearest open liquid (`ways_to_liquid`, `to_liquid`).

use super::place;
use crate::chem::Heat;
use crate::life::{Chunk, Parts};
use crate::store::{apply, Class, Move, Node};
use crate::World;

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
/// share molten at its temperature (`World::molten_last`) is liquid, open as far as the depth standing on it at the month's
/// start (`place`) holds, the rest in its pores; of the frozen, the soft solid lying on it as far as its depth at the
/// month's start holds, the rest in its pores; what ran through its main channel in the month's runoff pass over the
/// month. None without a key liquid in the chemistry.
pub fn liquid(w: &World, d: &Chunk, i: usize) -> Liquid {
    let Some(p) = Parts::of(&w.chem) else { return Liquid::default() };
    let (k, area, at) = (p.liquid, w.mesh.cells().area()[i], place(w, i));
    let held = d.moves.amount(w, Node::ground(i), k).max(0.0);
    let molten = held * w.molten_last(k, i);
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
    mol * s.carried_gas(t) - got
}

/// The fields of ways (`ways_to_liquid`) are found for open liquid of at least 10^k mol over the month, k from 0 to this less
/// one: a need is rounded up to its decade, so a cell found can meet it (a rule of the model: a decade of resolution).
pub const LEVELS: usize = 13;

/// The way from cell `i` over land to the nearest cell whose open liquid over the month can meet a need of `need` mol
/// (at least the decade above it), m, and that cell (`ways_to_liquid`; an infinite way where none is reached).
pub fn to_liquid(w: &World, i: usize, need: f64) -> (f64, usize) {
    let k = (need.max(1.0).log10().ceil() as usize).min(LEVELS - 1);
    w.life.to_liquid.get(i).map_or((f64::INFINITY, i), |v| (v[k].0, v[k].1 as usize))
}

/// What takers fetched from far cells in their month lands, after every chunk ran (`fauna::land`): each of `asks`
/// (the cell it was taken from, the cell it was carried to, mol of the key liquid), in their order, is taken from its
/// cell's ground as far as the liquid it holds now allows (the asks on a cell shared by what they ask), and given to the
/// ground of the cell it was carried to with its heat at the temperature of the ground it left: it is carried in bodies,
/// from the far cell's ground into its life and from the head of that cell to the head of the walkers' (the potential
/// energy booked as for life that moves, `store::life_sent`), then from their cell's life into its ground.
pub fn fetched(w: &mut World, asks: &[(usize, usize, f64)]) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (k, n) = (p.liquid, w.mesh.cells().n());
    let mut asked = std::collections::BTreeMap::new();
    asks.iter().for_each(|&(from, _, mol)| *asked.entry(from).or_insert(0.0) += mol);
    let s = &w.chem.subs[k];
    let (mut moves, mut sent) = (Vec::with_capacity(2 * asks.len()), vec![[0.0; 2]; n]);
    for &(from, to, mol) in asks.iter().filter(|a| a.2 > 0.0) {
        let liquid = w.store.amounts(Class::Ground)[k * n + from].max(0.0) * w.molten_last(k, from);
        let got = mol * (liquid / asked[&from]).min(1.0);
        let heat = got * s.carried(w.t_ground_last(from), 1.0);
        // It goes from the far cell's ground into the walkers there, over to their cell's head, and into its ground.
        moves.push(Move { from: Node::ground(from), to: Node::new(Class::Life, from), k: k as u16, mol: got, heat });
        moves.push(Move { from: Node::new(Class::Life, to), to: Node::ground(to), k: k as u16, mol: got, heat });
        let kg = got * s.molar_mass;
        (sent[from][0], sent[to][1]) = (sent[from][0] + kg, sent[to][1] + kg);
    }
    apply(w, &[], &moves);
    crate::store::life_sent(w, sent.into_iter().enumerate().filter(|x| x.1 != [0.0; 2]));
}

/// At the start of life's month, after Flow's: for each decade of liquid, the way of each cell to the nearest cell
/// whose open liquid over the month holds that much (`to_liquid`): what stands on it at the month's start (`place`, at the
/// liquid's density) and what ran through it in the month (`World::liquid_month`). From those cells the shortest ways
/// along the links between cells' centres reach every land cell (Dijkstra 1959; ties taken by the cell's index, so the
/// field is fixed by the data).
pub fn ways_to_liquid(w: &mut World) {
    let (cells, month) = (w.mesh.cells(), w.cfg.calendar().month as f64);
    let n = cells.n();
    let volume = Parts::of(&w.chem).map_or(f64::INFINITY, |p| w.chem.molar_volume(p.liquid, crate::chem::Phase::Liquid));
    let open: Vec<f64> = (0..n).map(|i| place(w, i).liquid * cells.area()[i] / volume + w.liquid_month.get(i).map_or(0.0, |&f| f as f64 * month)).collect();
    let land: Vec<bool> = (0..n).map(|i| super::land(w, i) > 0.0).collect();
    let mut field = vec![[(f64::INFINITY, 0); LEVELS]; n];
    for k in 0..LEVELS {
        let way = shortest(w, &land, (0..n).filter(|&i| open[i] >= 10f64.powi(k as i32)));
        field.iter_mut().zip(way).for_each(|(f, d)| f[k] = d);
    }
    w.life.to_liquid = field;
}

/// The shortest way of each cell over the cells that hold `land` to the nearest of `sources`, m (infinite where none is
/// reached), and that source.
fn shortest(w: &World, land: &[bool], sources: impl Iterator<Item = usize>) -> Vec<(f64, u32)> {
    let cells = w.mesh.cells();
    let mut way = vec![(f64::INFINITY, 0); cells.n()];
    let mut next = std::collections::BinaryHeap::new();
    for i in sources {
        way[i] = (0.0, i as u32);
        next.push(std::cmp::Reverse((Far(0.0), i)));
    }
    while let Some(std::cmp::Reverse((Far(d), i))) = next.pop() {
        if d > way[i].0 {
            continue;
        }
        let (links, around) = cells.around(i);
        for (&l, &j) in links.iter().zip(around) {
            let (j, there) = (j as usize, d + cells.dist()[l as usize]);
            if there < way[j].0 && land[j] {
                way[j] = (there, way[i].1);
                next.push(std::cmp::Reverse((Far(there), j)));
            }
        }
    }
    way
}

/// A length, ordered as `f64::total_cmp` orders it (for the heap of `ways_to_liquid`).
#[derive(Clone, Copy, PartialEq)]
struct Far(f64);

impl Eq for Far {}

impl PartialOrd for Far {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Far {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}


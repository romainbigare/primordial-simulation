//! Seeds (laws.md 4.5): what a slot sends out in its month lands, when every cell has had its month, in the cell it
//! goes to, cell by cell in the order made, and always takes hold there (`lodge`): in the nearest slot of its kind, or
//! as seedlings in a slot of its own; what lives after is the light's to decide.

use super::{built_kg, distance, Cover, Parts, APART, NG, NO_ROOT, NP, SLOTS};
use crate::store::{apply, Class, Move, Node};
use crate::World;

/// Seeds of one slot for cell `to`: mol of each part, their genes, their starting kind and their heat, J.
#[derive(Clone, Debug)]
pub struct Seed {
    pub to: u32,
    pub mol: [f64; NP],
    pub genes: [f32; NG],
    pub root: u32,
    pub heat: f64,
}

/// The whole lots the seeds of a slot's month go in: each lands at a point the dispersal kernel draws (`copy::lot`). It
/// sets the noise of where the seeds land, not the mean: in expectation they land as the kernel says, at any size of cell.
pub(super) const LOTS: usize = 8;

/// A share of a slot's seeds that stays in its cell below this is the rounding of the shares of where they land: it
/// goes with the others.
const STAYS_LEAST: f64 = 1e-9;

/// Seeds of slot `x` (`mol` of each part, `heat` J, `genes`) shared out over the cells they land in, `lands` (cell,
/// share; the lots of `copy::lot`), what stays in cell `i` last: each part as a packet of its own (`Seed`), the last taking
/// what the others leave of each part and of the heat, so they hold all of it exactly.
pub(super) fn send(c: &mut Cover, x: usize, (mol, heat): ([f64; NP], f64), genes: [f32; NG], (i, mut lands): (usize, Vec<(usize, f64)>)) {
    let stays = 1.0 - lands.iter().map(|l| l.1).sum::<f64>();
    if stays > STAYS_LEAST || lands.is_empty() {
        lands.push((i, stays));
    }
    let (mut left, mut warm, last) = (mol, heat, lands.len() - 1);
    for (k, &(j, share)) in lands.iter().enumerate() {
        let part: [f64; NP] = std::array::from_fn(|q| if k == last { left[q] } else { (mol[q] * share).min(left[q]) });
        let h = if k == last { warm } else { (heat * share).min(warm) };
        (0..NP).for_each(|q| left[q] -= part[q]);
        warm -= h;
        c.seeds.push(Seed { to: j as u32, mol: part, genes, root: c.root[x], heat: h });
    }
}

/// Land the seeds of every cell's last month (`Cover::seeds`), cell by cell in the order made; the cover of a slot in
/// deficit they take goes to the ground of their cell as remains, booked at once. The potential energy of the
/// life that left a cell's head and came to it is booked once for each cell and way (`store::life_sent`).
pub fn land(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (mm, alike) = (p.mm(&w.chem), w.cfg.phys.alike);
    let (mut moves, mut sent) = (Vec::new(), vec![[0.0; 2]; w.life.cells.len()]);
    for from in 0..w.life.cells.len() {
        for s in std::mem::take(&mut w.life.cells[from].cover.seeds) {
            let i = s.to as usize;
            let kg: f64 = (0..NP).map(|q| s.mol[q] * mm[q]).sum();
            (sent[from][0], sent[i][1]) = (sent[from][0] + kg, sent[i][1] + kg);
            let c = &mut w.life.cells[i].cover;
            let t = c.temp(&w.chem, &p);
            let (mol, heat) = match lodge(c, &mm, &s, alike) {
                Landed::In => continue,
                Landed::Out(m) => {
                    let heat = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * t).min(c.e);
                    c.e -= heat;
                    (m, heat)
                }
            };
            let cover = Node::new(Class::Cover, i);
            moves.extend((0..NP).filter(|&q| mol[q] > 0.0).map(|q| Move { from: cover, to: Node::ground(i), k: p.sub[q] as u16, mol: mol[q], heat: 0.0 }));
            moves.push(Move { from: cover, to: Node::ground(i), k: 0, mol: 0.0, heat });
        }
    }
    crate::store::life_sent(w, sent.into_iter().enumerate().filter(|x| x.1 != [0.0; 2]));
    apply(w, &[], &moves);
}

/// Where seeds went (`lodge`): into the cell's cover, or into a slot in deficit, whose amounts (mol of each part) die.
pub enum Landed {
    In,
    Out([f64; NP]),
}

/// Seeds `s` lodge in cover `c`, with their heat and their store: into the most alike slot of their starting kind within
/// `APART` × `alike` steps (`distance`), the genes the mean by built cover; else they found a slot of their own: an
/// empty one; else the one in deficit (`Cover::net` below zero, the worst first), whose cover dies; else the two slots
/// nearest each other by their genes join (`room`) and they found the slot it frees. So a cell is never closed to seeds
/// and a holder that pays its way is never pushed out: what lives after is the light's to decide, as seedlings live on
/// the light that reaches the bottom of the layers above them and what does not pay dies (the gap models, Shugart 1984).
pub fn lodge(c: &mut Cover, mm: &[f64; NP], s: &Seed, alike: f32) -> Landed {
    let kg = built_kg(mm, &s.mol);
    let kin = (0..SLOTS).filter(|&x| c.root[x] == s.root);
    if let Some((_, x)) = kin.map(|x| (distance(&c.genes[x], &s.genes), x)).filter(|e| e.0 <= APART * alike).min_by(|a, b| a.0.total_cmp(&b.0)) {
        // The seeds join as a slot of their own would, the genes the mean by built cover.
        (0..NG).for_each(|g| c.genes[x][g] += (kg / (kg + c.built(mm, x)).max(1e-300)) as f32 * (s.genes[g] - c.genes[x][g]));
        (0..NP).for_each(|q| c.mol[x][q] += s.mol[q]);
        c.e += s.heat;
        return Landed::In;
    }
    let poorest = (0..SLOTS).filter(|&x| c.net[x] < 0.0).min_by(|&a, &b| c.net[a].total_cmp(&c.net[b]).then(c.built(mm, a).total_cmp(&c.built(mm, b))));
    let (x, out) = match ((0..SLOTS).find(|&x| c.root[x] == NO_ROOT), poorest) {
        (Some(x), _) => (x, None),
        (None, Some(x)) => (x, Some(c.mol[x])),
        (None, None) => (room(c, mm), None),
    };
    (c.mol[x], c.genes[x], c.root[x], c.full[x], c.net[x]) = (s.mol, s.genes, s.root, false, 0.0);
    c.e += s.heat;
    out.map_or(Landed::In, Landed::Out)
}

/// Room in cover `c` whose slots all hold cover that pays its way: the two nearest each other by their genes
/// (`distance`), of any starting kind, join, the lighter into the heavier (`grow::join`): the slots are a limit of the
/// bookkeeping, and the two hold the same niche to within what it resolves. Returns the slot it leaves empty.
fn room(c: &mut Cover, mm: &[f64; NP]) -> usize {
    let pairs = (0..SLOTS).flat_map(|a| (a + 1..SLOTS).map(move |z| (a, z)));
    let (a, z) = pairs.min_by(|p, q| distance(&c.genes[p.0], &c.genes[p.1]).total_cmp(&distance(&c.genes[q.0], &c.genes[q.1]))).expect("a cell holds two slots");
    let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
    super::grow::join(c, mm, big, small);
    small
}

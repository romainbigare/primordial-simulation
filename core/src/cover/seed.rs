//! Seeds (laws.md 4.5): what a slot sends out in its month lands, when every cell has had its month, in the cell it
//! goes to, cell by cell in the order made. It joins the nearest slot of its kind there within `APART` of `alike`
//! (the genes the mean by built cover) unless that slot is full (`Cover::full`), or takes an empty slot (made, where
//! none is, by joining the two most alike slots of one kind), or the lightest if it outweighs it (whose cover then dies
//! as remains); else it dies.

use super::{built_kg, distance, Cover, Parts, APART, EMPTY_KG, NG, NO_ROOT, NP, SLOTS};
use crate::store::{apply, Class, Move, Node};
use crate::World;

/// Seeds of one slot for cell `to`: mol of each part, their genes, their starting kind and their heat, J.
#[derive(Clone, Debug)]
pub(crate) struct Seed {
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

/// Land the seeds of every cell's last month (`Cover::seeds`), cell by cell in the order made; those that find no slot,
/// and the cover they outweigh, go to the ground of their cell as remains, booked at once. The potential energy of the
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
                Landed::Dead => (s.mol, s.heat),
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

/// Where seeds went (`lodge`): into the cell's cover, nowhere (they die), or into a slot whose cover they outweighed
/// (its amounts, mol of each part, die).
pub(super) enum Landed {
    In,
    Dead,
    Out([f64; NP]),
}

/// Seeds `s` lodge in cover `c`, with their heat: into the most alike slot of their starting kind
/// within `APART` × `alike` steps (they die if it is full: one more kg there would not pay, `Cover::full`), else an
/// empty slot; where none is empty, two slots of one starting kind, the most alike, join (`grow::join`) to make room,
/// so a kind new to the cell can take hold beside the variants of another; else the lightest slot if they outweigh its
/// built cover, which dies.
pub(super) fn lodge(c: &mut Cover, mm: &[f64; NP], s: &Seed, alike: f32) -> Landed {
    let kg = built_kg(mm, &s.mol);
    let kin = (0..SLOTS).filter(|&x| c.root[x] == s.root);
    let near = kin.map(|x| (distance(&c.genes[x], &s.genes), x)).filter(|e| e.0 <= APART * alike).min_by(|a, b| a.0.total_cmp(&b.0));
    let place = |c: &mut Cover, x: usize| (c.mol[x], c.genes[x], c.root[x], c.full[x]) = (s.mol, s.genes, s.root, false);
    let x = match near {
        Some((_, x)) if c.full[x] => return Landed::Dead,
        Some((_, x)) => {
            // The seeds join as a slot of their own would, the genes the mean by built cover.
            (0..NG).for_each(|g| c.genes[x][g] += (kg / (kg + c.built(mm, x)).max(1e-300)) as f32 * (s.genes[g] - c.genes[x][g]));
            (0..NP).for_each(|q| c.mol[x][q] += s.mol[q]);
            c.e += s.heat;
            return Landed::In;
        }
        None => match (0..SLOTS).find(|&x| c.root[x] == NO_ROOT).or_else(|| room(c, mm)) {
            Some(x) => x,
            None => (0..SLOTS).min_by(|&a, &b| c.built(mm, a).total_cmp(&c.built(mm, b))).expect("a cell has slots"),
        },
    };
    let (was, out) = (c.root[x] != NO_ROOT, c.mol[x]);
    if was && c.built(mm, x) > EMPTY_KG && c.built(mm, x) >= kg {
        return Landed::Dead;
    }
    place(c, x);
    c.e += s.heat;
    if was { Landed::Out(out) } else { Landed::In }
}

/// Room in full cover `c`: the two slots of one starting kind that are most alike (`distance`) join, the lighter into
/// the heavier (`grow::join`); the slot it leaves empty, or none if no two slots share a starting kind.
fn room(c: &mut Cover, mm: &[f64; NP]) -> Option<usize> {
    let pairs = (0..SLOTS).flat_map(|a| (a + 1..SLOTS).map(move |z| (a, z))).filter(|&(a, z)| c.root[a] == c.root[z]);
    let (a, z) = pairs.min_by(|p, q| distance(&c.genes[p.0], &c.genes[p.1]).total_cmp(&distance(&c.genes[q.0], &c.genes[q.1])))?;
    let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
    super::grow::join(c, mm, big, small);
    Some(small)
}

//! Seeds (laws.md 4.5): what a slot sends out in its month lands, when every domain has had its month, in the cell it
//! goes to, domain by domain in the order made. It joins the nearest slot of its kind there within `APART` of `alike`
//! (the genes the mean by built cover) unless that slot is full (`Cover::full`), or takes an empty slot, or the
//! lightest if it outweighs it (whose cover then dies as remains); else it dies.

use super::{built_kg, distance, Cover, Parts, APART, EMPTY_KG, NG, NO_ROOT, NP};
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

/// Land the seeds of every domain's last month (`Cover::seeds`), domain by domain in the order made; those that find
/// no slot, and the cover they outweigh, go to the ground of their cell as remains, booked at once.
pub fn land(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (mm, alike) = (p.mm(&w.chem), w.cfg.phys.alike);
    let mut moves = Vec::new();
    for from in 0..w.life.domains.len() {
        for s in std::mem::take(&mut w.life.domains[from].cover.seeds) {
            let i = s.to as usize;
            let (d, k) = w.life.cell_slot(i);
            let c = &mut w.life.domains[d].cover;
            let t = c.temp(&w.chem, &p, k);
            let (mol, heat) = match settle(c, &mm, k, &s, alike) {
                Landed::In => continue,
                Landed::Dead => (s.mol, s.heat),
                Landed::Out(m) => {
                    let heat = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * t).min(c.e[k]);
                    c.e[k] -= heat;
                    (m, heat)
                }
            };
            let cover = Node::new(Class::Cover, i);
            moves.extend((0..NP).filter(|&q| mol[q] > 0.0).map(|q| Move { from: cover, to: Node::ground(i), k: p.sub[q] as u16, mol: mol[q], heat: 0.0 }));
            moves.push(Move { from: cover, to: Node::ground(i), k: 0, mol: 0.0, heat });
        }
    }
    apply(w, &[], &moves);
}

/// Where seeds went (`settle`): into the cell's cover, nowhere (they die), or into a slot whose cover they outweighed
/// (its amounts, mol of each part, die).
pub(super) enum Landed {
    In,
    Dead,
    Out([f64; NP]),
}

/// Seeds `s` settle in the `k`th cell of cover `c`, with their heat: into the most alike slot of their starting kind
/// within `APART` × `alike` steps (they die if it is full: one more kg there would not pay, `Cover::full`), else an
/// empty slot, or the lightest slot if they outweigh its built cover, which dies.
pub(super) fn settle(c: &mut Cover, mm: &[f64; NP], k: usize, s: &Seed, alike: f32) -> Landed {
    let kg = built_kg(mm, &s.mol);
    let kin = Cover::slots(k).filter(|&x| c.root[x] == s.root);
    let near = kin.map(|x| (distance(&c.genes[x], &s.genes), x)).filter(|e| e.0 <= APART * alike).min_by(|a, b| a.0.total_cmp(&b.0));
    let place = |c: &mut Cover, x: usize| (c.mol[x], c.genes[x], c.root[x], c.full[x]) = (s.mol, s.genes, s.root, false);
    let x = match near {
        Some((_, x)) if c.full[x] => return Landed::Dead,
        Some((_, x)) => {
            // The seeds join as a slot of their own would, the genes the mean by built cover.
            (0..NG).for_each(|g| c.genes[x][g] += (kg / (kg + c.built(mm, x)).max(1e-300)) as f32 * (s.genes[g] - c.genes[x][g]));
            (0..NP).for_each(|q| c.mol[x][q] += s.mol[q]);
            c.e[k] += s.heat;
            return Landed::In;
        }
        None => Cover::slots(k).min_by(|&a, &b| c.built(mm, a).total_cmp(&c.built(mm, b))).expect("a cell has slots"),
    };
    let (was, out) = (c.root[x] != NO_ROOT, c.mol[x]);
    if was && c.built(mm, x) > EMPTY_KG && c.built(mm, x) >= kg {
        return Landed::Dead;
    }
    place(c, x);
    c.e[k] += s.heat;
    if was { Landed::Out(out) } else { Landed::In }
}

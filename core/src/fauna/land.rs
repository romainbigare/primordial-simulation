//! The young that leave (laws.md 3b): of a slot's young that mature in a month a share leaves for cells the dispersal
//! kernel of the seeds draws (`life::lot`), and the liquid its members fetched from a far cell is that cell's. Both wait
//! in the outbox of their cell (`Outbox`) and land after every chunk ran (`land`), cell by cell in the order of the cells
//! and of what each sent, as the cover's seeds land.

use super::grow::whole;
use super::lodge::{room, take_slot};
use super::{mm, sub, Fauna, NF, NG, N_WHOLE};
use crate::contract;
use crate::life::{lot, Parts, SPENT};
use crate::rng::{Purpose, Rng};
use crate::World;

/// The lots the maturing young of a slot over `N_WHOLE` leave in, each landing where the kernel draws (`life::lot`); under
/// it each young leaves alone. It sets the noise of where they land, not the mean (as the seeds' lots).
const LOTS: usize = 8;
/// The bit that keeps the keys of the draws of landing apart from those of the slots' months (`Purpose::Fauna`).
const LANDING: u64 = 1 << 62;
/// The bits of a landing's key that number what one cell sent.
const SENT_BITS: u32 = 20;

/// Young that left a slot for cell `to`: their starting kind and genes, their members, their amounts (mol of each part)
/// and their heat, J.
#[derive(Clone, Debug)]
pub(crate) struct Brood {
    to: u32,
    root: u32,
    genes: [f32; NG],
    n: f64,
    mol: [f64; NF],
    pub(super) heat: f64,
}

impl Brood {
    /// Their heat capacity in `chem` for parts of the substances `sub`, J/K.
    pub(super) fn cap(&self, chem: &crate::chem::Chemistry, sub: &[usize; NF]) -> f64 {
        (0..NF).map(|q| self.mol[q] * chem.subs[sub[q]].cp).sum()
    }
    /// The temperature its body keeps on ground at `t` K.
    pub(super) fn tb(&self, t: f64) -> f64 {
        (self.genes[super::KEEP] as f64).max(t)
    }
}

/// What a cell's takers sent in their last month, landed after every chunk ran (`land`): the young that left, and per
/// far cell the key liquid they fetched from it, mol.
#[derive(Clone, Debug, Default)]
pub(crate) struct Outbox {
    pub(super) young: Vec<Brood>,
    pub(super) drink: Vec<(u32, f64)>,
}

/// The young of slot `x` of `f` in cell `i` that matured this month, `n` members holding `moved` mol of each part in its
/// adults: in lots (one a member under `N_WHOLE`, else `LOTS`), each drawn from `r` by the kernel (`life::lot`); the lots
/// that land in another cell leave its adults for the outbox.
pub(super) fn disperse(w: &World, (f, x, i): (&mut Fauna, usize, usize), (n, moved): (f64, &[f64; NF]), r: &mut Rng) {
    if n <= 0.0 {
        return;
    }
    let lots = if n < N_WHOLE { n as usize } else { LOTS };
    let share = 1.0 / lots as f64;
    for _ in 0..lots {
        let Some(j) = lot(w, i, r).filter(|&j| j != i) else { continue };
        let mol: [f64; NF] = std::array::from_fn(|q| moved[q] * share);
        f.n[x] -= n * share;
        (0..NF).for_each(|q| f.mol[x][q] -= mol[q]);
        f.out.young.push(Brood { to: j as u32, root: f.root[x], genes: f.genes[x], n: n * share, mol, heat: 0.0 });
    }
}

/// After every chunk ran: what each cell's takers sent (`Outbox`) lands, cell by cell in the order of the cells and of
/// what it sent. Young that reach land join the adults of the slot they belong to there by the slot rule (`room`;
/// a slot in deficit they take dies as remains), with their heat, their count whole under `N_WHOLE` by a keyed draw;
/// young that reach no land, or find no room, die there, their matter and heat to its ground (`contract::died_at`).
/// The potential energy of what moved is booked (`contract::travelled`). Then the liquid fetched is taken from the far
/// cells (`contract::fetched`).
pub fn land(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (sub, mm) = (sub(&p), mm(&w.chem, &p));
    let n = w.life.cells.len();
    let (mut sent, mut asks) = (vec![[0.0; 2]; n], Vec::new());
    for from in 0..n {
        let out = std::mem::take(&mut w.life.cells[from].fauna.out);
        asks.extend(out.drink.iter().map(|&(j, mol)| (j as usize, from, mol)));
        for (k, b) in out.young.into_iter().enumerate() {
            let to = b.to as usize;
            let kg: f64 = (0..NF).map(|q| b.mol[q] * mm[q]).sum();
            (sent[from][0], sent[to][1]) = (sent[from][0] + kg, sent[to][1] + kg);
            let mut r = Rng::new(w.seed, w.month(), LANDING | (from as u64) << SENT_BITS | k as u64, Purpose::Fauna);
            let died = if contract::land(w, to) > 0.0 { arrive(w, to, &b, &mut r) } else { Some(b.n) };
            if let Some(dead) = died {
                w.counts.takers_died[SPENT as usize] += dead;
                contract::died_at(w, to, (0..NF).map(|q| (sub[q], b.mol[q])), b.heat);
            }
        }
    }
    contract::travelled(w, sent.into_iter().enumerate().filter(|x| x.1 != [0.0; 2]));
    contract::fetched(w, &asks);
}

/// Young `b` arrive in cell `to` (its takers `f`): they join the adults of the slot `room` gives, a slot in deficit it
/// takes dying as remains (`contract::died_at`, its heat with it). Returns the members that die for want of room.
fn arrive(w: &mut World, to: usize, b: &Brood, r: &mut Rng) -> Option<f64> {
    let p = Parts::of(&w.chem)?;
    let (sub, mm) = (sub(&p), mm(&w.chem, &p));
    let alike = w.cfg.phys.alike;
    let f = &mut w.life.cells[to].fauna;
    let (Some((y, founds)), evicted) = room(alike, f, (&b.genes, b.root), &[]) else { return Some(b.n) };
    let lost = if evicted.n > 0.0 || evicted.mol != [0.0; NF] { std::mem::take(&mut f.e[y]) } else { 0.0 };
    f.n[y] = whole(f.n[y] + b.n, r);
    (0..NF).for_each(|q| f.mol[y][q] += b.mol[q]);
    f.e[y] += b.heat;
    let (kg, living) = (b.mol[super::BODY] * mm[super::BODY] / (1.0 - crate::life::WET_TISSUE), f.living(&mm, y));
    take_slot(f, y, (&b.genes, b.root), founds, (kg, living));
    if evicted.n > 0.0 {
        w.counts.takers_died[SPENT as usize] += evicted.n;
    }
    if evicted.mol != [0.0; NF] {
        contract::died_at(w, to, (0..NF).map(|q| (sub[q], evicted.mol[q])), lost);
    }
    None
}

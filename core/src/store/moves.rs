//! The three ways a law changes the world (`docs/foundation/plan.md`, Idea 2): a move between two nodes, a transform
//! of the amounts inside one node, and a link flux over a whole field; and `apply`, which books them.

use super::nodes::{Class, NodeId, Nodes, CLASSES};
use crate::chem::Chemistry;
use crate::ground::share;
use crate::World;

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;

/// `mol` of substance `k` and `heat` J leave node `from` for node `to`. `to` receives `heat` less the energy the mol
/// hold apart from heat more there than in `from` (`held`): so the law chooses who pays the latent heat of a gas that
/// enters the air, `from` by giving it with the heat, or `to` by being given only the heat the gas carries.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub from: NodeId,
    pub to: NodeId,
    pub k: u16,
    pub mol: f64,
    pub heat: f64,
}

/// The amounts of node `at` change by `dmol` (substance, mol), keeping every element; the energy they held apart from
/// heat (`held`) and no longer hold becomes the node's heat (the heat of reaction).
#[derive(Clone, Debug)]
pub struct Transform {
    pub at: NodeId,
    pub dmol: Vec<(u16, f64)>,
}

/// What crosses each link of a field of nodes of one class at once: link `l` joins the nodes at index `a[l]` and
/// `b[l]` and carries `mol[q * links + l]` mol of substance `ks[q]` and `heat[l]` J, + from `a` to `b` (`heat` empty:
/// no heat). What leaves one end enters the other, so a flux keeps the books by its form.
#[derive(Clone, Copy, Debug)]
pub struct LinkFlux<'a> {
    pub class: Class,
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub ks: &'a [u16],
    pub mol: &'a [f64],
    pub heat: &'a [f64],
}

/// Energy a mol of substance `k` holds in a node of `class` apart from the node's heat, J/mol: its formation enthalpy,
/// and in the air its fusion and vaporisation enthalpy (`Substance::latent_gas`), as `check::totals` books them.
pub fn held(chem: &Chemistry, class: Class, k: usize) -> f64 {
    let s = &chem.subs[k];
    s.hf + if class == Class::Air { s.latent_gas() } else { 0.0 }
}

/// One number of the world: an amount (class, `k * len + index`) or a heat (class, index).
#[derive(Clone, Copy)]
enum Slot {
    Amount(usize, usize),
    Heat(usize, usize),
}

impl Slot {
    /// 0 for an amount, 1 for a heat: its place in `Counters::scaled` and `Counters::rounded`.
    fn unit(self) -> usize {
        matches!(self, Slot::Heat(..)) as usize
    }
}

/// One array per class for the amounts and one for the heat: the world's, or `apply`'s own of the same shape.
struct Field<T> {
    amount: [T; CLASSES],
    heat: [T; CLASSES],
}

impl<T: AsMut<[f64]>> Field<T> {
    fn at(&mut self, s: Slot) -> &mut f64 {
        match s {
            Slot::Amount(c, x) => &mut self.amount[c].as_mut()[x],
            Slot::Heat(c, x) => &mut self.heat[c].as_mut()[x],
        }
    }
}

/// One change `apply` books: a link of a flux, a move or a transform.
#[derive(Clone, Copy)]
enum Item<'i, 'a> {
    Link(&'i LinkFlux<'a>, usize),
    Move(&'i Move),
    Transform(&'i Transform),
}

/// What `item` changes, unscaled, into `out`.
fn deltas(item: Item, table: &Nodes, chem: &Chemistry, len: &[usize; CLASSES], out: &mut Vec<(Slot, f64)>) {
    out.clear();
    match item {
        Item::Link(x, l) => {
            let (c, links, a, b) = (x.class as usize, x.a.len(), x.a[l] as usize, x.b[l] as usize);
            for (q, &k) in x.ks.iter().enumerate() {
                let (m, at) = (x.mol[q * links + l], k as usize * len[c]);
                out.extend([(Slot::Amount(c, at + a), -m), (Slot::Amount(c, at + b), m)]);
            }
            if let Some(&h) = x.heat.get(l) {
                out.extend([(Slot::Heat(c, a), -h), (Slot::Heat(c, b), h)]);
            }
        }
        Item::Move(m) => {
            let (((cf, i), (ct, j)), k) = ((table.place(m.from), table.place(m.to)), m.k as usize);
            let lift = m.mol * (held(chem, ct, k) - held(chem, cf, k));
            let (cf, ct) = (cf as usize, ct as usize);
            out.extend([(Slot::Amount(cf, k * len[cf] + i), -m.mol), (Slot::Heat(cf, i), -m.heat)]);
            out.extend([(Slot::Amount(ct, k * len[ct] + j), m.mol), (Slot::Heat(ct, j), m.heat - lift)]);
        }
        Item::Transform(t) => {
            let ((class, i), mut q) = (table.place(t.at), 0.0);
            let c = class as usize;
            for &(k, d) in &t.dmol {
                out.push((Slot::Amount(c, k as usize * len[c] + i), d));
                q -= d * held(chem, class, k as usize);
            }
            out.push((Slot::Heat(c, i), q));
        }
    }
}

/// Does `dmol` keep every element of the chemistry's table, to rounding? `sums` is room for the element totals.
fn keeps_elements(chem: &Chemistry, dmol: &[(u16, f64)], sums: &mut Vec<f64>) -> bool {
    sums.clear();
    sums.resize(chem.n_elements, 0.0);
    let mut size = 0.0;
    for &(k, d) in dmol {
        chem.add_elems(k as usize, d, sums);
        size += d.abs() * chem.subs[k as usize].elems.iter().map(|&c| c as f64).sum::<f64>();
    }
    sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
}

/// Book the link fluxes, then the moves, then the transforms, each in its order. Where all that would leave a number
/// (an amount of a node, or its heat) is more than it holds, each change that takes from it is scaled down by the
/// share it holds (the least share over all it takes from), so no amount and no heat falls below zero
/// (`docs/handover.md` 6.1). What the scaling left in place is counted in `Counters::scaled`, and the last rounding
/// below zero, set to zero, in `Counters::rounded`. Panics on a transform that does not keep its elements, and on the
/// id of a node that is gone.
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move], transforms: &[Transform]) {
    w.fit_deep();
    let World { chem, table, counts, grid, ground, e_ground, air, e_air, deep, e_deep, .. } = w;
    let len = [grid.n(), grid.na(), grid.n()];
    let mut books = Field { amount: [&mut ground[..], &mut air[..], &mut deep[..]], heat: [&mut e_ground[..], &mut e_air[..], &mut e_deep[..]] };
    let zeros = |v: &[&mut [f64]; CLASSES]| v.each_ref().map(|a| vec![0.0; a.len()]);
    let mut share_of = Field { amount: zeros(&books.amount), heat: zeros(&books.heat) };
    let items = || {
        let links = fluxes.iter().flat_map(|x| (0..x.a.len()).map(move |l| Item::Link(x, l)));
        links.chain(moves.iter().map(Item::Move)).chain(transforms.iter().map(Item::Transform))
    };
    for x in fluxes {
        let links = x.a.len();
        assert!(x.b.len() == links && x.mol.len() == x.ks.len() * links && (x.heat.is_empty() || x.heat.len() == links), "a link flux of the wrong shape");
    }
    let (mut d, mut sums) = (Vec::new(), Vec::new());
    // What each change asks of each number.
    for item in items() {
        if let Item::Transform(t) = item {
            assert!(keeps_elements(chem, &t.dmol, &mut sums), "a transform that does not keep its elements: {t:?}");
        }
        deltas(item, table, chem, &len, &mut d);
        d.iter().filter(|e| e.1 < 0.0).for_each(|&(s, x)| *share_of.at(s) -= x);
    }
    // The share of it each number can give.
    let mut asked = Vec::new();
    for c in 0..CLASSES {
        let amounts = (0..books.amount[c].len()).map(|x| Slot::Amount(c, x));
        let heats = (0..books.heat[c].len()).map(|x| Slot::Heat(c, x));
        for s in amounts.chain(heats) {
            let need = *share_of.at(s);
            if need > 0.0 {
                *share_of.at(s) = share(books.at(s).max(0.0), need);
                asked.push(s);
            }
        }
    }
    // Each change, scaled by the least share of what it takes from.
    for item in items() {
        deltas(item, table, chem, &len, &mut d);
        let f = d.iter().filter(|e| e.1 < 0.0).fold(1.0, |f: f64, &(s, _)| f.min(*share_of.at(s)));
        for &(s, x) in &d {
            *books.at(s) += x * f;
            if x < 0.0 {
                counts.scaled[s.unit()] -= x * (1.0 - f);
            }
        }
    }
    for s in asked {
        let v = books.at(s);
        if *v < 0.0 {
            counts.rounded[s.unit()] -= *v;
            *v = 0.0;
        }
    }
}

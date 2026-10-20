//! Law 2: Change. Substances react inside nodes by mass action, faster when hot (laws.md 2). `kernel`: the rate law
//! and the reactor; `balance`: reactions that undo each other; `gas`: the gas a body or a part exchanges with the air.
//! Here: the step over the ground and air nodes, which react when due and sleep while calm (`clock::Sleep`).

mod balance;
mod gas;
mod kernel;

pub use balance::reversible;
pub(crate) use gas::{send_gas, GasMove};
pub use kernel::Reactor;
pub(crate) use kernel::{denature, frac, pace, scarce};

use crate::{flow::FlowView, ground::share, store::{apply, Batch, Class, LinkFlux, Node, Transform}, AIR, World};

/// A node sleeps until its reactions would use this share of their inputs (but for those on a `SPECK` of what it
/// holds) or warm it `CALM_K` kelvin: for so little, one long span is as good as many steps.
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;

/// Run Law 2 for the step of `dt` seconds at the temperatures of the last flow step: each ground node, then each air
/// node, that is due, for all the time since it last reacted, from the world as it was before its class reacted, booked
/// in one call (`Batches`). Then it sleeps (`Reactor::rest`, at most `sleep_days`). The nodes react apart (in parallel,
/// with `threads`), each into its row (`ROW`), and their rows are booked in the order of the nodes.
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let (end, most) = (w.time + dt as u64, w.cfg.sleep_days as u64 * w.cfg.calendar().day);
    let ns = w.chem.ns();
    for (class, len) in [(Class::Ground, w.mesh.cells().n()), (Class::Air, w.mesh.level(AIR).n())] {
        w.sleep.watch(class, len, w.store.amounts(class), w.cfg.wake_share, w.time);
        // Before the law first runs, every node is due.
        if w.sleep.none(class) {
            (0..len).for_each(|i| w.sleep.sleep(class, i, w.time, w.time));
        }
        // By index, so in the order of their cells.
        let due = w.sleep.due(class, w.time);
        let mut rows = vec![0.0; due.len() * (ROW * ns + 1)];
        let (ww, nodes) = (&*w, w.scratch.view());
        crate::par::rows(&mut rows, ROW * ns + 1, || (Reactor::new(&ww.chem, &ww.cfg), Held { amt: vec![0.0; ns], pool: vec![0.0; ns] }), |(rc, h), x, row| {
            let (i, amounts) = (due[x], ww.store.amounts(class));
            h.amt.iter_mut().enumerate().for_each(|(k, a)| *a = amounts[k * len + i]);
            let (all, span) = (h.amt.iter().sum::<f64>(), end.saturating_sub(ww.sleep.since(class, i)) as f64);
            if class == Class::Ground {
                react_ground(ww, rc, h, &nodes, i, span, row);
            } else {
                // An air node holds its gas itself: no liquid's film stands between them, and it is no moist ground
                // (the reactor's `film` and `moist` stay 0: it reacts only air nodes).
                rc.react(&ww.chem, &mut h.amt, None, &mut { ww.store.heat(Class::Air)[i] }, 0.0, None, 0.0, span);
                rc.made(&ww.chem, &mut row[..ns]);
            }
            row[ROW * ns] = (rc.rest(CALM, SPECK * all, CALM_K).max(0.0) as u64).min(most) as f64;
        });
        let mut b = std::mem::take(&mut w.batches);
        b.clear(w.mesh.level(AIR).n(), ns);
        let rests: Vec<u64> = due.iter().zip(rows.chunks_exact(ROW * ns + 1)).map(|(&i, row)| b.add(w, Node::new(class, i), row)).collect();
        b.book(w);
        w.batches = b;
        for (&i, rest) in due.iter().zip(rests) {
            w.sleep.sleep(class, i, end, (end + rest).saturating_sub(dt as u64));
            w.sleep.keep(class, i, len, w.store.amounts(class));
        }
    }
}

/// A node's amounts and the gas around it (a ground node's share of its air node) as it reacts, mol.
struct Held {
    amt: Vec<f64>,
    pool: Vec<f64>,
}

/// A node's row holds this many numbers per substance: what its reactions made less what they used, mol; the gas it
/// gives its air node (+) or takes, mol, its heat, J, and its lift paid by the air, J; then how long it sleeps, s.
const ROW: usize = 4;

/// What the nodes of one class change in a step, booked in one call after they all reacted: their reactions, and the
/// gas the ground nodes exchange with the air, one link flux per substance (+ up), with the air paying the lift. Only
/// what a node holds at the end of the call counts. Kept between steps (`World::batches`).
#[derive(Clone, Debug, Default)]
pub struct Batches {
    gas: Vec<Batch>,
    reactions: Vec<Transform>,
    /// Per air node, J
    lift: Vec<f64>,
}

impl Batches {
    /// Nothing booked yet, for a world of `na` air nodes and `ns` substances.
    fn clear(&mut self, na: usize, ns: usize) {
        if self.gas.len() != ns {
            self.gas = (0..ns as u16).map(|k| Batch::new([Class::Ground, Class::Air], &[k])).collect();
        }
        self.gas.iter_mut().for_each(Batch::clear);
        self.reactions.clear();
        self.lift.clear();
        self.lift.resize(na, 0.0);
    }
    /// Note the row of node `at` (`Row`): its reactions as one transform; a ground node's gas over the link to its air
    /// node, one flux per substance, and the lift its air node pays. Returns how long it sleeps, s.
    fn add(&mut self, w: &World, at: Node, row: &[f64]) -> u64 {
        let ns = w.chem.ns();
        let dmol: Vec<_> = row[..ns].iter().enumerate().filter(|e| *e.1 != 0.0).map(|(k, &d)| (k as u16, d)).collect();
        if !dmol.is_empty() {
            self.reactions.push(Transform { at, dmol });
        }
        let j = w.mesh.up(at.index, 0, AIR);
        for k in (0..ns).filter(|&k| at.class == Class::Ground && row[ns + k] != 0.0) {
            self.lift[j] += row[3 * ns + k];
            self.gas[k].add((at.index, j), k as u16, (row[ns + k], row[2 * ns + k]));
        }
        row[ROW * ns] as u64
    }
    /// Book it all on the world.
    fn book(&mut self, w: &mut World) {
        let mut lift = Batch::fallen(Class::Air, self.lift.iter().map(|q| -q));
        let fluxes: Vec<LinkFlux> = self.gas.iter_mut().filter(|b| !b.is_empty()).chain([&mut lift]).map(|b| b.flux()).collect();
        apply(w, &fluxes, &[], &self.reactions);
    }
}

/// React ground node `i` for `time` s with the gas over it, noting in its row what it changes (laws.md 2.2). The
/// reactor sees sensible heat: the fusion enthalpy stays apart (`FlowView::fused`). A node gives no more heat than it
/// holds: each gas it gives is cut by the same share.
fn react_ground(w: &World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, i: usize, time: f64, row: &mut [f64]) {
    let ns = w.chem.ns();
    let (chem, na) = (&w.chem, w.mesh.level(AIR).n());
    let (j, air_share) = (w.mesh.up(i, 0, AIR), w.mesh.share(i, AIR));
    h.pool.iter_mut().enumerate().for_each(|(k, p)| *p = w.store.amounts(Class::Air)[k * na + j] * air_share);
    // The time the liquid standing over it takes to renew its gas through its film, s: depth / `dissolve` (the air's
    // side is 100 times faster or more, left out).
    (rc.film, rc.moist) = (nodes.standing(i).0 / w.cfg.dissolve, nodes.moist(i, w.cfg.phys.field));
    let cap = |amt: &[f64]| nodes.skin[i] + (0..chem.ns()).map(|k| amt[k] * chem.subs[k].cp).sum::<f64>();
    let fused = nodes.fused(i, w.store.heat(Class::Ground)[i], cap(&h.amt), |k| h.amt[k] * chem.subs[k].lf);
    let mut e = w.store.heat(Class::Ground)[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], None, 0.0, time);
    let (made, out) = row.split_at_mut(ns);
    rc.made(chem, made);
    // What they gave to the gas around the node, or took from it, mol: the rest of what they made was in the node.
    let gas = |k: usize| if rc.pooled[k] { made[k] - rc.own[k] } else { 0.0 };
    // Gas given to the air takes the node's heat by heat capacity, shared with the node as it is after its reactions.
    let c_out = (0..chem.ns()).map(|k| gas(k).max(0.0) * chem.subs[k].cp).sum::<f64>();
    let per_c = e.max(0.0) / (cap(&h.amt) + c_out).max(1e-9);
    let pay = (0..chem.ns()).map(|k| gas(k).max(0.0) * (chem.subs[k].latent_gas() + chem.subs[k].cp * per_c)).sum::<f64>();
    let (paid, lift) = (share(e.max(0.0), pay), nodes.lift(i, j, w.cfg.gravity));
    for (k, m) in (0..ns).map(|k| (k, gas(k))).filter(|e| e.1 != 0.0) {
        let (s, m) = (&chem.subs[k], if m > 0.0 { m * paid } else { m });
        let heat = if m > 0.0 { m * (s.latent_gas() + s.cp * per_c) } else { m * (s.cp * nodes.ta[j] + s.latent_gas()) };
        (out[k], out[ns + k], out[2 * ns + k]) = (m, heat, m * s.molar_mass * lift);
    }
}

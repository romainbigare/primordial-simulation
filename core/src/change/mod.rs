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

use crate::chem::{Chemistry, Phase};
use crate::flow::FlowView;
use crate::ground::share;
use crate::store::{apply, Batch, Class, LinkFlux, Node, Transform};
use crate::{World, AIR};

/// A node sleeps until its reactions would use this share of their inputs (but for those on a `SPECK` of what it
/// holds) or warm it `CALM_K` kelvin: for so little, one long span is as good as many steps.
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;

/// Run Law 2 for the step of `dt` seconds at the temperatures of the last flow step: each ground node, then each air
/// node, that is due, for all the time since it last reacted, from the world as it was before its class reacted, booked
/// in one call (`Batches`). Then it sleeps (`Reactor::rest`, at most `sleep_days`).
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let (end, most) = (w.time + dt as u64, w.cfg.sleep_days as u64 * w.cfg.calendar().day);
    let (mut rc, ns) = (Reactor::new(&w.chem, &w.cfg), w.chem.ns());
    let mut h = Held { amt: vec![0.0; ns], pool: vec![0.0; ns], made: vec![0.0; ns] };
    for (class, len) in [(Class::Ground, w.mesh.cells().n()), (Class::Air, w.mesh.level(AIR).n())] {
        let ground = class == Class::Ground;
        w.sleep.watch(class, len, w.store.amounts(class), w.cfg.wake_share, w.time);
        // Before the law first runs, every node is due.
        if w.sleep.none(class) {
            for i in 0..len {
                w.sleep.sleep(class, i, w.time, w.time);
            }
        }
        // By index, so in the order of their cells.
        let due = w.sleep.due(class, w.time);
        let (mut b, nodes) = (std::mem::take(&mut w.batches), w.scratch.view());
        b.clear(w.mesh.level(AIR).n(), ns);
        let rests: Vec<u64> = (due.iter())
            .map(|&i| {
                let (span, amounts) = (end.saturating_sub(w.sleep.since(class, i)) as f64, w.store.amounts(class));
                for (k, a) in h.amt.iter_mut().enumerate() {
                    *a = amounts[k * len + i];
                }
                let all = h.amt.iter().sum::<f64>();
                if ground {
                    react_ground(w, &mut rc, &mut h, &nodes, i, span, &mut b);
                } else {
                    // An air node holds its gas itself: no liquid's film stands between them.
                    rc.film = 0.0;
                    rc.react(&w.chem, &mut h.amt, None, &mut { w.store.heat(Class::Air)[i] }, 0.0, None, 0.0, span);
                    b.transform(&w.chem, Node::new(Class::Air, i), &mut rc, &mut h.made);
                }
                (rc.rest(CALM, SPECK * all, CALM_K).max(0.0) as u64).min(most)
            })
            .collect();
        b.book(w);
        w.batches = b;
        for (&i, rest) in due.iter().zip(rests) {
            w.sleep.sleep(class, i, end, (end + rest).saturating_sub(dt as u64));
            w.sleep.keep(class, i, len, w.store.amounts(class));
        }
    }
}

/// A node's amounts and the gas around it (a ground node's share of its air node) as it reacts, and what its reactions
/// made less what they used of each substance, mol.
struct Held {
    amt: Vec<f64>,
    pool: Vec<f64>,
    made: Vec<f64>,
}

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
    /// The reactions of node `at` in the last `rc.react`: what they made less what they used of each substance, mol,
    /// into `made` (`Reactor::made`), noted as one transform.
    fn transform(&mut self, chem: &Chemistry, at: Node, rc: &mut Reactor, made: &mut [f64]) {
        rc.made(chem, made);
        let dmol: Vec<_> = made.iter().enumerate().filter(|e| *e.1 != 0.0).map(|(k, &d)| (k as u16, d)).collect();
        if !dmol.is_empty() {
            self.reactions.push(Transform { at, dmol });
        }
    }
    /// Book it all on the world.
    fn book(&mut self, w: &mut World) {
        let mut lift = Batch::fallen(Class::Air, self.lift.iter().map(|q| -q));
        let fluxes: Vec<LinkFlux> = self.gas.iter_mut().filter(|b| !b.is_empty()).chain([&mut lift]).map(|b| b.flux()).collect();
        apply(w, &fluxes, &[], &self.reactions);
    }
}

/// Time the liquid standing over ground node `i` takes to renew its gas through its film, s: depth / `dissolve` (the
/// air's side is 100 times faster or more, left out). 0: no liquid stands.
fn film(w: &World, i: usize, amt: &[f64], t: f64) -> f64 {
    let (chem, mut vol) = (&w.chem, [0.0; 3]);
    for k in (0..chem.ns()).filter(|&k| amt[k] != 0.0) {
        let p = chem.phase_in(k, t, |j| amt[j]);
        vol[p as usize] += if p != Phase::Gas { amt[k] * chem.molar_volume(k, p) } else { 0.0 };
    }
    let wet = vol[Phase::Liquid as usize];
    if wet > w.cfg.phys.porosity * vol[Phase::Solid as usize] { wet / (w.mesh.cells().area()[i] * w.cfg.dissolve) } else { 0.0 }
}

/// React ground node `i` for `time` s with the gas over it, noting in `b` what it changes (laws.md 2.2). The reactor
/// sees sensible heat: the fusion enthalpy stays apart (`FlowView::fused`). A node gives no more heat than it holds:
/// each gas it gives is cut by the same share.
fn react_ground(w: &World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, i: usize, time: f64, b: &mut Batches) {
    let (chem, na) = (&w.chem, w.mesh.level(AIR).n());
    let (t, j, air_share) = (nodes.tg[i], w.mesh.up(i, 0, AIR), w.mesh.share(i, AIR));
    for (k, p) in h.pool.iter_mut().enumerate() {
        *p = w.store.amounts(Class::Air)[k * na + j] * air_share;
    }
    rc.film = film(w, i, &h.amt, t);
    let cap = |amt: &[f64]| nodes.skin[i] + (0..chem.ns()).map(|k| amt[k] * chem.subs[k].cp).sum::<f64>();
    let fused = nodes.fused(i, w.store.heat(Class::Ground)[i], cap(&h.amt), |k| h.amt[k] * chem.subs[k].lf);
    let mut e = w.store.heat(Class::Ground)[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], None, 0.0, time);
    b.transform(chem, Node::new(Class::Ground, i), rc, &mut h.made);
    // What they gave to the gas around the node, or took from it, mol: the rest of what they made was in the node.
    let gas = |k: usize| if rc.pooled[k] { h.made[k] - rc.own[k] } else { 0.0 };
    // Gas given to the air takes the node's heat by heat capacity, shared with the node as it is after its reactions.
    let c_out = (0..chem.ns()).map(|k| gas(k).max(0.0) * chem.subs[k].cp).sum::<f64>();
    let per_c = e.max(0.0) / (cap(&h.amt) + c_out).max(1e-9);
    let pay = (0..chem.ns()).map(|k| gas(k).max(0.0) * (chem.subs[k].latent_gas() + chem.subs[k].cp * per_c)).sum::<f64>();
    let (paid, lift) = (share(e.max(0.0), pay), nodes.lift(i, j, w.cfg.gravity));
    for (k, m) in (0..chem.ns()).map(|k| (k, gas(k))).filter(|e| e.1 != 0.0) {
        let (s, m) = (&chem.subs[k], if m > 0.0 { m * paid } else { m });
        b.lift[j] += m * s.molar_mass * lift;
        let heat = if m > 0.0 { m * (s.latent_gas() + s.cp * per_c) } else { m * (s.cp * nodes.ta[j] + s.latent_gas()) };
        b.gas[k].add((i, j), k as u16, (m, heat));
    }
}

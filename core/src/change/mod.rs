//! Law 2: Change. Substances react inside nodes, faster when hot.
//!
//! Mass action: a reaction runs at catalyst · k · exp(−Ea/T) · `e` · the product, over its inputs,
//! of their share of their phase to their order (`Reaction::order`; a solid is a phase of its own,
//! share 1), in reaction units a second. `e` is the contact of its phases: what `lim` would be
//! were each input all of its phase (the least, over the inputs, of its phase's amount per `nu`).
//! So a reaction within one phase goes with that phase, and one between a solid and the air with
//! the solid, as a surface would. In a step it goes as far as `lim × (1 − exp(−rate / lim ×
//! time))`, `lim` the amount of its scarcest input per `nu`: never past it, exact for a first-order
//! decay. The rate has no jump when another input becomes the scarcest, and is first order in a
//! trace input (a trace gas in plenty of the other goes at the same rate per unit whatever its
//! amount). A body's catalysts work only below the temperature where they lose their shape
//! (`denature`). A ground node with liquid standing over it reacts with the gas dissolved in that
//! liquid, which the liquid's film renews (`Reactor::film`).
//!
//! A ground or an air node reacts when it is due (`clock::Sleep`), for all the time since it last did: the kernel
//! integrates the rate law over a span. Then it sleeps until its own rates would change it by more than a share `CALM`
//! (`Reactor::rest`), at most `Config::sleep_days`; a node where a reaction goes that far within a step reacts every
//! step. A call of `store::apply` that brings it more than `Config::wake_share` of an amount wakes it. What reaches it
//! outside `apply` (what bodies take and give) and the change of its temperature do not wake it: `sleep_days` bounds
//! how long it reacts without them.
//!
//! `kernel`: the rate law and the reactor. `balance`: reactions that undo each other. `gas`: the gas a body or a part
//! exchanged with the air around it. Here: the step over the ground and air nodes, booked by `store::apply`.

mod balance;
mod gas;
mod kernel;

pub use balance::reversible;
pub(crate) use gas::{send_gas, GasMove};
pub use kernel::Reactor;
pub(crate) use kernel::{denature, frac, pace, scarce};

use crate::chem::{Chemistry, Phase};
use crate::flow::FlowView;
use crate::store::{apply, Batch, Class, LinkFlux, NodeId, Transform};
use crate::World;

/// A node sleeps until its reactions would go further than this share of their inputs (but for reactions on a `SPECK`
/// of what it holds), or warm it by more than `CALM_K` kelvin: for so little, one long span is as good as many steps.
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;

/// Run Law 2 for the step of `dt` seconds from the world's time, at the temperatures of the last flow step: each ground
/// node (loose amounts, gas from the air above), then each air node, that is due, in the order of their cells, for all
/// the time since it last reacted to the end of this step, each from the world as it was before the nodes of its class
/// reacted; booked by `store::apply` (`Booked`). Then it sleeps until the last step that starts before its reactions may have to run again
/// (`Reactor::rest`, at most `sleep_days`): the next step, for a rest within one.
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let (end, most) = (w.time + dt as u64, w.cfg.sleep_days as u64 * w.cfg.calendar().day);
    let (mut rc, ns) = (Reactor::new(&w.chem, &w.cfg), w.chem.ns());
    let mut h = Held { amt: vec![0.0; ns], pool: vec![0.0; ns], made: vec![0.0; ns] };
    for (class, len) in [(Class::Ground, w.grid.n()), (Class::Air, w.grid.na())] {
        // Before the law first runs, every node is due.
        if w.sleep.none(class) {
            for i in 0..len {
                w.sleep.sleep(class, w.table.id(class, i), w.time, w.time);
            }
        }
        let mut due: Vec<_> = w.sleep.due(class, w.time).into_iter().map(|id| (w.table.place(id).1, id)).collect();
        due.sort_unstable();
        let (mut b, nodes) = (Booked::new(w.grid.na(), ns), w.scratch.view());
        let rests: Vec<u64> = (due.iter())
            .map(|&(i, id)| {
                let (span, amounts) = (end.saturating_sub(w.sleep.since(id)) as f64, if class == Class::Ground { &w.ground } else { &w.air });
                for (k, a) in h.amt.iter_mut().enumerate() {
                    *a = amounts[k * len + i];
                }
                let all = h.amt.iter().sum::<f64>();
                if class == Class::Ground {
                    react_ground(w, &mut rc, &mut h, &nodes, (i, id), span, &mut b);
                } else {
                    // An air node holds its gas itself: no liquid's film stands between them.
                    rc.film = 0.0;
                    rc.react(&w.chem, &mut h.amt, None, &mut { w.e_air[i] }, 0.0, None, 0.0, span);
                    b.transform(&w.chem, id, &mut rc, &mut h.made);
                }
                (rc.rest(CALM, SPECK * all, CALM_K).max(0.0) as u64).min(most)
            })
            .collect();
        b.book(w);
        for (&(_, id), rest) in due.iter().zip(rests) {
            w.sleep.sleep(class, id, end, (end + rest).saturating_sub(dt as u64));
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

/// What the nodes of one class that react in a step change, booked by `store::apply` after they all reacted: the gas
/// the ground nodes take from the air, their reactions, then the gas they give it, each in its own call, as each takes
/// what the one before brought (`apply` scales a change by what a node holds before the call). The gas crosses the
/// links from each ground node to its air node, + up into the air: the air receives the heat that leaves the ground
/// less the latent heat it holds of the gas (`store::held`). Each air node pays the lift of the gas that goes up into
/// it into the world's potential energy (as `Sky::cross` books it). The gas is a flux for each substance (indexed by
/// it): a node exchanges a few of them, so no link carries the others.
struct Booked {
    taken: Vec<Batch>,
    reactions: Vec<Transform>,
    given: Vec<Batch>,
    /// Per air node, J
    lift: Vec<f64>,
}

impl Booked {
    /// Nothing booked yet, for a world of `na` air nodes and `ns` substances.
    fn new(na: usize, ns: usize) -> Booked {
        let gas = || (0..ns as u16).map(|k| Batch::new([Class::Ground, Class::Air], &[k])).collect();
        Booked { taken: gas(), reactions: Vec::new(), given: gas(), lift: vec![0.0; na] }
    }
    /// The reactions of node `at` in the last `rc.react`: what they made less what they used of each substance, mol,
    /// into `made` (`Reactor::made`), noted as one transform.
    fn transform(&mut self, chem: &Chemistry, at: NodeId, rc: &mut Reactor, made: &mut [f64]) {
        rc.made(chem, made);
        let dmol: Vec<(u16, f64)> = made.iter().enumerate().filter(|e| *e.1 != 0.0).map(|(k, &d)| (k as u16, d)).collect();
        if !dmol.is_empty() {
            self.reactions.push(Transform { at, dmol });
        }
    }
    /// Book it all on the world.
    fn book(mut self, w: &mut World) {
        let mut lift = Batch::falls(Class::Air);
        self.lift.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -q));
        fn gas(v: &mut [Batch]) -> Vec<LinkFlux<'_>> {
            v.iter_mut().filter(|b| !b.is_empty()).map(|b| b.flux()).collect()
        }
        apply(w, &gas(&mut self.taken), &[], &[]);
        if !self.reactions.is_empty() {
            apply(w, &[], &[], &self.reactions);
        }
        let given = [gas(&mut self.given), vec![lift.flux()]].concat();
        apply(w, &given, &[], &[]);
    }
}

/// Time the liquid standing over ground amounts `amt` at `t` (more than the pores of its loose ground hold) takes to
/// renew its gas through its film over its whole depth, s (`Reactor::film`; Flow's law, `air::dissolve`: depth /
/// `dissolve`; the air's side of the film is faster by 100 times or more for every gas and left out). 0: no liquid stands.
fn film(w: &World, amt: &[f64], t: f64) -> f64 {
    let (chem, mut vol) = (&w.chem, [0.0; 3]);
    for k in (0..chem.ns()).filter(|&k| amt[k] != 0.0) {
        let p = chem.phase_in(k, t, |j| amt[j]);
        vol[p as usize] += if p != Phase::Gas { amt[k] * chem.molar_volume(k, p) } else { 0.0 };
    }
    let wet = vol[Phase::Liquid as usize];
    if wet > w.cfg.phys.porosity * vol[Phase::Solid as usize] { wet / (w.grid.area() * w.cfg.dissolve) } else { 0.0 }
}

/// React ground node `i` (its amounts in `h.amt`) for `time` s with the gas over it, and note in `b` what it changes.
/// The reactor sees sensible heat: the fusion enthalpy the node holds by Flow's solve of its plateaus stays apart
/// (`FlowView::fused`), so a node on a plateau reacts at its melting point. Gas taken from the air brings its heat at the
/// air's temperature and gives back its fusion and vaporisation enthalpy (`store::held`); gas given to it takes its
/// share of the node's heat by heat capacity and that enthalpy, so gas the node cannot pay for stays in it
/// (`store::apply` scales the flux).
fn react_ground(w: &World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, (i, id): (usize, NodeId), time: f64, b: &mut Booked) {
    let (chem, na) = (&w.chem, w.grid.na());
    let (t, j, air_share) = (nodes.tg[i], nodes.air_of[i] as usize, w.grid.air_share(i));
    for (k, p) in h.pool.iter_mut().enumerate() {
        *p = w.air[k * na + j] * air_share;
    }
    rc.film = film(w, &h.amt, t);
    let cap = |amt: &[f64]| nodes.skin[i] + (0..chem.ns()).map(|k| amt[k] * chem.subs[k].cp).sum::<f64>();
    let fused = nodes.fused(i, w.e_ground[i], cap(&h.amt), |k| h.amt[k] * chem.subs[k].lf);
    let mut e = w.e_ground[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], None, 0.0, time);
    b.transform(chem, id, rc, &mut h.made);
    // What they gave to the gas around the node, or took from it, mol: the rest of what they made was in the node.
    let gas = |k: usize| if rc.pooled[k] { h.made[k] - rc.own[k] } else { 0.0 };
    // Gas given to the air takes the node's heat by heat capacity, shared with the node as it is after its reactions.
    let c_out = (0..chem.ns()).map(|k| gas(k).max(0.0) * chem.subs[k].cp).sum::<f64>();
    let per_c = e.max(0.0) / (cap(&h.amt) + c_out).max(1e-9);
    let lift = nodes.lift(i, w.cfg.gravity);
    for (k, m) in (0..chem.ns()).map(|k| (k, gas(k))).filter(|e| e.1 != 0.0) {
        let s = &chem.subs[k];
        b.lift[j] += m * s.molar_mass * lift;
        if m > 0.0 {
            b.given[k].add((i, j), k as u16, (m, m * (s.latent_gas() + s.cp * per_c)));
        } else {
            b.taken[k].add((i, j), k as u16, (m, m * (s.cp * nodes.ta[j] + s.latent_gas())));
        }
    }
}

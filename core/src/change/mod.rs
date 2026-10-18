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
//! step. A move booked by `store::apply` that brings it more than `Config::wake_share` of an amount wakes it. What
//! reaches it outside `apply` (radiation and the exchange with the air, Flow's; what bodies take and give) and the
//! change of its temperature do not wake it: `sleep_days` bounds how long it reacts without them.
//!
//! `kernel`: the rate law and the reactor. `balance`: reactions that undo each other. `gas`: the gas a reactor
//! exchanged with the air around it. Here: the step over the ground and air nodes.

mod balance;
mod gas;
mod kernel;

pub use balance::reversible;
pub(crate) use gas::{send_gas, GasMove};
pub use kernel::Reactor;
pub(crate) use kernel::{denature, frac, pace, scarce};

use crate::chem::Phase;
use crate::flow::{FlowView, Sky};
use crate::ground::{draw, share};
use crate::store::Class;
use crate::World;

/// A node sleeps until its reactions would go further than this share of their inputs (but for reactions on a `SPECK`
/// of what it holds), or warm it by more than `CALM_K` kelvin: for so little, one long span is as good as many steps.
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;

/// Run Law 2 for the step of `dt` seconds from the world's time, at the temperatures of the last flow step: each ground
/// node (loose amounts, gas from the air above), then each air node, that is due, in the order of their cells, for all
/// the time since it last reacted to the end of this step. Then it sleeps until the last step that starts before its
/// reactions may have to run again (`Reactor::rest`, at most `sleep_days`): the next step, for a rest within one.
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let s = std::mem::take(&mut w.scratch);
    let nodes = s.view();
    let (end, most) = (w.time + dt as u64, w.cfg.sleep_days as u64 * w.cfg.calendar().day);
    let (mut rc, ns) = (Reactor::new(&w.chem, &w.cfg), w.chem.ns());
    let mut h = Held { amt: vec![0.0; ns], pool: vec![0.0; ns], pool0: vec![0.0; ns] };
    for (class, len) in [(Class::Ground, w.grid.n()), (Class::Air, w.grid.na())] {
        // Before the law first runs, every node is due.
        if w.sleep.none(class) {
            for i in 0..len {
                w.sleep.sleep(class, w.table.id(class, i), w.time, w.time);
            }
        }
        let mut due: Vec<_> = w.sleep.due(class, w.time).into_iter().map(|id| (w.table.place(id).1, id)).collect();
        due.sort_unstable();
        for (i, id) in due {
            let (span, amounts) = (end.saturating_sub(w.sleep.since(id)) as f64, if class == Class::Ground { &w.ground } else { &w.air });
            for (k, a) in h.amt.iter_mut().enumerate() {
                *a = amounts[k * len + i];
            }
            let all = h.amt.iter().sum::<f64>();
            if class == Class::Ground {
                react_ground(w, &mut rc, &mut h, &nodes, i, span);
            } else {
                react_air(w, &mut rc, &mut h.amt, i, span);
            }
            let rest = (rc.rest(CALM, SPECK * all, CALM_K).max(0.0) as u64).min(most);
            w.sleep.sleep(class, id, end, (end + rest).saturating_sub(dt as u64));
        }
    }
    w.scratch = s;
}

/// A ground node's amounts and the gas of the air over it (its share of its air node) before and after it reacts, mol.
struct Held {
    amt: Vec<f64>,
    pool: Vec<f64>,
    pool0: Vec<f64>,
}

/// Time the liquid standing over ground amounts `amt` at `t` (more than the pores of its loose ground hold) takes to
/// renew its gas through its film over its whole depth, s (`Reactor::film`; Flow's law, `air::dissolve`: depth /
/// `dissolve`; the air's side of the film is faster by 100 times or more for every gas and left out). 0: no liquid stands.
fn film(w: &World, amt: &[f64], t: f64) -> f64 {
    let chem = &w.chem;
    let vol = |p: Phase| (0..chem.ns()).filter(|&k| chem.phase_in(k, t, |j| amt[j]) == p).map(|k| amt[k] * chem.molar_volume(k, p)).sum::<f64>();
    let wet = vol(Phase::Liquid);
    if wet > w.cfg.phys.porosity * vol(Phase::Solid) { wet / (w.grid.area() * w.cfg.dissolve) } else { 0.0 }
}

/// React ground node `i` (its amounts in `h.amt`) for `time` s with the gas over it, and settle the gas with its air node.
/// The reactor sees sensible heat: the fusion enthalpy the node holds by Flow's solve of its plateaus stays apart
/// (`FlowView::fused`), so a node on a plateau reacts at its melting point. Gas sent to the air takes its fusion
/// and vaporisation enthalpy from the ground (gas taken back returns it); gas the ground cannot pay for stays behind.
/// Gas taken from the air brings its heat at the air's temperature; gas sent leaves with its share of the node's heat by
/// heat capacity.
fn react_ground(w: &mut World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, i: usize, time: f64) {
    let (chem, ns, n, na) = (&w.chem, w.chem.ns(), w.grid.n(), w.grid.na());
    let (t, j, air_share) = (nodes.tg[i], nodes.air_of[i] as usize, w.grid.air_share(i));
    for (k, p0) in h.pool0.iter_mut().enumerate() {
        *p0 = w.air[k * na + j] * air_share;
    }
    h.pool.copy_from_slice(&h.pool0);
    rc.film = film(w, &h.amt, t);
    let cap = |amt: &[f64]| nodes.skin[i] + (0..ns).map(|k| amt[k] * chem.subs[k].cp).sum::<f64>();
    let fused = nodes.fused(i, w.e_ground[i], cap(&h.amt), |k| h.amt[k] * chem.subs[k].lf);
    let mut e = w.e_ground[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], None, 0.0, time);
    let gas = GasMove::of(chem, &h.pool, &h.pool0, nodes.ta[j]);
    let c_r = cap(&h.amt);
    let q_out = e.max(0.0) * gas.c_out / (c_r + gas.c_out).max(1e-9);
    w.e_ground[i] = e + fused + gas.back + gas.q_in - q_out;
    w.e_air[j] += q_out - gas.q_in;
    let cap = w.heat_cap_ground(i);
    let paid = share(draw(&mut w.e_ground[i], &mut w.counts, cap, gas.pay), gas.pay);
    // The heat of the gas that stays behind stays with it.
    (w.e_ground[i], w.e_air[j]) = (w.e_ground[i] + q_out * (1.0 - paid), w.e_air[j] - q_out * (1.0 - paid));
    let (ground, amt, lift) = (&mut w.ground, &h.amt, nodes.lift(i, w.cfg.gravity));
    let mut sky = Sky { chem: &w.chem, air: &mut w.air, e_air: &mut w.e_air, potential: &mut w.potential };
    send_gas(&h.pool, &h.pool0, paid, &mut sky, (j, lift), |k, kept| ground[k * n + i] = amt[k] + kept);
}

/// React air node `j` (its amounts in `amt`) for `time` s. Gas it makes or takes changes its latent heat held in the
/// air.
fn react_air(w: &mut World, rc: &mut Reactor, amt: &mut [f64], j: usize, time: f64) {
    let (chem, na) = (&w.chem, w.grid.na());
    let mut e = w.e_air[j];
    rc.react(chem, amt, None, &mut e, 0.0, None, 0.0, time);
    for (k, &a) in amt.iter().enumerate() {
        e -= (a - w.air[k * na + j]) * chem.subs[k].latent_gas();
        w.air[k * na + j] = a;
    }
    w.e_air[j] = e;
}

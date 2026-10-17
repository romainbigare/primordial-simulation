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
//! `kernel`: the rate law and the reactor. `balance`: reactions that undo each other. `screen`: which nodes react now.
//! `gas`: the gas a reactor exchanged with the air around it.
//! Here: the step over the ground and air nodes.

mod balance;
mod gas;
mod kernel;
mod screen;

pub use balance::reversible;
pub(crate) use gas::{send_gas, GasMove};
pub use kernel::Reactor;
pub(crate) use kernel::{denature, frac, pace, scarce, LIGHT_YIELD};

use crate::chem::Phase;
use crate::flow::FlowView;
use crate::ground::{draw, share};
use crate::World;
use screen::{bit, Plan};

/// A node that is neither frozen nor hot reacts every `LAZY` ticks with `LAZY` times the time:
/// reactions at ordinary temperatures are slow, so one long step is as good as many short ones.
/// A ground node waits `SLOW` ticks if over that time its reactions would go no further than a
/// share `CALM` of their inputs (but for reactions on a `SPECK` of what it holds) and warm it by
/// no more than `CALM_K` kelvin.
const LAZY: u64 = 40;
const SLOW: u64 = 8 * LAZY;
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;
/// Ground nodes take their turns in blocks of this many nodes next to each other in memory.
const BLOCK: usize = 64;
/// A reaction goes far within a step when its rate times the step reaches this: a node where one may react every step.
const FAR: f64 = 0.05;
/// A substance making up less than this share of a ground node's loose amounts (or of its phase,
/// for a gas) is a trace: reactions on traces alone never make a node react every step, and
/// lumping a reaction that runs out of a trace over `LAZY` steps is exact for a first-order decay.
const TRACE: f64 = 1e-6;

/// Per ground node: the tick it last reacted at its turn, the tick of its next turn, and the steps it reacted on its own
/// since. Blocks take turns: block `b` first reacts when `(b + tick) % LAZY == 0`.
fn turns(n: usize, tick: u32) -> Vec<[u32; 3]> {
    (0..n)
        .map(|i| tick + ((2 * LAZY - (i / BLOCK) as u64 % LAZY - tick as u64 % LAZY) % LAZY) as u32)
        .map(|due| [due.saturating_sub(LAZY as u32), due, 0])
        .collect()
}

/// Run Law 2 on ground nodes (loose amounts, gas from the air above) and on air nodes, at the
/// temperatures of the last flow step. A ground node whose reactions may go far within one step
/// (`screen`) reacts every step; at its turn, every `LAZY` (or `SLOW`) steps by block, it reacts
/// for all the time since it last did, unless all its reactions are frozen. Air nodes go far by
/// their exact rates, else react at their turn.
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let mut s = std::mem::take(&mut w.scratch);
    let n = w.grid.n();
    let tick = (w.day / w.cfg.phys_days as u64) as u32;
    if s.slow.len() != n {
        s.slow = turns(n, tick);
    }
    let mut slow = std::mem::take(&mut s.slow);
    let nodes = s.view();
    let plan = screen::screen(w, &nodes, dt);
    ground(w, &nodes, &plan, &mut slow, tick, dt);
    air(w, &nodes, &plan, tick, dt);
    s.slow = slow;
    w.scratch = s;
}

/// A ground node's amounts and the gas of the air over it (its share of its air node) before and after it reacts, mol.
struct Held {
    amt: Vec<f64>,
    pool: Vec<f64>,
    pool0: Vec<f64>,
}

/// The ground nodes' turn: each that may go far (`Plan::ground`) reacts for this step (at least; a first turn has no
/// earlier turn to count from); at its turn, for all the time since its last turn it has not reacted for.
fn ground(w: &mut World, nodes: &FlowView, plan: &Plan, slow: &mut [[u32; 3]], tick: u32, dt: f64) {
    let (ns, n) = (w.chem.ns(), w.grid.n());
    let mut rc = Reactor::new(&w.chem);
    let mut h = Held { amt: vec![0.0; ns], pool: vec![0.0; ns], pool0: vec![0.0; ns] };
    for (i, turn) in slow.iter_mut().enumerate() {
        let (due, fast) = (turn[1] <= tick, plan.ground[i]);
        if !due && !fast {
            continue;
        }
        let (t, j) = (nodes.tg[i], nodes.air_of[i] as usize);
        for (k, a) in h.amt.iter_mut().enumerate() {
            *a = w.ground[k * n + i];
        }
        let has = || (0..ns).filter(|&k| h.amt[k] > 0.0).fold(plan.in_air[j], |m, k| m | bit(k));
        let [last, due_at, fasts] = *turn;
        let ticks = if due && !plan.cold(t, has()) { (tick - last).saturating_sub(fasts).max(fast as u32) } else { fast as u32 };
        let mut next = LAZY;
        if ticks > 0 {
            let all = h.amt.iter().sum::<f64>();
            react_ground(w, &mut rc, &mut h, nodes, i, ticks as f64 * dt);
            if rc.calm(SLOW as f64 * dt, CALM, SPECK * all, CALM_K) {
                next = SLOW;
            }
        }
        *turn = if due { [tick, tick + next as u32, 0] } else { [last, due_at, fasts + 1] };
    }
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
/// The reactor sees sensible heat: molten amounts hold their fusion enthalpy apart. Gas sent to the air takes its fusion
/// and vaporisation enthalpy from the ground (gas taken back returns it); gas the ground cannot pay for stays behind.
/// Gas taken from the air brings its heat at the air's temperature; gas sent leaves with its share of the node's heat by
/// heat capacity.
fn react_ground(w: &mut World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, i: usize, time: f64) {
    let (chem, ns, n, na) = (&w.chem, w.chem.ns(), w.grid.n(), w.grid.na());
    let (t, j, air_share) = (nodes.tg[i], nodes.air_of[i] as usize, w.grid.air_share());
    for (k, p0) in h.pool0.iter_mut().enumerate() {
        *p0 = w.air[k * na + j] * air_share;
    }
    h.pool.copy_from_slice(&h.pool0);
    rc.film = film(w, &h.amt, t);
    let fused: f64 = (0..ns).map(|k| h.amt[k] * chem.fused_heat(k, t, |j| h.amt[j])).sum();
    let mut e = w.e_ground[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], None, 0.0, time);
    let gas = GasMove::of(chem, &h.pool, &h.pool0, nodes.ta[j]);
    let c_r = nodes.skin[i] + (0..ns).map(|k| h.amt[k] * chem.subs[k].cp).sum::<f64>();
    let q_out = e.max(0.0) * gas.c_out / (c_r + gas.c_out).max(1e-9);
    w.e_ground[i] = e + fused + gas.back + gas.q_in - q_out;
    w.e_air[j] += q_out - gas.q_in;
    let cap = w.heat_cap_ground(i);
    let paid = share(draw(&mut w.e_ground[i], &mut w.counts, cap, gas.pay), gas.pay);
    // The heat of the gas that stays behind stays with it.
    (w.e_ground[i], w.e_air[j]) = (w.e_ground[i] + q_out * (1.0 - paid), w.e_air[j] - q_out * (1.0 - paid));
    let (ground, amt) = (&mut w.ground, &h.amt);
    send_gas(&h.pool, &h.pool0, paid, &mut w.air, na, j, |k, kept| ground[k * n + i] = amt[k] + kept);
}

/// The air nodes' turn: each on its own, so from the amounts before (in parallel with the `threads` feature) into a row
/// [reacted, heat, amounts], then applied in order. A node that may go far (`Plan::air`) and does by its exact rates
/// reacts for this step; else at its turn, every `LAZY` steps, for `LAZY` steps, unless all its reactions are frozen.
/// Gas it makes or takes changes its latent heat held in the air.
fn air(w: &mut World, nodes: &FlowView, plan: &Plan, tick: u32, dt: f64) {
    let (chem, ns, na) = (&w.chem, w.chem.ns(), w.grid.na());
    let turn = ((LAZY - tick as u64 % LAZY) % LAZY) as usize;
    let mut rows = vec![0.0; na * (ns + 2)];
    let (air_now, e_now) = (&w.air, &w.e_air);
    crate::par::rows(&mut rows, ns + 2, || None, |st: &mut Option<(Reactor, Vec<f64>)>, j, row| {
        let t = nodes.ta[j];
        let lazy = j % LAZY as usize == turn && !plan.cold(t, plan.in_air[j]);
        if !plan.air[j] && !lazy {
            return;
        }
        let (rc, amt) = st.get_or_insert_with(|| (Reactor::new(chem), vec![0.0; ns]));
        for k in 0..ns {
            amt[k] = air_now[k * na + j];
        }
        let time = if plan.air[j] && rc.fastest(chem, amt, t) * dt >= FAR {
            dt
        } else if lazy {
            dt * LAZY as f64
        } else {
            return;
        };
        let mut e = e_now[j];
        rc.react(chem, amt, None, &mut e, 0.0, None, 0.0, time);
        for k in 0..ns {
            e -= (amt[k] - air_now[k * na + j]) * chem.subs[k].latent_gas();
        }
        row[0] = 1.0;
        row[1] = e;
        row[2..].copy_from_slice(amt);
    });
    for (j, row) in rows.chunks(ns + 2).enumerate().filter(|(_, r)| r[0] != 0.0) {
        w.e_air[j] = row[1];
        for k in 0..ns {
            w.air[k * na + j] = row[2 + k];
        }
    }
}

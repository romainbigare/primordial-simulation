//! Law 1: Flow. Stuff moves along links from high push to low push, with loss (laws.md 1). `scratch`: the law's
//! working memory (`spare`: room its passes keep); `land`: the state and surface of ground nodes; `liquid` (its pass,
//! `runoff`, and the potential energy the pass carries, `falls`), `drain`, `deep`, `basin`, `creep`; `air`: radiation,
//! exchange and wind in one step; `potential` with `stack`: the potential energy of the matter and its booking.

pub(crate) mod air;
mod creep;
mod curve;
mod deep;
mod drain;
mod bodies;
mod inside;
mod land;
mod level;
mod liquid;
mod parts;
mod potential;
mod runoff;
mod scratch;
mod shore;
mod spare;
mod stack;
mod basin;
pub mod build;
mod channel;
pub(crate) use air::air_temp;
pub use air::{band_depths, column, declination, insolation, on_slope, scale_height, sun_by_latitude, Heat, Planet, DOWN, UP};
pub use potential::total as potential;
pub use basin::Pool;
pub use channel::Channel;
pub(crate) use level::level_of;
pub use inside::{land_shares, levels, lowest, means, shores};
pub use shore::{shore, Feet, Shore};

pub(crate) use potential::air_head;
pub(crate) use stack::gain;
pub use scratch::Scratch;
pub(crate) use scratch::FlowView;

use crate::{store::Class, AIR, World};
use land::ground_state;
use scratch::tables;

/// Each part of the law (`state`, `air`, `liquid`, `creep`, `refresh`) is a pass of the month (`schedule`). This
/// one: the state of ground and air nodes from the world now, which the other laws read (in full at each month's start,
/// the air's columns too, `air::state`, so a world resumed there goes on as an unbroken one; the survey builds it in
/// full then, `built_now`).
pub fn state(w: &mut World, full: bool) {
    with(w, true, |w, s| {
        ground_now(w, s, full);
        if full {
            air::state(w, s);
        } else {
            air::sums(w, s);
            (0..w.mesh.level(AIR).n()).for_each(|j| s.ta[j] = air_temp(w.store.heat(Class::Air)[j], s.capa[j]));
        }
    });
}

/// The state of ground nodes (`ground_state`): only what changed since it was last rebuilt, or every node with `full`
/// (each month's start, after the month's refresh changed what is present) or when the caches are new. A full build
/// notes the time it was made (`built_now`).
fn ground_now(w: &World, s: &mut Scratch, full: bool) {
    let full = full || s.heat.len() != w.mesh.level(AIR).n();
    ground_state(w, s, (full, true));
    s.built_at = Some(w.time);
    if full {
        s.full_at = Some(w.time);
    }
}

/// Was the state of ground nodes built in full at the world's time now? Then the `state` process has nothing to add.
pub(crate) fn built_now(w: &World) -> bool {
    w.scratch.full_at == Some(w.time)
}

/// The air's step for `dt` seconds (`air::step`), booked whole, from the state of ground and air nodes (brought up to
/// date unless the `state` process just did; the heat of the aside nodes that the cover does not claim, which goes
/// back to the ground first, `cover::release`, then warms it from the next state); then, from the state the step's
/// heat leaves (what it melted and froze), the liquid over field capacity sinks into the bed's store (`drain::sink`),
/// as the rain the step brought would otherwise wait in the pores.
pub fn air(w: &mut World, dt: f64) {
    with(w, true, |w, s| {
        crate::cover::release(w);
        if s.built_at != Some(w.time) {
            ground_now(w, s, false);
        }
        air::state(w, s);
        air::step(w, s, dt);
        ground_now(w, s, false);
        drain::sink(w, s, dt);
    });
}

/// Liquid with its drainage and its work on the bed for `dt` seconds (`liquid::liquid`).
pub fn liquid(w: &mut World, dt: f64) {
    with(w, true, |w, s| liquid::liquid(w, s, dt));
}

/// The liquid settled over the ground as it stands now, in no time (`liquid::settle`).
pub fn settle(w: &mut World) {
    with(w, true, liquid::settle);
}

/// `m3` m³ of liquid an edit put in cell `i`'s ground node poured into its part of the body of standing liquid of pond
/// `pond` (`Samples::ponds`): that part holds it (`State::held`), so the runoff's pass keeps it in the body
/// (`runoff::surface`) and levels it there. Whether the cell holds a part of that pond.
pub fn pour(w: &mut World, i: usize, pond: u32, m3: f64) -> bool {
    let mut done = false;
    with(w, false, |_, s| {
        let h = &mut s.state.held[i];
        if let Some(q) = h.ponds.iter().position(|&p| p == pond) {
            (h.m3[q], done) = (h.m3[q] + m3, true);
        }
    });
    done
}

/// Heat conducted sideways through the ground (`land::conduct`), then creep (`creep::creep`), for `dt` seconds.
pub fn creep(w: &mut World, dt: f64) {
    with(w, true, |w, s| {
        land::conduct(w, s, dt);
        creep::creep(w, s, dt);
    });
}

/// The pores of some ground nodes change (`change`, `soil::month`, which names them), their lifts and falls booked: the
/// liquid they hold and let stand rises or falls with them, and its potential energy goes to or from its ground's heat
/// (`potential::Lifts`); the state of those nodes is found anew from them (`land::cells_state`), or of every node where
/// it was not built at the world's time now.
pub fn new_pores(w: &mut World, change: impl FnOnce(&mut World) -> Vec<u32>) {
    with(w, true, |w, s| {
        let cells = change(w);
        if s.built_at == Some(w.time) {
            land::cells_state(w, s, &cells, true);
        } else if !cells.is_empty() {
            ground_now(w, s, true);
        }
    });
}

/// The slow ground and the substances present are rechecked (`scratch::refresh`).
pub fn refresh(w: &mut World) {
    with(w, false, |w, s| scratch::refresh(w, s));
}

/// Build the working memory of the law where it is missing (`with`).
pub(crate) fn ensure(w: &mut World) {
    with(w, false, |_, _| {});
}

/// Run `f` on the world and the law's working memory, built first where it is missing (a new or loaded world): its
/// tables, a new world's state, and its caches from the world now, as at a liquid step. Where `f` moves matter, or the
/// caches of a world with a state are built (its phases may change), its lifts and falls are booked (`potential::Lifts`):
/// at its end, or at the end of the parts that follow each other (`chain`).
fn with(w: &mut World, moving: bool, f: impl FnOnce(&mut World, &mut Scratch)) {
    let n = w.mesh.cells().n();
    let mut s = std::mem::take(&mut w.scratch);
    if s.near.len() != n {
        tables(w, &mut s);
    }
    if curve::stale(w, &s) {
        curve::build(w, &mut s);
    }
    if s.bodies.of.len() != n {
        s.bodies = bodies::Bodies::of(w, &s.curves);
    }
    let fresh = s.state.tg.len() != n;
    if fresh {
        scratch::refresh(w, &mut s);
        // A new world's ground nodes, before its first step: no plateau, no clouds.
        let st = &mut s.state;
        (st.tg, st.pk, st.phi, st.cloud, st.parts, st.dissolved) = (vec![0.0; n], vec![land::NONE; n], vec![0.0; n], vec![[0.0; 2]; w.mesh.level(AIR).n()], vec![None; n], vec![0.0; n]);
    }
    let build = s.cap.len() != n;
    if s.lifts.is_none() && !fresh && (moving || build) {
        let (known, room) = (s.known.take().filter(|_| s.chained).map(|k| k.1), std::mem::take(&mut s.moved));
        s.lifts = Some(potential::Lifts::start(w, &s, known, room));
    }
    if build {
        scratch::room(w, &mut s);
        ground_state(w, &mut s, (true, false));
        air::state(w, &mut s);
    }
    f(w, &mut s);
    if !s.chained {
        book(w, &mut s);
    }
    // (Where no booking found them yet: a new or loaded world.)
    if s.lift_at[0].len() != n && s.state.tg.len() == n {
        s.lift_at = potential::heads(w, &s);
    }
    w.scratch = s;
}

/// Book the lifts and falls of the parts of Flow under way (`potential::Lifts`), if any, and keep the heads of the
/// ground cells and the mean heads of the air nodes the potential energy now has (`Scratch::lift_at`).
fn book(w: &mut World, s: &mut Scratch) {
    if let Some(lifts) = s.lifts.take() {
        let (now, room) = lifts.book(w, s);
        // Where the matter now lies, so the lifts the laws after Flow pay go from there.
        let (head, za) = now.heads();
        s.lift_at.iter_mut().zip([head, za]).for_each(|(v, h)| (v.clear(), v.extend_from_slice(h)).1);
        (s.known, s.moved) = (Some((w.time, now)), room);
    }
}

/// The parts of Flow that run from now until `chain(w, false)` follow each other, and the last part of Flow at the
/// world's time now, with nothing else between: their lifts and falls are booked once, at its end (`potential::Lifts`),
/// each mass at the mean of its node's heights at the start and at the end of them all, from the potential energy the
/// last part left (not found again).
pub fn chain(w: &mut World, on: bool) {
    let mut s = std::mem::take(&mut w.scratch);
    if !on {
        book(w, &mut s);
    }
    s.chained = on;
    s.known = s.known.take().filter(|k| on && k.0 == w.time);
    w.scratch = s;
}

/// Lowest and highest of `v`.
fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

/// The share of the minerals of growth in ground node `i` that stays dissolved in its liquid, the rest held on the
/// exchange sites of its loose ground, as Flow's last build of the state of its ground found it (`State::dissolved`).
pub(crate) fn dissolved(w: &World, i: usize) -> f64 {
    w.scratch.state.dissolved[i]
}

/// The liquid share of substance `k` in ground node `i` at `t` K with its `plateau` (`World::molten`), as Flow counts
/// it (`Sp::liquid`: of a mineral of growth only what its exchange sites leave dissolved); none before Flow first built
/// the state of its ground (`potential::stated`).
pub fn liquid_share(w: &World, k: usize, i: usize, t: f64, plateau: Option<(usize, f64)>) -> f64 {
    if !potential::stated(w) {
        return 0.0;
    }
    potential::scratch_of(w).sp[k].liquid(t, w.molten(k, i, t, plateau), dissolved(w, i))
}

/// The liquid in the bed's store under ground node `i`, as a depth over the cell, m: what is liquid of it at the
/// ground's temperature (`Class::Deep`).
pub(crate) fn store_depth(w: &World, i: usize) -> f64 {
    let (n, t) = (w.mesh.cells().n(), w.t_ground_last(i));
    let deep = w.store.amounts(Class::Deep);
    let liquid = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].boil > t).map(|k| deep[k * n + i] * w.chem.molar_volume(k, crate::chem::Phase::Liquid));
    liquid.sum::<f64>() / w.mesh.cells().area()[i]
}

/// Is cell `i` land (its solid surface above the open low liquid's level `STAND_M`, as Flow's state has it)? Creep,
/// heat conducted sideways and the runoff's work act only on land: below it they are far slower than the step.
fn on_land(s: &Scratch, i: usize) -> bool {
    s.z[i] > crate::space::wear::STAND_M
}

/// Share of carrying and creep the bodies on cell `i` leave there (1 when bare).
fn bare_share(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + crate::contract::stand(w, i).mass / w.mesh.cells().area()[i] / w.cfg.phys.hold)
}

/// Weight of a step of `dt` seconds in a moving average over about a year.
fn year_weight(w: &World, dt: f64) -> f32 {
    (dt / w.cfg.calendar().year as f64) as f32
}

/// Move the moving average `avg` toward `x` by the weight `a` (`year_weight`).
#[inline]
fn follow(avg: &mut f32, x: f32, a: f32) {
    *avg += (x - *avg) * a;
}

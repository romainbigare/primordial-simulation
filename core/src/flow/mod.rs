//! Law 1: Flow. Stuff moves along links from high push to low push, with loss (laws.md 1). `scratch`: the law's
//! working memory (`spare`: room its passes keep); `land`: the state and surface of ground nodes; `liquid` (its pass,
//! `runoff`, and the potential energy the pass carries, `falls`), `drain`, `deep`, `basin`, `creep`; `air`: radiation,
//! exchange and wind in one step; `potential` with `stack`: the potential energy of the matter and its booking.

pub(crate) mod air;
mod creep;
mod curve;
mod deep;
mod drain;
mod falls;
mod inside;
mod land;
mod liquid;
mod potential;
mod runoff;
mod scratch;
mod spare;
mod stack;
mod basin;
pub(crate) use air::air_temp;
pub use air::{column, declination, glow, insolation, on_slope, scale_height, sun_by_latitude};
pub use potential::total as potential;
pub use curve::level_of;
pub use inside::{levels, lowest, means, volume_under};
pub(crate) use inside::share_ground;
pub use potential::potential_of;
pub(crate) use potential::{air_head, stated};
pub(crate) use stack::gain;
pub use scratch::Scratch;
pub(crate) use scratch::FlowView;

use crate::{store::Class, AIR, World};
use land::ground_state;
use scratch::tables;

/// Each part of the law (`state`, `air`, `liquid`, `creep`, `refresh`) is a pass of the month (`schedule`). This
/// one: the state of ground and air nodes from the world now, which the other laws read (in full at each month's start,
/// so a world resumed there goes on as an unbroken one; the survey builds it in full then, `built_now`).
pub fn state(w: &mut World, full: bool) {
    with(w, true, |w, s| {
        ground_now(w, s, full);
        air::sums(w, s);
        (0..w.mesh.level(AIR).n()).for_each(|j| s.ta[j] = air_temp(w.store.heat(Class::Air)[j], s.capa[j]));
    });
}

/// The state of ground nodes (`ground_state`): only what changed since it was last rebuilt, or every node with `full`
/// (each month's start, after the month's refresh changed what is present) or when the caches are new. A full build
/// notes the time it was made (`built_now`).
fn ground_now(w: &World, s: &mut Scratch, full: bool) {
    let full = full || s.lay.len() != w.mesh.level(AIR).n();
    ground_state(w, s, full);
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
/// date unless the `state` process just did; the heat of the bodies' nodes that the cover does not claim, which goes
/// back to the ground first, `survey::release`, then warms it from the next state); then, from the state the step's
/// heat leaves (what it melted and froze), the liquid over field capacity sinks into the bed's store (`drain::sink`),
/// as the rain the step brought would otherwise wait in the pores.
pub fn air(w: &mut World, dt: f64) {
    with(w, true, |w, s| {
        crate::survey::release(w);
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

/// Heat conducted sideways through the ground (`land::conduct`), then creep (`creep::creep`), for `dt` seconds.
pub fn creep(w: &mut World, dt: f64) {
    with(w, true, |w, s| {
        land::conduct(w, s, dt);
        creep::creep(w, s, dt);
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
    let fresh = s.state.tg.len() != n;
    if fresh {
        scratch::refresh(w, &mut s);
        // A new world's ground nodes, before its first step: no plateau, no clouds.
        let st = &mut s.state;
        (st.tg, st.pk, st.phi, st.cloud) = (vec![0.0; n], vec![land::NONE; n], vec![0.0; n], vec![[0.0; 2]; w.mesh.level(AIR).n()]);
    }
    let build = s.cap.len() != n;
    if s.lifts.is_none() && !fresh && (moving || build) {
        let (known, room) = (s.known.take().filter(|_| s.chained).map(|k| k.1), std::mem::take(&mut s.moved));
        s.lifts = Some(potential::Lifts::start(w, &s, known, room));
    }
    if build {
        scratch::room(w, &mut s);
        ground_state(w, &mut s, true);
        air::state(w, &mut s);
    }
    f(w, &mut s);
    if !s.chained {
        book(w, &mut s);
    }
    w.scratch = s;
}

impl Scratch {
    /// The state of each ground node follows its cell (`World::remap`), a new cell's and a made cell's that of a new
    /// world's (no plateau, `with`; a made cell's temperature and plateau are then those of its heat, `World::thawed`);
    /// the caches are dropped, as a loaded world's, and rebuilt at the next step. A quarter a split made takes its share
    /// of the volumes of its fixed solids and of its bed (`volumes`, `Made::held`) and its bed's means; a joined cell the
    /// sum of its quarters' volumes and the means of their beds by volume. The next full build of the state (each
    /// month's start) finds the temperature and the plateaus from the heat.
    pub(crate) fn remap(&mut self, r: &crate::remap::Renumber, joined: &[Option<crate::remap::Joined>]) {
        use crate::remap::Of;
        let scratch::State { tg, pk, phi, cloud: _, loose: _, fixed: _, vary: _, fix, bed, bedk: _, gas: _ } = &mut self.state;
        *tg = r.apply(std::mem::take(tg), || 0.0);
        *pk = r.apply(std::mem::take(pk), || land::NONE);
        *phi = r.apply(std::mem::take(phi), || 0.0);
        *fix = r.apply_made(std::mem::take(fix), || [0.0; 2], |_, m, at| [0, 1].map(|x| m.held(joined, Some(Of::Flow(x)), |i| at(i)[x])));
        *bed = r.apply_made(std::mem::take(bed), scratch::Bed::default, |_, m, at| {
            let vol = m.held(joined, Some(Of::Flow(crate::remap::FLOW_BED)), |i| at(i).vol);
            let by_vol = |b: [scratch::Bed; 4], of: fn(&scratch::Bed) -> f64| crate::remap::mean_by(b.map(|x| of(&x)), b.map(|x| x.vol));
            scratch::Bed { vol, ..m.pick(at, |b| scratch::Bed { vol: 0.0, refl: by_vol(b, |x| x.refl), cond: by_vol(b, |x| x.cond) }) }
        });
        *self = Scratch { state: std::mem::take(&mut self.state), kept: std::mem::take(&mut self.kept), ..Default::default() };
    }
}

impl World {
    /// The ground nodes of the cells a remap made (`made`) take the temperature and the plateau of their heat
    /// (`World::thaw`), not their parent's or their quarters' mean: the phases of their stacks are their heat's, as the
    /// potential energy of their matter reads them (`potential`), so a join and the split that gives its quarters back
    /// see the stacks their heat holds. None before Flow first built the state of the ground.
    pub(crate) fn thawed(&mut self, made: &[usize]) {
        if !potential::stated(self) {
            return;
        }
        let at: Vec<_> = made.iter().map(|&i| (i, self.thaw(i))).collect();
        let st = &mut self.scratch.state;
        for (i, (t, on)) in at {
            (st.tg[i], (st.pk[i], st.phi[i])) = (t, on.unwrap_or((land::NONE, 0.0)));
        }
    }
}

/// The volumes Flow keeps of ground node `i` that its cell holds (`remap::Of::Flow`): of its fixed solids, that times
/// their reflectance, and of its bed, m³, as its last refresh found them (none before the first).
pub(crate) fn volumes(w: &World, i: usize) -> Option<[f64; crate::remap::FLOW_HELD]> {
    let st = &w.scratch.state;
    (st.fix.len() == w.mesh.cells().n()).then(|| [st.fix[i][0], st.fix[i][1], st.bed[i].vol])
}

/// Book the lifts and falls of the parts of Flow under way (`potential::Lifts`), if any.
fn book(w: &mut World, s: &mut Scratch) {
    if let Some(lifts) = s.lifts.take() {
        let (now, room) = lifts.book(w, s);
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

/// Is cell `i` land (its solid surface above 0 m, as Flow's state has it)? Creep and heat conducted sideways act only on
/// land: below 0 m they are far slower than the step.
fn on_land(s: &Scratch, i: usize) -> bool {
    s.z[i] > 0.0
}

/// Share of carrying and creep the bodies on cell `i` leave there (1 when bare).
fn bare_share(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + w.life.sum(i)[1] / w.mesh.cells().area()[i] / w.cfg.phys.hold)
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

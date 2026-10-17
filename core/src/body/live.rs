//! A body's update: Flow and Change on it since its last update, in stages (`live`). Upkeep and
//! the end checks are here; the physics of the time between is in `physics`.

use super::exchange::{cover_of, ground_t};
use super::pay::{hold, pay_at, plan, spend, upkeep};
use super::physics::physics;
use super::{thin, Body, Layout, AGE, KEEP_T, LACK, PHASE, REPAIR, SPENT};
use crate::change::Reactor;
use crate::chem::Phase;
use crate::rng::{Purpose, Rng};
use crate::units::DAY;
use crate::{bond, World};

/// Working memory of `live`, kept from body to body.
#[derive(Default)]
pub(crate) struct LiveTmp {
    /// The gas the body exchanges with its air node in this update, mol of each (`pool`; `pool0` as it was before its
    /// reactions), and the air's shares
    pub(super) pool: Vec<f64>,
    pub(super) pool0: Vec<f64>,
    pub(super) x: Vec<f64>,
    /// The catalyst factors of the reactions that run on their own
    pub(super) cat: Vec<f64>,
    /// The amounts of a share of its members that die (`thin`)
    dead: Vec<f64>,
}

/// What a body's update left for its decision (`choose::act`).
#[derive(Default)]
pub struct Lived {
    /// Light caught per kg per day, J
    pub lit: f64,
    /// What its reactions could release over the time against its upkeep (counted up to 2)
    pub spare: f64,
    /// Its mass and heat capacity (valid until its stores change)
    pub mass: f64,
    pub cap: f64,
}

/// Where a body is at its update, and the time since its last.
pub(super) struct Site {
    /// Its ground cell and air node
    pub i: usize,
    pub j: usize,
    /// The ground's temperature as of the last flow step, K (the monthly survey's is too old to feel a cell that turned hot)
    pub tg: f64,
    /// Liquid stands in the cell; the depth of liquid over the body, m (`bond::depth`)
    pub stands: bool,
    pub depth: f64,
    /// The walls around it (`bond::walls_around`), the cover of other bodies over it (`cover_of`), and the share of the
    /// light the walls let through
    pub walls: Option<(f64, f64)>,
    pub cover: f64,
    pub open: f64,
    /// Days since its last update (at least one), and the same in s
    pub days: f64,
    pub dt: f64,
}

fn site(w: &World, b: &Body) -> Site {
    let i = b.cell as usize;
    let (tg, depth) = (ground_t(w, i), w.life.env[i][1]);
    let (stands, depth, walls) = (depth > 0.0, bond::depth(w, b, depth), bond::walls_around(w, b));
    let cover = cover_of(w, b);
    let open = 1.0 - walls.map_or(0.0, |x| x.0);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    Site { i, j: w.grid.air_of(i), tg, stands, depth, walls, cover, open, days, dt: days * DAY }
}

/// Flow and Change on one body since its last update: upkeep, the physics of the time (`physics`), the heat it keeps,
/// the checks of its stores, growth and copying, and what it sheds. Returns what it left for its decision, or a death cause.
pub(crate) fn live(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> Result<Lived, u8> {
    let s = site(w, b);
    let (mass, cap) = b.mass_cap(&w.chem);
    b.last = w.day;
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Err(LACK);
    }
    let (upkeep_heat, span, mass, spare) = settle_upkeep(w, b, &s, mass, cap, tmp)?;
    let lit = physics(w, slot, b, rc, &mut *tmp, &s, mass, upkeep_heat);
    keep_warm(w, b, span, s.dt);
    let (mass, cap) = check_stores(w, slot, b, tmp)?;
    let mut lived = Lived { lit, spare, mass, cap };
    // Growth builds body matter, paid by work (`copy::grow`); past maturity a share `ALLOC` of it goes into the brood.
    // Offspring are made whenever the brood covers a child, by physics, whatever its brain does (`copy::copy`).
    if crate::copy::grow(w, b, s.days) {
        (lived.mass, lived.cap) = b.mass_cap(&w.chem);
    }
    if b.brood > 0.0 && crate::copy::copy(w, slot, b) {
        (lived.mass, lived.cap) = b.mass_cap(&w.chem);
    }
    shed(w, b);
    Ok(lived)
}

/// Upkeep (`upkeep`), paid by the reactions that breathe as fast as they go at the body's temperature, over the time but
/// at least an interval (a decision takes one). In a node the members whose upkeep could not be paid die; an
/// individual that cannot pay dies. Living wears the body (`Body::wear`). Returns the heat the paying released (held
/// in the update's heat), the span it was paid over, the mass left and the margin of its reactions over the upkeep.
fn settle_upkeep(w: &mut World, b: &mut Body, s: &Site, mut mass: f64, cap: f64, tmp: &mut LiveTmp) -> Result<(f64, f64, f64, f64), u8> {
    let (cost, span) = (upkeep(w, b, s.days, mass), s.days.max(b.interval() as f64));
    let spare = plan(w, b, 2.0 * cost, span, mass, cap).1 / cost.max(1e-300);
    let (e_before, node) = (b.e, b.n() > 1.0);
    let short = pay_at(w, b, cost, span, mass, cap, node) / cost.max(1e-300);
    if short > 1e-9 && !node {
        return Err(SPENT);
    }
    // Wear: the energy its upkeep turns over per kg of the body that is built (the tissue that wears, not the fuel it
    // carries), over what it can turn over in its life (`Physics::e_life`). A node's members are of all ages: this share
    // of them dies now (the death rate of members who each live that long, so the same mean life).
    let worn = cost / (b.built(&w.chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    // The heat set aside is what the burning released less what its products take to stand at the body's temperature, so
    // the body keeps its temperature as its stores change (as in `pay`).
    let kept = e_before / cap.max(1e-9) * b.heat_cap(&w.chem);
    let upkeep_heat = b.e - kept;
    b.e = kept;
    for (f, cause) in [(short, SPENT), (if node { worn.min(1.0) } else { 0.0 }, AGE)] {
        if f > 1e-9 {
            if !members_die(w, b, f, cause, &mut tmp.dead) {
                b.e += upkeep_heat;
                return Err(cause);
            }
            mass = b.mass(&w.chem);
        }
    }
    b.wear += if node { 0.0 } else { worn };
    if b.wear >= 1.0 {
        b.e += upkeep_heat;
        return Err(AGE);
    }
    Ok((upkeep_heat, span, mass, spare))
}

/// The share `f` of a node's members dies of `cause`, with that share of every store (`thin`); false if less than one
/// member is left.
fn members_die(w: &mut World, b: &mut Body, f: f64, cause: u8, dead: &mut Vec<f64>) -> bool {
    dead.clear();
    dead.extend(b.store.iter().map(|a| a * f));
    thin(w, b, f, dead, cause)
}

/// It keeps its temperature at its `KEEP_T` gene: if the update left it colder, it burns on demand, as for its upkeep,
/// the heat its surface sheds at that level over the time (the share `hold` of a heat released over the time stays in
/// it), as far as its reactions can release it. Twice, as its products take a part of the heat at its temperature.
fn keep_warm(w: &mut World, b: &mut Body, span: f64, dt: f64) {
    for _ in 0..2 {
        let (m, c) = b.mass_cap(&w.chem);
        let short = c * b.gene(KEEP_T) - b.e;
        if c <= 0.0 || short <= 1e-6 * b.e || spend(w, b, short / hold(w, b, m, b.e / c, dt), span, dt, true) > 0.0 {
            break;
        }
    }
}

/// A store below its safe minimum kills the body (a node loses its poorest members, `starve`). A key liquid that boiled
/// kills it; one that froze kills the share of it that is free liquid to freeze: the frozen share (all of it, the body
/// being at one temperature) times its activity, as a dry body holds little liquid to freeze. A node loses that share of
/// its members (`thin`); an individual dies with that chance. Returns the mass and heat capacity left.
fn check_stores(w: &mut World, slot: usize, b: &mut Body, tmp: &mut LiveTmp) -> Result<(f64, f64), u8> {
    let (mut mass, mut c) = b.mass_cap(&w.chem);
    if b.lacking_at(&w.chem, mass) {
        if !(b.n() > 1.0 && !b.empty_at(mass) && starve(w, b, &mut tmp.dead)) {
            return Err(LACK);
        }
        (mass, c) = b.mass_cap(&w.chem);
    }
    let (chem, mut left) = (&w.chem, (mass, c));
    let (t, ns) = (b.e / c.max(1e-9), chem.ns());
    let key = |k: usize| b.wet >> k & 1 == 1;
    if (0..ns).any(|k| key(k) && chem.phase(k, t) == Phase::Gas) {
        return Err(PHASE);
    }
    if (0..ns).any(|k| key(k) && t < b.melt(chem, k)) {
        let f = b.activity(chem, &w.cfg.phys);
        if f >= 1.0 - 1e-9 {
            return Err(PHASE);
        }
        if b.n() > 1.0 {
            if !members_die(w, b, f, PHASE, &mut tmp.dead) {
                return Err(PHASE);
            }
            left = b.mass_cap(&w.chem);
        } else if Rng::new(w.seed, w.day, b.id(slot), Purpose::Frost).f64() < f {
            return Err(PHASE);
        }
    }
    Ok(left)
}

/// A node short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The
/// share that dies is the least that lifts every key store to its safe minimum; false if that
/// is all of them.
fn starve(w: &mut World, b: &mut Body, dead: &mut Vec<f64>) -> bool {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let x = |k: usize| b.store[k] * chem.subs[k].molar_mass;
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&k: &usize| !b.burns(chem, k) && x(k) < b.gene(lay.min + k) * size;
    let live = (0..chem.ns()).filter(short).map(|k| x(k) / (b.gene(lay.min + k) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    dead.clear();
    dead.extend((0..chem.ns()).map(|k| if short(&k) { 0.0 } else { b.store[k] * f }));
    f < 1.0 && thin(w, b, f, dead, LACK) && !b.lacking(&w.chem)
}

/// A body is not a quarry: of what it holds besides its liquid, matter, gases and the fuel it burns, the minerals
/// (`Chemistry::minerals`) and the solids it builds (`Chemistry::materials`: what it took of one and did not break passes
/// through) go back to the ground past the level it keeps (`Body::level`), and anything else (a solid left by a salt
/// it held breaking down) all of it, with its share of the heat. Digestion (`Body::digest`) and its own burning of matter give it more
/// than it builds with: an eater that kept it would grow without end and, its room full, stop taking
/// matter. A plant takes only up to the level (`choose::wanted`), so for it the minerals shed nothing.
/// The key liquid past its level drains out the same way (15 Oct 2026, review 7 R11): its burning makes it in the body, and
/// before only evaporation took it out, so eaters held 40 to 65% of their liquid past the level (30 to 48% of their mass).
/// It goes to the ground with its heat; what is dissolved in it goes by its own level in this same loop (the minerals are
/// kept by the body's matter, not by its liquid), and gases leave by breath.
fn shed(w: &mut World, b: &mut Body) {
    let (i, n, mass, minerals) = (b.cell as usize, w.grid.n(), b.mass(&w.chem), w.chem.minerals());
    for k in 0..w.chem.ns() {
        let kept = b.wet >> k & 1 == 1 || minerals.iter().any(|m| m.0 == k) || w.chem.is_material(k);
        let over = b.store[k] - if kept { b.level(&w.chem, k, mass) } else { 0.0 };
        if over <= 0.0 || w.chem.gas_at_surface(k) || w.chem.matter() == Some(k) || b.burns(&w.chem, k) {
            continue;
        }
        let q = b.e * over * w.chem.subs[k].cp / b.heat_cap(&w.chem).max(1e-9);
        b.store[k] -= over;
        w.ground[k * n + i] += over;
        b.e -= q;
        w.e_ground[i] += q;
    }
}

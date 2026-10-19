//! A body's update: Flow and Change on it since its last update, in stages (`live`). Upkeep and
//! the end checks are here; the physics of the time between is in `physics`.

use super::exchange::cover_of;
use super::pay::{hold, pay_at, plan, spend, upkeep, Payer};
use super::physics::{physics, soak_again};
use super::{thin, to_ground, Body, Layout, AGE, KEEP_T, LACK, MARGIN, PHASE, REPAIR, SPENT};
use crate::bond::{self, Enclosure};
use crate::change::Reactor;
use crate::chem::Phase;
use crate::ground::Melting;
use crate::math::one_minus_exp;
use crate::rng::{Purpose, Rng};
use crate::units::DAY;
use crate::{World, AIR};

/// Passes of `keep_warm`: its products take a part of the heat it burns at its temperature, so a second pass tops it up.
const KEEP_T_PASSES: usize = 2;
/// `keep_warm` burns nothing for a shortfall under this share of the body's heat.
const KEEP_T_SLACK: f64 = 1e-6;

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
    /// What its reactions could release over the time against its upkeep (well-being counts it up to `MARGIN`)
    pub spare: f64,
    /// Its mass as its update leaves it, after it sheds (valid until its stores change)
    pub mass: f64,
    /// The log of what it holds that can become body (`copy::buildable`: its matter, brood with it, and what its fuel
    /// could build) after its growth over that as its update began: what its members gained in the time, by growth and
    /// reserves, less the deaths (`copy::select`)
    pub gain: f64,
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
    /// Its link to the outside through the walls around it (`bond::enclosure`), the cover of other bodies over it
    /// (`cover_of`), and the share of the light and rain the walls let through
    pub walls: Option<Enclosure>,
    pub cover: f64,
    pub open: f64,
    /// Days since its last update (at least one), and the same in s
    pub days: f64,
    pub dt: f64,
}

fn site(w: &World, b: &Body) -> Site {
    let i = b.cell as usize;
    let (tg, depth) = (w.t_ground_last(i), w.life.env[i][1]);
    let (stands, depth, walls) = (depth > 0.0, bond::depth(w, b, depth), bond::enclosure(w, b));
    let cover = cover_of(w, b);
    let open = 1.0 - walls.map_or(0.0, |x| x.closed as f64);
    let days = (w.day() as f64 - b.last).max(1.0);
    Site { i, j: w.mesh.up(i, 0, AIR), tg, stands, depth, walls, cover, open, days, dt: days * DAY }
}

/// Flow and Change on one body since its last update: upkeep, the physics of the time (`physics`), the heat it keeps,
/// the checks of its stores, growth and copying, and what it sheds. Returns what it left for its decision, or a death cause.
pub(crate) fn live(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> Result<Lived, u8> {
    let (g, s) = to_growth(w, slot, b, rc, tmp)?;
    // Offspring are made whenever the brood covers a child, by physics, whatever its brain does (`copy::copy`). The key
    // liquid the new tissue lacks and the children took soaks in again (`soak_again`).
    let born = b.brood > 0.0 && crate::copy::copy(w, slot, b);
    if g.built || born {
        soak_again(w, b, &s);
    }
    let mass = shed(w, b);
    Ok(Lived { lit: g.lit, spare: g.spare, mass, gain: g.gain })
}

/// What an update did as far as its growth (`to_growth`): the light it caught per kg per day, J, its spare, whether it
/// built, and what its members gained (`Lived::gain`).
struct Growth {
    lit: f64,
    spare: f64,
    built: bool,
    gain: f64,
}

/// The update of body `b` as far as its growth (`live`): upkeep, the physics of the time, the heat it keeps, the
/// checks of its stores, and growth, which builds body matter paid by work (`copy::grow`; past maturity a share
/// `ALLOC` of it goes into the brood). Returns what it did and where it is, or a death cause.
fn to_growth(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> Result<(Growth, Site), u8> {
    let s = site(w, b);
    let mass = b.mass(&w.chem);
    let held = crate::copy::buildable(w, b, s.tg);
    b.last = w.day() as f64;
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Err(LACK);
    }
    let (upkeep_heat, span, mass, spare, th) = settle_upkeep(w, b, &s, mass, tmp)?;
    let lit = physics(w, slot, b, rc, &mut *tmp, &s, mass, upkeep_heat, th);
    keep_warm(w, b, span, s.dt);
    let th = check_stores(w, slot, b, tmp)?;
    let built = crate::copy::grow(w, b, s.days, th);
    let gain = (crate::copy::buildable(w, b, s.tg) / held.max(f64::MIN_POSITIVE)).ln();
    Ok((Growth { lit, spare, built, gain }, s))
}

/// What the members of body `b` (slot `slot`, taken out of it) gain in an update as far as its growth (`Lived::gain`;
/// minus infinity if it dies), for a sample member that changes nothing (`copy::select`: the caller gives the world
/// back as it was).
pub(crate) fn gain_to_growth(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> f64 {
    to_growth(w, slot, b, rc, tmp).map_or(f64::NEG_INFINITY, |g| g.0.gain)
}

/// Upkeep (`upkeep`), paid by the reactions that breathe as fast as they go at the body's temperature, over the time but
/// at least an interval (a decision takes one). In a node the members whose upkeep could not be paid die: the share of
/// its upkeep over the update left unpaid, whatever the length of the update; an individual that cannot pay dies.
/// Living wears the body (`Body::wear`). Returns the heat the paying released (held
/// in the update's heat), the span it was paid over, the mass left, the margin of its reactions over the upkeep, and the
/// plateau state it is left at (`Body::thermal`: the stages that follow see the body at the temperature it came with).
fn settle_upkeep(w: &mut World, b: &mut Body, s: &Site, mut mass: f64, tmp: &mut LiveTmp) -> Result<(f64, f64, f64, f64, Melting), u8> {
    // (One temperature for the stages that see the body as it came: the upkeep, the plan and the paying.)
    let (th, node) = (b.thermal(&w.chem), b.n() > 1.0);
    let (cost, span) = (upkeep(w, b, s.days, mass, th.t), s.days.max(b.interval(w) as f64));
    let p = Payer::at(w, b, span, mass, th);
    let spare = plan(w, b, MARGIN * cost, &p).1 / cost.max(1e-300);
    let short = pay_at(w, b, cost, &p, node) / cost.max(1e-300);
    if short > 1e-9 && !node {
        return Err(SPENT);
    }
    // Wear: the energy its upkeep turns over per kg of the body that is built (the tissue that wears, not the fuel it
    // carries), over what it can turn over in its life (`Physics::e_life`). A node's members are of all ages: they die
    // at the rate of the wear, so over the time the share 1 - e^-wear of them dies now (the same mean life, whatever the
    // length of its updates).
    let worn = cost / (b.built(&w.chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    // The heat set aside is what the burning released less what its products take to stand at the body's temperature, so
    // the body keeps its temperature as its stores change (as in `pay`).
    let kept = b.energy_in(&w.chem, &p.th);
    let upkeep_heat = b.e - kept;
    b.e = kept;
    for (f, cause) in [(short, SPENT), (if node { one_minus_exp(worn) } else { 0.0 }, AGE)] {
        if f > 1e-9 {
            if !members_die(w, b, f, cause, &mut tmp.dead, &p.th) {
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
    Ok((upkeep_heat, span, mass, spare, p.th))
}

/// The share `f` of a node's members, at the plateau state `th`, dies of `cause`, with that share of every store (`thin`);
/// false if less than one member is left.
fn members_die(w: &mut World, b: &mut Body, f: f64, cause: u8, dead: &mut Vec<f64>, th: &Melting) -> bool {
    dead.clear();
    dead.extend(b.store.iter().map(|a| a * f));
    thin(w, b, f, dead, cause, th)
}

/// It keeps its temperature at its `KEEP_T` gene: if the update left it colder, it burns on demand, as for its upkeep,
/// the heat its surface sheds at that level over the time (the share `hold` of a heat released over the time stays in
/// it), as far as its reactions can release it. Twice, as its products take a part of the heat at its temperature.
fn keep_warm(w: &mut World, b: &mut Body, span: f64, dt: f64) {
    for _ in 0..KEEP_T_PASSES {
        let (m, c) = b.mass_cap(&w.chem);
        let short = b.energy_at(&w.chem, b.gene(KEEP_T)) - b.e;
        if c <= 0.0 || short <= KEEP_T_SLACK * b.e {
            break;
        }
        let p = Payer::at(w, b, span, m, b.thermal(&w.chem));
        if spend(w, b, short / hold(w, b, m, p.th.t, dt), dt, true, &p) > 0.0 {
            break;
        }
    }
}

/// A store below its safe minimum kills the body (a node loses its poorest members, `starve`). A key liquid that boiled
/// kills it; one that froze kills the share of it that is free liquid to freeze: the share frozen (all of it below its
/// melting point, the body being at one temperature; on its melting plateau, the share frozen there) times its
/// activity, as a dry body holds little liquid to freeze. A node loses that share of its members (`thin`; that share of
/// an interval of its gene, at that rate over a longer update, `Body::over_span`); an individual dies with that chance.
/// Returns the plateau state it is left at.
fn check_stores(w: &mut World, slot: usize, b: &mut Body, tmp: &mut LiveTmp) -> Result<Melting, u8> {
    let mass = b.mass(&w.chem);
    let mut th = b.thermal(&w.chem);
    if b.lacking(&w.chem, mass, th.t) {
        if !(b.n() > 1.0 && !b.empty_at(mass) && starve(w, b, &th, &mut tmp.dead)) {
            return Err(LACK);
        }
        th = b.thermal(&w.chem);
    }
    let (chem, ns) = (&w.chem, w.chem.ns());
    let key = |k: usize| b.wet >> k & 1 == 1;
    if (0..ns).any(|k| key(k) && chem.phase(k, th.t) == Phase::Gas) {
        return Err(PHASE);
    }
    let frozen = |k: usize| match th.on {
        Some((j, molten)) if j == k => 1.0 - molten,
        _ => (th.t < b.melt(chem, k)) as u8 as f64,
    };
    let frozen = (0..ns).filter(|&k| key(k)).map(frozen).fold(0.0, f64::max);
    if frozen > 0.0 {
        let f = frozen * b.activity(chem, &w.cfg.phys);
        if f >= 1.0 - 1e-9 {
            return Err(PHASE);
        }
        if b.n() > 1.0 {
            if !members_die(w, b, b.over_span(f, w), PHASE, &mut tmp.dead, &th) {
                return Err(PHASE);
            }
            return Ok(b.thermal(&w.chem));
        } else if Rng::new(w.seed, w.day(), b.id(slot), Purpose::Frost).f64() < f {
            return Err(PHASE);
        }
    }
    Ok(th)
}

/// A node at the plateau state `th` short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The share that dies is the least
/// that lifts every key store to its safe minimum; false if that is all of them.
fn starve(w: &mut World, b: &mut Body, th: &Melting, dead: &mut Vec<f64>) -> bool {
    let t = th.t;
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let x = |k: usize| b.store[k] * chem.subs[k].molar_mass;
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&k: &usize| !b.burns_at(chem, k, t) && x(k) < b.gene(lay.min + k) * size;
    let live = (0..chem.ns()).filter(short).map(|k| x(k) / (b.gene(lay.min + k) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    dead.clear();
    dead.extend((0..chem.ns()).map(|k| if short(&k) { 0.0 } else { b.store[k] * f }));
    f < 1.0 && thin(w, b, f, dead, LACK, th) && !b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem))
}

/// What a body sheds to the ground, with its share of the heat: of what it holds besides its matter, gases and the fuel
/// it burns, its key liquids, the minerals (`Chemistry::minerals`) and the solids it builds (`Chemistry::materials`)
/// past the level it keeps (`Body::level`), and all of anything else. Digestion and its own burning make more of them
/// than it builds with (its burning makes its key liquid): an eater that kept them would grow without end and, its room
/// full, stop taking matter. Gases (at its temperature) leave by breath. Returns its mass after.
fn shed(w: &mut World, b: &mut Body) -> f64 {
    let (chem, mass) = (&w.chem, b.mass(&w.chem));
    let th = b.thermal(chem);
    let t = th.t;
    let minerals = chem.minerals();
    let over: Vec<f64> = (0..chem.ns())
        .map(|k| {
            let kept = b.wet >> k & 1 == 1 || minerals.iter().any(|m| m.0 == k) || chem.is_material(k);
            let over = b.store[k] - if kept { b.level(chem, k, mass) } else { 0.0 };
            let stays = over <= 0.0 || chem.phase(k, t) == Phase::Gas || chem.matter() == Some(k) || b.burns_at(chem, k, t);
            if stays { 0.0 } else { over }
        })
        .collect();
    if over.iter().any(|&a| a > 0.0) {
        to_ground(w, b, &th, &over);
        return b.mass(&w.chem);
    }
    mass
}

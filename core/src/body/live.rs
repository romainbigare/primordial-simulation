//! A body's update: Flow and Change on it since its last update, in stages (`live`). Upkeep and
//! the end checks are here; the physics of the time between is in `physics`.

use super::exchange::cover_of;
use super::pay::{hold, pay_at, plan, spend, upkeep, Payer};
use super::physics::{physics, soak_again};
use super::{thin, to_ground, Body, Layout, AGE, COPY_MASS, KEEP_T, LACK, MARGIN, PHASE, REPAIR, SPENT};
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
    let (held, from) = (crate::copy::buildable(w, b, s.tg), b.member_body(&w.chem));
    b.last = w.day() as f64;
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Err(LACK);
    }
    let (upkeep_heat, span, mass, spare, th) = settle_upkeep(w, slot, b, &s, mass, tmp)?;
    let lit = physics(w, slot, b, rc, &mut *tmp, &s, mass, upkeep_heat, th);
    keep_warm(w, b, span, s.dt);
    let th = check_stores(w, slot, b, tmp)?;
    let built = crate::copy::grow(w, b, s.days, th);
    if !starved(w, slot, b, from, &mut tmp.dead) {
        return Err(SPENT);
    }
    let gain = (crate::copy::buildable(w, b, s.tg) / held.max(f64::MIN_POSITIVE)).ln();
    Ok((Growth { lit, spare, built, gain }, s))
}

/// What the members of body `b` (slot `slot`, taken out of it) gain in an update as far as its growth (`Lived::gain`;
/// minus infinity if it dies), for a sample member that changes nothing (`copy::select`: the caller gives the world
/// back as it was).
pub(crate) fn gain_to_growth(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> f64 {
    to_growth(w, slot, b, rc, tmp).map_or(f64::NEG_INFINITY, |g| g.0.gain)
}

/// A body whose growth did not rebuild the body it lost in the update, below its copy mass, starves: it keeps the share
/// x of its body (of `from` kg a member, or of its copy mass if it was larger) and dies with the chance 1 - x^a, a such
/// that one that starved to the lethal loss (`Physics::lethal`) has lived through it with even chances, in one update or
/// in many (`lose_share`). An adult larger than its copy mass shrinks towards it as it chooses (`ALLOC`): that is no
/// starving. False if it dies (a node: less than one member is left).
fn starved(w: &mut World, slot: usize, b: &mut Body, from: f64, dead: &mut Vec<f64>) -> bool {
    let (now, was) = (b.member_body(&w.chem), from.min(b.dec(COPY_MASS)));
    let a = -std::f64::consts::LN_2 / (1.0 - w.cfg.phys.lethal).ln();
    // (A body with no body left has starved whatever it was.)
    let f = if now > 0.0 { 1.0 - (now / was).min(1.0).powf(a) } else { 1.0 };
    let th = b.thermal(&w.chem);
    f <= 1e-9 || lose_share(w, slot, b, f, SPENT, dead, &th)
}

/// Upkeep over the time, at least an interval: the share left unpaid dies, and so does the share its wear gives
/// (`lose_share`). Returns the heat released, the span, the mass left, its spare and the plateau state it came at.
fn settle_upkeep(w: &mut World, slot: usize, b: &mut Body, s: &Site, mut mass: f64, tmp: &mut LiveTmp) -> Result<(f64, f64, f64, f64, Melting), u8> {
    // (One temperature for the stages that see the body as it came: the upkeep, the plan and the paying.)
    let th = b.thermal(&w.chem);
    let (cost, span) = (upkeep(w, b, s.days, mass, th.t), s.days.max(b.interval(w) as f64));
    let p = Payer::at(w, b, span, mass, th);
    let spare = plan(w, b, MARGIN * cost, &p).1 / cost.max(1e-300);
    let short = pay_at(w, b, cost, &p, true) / cost.max(1e-300);
    // Wear: the energy its upkeep turns over per kg of the body that is built (the tissue that wears, not the fuel it
    // carries), over what it can turn over in its life (`Physics::e_life`). Its members die at the rate of the wear: over
    // the time the share 1 - e^-wear of them (an individual, with that chance), so they live that life on average,
    // whatever the length of its updates.
    let worn = cost / (b.built(&w.chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    // The heat set aside is what the burning released less what its products take to stand at the body's temperature, so
    // the body keeps its temperature as its stores change (as in `pay`).
    let kept = b.energy_in(&w.chem, &p.th);
    let upkeep_heat = b.e - kept;
    b.e = kept;
    for (f, cause) in [(short, SPENT), (one_minus_exp(worn), AGE)] {
        if f > 1e-9 {
            if !lose_share(w, slot, b, f, cause, &mut tmp.dead, &p.th) {
                b.e += upkeep_heat;
                return Err(cause);
            }
            mass = b.mass(&w.chem);
        }
    }
    Ok((upkeep_heat, span, mass, spare, p.th))
}

/// Draws of death a body may take in a day, one for each cause (`Rng` keyed by the day and the cause).
const CAUSES: u64 = 8;

/// The share `f` of the members of body `b` (slot `slot`), at the plateau state `th`, dies of `cause`: a node loses that
/// share, with that share of every store (`thin`); an individual dies with that chance. One rule for both, so a node is
/// the average of its members. False if it dies (a node: less than one member is left).
fn lose_share(w: &mut World, slot: usize, b: &mut Body, f: f64, cause: u8, dead: &mut Vec<f64>, th: &Melting) -> bool {
    if b.n() <= 1.0 {
        return Rng::new(w.seed, w.day() * CAUSES + cause as u64, b.id(slot), Purpose::Death).f64() >= f;
    }
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

/// A store below its safe minimum kills it (a node: its poorest members, `starve`); a boiled key liquid kills it, a
/// frozen one kills the share frozen times its activity (`Body::over_span`). Returns the plateau state it is left at.
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
        if !lose_share(w, slot, b, b.over_span(f, w), PHASE, &mut tmp.dead, &th) {
            return Err(PHASE);
        }
        return Ok(b.thermal(&w.chem));
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
    let short = |&k: &usize| x(k) < b.gene(lay.min + k) * size && !b.burns_at(chem, k, t);
    let live = (0..chem.ns()).filter(short).map(|k| x(k) / (b.gene(lay.min + k) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    dead.clear();
    dead.extend((0..chem.ns()).map(|k| if short(&k) { 0.0 } else { b.store[k] * f }));
    f < 1.0 && thin(w, b, f, dead, LACK, th) && !b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem))
}

/// What it sheds to the ground with its heat: minerals, built solids and key liquids past their level, and anything
/// else it neither burns nor is made of; gases leave by breath. Returns its mass after.
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

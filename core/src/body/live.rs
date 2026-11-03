//! A body's update: its upkeep, its drink, its heat, the checks of its stores, its growth and children, and what it
//! sheds, over the time since its last update (laws.md 6).

use super::pay::{burn, settle_heat, upkeep};
use super::{thin, to_ground, Body, Layout, AGE, COOL_T, KEEP_T, LACK, MARGIN, PHASE, REPAIR, SPENT};
use crate::chem::{Phase, Stores};
use crate::ground::Melting;
use crate::math::one_minus_exp;
use crate::rng::{Purpose, Rng};
use crate::store::Node;
use crate::units::{BODY_STORES, DAY};
use crate::{life::Chunk, World};

/// Working memory of `live`, kept from body to body.
#[derive(Default)]
pub(crate) struct LiveTmp {
    /// The amounts of a share of its members that die (`thin`)
    dead: Stores,
}

/// What a body's update left for its decision (`choose::act`).
#[derive(Clone, Copy, Default)]
pub struct Lived {
    /// What its fuel could release over the time against its upkeep (well-being counts it up to `MARGIN`)
    pub spare: f64,
}

/// The update of body `b` over the time since its last: upkeep paid by burning (the share unpaid dies, `SPENT`, and the
/// share its wear gives, `AGE`), its drink, its heat (`keep_warm`), the checks of its stores, growth (`copy::grow`),
/// children (`copy::copy`) and what it sheds. Returns what it left for its decision, or a death cause.
pub(crate) fn live(w: &World, d: &mut Chunk, b: &mut Body, tmp: &mut LiveTmp) -> Result<Lived, u8> {
    let days = (w.day() as f64 - b.last).max(1.0);
    b.last = w.day() as f64;
    let (mass, from) = (b.mass(&w.chem), b.member_body(&w.chem));
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Err(LACK);
    }
    let (heat, spare) = settle_upkeep(w, d, b, days, mass, tmp)?;
    drink(w, d, b, days);
    let stands = keep_warm(w, d, b, heat, days * DAY);
    let th = check_stores(w, d, b, stands, tmp)?;
    crate::copy::grow(w, d, b, days, th.t);
    if !starved(w, d, b, from, &mut tmp.dead) {
        return Err(SPENT);
    }
    // Offspring are made whenever the brood covers a child, by physics, whatever its brain does (`copy::copy`).
    if b.brood > 0.0 {
        crate::copy::copy(w, d, b);
    }
    shed(w, d, b);
    Ok(Lived { spare })
}

/// Upkeep over the time: what its fuel releases is paid (`burn`), the share left unpaid dies, and so does the share its
/// wear gives (`lose_share`). Returns the heat it released, J, and its spare: what its fuel could release against its
/// upkeep, up to `MARGIN`.
fn settle_upkeep(w: &World, d: &mut Chunk, b: &mut Body, days: f64, mass: f64, tmp: &mut LiveTmp) -> Result<(f64, f64), u8> {
    let (chem, t, act) = (&w.chem, b.temp(&w.chem), b.activity(&w.chem, &w.cfg.phys));
    let cost = upkeep(w, b, days, mass, t, act);
    let spare = b.fuel(chem, t).min(MARGIN * cost) / cost.max(1e-300);
    let paid = burn(w, d, b, cost);
    // Wear: the energy its upkeep turns over per kg of the body that is built (the tissue that wears, not the fuel it
    // carries), over what it can turn over in its life (`Physics::e_life`). Its members die at the rate of the wear: over
    // the time the share 1 - e^-wear of them (an individual, with that chance), so they live that life on average,
    // whatever the length of its updates. A dormant body wears at least at `Physics::dormant` of its active rate, as
    // seeds and spores keep a slow metabolism and live years to decades, not for ever (Walters 1998).
    let floor = w.cfg.phys.dormant;
    let wearing = if act >= floor { cost } else { upkeep(w, b, days, mass, t, floor) };
    let worn = wearing / (b.built(chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    let short = if cost > 0.0 { 1.0 - paid / cost } else { 0.0 };
    for (f, cause) in [(short, SPENT), (one_minus_exp(worn), AGE)] {
        if f > 1e-9 && !lose_share(w, d, b, f, cause, &mut tmp.dead, &b.thermal(chem)) {
            return Err(cause);
        }
    }
    Ok((paid, spare))
}

/// It drinks its key liquids from its cell's ground up to the level it keeps (`Body::level`), as far as the ground holds
/// them liquid, each day at most its uptake gene's share of that level, paying to handle them (`c_handle` per kg); they
/// come with their heat.
fn drink(w: &World, d: &mut Chunk, b: &mut Body, days: f64) {
    let (chem, p, i) = (&w.chem, &w.cfg.phys, b.cell as usize);
    let (lay, tg, mass) = (Layout::of(chem), w.t_ground_last(i), b.mass(chem));
    for s in b.wet_ks() {
        let k = chem.body().subs()[s];
        if b.gene(lay.upt + s) <= 0.0 {
            continue;
        }
        let (ground, mm, level) = (d.moves.amount(w, Node::ground(i), k), chem.subs[k].molar_mass, b.level(chem, s, mass));
        let x = (level - b.store[s]).max(0.0).min(ground).min(b.gene(lay.upt + s) * level * days);
        if x <= 0.0 || chem.phase(k, tg) != Phase::Liquid || !super::pay(w, d, b, p.c_handle * x * mm) {
            continue;
        }
        let cp = chem.subs[k].cp;
        let want = x * (cp * tg + chem.subs[k].lf);
        let cap = d.moves.heat_cap_ground(w, i);
        let (got, heat) = d.moves.take_from_ground(w, i, (k, x, cp), want, cap);
        (b.store[s], b.e) = (b.store[s] + got, b.e + heat);
    }
}

/// Its heat over the time `dt` s (laws.md 6.3): the heat its upkeep released leaves through its coat (`HEAT`,
/// `Body::conductance`; in series with the walls around it, 3.4) to its ground, so it stands that much above the ground.
/// Below its `KEEP_T` gene it burns what its coat loses at that temperature beyond it. Above its `COOL_T` gene it opens
/// its coat as far as it needs to stand there, up to its bare surface (`Body::bare`: blood to the skin, the coat laid
/// flat; Scholander et al. 1950), and past that evaporates its key liquid for the excess, at its latent heat, never
/// below its safe minimum. It then stands at that temperature (`settle_heat`).
fn keep_warm(w: &World, d: &mut Chunk, b: &mut Body, heat: f64, dt: f64) -> f64 {
    let (chem, p, i) = (&w.chem, &w.cfg.phys, b.cell as usize);
    let (tg, mass) = (w.t_ground_last(i), b.mass(chem));
    let walls = crate::bond::enclosure(d, b).map(|x| x.g);
    // The conductance through a coat of conductance `c` and the walls, over the time, J/K.
    let through = |c: f64| walls.map_or(c, |x| 1.0 / (1.0 / c + 1.0 / x)) * dt;
    let (g, open) = (through(b.conductance(p, mass)), through(b.bare(p, mass)));
    if g <= 0.0 {
        return b.temp(chem);
    }
    let (keep, cool) = (b.gene(KEEP_T), b.gene(COOL_T));
    let free = tg + heat / g;
    let t = if free < keep {
        tg + (heat + burn(w, d, b, g * (keep - tg) - heat)) / g
    } else if free > cool {
        // (On ground at or above its `COOL_T` it opens all the way.)
        let opened = if cool > tg { (heat / (cool - tg)).clamp(g, open) } else { open };
        let t_open = tg + heat / opened;
        if t_open > cool { tg + (heat - evaporate(w, d, b, heat - opened * (cool - tg))) / opened } else { t_open }
    } else {
        free
    };
    settle_heat(w, d, b, t);
    t
}

/// It evaporates its key liquids to take `q` J away, at each one's vaporisation enthalpy, never below its safe minimum:
/// the vapour goes to its cell's bodies' node with its heat and its latent heat (`Moves::gas`). Returns the heat taken
/// away, J.
fn evaporate(w: &World, d: &mut Chunk, b: &mut Body, q: f64) -> f64 {
    let (chem, i) = (&w.chem, b.cell as usize);
    let (lay, t, size, lift) = (Layout::of(chem), b.temp(chem), b.size(chem), w.lift(i));
    let mut taken = 0.0;
    for p in b.wet_ks() {
        let k = chem.body().subs()[p];
        let s = &chem.subs[k];
        let free = (b.store[p] - b.gene(lay.min + p).max(0.0) * size / s.molar_mass).max(0.0);
        let x = free.min((q - taken) / s.lv);
        if x <= 0.0 {
            continue;
        }
        b.store[p] -= x;
        d.moves.gas(w, k, x, lift);
        d.moves.give(w, Node::bodies(i), 0, 0.0, x * s.cp * t);
        // (Its own heat held the liquid's sensible and fusion heat; the vapour takes its vaporisation heat too.)
        b.e -= x * (s.cp * t + s.latent_gas());
        taken += x * s.lv;
    }
    taken
}

/// A body whose growth did not rebuild the body it lost in the update, below its copy mass, starves: it keeps the share
/// x of its body (of `from` kg a member, or of its copy mass if it was larger) and dies with the chance 1 - x^a, a such
/// that one that starved to the lethal loss (`Physics::lethal`) has lived through it with even chances, in one update or
/// in many (`lose_share`). An adult larger than its copy mass shrinks towards it as it chooses (`ALLOC`): that is no
/// starving. False if it dies (a node: less than one member is left).
fn starved(w: &World, d: &mut Chunk, b: &mut Body, from: f64, dead: &mut Stores) -> bool {
    let (now, was) = (b.member_body(&w.chem), from.min(b.k.copy_mass));
    let a = -std::f64::consts::LN_2 / (1.0 - w.cfg.phys.lethal).ln();
    // (A body with no body left has starved whatever it was.)
    let f = if now > 0.0 { 1.0 - (now / was).min(1.0).powf(a) } else { 1.0 };
    // (Its plateau state is worked out only if it loses a share.)
    let th = (f > 1e-9).then(|| b.thermal(&w.chem));
    th.is_none_or(|th| lose_share(w, d, b, f, SPENT, dead, &th))
}

/// Draws of death a body may take in a day, one for each cause (`Rng` keyed by the day and the cause).
const CAUSES: u64 = 8;

/// The share `f` of the members of body `b`, at the plateau state `th`, dies of `cause`: a node loses that
/// share, with that share of every store (`thin`); an individual dies with that chance. One rule for both, so a node is
/// the average of its members. False if it dies (a node: less than one member is left).
fn lose_share(w: &World, d: &mut Chunk, b: &mut Body, f: f64, cause: u8, dead: &mut Stores, th: &Melting) -> bool {
    if b.n() <= 1.0 {
        return Rng::new(w.seed, w.day() * CAUSES + cause as u64, b.id, Purpose::Death).f64() >= f;
    }
    *dead = b.store.map(|a| a * f);
    thin(w, d, b, f, dead, cause, th)
}

/// A store below its safe minimum kills it (a node: its poorest members, `starve`); a boiled key liquid kills it; below
/// a key liquid's melting point the share of its members that freezes at `stands` K, the temperature its heat holds it
/// at over its ground (`keep_warm`, not the melting plateau its liquid passes on the way), dies (laws.md 6.5). Returns
/// the plateau state it is left at.
fn check_stores(w: &World, d: &mut Chunk, b: &mut Body, stands: f64, tmp: &mut LiveTmp) -> Result<Melting, u8> {
    let mass = b.mass(&w.chem);
    let mut th = b.thermal(&w.chem);
    if b.lacking(&w.chem, mass, th.t) {
        if !(b.n() > 1.0 && !b.empty_at(mass) && starve(w, d, b, &th, &mut tmp.dead)) {
            return Err(LACK);
        }
        th = b.thermal(&w.chem);
    }
    let (chem, subs) = (&w.chem, w.chem.body().subs());
    if b.wet_ks().any(|s| th.t > chem.subs[subs[s]].boil) {
        return Err(PHASE);
    }
    // Its members' microclimates spread over `frost_k` around the temperature it stands at: below the melting point the
    // share (melt - T) / `frost_k` of them freezes, as far as they are active (a dormant body holds little free liquid).
    let act = b.activity(chem, &w.cfg.phys);
    let frozen = |s: usize| ((chem.subs[subs[s]].melt - stands) / w.cfg.phys.frost_k).clamp(0.0, 1.0) * act;
    let f = b.wet_ks().map(frozen).fold(0.0, f64::max);
    if f > 0.0 {
        if f >= 1.0 - 1e-9 || !lose_share(w, d, b, f, PHASE, &mut tmp.dead, &th) {
            return Err(PHASE);
        }
        return Ok(b.thermal(&w.chem));
    }
    Ok(th)
}

/// A node at the plateau state `th` short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The share that dies is the least
/// that lifts every key store to its safe minimum; false if that is all of them.
fn starve(w: &World, d: &mut Chunk, b: &mut Body, th: &Melting, dead: &mut Stores) -> bool {
    let t = th.t;
    let (chem, lay, bc) = (&w.chem, Layout::of(&w.chem), w.chem.body());
    let x = |s: usize| b.store[s] * bc.mm[s];
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&s: &usize| x(s) < b.gene(lay.min + s) * size && !b.burns_at(chem, bc.subs()[s], t);
    let live = (0..bc.n()).filter(short).map(|s| x(s) / (b.gene(lay.min + s) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    *dead = std::array::from_fn(|s| if s < bc.n() && !short(&s) { b.store[s] * f } else { 0.0 });
    f < 1.0 && thin(w, d, b, f, dead, LACK, th) && !b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem))
}

/// What it sheds to the ground with its heat: minerals, built solids and key liquids past their level, and anything
/// else it neither burns nor is made of.
fn shed(w: &World, d: &mut Chunk, b: &mut Body) {
    let (chem, mass) = (&w.chem, b.mass(&w.chem));
    let th = b.thermal(chem);
    let t = th.t;
    let (minerals, subs, matter) = (chem.minerals(), chem.body().subs(), chem.matter());
    let (mut over, mut sheds) = ([0.0f64; BODY_STORES], false);
    for (s, (x, &k)) in over.iter_mut().zip(subs).enumerate() {
        if chem.phase(k, t) == Phase::Gas || matter == Some(k) || b.burns_at(chem, k, t) {
            continue;
        }
        let kept = b.wet >> s & 1 == 1 || minerals.iter().any(|m| m.0 == k) || chem.is_material(k);
        let past = b.store[s] - if kept { b.level(chem, s, mass) } else { 0.0 };
        if past > 0.0 {
            (*x, sheds) = (past, true);
        }
    }
    if sheds {
        to_ground(w, d, b, &th, &over);
    }
}

//! A body's update: its upkeep, its drink, its heat, the checks of its stores, its growth and children, and what it
//! sheds, over the time since its last update (laws.md 6).

use super::pay::{burn, settle_heat, upkeep};
use super::{thin, to_ground, Body, Layout, AGE, COOL_T, KEEP_T, LACK, MARGIN, PHASE, REPAIR, SPENT};
use crate::chem::{Phase, Stores};
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
    /// The time the update integrated, since its last, d: what its brain pays for and learns over (laws.md 5.4)
    pub days: f64,
    /// The heat its upkeep released over the update, J: the most it draws back to warm a cold meal (`fade_meal_heat`)
    pub released: f64,
}

/// The update of body `b` over the time since its last: its drink, upkeep paid by burning (the share unpaid dies,
/// `SPENT`, and the share its wear gives, `AGE`), its heat (`keep_warm`), the checks of its stores, growth
/// (`copy::grow`), children (`copy::copy`) and what it sheds. Returns what it left for its decision, or a death cause.
pub(crate) fn live(w: &World, d: &mut Chunk, b: &mut Body, tmp: &mut LiveTmp) -> Result<Lived, u8> {
    let days = (w.days_now() - b.last).max(1.0);
    b.last = w.days_now();
    let (mass, from) = (b.mass(&w.chem), b.member_body(&w.chem));
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Err(LACK);
    }
    drink(w, d, b, days);
    let (heat, spare) = settle_upkeep(w, d, b, days, b.mass(&w.chem), tmp)?;
    crate::bond::leave_outgrown(w, d, b);
    let stands = keep_warm(w, d, b, heat, days * DAY);
    let t = check_stores(w, d, b, (stands, days), tmp)?;
    crate::copy::grow(w, d, b, days, t);
    if !starved(w, d, b, from, &mut tmp.dead) {
        return Err(SPENT);
    }
    // Offspring are made whenever the brood covers a child, by physics, whatever its brain does (`copy::copy`).
    if b.brood > 0.0 {
        crate::copy::copy(w, d, b);
    }
    shed(w, d, b);
    Ok(Lived { spare, days, released: heat })
}

/// Upkeep over the time, at the temperature it stands at then (`standing`): what its fuel releases is paid (`burn`), the
/// share left unpaid dies, and so does the share its wear gives (`lose_share`). Returns the heat its surviving members
/// released, J (those who died shed theirs through their own surface while they lived), and its spare: what its fuel
/// could release against its upkeep, up to `MARGIN`.
fn settle_upkeep(w: &World, d: &mut Chunk, b: &mut Body, days: f64, mass: f64, tmp: &mut LiveTmp) -> Result<(f64, f64), u8> {
    let (chem, act) = (&w.chem, b.activity(&w.chem, &w.cfg.phys));
    let t = standing(w, d, b, (days, mass), act);
    let cost = upkeep(w, b, days, mass, t, act);
    let spare = b.fuel(chem, t).min(MARGIN * cost) / cost.max(1e-300);
    let (paid, n0) = (burn(w, d, b, cost), b.n());
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
        if f > 1e-9 && !lose_share(w, d, b, f, cause, &mut tmp.dead) {
            return Err(cause);
        }
    }
    Ok((paid * b.n() / n0, spare))
}

/// The temperature its fixed point is found to, K, and the most rounds it takes (`standing`).
const STAND_TOL: f64 = 1e-3;
const STAND_ROUNDS: usize = 64;

/// The temperature body `b` stands at over its update of `days` d, K: the pace of its upkeep (laws.md 6.3). Its heat
/// balance (`Balance`; an animal's energy budget, Porter and Gates 1969) takes it from where it is toward where the heat
/// its upkeep releases holds it, as `keep_warm` holds it (`hold`), and over the time it stands at the mean of that exact
/// solution. Its upkeep goes with the pace of that mean (`Body::pace`), so the temperature is a fixed point, found from
/// where it is the way its heat moves it (the next equilibrium up or down). A body of hours to days stands where its
/// heat holds it whatever the length of its update; one in a closed room, whose walls pass little, moves only part way.
fn standing(w: &World, d: &Chunk, b: &Body, (days, mass): (f64, f64), act: f64) -> f64 {
    let (chem, bal) = (&w.chem, Balance::of(w, b));
    let (coat, bare) = coat(w, d, b, mass);
    let (g, open) = (coat * days * DAY, bare * days * DAY);
    if g <= 0.0 {
        return bal.t0;
    }
    // (Upkeep is its pace times what does not depend on the temperature; what it can release bounds its heat.)
    let (per_pace, fuel) = (upkeep(w, b, days, mass, bal.t0, act) / b.pace(chem, bal.t0), b.fuel(chem, bal.t0));
    let (limits, boil) = ((b.gene(KEEP_T), b.gene(COOL_T)), b.boiling(chem));
    let at = |t: f64| {
        let q = (per_pace * b.pace(chem, t)).min(fuel);
        let h = hold(&bal, (g, open), q, limits);
        bal.mean(h.x, bal.end(h.x, q + h.more - h.sweat).min(boil))
    };
    let mut t = bal.t0;
    for _ in 0..STAND_ROUNDS {
        let next = at(t);
        if (next - t).abs() < STAND_TOL {
            return next;
        }
        t = next;
    }
    t
}

/// The heat balance of a body over a time (laws.md 6.3): C dT/dt = (q - x (T - tg)) / dt from `t0` K toward its ground
/// at `tg` K (Newton's law of cooling), C its heat capacity `cap`, J/K, with a heat `q` J released over the time and a
/// conductance `x` over the time, J/K. Its exact solution goes the share 1 - e^(-x / C) of the way to tg + q / x.
struct Balance {
    t0: f64,
    tg: f64,
    cap: f64,
}

impl Balance {
    /// Of body `b` now, over its cell's ground as Flow's last step left it.
    fn of(w: &World, b: &Body) -> Balance {
        Balance { t0: b.temp(&w.chem), tg: w.t_ground_last(b.cell as usize), cap: b.heat_cap(&w.chem).max(1e-300) }
    }
    /// The share of the way to its steady temperature it goes through `x`.
    fn reach(&self, x: f64) -> f64 {
        one_minus_exp(x / self.cap).max(1e-300)
    }
    /// Where it ends with the heat `q` through `x`, K.
    fn end(&self, x: f64, q: f64) -> f64 {
        self.t0 + (self.tg + q / x - self.t0) * self.reach(x)
    }
    /// The heat that ends it at `t` K through `x`, J.
    fn holds(&self, x: f64, t: f64) -> f64 {
        x * ((t - self.t0) / self.reach(x) + self.t0 - self.tg)
    }
    /// Its mean temperature over the time on its way to `t` K through `x`: of its steady temperature s,
    /// s + (t0 - s) (1 - e^(-x / C)) C / x.
    fn mean(&self, x: f64, t: f64) -> f64 {
        let s = self.t0 + (t - self.t0) / self.reach(x);
        s + (self.t0 - s) * self.reach(x) * self.cap / x
    }
}

/// How a body holds its temperature with a heat (`hold`): the conductance it passes heat through over the time, J/K, the
/// heat it burns more to stay at its `KEEP_T` gene and the heat it sweats away to stay at its `COOL_T` gene, J.
struct Held {
    x: f64,
    more: f64,
    sweat: f64,
}

/// How a body of `bal`, its coat passing `g` J/K and its bare surface `open` over the time, holds its temperature
/// between `keep` and `cool` K with the heat `q` J: below `keep` it burns what holds it there at the end; above `cool` it
/// opens its coat as far as it needs to stand there, up to its bare surface (`Body::bare`: blood to the skin, the coat
/// laid flat; Scholander et al. 1950), and past that sweats the excess away. (On ground at or above `cool` it opens all
/// the way.)
fn hold(bal: &Balance, (g, open): (f64, f64), q: f64, (keep, cool): (f64, f64)) -> Held {
    if bal.end(g, q) < keep {
        Held { x: g, more: bal.holds(g, keep) - q, sweat: 0.0 }
    } else if bal.end(g, q) > cool {
        let opened = if cool > bal.tg { (q / (cool - bal.tg)).clamp(g, open) } else { open };
        let sweat = if bal.end(opened, q) > cool { q - bal.holds(opened, cool) } else { 0.0 };
        Held { x: opened, more: 0.0, sweat }
    } else {
        Held { x: g, more: 0.0, sweat: 0.0 }
    }
}

/// First in its update, it drinks its key liquids from its cell's ground up to the level it keeps (`Body::level`), as
/// it drinks while it loses and builds over the time: its update runs at the activity of what it could drink then
/// (`Body::activity`), whatever the length of the update. As far as the ground holds them liquid, each day at most its
/// uptake gene's share of that level, paying to handle them (`c_handle` per kg); they come with their heat.
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

/// Its heat over the time `dt` s (laws.md 6.3): the heat its upkeep released leaves through its coat (`coat`) to its
/// ground. Its temperature moves from where it is toward where that heat holds it, tg + heat / g, by the share
/// 1 - e^(-g / C) its heat capacity C lets it go in the time (`Balance`): a body of hours to days gets there; a large node
/// in a closed room, whose walls pass little, moves only part way. Below its `KEEP_T` gene it burns what holds it at
/// that temperature at the end; above its `COOL_T` gene it opens its coat and sweats (`hold`), evaporating its key liquid
/// at its latent heat, never below its safe minimum. Its key liquid boils at its boiling point: past it the heat boils it
/// off, below its safe minimum too, and holds the body there; one that boils dry dies there. It then stands at that
/// temperature (`settle_heat`): what it gains from its ground there is never more than the heat it released, unless the
/// ground is the warmer.
fn keep_warm(w: &World, d: &mut Chunk, b: &mut Body, heat: f64, dt: f64) -> f64 {
    let (chem, bal) = (&w.chem, Balance::of(w, b));
    let (coat, bare) = coat(w, d, b, b.mass(chem));
    let (g, open) = (coat * dt, bare * dt);
    if g <= 0.0 {
        return bal.t0;
    }
    // The conductance it passes its heat through, J/K, and the heat that leaves by it, J.
    let h = hold(&bal, (g, open), heat, (b.gene(KEEP_T), b.gene(COOL_T)));
    let q = if h.more > 0.0 {
        heat + burn(w, d, b, h.more)
    } else if h.sweat > 0.0 {
        heat - evaporate(w, d, b, h.sweat, true)
    } else {
        heat
    };
    let boil = b.boiling(chem);
    let q = if bal.end(h.x, q) > boil { q - evaporate(w, d, b, q - bal.holds(h.x, boil), false) } else { q };
    // (One that boils dry dies there, `check_stores`: it stands no warmer, and the rest of its heat stays in its ground.)
    let t = bal.end(h.x, q).min(boil);
    settle_heat(w, d, b, t);
    t
}

/// After its decision (`survey::decide`): what it took and digesting it moved its temperature from `stands`, where its
/// update left it. Over the time to its next update (its cell's step, `World::step_days`) that difference fades by the
/// share e^(-g dt / C), as `keep_warm` lets its heat go: the heat of a meal leaves through its coat to its ground, and
/// a meal colder than the body is warmed by its own heat as it eats, drawn back from its ground, which its upkeep's heat
/// went to (`released`, J, the most it draws). So a body of hours to days stands at `stands` again, and a month's meal
/// eaten at once cools it no more than the meals of the same month in short steps.
pub(crate) fn fade_meal_heat(w: &World, d: &mut Chunk, b: &mut Body, stands: f64, released: f64) {
    let (chem, t) = (&w.chem, b.temp(&w.chem));
    let cap = b.heat_cap(chem).max(1e-300);
    let x = coat(w, d, b, b.mass(chem)).0 * w.step_days(b.cell as usize) * DAY / cap;
    let to = stands + (t - stands) * (-x).exp();
    settle_heat(w, d, b, if to > t { t + (to - t).min(released.max(0.0) / cap) } else { to });
}

/// The heat conductance of its coat and of its bare surface, each in series with the walls around it (3.4), W/K.
fn coat(w: &World, d: &Chunk, b: &Body, mass: f64) -> (f64, f64) {
    let walls = crate::bond::enclosure(d, b).map(|x| x.g);
    let through = |c: f64| walls.map_or(c, |x| 1.0 / (1.0 / c + 1.0 / x));
    let p = &w.cfg.phys;
    (through(b.conductance(p, mass)), through(b.bare(p, mass)))
}

/// It evaporates its key liquids to take `q` J away, at each one's vaporisation enthalpy, never below its safe minimum
/// when it sweats (`spare`; boiling takes all): the vapour goes to its cell's bodies' node with its heat and its latent
/// heat (`Moves::gas`). Returns the heat taken away, J.
fn evaporate(w: &World, d: &mut Chunk, b: &mut Body, q: f64, spare: bool) -> f64 {
    let (chem, i) = (&w.chem, b.cell as usize);
    let (lay, t, size, lift) = (Layout::of(chem), b.temp(chem), b.size(chem), w.lift(i));
    let mut taken = 0.0;
    for p in b.wet_ks() {
        let k = chem.body().subs()[p];
        let s = &chem.subs[k];
        let kept = if spare { b.gene(lay.min + p).max(0.0) * size / s.molar_mass } else { 0.0 };
        let free = (b.store[p] - kept).max(0.0);
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
    f <= 1e-9 || lose_share(w, d, b, f, SPENT, dead)
}

/// Draws of death a body may take in a day, one for each cause (`Rng` keyed by the day and the cause).
const CAUSES: u64 = 8;

/// The share `f` of the members of body `b` dies of `cause`: a node loses that share, with that share of every store
/// (`thin`); an individual dies with that chance. One rule for both, so a node is the average of its members. False if
/// it dies (a node: less than one member is left).
fn lose_share(w: &World, d: &mut Chunk, b: &mut Body, f: f64, cause: u8, dead: &mut Stores) -> bool {
    if b.n() <= 1.0 {
        return Rng::new(w.seed, w.day() * CAUSES + cause as u64, b.id, Purpose::Death).f64() >= f;
    }
    *dead = b.store.map(|a| a * f);
    thin(w, d, b, f, dead, cause)
}

/// A store below its safe minimum kills it (a node: its poorest members, `starve`); a boiled key liquid kills it; below
/// a key liquid's melting point the share of its members that freezes at `stands` K, the temperature its heat holds it
/// at over its ground (`keep_warm`), over the `days` d its update integrates, dies (laws.md 6.5). Returns the
/// temperature it is left at, K.
fn check_stores(w: &World, d: &mut Chunk, b: &mut Body, (stands, days): (f64, f64), tmp: &mut LiveTmp) -> Result<f64, u8> {
    let mass = b.mass(&w.chem);
    if b.lacking(&w.chem, mass, b.temp(&w.chem)) && !(b.n() > 1.0 && !b.empty_at(mass) && starve(w, d, b, &mut tmp.dead)) {
        return Err(LACK);
    }
    let (chem, subs, t) = (&w.chem, w.chem.body().subs(), b.temp(&w.chem));
    if b.wet_ks().any(|s| t > chem.subs[subs[s]].boil) {
        return Err(PHASE);
    }
    // Its members' microclimates spread over `frost_k` around the temperature it stands at: below the melting point the
    // share (melt - T) / `frost_k` of them freezes in a month, as far as they are active (a dormant body holds little
    // free liquid); over the time a share per month that composes, 1 - (1 - that)^(t / month), so a cold spell kills
    // alike in one update or in many (`Span::share64`).
    let act = b.activity(chem, &w.cfg.phys);
    let frozen = |s: usize| ((chem.subs[subs[s]].melt - stands) / w.cfg.phys.frost_k).clamp(0.0, 1.0) * act;
    let f = w.cfg.span(days).share64(b.wet_ks().map(frozen).fold(0.0, f64::max));
    if f > 0.0 {
        if f >= 1.0 - 1e-9 || !lose_share(w, d, b, f, PHASE, &mut tmp.dead) {
            return Err(PHASE);
        }
        return Ok(b.temp(&w.chem));
    }
    Ok(t)
}

/// A node short of key stores: the members poorest in them die (members differ around the mean), so the survivors keep
/// those stores and the dead take their share of the rest. The share that dies is the least that lifts every key store
/// to its safe minimum; false if that is all of them.
fn starve(w: &World, d: &mut Chunk, b: &mut Body, dead: &mut Stores) -> bool {
    let t = b.temp(&w.chem);
    let (chem, lay, bc) = (&w.chem, Layout::of(&w.chem), w.chem.body());
    let x = |s: usize| b.store[s] * bc.mm[s];
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&s: &usize| x(s) < b.gene(lay.min + s) * size && !b.burns_at(chem, bc.subs()[s], t);
    let live = (0..bc.n()).filter(short).map(|s| x(s) / (b.gene(lay.min + s) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    *dead = std::array::from_fn(|s| if s < bc.n() && !short(&s) { b.store[s] * f } else { 0.0 });
    f < 1.0 && thin(w, d, b, f, dead, LACK) && !b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem))
}

/// What it sheds to the ground with its heat: minerals, built solids and key liquids past their level, and anything
/// else it neither burns nor is made of.
fn shed(w: &World, d: &mut Chunk, b: &mut Body) {
    let (chem, mass, t) = (&w.chem, b.mass(&w.chem), b.temp(&w.chem));
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
        to_ground(w, d, b, &over);
    }
}

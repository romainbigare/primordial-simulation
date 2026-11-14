//! What a slot of takers does with its surplus in a month (laws.md 3b): the share `YOUNG` of the fuel its store gained
//! and what its store holds beyond `store_most` build body matter, first into its young that live, then into newborns
//! of its adults, from the body matter it took and did not burn; its young that grow past its adults' mass join them;
//! the rest of the fuel goes back to its store and what is left passes to the ground. Building is limited by the rate
//! of living (West, Brown and Enquist 2001).

use super::body::Body;
use super::gut::{burn, Gut};
use super::{adult, sub, Fauna, BODY, CHILD, FS, N_WHOLE, NF, STORE, YOUNG};
use crate::life::{Books, Parts, WET_TISSUE};
use crate::math::dec;
use crate::rng::Rng;
use crate::units::DAY;
use crate::World;

/// What slot `x` has for growth in a month: per group (its adults, its young) the fuel its store gained, mol; and the
/// body matter its groups took and did not burn, mol.
pub(super) struct Surplus {
    pub(super) gained: [f64; 2],
    pub(super) matter: f64,
}

/// The most living mass a member like `b` builds a s, kg: the power its body's network delivers, Kleiber's rest
/// (`c_basal` + `c_muscle`) × m^`upkeep_exp` at the pace of its body's temperature, over the energy a kg of living
/// tissue holds and costs to build (`Parts::living_cost`): the cap of West, Brown and Enquist 2001 (dm/dt at most
/// B0 m^(3/4) / E_m, E_m the energy content of tissue, 7e6 J a kg; Gillooly et al. 2002 for the temperature).
fn cap(w: &World, b: &Body, cost: f64) -> f64 {
    let ph = &w.cfg.phys;
    if b.n <= 0.0 { 0.0 } else { (ph.c_basal + ph.c_muscle) * b.m.powf(ph.upkeep_exp) * b.pace / DAY / cost }
}

/// Slot `x` of `f` (its adults `bodies.0`, its young `bodies.1`) grows on its `surplus` over `dt` s: its pot of fuel is
/// the share `YOUNG` of what each group's store gained and what each store holds beyond `store_most` of its group's
/// living mass; the pot pays the work of building, `c_copy` a kg of body matter, by burning (`gut::burn`), and the
/// body matter comes from what it took. First its young grow, each at most at its cap (`cap`); of them the share that
/// crosses its adults' mass in the month joins its adults (von Foerster's flux across a mass: their masses taken as
/// even in log mass between a newborn's and an adult's, the share min(1, Δln m / ln(M / m0))), with its share of the
/// young's amounts; then its adults bear newborns of `CHILD` of their mass, each adult at most at the share `bearing`
/// of its cap.
/// Below `N_WHOLE` the young that mature are a binomial draw from `r` and the newborns the whole part of what was built
/// and one more by a draw on the rest, so a draw never makes matter; a count that falls below `N_WHOLE` becomes whole
/// (`whole`). What is left of the pot goes back to the stores up to their most, the rest of it and of the matter to
/// the remains. Returns the newborns.
pub(super) fn grow(w: &World, (p, gut): (&Parts, &Gut), (f, x, bodies): (&mut Fauna, usize, (&Body, &Body)), surplus: &Surplus, (r, b): (&mut Rng, &mut Books<0>)) -> f64 {
    let (chem, ph, dt) = (&w.chem, &w.cfg.phys, w.cfg.calendar().month as f64);
    let (sub, mm, g) = (sub(p), super::mm(chem, p), f.genes[x]);
    let (old, young) = (x, FS + x);
    let store_most = |f: &Fauna, k: usize| ph.store_most * f.living_of(&mm, k) / mm[STORE];
    let mut pot = 0.0;
    for (k, gained) in [old, young].into_iter().zip(surplus.gained) {
        let to_young = (g[YOUNG].clamp(0.0, 1.0) as f64 * gained).min(f.mol[k][STORE].max(0.0));
        let over = (f.mol[k][STORE] - to_young - store_most(f, k)).max(0.0);
        f.mol[k][STORE] -= to_young + over;
        pot += to_young + over;
    }
    let cost = p.living_cost(chem, ph);
    let (per_kg, kg_of) = (ph.c_copy * (1.0 - WET_TISSUE), mm[BODY] / (1.0 - WET_TISSUE));
    let mut matter = surplus.matter;
    // Builds at most `upto` kg of living mass into group `k`, from the pot's fuel and the matter left; returns the kg.
    let mut build = |f: &mut Fauna, k: usize, upto: f64, pot: &mut f64, matter: &mut f64| -> f64 {
        let afford = *pot * gut.fuel(chem) / per_kg;
        let kg = upto.min(*matter * kg_of).min(afford).max(0.0);
        let mut fuel = [*pot, 0.0];
        let unpaid = burn(chem, gut, (&sub, &mut fuel), kg * per_kg, b);
        let kg = kg * if kg > 0.0 { 1.0 - unpaid / (kg * per_kg) } else { 0.0 };
        *pot = fuel[STORE];
        *matter -= kg / kg_of;
        f.mol[k][BODY] += kg / kg_of;
        kg
    };
    // The young grow, and those past an adult's mass join the adults.
    let (m_adult, m_child) = (adult(&g), adult(&g) * dec(g[CHILD]));
    if f.n[young] > 0.0 {
        let before = f.living_of(&mm, young);
        let upto = cap(w, bodies.1, cost) * f.n[young] * dt;
        build(f, young, upto, &mut pot, &mut matter);
        let (m, grown) = (f.living_of(&mm, young) / f.n[young], (f.living_of(&mm, young) / before.max(1e-300)).ln());
        let share = if m >= m_adult { 1.0 } else { (grown / (m_adult / m_child).ln().max(1e-9)).clamp(0.0, 1.0) };
        let n = f.n[young];
        let matured = if n < N_WHOLE { r.binomial(n as u32, share) as f64 } else { n * share };
        let part = matured / n;
        for q in 0..NF {
            let moved = f.mol[young][q] * part;
            (f.mol[young][q], f.mol[old][q]) = (f.mol[young][q] - moved, f.mol[old][q] + moved);
        }
        (f.n[young], f.n[old]) = (n - matured, f.n[old] + matured);
    }
    // The adults bear newborns, each at the share `bearing` of its cap.
    let mut born = 0.0;
    if f.n[old] > 0.0 && m_child > 0.0 {
        let upto = cap(w, bodies.0, cost) * bearing(m_child / m_adult) * f.n[old] * dt;
        let kg = build(f, young, upto, &mut pot, &mut matter);
        born = kg / m_child;
        if f.n[young] + born < N_WHOLE {
            born = born.floor() + (r.f64() < born.fract()) as u8 as f64;
        }
        f.n[young] += born;
    }
    for k in [old, young] {
        f.n[k] = whole(f.n[k], r);
        let back = pot.min((store_most(f, k) - f.mol[k][STORE]).max(0.0));
        (f.mol[k][STORE], pot) = (f.mol[k][STORE] + back, pot - back);
    }
    b.dead[sub[STORE]] += pot;
    b.dead[sub[BODY]] += matter;
    born
}

/// The newborn mass a mother bears a s over her cap (`cap`), where her newborns hold the share `child` of her mass M:
/// an unborn grows from nothing by the law every body grows by (West, Brown and Enquist 2001: dm/dt = a m^(3/4)
/// (1 − u), u = (m / M)^(1/4)), fed from her pot, so it reaches the newborn's u_c = `child`^(1/4) after
/// t = 4 M^(1/4) (−ln(1 − u_c)) / a; its need a M^(3/4) u³ (1 − u) is greatest at its birth (or at u = 3/4), and her cap,
/// a M^(3/4), feeds at once a litter of 1 / that greatest u³ (1 − u). Litter after litter, she bears u_c⁴ M a litter
/// each t: the share u_c⁴ / (4 (−ln(1 − u_c)) max u³ (1 − u)) of her cap, a quarter for a small newborn.
fn bearing(child: f64) -> f64 {
    let u = child.clamp(0.0, 1.0).powf(0.25);
    if u <= 0.0 || u >= 1.0 {
        return if u >= 1.0 { 0.0 } else { 0.25 };
    }
    let most = |v: f64| v.powi(3) * (1.0 - v);
    u.powi(4) / (4.0 * -(1.0 - u).ln() * most(u.min(0.75)))
}

/// A count `n` below `N_WHOLE` that is not whole becomes whole by a draw from `r` that keeps its mean: its whole part,
/// and one more with the chance of the rest (never rounded up as a rule).
pub(super) fn whole(n: f64, r: &mut Rng) -> f64 {
    if n < N_WHOLE && n.fract() != 0.0 { n.floor() + (r.f64() < n.fract()) as u8 as f64 } else { n }
}

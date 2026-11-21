//! What a slot of cover pays to live in a month (laws.md 3): its upkeep, paid by burning its fuel, and what dies of it:
//! the share unpaid, the share worn out, the share frozen.

use super::budget::{self, DIED, UPKEEP, UPKEEP_PARTS};
use super::build::per_input;
use super::grow::{Books, Slot};
use super::{dec, Cover, Parts, FROST, FUEL, GAS, LIGHT, MOIST, NG, REPAIR, WALL};
use crate::chem::Chemistry;
use crate::life::{AGE, DROWNED, PHASE, SPENT};
use crate::units::{DAY, FERMENT_YIELD};
use crate::World;

/// Slot `x` pays its upkeep over `dt` at `t` K by burning its fuel (its plants without the oxidising gas at the yield
/// of their fuel without it, `FERMENT_YIELD`), then loses to its cell's remains the plants on its
/// ground under the liquid past its roots' reach whose store is spent (`airless`, `DROWNED`), the share unpaid
/// (`SPENT`), the share its wear gives (`AGE`) and the share frozen (`PHASE`, kept in `Slot::frozen`). Returns the kg
/// lost by cause, and the upkeep, J.
pub(super) fn upkeep(w: &World, c: &mut Cover, p: &Parts, x: usize, s: &mut Slot, (t, dt): (f64, f64), b: &mut Books) -> ([f64; crate::life::DEATHS], f64) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (burn, days) = (&chem.reactions[p.burn], dt / DAY);
    let pace = burn.relative_rate(t, ph.t_cat);
    let k = ph.repair(g[REPAIR] as f64) * pace;
    let living = upkeep_rates(chem, ph, p, &g, pace).map(|x| s.living * days * x);
    let active = living.iter().sum::<f64>() + ph.c_leaf * s.leaf * days * k;
    // Its conducting walls breathe whatever the season gives (`stand::sapwood`), at the burning's pace. Its fine roots,
    // as many as the liquid it drew this month needs (`root_uptake`; the minerals come in that liquid by mass flow,
    // Barber 1995), breathe as living cover and are built anew `root_turnover` times a year (Gill and Jackson 2000).
    let roots = s.drunk / dt / uptake(ph, &g);
    let renew = roots * ph.root_turnover * p.living_cost(chem, ph) / w.cfg.months_per_year as f64;
    // Its living cover keeps its liquid whatever its ground holds (`WET_TISSUE`), so it breathes at its temperature's
    // pace in a dry month too: only its gain follows the liquid it can draw (Flexas and others 2006; Atkin and Macherel
    // 2009). A dry seed's upkeep falls with its liquid (`Physics::activity`, `lying`); a leaf's does not.
    let parts: [f64; UPKEEP_PARTS] = [living[0], living[1], living[2], ph.c_leaf * s.leaf * days * k, ph.c_sapwood * s.sapwood * days * pace, ph.c_basal * roots * days * pace, renew];
    let cost: f64 = parts.iter().sum();
    // Its plants under the liquid past their roots' reach (`Slot::airless`) get no oxidising gas: their tissue frees
    // only `FERMENT_YIELD` of its fuel's energy for its upkeep, so it burns that many times more fuel for it (what it
    // ferments is booked burnt: its products are burnt about it).
    let dear = 1.0 + s.airless * (1.0 / FERMENT_YIELD - 1.0);
    let (units, fuel) = (dear * cost / -chem.dh(p.burn), per_input(burn, p.sub[FUEL]).unwrap_or(1.0));
    // (What it burns is worked out first: one less a share near one would lose a small fuel to rounding.) It burns no
    // more than its fuel and the gas the cell's share of its air node still offers.
    let held_fuel = c.mol[x][FUEL].max(0.0);
    let burnt = (units * fuel).min(held_fuel).min(b.room(&p.sub, burn) * fuel);
    let short = if units > 0.0 { 1.0 - burnt / (units * fuel) } else { 0.0 };
    let e = p.content(chem);
    let unburnt = budget::held(&e, &c.mol[x]);
    b.run(chem, &p.sub, &mut c.mol[x], burn, burnt / fuel);
    // What the burning took of its content, shared out over the parts of the upkeep as they asked for it.
    let paid = budget::held(&e, &c.mol[x]) - unburnt;
    (0..UPKEEP_PARTS).for_each(|q| b.budget[UPKEEP + q] += if cost > 0.0 { paid * parts[q] / cost } else { 0.0 });
    // Members of a field wear out at the rate of living (Rubner 1908), a dormant one at least at `Physics::dormant` of
    // the active rate; below its frost gene, hardened by its dormancy, its frozen share dies (laws.md 3).
    let worn = active * s.act.max(ph.dormant) / (s.living * ph.e_life * dec(g[REPAIR])).max(1e-300);
    s.frozen = super::frozen(ph, chem.subs[p.liquid].melt, g[FROST] as f64, (t, s.act));
    let frozen = s.frozen;
    let drowned = airless(c, x, s, burnt / held_fuel.max(1e-300) / (FERMENT_YIELD * dear));
    let mut lost = [0.0; crate::life::DEATHS];
    let mut left = 1.0;
    for (f, cause) in [(drowned, DROWNED), (short, SPENT), (crate::math::one_minus_exp(worn), AGE), (frozen, PHASE)] {
        lost[cause as usize] = f * left;
        left *= 1.0 - f;
    }
    let dies = 1.0 - left;
    if dies > 0.0 {
        let mm = p.mm(chem);
        // The worn out are the stand's own turnover, spread over it; the rest die of an event that may open a gap.
        c.fell(ph, &mm, x, (dies - lost[AGE as usize]).max(0.0));
        let all = c.kg(&mm, x);
        lost.iter_mut().for_each(|l| *l *= all);
        // What dies are whole plants: each one left keeps its mass.
        c.plants[x].iter_mut().for_each(|n| *n *= left);
        // The dead of plants no taller than `surface_m` stand, cured, as their dead leaves do (`season::shed_to`): their
        // body matter and walls stay among the cover's own until they fall; their fuel goes to the ground.
        let stands = c.height(ph, &mm, chem.subs[p.sub[WALL]].density[0], x) <= ph.surface_m;
        for (q, &per) in e.iter().enumerate() {
            let m = c.mol[x][q].max(0.0) * dies;
            c.mol[x][q] -= m;
            if stands && q != FUEL {
                c.cured[q] += m;
            } else {
                b.dead[p.sub[q]] += m;
                b.budget[DIED] -= m * per;
            }
        }
    }
    (lost, dear * cost)
}

/// The share of slot `x` of cover `c` (its slot `s` in the month) that dies with its store spent on ground under the
/// liquid past its roots' reach, its plants there having burnt the share `burnt` of their fuel this month. They (the
/// share `Slot::airless`) gain nothing, their roots getting no oxidising gas (`Slot::work`), and live on their store,
/// paying their upkeep at the yield of their fuel without that gas (`FERMENT_YIELD`, `upkeep`): of its store each
/// burns that share (`Cover::drained`; those that came to stand there since its last month come with all of theirs). When they have
/// burnt all of it they die as whole plants, the line rising to where none stands so (`Slot::rise`). The burning keeps
/// its pace's fall with the cold, so a flood in the cold season, when the plants' tissue burns slowly, takes far longer
/// to kill them (Kozlowski 1997; Glenz and others 2006).
fn airless(c: &mut Cover, x: usize, s: &Slot, burnt: f64) -> f64 {
    let (now, was) = (s.airless as f32, c.airless[x]);
    if now <= 0.0 {
        (c.airless[x], c.drained[x]) = (0.0, 0.0);
        return 0.0;
    }
    if now > was {
        c.drained[x] *= was / now;
    }
    c.drained[x] += burnt as f32;
    if c.drained[x] < 1.0 {
        c.airless[x] = now;
        return 0.0;
    }
    (c.line[x], c.airless[x], c.drained[x]) = (s.rise as f32, 0.0, 0.0);
    s.airless
}

/// What one kg of the living cover of slot `x` costs to keep for `dt` at `t` K in a month it draws no liquid, J: its
/// upkeep (`upkeep_rates`), its capture area's (`c_leaf`) and that of the conducting walls that feed it (`stand::sapwood`
/// of a kg's capture area, as tall as its plants); its fine roots draw nothing then. Defined with no living cover too.
pub(super) fn keep_per_kg(w: &World, c: &Cover, p: &Parts, x: usize, (t, dt): (f64, f64)) -> f64 {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (mm, rho, days) = (p.mm(chem), chem.subs[p.sub[super::WALL]].density[0], dt / DAY);
    let pace = chem.reactions[p.burn].relative_rate(t, ph.t_cat);
    let leaf = dec(g[super::LEAF]);
    let sapwood = super::stand::sapwood(ph, (leaf, c.lift(ph, &mm, rho, x)), f64::INFINITY, rho);
    let living: f64 = upkeep_rates(chem, ph, p, &g, pace).iter().sum();
    (living + ph.c_leaf * leaf * ph.repair(g[REPAIR] as f64) * pace + ph.c_sapwood * sapwood * pace) * days
}

/// The share of what a leaf of a kind of genes `g` stores at `t` K over `dt` that the fine roots drawing its liquid cost,
/// where a unit of its light reaction draws `drink` mol of the key liquid: the roots that liquid needs (`uptake`), breathing
/// `c_basal` a kg a day at the burning's pace and built anew `root_turnover` times a year (`upkeep`), over what the
/// unit stores. It goes with the air's thirst, not with the leaf: from 1 a leaf's pores lose more than they bring.
pub(super) fn root_share(w: &World, p: &Parts, (t, dt): (f64, f64), drink: f64, g: &[f32; NG]) -> f64 {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let burn = &chem.reactions[p.burn];
    let pace = burn.relative_rate(t, ph.t_cat);
    let per_kg = ph.c_basal * dt / DAY * pace + ph.root_turnover * p.living_cost(chem, ph) / w.cfg.months_per_year as f64;
    drink / (dt * uptake(ph, g)) * per_kg / chem.dh(p.light)
}

/// The key liquid a kg of living fine roots of a kind of genes `g` carries, mol/s: `root_uptake` times the share of
/// their volume that is not gas space (1 − `GAS`). The layer that keeps the oxidising gas in a root's gas spaces also
/// keeps the liquid and its minerals out of it (a barrier to radial loss: Colmer 2003; Garthwaite and others 2008),
/// taken here to tighten as the gas spaces grow: so a kind that breathes in full pores needs more fine roots for the
/// same liquid, on any ground.
pub(super) fn uptake(ph: &crate::Physics, g: &[f32; NG]) -> f64 {
    ph.root_uptake * (1.0 - g[GAS] as f64)
}

/// The upkeep of a kg of living cover of genes `g` a day at full activity, at `pace` of its burning's pace at `t_cat`,
/// J: basal (`c_basal`), its capacity (a capacity that could store `cap` W per kg costs the share `c_capacity` of that) and
/// its firmness (dearer the colder it stands, as a firmer catalyst costs more), each times its repair factor.
pub(super) fn upkeep_rates(chem: &Chemistry, ph: &crate::Physics, p: &Parts, g: &[f32; NG], pace: f64) -> [f64; 3] {
    let cap = dec(g[LIGHT]) * dec(g[MOIST]) * chem.dh(p.light) * DAY;
    let firm = 1.0 + (chem.subs[p.liquid].melt - g[FROST] as f64).max(0.0) / ph.c_firm;
    let k = ph.repair(g[REPAIR] as f64) * pace;
    [ph.c_basal, ph.c_capacity * cap, ph.c_capacity * cap * (firm - 1.0)].map(|x| x * k)
}

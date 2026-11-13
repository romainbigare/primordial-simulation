//! What a slot of cover pays to live in a month (laws.md 3): its upkeep, paid by burning its fuel, and what dies of it:
//! the share unpaid, the share worn out, the share frozen.

use super::budget::{self, DIED, UPKEEP, UPKEEP_PARTS};
use super::build::per_input;
use super::grow::{Books, Slot};
use super::{dec, Cover, Parts, FROST, FUEL, LIGHT, MOIST, NG, NP, REPAIR};
use crate::chem::Chemistry;
use crate::life::{AGE, PHASE, SPENT};
use crate::units::DAY;
use crate::World;

/// Slot `x` pays its upkeep over `dt` at `t` K by burning its fuel, then loses the share unpaid (`SPENT`), the share
/// its wear gives (`AGE`) and the share frozen (`PHASE`) to its cell's remains. Returns the kg lost by cause, and the
/// upkeep, J.
pub(super) fn upkeep(w: &World, c: &mut Cover, p: &Parts, x: usize, s: &Slot, (t, dt): (f64, f64), b: &mut Books) -> ([f64; 6], f64) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (burn, days) = (&chem.reactions[p.burn], dt / DAY);
    let pace = burn.arrhenius(t.max(1.0)) / burn.arrhenius(ph.t_cat);
    let k = ph.repair(g[REPAIR] as f64) * pace;
    let living = upkeep_rates(chem, ph, p, &g, pace).map(|x| s.living * days * x);
    let active = living.iter().sum::<f64>() + ph.c_leaf * s.leaf * days * k;
    // Its conducting walls breathe whatever the season gives (`stand::sapwood`), at the burning's pace. Its fine roots,
    // as many as the liquid it drew this month needs (`root_uptake`; the minerals come in that liquid by mass flow,
    // Barber 1995), breathe as living cover and are built anew `root_turnover` times a year (Gill and Jackson 2000).
    let roots = s.drunk / dt / ph.root_uptake;
    let renew = roots * ph.root_turnover * super::build::living_cost(chem, ph, p) / w.cfg.months_per_year as f64;
    let parts: [f64; UPKEEP_PARTS] = [living[0] * s.act, living[1] * s.act, living[2] * s.act, ph.c_leaf * s.leaf * days * k * s.act, ph.c_sapwood * s.sapwood * days * pace, ph.c_basal * roots * days * pace, renew];
    let cost: f64 = parts.iter().sum();
    let (units, fuel) = (cost / -chem.dh(p.burn), per_input(burn, p.sub[FUEL]).unwrap_or(1.0));
    // (What it burns is worked out first: one less a share near one would lose a small fuel to rounding.) It burns no
    // more than its fuel and the gas the cell's share of its air node still offers.
    let burnt = (units * fuel).min(c.mol[x][FUEL].max(0.0)).min(b.room(&p.sub, burn) * fuel);
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
    let frozen = super::frozen(ph, chem.subs[p.liquid].melt, g[FROST] as f64, (t, s.act));
    let mut lost = [0.0; 6];
    let mut left = 1.0;
    for (f, cause) in [(short, SPENT), (crate::math::one_minus_exp(worn), AGE), (frozen, PHASE)] {
        lost[cause as usize] = f * left;
        left *= 1.0 - f;
    }
    let dies = 1.0 - left;
    if dies > 0.0 {
        let mm = p.mm(chem);
        let all = c.kg(&mm, x);
        lost.iter_mut().for_each(|l| *l *= all);
        // What dies are whole plants: each one left keeps its mass.
        c.plants[x] *= left;
        let alive = budget::held(&e, &c.mol[x]);
        for q in 0..NP {
            let m = c.mol[x][q].max(0.0) * dies;
            (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - m, b.dead[p.sub[q]] + m);
        }
        b.budget[DIED] += budget::held(&e, &c.mol[x]) - alive;
    }
    (lost, cost)
}

/// The upkeep of a kg of living cover of genes `g` a day at full activity, at `pace` of its burning's pace at `t_cat`,
/// J: basal (`c_basal`), its capacity (a capacity that could store `cap` W per kg costs the share `c_light` of that) and
/// its firmness (dearer the colder it stands, as a firmer catalyst costs more), each times its repair factor.
pub(super) fn upkeep_rates(chem: &Chemistry, ph: &crate::Physics, p: &Parts, g: &[f32; NG], pace: f64) -> [f64; 3] {
    let cap = dec(g[LIGHT]) * dec(g[MOIST]) * chem.dh(p.light) * DAY;
    let firm = 1.0 + (chem.subs[p.liquid].melt - g[FROST] as f64).max(0.0) / ph.c_firm;
    let k = ph.repair(g[REPAIR] as f64) * pace;
    [ph.c_basal, ph.c_light * cap, ph.c_light * cap * (firm - 1.0)].map(|x| x * k)
}

//! What a slot of cover pays to live in a month (laws.md 3): its upkeep, paid by burning its fuel, and what dies of it:
//! the share unpaid, the share worn out, the share frozen.

use super::budget::{self, DIED, UPKEEP, UPKEEP_PARTS};
use super::build::per_input;
use super::grow::{Books, Slot};
use super::{dec, Cover, Parts, CHANNEL, FROST, FUEL, LIGHT, MOIST, NG, NP, REPAIR};
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
    // as many as the liquid it drew this month needs (`root_uptake`, of the share of their section that is not open
    // channel; the minerals come in that liquid by mass flow, Barber 1995), breathe as living cover and are built anew
    // `root_turnover` times a year (Gill and Jackson 2000).
    // Its roots are as many as the draw they would take up with the gas (`soaked::uptake` of what they drew).
    let up = super::soaked::uptake(chem, p, s.drowned).max(1e-300);
    let roots = s.drunk / up / dt / (ph.root_uptake * (1.0 - g[CHANNEL] as f64));
    let renew = roots * ph.root_turnover * p.living_cost(chem, ph) / w.cfg.months_per_year as f64;
    // Its roots in full pores beyond its channels (`soaked::drowned`) split the fuel they would burn (`Parts::split`):
    // the same fuel, less work (their uptake falls instead, `soaked::uptake`); where the chemistry has no such reaction
    // they use none.
    let per_fuel = |r: usize| super::soaked::per_fuel(chem, p, r);
    let root_work = [ph.c_basal * roots * days * pace, renew].map(|x| x * if p.split.is_some() { 1.0 } else { 1.0 - s.drowned });
    let parts: [f64; UPKEEP_PARTS] = [living[0] * s.act, living[1] * s.act, living[2] * s.act, ph.c_leaf * s.leaf * days * k * s.act, ph.c_sapwood * s.sapwood * days * pace, root_work[0], root_work[1]];
    // Its upkeep as the fuel it burns would free burnt with the gas, J, and of it what splits.
    let cost: f64 = parts.iter().sum();
    let split = if p.split.is_some() { (root_work[0] + root_work[1]) * s.drowned } else { 0.0 };
    // It pays the share `paid` of it: no more than its fuel (`need` mol) and the gas the cell's share of its air node
    // still offers for what burns (`units` of the burning) allow.
    let (units, need) = ((cost - split) / -chem.dh(p.burn), cost / per_fuel(p.burn));
    let paid = if need > 0.0 { (c.mol[x][FUEL].max(0.0) / need).min(b.room(&p.sub, burn) / units.max(1e-300)).min(1.0) } else { 0.0 };
    let short = if cost > 0.0 { 1.0 - paid } else { 0.0 };
    let e = p.content(chem);
    let unburnt = budget::held(&e, &c.mol[x]);
    b.run(chem, &p.sub, &mut c.mol[x], burn, units * paid);
    if let Some(r) = p.split {
        let rx = &chem.reactions[r];
        b.run(chem, &p.sub, &mut c.mol[x], rx, split / per_fuel(p.burn) * paid / per_input(rx, p.sub[FUEL]).unwrap_or(1.0));
    }
    // What the burning took of its content, shared out over the parts of the upkeep as they asked for it.
    let spent = budget::held(&e, &c.mol[x]) - unburnt;
    (0..UPKEEP_PARTS).for_each(|q| b.budget[UPKEEP + q] += if cost > 0.0 { spent * parts[q] / cost } else { 0.0 });
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
/// J: basal (`c_basal`), its capacity (a capacity that could store `cap` W per kg costs the share `c_capacity` of that) and
/// its firmness (dearer the colder it stands, as a firmer catalyst costs more), each times its repair factor.
pub(super) fn upkeep_rates(chem: &Chemistry, ph: &crate::Physics, p: &Parts, g: &[f32; NG], pace: f64) -> [f64; 3] {
    let cap = dec(g[LIGHT]) * dec(g[MOIST]) * chem.dh(p.light) * DAY;
    let firm = 1.0 + (chem.subs[p.liquid].melt - g[FROST] as f64).max(0.0) / ph.c_firm;
    let k = ph.repair(g[REPAIR] as f64) * pace;
    [ph.c_basal, ph.c_capacity * cap, ph.c_capacity * cap * (firm - 1.0)].map(|x| x * k)
}

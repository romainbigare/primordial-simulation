//! The third stage of the flames' month (laws.md 3c): each cell burns its own share, the remains on its ground and
//! what of its cover stands in the flame, and scorches what stands under the height its plume kills at; booked through
//! the cell's view as the cover's books are (`life::book`), the heat freed to its air.

use super::{Bed, FLAME_EXP, FLAME_K, KILLED, KILLED_WALLS, LIVING, PLANTS, REMAINS, RESIDENCE_PER_M, SCORCHED, SCORCH_EXP, SCORCH_K, W_PER_KW};
use crate::chem::{Chemistry, Phase};
use crate::cover::{per_input, Books, Parts, BUDGET, BURNT, FUEL, MATTER, NP, WALL};
use crate::life::{Chunk, FLAME};
use crate::store::{Class, Node};
use crate::{World, AIR};

/// How high a flame through a bed reaches, m, and how high its plume kills the leaves it meets, m.
#[derive(Clone, Copy, Debug)]
pub struct Reach {
    pub flame: f64,
    pub scorch: f64,
}

/// The reach of a flame through `bed` with no wind: it moves at R = `flux_ahead` h δ / (τ Q) (Rothermel 1972: what the
/// burning kg sends ahead over what lighting a kg takes, a bed of depth δ, its load at `litter_rho`, burning for τ,
/// `RESIDENCE_PER_M` × `fine_d`), its intensity is I = h w R (Byram 1959), its flame 0.0775 I^0.46 long (Byram 1959) and
/// its plume kills leaves up to 0.1483 I^(2/3) (Van Wagner 1973), I in kW a m.
pub fn reach(ph: &crate::Physics, bed: &Bed) -> Reach {
    let depth = bed.load / ph.litter_rho;
    let speed = ph.flux_ahead * bed.heat * depth / (RESIDENCE_PER_M * ph.fine_d * bed.light.max(f64::MIN_POSITIVE));
    let kw = bed.heat * bed.load * speed / W_PER_KW;
    Reach { flame: FLAME_K * kw.powf(FLAME_EXP), scorch: SCORCH_K * kw.powf(SCORCH_EXP) }
}

/// Burn `mol` of substance `k` in amounts `m` (of the substances `sub`; an empty `sub`: all from outside) by its burning
/// reaction (`Chemistry::burner`), no more than the gas the cell's share of its air offers (`Books::room`). Returns the
/// mol burnt.
fn burn_up(chem: &Chemistry, b: &mut Books, (sub, m): (&[usize], &mut [f64]), k: usize, mol: f64) -> f64 {
    let Some(r) = chem.burner(k) else { return 0.0 };
    let rx = &chem.reactions[r];
    let n = per_input(rx, k).unwrap_or(1.0);
    let x = (mol.max(0.0) / n).min(b.room(sub, rx));
    b.run(chem, sub, m, rx, x);
    x * n
}

/// Cell `i` of chunk `d` burns the share `f` of its land, its remains lying as `bed`: there all its remains burn; of
/// each slot of its cover the soft part of its crown (`contract::crown`, spread evenly over it) that stands in the
/// flame burns, what stands between the flame and the scorch height dies to its remains, and a slot whose whole crown
/// stands under the scorch height loses its plants there (a young plant must outgrow the flame between flames; Bond and
/// Midgley 2001); its seeds lying survive (they lie in the top of the ground, which a passing flame barely warms). Gases
/// go to its air, what is not a gas to its ground (`life::book`), and the heat the burning frees to its air node.
pub fn burn(w: &World, d: &mut Chunk, p: &Parts, i: usize, (f, bed): (f64, &Bed)) {
    let (chem, mm, e) = (&w.chem, p.mm(&w.chem), p.content(&w.chem));
    let r = reach(&w.cfg.phys, bed);
    let crowns: Vec<(usize, (f64, f64))> = d.at(i).cover.held().map(|x| (x, crate::contract::crown(w, i, d.at(i), p, x))).collect();
    let air = crate::contract::air(w, d, i);
    let t = w.thaw_last(i).0;
    let (c, moves, counts) = d.cover_with(i);
    moves.open(w, i, Node::new(Class::Life, i));
    let mut b = Books::new(air, [0.0; BUDGET.len()]);
    let mut flows = [0.0; super::FLOWS.len()];
    for q in [MATTER, WALL] {
        let held = moves.amount(w, Node::ground(i), p.sub[q]).max(0.0);
        flows[REMAINS] += burn_up(chem, &mut b, (&[], &mut []), p.sub[q], held * f) * mm[q];
    }
    let before = c.content(&e);
    for (x, (base, top)) in crowns {
        let [lit, under] = [r.flame, r.scorch].map(|h| f * crate::contract::reached((0.0, h), 0.0, (base, top)));
        let m0 = c.mol[x];
        for q in [FUEL, MATTER] {
            flows[LIVING] += burn_up(chem, &mut b, (&p.sub, &mut c.mol[x]), p.sub[q], m0[q] * lit) * mm[q];
        }
        let mut dead = [0.0; NP];
        dead[MATTER] = (m0[MATTER] * (under - lit)).min(c.mol[x][MATTER]).max(0.0);
        flows[SCORCHED] += dead[MATTER] * mm[MATTER];
        if top <= r.scorch {
            // The plants of the burnt share die: their walls, and the fuel the flame left in them.
            (dead[FUEL], dead[WALL]) = ((m0[FUEL] * (f - lit)).min(c.mol[x][FUEL]).max(0.0), m0[WALL] * f);
            flows[KILLED] += dead[FUEL] * mm[FUEL] + dead[WALL] * mm[WALL];
            flows[KILLED_WALLS] += dead[WALL] * mm[WALL];
            flows[PLANTS] += c.plants[x] * f;
            c.plants[x] *= 1.0 - f;
        }
        (0..NP).for_each(|q| (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - dead[q], b.dead[p.sub[q]] + dead[q]));
    }
    b.budget[BURNT] += c.content(&e) - before;
    crate::cover::empty_spent(c, p, (&mm, &e), w.cfg.variety, &mut b);
    counts.cover_lost[FLAME as usize] += flows[LIVING] + flows[SCORCHED] + flows[KILLED];
    counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, x)| *a += x);
    counts.flame.iter_mut().zip(flows).for_each(|(a, x)| *a += x);
    counts.flame_air.resize(chem.ns(), 0.0);
    for k in (0..chem.ns()).filter(|&k| chem.phase(k, t) == Phase::Gas) {
        counts.flame_air[k] -= b.ext[k];
    }
    counts.burnt.push((i as u32, f));
    // The heat the burning frees goes to the air over the cell (its plume); the rest closes with the ground as the
    // cover's books close (`life::book`).
    let freed = std::mem::take(&mut b.heat);
    c.e = crate::life::book(w, moves, (i, t), (c.e, 0.0), c.cap(chem, p) * t, &b);
    moves.give(w, Node::air(w.mesh.up(i, 0, AIR)), 0, 0.0, freed);
}

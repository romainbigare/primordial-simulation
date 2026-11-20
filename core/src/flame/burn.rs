//! The third stage of the flames' month (laws.md 3c): each cell burns its own share, the remains on its ground and
//! what of its cover stands in the flame, and scorches what stands under the height its plume kills at; booked through
//! the cell's view as the cover's books are (`life::book`), the heat freed to its air.

use super::{Bed, FLAME_EXP, FLAME_K, KILLED, KILLED_WALLS, LIVING, PLANTS, REMAINS, SCORCHED, SMOULDERED, CURED, SCORCH_EXP, SCORCH_K, W_PER_KW};
use crate::chem::{Chemistry, Phase};
use crate::cover::{per_input, Books, Cover, Parts, BUDGET, BURNT, FUEL, MATTER, NP, WALL};
use crate::life::{Chunk, FLAME};
use crate::store::{Class, Node};
use crate::{World, AIR};

/// How fast a flame runs through a bed, m/s, how high it reaches, m, and how high its plume kills the leaves it meets, m.
#[derive(Clone, Copy, Debug)]
pub struct Reach {
    pub speed: f64,
    pub flame: f64,
    pub scorch: f64,
}

/// The reach of a flame through `bed` with no wind: it moves at R = `flux_ahead` h δ / (τ Q) (Rothermel 1972: what the
/// burning kg sends ahead over what lighting a kg takes, a bed of depth δ (`Bed::depth`), burning for τ,
/// `RESIDENCE_PER_M` × `fine_d`), its intensity is I = h w R (Byram 1959), its flame 0.0775 I^0.46 long (Byram 1959) and
/// its plume kills leaves up to 0.1483 I^(2/3) (Van Wagner 1973), I in kW a m.
pub fn reach(ph: &crate::Physics, bed: &Bed) -> Reach {
    let speed = ph.flux_ahead * bed.heat * bed.depth(ph) / (super::residence(ph) * bed.light.max(f64::MIN_POSITIVE));
    let kw = bed.heat * bed.flames() * speed / W_PER_KW;
    Reach { speed, flame: FLAME_K * kw.powf(FLAME_EXP), scorch: SCORCH_K * kw.powf(SCORCH_EXP) }
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

/// Slot `x` of cover `c`, its crown from `base` to `top` m (`contract::crown`), on the share `f` of its cell that a
/// flame of reach `r` burns. Its plants no taller than the scorch height die above the ground (`Cover::die_under`):
/// what of them stood above their base burns in the flame, those that die whole leave their body matter and the fuel
/// in them by their share of the built cover to the ground and their walls standing dead (`Cover::snag`). Of a slot
/// taller than `surface_m` the body matter of its crown that lives and stands in the flame burns, and what stands
/// between the flame and the scorch height dies to the ground. What burns and dies is counted in `flows`.
fn crown_burns(w: &World, p: &Parts, (c, x, (base, top)): (&mut Cover, usize, (f64, f64)), (f, r): (f64, &Reach), (b, flows): (&mut Books, &mut [f64; super::FLOWS.len()])) {
    let (chem, ph, mm) = (&w.chem, &w.cfg.phys, p.mm(&w.chem));
    let rho = chem.subs[p.sub[WALL]].density[0];
    let [lit, under] = [r.flame, r.scorch].map(|h| f * crate::contract::reached((0.0, h), 0.0, (base, top)));
    let (built, matter) = (c.built(&mm, x), c.mol[x][MATTER]);
    let ([tops, whole], died) = c.die_under(ph, (&mm, rho), x, (f, r.scorch));
    for (q, kg) in [(MATTER, tops[0]), (WALL, tops[1])] {
        let mol = (kg / mm[q]).min(c.mol[x][q]);
        flows[LIVING] += burn_up(chem, b, (&p.sub, &mut c.mol[x]), p.sub[q], mol) * mm[q];
    }
    let mut dead = [0.0; NP];
    dead[MATTER] = (whole[0] / mm[MATTER]).min(c.mol[x][MATTER]);
    dead[FUEL] = c.mol[x][FUEL].max(0.0) * (whole[0] + whole[1]) / built.max(f64::MIN_POSITIVE);
    let standing = (whole[1] / mm[WALL]).min(c.mol[x][WALL]);
    (c.mol[x][WALL], c.snag[WALL]) = (c.mol[x][WALL] - standing, c.snag[WALL] + standing);
    flows[KILLED] += dead.iter().zip(&mm).map(|(m, kg)| m * kg).sum::<f64>() + standing * mm[WALL];
    flows[KILLED_WALLS] += standing * mm[WALL];
    flows[PLANTS] += died;
    if top > ph.surface_m {
        // Of the leaves of the plants that live, those in the flame burn and those under the scorch height die.
        let alive = if matter > 0.0 { ((c.mol[x][MATTER] - dead[MATTER]) / matter).max(0.0) } else { 0.0 };
        flows[LIVING] += burn_up(chem, b, (&p.sub, &mut c.mol[x]), p.sub[MATTER], matter * lit * alive) * mm[MATTER];
        let scorched = (matter * (under - lit) * alive).clamp(0.0, (c.mol[x][MATTER] - dead[MATTER]).max(0.0));
        dead[MATTER] += scorched;
        flows[SCORCHED] += scorched * mm[MATTER];
    }
    // (Shares of the slot taken one after another may pass what it holds by rounding: no more than it holds dies.)
    for q in 0..NP {
        let d = dead[q].min(c.mol[x][q].max(0.0));
        (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - d, b.dead[p.sub[q]] + d);
    }
}

/// What the burning gave of each mineral (`Chemistry::minerals`) breaks down in the flame as the chemistry breaks it
/// down hot: each reaction that takes the mineral alone runs over the share 1 − e^(−k τ) of it, k its rate constant at
/// the flaming bed's temperature (`t_burn`), τ the time the bed flames (`residence`). So the fixed element of what burns
/// goes to the air as its gas where the flame is hot enough, and the rest lies as ash (Raison 1979: from 500 °C nearly
/// all of it is lost).
fn break_down(chem: &Chemistry, ph: &crate::Physics, b: &mut Books) {
    for &(k, _) in chem.minerals() {
        for rx in chem.reactions.iter().filter(|rx| rx.inputs.iter().all(|q| q.0 as usize == k)) {
            let (given, n) = ((-b.ext[k]).max(0.0), per_input(rx, k).unwrap_or(1.0));
            let share = 1.0 - (-rx.arrhenius(ph.t_burn) * super::residence(ph)).exp();
            b.run(chem, &[], &mut [], rx, given * share / n);
        }
    }
}

/// Cell `i` of chunk `d` burns the share `f` of its land, its remains lying as `bed`: there the part of its remains dry
/// enough to flame burns, and the part under it dry enough to smoulder after the flame (`Bed::flaming`, `smoulder`); of
/// each slot of its cover the body matter of its crown (`contract::crown`, spread evenly over it) that stands in the
/// flame burns, what stands between the flame and the scorch height dies to its remains, and a slot whose whole crown
/// stands under the scorch height loses its plants there (a young plant must outgrow the flame between flames; Bond and
/// Midgley 2001; a slot left with no built cover is emptied in the cover's next month); its seeds lying survive (they
/// lie in the top of the ground, which a passing flame barely warms). Gases go to its air, what is not a gas to its ground (`life::book`), and the heat the burning frees to its air node.
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
        let held = moves.amount(w, Node::ground(i), p.sub[q]).max(0.0) * f;
        flows[REMAINS] += burn_up(chem, &mut b, (&[], &mut []), p.sub[q], held * bed.flaming) * mm[q];
        flows[SMOULDERED] += burn_up(chem, &mut b, (&[], &mut []), p.sub[q], held * (bed.smoulder - bed.flaming)) * mm[q];
    }
    let before = c.content(&e);
    let cured = c.cured[MATTER] * if bed.standing > 0.0 { f } else { 0.0 };
    flows[CURED] += burn_up(chem, &mut b, (&p.sub, &mut c.cured), p.sub[MATTER], cured) * mm[MATTER];
    for (x, crown) in crowns {
        crown_burns(w, p, (c, x, crown), (f, &r), (&mut b, &mut flows));
    }
    break_down(chem, &w.cfg.phys, &mut b);
    b.budget[BURNT] += c.content(&e) - before;
    counts.cover_lost[FLAME as usize] += flows[LIVING] + flows[SCORCHED] + flows[KILLED];
    counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, x)| *a += x);
    counts.flame.iter_mut().zip(flows).for_each(|(a, x)| *a += x);
    counts.flame_air.resize(chem.ns(), 0.0);
    for k in (0..chem.ns()).filter(|&k| chem.phase(k, t) == Phase::Gas) {
        counts.flame_air[k] -= b.ext[k];
    }
    // For the tape: the gas given to the air and the minerals left on the ground, kg a m² of land.
    let given = |k: usize| (-b.ext[k]).max(0.0) * chem.subs[k].molar_mass / crate::contract::land(w, i).max(f64::MIN_POSITIVE);
    let air: f64 = (0..chem.ns()).filter(|&k| chem.phase(k, t) == Phase::Gas).map(given).sum();
    let ash: f64 = chem.minerals().iter().map(|m| given(m.0)).sum();
    counts.flamed.push((i as u32, super::Flamed { share: f as f32, length: r.flame as f32, ash: ash as f32, air: air as f32 }));
    // The heat the burning frees goes to the air over the cell (its plume); the rest closes with the ground as the
    // cover's books close (`life::book`).
    let freed = std::mem::take(&mut b.heat);
    c.e = crate::life::book(w, moves, (i, t), (c.e, 0.0), c.cap(chem, p) * t, &b);
    moves.give(w, Node::air(w.mesh.up(i, 0, AIR)), 0, 0.0, freed);
}

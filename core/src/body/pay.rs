//! Upkeep, and paying a cost by burning the body's fuel on demand (laws.md 6.2).

use super::{Body, REPAIR, STRENGTH};
use crate::chem::Phase;
use crate::store::Node;
use crate::{life::Chunk, lineage::Lineage, World, AIR};

/// Upkeep over `days` of a body of `mass` kg at `t` K, J (laws.md 6.2): per kg of living body its basal, strength and
/// learning costs, times (copy mass)^(`intake_exp` - 1) so adults pay Kleiber's mass^0.75 while one body grows
/// until keeping its tissue takes all it can supply (West, Brown and Enquist 2001); its brain per member over the time
/// (`Physics::think` per month, `Config::span`); times repair, its activity `dry` (`Body::activity`, read by the caller)
/// and the pace of its burning at its temperature (`Body::pace`).
pub fn upkeep(w: &World, b: &Body, days: f64, mass: f64, t: f64, dry: f64) -> f64 {
    let p = &w.cfg.phys;
    let mind = if w.life.brainless(b) { 0.0 } else { p.c_learn * b.learn_rate() as f64 };
    let think = w.life.has_brain(b.lineage).map_or(p.c_brain, |brain| Lineage::cost(brain, p, &b.genes));
    let tissue = b.built(&w.chem) * b.k.tissue;
    let u = days * tissue * (p.c_basal + p.c_strength * b.gene(STRENGTH) + mind) + w.cfg.span(days).amount(p.think(think, mass / b.n())) * b.n();
    u * p.repair(b.gene(REPAIR)) * dry * b.pace(&w.chem, t)
}

/// Pay `cost` J, all or nothing: false, and nothing paid, if its fuel cannot release that much (`burn`).
pub fn pay(w: &World, d: &mut Chunk, b: &mut Body, cost: f64) -> bool {
    if cost <= 0.0 {
        return true;
    }
    let t = b.temp(&w.chem);
    if b.fuel(&w.chem, t) < cost {
        return false;
    }
    burn_at(w, d, b, cost, t);
    true
}

/// Burn its fuel to release up to `cost` J, its burnings in order (`Consts::burn`), each as far as its fuel and the
/// gases of its air node allow: a gas it takes comes from its air node with its heat at the air's temperature and its
/// latent heat; a gas it makes goes to its cell's bodies' node with its own (`Moves::gas`), as a body breathes; the rest
/// goes to its stores. What it releases becomes heat in its cell's ground, as the work it pays for ends as heat there:
/// the body keeps its temperature (`settle_heat`). Returns the energy released, J.
pub(crate) fn burn(w: &World, d: &mut Chunk, b: &mut Body, cost: f64) -> f64 {
    burn_at(w, d, b, cost, b.temp(&w.chem))
}

/// `burn` for a body at `t` K, its temperature now.
fn burn_at(w: &World, d: &mut Chunk, b: &mut Body, cost: f64, t: f64) -> f64 {
    if cost <= 0.0 {
        return 0.0;
    }
    let (chem, i, bc) = (&w.chem, b.cell as usize, w.chem.body());
    let j = w.mesh.up(i, 0, AIR);
    let (lift, ta, held) = (w.lift(i), d.moves.t_air(w, j), b.matter_mol(chem));
    let mut got = 0.0;
    for q in 0..b.k.burn.len() {
        let (r, e) = b.k.burn[q];
        if got >= cost {
            break;
        }
        let rx = &chem.reactions[r];
        let have = |k: usize| if chem.phase(k, t) == Phase::Gas { d.moves.amount(w, Node::air(j), k) } else { b.amount(bc, k) };
        let x = rx.inputs.iter().map(|&(k, nu)| have(k as usize) / nu as f64).fold((cost - got) / e, f64::min);
        if x <= 0.0 {
            continue;
        }
        for (k, nu) in crate::change::terms(rx) {
            let (dn, s) = (nu * x, &chem.subs[k]);
            // (What it makes and cannot hold, a store of no body, leaves it as a gas does.)
            if let Some(p) = bc.place(k).filter(|_| chem.phase(k, t) != Phase::Gas) {
                super::add_store(&mut b.store[p], dn, &mut d.counts.rounded[0]);
            } else if dn > 0.0 {
                // (It gives the gas its heat and pays its latent heat, `Moves::gas`.)
                d.moves.gas(w, k, dn, lift);
                d.moves.give(w, Node::bodies(i), 0, 0.0, dn * s.cp * t);
                b.e -= dn * (s.cp * t + s.latent_gas());
            } else {
                d.moves.gas(w, k, dn, lift);
                d.moves.take(w, Node::air(j), 0, 0.0, -dn * s.cp * ta);
                b.e -= dn * (s.cp * ta + s.latent_gas());
            }
        }
        (b.e, got) = (b.e + e * x, got + e * x);
    }
    b.brood_share(chem, held);
    settle_heat(w, d, b, t);
    got
}

/// Body `b` comes to temperature `t` with its stores as they are: its heat over what it holds at `t` goes to its cell's
/// ground, and what it lacks comes from there (as far as the ground can give it, `Moves::take_from_ground`).
pub(crate) fn settle_heat(w: &World, d: &mut Chunk, b: &mut Body, t: f64) {
    let (i, target) = (b.cell as usize, b.energy_at(&w.chem, t));
    let over = b.e - target;
    if over > 0.0 {
        d.moves.give(w, Node::ground(i), 0, 0.0, over);
        b.e = target;
    } else if over < 0.0 {
        let cap = d.moves.heat_cap_ground(w, i);
        b.e += d.moves.take_from_ground(w, i, (0, 0.0, 0.0), -over, cap).1;
    }
}

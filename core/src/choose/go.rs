//! Moving: a body steps to a neighbour cell (`go`), wearing the bodies it pushes through (`wear`) and clearing a lane.

use crate::body::{self, Body, BROKEN};
use crate::chem::Phase;
use crate::{bond, lanes, World};

/// Move to a neighbour, if its force gives the push a mover makes (`bump`; laws.md 3.5): cost by its whole mass at the
/// real cost of transport (`Physics::walk`), the distance, the climb and the bodies in the way (`lanes::resist`); less
/// afloat. The part spent pushing through bodies breaks them (`wear`) and clears the lane. A node moves as one.
pub fn go(w: &mut World, slot: usize, b: &mut Body, j: usize) {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    if (w.life.sum[j][1] + mb) / w.cfg.phys.rho_body > w.cfg.phys.layer * w.mesh.cells().area()[j] {
        return;
    }
    let carried = bond::carry(w, b);
    let afloat = bond::afloat(w, b, j, mb);
    let climb = if afloat { 0.0 } else { (w.life.env[j][2] - w.life.env[i][2]).max(0.0) };
    let resist = lanes::resist(w, i, j);
    let mass = mb + carried.map_or(0.0, |c| c.1);
    // A mover pushes what it passes with `bump` of its weight, so it must push that much itself.
    if bond::bears(w, b) < w.cfg.phys.bump * mass * w.cfg.gravity {
        return;
    }
    let cost = mass * (w.cfg.phys.walk(mb / b.n()) * w.mesh.cells().dist()[lanes::link(w, i, j)] + w.cfg.gravity * climb) * resist * if afloat { w.cfg.phys.afloat } else { 1.0 };
    if !body::pay(w, b, cost, b.interval(w) as f64) {
        return;
    }
    bond::moved(w, b, carried, j, mb);
    w.counts.parts.afloat += afloat as u8 as f64;
    w.life.relocate(slot, i, j);
    b.cell = j as u32;
    (b.mark, b.rained) = (w.light[j], w.rained[j]);
    w.counts.crossings.resize(w.mesh.cells().n(), 0.0);
    w.counts.crossings[j] += b.n() as f32;
    let cover = w.life.sum[j][1] / w.mesh.cells().area()[j];
    let broken = wear(w, j, slot, cost * (1.0 - 1.0 / resist));
    if broken > 0.0 && cover > 0.0 {
        // The cover broken along the way was the lane's: that much ground is now clear.
        lanes::clear(w, i, j, broken / cover);
    }
}

/// Energy dissipated pushing through the bodies of cell `j` breaks their body matter: each takes its share by mass and
/// loses what that pays to break, each of its members alike (`Body::breaking`, `body::wound`; laws.md 6.8). Returns the mass broken, kg.
pub fn wear(w: &mut World, j: usize, mover: usize, energy: f64) -> f64 {
    let chem = &w.chem;
    let others: Vec<u32> = w.life.at[j].iter().copied().filter(|&s| s as usize != mover).collect();
    let total: f64 = others.iter().map(|&s| w.life.bodies[s as usize].mass(chem)).sum();
    let mut broken = 0.0;
    if energy <= 0.0 || total <= 0.0 {
        return 0.0;
    }
    for s in others {
        let s = s as usize;
        let o = std::mem::take(&mut w.life.bodies[s]);
        let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
        let f = (energy * m / total / o.breaking(&w.chem, &w.cfg, 1.0).max(1e-30)).min(1.0);
        broken += m * f;
        let dead: Vec<f64> = (0..w.chem.ns()).map(|k| if w.chem.phase(k, t) != Phase::Gas { o.store[k] * f } else { 0.0 }).collect();
        body::wound(w, s, o, (1.0, f), &dead, BROKEN);
    }
    broken
}

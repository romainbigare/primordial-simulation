//! Moving: a body steps to a neighbour cell (`go`), wearing the bodies it pushes through (`wear`) and clearing a lane.

use crate::body::{self, Body, BROKEN};
use crate::chem::Phase;
use crate::{bond, lanes, World};

/// Move to a neighbour, if its force gives the push a mover makes (`Physics::bump` of what it moves): cost by mass (with
/// the structure it carries) at the real cost of transport per kg and m, `Physics::walk` (more per kg for a small
/// member), distance, climb, and the body mass per area already there (`lanes::resist`); less, and no climb, into deep
/// liquid while held up by what it carries. The movers must fit in the free space over the cell (`Physics::layer`).
/// The part spent pushing through bodies breaks their body matter (`wear`) and clears the lane (`lanes::clear`).
/// What it moves is the whole mass it carries, fuel too; what it can carry and hold goes by its force, of its body
/// (`bond::carry`, `Body::built`). A node moves as one: all its members go (they drew the same action), and it stays one node.
pub fn go(w: &mut World, slot: usize, b: &mut Body, j: usize) {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    if (w.life.sum[j][1] + mb) / w.cfg.phys.rho_body > w.cfg.phys.layer * w.mesh.cells().area()[j] {
        return;
    }
    let carried = bond::carry(w, b);
    let afloat = carried.is_some() && bond::afloat(w, b, j, mb);
    let climb = if afloat { 0.0 } else { (w.life.env[j][2] - w.life.env[i][2]).max(0.0) };
    let resist = lanes::resist(w, i, j);
    let mass = mb + carried.map_or(0.0, |c| c.1);
    // Moving needs force: a mover pushes on what it passes with `bump` of its weight (`bond::moved`), so it must push
    // that much itself, by its own force (`bond::bears`, of its built body).
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

/// Most share of its body a body loses to one mover.
const MOST_WORN: f64 = 0.5;

/// Energy dissipated pushing through the bodies of cell `j` (all but the `mover`) breaks their body matter: each takes
/// its share of the energy by its mass and loses the share of its body that this pays to break (`Body::breaking`, by
/// what it is built of), and the stores that matter held spill in the same share (`body::wound`). A node loses members
/// when the share is one member's body or more, and else every member loses part of its matter; an individual lives on
/// smaller (it dies when half of its body is broken, or a store falls under its safe minimum). What breaks off goes to
/// the ground as loose amounts, with its heat. Returns the mass broken, kg.
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
        let (m, t, built0, matter0) = (o.mass(&w.chem), o.temp(&w.chem), o.built(&w.chem), o.matter_kg(&w.chem));
        let f = (energy * m / total / o.breaking(&w.chem, &w.cfg, 1.0).max(1e-30)).min(MOST_WORN);
        broken += m * f;
        let dead: Vec<f64> = (0..w.chem.ns()).map(|k| if w.chem.phase(k, t) != Phase::Gas { o.store[k] * f } else { 0.0 }).collect();
        body::wound(w, s, o, f, &dead, BROKEN, built0, matter0);
    }
    broken
}

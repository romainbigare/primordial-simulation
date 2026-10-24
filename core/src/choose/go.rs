//! Moving: a body steps to a neighbour cell (`go`), clearing a lane, and what it pushes through wears (`wear`).

use crate::body::{self, Body, BROKEN};
use crate::chem::Phase;
use crate::store::{Class, Node};
use crate::{bond, lanes, World};

/// Move to a neighbour, if its force gives the push a mover makes (`bump`; laws.md 3.5): cost by its whole mass at the
/// real cost of transport (`Physics::walk`), the distance, the climb and the bodies in the way (`lanes::resist`); less
/// afloat. The part spent pushing through bodies breaks them (`wear`) and clears the lane. A node moves as one.
pub fn go(w: &mut World, b: &mut Body, j: usize) {
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
    w.life.relocate(b.id, i, j);
    b.cell = j as u32;
    (b.mark, b.rained) = (w.light[j], w.rained[j]);
    w.counts.crossings.resize(w.mesh.cells().n(), 0.0);
    w.counts.crossings[j] += b.n() as f32;
    // What it spends pushing through the bodies there breaks them at the next survey (`wear`) and clears the lane: the
    // ground whose cover that energy breaks.
    let (pushed, all) = (cost * (1.0 - 1.0 / resist), w.life.sum[j][7]);
    w.life.pushed.resize(w.mesh.cells().n(), 0.0);
    w.life.pushed[j] += pushed;
    if all > 0.0 {
        lanes::clear(w, i, j, pushed * w.mesh.cells().area()[j] / all);
    }
}

/// Before each survey: the energy movers spent pushing through the bodies of each cell since the last (`Life::pushed`)
/// breaks their body matter. Each body there takes its share by mass and loses what that pays to break, each of its
/// members alike (`Body::breaking`, `body::wound`; laws.md 6.8). A cell's bodies are worn once a survey, not once a move.
pub fn wear(w: &mut World) {
    let pushed = std::mem::take(&mut w.life.pushed);
    for (j, &energy) in pushed.iter().enumerate().filter(|e| *e.1 > 0.0) {
        let here: Vec<usize> = w.life.cell(j).collect();
        let total: f64 = here.iter().map(|&s| w.life[s].mass(&w.chem)).sum();
        if total <= 0.0 {
            continue;
        }
        w.open_moves(j, Node::new(Class::Body, here[0]));
        for s in here {
            let o = std::mem::take(&mut w.life[s]);
            let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
            let f = (energy * m / total / o.breaking(&w.chem, &w.cfg, 1.0).max(1e-30)).min(1.0);
            let dead: Vec<f64> = (0..w.chem.ns()).map(|k| if w.chem.phase(k, t) != Phase::Gas { o.store[k] * f } else { 0.0 }).collect();
            body::wound(w, s, o, (1.0, f), &dead, BROKEN);
        }
        w.book_moves();
    }
}

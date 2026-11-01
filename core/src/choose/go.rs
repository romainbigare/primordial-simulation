//! Moving: a body steps to a neighbour cell (`go`), landing there when its batch ends (`land_move`), clearing a lane, and
//! what it pushes through wears (`wear`).

use crate::body::{self, Body, BROKEN};
use crate::chem::Phase;
use crate::store::{Class, Node};
use crate::{bond, copy, lanes, nodes, life::Domain, outbox::Entry, World};

/// Move to a neighbour, if its force gives the push a mover makes (`bump`; laws.md 3.5): cost by its whole mass at the
/// real cost of transport (`Physics::walk`), the distance, the climb and the bodies in the way (`lanes::resist`); less
/// afloat. The part spent pushing through bodies breaks them (`wear`) and clears the lane. A node moves as one. It pays
/// now and lands when the batch ends (`land_move`); until then it is leaving (`Body::leaving`). With `kin`, it is members of a
/// node who join their kind where they land.
pub fn go(w: &World, d: &mut Domain, b: &mut Body, j: usize, kin: bool) {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    let carried = bond::carry(w, b);
    if !can_go(w, b, j, 1.0, carried.map_or(0.0, |c| c.1)) {
        return;
    }
    if carried.is_none() {
        bond::let_go(w, d, b, true);
    }
    let afloat = bond::afloat(w, d, b, j, mb);
    let climb = if afloat { 0.0 } else { (w.life.env[j][2] - w.life.env[i][2]).max(0.0) };
    let resist = lanes::resist(w, i, j);
    let mass = mb + carried.map_or(0.0, |c| c.1);
    let cost = mass * (w.cfg.phys.walk(mb / b.n()) * w.mesh.cells().dist()[lanes::link(w, i, j)] + w.cfg.gravity * climb) * resist * if afloat { w.cfg.phys.afloat } else { 1.0 };
    if !body::pay(w, d, b, cost) {
        return;
    }
    d.counts.parts.afloat += afloat as u8 as f64;
    // What it spends pushing through the bodies there breaks them at the next survey (`wear`) and clears the lane: the
    // ground whose cover that energy breaks.
    let (pushed, all) = (cost * (1.0 - 1.0 / resist), w.life.sum(j)[7]);
    let lane = if all > 0.0 { pushed * w.mesh.cells().area()[j] / all } else { 0.0 };
    b.leaving = true;
    d.out.push(Entry::Go { id: b.id, from: i as u32, to: j as u32, pushed, lane, carries: carried.is_some(), kin, body: None });
}

/// Whether share `share` of body `b` (a node's movers, or the body whole with 1) carrying `load` kg can step into cell
/// `j`: the cell has room for its mass (`Physics::layer`), and the body bears the `bump` share of the weight it moves
/// (`Physics::bump`), since it pushes what it passes. Both go by the mover's mass, so a node asks before it splits any
/// movers off. (What a move pays and where it lands are for `go`.)
pub(super) fn can_go(w: &World, b: &Body, j: usize, share: f64, load: f64) -> bool {
    let mb = share * b.mass(&w.chem);
    (w.life.sum(j)[1] + mb) / w.cfg.phys.rho_body <= w.cfg.phys.layer * w.mesh.cells().area()[j]
        && share * bond::bears(w, b) >= w.cfg.phys.bump * (mb + load) * w.cfg.gravity
}

/// The move of body `b` from cell `i` to `j` that `go` queued lands in `d`, the domain of `j`: it enters `j` (inside
/// no structure), carries what it holds if it paid for that (`carries`;
/// else it lets go of it), and its push breaks the bodies there at the next survey. What it does to things no domain
/// owns is queued (`Entry::Passed`): it pushes the structures there (`bond::moved`) and clears `lane` m² of the lane.
/// With `kin`, the members of a node who moved join the first node of their kind and stage there (`copy::kin`).
pub(crate) fn land_move(w: &World, d: &mut Domain, mut b: Body, (i, j): (usize, usize), (pushed, lane): (f64, f64), (carries, kin): (bool, bool)) {
    let carried = bond::carry(w, &b).filter(|_| carries);
    if carried.is_none() {
        bond::let_go(w, d, &mut b, carries);
    }
    let mass = b.mass(&w.chem);
    // (Before it enters `j`, it is no node of its cell for `kin`.)
    let t = if kin { copy::kin(w, d, j, b.lineage, b.adult(&w.chem)) } else { None };
    (b.cell, b.leaving, b.inside) = (j as u32, false, None);
    d.cross(w.life.cell_slot(j).1, b.n() as f32, pushed);
    d.out.push(Entry::Passed { from: i as u32, to: j as u32, lane, mass, carried: carried.map(|c| bond::id(w, c.0)) });
    let s = d.arrive(b);
    if let Some(t) = t {
        nodes::merge(w, d, s, t);
    }
}

/// Before each survey: the energy `energy` J movers spent pushing through the bodies of cell `j` of domain `d` since the
/// last (recorded by `Domain::cross` in `land_move`) breaks their body matter. Each body there takes its share by mass and loses what that pays to
/// break, each of its members alike (`Body::breaking`, `body::wound`; laws.md 6.8). A cell's bodies are worn once a
/// survey, not once a move. (Those who held a part let go of it when the outbox is applied, `bond::let_go`.)
pub fn wear(w: &World, d: &mut Domain, j: usize, energy: f64) {
    let here: Vec<_> = d.cell(j).collect();
    let (k, parts) = (d.slot(j), crate::cover::Parts::of(&w.chem));
    let cover = parts.as_ref().map_or(0.0, |p| crate::cover::Cover::slots(k).map(|x| d.cover.kg(&p.mm(&w.chem), x)).sum());
    let total: f64 = here.iter().map(|&s| d[s].mass(&w.chem)).sum::<f64>() + cover;
    if total <= 0.0 {
        return;
    }
    d.moves.open(w, j, Node::new(Class::Cover, j));
    // The cover takes its share by mass as a body does: what that breaks of it goes to the ground as remains.
    if let Some(p) = parts.as_ref().filter(|_| cover > 0.0) {
        let f = (energy * cover / total / d.cover.breaking(&w.chem, &w.cfg, p, k, 1.0).max(1e-30)).min(1.0);
        d.counts.cover_lost[BROKEN as usize] += f * cover;
        crate::cover::lose(w, d, p, k, f);
    }
    for s in here {
        let o = std::mem::take(&mut d[s]);
        let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
        let f = (energy * m / total / o.breaking(&w.chem, &w.cfg, 1.0).max(1e-30)).min(1.0);
        let mut dead: Vec<f64> = (0..w.chem.ns()).map(|k| if w.chem.phase(k, t) != Phase::Gas { o.store[k] * f } else { 0.0 }).collect();
        body::wound(w, d, s, o, (1.0, f), &mut dead, BROKEN);
    }
}

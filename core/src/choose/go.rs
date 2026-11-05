//! Moving: a body steps to a neighbour cell (`go`), landing there in the serial pass (`land_move`), clearing a lane, and
//! what it pushes through wears (`wear`).

use crate::body::{self, Body, BROKEN};
use crate::chem::Phase;
use crate::store::{Class, Node};
use crate::units::DAY;
use crate::{bond, copy, lanes, nodes, life::{Chunk, Entry}, World};

/// Move to a neighbour, if its force gives the push a mover makes (`bump`; laws.md 3.5): cost by its whole mass at the
/// real cost of transport (`Physics::walk`), the distance, the climb and the bodies in the way (`lanes::resist`). The
/// part spent pushing through bodies breaks them (`wear`) and clears the lane. A node moves as one. It pays now and
/// lands in the serial pass (`land_move`); until then it is leaving (`Body::leaving`). The walk takes the time the
/// distance takes at its speed (`Physics::speed`), and it eats where it lands for the rest of the time to that cell's
/// next sub-step (`World::days_to_phys`), and at each later one (laws.md 5.3). With `kin`, it is members of a node who
/// join their kind where they land. False if it cannot move (no room there, too weak, or too poor to pay): it stays.
pub fn go(w: &World, d: &mut Chunk, b: &mut Body, j: usize, kin: bool) -> bool {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    let carried = bond::carry(d.at(i), w, b);
    if !can_go(w, b, j, carried.map_or(0.0, |c| c.1)) {
        return false;
    }
    if carried.is_none() {
        bond::let_go(d, b, true);
    }
    let climb = (w.life.env[j][2] - w.life.env[i][2]).max(0.0);
    let (resist, dist) = (lanes::resist(w, i, j), w.mesh.cells().dist()[lanes::link(w, i, j)]);
    let (p, m1, mass) = (&w.cfg.phys, mb / b.n(), mb + carried.map_or(0.0, |c| c.1));
    let cost = mass * (p.walk(m1) * dist + w.cfg.gravity * climb) * resist;
    if !body::pay(w, d, b, cost) {
        return false;
    }
    // What it spends pushing through the bodies there breaks them at the next survey (`wear`) and clears the lane: the
    // ground whose cover that energy breaks.
    let (pushed, all) = (cost * (1.0 - 1.0 / resist), w.life.sum(j)[7]);
    let lane = if all > 0.0 { pushed * w.mesh.cells().area()[j] / all } else { 0.0 };
    let eats = (w.days_to_phys(j) - dist / p.speed(m1, w.cfg.gravity) / DAY).max(0.0);
    b.leaving = true;
    d.out.push(Entry::Go { id: b.id, from: i as u32, to: j as u32, pushed, lane, carries: carried.is_some(), kin, eats, brain: Vec::new() });
    true
}

/// Whether body `b` carrying `load` kg can step into cell `j`: the cell has room for its mass (`Physics::layer`), and
/// the body bears the `bump` share of the weight it moves (`Physics::bump`), since it pushes what it passes.
fn can_go(w: &World, b: &Body, j: usize, load: f64) -> bool {
    let mb = b.mass(&w.chem);
    (w.life.sum(j)[1] + mb) / w.cfg.phys.rho_body <= w.cfg.phys.layer * w.mesh.cells().area()[j]
        && bond::bears(w, b) >= w.cfg.phys.bump * (mb + load) * w.cfg.gravity
}

/// The move of body `b` from cell `i` to `j` that `go` queued lands in `d`, the chunk of `j` in the serial pass, with
/// the numbers of its brain (`brain`, `Brains::all`): it enters `j` (inside no structure), carries the structure it
/// holds (`carried`: its root, as the cell it left held it) if it paid for that (`carries`; else it lets go of it), and
/// its push breaks the bodies there at the next survey. What it does to things no cell owns is queued
/// (`Entry::Passed`): it pushes the structures there (`bond::moved`) and clears `lane` m² of the lane. There it takes
/// from the ground for the `eats` d the walk left of its sub-step (`take_node`; laws.md 5.3), and the heat of that meal
/// fades as a meal's does (`body::fade_meal_heat`, drawing back at most what its upkeep releases while it eats); its
/// physics runs on to that cell's next step after the landing (`survey::batch_of`). With
/// `kin`, the members of a node who moved join the first node of their kind and stage there (`copy::kin`) as an
/// amount, their genes and rows (`nodes::add`), not as a body; else it is stored there with its brain.
pub(crate) fn land_move(w: &World, d: &mut Chunk, (mut b, brain): (Body, &[f32]), (i, j, carried): (usize, usize, Option<u64>), (pushed, lane, eats): (f64, f64, f64), (carries, kin): (bool, bool)) {
    let carried = carried.filter(|_| carries);
    if carried.is_none() {
        // (The part it holds lies in the cell it left: it lets go of it there, in the acts' landing.)
        if let Some(part) = b.hold.take() {
            let verb = if carries { bond::TORN } else { bond::LET_GO };
            d.out.push(Entry::Act { id: b.id, on: Ok((verb, part)), node: None, days: 0.0 });
        }
    }
    let mass = b.mass(&w.chem);
    d.went(i, j, mass);
    // (Before it enters `j`, it is no node of its cell for `kin`.)
    let t = if kin { copy::kin(w, d, j, b.lineage, b.adult(&w.chem)) } else { None };
    (b.cell, b.leaving, b.inside) = (j as u32, false, None);
    d.cross(j, b.n() as f32, pushed);
    d.out.push(Entry::Passed { by: b.id, from: i as u32, to: j as u32, lane, mass, carried });
    if eats > 0.0 {
        // (The store books no body's side of a move: a body node of any index serves a body out of its slot.)
        d.moves.open(w, j, Node::new(Class::Body, 0));
        super::feed::eat_for(w, d, &mut b, eats);
    }
    match t.filter(|_| b.hold.is_none()) {
        Some(t) => nodes::add(w, d, t, nodes::Lot::of(&mut b), (&b.genes, &b.spread), (b.lineage, b.genome, Some(nodes::Own::Carried(brain)), b.row_spread)),
        None => {
            b.brain = d.brains(j).take_in(brain);
            d.arrive(b);
        }
    }
}

/// Before each survey: the energy `energy` J movers spent pushing through the bodies of cell `j` of chunk `d` since the
/// last (recorded by `Chunk::cross` in `land_move`) breaks their body matter. Each body there takes its share by mass and loses what that pays to
/// break, each of its members alike (`Body::breaking`, `body::wound`; laws.md 6.8). A cell's bodies are worn once a
/// survey, not once a move. (Those who held a part let go of it in the serial pass, `bond::let_go`.)
pub fn wear(w: &World, d: &mut Chunk, j: usize, energy: f64) {
    let here: Vec<_> = d.cell(j).collect();
    let parts = crate::cover::Parts::of(&w.chem);
    let cover = parts.as_ref().map_or(0.0, |p| (0..crate::cover::SLOTS).map(|x| d.at(j).cover.kg(&p.mm(&w.chem), x)).sum());
    let total: f64 = here.iter().map(|&s| d[s].mass(&w.chem)).sum::<f64>() + cover;
    if total <= 0.0 {
        return;
    }
    d.moves.open(w, j, Node::new(Class::Cover, j));
    // The cover takes its share by mass as a body does: what that breaks of it goes to the ground as remains.
    if let Some(p) = parts.as_ref().filter(|_| cover > 0.0) {
        let f = (energy * cover / total / d.at(j).cover.breaking(&w.chem, &w.cfg, p, 1.0).max(1e-30)).min(1.0);
        d.counts.cover_lost[BROKEN as usize] += f * cover;
        crate::cover::lose(w, d, p, j, f);
    }
    for s in here {
        let o = std::mem::take(&mut d[s]);
        let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
        let f = (energy * m / total / o.breaking(&w.chem, &w.cfg, 1.0).max(1e-30)).min(1.0);
        let subs = w.chem.body().subs();
        let mut dead: crate::chem::Stores = std::array::from_fn(|s| if subs.get(s).is_some_and(|&k| w.chem.phase(k, t) != Phase::Gas) { o.store[s] * f } else { 0.0 });
        body::wound(w, d, s, o, (1.0, f), &mut dead, BROKEN);
    }
}

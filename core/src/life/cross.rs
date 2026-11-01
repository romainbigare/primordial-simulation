//! The serial pass (laws.md 0): what body code in a chunk did outside its cell waits in `Life::queued`, in the order of
//! the cells and then the order made: its moves to another cell, its children who start in another cell, its watches
//! and its acts on parts. After the parallel pass they are applied (`land`): the moves and litters land in their cells,
//! each chunk of cells on the pool, the others one by one on one thread; each by its law on the bodies and parts as they
//! are then. One whose body or target is gone does nothing, as its law says.

use crate::{body::Body, bond, choose, copy, lanes, life, store::NodeId, World};

/// One effect of an update outside its cell.
#[derive(Clone, Debug)]
pub enum Entry {
    /// Body `id` moves from cell `from` to `to` (`choose::go`), having spent `pushed` J pushing through the bodies
    /// there, which clears `lane` m² of its lane; `carries`: it paid to carry the structure it holds; with `kin`, it is
    /// members of a node who join their kind there
    Go { id: u64, from: u32, to: u32, pushed: f64, lane: f64, carries: bool, kin: bool },
    /// Children who start in another cell (`copy::emit`)
    Litter(Box<copy::Litter>),
    /// A body of mass `mass` kg landed in cell `to` from `from` (`choose::land_move`): it clears `lane` m² of the
    /// lane, the structure it carries (`carried`, its root) comes along, and it pushes the structures there
    /// (`bond::moved`)
    Passed { from: u32, to: u32, lane: f64, mass: f64, carried: Option<NodeId> },
    /// Children who landed with no node to join and less than one member, back to their parent (`copy::back`)
    Back(Box<copy::Litter>),
    /// Body `by`, of `n` members who watched and of `lineage` then, watched body `of` (`choose::watched`)
    Watch { by: u64, n: f64, lineage: u32, of: u64 },
    /// Body `id` does verb `on.0` to part `on.1` (`bond::act`; `bond::ANCHORS` anchors it; `bond::LET_GO` and
    /// `bond::TORN` let go of it, `bond::dropped`), or forms a part of the shape given (`bond::form`), as a part of node
    /// `node` if any (`choose::land_act`)
    Act { id: u64, on: Result<(usize, NodeId), [f32; 2]>, node: Option<u64> },
}

/// What is queued is applied (`land`): after a serial step, or by a test.
pub fn deliver(w: &mut World) {
    land(w);
}

/// The queue is taken and applied in rounds until nothing is left: in each, first the moves and the litters land
/// (`land_all`), then the other entries one by one in the order queued; what they queue in turn is the next round's.
pub(crate) fn land(w: &mut World) {
    loop {
        let all = std::mem::take(&mut w.life.queued);
        if all.is_empty() {
            break;
        }
        let (landing, rest): (Vec<Entry>, Vec<Entry>) = all.into_iter().partition(|e| matches!(e, Entry::Go { .. } | Entry::Litter(_)));
        land_all(w, landing);
        rest.into_iter().for_each(|e| apply(w, e));
    }
}

/// A move with its body, taken out of the cell it leaves, or a litter, landing in a cell.
enum Landing {
    Move { b: Box<Body>, from: u32, to: u32, pushed: f64, lane: f64, carries: bool, kin: bool },
    Litter(Box<copy::Litter>),
}

/// Moves and litters land, each in the chunk of the cell it goes to (`life::run`), in a chunk cell by cell and in a cell
/// in the order queued, so what they queue lies in the order of the cells. A moving body first leaves the cell it was
/// in; one that died or joined another since moves nowhere (its cost was paid).
fn land_all(w: &mut World, landing: Vec<Entry>) {
    let bounds = life::bounds(w);
    let mut per: Vec<Vec<(u32, usize, Landing)>> = (1..bounds.len()).map(|_| Vec::new()).collect();
    for (x, e) in landing.into_iter().enumerate() {
        let (cell, l) = match e {
            Entry::Go { id, from, to, pushed, lane, carries, kin } => {
                let Some(h) = w.life.find_in(from as usize, id) else { continue };
                (to, Landing::Move { b: Box::new(std::mem::take(&mut w.life[h])), from, to, pushed, lane, carries, kin })
            }
            Entry::Litter(x) => (x.c.cell, Landing::Litter(x)),
            _ => unreachable!("only moves and litters land"),
        };
        per[bounds.partition_point(|&b| b <= cell as usize) - 1].push((cell, x, l));
    }
    let load = per.iter().map(Vec::len).sum();
    per.iter_mut().for_each(|v| v.sort_by_key(|x| (x.0, x.1)));
    life::run(w, per, load, |_| (), |w, d, list, _| {
        for (_, _, l) in list.drain(..) {
            match l {
                Landing::Move { b, from, to, pushed, lane, carries, kin } => choose::land_move(w, d, *b, (from as usize, to as usize), (pushed, lane), (carries, kin)),
                Landing::Litter(x) => copy::land_litter(w, d, *x),
            }
        }
    });
}

/// Entry `e`, of those that do not land, is applied.
fn apply(w: &mut World, e: Entry) {
    match e {
        Entry::Go { .. } | Entry::Litter(_) => unreachable!("moves and litters land"),
        Entry::Passed { from, to, lane, mass, carried } => {
            lanes::clear(w, from as usize, to as usize, lane);
            bond::moved(w, carried, to as usize, mass);
        }
        Entry::Back(x) => copy::back(w, *x),
        Entry::Watch { by, n, lineage, of } => choose::watched(w, by, (n, lineage), of),
        Entry::Act { id, on: Ok((v @ (bond::LET_GO | bond::TORN), p)), .. } => bond::dropped(w, &mut Body { id, hold: Some(p), ..Default::default() }, v == bond::TORN),
        Entry::Act { id, on, node } => choose::land_act(w, id, on, node),
    }
}

//! The serial pass (laws.md 0): what body code in a chunk did outside its cell waits in `Life::queued`, in the order of
//! the cells and then the order made: its moves to another cell, its children who start in another cell, its watches
//! and its acts on parts. After the parallel pass they are applied one by one on one thread (`land`), each by its law on
//! the bodies and parts as they are then; one whose body or target is gone does nothing, as its law says.

use crate::{body::Body, bond, choose, copy, lanes, store::NodeId, World};

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

/// The queue is taken and applied in rounds until nothing is left: in each, first the moves and the litters land, each
/// a serial step on the cell it goes to (`World::in_cell`), then the other entries, in the order queued; what they
/// queue in turn is the next round's.
pub(crate) fn land(w: &mut World) {
    loop {
        let all = std::mem::take(&mut w.life.queued);
        if all.is_empty() {
            break;
        }
        let (landing, rest): (Vec<Entry>, Vec<Entry>) = all.into_iter().partition(|e| matches!(e, Entry::Go { .. } | Entry::Litter(_)));
        landing.into_iter().chain(rest).for_each(|e| apply(w, e));
    }
}

/// Entry `e` is applied.
fn apply(w: &mut World, e: Entry) {
    match e {
        Entry::Go { id, from, to, pushed, lane, carries, kin } => {
            // (A body that died or joined another since moves nowhere: its cost was paid.)
            let Some(h) = w.life.find_in(from as usize, id) else { return };
            let b = std::mem::take(&mut w.life[h]);
            w.in_cell(to as usize, |w, d| choose::land_move(w, d, b, (from as usize, to as usize), (pushed, lane), (carries, kin)));
        }
        Entry::Litter(x) => w.in_cell(x.c.cell as usize, |w, d| copy::land_litter(w, d, *x)),
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

//! The outbox (laws.md 0): what a body's update in a batch does outside it is queued in its domain's outbox, in the
//! order made: its moves to another cell, its children who start in another cell, its watches and its acts on parts.
//! When its domain's task ends, each body that moves leaves the domain's storage with its move (`Domain::post`). Then
//! (`land`) the moves and the litters land, each in the domain of the cell it goes to, in the order of the domains they
//! come from and then the order made, every domain on its own (on the pool); what a landing does to things no domain
//! owns (a lane, the structures, a parent elsewhere or the ground) is queued in turn. Then the other entries are applied
//! one by one, domain by domain in the order made, and then what landing and applying queued. Each kind is applied by
//! its law on the bodies and parts as they are then; one whose body or target is gone does nothing, as its law says.

use crate::{body::Body, life::{Domain, Held}};
use crate::{bond, choose, copy, lanes, store::NodeId, World};

/// One effect of an update outside it.
#[derive(Clone, Debug)]
pub enum Entry {
    /// Body `id` moves from cell `from` to `to` (`choose::go`), having spent `pushed` J pushing through the bodies
    /// there, which clears `lane` m² of its lane; `carries`: it paid to carry the structure it holds; with `kin`, it is
    /// members of a node who join their kind there; `body`: the body itself, once it left its domain's storage
    /// (`Domain::post`)
    Go { id: u64, from: u32, to: u32, pushed: f64, lane: f64, carries: bool, kin: bool, body: Option<Box<Body>> },
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

impl Entry {
    /// The lineage and born rows it names become `to(id, what)` (`Life::remap`).
    pub(crate) fn remap(&mut self, to: &impl Fn(u32, Held) -> u32) {
        let body = |b: &mut Body| (b.lineage, b.genome) = (to(b.lineage, Held::Lineage), to(b.genome, Held::Rows));
        match self {
            Entry::Go { body: Some(b), .. } => body(b),
            Entry::Litter(x) | Entry::Back(x) => body(&mut x.c),
            Entry::Watch { lineage, .. } => *lineage = to(*lineage, Held::Lineage),
            _ => {}
        }
    }
    /// The cell it lands in, for a move or a litter (they land by the domain of that cell).
    pub(crate) fn lands_in(&self) -> Option<usize> {
        match self {
            Entry::Go { to, .. } => Some(*to as usize),
            Entry::Litter(x) => Some(x.c.cell as usize),
            _ => None,
        }
    }
}

/// The outboxes of the domains that ran are applied (`land`): after a serial step, or by a test.
pub fn deliver(w: &mut World) {
    land(w, false, Vec::new());
}

/// The outboxes of the domains that ran are taken (`Life::take_queued`). The moves and the litters land per domain of
/// their cell, the domains taken out of the world and run on the pool (`survey::run`; with `reversed` the domains
/// start in reversed order, only for the order test), while the moves gathered before are booked beside them
/// (`World::book_moves_beside`); the domains of `faced` and those that landed something make their faces for the next
/// batch then, after landing (`Domain::make_next_faces`). The domains back and the world settled (`World::settle`:
/// also where the bodies that moved are stored), the other entries are applied one by one in order; then the same for
/// what landing and applying queued, until nothing is left; last the moves gathered are booked.
pub(crate) fn land(w: &mut World, reversed: bool, mut faced: Vec<usize>) {
    loop {
        let (to, rest) = w.life.take_queued();
        if to.is_empty() && rest.is_empty() && faced.is_empty() {
            break;
        }
        let face = !faced.is_empty();
        faced.extend(&to);
        faced.sort_unstable();
        faced.dedup();
        // The work of each: the entries it lands and the bodies whose faces it makes.
        let work: Vec<_> = std::mem::take(&mut faced).into_iter().map(|k| (k, w.life.domains[k].landing(face), ())).collect();
        let ran: Vec<usize> = work.iter().map(|x| x.0).collect();
        w.book_moves_beside(|w| {
            crate::survey::run(w, work, reversed, |_| (), |w, d, _, _| {
                for e in std::mem::take(&mut d.inbox) {
                    land_one(w, d, e);
                }
                if face {
                    d.make_next_faces(w);
                }
            })
        });
        ran.into_iter().for_each(|k| w.settle(k..k + 1));
        rest.into_iter().for_each(|e| apply(w, e));
    }
    w.book_moves();
}

/// Entry `e`, a move or a litter, lands in domain `d`, the domain of its cell.
fn land_one(w: &World, d: &mut Domain, e: Entry) {
    match e {
        Entry::Go { from, to, pushed, lane, carries, kin, body: Some(b), .. } => choose::land_move(w, d, *b, (from as usize, to as usize), (pushed, lane), (carries, kin)),
        Entry::Litter(x) => copy::land_litter(w, d, *x),
        _ => unreachable!("only moves and litters land by domain"),
    }
}

/// Entry `e`, of those that touch what no domain owns, is applied.
fn apply(w: &mut World, e: Entry) {
    match e {
        Entry::Passed { from, to, lane, mass, carried } => {
            lanes::clear(w, from as usize, to as usize, lane);
            bond::moved(w, carried, to as usize, mass);
        }
        Entry::Back(x) => copy::back(w, *x),
        Entry::Watch { by, n, lineage, of } => choose::watched(w, by, (n, lineage), of),
        Entry::Act { id, on: Ok((v @ (bond::LET_GO | bond::TORN), p)), .. } => bond::dropped(w, &mut Body { id, hold: Some(p), ..Default::default() }, v == bond::TORN),
        Entry::Act { id, on, node } => choose::land_act(w, id, on, node),
        Entry::Go { .. } | Entry::Litter(_) => unreachable!("moves and litters land by domain"),
    }
}

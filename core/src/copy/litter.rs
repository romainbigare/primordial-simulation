//! Children who start in another cell than their parent's: a litter queued until the serial pass (`Litter`), landing
//! there (`land_litter`) or going back to their parent (`back`).

use super::{kin, start};
use crate::body::{remains, Body};
use crate::brain::shake;
use crate::event::Event;
use crate::life::{Block, Chunk, Entry};
use crate::nodes::{add, Lot, Own};
use crate::rng::{Purpose, Rng};
use crate::World;

/// Children that start in another cell than their parent's (`emit`), queued until the serial pass (`land_litter`): their
/// body (in the cell they start in), their born rows (none: their parent's), their brain (`life::Brains::all`, to take
/// into their cell's brains), the generation's change of their born rows still to draw (`row_change`; then their brain
/// is their parent's rows alone, all a join reads), the cell their parent was in, and their birth.
#[derive(Clone, Debug)]
pub struct Litter {
    pub(crate) c: Body,
    pub(super) rows: Option<Vec<f32>>,
    pub(super) brain: Vec<f32>,
    pub(super) change: Option<f32>,
    pub(super) from: u32,
    pub(super) born: Event,
}

/// A litter (`emit`) lands in the serial pass (laws.md 0) in `d`, the chunk of its cell: its birth is recorded, and it
/// joins the young node of its kind and lineage there (`kin`) or starts a body of its own (`start`); it ate to the end
/// of the sub-step it was born in (`feed`), and its physics runs on from its birth after the landing
/// (`survey::batch_of`). Less than one member (of a node's children: an individual's are whole) with no
/// node to join goes back (`back`). A litter whose change of its born rows is still to draw joins as `emit` joins such
/// children; starting a body, it draws the change of its born rows and the same of its parent's rows it carries (what
/// its parent learned on top, as `Chunk::inherit` gives), keyed by the parent's id turned over, with no moving averages
/// and no trace.
pub(crate) fn land_litter(w: &World, d: &mut Chunk, x: Litter) {
    let i = x.c.cell as usize;
    let hit = kin(w, d, i, x.c.lineage, false);
    if hit.is_none() && x.c.count < 1.0 {
        return d.out.push(Entry::Back(Box::new(x)));
    }
    let Litter { mut c, rows, mut brain, change, born, from } = x;
    d.went(from as usize, i, c.mass(&w.chem));
    d.events.push(born);
    let Some(sd) = change else {
        c.brain = d.brains(i).take_in(&brain);
        return start(w, d, c, rows, hit);
    };
    if let Some(t) = hit {
        let row_spread = c.row_spread + sd * sd;
        return add(w, d, t, Lot::of(&mut c), (&c.genes, &c.spread, c.body_spread), (c.lineage, c.genome, Some(Own::Carried(&brain)), row_spread));
    }
    let key = || Rng::new(w.seed, w.key_at(w.time), !c.parent, Purpose::Mutate);
    let shape = d.shape_of(w, c.lineage).filter(|_| !brain.is_empty());
    if shape.is_some() && sd > 0.0 {
        shake(&mut brain, &mut key(), sd);
    }
    let rows = (sd > 0.0).then(|| {
        let mut v = d.born_of(w, c.genome).to_vec();
        shake(&mut v, &mut key(), sd);
        v
    });
    c.brain = shape.map_or_else(Block::default, |s| d.brains(i).alloc(s, &brain));
    start(w, d, c, rows, None);
}

/// Children of a litter who found no node to join (`land_litter`) go back into their parent's brood if the parent
/// lives; else what they hold goes to the ground of the cell their parent was in, as remains do (`body::remains`).
pub(crate) fn back(w: &mut World, x: Litter) {
    let Litter { mut c, from, .. } = x;
    if let Some(p) = w.life.find(c.parent) {
        let (mi, p) = (Body::matter_at(&w.chem).unwrap_or(0), &mut w.life[p]);
        p.store.iter_mut().zip(&c.store).for_each(|(a, x)| *a += x);
        (p.e, p.brood) = (p.e + c.e, p.brood + c.store[mi]);
    } else {
        c.cell = from;
        w.in_cell(from as usize, |w, d| remains(w, d, &c));
    }
}

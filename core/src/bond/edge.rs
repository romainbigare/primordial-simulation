//! An edge concentrates force (laws.md 3.3). Without a held part a body breaks matter bluntly: it gives way through its
//! whole width, so breaking it takes Σ V σ (`Body::breaking`). Pressed with the edge of a held part, only a zone as
//! wide as the edge gives way (Atkins 2009): the energy and the force to break each material fall by the edge's
//! thickness over the width of what it breaks, where the edge is harder than that material. The edge wears by the
//! work it does (Archard 1953).

use super::{at, fall_apart, held, restructure, shed, to_ground};
use crate::{body::Body, chem::Chemistry, life::{Cell, Chunk, Entry}, store::{Class, Node}, units::TABOR, World};

/// Scratch hardnesses within this share of each other are one: walls of substances of one hardness read it to rounding
/// (their mean by volume, `Part::walls`).
const SAME_HARDNESS: f64 = 1e-9;

/// The edge of a part a body holds: the part's id, its walls' thickness, m, and their scratch hardness.
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    part: u64,
    thick: f64,
    hard: f64,
}

impl Edge {
    /// The share of the blunt energy and force to break matter of scratch hardness `hard`, `width` m across, that this
    /// edge needs: its thickness over that width where it is harder than the matter (an indenter marks only what is
    /// softer than itself, Tabor 1951), else all of it.
    fn cut_at(&self, hard: f64, width: f64) -> f64 {
        if hard < self.hard * (1.0 - SAME_HARDNESS) { (self.thick / width.max(1e-30)).min(1.0) } else { 1.0 }
    }
    /// `cut_at` for substance `k`.
    pub fn cut(&self, chem: &Chemistry, k: usize, width: f64) -> f64 {
        self.cut_at(chem.subs[k].hardness, width)
    }
    /// `cut_at` for a joint of `area` m² between walls of hardness `a` and `b`: the softer gives way.
    pub(crate) fn cut_joint(&self, (a, b): (f64, f64), area: f64) -> f64 {
        self.cut_at(a.min(b), area.sqrt())
    }
}

/// The edge of the part body `b` holds in its cell `c`, if its walls have a breaking stress (a solid with none given
/// holds no edge).
pub fn edge(c: &Cell, w: &World, b: &Body) -> Option<Edge> {
    let pt = &c.parts[held(c, b)?];
    let walls = pt.walls(&w.chem);
    (walls.rupture > 0.0).then(|| Edge { part: pt.id, thick: pt.thickness(&w.chem), hard: walls.hardness })
}

/// What an edge leaves a body of `n` members, one of whom holds it, of the energy and force to break a thing that it
/// alone cuts to the share `cut`: (the share left, the share of the blunt work done through the edge). Its other
/// members break bluntly.
pub fn edge_leaves(n: f64, cut: f64) -> (f64, f64) {
    if cut >= 1.0 {
        return (1.0, 0.0);
    }
    let one = 1.0 / n.max(1.0);
    (1.0 - one + one * cut, one * cut)
}

/// The edge `e` did `work` J: it wears in the landing of the acts on parts (`blunted`), where parts change.
pub fn blunt(d: &mut Chunk, e: &Edge, work: f64) {
    if work > 0.0 {
        d.out.push(Entry::Blunted { part: e.part, work });
    }
}

/// The part named `part` of cell `i` of chunk `d` did `work` J with its edge: by Archard's law it loses the volume
/// `abrade` × work / H of its walls, H = `TABOR` × their breaking stress, each substance in its share, which goes to its
/// cell's ground with its heat (`shed`). Walls left under `min_kg` fall apart (`fall_apart`).
pub(crate) fn blunted(w: &World, d: &mut Chunk, i: usize, part: u64, work: f64) {
    let Some(p) = at(d.at(i), part) else { return };
    let chem = &w.chem;
    let walls = d.at(i).parts[p].walls(chem);
    if walls.rupture <= 0.0 {
        return;
    }
    let f = (w.cfg.phys.abrade * work / (TABOR * walls.rupture) / walls.volume.max(1e-30)).min(1.0);
    d.moves.open(w, i, Node::new(Class::Part, p));
    let mut off = vec![[0.0; 2]; chem.ns()];
    shed(chem, &mut d.at_mut(i).parts[p], |_, _, _| f, &mut off);
    to_ground(w, d, i, &off);
    if d.at(i).parts[p].walls(chem).mass < w.cfg.phys.min_kg {
        fall_apart(w, d, i, p);
    } else {
        let c = d.at_mut(i);
        c.parts[p].resize(chem);
        restructure(w, c, p);
    }
}

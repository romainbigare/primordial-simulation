//! The moves of one update of a body or a part, gathered while it runs and booked by one `apply` at its end, with the
//! nodes of its cell as those moves leave them, for what the update reads of them (`World::g`, `a`, `heat_now`). An update that is not to change the world (a sample member's, `copy::sample`) drops its moves.

use super::{apply, held_more, Class, Move, Node};
use crate::chem::Chemistry;
use crate::ground::{heat_above_floor, share};
use crate::{World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";

/// The moves of the update running, and the nodes of its cell as they leave them.
#[derive(Clone, Debug, Default)]
pub struct Moves {
    /// Whose update it is: a body or a part (`Class::Body`, `Class::Part`) by its slot, the one end of every move but
    /// those of the lift (what dies or is torn in its update moves in its name); none when no update runs
    who: Option<Node>,
    list: Vec<Move>,
    /// Its cell's ground node, air node and bodies' node, in this order, with what each holds as the moves leave it: its
    /// amounts, mol of each (none for the bodies' node), and its heat, J
    near: Vec<(Node, Vec<f64>, f64)>,
    /// Heat a ground node could not give (`World::take_from_ground`), J
    short: f64,
}

impl Moves {
    /// Where node `x` is among those kept near, if it is one.
    fn at(&self, x: Node) -> Option<usize> {
        self.who?;
        let at = [Class::Ground, Class::Air, Class::Bodies].iter().position(|&c| c == x.class)?;
        (self.near[at].0 == x).then_some(at)
    }
    /// Amount of substance `k` in node `x` as the moves of the update running leave it, if `x` is near it.
    pub(crate) fn amount(&self, x: Node, k: usize) -> Option<f64> {
        self.at(x).map(|n| self.near[n].1[k])
    }
    /// Gather move `m` (between any two nodes; `take` and `give` are those of the one whose update it is); the nodes kept
    /// near change as `apply` will change them.
    pub(crate) fn push(&mut self, chem: &Chemistry, m: Move) {
        let (k, into) = (m.k as usize, m.heat - m.mol * held_more(chem, m.from.class, m.to.class, m.k as usize));
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)] {
            if let Some(n) = self.at(x).map(|n| &mut self.near[n]) {
                if mol != 0.0 {
                    n.1[k] += mol;
                }
                n.2 += heat;
            }
        }
        self.list.push(m);
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is.
    pub(crate) fn take(&mut self, chem: &Chemistry, x: Node, k: usize, mol: f64, heat: f64) {
        self.push(chem, Move { from: x, to: self.who.expect(RUNNING), k: k as u16, mol, heat });
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (which receives
    /// `heat` less the energy the mol hold apart from heat more there, `held_more`).
    pub(crate) fn give(&mut self, chem: &Chemistry, x: Node, k: usize, mol: f64, heat: f64) {
        self.push(chem, Move { from: self.who.expect(RUNNING), to: x, k: k as u16, mol, heat });
    }
    /// `m` mol of gas `k` go up from the one whose update it is into the air node of its cell (negative `m`: come down
    /// out of it), from a place `lift` J/kg below its mean height. The gas brings no heat of its own, and the one that
    /// gives it pays its latent heat (the air holds it, `held_more`); the air pays the lift from its heat into the
    /// world's potential energy, and a fall gives it back (`World::lift`).
    pub(crate) fn gas(&mut self, chem: &Chemistry, k: usize, m: f64, lift: f64) {
        let air = self.near[1].0;
        if m > 0.0 {
            self.give(chem, air, k, m, m * held_more(chem, self.who.expect(RUNNING).class, Class::Air, k));
        } else {
            self.take(chem, air, k, -m, 0.0);
        }
        let e = m * chem.subs[k].molar_mass * lift;
        self.push(chem, Move { from: air, to: Node::new(Class::Potential, 0), k: 0, mol: 0.0, heat: e });
    }
}

impl World {
    /// The moves of the update of `who` (a body or a part) in ground cell `i` open: none yet, and its cell's nodes as
    /// they are.
    pub fn open_moves(&mut self, i: usize, who: Node) {
        let (j, n, na) = (self.mesh.up(i, 0, AIR), self.mesh.cells().n(), self.mesh.level(AIR).n());
        let amounts = |x: &dyn Fn(usize) -> f64| (0..self.chem.ns()).map(x).collect();
        let (st, (g, a)) = (&self.store, (Class::Ground, Class::Air));
        let near = vec![(Node::ground(i), amounts(&|k| st.amounts(g)[k * n + i]), st.heat(g)[i]), (Node::air(j), amounts(&|k| st.amounts(a)[k * na + j]), st.heat(a)[j]), (Node::bodies(i), Vec::new(), st.heat(Class::Bodies)[i])];
        let m = &mut self.moves;
        (m.who, m.near, m.short) = (Some(who), near, 0.0);
        m.list.clear();
    }
    /// The moves of the update running are booked, in one call (`apply`), and they close.
    pub fn book_moves(&mut self) {
        let list = std::mem::take(&mut self.moves.list);
        apply(self, &[], &list, &[]);
        self.counts.clamped_heat += self.moves.short;
        (self.moves.list, self.moves.who) = (list, None);
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves of the update running leave it, J.
    pub(crate) fn heat_now(&self, x: Node) -> f64 {
        self.moves.at(x).map_or(self.store.heat(x.class)[x.index], |n| self.moves.near[n].2)
    }
    /// `d` mol of substance `k` (of `cp` J/(mol K)) and `want` J from ground node `i` (of heat capacity `cap`) to the
    /// one whose update it is (the heat and fusion enthalpy the mol take, or heat alone): no more heat than the node
    /// holds above `T_FLOOR` without their heat capacity, so it is never asked for more than it holds, and as much less
    /// of the mol; the heat it could not give is counted (`Counters::clamped_heat`). A negative `want` is heat given to
    /// the node, whole. Returns the mol and the heat that moved.
    pub(crate) fn take_from_ground(&mut self, i: usize, (k, d, cp): (usize, f64, f64), want: f64, cap: f64) -> (f64, f64) {
        let took = if want > 0.0 { want.min(heat_above_floor(self.heat_now(Node::ground(i)), cap - d * cp)) } else { want };
        self.moves.short += want - took;
        let mol = d * share(took, want);
        if mol != 0.0 || took != 0.0 {
            self.moves.take(&self.chem, Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
}

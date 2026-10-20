//! The moves of one update of a body or a part, gathered while it runs and booked by one `apply` at its end, with its
//! cell's nodes as those moves leave them. A sample member's update drops its moves.

use super::{apply, held_more, Class, Move, Node};
use crate::chem::Chemistry;
use crate::ground::{heat_above_floor, share};
use crate::units::T_FLOOR;
use crate::{World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";

/// The moves of the update running, and the nodes of its cell as they leave them.
#[derive(Clone, Debug, Default)]
pub struct Moves {
    /// Whose update it is (a body or a part by its slot), one end of every move but the lift's; none when none runs
    who: Option<Node>,
    list: Vec<Move>,
    /// Its cell's ground, air and bodies' node, with what each holds as the moves leave it: amounts, mol, and heat, J
    near: Vec<(Node, Vec<f64>, f64)>,
    /// What bodies see of each substance in its air node (`World::a_seen`) as the moves leave it, mol
    seen: Vec<f64>,
    /// Heat a ground node could not give (`World::take_from_ground`), J
    short: f64,
    /// While the bodies' day runs on threads: the nodes of the store whose numbers updates booked moves of, for the
    /// worlds of the threads (`waves`)
    #[cfg(feature = "threads")]
    pub(crate) booked: Option<Vec<Node>>,
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
    /// Gather move `m`; the nodes kept near change as `apply` will change them.
    pub(crate) fn push(&mut self, chem: &Chemistry, m: Move) {
        let (k, into) = (m.k as usize, m.heat - m.mol * held_more(chem, m.from.class, m.to.class, m.k as usize));
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)] {
            if let Some(at) = self.at(x) {
                let n = &mut self.near[at];
                if mol != 0.0 {
                    n.1[k] += mol;
                    // (The air node and the bodies' node under it, `at` 1 and 2, are what bodies see of the air.)
                    if at > 0 {
                        self.seen[k] += mol;
                    }
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
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside).
    pub(crate) fn give(&mut self, chem: &Chemistry, x: Node, k: usize, mol: f64, heat: f64) {
        self.push(chem, Move { from: self.who.expect(RUNNING), to: x, k: k as u16, mol, heat });
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas(&mut self, chem: &Chemistry, k: usize, m: f64, lift: f64) {
        let (air, bodies) = (self.near[1].0, self.near[2].0);
        if m > 0.0 {
            self.give(chem, bodies, k, m, m * held_more(chem, self.who.expect(RUNNING).class, Class::Bodies, k));
        } else {
            self.take(chem, air, k, -m, 0.0);
            let e = m * chem.subs[k].molar_mass * lift;
            self.push(chem, Move { from: air, to: Node::new(Class::Potential, 0), k: 0, mol: 0.0, heat: e });
        }
    }
}

impl World {
    /// The moves of the update of `who` in ground cell `i` open, with its cell's nodes as they are.
    pub fn open_moves(&mut self, i: usize, who: Node) {
        let (j, n, na) = (self.mesh.up(i, 0, AIR), self.mesh.cells().n(), self.mesh.level(AIR).n());
        let amounts = |x: &dyn Fn(usize) -> f64| (0..self.chem.ns()).map(x).collect();
        let (st, (g, a, b)) = (&self.store, (Class::Ground, Class::Air, Class::Bodies));
        let near = vec![(Node::ground(i), amounts(&|k| st.amounts(g)[k * n + i]), st.heat(g)[i]), (Node::air(j), amounts(&|k| st.amounts(a)[k * na + j]), st.heat(a)[j]), (Node::bodies(i), amounts(&|k| st.amounts(b)[k * n + i]), st.heat(b)[i])];
        let kids = self.mesh.level(AIR).children(j);
        let seen = (0..self.chem.ns()).map(|k| st.amounts(a)[k * na + j] + kids.iter().map(|&c| st.amounts(b)[k * n + c as usize]).sum::<f64>()).collect();
        let m = &mut self.moves;
        (m.who, m.near, m.seen, m.short) = (Some(who), near, seen, 0.0);
        m.list.clear();
    }
    /// The moves of the update running are booked, in one call (`apply`), and they close.
    pub fn book_moves(&mut self) {
        let list = std::mem::take(&mut self.moves.list);
        apply(self, &[], &list, &[]);
        #[cfg(feature = "threads")]
        if let Some(b) = &mut self.moves.booked {
            let stored = [Class::Ground, Class::Air, Class::Deep, Class::Bed, Class::Bodies];
            b.extend(list.iter().flat_map(|m| [m.from, m.to]).filter(|x| stored.contains(&x.class)));
        }
        self.counts.clamped_heat += self.moves.short;
        (self.moves.list, self.moves.who) = (list, None);
    }
    /// What bodies see of substance `k` in the air node of the update running (`a_seen`), if `j` is that node.
    pub(crate) fn seen_now(&self, k: usize, j: usize) -> Option<f64> {
        (self.moves.who.is_some() && self.moves.near[1].0.index == j).then(|| self.moves.seen[k])
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves of the update running leave it, J.
    pub(crate) fn heat_now(&self, x: Node) -> f64 {
        self.moves.at(x).map_or(self.store.heat(x.class)[x.index], |n| self.moves.near[n].2)
    }
    /// `d` mol of substance `k` and `want` J from ground node `i` to the one whose update it is: no more heat than the
    /// node holds above `T_FLOOR`, and as much less of the mol; what it could not give is counted. A negative `want` is
    /// heat given to the node, whole. Returns the mol and the heat that moved.
    pub(crate) fn take_from_ground(&mut self, i: usize, (k, d, cp): (usize, f64, f64), want: f64, cap: f64) -> (f64, f64) {
        // A share x of both leaves it e - x want - (cap - x d cp) T_FLOOR above the floor.
        let took = if want > 0.0 { want * share(heat_above_floor(self.heat_now(Node::ground(i)), cap), want - d * cp * T_FLOOR) } else { want };
        self.moves.short += want - took;
        let mol = d * share(took, want);
        if mol != 0.0 || took != 0.0 {
            self.moves.take(&self.chem, Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
}

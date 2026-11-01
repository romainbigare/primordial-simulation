//! The view of the store that body code of a chunk reads and writes (laws.md 0). Each cell it runs on sees its own
//! ground, bed and bodies' node, and its share of its air node (`Mesh::share` of its amounts, of its heat and of what
//! bodies see of it, `World::a_seen`), as the store held them when the cell was first touched, with its own moves since;
//! any other node as the store holds it, which nothing changes while chunks run. So a cell never overdraws its own nodes,
//! nor its air node with the other cells under it. A move changes only the nodes of the cell running, the potential
//! energy of lifts (summed apart per cell, `Store::lifted`), and a body, a part or a cover, whose law books its side. The
//! view keeps beside each number its change since it was taken, which the booking adds to the store once
//! (`World::settle`): what a body reads differs from what is booked by less than `ROUNDING`.

use super::apply::ROUNDING;
use super::{heat_received, held_more, Booker, Class, Move, Node};
use crate::ground::{heat_above_floor, share};
use crate::units::T_FLOOR;
use crate::{World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";

/// Where a cell's numbers lie in its row of the view (`Moves::view`), by the count of substances `ns`: its ground's
/// amounts, its bed's, its bodies' node's, its share of its air node's, its share of what bodies see of that, then the
/// heat of its ground, of its bodies' node and its share of its air node's.
#[derive(Clone, Copy)]
struct Row(usize);

impl Row {
    fn len(self) -> usize {
        5 * self.0 + 3
    }
    /// The first amount of the nodes of `class` (a ground, bed, bodies' or air node).
    fn amounts(self, class: Class) -> usize {
        self.0 * match class {
            Class::Ground => 0,
            Class::Bed => 1,
            Class::Bodies => 2,
            Class::Air => 3,
            _ => unreachable!("a view holds no node of {class:?}"),
        }
    }
    fn seen(self) -> usize {
        4 * self.0
    }
    /// The heat of the node of `class` (a ground, bodies' or air node; the bed holds none).
    fn heat(self, class: Class) -> usize {
        5 * self.0 + match class {
            Class::Ground => 0,
            Class::Bodies => 1,
            Class::Air => 2,
            _ => unreachable!("a view holds no heat of {class:?}"),
        }
    }
}

/// The moves gathered of a chunk, and its view.
#[derive(Clone, Debug, Default)]
pub struct Moves {
    /// Whose update it is (a body, a part or a cover), one end of every move; none when none runs
    who: Option<Node>,
    ns: usize,
    /// The cells taken into the view and their air nodes, in the order taken
    cells: Vec<(u32, u32)>,
    /// The cell of the update running, its air node and the start of its row
    own: Option<(usize, usize, usize)>,
    /// The rows of the cells taken (`Row`), each number with its change since it was taken from the store
    view: Vec<[f64; 2]>,
    /// Per cell taken: the potential energy its lifts gained less what its falls gave back (`Store::lifted`), and the
    /// heat its ground could not give (`Moves::take_from_ground`), J
    far: Vec<[f64; 2]>,
}

impl Moves {
    /// The moves of the update of `who` in ground cell `i` open, on the row of that cell: as the moves gathered left it
    /// if it was taken, else as the store holds it.
    pub fn open(&mut self, w: &World, i: usize, who: Node) {
        self.who = Some(who);
        if self.own.is_some_and(|o| o.0 == i) {
            return;
        }
        let j = w.mesh.up(i, 0, AIR);
        // (Cells are mostly taken in order: a cell past the last taken is new.)
        let taken = if self.cells.last().is_none_or(|c| (c.0 as usize) < i) { None } else { self.cells.iter().position(|c| c.0 as usize == i) };
        let at = taken.unwrap_or_else(|| self.take_in(w, i, j));
        self.own = Some((i, j, at * Row(self.ns).len()));
    }
    /// Cell `i`, under air node `j`, is taken into the view from the store; its place among the cells taken.
    fn take_in(&mut self, w: &World, i: usize, j: usize) -> usize {
        let (st, n, na, ns) = (&w.store, w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns());
        self.ns = ns;
        let f = w.mesh.share(i, AIR);
        for c in [Class::Ground, Class::Bed, Class::Bodies] {
            self.view.extend((0..ns).map(|k| [st.amounts(c)[k * n + i], 0.0]));
        }
        self.view.extend((0..ns).map(|k| [f * st.amounts(Class::Air)[k * na + j], 0.0]));
        self.view.extend((0..ns).map(|k| [f * w.a_seen(k, j), 0.0]));
        self.view.extend([st.heat(Class::Ground)[i], st.heat(Class::Bodies)[i], f * st.heat(Class::Air)[j]].map(|e| [e, 0.0]));
        self.cells.push((i as u32, j as u32));
        self.far.push([0.0; 2]);
        self.cells.len() - 1
    }
    /// Whether it gathered anything to book.
    pub(crate) fn gathered(&self) -> bool {
        !self.cells.is_empty()
    }
    /// Number `at` of the view changes by `d`. A take never passes what the view holds (a law that does overdraws its
    /// node; checked in debug builds), but of what bodies see of the air.
    fn alter(&mut self, at: usize, d: f64, seen: bool) {
        let x = &mut self.view[at];
        debug_assert!(seen || d >= 0.0 || x[0] + d >= -ROUNDING * (x[0].abs() + d.abs()), "a take of {d} from a view that holds {x:?} (at {at})");
        *x = [x[0] + d, x[1] + d];
    }
    /// The start of the row of the cell running if node `x` is one of its nodes (its ground, bed, bodies' node or air
    /// node).
    #[inline(always)]
    fn own_row(&self, x: Node) -> Option<usize> {
        let (i, j, at) = self.own?;
        match x.class {
            Class::Ground | Class::Bed | Class::Bodies => (x.index == i).then_some(at),
            Class::Air => (x.index == j).then_some(at),
            _ => None,
        }
    }
    /// The start of the row that a move of node `x` changes: none for a part, a body or a cover. Panics on a node of
    /// the store that is not of the cell running.
    #[inline(always)]
    fn row_of(&self, x: Node) -> Option<usize> {
        if matches!(x.class, Class::Part | Class::Body | Class::Cover) {
            return None;
        }
        assert!(self.own.is_some(), "{RUNNING}");
        Some(self.own_row(x).unwrap_or_else(|| panic!("a move of {x:?}, outside the cell running")))
    }
    /// Node `x`, at row `at`, gains `mol` of substance `k` and `heat` J (one end of a move).
    #[inline(always)]
    fn end(&mut self, (x, at): (Node, usize), k: usize, mol: f64, heat: f64) {
        debug_assert!(heat == 0.0 || x.class != Class::Bed, "heat moved to the bed, which holds none");
        let row = Row(self.ns);
        if mol != 0.0 {
            self.alter(at + row.amounts(x.class) + k, mol, false);
            // (The air node and the bodies' node are what bodies see of the air.)
            if matches!(x.class, Class::Air | Class::Bodies) {
                self.alter(at + row.seen() + k, mol, true);
            }
        }
        if heat != 0.0 {
            self.alter(at + row.heat(x.class), heat, false);
        }
    }
    /// Gather move `m` of the update running: the view changes as `apply` would change the store.
    pub(crate) fn gather(&mut self, w: &World, m: Move) {
        let into = heat_received(m.heat, m.mol, held_more(&w.chem, m.from.class, m.to.class, m.k as usize));
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)] {
            if let Some(at) = self.row_of(x) {
                self.end((x, at), m.k as usize, mol, heat);
            }
        }
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is, gathered as
    /// `gather` gathers that move (the updater's end is its law's).
    pub(crate) fn take(&mut self, x: Node, k: usize, mol: f64, heat: f64) {
        if let Some(at) = self.row_of(x) {
            self.end((x, at), k, -mol, -heat);
        }
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside),
    /// gathered as `gather` gathers that move.
    pub(crate) fn give(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        let who = self.who.expect(RUNNING);
        if let Some(at) = self.row_of(x) {
            self.end((x, at), k, mol, heat_received(heat, mol, held_more(&w.chem, who.class, x.class, k)));
        }
    }
    /// The amounts `m` (mol of each substance; those not 0) and then `heat` J go from the one whose update it is to node
    /// `x` of the store, each as `give` gives it.
    pub(crate) fn give_all(&mut self, w: &World, x: Node, m: &[f64], heat: f64) {
        let who = self.who.expect(RUNNING);
        let Some(at) = self.row_of(x) else { return };
        for (k, &a) in m.iter().enumerate().filter(|e| *e.1 != 0.0) {
            self.end((x, at), k, a, heat_received(0.0, a, held_more(&w.chem, who.class, x.class, k)));
        }
        self.end((x, at), 0, 0.0, heat_received(heat, 0.0, held_more(&w.chem, who.class, x.class, 0)));
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas(&mut self, w: &World, k: usize, m: f64, lift: f64) {
        let ((i, j, at), who) = (self.own.expect(RUNNING), self.who.expect(RUNNING));
        if m > 0.0 {
            self.give(w, Node::bodies(i), k, m, m * held_more(&w.chem, who.class, Class::Bodies, k));
        } else {
            // The fall gives the air its lift back, from the potential energy.
            self.take(Node::air(j), k, -m, 0.0);
            let e = m * w.chem.subs[k].molar_mass * lift;
            self.alter(at + Row(self.ns).heat(Class::Air), -e, false);
            self.far[at / Row(self.ns).len()][0] += e;
        }
    }
    /// Amount of substance `k` in node `x` (a ground, bed, bodies' or air node) as the moves gathered leave it, mol.
    #[inline(always)]
    pub(crate) fn amount(&self, w: &World, x: Node, k: usize) -> f64 {
        self.amounts(w, x)(k)
    }
    /// The amounts of node `x` (a ground, bed, bodies' or air node) as the cell running sees them: of substance `k`, mol,
    /// at `k`. One look-up of the view for a node read substance by substance.
    #[inline]
    pub(crate) fn amounts<'a>(&'a self, w: &'a World, x: Node) -> impl Fn(usize) -> f64 + 'a {
        let own = self.own_row(x).map(|at| at + Row(self.ns).amounts(x.class));
        let (store, n) = (w.store.amounts(x.class), if x.class == Class::Air { w.mesh.level(AIR).n() } else { w.mesh.cells().n() });
        move |k| own.map_or_else(|| store[k * n + x.index], |at| self.view[at + k][0])
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the cell running sees it, J.
    pub(crate) fn heat(&self, w: &World, x: Node) -> f64 {
        self.own_row(x).map_or_else(|| w.store.heat(x.class)[x.index], |at| self.view[at + Row(self.ns).heat(x.class)][0])
    }
    /// What bodies and parts see in air node `j` (`World::a_seen`) as the cell running sees it (its share, if it is the
    /// cell's own): of substance `k`, mol, at `k`.
    #[inline]
    pub(crate) fn seen<'a>(&'a self, w: &'a World, j: usize) -> impl Fn(usize) -> f64 + 'a {
        let at = self.own_row(Node::air(j)).map(|at| at + Row(self.ns).seen());
        move |k| at.map_or_else(|| w.a_seen(k, j), |at| self.view[at + k][0])
    }
    /// Temperature of air node `j` as the cell running sees it, K (`World::t_air`).
    pub(crate) fn t_air(&self, w: &World, j: usize) -> f64 {
        crate::flow::air_temp(self.heat(w, Node::air(j)), w.cap_air(self.seen(w, j)))
    }
    /// Heat capacity of ground node `i`'s active layer as the cell running sees it, J/K (`World::heat_cap_ground`).
    pub(crate) fn heat_cap_ground(&self, w: &World, i: usize) -> f64 {
        w.heat_cap_of(i, self.amounts(w, Node::ground(i)), self.amounts(w, Node::new(Class::Bed, i)))
    }
    /// `d` mol of substance `k` and `want` J from ground node `i` to the one whose update it is: no more heat than the
    /// node holds above `T_FLOOR`, and as much less of the mol; what it could not give is counted. A negative `want` is
    /// heat given to the node, whole. Returns the mol and the heat that moved.
    pub(crate) fn take_from_ground(&mut self, w: &World, i: usize, (k, d, cp): (usize, f64, f64), want: f64, cap: f64) -> (f64, f64) {
        // A share x of both leaves it e - x want - (cap - x d cp) T_FLOOR above the floor.
        let took = if want > 0.0 { want * share(heat_above_floor(self.heat(w, Node::ground(i)), cap), want - d * cp * T_FLOOR) } else { want };
        let at = self.row_of(Node::ground(i)).expect("a ground node");
        self.far[at / Row(self.ns).len()][1] += want - took;
        let mol = d * share(took, want);
        if mol != 0.0 || took != 0.0 {
            self.take(Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
    /// The cells taken, by index, with their place among the cells taken: in the order of their indices.
    fn in_order(&self) -> Vec<(u32, u32, usize)> {
        let mut v: Vec<_> = self.cells.iter().enumerate().map(|(x, &(i, j))| (i, j, x)).collect();
        v.sort_unstable();
        v
    }
    /// Its changes to the arrays of booker `b` (`store::book_by_array`): the change of each number the view holds of
    /// them, cell by cell in the order of their indices.
    pub(crate) fn book_into(&self, b: &mut Booker) {
        let (row, len, class, ks) = (Row(self.ns), b.len, b.class, b.ks.clone());
        for (i, j, x) in self.in_order() {
            let (start, index) = (x * row.len(), if class == Class::Air { j } else { i } as usize);
            let mut add = |at: usize, to: usize| {
                let d = self.view[start + at][1];
                if d != 0.0 {
                    b.add(to, d);
                }
            };
            match ks.clone() {
                Some(ks) => ks.for_each(|k| add(row.amounts(class) + k, k * len + index)),
                None if class != Class::Bed => add(row.heat(class), index),
                None => {}
            }
        }
    }
    /// `f` of the potential energy each cell taken gained by its lifts and the heat its ground could not give, J, in the
    /// order of their indices.
    pub(crate) fn each_cell(&self, mut f: impl FnMut(f64, f64)) {
        self.in_order().into_iter().for_each(|(_, _, x)| f(self.far[x][0], self.far[x][1]));
    }
}

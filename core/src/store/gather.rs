//! The view of the store that body code of a chunk reads and writes (laws.md 0). Each cell it runs on sees its own
//! ground, bed and bodies' node, and its share of its air node (`Mesh::share` of its amounts, of its heat and of what
//! bodies see of it, `World::a_seen`), as the store held them when the cell was first touched, with its own moves since;
//! any other node as the store holds it, which nothing changes while chunks run. So a cell never overdraws its own nodes,
//! nor its air node with the other cells under it. A move changes only the nodes of the cell running, the potential
//! energy of lifts (summed apart per cell, `Store::lifted`), and a body, a part or a cover, whose law books its side. The
//! view keeps beside each number its change since it was taken, which the booking adds to the store once
//! (`World::take_chunks`): what a body reads differs from what is booked by less than `ROUNDING`.

use super::apply::ROUNDING;
use super::{heat_received, held_more, Booker, Class, Node};
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
    /// The cells taken into the view and their air nodes, in the order of the cells (their rows in that order)
    cells: Vec<(u32, u32)>,
    /// The cell of the update running: its index, its air node, its share of that node (`Mesh::share`) and the start of
    /// its row once it is taken (the first move of the cell takes it; until then a read reads the store)
    own: Option<(usize, usize, f64, Option<usize>)>,
    /// The rows of the cells taken (`Row`), each number with its change since it was taken from the store
    view: Vec<[f64; 2]>,
    /// Per cell taken: the potential energy its lifts gained less what its falls gave back (`Store::lifted`), and the
    /// heat its ground could not give (`Moves::take_from_ground`), J
    far: Vec<[f64; 2]>,
    /// Whether a move changed a number of it since it was made
    changed: bool,
}

impl Moves {
    /// The moves of the update of `who` in ground cell `i` open, on the row of that cell: as the moves gathered left it
    /// if it was taken, else as the store holds it.
    pub fn open(&mut self, w: &World, i: usize, who: Node) {
        self.who = Some(who);
        if self.own.is_some_and(|o| o.0 == i) {
            return;
        }
        let at = self.cells.binary_search_by_key(&(i as u32), |c| c.0).ok().map(|x| x * Row(self.ns).len());
        self.own = Some((i, w.mesh.up(i, 0, AIR), w.mesh.share(i, AIR), at));
    }
    /// The start of the row of the cell running, which is taken into the view from the store if it was not.
    fn own_row_mut(&mut self, w: &World) -> usize {
        let (i, j, f, at) = self.own.expect(RUNNING);
        at.unwrap_or_else(|| {
            // (Cells are mostly taken in order: a cell past the last taken is new and goes last.)
            let x = self.cells.partition_point(|c| (c.0 as usize) < i);
            self.take_in(w, (i, j, f), x);
            let at = x * Row(self.ns).len();
            self.own = Some((i, j, f, Some(at)));
            at
        })
    }
    /// Cell `i`, under air node `j` of which it has the share `f`, is taken into the view from the store, at place `at`
    /// among the cells taken.
    fn take_in(&mut self, w: &World, (i, j, f): (usize, usize, f64), at: usize) {
        let (st, n, na, ns) = (&w.store, w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns());
        self.ns = ns;
        // (The row is written at the end of the view, then moved into its place if that is not the end.)
        let end = self.view.len();
        let row = &mut self.view;
        for c in [Class::Ground, Class::Bed, Class::Bodies] {
            row.extend((0..ns).map(|k| [st.amounts(c)[k * n + i], 0.0]));
        }
        row.extend((0..ns).map(|k| [f * st.amounts(Class::Air)[k * na + j], 0.0]));
        row.extend((0..ns).map(|k| [f * air_seen(w, k, j), 0.0]));
        row.extend([st.heat(Class::Ground)[i], st.heat(Class::Bodies)[i], f * st.heat(Class::Air)[j]].map(|e| [e, 0.0]));
        let added = self.view.len() - end;
        self.view[at * Row(ns).len()..].rotate_right(added);
        self.cells.insert(at, (i as u32, j as u32));
        self.far.insert(at, [0.0; 2]);
    }
    /// Room for the rows of `cells` cells of `ns` substances, so taking them in never copies the view.
    pub(crate) fn reserve(&mut self, cells: usize, ns: usize) {
        self.view.reserve(cells * Row(ns).len());
        self.cells.reserve(cells);
        self.far.reserve(cells);
    }
    /// Whether it gathered anything to book.
    pub(crate) fn gathered(&self) -> bool {
        self.changed
    }
    /// Number `at` of the view changes by `d`. A take never passes what the view holds, to rounding relative to what
    /// the node holds of that unit, the numbers `node` of the view, as `store::apply` checks a booking (a law that does
    /// overdraws its node; checked in debug builds); none for what bodies see of the air.
    fn alter(&mut self, at: usize, d: f64, node: Option<std::ops::Range<usize>>) {
        if cfg!(debug_assertions) && d < 0.0 && self.view[at][0] + d < 0.0 {
            let held: f64 = node.map_or(f64::INFINITY, |r| self.view[r].iter().map(|x| x[0].abs()).sum());
            assert!(self.view[at][0] + d >= -ROUNDING * (held + d.abs()), "a take of {d} from a view that holds {:?} (at {at})", self.view[at]);
        }
        let x = &mut self.view[at];
        *x = [x[0] + d, x[1] + d];
        self.changed = true;
    }
    /// If node `x` is one of the nodes of the cell running (its ground, bed, bodies' node or air node): the start of its
    /// row if it was taken, and the share of the store's numbers of `x` it sees (of its air node its share, else all).
    #[inline(always)]
    fn own_of(&self, x: Node) -> Option<(Option<usize>, f64)> {
        let (i, j, f, at) = self.own?;
        match x.class {
            Class::Ground | Class::Bed | Class::Bodies => (x.index == i).then_some((at, 1.0)),
            Class::Air => (x.index == j).then_some((at, f)),
            _ => None,
        }
    }
    /// The start of the row that a move of node `x` changes, taken in if it was not: none for a part, a body or a cover.
    /// Panics on a node of the store that is not of the cell running.
    #[inline(always)]
    fn row_of(&mut self, w: &World, x: Node) -> Option<usize> {
        if matches!(x.class, Class::Part | Class::Body | Class::Cover) {
            return None;
        }
        assert!(self.own_of(x).is_some(), "a move of {x:?}, outside the cell running");
        Some(self.own_row_mut(w))
    }
    /// Node `x`, at row `at`, gains `mol` of substance `k` and `heat` J (one end of a move).
    #[inline(always)]
    fn end(&mut self, (x, at): (Node, usize), k: usize, mol: f64, heat: f64) {
        debug_assert!(heat == 0.0 || x.class != Class::Bed, "heat moved to the bed, which holds none");
        let row = Row(self.ns);
        if mol != 0.0 {
            let node = at + row.amounts(x.class);
            self.alter(node + k, mol, Some(node..node + self.ns));
            // (The air node and the bodies' node are what bodies see of the air.)
            if matches!(x.class, Class::Air | Class::Bodies) {
                self.alter(at + row.seen() + k, mol, None);
            }
        }
        if heat != 0.0 {
            let e = at + row.heat(x.class);
            self.alter(e, heat, Some(e..e + 1));
        }
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is, gathered as
    /// `apply` would change the store (the updater's end is its law's).
    pub(crate) fn take(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        if let Some(at) = self.row_of(w, x) {
            self.end((x, at), k, -mol, -heat);
        }
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside),
    /// gathered as `apply` would change the store.
    pub(crate) fn give(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        let who = self.who.expect(RUNNING);
        if let Some(at) = self.row_of(w, x) {
            self.end((x, at), k, mol, heat_received(heat, mol, held_more(&w.chem, who.class, x.class, k)));
        }
    }
    /// The amounts `m` (substance and mol, substances in order; those not 0) and then `heat` J go from the one whose
    /// update it is to node `x` of the store, each as `give` gives it.
    pub(crate) fn give_all(&mut self, w: &World, x: Node, m: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        let who = self.who.expect(RUNNING);
        let Some(at) = self.row_of(w, x) else { return };
        for (k, a) in m.into_iter().filter(|e| e.1 != 0.0) {
            self.end((x, at), k, a, heat_received(0.0, a, held_more(&w.chem, who.class, x.class, k)));
        }
        self.end((x, at), 0, 0.0, heat_received(heat, 0.0, held_more(&w.chem, who.class, x.class, 0)));
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas(&mut self, w: &World, k: usize, m: f64, lift: f64) {
        let ((i, j, ..), who) = (self.own.expect(RUNNING), self.who.expect(RUNNING));
        if m > 0.0 {
            self.give(w, Node::bodies(i), k, m, m * held_more(&w.chem, who.class, Class::Bodies, k));
        } else {
            // The fall gives the air its lift back, from the potential energy.
            self.take(w, Node::air(j), k, -m, 0.0);
            let (e, at) = (m * w.chem.subs[k].molar_mass * lift, self.own_row_mut(w));
            let air = at + Row(self.ns).heat(Class::Air);
            self.alter(air, -e, Some(air..air + 1));
            self.far[at / Row(self.ns).len()][0] += e;
            self.changed = true;
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
        let (at, f) = self.own_of(x).unwrap_or((None, 1.0));
        let at = at.map(|at| at + Row(self.ns).amounts(x.class));
        let (store, n) = (w.store.amounts(x.class), if x.class == Class::Air { w.mesh.level(AIR).n() } else { w.mesh.cells().n() });
        move |k| at.map_or_else(|| f * store[k * n + x.index], |at| self.view[at + k][0])
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the cell running sees it, J.
    pub(crate) fn heat(&self, w: &World, x: Node) -> f64 {
        let (at, f) = self.own_of(x).unwrap_or((None, 1.0));
        at.map_or_else(|| f * w.store.heat(x.class)[x.index], |at| self.view[at + Row(self.ns).heat(x.class)][0])
    }
    /// What bodies and parts see in air node `j` (`World::a_seen`) as the cell running sees it (its share, if it is the
    /// cell's own): of substance `k`, mol, at `k`.
    #[inline]
    pub(crate) fn seen<'a>(&'a self, w: &'a World, j: usize) -> impl Fn(usize) -> f64 + 'a {
        let (at, f) = self.own_of(Node::air(j)).unwrap_or((None, 1.0));
        let at = at.map(|at| at + Row(self.ns).seen());
        move |k| at.map_or_else(|| f * air_seen(w, k, j), |at| self.view[at + k][0])
    }
    /// Temperature of air node `j` as the cell running sees it, K (`World::t_air`).
    pub(crate) fn t_air(&self, w: &World, j: usize) -> f64 {
        crate::flow::air_temp(self.heat(w, Node::air(j)), w.cap_air(self.seen(w, j)))
    }
    /// Heat capacity of ground node `i`'s active layer as the cell running sees it, J/K (`World::heat_cap_ground`).
    pub(crate) fn heat_cap_ground(&self, w: &World, i: usize) -> f64 {
        w.heat_cap_of(i, self.amounts(w, Node::ground(i)), self.amounts(w, Node::new(Class::Bed, i)))
    }
    /// `d` mol of loose substance `k` from ground node `i`, at its temperature `tg` K, to the one whose update it is, with
    /// the heat it holds there (and its fusion enthalpy if it is molten): as far as the ground can give that heat
    /// (`take_from_ground`). Returns the mol and the heat that moved.
    pub(crate) fn take_loose(&mut self, w: &World, i: usize, (k, d): (usize, f64), tg: f64) -> (f64, f64) {
        let cp = w.chem.subs[k].cp;
        let heat = d * (cp * tg + w.chem.fused_heat(k, tg, |j| self.amount(w, Node::ground(i), j)));
        let cap = self.heat_cap_ground(w, i);
        self.take_from_ground(w, i, (k, d, cp), heat, cap)
    }
    /// `d` mol of substance `k` and `want` J from ground node `i` to the one whose update it is: no more heat than the
    /// node holds above `T_FLOOR`, and as much less of the mol; what it could not give is counted. A negative `want` is
    /// heat given to the node, whole. Returns the mol and the heat that moved.
    pub(crate) fn take_from_ground(&mut self, w: &World, i: usize, (k, d, cp): (usize, f64, f64), want: f64, cap: f64) -> (f64, f64) {
        // A share x of both leaves it e - x want - (cap - x d cp) T_FLOOR above the floor.
        let took = if want > 0.0 { want * share(heat_above_floor(self.heat(w, Node::ground(i)), cap), want - d * cp * T_FLOOR) } else { want };
        let at = self.row_of(w, Node::ground(i)).expect("a ground node");
        self.far[at / Row(self.ns).len()][1] += want - took;
        self.changed = true;
        let mol = d * share(took, want);
        if mol != 0.0 || took != 0.0 {
            self.take(w, Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
}

/// What bodies and parts see of substance `k` in air node `j` (`World::a_seen`): as the parallel pass running found it at
/// its start (`Life::air_seen`), or as the store holds it now.
#[inline]
fn air_seen(w: &World, k: usize, j: usize) -> f64 {
    match w.life.air_seen.get(j) {
        Some(seen) => seen.get_or_init(|| (0..w.chem.ns()).map(|k| w.a_seen(k, j)).collect())[k],
        None => w.a_seen(k, j),
    }
}

impl Moves {
    /// Its changes to the arrays of booker `b` (`store::book_by_array`): the change of each number the view holds of
    /// them, cell by cell in the order of their indices.
    pub(crate) fn book_into(&self, b: &mut Booker) {
        let (row, len, class, ks) = (Row(self.ns), b.len, b.class, b.ks.clone());
        for (x, &(i, j)) in self.cells.iter().enumerate() {
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
    /// Per cell taken whose ground or bed it changed, in the order of their indices: what they gained of each substance
    /// `k`, mol, its ground's at `k` and its bed's at `ns + k`. Every move of a view is one of the life of its cell
    /// (`store::life_moved`).
    pub(crate) fn fell(&self) -> impl Iterator<Item = (usize, Vec<f64>)> + '_ {
        let (row, ns) = (Row(self.ns), self.ns);
        self.cells.iter().enumerate().filter_map(move |(x, &(i, _))| {
            let of = |c: Class| (0..ns).map(move |k| self.view[x * row.len() + row.amounts(c) + k][1]);
            let d: Vec<f64> = of(Class::Ground).chain(of(Class::Bed)).collect();
            d.iter().any(|&v| v != 0.0).then_some((i as usize, d))
        })
    }
    /// `f` of the potential energy each cell taken gained by its lifts and the heat its ground could not give, J, in the
    /// order of their indices.
    pub(crate) fn each_cell(&self, mut f: impl FnMut(f64, f64)) {
        self.far.iter().for_each(|x| f(x[0], x[1]));
    }
}

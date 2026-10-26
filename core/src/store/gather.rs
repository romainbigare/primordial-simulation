//! The moves of the bodies' updates of a domain in a batch, or of one serial step, gathered while they run and booked
//! when they end. Each domain keeps its own (`life::Domain::moves`), with a view of its nodes as those moves leave them
//! (laws.md 0): its air node, taken from the store when the domain opens, and the ground, bed and bodies' node of each
//! of its cells, taken when an update in the cell or a move first touches it (until then the store holds what the view
//! would; nothing else changes the store while a domain is open). A move changes only its domain's nodes, the
//! potential, whose lift is summed apart as `apply` sums it, and a body or a part, whose law books its side. A read of
//! another domain's node reads the store, which nothing changes until the moves are booked. The view keeps beside each
//! number its change since it was taken, which the booking adds to the store once, as `apply` adds. The view's sums
//! are plain, not compensated as `apply` compensates the ground and the air (`Store::carry`): what a body reads differs
//! from what is booked by less than `ROUNDING`. A checkpoint of the view can be rolled back: a sample member's update
//! changes nothing (`copy::sample`).

use super::apply::ROUNDING;
use super::{book_by_array, held_more, Booker, Class, Move, Node, Store};
use crate::ground::{heat_above_floor, share};
use crate::par::SERIAL_BELOW;
use crate::units::T_FLOOR;
use crate::{life::Domain, World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";
/// The array of the view that holds what bodies see of the air (`World::a_seen`): after the amounts and heats of the
/// classes it keeps (`2 * class` and `2 * class + 1`), up to `Class::Bodies` (`Class::Deep` among them keeps none).
const SEEN: usize = 2 * (Class::Bodies as usize + 1);

/// The moves gathered of a domain, and its view while it is open.
#[derive(Clone, Debug, Default)]
pub struct Moves {
    /// Whose update it is (a body or a part by its slot), one end of every move, and its ground cell; none when none
    /// runs
    who: Option<(Node, usize)>,
    /// The domain open and its air node, and the count of substances. While a domain is open the store does not change
    /// (only the booking writes it then, `World::book_moves_beside`, and closes it), so a node taken into the view early
    /// holds what it would later (checked in debug builds by `apply` and the air's booking, `flow::air`)
    pub(crate) open: Option<(usize, usize)>,
    ns: usize,
    /// The view of the domain open: by class, the amounts of its nodes (`[2 * class][x * ns + k]`, mol) and their heat
    /// (`[2 * class + 1][x]`, J), `x` a cell's slot in its domain (`Life::cell_slot`; the air node: 0); then at `SEEN`
    /// what bodies see of each substance in its air node, mol. Each number with its change since it was taken from the
    /// store (`Moves::book_into`)
    view: [Vec<[f64; 2]>; SEEN + 1],
    /// Which cells of the domain open the view holds, by slot (it holds its air node), and those cells by slot and
    /// index in the order taken in; the cell of the update running and its slot
    held: Vec<bool>,
    taken: Vec<(u32, u32)>,
    own: (usize, usize),
    /// Heat a ground node could not give (`Moves::take_from_ground`), J
    short: f64,
    /// Heat lifted into the potential (`Class::Potential`), J
    lift: f64,
    /// While a checkpoint is set, each number of the view changed since, with what it held (and its change) then
    undo: Option<Vec<(u8, u32, [f64; 2])>>,
}

/// A checkpoint of the moves of a domain: its view (through `Moves::undo`), the heat it lifted and could not take, and
/// the events and the counters an update changes (`Domain::checkpoint`).
pub struct Mark {
    lift: f64,
    short: f64,
    events: usize,
    rounded: [f64; 2],
}

impl Moves {
    /// Number `at` of array `arr` of the view changes by `d`; noted while a checkpoint is set. A take never passes what
    /// the view holds (a law that does overdraws its node; checked in debug builds).
    fn alter(&mut self, arr: usize, at: usize, d: f64) {
        let x = &mut self.view[arr][at];
        debug_assert!(arr == SEEN || d >= 0.0 || x[0] + d >= -ROUNDING * (x[0].abs() + d.abs()), "a take of {d} from a view that holds {x:?} (array {arr}, at {at})");
        if let Some(u) = &mut self.undo {
            u.push((arr as u8, at as u32, *x));
        }
        *x = [x[0] + d, x[1] + d];
    }
    /// Node `x`, at slot `at` of the view, gains `mol` of substance `k` and `heat` J (one end of a move).
    #[inline(always)]
    fn end(&mut self, (x, at): (Node, usize), k: usize, mol: f64, heat: f64) {
        debug_assert!(heat == 0.0 || x.class != Class::Bed, "heat moved to the bed, which holds none");
        if mol != 0.0 {
            self.alter(2 * x.class as usize, at * self.ns + k, mol);
            // (The air node and the bodies' nodes under it are what bodies see of the air.)
            if matches!(x.class, Class::Air | Class::Bodies) {
                self.alter(SEEN, k, mol);
            }
        }
        if heat != 0.0 {
            self.alter(2 * x.class as usize + 1, at, heat);
        }
    }
    /// Gather move `m` of the update running: the view changes as `apply` would change the store, at the slots of its
    /// ends (a part's or a body's has none).
    pub(crate) fn gather(&mut self, w: &World, m: Move) {
        let into = m.heat - m.mol * held_more(&w.chem, m.from.class, m.to.class, m.k as usize);
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)] {
            if let Some(at) = self.view_load(w, x) {
                self.end((x, at), m.k as usize, mol, heat);
            }
        }
    }
    /// The slot of node `x` in the view, if the view holds it.
    #[inline(always)]
    fn view_at(&self, w: &World, x: Node) -> Option<usize> {
        let (open, air) = self.open?;
        match x.class {
            Class::Air => (x.index == air).then_some(0),
            Class::Ground | Class::Bed | Class::Bodies => {
                let (d, at) = w.life.cell_slot(x.index);
                (d == open && self.held[at]).then_some(at)
            }
            _ => None,
        }
    }
    /// The moves of the update of `who` in ground cell `i` open, on the view of its domain: as the moves gathered left
    /// it if it is open, else its air node as the store holds it; the nodes of cell `i` are taken into it too (most
    /// moves of an update are with them and the air: `view_load` finds them first).
    pub fn open(&mut self, w: &World, i: usize, who: Node) {
        let ((d, at), j) = (w.life.cell_slot(i), w.mesh.up(i, 0, AIR));
        (self.who, self.own) = (Some((who, i)), (i, at));
        if self.open.is_none_or(|o| o.0 != d) {
            debug_assert!(self.open.is_none(), "domain {d} opened over another whose moves are not booked");
            let (n, na, ns, st, cells) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns(), &w.store, w.mesh.level(AIR).children(j));
            (self.open, self.ns) = (Some((d, j)), ns);
            self.taken.clear();
            self.held.clear();
            self.held.resize(cells.len(), false);
            // (Room for its numbers, the cells' taken from the store as they are first touched: each array holds at
            // most a number of each substance in each cell.)
            self.view.iter_mut().for_each(|v| v.resize((cells.len() * ns).max(v.len()), [0.0; 2]));
            let (a, b) = (st.amounts(Class::Air), st.amounts(Class::Bodies));
            for k in 0..ns {
                self.view[2 * Class::Air as usize][k] = [a[k * na + j], 0.0];
                self.view[SEEN][k] = [a[k * na + j] + cells.iter().map(|&c| b[k * n + c as usize]).sum::<f64>(), 0.0];
            }
            self.view[2 * Class::Air as usize + 1][0] = [st.heat(Class::Air)[j], 0.0];
        }
        if !self.held[at] {
            self.take_in(w, at, i);
        }
    }
    /// The slot of node `x` in the view of the domain open, where it is taken from the store if it is not yet: none
    /// for a part or a body. Panics on a node of the store outside the domain open.
    #[inline(always)]
    fn view_load(&mut self, w: &World, x: Node) -> Option<usize> {
        let ((open, air), own) = (self.open.expect(RUNNING), self.own);
        let at = match x.class {
            Class::Ground | Class::Bed | Class::Bodies if x.index == own.0 => return Some(own.1),
            Class::Air if x.index == air => return Some(0),
            Class::Ground | Class::Bed | Class::Bodies if w.life.domain_of(x.index) == open => w.life.cell_slot(x.index).1,
            Class::Part | Class::Body => return None,
            _ => panic!("a move of {x:?}, outside the domain open"),
        };
        if !self.held[at] {
            self.take_in(w, at, x.index);
        }
        Some(at)
    }
    /// The nodes of ground cell `i`, at slot `at` of the view, are taken into it from the store (out of `view_load`'s
    /// way: a cell is taken once in a batch).
    #[cold]
    fn take_in(&mut self, w: &World, at: usize, i: usize) {
        let (n, ns, st) = (w.mesh.cells().n(), self.ns, &w.store);
        for c in [Class::Ground, Class::Bed, Class::Bodies] {
            let (a, h, c) = (st.amounts(c), st.heat(c), c as usize);
            (0..ns).for_each(|k| self.view[2 * c][at * ns + k] = [a[k * n + i], 0.0]);
            if let Some(&e) = h.get(i) {
                self.view[2 * c + 1][at] = [e, 0.0];
            }
        }
        self.held[at] = true;
        self.taken.push((at as u32, i as u32));
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is, gathered as
    /// `gather` gathers that move (the updater's end is its law's).
    pub(crate) fn take(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        if let Some(at) = self.view_load(w, x) {
            self.end((x, at), k, -mol, -heat);
        }
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside),
    /// gathered as `gather` gathers that move.
    pub(crate) fn give(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        let who = self.who.expect(RUNNING).0;
        if let Some(at) = self.view_load(w, x) {
            self.end((x, at), k, mol, heat - mol * held_more(&w.chem, who.class, x.class, k));
        }
    }
    /// The amounts `m` (mol of each substance; those not 0) and then `heat` J go from the one whose update it is to node
    /// `x` of the store, a `give` each.
    pub(crate) fn give_all(&mut self, w: &World, x: Node, m: &[f64], heat: f64) {
        for (k, &a) in m.iter().enumerate().filter(|e| *e.1 != 0.0) {
            self.give(w, x, k, a, 0.0);
        }
        self.give(w, x, 0, 0.0, heat);
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas(&mut self, w: &World, k: usize, m: f64, lift: f64) {
        let ((who, i), air) = (self.who.expect(RUNNING), Node::air(self.open.expect(RUNNING).1));
        if m > 0.0 {
            self.give(w, Node::bodies(i), k, m, m * held_more(&w.chem, who.class, Class::Bodies, k));
        } else {
            self.take(w, air, k, -m, 0.0);
            // (The heat of the potential is summed apart, as `apply` sums it.)
            let e = m * w.chem.subs[k].molar_mass * lift;
            self.alter(2 * Class::Air as usize + 1, 0, -e);
            self.lift += e;
        }
    }
    /// Amount of substance `k` in node `x` (a ground, bed, bodies' or air node) as the moves gathered leave it, mol.
    #[inline(always)]
    pub(crate) fn amount(&self, w: &World, x: Node, k: usize) -> f64 {
        self.amounts(w, x)(k)
    }
    /// The amounts of node `x` (a ground, bed, bodies' or air node) as the moves gathered leave them: of substance `k`,
    /// mol, at `k`. One look-up of the view for a node read substance by substance.
    #[inline]
    pub(crate) fn amounts<'a>(&'a self, w: &'a World, x: Node) -> impl Fn(usize) -> f64 + 'a {
        let (a, at, step) = match self.view_at(w, x) {
            Some(at) => (self.view[2 * x.class as usize].as_flattened(), 2 * at * self.ns, 2),
            None => (w.store.amounts(x.class), x.index, if x.class == Class::Air { w.mesh.level(AIR).n() } else { w.mesh.cells().n() }),
        };
        move |k| a[at + k * step]
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves gathered leave it, J.
    pub(crate) fn heat(&self, w: &World, x: Node) -> f64 {
        self.view_at(w, x).map_or_else(|| w.store.heat(x.class)[x.index], |at| self.view[2 * x.class as usize + 1][at][0])
    }
    /// Amount of substance `k` that bodies and parts see in air node `j` (`World::a_seen`), as the moves gathered leave
    /// it, mol.
    pub(crate) fn a_seen(&self, w: &World, k: usize, j: usize) -> f64 {
        self.view_at(w, Node::air(j)).map_or_else(|| w.a_seen(k, j), |_| self.view[SEEN][k][0])
    }
    /// All that bodies and parts see in air node `j` (`a_seen` summed over the substances), mol.
    pub(crate) fn a_seen_all(&self, w: &World, j: usize) -> f64 {
        (0..w.chem.ns()).map(|k| self.a_seen(w, k, j)).sum()
    }
    /// Heat capacity of ground node `i`'s active layer as the moves gathered leave it, J/K (`World::heat_cap_ground`).
    pub(crate) fn heat_cap_ground(&self, w: &World, i: usize) -> f64 {
        w.heat_cap_of(i, self.amounts(w, Node::ground(i)), self.amounts(w, Node::new(Class::Bed, i)))
    }
    /// `d` mol of substance `k` and `want` J from ground node `i` to the one whose update it is: no more heat than the
    /// node holds above `T_FLOOR`, and as much less of the mol; what it could not give is counted. A negative `want` is
    /// heat given to the node, whole. Returns the mol and the heat that moved.
    pub(crate) fn take_from_ground(&mut self, w: &World, i: usize, (k, d, cp): (usize, f64, f64), want: f64, cap: f64) -> (f64, f64) {
        // A share x of both leaves it e - x want - (cap - x d cp) T_FLOOR above the floor.
        let took = if want > 0.0 { want * share(heat_above_floor(self.heat(w, Node::ground(i)), cap), want - d * cp * T_FLOOR) } else { want };
        self.short += want - took;
        let mol = d * share(took, want);
        if mol != 0.0 || took != 0.0 {
            self.take(w, Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
    /// Its changes to the arrays of booker `b` (`store::book_by_array`): the change of each number the view holds of
    /// them, of its air node and of each cell taken in.
    fn book_into(&self, b: &mut Booker) {
        // (No domain open: nothing gathered since the last booking.)
        let Some((_, j)) = self.open else { return };
        let air = [(0, j as u32)];
        // The nodes of the class the view holds, by slot and index.
        let nodes: &[(u32, u32)] = match b.class {
            Class::Air => &air,
            Class::Ground | Class::Bed | Class::Bodies => &self.taken,
            _ => &[],
        };
        let (c, ks, len, ns) = (b.class as usize, b.ks.clone(), b.len, self.ns);
        // Array `v` of the view, which holds slot `at`'s number at `at * step + k`, to number `x + index` of the class.
        let mut each = |v: &[[f64; 2]], step: usize, k: usize, x: usize| {
            for &(at, i) in nodes {
                let d = v[at as usize * step + k][1];
                if d != 0.0 {
                    b.add(x + i as usize, d);
                }
            }
        };
        match ks {
            Some(ks) => ks.for_each(|k| each(&self.view[2 * c], ns, k, k * len)),
            None => each(&self.view[2 * c + 1], 1, 0, 0),
        }
    }
    /// The moves gathered close, once booked: returns the heat lifted into the potential and the heat the ground nodes
    /// could not give, J.
    fn close(&mut self) -> (f64, f64) {
        (self.who, self.open) = (None, None);
        (std::mem::take(&mut self.lift), std::mem::take(&mut self.short))
    }
}

impl Domain {
    /// A checkpoint of the moves of the update running, of the domain's view, and of its events and counters, to roll
    /// back to (`roll_back`).
    pub(crate) fn checkpoint(&mut self) -> Mark {
        debug_assert!(self.moves.undo.is_none(), "one checkpoint at a time");
        self.moves.undo = Some(Vec::new());
        Mark { lift: self.moves.lift, short: self.moves.short, events: self.events.len(), rounded: self.counts.rounded }
    }
    /// Everything since checkpoint `m` is undone: the moves gathered, the view's numbers, the events and the counters.
    /// (A node first taken into the view since holds what the store holds again.)
    pub(crate) fn roll_back(&mut self, m: Mark) {
        let undo = self.moves.undo.take().expect("a checkpoint set");
        for &(arr, at, x) in undo.iter().rev() {
            self.moves.view[arr as usize][at as usize] = x;
        }
        (self.moves.lift, self.moves.short) = (m.lift, m.short);
        self.events.truncate(m.events);
        self.counts.rounded = m.rounded;
    }
}

/// The classes whose nodes a domain's view holds, which its booking writes.
const VIEWED: [Class; 4] = [Class::Ground, Class::Air, Class::Bed, Class::Bodies];

impl World {
    /// The moves gathered by the domains that ran since the last booking are booked (`Moves::book_into`): each array of
    /// the store by one task, which adds each domain's changes in the domains' order (`store::book_by_array`); what
    /// rounding leaves below zero is set to zero and counted (`Counters::rounded`), then the lift into the potential
    /// and the heat the ground nodes could not give (`Counters::clamped_heat`), domain by domain. With work enough
    /// for the pool, `f` runs beside it in the same call, the store out of the world meanwhile: `f` must not read the
    /// store (a landing reads none: it moves no matter of the store, laws.md 0) nor open a view.
    pub(crate) fn book_moves_beside<R: Send>(&mut self, f: impl FnOnce(&mut World) -> R + Send) -> R {
        let crate::life::Life { unbooked, domains, .. } = &mut self.life;
        // (A domain that ran but gathered nothing has no view open.)
        let open: Vec<usize> = unbooked.take().filter(|&k| domains[k].moves.open.is_some()).collect();
        let views: Vec<(usize, Moves)> = open.into_iter().map(|k| (k, std::mem::take(&mut domains[k].moves))).collect();
        let ns = self.chem.ns();
        let book = |store: &mut Store| book_by_array(store, ns, &VIEWED, views.len(), |b| views.iter().for_each(|(_, m)| m.book_into(b)));
        let (r, rounded) = if views.is_empty() {
            (f(self), [0.0; 2])
        } else if views.len() < SERIAL_BELOW {
            let rounded = book(&mut self.store);
            (f(self), rounded)
        } else {
            let mut store = std::mem::take(&mut self.store);
            let both = crate::par::join(|| f(self), || book(&mut store));
            self.store = store;
            both
        };
        let World { life, counts, store, .. } = self;
        counts.add_rounded(rounded);
        for (k, mut m) in views {
            let (lift, short) = m.close();
            store.add_far_heat(Class::Potential, 0, lift);
            counts.clamped_heat += short;
            debug_assert!(life.domains[k].moves.open.is_none(), "a view opened while the moves gathered were booked");
            life.domains[k].moves = m;
        }
        r
    }
    /// The moves gathered by the domains that ran since the last booking are booked (`book_moves_beside`).
    pub fn book_moves(&mut self) {
        self.book_moves_beside(|_| ());
    }
}

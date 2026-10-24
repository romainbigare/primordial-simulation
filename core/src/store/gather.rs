//! The moves of the bodies' updates of a domain in a batch, or of one serial step, gathered while they run and booked by
//! one `apply` per domain when they end. Each domain keeps its own (`life::Domain::moves`), with a view of its nodes as
//! those moves leave them (plan 3.4): the ground, bed and bodies' node of each of its cells and its air node, each taken
//! from the store when a move first touches it (until then the store holds what the view would). A read of another
//! domain's node reads the store, which nothing changes until the moves are booked; so does the potential, which no
//! body reads, its lift kept in the moves. The view's sums are plain, not compensated as `apply` compensates the ground
//! and the air (`Store::carry`): what a body reads differs from what is booked by less than `ROUNDING`. A checkpoint of
//! the view can be rolled back: a sample member's update changes nothing (`copy::sample`).

use super::{apply, held_more, Class, Move, Node};
use crate::chem::Chemistry;
use crate::ground::{heat_above_floor, share};
use crate::units::T_FLOOR;
use crate::{life::Domain, World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";
/// The classes whose nodes a view keeps lie before this one (`Class::Deep` among them keeps none).
const KEPT: usize = Class::Bodies as usize + 1;
/// The array of the view that holds what bodies see of the air (`World::a_seen`): after the amounts and heats of the
/// kept classes (`2 * class` and `2 * class + 1`).
const SEEN: usize = 2 * KEPT;
/// The domain open, and its air node, when none is.
const NONE: usize = usize::MAX;

/// The moves gathered of a domain, and its view while it is open.
#[derive(Clone, Debug)]
pub struct Moves {
    /// Whose update it is (a body or a part by its slot), one end of every move but the lift's, and its ground cell;
    /// none when none runs
    who: Option<(Node, usize)>,
    list: Vec<Move>,
    /// The domain open and its air node (`NONE`: none), and the count of substances
    open: usize,
    air: usize,
    ns: usize,
    /// The view of the domain open: by class, the amounts of its nodes (`[2 * class][x * ns + k]`, mol) and their heat
    /// (`[2 * class + 1][x]`, J), `x` a cell's slot in its domain (`Life::cell_slot`; the air node: 0); then at `SEEN`
    /// what bodies see of each substance in its air node, mol
    view: [Vec<f64>; SEEN + 1],
    /// Which cells of the domain open the view holds, by slot, and whether it holds its air node
    held: Vec<bool>,
    air_held: bool,
    /// Heat a ground node could not give (`Moves::take_from_ground`), J
    short: f64,
    /// While a checkpoint is set, each number of the view changed since, with the value it had
    undo: Option<Vec<(u8, u32, f64)>>,
}

/// A checkpoint of the moves of a domain: their count, its view (through `Moves::undo`), and the events and the
/// counters an update changes (`Domain::checkpoint`).
pub struct Mark {
    moves: usize,
    short: f64,
    events: usize,
    rounded: [f64; 2],
}

impl Default for Moves {
    fn default() -> Moves {
        let (list, view, held) = (Vec::new(), Default::default(), Vec::new());
        Moves { who: None, list, open: NONE, air: NONE, ns: 0, view, held, air_held: false, short: 0.0, undo: None }
    }
}

impl Moves {
    /// Number `at` of array `arr` of the view changes by `d`; noted while a checkpoint is set. A take never passes what
    /// the view holds (a law that does overdraws its node; checked in debug builds).
    fn alter(&mut self, arr: usize, at: usize, d: f64) {
        let v = &mut self.view[arr][at];
        debug_assert!(arr == SEEN || d >= 0.0 || *v + d >= -super::ROUNDING * (v.abs() + d.abs()), "a take of {d} from a view that holds {v} (array {arr}, at {at})");
        if let Some(u) = &mut self.undo {
            u.push((arr as u8, at as u32, *v));
        }
        *v += d;
    }
    /// Gather move `m`, whose ends lie at slots `at` of the view (none: not held); the view changes as `apply` will
    /// change the store.
    fn push(&mut self, chem: &Chemistry, m: Move, at: [Option<usize>; 2]) {
        let (k, ns, into) = (m.k as usize, self.ns, m.heat - m.mol * held_more(chem, m.from.class, m.to.class, m.k as usize));
        for ((x, mol, heat), at) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)].into_iter().zip(at) {
            let (Some(at), c) = (at, x.class as usize) else { continue };
            if mol != 0.0 {
                self.alter(2 * c, at * ns + k, mol);
                // (The air node and the bodies' nodes under it are what bodies see of the air.)
                if matches!(x.class, Class::Air | Class::Bodies) {
                    self.alter(SEEN, k, mol);
                }
            }
            if heat != 0.0 {
                self.alter(2 * c + 1, at, heat);
            }
        }
        self.list.push(m);
    }
}

impl Moves {
    /// The slot of node `x` in the view, if the view holds it.
    #[inline(always)]
    fn view_at(&self, w: &World, x: Node) -> Option<usize> {
        match x.class {
            Class::Air => (x.index == self.air && self.air_held).then_some(0),
            Class::Ground | Class::Bed | Class::Bodies => {
                let (d, at) = w.life.cell_slot(x.index);
                (d == self.open && self.held[at]).then_some(at)
            }
            _ => None,
        }
    }
    /// The moves of the update of `who` in ground cell `i` open, on the view of its domain: as the moves gathered left
    /// it if it is open, else empty, its nodes as the store holds them.
    pub fn open(&mut self, w: &World, i: usize, who: Node) {
        let d = w.life.domain_of(i);
        self.who = Some((who, i));
        if self.open == d {
            return;
        }
        let (cells, ns) = (w.mesh.level(AIR).children(w.mesh.up(i, 0, AIR)).len(), w.chem.ns());
        (self.open, self.air, self.ns, self.air_held) = (d, w.mesh.up(i, 0, AIR), ns, false);
        self.held.clear();
        self.held.resize(cells, false);
        // (Room for its numbers, taken from the store as they are first touched, `view_load`: the bed holds no heat.)
        let size = |arr: usize| match (Class::ALL.get(arr / 2), arr.is_multiple_of(2)) {
            _ if arr == SEEN => ns,
            (Some(Class::Air), amounts) => if amounts { ns } else { 1 },
            (Some(Class::Ground | Class::Bodies), false) => cells,
            (Some(Class::Ground | Class::Bed | Class::Bodies), true) => cells * ns,
            _ => 0,
        };
        for (arr, v) in self.view.iter_mut().enumerate() {
            v.resize(size(arr).max(v.len()), 0.0);
        }
    }
    /// Node `x`, if it is of the domain open, is taken into its view from the store (if it is not yet), with its air node
    /// too where its amounts change and bodies see them (`mol`): what bodies see of the air is then taken before any of
    /// it changes. Returns its slot in the view, if it is of the domain open.
    fn view_load(&mut self, w: &World, x: Node, mol: bool) -> Option<usize> {
        let at = match x.class {
            Class::Air if x.index == self.air => None,
            Class::Ground | Class::Bed | Class::Bodies => match w.life.cell_slot(x.index) {
                (d, at) if d == self.open => Some(at),
                _ => return None,
            },
            _ => return None,
        };
        if let Some(at) = at.filter(|&at| !self.held[at]) {
            let (n, i, ns, st) = (w.mesh.cells().n(), x.index, self.ns, &w.store);
            for c in [Class::Ground, Class::Bed, Class::Bodies] {
                let (a, h, c) = (st.amounts(c), st.heat(c), c as usize);
                (0..ns).for_each(|k| self.view[2 * c][at * ns + k] = a[k * n + i]);
                if let Some(&e) = h.get(i) {
                    self.view[2 * c + 1][at] = e;
                }
            }
            self.held[at] = true;
        }
        if (at.is_none() || x.class == Class::Bodies && mol) && !self.air_held {
            let (n, na, j, st) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), self.air, &w.store);
            let (a, b, cells) = (st.amounts(Class::Air), st.amounts(Class::Bodies), w.mesh.level(AIR).children(j));
            for k in 0..self.ns {
                self.view[2 * Class::Air as usize][k] = a[k * na + j];
                self.view[SEEN][k] = a[k * na + j] + cells.iter().map(|&c| b[k * n + c as usize]).sum::<f64>();
            }
            self.view[2 * Class::Air as usize + 1][0] = st.heat(Class::Air)[j];
            self.air_held = true;
        }
        Some(at.unwrap_or(0))
    }
    /// Gather move `m` of the update running. A move takes only from its domain's nodes (checked in debug builds).
    pub(crate) fn gather(&mut self, w: &World, m: Move) {
        let at = [self.view_load(w, m.from, m.mol != 0.0), self.view_load(w, m.to, m.mol != 0.0)];
        let store = matches!(m.from.class, Class::Ground | Class::Bed | Class::Bodies | Class::Air);
        debug_assert!(!store || (m.mol <= 0.0 && m.heat <= 0.0) || at[0].is_some(), "a take from {:?}, outside the domain open", m.from);
        self.push(&w.chem, m, at);
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is.
    pub(crate) fn take(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        let to = self.who.expect(RUNNING).0;
        self.gather(w, Move { from: x, to, k: k as u16, mol, heat });
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside).
    pub(crate) fn give(&mut self, w: &World, x: Node, k: usize, mol: f64, heat: f64) {
        let from = self.who.expect(RUNNING).0;
        self.gather(w, Move { from, to: x, k: k as u16, mol, heat });
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas(&mut self, w: &World, k: usize, m: f64, lift: f64) {
        let ((who, i), air) = (self.who.expect(RUNNING), Node::air(self.air));
        if m > 0.0 {
            self.give(w, Node::bodies(i), k, m, m * held_more(&w.chem, who.class, Class::Bodies, k));
        } else {
            self.take(w, air, k, -m, 0.0);
            let e = m * w.chem.subs[k].molar_mass * lift;
            self.gather(w, Move { from: air, to: Node::new(Class::Potential, 0), k: 0, mol: 0.0, heat: e });
        }
    }
    /// Amount of substance `k` in node `x` (a ground, bed, bodies' or air node) as the moves gathered leave it, mol.
    #[inline(always)]
    pub(crate) fn amount(&self, w: &World, x: Node, k: usize) -> f64 {
        match self.view_at(w, x) {
            Some(at) => self.view[2 * x.class as usize][at * self.ns + k],
            None => w.stock(x)(k),
        }
    }
    /// The amounts of node `x` (a ground, bed, bodies' or air node) as the moves gathered leave them: of substance `k`,
    /// mol, at `k`. One look-up of the view for a node read substance by substance.
    #[inline]
    pub(crate) fn amounts<'a>(&'a self, w: &'a World, x: Node) -> impl Fn(usize) -> f64 + 'a {
        let (a, at, step) = match self.view_at(w, x) {
            Some(at) => (&self.view[2 * x.class as usize][..], at * self.ns, 1),
            None => (w.store.amounts(x.class), x.index, if x.class == Class::Air { w.mesh.level(AIR).n() } else { w.mesh.cells().n() }),
        };
        move |k| a[at + k * step]
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves gathered leave it, J.
    pub(crate) fn heat(&self, w: &World, x: Node) -> f64 {
        match self.view_at(w, x) {
            Some(at) => self.view[2 * x.class as usize + 1][at],
            None => w.store.heat(x.class)[x.index],
        }
    }
    /// Amount of substance `k` that bodies and parts see in air node `j` (`World::a_seen`), as the moves gathered leave
    /// it, mol.
    pub(crate) fn a_seen(&self, w: &World, k: usize, j: usize) -> f64 {
        self.view_at(w, Node::air(j)).map_or_else(|| w.a_seen(k, j), |_| self.view[SEEN][k])
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
}

impl Domain {
    /// A checkpoint of the moves of the update running, of the domain's view, and of its events and counters, to roll
    /// back to (`roll_back`).
    pub(crate) fn checkpoint(&mut self) -> Mark {
        debug_assert!(self.moves.undo.is_none(), "one checkpoint at a time");
        self.moves.undo = Some(Vec::new());
        Mark { moves: self.moves.list.len(), short: self.moves.short, events: self.events.len(), rounded: self.counts.rounded }
    }
    /// Everything since checkpoint `m` is undone: the moves gathered, the view's numbers, the events and the counters.
    /// (A node first taken into the view since holds what the store holds again.)
    pub(crate) fn roll_back(&mut self, m: Mark) {
        let undo = self.moves.undo.take().expect("a checkpoint set");
        for &(arr, at, v) in undo.iter().rev() {
            self.moves.view[arr as usize][at as usize] = v;
        }
        self.moves.list.truncate(m.moves);
        self.moves.short = m.short;
        self.events.truncate(m.events);
        self.counts.rounded = m.rounded;
    }
}

impl World {
    /// The moves each domain gathered are booked, one call of `apply` for each domain in domain order, with the heat its
    /// ground nodes could not give, and they close.
    pub fn book_moves(&mut self) {
        for d in 0..self.life.domains.len() {
            let mut m = std::mem::take(&mut self.life.domains[d].moves);
            if !m.list.is_empty() {
                apply(self, &[], &m.list, &[]);
            }
            self.counts.clamped_heat += std::mem::take(&mut m.short);
            (m.who, m.open, m.air, m.air_held) = (None, NONE, NONE, false);
            m.list.clear();
            self.life.domains[d].moves = m;
        }
    }
}

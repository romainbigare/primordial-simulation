//! The moves of the bodies' updates of a batch, or of one update of a body or a part outside the batches, gathered while
//! they run and booked by one `apply` per domain when they end. The domain open keeps a view of its nodes as those
//! moves leave them (plan 3.4): the ground, bed and bodies' node of each of its cells and its air node, each taken from
//! the store when a move first touches it (until then the store holds what the view would). A read of another domain's
//! node reads the store, which nothing changes until the moves are booked; so does the potential, which no body reads,
//! its lift kept in the moves. A checkpoint of the view can be rolled back: a sample member's update changes nothing
//! (`copy::sample`).

use super::{apply, held_more, Class, Move, Node};
use crate::chem::Chemistry;
use crate::ground::{heat_above_floor, share};
use crate::units::T_FLOOR;
use crate::{World, AIR};

/// What a move made when no update runs says (moves are made inside an update).
const RUNNING: &str = "an update running";
/// The classes whose nodes a view keeps lie before this one (`Class::Deep` among them keeps none).
const KEPT: usize = Class::Bodies as usize + 1;
/// The array of the view that holds what bodies see of the air (`World::a_seen`): after the amounts and heats of the
/// kept classes (`2 * class` and `2 * class + 1`).
const SEEN: usize = 2 * KEPT;
/// The domain open, and its air node, when none is.
const NONE: usize = usize::MAX;

/// The moves gathered, the domains they came from, and the view of the domain open.
#[derive(Clone, Debug)]
pub struct Moves {
    /// Whose update it is (a body or a part by its slot), one end of every move but the lift's, and its ground cell;
    /// none when none runs
    who: Option<(Node, usize)>,
    list: Vec<Move>,
    /// Each domain whose moves are gathered, in the order they opened, with where its moves start in `list`; the last
    /// is the one open
    opened: Vec<(usize, usize)>,
    /// The domain open and its air node (`NONE`: none), and the count of substances
    open: usize,
    air: usize,
    ns: usize,
    /// The view of the domain open: by class, the amounts of the nodes it holds (`[2 * class][x * ns + k]`, mol) and
    /// their heat (`[2 * class + 1][x]`, J), `x` a cell's place in `held` (the air node: 0); then at `SEEN` what bodies
    /// see of each substance in its air node, mol
    view: [Vec<f64>; SEEN + 1],
    /// The cells whose nodes the view holds, in the order taken; per ground cell of the world, 1 + its place there (0:
    /// not held); whether it holds the air node
    held: Vec<u32>,
    place: Vec<u32>,
    air_held: bool,
    /// Heat a ground node could not give (`World::take_from_ground`), J
    short: f64,
    /// While a checkpoint is set, each number of the view changed since, with the value it had
    undo: Option<Vec<(u8, u32, f64)>>,
}

/// A checkpoint of the moves of a domain: their count, its view (through `Moves::undo`), and the events and the
/// counters an update changes (`World::checkpoint`).
pub struct Mark {
    moves: usize,
    short: f64,
    events: usize,
    rounded: [f64; 2],
}

impl Default for Moves {
    fn default() -> Moves {
        let (list, opened, view, held, place) = (Vec::new(), Vec::new(), Default::default(), Vec::new(), Vec::new());
        Moves { who: None, list, opened, open: NONE, air: NONE, ns: 0, view, held, place, air_held: false, short: 0.0, undo: None }
    }
}

impl Moves {
    /// The view holds nothing: what is read is read from the store.
    fn empty(&mut self) {
        self.held.drain(..).for_each(|i| self.place[i as usize] = 0);
        self.view.iter_mut().for_each(Vec::clear);
        self.air_held = false;
    }
    /// Number `at` of array `arr` of the view changes by `d`; noted while a checkpoint is set. A take never passes what
    /// the view holds (a law that does overdraws its node; checked in debug builds).
    fn change(&mut self, arr: usize, at: usize, d: f64) {
        let v = &mut self.view[arr][at];
        debug_assert!(arr == SEEN || d >= 0.0 || *v + d >= -super::ROUNDING * (v.abs() + d.abs()), "a take of {d} from a view that holds {v} (array {arr}, at {at})");
        if let Some(u) = &mut self.undo {
            u.push((arr as u8, at as u32, *v));
        }
        *v += d;
    }
    /// Gather move `m`, whose ends lie at places `at` of the view (none: not held); the view changes as `apply` will
    /// change the store.
    fn push(&mut self, chem: &Chemistry, m: Move, at: [Option<usize>; 2]) {
        let (k, ns, into) = (m.k as usize, self.ns, m.heat - m.mol * held_more(chem, m.from.class, m.to.class, m.k as usize));
        for ((x, mol, heat), at) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, into)].into_iter().zip(at) {
            let (Some(at), c) = (at, x.class as usize) else { continue };
            if mol != 0.0 {
                self.change(2 * c, at * ns + k, mol);
                // (The air node and the bodies' nodes under it are what bodies see of the air.)
                if matches!(x.class, Class::Air | Class::Bodies) {
                    self.change(SEEN, k, mol);
                }
            }
            if heat != 0.0 {
                self.change(2 * c + 1, at, heat);
            }
        }
        self.list.push(m);
    }
}

impl World {
    /// The place of node `x` in the view, if the view holds it.
    #[inline(always)]
    fn kept(&self, x: Node) -> Option<usize> {
        match x.class {
            Class::Air => (x.index == self.moves.air && self.moves.air_held).then_some(0),
            Class::Ground | Class::Bed | Class::Bodies => match self.moves.place.get(x.index) {
                Some(&p) if p > 0 => Some(p as usize - 1),
                _ => None,
            },
            _ => None,
        }
    }
    /// The moves of the update of `who` in ground cell `i` open, on the view of its domain: as this batch's moves left
    /// it if the domain is open, else empty, its nodes as the store holds them.
    pub fn open_moves(&mut self, i: usize, who: Node) {
        let d = self.life.place(i).0;
        self.moves.who = Some((who, i));
        if self.moves.open == d {
            return;
        }
        debug_assert!(self.moves.opened.iter().all(|o| o.0 != d), "domain {d} opened twice before its moves were booked");
        let (n, m) = (self.mesh.cells().n(), &mut self.moves);
        m.empty();
        m.place.resize(n, 0);
        m.opened.push((d, m.list.len()));
        (m.open, m.air, m.ns) = (d, self.mesh.up(i, 0, AIR), self.chem.ns());
    }
    /// Node `x`, if it is of the domain open, is taken into its view from the store (if it is not yet), with its air node
    /// too where its amounts change and bodies see them (`mol`): what bodies see of the air is then taken before any of
    /// it changes. Returns its place in the view, if it is of the domain open.
    fn load(&mut self, x: Node, mol: bool) -> Option<usize> {
        let cell = match x.class {
            Class::Air if x.index == self.moves.air => None,
            Class::Ground | Class::Bed | Class::Bodies if self.moves.place[x.index] > 0 || self.life.place(x.index).0 == self.moves.open => Some(x.index),
            _ => return None,
        };
        if let Some(i) = cell.filter(|&i| self.moves.place[i] == 0) {
            let (n, ns, st, m) = (self.mesh.cells().n(), self.moves.ns, &self.store, &mut self.moves);
            for c in [Class::Ground, Class::Bed, Class::Bodies] {
                let (a, h, c) = (st.amounts(c), st.heat(c), c as usize);
                m.view[2 * c].extend((0..ns).map(|k| a[k * n + i]));
                m.view[2 * c + 1].extend(h.get(i));
            }
            m.held.push(i as u32);
            m.place[i] = m.held.len() as u32;
        }
        if (cell.is_none() || x.class == Class::Bodies && mol) && !self.moves.air_held {
            let (n, na, j, st) = (self.mesh.cells().n(), self.mesh.level(AIR).n(), self.moves.air, &self.store);
            let (a, b, cells, m) = (st.amounts(Class::Air), st.amounts(Class::Bodies), self.life.cells_of(self.moves.open), &mut self.moves);
            m.view[2 * Class::Air as usize].extend((0..m.ns).map(|k| a[k * na + j]));
            m.view[2 * Class::Air as usize + 1].push(st.heat(Class::Air)[j]);
            m.view[SEEN].extend((0..m.ns).map(|k| a[k * na + j] + cells.iter().map(|&c| b[k * n + c as usize]).sum::<f64>()));
            m.air_held = true;
        }
        self.kept(x)
    }
    /// The moves gathered are booked, one call of `apply` for each domain in the order they opened, and they close.
    pub fn book_moves(&mut self) {
        let (list, opened) = (std::mem::take(&mut self.moves.list), std::mem::take(&mut self.moves.opened));
        for (x, &(_, from)) in opened.iter().enumerate() {
            let to = opened.get(x + 1).map_or(list.len(), |o| o.1);
            apply(self, &[], &list[from..to], &[]);
        }
        self.counts.clamped_heat += std::mem::take(&mut self.moves.short);
        let m = &mut self.moves;
        m.empty();
        (m.list, m.opened, m.who, m.open, m.air) = (list, opened, None, NONE, NONE);
        m.list.clear();
        m.opened.clear();
    }
    /// A checkpoint of the moves of the update running, of its domain's view, and of the events and counters, to roll
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
    /// Gather move `m` of the update running.
    pub(crate) fn gather(&mut self, m: Move) {
        let at = [self.load(m.from, m.mol != 0.0), self.load(m.to, m.mol != 0.0)];
        self.moves.push(&self.chem, m, at);
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is.
    pub(crate) fn take_move(&mut self, x: Node, k: usize, mol: f64, heat: f64) {
        let to = self.moves.who.expect(RUNNING).0;
        self.gather(Move { from: x, to, k: k as u16, mol, heat });
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside).
    pub(crate) fn give_move(&mut self, x: Node, k: usize, mol: f64, heat: f64) {
        let from = self.moves.who.expect(RUNNING).0;
        self.gather(Move { from, to: x, k: k as u16, mol, heat });
    }
    /// `m` mol of gas `k` go from the one whose update it is into its cell's bodies' node, which the air's step empties
    /// into the air with the lift (laws.md 1.5); the giver pays its latent heat. Negative, `-m` mol come down from its
    /// cell's air node, from `lift` J/kg below its mean height, giving the lift back to the air.
    pub(crate) fn gas_move(&mut self, k: usize, m: f64, lift: f64) {
        let ((who, i), air) = (self.moves.who.expect(RUNNING), Node::air(self.moves.air));
        if m > 0.0 {
            self.give_move(Node::bodies(i), k, m, m * held_more(&self.chem, who.class, Class::Bodies, k));
        } else {
            self.take_move(air, k, -m, 0.0);
            let e = m * self.chem.subs[k].molar_mass * lift;
            self.gather(Move { from: air, to: Node::new(Class::Potential, 0), k: 0, mol: 0.0, heat: e });
        }
    }
    /// Amount of substance `k` in node `x` (a ground, bed, bodies' or air node) as the moves gathered leave it, mol.
    #[inline(always)]
    pub(crate) fn amount_now(&self, x: Node, k: usize) -> f64 {
        match self.kept(x) {
            Some(at) => self.moves.view[2 * x.class as usize][at * self.moves.ns + k],
            None => self.store.amounts(x.class)[k * if x.class == Class::Air { self.mesh.level(AIR).n() } else { self.mesh.cells().n() } + x.index],
        }
    }
    /// What bodies see of each substance in air node `j` (`World::a_seen`) as the moves gathered leave it, if the view
    /// holds that node.
    pub(crate) fn seen_now(&self, j: usize) -> Option<&[f64]> {
        self.kept(Node::air(j)).map(|_| &self.moves.view[SEEN][..])
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves gathered leave it, J.
    pub(crate) fn heat_now(&self, x: Node) -> f64 {
        match self.kept(x) {
            Some(at) => self.moves.view[2 * x.class as usize + 1][at],
            None => self.store.heat(x.class)[x.index],
        }
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
            self.take_move(Node::ground(i), k, mol, took);
        }
        (mol, took)
    }
}

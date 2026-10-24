//! The moves of the bodies' updates of a batch, or of one update of a body or a part outside the batches, gathered while
//! they run and booked by one `apply` per domain when they end. The domain open keeps a view of its nodes as those
//! moves leave them (plan 3.4): the ground, bed and bodies' node of each of its cells and its air node, each taken from
//! the store when a move first touches it (until then the store holds what the view would). A read of another domain's
//! node reads the store, which nothing changes until the moves are booked; so does the potential, which no body reads,
//! its lift kept in the moves. The view's sums are plain, not compensated as `apply` compensates the ground and the air
//! (`Store::carry`): what a body reads differs from what is booked by less than `ROUNDING`. A checkpoint of the view can
//! be rolled back: a sample member's update changes nothing (`copy::sample`).

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
    /// The view of the domain open: by class, the amounts of its nodes (`[2 * class][x * ns + k]`, mol) and their heat
    /// (`[2 * class + 1][x]`, J), `x` a cell's slot in its domain (`Life::cell_slot`; the air node: 0); then at `SEEN`
    /// what bodies see of each substance in its air node, mol
    view: [Vec<f64>; SEEN + 1],
    /// Which cells of the domain open the view holds, by slot, and whether it holds its air node
    held: Vec<bool>,
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
        let (list, opened, view, held) = (Vec::new(), Vec::new(), Default::default(), Vec::new());
        Moves { who: None, list, opened, open: NONE, air: NONE, ns: 0, view, held, air_held: false, short: 0.0, undo: None }
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

impl World {
    /// The slot of node `x` in the view, if the view holds it.
    #[inline(always)]
    fn view_at(&self, x: Node) -> Option<usize> {
        match x.class {
            Class::Air => (x.index == self.moves.air && self.moves.air_held).then_some(0),
            Class::Ground | Class::Bed | Class::Bodies => {
                let (d, at) = self.life.cell_slot(x.index);
                (d == self.moves.open && self.moves.held[at]).then_some(at)
            }
            _ => None,
        }
    }
    /// The moves of the update of `who` in ground cell `i` open, on the view of its domain: as this batch's moves left
    /// it if the domain is open, else empty, its nodes as the store holds them.
    pub fn open_moves(&mut self, i: usize, who: Node) {
        let d = self.life.domain_of(i);
        self.moves.who = Some((who, i));
        if self.moves.open == d {
            return;
        }
        debug_assert!(self.moves.opened.iter().all(|o| o.0 != d), "domain {d} opened twice before its moves were booked");
        let (cells, ns, m) = (self.life.cells_of(d).len(), self.chem.ns(), &mut self.moves);
        m.opened.push((d, m.list.len()));
        (m.open, m.air, m.ns, m.air_held) = (d, self.mesh.up(i, 0, AIR), ns, false);
        m.held.clear();
        m.held.resize(cells, false);
        // (Room for its numbers, taken from the store as they are first touched, `view_load`: the bed holds no heat.)
        let size = |arr: usize| match (Class::ALL.get(arr / 2), arr.is_multiple_of(2)) {
            _ if arr == SEEN => ns,
            (Some(Class::Air), amounts) => if amounts { ns } else { 1 },
            (Some(Class::Ground | Class::Bodies), false) => cells,
            (Some(Class::Ground | Class::Bed | Class::Bodies), true) => cells * ns,
            _ => 0,
        };
        for (arr, v) in m.view.iter_mut().enumerate() {
            v.resize(size(arr).max(v.len()), 0.0);
        }
    }
    /// Node `x`, if it is of the domain open, is taken into its view from the store (if it is not yet), with its air node
    /// too where its amounts change and bodies see them (`mol`): what bodies see of the air is then taken before any of
    /// it changes. Returns its slot in the view, if it is of the domain open.
    fn view_load(&mut self, x: Node, mol: bool) -> Option<usize> {
        let at = match x.class {
            Class::Air if x.index == self.moves.air => None,
            Class::Ground | Class::Bed | Class::Bodies => match self.life.cell_slot(x.index) {
                (d, at) if d == self.moves.open => Some(at),
                _ => return None,
            },
            _ => return None,
        };
        if let Some(at) = at.filter(|&at| !self.moves.held[at]) {
            let (n, i, ns, st, m) = (self.mesh.cells().n(), x.index, self.moves.ns, &self.store, &mut self.moves);
            for c in [Class::Ground, Class::Bed, Class::Bodies] {
                let (a, h, c) = (st.amounts(c), st.heat(c), c as usize);
                (0..ns).for_each(|k| m.view[2 * c][at * ns + k] = a[k * n + i]);
                if let Some(&e) = h.get(i) {
                    m.view[2 * c + 1][at] = e;
                }
            }
            m.held[at] = true;
        }
        if (at.is_none() || x.class == Class::Bodies && mol) && !self.moves.air_held {
            let (n, na, j, st) = (self.mesh.cells().n(), self.mesh.level(AIR).n(), self.moves.air, &self.store);
            let (a, b, cells, m) = (st.amounts(Class::Air), st.amounts(Class::Bodies), self.life.cells_of(self.moves.open), &mut self.moves);
            for k in 0..m.ns {
                m.view[2 * Class::Air as usize][k] = a[k * na + j];
                m.view[SEEN][k] = a[k * na + j] + cells.iter().map(|&c| b[k * n + c as usize]).sum::<f64>();
            }
            m.view[2 * Class::Air as usize + 1][0] = st.heat(Class::Air)[j];
            m.air_held = true;
        }
        Some(at.unwrap_or(0))
    }
    /// The moves gathered are booked, one call of `apply` for each domain in domain order, whatever the order they
    /// opened in, and they close.
    pub fn book_moves(&mut self) {
        let (list, mut opened) = (std::mem::take(&mut self.moves.list), std::mem::take(&mut self.moves.opened));
        opened.push((NONE, list.len()));
        let mut spans: Vec<[usize; 3]> = opened.windows(2).map(|x| [x[0].0, x[0].1, x[1].1]).collect();
        spans.sort_unstable();
        for [_, from, to] in spans {
            apply(self, &[], &list[from..to], &[]);
        }
        self.counts.clamped_heat += std::mem::take(&mut self.moves.short);
        let m = &mut self.moves;
        (m.list, m.opened, m.who, m.open, m.air, m.air_held) = (list, opened, None, NONE, NONE, false);
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
    /// Gather move `m` of the update running. A move takes only from its domain's nodes (checked in debug builds).
    pub(crate) fn gather_move(&mut self, m: Move) {
        let at = [self.view_load(m.from, m.mol != 0.0), self.view_load(m.to, m.mol != 0.0)];
        let store = matches!(m.from.class, Class::Ground | Class::Bed | Class::Bodies | Class::Air);
        debug_assert!(!store || (m.mol <= 0.0 && m.heat <= 0.0) || at[0].is_some(), "a take from {:?}, outside the domain open", m.from);
        self.moves.push(&self.chem, m, at);
    }
    /// `mol` of substance `k` and `heat` J go from node `x` of the store to the one whose update it is.
    pub(crate) fn take_move(&mut self, x: Node, k: usize, mol: f64, heat: f64) {
        let to = self.moves.who.expect(RUNNING).0;
        self.gather_move(Move { from: x, to, k: k as u16, mol, heat });
    }
    /// `mol` of substance `k` and `heat` J go from the one whose update it is to node `x` of the store (`held_more` aside).
    pub(crate) fn give_move(&mut self, x: Node, k: usize, mol: f64, heat: f64) {
        let from = self.moves.who.expect(RUNNING).0;
        self.gather_move(Move { from, to: x, k: k as u16, mol, heat });
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
            self.gather_move(Move { from: air, to: Node::new(Class::Potential, 0), k: 0, mol: 0.0, heat: e });
        }
    }
    /// Amount of substance `k` in node `x` (a ground, bed, bodies' or air node) as the moves gathered leave it, mol.
    #[inline(always)]
    pub(crate) fn amount_now(&self, x: Node, k: usize) -> f64 {
        match self.view_at(x) {
            Some(at) => self.moves.view[2 * x.class as usize][at * self.moves.ns + k],
            None => self.store.amounts(x.class)[k * if x.class == Class::Air { self.mesh.level(AIR).n() } else { self.mesh.cells().n() } + x.index],
        }
    }
    /// The amounts of node `x` (a ground, bed, bodies' or air node) as the moves gathered leave them: of substance `k`,
    /// mol, at `k`. One look-up of the view for a node read substance by substance.
    #[inline]
    pub(crate) fn amounts_now(&self, x: Node) -> impl Fn(usize) -> f64 + '_ {
        let (a, at, step) = match self.view_at(x) {
            Some(at) => (&self.moves.view[2 * x.class as usize][..], at * self.moves.ns, 1),
            None => (self.store.amounts(x.class), x.index, if x.class == Class::Air { self.mesh.level(AIR).n() } else { self.mesh.cells().n() }),
        };
        move |k| a[at + k * step]
    }
    /// What bodies see of each substance in air node `j` (`World::a_seen`) as the moves gathered leave it, if the view
    /// holds that node.
    pub(crate) fn seen_now(&self, j: usize) -> Option<&[f64]> {
        self.view_at(Node::air(j)).map(|_| &self.moves.view[SEEN][..])
    }
    /// Heat of node `x` (a ground node, an air node or a bodies' node) as the moves gathered leave it, J.
    pub(crate) fn heat_now(&self, x: Node) -> f64 {
        match self.view_at(x) {
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

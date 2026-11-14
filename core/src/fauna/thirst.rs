//! A slot's thirst in a month (laws.md 3b): the key liquid it needs is the vapour that cools it at its work plus the
//! least it passes through; it has its share of the open liquid its cell offers and the liquid of what it eats. Where
//! that is less than its need its work is cut first (searching, handling and building together), down to where the heat
//! it must shed needs no more liquid than it has: less work makes less heat. Only where at no work at all its need is
//! still unmet do its members die, by the unmet share (`LACK`).

use super::body::Body;
use super::GROUPS;

/// Halvings of the search for the share of work a slot's liquid allows: to a millionth.
const HALVINGS: u32 = 20;

/// What a slot asks and has of liquid over the month, mol: its need at a share of work (`need`), and the liquid of what
/// it eats at full work.
#[derive(Clone, Copy, Default)]
pub(super) struct Thirst {
    /// Its least need and its vapour at no work, and what its vapour grows by at full work
    least: f64,
    rest: f64,
    work: f64,
    /// The liquid of what it eats at full work
    pub(super) eaten: f64,
}

impl Thirst {
    /// The slot of members `b`, searching `walk` m a s at full work, over `dt` s, with the liquid's heat of
    /// vaporisation `lv` J/mol, eating `eaten` mol of liquid at full work.
    pub(super) fn of(b: &Body, walk: f64, (dt, lv): (f64, f64), eaten: f64) -> Thirst {
        let all = b.n * dt;
        let rest = b.vapour(walk, 0.0, lv) * all;
        Thirst { least: b.least * all, rest, work: b.vapour(walk, 1.0, lv) * all - rest, eaten }
    }
    /// Its need at the share `work` of its work, mol: its least need and the vapour of the heat its coat does not pass.
    pub(super) fn need(&self, b: &Body, walk: f64, (dt, lv): (f64, f64), work: f64) -> f64 {
        self.least + b.vapour(walk, work, lv) * b.n * dt
    }
    /// What it asks of its cell at full work, mol: its whole need (the liquid of what it eats is a share of its work,
    /// which a cut would lose).
    pub(super) fn asks(&self) -> f64 {
        self.least + self.rest + self.work
    }
}

/// The share of work each slot keeps and the share of its members that die of thirst (`LACK`), where `given` mol of its
/// cell's liquid is each slot's: the most work at which its need is no more than that and the liquid of what it eats at
/// that work (eaten as its work); at no work, the share of its need unmet.
pub(super) fn work(bodies: &[Body; GROUPS], thirsts: &[Thirst; GROUPS], walks: &[f64; GROUPS], given: &[f64; GROUPS], (dt, lv): (f64, f64)) -> ([f64; GROUPS], [f64; GROUPS]) {
    let (mut work, mut lack) = ([1.0; GROUPS], [0.0; GROUPS]);
    for x in (0..GROUPS).filter(|&x| bodies[x].n > 0.0) {
        let (b, s) = (&bodies[x], &thirsts[x]);
        let short = |f: f64| s.need(b, walks[x], (dt, lv), f) - given[x] - f * s.eaten;
        if short(1.0) <= 0.0 {
            continue;
        }
        let at_rest = s.need(b, walks[x], (dt, lv), 0.0);
        if short(0.0) > 0.0 {
            (work[x], lack[x]) = (0.0, short(0.0) / at_rest);
            continue;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..HALVINGS {
            let mid = 0.5 * (lo + hi);
            if short(mid) > 0.0 { hi = mid } else { lo = mid }
        }
        work[x] = lo;
    }
    (work, lack)
}

/// Each slot's share of `open` mol of its cell's liquid: what it asks (`Thirst::asks`), all of it where the cell holds
/// enough for all, else by the share of what they ask.
pub(super) fn shares(thirsts: &[Thirst; GROUPS], open: f64) -> [f64; GROUPS] {
    let all: f64 = thirsts.iter().map(Thirst::asks).sum();
    let f = if all > open { open / all } else { 1.0 };
    std::array::from_fn(|x| thirsts[x].asks() * f)
}

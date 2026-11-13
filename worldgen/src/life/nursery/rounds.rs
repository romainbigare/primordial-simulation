//! The two tests by the law a class's garden runs (`Garden`): the race, where kinds start equal on bare ground and the
//! heaviest after some years go on; and the invasion, where a newcomer arrives as seeds into the grown stand and holds
//! there or not. What holds the stand at the end is the set the law keeps.

use super::garden::Garden;
use super::year::Month;
use super::{ARRIVE_KG, CHAMPIONS, GROW_YEARS, INVADE_YEARS, MUTANT_GENERATIONS, RACE_YEARS, START_KG};
use pcore::cover::{self, Parts, NG, RANGE};
use pcore::rng::{Purpose, Rng};
use pcore::World;

/// One kind in a class's garden: its genes and its starting kind there (`Cover::root`: a number of its own, kept by its
/// mutants), and whether it came by the race (else it came as seeds into a grown stand).
#[derive(Clone, Copy, Debug)]
pub(super) struct Entrant {
    pub genes: [f32; NG],
    pub root: u32,
    pub raced: bool,
}

/// A year kept of a class's cell, month by month, with the minerals' change in each (`Garden::month`).
pub(super) type Lived<'a> = (&'a [Month], &'a [Vec<(usize, f64, f64)>]);

/// A class's garden over its tests: its cell, the champions of its race and the kinds that came into its stand as seeds
/// and held (`entrants`), its stand once grown, its next own number for a newcomer, and the newcomers that came as seeds
/// and those of them that held.
#[derive(Clone)]
pub(super) struct Class {
    pub cell: usize,
    pub entrants: Vec<Entrant>,
    race: Vec<(Entrant, f64)>,
    pub garden: Option<Garden>,
    next: u32,
    pub came: u32,
    pub held: u32,
}

/// Each class numbers its newcomers in a span of this many, past the rows of `KINDS` (the known kinds keep their rows).
const PER_CLASS: u32 = 1 << 16;

/// Run `years` of the year `lived` (its months in turn).
pub(super) fn run(g: &mut Garden, w: &World, p: &Parts, (year, wash): Lived, years: u32) {
    for k in 0..years as usize * year.len() {
        g.month(w, p, &year[k % year.len()], &wash[k % year.len()]);
    }
}

impl Class {
    /// Class `class`, whose garden is cell `cell`: no kinds yet.
    pub fn new(class: usize, cell: usize) -> Class {
        let next = (class as u32 + 1) * PER_CLASS;
        Class { cell, entrants: Vec::new(), race: Vec::new(), garden: None, next, came: 0, held: 0 }
    }

    /// A newcomer drawn over each gene's whole range (`cover::RANGE`), with a number of its own.
    fn drawn(&mut self, w: &World, raced: bool) -> Entrant {
        let mut r = Rng::new(w.seed, self.next as u64, self.cell as u64, Purpose::Nursery);
        let genes = std::array::from_fn(|g| (RANGE[g].1 as f64 + r.f64() * (RANGE[g].2 - RANGE[g].1) as f64) as f32);
        self.next += 1;
        Entrant { genes, root: self.next - 1, raced }
    }

    /// A mutant of the kind `of` (`MUTANT_GENERATIONS` generations of `cover::mutate`, keyed by round `r`), its starting
    /// kind kept: it joins its kind's slot or founds its own as the law's seeds do.
    fn mutant(&self, w: &World, of: &Entrant, r: u32, raced: bool) -> Entrant {
        let mut rg = Rng::new(w.seed, r as u64, self.cell as u64, Purpose::Mutate);
        let genes = (0..MUTANT_GENERATIONS).fold(of.genes, |g, _| cover::mutate(&g, w.cfg.phys.cover_mut, &mut rg));
        Entrant { genes, root: of.root, raced }
    }

    /// `trials` newcomers on bare ground on `lived`, each alone: put at `START_KG` and grown `RACE_YEARS`, it scores the
    /// cover it then holds per m², kg (with the genes the law's own seeds and merges gave it); the `CHAMPIONS` best so far
    /// are kept (`race`). The newcomers, in turn: `known` (kinds known before, each once), then a mutant of a champion,
    /// a guest (other classes' champions, the nearest climate first) where `guests` has any, and a draw.
    pub fn race(&mut self, w: &World, p: &Parts, lived: Lived, (known, guests): (&[Entrant], &[Entrant]), trials: u32) {
        let t = lived.0[0].around.t;
        let mut known = known.iter();
        for r in 0..trials {
            let e = match (known.next(), r % 3) {
                (Some(&e), _) => e,
                (None, 0) if !self.race.is_empty() => self.mutant(w, &self.race[(r / 3) as usize % self.race.len()].0, r, true),
                (None, 1) if !guests.is_empty() => guests[(r / 3) as usize % guests.len()],
                _ => self.drawn(w, true),
            };
            let mut g = Garden::bare(&w.chem, self.cell, lived.0);
            g.put(w, p, (e.genes, e.root), START_KG, t);
            run(&mut g, w, p, lived, RACE_YEARS);
            let Some((root, genes, kg)) = g.stand(w, p).into_iter().next() else { continue };
            self.race.push((Entrant { genes, root, raced: true }, kg));
            self.race.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.root.cmp(&b.0.root)));
            self.race.truncate(CHAMPIONS);
        }
        self.entrants = self.race.iter().map(|e| e.0).collect();
    }

    /// The race's best grown alone from `START_KG` for `GROW_YEARS` into the stand: the others come as seeds
    /// (`invade`).
    pub fn grow(&mut self, w: &World, p: &Parts, lived: Lived, known: &[Entrant]) {
        let mut g = Garden::bare(&w.chem, self.cell, lived.0);
        let sown: Vec<Entrant> = self.race.iter().map(|e| e.0).chain(known.iter().copied()).collect();
        for e in &sown {
            g.put(w, p, (e.genes, e.root), START_KG, lived.0[0].around.t);
        }
        run(&mut g, w, p, lived, GROW_YEARS);
        self.entrants = sown;
        self.garden = Some(g);
    }

    /// `rounds` rounds of the invasion on `lived` from round `first`: in each a newcomer arrives as `ARRIVE_KG` of seeds
    /// into the stand (as seeds lodge, `cover::lodge`) and the stand runs `INVADE_YEARS`; it held if it still holds a
    /// slot. The newcomers in turn: a mutant of a kind of the stand (`MUTANT_GENERATIONS` generations of
    /// `cover::mutate`, its starting kind kept, so it joins its kind's slot or founds its own as the law's seeds do); a
    /// guest (its own race's runners-up, then `guests`: the other classes' heaviest kinds, the nearest climate first, in
    /// turn); a draw.
    pub fn invade(&mut self, w: &World, p: &Parts, lived: Lived, guests: &[Entrant], (first, rounds): (u32, u32)) {
        let Some(mut g) = self.garden.take() else { return };
        let guests: Vec<Entrant> = self.race.iter().skip(1).map(|e| e.0).chain(guests.iter().copied()).collect();
        for r in first..first + rounds {
            let stand = g.stand(w, p);
            let e = match r % 3 {
                0 if !stand.is_empty() => {
                    let (root, genes, _) = stand[(r / 3) as usize % stand.len()];
                    self.mutant(w, &Entrant { genes, root, raced: false }, (1 << 20) + r, self.raced(root))
                }
                1 if !guests.is_empty() => Entrant { raced: false, ..guests[(r / 3) as usize % guests.len()] },
                _ => self.drawn(w, false),
            };
            let new = !self.entrants.iter().any(|x| x.root == e.root);
            g.put(w, p, (e.genes, e.root), ARRIVE_KG, lived.0[0].around.t);
            run(&mut g, w, p, lived, INVADE_YEARS);
            if new {
                self.came += 1;
                if g.stand(w, p).iter().any(|s| s.0 == e.root) {
                    self.held += 1;
                    self.entrants.push(e);
                }
            }
        }
        self.garden = Some(g);
    }

    /// Whether the kind of starting kind `root` came by the race.
    fn raced(&self, root: u32) -> bool {
        self.entrants.iter().find(|e| e.root == root).is_none_or(|e| e.raced)
    }

    /// The kinds of its stand now, heaviest first: each with its genes and cover per m², kg.
    pub fn set(&self, w: &World, p: &Parts) -> Vec<(Entrant, f64)> {
        let stand = self.garden.as_ref().map_or(Vec::new(), |g| g.stand(w, p));
        stand.into_iter().map(|(root, genes, kg)| (Entrant { genes, root, raced: self.raced(root) }, kg)).collect()
    }
}

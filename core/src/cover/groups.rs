//! The plants of a slot by size (laws.md 3): a slot keeps one pool of matter, its genes and its starting kind, and its
//! plants in up to `GROUPS` size groups, each a real count and a share of the slot's built cover, so seedlings that
//! join a stand leave its large plants as large as they were. A group's plant is its share of the built cover over its
//! count. Seeds that take hold come in as a group of their own, or, where every group is held, join the group of the
//! smallest plants: nothing happens to a plant because smaller plants arrived. Where slots join and their groups are
//! more than `GROUPS`, the two groups of smaller plants nearest in mass fuse, as cohorts fuse in the ecosystem
//! demography model (Moorcroft, Hurtt and Pacala 2001). The shares hold nothing: the books are the slot's.

use super::{stand, Cover, GROUPS, NO_ROOT, NP, SLOTS};
use crate::Physics;

/// A group as the bookkeeping moves it: its plants, a real count, and its built cover, kg.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Group {
    pub(super) n: f64,
    pub(super) kg: f64,
}

impl Group {
    /// Its plant's mass, kg.
    pub(super) fn plant(&self) -> f64 {
        self.kg / self.n.max(f64::MIN_POSITIVE)
    }
}

/// The plants of a slot by group as the laws read them (`Cover::sizes`): its held groups, the smallest plants first;
/// of each its plants (a real count), its plant's mass, kg, the share of that mass its walls make, and the group's share
/// of the slot's capture area.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sizes {
    pub held: usize,
    pub count: [f64; GROUPS],
    pub mass: [f64; GROUPS],
    pub walls: [f64; GROUPS],
    pub leaf: [f64; GROUPS],
}

impl Sizes {
    /// The group of its large plants: its last held one (0 when none is held).
    pub fn large(&self) -> usize {
        self.held.saturating_sub(1)
    }
    /// The height of a plant of group `g`, m (`stand::height`), its walls of density `rho` kg/m³.
    pub fn height(&self, ph: &Physics, rho: f64, g: usize) -> f64 {
        stand::height(ph, self.mass[g], self.walls[g], rho)
    }
    /// The width of the main stem of a plant of group `g`, m (`stand::stem`), its walls of density `rho` kg/m³.
    pub fn stem(&self, ph: &Physics, rho: f64, g: usize) -> f64 {
        stand::stem(ph, self.mass[g], self.walls[g], rho)
    }
    /// The height its plants lift their liquid to, m: each group's height by its share of the capture area (the
    /// conducting walls of each plant are as tall as it, `stand::sapwood`), its walls of density `rho` kg/m³.
    pub fn lift(&self, ph: &Physics, rho: f64) -> f64 {
        (0..self.held).map(|g| self.leaf[g] * self.height(ph, rho, g)).sum()
    }
}

impl Cover {
    /// The plants of slot `x`: all its groups' (a real count).
    pub fn count(&self, x: usize) -> f64 {
        self.plants[x].iter().sum()
    }
    /// The plants of slot `x` by group (`Sizes`). Its walls are shared out over its groups by their built cover times the
    /// share of the walls a plant of their mass builds (`stand::walled`: a seedling is mostly leaf), each at most all of
    /// its plant, what a full group cannot hold going to the others (`share_walls`); the rest of each group's built cover
    /// is its living cover, whose share is the group's capture area.
    pub fn sizes(&self, ph: &Physics, mm: &[f64; NP], x: usize) -> Sizes {
        let (built, walls) = (self.built(mm, x), self.walls(mm, x));
        let mut s = Sizes { held: self.held_groups(x), ..Sizes::default() };
        let kg = self.share[x].map(|f| f * built);
        (0..s.held).for_each(|g| (s.count[g], s.mass[g]) = (self.plants[x][g], kg[g] / self.plants[x][g]));
        // (One group holds all: its walls are the slot's.)
        if s.held == 1 {
            (s.walls[0], s.leaf[0]) = (walls / built.max(1e-300), 1.0);
            return s;
        }
        let by: [f64; GROUPS] = std::array::from_fn(|g| if g < s.held { stand::walled(ph, s.mass[g]) } else { 0.0 });
        s.walls = share_walls(&kg[..s.held], &by, walls);
        let living: f64 = (0..s.held).map(|g| kg[g] * (1.0 - s.walls[g])).sum();
        (0..s.held).for_each(|g| s.leaf[g] = kg[g] * (1.0 - s.walls[g]) / living.max(1e-300));
        s
    }
    /// The height of the large plants of slot `x` (`Sizes::large`), m: the top of its crown, its walls of density
    /// `rho` kg/m³.
    pub fn height(&self, ph: &Physics, mm: &[f64; NP], rho: f64, x: usize) -> f64 {
        let s = self.sizes(ph, mm, x);
        s.height(ph, rho, s.large())
    }
    /// The width of the main stem of the large plants of slot `x`, m, its walls of density `rho` kg/m³.
    pub fn width(&self, ph: &Physics, mm: &[f64; NP], rho: f64, x: usize) -> f64 {
        let s = self.sizes(ph, mm, x);
        s.stem(ph, rho, s.large())
    }
    /// The height the plants of slot `x` lift their liquid to, m (`Sizes::lift`), its walls of density `rho` kg/m³.
    pub fn lift(&self, ph: &Physics, mm: &[f64; NP], rho: f64, x: usize) -> f64 {
        self.sizes(ph, mm, x).lift(ph, rho)
    }
    /// The share of the walls its `WALLS` gene sets that the plants of slot `x` hold: each group's (`stand::walled` of
    /// its plant) by its share of the built cover.
    pub(super) fn walled_held(&self, ph: &Physics, mm: &[f64; NP], x: usize) -> f64 {
        let s = self.sizes(ph, mm, x);
        (0..s.held).map(|g| self.share[x][g] * stand::walled(ph, s.mass[g])).sum()
    }
    /// The share of the walls its `WALLS` gene sets that what slot `x` builds holds: each group's (`stand::walled` of its
    /// plant) by its share `growth` of what the slot builds (`Cover::growth`).
    pub(super) fn walled_new(&self, ph: &Physics, mm: &[f64; NP], x: usize, growth: &[f64; GROUPS]) -> f64 {
        let s = self.sizes(ph, mm, x);
        (0..s.held).map(|g| growth[g] * stand::walled(ph, s.mass[g])).sum()
    }
    /// The share of what each slot of cover `c` in `order` (on its ground of `area` m², its walls of density `rho`)
    /// builds in a month that goes to each of its groups: each group's capture area times the light its leaves get under
    /// the leaves of all the cell's plants taller than they are, whatever slot holds them: the light falls by e^-(the
    /// capture area per unit of ground it passed, each slot's over the share its plants hold, `Cover::on`) (Beer's law, as
    /// `canopy::shade`), and the leaves of plants as tall (to `SAME`) get the mean over theirs together
    /// (`math::mean_decay`). So the large plants take the most light a leaf, the asymmetry of the competition for light
    /// by size (Weiner 1990), and seedlings, all leaf, grow fast where light reaches them. A slot with no leaves gets
    /// none (`Cover::grew`).
    pub(super) fn growth(&self, ph: &Physics, (mm, rho): (&[f64; NP], f64), area: f64, order: &[usize]) -> [[f64; GROUPS]; SLOTS] {
        let (mut all, mut k, mut out) = ([(0, 0, 0.0, 0.0); SLOTS * GROUPS], 0, [[0.0; GROUPS]; SLOTS]);
        // (Slots of one group each take all they build.)
        if order.iter().all(|&x| self.held_groups(x) <= 1) {
            order.iter().for_each(|&x| out[x][0] = 1.0);
            return out;
        }
        for &x in order {
            let s = self.sizes(ph, mm, x);
            let per = self.leaf(mm, x) / (area * self.on(x)).max(1e-300);
            for g in 0..s.held {
                (all[k], k) = ((x, g, s.height(ph, rho, g), per * s.leaf[g]), k + 1);
            }
        }
        let all = &mut all[..k];
        all.sort_by(|a, b| b.2.total_cmp(&a.2));
        let (mut above, mut i) = (0.0f64, 0);
        while i < k {
            let j = i + all[i..].iter().take_while(|e| e.2 >= all[i].2 * (1.0 - SAME)).count();
            let y: f64 = all[i..j].iter().map(|e| e.3).sum();
            let lit = (-above).exp() * crate::math::mean_decay(y);
            all[i..j].iter().for_each(|&(x, g, _, leaf)| out[x][g] = leaf * lit);
            (above, i) = (above + y, j);
        }
        for &x in order {
            let sum: f64 = out[x].iter().sum();
            if sum > 0.0 {
                out[x].iter_mut().for_each(|f| *f /= sum);
            }
        }
        out
    }
    /// Its held groups of slot `x`: those that hold plants, the first ones.
    fn held_groups(&self, x: usize) -> usize {
        self.plants[x].iter().take_while(|&&n| n > 0.0).count()
    }
    /// The groups of slot `x` as the bookkeeping moves them (`Group`), and how many are held.
    pub(super) fn groups(&self, mm: &[f64; NP], x: usize) -> ([Group; GROUPS], usize) {
        let (built, mut g) = (self.built(mm, x), [Group::default(); GROUPS]);
        let held = self.held_groups(x);
        (0..held).for_each(|k| g[k] = Group { n: self.plants[x][k], kg: self.share[x][k] * built });
        (g, held)
    }
    /// The plants of slot `x` become `groups` (`fit`): their counts, and their shares by their built cover (by their
    /// counts where they hold none: plants of no mass are alike).
    pub(super) fn set_groups(&mut self, x: usize, groups: &mut [Group]) {
        let held = fit(groups);
        let all: f64 = groups[..held].iter().map(|g| g.kg).sum();
        let by = |g: &Group| if all > 0.0 { g.kg / all } else { g.n / groups[..held].iter().map(|g| g.n).sum::<f64>() };
        (self.plants[x], self.share[x]) = ([0.0; GROUPS], [0.0; GROUPS]);
        for (k, g) in groups[..held].iter().enumerate() {
            (self.plants[x][k], self.share[x][k]) = (g.n, by(g));
        }
    }
    /// Slot `x` is founded by `n` plants: one group, all its built cover.
    pub(super) fn found(&mut self, x: usize, n: f64) {
        (self.plants[x], self.share[x]) = ([0.0; GROUPS], [0.0; GROUPS]);
        (self.plants[x][0], self.share[x][0]) = (n, 1.0);
    }
    /// Slot `x` empties: no amounts, genes of no kind, no plants.
    pub(super) fn clear(&mut self, x: usize) {
        (self.mol[x], self.root[x], self.full[x], self.net[x], self.need[x]) = ([0.0; NP], NO_ROOT, false, 0.0, 0.0);
        (self.line[x], self.stands[x], self.airless[x], self.drained[x]) = (0.0, 0.0, 0.0, 0.0);
        (self.plants[x], self.share[x]) = ([0.0; GROUPS], [0.0; GROUPS]);
    }
    /// `n` plants of `kg` of built cover take hold in slot `x` (seeds of its kind, before their amounts join it): a
    /// group of their own, or, where every group is held, the group of its smallest plants.
    pub(super) fn recruit(&mut self, mm: &[f64; NP], x: usize, (n, kg): (f64, f64)) {
        let (mut g, held) = self.groups(mm, x);
        if held < GROUPS {
            g[held] = Group { n, kg };
            self.set_groups(x, &mut g[..held + 1]);
            return;
        }
        g[0] = Group { n: g[0].n + n, kg: g[0].kg + kg };
        self.set_groups(x, &mut g[..held]);
    }
    /// Slot `small` joins slot `big` (`stand::join`, before their amounts join): their groups together.
    pub(super) fn join_groups(&mut self, mm: &[f64; NP], big: usize, small: usize) {
        let ((a, na), (b, nb)) = (self.groups(mm, big), self.groups(mm, small));
        let mut all = [Group::default(); 2 * GROUPS];
        all[..na].copy_from_slice(&a[..na]);
        all[na..na + nb].copy_from_slice(&b[..nb]);
        self.set_groups(big, &mut all[..na + nb]);
    }
    /// The plants of slot `x` no taller than `height` m (its walls of density `rho` kg/m³) die above the ground on the
    /// share `f` of its ground, as a flame's plume kills them (`flame::burn`). A group of plants no taller than
    /// `surface_m` keeps its part below the ground, the share `base_share` of its built cover, and its plants: they grow
    /// again from it and their store, as herbs and shrubs burnt to the ground sprout from their base (Bond and Midgley
    /// 2001). A group of taller plants loses that share of its plants and of its built cover. Returns what they lost, kg:
    /// of the plants that sprout and of those that died, their body matter and their walls; and how many died. The caller
    /// takes those amounts from the slot.
    pub fn die_under(&mut self, ph: &Physics, (mm, rho): (&[f64; NP], f64), x: usize, (f, height): (f64, f64)) -> ([[f64; 2]; 2], f64) {
        let s = self.sizes(ph, mm, x);
        let (mut g, held) = self.groups(mm, x);
        let (mut kg, mut died) = ([[0.0; 2]; 2], 0.0);
        for k in (0..held).filter(|&k| s.height(ph, rho, k) <= height) {
            let sprouts = s.height(ph, rho, k) <= ph.surface_m;
            let dead = g[k].kg * f * if sprouts { 1.0 - ph.base_share } else { 1.0 };
            let lost = &mut kg[(!sprouts) as usize];
            (lost[0], lost[1]) = (lost[0] + dead * (1.0 - s.walls[k]), lost[1] + dead * s.walls[k]);
            let gone = if sprouts { 0.0 } else { g[k].n * f };
            (g[k], died) = (Group { n: g[k].n - gone, kg: g[k].kg - dead }, died + gone);
        }
        if kg.iter().flatten().any(|&m| m > 0.0) {
            self.set_groups(x, &mut g[..held]);
        }
        (kg, died)
    }
    /// Slot `x`, whose built cover went from `made` to `now` kg in its growth, shares what it built over its groups by
    /// `growth` (`Cover::growth`; by their shares where that is none, a slot with no leaves); what it lost goes from each
    /// group by its share. Its counts stay.
    pub(super) fn grew(&mut self, x: usize, (made, now): (f64, f64), growth: &[f64; GROUPS]) {
        let held = self.held_groups(x);
        let mut g = [Group::default(); GROUPS];
        let gain = (now - made).max(0.0);
        (0..held).for_each(|k| g[k] = Group { n: self.plants[x][k], kg: self.share[x][k] * made + growth[k] * gain });
        self.set_groups(x, &mut g[..held]);
    }
}

/// Plant masses and heights nearer than this share of each other are one: the rounding of sums taken in another order
/// (a stand cut into alike slots holds plants as large to it).
const SAME: f64 = 1e-9;

/// Groups `g` made a slot's: the empty ones (no plants) dropped, the rest in the order of their plants' mass; two that
/// hold plants of one mass (to `SAME`) are one group, and while they are more than `GROUPS`, the two of the smaller
/// plants (all but the largest) nearest in mass (the least ratio) fuse. Returns how many.
pub(super) fn fit(g: &mut [Group]) -> usize {
    let mut held = 0;
    for k in 0..g.len() {
        if g[k].n > 0.0 {
            (g[held], held) = (g[k], held + 1);
        }
    }
    g[..held].sort_by(|a, b| a.plant().total_cmp(&b.plant()));
    while held > 1 {
        let ratio = |k: usize| g[k + 1].plant() / g[k].plant().max(f64::MIN_POSITIVE);
        // (Plants of one mass are one group; room is made among the smaller plants: the large plants fuse only with
        // plants of their mass.)
        let k = match (0..held - 1).find(|&k| ratio(k) <= 1.0 + SAME) {
            Some(k) => k,
            None if held > GROUPS => (0..held - 2).min_by(|&a, &b| ratio(a).total_cmp(&ratio(b))).expect("two groups"),
            None => break,
        };
        g[k] = Group { n: g[k].n + g[k + 1].n, kg: g[k].kg + g[k + 1].kg };
        g.copy_within(k + 2..held, k + 1);
        held -= 1;
    }
    held
}

/// The share of walls of each of the groups of built cover `kg`, kg, that `walls` kg of walls make, shared out by each
/// group's built cover times its weight `by`, each group at most all walls: the walls a full group cannot hold go to
/// the others by the same weights, until none is over, so the groups hold all the walls (no more than their built
/// cover holds: the slot's walls are part of it).
fn share_walls(kg: &[f64], by: &[f64; GROUPS], walls: f64) -> [f64; GROUPS] {
    let (mut full, mut left) = ([false; GROUPS], walls);
    loop {
        let walled: f64 = (0..kg.len()).filter(|&g| !full[g]).map(|g| kg[g] * by[g]).sum();
        let (was, open, mut filled) = (left, full.map(|f| !f), false);
        for g in (0..kg.len()).filter(|&g| open[g] && was * by[g] / walled.max(1e-300) > 1.0) {
            (full[g], left, filled) = (true, left - kg[g], true);
        }
        if !filled {
            debug_assert!(full[..kg.len()].contains(&false) || left <= WALLS_LEFT * walls, "{left} kg of walls left over full groups {kg:?}");
            return std::array::from_fn(|g| if g >= kg.len() { 0.0 } else if full[g] { 1.0 } else { left * by[g] / walled.max(1e-300) });
        }
    }
}

/// Walls left when every group is full, as a share of the slot's: no more than the rounding of the sums.
const WALLS_LEFT: f64 = 1e-9;

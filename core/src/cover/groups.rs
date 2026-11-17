//! The plants of a slot by size (laws.md 3): a slot keeps one pool of matter, its genes and its starting kind, and its
//! plants in up to `GROUPS` size groups, each a real count and a share of the slot's built cover, so seedlings that
//! join a stand leave its large plants as large as they were. A group's plant is its share of the built cover over its
//! count. Seeds that take hold come in as a group of their own; when every group is held, the two groups whose plants
//! are nearest in mass fuse, as cohorts fuse in the ecosystem demography model (Moorcroft, Hurtt and Pacala 2001) and
//! as the slots of a cell join (`seed::room`). The shares hold nothing: the books are the slot's.

use super::{stand, Cover, GROUPS, NP, NO_ROOT};
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
    /// its plant; the rest of each group's built cover is its living cover, whose share is the group's capture area.
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
        let walled: f64 = (0..s.held).map(|g| kg[g] * stand::walled(ph, s.mass[g])).sum();
        (0..s.held).for_each(|g| s.walls[g] = (walls * stand::walled(ph, s.mass[g]) / walled.max(1e-300)).min(1.0));
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
    /// The share of what slot `x` builds in a month that goes to each of its groups: each group's capture area times the
    /// light its leaves get, its leaves under those of its larger plants: through the slot's layer the light falls by
    /// e^-(`layer` × the share of the capture area it passed) (Beer's law, `canopy::shade`), and a group's leaves get the
    /// mean over their own share (`math::mean_decay`). So the large plants take the most light a leaf, the asymmetry of
    /// the competition for light by size (Weiner 1990), and seedlings, all leaf, grow fast where light reaches them.
    pub(super) fn growth(&self, ph: &Physics, mm: &[f64; NP], x: usize, layer: f64) -> [f64; GROUPS] {
        let (mut out, mut above) = ([0.0; GROUPS], 0.0);
        // (One group takes all.)
        if self.held_groups(x) == 1 {
            out[0] = 1.0;
            return out;
        }
        let s = self.sizes(ph, mm, x);
        for g in (0..s.held).rev() {
            out[g] = s.leaf[g] * (-layer * above).exp() * crate::math::mean_decay(layer * s.leaf[g]);
            above += s.leaf[g];
        }
        let all: f64 = out.iter().sum();
        if all > 0.0 {
            out.iter_mut().for_each(|f| *f /= all);
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
        (self.plants[x], self.share[x]) = ([0.0; GROUPS], [0.0; GROUPS]);
    }
    /// `n` plants of `kg` of built cover take hold in slot `x` (seeds of its kind, before their amounts join it): a
    /// group of their own.
    pub(super) fn recruit(&mut self, mm: &[f64; NP], x: usize, (n, kg): (f64, f64)) {
        let (g, held) = self.groups(mm, x);
        let mut all = [Group::default(); GROUPS + 1];
        all[..held].copy_from_slice(&g[..held]);
        all[held] = Group { n, kg };
        self.set_groups(x, &mut all[..held + 1]);
    }
    /// Slot `small` joins slot `big` (`stand::join`, before their amounts join): their groups together.
    pub(super) fn join_groups(&mut self, mm: &[f64; NP], big: usize, small: usize) {
        let ((a, na), (b, nb)) = (self.groups(mm, big), self.groups(mm, small));
        let mut all = [Group::default(); 2 * GROUPS];
        all[..na].copy_from_slice(&a[..na]);
        all[na..na + nb].copy_from_slice(&b[..nb]);
        self.set_groups(big, &mut all[..na + nb]);
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

/// Groups `g` made a slot's: the empty ones (no plants) dropped, the rest in the order of their plants' mass, and while
/// they are more than `GROUPS`, or two hold plants of one mass (one group), the two whose plants are nearest in mass
/// (the least ratio) fuse. Returns how many.
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
        let k = (0..held - 1).min_by(|&a, &b| ratio(a).total_cmp(&ratio(b))).expect("two groups");
        if held <= GROUPS && ratio(k) > 1.0 {
            break;
        }
        g[k] = Group { n: g[k].n + g[k + 1].n, kg: g[k].kg + g[k + 1].kg };
        g.copy_within(k + 2..held, k + 1);
        held -= 1;
    }
    held
}

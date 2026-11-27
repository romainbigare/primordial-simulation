//! The ground edits shape (laws.md, Edits and their settling): it slides until it stands no steeper than loose ground
//! rests at (`World::slid`), and when the edits settle it is laid as the start's wear would have worn it
//! (`World::worn`).

use super::drain::{flood, starts};
use super::grid::Grid;
use super::mantle::REST;
use super::samples::grid_of;
use super::slopes::creep;
use super::strata::Strata;
use super::wear::{cut, CREEP, CUT, STAND_M};
use crate::World;

/// The steps of the wear of what edits shaped (`World::worn`): each finds the ways down again over the ground it left,
/// and the stream power law is solved exactly along them (`wear::cut`), so a few steps take the ways as the ground
/// moves.
const WORN_STEPS: usize = 8;

/// The most passes of a slide (`World::slid`): each brings every link of each sample that gives ground that stands
/// steeper than it can to the slope it comes to rest at, highest first; ground moves at most this many samples from
/// those the edits touched. The slide ends sooner, once no link fails.
pub(crate) const SLIDE_PASSES: usize = 32;
/// Ground that slid comes to rest this much gentler than the slope at which it fails (`REST`), degrees: a pile of loose
/// solid pieces fails at its most stable angle and comes to rest a couple of degrees under it (Jaeger, Liu and Nagel
/// 1989, from memory). So a link a slide left can steepen this much as the slide goes on before it fails again, and the
/// slide ends.
const SLID_UNDER_DEG: f64 = 2.0;

/// A box of the samples of a grid: its west column and north row, and its width and height.
#[derive(Clone, Copy)]
pub(super) struct Window {
    pub(super) x0: usize,
    pub(super) y0: usize,
    pub(super) w: usize,
    pub(super) h: usize,
}

impl Window {
    /// The box of the samples `ks` of grid `g` and `margin` samples about them, on the grid.
    pub(super) fn about(g: &Grid, ks: &[usize], margin: usize) -> Window {
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for &k in ks {
            let (x, y) = (k % g.across, k / g.across);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        if ks.is_empty() {
            return Window { x0: 0, y0: 0, w: 0, h: 0 };
        }
        let (x0, y0) = (x0.saturating_sub(margin), y0.saturating_sub(margin));
        let (x1, y1) = ((x1 + margin).min(g.across - 1), (y1 + margin).min(g.down - 1));
        Window { x0, y0, w: x1 + 1 - x0, h: y1 + 1 - y0 }
    }
    /// Its samples.
    pub(super) fn n(&self) -> usize {
        self.w * self.h
    }
    /// The sample of the grid `across` wide at its place `i`, row by row.
    pub(super) fn global(&self, i: usize, across: usize) -> usize {
        (self.y0 + i / self.w) * across + self.x0 + i % self.w
    }
    /// The place in it of sample `k` of the grid `across` wide, if it lies in it.
    pub(super) fn local(&self, k: usize, across: usize) -> Option<usize> {
        let (x, y) = (k % across, k / across);
        (x >= self.x0 && y >= self.y0 && x < self.x0 + self.w && y < self.y0 + self.h).then(|| (y - self.y0) * self.w + x - self.x0)
    }
}

/// Ground slides on grid `g` over the box `win` of its samples, of heights `h` there (m, row by row; moved in place),
/// that stood at `base` before any edit, from the samples that give it (`gives`: the volume each may give, m³; and what
/// each took in the passes before), in passes (`SLIDE_PASSES`): each sample that gives, highest first, sends ground to
/// each neighbour across a side or a corner (so its rest is near round, not of the grid's square) whose link it stands
/// over steeper than loose ground rests at (`REST`), or than the link stood before any edit where that is steeper
/// (steep ground no edit made keeps its slope), as much as brings that link to the slope it comes to rest at
/// (`SLID_UNDER_DEG` gentler; or as it stood before any edit, where steeper), the volume kept by the two samples'
/// areas. So ground moves no more than a sample a pass from those that give it. Neighbours off the box take none.
pub(super) fn slide(g: &Grid, win: Window, (h, base): (&mut [f64], &[f64]), gives: &[f64]) {
    let after = (REST.atan() - SLID_UNDER_DEG.to_radians()).tan();
    let mut has = gives.to_vec();
    let mut took = vec![0.0; win.n()];
    for _ in 0..SLIDE_PASSES {
        let mut slid = false;
        let mut order: Vec<usize> = (0..win.n()).filter(|&i| has[i] > 0.0).collect();
        order.sort_by(|&a, &b| h[b].total_cmp(&h[a]).then(a.cmp(&b)));
        for i in order {
            let k = win.global(i, g.across);
            for (j, len) in g.around(k) {
                let Some(l) = win.local(j, g.across) else { continue };
                let was = (base[i] - base[l]) / len;
                if h[i] - h[l] <= REST.max(was) * len || has[i] <= 0.0 {
                    continue;
                }
                let v = ((h[i] - h[l] - after.max(was) * len) / (1.0 / g.area(k) + 1.0 / g.area(j))).min(has[i]);
                h[i] -= v / g.area(k);
                h[l] += v / g.area(j);
                (has[i], took[l]) = (has[i] - v, took[l] + v);
                slid = true;
            }
        }
        if !slid {
            break;
        }
        has.iter_mut().zip(&mut took).for_each(|(a, t)| *a += std::mem::take(t));
    }
}

impl World {
    /// Where the ground slides after edits shaped the samples `givers` (`slide`): the ground as it stands
    /// (`Samples::height_now`), before any edit as it stands less what edits shaped (`Relief::shaped`), over the box of
    /// the givers and `SLIDE_PASSES` samples about them. Returns each sample that moved and by how much, m, in their
    /// order.
    pub(crate) fn slid(&self, givers: &[usize]) -> Vec<(usize, f64)> {
        let (g, _, _) = grid_of(&self.map, (self.cfg.lat_south, self.cfg.lat_north));
        let (s, r) = (self.terrain_samples(), &self.relief);
        let win = Window::about(&g, givers, SLIDE_PASSES);
        let was: Vec<f64> = (0..win.n()).map(|i| s.height_now(r, win.global(i, g.across))).collect();
        let base: Vec<f64> = (0..win.n()).map(|i| was[i] - r.shaped_at(win.global(i, g.across))).collect();
        let mut gives = vec![0.0; win.n()];
        givers.iter().filter_map(|&k| win.local(k, g.across)).for_each(|i| gives[i] = f64::INFINITY);
        let mut h = was.clone();
        slide(&g, win, (&mut h, &base), &gives);
        (0..win.n()).filter(|&i| h[i] != was[i]).map(|i| (win.global(i, g.across), h[i] - was[i])).collect()
    }

    /// The ground edits shaped since the samples were last built (`Samples::shaped_since`) as the start's wear would
    /// leave it worn for `Config::edit_wear_years` (laws.md, Edits): over the box of those samples and
    /// `SLIDE_PASSES` about them, the ground as it stands, each such sample first given the terrain's own roughness
    /// (`terrain::swells`) in proportion to how far the edits moved it, so the wear has something to cut; then
    /// `WORN_STEPS` steps of the start's law (`space::wear`): the flood from the box's edge, the stream power law with
    /// each bed's make's K (`wear::cut`, `Strata::cut`) and the creep (`slopes::creep`), on those samples alone and
    /// none at or under the liquid's stand (`wear::STAND_M`), the ground about them as it stands; no sample lower than
    /// it stood before the edits shaped it since (a pit is not cut deeper), and last the slide (`slide`). Returns each
    /// sample that moved and by how much, m, in their order.
    pub(crate) fn worn(&self) -> Vec<(usize, f64)> {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        let (g, _, leaf) = grid_of(&self.map, lat);
        let (s, r, t) = (self.terrain_samples(), &self.relief, &self.terrain);
        let shaped: Vec<usize> = (0..s.h.len()).filter(|&k| s.shaped_since(r, k) != 0.0).collect();
        if shaped.is_empty() || self.cfg.edit_wear_years <= 0.0 {
            return Vec::new();
        }
        let win = Window::about(&g, &shaped, SLIDE_PASSES);
        let at = |i: usize| win.global(i, g.across);
        let local = Grid::new(win.w, win.h, false, g.dy, g.dx[win.y0..win.y0 + win.h].to_vec());
        let moved: Vec<f64> = (0..win.n()).map(|i| s.shaped_since(r, at(i))).collect();
        let was: Vec<f64> = (0..win.n()).map(|i| s.height_now(r, at(i))).collect();
        let swells = |i: usize| super::terrain::swells(t.seed, &self.map, self.map.place_of(&leaf(at(i)), lat.0, lat.1).1);
        let mut h: Vec<f64> = (0..win.n()).map(|i| if moved[i] != 0.0 { was[i] + moved[i] * swells(i) } else { was[i] }).collect();
        // The box's edge drains it; what no edit moved, or lies under the liquid, holds.
        let edge: Vec<bool> = (0..win.n()).map(|i| i % win.w == 0 || i % win.w == win.w - 1 || i < win.w || i >= win.n() - win.w).collect();
        let held: Vec<bool> = (0..win.n()).map(|i| edge[i] || moved[i] == 0.0 || was[i] <= STAND_M).collect();
        let (strata, draws) = (Strata::of(t.seed, &t.makes), t.draws(t.seed, g.n()));
        let k_cut: Vec<f64> = (0..win.n()).map(|i| CUT * strata.cut(s.make[at(i)])).collect();
        let draw: Vec<f64> = (0..win.n()).map(|i| draws[at(i)]).collect();
        let (start, none, dt) = (starts(&local, &edge, false), vec![0.0; win.n()], self.cfg.edit_wear_years / WORN_STEPS as f64);
        let mut offshore = vec![0.0; win.n()];
        for _ in 0..WORN_STEPS {
            let f = flood(&local, &h, &edge, (&start, &draw));
            cut(&local, &mut h, &f, (&none, &held, &k_cut), dt);
            creep(&local, &mut h, (&held, f64::NEG_INFINITY), CREEP * dt, &mut offshore);
        }
        // (No lower than it stood before the edits shaped it since, nor by the slide.)
        let floor: Vec<f64> = (0..win.n()).map(|i| was[i] - moved[i].max(0.0)).collect();
        (0..win.n()).for_each(|i| h[i] = h[i].max(floor[i]));
        let base: Vec<f64> = (0..win.n()).map(|i| was[i] - r.shaped_at(at(i))).collect();
        let gives: Vec<f64> = (0..win.n()).map(|i| if moved[i] != 0.0 { (h[i] - floor[i]) * g.area(at(i)) } else { 0.0 }).collect();
        slide(&g, win, (&mut h, &base), &gives);
        (0..win.n()).filter(|&i| h[i] != was[i]).map(|i| (at(i), h[i] - was[i])).collect()
    }
}

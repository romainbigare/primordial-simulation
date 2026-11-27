//! The ground edits shape (laws.md, Edits and their settling): it slides until it stands no steeper than loose ground
//! rests at (`World::slid`), and when the edits settle it is laid as the start's wear would have worn it
//! (`World::worn`).

use super::drain::{flood, starts};
use super::grid::Grid;
use super::samples::grid_of;
use super::slide::{fails, slide, spread, SLIDE_PASSES};
use super::slopes::creep;
use super::strata::Strata;
use super::wear::{cut, CREEP, CUT, STAND_M};
use crate::World;

/// The steps of the wear of what edits shaped (`World::worn`): each finds the ways down again over the ground it left,
/// and the stream power law is solved exactly along them (`wear::cut`), so a few steps take the ways as the ground
/// moves.
const WORN_STEPS: usize = 8;

/// The samples about those that spread that the spread and the look for a slide read (`World::slid`): the sample
/// beside one that spreads may take its ground, and the one beside that tells its slope.
const SPREAD_MARGIN: usize = 2;

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
    /// Its place, row by row, of the sample `k` of the grid `across` wide; none where it lies outside.
    fn local(&self, k: usize, across: usize) -> Option<usize> {
        let (x, y) = ((k % across).checked_sub(self.x0)?, (k / across).checked_sub(self.y0)?);
        (x < self.w && y < self.h).then_some(y * self.w + x)
    }
    /// The part of it inside the box `o`.
    fn within(&self, o: &Window) -> Window {
        let (x0, y0) = (self.x0.max(o.x0), self.y0.max(o.y0));
        let (x1, y1) = ((self.x0 + self.w).min(o.x0 + o.w), (self.y0 + self.h).min(o.y0 + o.h));
        Window { x0, y0, w: x1.saturating_sub(x0), h: y1.saturating_sub(y0) }
    }
    /// Its own grid, of the rows of grid `g` it spans (not round a globe).
    pub(super) fn grid(&self, g: &Grid) -> Grid {
        Grid::new(self.w, self.h, false, g.dy, g.dx[self.y0..self.y0 + self.h].to_vec())
    }
}

impl World {
    /// Where the ground comes to rest after edits shaped the samples `givers`: the ground as it stands
    /// (`Samples::height_now`), before any edit as it stands less what edits shaped (`Relief::shaped`). What edits moved
    /// first spreads by `spreads` (a sample and its m², summed where it comes more than once; `spread`), over the box
    /// of those samples and `SPREAD_MARGIN` about them. Where a sample then stands too steep (`fails`), the ground
    /// slides over the box of the givers and `SLIDE_PASSES` samples about them (`slide`, fresh: walls fail), each
    /// sample giving what edits laid on it; where none does, the ground about it rests as it stands and is not read.
    /// Returns each sample that moved and by how much, m, in their order.
    pub(crate) fn slid(&self, givers: &[usize], spreads: &[(usize, f64)]) -> Vec<(usize, f64)> {
        let (g, _, _) = grid_of(&self.map, (self.cfg.lat_south, self.cfg.lat_north));
        let (s, r) = (self.terrain_samples(), &self.relief);
        // The ground of a box as it stands, and before any edit.
        let stands = |win: &Window| -> [Vec<f64>; 2] {
            let was: Vec<f64> = (0..win.n()).map(|i| s.height_now(r, win.global(i, g.across))).collect();
            let base = (0..win.n()).map(|i| was[i] - r.shaped_at(win.global(i, g.across))).collect();
            [was, base]
        };
        let moved = |win: &Window, h: &[f64], was: &[f64]| (0..win.n()).filter(|&i| h[i] != was[i]).map(|i| (win.global(i, g.across), h[i] - was[i])).collect();
        let all = Window::about(&g, givers, SLIDE_PASSES);
        let near = Window::about(&g, &spreads.iter().map(|p| p.0).collect::<Vec<_>>(), SPREAD_MARGIN).within(&all);
        let (local, [was, base]) = (near.grid(&g), stands(&near));
        let mut kt = vec![0.0; near.n()];
        spreads.iter().filter_map(|&(k, m2)| Some((near.local(k, g.across)?, m2))).for_each(|(i, m2)| kt[i] += m2);
        let mut h = was.clone();
        spread(&local, (&mut h, &base), &kt);
        if !fails(&local, &h, &base) {
            return moved(&near, &h, &was);
        }
        let (local, [was, base]) = (all.grid(&g), stands(&all));
        let mut slid = was.clone();
        (0..near.n()).filter_map(|i| Some((all.local(near.global(i, g.across), g.across)?, h[i]))).for_each(|(i, at)| slid[i] = at);
        let gives: Vec<f64> = (0..all.n()).map(|i| (slid[i] - base[i]).max(0.0) * local.area(i)).collect();
        slide(&local, (&mut slid, &base), &gives, true);
        moved(&all, &slid, &was)
    }

    /// The ground edits shaped since the samples were last built (`Samples::shaped_since`) as the start's wear would
    /// leave it worn for `Config::edit_wear_years` (laws.md, Edits): over the box of those samples and
    /// `SLIDE_PASSES` about them, the ground as it stands, each such sample first given the terrain's own roughness
    /// (`terrain::swells`, about their mean over those samples) in proportion to how far the edits moved it, so the wear
    /// has something to cut; then
    /// `WORN_STEPS` steps of the start's law (`space::wear`): the flood from the box's edge, the stream power law with
    /// each bed's make's K (`wear::cut`, `Strata::cut`) and the creep (`slopes::creep`), on those samples alone and
    /// none at or under the liquid's stand (`wear::STAND_M`), the ground about them as it stands; no sample lower than
    /// it stood before the edits shaped it since (a pit is not cut deeper), and last the slide (`slide`, not fresh: each
    /// sample gives no more than stands over that, and nothing spreads, so the valleys stay). Returns each sample that moved and by how
    /// much, m, in their order.
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
        let local = win.grid(&g);
        let moved: Vec<f64> = (0..win.n()).map(|i| s.shaped_since(r, at(i))).collect();
        let was: Vec<f64> = (0..win.n()).map(|i| s.height_now(r, at(i))).collect();
        let swells: Vec<f64> = (0..win.n()).map(|i| if moved[i] != 0.0 { super::terrain::swells(t.seed, &self.map, self.map.place_of(&leaf(at(i)), lat.0, lat.1).1) } else { 0.0 }).collect();
        // (About their mean over what the edits moved, by how far: the roughness neither raises nor lowers the whole.)
        let weight: f64 = moved.iter().map(|m| m.abs()).sum();
        let mean = moved.iter().zip(&swells).map(|(m, w)| m.abs() * w).sum::<f64>() / weight;
        let mut h: Vec<f64> = (0..win.n()).map(|i| was[i] + moved[i] * (swells[i] - mean)).collect();
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
        slide(&local, (&mut h, &base), &gives, false);
        (0..win.n()).filter(|&i| h[i] != was[i]).map(|i| (at(i), h[i] - was[i])).collect()
    }
}

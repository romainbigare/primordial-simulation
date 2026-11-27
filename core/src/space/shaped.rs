//! The ground edits shape (laws.md, Edits and their settling): it slides until it stands no steeper than loose ground
//! rests at (`World::slid`).

use super::grid::Grid;
use super::mantle::REST;
use super::samples::grid_of;
use crate::World;

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
/// that stood at `base` before any edit, from the samples that give it (`gives`: any of their ground; else no more than
/// what each took in the passes before), in passes (`SLIDE_PASSES`): each sample that gives, highest first, sends ground
/// to each neighbour across a side or a corner (so its rest is near round, not of the grid's square) whose link it
/// stands over steeper than loose ground rests at (`REST`), or than the link stood before any edit where that is
/// steeper (steep ground no edit made keeps its slope), as much as brings that link to the slope it comes to rest at
/// (`SLID_UNDER_DEG` gentler; or as it stood before any edit, where steeper), the volume kept by the two samples'
/// areas. So ground moves no more than a sample a pass from those that give it. Neighbours off the box take none.
pub(super) fn slide(g: &Grid, win: Window, (h, base): (&mut [f64], &[f64]), gives: &[bool]) {
    let after = (REST.atan() - SLID_UNDER_DEG.to_radians()).tan();
    let mut has: Vec<f64> = gives.iter().map(|&b| if b { f64::INFINITY } else { 0.0 }).collect();
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
        let mut gives = vec![false; win.n()];
        givers.iter().filter_map(|&k| win.local(k, g.across)).for_each(|i| gives[i] = true);
        let mut h = was.clone();
        slide(&g, win, (&mut h, &base), &gives);
        (0..win.n()).filter(|&i| h[i] != was[i]).map(|i| (win.global(i, g.across), h[i] - was[i])).collect()
    }
}

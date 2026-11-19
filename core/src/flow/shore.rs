//! A cell's ground by height, under the levels its liquid stood at at the month's start (laws.md 3, `cover::footing`):
//! where a kind of cover holds its ground (the cell's ground above its line) and where on it its roots meet the liquid.
//! Heights are over the cell's lowest ground, m.

use super::curve::{curve_of, Curve, Part};
use crate::World;
use std::borrow::Cow;

/// Halvings that find a height for an area (`Shore::line_of`): of a cell's span of heights, to well under a mm.
const LINE_HALVINGS: usize = 48;

/// The ground of a cell (`curve::Curve`) under the levels of its parts (over its mean surface, m; a part not given one
/// holds no liquid).
pub struct Shore<'a> {
    c: Cow<'a, Curve>,
    levels: &'a [f64],
}

/// What a kind finds on the ground of a cell above its line (`Shore::feet`), m² (the work, m² of full work): the ground
/// above its line; of it the land, the ground under the liquid, the ground under the liquid where its roots get the
/// oxidising gas and the work they do there, and the ground under the liquid deeper than its roots get it; the line
/// over which no such ground lies, m; and the ground where its roots get the gas below its line.
#[derive(Clone, Copy, Debug, Default)]
pub struct Feet {
    pub ground: f64,
    pub land: f64,
    pub wet: f64,
    pub aired: f64,
    pub work: f64,
    pub airless: f64,
    pub rise: f64,
    pub free: f64,
}

/// The ground of cell `i` under the levels `levels` of its parts (`levels` of the month's start, `contract::levels`).
pub fn shore<'a>(w: &'a World, i: usize, levels: &'a [f64]) -> Shore<'a> {
    Shore { c: curve_of(w, &w.scratch, i), levels }
}

impl Shore<'_> {
    /// The cell's area, m².
    pub fn area(&self) -> f64 {
        self.c.area()
    }
    /// The level of part `p` over the mean surface: its lowest floor where it holds no liquid.
    fn level(&self, p: usize, part: &Part) -> f64 {
        self.levels.get(p).copied().unwrap_or(part.floors()[0].0)
    }
    /// The ground above `h`, m²: its samples in no part (no liquid stands on them) and each part's floors at `h` and over.
    pub fn above(&self, h: f64) -> f64 {
        let y = self.c.low + h;
        self.c.apart + self.c.parts.iter().map(|p| p.area() - p.under(y)).sum::<f64>()
    }
    /// The height of its highest floor.
    fn top(&self) -> f64 {
        self.c.parts.iter().map(|p| p.floors()[p.floors().len() - 1].0).fold(self.c.low, f64::max) - self.c.low
    }
    /// The lowest height whose ground above is no more than `area` m² (`above`), at least 0 and at most its highest
    /// floor's.
    pub fn line_of(&self, area: f64) -> f64 {
        let (mut lo, mut hi) = (0.0, self.top());
        for _ in 0..LINE_HALVINGS {
            let mid = 0.5 * (lo + hi);
            if self.above(mid) > area { lo = mid } else { hi = mid }
        }
        hi
    }
    /// The line of a kind whose roots get the oxidising gas under liquid as deep as `reach`, m, that holds all the
    /// ground it may (`cover::footing`): the highest of its parts' levels that hold liquid less `reach`, so no ground
    /// above it lies deeper; 0 (all the ground) where none holds liquid; and no higher than its highest floor, so seeds
    /// that woke hold ground even where all of it lies deeper by now (and live there on their store, `cover::upkeep`).
    pub fn found(&self, reach: f64) -> f64 {
        let wet = self.c.parts.iter().enumerate().map(|(q, p)| (self.level(q, p), p)).filter(|(x, p)| *x > p.floors()[0].0);
        (wet.map(|(x, _)| x - reach).fold(f64::NEG_INFINITY, f64::max) - self.c.low).clamp(0.0, self.top())
    }
    /// What a kind finds above its line `h` (`Feet`) whose roots get the oxidising gas under liquid as deep as `reach`,
    /// m, roots `roots` m deep: where liquid stands `d` deep over its ground they work over (`reach` − d) / `roots` of
    /// their depth, all from `reach` − `roots` up (`cover::working`).
    pub fn feet(&self, h: f64, reach: f64, roots: f64) -> Feet {
        let y = self.c.low + h;
        let mut f = Feet { ground: self.above(h), land: self.c.apart, rise: h, ..Feet::default() };
        for (q, p) in self.c.parts.iter().enumerate() {
            let x = self.level(q, p);
            let (a, s) = (|z: f64| p.under_x(z)[0], |z: f64| p.under_x(z)[1]);
            f.land += p.area() - a(x.max(y));
            f.wet += (a(x) - a(y)).max(0.0);
            let lo = y.max(x - reach);
            if lo < x {
                // Of its floors under the liquid where the roots get the gas: from `mid` up they work in full, under it
                // as far as the depth leaves them.
                let mid = (x - reach + roots).clamp(lo, x);
                let (part, sum) = (a(mid) - a(lo), s(mid) - s(lo));
                f.aired += a(x) - a(lo);
                f.work += a(x) - a(mid) + ((reach - x) * part + sum) / roots.max(1e-9);
            }
            if y < x - reach && a(x - reach) > a(y) {
                f.airless += a(x - reach) - a(y);
                f.rise = f.rise.max(x - reach - self.c.low);
            }
            if y > x - reach {
                f.free += (a(y) - a(x - reach)).max(0.0);
            }
        }
        f
    }
}

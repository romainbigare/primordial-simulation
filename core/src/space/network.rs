//! The terrain's channels (laws.md 0): on the drained samples (`space::samples`) the liquid of each sample runs to the
//! neighbour its ground falls to most steeply (D8, O'Callaghan and Mark 1984), and the samples being drained, every
//! sample of land has one. The area that drains through a sample is its own and all that runs to it. Between the
//! samples' centres the liquid runs along the straight way from one to the next, so what leaves any part of the map is
//! found at any size, finer than the samples too (`Samples::leaving`). It is a function of the seed and the map alone.

use super::samples::Samples;
use super::square::Square;
use super::terrain::SAMPLE_DEPTH;
use super::tree::BLOCK;
use super::Leaf;

/// No sample: where a sample's liquid runs when it runs to none on the map (a sink), and the sample upstream of one that
/// none runs to.
pub const NONE: u32 = u32::MAX;
/// A piece of a way in a leaf shorter than this share of the way only touches the leaf (a corner): none of it is there.
const TOUCH: f64 = 1e-12;
/// The ends of two pieces of one way this close, as a share of the way, are one.
const SAME: f64 = 1e-9;

/// The way each sample's liquid runs (`NONE` for a sink), the area that drains through it, m², and its neighbour
/// upstream that the most drains through (`NONE` where none runs to it), for samples of heights `h`, m, and areas
/// `area`, m², drained, each with its neighbours and the distance to each, m (`around`): to the neighbour of the
/// steepest fall, the first of equal ones; the area accumulated from the highest sample down.
pub(super) fn network<I: Iterator<Item = (usize, f64)>>(h: &[f64], area: impl Fn(usize) -> f64, around: impl Fn(usize) -> I) -> (Vec<u32>, Vec<f64>, Vec<u32>) {
    let n = h.len();
    let to: Vec<u32> = (0..n)
        .map(|k| {
            let steepest = around(k).filter(|&(j, _)| h[j] < h[k]).fold((NONE, 0.0), |best, (j, d)| if (h[k] - h[j]) / d > best.1 { (j as u32, (h[k] - h[j]) / d) } else { best });
            steepest.0
        })
        .collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&p, &q| h[q].total_cmp(&h[p]).then(p.cmp(&q)));
    let mut up: Vec<f64> = (0..n).map(area).collect();
    let mut main = vec![NONE; n];
    for &k in &order {
        let j = to[k];
        if j != NONE {
            let j = j as usize;
            up[j] += up[k];
            if main[j] == NONE || up[k] > up[main[j] as usize] {
                main[j] = k as u32;
            }
        }
    }
    (to, up, main)
}

/// A rectangle of the map in leaves of the tree's most depth, west, north, east and south: it holds the points from
/// its west and north edges up to its east and south ones, so each point lies in one leaf.
type Rect = [f64; 4];

fn rect(l: &Leaf) -> Rect {
    let s = l.side() as f64;
    [l.x as f64, l.y as f64, l.x as f64 + s, l.y as f64 + s]
}

fn holds(r: &Rect, p: [f64; 2]) -> bool {
    (r[0]..r[2]).contains(&p[0]) && (r[1]..r[3]).contains(&p[1])
}

/// The part of the way from `a` to `b` in `r`, as shares of the way from `a` (Liang and Barsky 1984), if any of it is
/// there. A way along the edge of `r` lies in it only along its west or north edge (`holds`).
fn piece(a: [f64; 2], b: [f64; 2], r: &Rect) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for axis in 0..2 {
        let (lo, hi, d) = (r[axis], r[axis + 2], b[axis] - a[axis]);
        if d == 0.0 {
            if !(lo..hi).contains(&a[axis]) {
                return None;
            }
            continue;
        }
        let (u, v) = ((lo - a[axis]) / d, (hi - a[axis]) / d);
        (t0, t1) = (t0.max(u.min(v)), t1.min(u.max(v)));
    }
    (t1 - t0 > TOUCH).then_some((t0, t1))
}

impl Samples {
    /// The side of a sample in leaves of the tree's most depth.
    pub fn side() -> u64 {
        BLOCK >> SAMPLE_DEPTH
    }

    /// The centre of the sample at row `r` and column `c` (on a globe, any whole number: round the globe the same
    /// samples again), in leaves of the tree's most depth.
    pub fn centre(r: usize, c: i64) -> [f64; 2] {
        let s = Self::side() as f64;
        [(c as f64 + 0.5) * s, (r as f64 + 0.5) * s]
    }

    /// The sample at row `r` and column `c` (round a globe on `map`), if on the map.
    pub fn at(&self, map: &Square, r: i64, c: i64) -> Option<usize> {
        let c = if map.wrap { c.rem_euclid(self.across as i64) } else { c };
        ((0..self.down as i64).contains(&r) && (0..self.across as i64).contains(&c)).then(|| r as usize * self.across + c as usize)
    }

    /// The end of the way of the sample at row `r` and column `c` (`at`), where its liquid runs, beside its own centre
    /// (round a globe, the neighbour's centre the near way): none for a sink.
    pub fn way_end(&self, map: &Square, r: usize, c: i64) -> Option<[f64; 2]> {
        let k = self.at(map, r as i64, c)?;
        let j = self.to[k];
        if j == NONE {
            return None;
        }
        let (jr, jc, kc) = ((j as usize / self.across) as i64, (j as usize % self.across) as i64, (k % self.across) as i64);
        let wide = self.across as i64;
        let dc = (jc - kc + wide + 1).rem_euclid(wide) - 1;
        Some(Self::centre(jr as usize, c + if map.wrap { dc } else { jc - kc }))
    }

    /// The drained area that leaves leaf `l` of `map` and then leaf `within`, which holds it, m² (on a map between
    /// latitudes `south` and `north`): of every way with a part or its start in `l` whose end lies outside it, when that part is its
    /// last in `within` and its end lies outside that too; of every sink in `l`; and of a leaf finer than the samples
    /// whose sample's centre lies outside it, its own area, which runs there, when that centre lies outside `within`
    /// too. With `within` the leaf itself: all that leaves it.
    pub fn leaving(&self, map: &Square, (south, north): (f64, f64), l: Leaf, within: Leaf) -> f64 {
        let (r, w, side) = (rect(&l), rect(&within), Self::side());
        let span = |from: u64, k: u64| (from / side) as i64 - 1..=((from + k - 1) / side) as i64 + 1;
        let mut out = 0.0;
        for row in span(l.y, l.side()) {
            for col in span(l.x, l.side()) {
                let Some(k) = self.at(map, row, col) else { continue };
                let from = Self::centre(row as usize, col);
                let Some(end) = self.way_end(map, row as usize, col) else {
                    out += if holds(&r, from) { self.up[k] } else { 0.0 };
                    continue;
                };
                // Where the leaf holds the way's start alone (its corner), the liquid leaves it there.
                let last = |r: &Rect| piece(from, end, r).map(|p| p.1).or(holds(r, from).then_some(0.0));
                let Some(t1) = last(&r).filter(|_| !holds(&r, end)) else { continue };
                if l == within || (!holds(&w, end) && last(&w).is_some_and(|w1| w1 - t1 < SAME)) {
                    out += self.up[k];
                }
            }
        }
        if l.depth > SAMPLE_DEPTH {
            let centre = Self::centre((l.y / side) as usize, (l.x / side) as i64);
            if !holds(&r, centre) && !holds(&w, centre) {
                out += map.place_of(&l, south, north).0;
            }
        }
        out
    }
}

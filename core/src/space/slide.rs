//! The slide of the ground edits shaped (laws.md, Edits and their settling; `World::slid`, `World::worn`): ground that
//! stands steeper than loose ground rests at, or bends sharper than slid ground lies, moves down to its lower neighbours
//! until it rests, the volume kept. Its slope is the steepest way down a sample, read the same along any direction of
//! the grid, and its bend is read across its four sides alike, so its rest is round.

use super::grid::Grid;
use super::mantle::REST;

/// The most passes of a slide (`slide`), and the samples about those that give ground that may take some: no ground
/// leaves the box of the givers and this many samples about them. The slide ends sooner, once no sample fails.
pub(crate) const SLIDE_PASSES: usize = 32;
/// Ground that slid comes to rest this much gentler than the slope at which loose ground fails (`REST`), degrees: a
/// pile of loose solid pieces fails at its most stable angle and comes to rest a couple of degrees under it (Jaeger,
/// Liu and Nagel 1989, from memory).
const SLID_UNDER_DEG: f64 = 2.0;
/// The least radius over which slid ground bends, m: its crest rounds and its foot runs out over no less (a ridge's
/// across it; a dome's is twice).
const BEND_M: f64 = 800.0;
/// A sample fails once its bend passes the most it may bend by this share of it: a smaller excess is left by the
/// passes before, and the slide ends.
const FAILS_OVER: f64 = 0.05;

/// A sample lower than it stood before any edit by less than this is the rounding of the volumes it gave and took, m,
/// not ground an edit lowered (`Sliding::can`).
const DUG_M: f64 = 1e-6;

/// The samples of a grid as a slide reads them: of each its four neighbours east, west, north and south (itself off
/// the grid), and of each row its samples' width and length, m, and area, m².
struct Near {
    sides: Vec<[usize; 4]>,
    rows: Vec<[f64; 3]>,
    across: usize,
}

impl Near {
    fn of(g: &Grid) -> Near {
        Near { sides: (0..g.n()).map(|i| g.sides(i).map(|p| p.0)).collect(), rows: g.dx.iter().map(|&dx| [dx, g.dy, dx * g.dy]).collect(), across: g.across }
    }
    /// The width and the length of sample `i`, m (the lengths to its neighbours east or west and north or south).
    fn lengths(&self, i: usize) -> [f64; 2] {
        let r = &self.rows[i / self.across];
        [r[0], r[1]]
    }
    fn area(&self, i: usize) -> f64 {
        self.rows[i / self.across][2]
    }
    /// The slope of heights `h` at sample `i`: of its two lower neighbours across its sides, one east or west and one
    /// north or south, each with how much lower it stands per m (none where neither is lower: the neighbour is then the
    /// sample itself). The steepest way down the sample is their hypotenuse (an upwind slope, as of the eikonal
    /// equation, Osher and Sethian 1988): even ground tilted any way reads its own slope.
    fn downhill(&self, h: &[f64], i: usize) -> [(usize, f64); 2] {
        let ([e, w, n, s], [dx, dy]) = (self.sides[i], self.lengths(i));
        let lower = |a: usize, b: usize, len: f64| {
            let j = if h[a] <= h[b] { a } else { b };
            if h[j] < h[i] { (j, (h[i] - h[j]) / len) } else { (i, 0.0) }
        };
        [lower(e, w, dx), lower(n, s, dy)]
    }
    /// The slope of heights `h` at sample `i`, rise over run (`downhill`).
    fn slope(&self, h: &[f64], i: usize) -> f64 {
        let [(_, a), (_, b)] = self.downhill(h, i);
        a.hypot(b)
    }
    /// Of the sides of sample `i`, the neighbour and one over the square of its length, m⁻² (none off the grid).
    fn weights(&self, i: usize) -> [(usize, f64); 4] {
        let [dx, dy] = self.lengths(i);
        let [e, w, n, s] = self.sides[i];
        [(e, dx), (w, dx), (n, dy), (s, dy)].map(|(j, len)| (j, if j == i { 0.0 } else { 1.0 / (len * len) }))
    }
    /// How heights `h` bend at sample `i`, per m: the sum over its sides of how much higher the neighbour stands over the
    /// square of its length (the Laplacian; under none on a crest, over none in a hollow).
    fn bend(&self, h: &[f64], i: usize) -> f64 {
        self.weights(i).iter().map(|&(j, w)| (h[j] - h[i]) * w).sum()
    }
}

/// A slide under way: the grid, the heights now and before any edit, what each sample may still give, m³, whether a sample may fail as a wall
/// (`fresh`: give of its own ground where it stands too steep), the samples to look at in this pass, and those whose
/// ground or whose neighbours' ground moved in it, to look at in the next.
struct Sliding<'a> {
    at: Near,
    h: &'a mut [f64],
    base: &'a [f64],
    has: Vec<f64>,
    fresh: bool,
    now: Vec<bool>,
    stirred: Vec<bool>,
}

impl Sliding<'_> {
    /// At most `most` passes over the samples in `order`, each looked at by `rule` (whether it moved any ground) where
    /// ground moved about it in the pass before, or came to it in this one: they end once a pass moves none. A sample
    /// nothing stirred stands as it stood when it was last looked at, and is left. So a wall fails back a sample a pass
    /// whatever the order, and ground goes on down in the pass it came.
    fn passes(&mut self, order: &[usize], most: usize, rule: impl Fn(&mut Self, usize) -> bool) {
        for _ in 0..most {
            let mut slid = false;
            self.now = std::mem::replace(&mut self.stirred, vec![false; self.has.len()]);
            for &i in order {
                if std::mem::take(&mut self.now[i]) {
                    slid |= rule(self, i);
                }
            }
            if !slid {
                break;
            }
        }
    }

    /// What sample `i` may give where it fails, m³: what it took, or any of its own where it may fail as a wall: where
    /// the ground is fresh and it, or a neighbour across a side, stands lower than before any edit (a pit's wall; ground
    /// an edit only raised gives no more than came on it).
    fn can(&self, i: usize) -> f64 {
        let dug = |j: usize| self.h[j] < self.base[j] - DUG_M;
        if self.fresh && (dug(i) || self.at.sides[i].iter().any(|&j| dug(j))) { f64::INFINITY } else { self.has[i] }
    }

    /// `v` m³ moved from sample `i` to its lower neighbour `j`, kept by their areas: `j` may give it on, later in the same
    /// pass where it stands lower; `i` gives what it took first. Both and their neighbours are stirred for the next pass.
    fn send(&mut self, i: usize, j: usize, v: f64) {
        self.h[i] -= v / self.at.area(i);
        self.h[j] += v / self.at.area(j);
        self.has[i] = (self.has[i] - v).max(0.0);
        self.has[j] += v;
        self.now[j] = true;
        for k in [i, j] {
            self.stirred[k] = true;
            self.at.sides[k].iter().for_each(|&n| self.stirred[n] = true);
        }
    }

    /// Sample `i`, standing steeper than `fails`, sends its two lower neighbours (`Near::downhill`) the volume that brings
    /// it to `rests`, each by its part of the slope (the flux down the slope across each side), no more than it has, and
    /// neither standing over it after. Whether it sent any.
    fn steep(&mut self, i: usize, [fails, rests]: [f64; 2]) -> bool {
        let [(jx, a), (jy, b)] = self.at.downhill(self.h, i);
        let s = a.hypot(b);
        if s <= fails {
            return false;
        }
        let (wx, wy, ai, [dx, dy]) = (a / (a + b), b / (a + b), self.at.area(i), self.at.lengths(i));
        // Each part of the slope falls by v times these as v m³ leave the sample.
        let da = if a > 0.0 { (1.0 / ai + wx / self.at.area(jx)) / dx } else { 0.0 };
        let db = if b > 0.0 { (1.0 / ai + wy / self.at.area(jy)) / dy } else { 0.0 };
        // (a − v da)² + (b − v db)² = rests², its smaller root; where none, as close as it comes.
        let (p, q) = (a * da + b * db, da * da + db * db);
        let d = p * p - q * (s * s - rests * rests);
        let over = |part: f64, by: f64| if by > 0.0 { part / by } else { f64::INFINITY };
        let v = (if d > 0.0 { (p - d.sqrt()) / q } else { p / q }).min(over(a, da)).min(over(b, db)).min(self.can(i));
        self.send(i, jx, v * wx);
        self.send(i, jy, v * wy);
        v > 0.0
    }

    /// Sample `i`, bending sharper than `most` over its crest (under none), sends its lower neighbours across its sides
    /// the volume that brings it to `most`, each by how much lower it stands over the square of its length (as creep
    /// carries it), no more than it has, and none standing over it after. Whether it sent any.
    fn crest(&mut self, i: usize, most: f64) -> bool {
        let l = self.at.bend(self.h, i);
        if l >= most * (1.0 + FAILS_OVER) {
            return false;
        }
        let (hi, ai, sides) = (self.h[i], self.at.area(i), self.at.weights(i));
        // Each side's share: how much lower it stands over the square of its length (none where not lower).
        let share = sides.map(|(j, w)| (hi - self.h[j]).max(0.0) * w);
        let all: f64 = share.iter().sum();
        // The bend rises by v times this as v m³ leave the sample.
        let by: f64 = sides.iter().zip(&share).map(|(&(j, w), s)| (1.0 / ai + s / all / self.at.area(j)) * w).sum();
        let mut v = ((most - l) / by).min(self.can(i));
        for (&(j, _), &s) in sides.iter().zip(&share).filter(|p| *p.1 > 0.0) {
            v = v.min((hi - self.h[j]) / (1.0 / ai + s / all / self.at.area(j)));
        }
        for (&(j, _), &s) in sides.iter().zip(&share).filter(|p| *p.1 > 0.0) {
            self.send(i, j, v * s / all);
        }
        v > 0.0
    }

    /// Sample `j`, bending sharper than `most` in its hollow (over none), takes from its higher neighbours across its
    /// sides that may give (`Sliding::can`) the volume that brings it to `most`, from each by how much higher it stands over
    /// the square of its length (as creep carries it), no more than it has, nor so much that `j` stands over it. Whether
    /// it took any.
    fn foot(&mut self, j: usize, most: f64) -> bool {
        let l = self.at.bend(self.h, j);
        if l <= most * (1.0 + FAILS_OVER) {
            return false;
        }
        let (hj, aj, sides) = (self.h[j], self.at.area(j), self.at.weights(j));
        let share = sides.map(|(i, w)| if self.can(i) > 0.0 { (self.h[i] - hj).max(0.0) * w } else { 0.0 });
        let all: f64 = share.iter().sum();
        if all <= 0.0 {
            return false;
        }
        // The bend of `j` falls by v times this as v m³ come to it.
        let by: f64 = sides.iter().zip(&share).map(|(&(i, w), s)| (1.0 / aj + s / all / self.at.area(i)) * w).sum();
        let mut v = (l - most) / by;
        for (&(i, _), &s) in sides.iter().zip(&share).filter(|p| *p.1 > 0.0) {
            v = v.min(self.can(i) * all / s).min((self.h[i] - hj) / (s / all / self.at.area(i) + 1.0 / aj));
        }
        for (&(i, _), &s) in sides.iter().zip(&share).filter(|p| *p.1 > 0.0) {
            self.send(i, j, v * s / all);
        }
        v > 0.0
    }
}

/// Ground slides on grid `g` (a box of the terrain's samples), of heights `h` (m, row by row; moved in place) that stood
/// at `base` before any edit, from the samples that give it (`gives`: the volume each may give, m³, and what it takes),
/// in passes (`SLIDE_PASSES`), each over the samples highest first as they stood when the slide began
/// (`Sliding::passes`). A sample that stands steeper than loose ground rests at (`REST`; or than it stood before any
/// edit, where steeper: steep ground no edit made keeps its slope) sends its lower neighbours the ground that brings it
/// `SLID_UNDER_DEG` under that (`Sliding::steep`). Where the ground is `fresh` (a paint just shaped it), a sample beside
/// ground an edit lowered may so fail as a wall, with its own ground (`Sliding::can`), and the slid ground rounds: a
/// sample that bends over its crest sharper than slid ground lies (`BEND_M`; or than it bent before any edit) sends
/// them what rounds it (`Sliding::crest`), and one that bends so in its hollow takes what fills it from its higher
/// neighbours that may give (`Sliding::foot`). So the rest of a held brush is a round cone or bowl, its crest rounded
/// and its foot running out into the ground about it. The volume is kept by the samples' areas; no ground leaves the
/// box.
pub(super) fn slide(g: &Grid, (h, base): (&mut [f64], &[f64]), gives: &[f64], fresh: bool) {
    let at = Near::of(g);
    let under = (REST.atan() - SLID_UNDER_DEG.to_radians()).tan() / REST;
    let steepest: Vec<[f64; 2]> = (0..g.n()).map(|i| at.slope(base, i)).map(|s| [REST.max(s), (REST * under).max(s)]).collect();
    let crest: Vec<f64> = (0..g.n()).map(|i| (-1.0 / BEND_M).min(at.bend(base, i))).collect();
    let hollow: Vec<f64> = (0..g.n()).map(|i| (1.0 / BEND_M).max(at.bend(base, i))).collect();
    let mut order: Vec<usize> = (0..g.n()).collect();
    order.sort_by(|&a, &b| h[b].total_cmp(&h[a]).then(a.cmp(&b)));
    let mut s = Sliding { at, h, base, has: gives.to_vec(), fresh, now: Vec::new(), stirred: vec![true; g.n()] };
    s.passes(&order, SLIDE_PASSES, |s, i| s.steep(i, steepest[i]) || (fresh && (s.crest(i, crest[i]) | s.foot(i, hollow[i]))));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat grid `n` samples a side of `side` m, a brush of `r` samples at its middle held for `frames` paints of
    /// `height` m each at its centre (by (1 − (d/r)²)², raised or lowered), each paint followed by the slide, each sample
    /// giving what the paints laid on it (`World::slid`): the heights, the grid and the volume the paints raised, m³.
    fn held(n: usize, r: f64, (frames, height): (usize, f64)) -> (Vec<f64>, Grid, f64) {
        let (g, side) = (Grid::new(n, n, false, 125.0, vec![125.0; n]), n as f64 / 2.0 - 0.5);
        let d = |i: usize| ((i % n) as f64 - side).hypot((i / n) as f64 - side) / r;
        let (mut h, base) = (vec![0.0; n * n], vec![0.0; n * n]);
        let mut painted = 0.0;
        for _ in 0..frames {
            for i in (0..n * n).filter(|&i| d(i) < 1.0) {
                h[i] += height * (1.0 - d(i) * d(i)).powi(2);
                painted += height * (1.0 - d(i) * d(i)).powi(2) * g.area(i);
            }
            let gives: Vec<f64> = (0..n * n).map(|i| h[i].max(0.0) * g.area(i)).collect();
            slide(&g, (&mut h, &base), &gives, true);
        }
        (h, g, painted)
    }

    /// How far from the middle of the grid, in samples, the heights `h` (n a side) stand more than `above` m from
    /// none along the direction `deg` (bilinear between the samples).
    fn reach(h: &[f64], n: usize, deg: f64, above: f64) -> f64 {
        let m = n as f64 / 2.0 - 0.5;
        let at = |x: f64, y: f64| {
            let (x0, y0) = (x.floor() as usize, y.floor() as usize);
            let (fx, fy, v) = (x - x0 as f64, y - y0 as f64, |a: usize, b: usize| h[b * n + a].abs());
            (v(x0, y0) * (1.0 - fx) + v(x0 + 1, y0) * fx) * (1.0 - fy) + (v(x0, y0 + 1) * (1.0 - fx) + v(x0 + 1, y0 + 1) * fx) * fy
        };
        let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
        (0..(m as usize - 1) * 20).map(|k| k as f64 / 20.0).filter(|&t| at(m + t * c, m + t * s) > above).fold(0.0, f64::max)
    }

    /// A brush held long on flat ground raises a round hill and digs a round pit: its foot and its contours lie as far
    /// from the middle along a side, a corner and between (22.5°) to within a sample; no sample of the hill stands
    /// steeper than loose ground rests at (`REST`; a pit's walls, which fail back a sample a pass, may stand a little
    /// steeper while the brush digs); along a row the slope changes from one sample to the next by less than a third of
    /// `REST` (no crease, no spike); the volume is kept.
    #[test]
    fn a_held_brush_rests_round_and_smooth() {
        let n = 81;
        for height in [15.0, -15.0] {
            let (h, g, painted) = held(n, 8.0, (120, height));
            let at = Near::of(&g);
            let volume: f64 = (0..n * n).map(|i| h[i] * g.area(i)).sum();
            assert!((volume / painted - 1.0).abs() < 1e-9, "{volume} m3 of {painted}");
            for above in [1.0, 100.0, 300.0] {
                let r: Vec<f64> = [0.0, 22.5, 45.0].iter().map(|&deg| reach(&h, n, deg, above)).collect();
                let spread = r.iter().fold(f64::MIN, |a, &b| a.max(b)) - r.iter().fold(f64::MAX, |a, &b| a.min(b));
                assert!(spread < 1.0, "{height} m a paint: past {above} m at {r:?} samples");
            }
            let steepest = (0..n * n).map(|i| at.slope(&h, i)).fold(0.0, f64::max);
            assert!(height < 0.0 || steepest <= REST, "{height} m a paint: {steepest}");
            let row: Vec<f64> = (0..n).map(|x| h[n / 2 * n + x]).collect();
            let slopes: Vec<f64> = row.windows(2).map(|p| (p[1] - p[0]) / 125.0).collect();
            let worst = slopes.windows(2).map(|p| (p[1] - p[0]).abs()).fold(0.0, f64::max);
            assert!(worst < REST / 3.0, "{height} m a paint: the slope changes by {worst} between samples: {slopes:?}");
        }
    }
}

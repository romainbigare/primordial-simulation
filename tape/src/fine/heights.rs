//! The terrain's heights below the cells as `fine` reads them (`space::terrain`). At or above the terrain's samples
//! (`SAMPLE_DEPTH`) a leaf's height is the mean of the samples in it, as `Terrain::inside` takes it. Finer, it is the
//! departures of the terrain's midpoint displacement (`Terrain::departures`) laid on a smooth continuation of the
//! coarser heights: each leaf's quarters lean as the quadratic through the leaf and its neighbours on each axis does,
//! keeping its mean (`finer`), so the ground has no steps at the leaves' edges. A leaf's height depends on its own
//! place only, whatever window it is found for.

use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::space::tree::BLOCK;
use pcore::space::Leaf;
use pcore::World;
use std::collections::BTreeMap;

/// Leaves of each depth round a window whose heights are found too: the quadratic of the next depth reads one each way.
const MARGIN: i64 = 2;
/// A cell's points (`Samples::points`) are the leaves this many depths below it: 8 x 8.
pub(super) const POINT_DEPTHS: u8 = 3;

/// The terrain's samples of a map (`Terrain::sample` at the centres of the leaves of `SAMPLE_DEPTH`): the height, m,
/// and the slope of the noise below it, found when first needed and kept (the terrain does not change), with the
/// terrain's mean height over each cell's leaf and at its points found so far.
pub struct Samples {
    /// The map's width and height in samples, and the area of a sample in each row, m²
    across: usize,
    down: usize,
    area: Vec<f64>,
    h: Vec<f32>,
    g: Vec<f32>,
    means: BTreeMap<Leaf, f64>,
    points: BTreeMap<Leaf, Vec<f64>>,
}

impl Samples {
    /// None found yet, for world `w`.
    pub fn new(w: &World) -> Samples {
        let sq = w.map.square_depth();
        assert!(sq <= SAMPLE_DEPTH, "a square is no finer than the terrain's samples");
        let per = 1usize << (SAMPLE_DEPTH - sq);
        let (across, down) = (w.map.w * per, w.map.h * per);
        let area = (0..down).map(|y| w.map.place_of(&leaf(SAMPLE_DEPTH, 0, y as i64), w.cfg.lat_south, w.cfg.lat_north).0).collect();
        Samples { across, down, area, h: Vec::new(), g: Vec::new(), means: BTreeMap::new(), points: BTreeMap::new() }
    }
    /// Find the samples, if not yet found.
    fn fill(&mut self, w: &World) {
        if !self.h.is_empty() {
            return;
        }
        let place = |k: usize| w.map.place_of(&leaf(SAMPLE_DEPTH, (k % self.across) as i64, (k / self.across) as i64), w.cfg.lat_south, w.cfg.lat_north).1;
        (self.h, self.g) = (0..self.across * self.down).map(|k| w.terrain.sample(w.seed, &w.map, place(k))).map(|(h, g)| (h as f32, g as f32)).unzip();
    }
    /// The height and slope of the sample at column `x` and row `y`, the nearest on the map where it lies off it.
    fn at(&self, x: i64, y: i64) -> (f64, f64) {
        let k = y.clamp(0, self.down as i64 - 1) as usize * self.across + x.clamp(0, self.across as i64 - 1) as usize;
        (self.h[k] as f64, self.g[k] as f64)
    }
    /// The mean height by area of the samples from column `x` and row `y`, `n` a side.
    fn mean(&self, x: i64, y: i64, n: i64) -> f64 {
        let (mut sum, mut area) = (0.0, 0.0);
        for r in y..y + n {
            let a = self.area[r.clamp(0, self.down as i64 - 1) as usize];
            for c in x..x + n {
                (sum, area) = (sum + a * self.at(c, r).0, area + a);
            }
        }
        sum / area
    }
    /// The terrain's mean height over leaf `l`, m (kept once found).
    pub(super) fn mean_of(&mut self, w: &World, l: Leaf) -> f64 {
        if let Some(&h) = self.means.get(&l) {
            return h;
        }
        let (x, y) = ((l.x / l.side()) as i64, (l.y / l.side()) as i64);
        let h = window(w, self, [x, y, x, y], l.depth)[0];
        self.means.insert(l, h);
        h
    }
    /// The terrain's heights at the leaves `POINT_DEPTHS` below leaf `l`, m, row by row (kept once found).
    pub(super) fn points(&mut self, w: &World, l: Leaf) -> &[f64] {
        if !self.points.contains_key(&l) {
            let (k, d) = (1i64 << POINT_DEPTHS, l.depth + POINT_DEPTHS);
            let (x, y) = ((l.x / (BLOCK >> d)) as i64, (l.y / (BLOCK >> d)) as i64);
            let h = window(w, self, [x, y, x + k - 1, y + k - 1], d);
            self.points.insert(l, h);
        }
        &self.points[&l]
    }
}

/// The leaf of `depth` at column `x` and row `y` of them.
fn leaf(depth: u8, x: i64, y: i64) -> Leaf {
    let side = BLOCK >> depth;
    Leaf { x: x as u64 * side, y: y as u64 * side, depth }
}

/// The terrain's heights, m, and noise slopes at the leaves of one depth over a rectangle of them (from column `x` and
/// row `y`, in leaves of that depth), row by row.
struct Patch {
    depth: u8,
    x: i64,
    y: i64,
    w: usize,
    h: usize,
    z: Vec<f64>,
    g: Vec<f64>,
}

impl Patch {
    /// The height and slope at leaf (`x`, `y`), the nearest the patch holds where it holds none.
    fn near(&self, x: i64, y: i64) -> (f64, f64) {
        let k = (y - self.y).clamp(0, self.h as i64 - 1) as usize * self.w + (x - self.x).clamp(0, self.w as i64 - 1) as usize;
        (self.z[k], self.g[k])
    }
}

/// The rectangle of leaves of `depth` that hold the window `win` (from column `x0` and row `y0` to `x1` and `y1`,
/// inclusive, in leaves of `of`, no shallower) with `MARGIN` leaves round it, on a map `across` x `down` leaves of `of`.
fn cover([x0, y0, x1, y1]: [i64; 4], of: u8, depth: u8, (across, down): (i64, i64)) -> [i64; 4] {
    let k = of - depth;
    let (aw, ad) = ((across >> k).max(1), (down >> k).max(1));
    [((x0 >> k) - MARGIN).max(0), ((y0 >> k) - MARGIN).max(0), ((x1 >> k) + MARGIN).min(aw - 1), ((y1 >> k) + MARGIN).min(ad - 1)]
}

/// The leaves of the depth below `p`'s over the rectangle `want` of them (inside `p`'s quarters): each quarter of a leaf
/// of `p` takes the leaf's height, the lean of the quadratic through it and its neighbours along each axis (a mean of
/// the quarters of a leaf whose neighbours' heights are E, W, S and N leans by ±(E − W) / 8 and ±(S − N) / 8, and the
/// corners' (SE − SW − NE + NW) / 64; a neighbour off the map is the leaf), less that lean's mean by area, and the
/// terrain's own departure there (`Terrain::departures`), so each leaf's quarters keep its mean.
fn finer(w: &World, p: &Patch, [x0, y0, x1, y1]: [i64; 4]) -> Patch {
    let (depth, lat) = (p.depth + 1, (w.cfg.lat_south, w.cfg.lat_north));
    let (cw, ch) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut out = Patch { depth, x: x0, y: y0, w: cw, h: ch, z: vec![0.0; cw * ch], g: vec![0.0; cw * ch] };
    for py in y0 >> 1..=y1 >> 1 {
        for px in x0 >> 1..=x1 >> 1 {
            let at = |dx: i64, dy: i64| p.near(px + dx, py + dy).0;
            let (mid, g) = p.near(px, py);
            let (ew, sn) = ((at(1, 0) - at(-1, 0)) / 8.0, (at(0, 1) - at(0, -1)) / 8.0);
            let corners = (at(1, 1) - at(-1, 1) - at(1, -1) + at(-1, -1)) / 64.0;
            let parent = leaf(p.depth, px, py);
            let area = parent.quarters().map(|q| w.map.place_of(&q, lat.0, lat.1).0);
            let lean: [f64; 4] = std::array::from_fn(|q| {
                let (sx, sy) = (if q % 2 == 1 { 1.0 } else { -1.0 }, if q >= 2 { 1.0 } else { -1.0 });
                sx * ew + sy * sn + sx * sy * corners
            });
            let mean = (0..4).map(|q| area[q] * lean[q]).sum::<f64>() / area.iter().sum::<f64>();
            let d = w.terrain.departures(w.seed, &w.map, parent, lat, g);
            for q in 0..4 {
                let (cx, cy) = (2 * px + (q % 2) as i64, 2 * py + (q / 2) as i64);
                if (x0..=x1).contains(&cx) && (y0..=y1).contains(&cy) {
                    let k = (cy - y0) as usize * cw + (cx - x0) as usize;
                    (out.z[k], out.g[k]) = (mid + lean[q] - mean + d[q], g);
                }
            }
        }
    }
    out
}

/// The terrain's height at each leaf of `depth` (no shallower than a square) of the window `win` (from column `x0` and
/// row `y0` to `x1` and `y1`, inclusive, in those leaves), m, row by row.
pub(super) fn window(w: &World, s: &mut Samples, win: [i64; 4], depth: u8) -> Vec<f64> {
    s.fill(w);
    let (across, down) = ((win[2] - win[0] + 1) as usize, (win[3] - win[1] + 1) as usize);
    let pixel = |p: usize| (win[0] + (p % across) as i64, win[1] + (p / across) as i64);
    if depth <= SAMPLE_DEPTH {
        let n = 1i64 << (SAMPLE_DEPTH - depth);
        return (0..across * down).map(pixel).map(|(x, y)| s.mean(x * n, y * n, n)).collect();
    }
    let shift = depth - w.map.square_depth();
    let extent = ((w.map.w as i64) << shift, (w.map.h as i64) << shift);
    let first = cover(win, depth, SAMPLE_DEPTH, extent);
    let (pw, ph) = ((first[2] - first[0] + 1) as usize, (first[3] - first[1] + 1) as usize);
    let (z, g) = (0..pw * ph).map(|k| s.at(first[0] + (k % pw) as i64, first[1] + (k / pw) as i64)).unzip();
    let mut patch = Patch { depth: SAMPLE_DEPTH, x: first[0], y: first[1], w: pw, h: ph, z, g };
    for d in SAMPLE_DEPTH + 1..=depth {
        patch = finer(w, &patch, cover(win, depth, d, extent));
    }
    (0..across * down).map(pixel).map(|(x, y)| patch.near(x, y).0).collect()
}

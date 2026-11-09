//! The channels of a window (laws.md 0 and 1.2): the liquid each cell passes on runs down the terrain's channels
//! (`space::network`), each sample's way carrying the share of its cell's flux that drains through the sample (its
//! drained area over all that leaves the cell, `Samples::leaving`, as a split shares it out), in a channel as wide and
//! deep as that flux makes it (`World::channel`). Each way is drawn as a curve from the middle of the way into its
//! sample (from the neighbour upstream that the most drains through) to the middle of its own, bent at the sample's
//! centre (a quadratic Bézier), so the line is smooth, follows the drained ground and depends on the seed alone: the
//! same at every zoom and across the cells' edges. A pixel holds the share of the channel's width its footprint covers
//! across it (a tent a pixel wide each way), all of a pixel well inside a channel.

use super::heights::leaf;
use pcore::space::network::NONE;
use pcore::space::samples::Samples as Drained;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::space::tree::{BLOCK, DEPTH_MOST};
use pcore::space::Leaf;
use pcore::World;
use std::collections::BTreeMap;

/// The straight pieces each way's curve is drawn as.
const CURVE_STEPS: usize = 8;
/// A channel narrower than this share of a pixel shows less than half a display level (of 255) in it: not drawn.
const SHOWN: f64 = 1.0 / 510.0;
/// The depth below its air block of the regions over which the flux per drained area is read for cells finer than them
/// (4 x 4 of the terrain's samples): the core's liquid between cells finer than the samples runs by their own heights,
/// a sample or two off the samples' channels.
const REGION: u8 = SAMPLE_DEPTH - 2;
/// The start of a way's piece in a cell, as a share of the way from its sample's centre: the cell its flux is read
/// from holds the point just after the centre, as a split shares it out (`Samples::leaving`).
const NUDGE: f64 = 1e-6;

/// Of each pixel of the window from leaf column `gx` and row `gy`, `across` x `down` leaves of `depth`: the share of it
/// a channel covers, 0..1, and that channel's depth, m (none where no channel).
pub(super) fn channels(w: &World, drained: &Drained, ([gx, gy], [across, down], depth): ([i64; 2], [usize; 2], u8)) -> (Vec<f32>, Vec<f32>) {
    let (n, lat) = (across * down, (w.cfg.lat_south, w.cfg.lat_north));
    let (mut share, mut deep) = (vec![0.0f32; n], vec![0.0f32; n]);
    let (side, pside) = (Drained::side() as f64, (BLOCK >> depth) as f64);
    // Metres a leaf unit north to south, and east to west in each pixel row.
    let ns = w.map.length(depth) / pside;
    let ew: Vec<f64> = (0..down).map(|r| w.map.place_of(&leaf(depth, gx, gy + r as i64), lat.0, lat.1).0 / w.map.length(depth) / pside).collect();
    let least_ew = ew.iter().copied().fold(f64::MAX, f64::min);
    let unit = (BLOCK >> w.map.square_depth()) as f64;
    let tree = w.mesh.tree();
    // The flux per m² of drained area that leaves a region, mol/(m² s): of a cell no finer than `REGION`, its own; finer,
    // that of all the cells of the leaf of `REGION` that holds the sample together (their fluxes over their drained
    // areas that leave them), so a channel is drawn whichever of those cells the core's liquid runs through. A split
    // leaves both as they were (`World::split`).
    let mut per: BTreeMap<Leaf, f64> = BTreeMap::new();
    let mut flux_per_area = |i: usize, k: usize| {
        let l = w.map.leaves()[i];
        let shift = SAMPLE_DEPTH - REGION;
        let region = if l.depth <= REGION { l } else { leaf(REGION, (k % drained.across) as i64 >> shift, (k / drained.across) as i64 >> shift) };
        *per.entry(region).or_insert_with(|| {
            let (mut flux, mut area) = (0.0, 0.0);
            for c in cells_in(w, region) {
                let cl = w.map.leaves()[c];
                (flux, area) = (flux + w.liquid_flux[c] as f64, area + drained.leaving(&w.map, lat, cl, cl));
            }
            flux / area
        })
    };
    let (x0, y0, x1, y1) = (gx as f64 * pside, gy as f64 * pside, (gx + across as i64) as f64 * pside, (gy + down as i64) as f64 * pside);
    let rows = ((y0 / side).floor() as i64 - 1).max(0)..=((y1 / side).floor() as i64 + 1).min(drained.down as i64 - 1);
    let cols = (x0 / side).floor() as i64 - 1..=(x1 / side).floor() as i64 + 1;
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    // The neighbour's column beside `c` (round a globe, the near way).
    let near = |c: i64, k: usize| {
        let (kc, wide) = ((k % drained.across) as i64, drained.across as i64);
        c + (kc - c.rem_euclid(wide) + wide + 1).rem_euclid(wide) - 1
    };
    // The curve through sample (r, c): from the middle of the way into it to the middle of its own, bent at its centre.
    let curve = |r: usize, c: i64| -> Option<[[f64; 2]; 3]> {
        let k = drained.at(&w.map, r as i64, c)?;
        let (centre, end) = (Drained::centre(r, c), drained.way_end(&w.map, r, c)?);
        let up = drained.main[k];
        let start = if up == NONE { centre } else { mid(Drained::centre(up as usize / drained.across, near(c, up as usize)), centre) };
        Some([start, centre, mid(centre, end)])
    };
    let mut draw = |pts: &[[f64; 2]], width: f64, d: f64| {
        let half = width / 2.0;
        let (lo, hi) = pts.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]));
        let reach = half / ns.min(least_ew) + pside;
        let (c0, c1) = ((((lo[0] - reach) / pside).floor() as i64 - gx).max(0), (((hi[0] + reach) / pside).floor() as i64 - gx).min(across as i64 - 1));
        let (r0, r1) = ((((lo[1] - reach) / pside).floor() as i64 - gy).max(0), (((hi[1] + reach) / pside).floor() as i64 - gy).min(down as i64 - 1));
        for row in r0..=r1 {
            let (sx, px) = (ew[row as usize], pside * ns);
            for col in c0..=c1 {
                let p = [(gx + col) as f64 * pside + pside / 2.0, (gy + row) as f64 * pside + pside / 2.0];
                let dist = pts.windows(2).map(|s| to_segment(p, s[0], s[1], [sx, ns])).fold(f64::MAX, f64::min);
                let covered = tent((half - dist) / px) - tent((-half - dist) / px);
                let q = row as usize * across + col as usize;
                if covered as f32 > share[q] {
                    (share[q], deep[q]) = ((covered as f32).min(1.0), d as f32);
                }
            }
        }
    };
    for r in rows {
        for c in cols.clone() {
            let Some(k) = drained.at(&w.map, r, c) else { continue };
            let Some([a, b, e]) = curve(r as usize, c) else { continue };
            let end = drained.way_end(&w.map, r as usize, c).expect("a way");
            let from = Drained::centre(r as usize, c);
            let at = [from[0] + NUDGE * (end[0] - from[0]), from[1] + NUDGE * (end[1] - from[1])];
            let i = tree.cell_at(w.map.point_at(at[0] / unit, at[1] / unit));
            let ch = w.channel(i, drained.up[k] * flux_per_area(i, k));
            if ch.width / (pside * ns) < SHOWN {
                continue;
            }
            let mut pts: Vec<[f64; 2]> = (0..=CURVE_STEPS).map(|s| bezier(a, b, e, s as f64 / CURVE_STEPS as f64)).collect();
            // A way into a sample that more drains into from elsewhere runs on to the curve through that sample.
            let j = drained.to[k] as usize;
            if drained.main[j] != k as u32 {
                let (jr, jc) = (j / drained.across, near(c, j));
                if let Some([ja, jb, je]) = curve(jr, jc) {
                    pts.push(bezier(ja, jb, je, 0.5));
                }
            }
            draw(&pts, ch.width, ch.depth);
        }
    }
    (share, deep)
}

/// The cells of world `w` inside leaf `l` (`l` itself where it is one; none where `l` lies inside a coarser cell).
fn cells_in(w: &World, l: Leaf) -> Vec<usize> {
    match w.map.cell(l) {
        Some(i) => vec![i],
        None if l.depth < DEPTH_MOST => l.quarters().iter().flat_map(|&q| cells_in(w, q)).collect(),
        None => Vec::new(),
    }
}

/// The share of a tent of half-width 1 that lies below `t` (a pixel's footprint across a line, which spreads a line
/// narrower than a pixel evenly over the pixels along it, whatever its direction).
fn tent(t: f64) -> f64 {
    let t = t.clamp(-1.0, 1.0);
    if t <= 0.0 { (1.0 + t) * (1.0 + t) / 2.0 } else { 1.0 - (1.0 - t) * (1.0 - t) / 2.0 }
}

/// The point at `t` along the quadratic Bézier from `a` to `c` bent toward `b`.
fn bezier(a: [f64; 2], b: [f64; 2], c: [f64; 2], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|k| u * u * a[k] + 2.0 * u * t * b[k] + t * t * c[k])
}

/// The distance, m, from `p` to the segment from `a` to `b` (leaf units), at `m` metres a leaf unit east and south.
fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2], m: [f64; 2]) -> f64 {
    let (d, q) = ([(b[0] - a[0]) * m[0], (b[1] - a[1]) * m[1]], [(p[0] - a[0]) * m[0], (p[1] - a[1]) * m[1]]);
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 { ((q[0] * d[0] + q[1] * d[1]) / len).clamp(0.0, 1.0) } else { 0.0 };
    (q[0] - t * d[0]).hypot(q[1] - t * d[1])
}

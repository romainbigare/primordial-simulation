//! A window of the map finer than its cells, for a shell to draw (handover 10.3): at each pixel the height of the ground
//! from the terrain below the cells (`heights`) set on each cell's own height, the surface of the liquid that stands
//! nearby, and every layer blended between the cells' centres; and the way each cell's liquid runs. It reads the world
//! only.
//!
//! The ground of a pixel is the terrain's there plus an offset blended between the cells' centres: each cell's own
//! height (erosion and all) less the terrain's mean over it, made so that the blend keeps each cell's height as its mean
//! (`offsets`). So the ground has no steps at the cells' edges, and a pixel's ground does not depend on the window.

mod blend;
mod heights;

pub use heights::Samples;

use blend::{Blend, Weights};
use crate::layers::{catchment, cover_melt, quantise, Ctx, LAYERS, STANDING_M};
use pcore::World;

/// Passes that make the blended offsets keep each cell's mean (`offsets`).
const MEAN_PASSES: usize = 64;
/// How far each pass of `offsets` steps, past the plain step: the blend's mean over a cell weighs its own offset by 2/3
/// and each neighbour's by 1/6 along each axis, so a plain pass shrinks the slowest error (a checker of cells of one
/// size) only to 1 − 1/9 of it, this one to 1 − 1.5/9.
const MEAN_STEP: f64 = 1.5;
/// The surface of the liquid about a pixel sinks by this, m, as the share of the blend's weight of the cells that hold
/// standing liquid falls to none, so the liquid and the shore drawn within a few metres of it end smoothly before the
/// blend reaches no such cell.
const SHORE_M: f64 = 10.0;

/// Each cell's offset, m, so that blended between the centres its mean over each cell is `own` (each
/// cell's height less the terrain's mean over it): `MEAN_PASSES` passes of o ← o + `MEAN_STEP` (own − the mean over the
/// cell of o blended) from o = own (Richardson's iteration).
fn offsets(blend: &Blend, own: &[f64]) -> Vec<f64> {
    let means: Vec<Weights> = (0..own.len()).map(|i| blend.mean(i)).collect();
    let mut o = own.to_vec();
    for _ in 0..MEAN_PASSES {
        let blended: Vec<f64> = means.iter().map(|m| m.mix(&o)).collect();
        (0..own.len()).for_each(|i| o[i] += MEAN_STEP * (own[i] - blended[i]));
    }
    o
}

/// The surface of the liquid of each of `cells` (others: 0), m: of one that holds standing liquid, the level its liquid
/// stands at when it fills the lowest of its points first (`Samples::points`, the ground at each as a pixel's), as deep
/// over them all as it is over the cell; of one that holds none, its lowest point, so liquid that stands beside it ends
/// in it where its ground rises above the blend of the two.
fn levels(w: &World, s: &mut Samples, blend: &Blend, off: &[f64], depth: &[f64], cells: &[u32]) -> Vec<f64> {
    let mut out = vec![0.0; off.len()];
    let k = 1usize << heights::POINT_DEPTHS;
    for &i in cells {
        let i = i as usize;
        let [x0, y0, x1, y1] = w.map.bounds(i);
        let mut ground: Vec<f64> = s.points(w, w.map.leaves()[i]).to_vec();
        for (p, g) in ground.iter_mut().enumerate() {
            let at = |from: f64, to: f64, q: usize| from + (to - from) * (q as f64 + 0.5) / k as f64;
            *g += blend.at(i, at(x0, x1, p % k), at(y0, y1, p / k)).mix(off);
        }
        ground.sort_by(f64::total_cmp);
        out[i] = if depth[i] < STANDING_M { ground[0] } else { fill(&ground, depth[i]) };
    }
    out
}

/// The level liquid as deep as `depth` over ground of the heights `ground` (sorted, each of the same area) stands at
/// when it fills the lowest first, m.
fn fill(ground: &[f64], depth: f64) -> f64 {
    let (mut sum, n) = (0.0, ground.len() as f64);
    for (m, g) in ground.iter().enumerate() {
        sum += g;
        let level = (sum + depth * n) / (m + 1) as f64;
        if ground.get(m + 1).is_none_or(|&next| level <= next) {
            return level;
        }
    }
    unreachable!("the ground has a point")
}

/// A segment of a river's way: the centres of a cell and of the cell its liquid runs to (column, row, in squares), its
/// liquid flux, mol/s, and its catchment (`catchment`).
type Way = [f32; 6];

/// The way each of `cells` whose liquid runs (some flux, none standing) sends it: to the cell the steepest way down its
/// head takes the most of (`Level::steepest`, the liquid law's way), if any.
fn ways(c: &Ctx, blend: &Blend, cells: &[u32]) -> Vec<Way> {
    let (w, f, level) = (c.w, &c.f, c.w.mesh.cells());
    let head = |i: usize| f.z[i] + f.depth[i];
    let mut out = Vec::new();
    for &i in cells {
        let i = i as usize;
        let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
        if flux <= 0.0 || f.depth[i] >= STANDING_M {
            continue;
        }
        let to = level.around(i).1;
        let Some((_, [(a, sa), (b, sb)])) = level.steepest(i, |k| Some(head(i) - head(to[k] as usize)).filter(|&d| d > 0.0)) else { continue };
        let j = to[if sa >= sb { a } else { b }] as usize;
        let ([x0, y0], [mut x1, y1]) = (blend.centre[i], blend.centre[j]);
        if w.map.wrap {
            x1 -= w.map.w as f64 * ((x1 - x0) / w.map.w as f64).round();
        }
        out.push([x0, y0, x1, y1, flux, catchment(w, i)].map(|v| v as f32));
    }
    out
}

/// A window of the map finer than its cells: the window from column `x` and row `y`, `wide` x `high` squares, on its
/// edges' leaves of `depth` (no shallower than a square), a pixel each. Little-endian, in order: the melting point of the
/// frozen cover, K (f32; `cover_melt`); the count of river ways (u32) and each (`Way`, 6 f32) of the cells shown and
/// those about them; of each pixel row by row the height of the ground, m (f32), and that of the surface of the liquid of
/// the cells about it (f32, NaN where none of them holds standing liquid: blended between their surfaces, `levels`, and
/// sunk by `SHORE_M` times the share of the blend of the cells that hold none);
/// then every layer in table order, a byte a pixel, blended between the cells' centres (`blend`), the ground's
/// temperature along the ground's height at the rate the air cools with height (`World::air_drop`).
pub fn fine(w: &World, samples: &mut Samples, [x, y, wide, high]: [f64; 4], depth: u8) -> Vec<u8> {
    assert!(depth >= w.map.square_depth(), "a pixel is no coarser than a square");
    let per = (1u64 << (depth - w.map.square_depth())) as f64;
    let (gx, gy) = ((x * per).round() as i64, (y * per).round() as i64);
    let (across, down) = (((wide * per).round() as usize).max(1), ((high * per).round() as usize).max(1));
    let heights = heights::window(w, samples, [gx, gy, gx + across as i64 - 1, gy + down as i64 - 1], depth);
    let (tree, n) = (w.mesh.tree(), w.mesh.cells().n());
    let at = |p: usize| (((gx + (p % across) as i64) as f64 + 0.5) / per, ((gy + (p / across) as i64) as f64 + 0.5) / per);
    let cell: Vec<u32> = (0..across * down).map(&at).map(|(px, py)| tree.cell_at(w.map.point_at(px, py)) as u32).collect();
    let (blend, c) = (Blend::new(w), Ctx::new(w));
    let f = &c.f;
    let own: Vec<f64> = (0..n).map(|i| f.z[i] - samples.mean_of(w, w.map.leaves()[i])).collect();
    let off = offsets(&blend, &own);
    let cools: Vec<f64> = (0..n).map(|i| w.air_drop(i).unwrap_or(0.0)).collect();
    // Each layer's byte of each cell, and the surface of the liquid of each cell that holds standing liquid (0 else).
    let bytes: Vec<Vec<f64>> = LAYERS.iter().map(|l| (0..n).map(|i| quantise(l, (l.val)(&c, i)) as f64).collect()).collect();
    let t_layer = LAYERS.iter().position(|l| l.name == "t_ground").expect("a layer of the ground's temperature");
    let mut shown = cell.clone();
    shown.sort_unstable();
    shown.dedup();
    let mut near: Vec<u32> = shown.iter().flat_map(|&i| blend.near(i as usize).iter().copied()).collect();
    near.sort_unstable();
    near.dedup();
    let rivers = ways(&c, &blend, &near);
    let surface = levels(w, samples, &blend, &off, &f.depth, &near);
    let pixels = across * down;
    let mut out = Vec::with_capacity(8 + 24 * rivers.len() + pixels * (8 + LAYERS.len()));
    out.extend((cover_melt(w) as f32).to_le_bytes());
    out.extend((rivers.len() as u32).to_le_bytes());
    out.extend(rivers.iter().flatten().flat_map(|v| v.to_le_bytes()));
    let (mut ground, mut water, mut planes) = (Vec::with_capacity(pixels), Vec::with_capacity(pixels), vec![0u8; pixels * LAYERS.len()]);
    for p in 0..pixels {
        let (px, py) = at(p);
        let b = blend.at(cell[p] as usize, px, py);
        let z = heights[p] + b.mix(&off);
        let wet = b.share(|i| f.depth[i] >= STANDING_M);
        let level = if wet > 0.0 { b.mix(&surface) - (1.0 - wet) * SHORE_M } else { f64::NAN };
        ground.push(z as f32);
        water.push(level as f32);
        for (plane, layer) in planes.chunks_exact_mut(pixels).zip(&bytes) {
            plane[p] = b.mix(layer).round() as u8;
        }
        planes[t_layer * pixels + p] = quantise(&LAYERS[t_layer], b.mix(&f.t) - b.mix(&cools) * (z - b.mix(&f.z)));
    }
    out.extend(ground.iter().chain(&water).flat_map(|v| v.to_le_bytes()));
    out.extend(planes);
    out
}

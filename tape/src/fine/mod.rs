//! A window of the map finer than its cells, for a shell to draw (handover 10.3): at each pixel the height of the ground
//! from the terrain below the cells (`heights`) set on the cells' own heights, the level of the liquid its own cell
//! holds standing, and every layer, each a smooth field of the cells' values (`pyramid`); and the way each cell's liquid runs.
//! It reads the world only.
//!
//! The ground of a pixel is the field of the cells' heights (erosion and all) with the terrain's own shape finer than
//! the field laid on it: the terrain there less the field of its means over the cells. A cell split from a coarser one
//! holds what its parent held, so each is drawn at the scale of the cell that holds it (`pyramid`): the ground keeps
//! the mean height over each node of the coarsest cells' depth and each cell's own departure from its parent, and has
//! no steps at the cells'
//! edges, even where the cells beside each other hold heights that differ by far more than the terrain does there; a
//! pixel's ground does not depend on the window.

mod heights;
mod inside;
mod pyramid;

pub use heights::Samples;

use crate::layers::{cover_melt, quantise, Ctx, LAYERS, STANDING_M};
use pcore::World;
use pyramid::{Pixels, Pyramid};

/// The level of the liquid each cell holds standing, m, NaN for one that holds none: the liquid over the first
/// `STANDING_M` on the mean (that much may run, as a river), on the cell's points (`Samples::points`: the terrain there
/// less the terrain's mean over the cell, `terrain`, on the cell's height `z`), at the level it stands at when it fills
/// the lowest of them first (`flow::level_of`). So the liquid drawn lies only on the cell's ground below it, holds that
/// liquid, and its shore moves with the level as the level moves, from a puddle in its lowest point on.
fn levels(w: &World, s: &mut Samples, (z, terrain): (&[f64], &[f64]), depth: &[f64]) -> Vec<f64> {
    (0..z.len())
        .map(|i| {
            if depth[i] <= STANDING_M {
                return f64::NAN;
            }
            let mut floors: Vec<(f64, f64)> = s.points(w, w.map.leaves()[i]).iter().map(|g| (g - terrain[i] + z[i], 1.0)).collect();
            let vol = (depth[i] - STANDING_M) * floors.len() as f64;
            pcore::flow::level_of(&mut floors, vol)
        })
        .collect()
}

/// A segment of a river's way: the centres of a cell and of the cell its liquid runs to (column, row, in squares), its
/// liquid flux, mol/s, and its catchment (`World::catchment`).
type Way = [f32; 6];

/// The way each of `cells` whose liquid runs (some flux, none standing) sends it: to the cell the steepest way down its
/// head takes the most of (`Level::steepest`, the liquid law's way), if any.
fn ways(c: &Ctx, cells: &[u32]) -> Vec<Way> {
    let (w, f, level) = (c.w, &c.f, c.w.mesh.cells());
    let head = |i: usize| f.z[i] + f.depth[i];
    let centre = |i: usize| {
        let [x0, y0, x1, y1] = w.map.bounds(i);
        [(x0 + x1) / 2.0, (y0 + y1) / 2.0]
    };
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
        let ([x0, y0], [mut x1, y1]) = (centre(i), centre(j));
        if w.map.wrap {
            x1 -= w.map.w as f64 * ((x1 - x0) / w.map.w as f64).round();
        }
        out.push([x0, y0, x1, y1, flux, w.catchment(i)].map(|v| v as f32));
    }
    out
}

/// The cells within their own side of the window from column `x0` and row `y0` to `x1` and `y1`, in squares (round a
/// globe's rows).
fn about(w: &World, [x0, y0, x1, y1]: [f64; 4]) -> Vec<u32> {
    let wide = w.map.w as f64;
    (0..w.mesh.cells().n())
        .filter(|&i| {
            let [a, b, c, d] = w.map.bounds(i);
            let s = c - a;
            let shift = if w.map.wrap { wide * (((a + c) - (x0 + x1)) / 2.0 / wide).round() } else { 0.0 };
            a - s - shift < x1 && c + s - shift > x0 && b - s < y1 && d + s > y0
        })
        .map(|i| i as u32)
        .collect()
}

/// A window of the map finer than its cells: the window from column `x` and row `y`, `wide` x `high` squares, on its
/// edges' leaves of `depth` (no shallower than a square), a pixel each. Little-endian, in order: the melting point of the
/// frozen cover, K (f32; `cover_melt`); the count of river ways (u32) and each (`Way`, 6 f32) of the cells within their
/// own side of the window; of each pixel row by row the height of the ground, m (f32), and the level of the liquid its
/// own cell holds standing (f32: `levels`; NaN where it holds none), so the shore drawn is where the ground drawn rises
/// above it; then every layer in table
/// order, a byte a pixel, the field of the cells' bytes (`pyramid`), the ground's temperature along the ground's height
/// at the rate the air cools with height (`World::air_drop`); then, from a multiple of 4 bytes, what lies inside each
/// pixel's own cell by the shares of its ground (`inside::extend`): its wetness and the built cover of each slot of its
/// cell, f32 planes.
pub fn fine(w: &World, samples: &mut Samples, [x, y, wide, high]: [f64; 4], depth: u8) -> Vec<u8> {
    assert!(depth >= w.map.square_depth(), "a pixel is no coarser than a square");
    let per = (1u64 << (depth - w.map.square_depth())) as f64;
    let (gx, gy) = ((x * per).round() as i64, (y * per).round() as i64);
    let (across, down) = (((wide * per).round() as usize).max(1), ((high * per).round() as usize).max(1));
    let heights = heights::window(w, samples, [gx, gy, gx + across as i64 - 1, gy + down as i64 - 1], depth);
    let n = w.mesh.cells().n();
    let pyramid = Pyramid::new(w, Pixels { x: gx, y: gy, across, down, depth });
    let c = Ctx::new(w);
    let f = &c.f;
    let terrain: Vec<f64> = (0..n).map(|i| samples.mean_of(w, w.map.leaves()[i])).collect();
    let (z, below) = (pyramid.field(&f.z), pyramid.exact(&terrain));
    let level = levels(w, samples, (&f.z, &terrain), &f.depth);
    let (t, cools) = (pyramid.field(&f.t), pyramid.field(&(0..n).map(|i| w.air_drop(i).unwrap_or(0.0)).collect::<Vec<f64>>()));
    let rivers = ways(&c, &about(w, [x, y, x + wide, y + high]));
    let pixels = across * down;
    let mut out = Vec::with_capacity(8 + 24 * rivers.len() + pixels * (8 + LAYERS.len()));
    out.extend((cover_melt(w) as f32).to_le_bytes());
    out.extend((rivers.len() as u32).to_le_bytes());
    out.extend(rivers.iter().flatten().flat_map(|v| v.to_le_bytes()));
    let (tree, at) = (w.mesh.tree(), |p: usize| (((gx + (p % across) as i64) as f64 + 0.5) / per, ((gy + (p / across) as i64) as f64 + 0.5) / per));
    let cell: Vec<u32> = (0..pixels).map(at).map(|(px, py)| tree.cell_at(w.map.point_at(px, py)) as u32).collect();
    let ground: Vec<f64> = (0..pixels).map(|p| z[p] + heights[p] - below[p]).collect();
    let water = cell.iter().map(|&i| level[i as usize]);
    out.extend(ground.iter().map(|&g| g as f32).chain(water.map(|v| v as f32)).flat_map(|v| v.to_le_bytes()));
    for l in LAYERS.iter() {
        let plane: Vec<u8> = if l.name == "t_ground" {
            (0..pixels).map(|p| quantise(l, t[p] - cools[p] * (ground[p] - z[p]))).collect()
        } else {
            pyramid.field(&(0..n).map(|i| quantise(l, (l.val)(&c, i)) as f64).collect::<Vec<f64>>()).iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect()
        };
        out.extend(plane);
    }
    inside::extend(&mut out, w, samples, ([gx, gy], [across, down], depth), &cell);
    out
}

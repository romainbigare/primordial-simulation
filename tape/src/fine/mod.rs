//! A window of the map finer than its cells, for a shell to draw (handover 10.3): at each pixel the height of the ground,
//! the level of the liquid its own cell holds standing, and every layer; and the way each cell's liquid runs. It reads
//! the world only.
//!
//! The ground of a pixel is the terrain itself, drained (`heights`): the same samples whose means the cells' heights are
//! and over whose shape each cell's liquid stands (`flow::curve`), and finer than the samples their midpoint
//! continuation. So it has no steps at the cells' edges and depends neither on the window nor on the cells. Each cell's
//! standing liquid fills that ground inside the cell from its lowest point up (`levels`): a pixel is under liquid where
//! the ground drawn lies below its cell's level, so a shore lies where the ground rises, inside the cells. The layers
//! are the cells' values, each a smooth field of them (`pyramid`).

mod heights;
mod inside;
mod pyramid;

pub use heights::Samples;

use crate::layers::{cover_melt, quantise, Ctx, LAYERS, STANDING_M};
use pcore::inside::Means;
use pcore::space::tree::DEPTH_MOST;
use pcore::World;
use pyramid::{Pixels, Pyramid};

/// The cells a window shows: each one once, in order, with what each holds as the shares inside it read it
/// (`flow::means`; nothing before Flow first built its state), and of each pixel the number of its cell among them.
pub(super) struct Shown {
    pub cells: Vec<usize>,
    pub means: Vec<Means>,
    pub number: Vec<u32>,
}

impl Shown {
    /// The cells of world `w` that hold the pixels' centres, `cell` pixel by pixel.
    fn of(w: &World, cell: Vec<usize>) -> Shown {
        let mut cells = cell.clone();
        cells.sort_unstable();
        cells.dedup();
        let means = pcore::flow::means(w, &cells).unwrap_or_else(|| vec![Means::default(); cells.len()]);
        let number = cell.iter().map(|i| cells.binary_search(i).expect("a cell shown") as u32).collect();
        Shown { cells, means, number }
    }
}

/// A cell's level is found on its leaves no more than this many depths below it (128 x 128 of them): finer, the ground
/// drawn departs from them by so little that the liquid drawn still holds the cell's to a few parts in a thousand.
const LEVEL_DEPTHS_MOST: u8 = 7;

/// The level of the liquid each cell a window of pixels of `depth` shows holds standing over its pores, m, NaN for one
/// that holds none (`Means::stands`): over the ground drawn inside the cell (`Samples::floors`) at the pixels' grain
/// (no finer than `LEVEL_DEPTHS_MOST` below the cell), the level that holds that liquid when it fills the lowest ground
/// first (`flow::level_of`), as the core's level does over the cell's own ground (`flow::levels`). So the liquid drawn
/// holds the cell's at any zoom, it stands in the lowest ground drawn and not in every small pit finer than the ground
/// it is found on, and its shore moves with the level, from a puddle in the cell's lowest point on.
fn levels(w: &World, s: &mut Samples, shown: &Shown, depth: u8) -> Vec<f64> {
    (shown.cells.iter().zip(&shown.means))
        .map(|(&i, m)| {
            if m.stands <= 0.0 {
                return f64::NAN;
            }
            let leaf = w.map.leaves()[i];
            let grain = depth.clamp(leaf.depth, leaf.depth + LEVEL_DEPTHS_MOST).min(DEPTH_MOST);
            pcore::flow::level_of(&mut s.floors(w, leaf, grain), m.stands)
        })
        .collect()
}

/// A segment of a river's way: the centres of a cell and of the cell its liquid runs to (column, row, in squares), its
/// liquid flux, mol/s, its catchment (`World::catchment`), and the width and depth of its channel, m (`World::channel`).
type Way = [f32; 8];

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
        let ch = w.channel(i, flux);
        out.push([x0, y0, x1, y1, flux, w.catchment(i), ch.width, ch.depth].map(|v| v as f32));
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
/// frozen cover, K (f32; `cover_melt`); the count of river ways (u32) and each (`Way`, 8 f32) of the cells within their
/// own side of the window; of each pixel row by row the height of the ground, the terrain's (`heights`), m (f32), and
/// the level of the liquid its own cell holds standing (f32: `levels`; NaN where it holds none), so the shore drawn is
/// where the ground drawn rises above it; then every layer in table order, a byte a pixel, the field of the cells' bytes
/// (`pyramid`), the ground's temperature along the ground's height over its cell's mean at the rate the air cools with
/// height (`World::air_drop`); then, from a multiple of 4 bytes, what lies inside each
/// pixel's own cell by the shares of its ground (`inside::extend`): its wetness and the built cover of each slot of its
/// cell, f32 planes.
pub fn fine(w: &World, samples: &mut Samples, [x, y, wide, high]: [f64; 4], depth: u8) -> Vec<u8> {
    assert!(depth >= w.map.square_depth(), "a pixel is no coarser than a square");
    let per = (1u64 << (depth - w.map.square_depth())) as f64;
    let (gx, gy) = ((x * per).round() as i64, (y * per).round() as i64);
    let (across, down) = (((wide * per).round() as usize).max(1), ((high * per).round() as usize).max(1));
    let ground = heights::window(w, samples, [gx, gy, gx + across as i64 - 1, gy + down as i64 - 1], depth);
    let n = w.mesh.cells().n();
    let pyramid = Pyramid::new(w, Pixels { x: gx, y: gy, across, down, depth });
    let c = Ctx::new(w);
    let f = &c.f;
    // The ground's temperature is its cell's at the cell's mean height, the terrain's mean over the cell.
    let terrain = pyramid.field(&(0..n).map(|i| samples.mean_of(w, w.map.leaves()[i])).collect::<Vec<f64>>());
    let (t, cools) = (pyramid.field(&f.t), pyramid.field(&(0..n).map(|i| w.air_drop(i).unwrap_or(0.0)).collect::<Vec<f64>>()));
    let rivers = ways(&c, &about(w, [x, y, x + wide, y + high]));
    let pixels = across * down;
    let mut out = Vec::with_capacity(8 + 32 * rivers.len() + pixels * (8 + LAYERS.len()));
    out.extend((cover_melt(w) as f32).to_le_bytes());
    out.extend((rivers.len() as u32).to_le_bytes());
    out.extend(rivers.iter().flatten().flat_map(|v| v.to_le_bytes()));
    let (tree, at) = (w.mesh.tree(), |p: usize| (((gx + (p % across) as i64) as f64 + 0.5) / per, ((gy + (p / across) as i64) as f64 + 0.5) / per));
    let shown = Shown::of(w, (0..pixels).map(at).map(|(px, py)| tree.cell_at(w.map.point_at(px, py))).collect());
    let level = levels(w, samples, &shown, depth);
    let water = shown.number.iter().map(|&k| level[k as usize]);
    out.extend(ground.iter().map(|&g| g as f32).chain(water.map(|v| v as f32)).flat_map(|v| v.to_le_bytes()));
    for l in LAYERS.iter() {
        let plane: Vec<u8> = if l.name == "t_ground" {
            (0..pixels).map(|p| quantise(l, t[p] - cools[p] * (ground[p] - terrain[p]))).collect()
        } else {
            pyramid.field(&(0..n).map(|i| quantise(l, (l.val)(&c, i)) as f64).collect::<Vec<f64>>()).iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect()
        };
        out.extend(plane);
    }
    inside::extend(&mut out, w, samples, ([gx, gy], [across, down], depth), &shown);
    out
}

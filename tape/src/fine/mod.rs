//! A window of the map finer than its cells, for a shell to draw (handover 10.3): at each pixel the height of the ground,
//! the level of the liquid its own cell holds standing, the channels its liquid runs in (`channels`), and every layer.
//! It reads the world only.
//!
//! The ground of a pixel is the terrain itself, drained (`heights`): the same ground whose means the cells' heights are
//! and over whose shape each cell's liquid stands (`flow::curve`), finer than the samples too (`space::below`). So it
//! has no steps at the cells' edges and depends neither on the window nor on the cells. Each cell's standing liquid
//! stands at the core's own level (`levels`): a pixel is under liquid where the ground drawn lies below its cell's
//! level, so a shore lies where the ground rises, inside the cells, and a body of liquid at one level is flat. The
//! layers are the cells' values, each a smooth field of them (`pyramid`).

mod channels;
mod heights;
mod inside;
mod pyramid;

pub use heights::Samples;

use crate::layers::{cover_melt, quantise, Ctx, LAYERS};
use pcore::inside::Means;
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

/// The level of the liquid each cell a window shows holds standing over its pores, m, NaN for one that holds none
/// (`Means::stands`): the core's own level over the cell's own ground (`flow::levels`), the one its liquid moves by,
/// laid on the ground drawn: as high over the terrain's mean in the cell as the core's stands over the cell's surface
/// (its lowest ground less the curve's lowest departure, `Ground` at the cell's `relief_scale`). So the liquid drawn
/// over the cell's own samples holds its own, a body of liquid at one level draws flat across its cells (to the
/// centimetres by which wear and settling moved their surfaces off the terrain), and its shore lies where the ground
/// drawn rises above it.
fn levels(w: &World, s: &mut Samples, shown: &Shown) -> Vec<f64> {
    let level = pcore::flow::levels(w, &shown.cells).unwrap_or_else(|| vec![[f64::NAN; 2]; shown.cells.len()]);
    (shown.cells.iter().zip(&shown.means).zip(level))
        .map(|((&i, m), [lowest, top])| {
            if m.stands <= 0.0 {
                return f64::NAN;
            }
            let g = s.ground(w, w.map.leaves()[i]);
            let mean = g.mean(&g.h);
            let low = g.h.iter().copied().fold(f64::INFINITY, f64::min) - mean;
            top - (lowest - w.relief_scale[i] * low) + mean
        })
        .collect()
}

/// A window of the map finer than its cells: the window from column `x` and row `y`, `wide` x `high` squares, on its
/// edges' leaves of `depth` (no shallower than a square), a pixel each. Little-endian, in order: the melting point of the
/// frozen cover, K (f32); of each pixel row by row the height of the ground, the terrain's (`heights`), m (f32), the level
/// of the liquid its own cell holds standing, the core's (f32: `levels`; NaN where it holds none), so the shore drawn is where the
/// ground drawn rises above it, the share of the pixel a channel covers, 0..1, and that channel's depth, m (two f32:
/// `channels`); then every layer in table order, a byte a pixel, the field of the cells' bytes
/// (`pyramid`), the ground's temperature along the ground's height over its cell's mean at the rate the air cools with
/// height (`World::air_drop`); then, from a multiple of 4 bytes, what lies inside each
/// pixel's own cell by the shares of its ground (`inside::extend`): its wetness and the built cover of each slot of its
/// cell, f32 planes.
pub fn fine(w: &World, samples: &mut Samples, [x, y, wide, high]: [f64; 4], depth: u8) -> Vec<u8> {
    assert!(depth >= w.map.square_depth(), "a pixel is no coarser than a square");
    let per = (1u64 << (depth - w.map.square_depth())) as f64;
    let (gx, gy) = ((x * per).round() as i64, (y * per).round() as i64);
    let (across, down) = (((wide * per).round() as usize).max(1), ((high * per).round() as usize).max(1));
    let ground = heights::window(w, [gx, gy, gx + across as i64 - 1, gy + down as i64 - 1], depth);
    let n = w.mesh.cells().n();
    let pyramid = Pyramid::new(w, Pixels { x: gx, y: gy, across, down, depth });
    let c = Ctx::new(w);
    let f = &c.f;
    // The ground's temperature is its cell's at the cell's mean height, the terrain's mean over the cell.
    let terrain = pyramid.field(&(0..n).map(|i| samples.mean_of(w, w.map.leaves()[i])).collect::<Vec<f64>>());
    let (t, cools) = (pyramid.field(&f.t), pyramid.field(&(0..n).map(|i| w.air_drop(i).unwrap_or(0.0)).collect::<Vec<f64>>()));
    let pixels = across * down;
    let mut out = Vec::with_capacity(4 + pixels * (16 + LAYERS.len()));
    out.extend((cover_melt(w) as f32).to_le_bytes());
    let (tree, at) = (w.mesh.tree(), |p: usize| (((gx + (p % across) as i64) as f64 + 0.5) / per, ((gy + (p / across) as i64) as f64 + 0.5) / per));
    let shown = Shown::of(w, (0..pixels).map(at).map(|(px, py)| tree.cell_at(w.map.point_at(px, py))).collect());
    let level = levels(w, samples, &shown);
    let water = shown.number.iter().map(|&k| level[k as usize]);
    let (share, deep) = channels::channels(w, &w.drained(), ([gx, gy], [across, down], depth));
    out.extend(ground.iter().map(|&g| g as f32).chain(water.map(|v| v as f32)).chain(share).chain(deep).flat_map(|v| v.to_le_bytes()));
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

//! A window of the map finer than its cells, for a shell to draw (handover 10.3): at each pixel the height of the ground,
//! the level of the liquid its own cell holds standing, the channels its liquid runs in (`channels`), and every layer.
//! It reads the world only.
//!
//! The ground of a pixel is the terrain itself, drained (`heights`): the same ground whose means the cells' heights are
//! and over whose shape each cell's liquid stands (`flow::curve`), finer than the samples too (`space::below`). So it
//! has no steps at the cells' edges and depends neither on the window nor on the cells. The liquid standing in the
//! cells that hold some stands at the core's own levels, evened over half a cell (`standing`): a pixel is under liquid
//! where the ground drawn lies below that level, so a shore lies where the ground rises, inside the cells, a body of
//! liquid at one level is flat, and the levels of a lake's cells meet without a step. The
//! layers are the cells' values, each a smooth field of them (`pyramid`); what a cell holds that the ground shares
//! over it (the liquid in its soil, its cover, its ground's warmth and ice) is a field across the cells that holds each
//! cell's mean (`keep`), shared inside by the general law the core shares it by (`inside`).

mod channels;
mod heights;
mod inside;
mod keep;
mod pyramid;

pub use heights::Samples;

use crate::layers::{cover_melt, quantise, Ctx, LAYERS};
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

/// The level of the liquid standing over each pixel, m, NaN where none stands: the levels of the cells that hold liquid
/// standing (the core's, `flow::levels`, the levels its liquid moves by) evened over such cells across their borders
/// (`Cells::even`; so within half a cell of one), where it lies over the ground drawn `ground`. The core holds the cells of one lake at
/// levels that differ by centimetres to decimetres, as its liquid's pass leaves them: the surface drawn goes from one to
/// the next over the width of the cells, not in a step at each border. A body at one level (the sea's) is drawn flat.
fn standing(w: &World, shown: &Shown, cells: &keep::Cells, ground: &[f64]) -> Vec<f64> {
    let own: Vec<f64> = pcore::flow::levels(w, &shown.cells).map_or_else(|| vec![f64::NAN; shown.cells.len()], |l| l.iter().map(|l| l[1]).collect());
    let wet = |c: usize| shown.means[c].stands > 0.0;
    let even = cells.even(&own, wet);
    (0..ground.len()).map(|p| if even[p] > ground[p] { even[p] } else { f64::NAN }).collect()
}

/// A window of the map finer than its cells: the window from column `x` and row `y`, `wide` x `high` squares, on its
/// edges' leaves of `depth` (no shallower than a square), a pixel each. Little-endian, in order: the melting point of the
/// frozen cover, K (f32); of each pixel row by row the height of the ground, the terrain's (`heights`), m (f32), the level
/// of the liquid standing over it (f32: `standing`; NaN where none stands), so the shore drawn is where the
/// ground drawn rises above it, the share of the pixel a channel covers, 0..1, and that channel's depth, m (two f32:
/// `channels`); then every layer in table order, a byte a pixel, the field of the cells' bytes
/// (`pyramid`), but the ground's temperature and its ice, fields that hold each cell's mean (`keep`), the temperature
/// along the ground's height over its cell's mean at the rate the air cools with height (`World::air_drop`); then, from
/// a multiple of 4 bytes, the wetness of each pixel's ground and the built cover of each slot of its cell, f32 planes
/// (`inside::extend`).
pub fn fine(w: &World, samples: &mut Samples, [x, y, wide, high]: [f64; 4], depth: u8) -> Vec<u8> {
    assert!(depth >= w.map.square_depth(), "a pixel is no coarser than a square");
    let per = (1u64 << (depth - w.map.square_depth())) as f64;
    let (gx, gy) = ((x * per).round() as i64, (y * per).round() as i64);
    let (across, down) = (((wide * per).round() as usize).max(1), ((high * per).round() as usize).max(1));
    let n = w.mesh.cells().n();
    let win = inside::Window { at: [gx, gy], size: [across, down], depth };
    let (ground, twi) = samples.ground(w, &win);
    let (ground, twi) = (ground.to_vec(), twi.to_vec());
    let pyramid = Pyramid::new(w, Pixels { x: gx, y: gy, across, down, depth });
    let c = Ctx::new(w);
    let f = &c.f;
    // The ground's temperature is its cell's at the cell's mean height, the terrain's mean over the cell.
    let terrain = pyramid.field(&(0..n).map(|i| samples.mean_of(w, w.map.leaves()[i])).collect::<Vec<f64>>());
    let cools = pyramid.field(&(0..n).map(|i| w.air_drop(i).unwrap_or(0.0)).collect::<Vec<f64>>());
    let pixels = across * down;
    let mut out = Vec::with_capacity(4 + pixels * (16 + LAYERS.len()));
    out.extend((cover_melt(w) as f32).to_le_bytes());
    let (tree, at) = (w.mesh.tree(), |p: usize| (((gx + (p % across) as i64) as f64 + 0.5) / per, ((gy + (p / across) as i64) as f64 + 0.5) / per));
    let shown = Shown::of(w, (0..pixels).map(at).map(|(px, py)| tree.cell_at(w.map.point_at(px, py))).collect());
    let side: Vec<f64> = shown.cells.iter().map(|&i| (w.map.leaves()[i].side() >> (DEPTH_MOST - depth)) as f64).collect();
    let cells = keep::Cells::new(across, down, (&shown.number, &side), &win.rows(w));
    let level = standing(w, &shown, &cells, &ground);
    let water = level.iter().copied();
    let (share, deep) = channels::channels(w, &w.drained(), ([gx, gy], [across, down], depth));
    out.extend(ground.iter().map(|&g| g as f32).chain(water.map(|v| v as f32)).chain(share).chain(deep).flat_map(|v| v.to_le_bytes()));
    let held = |v: &[f64], law: &dyn Fn(usize, f64) -> f64, how: keep::Mean| {
        let target: Vec<f64> = shown.cells.iter().map(|&i| v[i]).collect();
        let smooth = cells.even(&target, |_| true);
        cells.keep((0..pixels).map(|p| law(p, smooth[p])).collect(), &target, how)
    };
    for l in LAYERS.iter() {
        let plane: Vec<u8> = if l.name == "t_ground" {
            held(&f.t, &|p, u| u - cools[p] * (ground[p] - terrain[p]), keep::Mean::Shifted).iter().map(|&t| quantise(l, t)).collect()
        } else if l.name == "ice" {
            held(&f.ice, &|_, u| u.max(0.0), keep::Mean::Scaled).iter().map(|&v| quantise(l, v)).collect()
        } else {
            pyramid.field(&(0..n).map(|i| quantise(l, (l.val)(&c, i)) as f64).collect::<Vec<f64>>()).iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect()
        };
        out.extend(plane);
    }
    let wet = inside::wetness(w, (&cells, &shown), (&ground, &twi), &level);
    inside::extend(&mut out, w, (&cells, &shown), &wet);
    out
}

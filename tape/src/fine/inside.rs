//! What lies inside the cells of a window, pixel by pixel, by the shares the core gives each cell's means over its own
//! ground (`pcore::inside`): the same shares a split gives its quarters, so what a shell draws inside a cell before a
//! zoom is what the world goes on with. Each pixel reads its own cell (the one that holds its centre), not a blend.

use super::heights::{leaf, Samples};
use pcore::cover::{Parts, MOIST, SLOTS};
use pcore::inside::{Ground, Inside, Means};
use pcore::space::tree::BLOCK;
use pcore::World;

/// The fields of one cell: its ground, how wet each sample is, its snow per sample, m, and the built cover of each slot
/// it holds per sample, kg/m², in the order of its slots (`tape::things`).
struct Cell {
    ground: Ground,
    wet: Vec<f64>,
    snow: Vec<f64>,
    cover: Vec<Vec<f64>>,
}

impl Cell {
    fn of(w: &World, samples: &mut Samples, i: usize, m: Means) -> Cell {
        let ground = samples.ground(w, w.map.leaves()[i]).clone();
        let ph = &w.cfg.phys;
        let inside = Inside::of(&ground, ph, w.inflow(i), m);
        let c = w.life.cover(i);
        let cover = Parts::of(&w.chem).map_or(Vec::new(), |p| {
            let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area()[i]);
            c.held().map(|x| inside.kind(&ground, ph, c.genes[x][MOIST]).iter().map(|s| c.built(&mm, x) / area * s).collect()).collect()
        });
        let snow = inside.snow.iter().map(|s| m.snow * s).collect();
        Cell { ground, wet: inside.wet, snow, cover }
    }
}

/// Of each pixel of the window from leaf column `gx` and row `gy`, `across` x `down` leaves of `depth`, whose cells are
/// `cell`, appended to `out` after zero bytes up to a multiple of 4 of its length: how wet its ground is, 0..1 (f32;
/// `Physics::wetness` of the liquid in its pores and standing there); its snow, m (f32); then `SLOTS` planes, plane `k` the
/// built cover per m² of the `k`-th slot its cell holds (in the order `tape::things` lists them), kg/m² (f32; 0 where
/// its cell holds fewer). Each the mean of its cell's means times their shares (`pcore::inside`) over the samples the
/// pixel covers (`Ground::over`), so a pixel finer than the samples reads the one that holds it, and the pixels of a
/// cell keep its means.
pub(super) fn extend(out: &mut Vec<u8>, w: &World, samples: &mut Samples, ([gx, gy], [across, down], depth): ([i64; 2], [usize; 2], u8), cell: &[u32]) {
    let mut shown: Vec<usize> = cell.iter().map(|&i| i as usize).collect();
    shown.sort_unstable();
    shown.dedup();
    let means = pcore::flow::means(w, &shown).unwrap_or_else(|| vec![Means::default(); shown.len()]);
    let cells: Vec<Cell> = shown.iter().zip(means).map(|(&i, m)| Cell::of(w, samples, i, m)).collect();
    let pixels = across * down;
    let mut planes = vec![0.0f32; pixels * (2 + SLOTS)];
    let side = BLOCK >> depth;
    for p in 0..pixels {
        let l = leaf(depth, gx + (p % across) as i64, gy + (p / across) as i64);
        let c = &cells[shown.binary_search(&(cell[p] as usize)).expect("a cell shown")];
        let span = [l.x, l.y, l.x + side, l.y + side];
        planes[p] = c.ground.over(span, &c.wet) as f32;
        planes[pixels + p] = c.ground.over(span, &c.snow) as f32;
        for (k, slot) in c.cover.iter().enumerate() {
            planes[(2 + k) * pixels + p] = c.ground.over(span, slot) as f32;
        }
    }
    out.resize(out.len().next_multiple_of(4), 0);
    out.extend(planes.iter().flat_map(|v| v.to_le_bytes()));
}

//! What lies inside the cells of a window, pixel by pixel, by the shares the core gives each cell's means over its own
//! ground (`pcore::inside`): the same shares a split gives its quarters, so what a shell draws inside a cell before a
//! zoom is what the world goes on with. Each pixel reads its own cell (the one that holds its centre), not a blend.

use super::heights::{leaf, Samples};
use pcore::cover::{Parts, SLOTS};
use pcore::inside::{Ground, Inside, Means};
use pcore::space::tree::BLOCK;
use pcore::World;

/// The fields of one cell: its ground, how wet each sample is, the built cover of each slot it holds per sample,
/// kg/m², in the order of its slots (`tape::things`), and of each of those slots its walls and its capture area per kg
/// of its built cover (kg/kg, m²/kg).
struct Cell {
    ground: Ground,
    wet: Vec<f64>,
    cover: Vec<Vec<f64>>,
    make: Vec<[f64; 2]>,
}

impl Cell {
    fn of(w: &World, samples: &mut Samples, i: usize, m: Means) -> Cell {
        let ground = samples.ground(w, w.map.leaves()[i]).clone();
        let ph = &w.cfg.phys;
        let inside = Inside::of(&ground, ph, w.inflow(i), m);
        let c = w.life.cover(i);
        let (cover, make) = Parts::of(&w.chem).map_or((Vec::new(), Vec::new()), |p| {
            let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area()[i]);
            let cover = c.held().map(|x| inside.kind(&ground, ph, &c.genes[x]).iter().map(|s| c.built(&mm, x) / area * s).collect()).collect();
            let per_kg = |x: usize, v: f64| v / c.built(&mm, x).max(1e-300);
            (cover, c.held().map(|x| [per_kg(x, c.walls(&mm, x)), per_kg(x, c.leaf(&mm, x))]).collect())
        });
        Cell { ground, wet: inside.wet, cover, make }
    }
}

/// Of each pixel of the window from leaf column `gx` and row `gy`, `across` x `down` leaves of `depth`, whose cells are
/// `cell`, appended to `out` after zero bytes up to a multiple of 4 of its length: how wet its ground is, 0..1 (f32;
/// `Physics::wetness` of the liquid in its pores and standing there); then `SLOTS` planes, plane `k` the
/// built cover per m² of the `k`-th slot its cell holds (in the order `tape::things` lists them), kg/m² (f32; 0 where
/// its cell holds fewer). Each the mean of its cell's means times their shares (`pcore::inside`) over the samples the
/// pixel covers (`Ground::over`), so a pixel finer than the samples reads the one that holds it, and the pixels of a
/// cell keep its means. Then of each pixel the number of its cell among the window's cells (u32), and the count of
/// those cells (u32) and of each of them, for each of its `SLOTS` slots, its walls and its capture area per kg of its
/// built cover (kg/kg, m²/kg; f32, 0 for a slot it does not hold), so a shell sees what each slot builds where it
/// stands.
pub(super) fn extend(out: &mut Vec<u8>, w: &World, samples: &mut Samples, ([gx, gy], [across, down], depth): ([i64; 2], [usize; 2], u8), cell: &[u32]) {
    let mut shown: Vec<usize> = cell.iter().map(|&i| i as usize).collect();
    shown.sort_unstable();
    shown.dedup();
    let means = pcore::flow::means(w, &shown).unwrap_or_else(|| vec![Means::default(); shown.len()]);
    let cells: Vec<Cell> = shown.iter().zip(means).map(|(&i, m)| Cell::of(w, samples, i, m)).collect();
    let pixels = across * down;
    let mut planes = vec![0.0f32; pixels * (1 + SLOTS)];
    let side = BLOCK >> depth;
    let number: Vec<u32> = cell.iter().map(|&i| shown.binary_search(&(i as usize)).expect("a cell shown") as u32).collect();
    for p in 0..pixels {
        let l = leaf(depth, gx + (p % across) as i64, gy + (p / across) as i64);
        let c = &cells[number[p] as usize];
        let span = [l.x, l.y, l.x + side, l.y + side];
        planes[p] = c.ground.over(span, &c.wet) as f32;
        for (k, slot) in c.cover.iter().enumerate() {
            planes[(1 + k) * pixels + p] = c.ground.over(span, slot) as f32;
        }
    }
    out.resize(out.len().next_multiple_of(4), 0);
    out.extend(planes.iter().flat_map(|v| v.to_le_bytes()));
    out.extend(number.iter().chain([&(cells.len() as u32)]).flat_map(|v| v.to_le_bytes()));
    for c in &cells {
        let make = (0..SLOTS).map(|k| c.make.get(k).copied().unwrap_or([0.0; 2]));
        out.extend(make.flatten().flat_map(|v| (v as f32).to_le_bytes()));
    }
}

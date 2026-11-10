//! The cells' table a shell draws the state of the ground from (laws.md 10): a row for each square of the map, the numbers
//! of the cell that holds its centre that the laws inside a cell read, so a shader blends them between the squares'
//! centres and applies the laws at each pixel of the ground's tiles (`ground`). It is made with each frame, from the
//! frame's own context (`Ctx`), and holds nothing a picture needs to make.

use crate::ground::sample_index;
use crate::layers::{cover_melt, frame, pixels, Ctx};
use pcore::cover::{dec, Parts, MOIST};
use pcore::inside::{soil_level, Means};
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::World;

/// Numbers a square's row holds (`Table::frame`).
pub const ROW: usize = 12;

/// What a shell keeps of a world's cells, which do not change: the cell of each square; of each cell the terrain's mean
/// height over it, m, the wetness index of the terrain's samples inside it with their shares of its area
/// (`ground::sample_index`), and its samples; and the area that drains through each sample, m² (`Samples::up`).
pub struct Table {
    square: Vec<usize>,
    z: Vec<f64>,
    inside: Vec<(Vec<f64>, Vec<f64>)>,
    samples: Vec<Vec<u32>>,
    up: Vec<f64>,
}

impl Table {
    pub fn new(w: &World) -> Table {
        let (map, n) = (&w.map, w.mesh.cells().n());
        let square = pixels(w);
        let lat = (w.cfg.lat_south, w.cfg.lat_north);
        let means = w.terrain.heights(w.terrain.seed, map, lat, [0, 0, map.w as i64 - 1, map.h as i64 - 1], map.square_depth());
        let (mut z, mut count) = (vec![0.0f64; n], vec![0.0f64; n]);
        for (s, &i) in square.iter().enumerate() {
            (z[i], count[i]) = (z[i] + means[s], count[i] + 1.0);
        }
        z.iter_mut().zip(&count).for_each(|(z, c)| *z /= c.max(1.0));
        let s = w.terrain_samples();
        let twi = sample_index(map, &s);
        let per = (1u64 << (SAMPLE_DEPTH - map.square_depth())) as f64;
        let tree = w.mesh.tree();
        let cell = |k: usize| tree.cell_at(map.point_at(((k % s.across) as f64 + 0.5) / per, ((k / s.across) as f64 + 0.5) / per));
        let of: Vec<usize> = (0..s.h.len()).map(cell).collect();
        let (mut inside, mut samples) = (vec![(Vec::new(), Vec::new()); n], vec![Vec::new(); n]);
        for (k, &i) in of.iter().enumerate() {
            inside[i].0.push(twi[k]);
            samples[i].push(k as u32);
        }
        for (t, a) in inside.iter_mut() {
            *a = vec![1.0 / t.len().max(1) as f64; t.len()];
        }
        Table { square, z, inside, samples, up: s.up.clone() }
    }

    /// The present state of world `w` as one tape frame, as a tape would keep it (`frame`), and its table: the melting point of the frozen
    /// cover, K (`cover_melt`), then `ROW` f32 numbers for each square, row by row (`row`), then the level of each pond
    /// of the terrain's samples, m (`World::pond_levels`: NaN where no body holds it), which a shell draws each pond's
    /// liquid flat at, over the ground that lies under it.
    pub fn frame(&self, w: &World) -> (Vec<u8>, Vec<f32>) {
        let c = Ctx::new(w);
        let mut bytes = Vec::with_capacity(w.map.n() * crate::LAYERS.len());
        frame(&c, &mut bytes);
        let n = w.mesh.cells().n();
        let all: Vec<usize> = (0..n).collect();
        let means = pcore::flow::means(w, &all);
        let levels = pcore::flow::levels(w, &all);
        let rows: Vec<[f64; ROW]> = (0..n).map(|i| self.row(&c, i, means.as_ref().map(|m| m[i]), levels.as_ref().map(|l| l[i]))).collect();
        let mut table = vec![cover_melt(w) as f32];
        table.extend(self.square.iter().flat_map(|&i| rows[i].map(|v| v as f32)));
        table.extend(w.pond_levels().iter().map(|&l| l as f32));
        (bytes, table)
    }

    /// The row of cell `i` of the world of context `c`, with what it holds as the shares read it (`flow::means`) and its
    /// lowest ground and level (`flow::levels`), none before Flow first built its state (then its solid surface and the
    /// depth of its liquid): the terrain's mean height over it, m; its ground's temperature, K, and how fast the air
    /// cools with height there, K/m (`World::air_drop`); its ice, m; the level its standing liquid reaches and its lowest
    /// ground, m, and the depth that stands, m; the liquid in its pores and what they hold full, m, and the level of the
    /// wetness index its soil's liquid departs about (`inside::soil_level`; 0 where it lies evenly); the volume of
    /// liquid that runs through its main channel per m² drained there, m/s (the discharge of the sample of it through
    /// which the most runs, `World::discharge`, over the area that drains through that sample, at the mol per m³ of its
    /// liquid, `World::liquid_in`), by which a shell sizes each channel by the area that drains through it; and how wet
    /// its heaviest kind of cover needs its ground (`cover::activity`; 0: none).
    fn row(&self, c: &Ctx, i: usize, m: Option<Means>, level: Option<[f64; 2]>) -> [f64; ROW] {
        let w = c.w;
        let (z, depth) = (c.f.z[i], c.f.depth[i]);
        let m = m.unwrap_or(Means { pore: 0.0, pores: 0.0, stands: depth });
        let [low, top] = level.unwrap_or([z, z + depth]);
        let (twi, area) = &self.inside[i];
        let soil = soil_level(&w.cfg.phys, twi, area, m).unwrap_or(0.0);
        let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
        let main = self.samples[i].iter().map(|&k| k as usize).max_by(|&a, &b| w.discharge[a].total_cmp(&w.discharge[b]));
        let q = main.filter(|_| per_m3 > 0.0).map_or(0.0, |k| w.discharge[k] as f64 / self.up[k] / per_m3);
        let cover = w.life.cover(i);
        let heaviest = Parts::of(&w.chem).and_then(|p| {
            let mm = p.mm(&w.chem);
            cover.held().max_by(|&a, &b| cover.built(&mm, a).total_cmp(&cover.built(&mm, b)))
        });
        let need = heaviest.map_or(0.0, |x| dec(cover.genes[x][MOIST]));
        let cools = w.air_drop(i).unwrap_or(0.0);
        [self.z[i], c.f.t[i], cools, c.f.ice[i], top, low, m.stands, m.pore, m.pores, soil, q, need]
    }
}

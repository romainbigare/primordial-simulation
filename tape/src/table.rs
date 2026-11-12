//! The cells' table a shell draws the state of the ground from (laws.md 10): a row for each square of the map, the numbers
//! of the cell that holds its centre that the laws inside a cell read, so a shader blends them between the squares'
//! centres and applies the laws at each pixel of the ground's tiles (`ground`). It is made with each frame, from the
//! frame's own context (`Ctx`), and holds nothing a picture needs to make.

use crate::ground::sample_index;
use crate::layers::{cover_melt, frame, pixels, Ctx};
use pcore::inside::{soil_level, Means};
use pcore::space::network::NONE;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::World;

/// Numbers a square's row holds (`Table::frame`).
pub const ROW: usize = 10;
/// A body that holds less liquid than this on its mean over its surface, m, is drawn dry: a film over a flat basin,
/// soaked ground rather than water.
const DRY_M: f64 = 0.1;

/// What a shell keeps of a world's cells, which do not change: the cell of each square; of each cell the terrain's mean
/// height over it, m, and the wetness index of the terrain's samples inside it with their shares of its area
/// (`ground::sample_index`); and the cell of each of the terrain's samples.
pub struct Table {
    square: Vec<usize>,
    z: Vec<f64>,
    inside: Vec<(Vec<f64>, Vec<f64>)>,
    cell: Vec<u32>,
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
        let mut inside = vec![(Vec::new(), Vec::new()); n];
        for (k, &i) in of.iter().enumerate() {
            inside[i].0.push(twi[k]);
        }
        for (t, a) in inside.iter_mut() {
            *a = vec![1.0 / t.len().max(1) as f64; t.len()];
        }
        Table { square, z, inside, cell: of.iter().map(|&i| i as u32).collect() }
    }

    /// The volume of liquid that runs through each of the terrain's samples, m³/s, row by row: its discharge
    /// (`World::discharge`) at the mol per m³ of the liquid its cell's ground holds (`World::liquid_in`); none where that
    /// cell holds no liquid. A shell sizes the channel of each sample by it (`Physics::channel`). A sample of a pond
    /// carries no less than its main way upstream brings: a river runs on along a body's floor to its
    /// lowest sample, and a shell draws that run where the body is drawn dry or its shore lies farther than the
    /// samples' (under the body's liquid it does not show).
    pub fn discharge(&self, w: &World) -> Vec<f32> {
        let per_m3: Vec<f64> = (0..w.mesh.cells().n()).map(|i| w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1)).collect();
        let per = |k: usize| per_m3[self.cell[k] as usize];
        let mut out: Vec<f32> = w.discharge.iter().enumerate().map(|(k, &q)| if per(k) > 0.0 { (q as f64 / per(k)) as f32 } else { 0.0 }).collect();
        let s = w.terrain_samples();
        // In the flood's order from the last sample reached: a sample comes before the one its liquid runs to.
        for &k in s.order.iter().rev() {
            let k = k as usize;
            if s.pond[k] != NONE && s.main[k] != NONE {
                out[k] = out[k].max(out[s.main[k] as usize]);
            }
        }
        out
    }

    /// The present state of world `w` as one tape frame, as a tape would keep it (`frame`), and its table: the melting point of the frozen
    /// cover, K (`cover_melt`), then `ROW` f32 numbers for each square, row by row (`row`), then the kinds of cover of
    /// each square, row by row (`stands::rows`, `KINDS` × `KIND_ROW` f32), then the level of each pond of the terrain's
    /// samples, m (`World::pond_levels`: NaN where no body holds it, or where its body holds less than `DRY_M` of liquid on
    /// its mean: dry ground), which a shell draws each pond's liquid flat at, over the ground that lies under it.
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
        let kinds: Vec<_> = (0..n).map(|i| crate::stands::rows(w, i)).collect();
        table.extend(self.square.iter().flat_map(|&i| kinds[i].map(|v| v as f32)));
        let mut ponds = w.pond_levels();
        for p in w.pools().iter().filter(|p| p.volume <= DRY_M * p.area) {
            ponds[p.pond] = f64::NAN;
        }
        table.extend(ponds.iter().map(|&l| l as f32));
        (bytes, table)
    }

    /// The row of cell `i` of the world of context `c`, with what it holds as the shares read it (`flow::means`) and its
    /// lowest ground and level (`flow::levels`), none before Flow first built its state (then its solid surface and the
    /// depth of its liquid): the terrain's mean height over it, m; its ground's temperature, K, and how fast the air
    /// cools with height there, K/m (`World::air_drop`); its ice, m; the level its standing liquid reaches and its lowest
    /// ground, m, and the depth that stands, m; the liquid in its pores and what they hold full, m, and the level of the
    /// wetness index its soil's liquid departs about (`inside::soil_level`; 0 where it lies evenly).
    fn row(&self, c: &Ctx, i: usize, m: Option<Means>, level: Option<[f64; 2]>) -> [f64; ROW] {
        let w = c.w;
        let (z, depth) = (c.f.z[i], c.f.depth[i]);
        let m = m.unwrap_or(Means { pore: 0.0, pores: 0.0, stands: depth });
        let [low, top] = level.unwrap_or([z, z + depth]);
        let (twi, area) = &self.inside[i];
        let soil = soil_level(&w.cfg.phys, twi, area, m).unwrap_or(0.0);
        let cools = w.air_drop(i).unwrap_or(0.0);
        [self.z[i], c.f.t[i], cools, c.f.ice[i], top, low, m.stands, m.pore, m.pores, soil]
    }
}

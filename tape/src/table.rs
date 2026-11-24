//! The cells' table a shell draws the state of the ground from (laws.md 10): a row for each square of the map, the numbers
//! of the cell that holds its centre that the laws inside a cell read, so a shader blends them between the squares'
//! centres and applies the laws at each pixel of the ground's tiles (`ground`). It is made with each frame, from the
//! frame's own context (`Ctx`), and holds nothing a picture needs to make.

use crate::ground::Dry;
use crate::layers::{cover_melt, frame, pixels, Ctx};
use pcore::inside::Means;
use pcore::space::network::NONE;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::World;

/// Numbers a square's row holds (`Table::frame`).
pub const ROW: usize = 11;
/// A body that holds less liquid than this on its mean over its surface, m, is drawn dry: a film over a flat basin,
/// soaked ground rather than water.
const DRY_M: f64 = 0.1;

/// What a shell keeps of a world's cells, found again when the world builds its samples anew (`Relief::version`,
/// `update`): the cell of each square; of each cell the terrain's mean height over it as built, m; the cell of each of
/// the terrain's samples; and whether the shore a shell draws leaves each sample out of its pond's liquid
/// (`ground::Dry`).
pub struct Table {
    square: Vec<usize>,
    z: Vec<f64>,
    cell: Vec<u32>,
    dry: Dry,
}

/// Of each of world `w`'s `n` cells, the terrain's mean height over the squares `square` it holds (the cell of each), m.
fn means(w: &World, square: &[usize], n: usize) -> Vec<f64> {
    let (map, lat) = (&w.map, (w.cfg.lat_south, w.cfg.lat_north));
    let means = w.terrain.heights_on(&w.terrain_samples(), map, lat, [0, 0, map.w as i64 - 1, map.h as i64 - 1], map.square_depth());
    let (mut z, mut count) = (vec![0.0f64; n], vec![0.0f64; n]);
    for (s, &i) in square.iter().enumerate() {
        (z[i], count[i]) = (z[i] + means[s], count[i] + 1.0);
    }
    z.iter_mut().zip(&count).for_each(|(z, c)| *z /= c.max(1.0));
    z
}

impl Table {
    pub fn new(w: &World) -> Table {
        let (map, n) = (&w.map, w.mesh.cells().n());
        let square = pixels(w);
        let s = w.terrain_samples();
        let z = means(w, &square, n);
        let per = (1u64 << (SAMPLE_DEPTH - map.square_depth())) as f64;
        let tree = w.mesh.tree();
        let cell = |k: usize| tree.cell_at(map.point_at(((k % s.across) as f64 + 0.5) / per, ((k / s.across) as f64 + 0.5) / per));
        let cell: Vec<u32> = (0..s.h.len()).map(|k| cell(k) as u32).collect();
        Table { square, z, cell, dry: Dry::new(s) }
    }

    /// The same for world `w`'s samples built anew (its cells are those it was made for): the cells' mean heights again,
    /// and the shore where it moved (`Dry::update`).
    pub fn update(&mut self, w: &World) {
        self.z = means(w, &self.square, w.mesh.cells().n());
        self.dry.update(w.terrain_samples());
    }

    /// The volume of liquid that runs through each of the terrain's samples, m³/s, row by row: its discharge
    /// (`World::discharge`) at the mol per m³ of the liquid its cell's ground holds (`World::liquid_in`); none where that
    /// cell holds no liquid. A shell sizes the channel of each sample by it (`Physics::channel`). A sample of a pond
    /// that holds no liquid, or that the shore a shell draws leaves out of the liquid (`dry`), carries no less than
    /// its main way upstream brings: a river runs on along a dry body's floor, and to the shore drawn, which lies up
    /// to a few samples from the samples' own. Under a body's liquid no channel runs.
    pub fn discharge(&self, w: &World) -> Vec<f32> {
        let per_m3: Vec<f64> = (0..w.mesh.cells().n()).map(|i| w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1)).collect();
        let per = |k: usize| per_m3[self.cell[k] as usize];
        let mut out: Vec<f32> = w.discharge.iter().enumerate().map(|(k, &q)| if per(k) > 0.0 { (q as f64 / per(k)) as f32 } else { 0.0 }).collect();
        let (s, levels) = (w.terrain_samples(), w.pond_levels());
        let shore = |k: usize| self.dry.dry[k] || levels[s.pond[k] as usize].is_nan();
        // In the flood's order from the last sample reached: a sample comes before the one its liquid runs to.
        for &k in s.order.iter().rev() {
            let k = k as usize;
            // (And one sample on, so the line's last piece runs into the liquid drawn.)
            if s.pond[k] != NONE && s.main[k] != NONE && (shore(k) || s.pond[s.main[k] as usize] == NONE || shore(s.main[k] as usize)) {
                out[k] = out[k].max(out[s.main[k] as usize]);
            }
        }
        out
    }

    /// What the liquid running through each of the terrain's samples carries, g/m³, row by row: of a sample that carries
    /// liquid (`q`, its discharge, `discharge`), what the main channel of its cell carried in the last month
    /// (`Counters::load_month`); none where it carries none, or before the first month. A shell colours a channel's
    /// water by it.
    pub fn load(&self, w: &World, q: &[f32]) -> Vec<f32> {
        let month = &w.counts.load_month;
        q.iter().zip(&self.cell).map(|(&q, &i)| if q > 0.0 { month.get(i as usize).map_or(0.0, |c| c[0]) } else { 0.0 }).collect()
    }

    /// The present state of world `w` as one tape frame, as a tape would keep it (`frame`), and its table: the melting point of the frozen
    /// cover, K (`cover_melt`), then `ROW` f32 numbers for each square, row by row (`row`), then the kinds of cover of
    /// each square, row by row (`stands::rows`, `stands::kinds` × `KIND_ROW` f32), then the places of its cell's
    /// patches, row by row (`stands::places`, `PLACE_ROW` f32), then the dead walls standing on its cell's patches, row
    /// by row (`stands::dead`, `DEAD_ROW` f32), then the level of each pond of the terrain's
    /// samples, m (`World::pond_levels`: NaN where no body holds it, or where its body holds less than `DRY_M` of liquid on
    /// its mean: dry ground), which a shell draws each pond's liquid flat at, over the ground that lies under it; then
    /// the fetch of each pond, m: the side of a square as large as its liquid's surface, the length of open water a
    /// wind blows over, which a shell sizes its waves by (0 where it holds none); last the kinds a square holds
    /// (`stands::kinds`).
    pub fn frame(&self, w: &World) -> (Vec<u8>, Vec<f32>) {
        let c = Ctx::new(w);
        let mut bytes = Vec::with_capacity(w.map.n() * crate::LAYERS.len());
        frame(&c, &mut bytes);
        let n = w.mesh.cells().n();
        let levels = pcore::flow::levels(w, &(0..n).collect::<Vec<usize>>());
        let rows: Vec<[f64; ROW]> = (0..n).map(|i| self.row(&c, i, c.means.as_ref().map(|m| m[i]), levels.as_ref().map(|l| l[i]))).collect();
        let mut table = vec![cover_melt(w) as f32];
        table.extend(self.square.iter().flat_map(|&i| rows[i].map(|v| v as f32)));
        let stands = |i: usize| levels.as_ref().map_or(0.0, |l| l[i][1] - l[i][0]);
        let count = crate::stands::kinds(w);
        let kinds: Vec<_> = (0..n).map(|i| crate::stands::rows(w, i, stands(i), count)).collect();
        table.extend(self.square.iter().flat_map(|&i| kinds[i].iter().map(|&v| v as f32)));
        let places: Vec<_> = (0..n).map(|i| crate::stands::places(w, i)).collect();
        table.extend(self.square.iter().flat_map(|&i| places[i].map(|v| v as f32)));
        let dead: Vec<_> = (0..n).map(|i| crate::stands::dead(w, i)).collect();
        table.extend(self.square.iter().flat_map(|&i| dead[i].map(|v| v as f32)));
        let mut ponds = w.pond_levels();
        let mut fetch = vec![0.0; ponds.len()];
        for p in w.pools() {
            if p.volume <= DRY_M * p.area {
                ponds[p.pond] = f64::NAN;
            } else {
                fetch[p.pond] = p.area.sqrt();
            }
        }
        table.extend(ponds.iter().map(|&l| l as f32));
        table.extend(fetch.iter().map(|&f| f as f32));
        table.push(count as f32);
        (bytes, table)
    }

    /// The row of cell `i` of the world of context `c`, with what it holds as the shares read it (`flow::means`) and its
    /// lowest ground and level (`flow::levels`), none before Flow first built its state (then its solid surface and the
    /// depth of its liquid): the terrain's mean height over it, m; its ground's temperature, K, and how fast the air
    /// cools with height there, K/m (`World::air_drop`); its ice, m; the level its standing liquid reaches and its lowest
    /// ground, m, and the depth that stands, m; the liquid in its pores and what they hold full, m, the level of the
    /// wetness index its soil's liquid departs about as the laws found it at the start of life's month
    /// (`inside::WetGround::soil`; 0 where it lies evenly or none was found), and the share of its area that is land
    /// (`Ctx::land`), on which a shell draws its cover.
    fn row(&self, c: &Ctx, i: usize, m: Option<Means>, level: Option<[f64; 2]>) -> [f64; ROW] {
        let w = c.w;
        let (z, depth) = (c.f.z[i], c.f.depth[i]);
        let m = m.unwrap_or(Means { pore: 0.0, pores: 0.0, stands: depth, ..Default::default() });
        let [low, top] = level.unwrap_or([z, z + depth]);
        let soil = c.saturated[i].map_or(0.0, |g| g.soil);
        let cools = w.air_drop(i).unwrap_or(0.0);
        [self.z[i], c.f.t[i], cools, c.f.ice[i], top, low, m.stands, m.pore, m.pores, soil, c.land[i]]
    }
}

//! A map of squares, numbered row by row from the north-west corner (x east, y south): flat, or a globe of squares in
//! latitude and longitude whose rows wrap east to west. Each square has 8 neighbours: 4 across a border and 4 across a
//! corner (width 0), so heat and pore liquid cross only real borders while the downhill path of surface liquid may use
//! all 8. On a globe the cells of a row narrow toward the poles, and no link crosses a pole. A rectangle of the map may
//! be refined: each of its squares split in four quarters, built into the mesh at the start.

use super::{Level, LinkSpec, Mesh};
use serde::{Deserialize, Serialize};
use std::f64::consts::{SQRT_2, TAU};

/// Forward link offsets (dx, dy). Every link is met once as (i, i + offset).
const FORWARD: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];

/// A map of `w × h` squares of side `side`, m; with `wrap`, a band of a sphere whose rows are whole circles of latitude
/// and whose squares are `side` from north to south; `region`, the rectangle (x, y, w, h) of squares split in four.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Square {
    pub w: usize,
    pub h: usize,
    pub side: f64,
    pub wrap: bool,
    pub region: Option<[usize; 4]>,
}

impl Square {
    pub fn n(&self) -> usize {
        self.w * self.h
    }
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    /// Neighbour of cell `i` by offset, if inside the map (round the globe east and west).
    #[inline]
    pub fn step(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        let (x, y) = self.xy(i);
        let (nx, ny) = (if self.wrap { (x as i32 + d.0).rem_euclid(self.w as i32) } else { x as i32 + d.0 }, y as i32 + d.1);
        (nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h).then(|| self.idx(nx as usize, ny as usize))
    }
    /// Whether square `s` lies in the refined region.
    fn refined(&self, s: usize) -> bool {
        let (x, y) = self.xy(s);
        self.region.is_some_and(|[x0, y0, rw, rh]| (x0..x0 + rw).contains(&x) && (y0..y0 + rh).contains(&y))
    }
    /// The square that holds cell `i` of the mesh: a refined square's north-west quarter keeps the square's number, its
    /// other three quarters come after all the squares, three by three in the region's order (row by row).
    pub fn square_of(&self, i: usize) -> usize {
        match self.region {
            Some([x0, y0, rw, _]) if i >= self.n() => self.idx(x0 + (i - self.n()) / 3 % rw, y0 + (i - self.n()) / 3 / rw),
            _ => i,
        }
    }
    /// The mesh of the map: its cells (the squares, or those of the region in quarters), rows spread from latitude
    /// `north` (row 0) to `south`, and a coarser level for each of `scales` whose cell holds a block of `scale`² squares
    /// (fewer at the south and east edges).
    pub fn mesh(&self, scales: &[usize], south: f64, north: f64) -> Mesh {
        assert!(!self.wrap || (self.w >= 3 && scales.iter().all(|&s| self.w.is_multiple_of(s))), "a globe's row has 3 cells or more, in whole blocks");
        let mut mesh = Mesh::of(match self.region {
            Some(region) => self.refine(region, south, north),
            None => self.level(south, north),
        });
        // The block of `scale`² squares that holds square `i`, numbered row by row.
        let block = |i: usize, scale: usize| {
            let ((x, y), to) = ((self.xy(i).0 / scale, self.xy(i).1 / scale), self.w.div_ceil(scale));
            (y * to + x) as u32
        };
        // A square of each cell of the coarsest level so far.
        let (mut fine, mut square): (usize, Vec<usize>) = (1, (0..mesh.cells().n()).map(|i| self.square_of(i)).collect());
        for &scale in scales {
            assert!(scale > fine && scale % fine == 0, "each coarser level's scale is a multiple of the one before");
            let parent: Vec<u32> = square.iter().map(|&i| block(i, scale)).collect();
            let mut up = vec![0; parent.iter().max().map_or(0, |&p| p as usize + 1)];
            parent.iter().zip(&square).for_each(|(&p, &i)| up[p as usize] = i);
            mesh.coarsen(parent);
            (fine, square) = (scale, up);
        }
        mesh
    }
    /// The level of the squares, longitudes east from 0 by the degrees of latitude per row (a whole turn on a globe).
    fn level(&self, south: f64, north: f64) -> Level {
        let (n, row) = (self.n(), (north - south) / self.h as f64);
        // On a globe: the sphere's radius, m, and the latitude of the north edge of row `y`, radians.
        let (radius, edge) = (self.side / row.to_radians(), |y: usize| (north - row * y as f64).to_radians());
        let lat = |y: usize| north + (south - north) * (y as f64 + 0.5) / self.h as f64;
        // The length of the circle of latitude `phi` (radians) across one cell, m: a side on the flat map.
        let across = |phi: f64| if self.wrap { radius * phi.cos() * TAU / self.w as f64 } else { self.side };
        let area = |y: usize| if self.wrap { radius * radius * TAU / self.w as f64 * (edge(y).sin() - edge(y + 1).sin()) } else { self.side * self.side };
        let centre = |i: usize| [(self.xy(i).0 as f64 + 0.5) * self.side, (self.xy(i).1 as f64 + 0.5) * self.side];
        let lon = |i: usize| (self.xy(i).0 as f64 + 0.5) * if self.wrap { 360.0 / self.w as f64 } else { row };
        let links: Vec<LinkSpec> = (0..n)
            .flat_map(|i| FORWARD.iter().filter_map(move |&d| self.step(i, d).map(|j| (i, j, d))))
            .map(|(i, j, d)| {
                let (y, side) = (self.xy(i).1, self.side);
                let (dist, width) = match d {
                    (_, 0) => (across(lat(y).to_radians()), side),
                    (0, _) => (side, across(edge(y + 1))),
                    _ if self.wrap => (side.hypot(across(edge(y + 1))), 0.0),
                    _ => (side * SQRT_2, 0.0),
                };
                (i.min(j) as u32, i.max(j) as u32, dist, width)
            })
            .collect();
        let place = ((0..n).map(|i| lat(self.xy(i).1)).collect(), (0..n).map(lon).collect());
        let mut cells = Level::of_cells((0..n).map(|i| area(self.xy(i).1)).collect(), (0..n).map(centre).collect(), place, if self.wrap { radius } else { 0.0 });
        cells.join(links);
        cells
    }
    /// The level of the squares with those of `region` (x, y, w, h) in quarters: the map's quarters joined back into
    /// their squares outside the region (`Level::coarser`). A square and a quarter link only across a border (so a
    /// square beside a region of at least 2 x 2 has at most 8 links), as far apart as their centres across it.
    fn refine(&self, [x0, y0, rw, rh]: [usize; 4], south: f64, north: f64) -> Level {
        assert!(rw >= 2 && rh >= 2 && x0 + rw <= self.w && y0 + rh <= self.h, "a refined region lies inside the map and is 2 x 2 squares or more");
        let (n, half) = (self.n(), Square { w: 2 * self.w, h: 2 * self.h, side: self.side / 2.0, region: None, ..*self });
        // The cell of each quarter of the map: numbered as `square_of` reads them.
        let parent: Vec<u32> = (0..half.n())
            .map(|q| {
                let (qx, qy) = half.xy(q);
                let (x, y, part) = (qx / 2, qy / 2, qx % 2 + 2 * (qy % 2));
                (if part == 0 || !self.refined(self.idx(x, y)) { self.idx(x, y) } else { n + 3 * ((y - y0) * rw + x - x0) + part - 1 }) as u32
            })
            .collect();
        let (mut cells, links) = half.level(south, north).coarser(&parent);
        let quarter = |c: u32| c as usize >= n || self.refined(c as usize);
        let links = (links.into_iter().filter(|&(a, b, _, wd)| wd > 0.0 || quarter(a) == quarter(b)))
            .map(|(a, b, dist, wd)| {
                if quarter(a) == quarter(b) {
                    return (a, b, dist, wd);
                }
                // Across the border of a square (east to west if its centres lie further apart that way, for the
                // square's width, than north to south) and a quarter.
                let (o, s) = (cells.offset(a as usize, b as usize), if quarter(a) { b } else { a } as usize);
                let east = (o[0] * self.side / cells.area[s]).abs() > (o[1] / self.side).abs();
                (a, b, if east { o[0].abs() } else { o[1].abs() }, wd)
            })
            .collect();
        cells.join(links);
        (cells.kid_start, cells.kids) = Default::default();
        cells
    }
}

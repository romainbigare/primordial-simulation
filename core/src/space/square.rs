//! A map of squares, numbered row by row from the north-west corner (x east, y south): flat, or a globe of squares in
//! latitude and longitude whose rows wrap east to west. Each square has 8 neighbours: 4 across a border and 4 across a
//! corner (width 0), so heat and pore liquid cross only real borders while the downhill path of surface liquid may use
//! all 8. On a globe the cells of a row narrow toward the poles, and no link crosses a pole.

use super::{LinkSpec, Mesh};
use serde::{Deserialize, Serialize};
use std::f64::consts::{SQRT_2, TAU};

/// Forward link offsets (dx, dy). Every link is met once as (i, i + offset).
const FORWARD: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];
/// All 8 neighbour offsets, clockwise from east.
pub const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// A map of `w × h` squares of side `side`, m; with `wrap`, a band of a sphere whose rows are whole circles of latitude
/// and whose squares are `side` from north to south.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Square {
    pub w: usize,
    pub h: usize,
    pub side: f64,
    pub wrap: bool,
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
    /// The mesh of the map: the squares, rows spread from latitude `north` (row 0) to `south`, longitudes east from 0 by
    /// the degrees of latitude per row (a whole turn on a globe), and a coarser level for each of `scales` whose cell
    /// holds a block of `scale`² squares (fewer at the south and east edges).
    pub fn mesh(&self, scales: &[usize], south: f64, north: f64) -> Mesh {
        assert!(!self.wrap || (self.w >= 3 && scales.iter().all(|&s| self.w.is_multiple_of(s))), "a globe's row has 3 cells or more, in whole blocks");
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
        let mut cells = super::Level::of_cells((0..n).map(|i| area(self.xy(i).1)).collect(), (0..n).map(centre).collect(), place, if self.wrap { radius } else { 0.0 });
        cells.join(links);
        let mut mesh = Mesh::of(cells);
        // The block of `scale`² squares that holds square `i`, numbered row by row; on a globe a row's blocks in the
        // folded order 0, w-1, 1, w-2, ..., so the blocks either side of the wrap are numbered side by side and a banded
        // solve of the level stays one row wide (`solve::band`).
        let block = |i: usize, scale: usize| {
            let ((x, y), to) = ((self.xy(i).0 / scale, self.xy(i).1 / scale), self.w.div_ceil(scale));
            (y * to + if !self.wrap { x } else if 2 * x < to { 2 * x } else { 2 * (to - 1 - x) + 1 }) as u32
        };
        // A square of each cell of the coarsest level so far.
        let (mut fine, mut square): (usize, Vec<usize>) = (1, (0..n).collect());
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
}

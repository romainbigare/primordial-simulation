//! A flat map of squares, numbered row by row from the north-west corner (x east, y south). Each square has 8
//! neighbours: 4 across a border (width = side) and 4 across a corner (width 0), so heat and pore liquid cross only
//! real borders while the downhill path of surface liquid may use all 8.

use super::{LinkSpec, Mesh};
use serde::{Deserialize, Serialize};
use std::f64::consts::SQRT_2;

/// Forward link offsets (dx, dy). Every link is met once as (i, i + offset).
const FORWARD: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];
/// All 8 neighbour offsets, clockwise from east.
pub const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// A map of `w × h` squares of side `side`, m.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Square {
    pub w: usize,
    pub h: usize,
    pub side: f64,
}

impl Square {
    pub fn n(&self) -> usize {
        self.w * self.h
    }
    /// Area of a square, m².
    pub fn area(&self) -> f64 {
        self.side * self.side
    }
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    /// Neighbour of cell `i` by offset, if inside the map.
    #[inline]
    pub fn step(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        let (x, y) = self.xy(i);
        let (nx, ny) = (x as i32 + d.0, y as i32 + d.1);
        (nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h).then(|| self.idx(nx as usize, ny as usize))
    }
    /// The mesh of the map: the squares, rows spread from latitude `north` (row 0) to `south`, and a coarser level for
    /// each of `scales` whose cell holds a block of `scale`² squares (fewer at the south and east edges).
    pub fn mesh(&self, scales: &[usize], south: f64, north: f64) -> Mesh {
        let n = self.n();
        let lat = |y: usize| north + (south - north) * (y as f64 + 0.5) / self.h as f64;
        let centre = |i: usize| {
            let (x, y) = self.xy(i);
            [(x as f64 + 0.5) * self.side, (y as f64 + 0.5) * self.side]
        };
        let links: Vec<LinkSpec> = (0..n)
            .flat_map(|i| FORWARD.iter().filter_map(move |&d| self.step(i, d).map(|j| (i, j, d))))
            .map(|(i, j, d)| {
                let corner = d.0 != 0 && d.1 != 0;
                (i as u32, j as u32, if corner { self.side * SQRT_2 } else { self.side }, if corner { 0.0 } else { self.side })
            })
            .collect();
        let cells = super::Level::new(vec![self.area(); n], (0..n).map(centre).collect(), (0..n).map(|i| lat(self.xy(i).1)).collect(), links);
        let mut mesh = Mesh::new(cells);
        let mut fine = 1;
        for &scale in scales {
            assert!(scale > fine && scale % fine == 0, "each coarser level's scale is a multiple of the one before");
            let (from, to) = (Square { w: self.w.div_ceil(fine), ..*self }, self.w.div_ceil(scale));
            mesh.coarsen((0..from.w * self.h.div_ceil(fine)).map(|c| {
                let (x, y) = from.xy(c);
                ((y * fine / scale) * to + x * fine / scale) as u32
            }).collect());
            fine = scale;
        }
        mesh
    }
}

//! What lies in the cells of a window of the map, slot by slot of their cover, for a viewer that draws each from its
//! numbers (laws.md 3). Read from the world, nothing written. A shell hands the list out where the camera looks
//! closely.
//!
//! The list is little-endian: `u16` genes, then each gene's name and unit (`cover::GENES`), each a `u8` length and
//! its UTF-8 bytes; `u32` cells, then for each cell `u32` its index, `f32` its west, north, east and south
//! edges in squares and its area in m², `u8` the patches of its cover in use, then of each `f32` its share of the cell
//! and its age, years since it was made (`cover::Patch`), `u16` its slots of cover, then each slot: `u32` starting
//! kind, `f32` built cover kg/m², capture area m²/m², its genes in the order the list names them, `u8` full (1) or not,
//! `u8` its patch (`Cover::patch`), then 2 zeros.

use pcore::cover::{Parts, GENES, NG};
use pcore::World;

/// The things of each cell that overlaps the window from column `x` and row `y` of the map, `w` squares wide and `h`
/// high, cell by cell in their order (the module's list).
pub fn things(world: &World, [x, y, wide, high]: [f64; 4]) -> Vec<u8> {
    let cells: Vec<usize> = (0..world.mesh.cells().n())
        .filter(|&i| {
            let [w0, n0, e0, s0] = world.map.bounds(i);
            w0 < x + wide && e0 > x && n0 < y + high && s0 > y
        })
        .collect();
    let mut out = Out(Vec::new());
    out.u16(NG as u16);
    GENES.iter().flat_map(|&(name, unit)| [name, unit]).for_each(|t| out.text(t));
    out.u32(cells.len() as u32);
    for i in cells {
        cell(world, i, &mut out);
    }
    out.0
}

/// The record of cell `i` and of its slots of cover.
fn cell(w: &World, i: usize, out: &mut Out) {
    let (life, area, parts) = (w.life.cell_at(i), w.mesh.cells().area()[i], Parts::of(&w.chem));
    let cover = &life.cover;
    let slots: Vec<usize> = if parts.is_some() { cover.held().collect() } else { vec![] };
    out.u32(i as u32);
    w.map.bounds(i).into_iter().chain([area]).for_each(|v| out.f32(v));
    let patches = &cover.patches[..cover.patch_count()];
    out.0.push(patches.len() as u8);
    patches.iter().for_each(|p| [p.share, p.age as f64].into_iter().for_each(|v| out.f32(v)));
    out.u16(slots.len().min(u16::MAX as usize) as u16);
    if let Some(p) = parts {
        let mm = p.mm(&w.chem);
        for x in slots {
            out.u32(cover.root[x]);
            out.f32(cover.built(&mm, x) / area);
            out.f32(cover.leaf(&mm, x) / area);
            cover.genes[x][..NG].iter().for_each(|&g| out.f32(g as f64));
            out.0.extend([cover.full[x] as u8, cover.patch[x], 0, 0]);
        }
    }
}

/// The list being written.
struct Out(Vec<u8>);

impl Out {
    /// `t` as a `u8` length and its UTF-8 bytes (at most 255 of them).
    fn text(&mut self, t: &str) {
        let b = &t.as_bytes()[..t.len().min(u8::MAX as usize)];
        self.0.push(b.len() as u8);
        self.0.extend(b);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f64) {
        self.0.extend((v as f32).to_le_bytes());
    }
}

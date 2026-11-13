//! The things of a window (`tape::things`): each cell there lists its slots of cover with their numbers, read from the
//! world.

use pcore::cover::{LEAF, STORE, WALLS};
use worldgen::life::{KINDS, LIGHT_ROW};
use worldgen::testkit;

/// Reads the list in order.
struct Read<'a>(&'a [u8]);

impl Read<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let (head, rest) = self.0.split_at(N);
        self.0 = rest;
        head.try_into().unwrap()
    }
    fn u16(&mut self) -> usize {
        u16::from_le_bytes(self.take()) as usize
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    fn f32(&mut self) -> f32 {
        f32::from_le_bytes(self.take())
    }
    fn f32s<const N: usize>(&mut self) -> [f32; N] {
        std::array::from_fn(|_| self.f32())
    }
}

#[test]
fn a_window_lists_what_lies_in_its_cells() {
    let (mut w, i) = testkit::start();
    testkit::cover(&mut w, i, 0.5);
    let ([x0, y0, x1, y1], area) = (w.map.bounds(i), w.mesh.cells().area()[i]);

    // The window of the cell itself holds it alone: its neighbours only touch it.
    let bytes = tape::things(&w, [x0, y0, x1 - x0, y1 - y0]);
    let mut r = Read(&bytes);
    assert_eq!((r.u32(), r.u32()), (1, i as u32));
    assert_eq!(r.f32s::<5>(), [x0, y0, x1, y1, area].map(|v| v as f32));
    assert_eq!(r.u16(), 1, "one slot of cover");

    // The slot of cover: its kind, its built cover and capture area per m² (10^`LEAF` per kg of its living cover,
    // `Cover::living`) and its genes.
    let genes = KINDS[LIGHT_ROW].genes;
    assert_eq!(r.u32(), LIGHT_ROW as u32);
    let built = r.f32() as f64;
    assert!((built - 0.5 / (1.0 + 10f64.powf(genes[STORE] as f64))).abs() < 1e-6, "built cover {built} kg/m²");
    let leaf = r.f32() as f64;
    let p = pcore::cover::Parts::of(&w.chem).expect("cover parts");
    let (c, area) = (w.life.cover(i), w.mesh.cells().area()[i]);
    let living = c.living(&p.mm(&w.chem), c.held().next().expect("a slot")) / area;
    assert!((leaf / (living * 10f64.powf(genes[LEAF] as f64)) - 1.0).abs() < 1e-5, "capture area {leaf} m²/m²");
    let seen = r.f32s::<{ pcore::cover::NG }>();
    assert_eq!(seen[WALLS], genes[WALLS]);
    assert_eq!(r.take::<4>(), [0; 4]);
    assert!(r.0.is_empty(), "nothing after the last record");

    // The whole map lists every cell.
    let all = tape::things(&w, [0.0, 0.0, w.map.w as f64, w.map.h as f64]);
    assert_eq!(u32::from_le_bytes(all[..4].try_into().unwrap()) as usize, w.mesh.cells().n());
}

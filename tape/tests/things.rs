//! The things of a window (`tape::things`): each cell there lists its bodies, its slots of cover and its parts with
//! their numbers, read from the world.

use pcore::brain::REST;
use pcore::cover::{LEAF, STORE, WALLS};
use worldgen::life::{Place, KINDS};
use worldgen::testkit::{self, Brain};

/// Reads the list in order.
struct Read<'a>(&'a [u8]);

impl Read<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let (head, rest) = self.0.split_at(N);
        self.0 = rest;
        head.try_into().unwrap()
    }
    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
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
    fn id(&mut self) -> u64 {
        self.u32() as u64 | (self.u32() as u64) << 32
    }
    fn f32s<const N: usize>(&mut self) -> [f32; N] {
        std::array::from_fn(|_| self.f32())
    }
}

/// A body's record: id, members, a member's mass, its diet bits, whether adult, the part it holds.
struct Seen {
    id: u64,
    members: f32,
    mass: f32,
    diet: u8,
    held: u64,
}

fn body(r: &mut Read) -> Seen {
    let id = r.id();
    let [members, mass, ..] = r.f32s::<11>();
    let _root_lineage = [r.u32(), r.u32()];
    let [diet, _adult, _hidden, _] = [r.u8(), r.u8(), r.u8(), r.u8()];
    let (held, _inside) = (r.id(), r.id());
    Seen { id, members, mass, diet, held }
}

#[test]
fn a_window_lists_what_lies_in_its_cells() {
    let (mut w, i) = testkit::start();
    testkit::cover(&mut w, i, 0.5);
    let one = testkit::founder(&mut w, 1, i, &Brain::neutral());
    let one = testkit::place(&mut w, one);
    let node = testkit::founder(&mut w, 1, i, &Brain { hidden: 0, ..Brain::only(REST) });
    let node = testkit::node_of(&mut w, node, 20.0);
    let node = testkit::place(&mut w, node);
    let part = testkit::plate(&mut w, i, 2.0, true);
    let ([x0, y0, x1, y1], area) = (w.map.bounds(i), w.mesh.cells().area()[i]);

    // The window of the cell itself holds it alone: its neighbours only touch it.
    let bytes = tape::things(&w, [x0, y0, x1 - x0, y1 - y0]);
    let mut r = Read(&bytes);
    assert_eq!((r.u32(), r.u32()), (1, i as u32));
    assert_eq!(r.f32s::<5>(), [x0, y0, x1, y1, area].map(|v| v as f32));
    assert_eq!([r.u16(), r.u16(), r.u16(), r.u16()], [2, 1, 1, 0]);

    // The individual has a brain to bite with; the node has none. Both graze and take remains.
    let (a, b) = (body(&mut r), body(&mut r));
    let graze = tape::DIET_COVER | tape::DIET_REMAINS;
    for (seen, h, diet) in [(a, one, graze | tape::DIET_BODIES), (b, node, graze)] {
        let x = &w.life[h];
        assert_eq!((seen.id, seen.members, seen.diet, seen.held), (x.id, x.n() as f32, diet, 0));
        assert_eq!(seen.mass, (x.mass(&w.chem) / x.n()) as f32, "the mass of one member");
    }

    // The slot of cover: its kind, its built cover and capture area per m² (10^`LEAF` per kg of its living cover,
    // `Cover::living`) and its genes.
    let Place::Field(f) = &KINDS[0].place else { unreachable!("kind 0 is cover") };
    assert_eq!(r.u32(), 0);
    let built = r.f32() as f64;
    assert!((built - 0.5 / (1.0 + 10f64.powf(f.genes[STORE] as f64))).abs() < 1e-6, "built cover {built} kg/m²");
    let leaf = r.f32() as f64;
    let p = pcore::cover::Parts::of(&w.chem).expect("cover parts");
    let (c, area) = (w.life.cover(i), w.mesh.cells().area()[i]);
    let living = c.living(&p.mm(&w.chem), c.held().next().expect("a slot")) / area;
    assert!((leaf / (living * 10f64.powf(f.genes[LEAF] as f64)) - 1.0).abs() < 1e-5, "capture area {leaf} m²/m²");
    let genes = r.f32s::<{ pcore::cover::NG }>();
    assert_eq!(genes[WALLS], f.genes[WALLS]);
    assert_eq!(r.take::<4>(), [0; 4]);

    // The part: its id and count, sizes longest first, mass, a colour and hardness of rock, on the ground, held by none.
    let p = w.life.part(part).expect("the part");
    assert_eq!((r.id(), r.f32()), (part, 1.0), "the part, one");
    let [l, b2, t, hollow, mass, red, green, blue, hard, buried] = r.f32s::<10>();
    assert_eq!(([l, b2, t], hollow), (p.size, p.hollow));
    assert!(l >= b2 && b2 >= t && (mass as f64 - 2.0).abs() < 1e-5, "sizes {l} {b2} {t}, {mass} kg");
    assert!([red, green, blue].iter().all(|c| (0.0..=1.0).contains(c)) && hard > 0.0 && buried == 0.0);
    assert_eq!((r.id(), r.id()), (0, part), "held by none, its own root");
    let [_space, _closed] = r.f32s::<2>();
    assert!(r.0.is_empty(), "nothing after the last record");

    // The whole map lists every cell.
    let all = tape::things(&w, [0.0, 0.0, w.map.w as f64, w.map.h as f64]);
    assert_eq!(u32::from_le_bytes(all[..4].try_into().unwrap()) as usize, w.mesh.cells().n());
}

/// Alike parts lying alone in a cell are one lot (laws.md 3.1): the window lists it once, with its count, and the
/// sizes and mass of each of its parts, so a viewer draws its parts one by one.
#[test]
fn a_lot_is_listed_once_with_its_count() {
    let (mut w, i) = testkit::start();
    let parts: Vec<u64> = (0..5).map(|_| testkit::plate(&mut w, i, 2.0, true)).collect();
    pcore::bond::step(&mut w);
    let ([x0, y0, x1, y1], lot) = (w.map.bounds(i), w.life.part(parts[0]).expect("the lot"));
    let bytes = tape::things(&w, [x0, y0, x1 - x0, y1 - y0]);
    let mut r = Read(&bytes);
    assert_eq!((r.u32(), r.u32()), (1, i as u32));
    r.f32s::<5>();
    assert_eq!([r.u16(), r.u16(), r.u16(), r.u16()], [0, 0, 1, 0], "one record of parts");
    assert_eq!((r.id(), r.f32()), (parts[0], 5.0), "the lot and its count");
    let [l, b, t, _hollow, mass] = r.f32s::<5>();
    assert_eq!([l, b, t], lot.size, "the sizes of each part");
    assert!((mass as f64 / 2.0 - 1.0).abs() < 1e-5, "the mass of each part: {mass} kg");
}

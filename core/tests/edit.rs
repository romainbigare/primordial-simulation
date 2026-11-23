//! Edits by a brush (handover.md 11): the cells under a round brush and the share of each it covers; the ground under
//! it raised and lowered, booked from the outside node, and the samples built again over it when the edits settle.

use pcore::chem::Phase;
use pcore::edit::{Act, Brush, Edit};
use pcore::World;
use worldgen::testkit::{inland, small_world, Books};

/// The area of a disc of radius `r`, in squares a side.
fn disc(r: f64) -> f64 {
    std::f64::consts::PI * r * r
}

/// A brush covers the cells under it by the terrain's samples whose centres lie within its radius: the squares it
/// covers, summed over its cells, are its disc's area to within the samples along its edge, and each cell's place lies
/// in that cell; a brush finer than a sample covers the sample under its centre alone.
#[test]
fn a_brush_covers_its_disc_of_the_cells_under_it() {
    let w = small_world();
    let centre = [w.map.w as f64 / 2.0, w.map.h as f64 / 2.0];
    let side = w.map.side;
    let under = w.under(Brush { centre, radius_m: 1.5 * side });
    let covered: f64 = under.iter().map(|u| u.share * w.map.squares(u.cell)).sum();
    assert!((covered / disc(1.5) - 1.0).abs() < 0.02, "covered {covered} squares of a disc of {}", disc(1.5));
    for u in &under {
        let [x0, y0, x1, y1] = w.map.bounds(u.cell);
        assert!(u.share > 0.0 && u.share <= 1.0 && (x0..x1).contains(&u.place[0]) && (y0..y1).contains(&u.place[1]), "{u:?} in {:?}", w.map.bounds(u.cell));
    }
    assert!(under.windows(2).all(|p| p[0].cell < p[1].cell), "the cells in their order");
    let fine = w.under(Brush { centre, radius_m: 1.0 });
    let samples = (side / w.map.length(pcore::space::terrain::SAMPLE_DEPTH)).powi(2);
    assert_eq!(fine.len(), 1, "{fine:?}");
    assert!((fine[0].share * w.map.squares(fine[0].cell) * samples - 1.0).abs() < 1e-12, "one sample: {fine:?}");
}

/// The volume of the solids that hold the ground up (`World::holds_up`) in cell `i`'s loose ground and in its bed, m³.
fn ground_volumes(w: &World, i: usize) -> [f64; 2] {
    let v = |at: &dyn Fn(usize) -> f64| (0..w.chem.ns()).filter(|&k| w.holds_up(k)).map(|k| at(k) * w.chem.molar_volume(k, Phase::Solid)).sum::<f64>();
    [v(&|k| w.g(k, i)), v(&|k| w.b(k, i))]
}

/// An edit of the ground under a brush inside cell `i` (radius 0.4 of a square about its middle), raising it by
/// `height` m at its centre.
fn shape(w: &mut World, i: usize, height: f64) -> bool {
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let act = Act::Ground { radius_m: 0.4 * w.map.side, height_m: height, stroke_m: 0.0, make: 0 };
    w.edit(&[Edit { place: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], act }])[0]
}

/// The volume the edits shaped on the terrain's samples, m³ (`Relief::shaped` by each sample's area).
fn shaped(w: &World) -> f64 {
    let s = w.terrain_samples();
    w.relief.shaped.iter().enumerate().map(|(k, &m)| m * s.area(k)).sum()
}

/// A raise lays the cell's own rock and, at the top of a stroke, `edit_top_m` of loose ground: the cell's ground rises by
/// the volume its samples rose by, the books are kept. When the edits settle, the samples are built again over it (a
/// new version of the relief, no build of the years): against the same world's samples built again with no edit, its
/// sample nearest its centre stands higher by what it was raised and a sample far from it stands where it does.
#[test]
fn a_raised_brush_raises_its_disc_and_keeps_the_books() {
    let mut w = small_world();
    let i = inland(&w);
    let (books, z0, [loose0, rock0]) = (Books::now(&w), w.ground_z(i), ground_volumes(&w, i));
    let (years, version) = (w.built_years(), w.relief.version);
    let mut control = w.clone();
    control.relief.due = true;
    control.settle();
    let before = control.terrain_samples();
    let height = 20.0;
    assert!(shape(&mut w, i, height));
    books.assert_kept(&w, 1e-9);
    let (v, area) = (shaped(&w), w.mesh.cells().area()[i]);
    let [loose, rock] = ground_volumes(&w, i);
    let top = w.cfg.edit_top_m / height;
    assert!(((w.ground_z(i) - z0) * area / v - 1.0).abs() < 1e-9, "the ground rose {} m for {v} m3", w.ground_z(i) - z0);
    assert!(((loose - loose0) / (top * v) - 1.0).abs() < 1e-9 && ((rock - rock0) / ((1.0 - top) * v) - 1.0).abs() < 1e-9, "loose {} and rock {} of {v} m3", loose - loose0, rock - rock0);
    let settled = w.settle();
    books.assert_kept(&w, 1e-9);
    assert!(settled.ground && w.relief.version == version + 1 && w.built_years() == years && !w.relief.due, "{settled:?}, version {}", w.relief.version);
    let after = w.terrain_samples();
    let centre = (0..w.relief.shaped.len()).max_by(|&a, &b| w.relief.shaped[a].total_cmp(&w.relief.shaped[b])).unwrap();
    let far = (centre + after.h.len() / 2) % after.h.len();
    let rose = w.relief.shaped[centre];
    assert!(rose > 0.9 * height && ((after.h[centre] - before.h[centre]) / rose - 1.0).abs() < 1e-6, "the centre rose {} m for {rose}", after.h[centre] - before.h[centre]);
    assert!((after.h[far] - before.h[far]).abs() < 1e-3, "a sample far away moved {} m", after.h[far] - before.h[far]);
}

/// A lowering takes the loose ground first: one shallower than the cell's loose ground leaves its rock as it was; the
/// books are kept.
#[test]
fn a_lowering_takes_the_loose_ground_first() {
    let mut w = small_world();
    let i = inland(&w);
    let (books, [loose0, rock0]) = (Books::now(&w), ground_volumes(&w, i));
    assert!(shape(&mut w, i, -0.5));
    let v = -shaped(&w);
    let [loose, rock] = ground_volumes(&w, i);
    assert!(v > 0.0 && v < loose0 && rock == rock0 && ((loose0 - loose) / v - 1.0).abs() < 1e-9, "took {} loose of {v} m3, rock {rock0} -> {rock}", loose0 - loose);
    books.assert_kept(&w, 1e-9);
}

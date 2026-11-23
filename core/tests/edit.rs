//! Edits by a brush (handover.md 11): the cells under a round brush and the share of each it covers.

use pcore::edit::Brush;
use worldgen::testkit::small_world;

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

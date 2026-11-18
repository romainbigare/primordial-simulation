//! The test card (`worldgen::testkit::card`): its world passes the checks, and the cells' table a shell draws from holds
//! what each square was set to.

use worldgen::testkit::{self, COLD_K, FROM, SQUARES, WARM_K};

/// Of a kind's numbers in a square's row (tape/src/stands.rs `row`): its starting kind, its built cover and its large
/// plants' height; of a square's row (`Table::frame`), its ground's temperature and its ice.
const ROOT: usize = 0;
const BUILT: usize = 1;
const HEIGHT: usize = 4;
const T: usize = 1;
const ICE: usize = 3;
/// How near a stand's large plants stand to the height set, as a share of it.
const TALL_WITHIN: f64 = 0.01;

/// The card passes the checks of the settings; each square's kinds in the table are its stands, of their starting
/// kinds, the large plants as tall as set; a bare square holds none; a cold square's ground is as cold as set, and a
/// snowy one holds ice.
#[test]
fn the_card_holds_what_was_set() {
    let w = testkit::card();
    w.cfg.validate().expect("the card's settings pass");
    let (_, t) = tape::Table::new(&w).frame(&w);
    let (n, kinds) = (w.map.w * w.map.h, w.cfg.variety);
    for (r, row) in SQUARES.iter().enumerate() {
        for (c, sq) in row.iter().enumerate() {
            let s = (FROM.1 + r) * w.map.w + FROM.0 + c;
            let cell = &t[1 + s * tape::ROW..1 + (s + 1) * tape::ROW];
            let at = 1 + n * tape::ROW + s * kinds * tape::KIND_ROW;
            let held: Vec<&[f32]> = t[at..at + kinds * tape::KIND_ROW].chunks(tape::KIND_ROW).filter(|k| k[ROOT] >= 0.0 && k[BUILT] > 0.0).collect();
            assert_eq!(held.len(), sq.stands.len(), "square {c} {r}: its stands");
            for st in sq.stands {
                let k = held.iter().find(|k| k[ROOT] as usize == st.row).unwrap_or_else(|| panic!("square {c} {r}: row {}", st.row));
                if let Some(tall) = st.tall {
                    assert!((k[HEIGHT] as f64 / tall - 1.0).abs() < TALL_WITHIN, "square {c} {r}: row {} {} m, set {tall} m", st.row, k[HEIGHT]);
                }
            }
            let t = if sq.cold { COLD_K } else { WARM_K };
            assert!((cell[T] as f64 - t).abs() < 0.5, "square {c} {r}: {} K, set {t} K", cell[T]);
            assert_eq!(cell[ICE] > 0.0, sq.snow, "square {c} {r}: ice {} m", cell[ICE]);
        }
    }
}

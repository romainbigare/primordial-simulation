//! The test card: a small island whose squares are set by hand, each a kind of stand or ground the map draws, so a
//! picture of the map before and after a change of the page shows the same scene whatever the laws do (`ew card`, the
//! scene set `card` of `web/look`). It is not stepped: the page shows it as it is set.

use super::{hold_ground, put, put_kg_m2, LIQUID};
use crate::life::KINDS;
use pcore::cover::{Parts, WALL};
use pcore::World;

/// The island the card stands on: one rise of seed `SEED` on `SIDES` squares (`one_rise`), whose land holds a block of
/// `CARD` squares from square `FROM` (column, row).
const SEED: u64 = 2;
const SIDES: (usize, usize) = (12, 10);
pub const FROM: (usize, usize) = (3, 3);
const CARD: (usize, usize) = (6, 4);

/// The rows of the starting kinds the card's stands are of (`life::KINDS`): small herbs, herbs, trees, trees of wet
/// ground, woody plants of dry ground, tall soft plants.
const SMALL_HERBS: usize = 0;
const HERBS: usize = 1;
const TREES: usize = 2;
const WET_TREES: usize = 3;
const DRY_WOODY: usize = 4;
const TALL_SOFT: usize = 5;

/// A stand of the card: of the starting kind of row `row`, `kg` per m² of its ground, its large plants `tall` m (none:
/// its plants as they are founded), and its groups, each its share of the stand's built cover and its plant's mass
/// against the large plant's.
pub struct Stand {
    pub row: usize,
    kg: f64,
    pub tall: Option<f64>,
    groups: &'static [(f64, f64)],
}

/// One group, all of the stand.
const ONE: &[(f64, f64)] = &[(1.0, 1.0)];
/// Four groups of a closed stand: the large plants hold most of it, the saplings and seedlings under them little.
const FOUR: &[(f64, f64)] = &[(0.7, 1.0), (0.2, 0.25), (0.07, 0.04), (0.03, 0.004)];

/// A sward of herbs under a stand or alone.
const SWARD: Stand = Stand { row: HERBS, kg: 0.3, tall: None, groups: ONE };
/// The ground temperature of a square in the cold season, K: below the trees' frost gene, so a kind of thin leaves
/// stands bare, and below the liquid's melting point, so the snow laid on it lies.
pub const COLD_K: f64 = 262.0;
/// The ground temperature of every other square of the card, K: late summer's.
pub const WARM_K: f64 = 290.0;
/// The snow laid on a cold square, kg per m² (as liquid).
const SNOW_KG: f64 = 60.0;

/// What a square of the card holds: its stands, and whether it is in the cold season with snow on it.
pub struct Square {
    pub stands: &'static [Stand],
    pub cold: bool,
    pub snow: bool,
}

/// The card's squares, row by row from `FROM` (`CARD`); the squares about the block keep their bare ground, and the
/// island its shore and the low liquid about it.
pub const SQUARES: [[Square; CARD.0]; CARD.1] = [
    [
        // A tall closed stand of four size groups.
        Square { stands: &[Stand { row: TREES, kg: 25.0, tall: Some(24.0), groups: FOUR }], cold: false, snow: false },
        // An open stand of tall trees over a sward.
        Square { stands: &[Stand { row: TREES, kg: 3.0, tall: Some(20.0), groups: ONE }, SWARD], cold: false, snow: false },
        // A young dense stand whose plants are near 4 m (where the page hands plants from one lattice to the next).
        Square { stands: &[Stand { row: TREES, kg: 4.0, tall: Some(4.0), groups: ONE }], cold: false, snow: false },
        // Tall soft plants.
        Square { stands: &[Stand { row: TALL_SOFT, kg: 3.0, tall: Some(6.0), groups: ONE }], cold: false, snow: false },
        // Bushes over a sward.
        Square { stands: &[Stand { row: DRY_WOODY, kg: 1.5, tall: Some(1.5), groups: ONE }, SWARD], cold: false, snow: false },
        // A sward of two kinds.
        Square { stands: &[SWARD, Stand { row: SMALL_HERBS, kg: 0.15, tall: None, groups: ONE }], cold: false, snow: false },
    ],
    [
        // Bare loose ground.
        Square { stands: &[], cold: false, snow: false },
        // Trees of wet ground over a sward.
        Square { stands: &[Stand { row: WET_TREES, kg: 12.0, tall: Some(16.0), groups: ONE }, SWARD], cold: false, snow: false },
        // Snow on a sward.
        Square { stands: &[SWARD], cold: true, snow: true },
        // A woody stand in the cold season, without leaves (a kind of thin leaves, which sheds them).
        Square { stands: &[Stand { row: WET_TREES, kg: 12.0, tall: Some(16.0), groups: ONE }], cold: true, snow: false },
        // Tall trees over bushes.
        Square { stands: &[Stand { row: TREES, kg: 10.0, tall: Some(22.0), groups: ONE }, Stand { row: DRY_WOODY, kg: 1.0, tall: Some(1.2), groups: ONE }], cold: false, snow: false },
        // A thin sward.
        Square { stands: &[Stand { row: HERBS, kg: 0.05, tall: None, groups: ONE }], cold: false, snow: false },
    ],
    [
        // A sward.
        Square { stands: &[SWARD], cold: false, snow: false },
        // A middle-aged stand of four size groups.
        Square { stands: &[Stand { row: TREES, kg: 6.0, tall: Some(10.0), groups: FOUR }], cold: false, snow: false },
        // Bare loose ground.
        Square { stands: &[], cold: false, snow: false },
        // Low soft plants over a sward.
        Square { stands: &[Stand { row: TALL_SOFT, kg: 1.0, tall: Some(3.0), groups: ONE }, SWARD], cold: false, snow: false },
        // Tall bushes.
        Square { stands: &[Stand { row: DRY_WOODY, kg: 3.0, tall: Some(2.5), groups: ONE }], cold: false, snow: false },
        // A sward.
        Square { stands: &[SWARD], cold: false, snow: false },
    ],
    [
        // Bare loose ground.
        Square { stands: &[], cold: false, snow: false },
        // A sward.
        Square { stands: &[SWARD], cold: false, snow: false },
        // A tall closed stand of trees of wet ground in four size groups.
        Square { stands: &[Stand { row: WET_TREES, kg: 25.0, tall: Some(24.0), groups: FOUR }], cold: false, snow: false },
        // Snow on a sward.
        Square { stands: &[SWARD], cold: true, snow: true },
        // Bare loose ground.
        Square { stands: &[], cold: false, snow: false },
        // A sward.
        Square { stands: &[SWARD], cold: false, snow: false },
    ],
];

/// The share of its pores' room the loose ground of a warm square with stands holds of the liquid: about as wet as a
/// soil at field capacity, so no kind stands in drought (the card's ground has had no rain).
const WET: f64 = 0.8;

/// The month of the year the card is set in: late summer, as the look's other scenes (9 of 12 months).
const MONTH: u64 = 9;

/// The test card: the island of `SEED` with its soil, set in late summer (`MONTH`), each square of `SQUARES` holding
/// its stands, its ground at `WARM_K` and wet (`wet`), or in the cold season at `COLD_K` with `SNOW_KG` of snow where
/// asked; and Flow's state of it.
pub fn card() -> World {
    let mut w = super::one_rise(SEED, SIDES.0, SIDES.1);
    crate::life::soil(&mut w, &crate::chemistry0().1.soil);
    w.time = MONTH * w.cfg.calendar().month;
    for (r, row) in SQUARES.iter().enumerate() {
        for (c, sq) in row.iter().enumerate() {
            let i = w.cell_of((FROM.0 + c, FROM.1 + r));
            assert!(w.is_land(i), "square {} {} of the card is land", FROM.0 + c, FROM.1 + r);
            sq.stands.iter().for_each(|s| stand(&mut w, i, s));
        }
    }
    pcore::flow::state(&mut w, true);
    wet(&mut w);
    for (r, row) in SQUARES.iter().enumerate() {
        for (c, sq) in row.iter().enumerate() {
            let i = w.cell_of((FROM.0 + c, FROM.1 + r));
            if sq.snow {
                put_kg_m2(&mut w, LIQUID, i, SNOW_KG);
            }
            hold_ground(&mut w, i, if sq.cold { COLD_K } else { WARM_K });
        }
    }
    // Flow's state found again from the ground as set, so what a shell reads of each cell is the card's.
    pcore::flow::state(&mut w, true);
    w
}

/// The loose ground of the card's warm squares that hold stands wetted to `WET` of its pores' room (as Flow found it,
/// `flow::means`); bare ground stays as dry as built (wet bare ground grows mats of moss on the map).
fn wet(w: &mut World) {
    let wetted = |&(c, r): &(usize, usize)| !SQUARES[r][c].cold && !SQUARES[r][c].stands.is_empty();
    let squares = (0..CARD.1).flat_map(|r| (0..CARD.0).map(move |c| (c, r))).filter(wetted);
    let cells: Vec<usize> = squares.map(|(c, r)| w.cell_of((FROM.0 + c, FROM.1 + r))).collect();
    let means = pcore::flow::means(w, &cells).expect("Flow's state of the card");
    let per_m = w.chem.molar_volume(LIQUID, pcore::chem::Phase::Liquid);
    for (&i, m) in cells.iter().zip(means) {
        put(w, LIQUID, i, (WET * m.pores - m.pore).max(0.0) * w.mesh.cells().area()[i] / per_m);
    }
}

/// Stand `s` placed in cell `i`: its kind's genes at `kg` per m² (`cover::place`), then its plants set in its groups,
/// the large ones `tall` m (`size_to`).
fn stand(w: &mut World, i: usize, s: &Stand) {
    let k = &KINDS[s.row];
    assert!(pcore::cover::place(w, i, 0, k.genes, s.row as u32, s.kg), "no room for row {} in cell {i}", s.row);
    if let Some(tall) = s.tall {
        size_to(w, i, s.row as u32, s.groups, tall);
    }
}

/// Bisection steps on the large plant's mass (`size_to`): a mass known to 2^-60 of its log range.
const BISECTIONS: usize = 60;
/// The range of a plant's mass `size_to` looks in, kg.
const MASS_KG: (f64, f64) = (1e-6, 1e6);

/// The plants of the slot of starting kind `root` in cell `i` set in `groups` (each its share of the slot's built cover
/// and its plant's mass against the large plant's; `cover::set_plants`), the large plant's mass found so it stands
/// `tall` m (`Cover::height`): each group's count is its built cover over its plant's mass.
fn size_to(w: &mut World, i: usize, root: u32, groups: &[(f64, f64)], tall: f64) {
    let p = Parts::of(&w.chem).expect("chemistry 0 has cover");
    let (mm, rho) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0]);
    let x = w.life.cover(i).held().find(|&x| w.life.cover(i).root[x] == root).expect("the stand's slot");
    let built = w.life.cover(i).built(&mm, x);
    let mut set = |large: f64| {
        let g: Vec<(f64, f64)> = groups.iter().map(|&(share, of)| (share * built / (of * large), share * built)).collect();
        assert!(pcore::cover::set_plants(w, i, root, &g), "the stand's slot");
        w.life.cover(i).height(&w.cfg.phys, &mm, rho, x)
    };
    let (mut lo, mut hi) = (MASS_KG.0.ln(), MASS_KG.1.ln());
    for _ in 0..BISECTIONS {
        let mid = 0.5 * (lo + hi);
        if set(mid.exp()) < tall {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    set(hi.exp());
}

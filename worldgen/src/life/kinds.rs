//! The table of starting kinds of cover: the genes each starts with, and where, when and how much is placed.

use pcore::cover;

/// One row of the table of starting kinds: its name, when it is placed (years after the first life), the key of its
/// random draws and its label (`Cover::root`, the tape's `root_<k>_*` columns), fixed with the kind so its placing does
/// not move when the table changes; its genes (`pcore::cover::NG`), kg per m², and the chance per land cell.
pub struct Kind {
    /// What the seeding line calls it (physical terms only)
    pub name: &'static str,
    pub year: u64,
    pub key: u32,
    pub genes: [f32; cover::NG],
    pub kg: f64,
    pub chance: f64,
}

/// `s` with the genes of `d` (gene, value) set.
const fn with<const N: usize>(mut s: [f32; N], d: &[(usize, f32)]) -> [f32; N] {
    let mut i = 0;
    while i < d.len() {
        s[d[i].0] = d[i].1;
        i += 1;
    }
    s
}

/// The cover of the starting light-eaters: a light capacity that a full summer's light about fills at a capture area of
/// 2.5 m² per kg of living cover (a herb's 20 m² per kg of leaf, a quarter of its dry mass leaf, and a kg of it dry is 2
/// kg living: half of it matter, at a quarter dry), walls as much as its matter (cell walls are about half a herb's dry
/// mass), a tenth of its built cover kept as fuel, frozen below 268 K, fully active at half the ground's field capacity,
/// a tenth of what it builds to seeds, the fixing reaction off (`FIXERS` of the cells turn it on); 0.1 kg per m² on
/// every land cell.
const COVER_GENES: [f32; cover::NG] = [-5.0, 0.4, 0.0, -1.0, 268.0, -0.3, 0.1, -1.0, -11.0];
const LIGHT: Kind = Kind { name: "light-eaters", year: 0, key: 0, genes: COVER_GENES, kg: 0.1, chance: 1.0 };

/// The genes of the big slow cover, a tree: walls ten times its matter (its matter holds 2.5% of the fixed element, so
/// the whole about 0.23%, as a forest stand's mean of 0.2 to 0.4%; Cole and Rapp 1981, from memory), the herb's light
/// capacity per kg of living cover, 1 m² of capture area per kg of living cover (a leaf area of 5 m² per m² on 14 kg/m²
/// of dry matter at walls ten times its matter, the low end of temperate forests' 10 to 50 kg/m², Whittaker and Likens
/// 1975; more living cover would cost more upkeep than a stand's light stores), a fifth of its built cover kept as fuel,
/// 5% of what it builds to seeds, and the repair gene 0: its wear life is then about 40 years at `t_cat`, as forest
/// stands turn over in 50 to 100 years (tree deaths of 1 to 2% a year).
const TREE: [f32; cover::NG] = with(COVER_GENES, &[(cover::LEAF, 0.0), (cover::WALLS, 1.0), (cover::STORE, -0.7), (cover::SEED, 0.05), (cover::REPAIR, 0.0)]);

/// The genes of a big slow cover of wet ground: the tree's, but fully active only on ground nine tenths wet (`MOIST`
/// -0.05: a light capacity per kg near twice the tree's where the ground is that wet, and dormant sooner where it dries),
/// thinner leaves (1.4 m² of capture area per kg of living cover), and frozen below 272 K (tender: the cheapest frost
/// limit below the key liquid's melting point, `c_firm`). Drought-avoiding trees of wet ground hold more capacity per
/// leaf and close their pores early (Wright and others 2004, from memory).
const WET_TREE: [f32; cover::NG] = with(TREE, &[(cover::MOIST, -0.05), (cover::LEAF, 0.15), (cover::FROST, 272.0)]);

/// The genes of a woody slow cover of dry ground: walls three times its matter, thick leaves (0.5 m² of capture area per
/// kg of living cover), fully active at a tenth of the ground's wetness (`MOIST` -1: its capacity per kg the herb's at
/// that need only with a light gene 0.6 higher, so it is slower where the ground is wet), a fifth of its built cover kept
/// as fuel, slower repair, and frozen below 266 K. Plants of dry ground have thick, long-lived leaves and slow growth
/// (the leaf economics spectrum, Wright and others 2004).
const DRY_WOODY: [f32; cover::NG] = with(COVER_GENES, &[(cover::LIGHT, -4.4), (cover::LEAF, -0.3), (cover::WALLS, 0.5), (cover::STORE, -0.7), (cover::FROST, 266.0), (cover::MOIST, -1.0), (cover::SEED, 0.08), (cover::REPAIR, -0.5)]);

/// The genes of a tall fast cover: walls 2.5 times its matter in thin stems, thin leaves (3.2 m² of capture area per kg
/// of living cover), a light capacity a quarter above the herb's, fully active at a third of the ground's wetness, a
/// quarter of what it builds to seeds, and a short life (repair -1).
const TALL_FAST: [f32; cover::NG] = with(COVER_GENES, &[(cover::LIGHT, -4.9), (cover::LEAF, 0.5), (cover::WALLS, 0.4), (cover::MOIST, -0.5), (cover::SEED, 0.25)]);

/// Starting kinds of cover. A row sets only what the laws read (genes, and where, when and how much): kinds are initial
/// conditions, never rules.
pub const KINDS: [Kind; 6] = [
    LIGHT,
    // Small and fast, seeds spread wide; big and slow (`TREE`), seeds stay.
    Kind { name: "small fast light-eaters", year: 0, key: 2, genes: with(COVER_GENES, &[(cover::LIGHT, -4.83), (cover::LEAF, 0.57), (cover::WALLS, -0.3), (cover::SEED, 0.3), (cover::REPAIR, -1.2)]), kg: 0.05, chance: 0.6 },
    Kind { name: "big slow light-eaters", year: 0, key: 3, genes: TREE, kg: 0.3, chance: 0.4 },
    // At the ends of what the laws weigh against each other: big and slow on wet ground (`WET_TREE`), woody and slow on
    // dry ground (`DRY_WOODY`), tall and fast with seeds spread wide (`TALL_FAST`).
    Kind { name: "big light-eaters of wet ground", year: 0, key: 10, genes: WET_TREE, kg: 0.3, chance: 0.4 },
    Kind { name: "woody light-eaters of dry ground", year: 0, key: 11, genes: DRY_WOODY, kg: 0.1, chance: 0.5 },
    Kind { name: "tall fast light-eaters", year: 0, key: 12, genes: TALL_FAST, kg: 0.05, chance: 0.6 },
];

/// The roster of `ew run --roster`: the six rows of `KINDS`, each placed at its `year`.
pub const ROSTER: [usize; 6] = [1, 0, 2, 3, 4, 5];

/// The kind `ew run` places without `--roster` (a row of `KINDS`): the light-eaters.
pub const ONE: [usize; 1] = [0];

//! The table of starting kinds of cover: the genes each starts with, and where, when and how much is placed.

use pcore::cover;

/// One row of the table of starting kinds: its name, when it is placed (years after the first life), the key of its
/// random draws, fixed with the kind so its placing does not move when the table changes; its genes
/// (`pcore::cover::NG`), kg per m², and the chance per land cell. Its row is its label (`Cover::root`, the tape's
/// `root_<k>_*` columns).
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
/// 2.5 m² per kg of living cover (a herb's 20 m² per kg of leaf, a quarter of its dry mass leaf, and a kg of it dry is
/// 2 kg living: half of it matter, at a quarter dry), walls as much as its matter (cell walls are about half a herb's
/// dry mass), a tenth of its built cover kept as fuel, frozen below 268 K, fully active until its roots have drawn half
/// the liquid its ground holds for them (`MOIST` -0.3: FAO-56's depletion fraction of 0.5), a tenth of what it builds
/// to seeds, the fixing reaction off (`FIXERS` of the cells turn it on), and 3% of the section of its roots and stems
/// open channels (the roots of plants of dry land hold under 5.5% of air space; Justin and Armstrong 1987); 0.1 kg per
/// m² on every land cell.
const COVER_GENES: [f32; cover::NG] = [-5.0, 0.4, 0.0, -1.0, 268.0, -0.3, 0.1, -1.0, -11.0, 0.03];

/// The genes of the big slow cover, a tree: walls ten times its matter (its matter holds 2.5% of the fixed element, so
/// the whole about 0.23%, as a forest stand's mean of 0.2 to 0.4%; Cole and Rapp 1981, from memory), the herb's light
/// capacity per kg of living cover, 1 m² of capture area per kg of living cover (a leaf area of 5 m² per m² on 14 kg/m²
/// of dry matter at walls ten times its matter, the low end of temperate forests' 10 to 50 kg/m², Whittaker and Likens
/// 1975; more living cover would cost more upkeep than a stand's light stores), a fifth of its built cover kept as
/// fuel, 5% of what it builds to seeds, and the repair gene 0: its wear life is then about 40 years at `t_cat`, as
/// forest stands turn over in 50 to 100 years (tree deaths of 1 to 2% a year).
const TREE: [f32; cover::NG] = with(COVER_GENES, &[(cover::LEAF, 0.0), (cover::WALLS, 1.0), (cover::STORE, -0.7), (cover::SEED, 0.05), (cover::REPAIR, 0.0)]);

/// The genes of a big slow cover of wet ground: the tree's, but fully active only while its ground holds nine tenths of
/// what it holds for roots (`MOIST` -0.05: a light capacity per kg near twice the tree's where the ground is that wet,
/// and dormant sooner where it dries), thinner leaves (1.4 m² of capture area per kg of living cover), and frozen below
/// 272 K (tender: the cheapest frost limit below the key liquid's melting point, `c_firm`). Drought-avoiding trees of
/// wet ground hold more capacity per leaf and close their pores early (Wright and others 2004, from memory).
const WET_TREE: [f32; cover::NG] = with(TREE, &[(cover::MOIST, -0.05), (cover::LEAF, 0.15), (cover::FROST, 272.0)]);

/// The genes of a woody slow cover of dry ground: walls three times its matter, thick leaves (0.5 m² of capture area
/// per kg of living cover), fully active down to a tenth of what its ground holds for roots (`MOIST` -1: its capacity
/// per kg the herb's at that need only with a light gene 0.6 higher, so it is slower where the ground is wet), a fifth
/// of its built cover kept as fuel, slower repair, and frozen below 266 K. Plants of dry ground have thick, long-lived
/// leaves and slow growth (the leaf economics spectrum, Wright and others 2004).
const DRY_WOODY: [f32; cover::NG] = with(COVER_GENES, &[(cover::LIGHT, -4.4), (cover::LEAF, -0.3), (cover::WALLS, 0.5), (cover::STORE, -0.7), (cover::FROST, 266.0), (cover::MOIST, -1.0), (cover::SEED, 0.08), (cover::REPAIR, -0.5)]);

/// The genes of a tall fast cover: walls 2.5 times its matter in thin stems, thin leaves (3.2 m² of capture area per kg
/// of living cover), a light capacity a quarter above the herb's, fully active down to a third of what its ground holds
/// for roots, a quarter of what it builds to seeds, and a short life (repair -1).
const TALL_FAST: [f32; cover::NG] = with(COVER_GENES, &[(cover::LIGHT, -4.9), (cover::LEAF, 0.5), (cover::WALLS, 0.4), (cover::MOIST, -0.5), (cover::SEED, 0.25)]);

/// The genes of a big slow cover of soaked ground: the tree's, fully active only while its ground holds nine tenths of
/// what it holds for roots (`MOIST` -0.05, as the tree of wet ground), with 40% of the section of its roots and stems
/// open channels (the roots of plants of wetlands hold 20 to 53% of air space; Justin and Armstrong 1987). Its channels
/// carry the air's gas 0.46 m down at `t_cat` (`cover::soaked`: L = (2 D ε C / ((1 − ε) q))^½, D 2e-5 m²/s, C 8.8 mol
/// per m³ of the gas at 21% of the air, q 1.1e-3 mol a m³ a s of its roots' basal upkeep), against 0.10 m at the
/// others' 3%. Its fine roots lie as everyone's, a share e^(−z / 0.3 m) below depth z (`fine_root_m`), so on ground
/// soaked to its top a plant 20 m tall (roots 2 m deep) splits its fuel in 21% of them, not 72%: its roots' work costs
/// 0.79 + 0.21 × 13.1 = 3.6 times the work on dry ground, against 9.7 times, and a plant under 4.6 m none. It pays for
/// its channels in every month: 1 / (1 − 0.4) = 1.67 times the fine roots for the liquid it draws and the conducting
/// walls for its leaves (`cover::upkeep`), so on soaked ground its roots cost 6.0 times what a dry-ground plant's do
/// on dry ground, the others' 9.7, and on dry ground 1.67 times the others'.
const SOAKED_TREE: [f32; cover::NG] = with(TREE, &[(cover::MOIST, -0.05), (cover::CHANNEL, 0.4)]);

/// Starting kinds of cover, in the order the roster places them. A row sets only what the laws read (genes, and where,
/// when and how much): kinds are initial conditions, never rules.
pub const KINDS: [Kind; 7] = [
    // Small and fast, seeds spread wide; the light-eaters (`COVER_GENES`); big and slow (`TREE`), seeds stay.
    Kind { name: "small fast light-eaters", year: 0, key: 2, genes: with(COVER_GENES, &[(cover::LIGHT, -4.83), (cover::LEAF, 0.57), (cover::WALLS, -0.3), (cover::SEED, 0.3), (cover::REPAIR, -1.2)]), kg: 0.05, chance: 0.6 },
    Kind { name: "light-eaters", year: 0, key: 0, genes: COVER_GENES, kg: 0.1, chance: 1.0 },
    Kind { name: "big slow light-eaters", year: 0, key: 3, genes: TREE, kg: 0.3, chance: 0.4 },
    // At the ends of what the laws weigh against each other: big and slow on wet ground (`WET_TREE`), woody and slow on
    // dry ground (`DRY_WOODY`), tall and fast with seeds spread wide (`TALL_FAST`).
    Kind { name: "big light-eaters of wet ground", year: 0, key: 10, genes: WET_TREE, kg: 0.3, chance: 0.4 },
    Kind { name: "woody light-eaters of dry ground", year: 0, key: 11, genes: DRY_WOODY, kg: 0.1, chance: 0.5 },
    Kind { name: "tall fast light-eaters", year: 0, key: 12, genes: TALL_FAST, kg: 0.05, chance: 0.6 },
    // Big and slow on soaked ground, its roots fed the air's gas by open channels (`SOAKED_TREE`).
    Kind { name: "big light-eaters of soaked ground", year: 0, key: 13, genes: SOAKED_TREE, kg: 0.3, chance: 0.4 },
];

/// The roster of `ew run --roster`: the seven rows of `KINDS`, each placed at its `year`.
pub const ROSTER: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];

/// The row of the light-eaters in `KINDS`.
pub const LIGHT_ROW: usize = 1;

/// The kind `ew run` places without `--roster` (a row of `KINDS`): the light-eaters.
pub const ONE: [usize; 1] = [LIGHT_ROW];

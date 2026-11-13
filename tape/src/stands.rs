//! The cover of a cell as a viewer draws it (laws.md 3, docs/tape.md): each of its slots, and of each its plants. The
//! model holds each slot's mass, its count of plants, the loose ground their roots need and the share of the cell that
//! holds them; a plant's height and its stem come from the cover's own law (`cover::height`, `cover::stem`: the buckling
//! limit, McMahon 1973), and the crown from its share of the ground. The page only draws what they give.

use pcore::cover::{self, dec, Cover, Parts, FROST, FUEL, LEAF, LIGHT, MOIST, NP, SEED, SLOTS, STORE};
use pcore::{Physics, World};

/// Kinds a square's row holds at most: the slots of its cell.
pub const KINDS: usize = SLOTS;
/// Numbers a kind holds in a square's row (`row`).
pub const KIND_ROW: usize = 19;

/// The plants of a stand: how many a m², each one's mass, kg, height, stem's width and crown's width, m, the share of
/// the ground the crowns cover, and how closed one crown is (the light it stops, as the cover's light law: 1 - e^-leaf
/// area, `math::shade`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Plants {
    pub per_m2: f64,
    pub mass: f64,
    pub height: f64,
    pub stem: f64,
    pub width: f64,
    pub cover: f64,
    pub crown: f64,
}

/// The plants of a stand of `kg` built cover, `leaf` capture area and `per_m2` plants per m² of its cell, standing on
/// the share `rooted` of it, of which `walls` is the share its walls make, of walls of density `rho` kg/m³. A kind
/// stands on the share of its cell that holds its roots alone, its capture area a layer of leaf / rooted there (laws.md
/// 4.5): its crowns cover of that ground the share whose light the layer stops, 1 - e^-(leaf / rooted), the cover's
/// own light law (`math::shade`), so a stand the law holds closed to the light is drawn closed; they stand in patches
/// where they close, each plant's crown the ground it holds there; its stem and height are the cover's law for a plant
/// of its mass (`cover::stem`, `cover::height`).
pub fn plants(ph: &Physics, (kg, leaf, per_m2): (f64, f64, f64), rooted: f64, walls: f64, rho: f64) -> Plants {
    if kg <= 0.0 || leaf <= 0.0 || per_m2 <= 0.0 || rooted <= 0.0 {
        return Plants::default();
    }
    let cover = -rooted * (-leaf / rooted).exp_m1();
    let (mass, inside) = (kg / per_m2, per_m2 / cover);
    let width = (1.0 / (std::f64::consts::FRAC_PI_4 * inside)).sqrt();
    let (stem, height) = (cover::stem(ph, mass, walls, rho), cover::height(ph, mass, walls, rho));
    Plants { per_m2, mass, height, stem, width, cover, crown: -(-leaf / cover).exp_m1() }
}

/// What a slot's row reads of its cell: how wet its ground is (`cover::wet`), its temperature and the melting point of
/// its key liquid, K, the density of its walls, kg/m³, and its area, m².
struct Ground {
    wet: f64,
    t: f64,
    melt: f64,
    rho: f64,
    area: f64,
}

/// The numbers a viewer draws slot `x` of cover `c` from, `KIND_ROW` of them, on the ground `g` of its cell: its
/// starting kind; its built cover, kg/m²; its plants (`plants`): the stem's width, m, how many a m² (`Cover::plants`),
/// height and crown's width, m, the share of the ground they cover, how closed a crown is; its walls' share; its capture
/// area per kg of living cover, log10 m²/kg (`LEAF`); its light capacity per m² of capture area, log10 mol/m²/s (more
/// pigment per area); its moisture gene (`MOIST`) and frost gene, K (`FROST`); its fuel over its reserve (`STORE`); its
/// activity there (`cover::activity`); the share of it frozen this month (`cover::frozen`); the share of what it builds
/// that goes to seeds (`SEED`); the share of the cell whose loose ground holds its plants (`Cover::rooted`) and the
/// loose ground their roots need, m (`Cover::need`; all of the cell and 0 for a slot whose need is not found yet: seeds
/// that landed since its cell's last month).
fn row(ph: &Physics, c: &Cover, mm: &[f64; NP], x: usize, g: &Ground) -> [f64; KIND_ROW] {
    let (built, genes) = (c.built(mm, x), c.genes[x]);
    let walls = c.walls(mm, x) / built.max(1e-300);
    let (rooted, need) = if c.need[x] > 0.0 { (c.rooted[x], c.need[x]) } else { (1.0, 0.0) };
    let p = plants(ph, (built / g.area, c.leaf(mm, x) / g.area, c.plants[x] / g.area), rooted as f64, walls, g.rho);
    let act = cover::activity(ph, &genes, g.wet);
    let reserve = dec(genes[STORE]) * built;
    let gene = |k: usize| genes[k] as f64;
    [
        c.root[x] as f64,
        built / g.area,
        p.stem,
        p.per_m2,
        p.height,
        p.width,
        p.cover,
        p.crown,
        walls,
        gene(LEAF),
        gene(LIGHT) + gene(MOIST) - gene(LEAF),
        gene(MOIST),
        gene(FROST),
        c.mol[x][FUEL] * mm[FUEL] / reserve.max(1e-300),
        act,
        cover::frozen(ph, g.melt, gene(FROST), (g.t, act)),
        gene(SEED),
        rooted as f64,
        need as f64,
    ]
}

/// The kinds of cell `i` of `w`, `KINDS` rows of `KIND_ROW` numbers (`row`): its slots, the heaviest first; an empty
/// kind is all 0 with its starting kind -1.
pub fn rows(w: &World, i: usize) -> [f64; KINDS * KIND_ROW] {
    let mut out = [0.0; KINDS * KIND_ROW];
    (0..KINDS).for_each(|k| out[k * KIND_ROW] = -1.0);
    let Some(p) = Parts::of(&w.chem) else { return out };
    let (mm, c) = (p.mm(&w.chem), w.life.cover(i));
    let g = Ground {
        wet: cover::wet(w, &p, i, w.g(p.liquid, i)).1,
        t: w.t_ground_last(i),
        melt: w.chem.subs[p.liquid].melt,
        rho: w.chem.subs[p.sub[cover::WALL]].density[0],
        area: w.mesh.cells().area()[i],
    };
    let mut slots: Vec<usize> = c.held().collect();
    slots.sort_by(|&a, &b| c.built(&mm, b).total_cmp(&c.built(&mm, a)));
    for (k, &x) in slots.iter().enumerate() {
        out[k * KIND_ROW..(k + 1) * KIND_ROW].copy_from_slice(&row(&w.cfg.phys, c, &mm, x, &g));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows hold the cell's cover slot by slot: two slots of one starting kind, far apart in their genes, make two
    /// rows, the heaviest first, whose built cover sums to the cell's and whose plants are each slot's own count.
    #[test]
    fn rows_hold_the_cells_slots() {
        let mut w = worldgen::testkit::small_world();
        let i = worldgen::testkit::mild_land(&w);
        worldgen::testkit::cover(&mut w, i, 0.4);
        let mut far = worldgen::life::KINDS[worldgen::life::LIGHT_ROW].genes;
        far[FROST] -= 30.0;
        assert!(cover::place(&mut w, i, far, worldgen::life::LIGHT_ROW as u32, 0.2), "no room for the second slot");
        let p = Parts::of(&w.chem).expect("chemistry 0 has cover");
        let (mm, area, c) = (p.mm(&w.chem), w.mesh.cells().area()[i], w.life.cover(i));
        let all: f64 = c.held().map(|x| c.built(&mm, x)).sum::<f64>() / area;
        let r = rows(&w, i);
        let kind = |k: usize| &r[k * KIND_ROW..(k + 1) * KIND_ROW];
        assert!(kind(0)[1] > kind(1)[1] && kind(1)[1] > 0.0 && kind(2)[0] == -1.0, "two rows, the heaviest first");
        assert!(((kind(0)[1] + kind(1)[1]) / all - 1.0).abs() < 1e-12, "{} and {} against {all}", kind(0)[1], kind(1)[1]);
        let counts: Vec<f64> = c.held().map(|x| c.plants[x] / area).collect();
        assert!((0..2).all(|k| counts.iter().any(|&n| (kind(k)[3] / n - 1.0).abs() < 1e-9)), "each row's plants are a slot's");
    }
}

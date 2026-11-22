//! The cover of a cell as a viewer draws it (laws.md 3, docs/tape.md): each of its slots, and of each its plants. The
//! model holds each slot's mass, its plants by size group (`cover::Sizes`), the loose ground their roots need and the
//! share of the cell that holds them; a plant's height and its stem come from the cover's own law (`cover::height`,
//! `cover::stem`: the buckling limit, McMahon 1973), and the crown from its share of the ground. The page only draws what they give.

use pcore::cover::{self, dec, Cover, Parts, FROST, FUEL, GROUPS, LEAF, LIGHT, MOIST, NP, SEED, STORE, WALLS};
use crate::layers::per_land;
use pcore::{Physics, World};

/// Numbers a kind holds in a square's row (`row`): `KIND_HEAD`, then `GROUP_ROW` for each of its groups of plants, then
/// `KIND_TAIL`.
pub const KIND_ROW: usize = KIND_HEAD + GROUPS * GROUP_ROW + KIND_TAIL;
/// The numbers of a kind's row before its groups.
const KIND_HEAD: usize = 22;
/// The numbers of a group of plants in a kind's row.
const GROUP_ROW: usize = 4;
/// The numbers of a kind's row after its groups: where it stands under the liquid, and how dense on its land's
/// saturated strips and on its drier rest (`dense`).
const KIND_TAIL: usize = 4;

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
/// 3): its crowns cover of that ground the share whose light the layer stops, 1 - e^-(leaf / rooted), the cover's
/// own light law (`math::shade`), so a stand the law holds closed to the light is drawn closed; they stand in patches
/// where they close, each plant's crown the ground it holds there; its stem and height are the cover's law for a plant
/// of its mass (`cover::stem`, `cover::height`). Plants with no capture area still stand, bare: their crowns cover no
/// ground, of no width and not closed (the limit of a crown as its leaves fall to none), so a viewer draws them by their
/// stems as it draws a crown that has shed its leaves.
pub fn plants(ph: &Physics, (kg, leaf, per_m2): (f64, f64, f64), rooted: f64, walls: f64, rho: f64) -> Plants {
    if kg <= 0.0 || per_m2 <= 0.0 || rooted <= 0.0 {
        return Plants::default();
    }
    let mass = kg / per_m2;
    let (stem, height) = (cover::stem(ph, mass, walls, rho), cover::height(ph, mass, walls, rho));
    if leaf <= 0.0 {
        return Plants { per_m2, mass, height, stem, ..Plants::default() };
    }
    let cover = -rooted * (-leaf / rooted).exp_m1();
    let inside = per_m2 / cover;
    let width = (1.0 / (std::f64::consts::FRAC_PI_4 * inside)).sqrt();
    Plants { per_m2, mass, height, stem, width, cover, crown: -(-leaf / cover).exp_m1() }
}

/// How dense a slot's plants stand on its land's saturated strips and on its drier rest, over their mean on its land,
/// [strips, rest]: of its plants on its land the share `strip` stand on the strips (`Cover::strip`), which are the share
/// `saturated` of the land (`WetGround::share`, as the month's start found it). The law knows no other place in a land:
/// these are its split, and their mean over the land is 1.
fn dense(strip: f64, saturated: f64) -> [f64; 2] {
    if saturated <= 0.0 {
        [0.0, 1.0]
    } else if saturated >= 1.0 {
        [1.0, 0.0]
    } else {
        [strip / saturated, (1.0 - strip) / (1.0 - saturated)]
    }
}

/// What a slot's row reads of its cell `i`: the height of its liquid's level over its lowest ground, m
/// (`flow::levels`), what its cover's slots ran at in its last month (`Counters::ran_month`; none before its first),
/// the share of its land saturated (`WetGround::share`; none where the month's start found no wet ground), the density
/// of its walls, kg/m³, and its area, m².
struct Ground<'a> {
    level: f64,
    ran: Option<&'a [cover::Ran; cover::SLOTS]>,
    saturated: f64,
    rho: f64,
    area: f64,
}

/// The numbers a viewer draws slot `x` of cover `c` of world `w` from, `KIND_ROW` of them, on the ground of its cell `g`
/// it stands on (each per m² of that ground: its patch's share of the cell, `Cover::patch_share`, and of that its share
/// `Cover::stands`: its land, and its ground under the liquid where its roots work, `cover::footing`): its starting kind; its built cover,
/// kg/m²; its large plants (`plants` of its large group, `cover::Sizes::large`, with that group's built cover and capture
/// area): the stem's width, m, how many a m², height and crown's width, m, the share of the ground they cover, how
/// closed a crown is; its walls' share; its capture
/// area per kg of living cover, log10 m²/kg (`LEAF`); its light capacity per m² of capture area, log10 mol/m²/s (more
/// pigment per area); its moisture gene (`MOIST`) and frost gene, K (`FROST`); its fuel over its reserve (`STORE`); its
/// activity there and the share of it frozen in its cell's last month, as the law used them (`cover::Ran`; a slot that
/// has not run since it took hold, its `Cover::need` not found yet, is active and not frozen); the share of what it builds
/// that goes to seeds (`SEED`); the share of the ground it stands on whose loose ground holds its plants (`Cover::rooted`, a share of the
/// ground its cover lives on) and the loose ground their roots need, m (`Cover::need`; all of the land and 0 for a slot
/// whose need is not found yet: seeds that landed since its cell's last month); and its walls gene, log10 kg of walls
/// per kg of matter (`WALLS`), by which a viewer knows a woody kind while its plants are young; its slot, which it
/// keeps while it lives (a slot that joins another is gone, the other keeps its own), so a viewer tells two slots of
/// one starting kind apart from month to month; and the patch of its cell's cover it stands in (`Cover::patch`), whose
/// ground a viewer draws it on (`inside::patch_ground`). Then of each of its
/// groups, the large plants first (empty groups last, all 0): how many a m², and a plant's height, stem's width and
/// crown's width, m (`plants` of that group). Then where it stands: how deep under its cell's level its line lies, m
/// (`Cover::line`: it stands on the ground above it, on land and under the liquid as deep as that; plants do not move,
/// so as the level rises by a metre this deepens by a metre, and a viewer draws them on the same ground; 0 before its
/// line is found), and the share of its ground that lies under the liquid in its last month (`cover::Ran`). Then how
/// dense its plants stand on its land's saturated strips and on its drier rest, over their mean on its land (`dense`).
fn row(w: &World, c: &Cover, mm: &[f64; NP], x: usize, g: &Ground) -> [f64; KIND_ROW] {
    let ph = &w.cfg.phys;
    let (built, genes) = (c.built(mm, x), c.genes[x]);
    let (rooted, need) = if c.need[x] > 0.0 { (c.rooted[x] as f64, c.need[x]) } else { (1.0, 0.0) };
    let per_ground = |v: f64| per_land(v / g.area, c.patch_share(x) * c.stands[x] as f64);
    let ran = g.ran.filter(|_| c.need[x] > 0.0).map_or(cover::Ran { act: 1.0, ..Default::default() }, |r| r[x]);
    let s = c.sizes(ph, mm, x);
    let group = |k: usize| plants(ph, (per_ground(s.mass[k] * s.count[k]), per_ground(c.leaf(mm, x) * s.leaf[k]), per_ground(s.count[k])), rooted, s.walls[k], g.rho);
    let p = group(s.large());
    let reserve = dec(genes[STORE]) * built;
    let gene = |k: usize| genes[k] as f64;
    let mut out = [0.0; KIND_ROW];
    out[..KIND_HEAD].copy_from_slice(&[
        c.root[x] as f64,
        per_ground(built),
        p.stem,
        p.per_m2,
        p.height,
        p.width,
        p.cover,
        p.crown,
        c.walls(mm, x) / built.max(1e-300),
        gene(LEAF),
        gene(LIGHT) + gene(MOIST) - gene(LEAF),
        gene(MOIST),
        gene(FROST),
        c.mol[x][FUEL] * mm[FUEL] / reserve.max(1e-300),
        ran.act as f64,
        ran.frozen as f64,
        gene(SEED),
        rooted,
        need as f64,
        gene(WALLS),
        x as f64,
        c.patch[x] as f64,
    ]);
    for (k, at) in (0..s.held).rev().zip((KIND_HEAD..KIND_ROW).step_by(GROUP_ROW)) {
        let q = group(k);
        out[at..at + GROUP_ROW].copy_from_slice(&[q.per_m2, q.height, q.stem, q.width]);
    }
    let deep = if c.line[x] >= 0.0 { g.level - c.line[x] as f64 } else { 0.0 };
    let [strips, rest] = dense(c.strip[x] as f64, g.saturated);
    out[KIND_ROW - KIND_TAIL..].copy_from_slice(&[deep, ran.wet as f64, strips, rest]);
    out
}

/// Numbers of the places of a cell's patches (`places`): three for each patch past the first.
pub const PLACE_ROW: usize = 3 * (cover::PATCHES - 1);

/// Where the patches of the cover of cell `i` of `w` past the first lie (`inside::patch_ground`), in squares from the
/// map's north-west corner: of each its point, x and y, and its reach (all 0 for a patch not in use). A viewer draws a
/// point of the cell in the first of them whose reach holds it, else in the first patch: the ground the samples give,
/// its edge drawn smooth between them.
pub fn places(w: &World, i: usize) -> [f64; PLACE_ROW] {
    let g = pcore::inside::patch_ground(w, i);
    let mut out = [0.0; PLACE_ROW];
    for (k, &([x, y], reach)) in g.reach.iter().enumerate().skip(1).filter(|(_, p)| p.1 > 0.0) {
        out[3 * (k - 1)..3 * k].copy_from_slice(&[g.bounds[0] + x / w.map.side, g.bounds[1] + y / w.map.side, reach / w.map.side]);
    }
    out
}

/// The kinds a square holds in the rows of world `w` (`rows`): the slots its world uses in a cell of one patch
/// (`Config::variety`), or as many as the cell that holds the most (a cell of patches uses up to `cover::SLOTS`).
pub fn kinds(w: &World) -> usize {
    (0..w.mesh.cells().n()).map(|i| standing(w.life.cover(i)).len()).fold(w.cfg.variety, usize::max)
}

/// The slots of cover `c` that stand on any of its cell's ground.
fn standing(c: &Cover) -> Vec<usize> {
    c.held().filter(|&x| c.stands[x] > 0.0).collect()
}

/// The kinds of cell `i` of `w`, a row of `KIND_ROW` numbers (`row`) for each of `kinds` slots (`kinds`): its slots
/// that stand on any of its ground, the heaviest first; an empty kind is all 0 with its starting kind -1. Its
/// liquid's level stands `level` m over its lowest ground (`flow::levels`, which a frame finds for all cells at once;
/// 0 before Flow first built its state).
pub fn rows(w: &World, i: usize, level: f64, kinds: usize) -> Vec<f64> {
    let mut out = vec![0.0; kinds * KIND_ROW];
    out.iter_mut().step_by(KIND_ROW).for_each(|r| *r = -1.0);
    let Some(p) = Parts::of(&w.chem) else { return out };
    let (mm, c) = (p.mm(&w.chem), w.life.cover(i));
    let saturated = w.life.wet.get(i).copied().flatten().map_or(0.0, |g| g.share);
    let g = Ground { level, ran: w.counts.ran_month.get(i).map(|r| &r.slots), saturated, rho: w.chem.subs[p.sub[cover::WALL]].density[0], area: w.mesh.cells().area()[i] };
    let mut slots = standing(c);
    slots.sort_by(|&a, &b| c.built(&mm, b).total_cmp(&c.built(&mm, a)));
    for (k, &x) in slots.iter().enumerate() {
        out[k * KIND_ROW..(k + 1) * KIND_ROW].copy_from_slice(&row(w, c, &mm, x, &g));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows hold the cell's cover slot by slot: two slots of one starting kind, far apart in their genes, make two
    /// rows, the heaviest first, whose built cover is each slot's and whose plants are each slot's own count, per m² of
    /// the ground it stands on (`Cover::stands`). Plants with no capture area stand
    /// as plants of very little capture area do, bare: their crowns of no width, covering nothing.
    #[test]
    fn rows_hold_the_cells_slots() {
        let mut w = worldgen::testkit::small_world();
        let i = worldgen::testkit::mild_land(&w);
        worldgen::testkit::cover(&mut w, i, 0.4);
        let mut far = worldgen::life::KINDS[worldgen::life::LIGHT_ROW].genes;
        far[FROST] -= 30.0;
        assert!(pcore::cover::place(&mut w, i, 0, far, worldgen::life::LIGHT_ROW as u32, 0.2), "no room for the second slot");
        let p = Parts::of(&w.chem).expect("chemistry 0 has cover");
        let (mm, area, c) = (p.mm(&w.chem), w.mesh.cells().area()[i], w.life.cover(i));
        let own = |x: usize| area * c.patch_share(x) * c.stands[x] as f64;
        let all: f64 = c.held().map(|x| c.built(&mm, x) / own(x)).sum::<f64>();
        let r = rows(&w, i, 0.0, w.cfg.variety);
        let kind = |k: usize| &r[k * KIND_ROW..(k + 1) * KIND_ROW];
        assert!(kind(0)[1] > kind(1)[1] && kind(1)[1] > 0.0 && kind(2)[0] == -1.0, "two rows, the heaviest first");
        assert!(((kind(0)[1] + kind(1)[1]) / all - 1.0).abs() < 1e-12, "{} and {} against {all}", kind(0)[1], kind(1)[1]);
        let counts: Vec<f64> = c.held().map(|x| c.count(x) / own(x)).collect();
        assert!((0..2).all(|k| counts.iter().any(|&n| (kind(k)[3] / n - 1.0).abs() < 1e-9)), "each row's plants are a slot's");
        assert!(kind(0)[KIND_HEAD - 2] != kind(1)[KIND_HEAD - 2], "each row names its own slot");
        let stand = |leaf: f64| plants(&w.cfg.phys, (2.0, leaf, 0.05), 0.8, 0.9, 500.0);
        let (bare, leafy) = (stand(0.0), stand(1e-12));
        assert!(bare.per_m2 == leafy.per_m2 && bare.mass == leafy.mass && bare.height == leafy.height && bare.stem == leafy.stem, "{bare:?} against {leafy:?}");
        assert!(bare.height > 0.0 && bare.width == 0.0 && bare.cover == 0.0 && bare.crown == 0.0 && leafy.width < 1e-5, "bare: {bare:?}");
    }

    /// A slot of a patch is drawn per m² of its patch's ground: cover placed in a clearing of a quarter of a cell at
    /// `PLACED` kg per m² of the ground it stands on there (`cover::place`, its built cover that less its store) reads as
    /// that in its row, not a quarter of it.
    #[test]
    fn a_patchs_slot_is_per_m2_of_its_patch() {
        const PLACED: f64 = 0.1;
        let mut w = worldgen::testkit::small_world();
        let i = worldgen::testkit::mild_land(&w);
        worldgen::testkit::cover(&mut w, i, 0.4);
        assert!(w.clear(i, 0.25, None), "the clearing");
        let (row, genes) = (worldgen::life::LIGHT_ROW, worldgen::life::KINDS[worldgen::life::LIGHT_ROW].genes);
        assert!(pcore::cover::place(&mut w, i, 1, genes, row as u32, PLACED), "the cover in the clearing");
        let built = PLACED / (1.0 + dec(genes[STORE]));
        let r = rows(&w, i, 0.0, cover::SLOTS);
        let kg = (0..cover::SLOTS).map(|k| &r[k * KIND_ROW..(k + 1) * KIND_ROW]).find(|k| k[KIND_HEAD - 1] == 1.0).expect("a row in the clearing")[1];
        assert!((kg / built - 1.0).abs() < 1e-6, "{kg} kg/m2 drawn of {built} built");
    }
}

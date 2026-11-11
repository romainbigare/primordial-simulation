//! The kinds of cover of a cell as a viewer draws them (laws.md 4.5, docs/tape.md): the slots of one starting kind
//! joined, and of each kind the plants its mass makes. The model holds kg per m², not plants: how many plants and how big
//! follow from the stand's mass by the self-thinning law (Yoda and others 1963), the height from a plant's walls by the
//! buckling limit (McMahon 1973), the crown from its share of the ground. One place for these laws; the page only
//! draws what they give.

use pcore::cover::{self, dec, Cover, Parts, FROST, FUEL, LEAF, LIGHT, MOIST, NG, NO_ROOT, NP, SEED, SLOTS, STORE};
use pcore::World;

/// Kinds a square's row holds at most: the slots of a cell joined by their starting kind.
pub const KINDS: usize = SLOTS;
/// Numbers a kind holds in a square's row (`Kind::row`).
pub const KIND_ROW: usize = 17;

/// The self-thinning law, mean plant mass = `THIN_KG` × (plants per m²)^(-3/2) (Yoda and others 1963; White 1981): a
/// stand of B kg/m² that has thinned holds (`THIN_KG` / B)² plants a m². kg (per m² to the 1/2). Forests of 10 to 40
/// kg/m² hold 0.2 to 0.03 stems a m² (Whittaker and Likens 1975 for the mass; the counts from memory): 3 to 8 kg.
pub const THIN_KG: f64 = 5.0;
/// A plant's height, m, is `TALL` × (its stem's width, m)^(2/3): the elastic buckling limit (McMahon 1973), trees standing
/// at about a quarter of the height at which they would buckle; McMahon's trees, about 35 (from memory).
pub const TALL: f64 = 35.0;
/// A stem's volume over that of a cylinder as wide and tall: about a half (a cone is a third; stem form factors 0.4 to
/// 0.5, from memory).
pub const STEM_FORM: f64 = 0.5;
/// The share of a stem's volume that is its walls' solid: wood of 300 to 900 kg/m³ is cell walls of about 1,500 kg/m³
/// with their hollows (Gibson and Ashby 1997, from memory).
pub const STEM_SOLID: f64 = 0.4;
/// The leaf area per m² of ground at which a stand's crowns close the ground to e^-1 of it: a stand whose crowns stand
/// in patches is that closed inside them (canopies close at a leaf area index of 3 to 4; Jarvis and Leverenz 1983, from
/// memory). m² per m².
pub const CLOSE_LAI: f64 = 3.0;

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

/// The plants of a stand of `kg` built cover and `leaf` capture area per m², of which `walls` is the share its walls
/// make, of walls of density `rho` kg/m³. Its crowns cover the share 1 - e^(-leaf / `CLOSE_LAI`) of the ground, in
/// patches where they close; there it has thinned (`THIN_KG`), each plant's crown the ground it holds; its stem is the
/// cylinder of `STEM_FORM` and `STEM_SOLID` that holds its walls, as tall as the buckling limit lets it (`TALL`).
pub fn plants(kg: f64, leaf: f64, walls: f64, rho: f64) -> Plants {
    if kg <= 0.0 || leaf <= 0.0 {
        return Plants::default();
    }
    let cover = -(-leaf / CLOSE_LAI).exp_m1();
    let (dense, lai) = (kg / cover, leaf / cover);
    let inside = (THIN_KG / dense).powi(2);
    let mass = dense / inside;
    // walls = form × solid × rho × π/4 × D² × TALL × D^(2/3)
    let stem = (mass * walls / (STEM_FORM * STEM_SOLID * rho * std::f64::consts::FRAC_PI_4 * TALL)).powf(3.0 / 8.0);
    let width = (1.0 / (std::f64::consts::FRAC_PI_4 * inside)).sqrt();
    Plants { per_m2: inside * cover, mass, height: TALL * stem.powf(2.0 / 3.0), stem, width, cover, crown: -(-lai).exp_m1() }
}

/// A kind of cover in a cell: its slots of one starting kind joined, their genes the mean by built cover.
#[derive(Clone, Copy, Debug)]
pub struct Kind {
    pub root: u32,
    /// Built cover, fuel, walls and capture area per m², kg and m²
    pub kg: f64,
    pub fuel: f64,
    pub walls: f64,
    pub leaf: f64,
    pub genes: [f64; NG],
}

impl Kind {
    /// The numbers a viewer draws it from, `KIND_ROW` of them, for a cell whose ground is as wet as `wet`
    /// (`cover::wet`) and at `t` K, its walls of density `rho`: its starting kind; its built cover, kg/m²; its plants
    /// (`plants`): the stem's width, m, a m², height and crown's width, m, the share of the ground they cover, how closed
    /// a crown is; its walls' share; its capture area per kg of living cover, log10 m²/kg (`LEAF`); its light capacity
    /// per m² of capture area, log10 mol/m²/s (more pigment per area); its moisture gene (`MOIST`) and frost gene, K
    /// (`FROST`); its fuel over its reserve (`STORE`); its activity there (`cover::activity`); the share of it
    /// frozen this month (`Physics::frozen`); and the share of what it builds that goes to seeds (`SEED`).
    pub fn row(&self, ph: &pcore::Physics, wet: f64, t: f64, rho: f64) -> [f64; KIND_ROW] {
        let g = self.genes;
        let p = plants(self.kg, self.leaf, self.walls / self.kg.max(1e-300), rho);
        let genes: [f32; NG] = g.map(|x| x as f32);
        let act = cover::activity(ph, &genes, wet);
        let reserve = dec(genes[STORE]) * self.kg;
        [
            self.root as f64, self.kg, p.stem, p.per_m2, p.height, p.width, p.cover, p.crown,
            self.walls / self.kg.max(1e-300), g[LEAF], g[LIGHT] + g[MOIST] - g[LEAF], g[MOIST], g[FROST],
            self.fuel / reserve.max(1e-300), act, ph.frozen(g[FROST], t, act), g[SEED],
        ]
    }
}

/// The kinds of cover `c` of a cell of `area` m² (`mm` the molar masses of its parts), the heaviest first.
pub fn kinds(c: &Cover, mm: &[f64; NP], area: f64) -> Vec<Kind> {
    let mut out: Vec<Kind> = Vec::new();
    for x in c.held().filter(|&x| c.root[x] != NO_ROOT) {
        let built = c.built(mm, x);
        let k = match out.iter_mut().position(|k| k.root == c.root[x]) {
            Some(at) => &mut out[at],
            None => {
                out.push(Kind { root: c.root[x], kg: 0.0, fuel: 0.0, walls: 0.0, leaf: 0.0, genes: [0.0; NG] });
                out.last_mut().expect("just pushed")
            }
        };
        // The genes the mean by built cover.
        let f = built / (k.kg * area + built).max(1e-300);
        k.genes.iter_mut().zip(c.genes[x]).for_each(|(a, b)| *a += f * (b as f64 - *a));
        k.kg += built / area;
        k.fuel += c.mol[x][FUEL] * mm[FUEL] / area;
        k.walls += c.walls(mm, x) / area;
        k.leaf += c.leaf(mm, x) / area;
    }
    out.sort_by(|a, b| b.kg.total_cmp(&a.kg));
    out
}

/// The kinds of cell `i` of `w`, `KINDS` rows of `KIND_ROW` numbers (`Kind::row`); an empty kind is all 0 with its
/// starting kind -1.
pub fn rows(w: &World, i: usize) -> [f64; KINDS * KIND_ROW] {
    let mut out = [0.0; KINDS * KIND_ROW];
    (0..KINDS).for_each(|k| out[k * KIND_ROW] = -1.0);
    let Some(p) = Parts::of(&w.chem) else { return out };
    let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area()[i]);
    let wet = cover::wet(w, &p, i, w.g(p.liquid, i)).1;
    let (t, rho) = (w.t_ground_last(i), w.chem.subs[p.sub[cover::WALL]].density[0]);
    for (k, kind) in kinds(w.life.cover(i), &mm, area).iter().take(KINDS).enumerate() {
        out[k * KIND_ROW..(k + 1) * KIND_ROW].copy_from_slice(&kind.row(&w.cfg.phys, wet, t, rho));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand's plants a m² times the mean plant's mass is its mass, at any closure.
    #[test]
    fn plants_times_their_mass_is_the_stand() {
        for (kg, leaf, walls) in [(0.05, 0.1, 0.3), (0.5, 2.0, 0.5), (14.0, 5.0, 0.9), (40.0, 8.0, 0.95)] {
            let p = plants(kg, leaf, walls, 1500.0);
            assert!((p.per_m2 * p.mass / kg - 1.0).abs() < 1e-12, "{kg} kg/m²: {p:?}");
        }
    }

    /// Joined kinds hold the cell's cover: two slots of one starting kind, far apart in their genes, make one kind of
    /// their summed mass.
    #[test]
    fn kinds_hold_the_cells_cover() {
        let mut w = worldgen::testkit::small_world();
        let i = worldgen::testkit::mild_land(&w);
        worldgen::testkit::cover(&mut w, i, 0.4);
        let worldgen::life::Place::Field(f) = &worldgen::life::KINDS[0].place else { unreachable!("kind 0 is cover") };
        let mut far = f.genes;
        far[FROST] -= 30.0;
        assert!(cover::place(&mut w, i, far, 0, 0.2), "no room for the second slot");
        let p = Parts::of(&w.chem).expect("chemistry 0 has cover");
        let (mm, area, c) = (p.mm(&w.chem), w.mesh.cells().area()[i], w.life.cover(i));
        assert_eq!(c.held().count(), 2, "two slots of one kind");
        let all: f64 = c.held().map(|x| c.built(&mm, x)).sum::<f64>() / area;
        let joined = kinds(c, &mm, area);
        assert_eq!(joined.len(), 1);
        assert!((joined[0].kg / all - 1.0).abs() < 1e-12, "{} against {all}", joined[0].kg);
    }
}

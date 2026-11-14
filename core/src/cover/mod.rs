//! The cover: the light-eaters as a field (laws.md 4). Each ground cell holds up to its world's `Config::variety` kinds
//! of cover, each the amounts of a few substances (`Parts`: the fuel its light reaction makes, body matter, and the solid
//! it builds) and a small vector of genes, as plain arrays per cell (`Cover`). Once a month each cell's cover catches light,
//! runs its light reaction, pays its upkeep, fixes, builds and dies (`grow`), and sends seeds to the cells around
//! (`seed`), where they lie on the ground until it lets them grow (`lying`); alike kinds share a slot (`life::kin`). The
//! cover of a cell, with the seeds lying on its ground, is a node of the store (`Class::Life`) whose law books its own
//! side; what takes from it or presses on it does so through the contract (`contract`), whose cover side is `show`.

mod around;
mod budget;
mod build;
mod grow;
mod lying;
mod month;
mod seed;
mod show;
mod stand;
mod upkeep;

pub use around::{pores, thirst, Around};
pub use budget::{content, BUDGET, TAKERS};
pub use grow::{live, step, Books};
pub use lying::{Lying, FROZEN, LANDED, LOTS_LYING, SEED_FLOWS, SPROUTED, TAKEN_LYING, WORN};
pub use month::{landed, release};
pub use seed::{land, lodge, place, sown, Landed, Seed};
pub(crate) use show::lose;
pub use stand::{height, loose_bins, stem, BINS};

use crate::life::WET_TISSUE;
use crate::chem::Chemistry;
use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};

/// Kinds of cover a cell's arrays hold: the most a world's `Config::variety` may be. A cell uses its first `variety`
/// slots; the rest stay empty.
pub const SLOTS: usize = 12;
// The genes of a kind of cover (laws.md 3).
/// Capacity of the light reaction, log10 mol per kg of living cover per s at `t_cat` (at a moisture need of 1)
pub const LIGHT: usize = 0;
/// Capture area, log10 m² per kg of living cover
pub const LEAF: usize = 1;
/// Walls, log10 kg of the solid built of fuel alone per kg of body matter
pub const WALLS: usize = 2;
/// Fuel kept, log10 kg per kg of built cover
pub const STORE: usize = 3;
/// The ground temperature below which its frozen share dies, K
pub const FROST: usize = 4;
/// The share of what its ground holds for roots at which it is fully active, log10 (`cover::activity`)
pub const MOIST: usize = 5;
/// Share of what it builds that goes to seeds
pub const SEED: usize = 6;
/// Repair, log10: its wear life is 10^REPAIR times the rate of living's (laws.md 3)
pub const REPAIR: usize = 7;
/// Capacity of the fixing reaction, log10 mol per kg of living cover per s at `t_cat`; off at or below `FIX_OFF`
pub const FIX: usize = 8;
pub const NG: usize = 9;
/// A fixing gene at or below this is off.
pub const FIX_OFF: f32 = -10.0;

/// Mutation step (times `Physics::mut_size`) and range of each gene.
pub const RANGE: [(f32, f32, f32); NG] = [(0.3, -8.0, -2.0), (0.3, -2.0, 2.0), (0.2, -2.0, 2.0), (0.2, -3.0, 0.0), (5.0, 200.0, 300.0), (0.1, -2.0, 0.0), (0.05, 0.0, 1.0), (0.2, -3.0, 3.0), (0.3, -12.0, -6.0)];
/// A slot whose built cover is under this, kg, is empty.
pub const EMPTY_KG: f64 = 1e-9;
pub use crate::life::kin::APART;
pub use crate::life::{Parts, FUEL, MATTER, NO_ROOT, NP, WALL};
pub use crate::math::dec;

/// The cover of one ground cell, in `SLOTS` slots, of which its world uses the first `Config::variety`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cover {
    /// Per slot: mol of each part
    pub mol: [[f64; NP]; SLOTS],
    /// Per slot: its genes, and the starting kind it descends from (its row of the starting kinds, a label for the tape;
    /// `NO_ROOT` when empty)
    pub genes: [[f32; NG]; SLOTS],
    pub root: [u32; SLOTS],
    /// Per slot: whether one more kg of it did not pay its upkeep in its last month (`grow::build`): all it builds then
    /// leaves as seeds
    pub full: [bool; SLOTS],
    /// Per slot: its balance over about its last year, J per kg of built cover a month: what its light reaction stored
    /// less its upkeep, a running mean over a year's months (`grow::live`); below zero it is in deficit and gives way to
    /// arriving seeds (`seed::lodge`)
    pub net: [f32; SLOTS],
    /// Per slot: its plants in the cell, a real count (`stand`): its built cover over them is a plant's mass
    pub plants: [f64; SLOTS],
    /// Per slot: the share of its ground (`ground`) whose loose ground holds its plants (`stand::rooted_share`), and the loose ground
    /// its roots needed when that was found, m (`stand::need`; 0: not found yet)
    pub rooted: [f32; SLOTS],
    pub need: [f32; SLOTS],
    /// The film its leaves' pores were open to the key liquid's vapour over its last month, mol K/(Pa s) per m² of the
    /// cell (`around::pores`), and the share of its ground's liquid their roots reach
    pub film: f32,
    pub reach: f32,
    /// The heat of its cover, J; the cell's light (`World::light`) at its last step, J/m²; the share of the light its
    /// cover absorbs that its aside node sets aside for its light reaction
    pub e: f64,
    pub mark: f64,
    pub keep: f32,
    /// The seeds lying on its cell's ground (`lying`), at its temperature
    pub lying: Lying,
    /// Seeds made in its last step, landed after every cell had its step (`land`); not saved
    #[serde(skip)]
    pub(crate) seeds: Vec<Seed>,
}

impl Default for Cover {
    /// No cover.
    fn default() -> Cover {
        Cover { mol: [[0.0; NP]; SLOTS], genes: [[0.0; NG]; SLOTS], root: [NO_ROOT; SLOTS], full: [false; SLOTS], net: [0.0; SLOTS], plants: [0.0; SLOTS], rooted: [0.0; SLOTS], need: [0.0; SLOTS], film: 0.0, reach: 0.0, e: 0.0, mark: 0.0, keep: 0.0, lying: Lying::default(), seeds: Vec::new() }
    }
}

impl Cover {
    /// The seeds made in its last month (`live`), taken from it: where the full world lands them (`land`) or a
    /// nursery does.
    pub fn sent(&mut self) -> Vec<Seed> {
        std::mem::take(&mut self.seeds)
    }
    /// Whether it holds any cover or seeds lying.
    pub fn any(&self) -> bool {
        self.root.iter().chain(&self.lying.root).any(|&r| r != NO_ROOT)
    }
    /// The amounts of each of its slots, then of each lot of seeds lying (mol of each part).
    pub fn all_mol(&self) -> impl Iterator<Item = &[f64; NP]> + '_ {
        self.mol.iter().chain(&self.lying.mol)
    }
    /// Its slots that hold cover.
    pub fn held(&self) -> impl Iterator<Item = usize> + '_ {
        (0..SLOTS).filter(|&x| self.root[x] != NO_ROOT)
    }
    /// The built cover of slot `x` (body matter and solids), kg.
    pub fn built(&self, mm: &[f64; NP], x: usize) -> f64 {
        built_kg(mm, &self.mol[x])
    }
    /// The built cover of one plant of slot `x`, kg.
    pub fn plant(&self, mm: &[f64; NP], x: usize) -> f64 {
        self.built(mm, x) / self.plants[x].max(1e-300)
    }
    /// The height of the plants of slot `x`, m (`stand::height`), its walls of density `rho` kg/m³.
    pub fn height(&self, ph: &crate::Physics, mm: &[f64; NP], rho: f64, x: usize) -> f64 {
        stand::height(ph, self.plant(mm, x), self.walls(mm, x) / self.built(mm, x).max(1e-300), rho)
    }
    /// The width of the main stem of the plants of slot `x`, m (`stand::stem`), its walls of density `rho` kg/m³.
    pub fn width(&self, ph: &crate::Physics, mm: &[f64; NP], rho: f64, x: usize) -> f64 {
        stand::stem(ph, self.plant(mm, x), self.walls(mm, x) / self.built(mm, x).max(1e-300), rho)
    }
    /// All the cover of slot `x`, kg.
    pub fn kg(&self, mm: &[f64; NP], x: usize) -> f64 {
        self.built(mm, x) + self.mol[x][FUEL] * mm[FUEL]
    }
    /// The living cover of slot `x`, kg: its body matter with the liquid of living tissue (`life::WET_TISSUE`); its walls
    /// are a frame, which neither works nor wears (laws.md 3).
    pub fn living(&self, mm: &[f64; NP], x: usize) -> f64 {
        self.mol[x][MATTER] * mm[MATTER] / (1.0 - WET_TISSUE)
    }
    /// Its walls, kg.
    pub fn walls(&self, mm: &[f64; NP], x: usize) -> f64 {
        self.mol[x][WALL] * mm[WALL]
    }
    /// Its capture area, m².
    pub fn leaf(&self, mm: &[f64; NP], x: usize) -> f64 {
        dec(self.genes[x][LEAF]) * self.living(mm, x)
    }
    /// The mol of substance `k` it holds with its seeds lying (0 for a substance it is not made of).
    pub fn amount(&self, p: &Parts, k: usize) -> f64 {
        p.sub.iter().position(|&s| s == k).map_or(0.0, |q| self.all_mol().map(|m| m[q]).sum())
    }
    /// Its heat capacity with its seeds lying, J/K.
    pub fn cap(&self, chem: &Chemistry, p: &Parts) -> f64 {
        self.all_mol().map(|m| (0..NP).map(|q| m[q] * chem.subs[p.sub[q]].cp).sum::<f64>()).sum()
    }
    /// Its temperature, K (0 for none).
    pub fn temp(&self, chem: &Chemistry, p: &Parts) -> f64 {
        let c = self.cap(chem, p);
        if c > 0.0 { self.e / c } else { 0.0 }
    }
    /// The reflectance of the cover of slot `x`: the mean of its solid parts' three bands, by volume.
    pub fn reflectance(&self, chem: &Chemistry, p: &Parts, x: usize) -> f64 {
        let (mut v, mut r) = (1e-30, 0.0);
        for (q, &k) in p.sub.iter().enumerate() {
            let s = &chem.subs[k];
            let vol = self.mol[x][q] * s.molar_mass / s.density[0];
            (v, r) = (v + vol, r + vol * s.refl[0].iter().map(|&b| b as f64).sum::<f64>() / 3.0);
        }
        r / v
    }
    /// Its amounts by substance with its seeds lying, mol (`MAX_SUBSTANCES`).
    pub fn amounts(&self, p: &Parts) -> [f64; MAX_SUBSTANCES] {
        let mut a = [0.0; MAX_SUBSTANCES];
        for m in self.all_mol() {
            (0..NP).for_each(|q| a[p.sub[q]] += m[q]);
        }
        a
    }
}

/// The built cover of amounts `m` (mol of each part): body matter and walls, kg.
pub fn built_kg(mm: &[f64; NP], m: &[f64; NP]) -> f64 {
    m[MATTER] * mm[MATTER] + m[WALL] * mm[WALL]
}

/// The cover of one starting kind (`Cover::root`), summed over the world for the shell: its slots, its cover (kg), its
/// built cover (kg), its capture area (m²), and its genes summed by built cover.
#[derive(Clone, Debug, Default)]
pub struct Sum {
    pub slots: f64,
    pub kg: f64,
    pub built: f64,
    pub leaf: f64,
    pub genes: [f64; NG],
}

/// The cover of the world by starting kind (index: the root), in the order of the cells and their slots.
pub fn sums(w: &crate::World) -> Vec<Sum> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mm = p.mm(&w.chem);
    let mut out: Vec<Sum> = Vec::new();
    for c in w.life.cells.iter().map(|x| &x.cover) {
        for x in c.held() {
            let r = c.root[x] as usize;
            if r >= out.len() {
                out.resize(r + 1, Sum::default());
            }
            let (s, built) = (&mut out[r], c.built(&mm, x));
            (s.slots, s.kg, s.built, s.leaf) = (s.slots + 1.0, s.kg + c.kg(&mm, x), s.built + built, s.leaf + c.leaf(&mm, x));
            s.genes.iter_mut().zip(c.genes[x]).for_each(|(a, g)| *a += built * g as f64);
        }
    }
    out
}

/// Each slot of cover in the world, in the order of the cells: its starting kind and its genes.
pub fn slots(w: &crate::World) -> impl Iterator<Item = (u32, &[f32; NG])> {
    w.life.cells.iter().flat_map(|c| c.cover.root.iter().zip(&c.cover.genes)).filter(|x| *x.0 != NO_ROOT).map(|(&r, g)| (r, g))
}

/// The cover of the whole world with its seeds lying, by substance, mol (`MAX_SUBSTANCES`).
pub fn totals(w: &crate::World) -> [f64; MAX_SUBSTANCES] {
    let mut a = [0.0; MAX_SUBSTANCES];
    let Some(p) = Parts::of(&w.chem) else { return a };
    for c in w.life.cells.iter().map(|x| &x.cover) {
        c.all_mol().for_each(|m| (0..NP).for_each(|q| a[p.sub[q]] += m[q]));
    }
    a
}

/// The seeds lying on the ground of each cell, kg per m² (`Lying::kg`).
pub fn lying(w: &crate::World) -> Vec<f64> {
    let Some(p) = Parts::of(&w.chem) else { return vec![0.0; w.mesh.cells().n()] };
    let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area());
    w.life.cells.iter().enumerate().map(|(i, c)| c.cover.lying.kg(&mm) / area[i]).collect()
}

/// What the cover of each ground cell holds per m², summed over its slots by `of` (`Cover::kg`, `Cover::walls`,
/// `Cover::leaf`).
pub fn per_m2(w: &crate::World, of: impl Fn(&Cover, &[f64; NP], usize) -> f64) -> Vec<f64> {
    let mut out = vec![0.0; w.mesh.cells().n()];
    let Some(p) = Parts::of(&w.chem) else { return out };
    let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area());
    for (i, c) in w.life.cells.iter().enumerate() {
        out[i] = (0..w.cfg.variety).map(|x| of(&c.cover, &mm, x)).sum::<f64>() / area[i];
    }
    out
}

/// The least ground a cover stands on, m² (one plant's): a cell whose land is gone still gives its cover a finite
/// room, which it thins to and starves in under the liquid.
const LEAST_GROUND_M2: f64 = 1.0;

/// The ground the cover of cell `i` lives on, m²: its land (`contract::land`), at least `LEAST_GROUND_M2`. Its leaves
/// layer over it, its plants find their room on it, its seeds lodge on it.
pub fn ground(w: &crate::World, i: usize) -> f64 {
    crate::contract::land(w, i).max(LEAST_GROUND_M2)
}

/// Of `held` mol of the key liquid in the ground of cell `i`: what is liquid that its cover reaches, mol, and the share
/// of what its ground holds for roots that it holds (`available`), its pores' space read from Flow (`Scratch::land_pores`;
/// one store over the depth of its loose ground). Of a cell of parts with land, its land part's own (`flow::parts`) over
/// that part's area, no more than the cell holds; else the cell's over its area.
pub fn wet(w: &crate::World, p: &Parts, i: usize, held: f64) -> (f64, f64) {
    let (t, plateau) = w.thaw_last(i);
    let molten = w.molten(p.liquid, i, t, plateau);
    let area = w.mesh.cells().area()[i];
    let (liquid, over) = match w.scratch.land_part(i, p.liquid) {
        Some((mol, share)) => (mol.min(held) * molten, area * share),
        None => (held * molten, area),
    };
    let depth = liquid * w.chem.molar_volume(p.liquid, crate::chem::Phase::Liquid) / over;
    (liquid, available(&w.cfg.phys, depth / w.scratch.land_pores(i).max(1e-9)))
}

/// The share of the liquid ground holds for roots that ground whose pores are the share `filled` full holds: none at the
/// wilting point (`wilt`), all at field capacity (`field`) and over it, in proportion between (the available liquid of
/// soil physics; Saxton and Rawls 2006).
pub fn available(ph: &crate::Physics, filled: f64) -> f64 {
    ((filled - ph.wilt) / (ph.field - ph.wilt)).clamp(0.0, 1.0)
}

/// The activity of a kind of cover of genes `genes` on ground that holds the share `available` of what it holds for roots
/// (`available`): full from the share its moisture gene sets (`MOIST`), falling in a straight line to none at the wilting
/// point. A plant transpires freely until its roots have drawn about half the liquid available to them (FAO-56's
/// depletion fraction p of 0.3 to 0.7, 0.5 for most kinds), a `MOIST` of log10(1 − p), and below that its transpiration
/// falls with what is left, its stress coefficient Ks (Allen and others 1998, from memory). A dry seed's activity
/// falls far more steeply (`Physics::activity`, `lying`).
pub fn activity(genes: &[f32; NG], available: f64) -> f64 {
    (available / dec(genes[MOIST])).clamp(0.0, 1.0)
}

/// The share of a kind of cover of frost gene `frost` K that freezes in a month at `t` K at activity `act`, where its key
/// liquid melts at `melt` K: its frost gene is the limit it reaches hardened, when dormant; growing tissue loses that
/// hardiness and freezes at the liquid's melting point (`Physics::frozen`, the depth between the two lost as it wakes).
/// So hardiness is the kind's own, paid for in every month (`Physics::c_firm`), not given by dormancy (Levitt 1980;
/// Sakai and Larcher 1987).
pub fn frozen(ph: &crate::Physics, melt: f64, frost: f64, (t, act): (f64, f64)) -> f64 {
    let grown = frost.max(melt);
    ph.frozen((grown, grown - frost), t, act)
}

impl crate::World {
    /// The second film over ground cell `i` beside the air's, through which its ground's liquid `k` evaporates by the
    /// ground's own law (1.5): its cover's pores (`Cover::film`, mol K/(Pa s) per m²) and the share of the ground's
    /// liquid behind them (`Cover::reach`). None where nothing stands open.
    pub fn second_film(&self, i: usize) -> Option<(usize, f64, f64)> {
        let c = self.life.cover(i);
        (c.film > 0.0).then(|| Parts::of(&self.chem).map(|p| (p.liquid, c.film as f64, c.reach as f64))).flatten()
    }
}

/// How far apart two gene vectors are, in steps: the largest gap of a gene over its step.
/// A fixing gene that is off counts as `FIX_OFF`, where differences do nothing.
pub fn distance(a: &[f32; NG], b: &[f32; NG]) -> f32 {
    let at = |v: &[f32; NG], g: usize| if g == FIX { v[g].max(FIX_OFF) } else { v[g] };
    (0..NG).map(|g| (at(a, g) - at(b, g)).abs() / RANGE[g].0).fold(0.0, f32::max)
}

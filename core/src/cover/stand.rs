//! The plants of a slot of cover (laws.md 4.5): the model holds a slot's mass and its count of plants, so each plant's
//! mass is known, and from it the plant's height by the buckling limit (McMahon 1973), how many plants a stand of that
//! mass holds at most (the self-thinning law, Yoda and others 1963), how deep its roots reach (Schenk and Jackson
//! 2002), and the share of its cell whose loose ground holds them. One place for these laws: the light's layers
//! (`grow`), the seeds (`seed`) and the page's drawing (`tape::stands`) read them.

use super::{distance, Cover, APART, NG, NO_ROOT, NP, SLOTS};
use crate::Physics;
use std::f64::consts::FRAC_PI_4;

/// A cell's loose ground is kept as the share of its area in each of `BINS` bins of depth, spaced evenly in the log of
/// the depth from `LEAST_M` to `MOST_M` m (all under the first in the first, all over the last in the last): the
/// resolution at which a kind's rooted share is known (the page draws with that share, `tape::stands`).
pub const BINS: usize = 24;
pub const LEAST_M: f64 = 0.005;
pub const MOST_M: f64 = 20.0;

/// The loose depth bin `b` stands for, m: its middle on the log scale.
fn bin_m(b: usize) -> f64 {
    LEAST_M * (MOST_M / LEAST_M).powf((b as f64 + 0.5) / BINS as f64)
}

/// The mean over a cell of the log of its loose ground's depth, ln m, from the share of its area in each bin of depth
/// (`loose_bins`).
pub fn loose_log(bins: &[f32; BINS]) -> f64 {
    bins.iter().enumerate().map(|(b, &s)| s as f64 * bin_m(b).ln()).sum()
}

/// The width of a bin in the log of the depth.
fn bin_width() -> f64 {
    (MOST_M / LEAST_M).ln() / BINS as f64
}

/// The width of the stem of a plant of `mass` kg of built cover, the share `walls` of it walls of density `rho` kg/m³,
/// m: the cylinder of `stem_form` and `stem_solid` that holds its walls, as tall as the buckling limit lets it stand
/// (walls = form × solid × rho × π/4 × D² × `tall` × D^(2/3)).
pub fn stem(ph: &Physics, mass: f64, walls: f64, rho: f64) -> f64 {
    (mass * walls / (ph.stem_form * ph.stem_solid * rho * FRAC_PI_4 * ph.tall)).max(0.0).powf(3.0 / 8.0)
}

/// The height of that plant, m: `tall` × its stem's width^(2/3), the elastic buckling limit (McMahon 1973).
pub fn height(ph: &Physics, mass: f64, walls: f64, rho: f64) -> f64 {
    ph.tall * stem(ph, mass, walls, rho).powf(2.0 / 3.0)
}

/// The share of the walls its `WALLS` gene sets that a plant of `mass` kg of built cover builds: mass / (mass +
/// `wall_kg`). A seedling is mostly leaf and the share of stem grows with a plant's size (Poorter and others 2012), so a
/// small plant of a kind of many walls catches light as a leafy one does, and its walls come as it grows tall.
pub(super) fn walled(ph: &Physics, mass: f64) -> f64 {
    mass / (mass + ph.wall_kg)
}

/// The walls of a slot of `leaf` m² of capture area whose plants stand `height` m tall that conduct liquid to its leaves,
/// kg, at most its walls `walls` kg of density `rho`: by the pipe model their cross-section goes with the leaf area they
/// feed, `huber` m² a m² (Shinozaki and others 1964; Tyree and Ewers 1991), as tall as the plant, in stems of
/// `stem_form` and `stem_solid`. They are alive and breathe (`c_sapwood`); the core inside them is dead.
pub(super) fn sapwood(ph: &Physics, (leaf, height): (f64, f64), walls: f64, rho: f64) -> f64 {
    (ph.huber * leaf * height * ph.stem_form * ph.stem_solid * rho).min(walls)
}

/// The most plants a m² a stand of `kg` per m² holds: (`thin_kg` / kg)², the self-thinning law, mean plant mass =
/// `thin_kg` × (plants per m²)^(-3/2) (Yoda and others 1963; White 1981).
pub(super) fn most_plants(ph: &Physics, kg: f64) -> f64 {
    (ph.thin_kg / kg.max(1e-300)).powi(2)
}

/// Slot `x` of cover `c` of a cell of `area` m² thins out: a stand denser than the self-thinning law allows for its mass
/// loses plants, not mass, and the survivors take what the dead freed (Yoda and others 1963).
pub(super) fn thin(c: &mut Cover, mm: &[f64; NP], ph: &Physics, area: f64, x: usize) {
    c.plants[x] = c.plants[x].min(area * most_plants(ph, c.built(mm, x) / area));
}

/// The loose ground a plant `height` m tall roots in, m: `root_per_height` of its height, no less than `root_least`
/// (rooting depth grows with a plant's size; Schenk and Jackson 2002).
pub(super) fn need(ph: &Physics, height: f64) -> f64 {
    (ph.root_per_height * height).max(ph.root_least)
}

/// The share of a point that holds a kind whose roots need `need` m where `loose` m of loose ground lies: none up to
/// `rooted_from` of its need, all from its need, smoothly between (bare bed anchors nothing and holds no liquid).
fn rooted(ph: &Physics, loose: f64, need: f64) -> f64 {
    let t = ((loose - ph.rooted_from * need) / ((1.0 - ph.rooted_from) * need).max(1e-300)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The share of a cell that holds a kind whose roots need `need` m, of loose ground `bins` (the share of its area in
/// each bin, `loose_bins`): the cell's rooted mean.
pub(super) fn rooted_share(ph: &Physics, bins: &[f32; BINS], need: f64) -> f64 {
    bins.iter().enumerate().map(|(b, &s)| s as f64 * rooted(ph, bin_m(b), need)).sum()
}

/// Whether a kind whose roots needed `was` m when its rooted share was last found needs a new one at `now` m: when the
/// need has moved by more than a bin.
pub(super) fn moved(was: f32, now: f64) -> bool {
    !(was > 0.0 && (now / was as f64).ln().abs() <= bin_width())
}

/// The loose ground of each cell of a world as built: the share of its area in each bin of depth (`BINS`), from its
/// terrain's samples (`Samples::loose`; laws.md 1.1).
pub fn loose_bins(w: &crate::World) -> Vec<[f32; BINS]> {
    let lat = (w.cfg.lat_south, w.cfg.lat_north);
    let bin = |m: f64| (((m.max(LEAST_M) / LEAST_M).ln() / bin_width()) as usize).min(BINS - 1);
    crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| {
        std::array::from_fn(|b| w.terrain.over(w.terrain.seed, &w.map, w.map.leaves()[i], lat, |s, k| if bin(s.loose[k]) == b { 1.0 } else { 0.0 }) as f32)
    })
}
/// The plants that `kg` of seeds found in a cell of `area` m²: one for each `seed_kg`, as many as the self-thinning
/// law lets stand (`most_plants`).
pub(super) fn founded(ph: &Physics, kg: f64, area: f64) -> f64 {
    (kg / ph.seed_kg).min(area * most_plants(ph, kg / area))
}

/// Alike kinds of a cell (`APART` × `Physics::alike` steps in every gene, `distance`) share a slot: the lighter joins
/// the heavier (`join`).
pub(super) fn merge_alike(c: &mut Cover, mm: &[f64; NP], ph: &Physics, area: f64) {
    for a in 0..SLOTS {
        for z in a + 1..SLOTS {
            if c.root[a] == NO_ROOT || c.root[z] == NO_ROOT || distance(&c.genes[a], &c.genes[z]) > APART * ph.alike {
                continue;
            }
            let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
            join(c, (mm, ph, area), big, small);
        }
    }
}

/// Slot `small` joins slot `big` of the same cell of `area` m²: amounts and plants added (then thinned, `thin`), genes and
/// balance the mean by built cover, its starting kind the heavier's; `small` is empty after.
pub(super) fn join(c: &mut Cover, (mm, ph, area): (&[f64; NP], &Physics, f64), big: usize, small: usize) {
    let (wb, ws) = (c.built(mm, big), c.built(mm, small));
    let f = (ws / (wb + ws).max(1e-300)) as f32;
    let (gs, ms) = (c.genes[small], c.mol[small]);
    (0..NG).for_each(|g| c.genes[big][g] += f * (gs[g] - c.genes[big][g]));
    c.net[big] += f * (c.net[small] - c.net[big]);
    c.plants[big] += c.plants[small];
    (0..NP).for_each(|q| c.mol[big][q] += ms[q]);
    thin(c, mm, ph, area, big);
    (c.mol[small], c.root[small], c.full[small], c.net[small], c.plants[small], c.need[small]) = ([0.0; NP], NO_ROOT, false, 0.0, 0.0, 0.0);
}

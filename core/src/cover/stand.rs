//! The plants of a slot of cover (laws.md 3): the model holds a slot's mass and its plants by size group (`groups`), so
//! each group's plant's mass is known, and from it the plant's height by the buckling limit (McMahon 1973), how many
//! plants a stand of its mean mass holds at most (the self-thinning law, Yoda and others 1963), how deep its roots reach
//! (Schenk and Jackson 2002), and the share of its cell whose loose ground holds them. One place for these laws: the light's layers
//! (`grow`), the seeds (`seed`) and the page's drawing (`tape::stands`) read them.

use super::groups::Group;
use super::{Cover, NG, NO_ROOT, NP, SLOTS};
use crate::life::kin;
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

/// The width of a bin in the log of the depth.
fn bin_width() -> f64 {
    (MOST_M / LEAST_M).ln() / BINS as f64
}

/// The width of the stem of a plant of `mass` kg of built cover, the share `walls` of it walls of density `rho` kg/m³,
/// m: the cylinder of `stem_form` and `stem_solid` that holds its walls, as tall as the buckling limit lets it stand
/// (walls = form × solid × rho × π/4 × D² × `tall` × D^(2/3), so D^(8/3) is `width_eight_thirds`).
pub fn stem(ph: &Physics, mass: f64, walls: f64, rho: f64) -> f64 {
    width_eight_thirds(ph, mass * walls, rho).powf(3.0 / 8.0)
}

/// The height of that plant, m: `tall` × its stem's width^(2/3), the elastic buckling limit (McMahon 1973), which is
/// `tall` × (D^(8/3))^(1/4).
pub fn height(ph: &Physics, mass: f64, walls: f64, rho: f64) -> f64 {
    ph.tall * width_eight_thirds(ph, mass * walls, rho).sqrt().sqrt()
}

/// D^(8/3) of the stem that holds `kg` of walls of density `rho` kg/m³ as tall as the buckling limit lets it stand
/// (`stem`), m^(8/3).
fn width_eight_thirds(ph: &Physics, kg: f64, rho: f64) -> f64 {
    (kg / (ph.stem_form * ph.stem_solid * rho * FRAC_PI_4 * ph.tall)).max(0.0)
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

/// The ground a plant of `mass` kg of built cover holds where its stand is as dense as the self-thinning law allows, m²:
/// (mass / `thin_kg`)^(2/3) (`most_plants`: n plants a m² of mean mass m fill the ground at n (m / thin_kg)^(2/3) = 1).
pub fn plant_ground(ph: &Physics, mass: f64) -> f64 {
    (mass / ph.thin_kg).powf(2.0 / 3.0)
}

/// The plants of the `n` slots in use of cover `c` on its cell's ground of `area` m² (`cover::ground`), each slot on
/// its share of its patch's ground (`Cover::stands`; the grounds nested, so of another slot's plants the share on a
/// slot's own ground is the lesser ground over that slot's, and none of another patch's), its walls of density `rho`
/// kg/m³, thin out together: plants crowd the plants of their patch whose crowns share their layer of the air (`Cover::crown`),
/// whatever slot holds them, as the self-thinning law is the crowns of a stand filling its space (Yoda and others 1963)
/// and the layers of a stand thin apart (an understory under a closed canopy). Of each slot the ground its layer's
/// plants hold (its plants at their mean mass, the law's own measure of a stand, `plant_ground`), each other slot's
/// counted by the share of its crown's depth that slot's crown shares, is set against its ground; where it is more,
/// the slot loses plants, not mass, as its whole layer would: by that excess cubed (a plant's ground goes as its
/// mass^(2/3), and its mass as 1 / the count), the small first (`lose`); its large plants lose no more than the crowd
/// of the large plants of the slots that share their layer asks, each at their own mass (smaller plants take no room
/// from them: the size-asymmetric competition of Weiner 1990; Purves and others 2008).
pub(super) fn thin(c: &mut Cover, mm: &[f64; NP], (ph, rho, n): (&Physics, f64, usize), area: f64) {
    let (mut held, mut ground, mut span) = ([0; SLOTS], [0.0; SLOTS], [(0.0, 0.0); SLOTS]);
    let mut k = 0;
    for x in (0..n).filter(|&x| c.root[x] != NO_ROOT && c.count(x) > 0.0 && c.built(mm, x) > 0.0) {
        (held[k], ground[k]) = (x, c.count(x) * plant_ground(ph, c.built(mm, x) / c.count(x)));
        k += 1;
    }
    let (held, ground) = (&held[..k], &ground[..k]);
    // The ground each slot stands on, m², and the share of each other slot's plants that stand on it: none of a slot
    // of another patch.
    let own: [f64; SLOTS] = std::array::from_fn(|q| if q < k { area * c.patch_share(held[q]) * c.stands[held[q]] as f64 } else { 0.0 });
    let patch = c.patch;
    let apart = |q: usize, r: usize| patch[held[q]] != patch[held[r]];
    let on_own = |q: usize, r: usize| if r == q { 1.0 } else if apart(q, r) { 0.0 } else { own[q].min(own[r]) / own[r].max(1e-300) };
    // A crown shares all of itself: a lone slot's crowd is its own ground, and its crown is not needed.
    if k > 1 {
        held.iter().zip(span.iter_mut()).for_each(|(&x, s)| *s = c.crown(ph, mm, area, x, c.height(ph, mm, rho, x)));
    }
    let by = |q: usize, r: usize| if r == q { 1.0 } else { shared(span[q], span[r]) * on_own(q, r) };
    let crowd: [f64; SLOTS] = std::array::from_fn(|q| if q < k { (0..k).map(|r| by(q, r) * ground[r]).sum() } else { 0.0 });
    if (0..k).all(|q| crowd[q] <= own[q]) {
        return;
    }
    // Each slot's large plants, their count and the ground they hold at their own mass.
    let mut large = [(0.0, 0.0); SLOTS];
    for (q, &x) in held.iter().enumerate() {
        let s = c.sizes(ph, mm, x);
        large[q] = (s.count[s.large()], s.count[s.large()] * plant_ground(ph, s.mass[s.large()]));
    }
    for (q, &x) in held.iter().enumerate() {
        let (crowd, room) = (crowd[q], own[q]);
        if crowd > room {
            // The large plants lose no more than the crowd of the large plants of the slots that share their layer
            // asks, each at their own mass: smaller plants take no room from them.
            let alone: f64 = (0..k).map(|r| by(q, r) * large[r].1).sum();
            let most = if alone > room { large[q].0 * (1.0 - (room / alone).powi(3)) } else { 0.0 };
            lose(c, mm, x, (c.count(x) * (1.0 - (room / crowd).powi(3)), most));
        }
    }
}

/// Slot `x` of cover `c` loses `dead` of its plants, the small first: the suppressed plants of a crowded stand die
/// first (Yoda and others 1963; Weiner 1990), and the survivors take what the dead freed. Its smallest group loses
/// plants until its survivors are as large as the next group's plants, the two then fuse (their plants alike), and so
/// on; what is left falls on its large plants, at most `most` of them.
fn lose(c: &mut Cover, mm: &[f64; NP], x: usize, (mut dead, most): (f64, f64)) {
    let (mut g, held) = c.groups(mm, x);
    let mut k = 0;
    while k + 1 < held && dead > 0.0 {
        // The share of its plants that, taking all its mass, are as large as the next group's (all where they are).
        let keep = (g[k].plant() / g[k + 1].plant()).min(1.0);
        if dead <= g[k].n * (1.0 - keep) {
            break;
        }
        dead -= g[k].n * (1.0 - keep);
        g[k + 1] = Group { n: g[k + 1].n + g[k].n * keep, kg: g[k + 1].kg + g[k].kg };
        g[k] = Group::default();
        k += 1;
    }
    if k + 1 == held {
        dead = dead.min(most);
    }
    g[k].n = (g[k].n - dead.max(0.0)).max(f64::MIN_POSITIVE);
    c.set_groups(x, &mut g[..held]);
}

impl Cover {
    /// The share `dead` of the large plants of slot `x` (its last group) died where no crowding killed them (wear,
    /// upkeep unpaid, frost, flame): the ground their crowns held where their stand is closed (`plant_ground`) adds to
    /// its fallen crowns (`Cover::fell`), the opening a gap is made of (`gaps`). Plants crowded out (`thin`) open no
    /// ground: the crowns of the living fill it as it frees (Yoda and others 1963).
    pub(super) fn fell(&mut self, ph: &Physics, mm: &[f64; NP], x: usize, dead: f64) {
        let (g, held) = self.groups(mm, x);
        if let Some(l) = held.checked_sub(1).map(|l| g[l]) {
            self.fell[x] += l.n * dead * plant_ground(ph, l.plant());
        }
    }
}

/// The share of the crown from `b` to `t` m that the crown from `b2` to `t2` shares; of a crown of no depth, whether the
/// other holds its height.
fn shared((b, t): (f64, f64), (b2, t2): (f64, f64)) -> f64 {
    if t > b {
        (t.min(t2) - b.max(b2)).max(0.0) / (t - b)
    } else if b2 <= t && t <= t2 {
        1.0
    } else {
        0.0
    }
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
/// The plants that `kg` of seeds found on ground of `area` m²: one for each `seed_kg`, as many as the self-thinning
/// law lets stand (`most_plants`).
pub(super) fn founded(ph: &Physics, kg: f64, area: f64) -> f64 {
    (kg / ph.seed_kg).min(area * most_plants(ph, kg / area))
}

/// Alike kinds of a patch of a cell (`APART` × `Physics::alike` steps in every gene, `distance`) share a slot: of its
/// `n` slots in use, the lighter joins the heavier (`join`).
pub(super) fn merge_alike(c: &mut Cover, mm: &[f64; NP], (ph, n): (&Physics, usize)) {
    for a in 0..n {
        for z in a + 1..n {
            if c.patch[a] != c.patch[z] || !kin::together(&c.genes, &c.root, (a, z), super::APART * ph.alike, super::distance) {
                continue;
            }
            let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
            join(c, mm, big, small);
        }
    }
}

/// Slot `small` joins slot `big` of the same cover: amounts added, and their groups of plants together
/// (`Cover::join_groups`; the stand thins at its month, `thin`), genes, balance and the store its plants under the
/// liquid burnt and its share in the strips the mean by built cover, its line the lower of the two (the ground of both; the one set where the other
/// is not, `UNSET`), its starting kind the
/// heavier's; `small` is empty after.
pub(super) fn join(c: &mut Cover, mm: &[f64; NP], big: usize, small: usize) {
    let (wb, ws) = (c.built(mm, big), c.built(mm, small));
    let f = (ws / (wb + ws).max(1e-300)) as f32;
    let (gs, ms) = (c.genes[small], c.mol[small]);
    (0..NG).for_each(|g| c.genes[big][g] += f * (gs[g] - c.genes[big][g]));
    c.net[big] += f * (c.net[small] - c.net[big]);
    c.airless[big] += f * (c.airless[small] - c.airless[big]);
    c.drained[big] += f * (c.drained[small] - c.drained[big]);
    c.strip[big] += f * (c.strip[small] - c.strip[big]);
    let lines = (c.line[big], c.line[small]);
    c.line[big] = if lines.0 < 0.0 || lines.1 < 0.0 { lines.0.max(lines.1) } else { lines.0.min(lines.1) };
    c.join_groups(mm, big, small);
    c.fell[big] += c.fell[small];
    (0..NP).for_each(|q| c.mol[big][q] += ms[q]);
    c.clear(small);
}

//! Gaps (laws.md 3): a closed forest is a mosaic of gaps, each opened where a canopy tree died and its crown left its
//! ground to the light, and of the stands that grow back in them (Watt 1947; the gap models, Shugart 1984). The large
//! plants of a slot that die uncrowded leave the ground their crowns held (`Cover::fell`); once a year, where the
//! canopy of a patch left at least `GAP_LEAST` of its cell, that share of the patch becomes a patch of its own: the
//! plants under the fallen trees, in full light.

use super::patches::{carve, fold};
use super::{stand, Cover, Parts, GROUPS, NO_ROOT, NP, SLOTS, WALL};
use crate::rng::{Purpose, Rng};
use crate::{Physics, World};

/// The least share of a cell's ground the fallen crowns of a patch's canopy open as a gap: less, they gather over the
/// years.
pub const GAP_LEAST: f64 = 0.01;

/// A gap to open: of patch `k`, the share `s` of its cell, about its sample `point` (`Patch::at`).
struct Gap {
    k: usize,
    s: f64,
    point: u32,
}

/// The slots of patch `k` of cover `c` on its cell's ground of `ground` m² (`cover::ground`), their walls of density
/// `rho` kg/m³, whose large plants make its canopy: taller than `surface_m` (lower plants die standing, `upkeep`) and
/// reaching the base of the crown of its tallest plants (`Cover::crown`).
fn canopy(c: &Cover, (ph, mm, rho): (&Physics, &[f64; NP], f64), ground: f64, k: usize) -> [bool; SLOTS] {
    let mut top = [false; SLOTS];
    let tall: Vec<(usize, f64)> = c.held_in(k).map(|x| (x, c.height(ph, mm, rho, x))).collect();
    let Some(&(t, h)) = tall.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else { return top };
    let base = c.crown(ph, mm, ground, t, h).0;
    tall.iter().filter(|e| e.1 > ph.surface_m && e.1 >= base).for_each(|e| top[e.0] = true);
    top
}

/// What the year's end does to the cover of cell `i` of `w`: the slots whose fallen crowns gather on (those of the
/// canopy of a patch whose fall is under `GAP_LEAST`), the gaps it opens, and its ground, m² (`cover::ground`). A
/// patch whose canopy fell over all its ground opens none: it is a gap whole.
fn plan(w: &World, i: usize, at: (&Physics, &[f64; NP], f64)) -> ([bool; SLOTS], Vec<Gap>, f64) {
    let c = w.life.cover(i);
    let mut out = ([false; SLOTS], Vec::new(), 0.0);
    if !c.fell.iter().any(|&f| f > 0.0) {
        return out;
    }
    out.2 = super::ground(w, i);
    for k in (0..c.patch_count()).filter(|_| out.2 > 0.0) {
        let top = canopy(c, at, out.2, k);
        let s = (0..SLOTS).filter(|&x| top[x]).map(|x| c.fell[x]).sum::<f64>() / out.2;
        if s < GAP_LEAST {
            (0..SLOTS).for_each(|x| out.0[x] |= top[x]);
        } else if s < c.patches[k].share {
            out.1.push(Gap { k, s, point: point(w, i, k) });
        }
    }
    out
}

/// Where in patch `k` of the cover of cell `i` of `w` a gap opens this year: one of its samples (`inside::patch_ground`),
/// drawn from the world's seed by the month, the cell and the patch.
fn point(w: &World, i: usize, k: usize) -> u32 {
    let g = crate::inside::patch_ground(w, i);
    let mine: Vec<usize> = (0..g.of.len()).filter(|&p| g.of[p] as usize == k).collect();
    let mut r = Rng::new(w.seed, w.month(), (i * super::PATCHES + k) as u64, Purpose::Gap);
    mine.get(r.below(mine.len())).map_or_else(|| w.centre_sample(i), |&p| p) as u32
}

/// Patch `k` of cover `c`, on its cell's ground of `ground` m², opens the share `s` of its cell as a gap about its
/// sample `point` (`patches::open`, joining the patch most alike to bare ground where `PATCHES` are in use): of every
/// slot of the patch the gap takes the share of its plants that stood on its ground, but none of the large plants of
/// the canopy (`canopy`), the trees whose fall opened it, already dead (`Cover::fell`). A slot of the gap meeting one of
/// its kind joins it (`stand::merge_alike`), and a patch of more than `variety` slots folds (`patches::fold`). Room for
/// the gap's slots is made first: while the arrays hold fewer empty slots than the patch holds slots, its two nearest
/// join (`seed::room`).
fn open_gap(c: &mut Cover, at: (&Physics, &[f64; NP], f64), (variety, ground): (usize, f64), Gap { k, s, point }: Gap) {
    let (ph, mm, _) = at;
    let empty = |c: &Cover| c.root.iter().filter(|&&r| r == NO_ROOT).count();
    while empty(c) < c.held_in(k).count() && super::seed::room(c, mm, SLOTS, k).is_some() {}
    let (top, n) = (canopy(c, at, ground, k), c.patch_count());
    let mut by = [[s / c.patches[k].share; GROUPS]; SLOTS];
    for x in (0..SLOTS).filter(|&x| top[x]) {
        if let Some(large) = c.held_groups(x).checked_sub(1) {
            by[x][large] = 0.0;
        }
    }
    if carve(c, at, (k, s, point), &by).is_some_and(|into| into < n) {
        stand::merge_alike(c, mm, (ph, SLOTS));
        fold(c, (ph, mm), variety);
    }
}

/// The gaps of every cell at the year's end (`patches::year`): each patch whose canopy's fallen crowns hold at least
/// `GAP_LEAST` of its cell opens them as a gap (`open_gap`); the fallen crowns of all other slots are forgotten but
/// those of the canopy of a patch whose fall is still under it (`plan`).
pub(super) fn year(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (mm, rho) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0]);
    let plans = crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| plan(w, i, (&w.cfg.phys, &mm, rho)));
    let (ph, variety) = (&w.cfg.phys, w.cfg.variety);
    for (cell, (keep, gaps, ground)) in w.life.cells.iter_mut().zip(plans) {
        let c = &mut cell.cover;
        (0..SLOTS).filter(|&x| !keep[x]).for_each(|x| c.fell[x] = 0.0);
        gaps.into_iter().for_each(|g| open_gap(c, (ph, &mm, rho), (variety, ground), g));
    }
}

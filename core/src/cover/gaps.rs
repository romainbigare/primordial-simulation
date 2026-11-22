//! Gaps (laws.md 3): a closed forest is a mosaic of gaps, each opened where a canopy tree died and its crown left its
//! ground to the light, and of the stands that grow back in them (Watt 1947; the gap models, Shugart 1984). The large
//! plants of a slot that die of an event (frost, drowning, flame, a month its upkeep went unpaid) leave the ground their
//! crowns held (`Cover::fell`); once a year, where the canopy of a patch left a share of the patch's ground in that
//! year that an event makes a patch of (`patches::hit`), a die-off, that share of the patch becomes a patch of its own:
//! the plants under the fallen trees, in full light. A canopy's own turnover, its worn out and crowded out, stays spread over its stand, as its size groups
//! and thinning hold it.

use super::patches::{hit, open_event, point, settle, Hit};
use super::{Cover, Parts, GROUPS, NP, SLOTS, WALL};
use crate::rng::Purpose;
use crate::{Physics, World};

/// A gap to open: of patch `k`, the share `s` of its cell, about its sample `point` (`Patch::at`); or the patch whole
/// (`Hit`).
struct Gap {
    k: usize,
    s: f64,
    point: u32,
    hit: Hit,
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

/// The gaps the year's end opens in the cover of cell `i` of `w`, and its ground, m² (`cover::ground`): of each patch
/// whose canopy's crowns that fell this year (`canopy`, `Cover::fell`) held a part of its ground (`patches::hit`), that
/// share of the cell; a patch whose canopy fell over all of its ground but less than `EVENT_LEAST` is a gap whole.
fn plan(w: &World, i: usize, at: (&Physics, &[f64; NP], f64)) -> (Vec<Gap>, f64) {
    let c = w.life.cover(i);
    if !c.fell.iter().any(|&f| f > 0.0) {
        return (Vec::new(), 0.0);
    }
    let ground = super::ground(w, i);
    let mut gaps = Vec::new();
    for k in (0..c.patch_count()).filter(|_| ground > 0.0) {
        let top = canopy(c, at, ground, k);
        let s = (0..SLOTS).filter(|&x| top[x]).map(|x| c.fell[x]).sum::<f64>() / ground;
        let share = c.patches[k].share;
        match hit(s / share) {
            Some(Hit::Part) => gaps.push(Gap { k, s, point: point(w, i, k, Purpose::Gap), hit: Hit::Part }),
            Some(Hit::Whole) => gaps.push(Gap { k, s, point: 0, hit: Hit::Whole }),
            None => {}
        }
    }
    (gaps, ground)
}

/// Patch `k` of cover `c`, on its cell's ground of `ground` m², opens the share `s` of its cell as a gap about its
/// sample `point` (`patches::open_event`, joining the patch most alike to bare ground where `PATCHES` are in use): of
/// every slot of the patch the gap takes the share of its plants that stood on its ground, but none of the large plants
/// of the canopy (`canopy`), the trees whose fall opened it, already dead (`Cover::fell`). Where it joined a patch in
/// use, its slots settle there (`patches::settle`).
fn open_gap(c: &mut Cover, at: (&Physics, &[f64; NP], f64), (variety, ground): (usize, f64), Gap { k, s, point, .. }: Gap) {
    let n = c.patch_count();
    let by = |c: &Cover| {
        let top = canopy(c, at, ground, k);
        let mut by = [[s / c.patches[k].share; GROUPS]; SLOTS];
        for x in (0..SLOTS).filter(|&x| top[x]) {
            if let Some(large) = c.held_groups(x).checked_sub(1) {
                by[x][large] = 0.0;
            }
        }
        by
    };
    if open_event(c, at, (k, s, point), by).is_some_and(|(into, _)| into < n) {
        settle(c, (at.0, at.1), variety);
    }
}

/// The gaps of every cell at the year's end (`patches::year`): each patch whose canopy's crowns that fell this year
/// held a part of its ground (`patches::hit`) opens it as a gap (`plan`, `open_gap`), and a gap whole is made new (age
/// 0); then every slot's fallen crowns are forgotten, the next year counting its own.
pub(super) fn year(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (mm, rho) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0]);
    let plans = crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| plan(w, i, (&w.cfg.phys, &mm, rho)));
    let (ph, variety) = (&w.cfg.phys, w.cfg.variety);
    for (cell, (gaps, ground)) in w.life.cells.iter_mut().zip(plans) {
        let c = &mut cell.cover;
        c.fell = [0.0; SLOTS];
        for g in gaps {
            match g.hit {
                Hit::Part => open_gap(c, (ph, &mm, rho), (variety, ground), g),
                Hit::Whole => c.patches[g.k].age = 0.0,
            }
        }
    }
}

//! PROBE (wip/patch-probe): patches as a partition of a cell's slots. Each slot belongs to one patch (`Cover::patch`);
//! a patch is a share of the cell (`Cover::pshare`) and an age (`Cover::page`). Shade, crowding, the founding of
//! seedlings and the liquid the roots draw are per patch (`canopy`, `stand::thin`, `groups`, `seed::lodge`, `lying`).

use super::budget::{held, DIED};
use super::{Cover, Parts, EMPTY_KG, NO_ROOT, NP, PATCHES, WALL};
use crate::store::{apply, Class, Move, Node};
use crate::World;

/// PROBE: whether flames' burnt share makes a patch (variant C).
/// PROBE: founders that met their patch's cap (`variety` slots) though the cell had a free slot, and founders that
/// met a full cell (all `SLOTS` held).
pub static HITS: [std::sync::atomic::AtomicU64; 2] = [std::sync::atomic::AtomicU64::new(0), std::sync::atomic::AtomicU64::new(0)];
pub static MORE_SLOTS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// PROBE: the slots a cell's month uses: its world's `Config::variety`, or with `MORE_SLOTS` all `SLOTS` where the
/// cell holds more than one patch.
pub fn n_slots(c: &Cover, variety: usize) -> usize {
    if MORE_SLOTS.load(std::sync::atomic::Ordering::Relaxed) && c.pshare.iter().filter(|&&f| f > 0.0).count() > 1 { super::SLOTS } else { variety }
}

/// PROBE: whether flames' burnt share makes a patch (variant C).
pub static FIRE_PATCHES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Kill the share `s` of every slot of cell `i` (amounts and plants), to the ground as remains with its heat, as
/// `seed::place` books a slot it pushes out. With `patched`, the share becomes a new patch of age 0 with no slots: each
/// patch gives the share `s` of its ground. Returns the kg killed.
pub fn clear(w: &mut World, i: usize, s: f64, patched: bool) -> f64 {
    let Some(p) = Parts::of(&w.chem) else { return 0.0 };
    let (mm, e) = (p.mm(&w.chem), p.content(&w.chem));
    let c = &mut w.life.cells[i].cover;
    let was = c.temp(&w.chem, &p);
    let mut m = [0.0; NP];
    let held_slots: Vec<usize> = c.held().collect();
    for x in held_slots {
        for q in 0..NP {
            let d = c.mol[x][q] * s;
            (c.mol[x][q], m[q]) = (c.mol[x][q] - d, m[q] + d);
        }
        c.plants[x].iter_mut().for_each(|n| *n *= 1.0 - s);
    }
    let heat = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * was).min(c.e);
    c.e -= heat;
    if patched {
        let free = (0..PATCHES).find(|&k| c.pshare[k] <= 0.0);
        let k = match free {
            Some(k) => k,
            // Full table: the new clearing joins the youngest patch.
            None => (0..PATCHES).min_by(|&a, &b| c.page[a].total_cmp(&c.page[b])).unwrap(),
        };
        let was_k = c.pshare[k];
        (0..PATCHES).filter(|&q| q != k).for_each(|q| c.pshare[q] *= (1.0 - s) as f32);
        c.pshare[k] = was_k * (1.0 - s) as f32 + s as f32;
        c.page[k] = 0.0;
        let all: f32 = c.pshare.iter().sum();
        c.pshare.iter_mut().for_each(|f| *f /= all);
    }
    w.counts.cover_budget[DIED] -= held(&e, &m);
    let (from, to) = (Node::new(Class::Life, i), Node::ground(i));
    let mut moves: Vec<_> = (0..NP).map(|x| Move { from, to, k: p.sub[x] as u16, mol: m[x], heat: 0.0 }).collect();
    moves.push(Move { from, to, k: 0, mol: 0.0, heat });
    apply(w, &[], &moves);
    super::built_kg(&mm, &m) + m[super::FUEL] * mm[super::FUEL]
}

/// What a patch of cover `c` is for its fusion: its tallest plants, m, and the starting kinds that hold at least
/// `DOMINANT` of its built cover (a bit set).
fn sketch(c: &Cover, ph: &crate::Physics, mm: &[f64; NP], rho: f64, k: u8) -> (f64, u64, f64) {
    let xs: Vec<usize> = c.held().filter(|&x| c.patch[x] == k && c.built(mm, x) > EMPTY_KG).collect();
    let all: f64 = xs.iter().map(|&x| c.built(mm, x)).sum();
    let top = xs.iter().map(|&x| c.height(ph, mm, rho, x)).fold(0.0, f64::max);
    let mut by = [0.0f64; 64];
    xs.iter().for_each(|&x| by[(c.root[x] as usize).min(63)] += c.built(mm, x));
    let set = (0..64).filter(|&r| by[r] >= DOMINANT * all && all > 0.0).fold(0u64, |s, r| s | (1 << r));
    (top, set, all)
}

/// A kind holds a patch when it holds at least this share of its built cover.
const DOMINANT: f64 = 0.1;
/// Two patches fuse when their tallest plants are within this share of each other's height.
pub const ALIKE_HEIGHT: f64 = 0.2;

/// A year of the patches of every cell: each ages a year, and two alike (the same kinds holding them, their tallest
/// plants within `ALIKE_HEIGHT`) fuse, the younger into the older: shares add, its slots go to the older patch (where
/// they and a slot of their kind meet, `stand::merge_alike` joins them in the next month). Returns how many fused.
pub fn year(w: &mut World) -> usize {
    let Some(p) = Parts::of(&w.chem) else { return 0 };
    let mm = p.mm(&w.chem);
    let (ph, rho) = (w.cfg.phys.clone(), w.chem.subs[p.sub[WALL]].density[0]);
    let mut fused = 0;
    let w_variety = w.cfg.variety;
    for cell in w.life.cells.iter_mut() {
        let c = &mut cell.cover;
        if c.pshare.iter().filter(|&&f| f > 0.0).count() < 2 {
            continue;
        }
        (0..PATCHES).filter(|&k| c.pshare[k] > 0.0).for_each(|k| c.page[k] += 1.0);
        loop {
            let live: Vec<u8> = (0..PATCHES as u8).filter(|&k| c.pshare[k as usize] > 0.0).collect();
            let s: Vec<(f64, u64, f64)> = live.iter().map(|&k| sketch(c, &ph, &mm, rho, k)).collect();
            let mut pair = None;
            'find: for a in 0..live.len() {
                for z in a + 1..live.len() {
                    let (ha, hz) = (s[a].0, s[z].0);
                    if s[a].1 == s[z].1 && s[a].2 > 0.0 && s[z].2 > 0.0 && (ha - hz).abs() <= ALIKE_HEIGHT * ha.max(hz) {
                        pair = Some((live[a], live[z]));
                        break 'find;
                    }
                }
            }
            let Some((a, z)) = pair else { break };
            let (old, young) = if c.page[a as usize] >= c.page[z as usize] { (a, z) } else { (z, a) };
            let (fo, fy) = (c.pshare[old as usize], c.pshare[young as usize]);
            c.page[old as usize] = (c.page[old as usize] * fo + c.page[young as usize] * fy) / (fo + fy);
            c.pshare[old as usize] = fo + fy;
            (c.pshare[young as usize], c.page[young as usize]) = (0.0, 0.0);
            for x in 0..super::SLOTS {
                if c.patch[x] == young {
                    c.patch[x] = old;
                }
            }
            fused += 1;
        }
        // With `MORE_SLOTS`, a cell back to one patch keeps its world's first `variety` slots: each slot past them
        // moves to an empty one, else joins the nearest by genes.
        if MORE_SLOTS.load(std::sync::atomic::Ordering::Relaxed) && c.pshare.iter().filter(|&&f| f > 0.0).count() == 1 {
            for x in w_variety..super::SLOTS {
                if c.root[x] == NO_ROOT {
                    continue;
                }
                match (0..w_variety).find(|&q| c.root[q] == NO_ROOT) {
                    Some(q) => move_slot(c, x, q),
                    None => {
                        let q = (0..w_variety).min_by(|&a, &b| super::distance(&c.genes[a], &c.genes[x]).total_cmp(&super::distance(&c.genes[b], &c.genes[x]))).unwrap();
                        let (big, small) = (q, x);
                        super::stand::join(c, &mm, big, small);
                    }
                }
            }
        }
        // A slot left in a patch of no share (emptied) is set to the first patch that holds ground.
        let first = (0..PATCHES as u8).find(|&k| c.pshare[k as usize] > 0.0).unwrap_or(0);
        for x in 0..super::SLOTS {
            if c.root[x] == NO_ROOT || c.pshare[c.patch[x] as usize] <= 0.0 {
                c.patch[x] = first;
            }
        }
    }
    fused
}

/// Slot `x` of cover `c` moves to the empty slot `q`, all it holds.
fn move_slot(c: &mut Cover, x: usize, q: usize) {
    c.mol[q] = c.mol[x];
    c.genes[q] = c.genes[x];
    c.root[q] = c.root[x];
    c.full[q] = c.full[x];
    c.net[q] = c.net[x];
    c.plants[q] = c.plants[x];
    c.share[q] = c.share[x];
    c.rooted[q] = c.rooted[x];
    c.need[q] = c.need[x];
    c.line[q] = c.line[x];
    c.stands[q] = c.stands[x];
    c.airless[q] = c.airless[x];
    c.drained[q] = c.drained[x];
    c.strip[q] = c.strip[x];
    c.patch[q] = c.patch[x];
    c.clear(x);
}

/// PROBE: split cell `i` into two patches alike, the second of share `s`: every slot held is copied into the second
/// patch by its share (amounts and plants), into the empty slots; no matter leaves the cover. False (nothing done)
/// where the cell has not enough empty slots or already holds patches.
pub fn split(w: &mut World, i: usize, s: f64) -> bool {
    let c = &mut w.life.cells[i].cover;
    let held: Vec<usize> = c.held().collect();
    let empty: Vec<usize> = (0..super::SLOTS).filter(|&x| c.root[x] == NO_ROOT).collect();
    if empty.len() < held.len() || c.pshare.iter().filter(|&&f| f > 0.0).count() > 1 {
        return false;
    }
    for (&x, &q) in held.iter().zip(&empty) {
        let mol = c.mol[x];
        move_slot(c, x, q);
        // `move_slot` emptied x: put it back, and give q its share.
        for k in 0..NP {
            let d = mol[k] * s;
            (c.mol[x][k], c.mol[q][k]) = (mol[k] - d, d);
        }
        c.genes[x] = c.genes[q];
        c.root[x] = c.root[q];
        c.full[x] = c.full[q];
        c.net[x] = c.net[q];
        c.share[x] = c.share[q];
        c.rooted[x] = c.rooted[q];
        c.need[x] = c.need[q];
        c.line[x] = c.line[q];
        c.stands[x] = c.stands[q];
        c.airless[x] = c.airless[q];
        c.drained[x] = c.drained[q];
        c.strip[x] = c.strip[q];
        c.patch[x] = 0;
        for g in 0..super::GROUPS {
            let n = c.plants[q][g];
            (c.plants[x][g], c.plants[q][g]) = (n * (1.0 - s), n * s);
        }
        c.patch[q] = 1;
    }
    c.pshare = [(1.0 - s) as f32, s as f32, 0.0, 0.0];
    c.page = [0.0; PATCHES];
    true
}

/// PROBE: weeding: every slot of patch `pp` of cell `i` not of starting kind `keep` dies whole, to the ground as remains
/// with its heat (as `clear`). Returns the kg killed.
pub fn weed(w: &mut World, i: usize, pp: u8, keep: u32) -> f64 {
    let Some(p) = Parts::of(&w.chem) else { return 0.0 };
    let (mm, e) = (p.mm(&w.chem), p.content(&w.chem));
    let c = &mut w.life.cells[i].cover;
    let was = c.temp(&w.chem, &p);
    let mut m = [0.0; NP];
    let xs: Vec<usize> = c.held().filter(|&x| c.patch[x] == pp && c.root[x] != keep).collect();
    if xs.is_empty() {
        return 0.0;
    }
    for x in xs {
        (0..NP).for_each(|q| m[q] += c.mol[x][q]);
        c.clear(x);
    }
    let heat = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * was).min(c.e);
    c.e -= heat;
    w.counts.cover_budget[DIED] -= held(&e, &m);
    let (from, to) = (Node::new(Class::Life, i), Node::ground(i));
    let mut moves: Vec<_> = (0..NP).map(|x| Move { from, to, k: p.sub[x] as u16, mol: m[x], heat: 0.0 }).collect();
    moves.push(Move { from, to, k: 0, mol: 0.0, heat });
    apply(w, &[], &moves);
    super::built_kg(&mm, &m) + m[super::FUEL] * mm[super::FUEL]
}

/// PROBE: start every cell's count of the seeds its slots sent (`Cover::seeded`) again.
pub fn reset_seeded(w: &mut World) {
    w.life.cells.iter_mut().for_each(|c| c.cover.seeded = [[0.0; 2]; super::SLOTS]);
}

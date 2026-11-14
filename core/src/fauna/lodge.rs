//! Where the newborns of a slot of takers belong (laws.md 3b), by the rule of the shared slots of life (`life::kin`):
//! the newborns of a month are one packet with their parent's genes after a generation's mutation; it joins the slot
//! of its kind it is alike to, or founds one of its own. Slots of takers join only when alike in every gene, never
//! because the cell's slots are all held: a mean of two unlike kinds would be good at neither.

use super::{distance, grow::whole, Fauna, BODY, FS, NF, RANGE};
use crate::life::kin::{self, APART};
use crate::life::{Books, NO_ROOT, WET_TISSUE};
use crate::rng::Rng;
use crate::World;

/// The newborns of a slot in a month: their slot, their count and the mol of body matter they hold (in its young).
pub(super) struct Packet {
    pub(super) x: usize,
    pub(super) n: f64,
    pub(super) body: f64,
}

/// The packet `p` of the newborns of `f` (its parts the substances `sub` of molar masses `mm`), its genes its parent slot's after a generation's mutation drawn from `r` (`life::kin::mutate`, `RANGE`,
/// `mut_size`), lodges: it joins the slot of its kind within `APART` × `alike` steps of it, the nearest (`kin::kin`;
/// the genes the mean by living mass); else it founds a slot: an empty one, else the one in deficit (`Fauna::net`
/// below zero, the worst first, not its own nor one whose newborns still wait in `waiting`), whose members die as
/// remains (their count added to `died`); else it stays in its
/// parent slot with no new genes. Its young count becomes whole where it is under `N_WHOLE` (`whole`).
pub(super) fn lodge(w: &World, (f, p): (&mut Fauna, &Packet), (sub, mm): (&[usize; NF], &[f64; NF]), (r, b, died): (&mut Rng, &mut Books<0>, &mut f64), waiting: &[usize]) {
    let ph = &w.cfg.phys;
    let (x, root) = (p.x, f.root[p.x]);
    let genes = kin::mutate(&f.genes[x], &RANGE, ph.mut_size, r);
    let kin = kin::kin(&f.genes, &f.root, (&genes, root), APART * ph.alike, distance);
    let poorest = (0..FS).filter(|&y| f.root[y] != NO_ROOT && f.net[y] < 0.0 && y != x && !waiting.contains(&y)).min_by(|&a, &z| f.net[a].total_cmp(&f.net[z]));
    let (to, founds) = match (kin, kin::empty(&f.root), poorest) {
        (Some(y), ..) => (y, false),
        (None, Some(y), _) => (y, true),
        (None, None, Some(y)) => {
            *died += f.members(y);
            for g in [y, FS + y] {
                (0..NF).for_each(|q| b.dead[sub[q]] += f.mol[g][q]);
                (f.n[g], f.mol[g]) = (0.0, [0.0; NF]);
            }
            (y, true)
        }
        (None, None, None) => return,
    };
    let kg = p.body * mm[BODY] / (1.0 - WET_TISSUE);
    if founds {
        (f.genes[to], f.root[to], f.net[to]) = (genes, root, 0.0);
    } else {
        // Its parent slot already holds it.
        let held = f.living(mm, to) - if to == x { kg } else { 0.0 };
        let share = (kg / (kg + held).max(1e-300)) as f32;
        (0..genes.len()).for_each(|g| f.genes[to][g] += share * (genes[g] - f.genes[to][g]));
    }
    if to != x {
        (f.n[FS + x], f.mol[FS + x][BODY]) = (f.n[FS + x] - p.n, f.mol[FS + x][BODY] - p.body);
        (f.n[FS + to], f.mol[FS + to][BODY]) = (f.n[FS + to] + p.n, f.mol[FS + to][BODY] + p.body);
        f.n[FS + to] = whole(f.n[FS + to], r);
    }
}

/// Slots of `f` alike in every gene (within `APART` × `alike` steps, `kin::together`) are one: the lighter joins the
/// heavier, its groups' members and amounts added to the heavier's, the genes and balance the mean by living mass
/// (its parts of molar masses `mm`).
pub(super) fn merge_alike(w: &World, f: &mut Fauna, mm: &[f64; NF]) {
    let within = APART * w.cfg.phys.alike;
    let living = |f: &Fauna, x: usize| f.living(mm, x);
    for a in 0..FS {
        for z in a + 1..FS {
            if !kin::together(&f.genes, &f.root, (a, z), within, distance) {
                continue;
            }
            let (big, small) = if living(f, z) > living(f, a) { (z, a) } else { (a, z) };
            let share = (living(f, small) / (living(f, big) + living(f, small)).max(1e-300)) as f32;
            let (gs, ns) = (f.genes[small], f.net[small]);
            (0..gs.len()).for_each(|g| f.genes[big][g] += share * (gs[g] - f.genes[big][g]));
            f.net[big] += share * (ns - f.net[big]);
            for (from, to) in [(small, big), (FS + small, FS + big)] {
                f.n[to] += std::mem::take(&mut f.n[from]);
                (0..NF).for_each(|q| f.mol[to][q] += std::mem::take(&mut f.mol[from][q]));
            }
            (f.root[small], f.net[small]) = (NO_ROOT, 0.0);
        }
    }
}

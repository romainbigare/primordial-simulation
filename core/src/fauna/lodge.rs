//! Where the newborns of a slot of takers belong (laws.md 3b), by the rule of the shared slots of life (`life::kin`):
//! the newborns of a month are one packet with their parent's genes after a generation's mutation; it joins the slot
//! of its kind it is alike to, or founds one of its own. Slots of takers join only when alike in every gene, never
//! because the cell's slots are all held: a mean of two unlike kinds would be good at neither.

use super::{distance, grow::whole, Fauna, BODY, FS, NF, NG, RANGE};
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

/// A slot that something landing took because it was in deficit (`room`): its members and its amounts, mol of each
/// part over both its groups, now cleared.
#[derive(Default)]
pub(super) struct Evicted {
    pub(super) n: f64,
    pub(super) mol: [f64; NF],
}

/// Where what lands with genes `genes` of starting kind `root` goes among the slots of `f`: the slot of its kind within
/// `APART` × `alike` (`Physics::alike`) steps of it in every gene, the nearest (`kin::kin`; false: it joins); else a slot it founds (true):
/// an empty one, else the one most in deficit (`Fauna::net` below zero, not one of `not`), whose members and amounts it
/// clears (`Evicted`); else none.
pub(super) fn room(alike: f32, f: &mut Fauna, (genes, root): (&[f32; NG], u32), not: &[usize]) -> (Option<(usize, bool)>, Evicted) {
    let mut out = Evicted::default();
    if let Some(y) = kin::kin(&f.genes, &f.root, (genes, root), APART * alike, distance) {
        return (Some((y, false)), out);
    }
    if let Some(y) = kin::empty(&f.root) {
        return (Some((y, true)), out);
    }
    let poorest = (0..FS).filter(|&y| f.root[y] != NO_ROOT && f.net[y] < 0.0 && !not.contains(&y)).min_by(|&a, &z| f.net[a].total_cmp(&f.net[z]));
    let Some(y) = poorest else { return (None, out) };
    out.n = f.members(y);
    for g in [y, FS + y] {
        (0..NF).for_each(|q| out.mol[q] += f.mol[g][q]);
        (f.n[g], f.mol[g]) = (0.0, [0.0; NF]);
    }
    (Some((y, true)), out)
}

/// What lands with genes `genes` of starting kind `root` takes slot `y` of `f` (`room`): it founds it (`founds`) with
/// its genes, or joins it, the genes the mean by living mass, it holding `kg` of the `living` kg of the slot after.
pub(super) fn take_slot(f: &mut Fauna, y: usize, (genes, root): (&[f32; NG], u32), founds: bool, (kg, living): (f64, f64)) {
    if founds {
        (f.genes[y], f.root[y], f.net[y]) = (*genes, root, 0.0);
    } else {
        let share = (kg / living.max(1e-300)) as f32;
        (0..NG).for_each(|g| f.genes[y][g] += share * (genes[g] - f.genes[y][g]));
    }
}

/// The packet `p` of the newborns of `f` (its parts the substances `sub` of molar masses `mm`), its genes its parent
/// slot's after a generation's mutation drawn from `r` (`life::kin::mutate`, `RANGE`, `mut_size`), lodges by `room`;
/// a slot in deficit it takes (not its own, nor one whose newborns still wait in `waiting`) dies as remains (its
/// members added to `died`); with no room it stays in its parent slot with no new genes. Its young count becomes whole
/// where it is under `N_WHOLE` (`whole`).
pub(super) fn lodge(w: &World, (f, p): (&mut Fauna, &Packet), (sub, mm): (&[usize; NF], &[f64; NF]), (r, b, died): (&mut Rng, &mut Books<0>, &mut f64), waiting: &[usize]) {
    let (x, root) = (p.x, f.root[p.x]);
    let genes = kin::mutate(&f.genes[x], &RANGE, w.cfg.phys.mut_size, r);
    let not: Vec<usize> = waiting.iter().copied().chain([x]).collect();
    let (Some((to, founds)), evicted) = room(w.cfg.phys.alike, f, (&genes, root), &not) else { return };
    *died += evicted.n;
    (0..NF).for_each(|q| b.dead[sub[q]] += evicted.mol[q]);
    let kg = p.body * mm[BODY] / (1.0 - WET_TISSUE);
    if to != x {
        (f.n[FS + x], f.mol[FS + x][BODY]) = (f.n[FS + x] - p.n, f.mol[FS + x][BODY] - p.body);
        (f.n[FS + to], f.mol[FS + to][BODY]) = (f.n[FS + to] + p.n, f.mol[FS + to][BODY] + p.body);
        f.n[FS + to] = whole(f.n[FS + to], r);
    }
    let living = f.living(mm, to);
    take_slot(f, to, (&genes, root), founds, (kg, living));
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

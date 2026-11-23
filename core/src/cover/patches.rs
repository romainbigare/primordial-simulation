//! The patches of a cell's cover (laws.md 3): its ground falls into up to `PATCHES` patches, each a share of the cell
//! and an age, years since it was made, and each slot stands in one of them (`Cover::patch`). A patch is a stand of its
//! own, as a gap in a forest is (Watt 1947; the gap models, Shugart 1984): its plants hold its ground, shade, crowd and
//! draw the key liquid and the minerals on it alone; the soil, its liquid and minerals and the air stay the cell's,
//! each patch drawing on its share. The laws know no place of a patch in its cell; a patch past the first keeps the
//! sample of its cell where it was made (`Patch::at`), about which its ground lies (`inside::patch_ground`), for the
//! pictures. Events make patches, each where it hits a share of a patch at once (`hit`, `open_event`: a die-off,
//! `gaps`; a flame, `flame::burn`), and so do edits (`World::clear`, `World::split`); once a year alike patches fuse
//! (`year`).

use super::budget::{held, DIED};
use super::groups::Group;
use super::{stand, Cover, Parts, FUEL, GROUPS, NO_ROOT, NP, SLOTS, WALL};
use crate::life::kin;
use crate::rng::{Purpose, Rng};
use crate::{Physics, World};
use serde::{Deserialize, Serialize};

/// The most patches a cell's cover holds.
pub const PATCHES: usize = 3;
/// Two patches are alike when the same kinds hold them (`HOLDS`), their tallest plants stand within this share of
/// the taller's height and their built cover per m² of their own ground within `ALIKE_COVER` of the larger's: rules
/// of the model, as `kin::APART` is for slots.
pub const ALIKE_HEIGHT: f64 = 0.2;
/// Two patches are alike only when their built cover per m² of their own ground is within this share of the larger's
/// (`ALIKE_HEIGHT`): young plants reach the height of the old while they hold far less mass.
pub const ALIKE_COVER: f64 = 0.2;
/// A starting kind holds a patch when its slots there hold at least this share of the patch's built cover.
pub const HOLDS: f64 = 0.1;
/// An event makes a patch of the ground it hits on a patch only where that is at least this share of the patch's
/// ground, and the patch's own where it leaves less than this share (`hit`): a patch is for an event that hits a share
/// of a stand at once, never for one plant.
pub const EVENT_LEAST: f64 = 0.15;

/// What an event is to a patch (`hit`): it hits a share of its ground, which becomes a patch of its own
/// (`open_event`), or all of it but less than `EVENT_LEAST`, and the patch is the event's whole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Hit {
    Part,
    Whole,
}

/// What an event that hits the share `s` of a patch's own ground is to it (`Hit`): none under `EVENT_LEAST`.
pub(crate) fn hit(s: f64) -> Option<Hit> {
    if s >= 1.0 - EVENT_LEAST {
        Some(Hit::Whole)
    } else if s >= EVENT_LEAST {
        Some(Hit::Part)
    } else {
        None
    }
}

/// Where in patch `k` of the cover of cell `i` of `w` the new patch of an event opens: one of its samples
/// (`inside::patch_ground`), drawn from the world's seed by the month, the cell and the patch for the event's
/// `purpose`.
pub(crate) fn point(w: &World, i: usize, k: usize, purpose: Purpose) -> u32 {
    let g = crate::inside::patch_ground(w, i);
    let mine: Vec<usize> = (0..g.of.len()).filter(|&p| g.of[p] as usize == k).collect();
    let mut r = Rng::new(w.seed, w.month(), (i * PATCHES + k) as u64, purpose);
    mine.get(r.below(mine.len())).map_or_else(|| w.centre_sample(i), |&p| p) as u32
}

/// A patch of a cell's cover: its share of the cell's ground (0: not in use), its age, years since it was made, the
/// sample of its cell where it was made, row by row from the cell's north-west corner (`inside::PatchGround`; the first
/// patch, the rest of the cell, has none: 0), the walls of its plants flames killed whole that still stand dead on its
/// ground until they fall (`flame::burn`, `grow::fall`), mol of each part, the height they stood to, m, and the years
/// since they died (each the mean of theirs by amount).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub share: f64,
    pub age: f32,
    pub at: u32,
    pub snag: [f64; NP],
    pub snag_h: f32,
    pub snag_age: f32,
}

impl Patch {
    /// The amounts `mol` of walls of plants `h` m tall, dead for `age` years, now stand dead on its ground (`snag`).
    pub(crate) fn stand_dead(&mut self, mol: &[f64; NP], h: f64, age: f64) {
        let (was, add) = (self.snag.iter().sum::<f64>(), mol.iter().sum::<f64>());
        if add <= 0.0 {
            return;
        }
        let mean = |a: f32, b: f64| ((a as f64 * was + b * add) / (was + add)) as f32;
        (self.snag_h, self.snag_age) = (mean(self.snag_h, h), mean(self.snag_age, age));
        self.snag.iter_mut().zip(mol).for_each(|(a, &m)| *a += m);
    }
    /// The share `f` of its dead walls standing, taken from it (they stand on that share of its ground).
    fn take_dead(&mut self, f: f64) -> [f64; NP] {
        let out = self.snag.map(|m| m * f);
        self.snag.iter_mut().zip(&out).for_each(|(a, m)| *a -= m);
        out
    }
}

/// One patch that holds all of its cell.
pub(super) fn whole() -> [Patch; PATCHES] {
    std::array::from_fn(|k| Patch { share: if k == 0 { 1.0 } else { 0.0 }, ..Patch::default() })
}

impl Cover {
    /// Its patches in use: they come first.
    pub fn patch_count(&self) -> usize {
        self.patches.iter().take_while(|p| p.share > 0.0).count()
    }
    /// The share of its cell's ground that the patch of slot `x` holds.
    pub fn patch_share(&self, x: usize) -> f64 {
        self.patches[self.patch[x] as usize].share
    }
    /// The slots of its arrays it uses in a world of `variety` kinds a cell (`Config::variety`): of one patch the first
    /// `variety`, of more patches all, each patch at most `variety` (`seed::lodge`).
    pub fn slots(&self, variety: usize) -> usize {
        if self.patch_count() > 1 { SLOTS } else { variety }
    }
    /// Its slots that hold cover in patch `k`.
    pub fn held_in(&self, k: usize) -> impl Iterator<Item = usize> + '_ {
        self.held().filter(move |&x| self.patch[x] as usize == k)
    }
    /// Slot `q`, empty, takes of slot `x` the share `s[g]` of the plants of each of its groups `g` and of their built
    /// cover: of its body matter and its walls what those plants hold (`Cover::sizes`), of its fuel and of its fallen
    /// crowns the share of its built cover they hold; and the rest of its numbers as they are. With one share for every
    /// group, that share of each amount (all of it with 1, and slot `x` is emptied then).
    pub(super) fn part(&mut self, (ph, mm): (&Physics, &[f64; NP]), x: usize, q: usize, s: &[f64; GROUPS]) {
        let (g, held) = self.groups(mm, x);
        let even = s[..held].iter().all(|&f| f == s[0]);
        let by = if even { [s[0]; NP] } else { self.shares_of(ph, mm, x, s) };
        for (k, f) in by.iter().enumerate() {
            let m = self.mol[x][k] * f;
            (self.mol[q][k], self.mol[x][k]) = (m, self.mol[x][k] - m);
        }
        let f = self.fell[x] * by[FUEL];
        (self.fell[q], self.fell[x]) = (f, self.fell[x] - f);
        if even {
            for k in 0..GROUPS {
                let n = self.plants[x][k] * s[0];
                (self.plants[q][k], self.plants[x][k]) = (n, self.plants[x][k] - n);
            }
            self.share[q] = self.share[x];
        } else {
            let mut taken: [Group; GROUPS] = std::array::from_fn(|k| Group { n: g[k].n * s[k], kg: g[k].kg * s[k] });
            let mut left: [Group; GROUPS] = std::array::from_fn(|k| Group { n: g[k].n - taken[k].n, kg: g[k].kg - taken[k].kg });
            self.set_groups(q, &mut taken[..held]);
            self.set_groups(x, &mut left[..held]);
        }
        (self.genes[q], self.root[q], self.full[q], self.net[q]) = (self.genes[x], self.root[x], self.full[x], self.net[x]);
        (self.rooted[q], self.need[q], self.line[q], self.stands[q]) = (self.rooted[x], self.need[x], self.line[x], self.stands[x]);
        (self.airless[q], self.drained[q], self.strip[q], self.patch[q]) = (self.airless[x], self.drained[x], self.strip[x], self.patch[x]);
        if even && s[0] >= 1.0 {
            self.clear(x);
        }
    }
    /// The share of each of the amounts of slot `x` that the share `s[g]` of the plants of each of its groups `g` hold
    /// (`Cover::part`): of its soft tissue (`life::SOFT`) and walls each group's (`Cover::sizes`), of its
    /// fuel its built cover's.
    fn shares_of(&self, ph: &Physics, mm: &[f64; NP], x: usize, s: &[f64; GROUPS]) -> [f64; NP] {
        let z = self.sizes(ph, mm, x);
        let of = |part: &dyn Fn(usize) -> f64| {
            let all: f64 = (0..z.held).map(|g| z.count[g] * z.mass[g] * part(g)).sum();
            (0..z.held).map(|g| s[g] * z.count[g] * z.mass[g] * part(g)).sum::<f64>() / all.max(1e-300)
        };
        let (fuel, soft, walls) = (of(&|_| 1.0), of(&|g| 1.0 - z.walls[g]), of(&|g| z.walls[g]));
        std::array::from_fn(|q| match q {
            FUEL => fuel,
            WALL => walls,
            _ => soft,
        })
    }
}

/// What patch `k` of cover `c` is to another (`alike`): the height its tallest plants stand at, m, its built cover over
/// its share of the cell (kg; as its cover per m² of its own ground, the cell's ground the same for every patch), and
/// the starting kinds that hold it (`HOLDS`), in order.
fn sketch(c: &Cover, (ph, mm, rho): (&Physics, &[f64; NP], f64), k: usize) -> (f64, f64, Vec<u32>) {
    let all: f64 = c.held_in(k).map(|x| c.built(mm, x)).sum();
    let tallest = c.held_in(k).map(|x| c.height(ph, mm, rho, x)).fold(0.0, f64::max);
    let mut kinds: Vec<u32> = c.held_in(k).map(|x| c.root[x]).collect();
    kinds.sort_unstable();
    kinds.dedup();
    kinds.retain(|&r| c.held_in(k).filter(|&x| c.root[x] == r).map(|x| c.built(mm, x)).sum::<f64>() >= HOLDS * all);
    (tallest, all / c.patches[k].share, kinds)
}

/// Whether two patches of sketches `a` and `b` (`sketch`) are alike: the same kinds hold them, their tallest plants
/// stand within `ALIKE_HEIGHT` of the taller's height and their built cover per m² of their own ground within
/// `ALIKE_COVER` of the larger's.
fn alike(a: &(f64, f64, Vec<u32>), b: &(f64, f64, Vec<u32>)) -> bool {
    let near = |x: f64, y: f64, within: f64| (x - y).abs() <= within * x.max(y);
    a.2 == b.2 && near(a.0, b.0, ALIKE_HEIGHT) && near(a.1, b.1, ALIKE_COVER)
}

/// Patch `from` of cover `c` joins patch `into`: their shares add, its age the mean of theirs by share, its dead walls
/// standing join `into`'s, its slots stand in `into`. The patches after `from` move down a place.
fn join(c: &mut Cover, into: usize, from: usize) {
    let (a, b) = (c.patches[into], c.patches[from]);
    let share = a.share + b.share;
    c.patches[into] = Patch { share, age: ((a.age as f64 * a.share + b.age as f64 * b.share) / share) as f32, ..a };
    c.patches[into].stand_dead(&b.snag, b.snag_h as f64, b.snag_age as f64);
    c.patches.copy_within(from + 1.., from);
    c.patches[PATCHES - 1] = Patch::default();
    for x in 0..SLOTS {
        let k = c.patch[x] as usize;
        c.patch[x] = match k {
            _ if k == from => into - (into > from) as usize,
            _ if k > from => k - 1,
            _ => k,
        } as u8;
    }
}

/// A new patch of the share `s` of the cell of cover `c`, age 0, made at its sample `point` (`Patch::at`), its ground
/// given by patch `from` (a gap, `gaps`), or where none by each patch in use, the share `s` of its own (the laws know no
/// place of a patch in its cell), with the dead walls standing on that ground. Where `PATCHES` are in use the new one joins the one most alike to bare ground: the
/// one whose tallest plants stand lowest, which keeps its sample. Returns the patch it is; none where that is `from`
/// (nothing changes then).
pub(super) fn open(c: &mut Cover, s: f64, point: u32, from: Option<usize>, at: (&Physics, &[f64; NP], f64)) -> Option<usize> {
    let n = c.patch_count();
    let low = (n == PATCHES).then(|| (0..n).min_by(|&a, &b| sketch(c, at, a).0.total_cmp(&sketch(c, at, b).0)).unwrap_or(0));
    if low.is_some() && low == from {
        return None;
    }
    let mut dead = [0.0; NP];
    let mut gives = |p: &mut Patch, f: f64| p.take_dead(f).iter().zip(&mut dead).for_each(|(m, d)| *d += m);
    // (The height and the age of what moves: of the patch that gives it, or the mean of all by amount.)
    let (mut h, mut age, mut of) = (0.0, 0.0, 0.0);
    let mut weigh = |p: &Patch, f: f64| {
        let m = f * p.snag.iter().sum::<f64>();
        (h, age, of) = (h + m * p.snag_h as f64, age + m * p.snag_age as f64, of + m);
    };
    match from {
        Some(k) => {
            let f = s / c.patches[k].share;
            weigh(&c.patches[k], f);
            gives(&mut c.patches[k], f);
            c.patches[k].share -= s;
        }
        None => c.patches[..n].iter_mut().for_each(|p| {
            weigh(p, s);
            gives(p, s);
            p.share *= 1.0 - s;
        }),
    }
    let (h, age) = if of > 0.0 { (h / of, age / of) } else { (0.0, 0.0) };
    let k = match low {
        None => {
            c.patches[n] = Patch { share: s, age: 0.0, at: point, ..Patch::default() };
            n
        }
        Some(low) => {
            let p = c.patches[low];
            c.patches[low] = Patch { share: p.share + s, age: (p.age as f64 * p.share / (p.share + s)) as f32, ..p };
            low
        }
    };
    c.patches[k].stand_dead(&dead, h, age);
    let all: f64 = c.patches.iter().map(|p| p.share).sum();
    c.patches.iter_mut().for_each(|p| p.share /= all);
    Some(k)
}

/// Patch `k` of cover `c` gives the share `s` of its cell as a new patch about its sample `point` (`open`), each of its
/// slots `x` giving it the share `by[x][g]` of the plants of each of its groups `g` as a slot of its own there
/// (`Cover::part`; none where it gives no plants). Returns the patch it is; none (nothing done) where the arrays hold
/// fewer empty slots than it has slots that give, or the new one would join `k`.
pub(super) fn carve(c: &mut Cover, at: (&Physics, &[f64; NP], f64), (k, s, point): (usize, f64, u32), by: &[[f64; GROUPS]; SLOTS]) -> Option<usize> {
    let giving: Vec<usize> = c.held_in(k).filter(|&x| by[x][..c.held_groups(x)].iter().any(|&f| f > 0.0)).collect();
    if c.root.iter().filter(|&&r| r == NO_ROOT).count() < giving.len() {
        return None;
    }
    let into = open(c, s, point, Some(k), at)?;
    for x in giving {
        let q = kin::empty(&c.root).expect("an empty slot");
        c.part((at.0, at.1), x, q, &by[x]);
        c.patch[q] = into as u8;
    }
    Some(into)
}

/// Patch `k` of cover `c` opens the share `s` of its cell as a new patch of an event about its sample `point`
/// (`carve`), each of its slots `x` giving it the share `by(c)[x][g]` of the plants of each group `g`. Room for its slots
/// is made first: while the arrays hold fewer empty slots than the patch holds slots, its two nearest join
/// (`seed::room`), and `by` reads the cover after that. Returns the patch it is and the slots it took, which the
/// event's law acts on before they settle (`settle`); none where `carve` makes none.
pub(crate) fn open_event(c: &mut Cover, at: (&Physics, &[f64; NP], f64), (k, s, point): (usize, f64, u32), by: impl Fn(&Cover) -> [[f64; GROUPS]; SLOTS]) -> Option<(usize, Vec<usize>)> {
    let empty = |c: &Cover| c.root.iter().filter(|&&r| r == NO_ROOT).count();
    while empty(c) < c.held_in(k).count() && super::seed::room(c, at.1, SLOTS, k).is_some() {}
    let free: Vec<usize> = (0..SLOTS).filter(|&x| c.root[x] == NO_ROOT).collect();
    let into = carve(c, at, (k, s, point), &by(c))?;
    Some((into, free.into_iter().filter(|&x| c.root[x] != NO_ROOT).collect()))
}

/// The slots of cover `c` meeting slots of their kind in their patch join them (`stand::merge_alike`), and a patch of
/// more than `variety` slots folds (`fold`): after an event's new patch joined one in use.
pub(crate) fn settle(c: &mut Cover, (ph, mm): (&Physics, &[f64; NP]), variety: usize) {
    stand::merge_alike(c, mm, (ph, SLOTS));
    fold(c, (ph, mm), variety);
}

/// A patch of cover `c` holds no more than `variety` slots: while it holds more, its two nearest slots join
/// (`seed::room`); a cover left with one patch keeps its slots in its first `variety` (`Cover::slots`).
pub(super) fn fold(c: &mut Cover, (ph, mm): (&Physics, &[f64; NP]), variety: usize) {
    for k in 0..c.patch_count() {
        while c.held_in(k).count() > variety && super::seed::room(c, mm, SLOTS, k).is_some() {}
    }
    if c.patch_count() == 1 {
        let past: Vec<usize> = (variety..SLOTS).filter(|&x| c.root[x] != NO_ROOT).collect();
        for x in past {
            if let Some(q) = kin::empty(&c.root[..variety]) {
                c.part((ph, mm), x, q, &[1.0; GROUPS]);
            }
        }
    }
}

/// A year of the patches of every cell: each ages a year, and so do the dead walls standing on every patch; while two
/// are alike (`alike`) the younger joins the older (`join`), its slots meeting slots of their kind joining them (`stand::merge_alike`), and a patch of more than
/// `variety` slots folding (`fold`); then the canopy that fell over the year opens gaps (`gaps::year`).
pub(super) fn year(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let (mm, rho) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0]);
    let (ph, variety) = (&w.cfg.phys, w.cfg.variety);
    for p in w.life.cells.iter_mut().flat_map(|x| x.cover.patches.iter_mut()) {
        p.snag_age += 1.0;
    }
    for c in w.life.cells.iter_mut().map(|x| &mut x.cover).filter(|c| c.patch_count() > 1) {
        let n = c.patch_count();
        c.patches[..n].iter_mut().for_each(|p| p.age += 1.0);
        loop {
            let s: Vec<_> = (0..c.patch_count()).map(|k| sketch(c, (ph, &mm, rho), k)).collect();
            let mut pairs = (0..s.len()).flat_map(|a| (a + 1..s.len()).map(move |z| (a, z)));
            let Some((a, z)) = pairs.find(|&(a, z)| alike(&s[a], &s[z])) else { break };
            let (into, from) = if c.patches[z].age > c.patches[a].age { (z, a) } else { (a, z) };
            join(c, into, from);
            stand::merge_alike(c, &mm, (ph, SLOTS));
            fold(c, (ph, &mm), variety);
        }
    }
    super::gaps::year(w);
}

impl World {
    /// An edit: the share `s` (0 to 1) of cell `i` becomes a new patch of its cover (`open`), made at its sample `at`
    /// (`World::sample_in`; none: its centre, `World::centre_sample`), whose plants all die above
    /// the ground, a clearing: of every slot the share `s` of its amounts goes to the cell's ground as remains with its
    /// heat, its plants with it (`Cover::cut`), booked as the cover's deaths (`DIED`). Its seeds lying, its cured leaves
    /// and its dead walls stay. False (nothing done) for a share outside 0 to 1 or a chemistry with no cover.
    pub fn clear(&mut self, i: usize, s: f64, at: Option<usize>) -> bool {
        let Some(p) = Parts::of(&self.chem) else { return false };
        if !(s > 0.0 && s < 1.0) {
            return false;
        }
        let point = at.unwrap_or_else(|| self.centre_sample(i)) as u32;
        let (mm, rho, e) = (p.mm(&self.chem), self.chem.subs[p.sub[WALL]].density[0], p.content(&self.chem));
        let c = &mut self.life.cells[i].cover;
        let (dead, heat) = c.cut(&self.chem, &p, s);
        for x in (0..SLOTS).filter(|&x| c.root[x] != NO_ROOT) {
            c.plants[x].iter_mut().for_each(|n| *n -= *n * s);
        }
        open(c, s, point, None, (&self.cfg.phys, &mm, rho));
        self.counts.cover_budget[DIED] -= held(&e, &dead);
        crate::contract::died_at(self, i, (0..NP).map(|q| (p.sub[q], dead[q])), heat);
        true
    }
    /// An edit that changes nothing: the share `s` (0 to 1) of the first patch of cell `i` becomes a new patch of its
    /// cover at its centre (`World::centre_sample`), of its age, each slot of the first patch giving it the share `s` of
    /// its amounts and plants as a slot of its own there (`carve`); the two patches are alike. A control run the
    /// patches' law is checked against: each patch stands as the cell did. False (nothing done) for a share outside 0 to
    /// 1, a cell whose `PATCHES` are in use or one with too few empty slots.
    pub fn split(&mut self, i: usize, s: f64) -> bool {
        let Some(p) = Parts::of(&self.chem) else { return false };
        let (mm, rho, point) = (p.mm(&self.chem), self.chem.subs[p.sub[WALL]].density[0], self.centre_sample(i) as u32);
        let c = &mut self.life.cells[i].cover;
        if !(s > 0.0 && s < 1.0) || c.patch_count() == PATCHES {
            return false;
        }
        let Some(k) = carve(c, (&self.cfg.phys, &mm, rho), (0, s * c.patches[0].share, point), &[[s; GROUPS]; SLOTS]) else { return false };
        c.patches[k].age = c.patches[0].age;
        true
    }
}

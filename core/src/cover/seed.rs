//! Seeds (laws.md 3): what a slot sends out in its month lands, when every cell has had its month, in the cell it
//! goes to, cell by cell in the order made, and lies on its ground (`lying`); what wakes there takes hold in the cover
//! (`lodge`): in the nearest slot of its kind, or as seedlings in a slot of its own; what lives after is the light's to
//! decide.

use super::budget::{held, DIED, PLACED, SEEDS};
use super::lying::LANDED;
use super::{built_kg, dec, distance, Cover, APART, Parts, FUEL, MATTER, NG, NP, STORE, WALL, WALLS};
use crate::chem::Chemistry;
use crate::life::kin;
use crate::store::{apply, Class, Move, Node};
use crate::World;

/// Seeds of one slot for cell `to`: mol of each part, their genes, their starting kind and their heat, J.
#[derive(Clone, Debug)]
pub struct Seed {
    pub to: u32,
    pub mol: [f64; NP],
    pub genes: [f32; NG],
    pub root: u32,
    pub heat: f64,
}

/// The whole lots the seeds of a slot's month go in: each lands at a point the dispersal kernel draws (`life::lot`). It
/// sets the noise of where the seeds land, not the mean: in expectation they land as the kernel says, at any size of cell.
pub(super) const LOTS: usize = 8;

/// A share of a slot's seeds that stays in its cell below this is the rounding of the shares of where they land: it
/// goes with the others.
const STAYS_LEAST: f64 = 1e-9;

/// Seeds of slot `x` (`mol` of each part, `heat` J, `genes`) shared out over the cells they land in, `lands` (cell,
/// share; the lots of `life::lot`), what stays in cell `i` last: each part as a packet of its own (`Seed`), the last taking
/// what the others leave of each part and of the heat, so they hold all of it exactly.
pub(super) fn send(c: &mut Cover, x: usize, (mol, heat): ([f64; NP], f64), genes: [f32; NG], (i, mut lands): (usize, Vec<(usize, f64)>)) {
    let stays = 1.0 - lands.iter().map(|l| l.1).sum::<f64>();
    if stays > STAYS_LEAST || lands.is_empty() {
        lands.push((i, stays));
    }
    let (mut left, mut warm, last) = (mol, heat, lands.len() - 1);
    for (k, &(j, share)) in lands.iter().enumerate() {
        let part: [f64; NP] = std::array::from_fn(|q| if k == last { left[q] } else { (mol[q] * share).min(left[q]) });
        let h = if k == last { warm } else { (heat * share).min(warm) };
        (0..NP).for_each(|q| left[q] -= part[q]);
        warm -= h;
        c.seeds.push(Seed { to: j as u32, mol: part, genes, root: c.root[x], heat: h });
    }
}

/// Land the seeds of every cell's last month (`Cover::seeds`), cell by cell in the order made: they lie on the ground of
/// the cell they reach (`Lying::lodge`) with their heat. The potential energy of the life that left a cell's head and
/// came to it is booked once for each cell and way (`store::life_sent`).
pub fn land(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let mm = p.mm(&w.chem);
    let mut sent = vec![[0.0; 2]; w.life.cells.len()];
    let e = p.content(&w.chem);
    for from in 0..w.life.cells.len() {
        for s in std::mem::take(&mut w.life.cells[from].cover.seeds) {
            let i = s.to as usize;
            let kg: f64 = (0..NP).map(|q| s.mol[q] * mm[q]).sum();
            (sent[from][0], sent[i][1]) = (sent[from][0] + kg, sent[i][1] + kg);
            w.counts.cover_budget[SEEDS] += held(&e, &s.mol);
            w.counts.seeds[LANDED] += kg;
            let c = &mut w.life.cells[i].cover;
            c.lying.lodge(&mm, &s, w.cfg.phys.alike);
            c.e += s.heat;
        }
    }
    crate::store::life_sent(w, sent.into_iter().enumerate().filter(|x| x.1 != [0.0; 2]));
}

/// Where seeds went (`lodge`): into the cell's cover; into a slot in deficit, whose amounts (mol of each part) die; or
/// nowhere, the cell's one slot held by a stand of another kind that pays its way (they stay as seeds).
pub enum Landed {
    In,
    Out([f64; NP]),
    Refused,
}

/// Seeds `s` lodge in cover `c` on the share `share` of its cell's ground of `area` m² (`cover::ground`) their kind
/// stands on (`cover::footing` of the ground it may hold), of a world of settings `cfg`, with their heat and their store: into the most alike slot of their starting kind within `APART` × `alike` steps
/// (`distance`), the genes the mean by built cover, the plants each slot took in added to `recruits` (where they take
/// ground below its line, `soak::lowered`); else they found a slot of their own (its line `UNSET`) among the world's first `Config::variety`: an
/// empty one; else the one in deficit (`Cover::net` below zero, the worst first), whose cover dies; else the two slots
/// nearest each other by their genes join (`room`) and they found the slot it frees; a world whose cells hold one kind
/// (`Config::variety` 1) has no two to join, and the seeds do not take hold (`Landed::Refused`). So a cell of more slots
/// is never closed to seeds and a holder that pays its way is never pushed out: what lives after is the light's to decide, as seedlings live on
/// the light that reaches the bottom of the layers above them and what does not pay dies (the gap models, Shugart 1984).
pub fn lodge(c: &mut Cover, mm: &[f64; NP], s: &Seed, cfg: &crate::Config, (area, share): (f64, f64), recruits: &mut [f64; super::SLOTS]) -> Landed {
    let (ph, n) = (&cfg.phys, cfg.variety);
    let kg = built_kg(mm, &s.mol);
    let plants = super::stand::founded(ph, kg, area * share);
    let (mut kin, mut n_kin) = ([0; super::SLOTS], 0);
    for x in kin::kins(&c.genes[..n], &c.root[..n], (&s.genes, s.root), APART * ph.alike, distance) {
        (kin[n_kin], n_kin) = (x, n_kin + 1);
    }
    let kin = &kin[..n_kin];
    if !kin.is_empty() {
        // The seeds join the stand of their kind, shared over the slots that hold it by their built cover (a kind cut
        // into slots takes its seeds as the whole would), each as a slot of its own would, the genes the mean by built
        // cover; the last takes what the others leave, so the amounts are kept exactly.
        let all: f64 = kin.iter().map(|&x| c.built(mm, x)).sum();
        let mut left = s.mol;
        for (k, &x) in kin.iter().enumerate() {
            let f = if all > 0.0 { c.built(mm, x) / all } else { 1.0 / kin.len() as f64 };
            let part: [f64; NP] = std::array::from_fn(|q| if k + 1 == kin.len() { left[q] } else { (s.mol[q] * f).min(left[q]) });
            (0..NP).for_each(|q| left[q] -= part[q]);
            c.recruit(mm, x, (plants * f, built_kg(mm, &part)));
            recruits[x] += plants * f;
            (0..NG).for_each(|g| c.genes[x][g] += (kg * f / (kg * f + c.built(mm, x)).max(1e-300)) as f32 * (s.genes[g] - c.genes[x][g]));
            (0..NP).for_each(|q| c.mol[x][q] += part[q]);
        }
        c.e += s.heat;
        return Landed::In;
    }
    let poorest = (0..n).filter(|&x| c.net[x] < 0.0).min_by(|&a, &b| c.net[a].total_cmp(&c.net[b]).then(c.built(mm, a).total_cmp(&c.built(mm, b))));
    let (x, out) = match (kin::empty(&c.root[..n]), poorest) {
        (Some(x), _) => (x, None),
        (None, Some(x)) => (x, Some(c.mol[x])),
        (None, None) if n > 1 => (room(c, mm, n), None),
        (None, None) => return Landed::Refused,
    };
    (c.mol[x], c.genes[x], c.root[x], c.full[x], c.net[x], c.need[x]) = (s.mol, s.genes, s.root, false, 0.0, 0.0);
    (c.line[x], c.stands[x], c.airless[x], c.drained[x]) = (super::UNSET, share as f32, 0.0, 0.0);
    c.found(x, plants);
    c.e += s.heat;
    out.map_or(Landed::In, Landed::Out)
}

/// The lines of the slots of cover `c` of cell `i`, its land meeting their roots as `soak`, after seeds of `kg` of
/// built cover took hold in them as `recruits` plants (`lodge`), on `aired` m² of its ground their kind stands on: each
/// falls as its seedlings take its ground below it where their roots get the oxidising gas (`Footing::free`,
/// `soak::lowered`).
pub(super) fn lines(w: &World, c: &Cover, i: usize, soak: &super::Soak, (recruits, kg): (&[f64; super::SLOTS], f64), aired: f64) -> [f32; super::SLOTS] {
    let n: f64 = recruits.iter().sum();
    let lower = |x: usize| {
        let free = super::footing(w, i, c.genes[x][super::GAS] as f64, w.cfg.phys.root_least, soak, Some(c.line[x] as f64)).free;
        super::soak::lowered(w, i, c.line[x] as f64, (recruits[x], kg / n), (aired, free)) as f32
    };
    std::array::from_fn(|x| if recruits[x] > 0.0 { lower(x) } else { c.line[x] })
}

/// Room in cover `c` whose `n` slots in use all hold cover that pays its way: the two nearest each other by their genes
/// (`distance`), of any starting kind, join, the lighter into the heavier (`stand::join`): the slots are a limit of the
/// bookkeeping, and the two hold the same niche to within what it resolves. Returns the slot it leaves empty.
fn room(c: &mut Cover, mm: &[f64; NP], n: usize) -> usize {
    let (a, z) = kin::nearest(&c.genes[..n], distance);
    let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
    super::stand::join(c, mm, big, small);
    small
}

/// Place `kg` per m² of the ground its kind stands on (`cover::footing` at the least root need, of the cell's ground,
/// `cover::ground`) of cover of genes `genes` and starting kind `root` in cell `i`, an edit booked from the outside
/// node (`World::cross`): its built cover at the shares of its genes, its fuel at its `STORE` gene, at the cell's
/// ground temperature; it lodges as seeds do (`seed::lodge`). False if the chemistry has no cover or its kind has no
/// ground or no room in the cell (nothing is placed then).
pub fn place(w: &mut crate::World, i: usize, genes: [f32; NG], root: u32, kg: f64) -> bool {
    let Some(p) = Parts::of(&w.chem) else { return false };
    let (mm, cell) = (p.mm(&w.chem), super::ground(w, i));
    let soak = super::soak(w, &p, i, w.g(p.liquid, i)).1;
    let foot = super::footing(w, i, genes[super::GAS] as f64, w.cfg.phys.root_least, &soak, None);
    let area = cell * foot.share;
    if area <= 0.0 {
        return false;
    }
    let s = sown(&w.chem, &w.cfg.phys, &p, (i, area, w.t_ground_last(i)), genes, root, kg);
    let mol = s.mol;
    let heat = s.heat;
    let mut recruits = [0.0; super::SLOTS];
    let c = &mut w.life.cells[i].cover;
    let was = c.temp(&w.chem, &p);
    let landed = lodge(c, &mm, &s, &w.cfg, (cell, foot.share), &mut recruits);
    let to = lines(w, &w.life.cells[i].cover, i, &soak, (&recruits, built_kg(&mm, &mol)), area);
    let c = &mut w.life.cells[i].cover;
    c.line = to;
    match landed {
        Landed::Refused => return false,
        Landed::In => {}
        // The slot in deficit it took goes to the ground as remains, with its heat.
        Landed::Out(m) => {
            w.counts.cover_budget[DIED] -= held(&p.content(&w.chem), &m);
            let q = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * was).min(c.e);
            c.e -= q;
            let (from, to) = (Node::new(Class::Life, i), Node::ground(i));
            let mut moves: Vec<_> = (0..NP).map(|x| Move { from, to, k: p.sub[x] as u16, mol: m[x], heat: 0.0 }).collect();
            moves.push(Move { from, to, k: 0, mol: 0.0, heat: q });
            apply(w, &[], &moves);
        }
    }
    w.counts.cover_budget[PLACED] += held(&p.content(&w.chem), &mol);
    // (The cover's side is its own: the store books the outside node's.)
    w.cross((crate::store::Boundary::Outside, Node::new(Class::Life, i)), (0..NP).map(|q| (p.sub[q], mol[q])), heat);
    w.life.cells[i].cover.mark = w.light[i];
    true
}

/// The plants of the slot of starting kind `root` in cell `i` become `groups`, each its plants (a real count) and its
/// built cover, kg, as the laws order and fuse them (`Cover::set_groups`): a stand of several sizes set by hand, as the
/// test card sets one. Its amounts stay. False where the cell holds no slot of that kind.
pub fn set_plants(w: &mut World, i: usize, root: u32, groups: &[(f64, f64)]) -> bool {
    let c = &mut w.life.cells[i].cover;
    let Some(x) = c.held().find(|&x| c.root[x] == root) else { return false };
    let mut g: Vec<super::groups::Group> = groups.iter().map(|&(n, kg)| super::groups::Group { n, kg }).collect();
    c.set_groups(x, &mut g);
    true
}

/// `kg` per m² of cover of genes `genes` and starting kind `root` as seeds for cell `i` of `area` m² at `t` K: its
/// built cover at the share of walls a plant of its mass builds (the plants it founds, `super::stand::founded` and
/// `super::stand::walled`), its fuel at its `STORE` gene, with its heat.
pub fn sown(chem: &Chemistry, ph: &crate::Physics, p: &Parts, (i, area, t): (usize, f64, f64), genes: [f32; NG], root: u32, kg: f64) -> Seed {
    let mm = p.mm(chem);
    let built = kg * area / (1.0 + dec(genes[STORE]));
    let walls = dec(genes[WALLS]) * super::stand::walled(ph, built / super::stand::founded(ph, built, area));
    let fm = 1.0 / (1.0 + walls);
    let mol = [built * dec(genes[STORE]) / mm[FUEL], built * fm / mm[MATTER], built * fm * walls / mm[WALL]];
    let heat = (0..NP).map(|q| mol[q] * chem.subs[p.sub[q]].cp).sum::<f64>() * t;
    Seed { to: i as u32, mol, genes, root, heat }
}

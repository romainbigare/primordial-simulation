//! The bodies of standing liquid as Flow's state holds them (laws.md 1.2): `find` names, at each full build of the
//! ground's state, the cells of the bodies (`bodies::Bodies`) and those under their body's level; `share_out` brings
//! the cells of a body to one level, as the runoff's pass does once a month (`runoff`); and each body mixes its heat
//! under each air block (`mix`).

use super::{air::blocks, bodies::Body, curve::{self, first_held, holds_all, Part}, level::level_over, land::{cells_state, standing, NONE}, parts::TOP, potential::apply, Scratch};
use crate::{space::network::NONE as NO_BODY, store::{Class, LinkFlux}, World, AIR};
use std::collections::BTreeMap;

/// The bodies now: whether each cell holds part of a body, those cells in order, whether each lies under its body's
/// level (its mean surface under it: a cell under the body, not one of its shore), the body of each cell's largest part
/// (`bodies::Bodies::of`), and the level of each body, m (as what its parts held at the last pass stands over their
/// samples, no higher than its pass).
#[derive(Clone, Debug, Default)]
pub(super) struct Basin {
    pub(super) body: Vec<bool>,
    pub(super) cells: Vec<u32>,
    pub(super) is: Vec<bool>,
    pub(super) of: Vec<u32>,
    pub(super) level: Vec<f64>,
}

/// The bodies now (`Basin`): the level of each the one level at which what its parts held at the last pass
/// (`State::held`) stands over their samples together, no higher than its pass. A world whose parts held nothing yet
/// gives each part its share of what stands over its cell's pores now (`curve::first_held`).
pub(super) fn find(w: &World, s: &mut Scratch) {
    if !holds_all(&s.curves, &s.state.held) {
        s.state.held = first_held(s);
    }
    let (n, b) = (w.mesh.cells().n(), &s.bodies);
    let level: Vec<f64> = b.list.iter().map(|x| body_level(s, x)).collect();
    let is = (0..n).map(|i| b.of[i] != NO_BODY && s.z[i] < level[b.of[i] as usize]).collect();
    let cells = (0..n as u32).filter(|&i| b.of[i as usize] != NO_BODY).collect();
    s.basin = Basin { body: b.of.iter().map(|&x| x != NO_BODY).collect(), cells, is, of: b.of.clone(), level };
}

impl World {
    /// The level of the liquid of each pond of the terrain's samples (`Samples::ponds`), m, as Flow's last runoff's pass
    /// or full build of the ground's state left it; NaN before Flow first ran.
    pub fn pond_levels(&self) -> Vec<f64> {
        let mut out = vec![f64::NAN; self.terrain_samples().ponds.len()];
        let s = &self.scratch;
        s.bodies.list.iter().zip(&s.basin.level).for_each(|(b, &l)| out[b.pond as usize] = l);
        out
    }

    /// What stands over the pores of each cell at the world's start, m³ (laws.md 5): each of its parts of a body of
    /// standing liquid with a pass full to the pass; a part of the open low ground, and all the ground of a cell that
    /// holds no pond's samples, up to the level `low`, m. Each part keeps what it holds as the runoff's pass leaves a
    /// part (`State::held`), so every body stands at that level from the first step.
    pub fn full_bodies(&mut self, low: f64) -> Vec<f64> {
        let mut s = Scratch::default();
        curve::build(self, &mut s);
        let samples = self.terrain_samples();
        let level = |p: &Part| samples.ponds.get(p.pond as usize).filter(|d| d.pass != NO_BODY).map_or(low, |d| d.level);
        let (mut stands, mut held) = (Vec::with_capacity(s.curves.len()), Vec::with_capacity(s.curves.len()));
        for (i, c) in s.curves.iter().enumerate() {
            let z = self.surface_z(i);
            let parts: Vec<f64> = c.parts.iter().map(|p| p.volume(level(p) - z)).collect();
            stands.push(parts.iter().sum());
            held.push(if c.ponds().is_empty() { Vec::new() } else { parts });
        }
        self.scratch.state.held = held;
        stands
    }

    /// Does cell `i` hold part of a body of standing liquid (`bodies::Bodies`): any sample of a pond?
    pub fn in_body(&self, i: usize) -> bool {
        let s = self.terrain_samples();
        self.map.leaves()[i].split(crate::space::terrain::SAMPLE_DEPTH.max(self.map.leaves()[i].depth)).any(|f| s.pond[s.index(&f)] != NO_BODY)
    }

    /// Each body of standing liquid as Flow's state holds it (`Pool`); none before Flow first built the state.
    pub fn pools(&self) -> Vec<Pool> {
        let s = &self.scratch;
        if s.curves.len() != self.mesh.cells().n() || s.head.len() != s.curves.len() || s.bodies.of.len() != s.curves.len() {
            return Vec::new();
        }
        let pool = |(b, &level): (&Body, &f64)| {
            let parts = b.cells.iter().zip(&b.part).map(|(&i, &q)| (i as usize, &s.curves[i as usize].parts[q]));
            let stands: Vec<f64> = b.cells.iter().zip(&b.part).map(|(&i, &q)| s.fill(i as usize).shares(standing(s, i as usize))[q]).collect();
            let under = parts.clone().map(|(i, p)| p.volume(level - s.z[i])).collect();
            let area = parts.zip(&stands).map(|((_, p), &v)| p.under(p.level(v))).sum();
            let next = (b.next.body != NO_BODY).then_some(b.next.body as usize);
            Pool { pond: b.pond as usize, cells: b.cells.clone(), volume: stands.iter().sum(), stands, under, next, level, low: b.pass == NO_BODY, area }
        };
        s.bodies.list.iter().zip(&s.basin.level).map(pool).collect()
    }
}

/// A body of standing liquid (laws.md 1.2): its pond (`Samples::ponds`), its cells, the volume of liquid its part in
/// each holds over the cell's pores as Flow's state holds it (`curve::Fill::shares`) and the volume its part's samples
/// hold under the body's level, m³; the body its pass leads to (an index of `World::pools`; none for open low ground or
/// where the way ends in no body), its level, m; whether it is open low ground (no pass); the area its liquid covers
/// over its parts' samples, m², and the volume its parts hold, m³.
#[derive(Clone, Debug)]
pub struct Pool {
    pub pond: usize,
    pub cells: Vec<u32>,
    pub stands: Vec<f64>,
    pub under: Vec<f64>,
    pub next: Option<usize>,
    pub level: f64,
    pub low: bool,
    pub area: f64,
    pub volume: f64,
}

/// The level of body `b`: what its parts held at the last pass (`State::held`) at one level over their samples
/// (`level::level_over`), m, no higher than its pass.
fn body_level(s: &Scratch, b: &Body) -> f64 {
    let parts: Vec<(&Part, f64)> = b.cells.iter().zip(&b.part).map(|(&i, &q)| (&s.curves[i as usize].parts[q], s.z[i as usize])).collect();
    let held: f64 = b.cells.iter().zip(&b.part).map(|(&i, &q)| s.state.held[i as usize][q]).sum();
    level_over(&parts, held).min(b.level)
}

/// What may move of the liquids `liq` in each cell of `cells` (`[c * (liq.len() + 1) + q]`): all of one that boils below
/// the coldest node, `tmin` (a gas dissolved in the liquid, which takes no room), else its liquid share `wet(k, i)`
/// (`Sp::liquid`: of a mineral what stays dissolved), mol, then their heat as they leave,
/// `heat(i, mol)`, J; and their volume, m³.
pub(super) fn movable(w: &World, s: &Scratch, liq: &[usize], cells: &[u32], tmin: f64, (wet, heat): (impl Fn(usize, usize) -> f64, impl Fn(usize, &[f64]) -> f64)) -> (Vec<f64>, Vec<f64>) {
    let (n, nl, m) = (w.mesh.cells().n(), liq.len(), cells.len());
    let width = nl + 1;
    let (mut held, mut vol) = (vec![0.0; m * width], vec![0.0; m]);
    let gas: Vec<bool> = liq.iter().map(|&k| s.sp[k].boil <= tmin).collect();
    for (c, &i) in cells.iter().enumerate() {
        let i = i as usize;
        let row = &mut held[c * width..(c + 1) * width];
        for (q, &k) in liq.iter().enumerate() {
            row[q] = w.store.amounts(Class::Ground)[k * n + i] * if gas[q] { 1.0 } else { wet(k, i) };
            vol[c] += if gas[q] { 0.0 } else { row[q] * s.sp[k].v[1] };
        }
        row[nl] = heat(i, &row[..nl]);
    }
    (held, vol)
}

/// The liquid standing in ground node `i` gains per mol of liquid `k` that comes to it with the liquid (less: leaves),
/// m³: its liquid's molar volume; of a mineral only its share that stays dissolved (`State::dissolved`), as the node's
/// exchange sites take the rest of what comes (give the rest of what leaves) at once, its state splitting what it holds
/// so (`Sp::volume`; of a cell of parts its liquid's top's, where what comes gathers, `parts::to_liquid`); none of a
/// gas, which takes no room.
pub(super) fn gain(s: &Scratch, k: usize, i: usize) -> f64 {
    let free = s.mixed(i).map_or(s.state.dissolved[i], |m| m.piece[super::parts::TOP].free);
    s.sp[k].v[1] * s.sp[k].liquid(s.state.tg[i], 1.0, free)
}

/// Cells `cells`, of the liquids that may move `held` (`movable`: rows of `width - 1` liquids, then the heat) and
/// their volumes `vol`, m³, come to hold `want` m³: each cell that holds more gives the share of its liquid over it,
/// with that share of its heat; mixed, it goes to the cells that hold less, by what each lacks. Where they hold more
/// than they want together (`over`), what the cells that hold less do not take is left over, mixed. A row's volume to a
/// cell is what it gains by it, `gains(c, row)`, m³ (`gain`). Returns the moves from the cell that wants the most to each
/// other one (each other's change, a row of `held`, `[c * width + q]`, + to it) with that cell, and what is left over
/// (a row).
pub(super) fn share_out(cells: &[u32], (held, vol): (&[f64], &[f64]), want: &[f64], (width, over): (usize, bool), gains: impl Fn(usize, &[f64]) -> f64) -> (usize, Vec<f64>, Vec<f64>) {
    let m = cells.len();
    // What the cells over the level give, mixed (as a row of `held`), its volume, and the volume the cells under it lack, m³.
    let (mut spare, mut given, mut lack) = (vec![0.0; width], 0.0, 0.0);
    let mut change = vec![0.0; m * width];
    for c in 0..m {
        if vol[c] > want[c] {
            let row = &held[c * width..(c + 1) * width - 1];
            let gives = ((vol[c] - want[c]) / gains(c, row).max(f64::MIN_POSITIVE)).min(1.0);
            for (q, x) in spare.iter_mut().enumerate() {
                change[c * width + q] = -held[c * width + q] * gives;
                *x -= change[c * width + q];
            }
            given += vol[c] - want[c];
        } else {
            lack += want[c] - vol[c];
        }
    }
    // Shared by what each lacks: all that was given where the cells want what they hold, else as much as each lacks by
    // what the mix gives it (no more than all of it).
    let takes: Vec<f64> = (0..m).map(|c| if want[c] > vol[c] { (want[c] - vol[c]) / if over { gains(c, &spare[..width - 1]) } else { lack } } else { 0.0 }).collect();
    let all: f64 = takes.iter().sum();
    let whole = if over { all.max(1.0) } else { 1.0 };
    let mut left = spare.clone();
    for c in (0..m).filter(|&c| takes[c] > 0.0 && takes[c].is_finite() && given > 0.0) {
        let gets = takes[c] / whole;
        for q in 0..width {
            change[c * width + q] = spare[q] * gets;
            left[q] -= change[c * width + q];
        }
    }
    if !over {
        left.fill(0.0);
    }
    let hub = (0..m).max_by(|&a, &b| want[a].total_cmp(&want[b]).then(b.cmp(&a))).unwrap_or(0);
    (hub, change, left)
}

/// Does cell `i` mix as one with its body under its air block (`mix_heat`) and face the air as one surface with the
/// body under its air node (`air::surf`)? A cell under its body's level all under its liquid (a node that is one, or
/// of parts with no land, its liquid's top, `parts`), off its melting plateaus, with no soft cover and no cover of
/// bodies.
pub(super) fn one_surface(w: &World, s: &Scratch, i: usize) -> bool {
    let (pk, cover) = match s.mixed(i) {
        Some(m) if m.two_faces() => return false,
        Some(m) => (m.piece[TOP].pk, m.piece[TOP].cover[2]),
        None => (s.state.pk[i], s.cover[i][2]),
    };
    s.basin.is[i] && pk == NONE && cover <= 0.0 && crate::contract::stand(w, i).catch(w.mesh.cells().area()[i]) <= 0.0
}

/// Does the liquid of cell `i` of a body mix as one with the body's (`mix_heat`, `air::dissolve`)? A cell that faces
/// the air as one surface with it (`one_surface`); of a cell of parts, its liquid's top, off its plateaus, with no soft
/// cover and no cover of bodies.
pub(super) fn mixes(w: &World, s: &Scratch, i: usize) -> bool {
    s.mixed(i).map(|m| &m.piece[TOP]).map_or(one_surface(w, s, i), |p| s.basin.body[i] && p.pk == NONE && p.cover[2] <= 0.0 && crate::contract::stand(w, i).catch(w.mesh.cells().area()[i]) <= 0.0)
}

/// What is molten in the liquid of cell `i` that mixes (`mixes`), one bit per substance: of a cell of parts its
/// liquid's top's.
pub(super) fn phases(s: &Scratch, i: usize) -> u64 {
    s.mixed(i).map_or(s.latent[i].molten, |m| m.piece[TOP].latent.molten)
}

/// The cells of the bodies, which the runoff's pass moved, find their state again; then each body mixes its heat under
/// each air block (`mix_heat`), and the cells that mixed find their state again.
pub(super) fn mix(w: &mut World, s: &mut Scratch) {
    let cells = std::mem::take(&mut s.basin.cells);
    cells_state(w, s, &cells, false);
    s.basin.cells = cells;
    let mixed = mix_heat(w, s);
    cells_state(w, s, &mixed, false);
}

/// Each body under each air block mixes its heat in a month: the liquid of its cells that mixes as one (`mixes`), with
/// the same phases, takes the block's temperature, the heat of their sum over their summed heat capacity (apart from
/// the fusion enthalpy each holds), as one heat flux from the cell of the most heat capacity to each other, a cell of
/// parts' its liquid's top's (`parts::Parts`). Returns the cells that mixed.
fn mix_heat(w: &mut World, s: &mut Scratch) -> Vec<u32> {
    let mut block = vec![0; w.mesh.level(AIR).n()];
    blocks(w).iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let parent = w.mesh.cells().parent();
    let liquid = |i: usize| s.mixed(i).map(|m| &m.piece[TOP]);
    let mut by: BTreeMap<(u32, usize, u64), Vec<usize>> = BTreeMap::new();
    for &i in s.basin.cells.iter().filter(|&&i| mixes(w, s, i as usize)) {
        let i = i as usize;
        by.entry((s.basin.of[i], block[parent[i] as usize], phases(s, i))).or_default().push(i);
    }
    let free = |i: usize| liquid(i).map_or(w.store.heat(Class::Ground)[i] - s.latent[i].held, |p| p.heat - p.latent.held);
    let cap = |i: usize| liquid(i).map_or(s.cap[i], |p| p.cap);
    let (mut a, mut b, mut heat, mut mixed, mut parts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for cells in by.values().filter(|c| c.len() > 1) {
        mixed.extend(cells.iter().map(|&i| i as u32));
        let t = cells.iter().map(|&i| free(i)).sum::<f64>() / cells.iter().map(|&i| cap(i)).sum::<f64>();
        let hub = cells.iter().copied().max_by(|&p, &q| cap(p).total_cmp(&cap(q)).then(q.cmp(&p))).unwrap_or(cells[0]);
        for &i in cells.iter().filter(|&&i| i != hub) {
            a.push(hub as u32);
            b.push(i as u32);
            heat.push(cap(i) * t - free(i));
        }
        parts.extend(cells.iter().filter(|&&i| liquid(i).is_some()).map(|&i| (i, cap(i) * t - free(i))));
    }
    apply(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[]);
    for (i, q) in parts {
        if let Some(Some(p)) = s.state.parts.get_mut(i) {
            p.heat[TOP] += q;
        }
    }
    mixed.sort_unstable();
    mixed
}

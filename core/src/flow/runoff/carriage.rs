//! The key samples the load is followed at (`runoff::load`, laws.md 1.3), what passes each, and the links between cells
//! the load's moves take: found once with the bodies (`Bodies::of`).

use super::Bodies;
use crate::soil::{flood_shear, passes, threshold_slope};
use crate::space::{delta::FLOOD_RUNOFF, network::NONE, samples::Samples};
use crate::World;
use std::collections::HashMap;

/// The particle classes of the load: fine, middle, coarse (`soil::CLASSES`).
pub(super) const CLASS_COUNT: usize = crate::soil::CLASSES.len();
/// A sample that lets each class pass to within this share is no key sample: what settles there is taken at the next
/// key sample down its way, by the product of the passes of the samples on the way (`Carriage::skip`).
const PASS_TOLERANCE: f64 = 1e-4;

/// Of each particle class, the share of a load that passes a sample's reach, the run to the sample its liquid runs to,
/// and the share of what is picked up along that reach that does (`soil::passes`; f32: shares hold no matter).
pub(super) type Shares = [[f32; CLASS_COUNT]; 2];

/// A cell with a key sample its samples' ways reach first (`Carriage::pairs`): the cell, the key sample's place, and
/// the link the load takes from the cell to the key sample's (`NONE` where they are one).
#[derive(Clone, Copy, Debug)]
pub(super) struct Pair {
    pub(super) cell: u32,
    pub(super) key: u32,
    pub(super) link: u32,
}

/// The key samples of the terrain's ways (`Carriage::of`), those whose liquid runs on in the order of `Bodies::down`,
/// then the ends; of each sample its place among them (`NONE` for none). Of each key sample: its next key sample down
/// its way (`NONE` for an end); its shares (none pass an end); of each class the product of the passes of the samples
/// between it and the next (`skip`); and the link to the next's cell (`NONE` where it is its own). Of each land sample
/// its pair (`NONE` for none), and whether it is a key sample; of each key sample its cell and where its way ends
/// (`End`); the pairs, in the order of their key samples; the links
/// between cells, from and to, in the order they first come (the pairs', then the key samples'); of each cell its
/// main channel's key sample (`NONE` for none); and of each key sample whose way runs on its room (`Carriage::room`).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Carriage {
    pub(super) keys: Vec<u32>,
    pub(super) cell: Vec<u32>,
    pub(super) end: Vec<End>,
    pub(super) place: Vec<u32>,
    pub(super) next: Vec<u32>,
    pub(super) shares: Vec<Shares>,
    pub(super) skip: Vec<[f32; CLASS_COUNT]>,
    pub(super) link: Vec<u32>,
    pub(super) pair_of: Vec<u32>,
    pub(super) pairs: Vec<Pair>,
    pub(super) links: Vec<(u32, u32)>,
    pub(super) main: Vec<u32>,
    pub(super) room: Vec<f64>,
}

/// Where a key sample's way ends: on it, in the open low ground (`Open`), in another body (`Body`) or on land (`Land`);
/// or it runs on (`On`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum End {
    On,
    Open,
    Body,
    Land,
}

impl Carriage {
    /// The carriage of the bodies `b` of world `w` (`Carriage`): a sample whose liquid runs on is a key where some class
    /// passes its reach by less than 1 − `PASS_TOLERANCE` (`shares`), or where it is its cell's main channel, its land
    /// sample the most drains through (`Samples::up`); a way ends where it enters a body, or at a land sample whose
    /// liquid runs to none. A land sample's pair is its cell with the first key sample at it or below it; every land
    /// sample has one (`pair_of[k] != NONE`), as every way ends at a key sample: the pick-up reads it so. A key sample
    /// whose way runs on has a room: the volume that lifts it to the threshold slope over the sample its liquid runs to
    /// (`soil::threshold_slope`, Parker 1978), no higher than the lowest sample whose liquid runs to it (a deposit dams
    /// no way that feeds it), m³, as these samples stand; the load fills it (`runoff::load`).
    pub(in crate::flow) fn of(w: &World, b: &Bodies) -> Carriage {
        let s = w.terrain_samples();
        let mut main = vec![NONE; b.of.len()];
        for &k in &b.land {
            let m = &mut main[b.cell_of[k as usize] as usize];
            if *m == NONE || s.up[k as usize] > s.up[*m as usize] {
                *m = k;
            }
        }
        let mut c = Carriage { place: vec![NONE; s.h.len()], ..Carriage::default() };
        let mut grade = Vec::new();
        let mut key = |c: &mut Carriage, k: u32, (shares, slope): (Shares, f64)| {
            if c.place[k as usize] == NONE {
                c.place[k as usize] = c.keys.len() as u32;
                c.keys.push(k);
                c.shares.push(shares);
                grade.push(slope);
            }
        };
        let of_down = crate::par::map_n(b.down.len(), crate::par::UNCOUNTED, |q| shares(w, &s, b.down[q] as usize));
        for (&k, &(sh, slope)) in b.down.iter().zip(&of_down) {
            let settles = sh[0].iter().any(|&p| f64::from(p) < 1.0 - PASS_TOLERANCE);
            if settles || main[b.cell_of[k as usize] as usize] == k {
                key(&mut c, k, (sh, slope));
            }
        }
        for &k in &b.land {
            let j = s.to[k as usize];
            if j == NONE {
                key(&mut c, k, ([[0.0; CLASS_COUNT]; 2], 0.0));
            } else if b.at[j as usize] != NONE {
                key(&mut c, j, ([[0.0; CLASS_COUNT]; 2], 0.0));
            }
        }
        let of_down: Vec<Shares> = of_down.into_iter().map(|x| x.0).collect();
        let (mut first, skip) = c.below(b, &s, &of_down);
        // The key samples in an order that keeps each way's key samples together, each before the next.
        let order = upstream_first(&c.next_of(b, &s, &first));
        let mut new = vec![0u32; order.len()];
        order.iter().enumerate().for_each(|(q, &old)| new[old as usize] = q as u32);
        c.keys = order.iter().map(|&q| c.keys[q as usize]).collect();
        c.shares = order.iter().map(|&q| c.shares[q as usize]).collect();
        c.keys.iter().enumerate().for_each(|(q, &k)| c.place[k as usize] = q as u32);
        first.iter_mut().filter(|f| **f != NONE).for_each(|f| *f = new[*f as usize]);
        c.next = c.next_of(b, &s, &first);
        let at_body = |k: u32| b.at[k as usize] != NONE;
        c.skip = c.keys.iter().map(|&k| if at_body(k) { [1.0; CLASS_COUNT] } else { skip[k as usize] }).collect();
        c.pair(b, &first);
        c.cell = c.keys.iter().map(|&k| b.cell_of[k as usize]).collect();
        c.end = c.keys.iter().zip(&c.next).map(|(&k, &n)| match b.at[k as usize] {
            NONE if n == NONE => End::Land,
            NONE => End::On,
            body if b.list[body as usize].pass == NONE => End::Open,
            _ => End::Body,
        }).collect();
        c.main = main.iter().map(|&k| if k == NONE { NONE } else { c.place[k as usize] }).collect();
        let area = w.mesh.cells().area();
        // Of each sample, the lowest of the samples whose liquid runs to it (infinite for none).
        let mut feeds = vec![f64::INFINITY; s.h.len()];
        b.down.iter().for_each(|&k| feeds[s.to[k as usize] as usize] = feeds[s.to[k as usize] as usize].min(s.h[k as usize]));
        c.room = order.iter().zip(&c.keys).zip(&c.end).map(|((&old, &k), &end)| {
            let (k, slope) = (k as usize, grade[old as usize]);
            let top = if end == End::On { (s.h[s.to[k] as usize] + slope * s.run[k]).min(feeds[k]) } else { s.h[k] };
            (top - s.h[k]).max(0.0) * b.share[k] * area[b.cell_of[k] as usize]
        }).collect();
        c
    }

    /// Of each key sample, the next key sample down its way (`NONE` where its way ends): `first` of the sample its
    /// liquid runs to.
    fn next_of(&self, b: &Bodies, s: &Samples, first: &[u32]) -> Vec<u32> {
        let end = |k: usize| b.at[k] != NONE || s.to[k] == NONE;
        self.keys.iter().map(|&k| if end(k as usize) { NONE } else { first[s.to[k as usize] as usize] }).collect()
    }

    /// Of each sample, the first key sample at it or below it, and of each class the product of the passes of the
    /// samples after it down to that key sample, found from the ends up (the reverse of `Bodies::down`; `of_down` the
    /// shares of the samples in that order).
    fn below(&self, b: &Bodies, s: &Samples, of_down: &[Shares]) -> (Vec<u32>, Vec<[f32; CLASS_COUNT]>) {
        let mut first = self.place.clone();
        let mut skip = vec![[1.0f32; CLASS_COUNT]; s.h.len()];
        let mut pass = vec![[1.0f32; CLASS_COUNT]; s.h.len()];
        for (q, &k) in b.down.iter().enumerate().rev() {
            let (k, j) = (k as usize, s.to[k as usize] as usize);
            pass[k] = of_down[q][0];
            if self.place[j] == NONE {
                skip[k] = std::array::from_fn(|l| pass[j][l] * skip[j][l]);
            }
            if first[k] == NONE {
                first[k] = first[j];
            }
        }
        (first, skip)
    }

    /// The pairs of the land samples (`Pair`, each sample's cell with `first` of it), in the order of their key samples,
    /// and the links between cells: the pairs', then those of the key samples to their next.
    fn pair(&mut self, b: &Bodies, first: &[u32]) {
        let mut index: HashMap<(u32, u32), u32> = HashMap::new();
        let mut links = Vec::new();
        let mut link = |from: u32, to: u32| {
            if from == to {
                return NONE;
            }
            *index.entry((from, to)).or_insert_with(|| {
                links.push((from, to));
                (links.len() - 1) as u32
            })
        };
        let mut pairs: HashMap<(u32, u32), u32> = HashMap::new();
        self.pair_of = vec![NONE; first.len()];
        for &k in &b.land {
            let key = first[k as usize];
            if key != NONE {
                let cell = b.cell_of[k as usize];
                let to = b.cell_of[self.keys[key as usize] as usize];
                let list = &mut self.pairs;
                self.pair_of[k as usize] = *pairs.entry((cell, key)).or_insert_with(|| {
                    list.push(Pair { cell, key, link: link(cell, to) });
                    (list.len() - 1) as u32
                });
            }
        }
        // The pairs in the order of their key samples, so the pass gathers into the pools in their order.
        let mut order: Vec<usize> = (0..self.pairs.len()).collect();
        order.sort_by_key(|&p| self.pairs[p].key);
        let mut new = vec![0u32; order.len()];
        order.iter().enumerate().for_each(|(at, &p)| new[p] = at as u32);
        self.pairs = order.iter().map(|&p| self.pairs[p]).collect();
        self.pair_of.iter_mut().filter(|p| **p != NONE).for_each(|p| *p = new[*p as usize]);
        let cell = |q: u32| b.cell_of[self.keys[q as usize] as usize];
        let to_next = |(q, &n): (u32, &u32)| if n == NONE { NONE } else { link(cell(q), cell(n)) };
        self.link = (0..self.keys.len() as u32).zip(&self.next).map(to_next).collect();
        self.links = links;
    }
}

/// An order of the key samples whose next key samples are `next` (`NONE` for an end): each after all those whose ways
/// lead to it, and each way's key samples together (the ends in their order, each after the tree of ways above it,
/// each tree's branches in their order, depth first): the pass then reads the pools it writes close together.
fn upstream_first(next: &[u32]) -> Vec<u32> {
    let m = next.len();
    let mut start = vec![0u32; m + 1];
    next.iter().filter(|&&n| n != NONE).for_each(|&n| start[n as usize + 1] += 1);
    (0..m).for_each(|q| start[q + 1] += start[q]);
    let mut fill = start.clone();
    let mut above = vec![0u32; start[m] as usize];
    for (q, &n) in next.iter().enumerate().filter(|(_, &n)| n != NONE) {
        above[fill[n as usize] as usize] = q as u32;
        fill[n as usize] += 1;
    }
    let mut order = Vec::with_capacity(m);
    for end in (0..m as u32).filter(|&q| next[q as usize] == NONE) {
        // Depth first: each key sample with how many of its branches are done.
        let mut stack = vec![(end, 0u32)];
        while let Some(&mut (q, ref mut done)) = stack.last_mut() {
            let branches = &above[start[q as usize] as usize..start[q as usize + 1] as usize];
            if (*done as usize) < branches.len() {
                let up = branches[*done as usize];
                *done += 1;
                stack.push((up, 0));
            } else {
                order.push(q);
                stack.pop();
            }
        }
    }
    order
}

/// The shares of sample `k` of samples `s` in world `w` (`Shares`, `soil::passes`): under the flood (`FLOOD_RUNOFF`
/// from the area that drains through it) in its channel (`Physics::channel`; `soil::flood_shear`), over the run to the
/// sample its liquid runs to, of particles of its bed's make; and the threshold slope of that flood
/// (`soil::threshold_slope`).
fn shares(w: &World, s: &Samples, k: usize) -> (Shares, f64) {
    let g = w.cfg.gravity;
    let flood = FLOOD_RUNOFF * s.up[k];
    let channel = w.cfg.phys.channel(flood);
    let buoyant = w.terrain.makes.get(s.make[k] as usize).copied().unwrap_or_default().buoyant;
    let shares = passes(flood_shear(s, k, g, channel.depth), flood / channel.width, s.run[k], buoyant, g);
    (shares.map(|r| r.map(|x| x as f32)), threshold_slope(channel.depth, buoyant, g))
}

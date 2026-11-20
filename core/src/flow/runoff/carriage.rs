//! The key samples the load is followed at (`runoff::load`, laws.md 1.3), and what passes each: found once with the
//! bodies (`Bodies::of`).

use super::Bodies;
use crate::soil::{flood_shear, passes};
use crate::space::{delta::FLOOD_RUNOFF, network::NONE, samples::Samples};
use crate::World;
use std::collections::HashMap;

/// The particle classes of the load: fine, middle, coarse (`soil::CLASSES`).
pub(super) const CLASS_COUNT: usize = crate::soil::CLASSES.len();
/// A sample that lets each class pass to within this share is no key sample: what would settle there is left to the
/// key samples below.
const PASS_TOLERANCE: f64 = 1e-4;

/// Of each particle class, the share of a load that passes a key sample and the share of what was picked up on the ways
/// to it that does (`soil::passes`; f32: shares hold no matter).
pub(super) type Shares = [[f32; CLASS_COUNT]; 2];

/// The key samples of the terrain's ways (`Carriage::of`): the key samples, those whose liquid runs on in the order of
/// `Bodies::down`, then the ends; of each sample its place among them (`NONE` for none); of each its next key sample
/// down its way (`NONE` for an end) and its shares (none pass an end); of each land sample its pair (`NONE` for none);
/// the pairs, each a cell with the first key sample below one of its samples, in the order of the land samples; of each
/// cell its main channel's key sample (`NONE` for none).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Carriage {
    pub(super) keys: Vec<u32>,
    place: Vec<u32>,
    pub(super) next: Vec<u32>,
    pub(super) shares: Vec<Shares>,
    pub(super) pair_of: Vec<u32>,
    pub(super) pairs: Vec<(u32, u32)>,
    pub(super) main: Vec<u32>,
}

impl Carriage {
    /// The carriage of the bodies `b` of world `w` (`Carriage`): a sample whose liquid runs on is a key where some class
    /// settles (`shares`), or where it is its cell's main channel, its land sample the most drains through
    /// (`Samples::up`); a way ends where it enters a body, or at a land sample whose liquid runs to none.
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
        let key = |c: &mut Carriage, k: u32, shares: Shares| {
            if c.place[k as usize] == NONE {
                c.place[k as usize] = c.keys.len() as u32;
                c.keys.push(k);
                c.shares.push(shares);
            }
        };
        let of_down = crate::par::map_n(b.down.len(), crate::par::UNCOUNTED, |q| shares(w, &s, b.down[q] as usize));
        for (&k, sh) in b.down.iter().zip(of_down) {
            let settles = sh[0].iter().any(|&p| f64::from(p) < 1.0 - PASS_TOLERANCE);
            if settles || main[b.cell_of[k as usize] as usize] == k {
                key(&mut c, k, if settles { sh } else { [[1.0; CLASS_COUNT]; 2] });
            }
        }
        for &k in &b.land {
            let j = s.to[k as usize];
            if j == NONE {
                key(&mut c, k, [[0.0; CLASS_COUNT]; 2]);
            } else if b.at[j as usize] != NONE {
                key(&mut c, j, [[0.0; CLASS_COUNT]; 2]);
            }
        }
        // Of each sample, the first key sample at it or below it: found from the ends up.
        let mut first = c.place.clone();
        for &k in b.down.iter().rev() {
            if first[k as usize] == NONE {
                first[k as usize] = first[s.to[k as usize] as usize];
            }
        }
        let below = |k: u32| if s.to[k as usize] == NONE { c.place[k as usize] } else { first[s.to[k as usize] as usize] };
        let next = c.keys.iter().map(|&k| if b.at[k as usize] != NONE || s.to[k as usize] == NONE { NONE } else { below(k) }).collect();
        let mut index: HashMap<(u32, u32), u32> = HashMap::new();
        let (mut pairs, mut pair_of) = (Vec::new(), vec![NONE; s.h.len()]);
        for &k in &b.land {
            let key = below(k);
            if key != NONE {
                let pair = (b.cell_of[k as usize], key);
                pair_of[k as usize] = *index.entry(pair).or_insert_with(|| {
                    pairs.push(pair);
                    (pairs.len() - 1) as u32
                });
            }
        }
        c.main = main.iter().map(|&k| if k == NONE { NONE } else { c.place[k as usize] }).collect();
        (c.next, c.pair_of, c.pairs) = (next, pair_of, pairs);
        c
    }
}

/// The shares of sample `k` of samples `s` in world `w` (`Shares`, `soil::passes`): under the flood (`FLOOD_RUNOFF`
/// from the area that drains through it) in its channel (`Physics::channel`; `soil::flood_shear`), over the run to the
/// sample its liquid runs to, of particles of its bed's make.
fn shares(w: &World, s: &Samples, k: usize) -> Shares {
    let g = w.cfg.gravity;
    let flood = FLOOD_RUNOFF * s.up[k];
    let channel = w.cfg.phys.channel(flood);
    let buoyant = w.terrain.makes.get(s.make[k] as usize).copied().unwrap_or_default().buoyant;
    passes(flood_shear(s, k, g, channel.depth), flood / channel.width, s.run[k], buoyant, g).map(|r| r.map(|x| x as f32))
}

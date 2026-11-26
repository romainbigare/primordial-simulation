//! A basin of liquid made by an edit (handover.md 11.1: add liquid; the ground dug and the key liquid added): the
//! ground under a brush cut to a smooth bowl, a depth under the level the liquid is to stand at in its middle, its
//! banks meeting the ground about it with no step; that level the lowest dry ground of its rim, or the level of the
//! liquid of the ponds its cut opens, so a pond it reaches grows over it at its own level; a bank of `edit_bank_m` over that level only where its rim would let
//! the liquid out, but at its outlet, at the level; and the key liquid poured in to its pass, all from the outside
//! node. So the basin is closed by its making: a pond of the samples, whose body keeps what is poured into it, and
//! never one that lowers a pond it reaches.

use super::brush::Covered;
use super::Brush;
use crate::space::network::NONE;
use crate::space::samples::Samples;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use std::collections::{BTreeMap, BTreeSet};

/// The ring of samples round a brush its rim is found in, samples wide: wide enough that every sample under the brush
/// has its neighbours across sides and corners under it or on the ring.
const RIM_SAMPLES: f64 = 1.5;

/// Where a basin's bowl meets its level, in radii of its brush: its shore on even ground.
const SHORE: f64 = 0.85;

/// Where a basin's banks begin, in radii of its brush: from there to its edge its cut lessens to none.
const BANKS_FROM: f64 = 0.5;

/// How many times at most a basin pours again what its cells' other liquid and pores took of its liquid as it settled.
const TOP_UPS: usize = 3;

/// A basin is full once what it still lacks to its pass is under this share of its room over its floor.
const TOP_UP_SHARE: f64 = 0.01;

impl World {
    /// A basin of liquid `depth` m deep under the brush `b` (the module's doc). Its rim is the samples beside those it
    /// covers (within `RIM_SAMPLES` round it, as built over the cells' grounds now). Its level is the highest level of
    /// the liquid of the closed ponds that stands on its samples or its rim (`pond_levels`, where it lies over their
    /// ground), else the way out of its outlet: of the samples of its rim on which no liquid stands, the one the higher
    /// of whose height and the lowest sample beside it outside the rim is lowest, that height. Every sample it
    /// covers is cut to a bowl `depth` under that level at its centre whose banks meet the level at `SHORE` of the
    /// brush's radius and the ground about it with no step (`bowl`), none filled; every dry sample of the rim whose way out lies under the level (the liquid would leave by it)
    /// is raised to `edit_bank_m` over it, but the outlet, moved to the level (so a pond it grows leaves by a pass at
    /// its own level, wherever its own pass lay); the ground cut or raised with each cell's own rock
    /// (`move_ground`), and the samples built again over it (`flow::build::reshape`). The key liquid fills the pond
    /// under the brush's centre to its level, its pass, from the outside node at each cell's ground temperature, poured
    /// into each cell's part of it (`pour_pond`, one pour for all its cells); the liquid settled with no time
    /// (`flow::settle`) levels it (the pores of the ground it covers were given what they take first), and the passes
    /// of a month's start find the world's state from it once it is full. Whether a
    /// basin was made: none whose centre lies in the open low ground (its liquid stands there already). Where none is
    /// made but the samples were built again to read the ground, the liquid settles over them as `World::settle` would
    /// have (which finds nothing due then).
    pub(crate) fn basin(&mut self, b: Brush, depth: f64) -> bool {
        let Some(liquid) = crate::cover::Parts::of(&self.chem).map(|p| p.liquid) else { return false };
        if depth <= 0.0 {
            return false;
        }
        // (The ground is read on the samples as they stand over the cells' grounds: built again first where edits
        // shaped them, or where they were never built from the running world, the seed's.)
        let reshaped = self.relief.due || self.relief.version == 0;
        if reshaped {
            crate::flow::build::reshape(self);
        }
        let made = self.dig(b, depth, liquid);
        if reshaped && !made {
            crate::month_start(self);
            crate::flow::settle(self);
        }
        made
    }

    /// The basin dug under the brush `b`, `depth` m deep, and filled with the liquid `liquid`, over the samples as they
    /// stand (`basin`); whether it was made.
    fn dig(&mut self, b: Brush, depth: f64, liquid: usize) -> bool {
        let s = self.terrain_samples();
        let wide = Brush { centre: b.centre, radius_m: b.radius_m + RIM_SAMPLES * self.map.length(SAMPLE_DEPTH) };
        let all = self.covered(wide, s.across);
        let inside = |c: &Covered| c.d * wide.radius_m <= b.radius_m;
        let Some(centre) = all.iter().filter(|c| inside(c)).min_by(|a, z| a.d.total_cmp(&z.d)).map(|c| c.k) else { return false };
        if open(&s, centre) {
            return false;
        }
        let levels = self.pond_levels();
        // The level of the liquid standing on a sample: its pond's, where that lies over its ground (the open low
        // ground's at any level); NaN where none stands.
        let wet = |k: usize| {
            let p = s.pond[k];
            let l = if p == NONE { f64::NAN } else if s.ponds[p as usize].pass == NONE { f64::INFINITY } else { levels[p as usize] };
            if l > s.h[k] { l } else { f64::NAN }
        };
        // Its rim: the samples beside one it covers (across a side or a corner), every way out of it. The ponds it
        // opens: those whose liquid stands on its rim (or under it); a pond whose liquid stands on no sample of its rim
        // stays closed: a way out of it leaves by a sample beside its liquid, which the basin only raises.
        let cut: BTreeSet<usize> = all.iter().filter(|c| inside(c)).map(|c| c.k).collect();
        let around = |k: usize| {
            let (x, y, across, down) = ((k % s.across) as i64, (k / s.across) as i64, s.across as i64, s.down as i64);
            (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy))).filter(move |&(i, j)| (i, j) != (x, y) && i >= 0 && i < across && j >= 0 && j < down).map(move |(i, j)| (j * across + i) as usize)
        };
        let on_rim = |k: usize| !cut.contains(&k) && around(k).any(|j| cut.contains(&j));
        let reached = all.iter().filter(|c| cut.contains(&c.k) || on_rim(c.k)).map(|c| wet(c.k)).filter(|l| l.is_finite()).fold(f64::NEG_INFINITY, f64::max);
        // The outlet: the dry sample of the rim whose way out is lowest, the higher of its own height and the lowest
        // sample beside it outside the rim.
        let way = |k: usize| around(k).filter(|&j| !cut.contains(&j) && !on_rim(j)).map(|j| s.h[j]).fold(f64::INFINITY, f64::min).max(s.h[k]);
        let outlet = all.iter().filter(|c| on_rim(c.k) && wet(c.k).is_nan()).map(|c| (c.k, way(c.k))).min_by(|a, z| a.1.total_cmp(&z.1));
        let level = if reached.is_finite() { reached } else if let Some((_, l)) = outlet { l } else { return false };
        let outlet = outlet.map(|o| o.0);
        let bank = level + self.cfg.edit_bank_m;
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        for c in &all {
            let h = s.h[c.k];
            let to = if inside(c) {
                bowl(h, level, depth, c.d * wide.radius_m / b.radius_m)
            } else if !on_rim(c.k) || !wet(c.k).is_nan() {
                h
            } else if Some(c.k) == outlet {
                level
            } else if way(c.k) < level {
                h.max(bank)
            } else {
                h
            };
            if to != h {
                cells.entry(c.cell).or_default().push((c.k, to - h));
            }
        }
        drop(s);
        self.move_ground(&cells, 0.0, &[]);
        crate::flow::build::reshape(self);
        // What fills it: the pond under its centre, to its level (its pass, the lowest the liquid leaves by); none
        // where its floor opened to the open low ground, whose liquid stands there already.
        let s = self.terrain_samples();
        let pond = s.pond[centre];
        let filled = pond != NONE && !open(&s, centre);
        drop(s);
        // What a cell's other liquid and its ground's pores took of it as it settled is poured again, `TOP_UPS`
        // times at most. (A pour finds the state of the cells it fills: the passes of a month's start find the rest
        // from the basin once it settled.)
        let mut from = f64::NEG_INFINITY;
        for _ in 0..=TOP_UPS {
            let more = filled && self.pour_pond(pond, liquid, from);
            crate::flow::settle(self);
            if !more {
                break;
            }
            from = self.pond_levels()[pond as usize];
        }
        crate::month_start(self);
        true
    }

    /// The key liquid `liquid` poured into each cell's part of pond `pond`, in one pour (`flow::pour`): over each of its
    /// samples to the pond's level from its ground or from the level `from` where higher, and what the pores of the
    /// ground that covers lack to be full, which they take first (`flow::pores_lack`, under the pond's samples in the
    /// cell: so it stands that deep over a dry floor too), m³, from the outside node at each cell's ground temperature,
    /// liquid (`put_liquid`). Whether it poured more than `TOP_UP_SHARE` of the pond's room over its floor.
    fn pour_pond(&mut self, pond: u32, liquid: usize, from: f64) -> bool {
        let s = self.terrain_samples();
        let level = s.ponds[pond as usize].level;
        let (mut by, mut room): (BTreeMap<usize, f64>, f64) = (BTreeMap::new(), 0.0);
        for (i, l) in self.map.leaves().iter().enumerate() {
            let members: Vec<usize> = l.split(SAMPLE_DEPTH.max(l.depth)).map(|f| s.index(&f)).filter(|&k| s.pond[k] == pond).collect();
            room += members.iter().map(|&k| (level - s.h[k]).max(0.0) * s.area(k)).sum::<f64>();
            let stands: f64 = members.iter().map(|&k| (level - s.h[k].max(from)).max(0.0) * s.area(k)).sum();
            let covers: f64 = members.iter().filter(|&&k| s.h[k].max(from) < level).map(|&k| s.area(k)).sum();
            let v = stands + crate::flow::pores_lack(self, i, covers);
            if v > 0.0 {
                by.insert(i, v);
            }
        }
        drop(s);
        let sub = &self.chem.subs[liquid];
        let per_m3 = sub.density[1] / sub.molar_mass;
        let cells: Vec<(usize, f64)> = by.into_iter().collect();
        crate::flow::pour(self, pond, liquid, &cells, |w, i, v| w.put_liquid(i, liquid, v * per_m3));
        cells.iter().map(|c| c.1).sum::<f64>() > TOP_UP_SHARE * room
    }
}

/// The height a basin cuts ground `h` m high to at `d` radii from its centre, its liquid to stand at `level` m and
/// `depth` m deep at its centre: toward its bowl, `depth` × (1 − (d/`SHORE`)²) under the level, a paraboloid that
/// meets the level at `SHORE`; across its banks, from `BANKS_FROM` to the brush's edge, the cut lessens by the brush's
/// `falloff` to none, so where the ground about it lies over the level its banks rise to it over that band with no
/// step. None filled.
fn bowl(h: f64, level: f64, depth: f64, d: f64) -> f64 {
    let floor = level - depth * (1.0 - (d / SHORE).powi(2));
    h - (h - floor).max(0.0) * super::brush::falloff(((d - BANKS_FROM) / (1.0 - BANKS_FROM)).max(0.0))
}

/// Whether sample `k` of `s` lies in the open low ground (a pond of no pass).
fn open(s: &Samples, k: usize) -> bool {
    s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE
}

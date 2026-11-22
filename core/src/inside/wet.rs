//! Where a cell's land is saturated (TOPMODEL, Beven and Kirkby 1979; Beven 1997): its liquid table, over the bed's
//! store or in its loose ground (`Means::deficit`), departs over its land's samples by the wetness index (`soil_level`,
//! `deficit_at`) and reaches the surface of those where the most gathers. It moves no liquid. A law reads the share of its land saturated and the liquid of the drier rest (`wet_ground`).

use super::{deficit_at, soil_level, Means};
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use std::sync::Arc;

/// The land of a cell as its soil's liquid lies over it (`wet_ground`): the share of its land saturated; the wetness
/// index at and over which a sample is saturated (infinite where none is, minus infinite where all are); and the share
/// of its land's pores its key liquid fills on the mean (past 1 where liquid stands over them), at the same moment; and
/// the level of the wetness index its liquid table departs about (`soil_level`; 0 where it lies evenly), which the
/// shells draw the table by.
#[derive(Clone, Copy, Debug)]
pub struct WetGround {
    pub share: f64,
    pub level: f64,
    pub filled: f64,
    pub soil: f64,
}

/// The land of cell `i` of means `m` (`Means`, over its land): its samples (`World::terrain_samples`) but the lowest,
/// which the liquid that stands over the cell covers, by its land's share `land` of its area (the liquid stands over
/// the lowest first, `flow::curve`), where its liquid table lies (`saturated`), its parts in the order of their wetness
/// index (`Sorted`).
pub(crate) fn wet_ground(w: &World, i: usize, m: Means, land: f64) -> WetGround {
    let (s, (of, sorted)) = (w.terrain_samples(), w.life.sorted.as_ref().expect("the cells' samples sorted (`sort_samples`)"));
    assert!(Arc::ptr_eq(of, &s), "the cells' samples are sorted for the samples now (`sort_samples`)");
    let c = &sorted[i];
    let count = (land * c.high.len() as f64).round() as usize;
    let ks = c.wettest.iter().filter(|&&q| (q as usize) < count).map(|&q| c.high[q as usize] as usize);
    let (twi, area) = parts_now(w, &s, i, (ks, count));
    saturated(&w.cfg.phys, (&twi, &area), m)
}

/// The samples of a cell (`World::terrain_samples`) sorted as its wet ground reads them (`wet_ground`): from the
/// highest down (its land is the highest of them), and their places in that order from the lowest wetness index up
/// (`Samples::twi`, as `soil_level` reads its parts); each order keeps the cell's order of equal values.
#[derive(Clone, Debug)]
pub struct Sorted {
    high: Vec<u32>,
    wettest: Vec<u32>,
}

/// Each cell's samples sorted (`Sorted`), kept with the samples they are of (`Life::sorted`): sorted again only where
/// those are not the samples now (a build of the terrain, a cell's ground made level, a world loaded), as the orders
/// change with nothing else.
pub(crate) fn sort_samples(w: &mut World) {
    let s = w.terrain_samples();
    if w.life.sorted.as_ref().is_some_and(|(of, _)| Arc::ptr_eq(of, &s)) {
        return;
    }
    let leaves = w.map.leaves();
    let sorted = crate::par::map_n(leaves.len(), crate::par::UNCOUNTED, |i| {
        let mut high: Vec<u32> = leaves[i].split(SAMPLE_DEPTH).map(|f| s.index(&f) as u32).collect();
        high.sort_by(|&a, &b| s.h[b as usize].total_cmp(&s.h[a as usize]));
        let mut wettest: Vec<u32> = (0..high.len() as u32).collect();
        wettest.sort_by(|&a, &b| s.twi[high[a as usize] as usize].total_cmp(&s.twi[high[b as usize] as usize]));
        Sorted { high, wettest }
    });
    w.life.sorted = Some((s, sorted));
}

/// The terrain samples `ks` of cell `i` as the table reads them now, each of equal area, `count` of them: of each, the
/// part its channel's flowing liquid covers (`channel_share`) at an infinite wetness index, its room none, and the rest
/// at its own index (`Samples::twi`). Returns the indices and the shares of the area, summing to 1.
fn parts_now(w: &World, s: &crate::space::samples::Samples, i: usize, (ks, count): (impl Iterator<Item = usize>, usize)) -> (Vec<f64>, Vec<f64>) {
    let each = 1.0 / count.max(1) as f64;
    let (mut twi, mut area) = (Vec::with_capacity(2 * count), Vec::with_capacity(2 * count));
    let per_m3 = carried(w, i);
    for k in ks {
        let wet = share_of(w, s, per_m3, k);
        if wet > 0.0 {
            twi.push(f64::INFINITY);
            area.push(each * wet);
        }
        twi.push(s.twi[k]);
        area.push(each * (1.0 - wet));
    }
    (twi, area)
}

/// Ground of parts of wetness index `twi` and shares of the area `area` (summing to 1), of means `m`: its liquid table
/// departs over them by the index (`soil_level`, `deficit_at`), and a part is saturated where it reaches the surface, at
/// an index of at least the level plus the mean room over `soil_m`; its pores' mean fill and the level it departs about.
pub fn saturated(ph: &crate::Physics, (twi, area): (&[f64], &[f64]), m: Means) -> WetGround {
    let filled = if m.pores > 0.0 { m.pore / m.pores } else { 1.0 };
    let [d, most] = m.deficit(ph);
    let by = |wet: &dyn Fn(f64) -> bool| twi.iter().zip(area).filter(|p| wet(*p.0)).map(|p| p.1).sum::<f64>();
    let channels = by(&|t| t == f64::INFINITY);
    let at = soil_level(ph, twi, area, m);
    let (share, level) = match at {
        _ if d <= 0.0 || filled >= 1.0 => (1.0, f64::NEG_INFINITY),
        None if d >= most || twi.iter().all(|&t| t == f64::INFINITY) => (channels, f64::INFINITY),
        None => (1.0, f64::NEG_INFINITY),
        Some(c) => (by(&|t| deficit_at(ph, m, t, c) <= 0.0), c + d / ph.soil_m),
    };
    WetGround { share, level, filled, soil: at.unwrap_or(0.0) }
}

/// The share of the pores of the drier rest of a land that its liquid fills, its share `share` saturated and its pores
/// the share `filled` full on the mean: its saturated share's pores are full, so the rest hold what they do not.
pub fn rest(share: f64, filled: f64) -> f64 {
    if share < 1.0 { ((filled - share) / (1.0 - share)).clamp(0.0, 1.0) } else { 1.0 }
}

/// The share of a land whose pores hold no gas, its share `share` saturated (`wet_ground`) and its pores `soil` the
/// share `filled` full on the mean: its saturated share, and of the drier rest (`rest`) the share its liquid fills from
/// the bottom up (`Pores::full`). Every law that asks how much of the ground is waterlogged reads this: roots there get
/// the oxidising gas only down their gas spaces (`cover::working`), and the decomposers none (`change`, laws.md 2.3).
pub fn airless(soil: &crate::soil::Pores, share: f64, filled: f64) -> f64 {
    share + (1.0 - share) * soil.full(rest(share, filled))
}

/// The share of the ground of cell `i` where its decomposers get the oxidising gas (laws.md 2.3): all of it under
/// standing liquid (`standing` over 0, its depth or its film's time: the liquid's film brings their gas), else the share
/// whose pores hold gas (1 − `airless`, its pores the share `filled` full, its saturated share `saturated_share`).
pub fn aired(w: &World, i: usize, standing: f64, filled: f64) -> f64 {
    if standing > 0.0 { 1.0 } else { 1.0 - airless(w.pores(i), saturated_share(w, i), filled) }
}

/// The share of the land of cell `i` saturated at the start of life's month (`contract::wet_grounds`, `wet_land`); none
/// where it holds no land or life has no key liquid.
pub fn saturated_share(w: &World, i: usize) -> f64 {
    w.life.wet.get(i).copied().flatten().map_or(0.0, |g| g.share)
}

/// The share of the pores of the land of cell `i` its key liquid fills, as its wet ground was found at the start of
/// life's month (`WetGround::filled`); None where it was not (no land, or life has no key liquid).
pub fn land_filled(w: &World, i: usize) -> Option<f64> {
    w.life.wet.get(i).copied().flatten().map(|g| g.filled)
}

/// Where the liquid table of the land of cell `i`, the share `land` of its area, reaches its surface (`wet_ground`),
/// its key liquid (that of `p`) as the cover's roots meet it now (`cover::fill_now`): its pores as Flow has them
/// (`Scratch::land_pores`) over the bed's store under the cell (`Class::Deep`).
pub(crate) fn wet_land(w: &World, p: &crate::cover::Parts, i: usize, land: f64) -> WetGround {
    let filled = crate::cover::fill_now(w, p, i, w.g(p.liquid, i).max(0.0))[1];
    let pores = w.scratch.land_pores(i).max(1e-9);
    let m = Means { pore: filled.min(1.0) * pores, pores, stands: 0.0, store: crate::flow::store_depth(w, i), field: w.pores(i).field };
    WetGround { filled, ..wet_ground(w, i, m, land) }
}

/// Whether terrain sample `k` of `s` (`World::terrain_samples`) drains a channel's source: the square of the spacing of
/// the channels inside a cell (2 `hill` a side, the hillslopes on both sides of a channel; Horton 1945). A function
/// of the place alone, by which the page draws the channels' wet strips.
pub fn drains_source(ph: &crate::Physics, s: &crate::space::samples::Samples, k: usize) -> bool {
    s.up[k] >= (2.0 * ph.hill).powi(2)
}

/// Whether liquid flows in the channel of terrain sample `k` of `s`: it drains a channel's source (`drains_source`) and
/// its liquid runs (its discharge over a year, `World::discharge`). Flowing liquid is its table at the surface: it has
/// no room (`channel_share`).
pub fn flowing(w: &World, s: &crate::space::samples::Samples, k: usize) -> bool {
    drains_source(&w.cfg.phys, s, k) && w.discharge.get(k).is_some_and(|&q| q > 0.0)
}

/// The share of the area of terrain sample `k` of `s`, in cell `i`, that the liquid flowing in its channel covers
/// (`flowing`): its channel's width (`World::channel`, the channel law on its discharge) over the sample's side, at
/// most all of it; none where none flows. Flowing liquid is its table at the surface (`parts_now`).
pub fn channel_share(w: &World, s: &crate::space::samples::Samples, i: usize, k: usize) -> f64 {
    share_of(w, s, carried(w, i), k)
}

/// The mol per m³ of the liquid the channels of cell `i` carry (`World::liquid_in`, as `World::channel` reads it);
/// none where none is liquid.
fn carried(w: &World, i: usize) -> Option<f64> {
    w.liquid_in(i, w.stock(crate::store::Node::ground(i))).map(|l| l.1)
}

/// `channel_share` of terrain sample `k` of `s`, its cell's liquid of `per_m3` mol per m³ (`carried`): its channel's
/// width (`Physics::channel`, on its discharge as a volume) over the sample's side.
fn share_of(w: &World, s: &crate::space::samples::Samples, per_m3: Option<f64>, k: usize) -> f64 {
    if !flowing(w, s, k) {
        return 0.0;
    }
    let channel = per_m3.map_or_else(crate::flow::Channel::default, |p| w.cfg.phys.channel(w.discharge[k] as f64 / p));
    (channel.width / w.map.length(SAMPLE_DEPTH)).min(1.0)
}

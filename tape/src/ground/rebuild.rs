//! What changed of the ground when a world built its samples again (`Ground::rebuild`): which samples a tile reads
//! differently, and which squares' tiles hold them.

use super::bends::REACH_SAMPLES;
use super::channels::STEP_MOST;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;

/// The tiles about a changed sample that change with it, in samples each way: a texel names the channel it keeps by its
/// step from its own sample, up to `STEP_MOST` (`channels`), which also holds the spline of the shore drawn smooth and
/// the margin the heights finer than the samples read (`space::below`).
const TILES_MARGIN: i64 = STEP_MOST;

/// What a tile reads of a sample: its height, way, main way upstream, the area that drains through it, its pond and
/// that pond's level, its wetness index as the tiles hold it and whether its line carries it (`wetness`), and the point
/// its line runs through.
pub(super) struct Read<'a> {
    pub s: &'a Samples,
    pub hill: &'a [f64],
    pub strips: &'a [bool],
    pub at: &'a [[f64; 2]],
}

/// Of each sample, whether a tile reads it differently in `new` than in `old` (the same grid), to the last bit.
pub(super) fn changed(old: &Read, new: &Read) -> Vec<bool> {
    let level = |s: &Samples, k: usize| if s.pond[k] == NONE { f64::NAN } else { s.ponds[s.pond[k] as usize].level };
    let (o, n) = (old.s, new.s);
    (0..n.h.len())
        .map(|k| {
            o.h[k] != n.h[k]
                || o.to[k] != n.to[k]
                || o.main[k] != n.main[k]
                || o.up[k] != n.up[k]
                || o.pond[k] != n.pond[k]
                || level(o, k).to_bits() != level(n, k).to_bits()
                || o.twi[k] != n.twi[k]
                || old.hill[k] != new.hill[k]
                || old.strips[k] != new.strips[k]
                || old.at[k] != new.at[k]
        })
        .collect()
}

/// The samples round a sample that its piece of channel reads (`bends::Bends::update`): its reach, `REACH_SAMPLES` steps
/// up its main way and down its own, and a sample across them (`Reaches::rise_across`), each step a sample at most.
pub(super) const BENDS_READ: i64 = REACH_SAMPLES as i64 + 1;

/// `marked` with every sample of `s` within `by` samples of a marked one, row and column, marked too.
pub(super) fn near(s: &Samples, marked: &[bool], by: i64) -> Vec<bool> {
    let mut out = marked.to_vec();
    let (across, down) = (s.across as i64, s.down as i64);
    for k in (0..marked.len()).filter(|&k| marked[k]) {
        let (x, y) = ((k % s.across) as i64, (k / s.across) as i64);
        for j in (y - by).max(0)..=(y + by).min(down - 1) {
            for i in (x - by).max(0)..=(x + by).min(across - 1) {
                out[(j * across + i) as usize] = true;
            }
        }
    }
    out
}

/// `marked` with what reads a marked sample of `s` marked too: the samples `REACH_SAMPLES` up its main way and down its
/// own (the fall of a reach, `bends::Reaches::fall`), and every sample down the way from those (a line's pieces start
/// where the one above ended).
pub(super) fn readers(s: &Samples, marked: &[bool]) -> Vec<bool> {
    let mut out = marked.to_vec();
    for k in (0..marked.len()).filter(|&k| marked[k]) {
        let (mut up, mut down) = (k, k);
        for _ in 0..REACH_SAMPLES {
            if s.main[up] != NONE {
                up = s.main[up] as usize;
                out[up] = true;
            }
            if s.to[down] != NONE {
                down = s.to[down] as usize;
                out[down] = true;
            }
        }
    }
    // In the flood's order from the last sample reached: a sample comes before the one its liquid runs to.
    for &k in s.order.iter().rev() {
        let (k, to) = (k as usize, s.to[k as usize]);
        if out[k] && to != NONE {
            out[to as usize] = true;
        }
    }
    out
}

/// The squares of `map` whose tiles hold a sample of `s` marked in `marked`, or one within `TILES_MARGIN` of it, row
/// by row.
pub(super) fn squares(map: &Square, s: &Samples, marked: &[bool]) -> Vec<bool> {
    let per = (s.across / map.w).max(1) as i64;
    let mut out = vec![false; map.w * map.h];
    for k in (0..marked.len()).filter(|&k| marked[k]) {
        let (x, y) = ((k % s.across) as i64, (k / s.across) as i64);
        let lo = |v: i64| ((v - TILES_MARGIN).max(0) / per) as usize;
        let hi = |v: i64, n: usize| (((v + TILES_MARGIN) / per) as usize).min(n - 1);
        for j in lo(y)..=hi(y, map.h) {
            for i in lo(x)..=hi(x, map.w) {
                out[j * map.w + i] = true;
            }
        }
    }
    out
}

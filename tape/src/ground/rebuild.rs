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

/// The samples round a sample that its texture reads (`pcore::soil::texture_at`): its main sample upstream and the
/// samples each runs to, a step each.
pub(super) const GRAIN_READ: i64 = 2;

/// Of each sample, whether one of the numbers its texture reads differs in `new` from `old` (its height, its ways,
/// what drains through it, its run, its pond, its make and its loose ground), to the last bit.
pub(super) fn moved(old: &Samples, new: &Samples) -> Vec<bool> {
    (0..new.h.len())
        .map(|k| {
            old.h[k].to_bits() != new.h[k].to_bits()
                || old.to[k] != new.to[k]
                || old.main[k] != new.main[k]
                || old.up[k].to_bits() != new.up[k].to_bits()
                || old.run[k].to_bits() != new.run[k].to_bits()
                || old.pond[k] != new.pond[k]
                || old.make[k] != new.make[k]
                || old.loose[k].to_bits() != new.loose[k].to_bits()
                || old.settled[k].to_bits() != new.settled[k].to_bits()
                || old.age[k].to_bits() != new.age[k].to_bits()
        })
        .collect()
}

/// `marked` with every sample of `s` within `by` samples of a marked one, row and column, marked too: spread along
/// the rows, then down the columns, each by a count of the marked ones in a window that slides along it.
pub(super) fn near(s: &Samples, marked: &[bool], by: i64) -> Vec<bool> {
    let (across, down) = (s.across, s.down);
    let rows = spread(marked, across, (down, across, 1), by);
    spread(&rows, 1, (across, down, across), by)
}

/// Of `marked`, along each of `lines` lines (the first of line `l` at `l * line_step`) of `len` places `step` apart,
/// each place marked where one within `by` places of it is.
fn spread(marked: &[bool], line_step: usize, (lines, len, step): (usize, usize, usize), by: i64) -> Vec<bool> {
    let mut out = vec![false; marked.len()];
    let by = by.max(0) as usize;
    for l in 0..lines {
        let at = |i: usize| l * line_step + i * step;
        // The marked ones in the window [i - by, i + by] of place i.
        let mut count = (0..by.min(len)).filter(|&i| marked[at(i)]).count();
        for i in 0..len {
            if i + by < len && marked[at(i + by)] {
                count += 1;
            }
            if i > by && marked[at(i - by - 1)] {
                count -= 1;
            }
            out[at(i)] = count > 0;
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

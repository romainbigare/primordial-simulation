//! What changed of the ground when a world built its samples again (`Ground::rebuild`): which samples, which ways down
//! from them, and which squares' tiles hold them.

use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;

/// A sample's height changed where it moved by more than this, m: a build raises each cell's samples by its offset, a
/// few millimetres to centimetres a decade where no load lies, which shows on no tile (a shore on ground of slope 0.01
/// moves a metre).
const MOVED_M: f64 = 0.01;
/// The area that drains through a sample changed where it moved by more than this share of itself: a channel's width
/// goes as its root.
const DRAINS_SHARE: f64 = 0.01;
/// A sample's line point changed where it moved by more than this, in samples.
const MOVED_POINT: f64 = 1e-6;
/// The tiles about a changed sample that change with it, in samples each way: the spline of the shore drawn smooth
/// reads two samples each way and the nearest pond within two more (`shore`), a valley about a channel reaches 250 m
/// (`valley::VALLEY_M`, two samples), and a texel's spline two texels.
const TILES_MARGIN: i64 = 4;

/// Of each sample of `new` (the samples as built again over `old`'s grid), whether it changed: its height, its way,
/// its main way upstream, the area that drains through it, its pond or that pond's level, its loose ground, or the point
/// its line runs through (`at`: the old points and the new).
pub(super) fn changed(old: &Samples, new: &Samples, (at_old, at_new): (&[[f64; 2]], &[[f64; 2]])) -> Vec<bool> {
    let level = |s: &Samples, k: usize| if s.pond[k] == NONE { f64::NAN } else { s.ponds[s.pond[k] as usize].level };
    let same_level = |a: f64, b: f64| (a.is_nan() && b.is_nan()) || a == b;
    (0..new.h.len())
        .map(|k| {
            (new.h[k] - old.h[k]).abs() > MOVED_M
                || new.to[k] != old.to[k]
                || new.main[k] != old.main[k]
                || (new.up[k] - old.up[k]).abs() > DRAINS_SHARE * old.up[k]
                || (new.pond[k] == NONE) != (old.pond[k] == NONE)
                || !same_level(level(new, k), level(old, k))
                || new.loose[k] != old.loose[k]
                || (at_new[k][0] - at_old[k][0]).abs().max((at_new[k][1] - at_old[k][1]).abs()) > MOVED_POINT
        })
        .collect()
}

/// `marked` with the samples next to each marked one marked too (a piece of a line reads its neighbours).
pub(super) fn ring(s: &Samples, marked: &[bool]) -> Vec<bool> {
    let mut out = marked.to_vec();
    for k in (0..marked.len()).filter(|&k| marked[k]) {
        let (x, y) = ((k % s.across) as i64, (k / s.across) as i64);
        for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let (i, j) = (x + dx, y + dy);
            if i >= 0 && j >= 0 && (i as usize) < s.across && (j as usize) < s.down {
                out[j as usize * s.across + i as usize] = true;
            }
        }
    }
    out
}

/// `marked` with every sample down the way of a marked one marked too: a line's pieces start where the one above
/// ended, so a change runs down the channel to its end.
pub(super) fn downstream(s: &Samples, marked: &[bool]) -> Vec<bool> {
    let mut out = marked.to_vec();
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

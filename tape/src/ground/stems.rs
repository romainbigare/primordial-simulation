//! Where a channel's line runs between the terrain's samples: a channel runs from sample to sample along their sides
//! and corners, so down a valley across the grid it steps like a stair. Its line runs through each sample's point here
//! instead: the mean of the centres of the samples along its stem, `SMOOTH_SAMPLES` up and down, weighted the more the
//! nearer (a moving average along the polyline of the centres, as corner cutting smooths a polygon: Chaikin 1974), as
//! many each way as the stem has (a stem runs on through a body's samples, along its floor), so a head, a stem's end
//! and the sample a tributary joins keep their place, and a tributary ends on the smoothed point of the stem it joins. A point moves at
//! most `SMOOTH_MOST` from its sample's centre. The point of a sample that liquid runs into and none out of (a line's
//! end in standing liquid) lies where the shore drawn puts that liquid (`shore::Shore::mouth`), so a river reaches
//! its lake or its sea. A function of the samples alone.

use super::shore::Shore;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The samples up and down a stem a point is the mean of.
const SMOOTH_SAMPLES: usize = 6;
/// The farthest a point moves from its sample's centre, samples.
pub(super) const SMOOTH_MOST: f64 = 1.0;

/// The centre of sample `k` of `s`, in samples.
fn centre(s: &Samples, k: usize) -> [f64; 2] {
    [(k % s.across) as f64 + 0.5, (k / s.across) as f64 + 0.5]
}

/// The point of each sample of `s`, whose shore is drawn as `shore`, its line runs through, samples (the module's doc),
/// and before a line's end is moved to its shore (`raw`). Down the stem a sample's way goes on to the sample it runs to,
/// but not past the sample a tributary joins: that one counts at its own point. In the order the flood reached them, so
/// a joined sample's point is found first.
pub(super) fn points(s: &Samples, shore: &Shore) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let mut raw: Vec<[f64; 2]> = (0..s.h.len()).map(|k| centre(s, k)).collect();
    for &k in &s.order {
        raw[k as usize] = point(s, k as usize, &raw);
    }
    let at = (0..s.h.len()).map(|k| ended(s, shore, k, &raw)).collect();
    (at, raw)
}

/// The same found again on the samples `s` (other than those they were found on) in place, `at` and `raw` as `points`
/// made them: a sample's point read again only where a sample within `SMOOTH_SAMPLES` of it runs or is joined otherwise
/// (`ways`), or the point its stem ends on moved; a line's end moved to the shore again where its point moved or the
/// shore may have (`shore_moved`, within the reach of its walk, `Shore::mouth`). So it is what `points` makes, to the
/// last bit.
pub(super) fn update(s: &Samples, shore: &Shore, (at, raw): (&mut [[f64; 2]], &mut [[f64; 2]]), (ways, shore_moved): (&[bool], &[bool])) {
    let near = super::rebuild::near(s, ways, SMOOTH_SAMPLES as i64);
    let mut moved = vec![false; s.h.len()];
    for &k in &s.order {
        let k = k as usize;
        if near[k] || joins(s, k).is_some_and(|j| moved[j]) {
            let p = point(s, k, raw);
            moved[k] = p[0].to_bits() != raw[k][0].to_bits() || p[1].to_bits() != raw[k][1].to_bits();
            raw[k] = p;
        }
    }
    let shore_near = super::rebuild::near(s, shore_moved, MOUTH_SAMPLES);
    for k in (0..s.h.len()).filter(|&k| moved[k] || near[k] || shore_near[k]) {
        at[k] = ended(s, shore, k, raw);
    }
}

/// The samples about a line's end whose shore its walk to the liquid reads (`Shore::mouth`): its steps and the spline's
/// reach.
const MOUTH_SAMPLES: i64 = 8;

/// The sample whose point the stem of sample `k` of `s` ends on down its way (`points`): the one a tributary joins.
fn joins(s: &Samples, k: usize) -> Option<usize> {
    let (mut q, mut steps) = (k, 0);
    while steps < SMOOTH_SAMPLES && s.to[q] != NONE {
        let j = s.to[q] as usize;
        if s.main[j] != q as u32 {
            return Some(j);
        }
        (q, steps) = (j, steps + 1);
    }
    None
}

/// The point of sample `k` of `s`, the points `raw` of those its stem joins found (`points`), before a line's end moves.
fn point(s: &Samples, k: usize, raw: &[[f64; 2]]) -> [f64; 2] {
    let mut up = Vec::with_capacity(SMOOTH_SAMPLES);
    let mut q = k;
    while up.len() < SMOOTH_SAMPLES && s.main[q] != NONE {
        q = s.main[q] as usize;
        up.push(centre(s, q));
    }
    let mut down = Vec::with_capacity(SMOOTH_SAMPLES);
    let mut q = k;
    while down.len() < SMOOTH_SAMPLES && s.to[q] != NONE {
        let j = s.to[q] as usize;
        if s.main[j] != q as u32 {
            down.push(raw[j]);
            break;
        }
        down.push(centre(s, j));
        q = j;
    }
    let n = up.len().min(down.len());
    let own = centre(s, k);
    let (mut sum, mut weight) = ([own[0] * (n + 1) as f64, own[1] * (n + 1) as f64], (n + 1) as f64);
    for (i, p) in up.iter().take(n).chain(down.iter().take(n)).enumerate() {
        let w = (n - i % n.max(1)) as f64;
        (sum, weight) = ([sum[0] + w * p[0], sum[1] + w * p[1]], weight + w);
    }
    let mean = [sum[0] / weight, sum[1] / weight];
    let (dx, dy) = (mean[0] - own[0], mean[1] - own[1]);
    let shift = dx.hypot(dy);
    let keep = if shift > SMOOTH_MOST { SMOOTH_MOST / shift } else { 1.0 };
    [own[0] + dx * keep, own[1] + dy * keep]
}

/// The point of sample `k` of `s` its line runs through, its point `raw` found: a line's end (liquid runs into it and
/// none out) where the shore `shore` puts its liquid.
fn ended(s: &Samples, shore: &Shore, k: usize, raw: &[[f64; 2]]) -> [f64; 2] {
    if s.to[k] == NONE && s.main[k] != NONE { shore.mouth(raw[k]) } else { raw[k] }
}

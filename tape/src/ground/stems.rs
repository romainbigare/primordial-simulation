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

/// The point of each sample of `s`, whose shore is drawn as `shore`, its line runs through, samples (the module's doc). Down the stem a sample's way goes
/// on to the sample it runs to, but not past the sample a tributary joins: that one counts at its own point. In the order the flood reached them, so a joined sample's point is found first.
pub(super) fn points(s: &Samples, shore: &Shore) -> Vec<[f64; 2]> {
    let mut at: Vec<[f64; 2]> = (0..s.h.len()).map(|k| centre(s, k)).collect();
    for &k in &s.order {
        let k = k as usize;
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
                down.push(at[j]);
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
        at[k] = [own[0] + dx * keep, own[1] + dy * keep];
    }
    for k in (0..s.h.len()).filter(|&k| s.to[k] == NONE && s.main[k] != NONE) {
        at[k] = shore.mouth(at[k]);
    }
    at
}

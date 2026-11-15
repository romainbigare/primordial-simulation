//! One air node's places for its rain (laws.md 1.5): the places that see it, the parts of its air over each that
//! saturate one after another as it moistens (`saturated`), what each would hold over its own saturation, and where
//! what the node rains lands (`Seen::landing`).

/// The parts of a node's air over a place (`parts`), each an equal share of it.
pub(super) const MOIST_PARTS: usize = 4;

/// The share of a body of air that is saturated, the cloud's share of its sky, where its mean amount is the ratio `r`
/// of what saturates it and its moistest air saturates at the ratio `r0` (Sundqvist et al. 1989): none below `r0`, all
/// at 1, 1 − √((1 − r) / (1 − r0)) between. It is the share past saturation of a spread of moisture as even as a
/// uniform one whose saturated part holds no more than saturates it (Tompkins 2005).
pub(super) fn saturated(r: f64, r0: f64) -> f64 {
    if r <= r0 { 0.0 } else if r >= 1.0 { 1.0 } else { 1.0 - ((1.0 - r) / (1.0 - r0)).sqrt() }
}

/// The ratios to a place's saturation at which each of the `MOIST_PARTS` parts of the node's air over it saturates,
/// moistest first: `saturated` turned around at the middle of each part's share, 1 − (1 − `r0`)(1 − (k + ½) / parts)².
pub(super) fn parts(r0: f64) -> [f64; MOIST_PARTS] {
    std::array::from_fn(|k| 1.0 - (1.0 - r0) * (1.0 - (k as f64 + 0.5) / MOIST_PARTS as f64).powi(2))
}

/// One node's entries (`rain::Wet`): the places that see it, then the places only its drifting rain reaches (`cell`);
/// of the places that see it (the first `share.len()`) the share of the node's air over each, and per active gas the
/// node's saturation ratio (its amount over what saturates it at its mean ground) at which each place's air saturates
/// (`[y * len + e]`); the ratios, as shares of that, at which the parts of the air over a place saturate (`parts`); per
/// active gas the parts of those places in rising order of the ratio past which each rains, each `[ratio, Σ share so
/// far, Σ share × ratio so far]` (`Seen::new`); per place that sees it the share of what condenses over it that lands
/// there (`kept`); and per entry the share of what drifts that lands there (`drift`, summing to one).
pub(super) struct Seen {
    pub(super) cell: Vec<u32>,
    share: Vec<f64>,
    ratio: Vec<f64>,
    parts: Vec<f64>,
    steps: Vec<[f64; 3]>,
    kept: Vec<f64>,
    drift: Vec<f64>,
}

impl Seen {
    /// From its places (those that see it first), the shares of those, per active gas their ratios, the parts of the
    /// air over each, what each keeps of what condenses over it, and what of the drift each entry gets; for a gas that
    /// cannot rain (`rains`) no entries in order (nothing reads them: `Pool::day` follows only nodes whose gas may
    /// condense), zeros.
    pub(super) fn new(cell: Vec<u32>, share: Vec<f64>, ratio: Vec<f64>, (rains, parts): (&[bool], &[f64]), (kept, drift): (Vec<f64>, Vec<f64>)) -> Seen {
        let mut steps = Vec::with_capacity(ratio.len() * parts.len());
        for (by, &rain) in ratio.chunks(share.len().max(1)).zip(rains) {
            if !rain {
                steps.resize(steps.len() + by.len() * parts.len(), [0.0; 3]);
                continue;
            }
            let mut order: Vec<(f64, f64)> = by.iter().zip(&share).flat_map(|(&r, &a)| parts.iter().map(move |&u| (r * u, a / parts.len() as f64))).collect();
            order.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
            let mut sum = [0.0; 2];
            for (r, a) in order {
                sum = [sum[0] + a, sum[1] + a * r];
                steps.push([r, sum[0], sum[1]]);
            }
        }
        Seen { cell, share, ratio, parts: parts.to_vec(), steps, kept, drift }
    }
    /// The entries of active gas `y` in rising order (`Seen::new`).
    pub(super) fn steps(&self, y: usize) -> &[[f64; 3]] {
        let len = self.share.len() * self.parts.len();
        &self.steps[y * len..(y + 1) * len]
    }
    /// The share of the node's sky its cloud of active gas `y` covers at its saturation ratio `r`: over each place the
    /// share of its air that is saturated (`saturated`, from the ratio its moistest part saturates at), by its share.
    pub(super) fn clouded(&self, y: usize, r: f64, r0: f64) -> f64 {
        let len = self.share.len();
        let all: f64 = self.share.iter().sum();
        (0..len).map(|e| self.share[e] * saturated(r / self.ratio[y * len + e], r0)).sum::<f64>() / all.max(f64::MIN_POSITIVE)
    }
    /// Where what it rains of active gas `y` at its saturation ratio `r` lands, each entry with its share of it: what
    /// condenses over each place (each part of the air over it, by its share, holds over its saturation what the node
    /// holds over the part's ratio) lands there by the share `kept` of it, and the rest drifts with the wind before it
    /// falls, over its entries by their share of the drift (`drift`, Smith and Barstad 2004): `each(entry, share)` for
    /// each entry that gets any, in order. Whether any condenses.
    pub(super) fn landing(&self, y: usize, r: f64, mut each: impl FnMut(usize, f64)) -> bool {
        let len = self.share.len();
        let over = |e: usize| self.share[e] * self.parts.iter().map(|&u| (r - self.ratio[y * len + e] * u).max(0.0)).sum::<f64>() / self.parts.len() as f64;
        let all: f64 = (0..len).map(over).sum();
        if all <= 0.0 {
            return false;
        }
        let drift = (0..len).map(|e| (1.0 - self.kept[e]) * over(e)).sum::<f64>() / all;
        let kept = |e: usize| if e < len { self.kept[e] * over(e) / all } else { 0.0 };
        (0..self.cell.len()).map(|e| (e, kept(e) + drift * self.drift[e])).filter(|p| p.1 > 0.0).for_each(|(e, f)| each(e, f));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{parts, saturated, Seen, MOIST_PARTS};

    /// A node's rain lands whole: kept, it falls where it condenses; drifted, by the node's air over each place, so a
    /// place where nothing condenses gets its share too.
    #[test]
    fn a_nodes_rain_lands_where_it_condenses_and_drifts_by_its_air() {
        let (share, ratio, r) = (vec![0.2, 0.3, 0.5], vec![0.8, 0.9, 1.2], 1.0);
        let over = [0.2 * 0.2, 0.3 * 0.1, 0.0];
        for kept in [1.0, 0.0, 0.4] {
            let all: f64 = share.iter().sum();
            let seen = Seen::new(vec![0, 1, 2], share.clone(), ratio.clone(), (&[true], &[1.0]), (vec![kept; 3], share.iter().map(|a| a / all).collect()));
            let mut land = Vec::new();
            seen.landing(0, r, |e, f| land.push((e, f)));
            let got = |e: usize| land.iter().find(|p| p.0 == e).map_or(0.0, |p| p.1);
            assert!((land.iter().map(|p| p.1).sum::<f64>() - 1.0).abs() < 1e-12, "kept {kept}: {land:?}");
            for e in 0..3 {
                let want = kept * over[e] / (over[0] + over[1]) + (1.0 - kept) * share[e];
                assert!((got(e) - want).abs() < 1e-12, "kept {kept}, place {e}: {} against {want}", got(e));
            }
        }
    }

    /// The cloud's share of the sky is none until the moistest air saturates, all at saturation, and grows between; the
    /// parts of the air saturate in the order of that share, one in each equal piece of it.
    #[test]
    fn the_cloud_covers_none_below_the_least_ratio_and_all_at_saturation() {
        let r0 = 0.7;
        assert_eq!((saturated(0.5, r0), saturated(r0, r0), saturated(1.0, r0), saturated(1.3, r0)), (0.0, 0.0, 1.0, 1.0));
        let b: Vec<f64> = (1..30).map(|i| saturated(r0 + (1.0 - r0) * i as f64 / 30.0, r0)).collect();
        assert!(b.windows(2).all(|p| p[1] > p[0]) && b[0] > 0.0 && b[28] < 1.0, "{b:?}");
        let u = parts(r0);
        for (k, &uk) in u.iter().enumerate() {
            assert!(r0 < uk && uk < 1.0 && (saturated(uk, r0) - (k as f64 + 0.5) / MOIST_PARTS as f64).abs() < 1e-12, "part {k} at {uk}");
        }
    }
}

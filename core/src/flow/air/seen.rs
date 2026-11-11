//! One air node's places for its rain (laws.md 1.5): the places that see it, what each would hold over its own
//! saturation, and where what the node rains lands (`Seen::landing`).

/// One node's entries (`rain::Wet`): the places that see it (`cell`), the share of the node's air over each, and per
/// active gas the node's saturation ratio (its amount over what saturates it at its mean ground) past which each rains
/// (`[y * len + e]`); per active gas the entries in rising order of that ratio, each `[ratio, Σ share so far, Σ share ×
/// ratio so far]` (`Seen::new`); and per place the share of what condenses over it that lands there (`kept`).
pub(super) struct Seen {
    pub(super) cell: Vec<u32>,
    pub(super) share: Vec<f64>,
    pub(super) ratio: Vec<f64>,
    pub(super) steps: Vec<[f64; 3]>,
    kept: Vec<f64>,
}

impl Seen {
    /// From its cells, their shares and per active gas their ratios; for a gas that cannot rain (`rains`) no entries
    /// in order (nothing reads them: `Pool::day` follows only nodes whose gas may condense), zeros.
    pub(super) fn new(cell: Vec<u32>, share: Vec<f64>, ratio: Vec<f64>, rains: &[bool], kept: Vec<f64>) -> Seen {
        let mut steps = Vec::with_capacity(ratio.len());
        for (by, &rain) in ratio.chunks(cell.len().max(1)).zip(rains) {
            if !rain {
                steps.resize(steps.len() + by.len(), [0.0; 3]);
                continue;
            }
            let mut order: Vec<(f64, f64)> = by.iter().copied().zip(share.iter().copied()).collect();
            order.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
            let mut sum = [0.0; 2];
            for (r, a) in order {
                sum = [sum[0] + a, sum[1] + a * r];
                steps.push([r, sum[0], sum[1]]);
            }
        }
        Seen { cell, share, ratio, steps, kept }
    }
    /// Where what it rains of active gas `y` at its saturation ratio `r` lands, each entry with its share of it: what
    /// condenses over each place (the share of the node's air over it times what that holds over its saturation) lands
    /// there by the share `kept` of it, and the rest drifts with the wind before it falls, over all its places by the
    /// share of its air over each (Smith and Barstad 2004): `each(entry, share)` for each place that gets any, in order.
    /// Whether any condenses.
    pub(super) fn landing(&self, y: usize, r: f64, mut each: impl FnMut(usize, f64)) -> bool {
        let len = self.cell.len();
        let over = |e: usize| self.share[e] * (r - self.ratio[y * len + e]).max(0.0);
        let all: f64 = (0..len).map(over).sum();
        if all <= 0.0 {
            return false;
        }
        let drift = (0..len).map(|e| (1.0 - self.kept[e]) * over(e)).sum::<f64>() / all / self.share.iter().sum::<f64>();
        (0..len).map(|e| (e, self.kept[e] * over(e) / all + drift * self.share[e])).filter(|p| p.1 > 0.0).for_each(|(e, f)| each(e, f));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Seen;

    /// A node's rain lands whole: kept, it falls where it condenses; drifted, by the node's air over each place, so a
    /// place where nothing condenses gets its share too.
    #[test]
    fn a_nodes_rain_lands_where_it_condenses_and_drifts_by_its_air() {
        let (share, ratio, r) = (vec![0.2, 0.3, 0.5], vec![0.8, 0.9, 1.2], 1.0);
        let over = [0.2 * 0.2, 0.3 * 0.1, 0.0];
        for kept in [1.0, 0.0, 0.4] {
            let seen = Seen::new(vec![0, 1, 2], share.clone(), ratio.clone(), &[true], vec![kept; 3]);
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
}

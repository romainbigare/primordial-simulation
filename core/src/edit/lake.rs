//! A lake made by an edit (handover.md 11.1: "Flood: add liquid"; the owner: "flatten terrain + add water"): the
//! ground under a brush levelled to a floor under its rim, and the key liquid poured in to the rim, all from the
//! outside node. Whether it holds is the ground's: a floor under every sample round it is a closed basin, a pond of the
//! samples, whose body keeps what is poured into it; where the brush's rim is lower than its floor somewhere, the
//! liquid runs off as all liquid does.

use super::brush::Covered;
use super::Brush;
use crate::space::network::NONE;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use std::collections::BTreeMap;

/// The ring of samples round a brush that its rim is read on, samples wide.
const RIM_SAMPLES: f64 = 1.5;

impl World {
    /// A lake `depth` m deep under the brush `b` (the module's doc). Its level is that of the lowest sample on its rim
    /// (the samples within `RIM_SAMPLES` round it, as last built: the samples are built first where edits shaped them);
    /// every sample it covers moves to its floor, `depth` under that level: the ground cut or filled with each cell's
    /// own rock (`move_ground`), and the samples built again over it (`flow::build::reshape`). The key liquid fills it to
    /// its level from the outside node, each cell `depth` × its samples' area at its ground's temperature, poured into
    /// its part of the pond under the brush's centre (`flow::pour`); then the passes of a month's start and the liquid
    /// settled with no time (`flow::settle`) level it. Whether a lake was made.
    pub(crate) fn lake(&mut self, b: Brush, depth: f64) -> bool {
        let Some(liquid) = crate::cover::Parts::of(&self.chem).map(|p| p.liquid) else { return false };
        if self.relief.due {
            crate::flow::build::reshape(self);
        }
        let s = self.terrain_samples();
        let wide = Brush { centre: b.centre, radius_m: b.radius_m + RIM_SAMPLES * self.map.length(SAMPLE_DEPTH) };
        let all = self.covered(wide, s.across);
        let inside = |c: &Covered| c.d * wide.radius_m <= b.radius_m;
        let rim = all.iter().filter(|c| !inside(c)).map(|c| s.h[c.k]).fold(f64::INFINITY, f64::min);
        let Some(centre) = all.iter().filter(|c| inside(c)).min_by(|a, z| a.d.total_cmp(&z.d)).map(|c| c.k) else { return false };
        if !rim.is_finite() || depth <= 0.0 {
            return false;
        }
        let floor = rim - depth;
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        all.iter().filter(|c| inside(c)).for_each(|c| cells.entry(c.cell).or_default().push((c.k, floor - s.h[c.k])));
        let area: BTreeMap<usize, f64> = cells.iter().map(|(&i, ks)| (i, ks.iter().map(|&(k, _)| s.area(k)).sum())).collect();
        drop(s);
        self.move_ground(&cells, 0.0, &[]);
        crate::flow::build::reshape(self);
        let pond = self.terrain_samples().pond[centre];
        let sub = &self.chem.subs[liquid];
        let per_m3 = sub.density[1] / sub.molar_mass;
        for (&i, &a) in &area {
            let mut g = vec![0.0; self.chem.ns()];
            g[liquid] = depth * a * per_m3;
            self.put(i, &g, &vec![0.0; self.chem.ns()]);
            if pond != NONE {
                crate::flow::pour(self, i, pond, depth * a);
            }
        }
        crate::month_start(self);
        crate::flow::settle(self);
        true
    }
}

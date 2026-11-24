//! A basin of liquid made by an edit (handover.md 11.1: add liquid; the ground levelled and the key liquid added): the
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
    /// A basin of liquid `depth` m deep under the brush `b` (the module's doc). Its level is that of the lowest sample on its rim
    /// (the samples within `RIM_SAMPLES` round it, as built over the cells' grounds now);
    /// every sample it covers moves to its floor, `depth` under that level: the ground cut or filled with each cell's
    /// own rock (`move_ground`), and the samples built again over it (`flow::build::reshape`). The key liquid fills the
    /// pond under the brush's centre to its level, its pass (at least `depth` over the floor), from the outside node at
    /// each cell's ground temperature, poured into each cell's part of it (`flow::pour`); with no pond there (the floor
    /// opens to lower ground), `depth` over the floor, which runs off. Then the passes of a month's start and the liquid
    /// settled with no time (`flow::settle`) level it. Whether a basin was made.
    pub(crate) fn basin(&mut self, b: Brush, depth: f64) -> bool {
        let Some(liquid) = crate::cover::Parts::of(&self.chem).map(|p| p.liquid) else { return false };
        // (The rim is read on the samples as they stand over the cells' grounds: built again first where edits shaped
        // them, or where they were never built from the running world, the seed's.)
        if self.relief.due || self.relief.version == 0 {
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
        // What fills it: of the pond under its centre, each member sample to the pond's level (its pass, the lowest
        // the liquid leaves by), by cell; with no pond, `depth` over the floor, which runs off.
        let s = self.terrain_samples();
        let pond = s.pond[centre];
        let fill: BTreeMap<usize, f64> = if pond == NONE {
            area.iter().map(|(&i, &a)| (i, depth * a)).collect()
        } else {
            let level = s.ponds[pond as usize].level;
            let mut by: BTreeMap<usize, f64> = BTreeMap::new();
            for (i, l) in self.map.leaves().iter().enumerate() {
                let v: f64 = l.split(SAMPLE_DEPTH.max(l.depth)).map(|f| s.index(&f)).filter(|&k| s.pond[k] == pond).map(|k| (level - s.h[k]).max(0.0) * s.area(k)).sum();
                if v > 0.0 {
                    by.insert(i, v);
                }
            }
            by
        };
        drop(s);
        let sub = &self.chem.subs[liquid];
        let per_m3 = sub.density[1] / sub.molar_mass;
        for (&i, &v) in &fill {
            let mut g = vec![0.0; self.chem.ns()];
            g[liquid] = v * per_m3;
            self.put(i, &g, &vec![0.0; self.chem.ns()]);
            if pond != NONE {
                crate::flow::pour(self, i, pond, v);
            }
        }
        crate::month_start(self);
        crate::flow::settle(self);
        true
    }
}

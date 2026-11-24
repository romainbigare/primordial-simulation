//! A basin of liquid made by an edit (handover.md 11.1: add liquid; the ground levelled and the key liquid added): the
//! ground under a brush levelled to a floor, its rim raised where it lies lower than the level the liquid is to stand
//! at, and the key liquid poured in to that level, all from the outside node. So the basin is closed by its making: a
//! pond of the samples, whose body keeps what is poured into it.

use super::brush::Covered;
use super::Brush;
use crate::space::network::NONE;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use std::collections::BTreeMap;

/// The ring of samples round a brush that its rim is built on, samples wide: wide enough that every sample under the
/// brush has its neighbours across sides and corners under it or on the ring.
const RIM_SAMPLES: f64 = 1.5;

impl World {
    /// A basin of liquid `depth` m deep under the brush `b` (the module's doc). Its level is the mean height of the
    /// samples it covers (as built over the cells' grounds now), the ground levelled: every sample it covers moves to its
    /// floor, `depth` under that level, and every sample on its rim (within `RIM_SAMPLES` round it) lower than the
    /// level is raised to it; the ground cut or filled with each cell's own rock (`move_ground`), and the samples built
    /// again over it (`flow::build::reshape`). The key liquid fills the pond under the brush's centre to its level, its
    /// pass, from the outside node at each cell's ground temperature, poured into each cell's part of it
    /// (`pour_pond`); then the passes of a month's start and the liquid settled with no time (`flow::settle`) level
    /// it (a dry cell's pores take their share first). Whether a basin was made: none whose centre lies in the open low ground (its liquid stands there already).
    pub(crate) fn basin(&mut self, b: Brush, depth: f64) -> bool {
        let Some(liquid) = crate::cover::Parts::of(&self.chem).map(|p| p.liquid) else { return false };
        // (The ground is read on the samples as they stand over the cells' grounds: built again first where edits
        // shaped them, or where they were never built from the running world, the seed's.)
        if self.relief.due || self.relief.version == 0 {
            crate::flow::build::reshape(self);
        }
        let s = self.terrain_samples();
        let wide = Brush { centre: b.centre, radius_m: b.radius_m + RIM_SAMPLES * self.map.length(SAMPLE_DEPTH) };
        let all = self.covered(wide, s.across);
        let inside = |c: &Covered| c.d * wide.radius_m <= b.radius_m;
        let (sum, area) = all.iter().filter(|c| inside(c)).fold((0.0, 0.0), |(v, a), c| (v + s.h[c.k] * s.area(c.k), a + s.area(c.k)));
        let Some(centre) = all.iter().filter(|c| inside(c)).min_by(|a, z| a.d.total_cmp(&z.d)).map(|c| c.k) else { return false };
        let open = |s: &crate::space::samples::Samples, k: usize| s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE;
        if area <= 0.0 || depth <= 0.0 || open(&s, centre) {
            return false;
        }
        let level = sum / area;
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        for c in &all {
            let to = if inside(c) { level - depth } else { s.h[c.k].max(level) };
            cells.entry(c.cell).or_default().push((c.k, to - s.h[c.k]));
        }
        drop(s);
        self.move_ground(&cells, 0.0, &[]);
        crate::flow::build::reshape(self);
        // What fills it: the pond under its centre, to its level (its pass, the lowest the liquid leaves by); none
        // where its floor opened to the open low ground, whose liquid stands there already.
        let s = self.terrain_samples();
        let pond = s.pond[centre];
        let filled = pond != NONE && !open(&s, centre);
        drop(s);
        if filled {
            self.pour_pond(pond, liquid);
        }
        crate::month_start(self);
        crate::flow::settle(self);
        true
    }

    /// The key liquid `liquid` poured into each cell's part of pond `pond` (`flow::pour`): over each of its samples to
    /// the pond's level, m³, from the outside node at each cell's ground temperature.
    fn pour_pond(&mut self, pond: u32, liquid: usize) {
        let s = self.terrain_samples();
        let level = s.ponds[pond as usize].level;
        let mut by: BTreeMap<usize, f64> = BTreeMap::new();
        for (i, l) in self.map.leaves().iter().enumerate() {
            let v: f64 = l.split(SAMPLE_DEPTH.max(l.depth)).map(|f| s.index(&f)).filter(|&k| s.pond[k] == pond).map(|k| (level - s.h[k]).max(0.0) * s.area(k)).sum();
            if v > 0.0 {
                by.insert(i, v);
            }
        }
        drop(s);
        let sub = &self.chem.subs[liquid];
        let per_m3 = sub.density[1] / sub.molar_mass;
        for (&i, &v) in &by {
            let mut g = vec![0.0; self.chem.ns()];
            g[liquid] = v * per_m3;
            self.put(i, &g, &vec![0.0; self.chem.ns()]);
            crate::flow::pour(self, i, pond, v);
        }
    }
}

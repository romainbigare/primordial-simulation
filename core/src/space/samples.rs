//! The terrain's samples, drained (laws.md 0 and 8): the noise at the centre of every leaf of `SAMPLE_DEPTH` of the
//! map, its height and its slope finer than the samples (`Terrain::sample`), and a priority flood (`space::drain`)
//! that raises each closed basin of the samples to its outlet, so liquid runs off every sample of land it reaches
//! (Planchon and Darboux 2001; Barnes, Lehman and Mulla 2014): noise has basins that no liquid ever cut. Every cell's
//! height and relief, at the start and at each split, are means over these samples (`Terrain::inside`), so a cell and
//! its quarters stand on the same ground at every size; so is the way it faces the sun (`Terrain::rise_south`). They are a function of the seed and the map alone, found when
//! first read and kept with the terrain, not saved.

use super::drain::drain;
use crate::math::central;
use super::square::Square;
use super::terrain::{Terrain, SAMPLE_DEPTH};
use super::tree::BLOCK;
use super::Leaf;
use std::sync::{Arc, Mutex};

/// The drained samples of a map for one seed: its width and height in samples, and of each sample row by row its
/// height, m, and the rms slope of the noise below it.
#[derive(Debug)]
pub struct Samples {
    pub seed: u64,
    pub across: usize,
    pub down: usize,
    pub h: Vec<f64>,
    pub g: Vec<f32>,
}

impl Samples {
    /// The index of the sample over leaf `l` of `SAMPLE_DEPTH` or finer.
    pub fn index(&self, l: &Leaf) -> usize {
        let side = BLOCK >> SAMPLE_DEPTH;
        (l.y / side) as usize * self.across + (l.x / side) as usize
    }
}

/// The samples a terrain found, kept with it (not saved: found again from the seed when first read).
#[derive(Debug, Default)]
pub(super) struct Kept(Mutex<Option<Arc<Samples>>>);

impl Clone for Kept {
    fn clone(&self) -> Kept {
        Kept(Mutex::new(self.0.lock().expect("the kept samples").clone()))
    }
}

impl Terrain {
    /// The drained samples of `map` for `seed` (its rows between latitudes `south` and `north`), found once and kept.
    /// They are found with no lock held (finding them works in parallel, and a worker that waited on the lock inside
    /// another parallel section would hold up the workers it needs); where two find them at once, the first kept is
    /// the one (they are the same).
    pub fn samples(&self, seed: u64, map: &Square, lat: (f64, f64)) -> Arc<Samples> {
        let known = |kept: &Option<Arc<Samples>>| kept.as_ref().filter(|s| s.seed == seed).cloned();
        if let Some(s) = known(&self.kept.0.lock().expect("the kept samples")) {
            return s;
        }
        let found = Arc::new(self.drained(seed, map, lat));
        let mut kept = self.kept.0.lock().expect("the kept samples");
        known(&kept).unwrap_or_else(|| {
            *kept = Some(found.clone());
            found
        })
    }

    /// The mean rise of the ground toward the south over leaf `l` of `map` for `seed` (its rows between latitudes `south`
    /// and `north`), m per m: by area over the drained samples it covers, each its central difference north to south
    /// (one-sided at the map's edge); a leaf finer than a sample takes its sample's. So a leaf's is its quarters' mean by
    /// area at every depth. It is what the sun on a cell reads of the way the cell faces (`flow::on_slope`).
    pub fn rise_south(&self, seed: u64, map: &Square, l: Leaf, (south, north): (f64, f64)) -> f64 {
        let s = self.samples(seed, map, (south, north));
        let dy = map.length(SAMPLE_DEPTH);
        let at = |f: &Leaf| {
            let k = s.index(f);
            central(k / s.across, s.down, |r| s.h[r * s.across + k % s.across], dy)
        };
        if l.depth <= SAMPLE_DEPTH {
            let (area, sum) = l.split(SAMPLE_DEPTH).map(|f| (map.place_of(&f, south, north).0, at(&f))).fold((0.0, 0.0), |(a, by), (da, r)| (a + da, by + da * r));
            return sum / area;
        }
        at(&l)
    }

    /// The samples of `map` for `seed`, drained: a priority flood from the low ground (on a globe every sample under
    /// 0 m; on an island those under 0 m joined to its lowest sample, so ground under 0 m inland is a closed basin too)
    /// over the samples and their 8 neighbours (east and west round a globe), the lowest first, then the least index.
    fn drained(&self, seed: u64, map: &Square, (south, north): (f64, f64)) -> Samples {
        let per = 1usize << (SAMPLE_DEPTH - map.square_depth());
        let (across, down) = (map.w * per, map.h * per);
        let side = BLOCK >> SAMPLE_DEPTH;
        let leaf = |k: usize| Leaf { x: (k % across) as u64 * side, y: (k / across) as u64 * side, depth: SAMPLE_DEPTH };
        let found = crate::par::map_n(across * down, crate::par::UNCOUNTED, |k| self.sample(seed, map, map.place_of(&leaf(k), south, north).1));
        let (mut h, g): (Vec<f64>, Vec<f32>) = found.into_iter().map(|(h, g)| (h, g as f32)).unzip();
        let dy = map.length(SAMPLE_DEPTH);
        let dx: &Vec<f64> = &(0..down).map(|r| map.place_of(&leaf(r * across), south, north).0 / dy).collect();
        let around = |k: usize| {
            let (r, c) = ((k / across) as i64, (k % across) as i64);
            let steps = (-1..=1i64).flat_map(|a| (-1..=1i64).map(move |b| (a, b)));
            steps.filter(|&s| s != (0, 0)).filter_map(move |(a, b)| {
                let (r2, c2) = (r + a, if map.wrap { (c + b).rem_euclid(across as i64) } else { c + b });
                ((0..down as i64).contains(&r2) && (0..across as i64).contains(&c2)).then(|| {
                    let x = (dx[r as usize] + dx[r2 as usize]) / 2.0;
                    ((r2 as usize) * across + c2 as usize, (x * b as f64).hypot(dy * a as f64))
                })
            })
        };
        let mut reached: Vec<bool> = h.iter().map(|&z| z < 0.0 && map.wrap).collect();
        if let Some(low) = (0..h.len()).min_by(|&a, &b| h[a].total_cmp(&h[b]).then(a.cmp(&b))).filter(|&k| !map.wrap && h[k] < 0.0) {
            let mut todo = vec![low];
            reached[low] = true;
            while let Some(k) = todo.pop() {
                for (j, _) in around(k) {
                    if h[j] < 0.0 && !reached[j] {
                        reached[j] = true;
                        todo.push(j);
                    }
                }
            }
        }
        drain(&mut h, reached, around);
        Samples { seed, across, down, h, g }
    }
}

//! How high the ground stands toward the light, per terrain sample: what a shell draws the shade of the heights by. A
//! shell lights the ground from the north-west (web/shaders/lib/draw.wgsl `LIGHT`); a point lies in the shade of
//! higher ground where that ground stands over the light's ray to it, so where the slope up to it from the point is
//! more than the light's own rise. That slope is found here once for each sample, and the shell compares it with its
//! light.

use pcore::space::samples::Samples;

/// Of each sample of `s` (`metres` m a side), the steepest slope up from it to a sample toward the north-west, along
/// its diagonal of samples, rise over run; none where none stands higher. Each diagonal is walked once from its
/// north-west end, keeping the samples behind that can still stand highest over one ahead (the upper hull of their
/// heights: one under the line between two others never does), the last of them the one the sample looks up to.
pub(super) fn horizon(s: &Samples, metres: f64) -> Vec<f32> {
    let step = metres * std::f64::consts::SQRT_2;
    let mut out = vec![0.0; s.h.len()];
    let mut behind: Vec<(f64, f64)> = Vec::new();
    for (x, y) in (0..s.across).map(|x| (x, 0)).chain((1..s.down).map(|y| (0, y))) {
        behind.clear();
        for n in 0..(s.across - x).min(s.down - y) {
            let (k, at) = ((y + n) * s.across + x + n, n as f64 * step);
            let up = |p: &(f64, f64)| (p.1 - s.h[k]) / (at - p.0);
            while behind.len() > 1 && up(&behind[behind.len() - 2]) >= up(&behind[behind.len() - 1]) {
                behind.pop();
            }
            out[k] = behind.last().map_or(0.0, |p| up(p).max(0.0)) as f32;
            behind.push((at, s.h[k]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::horizon;
    use pcore::space::terrain::SAMPLE_DEPTH;

    /// On the island of seed 1, each sample's horizon is the steepest slope up from it to any sample toward the
    /// north-west on its diagonal, found here one by one, and none where none of them stands higher: its highest
    /// sample looks up to none, and some of its ground lies under a light that rises a quarter.
    #[test]
    fn a_samples_horizon_is_the_steepest_way_up_toward_the_light() {
        let cfg = pcore::Config::default();
        let (map, terrain) = worldgen::terrain::island_ground(1, (16, 16), pcore::space::terrain::Shape::One, &cfg.phys);
        let s = terrain.samples(1, &map, (cfg.lat_south, cfg.lat_north));
        let step = map.length(SAMPLE_DEPTH) * std::f64::consts::SQRT_2;
        let found = horizon(&s, map.length(SAMPLE_DEPTH));
        for k in (0..s.h.len()).step_by(7) {
            let (x, y) = (k % s.across, k / s.across);
            let up = (1..=x.min(y)).map(|n| (s.h[k - n * (s.across + 1)] - s.h[k]) / (n as f64 * step)).fold(0.0, f64::max);
            assert!((found[k] as f64 - up).abs() <= 1e-6 * up.max(1.0), "sample {k}: {} for {up}", found[k]);
        }
        let top = (0..s.h.len()).max_by(|&a, &b| s.h[a].total_cmp(&s.h[b])).unwrap();
        assert!(found[top] == 0.0 && found.iter().any(|&v| v > 0.25), "the top looks up to {}", found[top]);
    }
}

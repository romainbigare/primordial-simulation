//! The channels as a shell draws them (`channels`, `bends`): a texel names the channel whose line it measures, and a
//! channel's line runs on unbroken from sample to sample.

use super::super::bends::phases;
use super::{code, to_segment, Channels, Law, STEP_MOST};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;
use std::sync::Arc;

/// The map of the island of seed 1, 16 x 16 squares, its samples, the metres a sample takes, and the default channel
/// law.
fn island() -> (Square, Arc<Samples>, f64, Law) {
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(1, 16, 16);
    let s = terrain.samples(1, &map, (cfg.lat_south, cfg.lat_north));
    let metres = map.length(SAMPLE_DEPTH);
    (map, s, metres, Law::of(&cfg.phys))
}

impl Channels<'_> {
    /// The distance from `at` (samples) to the line of sample `q` drawn on texels of `texel` m, m.
    fn distance(&self, q: usize, at: [f64; 2], texel: f64) -> f64 {
        let p = self.piece(q, texel).expect("a channel");
        let pts = self.line(q, &p, texel).0;
        pts.windows(2).map(|w| to_segment(at, w[0], w[1]).1.sqrt()).fold(f64::INFINITY, f64::min) * self.metres
    }
}

/// Texels this many samples a side: fine enough to draw the bends of the larger channels.
const TEXEL_SAMPLES: f64 = 1.0 / 32.0;

/// Each texel a tile holds a channel in names the sample whose line it holds the distance to: the distance from the
/// texel's centre to that sample's own line is the texel's (so the discharge a shell reads there is the channel's own),
/// and the sample comes back from the texel's step as a shell finds it.
#[test]
fn a_texel_names_the_channel_it_measures() {
    let (map, s, metres, law) = island();
    let phase = phases(&law, &s, metres);
    let ch = Channels::new(&map, &s, law, &phase);
    let k = TEXEL_SAMPLES;
    // A window about the sample the most drains through, its outlet's river.
    let most = (0..s.up.len()).filter(|&q| s.to[q] != NONE).max_by(|&a, &b| s.up[a].total_cmp(&s.up[b])).expect("a river");
    let [x0, y0] = [((most % s.across) as f64 / k) as i64 - 64, ((most / s.across) as f64 / k) as i64 - 64];
    let f = ch.field([x0, y0], [128, 128], k);
    let mut named = 0;
    for (i, &q) in f.sample.iter().enumerate().filter(|p| *p.1 != NONE) {
        let texel = [x0 + (i % 128) as i64, y0 + (i / 128) as i64];
        let at = [(texel[0] as f64 + 0.5) * k, (texel[1] as f64 + 0.5) * k];
        let own = ch.distance(q as usize, at, k * metres);
        assert!((f.dist[i].abs() - own).abs() < 1e-9, "texel {texel:?}: {} m against {own} m to its own sample's line", f.dist[i]);
        let c = code(&s, texel, k, q) as i64 - 1;
        let side = 2 * STEP_MOST + 1;
        let back = |t: i64, step: i64| ((t as f64 + 0.5) * k).floor() as i64 + step - STEP_MOST;
        assert_eq!((back(texel[1], c / side) * s.across as i64 + back(texel[0], c % side)) as u32, q, "texel {texel:?}");
        named += 1;
    }
    assert!(named > 100, "{named} texels hold a channel");
}

/// Where a sample is the main one upstream of the next, its line ends where the next one's begins, its bends and all:
/// a river's line has no break.
#[test]
fn a_channels_line_runs_on_from_sample_to_sample() {
    let (map, s, metres, law) = island();
    let phase = phases(&law, &s, metres);
    let ch = Channels::new(&map, &s, law, &phase);
    let texel = TEXEL_SAMPLES * metres;
    let end = |k: usize, t: f64| ch.piece(k, texel).expect("a channel").at(t);
    let mut joints = 0;
    for k in (0..s.up.len()).filter(|&k| s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32) {
        let (a, b) = (end(k, 1.0), end(s.to[k] as usize, 0.0));
        let gap = (a[0] - b[0]).hypot(a[1] - b[1]) * metres;
        assert!(gap < 1e-6, "sample {k}: a gap of {gap} m to the next");
        joints += 1;
    }
    assert!(joints > 1000, "{joints} joints");
}

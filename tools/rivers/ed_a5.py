p = 'tape/src/ground/tests.rs'
t = open(p).read()
i = t.index('/// The shore drawn smooth (`shore`) moves the line')
shore = t[i:]
head = '''//! The channels as a shell draws them (`channels`, `bends`, `belt`): a texel names the channel whose line it measures,
//! a channel's line runs on unbroken from sample to sample, and its bends stay in their belt.

use super::super::bends::{Bends, Reaches, BEND_MOST};
use super::super::stems::points;
use super::{code, to_segment, Channels, Law, Window, STEP_MOST};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;
use std::sync::Arc;

/// The samples of the island of seed 1, 16 x 16 squares, the metres a sample takes, and the default channel law.
fn island() -> (Arc<Samples>, f64, Law) {
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(1, 16, 16);
    let s = terrain.samples(1, &map, (cfg.lat_south, cfg.lat_north));
    (s, map.length(SAMPLE_DEPTH), Law::of(&cfg.phys))
}

/// The reaches of the island's channels through the points `at`.
fn reaches<'a>(s: &'a Samples, at: &'a [[f64; 2]], metres: f64, law: Law) -> Reaches<'a> {
    Reaches { seed: 1, s, at, law, metres }
}

/// Texels this many samples a side: fine enough to draw the bends of the larger channels.
const TEXEL_SAMPLES: f64 = 1.0 / 128.0;

/// Each texel a tile holds a channel in names the sample whose line it holds the distance to: the distance from the
/// texel's centre to that sample's own line is the texel's (so the discharge a shell reads there is the channel's own),
/// and the sample comes back from the texel's step as a shell finds it.
#[test]
fn a_texel_names_the_channel_it_measures() {
    let (s, metres, law) = island();
    let at = points(&s);
    let r = reaches(&s, &at, metres, law);
    let bends = Bends::new(&r);
    let ch = Channels::new(r, &bends);
    let k = TEXEL_SAMPLES;
    // A window about the sample the most drains through, its outlet's river.
    let most = (0..s.up.len()).filter(|&q| s.to[q] != NONE).max_by(|&a, &b| s.up[a].total_cmp(&s.up[b])).expect("a river");
    let [x0, y0] = [((most % s.across) as f64 / k) as i64 - 64, ((most / s.across) as f64 / k) as i64 - 64];
    let f = ch.field(Window { x0, y0, wide: 128, high: 128, k });
    let mut named = 0;
    for (i, &q) in f.sample.iter().enumerate().filter(|p| *p.1 != NONE) {
        let texel = [x0 + (i % 128) as i64, y0 + (i / 128) as i64];
        let at = [(texel[0] as f64 + 0.5) * k, (texel[1] as f64 + 0.5) * k];
        let pts = ch.line(q as usize, k * metres).expect("a channel").0;
        let own = pts.windows(2).map(|w| to_segment(at, w[0].p, w[1].p).1.sqrt()).fold(f64::INFINITY, f64::min) * metres;
        assert!((f.dist[i].abs() - own).abs() < 1e-9, "texel {texel:?}: {} m against {own} m to its own sample's line", f.dist[i]);
        let c = code(&s, texel, k, q) as i64 - 1;
        let side = 2 * STEP_MOST + 1;
        let back = |t: i64, step: i64| ((t as f64 + 0.5) * k).floor() as i64 + step - STEP_MOST;
        assert_eq!((back(texel[1], c / side) * s.across as i64 + back(texel[0], c % side)) as u32, q, "texel {texel:?}");
        named += 1;
    }
    assert!(named > 100, "{named} texels hold a channel");
}

/// Where a sample is the main one upstream of the next, its line ends where the next one's begins, its bends and all,
/// and the length along the channel runs on: a river's line has no break. Some of the island's channels bend, and no
/// bend leaves its belt (`BEND_MOST`).
#[test]
fn a_channels_line_runs_on_from_sample_to_sample() {
    let (s, metres, law) = island();
    let at = points(&s);
    let r = reaches(&s, &at, metres, law);
    let bends = Bends::new(&r);
    let ch = Channels::new(r, &bends);
    let texel = TEXEL_SAMPLES * metres;
    let (mut joints, mut bent) = (0, 0);
    for k in (0..s.up.len()).filter(|&k| s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32) {
        let (own, next) = (ch.piece(k, texel).expect("a channel"), ch.piece(s.to[k] as usize, texel).expect("a channel"));
        let (a, b) = (own[own.len() - 1], next[0]);
        let gap = (a.p[0] - b.p[0]).hypot(a.p[1] - b.p[1]) * metres;
        assert!(gap < 1e-6, "sample {k}: a gap of {gap} m to the next");
        assert!((a.s - b.s).abs() < 1e-2, "sample {k}: its line ends {} m along, the next starts {} m along", a.s, b.s);
        let frame = r.frame(k).expect("a channel");
        let off = own.iter().map(|v| (0..=64).map(|i| { let q = frame.at(frame.len() * i as f64 / 64.0, 0.0); (v.p[0] - q[0]).hypot(v.p[1] - q[1]) }).fold(f64::INFINITY, f64::min)).fold(0.0, f64::max);
        assert!(off <= BEND_MOST + 0.05, "sample {k}: its line lies {off} samples from its curve");
        bent += (off > 0.01) as usize;
        joints += 1;
    }
    assert!(joints > 1000, "{joints} joints");
    assert!(bent > 20, "{bent} pieces bend");
}

'''
open(p, 'w').write(head + shore)

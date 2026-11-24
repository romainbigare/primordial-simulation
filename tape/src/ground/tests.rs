//! The channels as a shell draws them (`channels`, `bends`, `belt`): a texel names the channel whose line it measures,
//! a channel's line runs on unbroken from sample to sample, and its bends stay in their belt.

use super::super::bends::{Bends, Reaches, BEND_MOST};
use super::super::stems::points;
use super::{code, to_segment, Channels, Law, Window, NO_CHANNEL_M, STEP_MOST};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;
use std::sync::Arc;

/// The samples of the island of seed 1, 16 x 16 squares, the metres a sample takes, and the default channel law.
fn island() -> (Arc<Samples>, f64, Law) {
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(1, (16, 16), pcore::space::terrain::Shape::One, &cfg.phys);
    let s = terrain.samples(1, &map, (cfg.lat_south, cfg.lat_north));
    (s, map.length(SAMPLE_DEPTH), Law::of(&cfg.phys))
}

/// The reaches of the island's channels through the points `at`.
fn reaches<'a>(s: &'a Samples, at: &'a [[f64; 2]], metres: f64, law: Law) -> Reaches<'a> {
    Reaches { seed: 1, s, at, law, metres }
}

/// Texels this many samples a side: fine enough to draw the bends of the larger channels.
const TEXEL_SAMPLES: f64 = 1.0 / 128.0;

/// Each texel a tile holds a channel's line in names the sample whose line it holds the distance to: the distance
/// from the texel's centre to that sample's own line is the texel's (so the discharge a shell reads there is the
/// channel's own), and the sample comes back from the texel's step as a shell finds it.
#[test]
fn a_texel_names_the_channel_it_measures() {
    let (s, metres, law) = island();
    let at = points(&s, &super::super::shore::Shore::new(&s)).0;
    let r = reaches(&s, &at, metres, law);
    let bends = Bends::new(&r);
    let none = vec![false; s.up.len()];
    let ch = Channels::new(r, &bends, &none);
    let k = TEXEL_SAMPLES;
    // A window about the sample the most drains through, its outlet's river.
    let most = (0..s.up.len()).filter(|&q| s.to[q] != NONE).max_by(|&a, &b| s.up[a].total_cmp(&s.up[b])).expect("a river");
    let [x0, y0] = [((most % s.across) as f64 / k) as i64 - 64, ((most / s.across) as f64 / k) as i64 - 64];
    let f = ch.field(Window { x0, y0, wide: 128, high: 128, k });
    let mut named = 0;
    for (i, &q) in f.sample.iter().enumerate().filter(|p| *p.1 != NONE && f.dist[p.0].abs() < NO_CHANNEL_M) {
        let texel = [x0 + (i % 128) as i64, y0 + (i / 128) as i64];
        let at = [(texel[0] as f64 + 0.5) * k, (texel[1] as f64 + 0.5) * k];
        let pts = ch.line(q as usize, k * metres).expect("a channel").0.line;
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
/// and the length along the channel runs on: a river's line has no break. Some of the island's channels sway, and none
/// farther from its curve than `BEND_MOST`.
#[test]
fn a_channels_line_runs_on_from_sample_to_sample() {
    let (s, metres, law) = island();
    let at = points(&s, &super::super::shore::Shore::new(&s)).0;
    let r = reaches(&s, &at, metres, law);
    let bends = Bends::new(&r);
    let none = vec![false; s.up.len()];
    let ch = Channels::new(r, &bends, &none);
    let texel = TEXEL_SAMPLES * metres;
    let (mut joints, mut bent) = (0, 0);
    for k in (0..s.up.len()).filter(|&k| s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32) {
        let (own, next) = (ch.piece(k, texel).expect("a channel").line, ch.piece(s.to[k] as usize, texel).expect("a channel").line);
        let (a, b) = (own[own.len() - 1], next[0]);
        let gap = (a.p[0] - b.p[0]).hypot(a.p[1] - b.p[1]) * metres;
        assert!(gap < 1e-6, "sample {k}: a gap of {gap} m to the next");
        assert!((a.s - b.s).abs() < 1e-3, "sample {k}: its line ends {} m along, the next starts {} m along", a.s, b.s);
        let frame = r.frame(k).expect("a channel");
        let off = own.iter().map(|v| (0..=64).map(|i| { let q = frame.at(frame.len() * i as f64 / 64.0, 0.0); (v.p[0] - q[0]).hypot(v.p[1] - q[1]) }).fold(f64::INFINITY, f64::min)).fold(0.0, f64::max);
        assert!(off <= BEND_MOST + 0.05, "sample {k}: its line lies {off} samples from its curve");
        bent += (off > 0.01) as usize;
        joints += 1;
    }
    assert!(joints > 1000, "{joints} joints");
    assert!(bent > 20, "{bent} pieces bend");
}

/// The shore drawn smooth (`shore`) moves the line where the samples' ground crosses its level, not the area under it:
/// over the island the area under the level moves by under 0.3 of a sample times the shore's length (a blur shrinks what
/// bulges: an island and its capes).
#[test]
fn the_smooth_shore_keeps_the_area_under_the_level() {
    let (s, _, _) = island();
    let shore = super::super::shore::Shore::new(&s);
    // Points a quarter of a sample apart; the samples' ground blended between their centres.
    const PER: usize = 4;
    let at = |c: i64, r: i64| s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    let h = |p: [f64; 2]| {
        let (u, v) = (p[0] - 0.5, p[1] - 0.5);
        let (c, r, fu, fv) = (u.floor() as i64, v.floor() as i64, u - u.floor(), v - v.floor());
        (at(c, r) * (1.0 - fu) + at(c + 1, r) * fu) * (1.0 - fv) + (at(c, r + 1) * (1.0 - fu) + at(c + 1, r + 1) * fu) * fv
    };
    let (mut moved, mut crossings) = (0i64, 0i64);
    for y in 0..s.down * PER {
        let mut last = None;
        for x in 0..s.across * PER {
            let p = [(x as f64 + 0.5) / PER as f64, (y as f64 + 0.5) / PER as f64];
            let by = shore.beside(p);
            if by.level.is_nan() {
                last = None;
                continue;
            }
            let under = h(p) < by.level;
            moved += (shore.drawn(p, h(p), &by, [0.0; 2]).0 < by.level) as i64 - under as i64;
            crossings += (last.is_some_and(|l| l != under)) as i64;
            last = Some(under);
        }
    }
    // The shore's length, samples: at least its crossings of the rows over the rows a sample.
    let (length, area) = (crossings as f64 / PER as f64, moved as f64 / (PER * PER) as f64);
    assert!(length > 100.0, "a shore of {length} samples");
    assert!(area.abs() < 0.3 * length, "the area under the level moves by {area} samples² along a shore of {length} samples");
}

/// A ground made again after edits shaped the world's ground (`Ground::rebuild`) is the ground made anew on the same
/// samples: its points, shore, texture and tiles are the same to the last bit, though it finds the shore, the points
/// and the texture of each sample and the pieces of the channels again only where they may read what changed; and so
/// is the cells' table found again (`Table::update`). Two strokes, the second made again from the first's ground.
#[test]
fn a_ground_made_again_is_the_ground_made_anew() {
    use pcore::edit::{Act, Edit};
    let mut w = worldgen::testkit::small_world();
    let s = w.terrain_samples();
    // A place on a channel: the sample the most drains through that lies in no pond and has land round it.
    let k = (0..s.h.len()).filter(|&k| s.pond[k] == NONE && s.to[k] != NONE && s.h[k] > 0.0).max_by(|&a, &b| s.up[a].total_cmp(&s.up[b])).unwrap();
    let per = (s.across / w.map.w) as f64;
    let place = [((k % s.across) as f64 + 0.5) / per, ((k / s.across) as f64 + 0.5) / per];
    drop(s);
    // A stroke settled; the relief the world's samples will be built from, told before it settles, is the one they are
    // built from (none before the world's first build of its offsets).
    let stroke = |w: &mut pcore::World, height_m: f64| {
        w.edit(&[Edit { place, act: Act::Ground { radius_m: 0.3 * w.map.side, height_m, stroke_m: 0.0, make: 0 } }]);
        let told = super::super::Ground::settled_relief_bytes(w);
        w.settle();
        assert!(told.as_ref().is_none_or(|r| *r == super::super::Ground::relief_bytes(w)), "the relief told before the world settled");
        told.is_some()
    };
    stroke(&mut w, 20.0);
    let (mut again, mut table) = (super::super::Ground::of(&w), crate::Table::new(&w));
    assert!(stroke(&mut w, -10.0), "the second stroke's relief is told before it settles");
    table.update(&w);
    assert!(table.frame(&w).1 == crate::Table::new(&w).frame(&w).1 && table.discharge(&w) == crate::Table::new(&w).discharge(&w), "the cells' table found again");
    again.rebuild_from(&super::super::Ground::relief_bytes(&w)).expect("the relief");
    let anew = super::super::Ground::of(&w);
    let colours = vec![[0.5; 3]; w.terrain.makes.len()];
    assert!(again.at == anew.at && again.raw == anew.raw && again.shore.dry() == anew.shore.dry(), "the points and the shore");
    assert!(again.rock(&colours) == anew.rock(&colours), "the rock and the texture");
    let depths = super::super::depths(&w.map);
    for depth in [*depths.start(), depths.start() + 2] {
        let tiles = 1i64 << (depth - depths.start());
        for (tx, ty) in (0..tiles).flat_map(|y| (0..tiles).map(move |x| (x, y))) {
            assert!(again.tile(depth, tx, ty) == anew.tile(depth, tx, ty), "tile {tx}, {ty} of depth {depth}");
        }
    }
}

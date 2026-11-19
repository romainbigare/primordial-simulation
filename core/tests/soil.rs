//! The soil (laws.md 4): the texture the laws leave in the terrain's samples, and the pores that follow from it.

use pcore::soil::{Pores, Texture};
use pcore::space::network::NONE;

/// Steps of the coarse and the fine share, and of the organic share, over the textures checked.
const STEPS: usize = 20;
const ORGANIC_STEPS: usize = 4;

/// For every texture (coarse and fine shares that sum to at most 1, organic matter to 0.1) the pores hold liquid at the
/// wilting point, more at field capacity and more again when full (0 < θ1500 < θ33 < θs < 1), and pass liquid when
/// full (Saxton and Rawls 2006).
#[test]
fn ground_holds_less_at_the_wilting_point_than_at_field_capacity_than_full() {
    for o in 0..=ORGANIC_STEPS {
        for f in 0..=STEPS {
            for c in 0..=STEPS - f {
                let t = Texture { coarse: c as f64 / STEPS as f64, fine: f as f64 / STEPS as f64, organic: 0.1 * o as f64 / ORGANIC_STEPS as f64 };
                let p = Pores::of(&t);
                assert!(p.porosity > 0.0 && p.porosity < 1.0 && 0.0 < p.wilt && p.wilt < p.field && p.field < 1.0, "{t:?}: {p:?}");
                assert!(p.seep > 0.0, "{t:?}: {p:?}");
            }
        }
    }
}

/// The texture of every sample of an island is shares (each within 0 and 1, coarse and fine summing to at most 1), and
/// still liquid lays the finest: the floors of the ponds and of the open low ground hold more fine particles than the
/// ground that flowing liquid laid (Hjulström 1935).
#[test]
fn still_liquid_lays_finer_ground_than_flowing_liquid() {
    let w = worldgen::testkit::one_rise(1, 16, 16);
    let (tex, s) = (pcore::soil::samples(&w), w.terrain.samples(w.terrain.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north)));
    assert!(tex.iter().all(|t| (0.0..=1.0).contains(&t.coarse) && (0.0..=1.0).contains(&t.fine) && t.middle() >= -1e-12), "a share out of range");
    let laid = |k: usize| s.loose[k] > 0.0 && s.settled[k] >= s.loose[k];
    let mean = |still: bool| {
        let ks: Vec<usize> = (0..tex.len()).filter(|&k| laid(k) && (s.pond[k] != NONE) == still).collect();
        (ks.iter().map(|&k| tex[k].fine).sum::<f64>() / ks.len().max(1) as f64, ks.len())
    };
    let ((still, n_still), (flowing, n_flowing)) = (mean(true), mean(false));
    println!("fine share laid by still liquid {still:.3} ({n_still} samples), by flowing liquid {flowing:.3} ({n_flowing})");
    assert!(n_still > 0 && n_flowing > 0 && still > flowing, "still {still}, flowing {flowing}");
}

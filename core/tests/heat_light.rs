//! Heat radiation through the air (laws.md 1.4): its bands' shares, a column's books, and chemistry 0's anchors.

use pcore::chem::Chemistry;
use pcore::flow::{bands, scale_height, Heat, DOWN, UP};
use pcore::units::{C2, SIGMA};
use worldgen::testkit::{ABSORBER, LIQUID};

/// The air of the anchors: a column of 10 km cooling by 6.5 K a km, of mean molar mass 0.029 kg/mol under 9.81 m/s².
const GAM: f64 = 6.5e-3;
const TOP: f64 = 1e4;
const AIR_KG_PER_MOL: f64 = 0.029;
const GRAVITY: f64 = 9.81;

/// The share of a black body's light at `t` K between `lo` and `hi` µm, by Planck's law summed in steps of 1 nm.
fn planck_share(lo: f64, hi: f64, t: f64) -> f64 {
    let b = |um: f64| um.powi(-5) / ((C2 / (um * 1e-6 * t)).exp() - 1.0);
    let (step, mut sum) = (1e-3, 0.0);
    let mut um = lo + step / 2.0;
    while um < hi {
        sum += b(um) * step;
        um += step;
    }
    // ∫ B dλ over all λ in these units is π⁴ / (15 a⁴), a = C2 / T in µm.
    sum * 15.0 * (C2 / (t * 1e-6)).powi(4) / std::f64::consts::PI.powi(4)
}

/// The bands' shares of a body's heat radiation sum to 1 at every temperature the world meets, and each is Planck's
/// share of its span of wavelengths.
#[test]
fn the_bands_share_all_the_light_as_planck_says() {
    let (chem, _) = worldgen::chemistry0();
    for t in (50..=400).step_by(10).map(f64::from) {
        let s = chem.heat_shares(t);
        assert!((s.iter().sum::<f64>() - 1.0).abs() < 1e-12 && s.iter().all(|&x| (0.0..=1.0).contains(&x)), "{t} K: {s:?}");
    }
    for t in [200.0, 255.0, 288.0, 330.0] {
        let s = chem.heat_shares(t);
        for (b, span) in chem.heat_spans.iter().enumerate() {
            let want = planck_share(span[0], span[1], t);
            assert!((s[b] - want).abs() < 1e-4, "{t} K band {b}: {} against Planck's {want}", s[b]);
        }
    }
}

/// A column at one temperature over a ground at the same temperature closes its books: it sends down what it absorbs
/// of the ground's light (Kirchhoff), and what leaves it upward is the ground's black-body light again.
#[test]
fn an_even_column_sends_what_it_takes() {
    let (chem, _) = worldgen::chemistry0();
    let heat = Heat::of([(0.3, 2000.0), (2.0, 8000.0), (40.0, 1500.0)], (0.0, TOP), 4.0);
    for t in [220.0_f64, 288.0, 320.0] {
        let (black, f) = (SIGMA * t.powi(4), heat.absorbs(&chem, t));
        let (down, up) = (heat.glow(&chem, t, DOWN)[0], heat.glow(&chem, t, UP)[0]);
        assert!((down - f * black).abs() < 1e-12 * black, "{t} K: down {down} against {}", f * black);
        assert!(((1.0 - f) * black + up - black).abs() < 1e-12 * black, "{t} K: up {} against {black}", (1.0 - f) * black + up);
    }
}

/// The ground's air at `t` K with `w` cm of the liquid's gas (its own height 2 km) and `ppm` of the absorber: the
/// share of the ground's black-body light it sends down, and what leaves it to space, W/m².
fn clear_sky(chem: &Chemistry, t: f64, w: f64, ppm: f64) -> (f64, f64) {
    let big = scale_height(t, 1.0, AIR_KG_PER_MOL, GRAVITY);
    let air_mol = 101_325.0 / (GRAVITY * AIR_KG_PER_MOL);
    let gases = [(LIQUID, w * 10.0 / chem.subs[LIQUID].molar_mass, 2000.0), (ABSORBER, ppm * 1e-6 * air_mol, big)];
    let heat = Heat::of(bands(chem, &gases, (t, big), 0.0), (GAM, TOP), 4.0);
    let black = SIGMA * t.powi(4);
    (heat.glow(chem, t, DOWN)[0] / black, (1.0 - heat.absorbs(chem, t)) * black + heat.glow(chem, t, UP)[0])
}

/// Chemistry 0's air meets its measured anchors: a clear sky of 2.5 cm of the liquid's gas at 288 K sends down the
/// share of the ground's light Prata's (1996) fit of soundings gives, within 0.02 of it; doubling the absorber from 400
/// ppm forces the outgoing light by 3.7 W/m² (Myhre et al. 1998) within a fifth.
#[test]
fn chemistry_0_sends_down_and_holds_what_is_measured() {
    let (chem, _) = worldgen::chemistry0();
    let prata = |w: f64| 1.0 - (1.0 + w) * (-(1.2 + 3.0 * w).sqrt()).exp();
    let (down, out) = clear_sky(&chem, 288.0, 2.5, 400.0);
    let (_, doubled) = clear_sky(&chem, 288.0, 2.5, 800.0);
    println!("down {down:.3} (Prata {:.3}), out {out:.1} W/m², forcing {:.2} W/m²", prata(2.5), out - doubled);
    assert!((down - prata(2.5)).abs() < 0.02, "down {down} against Prata's {}", prata(2.5));
    assert!(((out - doubled) / 3.7 - 1.0).abs() < 0.2, "forcing {}", out - doubled);
}

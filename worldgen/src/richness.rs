//! Richness checks: physical possibilities a chemistry must offer.
//! They look at numbers only, never at names or fixed substance indices.

use crate::Init;
use pcore::chem::{Chemistry, Phase, Reaction, Substance};
use pcore::units::T_SURF;

/// Smallest mole fraction that counts as abundant.
const ABUNDANT: f64 = 0.01;

/// The rate at which a reaction counts as running (its ignition), 1/s.
pub(crate) const IGNITION_RATE: f64 = 1e-6;
/// The surface temperatures a liquid is looked for at: `WINDOW_STEPS` + 1 of them, `WINDOW_STEP` K apart from
/// `WINDOW_LOW` K (260 to 310 K).
const WINDOW_LOW: f64 = 260.0;
const WINDOW_STEP: f64 = 5.0;
const WINDOW_STEPS: usize = 10;

/// Temperature where a reaction runs at `IGNITION_RATE`: `k exp(-ea/T) = IGNITION_RATE`.
pub fn ignition(rx: &Reaction) -> f64 {
    let l = (rx.k / IGNITION_RATE).ln();
    if l > 0.0 {
        rx.ea / l
    } else {
        f64::INFINITY
    }
}

/// At how many of the surface temperatures of the window (`WINDOW_LOW`) substance `s` is liquid.
pub(crate) fn liquid_temps(s: &Substance) -> usize {
    (0..=WINDOW_STEPS).filter(|i| (s.melt..s.boil).contains(&(WINDOW_LOW + WINDOW_STEP * *i as f64))).count()
}

pub fn richness(chem: &Chemistry, init: &Init) -> Vec<(&'static str, bool)> {
    let at = |k: usize| chem.phase(k, T_SURF);
    let hard = |k: u16| chem.subs[k as usize].hardness;
    let ground: Vec<usize> = [&init.bed, &init.loose, &init.deposits].iter().flat_map(|v| v.iter().map(|p| p.0)).collect();
    let air: Vec<usize> = init.air.iter().filter(|p| p.1 >= ABUNDANT && at(p.0) == Phase::Gas).map(|p| p.0).collect();
    let mut all: Vec<usize> = ground.clone();
    all.extend(init.air.iter().map(|p| p.0));
    all.push(init.low);
    let in_air = |k: u16| air.contains(&(k as usize));

    let liquid_often = all.iter().any(|&k| liquid_temps(&chem.subs[k]) > WINDOW_STEPS / 2);
    let greenhouse = air.len() >= 2 && init.air.iter().any(|&(k, _)| at(k) == Phase::Gas && chem.subs[k].ir_abs > 0.0);
    let stores_light = chem.reactions.iter().enumerate().any(|(r, rx)| {
        rx.light
            && chem.dh(r) > 0.0
            && rx.inputs.iter().all(|&(k, _)| all.contains(&(k as usize)))
            && rx.outputs.iter().any(|&(k, _)| at(k as usize) != Phase::Gas)
    });
    // Burning: exothermic, takes an abundant gas plus another substance, needs heat to start.
    let burns: Vec<&Reaction> = chem
        .reactions
        .iter()
        .enumerate()
        .filter(|&(r, rx)| {
            let t = ignition(rx);
            !rx.light && chem.dh(r) < -50e3 && (330.0..1500.0).contains(&t) && rx.inputs.iter().any(|&(k, _)| in_air(k))
        })
        .map(|(_, rx)| rx)
        .collect();
    let fuels: Vec<u16> = burns.iter().flat_map(|rx| rx.inputs.iter().map(|p| p.0)).filter(|&k| !in_air(k)).collect();
    // A fuel or what its partial burning leaves can free a harder solid from the ground.
    let mut reducers = fuels.clone();
    reducers.extend(burns.iter().flat_map(|rx| rx.outputs.iter().map(|p| p.0)).filter(|&k| !in_air(k)));
    let frees_harder = chem.reactions.iter().any(|rx| {
        let solid = |k: u16| at(k as usize) == Phase::Solid;
        let t = ignition(rx);
        !rx.light
            && (450.0..3000.0).contains(&t)
            && rx.inputs.iter().any(|&(g, _)| {
                ground.contains(&(g as usize))
                    && solid(g)
                    && rx.inputs.iter().any(|&(f, _)| f != g && reducers.contains(&f))
                    && rx.outputs.iter().any(|&(h, _)| solid(h) && hard(h) >= 0.3 && hard(h) > hard(g) + 0.1)
            })
    });
    let hard_mix = chem.reactions.iter().any(|rx| {
        let hs: Vec<u16> = rx.inputs.iter().map(|p| p.0).filter(|&k| at(k as usize) == Phase::Solid && hard(k) >= 0.1).collect();
        let top = hs.iter().map(|&k| hard(k)).fold(0.0, f64::max);
        !rx.light && hs.len() >= 2 && rx.outputs.iter().any(|&(k, _)| at(k as usize) == Phase::Solid && hard(k) > top + 0.05)
    });
    let span = |f: fn(&pcore::chem::Substance) -> f64| {
        let v: Vec<f64> = chem.subs.iter().map(f).collect();
        v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
    };
    let solids = (0..chem.ns()).filter(|&k| at(k) == Phase::Solid).count();
    let wide = span(|s| s.melt) >= 1000.0 && span(|s| s.strength) >= 0.6 && solids >= 4;

    vec![
        ("common liquid across surface temperatures", liquid_often),
        ("gas mixture with a heat-absorbing gas", greenhouse),
        ("light stored in a solid or liquid", stores_light),
        ("energy-rich substances burn with an abundant gas", !fuels.is_empty()),
        ("ground solid frees a harder solid with fuel", frees_harder),
        ("hard solids mix into a harder solid", hard_mix),
        ("wide range of melting points and bond strengths", wide),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemistry_0_is_rich() {
        let (c, i) = crate::chemistry0();
        for (name, ok) in richness(&c, &i) {
            assert!(ok, "chemistry 0 fails: {name}");
        }
    }

    /// Every generated chemistry is valid, and the generator makes rich ones: some of them pass every check.
    #[test]
    fn generated_chemistries_validate_and_some_are_rich() {
        let (mut all, mut each) = (0, Vec::new());
        for seed in 0..200 {
            let (c, i) = crate::generate(seed);
            c.validate().unwrap();
            let checks = richness(&c, &i);
            each.resize(checks.len(), 0);
            checks.iter().zip(&mut each).for_each(|(p, n)| *n += p.1 as u32);
            all += checks.iter().all(|p| p.1) as u32;
        }
        println!("of 200 generated chemistries, {all} pass all richness checks; each check: {each:?}");
        assert!(all > 0, "no generated chemistry passes every check; each check: {each:?}");
    }
}

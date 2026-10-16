//! Richness checks (handover 6.0): physical possibilities a chemistry must offer.
//! They look at numbers only, never at names or fixed substance indices.

use crate::Init;
use pcore::chem::{Chemistry, Phase, Reaction};
use pcore::units::T_SURF;

/// Smallest mole fraction that counts as abundant.
const ABUNDANT: f64 = 0.01;

/// Temperature where a reaction runs at 1e-6 per second: `k exp(-ea/T) = 1e-6`.
pub fn ignition(rx: &Reaction) -> f64 {
    let l = (rx.k / 1e-6).ln();
    if l > 0.0 {
        rx.ea / l
    } else {
        f64::INFINITY
    }
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

    let liquid_often = all.iter().any(|&k| (0..=10).filter(|i| chem.phase(k, 260.0 + 5.0 * *i as f64) == Phase::Liquid).count() > 5);
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

    #[test]
    fn generated_chemistries_validate() {
        let mut pass = 0;
        for seed in 0..200 {
            let (c, i) = crate::generate(seed);
            c.validate().unwrap();
            pass += richness(&c, &i).iter().all(|p| p.1) as u32;
        }
        println!("generated chemistries passing all richness checks: {pass} of 200");
    }
}

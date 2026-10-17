//! Random chemistry: element-count vectors, properties derived from the elements plus noise,
//! and reactions that balance by construction (two different ways to make the same element totals).

use crate::Init;
use pcore::chem::{Chemistry, Reaction, Substance};
use pcore::rng::{Purpose, Rng};
use pcore::units::R_GAS;
use std::collections::BTreeMap;

type Side = Vec<(u16, u16)>;

fn noise(r: &mut Rng, s: f64) -> f64 {
    (s * r.normal()).exp()
}

fn shuffle<T>(r: &mut Rng, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        v.swap(i, r.below(i + 1));
    }
}

pub fn generate(seed: u64) -> (Chemistry, Init) {
    let mut r = Rng::new(seed, 0, 0, Purpose::Chem);
    let ne = 7 + r.below(2);
    let mass: Vec<f64> = (0..ne).map(|i| if i == 0 { 1.0 + 3.0 * r.f64() } else { 6.0 + 50.0 * r.f64() }).collect();
    let bond: Vec<f64> = (0..ne).map(|_| 0.3 + 0.7 * r.f64()).collect();
    let pull: Vec<f64> = (0..ne).map(|_| r.f64()).collect();

    // Each element alone, then sums of two earlier substances.
    let mut elems: Vec<Vec<u16>> = (0..ne).map(|i| (0..ne).map(|j| if i == j { 1 + r.below(2) as u16 } else { 0 }).collect()).collect();
    let want = 16 + r.below(33);
    for _ in 0..4000 {
        if elems.len() >= want {
            break;
        }
        let (a, b) = (r.below(elems.len()), r.below(elems.len()));
        let (ca, cb) = (1 + r.below(3) as u16, 1 + r.below(3) as u16);
        let v: Vec<u16> = (0..ne).map(|j| ca * elems[a][j] + cb * elems[b][j]).collect();
        if a != b && v.iter().sum::<u16>() <= 24 && !elems.contains(&v) {
            elems.push(v);
        }
    }

    let subs: Vec<Substance> = elems
        .iter()
        .map(|e| {
            let sum = |p: &[f64]| e.iter().zip(p).map(|(&c, &x)| c as f64 * x).sum::<f64>();
            let n = e.iter().sum::<u16>() as f64;
            let used: Vec<f64> = (0..ne).filter(|&j| e[j] > 0).map(|j| pull[j]).collect();
            let spread = used.iter().cloned().fold(0.0, f64::max) - used.iter().cloned().fold(1.0, f64::min);
            let coh = sum(&bond) / (n + 3.0);
            let melt = 30.0 + 2200.0 * coh * coh * noise(&mut r, 0.25);
            let u = r.f64();
            let boil = melt * (1.1 + 1.2 * u * u);
            let strength = (coh * 1.1 * noise(&mut r, 0.15)).min(1.0);
            let rho = 500.0 + 8000.0 * coh * noise(&mut r, 0.2);
            let bright = 0.1 + 0.8 * r.f64();
            let band = |r: &mut Rng, f: f64| [0; 3].map(|_| (bright * f * (0.8 + 0.4 * r.f64())) as f32);
            Substance {
                elems: e.clone(),
                molar_mass: sum(&mass) / 1000.0,
                cp: (21.0 + 4.0 * n).min(150.0),
                melt,
                boil,
                density: [rho, rho * 0.85],
                hardness: (strength * (0.5 + 0.7 * r.f64())).min(1.0),
                strength,
                refl: [band(&mut r, 1.0), band(&mut r, 0.3)],
                ir_abs: if boil < 290.0 && used.len() >= 2 && n >= 3.0 { 10f64.powf(-3.0 + 2.0 * r.f64()) } else { 0.0 },
                ir_sat: 0.0,
                solubility: r.f64().powi(3),
                conductivity: if melt > 290.0 { 0.5 + 60.0 * coh * coh } else { 0.03 },
                hf: -120e3 * n * spread * (0.4 + 0.7 * r.f64()),
                lv: 88.0 * boil * (0.9 + 0.4 * r.f64()),
                lf: 9.5 * melt,
                modulus: 0.0,
                rupture: 0.0,
            }
        })
        .collect();
    let ns = subs.len();

    // Starting world: the lightest gases make the air, the best liquid fills low ground, solids make the ground.
    let gas = |k: usize| subs[k].boil < 290.0;
    let mut gases: Vec<usize> = (0..ns).filter(|&k| gas(k)).collect();
    shuffle(&mut r, &mut gases);
    let mut air: Vec<(usize, f64)> = gases.iter().take(2).zip([0.78, 0.21]).map(|(&k, f)| (k, f)).collect();
    if let Some(&k) = gases.iter().find(|&&k| subs[k].ir_abs > 0.0 && !air.iter().any(|p| p.0 == k)) {
        air.push((k, 4e-4));
    }
    let liquid_temps = |k: usize| (0..=10).filter(|i| (subs[k].melt..subs[k].boil).contains(&(260.0 + 5.0 * *i as f64))).count();
    let low = (0..ns).filter(|k| !air.iter().any(|p| p.0 == *k)).max_by_key(|&k| (liquid_temps(k), ns - k)).unwrap();
    air.push((low, 0.004));
    let mut solids: Vec<usize> = (0..ns).filter(|&k| subs[k].melt > 320.0).collect();
    if solids.is_empty() {
        solids = (0..ns).collect();
    }
    shuffle(&mut r, &mut solids);
    let mut pick = |fr: &[f64]| -> Vec<(usize, f64)> {
        let v = fr.iter().map(|&f| (solids[r.below(solids.len())], f)).collect();
        v
    };
    let init = Init {
        air,
        low,
        bed: pick(&[0.7, 0.2, 0.1]),
        loose: pick(&[0.5, 0.4, 0.08, 0.02]),
        deposits: pick(&[0.03, 0.01, 0.005, 0.02]),
        soil: Vec::new(),
    };
    let abundant: Vec<usize> = [&init.bed, &init.loose, &init.deposits, &init.air].iter().flat_map(|v| v.iter().map(|p| p.0)).chain([low]).collect();

    // Every one- or two-substance side, keyed by its element totals: equal keys balance.
    let mut sides: BTreeMap<Vec<u32>, Vec<Side>> = BTreeMap::new();
    let mut add = |s: Side| {
        let mut key = vec![0u32; ne];
        for &(k, c) in &s {
            for j in 0..ne {
                key[j] += c as u32 * elems[k as usize][j] as u32;
            }
        }
        sides.entry(key).or_default().push(s);
    };
    for a in 0..ns as u16 {
        for ca in 1..=3 {
            add(vec![(a, ca)]);
            for b in a + 1..ns as u16 {
                for cb in 1..=3 {
                    add(vec![(a, ca), (b, cb)]);
                }
            }
        }
    }
    let mut pool: Vec<(Side, Side)> = Vec::new();
    for v in sides.values().filter(|v| v.len() >= 2) {
        for (i, a) in v.iter().enumerate() {
            for b in &v[i + 1..] {
                if !a.iter().any(|x| b.iter().any(|y| x.0 == y.0)) {
                    pool.push((a.clone(), b.clone()));
                    pool.push((b.clone(), a.clone()));
                }
            }
        }
    }
    shuffle(&mut r, &mut pool);

    // Keep a few light reactions that store energy, a few burning ones, and the rest as they come.
    let total = 18 + r.below(10);
    let hf = |s: &Side| s.iter().map(|&(k, c)| c as f64 * subs[k as usize].hf).sum::<f64>();
    let (mut lights, mut burns) = (0, 0);
    let mut reactions = Vec::new();
    for (a, b) in pool {
        if reactions.len() >= total {
            break;
        }
        let dh = hf(&b) - hf(&a);
        let abundant_in = a.iter().all(|p| abundant.contains(&(p.0 as usize)));
        let air_in = a.iter().any(|p| gas(p.0 as usize) && init.air.iter().any(|q| q.0 == p.0 as usize));
        let light = lights < 3 && dh > 0.0 && abundant_in && b.iter().any(|p| !gas(p.0 as usize));
        let burn = !light && burns < 6 && dh < -50e3 && air_in;
        if !light && !burn && reactions.len() + 9 - lights - burns >= total {
            continue;
        }
        lights += light as usize;
        burns += burn as usize;
        let k = 10f64.powf(3.0 + 5.0 * r.f64());
        let top = if burn { 700.0 } else { 1500.0 };
        let t_start = 340.0 * (top / 340.0f64).powf(r.f64());
        let (k, ea) = if light { (1e-9, 5800.0 * noise(&mut r, 0.1)) } else { (k, t_start * (k / 1e-6).ln() + dh.max(0.0) / R_GAS) };
        reactions.push(Reaction { inputs: a, outputs: b, k, ea, light, ord: Vec::new() });
    }
    // (No growth in a random chemistry yet: its bodies are their mass, `Chemistry::build`.)
    let chem = Chemistry::new(ne, subs, reactions, None, Vec::new(), &init.air);
    debug_assert!(chem.validate().is_ok());
    (chem, init)
}

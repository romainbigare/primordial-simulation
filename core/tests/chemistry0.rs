//! Chemistry 0 as its numbers say: what body matter is made of and burns for, the minerals, fixing, the reversible
//! pair that stores the heat-absorbing gas in the liquid, and what the liquid freezes at.

use pcore::change::reversible;
use pcore::chem::Phase;
use pcore::units::T_SURF;
use worldgen::testkit::{ABSORBER, DISSOLVED, DISSOLVING, FIX_GAS, LIQUID, MATTER, MINERAL, RICH, ROCK, ROCK_ELEMENT, SETTLING};

/// Elements of chemistry 0: the one in the heat-absorbing gas and the energy-rich solid, and the fixed gas's.
const SOLID_ELEMENT: usize = 1;
const FIX_ELEMENT: usize = 2;

/// What the chemistry says: body matter holds one atom of the fixed gas's element to 20 of the solid's (118 and 6 in
/// its formula unit) and none of the rock element; it is made of 118 of the energy-rich solid and 3 of the fixed
/// mineral without a gain or loss of heat, and burns for 16.7 MJ a kg giving the fixed gas's element back as its
/// mineral; the minerals live dissolved in the liquid; fixing the gas burns what real fixers respire for it
/// and gives all the solid's element back as the heat-absorbing gas; every reaction and growth balance in elements.
#[test]
fn the_chemistry_balances_and_fixing_costs_what_fixers_respire() {
    let (chem, _) = worldgen::chemistry0();
    assert!(chem.validate().is_ok());
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    assert_eq!((m, minerals.to_vec()), (MATTER, vec![(MINERAL, 3.0)]));
    let elems = &chem.subs[m].elems;
    assert_eq!((elems[SOLID_ELEMENT], elems[FIX_ELEMENT], elems[ROCK_ELEMENT]), (118, 6, 0));
    // Growth: 118 of the energy-rich solid and 3 of the fixed mineral make 1 of matter and 26 of the liquid, with no
    // gain or loss of heat in a body (its liquids hold their fusion enthalpy); nothing goes to the air.
    let build = chem.build.as_ref().expect("growth");
    assert_eq!((build.inputs.as_slice(), build.outputs.as_slice()), (&[(RICH as u16, 118), (MINERAL as u16, 3)][..], &[(MATTER as u16, 1), (LIQUID as u16, 26)][..]));
    let fused = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * chem.fused_heat(k as usize, T_SURF, |_| 0.0)).sum::<f64>();
    let growth = chem.dh_of(build) + fused(&build.outputs) - fused(&build.inputs);
    assert!(growth.abs() < 1.0, "growth takes {growth} J");
    assert!(build.outputs.iter().all(|p| p.0 == m as u16 || chem.phase(p.0 as usize, 290.0) == Phase::Liquid));
    // Rotting: 55,165.0 kJ a mol (16.7 MJ a kg), from and to the states found at the surface (`Chemistry::dh_at`).
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m) && chem.reactions[r].outputs.iter().any(|p| p.0 as usize == MINERAL)).expect("rotting");
    let heat = -chem.dh_at(burn, T_SURF);
    assert!((heat / 55165.0e3 - 1.0).abs() < 1e-6, "body matter burns for {heat} J a mol");
    let per_kg = heat / chem.subs[m].molar_mass / 1e6;
    println!("body matter burns for {per_kg:.2} MJ/kg");
    assert!(per_kg > 15.5 && per_kg < 17.0, "{per_kg} MJ/kg");
    // Rotting gives back the fixed mineral (3 a mol), and neither the rock nor the dissolved mineral; it takes only the
    // matter and gases.
    let out = |k: usize| chem.reactions[burn].outputs.iter().find(|p| p.0 as usize == k).map_or(0, |p| p.1);
    assert!(out(MINERAL) == 3 && out(ROCK) == 0 && out(DISSOLVED) == 0);
    assert!(chem.reactions[burn].inputs.iter().all(|p| p.0 as usize == m || chem.phase(p.0 as usize, 290.0) == Phase::Gas));
    // The dissolved mineral lives in the liquid and never boils; only the reaction that undoes its making takes it.
    assert!(chem.phase(DISSOLVED, 290.0) == Phase::Liquid && chem.subs[DISSOLVED].boil > 2000.0);
    let takes: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == DISSOLVED)).collect();
    assert!(takes.len() == 1 && chem.dh(takes[0]) > 0.0 && chem.reactions[takes[0]].outputs.iter().any(|p| p.0 as usize == ROCK));
    // The fixing reaction: the fixed gas, the energy-rich solid and the burning gas make the mineral, the
    // heat-absorbing gas and the liquid, nothing else, so all the solid's element it burns goes back to the air as the
    // gas. It burns the units of the solid that real fixers respire for it: 4.7 to 6.6 g of the solid's element a g of
    // the fixed gas's (Layzell et al. 1979; models take 6). It keeps in the mineral the mineral's own formation energy
    // from the gas, the liquid and the burning gas (about 200 kJ a mol of the gas); the energy the enzyme spends is
    // respired as heat. It releases heat, but less from each unit of the solid than burning does: it keeps a part
    // (`Chemistry::keeps`), so it is no burning and a body never pays its upkeep with it.
    let fix = chem.fixing().expect("fixing");
    let (_, rich, _, burn1) = chem.life_roles().expect("roles");
    assert_eq!(rich, RICH);
    let rx = &chem.reactions[fix];
    assert!(rx.outputs.iter().all(|p| [MINERAL, ABSORBER, LIQUID].contains(&(p.0 as usize))), "fixing makes {:?}", rx.outputs);
    let units = |r: usize| chem.reactions[r].inputs.iter().find(|p| p.0 as usize == rich).map_or(0.0, |p| p.1 as f64);
    let gas = rx.inputs.iter().find(|p| p.0 as usize == FIX_GAS).expect("the fixed gas").1 as f64;
    let (unit, n, dh_fix) = (-chem.dh_at(burn1, T_SURF) / units(burn1), units(fix) / gas, chem.dh_at(fix, T_SURF) / gas);
    let ratio = n * 12.0 / 28.0;
    println!("fixing burns {n} units of the solid a mol of the gas ({ratio:.1} g a g), {:.1} kJ; it keeps {:.0} kJ of their {:.1} kJ each", dh_fix / 1e3, (dh_fix + n * unit) / 1e3, unit / 1e3);
    assert!((4.7..=6.6).contains(&ratio), "{ratio} g of the solid's element a g of the fixed gas's");
    assert!(chem.dh(fix) < 0.0 && chem.keeps(fix) && !chem.keeps(burn1) && !chem.keeps(burn));
    // The fixed mineral lives dissolved in the liquid and never boils; nothing burns it in a gas, and hot it breaks down
    // into the fixed gas (releasing heat).
    assert!(chem.phase(MINERAL, 290.0) == Phase::Liquid && chem.subs[MINERAL].boil > 2000.0, "a mineral that lives in the liquid and never boils");
    let uses: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == MINERAL)).collect();
    assert_eq!(uses.len(), 1);
    assert!(chem.dh(uses[0]) < 0.0 && chem.reactions[uses[0]].outputs.iter().any(|p| p.0 as usize == FIX_GAS) && !chem.reactions[uses[0]].inputs.iter().any(|p| chem.phase(p.0 as usize, 290.0) == Phase::Gas));
    assert!(chem.reactions[uses[0]].ea > 15000.0);
}

/// The reversible pair (the rock, the heat-absorbing gas and the liquid to the dissolved mineral, and back) is found
/// from the chemistry alone.
#[test]
fn the_pair_is_found_from_the_chemistry_alone() {
    let (chem, _) = worldgen::chemistry0();
    assert!(reversible(&chem).contains(&(DISSOLVING, SETTLING, ABSORBER as u16, DISSOLVED)), "{:?}", reversible(&chem));
}

/// The liquid freezes lower by what is dissolved in it (1.86 K for a mol of the mineral in a kg, the measured cryoscopic
/// constant), not by a gas it holds; where there is none of it, it melts at its own melting point.
#[test]
fn the_liquid_freezes_lower_by_what_is_dissolved_in_it() {
    let (chem, _) = worldgen::chemistry0();
    let melt = chem.subs[LIQUID].melt;
    let mut a = vec![0.0; chem.ns()];
    (a[LIQUID], a[DISSOLVED]) = (1.0 / chem.subs[LIQUID].molar_mass, 1.0);
    let salty = chem.melt_in(LIQUID, |k| a[k]);
    assert!((1.7..2.0).contains(&(melt - salty)), "{} K lower", melt - salty);
    a[ABSORBER] = 10.0;
    assert_eq!(chem.melt_in(LIQUID, |k| a[k]), salty);
    a[LIQUID] = 0.0;
    assert_eq!(chem.melt_in(LIQUID, |k| a[k]), melt);
}

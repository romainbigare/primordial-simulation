//! Chemistry 0 as the numbers say: what body matter is made of and burns for, the minerals, fixing, the reversible pair.

use pcore::change::reversible;
use worldgen::testkit::DISSOLVED;

/// What the chemistry says: the matter holds one nitrogen to 20 carbon (120 and 6 in its formula unit, three times the unit it was) and
/// one atom of the rock element, 1.16% of its mass, in the range of the dry matter of land plants (0.5 to 2%); it is made of 118 of the
/// energy-rich solid, 3 of the nitrogen mineral and 1 of the rock salt without a gain or loss of heat, and burns for about what it
/// did (15.9 MJ a kg; 16.7 before the rock salt) giving the nitrogen back as the mineral and the rock element as lime; the minerals dissolve
/// in the liquid; fixing the air's nitrogen takes 490 kJ a mol of the gas, what 16 molecules of the carrier of the enzyme's energy
/// (30.5 kJ each) hold; every reaction and growth balance in elements.
#[test]
fn the_chemistry_balances_and_fixing_costs_what_the_enzyme_spends() {
    let (chem, _) = worldgen::chemistry0();
    assert!(chem.validate().is_ok());
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    let (mineral, salt, lime) = (24, 23, 10);
    assert_eq!((m, minerals.clone()), (22, vec![(mineral, 3.0), (salt, 1.0)]));
    // Carbon (element 1) to nitrogen (element 2) of the matter: 120 to 6; the rock element (5): one atom.
    assert_eq!((chem.subs[m].elems[1], chem.subs[m].elems[2], chem.subs[m].elems[5]), (120, 6, 1));
    // Its mass share, the rock element's mass found from the lime burnt (substance 11, 56 g a mol) less its oxygen (16).
    let rock = chem.subs[11].molar_mass - chem.subs[2].molar_mass / 2.0;
    let share = rock / chem.subs[m].molar_mass;
    println!("the rock element is {:.2}% of the dry matter ({} g of {} g)", 100.0 * share, 1e3 * rock, 1e3 * chem.subs[m].molar_mass);
    assert!(share > 0.005 && share < 0.02, "{share}");
    let growth = chem.dh_of(chem.build.as_ref().unwrap());
    assert!(growth.abs() < 1.0, "growth takes {growth} J");
    // Growth gives off only the liquid (the salt's carbon goes into the matter): nothing to the air.
    assert!(chem.build.as_ref().expect("growth").outputs.iter().all(|p| p.0 == m as u16 || chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Liquid));
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m) && chem.reactions[r].outputs.iter().any(|p| p.0 as usize == mineral)).expect("rotting");
    let per_kg = -chem.dh(burn) / chem.subs[m].molar_mass / 1e6;
    println!("body matter burns for {per_kg:.2} MJ/kg");
    assert!(per_kg > 15.5 && per_kg < 17.0, "{per_kg} MJ/kg");
    // Rotting gives back the mineral for the nitrogen (3 a mol) and the lime for the rock element (1 a mol), and no salt (no liquid goes in).
    let out = |k: usize| chem.reactions[burn].outputs.iter().find(|p| p.0 as usize == k).map_or(0, |p| p.1);
    assert!(out(mineral) == 3 && out(lime) == 1 && out(salt) == 0);
    assert!(chem.reactions[burn].inputs.iter().all(|p| p.0 as usize == m || chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Gas));
    // The rock salt lives dissolved in the liquid and never boils; only the reaction that undoes its making takes it.
    assert!(chem.phase(salt, 290.0) == pcore::chem::Phase::Liquid && chem.subs[salt].boil > 2000.0);
    let takes: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == salt)).collect();
    assert!(takes.len() == 1 && chem.dh(takes[0]) > 0.0 && chem.reactions[takes[0]].outputs.iter().any(|p| p.0 as usize == lime));
    // The fixing reaction: air's nitrogen gas (1), the energy-rich solid and the oxygen-like gas -> the mineral, the reduced carbon gas,
    // the carbon gas and the liquid. It is the reaction that takes 490 kJ a mol of the gas (16 x 30.5, the 16 ATP of nitrogenase)
    // together with the burning of the units of the solid that real fixers respire for it: 4.7 to 6.6 g of carbon a g of nitrogen
    // (Layzell et al. 1979; models take 6). It
    // releases heat, but less from each unit of the solid than burning does: it keeps a part (`Chemistry::keeps`), so it is no burning
    // and a body never pays its upkeep with it; burning itself is a burning.
    let fix = chem.fixing().expect("fixing");
    let (_, rich, _, burn1) = chem.life_roles().expect("roles");
    let units = |r: usize| chem.reactions[r].inputs.iter().find(|p| p.0 as usize == rich).map_or(0.0, |p| p.1 as f64);
    let (unit, n) = (-chem.dh(burn1) / units(burn1), units(fix));
    let carbon = n * 12.0 / 28.0;
    println!("fixing burns {n} units of the solid a mol of nitrogen gas ({carbon:.1} g of carbon a g of nitrogen), {:.1} kJ; less the {:.1} kJ of each unit, {:.0}: 16 x 30.5 = {:.0}", chem.dh(fix) / 1e3, unit / 1e3, (chem.dh(fix) + n * unit) / 1e3, 16.0 * 30.5);
    assert!((4.7..=6.6).contains(&carbon), "{carbon} g of carbon a g of nitrogen");
    assert!(((chem.dh(fix) + n * unit) / (16.0 * 30.5e3) - 1.0).abs() < 0.01, "{} J", chem.dh(fix) + n * unit);
    assert!(chem.dh(fix) < 0.0 && chem.keeps(fix) && !chem.keeps(burn1) && !chem.keeps(burn));
    // The mineral is a salt that lives dissolved in the liquid (a liquid that never boils); nothing burns it in a gas, and hot it breaks down into the air's nitrogen gas (exothermic).
    assert!(chem.phase(mineral, 290.0) == pcore::chem::Phase::Liquid && chem.subs[mineral].boil > 2000.0, "a salt that lives in the liquid and never boils");
    let uses: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == mineral)).collect();
    assert_eq!(uses.len(), 1);
    assert!(chem.dh(uses[0]) < 0.0 && chem.reactions[uses[0]].outputs.iter().any(|p| p.0 == 1) && !chem.reactions[uses[0]].inputs.iter().any(|p| chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Gas));
    assert!(chem.reactions[uses[0]].ea > 15000.0);
}

#[test]
fn the_pair_is_found_from_the_chemistry_alone() {
    let (chem, _) = worldgen::chemistry0();
    assert!(reversible(&chem).contains(&(22, 23, 3, DISSOLVED)), "{:?}", reversible(&chem));
}

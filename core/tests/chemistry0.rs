//! Chemistry 0 as its numbers say: what body matter is made of and burns for, the minerals, fixing, the reversible
//! pair that stores the heat-absorbing gas in the liquid, and what the liquid freezes at.

use pcore::change::reversible;
use pcore::chem::Phase;
use pcore::units::T_SURF;
use worldgen::testkit::{ABSORBER, BURNT_ROCK, DISSOLVED, FIX_GAS, LIQUID, MATTER, MINERAL, OX, RICH, ROCK};

/// Elements of chemistry 0: the one in the heat-absorbing gas and the energy-rich solid, the fixed gas's, and the rock's.
const SOLID_ELEMENT: usize = 1;
const FIX_ELEMENT: usize = 2;
const ROCK_ELEMENT: usize = 5;
/// The reactions that dissolve the rock with the heat-absorbing gas and the liquid, and settle it back.
const DISSOLVING: usize = 22;
const SETTLING: usize = 23;

/// What the chemistry says: body matter holds one atom of the fixed gas's element to 20 of the solid's (120 and 6 in
/// its formula unit) and one atom of the rock element, 1.16% of its mass, in the range of the dry matter of land plants
/// (0.5 to 2%); it is made of 118 of the energy-rich solid, 3 of the fixed mineral and 1 of the dissolved one without
/// a gain or loss of heat, and burns for 16.0 MJ a kg giving the fixed gas's element back as its mineral and the rock
/// element as the rock; the minerals live dissolved in the liquid; fixing the gas takes 490 kJ a mol of it, what 16
/// molecules of the carrier of the enzyme's energy (30.5 kJ each) hold; every reaction and growth balance in elements.
#[test]
fn the_chemistry_balances_and_fixing_costs_what_the_enzyme_spends() {
    let (chem, _) = worldgen::chemistry0();
    assert!(chem.validate().is_ok());
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    assert_eq!((m, minerals.clone()), (MATTER, vec![(MINERAL, 3.0), (DISSOLVED, 1.0)]));
    let elems = &chem.subs[m].elems;
    assert_eq!((elems[SOLID_ELEMENT], elems[FIX_ELEMENT], elems[ROCK_ELEMENT]), (120, 6, 1));
    // Its mass share: the rock element's mass found from the burnt rock (56 g a mol) less the half of the burning gas
    // it holds (16 g).
    let rock = chem.subs[BURNT_ROCK].molar_mass - chem.subs[OX].molar_mass / 2.0;
    let share = rock / chem.subs[m].molar_mass;
    println!("the rock element is {:.2}% of the dry matter ({} g of {} g)", 100.0 * share, 1e3 * rock, 1e3 * chem.subs[m].molar_mass);
    assert!(share > 0.005 && share < 0.02, "{share}");
    // Growth: 118 of the energy-rich solid, 3 of the fixed mineral and 1 of the dissolved one make 1 of matter and 27 of
    // the liquid, with no gain or loss of heat; nothing goes to the air (the dissolved mineral's gas goes into the matter).
    let build = chem.build.as_ref().expect("growth");
    assert_eq!((build.inputs.as_slice(), build.outputs.as_slice()), (&[(5, 118), (24, 3), (23, 1)][..], &[(22, 1), (0, 27)][..]));
    let growth = chem.dh_of(build);
    assert!(growth.abs() < 1.0, "growth takes {growth} J");
    assert!(build.outputs.iter().all(|p| p.0 == m as u16 || chem.phase(p.0 as usize, 290.0) == Phase::Liquid));
    // Rotting: 55,266.4 kJ a mol (16.0 MJ a kg), from and to the states found at the surface (`Chemistry::dh_at`).
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m) && chem.reactions[r].outputs.iter().any(|p| p.0 as usize == MINERAL)).expect("rotting");
    let heat = -chem.dh_at(burn, T_SURF);
    assert!((heat / 55266.425e3 - 1.0).abs() < 1e-6, "body matter burns for {heat} J a mol");
    let per_kg = heat / chem.subs[m].molar_mass / 1e6;
    println!("body matter burns for {per_kg:.2} MJ/kg");
    assert!(per_kg > 15.5 && per_kg < 17.0, "{per_kg} MJ/kg");
    // Rotting gives back the fixed mineral (3 a mol) and the rock (1 a mol), and none of the dissolved mineral (no liquid
    // goes in); it takes only the matter and gases.
    let out = |k: usize| chem.reactions[burn].outputs.iter().find(|p| p.0 as usize == k).map_or(0, |p| p.1);
    assert!(out(MINERAL) == 3 && out(ROCK) == 1 && out(DISSOLVED) == 0);
    assert!(chem.reactions[burn].inputs.iter().all(|p| p.0 as usize == m || chem.phase(p.0 as usize, 290.0) == Phase::Gas));
    // The dissolved mineral lives in the liquid and never boils; only the reaction that undoes its making takes it.
    assert!(chem.phase(DISSOLVED, 290.0) == Phase::Liquid && chem.subs[DISSOLVED].boil > 2000.0);
    let takes: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == DISSOLVED)).collect();
    assert!(takes.len() == 1 && chem.dh(takes[0]) > 0.0 && chem.reactions[takes[0]].outputs.iter().any(|p| p.0 as usize == ROCK));
    // The fixing reaction: the fixed gas, the energy-rich solid and the burning gas make the mineral, the gas fixing gives
    // off, the heat-absorbing gas and the liquid. It is the reaction that takes 490 kJ a mol of the fixed gas (16 x 30.5,
    // what the enzyme of real fixers spends) together with the burning of the units of the solid that real fixers
    // respire for it: 4.7 to 6.6 g of the solid's element a g of the fixed gas's (Layzell et al. 1979; models take 6). It
    // releases heat, but less from each unit of the solid than burning does: it keeps a part (`Chemistry::keeps`), so it
    // is no burning and a body never pays its upkeep with it.
    let fix = chem.fixing().expect("fixing");
    let (_, rich, _, burn1) = chem.life_roles().expect("roles");
    assert_eq!(rich, RICH);
    let units = |r: usize| chem.reactions[r].inputs.iter().find(|p| p.0 as usize == rich).map_or(0.0, |p| p.1 as f64);
    let (unit, n, dh_fix) = (-chem.dh_at(burn1, T_SURF) / units(burn1), units(fix), chem.dh_at(fix, T_SURF));
    let ratio = n * 12.0 / 28.0;
    println!("fixing burns {n} units of the solid a mol of the gas ({ratio:.1} g a g), {:.1} kJ; less the {:.1} kJ of each unit, {:.0}: 16 x 30.5 = {:.0}", dh_fix / 1e3, unit / 1e3, (dh_fix + n * unit) / 1e3, 16.0 * 30.5);
    assert!((4.7..=6.6).contains(&ratio), "{ratio} g of the solid's element a g of the fixed gas's");
    assert!(((dh_fix + n * unit) / (16.0 * 30.5e3) - 1.0).abs() < 0.01, "{} J", dh_fix + n * unit);
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

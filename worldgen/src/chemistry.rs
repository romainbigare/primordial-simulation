//! The loader of a chemistry file (chemistry 0, `chem0.toml`), and the latent heats where a file gives none.

use pcore::chem::{Chemistry, Reaction, Substance, HEAT_BANDS};
use serde::Deserialize;

/// Trouton's rule: the vaporisation entropy of a liquid at its boiling point, J/(mol K) (lv = TROUTON x boil).
pub(crate) const TROUTON: f64 = 88.0;
/// Richards's rule: the fusion entropy of a solid at its melting point, J/(mol K) (lf = RICHARDS x melt).
pub(crate) const RICHARDS: f64 = 9.5;

#[derive(Deserialize)]
struct File {
    /// g/mol of each element
    element_masses: Vec<f64>,
    /// µm: the bands of heat radiation but the last (`Chemistry::heat_spans`)
    heat_bands: [[f64; 2]; HEAT_BANDS - 1],
    s: Vec<S>,
    r: Vec<R>,
    build: B,
    builds: Vec<B>,
    init: Init,
}

/// A conversion: what bodies make of what they hold (`pcore::chem::Chemistry::build`, `builds`).
#[derive(Deserialize)]
struct B {
    i: Vec<[u16; 2]>,
    o: Vec<[u16; 2]>,
}

#[derive(Deserialize)]
struct S {
    e: Vec<u16>,
    cp: f64,
    melt: f64,
    boil: f64,
    rho: [f64; 2],
    /// K (`Substance::dense`); optional in the file (none: its melting point)
    dense: Option<f64>,
    /// K per g/kg (`Substance::dense_drop`, per kg/kg here); optional in the file (none: 0)
    #[serde(default)]
    dense_drop: f64,
    hard: f64,
    str: f64,
    refl: [[f32; 3]; 2],
    /// m²/mol and mol/m² in each band of heat radiation (`Substance::heat`); m²/(mol Pa) (`Substance::continuum`);
    /// optional in the file (none: 0)
    #[serde(default)]
    heat: [[f64; 2]; HEAT_BANDS],
    #[serde(default)]
    cont: f64,
    /// m²/mol and mol/m² (`Substance::sw_abs`, `sw_sat`), and m²/mol (`Substance::scatter`); optional in the file
    /// (none: 0)
    #[serde(default)]
    sw: f64,
    #[serde(default)]
    swsat: f64,
    #[serde(default)]
    ray: f64,
    sol: f64,
    cond: f64,
    hf: f64,
    /// kJ/mol; none: Trouton's rule (`TROUTON`), and Richards's rule (`RICHARDS`) for `lf`
    lv: Option<f64>,
    lf: Option<f64>,
    /// MPa (`Substance::rupture`); optional in the file (none: 0)
    #[serde(default)]
    rupture: f64,
    /// MPa (`Substance::shear`); optional in the file (none: 0)
    #[serde(default)]
    shear: f64,
    /// Held on the faces of fine particles (`Substance::bound`); optional in the file (none: not)
    #[serde(default)]
    bound: bool,
}

#[derive(Deserialize)]
struct R {
    i: Vec<[u16; 2]>,
    o: Vec<[u16; 2]>,
    k: f64,
    ea: f64,
    /// Driven by light (`Reaction::light`); optional in the file (none: not)
    #[serde(default)]
    light: bool,
    /// How many times faster it runs in the open world (`Reaction::open`); optional in the file (none: 1)
    open: Option<f64>,
    /// The power of each input's share in the rate law (`Reaction::ord`); optional in the file (none: each its count)
    #[serde(default)]
    ord: Vec<u16>,
}

/// How a chemistry fills a new world: lists of (substance, fraction).
#[derive(Clone, Debug, Deserialize)]
pub struct Init {
    /// Mole fractions of the air
    pub air: Vec<(usize, f64)>,
    /// Substance that fills low ground as liquid
    pub low: usize,
    /// Volume fractions of each make of bonded ground (`space::strata`), the first the one the wear's `CUT` is
    /// anchored at
    pub makes: Vec<Vec<(usize, f64)>>,
    /// The makes that lie as seams in the stack (`space::strata::Lies::Seam`): each its make, its thickness, m, and
    /// how many seams of it lie in each repeat of the stack
    pub seams: Vec<(usize, f64, u8)>,
    /// The make round the rims of the bodies (`space::strata::Lies::Rim`)
    pub rim: usize,
    /// Volume fractions of loose ground
    pub loose: Vec<(usize, f64)>,
    /// Amounts in the loose ground of every land cell when life starts, mol per m² (`life::soil`; empty: none)
    pub soil: Vec<(usize, f64)>,
}

/// Molar mass of a substance of element counts `elems`, kg/mol: the sum of its elements' `grams` a mol, so every
/// reaction that keeps elements keeps mass.
pub(crate) fn molar_mass(elems: &[u16], grams: &[f64]) -> f64 {
    elems.iter().zip(grams).map(|(&c, &x)| c as f64 * x).sum::<f64>() / 1000.0
}

pub fn parse(text: &str) -> Result<(Chemistry, Init), String> {
    let f: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let pair = |v: &[[u16; 2]]| v.iter().map(|p| (p[0], p[1])).collect();
    let subs = f
        .s
        .into_iter()
        .map(|s| Substance {
            molar_mass: molar_mass(&s.e, &f.element_masses),
            elems: s.e,
            cp: s.cp,
            melt: s.melt,
            boil: s.boil,
            density: s.rho,
            dense: s.dense.unwrap_or(s.melt),
            dense_drop: s.dense_drop * 1000.0,
            hardness: s.hard,
            strength: s.str,
            refl: s.refl,
            heat: s.heat,
            continuum: s.cont,
            sw_abs: s.sw,
            sw_sat: s.swsat,
            scatter: s.ray,
            solubility: s.sol,
            conductivity: s.cond,
            hf: s.hf * 1000.0,
            lv: s.lv.map_or(TROUTON * s.boil, |l| l * 1000.0),
            lf: s.lf.map_or(RICHARDS * s.melt, |l| l * 1000.0),
            rupture: s.rupture * 1e6,
            shear: s.shear * 1e6,
            bound: s.bound,
        })
        .collect();
    let reactions = f.r.into_iter().map(|r| Reaction { inputs: pair(&r.i), outputs: pair(&r.o), k: r.k, ea: r.ea, light: r.light, open: r.open.unwrap_or(1.0), ord: r.ord }).collect();
    let conversion = |b: &B| Reaction::conversion(pair(&b.i), pair(&b.o));
    let chem = Chemistry::new(f.element_masses.len(), subs, reactions, (Some(conversion(&f.build)), f.builds.iter().map(conversion).collect()), &f.init.air, f.heat_bands);
    chem.validate()?;
    check_makes(&f.init)?;
    Ok((chem, f.init))
}

/// Refuses makes that the seams or the rim name but the file does not hold, a make named twice, and a stack left with
/// no make of its own.
fn check_makes(init: &Init) -> Result<(), String> {
    let named: Vec<usize> = init.seams.iter().map(|s| s.0).chain([init.rim]).collect();
    let n = init.makes.len();
    if let Some(&m) = named.iter().find(|&&m| m >= n) {
        return Err(format!("make {m} is named as a seam or the rim, but there are {n} makes"));
    }
    if (1..named.len()).any(|i| named[..i].contains(&named[i])) {
        return Err("a make is named twice as a seam or the rim".into());
    }
    if named.len() >= n || named.contains(&0) {
        return Err("the stack needs its own makes, the first among them".into());
    }
    Ok(())
}

pub fn chemistry0() -> (Chemistry, Init) {
    parse(include_str!("chem0.toml")).expect("chemistry 0 is valid")
}

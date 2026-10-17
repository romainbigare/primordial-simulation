//! The loader of a chemistry file (chemistry 0, `chem0.toml`), and the latent heats where a file gives none.

use pcore::chem::{Chemistry, Reaction, Substance};
use serde::Deserialize;

/// Trouton's rule: the vaporisation entropy of a liquid at its boiling point, J/(mol K) (lv = TROUTON x boil).
pub(crate) const TROUTON: f64 = 88.0;
/// Richards's rule: the fusion entropy of a solid at its melting point, J/(mol K) (lf = RICHARDS x melt).
pub(crate) const RICHARDS: f64 = 9.5;

#[derive(Deserialize)]
struct File {
    elements: usize,
    s: Vec<S>,
    r: Vec<R>,
    #[serde(default)]
    build: Option<B>,
    #[serde(default)]
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
    m: f64,
    cp: f64,
    melt: f64,
    boil: f64,
    rho: [f64; 2],
    hard: f64,
    str: f64,
    refl: [[f32; 3]; 2],
    ir: f64,
    #[serde(default)]
    sat: f64,
    sol: f64,
    cond: f64,
    hf: f64,
    /// kJ/mol; none: Trouton's rule (`TROUTON`), and Richards's rule (`RICHARDS`) for `lf`
    lv: Option<f64>,
    lf: Option<f64>,
    /// GPa (`Substance::modulus`); none: 0
    #[serde(default)]
    modulus: f64,
    /// MPa (`Substance::rupture`); none: 0
    #[serde(default)]
    rupture: f64,
}

#[derive(Deserialize)]
struct R {
    i: Vec<[u16; 2]>,
    o: Vec<[u16; 2]>,
    k: f64,
    ea: f64,
    #[serde(default)]
    light: bool,
    /// The power of each input's share in the rate law, where it is not its count (`Reaction::ord`)
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
    /// Volume fractions of bonded ground
    pub bed: Vec<(usize, f64)>,
    /// Volume fractions of loose ground
    pub loose: Vec<(usize, f64)>,
    /// Extra volume fractions found in patches
    pub deposits: Vec<(usize, f64)>,
    /// Amounts in the loose ground of every land cell when life starts, mol per m² (`life::soil`; empty: none)
    pub soil: Vec<(usize, f64)>,
}

pub fn parse(text: &str) -> Result<(Chemistry, Init), String> {
    let f: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let pair = |v: &[[u16; 2]]| v.iter().map(|p| (p[0], p[1])).collect();
    let subs = f
        .s
        .into_iter()
        .map(|s| Substance {
            elems: s.e,
            molar_mass: s.m / 1000.0,
            cp: s.cp,
            melt: s.melt,
            boil: s.boil,
            density: s.rho,
            hardness: s.hard,
            strength: s.str,
            refl: s.refl,
            ir_abs: s.ir,
            ir_sat: s.sat,
            solubility: s.sol,
            conductivity: s.cond,
            hf: s.hf * 1000.0,
            lv: s.lv.map_or(TROUTON * s.boil, |l| l * 1000.0),
            lf: s.lf.map_or(RICHARDS * s.melt, |l| l * 1000.0),
            modulus: s.modulus * 1e9,
            rupture: s.rupture * 1e6,
        })
        .collect();
    let reactions = f.r.into_iter().map(|r| Reaction { inputs: pair(&r.i), outputs: pair(&r.o), k: r.k, ea: r.ea, light: r.light, ord: r.ord }).collect();
    let conversion = |b: &B| Reaction::conversion(pair(&b.i), pair(&b.o));
    let chem = Chemistry::new(f.elements, subs, reactions, f.build.as_ref().map(conversion), f.builds.iter().map(conversion).collect(), &f.init.air);
    chem.validate()?;
    Ok((chem, f.init))
}

pub fn chemistry0() -> (Chemistry, Init) {
    parse(include_str!("chem0.toml")).expect("chemistry 0 is valid")
}

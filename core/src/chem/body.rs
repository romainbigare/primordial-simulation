//! What bodies hold and run (laws.md 6.1): the substances of their stores, each at a place of a fixed array, and the
//! reactions their diet genes may turn on, found from the chemistry's roles.

use super::Chemistry;
use crate::units::{BODY_REACTIONS, BODY_STORES, MAX_REACTIONS, MAX_SUBSTANCES};

/// The place of a substance or a reaction that bodies have none for.
const NONE: u8 = u8::MAX;

/// The stores of a body, mol of each substance it holds, at the places of `BodyChem`.
pub type Stores = [f64; BODY_STORES];

/// What bodies hold and run. Their stores: the liquid and the energy-rich solid of life's roles, and every substance
/// growth and the other solids bodies build take or make that is not a gas at the surface (the fuel, the minerals, the
/// body matter, the built solids). Their reactions: those whose every substance is a store or a gas at the surface,
/// taking at least one store. A body holds nothing else: what it cannot hold it never takes, and a gas leaves it.
#[derive(Clone, Debug)]
pub struct BodyChem {
    /// The substance of each store, lowest first
    subs: Vec<usize>,
    /// The place of each substance among the stores (`NONE`: bodies hold none of it)
    place: [u8; MAX_SUBSTANCES],
    /// The reactions bodies may run, lowest first, and the place of each among them (`NONE`: none)
    reactions: Vec<usize>,
    rplace: [u8; MAX_REACTIONS],
    /// Of each store: its molar mass, kg/mol, and heat capacity, J/(mol K)
    pub mm: Stores,
    pub cp: Stores,
}

impl BodyChem {
    /// What bodies of `chem` hold and run (`BodyChem`).
    pub(super) fn of(chem: &Chemistry) -> BodyChem {
        let gas = |k: usize| chem.gas_at_surface(k);
        let mut held = 0u64;
        if let Some((_, rich, liquid, _)) = chem.find_life_roles() {
            held |= 1 << rich | 1 << liquid;
        }
        for rx in chem.build.iter().chain(&chem.builds) {
            held = rx.inputs.iter().chain(&rx.outputs).map(|p| p.0 as usize).filter(|&k| !gas(k)).fold(held, |m, k| m | 1 << k);
        }
        let subs: Vec<usize> = crate::math::ones(held).collect();
        let mut place = [NONE; MAX_SUBSTANCES];
        subs.iter().enumerate().for_each(|(s, &k)| place[k] = s as u8);
        let store = |k: u16| held >> k & 1 == 1;
        let runs = |r: usize| {
            let rx = &chem.reactions[r];
            rx.inputs.iter().chain(&rx.outputs).all(|p| store(p.0) || gas(p.0 as usize)) && rx.inputs.iter().any(|p| store(p.0))
        };
        let reactions: Vec<usize> = (0..chem.reactions.len()).filter(|&r| runs(r)).collect();
        let mut rplace = [NONE; MAX_REACTIONS];
        reactions.iter().enumerate().for_each(|(q, &r)| rplace[r] = q as u8);
        let (mut mm, mut cp) = ([0.0; BODY_STORES], [0.0; BODY_STORES]);
        for (s, &k) in subs.iter().enumerate().take(BODY_STORES) {
            (mm[s], cp[s]) = (chem.subs[k].molar_mass, chem.subs[k].cp);
        }
        BodyChem { subs, place, reactions, rplace, mm, cp }
    }
    /// The count of stores.
    #[inline]
    pub fn n(&self) -> usize {
        self.subs.len()
    }
    /// The substance of each store, lowest first.
    #[inline]
    pub fn subs(&self) -> &[usize] {
        &self.subs
    }
    /// The place of substance `k` among the stores, if bodies hold it.
    #[inline]
    pub fn place(&self, k: usize) -> Option<usize> {
        self.place.get(k).filter(|&&s| s != NONE).map(|&s| s as usize)
    }
    /// The reactions bodies may run, lowest first.
    #[inline]
    pub fn reactions(&self) -> &[usize] {
        &self.reactions
    }
    /// The place of reaction `r` among those bodies may run, if they may.
    #[inline]
    pub fn reaction(&self, r: usize) -> Option<usize> {
        self.rplace.get(r).filter(|&&q| q != NONE).map(|&q| q as usize)
    }
    /// Mass of amounts `m` (mol of each store), kg.
    #[inline]
    pub fn mass_of(&self, m: &Stores) -> f64 {
        m.iter().zip(&self.mm).take(self.n()).map(|(a, mm)| a * mm).sum()
    }
    /// Heat capacity of amounts `m` (mol of each store), J/K.
    #[inline]
    pub fn heat_cap_of(&self, m: &Stores) -> f64 {
        m.iter().zip(&self.cp).take(self.n()).map(|(a, cp)| a * cp).sum()
    }
    /// Amounts `m` of the stores as mol of each substance of the chemistry.
    pub fn spread_out(&self, m: &Stores) -> [f64; MAX_SUBSTANCES] {
        let mut all = [0.0; MAX_SUBSTANCES];
        self.subs.iter().zip(m).for_each(|(&k, &a)| all[k] = a);
        all
    }
    /// What bodies can hold of amounts `from` (mol of each substance), as stores.
    pub fn gather(&self, from: impl Fn(usize) -> f64) -> Stores {
        let mut m = [0.0; BODY_STORES];
        self.subs.iter().zip(&mut m).for_each(|(&k, x)| *x = from(k));
        m
    }
    /// Whether bodies of this chemistry can be held in a body's fixed arrays.
    pub(super) fn fits(&self) -> Result<(), String> {
        let (ns, nr) = (self.n(), self.reactions.len());
        crate::params::require(ns <= BODY_STORES, || format!("bodies hold {ns} substances: at most {BODY_STORES}"))?;
        crate::params::require(nr <= BODY_REACTIONS, || format!("bodies may run {nr} reactions: at most {BODY_REACTIONS}"))
    }
}

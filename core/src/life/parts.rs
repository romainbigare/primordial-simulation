//! The substances life is made of and the reactions it runs, found from the chemistry's numbers
//! (`Chemistry::life_roles`): the fuel its light reaction makes, its body matter, and the solid it builds of the fuel
//! alone (its walls). The cover holds all three (`cover::Cover::mol`); a taker holds the fuel and the body matter.

use super::WET_TISSUE;
use crate::chem::Chemistry;

/// The parts of life (`Parts`), by their place in its amounts: the fuel, the body matter, the walls.
pub const NP: usize = 3;
pub const FUEL: usize = 0;
pub const MATTER: usize = 1;
pub const WALL: usize = 2;

/// The substances of life's parts and the reactions it runs.
#[derive(Clone, Copy, Debug)]
pub struct Parts {
    /// The substance of each part
    pub sub: [usize; NP],
    /// The light reaction, the burning of the fuel, the fixing reaction (if any), and the key liquid
    pub light: usize,
    pub burn: usize,
    pub fix: Option<usize>,
    pub liquid: usize,
}

impl Parts {
    /// The parts in `chem`: none unless it has the roles of life, body matter and a solid built of the fuel alone.
    pub fn of(chem: &Chemistry) -> Option<Parts> {
        let ((light, rich, liquid, burn), matter) = (chem.life_roles()?, chem.matter()?);
        let wall = chem.builds.iter().find(|b| b.inputs.iter().all(|p| p.0 as usize == rich))?.outputs[0].0 as usize;
        Some(Parts { sub: [rich, matter, wall], light, burn, fix: chem.fixing(), liquid })
    }
    /// The molar mass of each part, kg/mol.
    pub fn mm(&self, chem: &Chemistry) -> [f64; NP] {
        self.sub.map(|k| chem.subs[k].molar_mass)
    }
    /// The energy one kg of living tissue costs to build, J: per kg of its body matter the fuel its conversion
    /// (`Chemistry::build`) takes, as burning it would free it, and the work of building, `c_copy`.
    pub fn living_cost(&self, chem: &Chemistry, ph: &crate::Physics) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return ph.c_copy * (1.0 - WET_TISSUE) };
        let per = |rx: &crate::chem::Reaction, k: usize| rx.inputs.iter().find(|q| q.0 as usize == k).map(|q| q.1 as f64);
        let burn = &chem.reactions[self.burn];
        let fuel = per(bl, self.sub[FUEL]).unwrap_or(0.0) / self.mm(chem)[MATTER] / bl.outputs[0].1 as f64;
        (fuel * -chem.dh(self.burn) / per(burn, self.sub[FUEL]).unwrap_or(1.0) + ph.c_copy) * (1.0 - WET_TISSUE)
    }
}

//! The substances life is made of and the reactions it runs, found from the chemistry's numbers
//! (`Chemistry::life_roles`): the fuel its light reaction makes, its body matter, and the solid it builds of the fuel
//! alone (its walls). The cover holds all three (`cover::Cover::mol`); a taker holds the fuel and the body matter.

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
}

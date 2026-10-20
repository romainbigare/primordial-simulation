//! The three ways a law changes the world: a move between two nodes, a transform of the amounts inside one node, and a
//! link flux over a whole field. Each books both sides, so the books are kept by their form (laws.md 0).

use super::nodes::{Class, Node};
use crate::chem::Chemistry;

/// `mol` of substance `k` and `heat` J leave node `from` for node `to`, which receives `heat` less what the mol hold
/// more there (`held_more`): so the law chooses who pays a gas's latent heat. A body's or part's end is its law's.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub from: Node,
    pub to: Node,
    pub k: u16,
    pub mol: f64,
    pub heat: f64,
}

impl Move {
    /// `mol` of substance `k` of the bed under ground node `i`, which holds `bed` of it, break into its loose ground
    /// (wear, digging). It moves only what the bed can lose exactly (`bed - (bed - mol)`): the bed holds far more than
    /// its loose ground, so an amount below the bed's rounding would leave the bed whole and still reach the ground.
    pub fn loosen(i: usize, k: usize, mol: f64, bed: f64) -> Move {
        Move { from: Node::new(Class::Bed, i), to: Node::ground(i), k: k as u16, mol: bed - (bed - mol), heat: 0.0 }
    }
}

/// The amounts of node `at` change by `dmol` (substance, mol), keeping every element; the energy they held apart from
/// heat (`held`) and no longer hold becomes the node's heat (the heat of reaction).
#[derive(Clone, Debug)]
pub struct Transform {
    pub at: Node,
    pub dmol: Vec<(u16, f64)>,
}

/// What crosses each link of a field at once: link `l` joins node `a[l]` of `class[0]` to node `b[l]` of `class[1]` and
/// carries `mol[q * links + l]` of substance `ks[q]` and `heat[l]` J, + from `a` to `b`. Where the classes hold a
/// substance unlike (`held`), `b` receives the heat less what the mol hold more there, as a `Move`'s `to` does. Only
/// what each node holds at the end must be at zero or above.
#[derive(Clone, Copy, Debug)]
pub struct LinkFlux<'a> {
    pub class: [Class; 2],
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub ks: &'a [u16],
    pub mol: &'a [f64],
    pub heat: &'a [f64],
}

/// Energy a mol of substance `k` holds in a node of `class` apart from its heat, J/mol: its formation enthalpy, and in
/// the air and in a bodies' node (which holds only gas) also its fusion and vaporisation enthalpy.
pub fn held(chem: &Chemistry, class: Class, k: usize) -> f64 {
    let s = &chem.subs[k];
    s.hf + if matches!(class, Class::Air | Class::Bodies) { s.latent_gas() } else { 0.0 }
}

/// What a mol of substance `k` holds apart from heat more in a node of class `to` than in one of class `from`, J/mol
/// (`held`): the air's latent heat, or nothing.
pub fn held_more(chem: &Chemistry, from: Class, to: Class, k: usize) -> f64 {
    held(chem, to, k) - held(chem, from, k)
}

impl<'a> LinkFlux<'a> {
    /// Whether its two classes hold each of its substances apart from heat alike (`held`).
    pub(super) fn alike(&self, chem: &Chemistry) -> bool {
        self.ks.iter().all(|&k| held_more(chem, self.class[0], self.class[1], k as usize) == 0.0)
    }
    /// The heat its `b` ends receive, by link: what leaves `a` less what the mol hold more at `b` (empty when `alike`).
    pub(super) fn lifted(&self, chem: &Chemistry) -> Vec<f64> {
        if self.alike(chem) {
            return Vec::new();
        }
        let (links, mut q) = (self.a.len(), self.heat.to_vec());
        for (x, &k) in self.ks.iter().enumerate() {
            let more = held_more(chem, self.class[0], self.class[1], k as usize);
            q.iter_mut().zip(&self.mol[x * links..(x + 1) * links]).for_each(|(q, m)| *q -= m * more);
        }
        q
    }
}

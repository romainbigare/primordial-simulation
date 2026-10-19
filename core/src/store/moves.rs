//! The three ways a law changes the world (`docs/foundation/plan.md`, Idea 2): a move between two nodes, a transform
//! of the amounts inside one node, and a link flux over a whole field.

use super::nodes::{Class, Node};
use crate::chem::Chemistry;

/// `mol` of substance `k` and `heat` J leave node `from` for node `to`. `to` receives `heat` less the energy the mol
/// hold apart from heat more there than in `from` (`held_more`): so the law chooses who pays the latent heat of a gas
/// that enters the air, `from` by giving it with the heat, or `to` by being given only the heat the gas carries. An end
/// that holds its own amounts (a body, a part) is booked by the law that changes it.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub from: Node,
    pub to: Node,
    pub k: u16,
    pub mol: f64,
    pub heat: f64,
}

/// The amounts of node `at` change by `dmol` (substance, mol), keeping every element; the energy they held apart from
/// heat (`held`) and no longer hold becomes the node's heat (the heat of reaction).
#[derive(Clone, Debug)]
pub struct Transform {
    pub at: Node,
    pub dmol: Vec<(u16, f64)>,
}

/// What crosses each link of a field of nodes at once: link `l` joins the node at index `a[l]` of class `class[0]` and
/// the one at `b[l]` of `class[1]` (one class for the links of a level; two for the vertical links of a stack, between
/// a ground node and its air node, or from a boundary node or the potential to each node of a field) and carries
/// `mol[q * links + l]` mol of substance `ks[q]` (each substance once) and `heat[l]` J, + from `a` to `b` (`heat`
/// empty: no heat). What leaves one end enters the other, so a flux keeps the books by its form; where the two classes
/// hold a substance apart from heat unlike (`held`: the air holds its latent heat), `b` receives the heat less what the
/// mol hold more there than at `a`, as the `to` of a `Move` does (so such a flux has heat). What passes through a node,
/// in by one link and out by another, as liquid down a slope, needs nothing held: a law keeps only what each node holds
/// at the end of the flux at zero or above.
#[derive(Clone, Copy, Debug)]
pub struct LinkFlux<'a> {
    pub class: [Class; 2],
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub ks: &'a [u16],
    pub mol: &'a [f64],
    pub heat: &'a [f64],
}

/// Energy a mol of substance `k` holds in a node of `class` apart from the node's heat, J/mol: its formation enthalpy,
/// and in the air its fusion and vaporisation enthalpy (`Substance::latent_gas`), as `check::totals` books them.
pub fn held(chem: &Chemistry, class: Class, k: usize) -> f64 {
    let s = &chem.subs[k];
    s.hf + if class == Class::Air { s.latent_gas() } else { 0.0 }
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
    /// The heat its `b` ends receive, by link: what leaves `a` less what the mol hold more at `b` (empty where the
    /// classes hold its substances alike, `alike`).
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

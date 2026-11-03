//! The links of parts: the bonds between them, and the enclosure that joins the inside of a structure to its cell.

use serde::{Deserialize, Serialize};

/// The bonds of a cell's parts, each once, in the order they were made: bond `l` joins part `a[l]` to part `b[l]` (ids)
/// and holds `strength[l]` N.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bonds {
    pub a: Vec<u64>,
    pub b: Vec<u64>,
    pub strength: Vec<f32>,
}

impl Bonds {
    /// The bonds of the part named `p`, in the order they were made: the other end, and the strength.
    pub fn of(&self, p: u64) -> impl Iterator<Item = (u64, f32)> + '_ {
        (0..self.a.len()).filter_map(move |l| {
            let other = if self.a[l] == p { self.b[l] } else if self.b[l] == p { self.a[l] } else { return None };
            Some((other, self.strength[l]))
        })
    }
    pub(super) fn add(&mut self, a: u64, b: u64, strength: f32) {
        self.a.push(a);
        self.b.push(b);
        self.strength.push(strength);
    }
    /// Break the first bond between `a` and `b`.
    pub(super) fn remove(&mut self, a: u64, b: u64) {
        if let Some(l) = (0..self.a.len()).find(|&l| (self.a[l], self.b[l]) == (a, b) || (self.a[l], self.b[l]) == (b, a)) {
            self.a.remove(l);
            self.b.remove(l);
            self.strength.remove(l);
        }
    }
    /// The bonds of the parts named `ids`, taken out, in their order (a structure that leaves its cell).
    pub(super) fn take(&mut self, ids: &[u64]) -> Bonds {
        let mut out = Bonds::default();
        let mut kept = Bonds::default();
        for l in 0..self.a.len() {
            let to = if ids.contains(&self.a[l]) || ids.contains(&self.b[l]) { &mut out } else { &mut kept };
            to.add(self.a[l], self.b[l], self.strength[l]);
        }
        *self = kept;
        out
    }
    /// Bonds `o` join these, after them.
    pub(super) fn extend(&mut self, o: Bonds) {
        self.a.extend(o.a);
        self.b.extend(o.b);
        self.strength.extend(o.strength);
    }
    /// How many bonds there are.
    pub fn count(&self) -> usize {
        self.a.len()
    }
}

/// The link between the inside of a structure and its cell: heat passes through the walls and the openings, light and
/// rain only through the openings.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Enclosure {
    /// How closed the walls are around the space inside (0..1): light and rain pass by 1 - this
    pub closed: f32,
    /// Heat conductance between the inside and the outside, W/K
    pub g: f64,
}

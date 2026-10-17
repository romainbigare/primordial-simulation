//! The links of parts: the bonds between them, and the enclosure that joins the inside of a structure to its cell.

use serde::{Deserialize, Serialize};

/// The bonds of parts, each once, in the order they were made: bond `l` joins part `a[l]` to part `b[l]` (or to the
/// bonded ground of its cell, `GROUND`) and holds `strength[l]` N.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bonds {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub strength: Vec<f32>,
}

impl Bonds {
    /// The bonds of part `p`, in the order they were made: the other end, and the strength.
    pub fn of(&self, p: usize) -> impl Iterator<Item = (u32, f32)> + '_ {
        let p = p as u32;
        (0..self.a.len()).filter_map(move |l| {
            let other = if self.a[l] == p { self.b[l] } else if self.b[l] == p { self.a[l] } else { return None };
            Some((other, self.strength[l]))
        })
    }
    pub(super) fn add(&mut self, a: u32, b: u32, strength: f32) {
        self.a.push(a);
        self.b.push(b);
        self.strength.push(strength);
    }
    /// Break the first bond between `a` and `b`.
    pub(super) fn remove(&mut self, a: u32, b: u32) {
        if let Some(l) = (0..self.a.len()).find(|&l| (self.a[l], self.b[l]) == (a, b) || (self.a[l], self.b[l]) == (b, a)) {
            self.a.remove(l);
            self.b.remove(l);
            self.strength.remove(l);
        }
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

//! The potential energy the liquid's pass carries (laws.md 0, "The books"; 1.2): each move carries it from the level
//! its cell is left at, and where it lands it turns to heat in the liquid that lands, so what passes on takes it along;
//! a climb is paid from that heat, which holds the liquid's falls.

use super::potential::heads;
use super::Scratch;
use crate::World;

/// The potential energy of the liquid's pass (`runoff::surface`), which it books itself (`potential::apply_carried`),
/// and the heat its falls give on the way (`Lifts::give`).
pub(super) struct Falls {
    /// The head of each cell, m, as the booking stacks it (`potential::heads`): at the pass's start, and from its visit
    /// on as the pass leaves it.
    level: Vec<f64>,
    /// Of each cell still to come: the mass that came in, kg, and the potential energy it carries, J.
    came: Vec<[f64; 2]>,
    /// Of each ground node: the potential energy that came in less what left, J.
    pub(super) carried: Vec<f64>,
    /// Of each ground node: the heat the falls gave there, J (less than none where the liquid paid a climb).
    pub(super) given: Vec<f64>,
    /// Gravity, m/s².
    g: f64,
}

impl Falls {
    /// At the pass's start: the heads of the world now.
    pub(super) fn new(w: &World, s: &Scratch) -> Falls {
        let n = w.mesh.cells().n();
        Falls { level: heads(w, s), came: vec![[0.0; 2]; n], carried: vec![0.0; n], given: vec![0.0; n], g: w.cfg.gravity }
    }

    /// Cell `i` is visited: of what it holds, `kg` leave, each from its head as the pass leaves it, `rise` m over its
    /// head before; what came in carried the potential energy of where it left; and its stack gains the mass it keeps
    /// at the mean of its head before and after (a column's potential energy, as the booking finds it). What is left
    /// over turns to heat here, J, returned for the liquid that came in to take before it mixes with the node, so what
    /// passes on takes it along: the fall of what came in, and of the cell's own column from its middle to where it is
    /// left (a column that runs off whole falls from its middle); less than none where what came in left from lower
    /// than its head is left at, a climb the liquid pays from the heat it carries.
    pub(super) fn visit(&mut self, i: usize, kg: f64, rise: f64) -> f64 {
        let ([mass, e], was) = (self.came[i], self.level[i]);
        let end = was + rise;
        let heat = e - kg * self.g * end - (mass - kg) * self.g * (was + end) / 2.0;
        (self.level[i], self.given[i]) = (end, self.given[i] + heat);
        heat
    }

    /// `kg` that fill `m3` leave cell `i`, visited, for cell `j` of area `a`, with the potential energy of `i`'s head as
    /// the pass leaves it. A cell still to come takes it at its visit (`visit`); one passed at once, the fall to the
    /// middle of the layer it makes on its head; the low basin, which the pass does not visit and whose liquid then
    /// stands at one level (`basin::level`), at once, the fall to its head.
    pub(super) fn send(&mut self, (i, j): (usize, usize), (kg, m3, a): (f64, f64, f64), lands: Lands) {
        let e = kg * self.g * self.level[i];
        (self.carried[i], self.carried[j]) = (self.carried[i] - e, self.carried[j] + e);
        match lands {
            Lands::Ahead => self.came[j] = [self.came[j][0] + kg, self.came[j][1] + e],
            Lands::Passed => {
                let rise = m3 / a;
                self.given[j] += e - kg * self.g * (self.level[j] + rise / 2.0);
                self.level[j] += rise;
            }
            Lands::Basin => self.given[j] += e - kg * self.g * self.level[j],
        }
    }
}

/// The cell liquid goes to in the pass (`Falls::send`): one still to come, one passed, or one of the low basin.
#[derive(Clone, Copy, Debug)]
pub(super) enum Lands {
    Ahead,
    Passed,
    Basin,
}

//! The potential energy the liquid's pass carries (laws.md 0, "The books"; 1.2): each move carries it from the top its
//! cell's liquid is left at (in its pores and over its own ground, `stack::Column`), and where it lands it turns to heat in the liquid that lands, so what passes on takes it along;
//! a climb is paid from that heat, which holds the liquid's falls.

use super::curve::{of_cell, Curve};
use super::potential::columns;
use super::stack::Column;
use super::Scratch;
use crate::World;
use std::borrow::Cow;

/// The potential energy of the liquid's pass (`runoff::surface`), which it books itself (`potential::apply_carried`),
/// and the heat its falls give on the way (`Lifts::give`).
pub(super) struct Falls {
    /// The liquid of each cell as the booking stacks it (`stack::Column`), and the height it leaves from, m
    /// (`Column::top`): at the pass's start, and from its visit on as the pass leaves it.
    column: Vec<Column>,
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
    /// At the pass's start: the liquid of the world now.
    pub(super) fn new(w: &World, s: &Scratch) -> Falls {
        let (n, column) = (w.mesh.cells().n(), columns(w, s));
        let level = column.iter().enumerate().map(|(i, c)| c.top(|| of_cell(w, s, i))).collect();
        Falls { column, level, came: vec![[0.0; 2]; n], carried: vec![0.0; n], given: vec![0.0; n], g: w.cfg.gravity }
    }

    /// Cell `i` (its ground `curve`) is visited: its liquid changes by `liquid` m³ and its loose solids by `solid` m³
    /// over the pass, and of what it holds, `kg` leave, each from its liquid's top as the pass leaves it; what came in
    /// carried the potential energy of where it left; and its stack gains the mass it keeps at the mean of its top
    /// before and after (the potential energy of a layer on its top, as the booking finds it to first order). What is
    /// left over turns to heat here, J, returned for the liquid that came in to take before it mixes with the node, so
    /// what passes on takes it along: the fall of what came in, and of the cell's own liquid from where it lay to
    /// where it is left; less than none where what came in left from lower than the top it is left at, a climb the
    /// liquid pays from the heat it carries.
    pub(super) fn visit(&mut self, (i, curve): (usize, &Curve), kg: f64, [liquid, solid]: [f64; 2]) -> f64 {
        let ([mass, e], was) = (self.came[i], self.level[i]);
        let end = self.change(i, curve, [liquid, solid]);
        let heat = e - kg * self.g * end - (mass - kg) * self.g * (was + end) / 2.0;
        self.given[i] += heat;
        heat
    }

    /// `kg` that fill `[liquid, solid]` m³ leave cell `i`, visited, for cell `j` (its ground `curve`), with the
    /// potential energy of `i`'s top as the pass leaves it. A cell still to come takes it at its visit (`visit`); one
    /// passed at once, the fall to the middle of the layer it makes on its top; the low basin, which the pass does not
    /// visit and whose liquid then stands at one level (`basin::level`), at once, the fall to its top.
    pub(super) fn send(&mut self, (i, (j, curve)): (usize, (usize, &Curve)), (kg, m3): (f64, [f64; 2]), lands: Lands) {
        let e = kg * self.g * self.level[i];
        (self.carried[i], self.carried[j]) = (self.carried[i] - e, self.carried[j] + e);
        match lands {
            Lands::Ahead => self.came[j] = [self.came[j][0] + kg, self.came[j][1] + e],
            Lands::Passed => {
                let was = self.level[j];
                let end = self.change(j, curve, m3);
                self.given[j] += e - kg * self.g * (was + end) / 2.0;
            }
            Lands::Basin => self.given[j] += e - kg * self.g * self.level[j],
        }
    }

    /// Cell `i` (its ground `curve`) gains `liquid` m³ of liquid and `solid` m³ of loose solids: its column, and the
    /// top its liquid leaves from now, m.
    fn change(&mut self, i: usize, curve: &Curve, [liquid, solid]: [f64; 2]) -> f64 {
        let c = &mut self.column[i];
        (c.vol, c.z, c.loose) = (c.vol + liquid, c.z + solid / c.area, c.loose + solid);
        self.level[i] = c.top(|| Cow::Borrowed(curve));
        self.level[i]
    }
}

/// The cell liquid goes to in the pass (`Falls::send`): one still to come, one passed, or one of the low basin.
#[derive(Clone, Copy, Debug)]
pub(super) enum Lands {
    Ahead,
    Passed,
    Basin,
}

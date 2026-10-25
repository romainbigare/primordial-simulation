//! The heat bodies draw from their cells' ground between their updates (`Draw`), and what the bodies of each cell draw
//! and claim of their bodies' node at an air step (`Draws`).

use super::light_area;
use crate::store::{apply, Batch, Class};
use crate::{life::Handle, units::DAY, World};
use serde::{Deserialize, Serialize};

/// Per cell, what its bodies claim of its bodies' node, J: the light set aside for each since its mark, and the heat its
/// ground gave the node for each since its last update (`Draw::owed`).
pub(crate) fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    for b in w.life.alive() {
        let i = b.cell as usize;
        claim[i] += light_area(b) * b.keep as f64 * (w.light[i] - b.mark) + b.draw.owed;
    }
    claim
}

/// The heat a body draws from its ground between its updates (laws.md 1.4): what it drew at its last update, less in
/// proportion as the ground comes nearer the temperature at which it draws nothing: where what it evaporates stops (the
/// dew point of the air it breathes, the vapour's deficit going nearly with the temperature above it; or `COOL_T` where
/// it evaporated to cool), but no nearer than its conductance to the surface allows. No more than it drew then.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Draw {
    /// The heat it drew from the ground at its last update, W (0 if it gave heat)
    pub most: f64,
    /// What it draws per K of ground above `t`, W/K, and the ground temperature at which it draws nothing, K
    pub g: f64,
    pub t: f64,
    /// Heat its cell's ground gave its bodies' node for it since its last update, J
    pub owed: f64,
}

impl Draw {
    /// After an update that drew `drew` W through a conductance `g` W/K from ground at `tg` K, what it evaporated
    /// stopping at `stop` K (none if it evaporated nothing), owing nothing yet.
    pub(crate) fn after(drew: f64, g: f64, tg: f64, stop: Option<f64>) -> Draw {
        if drew <= 0.0 || g <= 0.0 {
            return Draw::default();
        }
        let t = stop.map_or(tg - drew / g, |s| s.min(tg - drew / g));
        Draw { most: drew, g: drew / (tg - t), t, owed: 0.0 }
    }
    /// What it draws from ground at `t` K, W, and its slope, W/K.
    #[inline]
    fn at(&self, t: f64) -> (f64, f64) {
        match self.g * (t - self.t) {
            r if r <= 0.0 => (0.0, 0.0),
            r if r >= self.most => (self.most, 0.0),
            r => (r, self.g),
        }
    }
}

/// The bodies that draw heat from each cell's ground (`Draw`), cell by cell, for an air step.
pub(crate) struct Draws {
    /// Per cell, its entries `at[i]..at[i + 1]`
    at: Vec<u32>,
    /// Per entry: the body and its draw
    body: Vec<Handle>,
    each: Vec<Draw>,
    /// Per cell, the heat its bodies' node holds that no body claims and its bodies draw first, over the step, W
    pool: Vec<f64>,
}

impl Draws {
    /// Before an air step of `dt` seconds: the bodies that draw from each cell's ground, and the heat each bodies' node
    /// holds that none of its bodies claim (`claims`). What its bodies could draw of that until their next updates
    /// stays for them to draw first, so a draw an update did not use spares the ground later rather than going back at
    /// once; the rest goes back to the ground now, so the step starts from it.
    pub(crate) fn settle(w: &mut World, dt: f64) -> Draws {
        let n = w.mesh.cells().n();
        let mut d = Draws { at: vec![0], body: Vec::new(), each: Vec::new(), pool: vec![0.0; n] };
        // What the bodies of each cell could draw until their next updates, J.
        let mut most = vec![0.0; n];
        for (i, most) in most.iter_mut().enumerate() {
            for b in w.life.cell(i).map(|s| (s, &w.life[s])).filter(|(_, b)| b.alive && b.draw.most > 0.0) {
                d.body.push(b.0);
                d.each.push(b.1.draw);
                *most += b.1.draw.most * b.1.interval(w) as f64 * DAY;
            }
            d.at.push(d.body.len() as u32);
        }
        let mut back = Batch::new([Class::Bodies, Class::Ground], &[]);
        for (i, c) in claims(w).into_iter().enumerate() {
            let free = w.store.heat(Class::Bodies)[i] - c;
            d.pool[i] = free.clamp(0.0, most[i]) / dt;
            if free > most[i] {
                back.link((i, i), free - most[i]);
            }
        }
        apply(w, &[back.flux()], &[], &[]);
        d
    }
    /// Do bodies draw from cell `i`'s ground?
    pub(crate) fn any(&self, i: usize) -> bool {
        self.at[i] < self.at[i + 1]
    }
    /// What the bodies of cell `i` draw at `t` K, W, and its slope, W/K.
    fn drawn(&self, i: usize, t: f64) -> (f64, f64) {
        self.each[self.at[i] as usize..self.at[i + 1] as usize].iter().map(|d| d.at(t)).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1))
    }
    /// What the bodies of cell `i` draw from its ground at `t` K, past what they draw from their node's pool, W, and
    /// its slope, W/K.
    pub(crate) fn ground(&self, i: usize, t: f64) -> (f64, f64) {
        match self.drawn(i, t) {
            (r, s) if r > self.pool[i] => (r - self.pool[i], s),
            _ => (0.0, 0.0),
        }
    }
    /// Each body of cell `i` owes its share, by what it draws at `t(i)` K over `dt` seconds, of what its node got:
    /// `given[i]` J from the ground and what it drew of the pool.
    pub(crate) fn owe(&self, w: &mut World, t: impl Fn(usize) -> f64, given: &[f64], dt: f64) {
        for (i, &got) in given.iter().enumerate().filter(|e| self.any(e.0)) {
            let r = self.drawn(i, t(i)).0;
            if r > 0.0 {
                let per = (got + r.min(self.pool[i]) * dt) / r;
                for x in self.at[i] as usize..self.at[i + 1] as usize {
                    w.life.quiet(self.body[x]).draw.owed += self.each[x].at(t(i)).0 * per;
                }
            }
        }
    }
}

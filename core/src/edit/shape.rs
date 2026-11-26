//! The ground under a brush raised or lowered (handover.md 11: "Raise or lower land", a terrain brush). The matter comes
//! from the outside node and goes back to it (`World::cross`), with its heat at the ground's temperature and the
//! potential energy of where it lies, so the books close. A raise lays the cell's own rock (its bed's mixture) and, at
//! the top of a stroke, `Config::edit_top_m` of loose ground of one of the terrain's makes (`Make::of`); a lowering
//! takes the loose ground first, then the rock. Where in a cell the ground moved is kept on the terrain's samples
//! (`Relief::shaped`), which are built again over it when the edits settle (`World::settle`).

use super::Brush;
use crate::rng::{Purpose, Rng};
use crate::store::{Boundary, Class, Node};
use crate::World;
use std::collections::BTreeMap;

impl World {
    /// The make of the loose ground the stroke `stroke` of a shell's edits lays (`Act::Ground`): one of the terrain's
    /// makes, drawn from the world's seed.
    pub fn draw_make(&self, stroke: u64) -> u8 {
        Rng::new(self.seed, stroke, 0, Purpose::Edit).below(self.terrain.makes.len().max(1)) as u8
    }

    /// The ground under the brush `b` raised by `height` m at its centre (lowered where under none), by `falloff` of
    /// that at a distance d from it (in radii: a smooth bump, flat at its edge), over the samples it covers
    /// (`covered`). Each cell takes the volume its samples move by; a raise is the share `loose` of loose ground of the
    /// make `make` (the part of the stroke's height at the centre, which has raised `stroke` m there before, within
    /// `edit_top_m` of its top) and the rest the cell's rock (`raise`); a lowering takes at most what the cell holds
    /// (`lower`), and its samples move by the share it took. Whether any ground moved.
    pub(crate) fn shape(&mut self, b: Brush, (height, stroke): (f64, f64), make: usize) -> bool {
        let s = self.terrain_samples();
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        for c in self.covered(b, s.across) {
            cells.entry(c.cell).or_default().push((c.k, height * super::falloff(c.d)));
        }
        drop(s);
        let top = self.cfg.edit_top_m;
        let loose = if height > 0.0 { ((stroke + height).min(top) - stroke.min(top)).max(0.0) / height } else { 0.0 };
        let mix = self.terrain.makes.get(make).map_or_else(Vec::new, |m| m.of.clone());
        self.move_ground(&cells, loose, &mix)
    }

    /// The ground of each cell of `cells` moved by its samples' heights, m (each sample and how far it moves; raised
    /// where above none): the cell takes the volume they move by, a raise the share `loose` of it as loose ground of the
    /// substances `mix` and the rest as its rock (`raise`), a lowering at most what it holds (`lower`), and its samples
    /// move by the share it took (`Relief::shape`). Whether any ground moved.
    pub(super) fn move_ground(&mut self, cells: &BTreeMap<usize, Vec<(usize, f64)>>, loose: f64, mix: &[(usize, f64)]) -> bool {
        let s = self.terrain_samples();
        let mut moved = false;
        for (&i, samples) in cells {
            let v: f64 = samples.iter().map(|&(k, dh)| dh * s.area(k)).sum();
            let got = if v >= 0.0 { self.raise(i, v, loose, mix) } else { -self.lower(i, -v) };
            let share = if v != 0.0 { got / v } else { 0.0 };
            samples.iter().for_each(|&(k, dh)| self.relief.shape(k, dh * share, s.h.len()));
            moved |= got != 0.0;
        }
        moved
    }

    /// `v` m³ put on cell `i`: the share `loose` as loose ground of the substances `mix` (each with its share by volume),
    /// the rest as its bed's own mixture of the substances that hold the ground up (`holds_up`; all of `mix` where it
    /// holds none). Returns the volume put, m³ (none where neither can be made).
    fn raise(&mut self, i: usize, v: f64, loose: f64, mix: &[(usize, f64)]) -> f64 {
        let ns = self.chem.ns();
        let mv = |w: &World, k: usize| w.chem.molar_volume(k, crate::chem::Phase::Solid);
        let rock: f64 = (0..ns).filter(|&k| self.holds_up(k)).map(|k| self.b(k, i) * mv(self, k)).sum();
        let whole: f64 = mix.iter().map(|p| p.1).sum();
        let (to_loose, to_bed) = if rock > 0.0 { (v * loose, v * (1.0 - loose)) } else { (v, 0.0) };
        if (to_loose > 0.0 && whole <= 0.0) || v <= 0.0 {
            return 0.0;
        }
        let mut g = vec![0.0; ns];
        mix.iter().for_each(|&(k, f)| g[k] += to_loose * f / whole / mv(self, k));
        let bed: Vec<f64> = (0..ns).map(|k| if self.holds_up(k) && rock > 0.0 { self.b(k, i) * to_bed / rock } else { 0.0 }).collect();
        self.put(i, &g, &bed);
        v
    }

    /// At most `v` m³ taken from cell `i`: its loose solids that hold the ground up first (`holds_up`), then its bed's,
    /// each by its mixture. Returns the volume taken, m³.
    fn lower(&mut self, i: usize, v: f64) -> f64 {
        let ns = self.chem.ns();
        let mv = |w: &World, k: usize| w.chem.molar_volume(k, crate::chem::Phase::Solid);
        let held = |w: &World, at: &dyn Fn(usize) -> f64| (0..ns).filter(|&k| w.holds_up(k)).map(|k| at(k) * mv(w, k)).sum::<f64>();
        let (loose, rock) = (held(self, &|k| self.g(k, i)), held(self, &|k| self.b(k, i)));
        let from_loose = v.min(loose);
        let from_rock = (v - from_loose).min(rock);
        let take = |share: f64, at: &dyn Fn(usize) -> f64| (0..ns).map(|k| if self.holds_up(k) && share > 0.0 { -at(k) * share } else { 0.0 }).collect::<Vec<f64>>();
        let g = take(if loose > 0.0 { from_loose / loose } else { 0.0 }, &|k| self.g(k, i));
        let bed = take(if rock > 0.0 { from_rock / rock } else { 0.0 }, &|k| self.b(k, i));
        self.put(i, &g, &bed);
        from_loose + from_rock
    }

    /// `mol` of the liquid `k` put in ground node `i` from the outside node (`put`) as a liquid: where the node's
    /// temperature holds `k` solid, with its fusion enthalpy too (liquid at its melting point, as the node's heat then
    /// freezes it or not).
    pub(super) fn put_liquid(&mut self, i: usize, k: usize, mol: f64) {
        let (ns, t) = (self.chem.ns(), self.t_ground(i));
        let mut g = vec![0.0; ns];
        g[k] = mol;
        self.put(i, &g, &vec![0.0; ns]);
        let now = self.stock(Node::ground(i));
        if self.chem.fused_heat(k, t, now) == 0.0 {
            self.cross((Boundary::Outside, Node::ground(i)), Vec::new(), mol * self.chem.subs[k].lf);
        }
    }

    /// `g` mol of each substance put in ground node `i` and `bed` in its bed (taken where under none) from the outside
    /// node, with the heat that keeps the node at its temperature (`energy_of` after less before).
    pub(super) fn put(&mut self, i: usize, g: &[f64], bed: &[f64]) {
        assert!(g.iter().chain(bed).all(|m| m.is_finite()), "an edit puts a finite amount in cell {i}");
        let t = self.t_ground(i);
        let heat = {
            let (now_g, now_b) = (self.stock(Node::ground(i)), self.stock(Node::new(Class::Bed, i)));
            self.energy_of(i, t, |k| now_g(k) + g[k], |k| now_b(k) + bed[k]) - self.energy_of(i, t, &now_g, &now_b)
        };
        let some = |v: &[f64]| v.iter().enumerate().filter(|p| *p.1 != 0.0).map(|(k, &m)| (k, m)).collect::<Vec<_>>();
        let (g, bed) = (some(g), some(bed));
        if !bed.is_empty() {
            self.cross((Boundary::Outside, Node::new(Class::Bed, i)), bed, 0.0);
        }
        self.cross((Boundary::Outside, Node::ground(i)), g, heat);
    }
}

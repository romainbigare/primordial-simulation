//! The ground under a brush raised or lowered (handover.md 11: "Raise or lower land", a terrain brush). The matter comes
//! from the outside node and goes back to it (`World::cross`), with its heat at the ground's temperature and the
//! potential energy of where it lies, so the books close. A raise lays the cell's own rock (its bed's mixture) and, at
//! the top of a stroke, `Config::edit_top_m` of loose ground of one of the terrain's makes (`Make::of`); a lowering
//! takes the loose ground first, then the rock. Where in a cell the ground moved is kept on the terrain's samples
//! (`Relief::shaped`), which are built again over it when the edits settle (`World::settle`). The ground the brush
//! shaped comes to rest as it is laid: it spreads about the brush and slides till it stands no steeper than loose ground
//! rests at (`World::rest`).

use super::Brush;
use crate::rng::{Purpose, Rng};
use crate::store::{Boundary, Class, Node};
use crate::World;
use std::collections::BTreeMap;

/// What edits moved spreads by this, m² per m of a brush's radius and per m it shapes at its centre (`World::rest`).
const SPREAD: f64 = 0.2;
/// What edits moved spreads out to this many radii from a brush's centre, less toward there (`World::rest`).
const SPREAD_RADII: f64 = 2.0;
/// The most a layer of a paint's edits of the ground lays at their centres, as a share of their radius (`World::shape`).
const LAYER: f64 = 1.0 / 64.0;
/// The most layers of one paint's edits of the ground (`World::shape`): past it the layers are higher.
const LAYERS_MOST: f64 = 64.0;

/// An edit of the ground (`Act::Ground`): its brush, the height it lays at its centre, m (takes, where under none), what
/// its stroke raised there before, m, and the terrain's make of the loose ground it lays.
pub(super) struct Laid {
    pub b: Brush,
    pub height: f64,
    pub stroke: f64,
    pub make: usize,
}

impl World {
    /// The make of the loose ground the stroke `stroke` of a shell's edits lays (`Act::Ground`): one of the terrain's
    /// makes, drawn from the world's seed.
    pub fn draw_make(&self, stroke: u64) -> u8 {
        Rng::new(self.seed, stroke, 0, Purpose::Edit).below(self.terrain.makes.len().max(1)) as u8
    }

    /// The ground under the brushes of `laid` (the edits of the ground of one paint, in their order) raised, each by its
    /// height at its centre (lowered where under none) and by `falloff` of that at a distance d from it (in radii: a
    /// smooth bump, flat at its edge), over the samples it covers (`covered`). They are laid in layers, together no
    /// higher at their centres than `LAYER` of their radius, and after each layer the ground about them comes to rest
    /// once (`lay`, `rest`): the ground stands the same however much one paint lays, and a paint of many places costs
    /// one rest a layer. A stroke that has raised `stroke` m at a centre before lays the loose ground of its top
    /// (`lay`). Whether each moved any ground.
    pub(super) fn shape(&mut self, laid: &[Laid]) -> Vec<bool> {
        let high: f64 = laid.iter().map(|l| l.height.abs() / (LAYER * l.b.radius_m).max(f64::MIN_POSITIVE)).sum();
        let layers = high.ceil().clamp(1.0, LAYERS_MOST);
        let mut moved = vec![false; laid.len()];
        for n in 0..layers as usize {
            let mut resting = Vec::new();
            for (l, m) in laid.iter().zip(&mut moved) {
                let height = l.height / layers;
                if self.lay(l.b, (height, l.stroke + l.height.max(0.0) * n as f64 / layers), l.make) {
                    resting.push((l.b, height));
                    *m = true;
                }
            }
            if let (Some(last), false) = (laid.last(), resting.is_empty()) {
                self.rest(&resting, last.make);
            }
        }
        moved
    }

    /// A layer `height` m high at the centre of the brush `b` laid (taken where under none). Each cell takes the volume
    /// its samples move by; a raise is the share `loose` of loose ground of the make `make` (the part of the stroke's
    /// height at the centre, which has raised `stroke` m there before, within `edit_top_m` of its top) and the rest the
    /// cell's rock (`raise`); a lowering takes at most what the cell holds (`lower`), and its samples move by the share
    /// it took. Whether any ground moved.
    fn lay(&mut self, b: Brush, (height, stroke): (f64, f64), make: usize) -> bool {
        let s = self.terrain_samples();
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        for c in self.covered(b, s.across) {
            cells.entry(c.cell).or_default().push((c.k, height * super::falloff(c.d)));
        }
        drop(s);
        let top = self.cfg.edit_top_m;
        let loose = if height > 0.0 { ((stroke + height).min(top) - stroke.min(top)).max(0.0) / height } else { 0.0 };
        self.move_ground(&cells, loose, &self.mix_of(make))
    }

    /// The ground edits shaped since the samples were last built, worn as the start's wear would leave it worn for
    /// `Config::edit_wear_years` (`World::worn`; laws.md, Edits): a part of the edit, laid so by the hand, with no time
    /// on the world's clock. Each cell moves by its samples' heights (`move_ground`, from and to the outside node), the
    /// cells' own rock. Once, till edits shape the ground again (`Relief::worn`): `settle` runs it, and a shell may run
    /// it before to tell the relief the settle builds from.
    pub fn wear_edits(&mut self) {
        if !self.relief.due || self.relief.worn {
            return;
        }
        let cells = self.cells_of(&self.worn(), self.terrain_samples().across);
        self.move_ground(&cells, 0.0, &[]);
        self.relief.worn = true;
    }

    /// The samples `moved` (each with how far it moves, m; of samples `across` a row) by the cell each lies in.
    fn cells_of(&self, moved: &[(usize, f64)], across: usize) -> BTreeMap<usize, Vec<(usize, f64)>> {
        let mut cells: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        moved.iter().for_each(|&(k, dh)| cells.entry(self.cell_under(self.sample_place(k, across))).or_default().push((k, dh)));
        cells
    }

    /// The substances of the terrain's make `make` (`Make::of`), each with its share by volume; none where there is none.
    fn mix_of(&self, make: usize) -> Vec<(usize, f64)> {
        self.terrain.makes.get(make).map_or_else(Vec::new, |m| m.of.clone())
    }

    /// The ground the brushes of `laid` just shaped, each by its height at its centre, m, comes to rest (`World::slid`;
    /// laws.md, Edits): what edits moved spreads about each brush, out to `SPREAD_RADII` of its radius and less toward
    /// there (`falloff`), by `SPREAD` of its radius, m², for each m it shaped at its centre, and the ground slides till
    /// it stands no steeper than loose ground rests at. Each cell moves by its samples' heights (`move_ground`), what
    /// comes to it laid as loose ground of the make `make` (ground that slid lies loose at the foot of its slope).
    fn rest(&mut self, laid: &[(Brush, f64)], make: usize) {
        let across = self.terrain_samples().across;
        let givers: Vec<usize> = laid.iter().flat_map(|&(b, _)| self.spanned(b, across)).map(|p| p.0).collect();
        let spreads = |&(b, height): &(Brush, f64)| {
            let about = Brush { centre: b.centre, radius_m: SPREAD_RADII * b.radius_m };
            self.spanned(about, across).into_iter().map(move |(k, d)| (k, SPREAD * b.radius_m * height.abs() * super::falloff(d)))
        };
        let cells = self.cells_of(&self.slid(&givers, &laid.iter().flat_map(spreads).collect::<Vec<_>>()), across);
        self.move_ground(&cells, 1.0, &self.mix_of(make));
    }

    /// The ground of each cell of `cells` moved by its samples' heights, m (each sample and how far it moves; raised
    /// where above none): the cell takes the volume they move by, a raise the share `loose` of it as loose ground of the
    /// substances `mix` and the rest as its rock (`raise`), a lowering at most what it holds (`lower`), and its samples
    /// move by the share it took (`Relief::shape`; all of what they say where the cell's volume does not change, as
    /// where ground slides inside it). Whether any ground moved.
    pub(super) fn move_ground(&mut self, cells: &BTreeMap<usize, Vec<(usize, f64)>>, loose: f64, mix: &[(usize, f64)]) -> bool {
        let s = self.terrain_samples();
        let mut moved = false;
        for (&i, samples) in cells {
            let v: f64 = samples.iter().map(|&(k, dh)| dh * s.area(k)).sum();
            let got = if v >= 0.0 { self.raise(i, v, loose, mix) } else { -self.lower(i, -v) };
            let share = if v != 0.0 { got / v } else { 1.0 };
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

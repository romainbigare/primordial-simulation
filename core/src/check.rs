//! Conservation and sanity checks.

use crate::{store::{BOUNDARIES, Boundary, Class}, AIR, World};

#[derive(Clone, Debug)]
pub struct Totals {
    /// mol of each element
    pub elems: Vec<f64>,
    /// J
    pub thermal: f64,
    /// J, chemical energy held in substances (formation enthalpies, and the latent heat held in the air and the bodies' nodes)
    pub chemical: f64,
    /// J, potential energy gained since the start (the heat of `store::Class::Potential`)
    pub potential: f64,
    /// J, the energy each boundary node holds (`store::Boundary`): its heat and its amounts' formation enthalpy
    pub boundary_energy: [f64; BOUNDARIES],
    /// mol of each element the boundary nodes hold
    pub boundary_elems: Vec<f64>,
}

/// A sum that carries the rounding error of each addition (Neumaier): the world's chemical energy is about 100 times
/// its heat, so a plain sum would lose more to rounding than the laws move by mistake.
#[derive(Clone, Copy, Default)]
struct Sum(f64, f64);

impl Sum {
    fn add(&mut self, x: f64) {
        let y = self.0 + x;
        self.1 += crate::math::rounding_lost(self.0, x, y);
        self.0 = y;
    }
    fn get(self) -> f64 {
        self.0 + self.1
    }
}

pub fn totals(w: &World) -> Totals {
    let (chem, ns) = (&w.chem, w.chem.ns());
    // mol of each substance, and the energy held as latent heat in the air and the bodies' nodes
    let (mut subs, mut held, mut thermal) = (vec![Sum::default(); ns], Sum::default(), Sum::default());
    for (class, len) in [(Class::Ground, w.mesh.cells().n()), (Class::Bed, w.mesh.cells().n()), (Class::Air, w.mesh.level(AIR).n()), (Class::Deep, w.mesh.cells().n()), (Class::Bodies, w.mesh.cells().n())] {
        let (store, latent) = (w.store.amounts(class), matches!(class, Class::Air | Class::Bodies));
        // The air's amounts with what rounding took from their sums (`Store::air_carry`).
        let carry = if class == Class::Air { w.store.air_carry() } else { &[] };
        for (k, sum) in subs.iter_mut().enumerate() {
            let mut s = Sum::default();
            let at = k * len..(k + 1) * len;
            store.get(at.clone()).unwrap_or(&[]).iter().chain(carry.get(at).unwrap_or(&[])).for_each(|&a| s.add(a));
            sum.add(s.get());
            // Air and the bodies' nodes hold their gases' fusion and vaporisation enthalpy; the ground holds fusion in
            // `e_ground`.
            if latent {
                held.add(s.get() * chem.subs[k].latent_gas());
            }
        }
    }
    // The light set aside in the bodies' nodes counts as heat until they store it.
    [Class::Ground, Class::Air, Class::Bodies, Class::Deep].iter().flat_map(|&c| w.store.heat(c)).for_each(|&e| thermal.add(e));
    for b in w.life.alive() {
        b.store.iter().zip(subs.iter_mut()).for_each(|(&a, s)| s.add(a));
        thermal.add(b.e);
    }
    for p in w.parts.alive() {
        p.wall.iter().zip(&p.fill).zip(subs.iter_mut()).for_each(|((&a, &f), s)| s.add(a + f));
        thermal.add(p.e);
    }
    let mut elems = vec![0.0; chem.n_elements];
    let mut chemical = held;
    for (k, s) in subs.iter().enumerate() {
        chem.add_elems(k, s.get(), &mut elems);
        chemical.add(s.get() * chem.subs[k].hf);
    }
    let (mut boundary_energy, mut boundary_elems) = (std::array::from_fn(|b| w.store.heat(Class::Boundary)[b]), vec![0.0; chem.n_elements]);
    for (x, &a) in w.store.amounts(Class::Boundary).iter().enumerate() {
        let (k, b) = (x / BOUNDARIES, x % BOUNDARIES);
        chem.add_elems(k, a, &mut boundary_elems);
        boundary_energy[b] += a * chem.subs[k].hf;
    }
    Totals { elems, thermal: thermal.get(), chemical: chemical.get(), potential: w.store.heat(Class::Potential)[0], boundary_energy, boundary_elems }
}

/// The relative drift of element `j` from `start` to `now`, signed: its change less what the boundary nodes gave.
pub fn elem_drift(now: &Totals, start: &Totals, j: usize) -> f64 {
    (now.elems[j] - start.elems[j] - (start.boundary_elems[j] - now.boundary_elems[j])) / start.elems[j].abs().max(1.0)
}

/// Compare now with a start point: the world's elements and energy change only by what the boundary nodes gave it.
/// Returns the worst relative drift of elements and of energy.
pub fn drift(w: &World, start: &Totals) -> (f64, f64) {
    let now = totals(w);
    let worst_e = (0..start.elems.len()).map(|j| elem_drift(&now, start, j).abs()).fold(0.0, f64::max);
    let given: [f64; BOUNDARIES] = std::array::from_fn(|b| start.boundary_energy[b] - now.boundary_energy[b]);
    let change = (now.thermal + now.chemical + now.potential) - (start.thermal + start.chemical + start.potential);
    let scale = given[Boundary::Sun as usize].abs().max(start.thermal.abs()).max(1.0);
    (worst_e, (change - given.iter().sum::<f64>()).abs() / scale)
}

/// The first element of `v` that `ok` refuses, as `name[i] = value`.
fn first_bad<T: Copy + Into<f64>>(name: &str, v: &[T], ok: impl Fn(f64) -> bool) -> Option<String> {
    v.iter().position(|&x| !ok(x.into())).map(|i| format!("{name}[{i}] = {}", v[i].into()))
}

/// The first state value that is NaN or infinite, or below 0 where it cannot be: None for a sound world.
pub fn broken(w: &World) -> Option<String> {
    let (at_least_0, above_0) = (|x: f64| x.is_finite() && x >= 0.0, |x: f64| x.is_finite() && x > 0.0);
    let amounts = [("ground", w.store.amounts(Class::Ground)), ("bed", w.store.amounts(Class::Bed)), ("air", w.store.amounts(Class::Air)), ("deep", w.store.amounts(Class::Deep)), ("bodies", w.store.amounts(Class::Bodies))];
    // A heat may reach 0 J, where `store::apply` stops what takes from it, not below.
    let energies = [("e_ground", w.store.heat(Class::Ground)), ("e_air", w.store.heat(Class::Air)), ("e_deep", w.store.heat(Class::Deep)), ("e_bodies", w.store.heat(Class::Bodies)), ("light", &w.light), ("rained", &w.rained)];
    let f32s = [("rain", &w.rain), ("liquid_flux", &w.liquid_flux), ("canopy", &w.canopy), ("cleared", &w.links.cleared)];
    let found = amounts.into_iter().chain(energies).find_map(|(name, v)| first_bad(name, v, at_least_0));
    if let Some(e) = found.or_else(|| f32s.into_iter().find_map(|(name, v)| first_bad(name, v, at_least_0))) {
        return Some(e);
    }
    for (s, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        if !above_0(b.e) || !above_0(b.count) || !b.store.iter().all(|&x| at_least_0(x)) {
            return Some(format!("body {s}: energy {}, members {}, stores {:?}", b.e, b.count, b.store));
        }
    }
    if let Some(s) = w.life.bodies.iter().position(|b| b.alive && !b.rows.iter().all(|x| x.is_finite())) {
        return Some(format!("body {s}: a brain weight is not finite"));
    }
    for (s, p) in w.parts.parts.iter().enumerate().filter(|(_, p)| p.alive) {
        if !above_0(p.e) || !p.wall.iter().chain(&p.fill).all(|&x| at_least_0(x)) {
            return Some(format!("part {s}: energy {}, walls {:?}, fill {:?}", p.e, p.wall, p.fill));
        }
    }
    None
}

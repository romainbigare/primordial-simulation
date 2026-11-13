//! Conservation and sanity checks.

use crate::{store::{BOUNDARIES, Boundary, Class}, AIR, World};

#[derive(Clone, Debug)]
pub struct Totals {
    /// mol of each element
    pub elems: Vec<f64>,
    /// J
    pub thermal: f64,
    /// J, chemical energy held in substances (formation enthalpies, and the latent heat held in the air and the aside nodes)
    pub chemical: f64,
    /// J, the potential energy the world's matter gained since the start as the store books it (`Store::lifted`)
    pub lifted: f64,
    /// J, the potential energy of the world's matter as its nodes hold it (`flow::potential`); none before Flow built
    /// the state of the ground it reads
    pub potential: Option<f64>,
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
    // mol of each substance, and the energy held as latent heat in the air and the aside nodes
    let (mut subs, mut held, mut thermal) = (vec![Sum::default(); ns], Sum::default(), Sum::default());
    for (class, len) in [(Class::Ground, w.mesh.cells().n()), (Class::Bed, w.mesh.cells().n()), (Class::Air, w.mesh.level(AIR).n()), (Class::Deep, w.mesh.cells().n()), (Class::Aside, w.mesh.cells().n())] {
        let (store, latent) = (w.store.amounts(class), matches!(class, Class::Air | Class::Aside));
        for (k, sum) in subs.iter_mut().enumerate() {
            let mut s = Sum::default();
            store.get(k * len..(k + 1) * len).unwrap_or(&[]).iter().for_each(|&a| s.add(a));
            sum.add(s.get());
            // Air and the aside nodes hold their gases' fusion and vaporisation enthalpy; the ground holds fusion in
            // `e_ground`.
            if latent {
                held.add(s.get() * chem.subs[k].latent_gas());
            }
        }
    }
    // The light set aside for the cover counts as heat until it stores it.
    [Class::Ground, Class::Air, Class::Aside, Class::Deep].iter().flat_map(|&c| w.store.heat(c)).for_each(|&e| thermal.add(e));
    if let Some(p) = crate::cover::Parts::of(chem) {
        for c in w.life.cells.iter().map(|x| &x.cover) {
            c.all_mol().flat_map(|m| m.iter().zip(p.sub)).for_each(|(&a, k)| subs[k].add(a));
            thermal.add(c.e);
        }
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
    let potential = crate::flow::potential(w);
    Totals { elems, thermal: thermal.get(), chemical: chemical.get(), lifted: w.store.lifted(), potential, boundary_energy, boundary_elems }
}

/// The relative drift of element `j` from `start` to `now`, signed: its change less what the boundary nodes gave.
pub fn elem_drift(now: &Totals, start: &Totals, j: usize) -> f64 {
    (now.elems[j] - start.elems[j] - (start.boundary_elems[j] - now.boundary_elems[j])) / start.elems[j].abs().max(1.0)
}

/// The most the elements and the energy may drift since a start, relative (`drift`): the rounding of plain sums over a
/// long run.
pub const DRIFT: f64 = 1e-9;

/// Compare now with a start point: the world's elements and energy change only by what the boundary nodes gave it, its
/// potential energy as its nodes hold it (`Totals::potential`), so a move that books no lift or fall shows; as the store
/// booked it where Flow had built no state of the ground then. Returns the worst relative drift of elements and of
/// energy.
pub fn drift(w: &World, start: &Totals) -> (f64, f64) {
    let now = totals(w);
    let worst_e = (0..start.elems.len()).map(|j| elem_drift(&now, start, j).abs()).fold(0.0, f64::max);
    let given: [f64; BOUNDARIES] = std::array::from_fn(|b| start.boundary_energy[b] - now.boundary_energy[b]);
    let gained = match (now.potential, start.potential) {
        (Some(now), Some(start)) => now - start,
        _ => now.lifted - start.lifted,
    };
    let change = (now.thermal + now.chemical) - (start.thermal + start.chemical) + gained;
    let scale = given[Boundary::Sun as usize].abs().max(start.thermal.abs()).max(1.0);
    (worst_e, (change - given.iter().sum::<f64>()).abs() / scale)
}

/// The drift of the elements or the energy since `start`, if more than `DRIFT`.
pub fn drifted(w: &World, start: &Totals) -> Option<String> {
    let (de, dq) = drift(w, start);
    (de.max(dq) > DRIFT).then(|| format!("the books drifted: elements {de:.2e}, energy {dq:.2e}, over {DRIFT:e}"))
}

/// The first element of `v` that `ok` refuses, as `name[i] = value`.
fn first_bad<T: Copy + Into<f64>>(name: &str, v: &[T], ok: impl Fn(f64) -> bool) -> Option<String> {
    v.iter().position(|&x| !ok(x.into())).map(|i| format!("{name}[{i}] = {}", v[i].into()))
}

/// The first state value that is NaN or infinite, or below 0 where it cannot be: None for a sound world.
pub fn broken(w: &World) -> Option<String> {
    let at_least_0 = |x: f64| x.is_finite() && x >= 0.0;
    let amounts = [("ground", w.store.amounts(Class::Ground)), ("bed", w.store.amounts(Class::Bed)), ("air", w.store.amounts(Class::Air)), ("deep", w.store.amounts(Class::Deep)), ("aside", w.store.amounts(Class::Aside))];
    // A heat may reach 0 J, where `store::apply` stops what takes from it, not below.
    let energies = [("e_ground", w.store.heat(Class::Ground)), ("e_air", w.store.heat(Class::Air)), ("e_deep", w.store.heat(Class::Deep)), ("e_aside", w.store.heat(Class::Aside)), ("light", &w.light)];
    let f32s = [("rain", &w.rain), ("liquid_flux", &w.liquid_flux)];
    let found = amounts.into_iter().chain(energies).find_map(|(name, v)| first_bad(name, v, at_least_0));
    let found = found.or_else(|| f32s.into_iter().find_map(|(name, v)| first_bad(name, v, at_least_0)));
    let cover = w.life.cells.iter().enumerate().find_map(|(i, x)| {
        let c = &x.cover;
        let mol = first_bad("mol", c.mol.as_flattened(), at_least_0).or_else(|| first_bad("lying", c.lying.mol.as_flattened(), at_least_0));
        mol.or_else(|| first_bad("e", &[c.e], at_least_0)).or_else(|| first_bad("keep", &[c.keep], at_least_0)).map(|x| format!("the cover of cell {i}: {x}"))
    });
    found.or(cover)
}

//! Conservation and sanity checks.

use crate::World;

#[derive(Clone, Debug)]
pub struct Totals {
    /// mol of each element
    pub elems: Vec<f64>,
    /// J
    pub thermal: f64,
    /// J, chemical energy held in substances (formation enthalpies, and the latent heat held in the air)
    pub chemical: f64,
}

/// A sum that carries the rounding error of each addition (Neumaier): the chemical energy of
/// the world is about 100 times its heat, so a plain sum over many bodies loses more to rounding
/// than the laws ever move by mistake.
#[derive(Clone, Copy, Default)]
struct Sum(f64, f64);

impl Sum {
    fn add(&mut self, x: f64) {
        let y = self.0 + x;
        self.1 += if self.0.abs() >= x.abs() { (self.0 - y) + x } else { (x - y) + self.0 };
        self.0 = y;
    }
    fn get(self) -> f64 {
        self.0 + self.1
    }
}

pub fn totals(w: &World) -> Totals {
    let (chem, ns) = (&w.chem, w.chem.ns());
    // mol of each substance, and the energy held as latent heat in the air
    let (mut subs, mut held, mut thermal) = (vec![Sum::default(); ns], Sum::default(), Sum::default());
    for (store, len, air) in [(&w.ground, w.grid.n(), false), (&w.bed, w.grid.n(), false), (&w.air, w.grid.na(), true)] {
        for (k, sum) in subs.iter_mut().enumerate() {
            let mut s = Sum::default();
            store[k * len..(k + 1) * len].iter().for_each(|&a| s.add(a));
            sum.add(s.get());
            // Air holds its gases' fusion and vaporisation enthalpy; the ground holds fusion in `e_ground`.
            if air {
                held.add(s.get() * (chem.subs[k].lf + chem.subs[k].lv));
            }
        }
    }
    w.e_ground.iter().chain(&w.e_air).for_each(|&e| thermal.add(e));
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
    Totals { elems, thermal: thermal.get(), chemical: chemical.get() }
}

/// Compare now with a start point. Returns the worst relative drift of elements and of energy.
pub fn drift(w: &World, start: &Totals, start_ledger: &crate::Ledger) -> (f64, f64) {
    let now = totals(w);
    let l = &w.ledger;
    let mut worst_e: f64 = 0.0;
    for j in 0..start.elems.len() {
        let imposed = l.imposed_elems.get(j).copied().unwrap_or(0.0) - start_ledger.imposed_elems.get(j).copied().unwrap_or(0.0);
        let d = (now.elems[j] - start.elems[j] - imposed).abs() / start.elems[j].abs().max(1.0);
        worst_e = worst_e.max(d);
    }
    let boundary = (l.sun_in - start_ledger.sun_in) - (l.space_out - start_ledger.space_out)
        + (l.gravity - start_ledger.gravity)
        + (l.imposed_energy - start_ledger.imposed_energy);
    let change = (now.thermal + now.chemical) - (start.thermal + start.chemical);
    let scale = (l.sun_in - start_ledger.sun_in).abs().max(start.thermal.abs()).max(1.0);
    (worst_e, (change - boundary).abs() / scale)
}

/// True if any amount or energy is NaN, infinite or negative.
pub fn broken(w: &World) -> Option<String> {
    for (name, v) in [("ground", &w.ground), ("bed", &w.bed), ("air", &w.air)] {
        if let Some(i) = v.iter().position(|x| !x.is_finite() || *x < 0.0) {
            return Some(format!("{name}[{i}] = {}", v[i]));
        }
    }
    for (name, v) in [("e_ground", &w.e_ground), ("e_air", &w.e_air)] {
        if let Some(i) = v.iter().position(|x| !x.is_finite() || *x <= 0.0) {
            return Some(format!("{name}[{i}] = {}", v[i]));
        }
    }
    for (s, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        if !b.e.is_finite() || b.e <= 0.0 || b.store.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Some(format!("body {s}: energy {}, stores {:?}", b.e, b.store));
        }
    }
    for (s, p) in w.parts.parts.iter().enumerate().filter(|(_, p)| p.alive) {
        if !p.e.is_finite() || p.e <= 0.0 || p.wall.iter().chain(&p.fill).any(|x| !x.is_finite() || *x < 0.0) {
            return Some(format!("part {s}: energy {}, walls {:?}, fill {:?}", p.e, p.wall, p.fill));
        }
    }
    None
}

/// FNV-1a over the bits of the clock and every state array. Equal states give equal hashes.
pub fn hash(w: &World) -> u64 {
    fn eat(h: &mut u64, x: u64) {
        for b in x.to_le_bytes() {
            *h = (*h ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    eat(&mut h, w.day);
    for v in [&w.ground, &w.bed, &w.e_ground, &w.air, &w.e_air] {
        for x in v {
            eat(&mut h, x.to_bits());
        }
    }
    for b in w.life.alive() {
        eat(&mut h, b.cell as u64);
        if b.count != 1.0 {
            eat(&mut h, b.count.to_bits());
        }
        for x in b.store.iter().chain([&b.e]) {
            eat(&mut h, x.to_bits());
        }
    }
    for x in w.life.groups.iter().flat_map(|g| &g.rows) {
        eat(&mut h, x.to_bits() as u64);
    }
    for p in w.parts.alive() {
        eat(&mut h, p.cell as u64 | (p.links.len() as u64) << 32);
        for x in p.wall.iter().chain(&p.fill).chain([&p.e]) {
            eat(&mut h, x.to_bits());
        }
    }
    h
}

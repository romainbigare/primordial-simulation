//! Conservation and sanity checks.

use crate::World;

#[derive(Clone, Debug)]
pub struct Totals {
    /// mol of each element
    pub elems: Vec<f64>,
    /// J
    pub thermal: f64,
    /// J, chemical energy held in substances (from formation enthalpies)
    pub chemical: f64,
}

pub fn totals(w: &World) -> Totals {
    let chem = &w.chem;
    let mut elems = vec![0.0; chem.n_elements];
    let mut chemical = 0.0;
    for (store, len) in [(&w.ground, w.grid.n()), (&w.bed, w.grid.n()), (&w.air, w.grid.na())] {
        for k in 0..chem.ns() {
            let sum: f64 = store[k * len..(k + 1) * len].iter().sum();
            chem.add_elems(k, sum, &mut elems);
            chemical += sum * chem.subs[k].hf;
        }
    }
    let thermal = w.e_ground.iter().sum::<f64>() + w.e_air.iter().sum::<f64>();
    Totals { elems, thermal, chemical }
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
    h
}

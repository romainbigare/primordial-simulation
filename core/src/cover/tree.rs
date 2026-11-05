//! The cover of the cells a split or a join makes (laws.md 4.5): a split's quarters are flat copies of their parent, so
//! each takes its share by area of each slot and of the heat (`Cover::quarter`); a joined cell takes the slots of its
//! quarters as seeds that land by the cover's own rule (`Cover::gathered`, then `cover::land`).

use super::{Cover, Parts, Seed, NP};
use crate::chem::Chemistry;
use crate::remap::mean_by;

impl Cover {
    /// The share `a` of it, for a quarter of `a` of its cell's area: each slot's amounts and the heat by that share, its
    /// genes, starting kind and whether it is full as they are, its light mark (per m²) and the share set aside as they
    /// are.
    pub(crate) fn quarter(&self, a: f64) -> Cover {
        Cover { mol: self.mol.map(|m| m.map(|x| x * a)), e: self.e * a, seeds: Vec::new(), ..self.clone() }
    }

    /// The cover of cell `to` joined of quarters `kids` (row by row, each `area[q]` of its area): their light marks (per
    /// m²) and shares set aside the means by area; each slot that holds cover, quarter by quarter, a seed for it with its
    /// heat (the slot's heat capacity at its quarter's temperature, the quarter's last held slot what is left of its
    /// heat), which `cover::land` lodges as it lodges seeds (`seed::lodge`). The heat of a quarter with no slot held
    /// stays in the cover.
    pub(crate) fn gathered(kids: [&Cover; 4], area: [f64; 4], chem: &Chemistry, to: usize) -> Cover {
        let mut c = Cover { mark: mean_by(kids.map(|k| k.mark), area), keep: mean_by(kids.map(|k| k.keep as f64), area) as f32, ..Cover::default() };
        let Some(p) = Parts::of(chem) else { return c };
        for k in kids {
            let (t, held): (f64, Vec<usize>) = (k.temp(chem, &p), k.held().collect());
            let mut left = k.e;
            for (n, &x) in held.iter().enumerate() {
                let heat = if n + 1 == held.len() { left } else { ((0..NP).map(|q| k.mol[x][q] * chem.subs[p.sub[q]].cp).sum::<f64>() * t).min(left) };
                left -= heat;
                c.seeds.push(Seed { to: to as u32, mol: k.mol[x], genes: k.genes[x], root: k.root[x], heat });
            }
            c.e += left;
        }
        c
    }
}

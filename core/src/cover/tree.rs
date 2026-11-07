//! The cover of the cells a split or a join makes (laws.md 4.5): each quarter of a split takes its part of each slot,
//! by the share of its ground the slot's kind likes (`Cover::parts`, `inside::Inside::kind`), with the heat that goes
//! with it (`Cover::quarter`); a joined cell takes the slots of its quarters as seeds that land by the cover's own rule
//! (`Cover::gathered`, then `cover::land`).

use super::{Cover, Parts, Seed, MOIST, NO_ROOT, NP, SLOTS};
use crate::chem::Chemistry;
use crate::inside::{Ground, Inside};
use crate::remap::mean_by;
use crate::Physics;

impl Cover {
    /// The part of each slot of it that each quarter of its cell takes (row by row, slot by slot): the quarter's part of
    /// the share of the slot's kind over the cell's ground `g` (`Inside::kind`, by its moisture gene; `Ground::quarters`),
    /// an empty slot's by area.
    pub(crate) fn parts(&self, g: &Ground, inside: &Inside, ph: &Physics) -> [[f64; SLOTS]; 4] {
        let each: [[f64; 4]; SLOTS] = std::array::from_fn(|x| {
            let share = if self.root[x] == NO_ROOT { vec![1.0; g.area.len()] } else { inside.kind(g, ph, self.genes[x][MOIST]) };
            g.quarters(&share)
        });
        std::array::from_fn(|q| each.map(|p| p[q]))
    }
    /// What a quarter of its cell takes of it, of `a` of the cell's area: the part `f[x]` of each slot's amounts
    /// (`parts`), the heat by its share of the heat capacity (so it stays at the cover's temperature; by area where it
    /// holds none), its genes, starting kind and whether it is full as they are, its light mark (per m²) and the share set
    /// aside as they are.
    pub(crate) fn quarter(&self, f: [f64; SLOTS], a: f64, chem: &Chemistry) -> Cover {
        let mut c = Cover { mol: std::array::from_fn(|x| self.mol[x].map(|m| m * f[x])), seeds: Vec::new(), ..self.clone() };
        let cap = |x: &Cover| Parts::of(chem).map_or(0.0, |p| x.cap(chem, &p));
        let whole = cap(self);
        c.e = if whole > 0.0 { self.e * (cap(&c) / whole) } else { self.e * a };
        c
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

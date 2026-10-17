//! Made objects (parts), their bonds and structures, and the bodies inside walls.

use super::Row;
use crate::rec::Recorder;
use pcore::bond::{self, Part};
use pcore::World;

/// A structure counts as enclosed from this closure of its walls (`Info::encl`).
const ENCLOSED: f32 = 0.5;

impl Recorder {
    pub(super) fn parts_cols(&self, w: &World, row: &mut Row) {
        let (parts, chem, y) = (&w.parts, &w.chem, &self.year);
        let roots: Vec<&Part> = parts.parts.iter().enumerate().filter(|(r, p)| p.alive && p.root as usize == *r).map(|p| p.1).collect();
        let links = |g: bool| parts.alive().flat_map(|p| &p.links).filter(|l| (l.0 == bond::GROUND) == g).count() as f64;
        let filled: Vec<f64> = parts.alive().map(|p| p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.molar_mass).sum::<f64>()).filter(|&m| m > 0.0).collect();
        let (t, share) = (&y.parts, |c: [f64; 2]| c[1] / c[0].max(1.0));
        row.push("parts", "1", parts.alive().count() as f64);
        row.push("parts_held", "1", parts.alive().filter(|p| p.held != bond::NOBODY).count() as f64);
        row.push("parts_mass", "t", parts.alive().map(|p| p.mass(chem)).sum::<f64>() / 1e3);
        row.push("parts_formed", "1/yr", t.formed);
        row.push("put_to_parts", "kg/yr", t.to_parts);
        row.push("taken_from_parts", "kg/yr", t.from_parts);
        row.push("bonds_joined", "1", links(false) / 2.0);
        row.push("bonds_ground", "1", links(true));
        row.push("structures", "1", roots.iter().filter(|r| r.s.parts > 1).count() as f64);
        row.push("enclosed", "1", roots.iter().filter(|r| r.s.encl >= ENCLOSED).count() as f64);
        for (name, x) in ["made_held", "made_joined", "made_ground"].into_iter().zip(y.made) {
            row.push(name, "1/yr", x);
        }
        row.push("bonds_broken", "1/yr", y.broken);
        row.push("bonds_failed", "1/yr", y.failed);
        // Bodies inside walls now; the share of the body-months of the others inside, in the cold and the warm, and
        // their cold months.
        row.push("inside_now", "1", w.life.alive().filter(|b| bond::room(w, b).is_some()).count() as f64);
        row.push("inside_cold", "1", share(y.inside[0]));
        row.push("inside_warm", "1", share(y.inside[1]));
        row.push("cold_months", "1/yr", y.inside[0][0]);
        row.push("holding_liquid", "1", filled.len() as f64);
        row.push("liquid_held", "kg", filled.iter().sum());
        row.push("afloat", "1", roots.iter().filter(|r| r.s.afloat).count() as f64);
        row.push("afloat_moves", "1/yr", t.afloat);
    }
}

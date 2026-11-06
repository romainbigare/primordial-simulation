//! Made objects (parts), their bonds and structures, and the bodies inside walls.

use super::Row;
use crate::rec::Recorder;
use pcore::bond::{self, Part};
use pcore::World;

/// A structure counts as enclosed from this closure of its walls (`Enclosure::closed`).
pub const ENCLOSED: f32 = 0.5;

impl Recorder {
    pub(super) fn parts_cols(&self, w: &World, row: &mut Row) {
        let (chem, y) = (&w.chem, &self.year);
        let roots: Vec<&Part> = w.life.parts().filter(|p| p.root == p.id).collect();
        let links = (0..w.mesh.cells().n()).map(|i| w.life.cell_at(i).bonds.count()).sum::<usize>() as f64;
        // Of each lot holding liquid: its parts and the liquid they hold, kg.
        let filled: Vec<[f64; 2]> = w.life.parts().map(|p| [p.count, p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.molar_mass).sum::<f64>()]).filter(|m| m[1] > 0.0).collect();
        let (t, share) = (&y.parts, |c: [f64; 2]| c[1] / c[0].max(1.0));
        // Parts counted one by one (a lot by its count), and the records that hold them.
        let count = |f: &dyn Fn(&Part) -> bool| w.life.parts().filter(|p| f(p)).map(|p| p.count).sum::<f64>();
        row.push("parts", "1", count(&|_| true));
        row.push("part_records", "1", w.life.parts().count() as f64);
        row.push("parts_held", "1", count(&|p| p.held != bond::NOBODY));
        row.push("parts_mass", "t", w.life.parts().map(|p| p.mass(chem)).sum::<f64>() / 1e3);
        row.push("parts_formed", "1/yr", t.formed);
        row.push("put_to_parts", "kg/yr", t.to_parts);
        row.push("taken_from_parts", "kg/yr", t.from_parts);
        row.push("edge_kg", "kg/yr", t.edged);
        row.push("bonds_joined", "1", links);
        row.push("structures", "1", roots.iter().filter(|r| r.s.parts > 1).map(|r| r.count).sum::<f64>());
        row.push("enclosed", "1", roots.iter().filter(|r| r.s.encl.closed >= ENCLOSED).map(|r| r.count).sum::<f64>());
        for (name, x) in ["made_held", "made_joined"].into_iter().zip(y.made) {
            row.push(name, "1/yr", x);
        }
        row.push("bonds_broken", "1/yr", y.broken);
        row.push("bonds_failed", "1/yr", y.failed);
        // Bodies inside walls now; the share of the body-months of the others inside, in the cold and the warm, and
        // their cold months.
        row.push("inside_now", "1", w.life.alive().filter(|b| bond::room(w.life.cell_at(b.cell as usize), b).is_some()).count() as f64);
        row.push("inside_cold", "1", share(y.inside[0]));
        row.push("inside_warm", "1", share(y.inside[1]));
        row.push("cold_months", "1/yr", y.inside[0][0]);
        row.push("holding_liquid", "1", filled.iter().map(|m| m[0]).sum::<f64>());
        row.push("liquid_held", "kg", filled.iter().map(|m| m[1]).sum::<f64>());
        self.form_cols(row);
    }

    /// Who drew "form" in the year (members): by starting kind (every kind seen so far keeps its column), by size
    /// class of the brain, and by whether "form" had the top score.
    fn form_cols(&self, row: &mut Row) {
        let f = &self.year.forms;
        let seen = self.files.series.iter().filter(|c| c.name.strip_prefix("form_").is_some_and(|k| k.parse::<usize>().is_ok())).count();
        for k in 0..f.by_kind.len().max(seen) {
            row.push(format!("form_{k}"), "1/yr", f.by_kind.get(k).copied().unwrap_or(0.0));
        }
        for (c, &x) in f.by_size.iter().enumerate() {
            row.push(format!("form_size_{c}"), "1/yr", x);
        }
        row.push("form_learned", "1/yr", f.by_score[0]);
        row.push("form_chance", "1/yr", f.by_score[1]);
    }
}

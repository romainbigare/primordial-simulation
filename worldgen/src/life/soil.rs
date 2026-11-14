//! The soil: the mineral the loose ground of the land gets when life starts.

use super::open_land;
use pcore::store::{Boundary, Node};
use pcore::World;

/// Give the loose ground of the open land the soil `soil` (substance, mol per m² of a cell's land, `open_land`) now,
/// when life starts, as an edit at each cell's temperature: placed at year 0, rain would wash it into the basins before
/// any body. Returns one line.
pub fn soil(w: &mut World, soil: &[(usize, f64)]) -> String {
    // The standing liquid as Flow's state of the world now has it (`contract::env_of`).
    pcore::flow::state(w, true);
    let land = open_land(w);
    let all: f64 = land.iter().map(|&(_, m2)| m2).sum();
    let mut said = Vec::new();
    for &(s, per) in soil {
        for &(i, m2) in &land {
            let a = per * m2;
            let e = a * w.chem.subs[s].cp * w.t_ground(i);
            w.cross((Boundary::Outside, Node::ground(i)), [(s, a)], e);
        }
        said.push(format!("{per:.3} mol per m2 of substance {s} ({:.1} g per m2, {:.3e} kg in all)", per * w.chem.subs[s].molar_mass * 1000.0, per * all * w.chem.subs[s].molar_mass));
    }
    format!("year {:.1}: soil on {} land cells (not under standing liquid): {}", w.year(), land.len(), said.join(", "))
}

//! The soil: the mineral the loose ground of the land gets when life starts.

use super::open_land;
use pcore::store::{Boundary, Node};
use pcore::World;

/// Give the loose ground of the open land the soil `soil` (substance, mol per m²) now, when life starts, as an edit at
/// each cell's temperature: placed at year 0, rain would wash it into the basins before any body. Returns one line.
pub fn soil(w: &mut World, soil: &[(usize, f64)]) -> String {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area().to_vec());
    let land: Vec<usize> = (0..n).filter(|&i| open_land(w, i)).collect();
    let land_area: f64 = land.iter().map(|&i| area[i]).sum();
    let mut said = Vec::new();
    for &(s, per) in soil {
        for &i in &land {
            let a = per * area[i];
            let e = a * w.chem.subs[s].cp * w.t_ground(i);
            w.cross((Boundary::Outside, Node::ground(i)), [(s, a)], e);
        }
        said.push(format!("{per:.3} mol per m2 of substance {s} ({:.1} g per m2, {:.3e} kg in all)", per * w.chem.subs[s].molar_mass * 1000.0, per * land_area * w.chem.subs[s].molar_mass));
    }
    format!("year {:.1}: soil on {} land cells (not under standing liquid): {}", w.year(), land.len(), said.join(", "))
}

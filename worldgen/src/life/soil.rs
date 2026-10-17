//! The soil: the mineral the loose ground of the land gets when life starts.

use super::open_land;
use pcore::store::Boundary;
use pcore::World;

/// Give the loose ground of every land cell the soil `soil` (substance, mol per m²: `Init::soil`, the mineral of the
/// fixing reaction's gas in chemistry 0) at this moment, which is when life starts: booked as new matter and energy
/// like an edit, at the cell's own temperature. Only the land founders are placed on gets it (`open_land`: not a basin
/// of standing liquid). It is not placed at year 0: the first years of a world are the liquid's, before any body, and
/// rain washes a dissolved mineral from the slopes into the basins in the first year.
/// Returns what was placed, in one line.
pub fn soil(w: &mut World, soil: &[(usize, f64)]) -> String {
    let (n, area) = (w.grid.n(), w.grid.area());
    let land: Vec<usize> = (0..n).filter(|&i| open_land(w, i)).collect();
    let mut said = Vec::new();
    for &(s, per) in soil {
        let a = per * area;
        for &i in &land {
            let e = a * w.chem.subs[s].cp * w.t_ground(i);
            w.ground[s * n + i] += a;
            w.e_ground[i] += e;
            w.cross(Boundary::Outside, [(s, a)], e);
        }
        said.push(format!("{per:.3} mol per m2 of substance {s} ({:.1} g per m2, {:.3e} kg in all)", per * w.chem.subs[s].molar_mass * 1000.0, a * land.len() as f64 * w.chem.subs[s].molar_mass));
    }
    format!("year {:.1}: soil on {} land cells (not under standing liquid): {}", w.year(), land.len(), said.join(", "))
}

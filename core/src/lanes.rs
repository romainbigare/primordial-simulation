//! Lanes: ground cleared of bodies along each link (`Links::cleared`; laws.md 1.7): movers clear it, it eases the step,
//! and bodies spread back into it yearly.

use crate::World;

/// Body mass `kg` over `area` m², over `Physics::rho_pass`: how thick bodies are to pass through.
pub(crate) fn thickness(w: &World, kg: f64, area: f64) -> f64 {
    kg / area / w.cfg.phys.rho_pass
}

/// The link between the neighbouring cells `i` and `j`.
pub(crate) fn link(w: &World, i: usize, j: usize) -> usize {
    w.mesh.cells().link(i, j).expect("a mover steps to a neighbour")
}

/// How much of the lane of link `l` is cleared: the cleared ground over `Physics::lane` x the distance, at most one.
fn open(w: &World, l: usize) -> f64 {
    (w.links.cleared[l] as f64 / (w.cfg.phys.lane * w.mesh.cells().dist()[l])).min(1.0)
}

/// A mover from `i` to `j` cleared `m2` of ground, up to the whole lane (`Physics::lane` × the distance).
pub(crate) fn clear(w: &mut World, i: usize, j: usize, m2: f64) {
    let l = link(w, i, j);
    let lane = (w.cfg.phys.lane * w.mesh.cells().dist()[l]) as f32;
    let e = &mut w.links.cleared[l];
    *e = (*e + m2 as f32).min(lane);
}

/// What it costs, against bare ground, to step from `i` into `j`: one, plus `j`'s body mass per area over `rho_pass`
/// for the share of the passage not yet cleared.
pub(crate) fn resist(w: &World, i: usize, j: usize) -> f64 {
    1.0 + (1.0 - open(w, link(w, i, j))) * thickness(w, w.life.sum(j)[1], w.mesh.cells().area()[j])
}

/// How much cheaper link `l` is than uncleared ground, for the mean cover of its two cells (1 when bare or uncleared).
pub fn eased(w: &World, l: usize) -> f64 {
    if w.links.cleared[l] <= 0.0 {
        return 1.0;
    }
    let cells = w.mesh.cells();
    let (i, j) = (cells.a()[l] as usize, cells.b()[l] as usize);
    let c = thickness(w, (w.life.sum(i)[1] + w.life.sum(j)[1]) / 2.0, (cells.area()[i] + cells.area()[j]) / 2.0);
    (1.0 + c) / (1.0 + (1.0 - open(w, l)) * c)
}

/// Yearly: bodies spread back into every cleared link from both sides, at `regrow` times the cover over `rho_pass`
/// (all of it when dense), over `dt`.
pub fn regrow(w: &mut World, dt: f64) {
    for l in 0..w.links.cleared.len() {
        if w.links.cleared[l] <= 0.0 {
            continue;
        }
        let cells = w.mesh.cells();
        let (i, j, dist) = (cells.a()[l] as usize, cells.b()[l] as usize, cells.dist()[l]);
        let fill = thickness(w, (w.life.sum(i)[1] + w.life.sum(j)[1]) / 2.0, (cells.area()[i] + cells.area()[j]) / 2.0).min(1.0);
        let a = &mut w.links.cleared[l];
        *a = (*a - (2.0 * w.cfg.regrow * fill * dist * dt) as f32).max(0.0);
    }
}

//! Lanes: ground cleared of bodies along each link between two cells (`Links::cleared`). Movers pushing through bodies
//! clear it (`clear`, from `choose::go`), which makes the step cheaper (`resist`, `eased`), and bodies spread back into
//! it (`regrow`, yearly from Flow).

use crate::World;

/// Body mass `kg` per area of a cell, over `Physics::rho_pass`: how thick bodies are to pass through.
pub(crate) fn thickness(w: &World, kg: f64) -> f64 {
    kg / w.grid.area() / w.cfg.phys.rho_pass
}

/// The link between the neighbouring cells `i` and `j`.
fn link(w: &World, i: usize, j: usize) -> usize {
    w.grid.mesh().cells().link(i, j).expect("a mover steps to a neighbour")
}

/// How much of the lane of link `l` is cleared: the cleared ground over `Physics::lane` x the distance, at most one.
fn open(w: &World, l: usize) -> f64 {
    (w.links.cleared[l] as f64 / (w.cfg.phys.lane * w.grid.mesh().cells().dist()[l])).min(1.0)
}

/// A mover from `i` to its neighbour `j` cleared `m2` of ground: the link is cleared up to its whole lane
/// (`Physics::lane` x the distance).
pub(crate) fn clear(w: &mut World, i: usize, j: usize, m2: f64) {
    let l = link(w, i, j);
    let lane = (w.cfg.phys.lane * w.grid.mesh().cells().dist()[l]) as f32;
    let e = &mut w.links.cleared[l];
    *e = (*e + m2 as f32).min(lane);
}

/// What it costs, against bare ground, to step from `i` into its neighbour `j`: one, plus the body mass per area in
/// `j` over `Physics::rho_pass` for the share of the passage not yet cleared.
pub(crate) fn resist(w: &World, i: usize, j: usize) -> f64 {
    1.0 + (1.0 - open(w, link(w, i, j))) * thickness(w, w.life.sum[j][1])
}

/// How much cheaper link `l` is than uncleared ground, for the mean cover of its two cells (1 when bare or not
/// cleared).
pub fn eased(w: &World, l: usize) -> f64 {
    if w.links.cleared[l] <= 0.0 {
        return 1.0;
    }
    let cells = w.grid.mesh().cells();
    let (i, j) = (cells.a()[l] as usize, cells.b()[l] as usize);
    let c = thickness(w, (w.life.sum[i][1] + w.life.sum[j][1]) / 2.0);
    (1.0 + c) / (1.0 + (1.0 - open(w, l)) * c)
}

/// Yearly (from Flow): bodies spread back into the cleared ground of every link from both sides, at `Config::regrow`
/// times the cover there over `Physics::rho_pass` (all of it when dense), over the time `dt`.
pub fn regrow(w: &mut World, dt: f64) {
    for l in 0..w.links.cleared.len() {
        if w.links.cleared[l] <= 0.0 {
            continue;
        }
        let cells = w.grid.mesh().cells();
        let (i, j, dist) = (cells.a()[l] as usize, cells.b()[l] as usize, cells.dist()[l]);
        let fill = thickness(w, (w.life.sum[i][1] + w.life.sum[j][1]) / 2.0).min(1.0);
        let a = &mut w.links.cleared[l];
        *a = (*a - (2.0 * w.cfg.regrow * fill * dist * dt) as f32).max(0.0);
    }
}

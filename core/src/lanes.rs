//! Lanes: ground cleared of bodies along each edge between two cells (`World::pass`). Movers pushing through bodies
//! clear it (`clear`, from `choose::go`), which makes the step cheaper (`resist`, `eased`), and bodies spread back into
//! it (`regrow`, yearly from Flow).

use crate::grid::{DIRS, FORWARD};
use crate::World;

/// Body mass `kg` per area of a cell, over `Physics::rho_pass`: how thick bodies are to pass through.
pub(crate) fn thickness(w: &World, kg: f64) -> f64 {
    kg / w.grid.area() / w.cfg.phys.rho_pass
}

/// The edge from cell `i` along `DIRS[d]` to `j` in `World::pass`: each edge once, as the forward edge of its lower end.
fn edge(i: usize, d: usize, j: usize) -> usize {
    // `FORWARD` index of each of the first four `DIRS` (the other four are their reverses).
    const F: [usize; 4] = [0, 2, 1, 3];
    if d < 4 { 4 * i + F[d] } else { 4 * j + F[d - 4] }
}

/// A mover from `i` along `DIRS[d]` to `j` cleared `m2` of ground: the edge is cleared up to its whole lane
/// (`Physics::lane` x the distance).
pub(crate) fn clear(w: &mut World, i: usize, d: usize, j: usize, m2: f64) {
    w.pass.resize(4 * w.grid.n(), 0.0);
    let e = &mut w.pass[edge(i, d, j)];
    *e = (*e + m2 as f32).min((w.cfg.phys.lane * w.grid.dist(DIRS[d])) as f32);
}

/// What it costs, against bare ground, to step from `i` along `DIRS[d]` into `j`: one, plus the body mass per area in
/// `j` over `Physics::rho_pass` for the share of the passage not yet cleared.
pub(crate) fn resist(w: &World, i: usize, d: usize, j: usize) -> f64 {
    let open = w.pass.get(edge(i, d, j)).map_or(0.0, |&a| (a as f64 / (w.cfg.phys.lane * w.grid.dist(DIRS[d]))).min(1.0));
    1.0 + (1.0 - open) * thickness(w, w.life.sum[j][1])
}

/// How much cheaper the edge from `i` along `FORWARD[f]` is than uncleared ground, for the mean cover of its two cells
/// (1 when bare or not cleared).
pub fn eased(w: &World, i: usize, f: usize) -> f64 {
    let Some(j) = w.grid.step(i, FORWARD[f]).filter(|_| !w.pass.is_empty()) else { return 1.0 };
    let (a, c) = (w.pass.get(4 * i + f).copied().unwrap_or(0.0) as f64, thickness(w, (w.life.sum[i][1] + w.life.sum[j][1]) / 2.0));
    (1.0 + c) / (1.0 + (1.0 - (a / (w.cfg.phys.lane * w.grid.dist(FORWARD[f]))).min(1.0)) * c)
}

/// Yearly (from Flow): bodies spread back into the cleared ground of every edge from both sides, at `Config::regrow`
/// times the cover there over `Physics::rho_pass` (all of it when dense), over the time `dt`.
pub fn regrow(w: &mut World, dt: f64) {
    for i in 0..w.pass.len() / 4 {
        for (f, &off) in FORWARD.iter().enumerate() {
            if w.pass[4 * i + f] <= 0.0 {
                continue;
            }
            let Some(j) = w.grid.step(i, off) else { continue };
            let fill = thickness(w, (w.life.sum[i][1] + w.life.sum[j][1]) / 2.0).min(1.0);
            let a = &mut w.pass[4 * i + f];
            *a = (*a - (2.0 * w.cfg.regrow * fill * w.grid.dist(off) * dt) as f32).max(0.0);
        }
    }
}

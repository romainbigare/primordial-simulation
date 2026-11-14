//! The part of the open low ground at the map's edge where the edges are open (laws.md 1.2, 1.6): it stands at the
//! planet's level, what it lacks coming in from the boundary for outside (`come_in`).

use super::super::bodies::Body;
use super::{soaks, Crossings, Ground, Held};
use crate::{space::wear::STAND_M, store::Boundary};

/// What the parts `parts` of a body hold standing at `level` m, with what the pores of the ground they newly cover
/// take there, m³ (`body_level` the other way).
fn volume_at(parts: &[Ground], level: f64) -> f64 {
    parts.iter().map(|g| g.0.volume(level - g.1) + soaks(g, g.0.under(level - g.1))).sum()
}

/// The part of the open low ground at the map's edge of a world whose edges are open (`Body::edge`,
/// `World::edges_open`) stands at the planet's level, the level the world is made with (`STAND_M`): what its parts
/// `parts` hold short of that comes in at its cells at the edge, by the area their parts cover, of the liquid and heat
/// those cells hold, mixed (the planet's strip holds no liquid); booked from the boundary for outside (`edge`). Returns
/// the volume that came in, m³.
pub(super) fn come_in(body: &Body, parts: &[Ground], h: &mut Held, edge: &mut Crossings) -> f64 {
    let held: f64 = body.cells.iter().zip(&body.part).map(|(&i, &q)| h.part[i as usize][q]).sum();
    let (lack, width) = (volume_at(parts, STAND_M) - held, h.width);
    let (mut mixed, mut vol, mut area) = (vec![0.0; width], 0.0, 0.0);
    for &c in &body.edge {
        let i = body.cells[c] as usize;
        (0..width).for_each(|r| mixed[r] += h.q[i * width + r]);
        (vol, area) = (vol + h.vol[i], area + parts[c].2);
    }
    if lack <= 0.0 || vol <= 0.0 {
        return 0.0;
    }
    for &c in &body.edge {
        let (i, q) = (body.cells[c] as usize, body.part[c]);
        let m3 = lack * if area > 0.0 { parts[c].2 / area } else { 1.0 / body.edge.len() as f64 };
        let row: Vec<f64> = mixed.iter().map(|x| x * m3 / vol).collect();
        (0..width).for_each(|r| h.q[i * width + r] += row[r]);
        (h.vol[i], h.part[i][q]) = (h.vol[i] + m3, h.part[i][q] + m3);
        edge.push(Boundary::Outside as u32, i as u32, row[..width - 1].iter().copied().enumerate(), row[width - 1]);
    }
    lack
}

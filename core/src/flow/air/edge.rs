//! The map's open edges (laws.md 1.6): an island stands in the planet's air. Beyond each air node of an edge of the
//! map lies a column of the planet's air at its latitude and time of the year (`Planet::at`), linked to the node by the
//! eddies and the overturning as any link between nodes (`wind::push`, with the stretch), its pressure that of the
//! planet; what crosses comes from and goes to the boundary node for outside (`store::Boundary::Outside`, `book`).
//! Open only where the world holds the planet's air (`World::planet`).

use super::super::Scratch;
use super::planet::Beyond;
use super::stretch::Stretch;
use super::cell::Cell;
use super::wind::{push, End};
use crate::{World, AIR};

/// The edges that are open, as the side of the map beyond them [east, south] from a node of that edge: north, south,
/// east and west. Across the east and west edges the planet's mean wind along the row, its overturning turned by the
/// rotation, carries its air through the map.
const SIDES: [[f64; 2]; 4] = [[0.0, -1.0], [0.0, 1.0], [1.0, 0.0], [-1.0, 0.0]];

/// The links of the step to the columns beyond the map's open edges (`SIDES`): per link its node, its side, its pushes
/// (eddies, mol/s each way; overturning, mol/s of the lower half from the node outward; the heat its halves carry
/// inward over their mean heat per mol, W; and the heat the eddies carry outward over the columns' mean heat per mol, W), the share of each gas of the air (`Scratch::state`) of the column beyond and its
/// heat per mol, J; and what crossed into the node over the step: per gas and link (`[q * links + l]`) mol, and per
/// link the heat, J (its sensible heat as the air's days move it, `store::Class::Air`).
pub(super) struct Edges {
    pub(super) node: Vec<u32>,
    pub(super) side: Vec<usize>,
    push: Vec<[f64; 4]>,
    share: Vec<Vec<f64>>,
    heat_of: Vec<f64>,
    pub(super) mol: Vec<f64>,
    pub(super) heat: Vec<f64>,
}

impl Edges {
    /// The links of the air nodes `ends` (`wind::ends`) of `w` to the planet's air beyond its open edges, at the time of
    /// the step's start; none where `w` holds no planet's air.
    pub(super) fn of(w: &World, s: &Scratch, ends: &[End], cell: &Cell) -> Edges {
        let mut x = Edges { node: Vec::new(), side: Vec::new(), push: Vec::new(), share: Vec::new(), heat_of: Vec::new(), mol: Vec::new(), heat: Vec::new() };
        let (air, st, tree) = (w.mesh.level(AIR), Stretch::of(w), w.mesh.tree());
        let Some(planet) = w.planet.as_ref().filter(|_| w.edges_open()) else { return x };
        let corner = (0..air.n()).map(|j| tree.block_xy(j)).fold((0, 0), |m, (a, b)| (m.0.max(a), m.1.max(b)));
        // Degrees of latitude from one row of air nodes to the next.
        let row = (w.cfg.lat_north - w.cfg.lat_south) / (corner.1 + 1) as f64;
        for (j, node) in ends.iter().enumerate() {
            let (bx, by) = tree.block_xy(j);
            let at_edge = [by == 0, by == corner.1, bx == corner.0, bx == 0];
            for (side, dir) in SIDES.iter().enumerate().filter(|&(side, _)| at_edge[side]) {
                let spacing = air.area()[j].sqrt();
                let lat = node.lat - dir[1] * row;
                let beyond = planet.at(lat, w.season());
                let share: Vec<f64> = s.state.gas.iter().map(|&k| beyond.share[k]).collect();
                let all = share.iter().sum::<f64>().max(f64::MIN_POSITIVE);
                let share: Vec<f64> = share.iter().map(|v| v / all).collect();
                let cp = s.state.gas.iter().zip(&share).map(|(&k, v)| v * s.sp[k].cp).sum::<f64>();
                let end = beyond_end(w, (node, lat, &beyond, cp), st);
                let ab = [dir[0] * spacing, dir[1] * spacing];
                let [mix, over, contrast, _, stir] = push(&w.cfg, st, (node, &end), (st.length(spacing), st.length(spacing), ab), cell);
                x.heat_of.push(beyond.t * cp);
                x.node.push(j as u32);
                x.side.push(side);
                x.push.push([mix, over, contrast, stir]);
                x.share.push(share);
            }
        }
        (x.mol, x.heat) = (vec![0.0; s.state.gas.len() * x.node.len()], vec![0.0; x.node.len()]);
        x
    }

    /// One day of `dd` seconds of the exchange across the open edges, after the wind, from the air `x` (`[gi * na + j]`)
    /// and its heat `e`, each link implicit (backward Euler) with the column beyond held: the eddies' mol each way and the
    /// overturning's halves, the lower half outward where the column beyond is warmer and the upper back, each half of
    /// its own air (vapour crowding low, `Scratch::split`), heat by mol, the halves carrying `contrast` more inward, the
    /// eddies' mol each its column's near-ground energy (`wind::push`, `stir`). The
    /// column beyond lies over low open ground at 0 m: the node pays the lift of the mass that enters it (`wind::lift`).
    pub(super) fn day(&mut self, s: &Scratch, (x, e): (&mut [f64], &mut [f64]), dd: f64, g: f64) {
        let (na, ng, links) = (e.len(), s.state.gas.len(), self.node.len());
        for l in 0..links {
            let (j, [mix, o, contrast, stir]) = (self.node[l] as usize, self.push[l]);
            let split = |q: usize| s.split[q * na + j];
            let nt: f64 = (0..ng).map(|q| x[q * na + j]).sum::<f64>().max(f64::MIN_POSITIVE);
            let upper: f64 = (0..ng).map(|q| x[q * na + j] * split(q)).sum();
            // The node's upper half leaves where the lower half comes in (`o` < 0), and the column beyond's other half
            // enters: of its own shares, its vapour crowding low.
            let (up, half) = (o < 0.0, |q: usize| if o > 0.0 { split(q) } else { 1.0 - split(q) });
            let enters = (0..ng).map(|q| self.share[l][q] * half(q)).sum::<f64>().max(f64::MIN_POSITIVE);
            let mut kg = 0.0;
            for q in 0..ng {
                let leaves = match (s.cond[q], up) {
                    (false, _) => 1.0 / nt,
                    (true, true) => split(q) / upper.max(f64::MIN_POSITIVE),
                    (true, false) => (1.0 - split(q)) / (nt - upper).max(f64::MIN_POSITIVE),
                };
                let (out, into) = (mix / nt + o.abs() * leaves, mix * self.share[l][q] + o.abs() * self.share[l][q] * half(q) / enters);
                let v = &mut x[q * na + j];
                let now = (*v + dd * into) / (1.0 + dd * out);
                self.mol[q * links + l] += now - *v;
                kg += (now - *v) * s.sp[s.state.gas[q]].mm;
                *v = now;
            }
            let both = mix + o.abs();
            let now = (e[j] + dd * (both * self.heat_of[l] + contrast - stir)) / (1.0 + dd * both / nt);
            self.heat[l] += now - e[j];
            e[j] = now - if kg > 0.0 { g * kg * s.za[j] } else { 0.0 };
        }
    }
}

/// The column of the planet's air `beyond` an edge node `node`, at latitude `lat`, a mol of it holding `cp` J/K, as an
/// end of its link (`wind::End`): over low open ground at 0 m, where its air is at its warmth (its near-ground energy
/// per mol over its mean heat, `wind::aloft`, is cp times its warmth less its temperature), its warmth's gradient
/// toward the south on the map's metres as the stretched air takes them, the node's rise and sound (its pressure is
/// the planet's: no evening across the edge).
fn beyond_end(w: &World, (node, lat, beyond, cp): (&End, f64, &Beyond, f64), st: Stretch) -> End {
    let aloft = cp * (beyond.warmth - beyond.t);
    End { rho: beyond.mol / w.cfg.air_top_m, aloft, za: 0.0, lat, grad: [0.0, st.per_map_m(beyond.south)], rise: node.rise, sound: node.sound, c: node.c }
}

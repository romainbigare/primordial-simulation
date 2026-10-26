//! What an air step moves over its span, booked at once as it is solved: backward Euler knows it at the start, and the
//! other laws read what it leaves until the next step.

use super::super::{follow, year_weight, Scratch};
use super::exchange::cloud;
use super::newton::Wind;
use super::step::{Active, Iterate, Start, FIXED, PER_GAS};
use super::wind::Links;
use super::{lerp, shares, sums};
use crate::store::{book_by_array, held_more, less_held_more, Booker, Boundary, Class};
use crate::{World, AIR};

/// Ground cells whose booking counts as one item of the pool's work (`par::SERIAL_BELOW`): a world of fewer than
/// `SERIAL_BELOW` x this many cells books on the calling thread. Not measured: a guess.
const CELLS_PER_ITEM: usize = 256;

/// What the ground nodes give the air over a step, per link of a ground node to the air: link `e = c * n + i` of ground
/// node `i` goes to its own air node (`c` 0) or to one of the four it sees (`c` 1 to 4), air node `ends[e]`.
struct Gives {
    ne: usize,
    ends: Vec<u32>,
    /// Per gas and link (`[x * ne + e]`): what goes up, mol (what evaporates into its own air node, what rains out of
    /// each node it sees by its share), and the heat each mol takes from the ground, J (what evaporates leaves at the
    /// node's temperature, the heat it held there).
    up: Vec<f64>,
    up_heat: Vec<f64>,
    /// Per gas and node (`[x * n + i]`), what evaporates of the rain it gets in the step, mol.
    of_rain: Vec<f64>,
    /// Per link: the heat the ground gives its air node over it, J.
    heat: Vec<f64>,
    /// Per ground node: the sunlight it absorbs, J; the light on its bodies, J; the heat set aside for them, J; the heat
    /// it gives them, J; what rains on it, mol.
    sun: Vec<f64>,
    light: Vec<f64>,
    kept: Vec<f64>,
    drawn: Vec<f64>,
    rain: Vec<f64>,
    /// Per air node: the heat it radiates to space, J.
    space: Vec<f64>,
}

/// The rows of the last round as what each ground node gives the air (`Gives`).
fn gives(w: &World, s: &Scratch, (act, rows, width): (&[Active], &[f64], usize), it: &Iterate, dt: f64) -> Gives {
    let (n, na, cells) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells());
    let (parent, area) = (cells.parent(), cells.area());
    let ends: Vec<u32> = parent.iter().copied().chain((0..4).flat_map(|c| s.near.iter().map(move |j| j[c]))).collect();
    let ne = ends.len();
    let v = |len: usize| vec![0.0; len];
    let mut g = Gives { ne, ends, up: v(act.len() * ne), up_heat: v(act.len() * ne), of_rain: v(act.len() * n), heat: v(ne), sun: v(n), light: v(n), kept: v(n), drawn: v(n), rain: v(n), space: v(na) };
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (j, tn) = (parent[i] as usize, s.state.tg[i]);
        (g.sun[i], g.heat[i], g.light[i], g.kept[i], g.drawn[i]) = (row[3], row[0] + row[2], row[5], row[6] * area[i] * dt, row[7]);
        g.space[j] += row[2];
        for (x, ac) in act.iter().enumerate() {
            let (o, sp, e) = (FIXED + PER_GAS * x, s.sp[ac.k], x * ne);
            let (r, a) = (row[o + 3], w.store.amounts(Class::Ground)[ac.k * n + i]);
            let m = row[o].min(a + r);
            (g.up[e + i], g.up_heat[e + i], g.of_rain[x * n + i]) = (m.min(a), sp.cp * tn + sp.lf + sp.lv, (m - a).max(0.0));
            for (c, (q, share)) in shares(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i).into_iter().enumerate() {
                (g.up[e + (c + 1) * n + i], g.up_heat[e + (c + 1) * n + i]) = (-r * share, sp.cp * it.t[q] + row[o + 7]);
            }
            g.rain[i] += r;
        }
    }
    g
}

/// Book what the step of `dt` seconds moves, from its last round's rows, wind and air: sunlight, light set
/// aside for bodies, the heat each ground node gives its bodies' node, heat to the air and to space, the ground's gas
/// with its heat, the wind and the lifts; then the light on bodies, the rain and the clouds of what condensed in each
/// air node (`rained`). No node gives more of a gas than it holds and gets (`hold_air`). No domain's view of the store
/// is open then (`Moves::open`; checked in debug builds, as `apply` checks).
pub(super) fn book(w: &mut World, s: &mut Scratch, (act, rained): (&[Active], &[[f64; 2]]), (rows, width): (&[f64], usize), wind: &Wind, (it, start): (&Iterate, &Start), dt: f64) {
    debug_assert!(w.life.domains.iter().all(|d| d.moves.open.is_none()), "the store changes while a domain is open");
    let (n, na, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells().area().to_vec());
    let mut g = gives(w, s, (act, rows, width), it, dt);
    // The clouds of what condensed, which the next step's sunlight passes.
    let (refl, depth): (Vec<f64>, Vec<f64>) = rained.iter().zip(w.mesh.level(AIR).area()).map(|(&r, &a)| cloud((w.cfg.cloud, w.cfg.cloud_ir), (r[0], r[1]), a, dt)).map(|[r, d]| (r, d)).unzip();
    s.state.cloud = (0..n).map(|i| lerp(s, &refl, i)).collect();
    s.state.cloud_depth = (0..n).map(|i| lerp(s, &depth, i)).collect();
    // The wind and the rain of each gas, cut where an air node would give more than it holds.
    let links = Links::of(w.mesh.level(AIR));
    let (nl, mut wind_mol, ne) = (links.a.len(), wind.mol.clone(), g.ne);
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let rain = act.iter().position(|a| a.k == k).map_or(&mut [][..], |x| &mut g.up[x * ne..(x + 1) * ne]);
        hold_air(&start.x[gi * na..(gi + 1) * na], (&links.a, &links.b), &mut wind_mol[gi * nl..(gi + 1) * nl], (&g.ends, rain));
    }
    // What evaporates of the rain comes up once the cut has set the rain each node gets, so no node gives more than it
    // holds and gets (the cut counted on none of it).
    for (x, i) in (0..act.len() * n).filter(|&y| g.of_rain[y] > 0.0).map(|y| (y / n, y % n)) {
        g.up[x * ne + i] += g.of_rain[x * n + i].min((1..ne / n).map(|c| -g.up[x * ne + c * n + i]).sum());
    }
    w.light.iter_mut().zip(&g.light).for_each(|(l, got)| *l += got);
    w.life.each_mut(|b| b.mark -= (b.shade - 1.0) * g.light[b.cell as usize]);
    // Each ground node's gas crosses its links to the air with its heat; the air pays the lift.
    let (grav, view) = (w.cfg.gravity, s.view());
    let (mut paid, mut net) = (wind.paid.iter().zip(&start.lift).map(|(a, b)| a + b).collect::<Vec<f64>>(), vec![0.0; act.len() * n]);
    for (x, ac) in act.iter().enumerate() {
        for e in (0..ne).filter(|&e| g.up[x * ne + e] != 0.0) {
            let (m, i, j) = (g.up[x * ne + e], e % n, g.ends[e] as usize);
            g.heat[e] += g.up_heat[x * ne + e] * m;
            paid[j] += m * s.sp[ac.k].mm * view.lift(i, j, grav);
            net[x * n + i] += m;
        }
    }
    for (x, ac) in act.iter().enumerate() {
        net[x * n..(x + 1) * n].iter().any(|&u| u < 0.0).then(|| s.add_loose(ac.k, false));
        (0..n).filter(|&i| net[x * n + i] != 0.0).for_each(|i| s.note(ac.k, i, -net[x * n + i], area[i]));
    }
    // The gas of the bodies' nodes comes up whole, with the latent heat it holds; the air paid its lift (`Start`). The
    // heat each link to the air gives it is what the ground gives less what its gas holds more in the air.
    let from_bodies = w.store.amounts(Class::Bodies).to_vec();
    let mut given = g.heat.clone();
    for (x, ac) in act.iter().enumerate() {
        less_held_more(&mut given, &g.up[x * ne..(x + 1) * ne], held_more(&w.chem, Class::Ground, Class::Air, ac.k));
    }
    let cols = Columns { g: &g, act, given: &given, from_bodies: &from_bodies, paid: &paid, links: &links, gas: &s.state.gas, wind_mol: &wind_mol, wind_heat: &wind.heat };
    let rounded = book_by_array(&mut w.store, w.chem.ns(), &[Class::Ground, Class::Air, Class::Bodies], n / CELLS_PER_ITEM, |b| cols.changes(b));
    w.counts.add_rounded(rounded);
    // The boundary nodes and the potential hold a whole history's sum: what the step moves is summed apart and added once.
    let sun_taken = g.sun.iter().chain(&g.kept).fold(0.0, |far, d| far - d);
    w.store.add_far_heat(Class::Boundary, Boundary::Sun as usize, sun_taken);
    w.store.add_far_heat(Class::Boundary, Boundary::Space as usize, g.space.iter().fold(0.0, |far, d| far + d));
    w.store.add_far_heat(Class::Potential, 0, paid.iter().fold(0.0, |far, d| far + d));
    let ema = year_weight(w, dt);
    for (i, ((r, sum), e)) in w.rain.iter_mut().zip(&mut w.rained).zip(&mut w.evap).enumerate() {
        follow(r, (g.rain[i] / (area[i] * dt)) as f32, ema);
        follow(e, ((0..act.len()).map(|x| g.up[x * ne + i]).sum::<f64>() / (area[i] * dt)) as f32, ema);
        *sum += g.rain[i] / area[i];
    }
    sums(w, s);
}

/// What the step adds to the arrays of the store, as column moves: each ground node's gas goes up into its own air node
/// and rains out of the four it sees, with its heat; the bodies' gas comes up whole; the wind and the lifts move the
/// air's own.
struct Columns<'a> {
    g: &'a Gives,
    act: &'a [Active],
    /// The heat each link of a ground node to the air gives its air node, J (`Gives::heat` less what the gas holds more
    /// in the air).
    given: &'a [f64],
    /// The gas of each bodies' node, `[k * n + i]`, mol.
    from_bodies: &'a [f64],
    /// What the air pays for the lifts, J, per air node.
    paid: &'a [f64],
    /// The wind: its links, the gases it carries, their mol (`[gas * links + link]`) and its heat per link, J.
    links: &'a Links,
    gas: &'a [usize],
    wind_mol: &'a [f64],
    wind_heat: &'a [f64],
}

impl Columns<'_> {
    /// The changes to the numbers of the array of booker `b`, both sides of each move (`book_by_array`): the ground's
    /// gas over its five links and its heat (from the sun, to its bodies' node, to the air); the air's gas (over the
    /// links to the ground, from the bodies' nodes, the wind) and heat (from the ground, to space, the wind's, the lifts
    /// it pays); the bodies' nodes' gas, and the heat they keep of the sun and take from the ground.
    fn changes(&self, b: &mut Booker) {
        let (g, n, ne, nl) = (self.g, self.g.sun.len(), self.g.ne, self.links.a.len());
        match (b.class, b.ks.clone()) {
            (Class::Ground, Some(ks)) => {
                let gases = self.act.iter().enumerate().filter(|(_, ac)| ks.contains(&ac.k));
                gases.for_each(|(x, ac)| b.column(|e| ac.k * n + e % n, &g.up[x * ne..(x + 1) * ne], -1.0));
            }
            (Class::Ground, None) => {
                b.column(|i| i, &g.sun, 1.0);
                b.column(|i| i, &g.drawn, -1.0);
                b.column(|e| e % n, &g.heat, -1.0);
            }
            (Class::Air, Some(ks)) => {
                for k in ks {
                    let off = k * b.len;
                    if let Some(x) = self.act.iter().position(|ac| ac.k == k) {
                        b.column(|e| off + g.ends[e] as usize, &g.up[x * ne..(x + 1) * ne], 1.0);
                    }
                    b.column(|i| off + g.ends[i] as usize, &self.from_bodies[k * n..(k + 1) * n], 1.0);
                    if let Some(gi) = self.gas.iter().position(|&q| q == k) {
                        self.wind(b, off, &self.wind_mol[gi * nl..(gi + 1) * nl]);
                    }
                }
            }
            (Class::Air, None) => {
                b.column(|e| g.ends[e] as usize, self.given, 1.0);
                b.column(|j| j, &g.space, -1.0);
                self.wind(b, 0, self.wind_heat);
                b.column(|j| j, self.paid, -1.0);
            }
            (Class::Bodies, Some(ks)) => ks.for_each(|k| b.column(|i| k * n + i, &self.from_bodies[k * n..(k + 1) * n], -1.0)),
            (Class::Bodies, None) => {
                b.column(|i| i, &g.kept, 1.0);
                b.column(|i| i, &g.drawn, 1.0);
            }
            _ => {}
        }
    }
    /// What the wind moves over each of its links, `d`, out of its `a` end and into its `b` end, numbers `off` on.
    fn wind(&self, b: &mut Booker, off: usize, d: &[f64]) {
        let (a, to) = (&self.links.a, &self.links.b);
        for (l, &m) in d.iter().enumerate().filter(|(_, m)| **m != 0.0) {
            b.add(off + a[l] as usize, -m);
            b.add(off + to[l] as usize, m);
        }
    }
}

/// Passes of `hold_air` before every air node gives at most what it holds.
const CUT_PASSES: usize = 4;

/// Cuts the wind and the rain out of each air node of one gas where it would end below zero, by the share that leaves
/// it nothing; as that takes from its neighbours, again, `CUT_PASSES` times at most (as `solve::spread::within`). `up`
/// is what comes up into air node `ends[e]` over each link `e` with the ground.
fn hold_air(held: &[f64], (a, b): (&[u32], &[u32]), wind: &mut [f64], (ends, up): (&[u32], &mut [f64])) {
    for pass in 0..=CUT_PASSES {
        let (mut end, mut out) = (held.to_vec(), vec![0.0; held.len()]);
        for ((&i, &j), &m) in a.iter().zip(b).zip(wind.iter()) {
            let (from, to) = if m > 0.0 { (i as usize, j as usize) } else { (j as usize, i as usize) };
            (end[from], end[to], out[from]) = (end[from] - m.abs(), end[to] + m.abs(), out[from] + m.abs());
        }
        for (&j, &u) in ends.iter().zip(up.iter()) {
            (end[j as usize], out[j as usize]) = (end[j as usize] + u, out[j as usize] + (-u).max(0.0));
        }
        if end.iter().all(|&e| e >= 0.0) {
            return;
        }
        let keep: Vec<f64> = (0..held.len())
            .map(|j| match (end[j] < 0.0 && out[j] > 0.0, pass == CUT_PASSES) {
                (_, true) if out[j] > 0.0 => (held[j] / out[j]).min(1.0),
                (true, false) => ((end[j] + out[j]) / out[j]).max(0.0),
                _ => 1.0,
            })
            .collect();
        for ((&i, &j), m) in a.iter().zip(b).zip(wind.iter_mut()) {
            *m *= keep[if *m > 0.0 { i } else { j } as usize];
        }
        for (&j, u) in ends.iter().zip(up.iter_mut()).filter(|(_, u)| **u < 0.0) {
            *u *= keep[j as usize];
        }
    }
}

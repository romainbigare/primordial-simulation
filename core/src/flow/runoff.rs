//! The liquid that may run off moves down the cells in one pass in the order of a priority flood (`solve::downhill`;
//! laws.md 1.2), with its heat and its potential energy (`Falls`); lost head heats what runs out and does work on the
//! bed (`load`, `wear`).

use super::{bare_share, falls::{Falls, Lands}, follow, land::{lift, solid, NONE}, liquid::{book, mobile}, potential::apply, year_weight, Scratch};
use crate::{math::one_minus_exp, solve::downhill::{pass, Adjacency, Crossed, Order}, solve::sweep::{by_size, Net, LIMIT}, space::Level, store::{Class, LinkFlux, Move, Node}, units::T_FLOOR, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;
/// The capacity of a way in the sweep of a node's carrying (`Pass::worked`), m²: so large that what it gets leaves its
/// surface where the step started.
const HELD_CAP: f64 = 1e30;

/// What a cell's liquid finds toward a neighbour in the pass (`Pass::ways`).
#[derive(Clone, Copy, Debug)]
struct Toward {
    /// The drop, m, where it is a way down.
    drop: Option<f64>,
    /// The volume that may move there, m³.
    room: f64,
    /// The fall of the cell's head, m.
    fall: f64,
    /// The rim, m: the level the liquid sent there comes to stand at, or crosses (the ground of a cell it passes on
    /// through), so the cell it leaves does not drain below it by that way.
    rim: f64,
}

/// What the pass reads at each cell (`surface`): the world and Flow's state as the pass found them, the order of the
/// cells, the rows that move together per node as at the pass's start (`q0`, `q[i * width + r]`: each liquid of the
/// first `nl` of `ks`, each load of the rest, `sol`, their heat, J, and the liquid's volume, m³), and which cells are
/// land (`World::is_land`: the solid surface above 0 m, as Flow's state has it), and the step, s.
struct Pass<'a> {
    w: &'a World,
    s: &'a Scratch,
    order: &'a Order,
    ks: &'a [usize],
    sol: &'a [usize],
    nl: usize,
    q0: &'a [f64],
    land: &'a [bool],
    dt: f64,
}

/// What a cell's visit leaves of the pass: the work left to break its bonded ground, J (`load`), the liquid it sent on
/// and the liquid it sent off the land, mol.
struct Left {
    worn: f64,
    sent: f64,
    off_land: f64,
}

/// The liquid that may run off (what stands, what came up from the store, and the pores' drainage to the channels, a
/// share 1 - exp(-`seep` × slope × time / (`porosity` × `hill`))) moves down the cells in one pass (`solve::downhill`;
/// laws.md 1.2), with its heat and, where Flow's lifts and falls are booked, its potential energy (`Falls`); lost head
/// heats what runs out and does work on the bed (`load`, `wear`).
pub(super) fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let (n, area, ph) = (w.mesh.cells().n(), w.mesh.cells().area(), w.cfg.phys);
    let mob = mobile(w, s, liq, tmin, |_| true);
    let sol = s.solids(tmin);
    let ks: Vec<usize> = liq.iter().chain(&sol).copied().collect();
    let (nl, width) = (liq.len(), ks.len() + 2);
    let vol = width - 1;
    let mut q = vec![0.0; width * n];
    for i in 0..n {
        let (pores, own) = (s.pd[i] * area[i], mob.vol[i] - runs[i]);
        let lp = own.min(pores);
        let free = if s.head[i] > s.z[i] { lp - ph.field * pores } else { 0.0 };
        let hill = if free > 0.0 && slope[i] > 0.0 { free.min(lp * one_minus_exp(ph.seep * slope[i] * dt / (ph.porosity * ph.hill))) } else { 0.0 };
        let off = if mob.vol[i] > 0.0 { (((own - pores).max(0.0) + runs[i] + hill) / mob.vol[i]).min(1.0) } else { 0.0 };
        (0..nl).for_each(|r| q[i * width + r] = mob.mol[r * n + i] * off);
        q[i * width + vol] = mob.vol[i] * off;
    }
    let (q0, cells) = (q.clone(), w.mesh.cells());
    let adj = Adjacency { start: cells.start(), link: cells.adj_link(), other: cells.adj_cell(), a: cells.a() };
    let mut order = std::mem::take(&mut s.order);
    order.update(&adj, &s.z, |i| s.z[i] + q[i * width + vol] / area[i], &s.basin.cells);
    let land: Vec<bool> = s.z.iter().map(|&z| z > 0.0).collect();
    let mut falls = s.lifts.as_ref().map(|_| Falls::new(w, s));
    let (mut left, mut crossed, mut outs, mut rest) = (Vec::with_capacity(n), Crossed::default(), Vec::new(), Vec::new());
    left.resize_with(n, || Left { worn: 0.0, sent: 0.0, off_land: 0.0 });
    let p = Pass { w, s, order: &order, ks: &ks, sol: &sol, nl, q0: &q0, land: &land, dt };
    pass(&order, &adj, &mut q, &mut crossed, |i, q, share| {
        if q[i * width + vol] > 0.0 {
            let (out, drop) = p.ways(i, q, share, (&mut outs, &mut rest));
            left[i] = p.leave(i, q, share, (out, drop), falls.as_mut());
        }
    });
    // The crossings as a link flux: each substance once (liquid and load rows of one substance share it), then the heat.
    let kinds: Vec<u16> = liq.iter().chain(sol.iter().filter(|k| !liq.contains(k))).map(|&k| k as u16).collect();
    let col_of: Vec<usize> = ks.iter().map(|&k| kinds.iter().position(|&x| x as usize == k).unwrap_or(0)).chain([kinds.len()]).collect();
    let (a, b, mol) = by_link(&crossed, width, &col_of, (cells.a(), cells.b()));
    let at = kinds.len() * a.len();
    // The heat of the falls lands before the crossings, which carry it on; a climb is paid after them, from the heat the
    // liquid brought: the node gives no more than it holds at either time.
    let (fell, climbed): (Vec<f64>, Vec<f64>) = falls.as_ref().map_or_else(Default::default, |f| f.given.iter().map(|&x| (x.max(0.0), x.min(0.0))).unzip());
    if let Some(l) = s.lifts.as_mut() {
        l.give(w, &fell);
    }
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &kinds, mol: &mol[..at], heat: &mol[at..] }], falls.as_ref().map(|f| &f.carried[..]));
    if let Some(l) = s.lifts.as_mut() {
        l.give(w, &climbed);
    }
    s.order = order;
    let (ema, mut broke) = (year_weight(w, dt), Vec::new());
    for (i, x) in left.iter().enumerate() {
        follow(&mut w.liquid_flux[i], (x.sent / dt) as f32, ema);
        follow(&mut w.off_land[i], (x.off_land / dt) as f32, ema);
        if x.worn > 0.0 {
            wear(w, s, i, x.worn, &mut broke);
        }
    }
    apply(w, s, &[], &broke);
}

impl Pass<'_> {
    /// The width of a node's rows in `q`; its heat row is `width - 2`, its volume row `width - 1`.
    fn width(&self) -> usize {
        self.ks.len() + 2
    }

    /// The head of cell `j` in the pass, m: its ground and the liquid that may run off over it.
    fn head(&self, q: &[f64], j: usize) -> f64 {
        self.s.z[j] + q[j * self.width() + self.width() - 1] / self.w.mesh.cells().area()[j]
    }

    /// The share of cell `i`'s liquid that leaves down each way (`share`), with `outs` and `rest` as room. The surface
    /// of a cell whose liquid stands is its head, but no higher than its spill level (what stands over it passes on);
    /// of one its liquid passes on through, its ground. To a neighbour still to come whose liquid passes on, the liquid
    /// goes down to that one's ground; to one whose liquid stands, what levels the two heads, whatever their areas, and
    /// all that stands over that one's spill level, which passes on through it. Liquid never goes back to a neighbour
    /// already passed whose liquid passes on: that one lies upstream in the flood, and what it got would come back the
    /// next step; to one whose liquid stands it levels the two heads, but no higher than that one's spill level: what
    /// stands over it would pass on, and that one has passed. It takes the steepest way down (`Level::steepest`), and on
    /// level ground the flood's way out; what that way cannot take runs down the others (`run_on`), and no way takes it
    /// below the rim of another it used (`to_rims`). Returns the share that leaves and the fall of the head, m.
    fn ways(&self, i: usize, q: &[f64], share: &mut [f64], (outs, rest): (&mut Vec<Toward>, &mut Vec<usize>)) -> (f64, f64) {
        let (order, cells, still) = (self.order, self.w.mesh.cells(), self.w.cfg.phys.still);
        let (area, zi, hd, sv) = (cells.area(), self.s.z[i], self.head(q, i), q[i * self.width() + self.width() - 1]);
        outs.clear();
        let (top, mut way) = (if order.stands[i] { hd } else { zi }, None);
        for (k, j) in cells.around(i).1.iter().map(|&j| j as usize).enumerate() {
            let (hj, passed) = (self.head(q, j), order.before(j, i));
            let to = match (passed, order.stands[j]) {
                (true, _) => hj,
                (false, true) => hj.min(order.spill[j]),
                (false, false) => self.s.z[j],
            };
            let from = if passed { top } else { hd };
            let (room, rim) = if passed {
                let room = ((from - to) * (area[i] * area[j] / (area[i] + area[j]))).min((order.spill[j] - hj) * area[j]);
                (room, hj + room / area[j])
            } else {
                let level = if order.stands[j] { ((hd * area[i] + hj * area[j]) / (area[i] + area[j])).min(order.spill[j]) } else { to };
                ((hd - level.max(zi)) * area[i], level.max(zi))
            };
            let down = from - to > still && (!passed || order.stands[j]) && room > 0.0;
            if down && j == order.outlet[i] as usize {
                way = Some([(k, 1.0), (k, 0.0)]);
            }
            outs.push(Toward { drop: down.then_some((top - to).max(0.0)), room, fall: (hd - hj).max(0.0), rim });
        }
        let Some(ways) = cells.steepest(i, |k| outs[k].drop).map(|w| w.1).or(way) else { return (0.0, 0.0) };
        let (mut out, mut drop, mut short) = (0.0, 0.0, false);
        for (k, f) in ways.into_iter().filter(|p| p.1 > 0.0) {
            share[k] = (sv * f).min(outs[k].room) / sv;
            (out, drop, short) = (out + share[k], drop + share[k] * outs[k].fall, short || sv * f > outs[k].room);
        }
        if short {
            (out, drop) = run_on(cells, i, (outs, rest), share, sv, (out, drop));
        }
        to_rims(outs, share, (sv, zi, area[i], still), (out, drop))
    }

    /// Cell `i` sends a share `out` of its liquid (`share` down each way, its head falling by `drop` m): its liquid does
    /// work on its load (`load`), the potential energy of what came in and of its own column turns to heat there
    /// (`Falls::visit`; none where Flow's lifts and falls are not booked), and what runs out leaves at the temperature of
    /// the node mixed with what came in, with that heat (`Falls::send`).
    fn leave(&self, i: usize, q: &mut [f64], share: &[f64], (out, drop): (f64, f64), falls: Option<&mut Falls>) -> Left {
        let (w, s, (nl, ks)) = (self.w, self.s, (self.nl, self.ks));
        let (n, cells, width) = (w.mesh.cells().n(), w.mesh.cells(), self.width());
        let (rows, heat, vol, area) = (ks.len(), width - 2, width - 1, cells.area()[i]);
        // The node's heat capacity and fusion enthalpy with what came in. Liquid that came in holds its fusion enthalpy
        // as it passes, whatever the node's own phase (its heat brought it); a load holds none.
        let (mut c, mut held) = (s.cap[i], s.latent[i].held);
        let (cell, was) = (i * width..(i + 1) * width, &self.q0[i * width..(i + 1) * width]);
        for (r, &k) in ks.iter().enumerate().filter(|p| q[cell.start + p.0] != was[p.0]) {
            let d = q[cell.start + r] - was[r];
            (c, held) = (c + d * s.sp[k].cp, held + if r < nl { d * s.sp[k].lf } else { 0.0 });
        }
        // The fall of its head down each way, m (`ways`).
        let fall: Vec<f64> = cells.around(i).1.iter().map(|&j| (self.head(q, i) - self.head(q, j as usize)).max(0.0)).collect();
        let q = &mut q[cell];
        let loads = |q: &[f64]| (nl..rows).map(|r| q[r] * s.sp[ks[r]].v[0]).sum::<f64>();
        let kg = |q: &[f64], rs: std::ops::Range<usize>| rs.map(|r| q[r] * s.sp[ks[r]].mm).sum::<f64>();
        let (sv, came) = (q[vol], loads(q));
        // The work, on the bed below the liquid left standing (which hides it); none below 0 m, where the load settles.
        let kept = sv * (1.0 - out) / area;
        let bedwork = if self.land[i] { w.cfg.gravity * drop * kg(q, 0..nl) * w.cfg.phys.hide[1] / (w.cfg.phys.hide[1] + kept) } else { 0.0 };
        let per_m = w.cfg.carry * bare_share(w, i) * w.cfg.gravity * kg(q, 0..nl) * w.cfg.phys.hide[1] / (w.cfg.phys.hide[1] + kept);
        let bedwork = match self.worked(i, (share, &fall), per_m) {
            Some(f) if bedwork > 0.0 => bedwork * f,
            _ => bedwork,
        };
        let worn = load(w, s, i, q, (nl..rows, self.sol), bedwork, out);
        // How far its head rises over the pass, m: the liquid it keeps less what it held at the start, and the loads that
        // came in less those that leave (all it picked up came from it).
        let (mass, moved) = (kg(q, 0..rows), sv + loads(q));
        let rise = (sv * (1.0 - out) - was[vol] + came - out * loads(q)) / area;
        let mut falls = falls;
        q[heat] += falls.as_mut().map_or(0.0, |f| f.visit(i, out * mass, rise));
        debug_assert!(q[heat] >= 0.0, "cell {i}: a climb the liquid cannot pay, {} J short", -q[heat]);
        // The temperature of the node mixed with what came in, at which what runs out leaves: off a melting plateau its
        // energy over its fusion enthalpy and heat capacity; on one, what it gains or loses on the plateau's temperature
        // melts or freezes the plateau's substance first (`FlowView::fused`), and only the rest warms or cools it. So
        // the liquid passing on leaves at a temperature the node pays for, and what stays keeps it.
        let lat = match s.state.pk[i] { NONE => 0.0, k => w.store.amounts(Class::Ground)[k * n + i] * s.sp[k].lf };
        let x = w.store.heat(Class::Ground)[i] + q[heat] - held - c * s.state.tg[i];
        let t = (s.state.tg[i] + (x.min(0.0) + (x - lat).max(0.0)) / c.max(1.0)).max(T_FLOOR);
        q[heat] = ks.iter().enumerate().map(|(r, &k)| q[r] * (s.sp[k].cp * t + if r < nl { s.sp[k].lf } else { 0.0 })).sum();
        let around = cells.around(i).1;
        if let Some(f) = falls {
            for (k, &j) in around.iter().enumerate().filter(|p| share[p.0] > 0.0) {
                let j = j as usize;
                let lands = if s.basin.is[j] { Lands::Basin } else if self.order.before(i, j) { Lands::Ahead } else { Lands::Passed };
                f.send((i, j), (share[k] * mass, share[k] * moved, cells.area()[j]), lands);
            }
        }
        // What leaves a land cell for cells that are not land runs off the land.
        let (liquid, off) = (q[..nl].iter().sum::<f64>(), (0..share.len()).filter(|&k| self.land[i] && !self.land[around[k] as usize]).map(|k| share[k]).sum::<f64>());
        Left { worn, sent: out * liquid, off_land: off * liquid }
    }
}

impl Pass<'_> {
    /// The share of the work of one step that the liquid leaving node `i` does in the step, where the node is too small
    /// for it (laws.md 1.8; none where it is not). Its share `share[k]` going down way `k`, its head falling `falls[k]`
    /// there, can carry `per_m` m³ of load per metre of fall (`carry` times the work per metre, `load`), which lowers
    /// the node's surface by that over its area: past `sweep::LIMIT` (half the fall in a step) one step would cut the
    /// node below its ways. So the carrying sub-steps as a sweep of the node's surface toward those of its ways, which
    /// stay as the step started (`sweep::by_size`), and the work is what the sub-steps carry over what one step would.
    fn worked(&self, i: usize, (share, falls): (&[f64], &[f64]), per_m: f64) -> Option<f64> {
        let ways: Vec<usize> = (0..share.len()).filter(|&k| share[k] > 0.0 && falls[k] > 0.0).collect();
        let area = self.w.mesh.cells().area()[i];
        let step: f64 = ways.iter().map(|&k| per_m * share[k] * falls[k]).sum();
        if step <= 0.0 || per_m * ways.iter().map(|&k| share[k]).sum::<f64>() / area <= LIMIT {
            return None;
        }
        // The node first, then its ways, their surfaces from the node's, m.
        let m = ways.len();
        let (a, b): (Vec<u32>, Vec<u32>) = (1..=m as u32).map(|u| (0, u)).unzip();
        let g: Vec<f64> = ways.iter().map(|&k| per_m * share[k] / self.dt).collect();
        let cap: Vec<f64> = std::iter::once(area).chain(std::iter::repeat_n(HELD_CAP, m)).collect();
        let x: Vec<f64> = std::iter::once(0.0).chain(ways.iter().map(|&k| -falls[k])).collect();
        let held: Vec<f64> = std::iter::once(step).chain(std::iter::repeat_n(0.0, m)).collect();
        let again = |now: &[f64], us: &[usize]| ((0..=m).map(|p| x[p] + (now[p] - held[p]) / cap[p]).collect(), us.iter().map(|&u| g[u]).collect());
        let moved = by_size(&Net { a: &a, b: &b, g: &g, cap: &cap }, &x, self.dt, (&held, &[]), &|_, v| v, &again);
        Some(moved.flux.iter().sum::<f64>() / step)
    }
}

/// What the steepest way of cell `i` cannot take (`share`, of `out` of its `sv` m³, the head falling by `drop`) runs
/// down its other ways (`outs`), the steepest first, each as far as it takes. Returns the share that leaves and the
/// fall of the head, as `out` and `drop`.
fn run_on(cells: &Level, i: usize, (outs, rest): (&[Toward], &mut Vec<usize>), share: &mut [f64], sv: f64, (mut out, mut drop): (f64, f64)) -> (f64, f64) {
    let slope = |k: usize| {
        let u = cells.offset(i, cells.around(i).1[k] as usize);
        outs[k].drop.unwrap_or(0.0) / u[0].hypot(u[1])
    };
    rest.clear();
    rest.extend((0..outs.len()).filter(|&k| outs[k].drop.is_some() && outs[k].room > share[k] * sv));
    rest.sort_by(|&a, &b| slope(b).total_cmp(&slope(a)).then(a.cmp(&b)));
    for &k in rest.iter() {
        let more = ((1.0 - out) * sv).min(outs[k].room - share[k] * sv) / sv;
        (share[k], out, drop) = (share[k] + more, out + more, drop + more * outs[k].fall);
    }
    (out, drop)
}

/// A cell does not drain below the rim of a way it sent along (`Toward::rim`): each way's room counts all the cell
/// holds, so together they may take it lower, and liquid would cross a rim from below it. Where the cell, which of `sv`
/// m³ over its ground `zi` (area `a`) sends a share `out`, would end more than `still` under the highest rim of the
/// ways it uses, each takes less (`share`), in proportion, so it ends at that rim. Returns the share that leaves and the
/// fall of the head, as `out` and `drop`.
fn to_rims(outs: &[Toward], share: &mut [f64], (sv, zi, a, still): (f64, f64, f64, f64), (out, drop): (f64, f64)) -> (f64, f64) {
    let rim = outs.iter().zip(share.iter()).filter(|p| *p.1 > 0.0).map(|p| p.0.rim).fold(f64::MIN, f64::max);
    if zi + sv * (1.0 - out) / a >= rim - still {
        return (out, drop);
    }
    let less = ((sv - (rim - zi) * a) / (out * sv)).clamp(0.0, 1.0);
    share.iter_mut().for_each(|f| *f *= less);
    (out * less, drop * less)
}

/// The crossings of a pass as the columns of a link flux, row `r` in column `col_of[r]`: the links that carried
/// something, in order, their ends, and what crossed each (`[c * links + u]`).
fn by_link(crossed: &Crossed, rows: usize, col_of: &[usize], (a, b): (&[u32], &[u32])) -> (Vec<u32>, Vec<u32>, Vec<f64>) {
    let cols = col_of.iter().max().map_or(0, |c| c + 1);
    let mut slot = vec![u32::MAX; a.len()];
    crossed.links.iter().for_each(|&l| slot[l as usize] = 0);
    let used: Vec<u32> = (0..a.len() as u32).filter(|&l| slot[l as usize] == 0).collect();
    used.iter().enumerate().for_each(|(u, &l)| slot[l as usize] = u as u32);
    let (links, mut mol) = (used.len(), vec![0.0; cols * used.len()]);
    for (&l, m) in crossed.links.iter().zip(crossed.amount.chunks_exact(rows)) {
        let u = slot[l as usize] as usize;
        col_of.iter().enumerate().for_each(|(r, &c)| mol[c * links + u] += m[r]);
    }
    let ends = |v: &[u32]| used.iter().map(|&l| v[l as usize]).collect();
    (ends(a), ends(b), mol)
}

/// The work `bedwork`, J, of the liquid leaving node `i` (a share `out`) on its load: it drops what is over capacity,
/// else picks up loose solids up to it (at most all that lies loose). Returns the work left to break bonded ground: the
/// share of the capacity its load leaves free (the cover effect, Sklar and Dietrich 2004).
fn load(w: &World, s: &Scratch, i: usize, q: &mut [f64], (rows, sol): (std::ops::Range<usize>, &[usize]), bedwork: f64, out: f64) -> f64 {
    let held: f64 = rows.clone().zip(sol).map(|(r, &k)| q[r] * s.sp[k].v[0]).sum();
    let capv = w.cfg.carry * bedwork * bare_share(w, i);
    if out <= 0.0 {
        return 0.0;
    }
    if held > capv {
        rows.for_each(|r| q[r] *= capv / held);
        return 0.0;
    }
    let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
    let f = if avail > 0.0 { ((capv - held) / avail).min(1.0) } else { 0.0 };
    rows.zip(sol).for_each(|(r, &k)| q[r] += solid(w, s, k, i) * f);
    if capv > 0.0 { bedwork * (1.0 - (held + f * avail) / capv).max(0.0) } else { 0.0 }
}

/// Break bonded ground of node `i` into loose by `bedwork` over bond strength (`wear`), each substance by its share of
/// the bed's volume, at most all of it: moves from `Class::Bed` gathered in `broke`. Its heat was already the node's.
fn wear(w: &World, s: &mut Scratch, i: usize, bedwork: f64, broke: &mut Vec<Move>) {
    let n = w.mesh.cells().n();
    for bi in 0..s.state.bedk.len() {
        let (k, bv) = (s.state.bedk[bi], s.state.bed[i].vol);
        let b = w.store.amounts(Class::Bed)[k * n + i];
        if b > 0.0 && bv > 0.0 {
            let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(STRENGTH_MIN);
            let go = Move { from: Node::new(Class::Bed, i), to: Node::ground(i), k: k as u16, mol: (v / s.sp[k].v[0]).min(b), heat: 0.0 };
            let area = w.mesh.cells().area()[i];
            let (m, dz) = (go.mol, go.mol * s.sp[k].v[0] / area);
            broke.push(go);
            s.state.bed[i].vol -= m * s.sp[k].v[0];
            s.add_loose(k, false);
            s.note(k, i, m, area);
            lift(s, i, [-dz, -dz]);
        }
    }
}

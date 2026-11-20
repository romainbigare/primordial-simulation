//! Wind: what moves air between neighbouring air nodes (laws.md 1.6): the three pushes on each link as rates, from the
//! air of the step's start (`pushes`), and what the air's days move by them (`daily`): each block evened (`even`), then
//! the wind across the links between blocks (`wind`).

use super::super::Scratch;
use super::daily::{temps, Moved};
use super::edge::{beyond_offset, columns};
use super::cell::Cell;
use super::stretch::Stretch;
use crate::{solve::line::{self, Lines}, solve::sweep::HALF_WAY, space::Level, units::R_GAS, Config, World, AIR};

/// A fit of a gradient whose neighbours nearly line up (the determinant below this share of the product of its
/// diagonal) is made along each axis alone.
const SINGULAR: f64 = 1e-9;

/// The links of the air level that share a border (width above 0), with their ends.
pub(super) struct Links {
    pub(super) id: Vec<usize>,
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
}

impl Links {
    pub(super) fn of(air: &Level) -> Links {
        let id: Vec<usize> = (0..air.a().len()).filter(|&l| air.width()[l] > 0.0).collect();
        let (a, b) = id.iter().map(|&l| (air.a()[l], air.b()[l])).unzip();
        Links { id, a, b }
    }
}

/// The pushes on each link (`Links`): mixing by eddies, mol/s each way, and the heat they carry from its `a` to its
/// `b` over what the two columns' mean heat per mol carries, W (`stir`); overturning, mol/s of the lower half from its
/// `a` to its `b` (the upper half comes back), and the heat its halves carry from its `b` to its `a` over what their
/// mean heat per mol carries, W (`contrast`); and the conductance of the evening of pressure, mol/(Pa s).
#[derive(Clone)]
pub(super) struct Pushes {
    pub(super) mix: Vec<f64>,
    pub(super) stir: Vec<f64>,
    pub(super) over: Vec<f64>,
    pub(super) contrast: Vec<f64>,
    pub(super) even: Vec<f64>,
    /// STUDY PROBE D: the weather's wind, whole columns, mol/s from `a` to `b` (empty: none)
    pub(super) adv: Vec<f64>,
}

/// One end of a link as its pushes read it (`push`): its air's mol per m³, the mean height of its ground, m, its
/// latitude, degrees, its warmth's gradient [east, south], K per m of the map (the warmth, K, its temperature raised to
/// a common height by its drop with height); the rise of a mol's static
/// energy over its column's height, J/(mol m); the energy a mol of its near-ground air holds over its column's mean heat
/// per mol, J/mol (`aloft`); the speed of sound in it, m/s; and its pressure per mol, Pa/mol.
pub(super) struct End {
    pub(super) rho: f64,
    pub(super) aloft: f64,
    pub(super) za: f64,
    pub(super) lat: f64,
    pub(super) grad: [f64; 2],
    pub(super) rise: f64,
    pub(super) sound: f64,
    pub(super) c: f64,
}

impl End {
    /// Air node `j` as the last state left it (`Scratch::nt`, `capa`, `mass`, `ta`, `gam`) and its warmth's gradient
    /// `grad`.
    pub(super) fn of(cfg: &Config, s: &Scratch, air: &Level, j: usize, grad: [f64; 2]) -> End {
        // A mol's static energy, cp T + M g z, grows with height by M g - cp gam as its temperature falls by gam.
        let rise = (cfg.gravity * s.mass[j] - s.capa[j] * s.gam[j]) / s.nt[j].max(f64::MIN_POSITIVE);
        // gamma = cp / (cp - R) per mol; p = c n with c = RT/V.
        let cp = s.capa[j] / s.nt[j];
        let sound = (cp / (cp - R_GAS) * R_GAS * s.ta[j] * s.nt[j] / s.mass[j]).sqrt();
        let aloft = aloft(cp, s.mass[j] / s.nt[j] * cfg.gravity, s.gam[j], (s.zair[j], s.za[j]));
        End { rho: s.nt[j] / s.vol[j], aloft, za: s.za[j], lat: air.lat()[j], grad, rise, sound, c: R_GAS * s.ta[j] / s.vol[j] }
    }
}

/// What a mol of a column's near-ground air holds over the column's mean heat per mol, J/mol, the column of `cp`
/// J/(mol K) cooling by `gam` K/m with height, its mean at `zair` m, its ground at `za` m, a mol weighing `mg` N: the
/// near-ground air's static energy cp T + M g z (its temperature at the ground, `at_height`) less the mean's cp T. So
/// a column's near-ground energy per mol is its heat per mol plus this.
pub(super) fn aloft(cp: f64, mg: f64, gam: f64, (zair, za): (f64, f64)) -> f64 {
    cp * gam * (zair - za) + mg * za
}

/// The pushes on a link from end `a` to end `b`, of width `wd` and length `d` as the stretched air takes them
/// (`Stretch::length`), `ab` apart on the map, m [east, south]: its eddies, mol/s each way; its overturning, mol/s of the
/// lower half from `a` to `b`; the heat its halves carry from `b` to `a` over their mean heat per mol, W; the conductance
/// of its evening of pressure, mol/(Pa s) (`Pushes`); and the heat its eddies carry from `a` to `b` over what the
/// columns' mean heat per mol carries, W: each mol they exchange holds its column's near-ground energy (`aloft`), as
/// eddies stir the lower air, where the contrast is the ground's. The upper half holds more heat per mol than the lower
/// (the rise of the static energy over half the column's depth), and the part of the overturning down the gradient
/// carries that from the warmer column to the colder; the part turned across the link runs along the isotherms, so what
/// its halves carry of it warms no column (a flow along the isotherms crosses none; Holton 2004, 3.4) and it carries
/// none. Down the gradient it runs inside the month's overturning cell (`Cell::share`).
pub(super) fn push(cfg: &Config, st: Stretch, (a, b): (&End, &End), (wd, d, ab): (f64, f64, [f64; 2]), cell: &Cell) -> [f64; 5] {
    let (rho, depth) = ((a.rho + b.rho) / 2.0, cfg.air_top_m - (a.za + b.za) / 2.0);
    let lat = (a.lat + b.lat) / 2.0;
    let (turn, cell) = (cfg.rotation * lat.to_radians().sin(), cell.share(lat));
    // The warmth's differences along the link and across it, as the mean of its ends' fitted gradients times its
    // offset: on the map's metres, as the stretch lengthens both alike. A fitted gradient reads the warmth over each
    // end's neighbours, so a wave no longer than two nodes, the mesh's own mode and no flow of air's, drives none.
    let (ga, gb) = (a.grad, b.grad);
    let along = ((ga[0] + gb[0]) * ab[0] + (ga[1] + gb[1]) * ab[1]) / 2.0;
    let across = ((ga[0] + gb[0]) * ab[1] - (ga[1] + gb[1]) * ab[0]) / 2.0;
    let mix = st.per_column(cfg.mix * rho * depth * wd / d);
    // A mass flux by the gradient, the difference along the link over its length `d`, through its half cross-section,
    // so it does not depend on `d`: down the gradient inside the cell of the low latitudes (`in_cell`).
    let per_k = st.per_column(cfg.wind / d * rho * wd * depth / 2.0);
    let (down, over) = (per_k * cell * along, per_k * (cell * along + turn * across));
    // The middles of a column's halves lie half its depth apart.
    let contrast = down * (a.rise + b.rise) / 2.0 * depth / 2.0;
    // Sound crosses the link in d / its speed. The stretched column holds the stretch squared times the mol at the same
    // pressure, so its c is that much less, and its rate per column of the map (`Stretch::per_column`) is this, with
    // the map's c, over the stretched `d`.
    let even = (a.sound + b.sound) / 2.0 / (d * (a.c + b.c));
    [mix, over, contrast, even, mix * (a.aloft - b.aloft)]
}

/// Three pushes on each link (`push`): the column's pressure, evened in the time sound takes to cross it (Lamb waves,
/// Lindzen 1990, 9.2); overturning by the warmth gradient, its lower half toward the warmer node at `wind` per K/m
/// inside the month's cell of the low latitudes (`Cell`), turned by the rotation at the link's latitude; and eddy
/// mixing (`mix` × width / length). Each over the link's length and
/// width as the stretched air takes them, and a rate between the stretched columns (`Stretch`). From the air of each
/// node as the last state left it (`ends`).
pub(super) fn pushes(cfg: &Config, st: Stretch, ends: &[End], air: &Level, (links, cell): (&Links, &Cell)) -> Pushes {
    let v = || Vec::with_capacity(links.id.len());
    let mut p = Pushes { mix: v(), stir: v(), over: v(), contrast: v(), even: v(), adv: Vec::new() };
    for &l in &links.id {
        let (a, b, wd, d) = (air.a()[l] as usize, air.b()[l] as usize, st.length(air.width()[l]), st.length(air.dist()[l]));
        let [mix, over, contrast, even, stir] = push(cfg, st, (&ends[a], &ends[b]), (wd, d, air.offset(a, b)), cell);
        p.mix.push(mix);
        p.stir.push(stir);
        p.over.push(over);
        p.contrast.push(contrast);
        p.even.push(even);
    }
    p
}

/// Each air node as an end of its links (`End`), as the last state left it: its warmth, its temperature raised to a
/// common height by its drop with height, and that's gradient (`gradient`).
pub(super) fn ends(w: &World, s: &Scratch) -> Vec<End> {
    let (cfg, air) = (&w.cfg, w.mesh.level(AIR));
    let warmth: Vec<f64> = (0..air.n()).map(|j| s.ta[j] + s.gam[j] * s.zair[j]).collect();
    // An edge node's neighbours include the columns of the planet's air beyond it (`edge::columns`), as a node inside
    // the map has its neighbours on every side.
    let mut beyond = vec![Vec::new(); air.n()];
    for (j, side, _, b) in columns(w) {
        beyond[j].push((beyond_offset(side, air.area()[j].sqrt()), b.warmth));
    }
    let grad = gradient(air, &warmth, &beyond);
    (0..air.n()).map(|j| End::of(cfg, s, air, j, grad[j])).collect()
}

/// The gradient of `v` at each node [east, south], per m: the least-squares fit of its differences to its bordering
/// nodes and to the points `beyond` it (per node, each its offset, m, and value; none where it is empty), 0 along an
/// axis with none.
pub(super) fn gradient(air: &Level, v: &[f64], beyond: &[Vec<([f64; 2], f64)>]) -> Vec<[f64; 2]> {
    (0..air.n())
        .map(|j| {
            let (mut m, mut r) = ([0.0; 3], [0.0; 2]);
            let links = air.around(j).0.iter().zip(air.around(j).1).filter(|e| air.width()[*e.0 as usize] > 0.0);
            let points = links.map(|(_, &k)| (air.offset(j, k as usize), v[k as usize])).chain(beyond.get(j).into_iter().flatten().copied());
            for (d, value) in points {
                let dv = value - v[j];
                m = [m[0] + d[0] * d[0], m[1] + d[0] * d[1], m[2] + d[1] * d[1]];
                r = [r[0] + d[0] * dv, r[1] + d[1] * dv];
            }
            let det = m[0] * m[2] - m[1] * m[1];
            if det > SINGULAR * m[0] * m[2] {
                [(m[2] * r[0] - m[1] * r[1]) / det, (m[0] * r[1] - m[1] * r[0]) / det]
            } else {
                [r[0] / m[0].max(f64::MIN_POSITIVE), r[1] / m[2].max(f64::MIN_POSITIVE)]
            }
        })
        .collect()
}

/// Each block's air evened out: every gas of each node in proportion to its volume, and the heat so that the warmth of
/// each (its temperature at its mean height, raised to a common height by the block's drop with height, as the wind's
/// overturning reads it, `wind::pushes`) is the block's; so its pressure, shares and warmth are the block's. What
/// moves goes from the block's hub to each other node, pairs `first` on (`Moved`); the node the mass enters pays its
/// lift.
pub(super) fn even(w: &World, s: &Scratch, blocks: &[Vec<u32>], (x, e): (&mut [f64], &mut [f64]), first: usize, moved: &mut Moved) {
    let (na, ng, pairs, g) = (e.len(), s.state.gas.len(), moved.heat.len(), w.cfg.gravity);
    let mut p = first;
    for nodes in blocks.iter().filter(|b| b.len() > 1) {
        let (hub, vol) = (nodes[0] as usize, nodes.iter().map(|&j| s.vol[j as usize]).sum::<f64>());
        let gas: Vec<f64> = (0..ng).map(|q| nodes.iter().map(|&j| x[q * na + j as usize]).sum()).collect();
        let heat: f64 = nodes.iter().map(|&j| e[j as usize]).sum();
        // Each node's heat capacity once evened, J/K; the block's drop with height, K/m, and its warmth, K: its heat with
        // what that drop takes from its mean height, over its capacity.
        let capa = |j: usize| s.state.gas.iter().enumerate().map(|(q, &k)| gas[q] * s.vol[j] / vol * s.sp[k].cp).sum::<f64>();
        let caps: Vec<f64> = nodes.iter().map(|&j| capa(j as usize)).collect();
        let all = caps.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        let gam = nodes.iter().zip(&caps).map(|(&j, c)| c * s.gam[j as usize]).sum::<f64>() / all;
        let warmth = (heat + nodes.iter().zip(&caps).map(|(&j, c)| c * gam * s.zair[j as usize]).sum::<f64>()) / all;
        for (&m, cap) in nodes[1..].iter().zip(&caps[1..]) {
            let (j, share) = (m as usize, s.vol[m as usize] / vol);
            let mut kg = 0.0;
            for (q, &k) in s.state.gas.iter().enumerate() {
                let d = gas[q] * share - x[q * na + j];
                x[q * na + j] += d;
                x[q * na + hub] -= d;
                moved.mol[q * pairs + p] += d;
                kg += d * s.sp[k].mm;
            }
            let d = cap * (warmth - gam * s.zair[j]) - e[j];
            e[j] += d;
            e[hub] -= d;
            moved.heat[p] += d;
            lift((hub, j), kg, (s, g), e);
            p += 1;
        }
    }
}

/// `kg` of mass moves from node `a` to node `b` (back where negative): the node it enters pays its lift from its heat,
/// from the mean ground of the node it leaves to its own (`potential`).
pub(super) fn lift((a, b): (usize, usize), kg: f64, (s, g): (&Scratch, f64), e: &mut [f64]) {
    let (from, to) = if kg > 0.0 { (a, b) } else { (b, a) };
    e[to] -= g * kg.abs() * (s.za[to] - s.za[from]);
}

/// The lines (`solve::line`) that the links between blocks (`border`, of `links`) make, by their place in `border` (as
/// `Moved` holds their ends `a`, `b`): those east and west, then those north and south, as every link of the air that
/// shares a border runs (`Square::mesh`).
pub(super) fn lines(air: &Level, (links, border): (&Links, &[usize]), (a, b): (&[u32], &[u32])) -> [Lines; 2] {
    let east = |p: &usize| [0, 4].iter().any(|&d| air.way(a[*p] as usize, d).is_some_and(|e| air.adj_link()[e] as usize == links.id[border[*p]]));
    let (zonal, meridional): (Vec<usize>, Vec<usize>) = (0..border.len()).partition(east);
    [zonal, meridional].map(|chosen| line::lines(a, b, &chosen, air.n()))
}

/// The implicit part of each day of the wind (`wind`), from the air of the step's start: for each group of what moves
/// alike (each gas that condenses, by its own halves; the other gases alike; the heat, by mol) and each class of
/// `lines`, the flows of the links (`solve::line`) and their step, factored.
pub(super) struct Implicit {
    groups: Vec<(Vec<usize>, [Flows; 2])>,
}

/// The flows of the links of a class of lines for a group (`Implicit`): per link `ca`, `cb`, `k`, and their step.
struct Flows {
    ca: Vec<f64>,
    cb: Vec<f64>,
    k: Vec<f64>,
    step: line::Step,
}

impl Implicit {
    /// The eddies and the overturning (`Pushes`) over the links between blocks (`border`, their `lines`, their ends
    /// `a`, `b`), for days of `dd` seconds, from the air `x` (`[gi * na + j]`).
    pub(super) fn of(s: &Scratch, (push, border, lines): (&Pushes, &[usize], &[Lines; 2]), (a, b): (&[u32], &[u32]), x: &[f64], dd: f64) -> Implicit {
        let (na, ng, nb) = (s.nt.len(), s.state.gas.len(), border.len());
        let [nt, upper, lower] = halves(s, x, na);
        let flows = |q: usize, class: &Lines| {
            let (mut ca, mut cb, mut k) = (vec![0.0; nb], vec![0.0; nb], vec![0.0; nb]);
            for p in class.links.iter().map(|&p| p as usize) {
                let (ia, ib, l) = (a[p] as usize, b[p] as usize, border[p]);
                let (mix, o) = (push.mix[l], push.over[l]);
                // The share of what a node holds that leaves it in a half of its column (its upper half, `up`): the
                // lower half of the colder node, the upper half of the warmer; the heat by mol.
                let half = |j: usize, up: bool| match (q < ng, up) {
                    (false, _) => 1.0 / nt[j],
                    (true, true) => s.split[q * na + j] / upper[j],
                    (true, false) => (1.0 - s.split[q * na + j]) / lower[j],
                };
                ca[p] = mix / nt[ia] + o.abs() * half(ia, o < 0.0);
                cb[p] = mix / nt[ib] + o.abs() * half(ib, o > 0.0);
                k[p] = if q < ng { 0.0 } else { push.stir[l] - push.contrast[l] };
            }
            let step = line::factor(class, a, (&ca, &cb), dd);
            Flows { ca, cb, k, step }
        };
        let (cond, dry): (Vec<usize>, Vec<usize>) = (0..ng).partition(|&q| s.cond[q]);
        let groups = cond.iter().map(|&q| vec![q]).chain([dry, vec![ng]]).filter(|g| !g.is_empty());
        Implicit { groups: groups.map(|g| (g.clone(), [flows(g[0], &lines[0]), flows(g[0], &lines[1])])).collect() }
    }
}

/// A gas of the air, or the heat where it is the number of gases, in a day of the wind (`wind`): its flows along each
/// class of lines (`Implicit`) and what crosses their links.
type Job<'a> = (usize, [&'a Flows; 2], [Vec<f64>; 2]);

/// One day of `dd` seconds of the wind over the links between blocks (`border`, their `lines`). First the evening of
/// pressure (`pressure`); then the eddies and the overturning, each implicit along the lines east and west, then along
/// those north and south (`Implicit`, `solve::line`), so the day is stable and keeps its rates on any grid; each gas
/// and the heat on its own task of the pool. What crosses is the sender's air, each mol with its node's heat per mol,
/// a mol of the eddies with its node's near-ground energy (`Pushes::stir`); for the overturning by halves, the lower
/// half to the warmer node and the upper half back, as many mol each way, the halves carrying `Pushes::contrast`. The
/// node the mass enters pays its lift.
pub(super) fn wind(w: &World, s: &Scratch, (push, border, lines, imp): (&Pushes, &[usize], &[Lines; 2], &Implicit), (x, e): (&mut [f64], &mut [f64]), dd: f64, moved: &mut Moved) {
    if border.is_empty() {
        return;
    }
    let (na, ng, nb) = (e.len(), s.state.gas.len(), border.len());
    let mm = masses(s);
    // The mass that crosses each link in the day, kg, + from its `a` to its `b`.
    let mut kg = vec![0.0; nb];
    pressure(s, (push, border), (x, e), dd, (moved, &mm, &mut kg));
    let (a, b) = (&moved.a[..nb], &moved.b[..nb]);
    // Per gas and the heat (`q`, the heat at `ng`), what crosses the links of each class of lines.
    let mut jobs: Vec<Job> = imp.groups.iter().flat_map(|(g, f)| g.iter().map(move |&q| (q, [&f[0], &f[1]], [Vec::new(), Vec::new()]))).collect();
    let (xs, es) = (&*x, &*e);
    crate::par::each_init(&mut jobs, na * (ng + 1), || (), |_, (q, flows, out)| {
        let mut y = if *q < ng { xs[*q * na..(*q + 1) * na].to_vec() } else { es.to_vec() };
        for ((class, f), out) in lines.iter().zip(flows.iter()).zip(out.iter_mut()) {
            let start = y.clone();
            line::solve(class, &f.step, a, (*q == ng).then_some(&f.k[..]), &start, &mut y);
            *out = class.links.iter().map(|&p| p as usize).map(|p| dd * (f.ca[p] * y[a[p] as usize] - f.cb[p] * y[b[p] as usize] + f.k[p])).collect();
        }
    });
    for (q, _, out) in &jobs {
        for (class, d) in lines.iter().zip(out) {
            cross((*q, class.links.iter().map(|&p| p as usize).zip(d.iter().copied())), (x, e), (moved, &mm, &mut kg));
        }
    }
    for (p, &kg) in kg.iter().enumerate() {
        lift((moved.a[p] as usize, moved.b[p] as usize), kg, (s, w.cfg.gravity), e);
    }
}

/// Each node's air, mol, and its upper and lower halves (`Scratch::split`), from the amounts `x`.
fn halves(s: &Scratch, x: &[f64], na: usize) -> [Vec<f64>; 3] {
    let sum = |f: &dyn Fn(usize, usize) -> f64, j: usize| (0..s.state.gas.len()).map(|q| f(q, j)).sum::<f64>();
    let nt: Vec<f64> = (0..na).map(|j| sum(&|q, j| x[q * na + j], j)).collect();
    let upper: Vec<f64> = (0..na).map(|j| sum(&|q, j| x[q * na + j] * s.split[q * na + j], j)).collect();
    let lower: Vec<f64> = (0..na).map(|j| nt[j] - upper[j]).collect();
    [nt, upper, lower]
}

/// The evening of pressure over a day of `dd` seconds (`wind`), explicit, from the air at its start: the column's
/// pressure (p = nRT/V) evened in the time sound takes to cross the link, the sender's air, each link at most
/// `HALF_WAY` of what evens it over the links of the busier of its nodes, so no node gives more than half of each gas.
/// Its mass is added to `kg` (`cross`).
fn pressure(s: &Scratch, (push, border): (&Pushes, &[usize]), (x, e): (&mut [f64], &mut [f64]), dd: f64, (moved, mm, kg): (&mut Moved, &[f64], &mut [f64])) {
    let (na, ng, nb) = (e.len(), s.state.gas.len(), border.len());
    let t = temps(s, x, e);
    let nt: Vec<f64> = (0..na).map(|j| (0..ng).map(|q| x[q * na + j]).sum()).collect();
    let c: Vec<f64> = (0..na).map(|j| R_GAS * t[j] / s.vol[j]).collect();
    let mut links_of = vec![0usize; na];
    moved.a[..nb].iter().chain(&moved.b[..nb]).for_each(|&j| links_of[j as usize] += 1);
    // Per link the share of its sender's air that crosses, + from its `a` to its `b`, and the sender.
    let send: Vec<(f64, usize)> = border
        .iter()
        .enumerate()
        .map(|(p, &l)| {
            let (ia, ib) = (moved.a[p] as usize, moved.b[p] as usize);
            let even = (dd * push.even[l] * (c[ia] + c[ib])).min(1.0).min(HALF_WAY / links_of[ia].max(links_of[ib]) as f64);
            let push_mol = even * (nt[ia] * c[ia] - nt[ib] * c[ib]) / (c[ia] + c[ib]);
            let sender = if push_mol > 0.0 { ia } else { ib };
            (push_mol / nt[sender], sender)
        })
        .collect();
    for q in 0..=ng {
        let held: Vec<f64> = if q < ng { x[q * na..(q + 1) * na].to_vec() } else { e.to_vec() };
        cross((q, send.iter().enumerate().map(|(p, &(share, from))| (p, share * held[from]))), (x, e), (moved, mm, kg));
    }
}

/// The mass of a mol of each gas of the air, kg, then 0 for the heat (as `cross` counts them).
pub(super) fn masses(s: &Scratch) -> Vec<f64> {
    s.state.gas.iter().map(|&k| s.sp[k].mm).chain([0.0]).collect()
}

/// What crosses links between blocks of gas `q`, or of the heat where `q` is the number of gases (each link `p` and
/// `d`, + from its `a` to its `b`), moved and counted (`Moved`), its mass added to `kg` (`masses`, `mm`).
pub(super) fn cross((q, flows): (usize, impl Iterator<Item = (usize, f64)>), (x, e): (&mut [f64], &mut [f64]), (moved, mm, kg): (&mut Moved, &[f64], &mut [f64])) {
    let (na, pairs) = (e.len(), moved.heat.len());
    let (v, count) = if q < mm.len() - 1 { (&mut x[q * na..(q + 1) * na], &mut moved.mol[q * pairs..(q + 1) * pairs]) } else { (&mut e[..], &mut moved.heat[..]) };
    for (p, d) in flows {
        let (ia, ib) = (moved.a[p] as usize, moved.b[p] as usize);
        v[ia] -= d;
        v[ib] += d;
        count[p] += d;
        kg[p] += mm[q] * d;
    }
}

#[cfg(test)]
mod tests {
    use super::super::stretch::Stretch;
    use super::super::cell::Cell;
    use super::{aloft, push, End};
    use crate::Config;

    /// Two columns with the same air at the ground but different heat aloft (one cooling faster with height, so a lower
    /// mean heat per mol) hold the same near-ground energy per mol, and their eddies carry no heat; a contrast of the
    /// air at the ground carries cp per K of it per mol exchanged.
    #[test]
    fn the_eddies_carry_the_near_ground_airs_energy() {
        let (cp, mg, (zair, za), ground) = (29.1, 0.285, (4000.0, 200.0), 288.0);
        // A mol's heat at the column's mean, cp T there, and its near-ground energy.
        let near = |gam: f64, ground: f64| {
            let mean = cp * (ground - gam * (zair - za));
            mean + aloft(cp, mg, gam, (zair, za))
        };
        assert!((near(0.0045, ground) - near(0.0080, ground)).abs() < 1e-9, "heat aloft moved heat");
        assert!((near(0.0065, ground + 2.0) - near(0.0065, ground) - 2.0 * cp).abs() < 1e-9);
        assert!((near(0.0065, ground) - cp * ground - mg * za).abs() < 1e-9);
    }

    /// The overturning's halves carry the static energy the upper holds over the lower from the warmer column to the colder
    /// as far as they run down the gradient, and none as far as they run along the isotherms: at 45° (past the cell),
    /// two columns whose warmth falls across their link move air along it, and no heat by their halves; inside the
    /// cell, where it rises along the link, its warmer end draws heat from it.
    #[test]
    fn the_overturning_along_the_isotherms_carries_no_heat_by_its_halves() {
        let cfg = Config::default();
        let end = |lat: f64, grad: [f64; 2]| End { rho: 40.0, aloft: 0.0, za: 0.0, lat, grad, rise: 1.5, sound: 1e-6, c: 1e-3 };
        let (st, link) = (Stretch(1.0), (1e5, 1e5, [1e5, 0.0]));
        let [_, over, contrast, _, _] = push(&cfg, st, (&end(45.0, [0.0, 1e-5]), &end(45.0, [0.0, 1e-5])), link, &Cell::year(&cfg));
        assert!(over != 0.0 && contrast == 0.0, "along the isotherms: {over} mol/s, {contrast} W by the halves");
        let [_, over, contrast, _, _] = push(&cfg, st, (&end(10.0, [1e-5, 0.0]), &end(10.0, [1e-5, 0.0])), link, &Cell::year(&cfg));
        assert!(over > 0.0 && contrast > 0.0, "down the gradient: {over} mol/s, {contrast} W from the warmer end");
    }
}

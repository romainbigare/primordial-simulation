//! The cells and their air (laws.md 1.4, 1.5): each cell's exchange over a step with the air it sees, solved once in its
//! own implicit surface balance at the air of the step's start and taken as linear in that air, its slopes from that
//! balance on the branch it ended on (`exchange`). The air level's days sum that one law over each node's cells
//! (`Response::ground`), and the book takes each cell's at the step's mean air (`Response::booked`), so the air and its
//! cells agree.

use super::super::Scratch;
use super::daily::Ground;
use super::rain::Wet;
use super::radiation::{on_slope, sun_by_latitude, Air, Surface, DOWN};
use super::step::Active;
use super::outs::{carried, column, hold, put};
use super::soak::{cols, Soak, SOAK};
use super::surf::{end_reflectance, parts_respond, Patch, Surf, Vapour};
use super::{at_height, lerp};
use crate::{math::one_minus_exp, World, AIR};

/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 8;
const SURFACE_STILL_K: f64 = 1e-3;
/// The reflectance a ground node's balance takes is guessed again up to this many times, until it moves by less than
/// `REFLECT_STILL` from the one its end state gives.
const REFLECT_ROUNDS: usize = 8;
const REFLECT_STILL: f64 = 1e-3;

/// The outputs of a cell's exchange that change with its air: what its air node gains over the step, J (sensible heat,
/// the heat radiation the air absorbs less what it sends down, and the heat of the gas that goes up less its lift,
/// `potential`); the heat over its link to the air as the store books it, J (with the gas's latent heat); the heat it
/// sends to space, J; then per active gas what evaporates, mol.
pub(super) const GAINS: usize = 0;
pub(super) const LINK: usize = 1;
pub(super) const SPACE: usize = 2;
pub(super) const UP: usize = 3;
/// The outputs that do not: the sunlight it absorbs, J; the light per unit of its cover's capture area, J/m²; the light
/// set aside for its cover, J; the sunlight the air over it and its cloud absorb on its way down and back up, J
/// (`Light::shares`).
pub(super) const SUN: usize = 0;
pub(super) const LIGHT: usize = 1;
pub(super) const KEPT: usize = 2;
pub(super) const AIR_SUN: usize = 3;
pub(super) const FIXED: usize = 4;
/// The parts the changing outputs are made of (`put`), each laid out as an output is: the heat the cell gives its air
/// node and the heat it sends to space, J (`Balance`), then what evaporates of each active gas, mol.
pub(super) const TO_AIR: usize = 0;
pub(super) const TO_SPACE: usize = 1;
pub(super) const GAS: usize = 2;

/// What a pass reads: the world, the active gases, the air of the step's start (per node its temperature, K; per active
/// gas and node, `[x * na + j]`, mol), the step's length and the sun's declination on each of its days.
pub(super) struct Ctx<'a> {
    pub(super) w: &'a World,
    pub(super) s: &'a Scratch,
    pub(super) act: &'a [Active],
    pub(super) t0: &'a [f64],
    pub(super) x0: &'a [f64],
    pub(super) dt: f64,
    pub(super) decl: &'a [f64],
    /// What the air of each node does to sunlight (`Light`)
    pub(super) light: &'a [super::radiation::Light],
}

/// Each cell's response to the air it sees: per cell the fixed outputs, then, of a cell alone, each changing output's
/// value at the air it sees at the step's start, its slope per K of that air, and per mol of each active gas there; and
/// that air. A body of standing liquid under each air node answers as one (`Patch`): its cells hold only their fixed outputs. A
/// cell whose land and liquid's top both face the air (`parts`) answers as those two, its rows their sum; of each such
/// cell, its top's rows too, its fixed outputs and then its changing ones (`wet`).
pub(super) struct Response {
    nx: usize,
    rows: Vec<f64>,
    seen: Vec<f64>,
    patches: Vec<Patch>,
    /// Per cell: does it answer alone (not in a patch)?
    alone: Vec<bool>,
    pub(super) wet: Vec<(u32, Vec<f64>)>,
}

impl Response {
    pub(super) fn width(nx: usize) -> usize {
        FIXED + changing(nx) + nx * cols(nx)
    }
    /// Its rows, for the next step's room (`spare::Spare`).
    pub(super) fn rows(self) -> Vec<f64> {
        self.rows
    }
    /// Fixed output `o` of cell `i`.
    pub(super) fn fixed(&self, i: usize, o: usize) -> f64 {
        self.rows[i * Self::width(self.nx) + o]
    }
    /// Each node's ground (`Ground`): what its air gains and what evaporates into it, summed over its cells alone and its
    /// patches, with its slopes with the air of each node they see, by their shares of it (`lerp`; a patch's by its
    /// cells' areas): node by node, its cells alone in the order of their indices, then its patches; each node it sees
    /// where one of its cells first does.
    pub(super) fn ground(&self, cx: &Ctx, wet: Wet) -> Ground {
        let (w, s, nx) = (cx.w, cx.s, self.nx);
        let (air, outs, width) = (w.mesh.level(AIR), 1 + nx, Self::width(nx));
        let per = outs * outs;
        let mut g = Ground::new(air.n(), nx, (cx.t0.to_vec(), cx.x0.to_vec()), wet);
        let c = &s.corners;
        (g.near, g.at, g.slope) = (c.sees.clone(), c.sees_at.clone(), vec![0.0; c.sees.len() * per]);
        let out = |o: usize| if o == 0 { GAINS } else { UP + o - 1 };
        // Its outputs `row` (as `put` lays them out) into node `j`, its slopes into each entry of the node's list by weight.
        let mut add = |j: usize, row: &[f64], sees: &mut dyn Iterator<Item = (usize, f64)>| {
            (0..outs).for_each(|o| g.base[j * outs + o] += row[out(o) * (2 + nx)]);
            for (at, wt) in sees {
                for (o, slope) in g.slope[at * per..(at + 1) * per].chunks_exact_mut(outs).enumerate() {
                    let by = &row[out(o) * (2 + nx) + 1..out(o) * (2 + nx) + 1 + outs];
                    slope.iter_mut().zip(by).for_each(|(d, r)| *d += wt * r);
                }
            }
        };
        for j in 0..air.n() {
            for &i in air.children(j).iter().filter(|&&i| self.alone[i as usize]) {
                let i = i as usize;
                let corners = c.entry[i].iter().zip(&s.wt[i]).filter(|e| *e.1 > 0.0).map(|(&at, &wt)| (at as usize, wt));
                add(j, &self.rows[i * width + FIXED..(i + 1) * width], &mut corners.into_iter());
            }
        }
        for p in &self.patches {
            add(w.mesh.cells().parent()[p.cells[0] as usize] as usize, &p.out, &mut p.sees.iter().map(|&(at, wt)| (at as usize, wt)));
        }
        // The sunlight the air of each node absorbs over its cells: a gain the air it sees does not move.
        for j in 0..air.n() {
            g.base[j * outs] += air.children(j).iter().map(|&i| self.rows[i as usize * width + AIR_SUN]).sum::<f64>();
        }
        g.soaks = self.soaks(w);
        g
    }
    /// The buckets the step's rain wets as it falls (`Soak`), of the cells alone in their order, each's active gases in
    /// theirs.
    pub(super) fn soaks(&self, w: &World) -> Vec<Soak> {
        let (nx, width, parent) = (self.nx, Self::width(self.nx), w.mesh.cells().parent());
        let at = |i: usize, y: usize| i * width + FIXED + changing(nx) + y * cols(nx);
        let each = |i: usize| (0..nx).filter_map(move |y| Soak::of(i, parent[i], y, &self.rows[at(i, y)..at(i, y) + cols(nx)]));
        (0..self.alone.len()).filter(|&i| self.alone[i]).flat_map(each).collect()
    }
    /// Each cell's changing outputs at the mean air of the step, as the book takes them (`[i * (UP + nx) + o]`): `t`
    /// per node, K, and `x[y]` per active gas, mol per node. A cell alone at the air it sees (`lerp`); a patch's at the
    /// air its cells see, by their areas, its parts shared to its cells (`Patch::share`).
    /// Of each cell of two faces (`wet`), its liquid's top's changing outputs at the same air, `[c * (UP + nx) + o]`.
    /// A cell's outputs move with what the rain added to what its buckets (`soaks`) evaporate, `soaked`, mol.
    pub(super) fn booked(&self, s: &Scratch, (t, x): (&[f64], &[&[f64]]), (soaks, soaked): (&[Soak], &[f64])) -> (Vec<f64>, Vec<f64>) {
        let (nx, width) = (self.nx, Self::width(self.nx));
        // Cell `i`'s changing outputs from its rows `rows` (as `rows` lays them out from `FIXED` on) at the mean air.
        let at = |i: usize, rows: &[f64], out: &mut [f64]| {
            let seen = &self.seen[i * (1 + nx)..(i + 1) * (1 + nx)];
            let (t, x) = (lerp(s, t, i) - seen[0], (0..nx).map(|y| lerp(s, x[y], i) - seen[1 + y]).collect::<Vec<f64>>());
            for (o, v) in out.iter_mut().enumerate() {
                let r = &rows[o * (2 + nx)..];
                *v = r[0] + r[1] * t + (0..nx).map(|y| r[2 + y] * x[y]).sum::<f64>();
            }
        };
        let mut out = vec![0.0; self.alone.len() * (UP + nx)];
        crate::par::rows(&mut out, UP + nx, crate::par::UNCOUNTED, || (), |_, i, row| {
            if self.alone[i] {
                at(i, &self.rows[i * width + FIXED..(i + 1) * width], row);
            }
        });
        let mut wet = vec![0.0; self.wet.len() * (UP + nx)];
        for ((i, rows), row) in self.wet.iter().zip(wet.chunks_exact_mut(UP + nx)) {
            at(*i as usize, &rows[FIXED..], row);
        }
        let stride = 2 + nx;
        let (mut parts, mut each) = (vec![0.0; (GAS + nx) * stride], vec![0.0; (UP + nx) * stride]);
        for p in &self.patches {
            let mean = |v: &[f64]| s.corners.mean(&p.sees, v);
            let (dt, dx): (f64, Vec<f64>) = (mean(t) - p.air[0], (0..nx).map(|y| mean(x[y]) - p.air[1 + y]).collect());
            let part = |r: usize| p.parts[r * stride] + p.parts[r * stride + 1] * dt + (0..nx).map(|y| p.parts[r * stride + 2 + y] * dx[y]).sum::<f64>();
            let at: Vec<f64> = (0..GAS + nx).map(part).collect();
            // Each cell's parts, its shares of the patch's (`Patch::share`), as its outputs (`put`).
            for (&i, share) in p.cells.iter().zip(p.share.chunks_exact(1 + nx)) {
                for (r, &v) in at.iter().enumerate() {
                    parts[r * stride] = v * if r < GAS { share[0] } else { share[1 + r - GAS] };
                }
                put(&parts, &p.carry, &mut each);
                out[i as usize * (UP + nx)..(i as usize + 1) * (UP + nx)].iter_mut().enumerate().for_each(|(o, v)| *v = each[o * stride]);
            }
        }
        for (soak, &m) in soaks.iter().zip(soaked) {
            let i = soak.cell as usize;
            out[i * (UP + nx)..(i + 1) * (UP + nx)].iter_mut().zip(&soak.per).for_each(|(v, per)| *v += m * per);
        }
        (out, wet)
    }
    /// Fixed output `o` of the liquid's top of the `c`-th cell of two faces (`wet`).
    pub(super) fn wet_fixed(&self, c: usize, o: usize) -> f64 {
        self.wet[c].1[o]
    }
}

/// Each cell's response over the step (`Response`), in parallel with `threads`, its rows in `room` (`spare::Spare`).
pub(super) fn respond(cx: &Ctx, patches: &[Vec<u32>], room: Vec<f64>) -> Response {
    let (n, nx, na) = (cx.w.mesh.cells().n(), cx.act.len(), cx.w.mesh.level(AIR).n());
    // A body under each air node solves one surface (`surf::patches`); the other cells each their own.
    let mut alone = vec![true; n];
    patches.iter().flatten().for_each(|&i| alone[i as usize] = false);
    let mut seen = vec![0.0; n * (1 + nx)];
    for (i, air) in seen.chunks_exact_mut(1 + nx).enumerate().filter(|e| alone[e.0]) {
        air[0] = lerp(cx.s, cx.t0, i);
        (0..nx).for_each(|y| air[1 + y] = lerp(cx.s, &cx.x0[y * na..(y + 1) * na], i));
    }
    let width = Response::width(nx);
    // Every cell's fixed outputs are written: alone, or of a patch.
    let mut rows = room;
    rows.resize(n * width, 0.0);
    // Each task keeps the last latitude and its mean sunlight over the step: neighbouring cells often share a latitude.
    crate::par::rows(&mut rows, width, crate::par::UNCOUNTED, || (f64::NAN, [0.0; 2], Work::default()), |(lat, q, work), i, row| {
        if !alone[i] || cx.s.two_faces(i) {
            return;
        }
        let at = cx.w.mesh.cells().lat()[i];
        if at != *lat {
            (*lat, *q) = (at, sunlight(cx.w, cx.decl, at));
        }
        let ground = Surf::of_cell(cx, i, on_slope(*q, rise(cx, i)));
        let fixed = exchange(cx, &ground, &seen[i * (1 + nx)..(i + 1) * (1 + nx)], work);
        put(&work.p, &work.carry, &mut row[FIXED..]);
        row[FIXED + changing(nx)..].copy_from_slice(&work.soak);
        row[..FIXED].copy_from_slice(&fixed);
    });
    let mixed: Vec<usize> = (0..n).filter(|&i| alone[i] && cx.s.two_faces(i)).collect();
    let both = crate::par::map(&mixed, |&i| parts_respond(cx, i, &seen[i * (1 + nx)..(i + 1) * (1 + nx)]));
    let mut wet = Vec::with_capacity(mixed.len());
    for (&i, [land, liquid]) in mixed.iter().zip(both) {
        let row = &mut rows[i * width..(i + 1) * width];
        row.iter_mut().zip(land.iter().zip(&liquid)).for_each(|(r, (a, b))| *r = a + b);
        // The light per unit of the cover's capture area is the same on both parts.
        row[LIGHT] = land[LIGHT];
        wet.push((i as u32, liquid));
    }
    let patches = crate::par::map(patches, |cells| Patch::of(cx, cells));
    for p in &patches {
        for (&i, share) in p.cells.iter().zip(p.share.chunks_exact(1 + nx)) {
            let heat = share[0];
            rows[i as usize * width..i as usize * width + FIXED].copy_from_slice(&[p.fixed[SUN] * heat, p.fixed[LIGHT], p.fixed[KEPT] * heat, p.fixed[AIR_SUN] * heat]);
        }
    }
    Response { nx, rows, seen, patches, alone, wet }
}

/// The mean sunlight at the top of the air at latitude `lat`, degrees, over the days of the step, their sun's
/// declinations `decl`, W/m², and its slope with latitude, W/m² per radian (`sun_by_latitude`).
pub(super) fn sunlight(w: &World, decl: &[f64], lat: f64) -> [f64; 2] {
    let days = decl.iter().map(|&d| sun_by_latitude(w.cfg.sun, lat.to_radians(), d));
    days.fold([0.0; 2], |a, b| [a[0] + b[0], a[1] + b[1]]).map(|x| x / decl.len() as f64)
}

/// The rise toward the south of the surface of cell `i`, m per m (`World::rise_south`): none where it lies under the
/// body's liquid or all under its own, which stand level (`basin`, `Scratch::top_alone`).
pub(super) fn rise(cx: &Ctx, i: usize) -> f64 {
    if cx.s.basin.is[i] || cx.s.top_alone(i).is_some() { 0.0 } else { cx.w.rise_south[i] }
}

/// The buffers of one task's exchanges (`exchange`), kept from cell to cell: its active gases as their evaporation reads
/// them (`Vapour`), what each evaporates (`Vapour::evaporate`), its parts (`put`), what each mol of each gas carries
/// (`carried`), its changing outputs as a soft cover's end reflectance reads them (`end_reflectance`), the heat it
/// may lose (`hold`), and its buckets the step's rain wets as it falls (`soak::cols`, per active gas).
#[derive(Default)]
pub(super) struct Work {
    gas: Vec<Vapour>,
    ev: Vec<[f64; 3]>,
    pub(super) p: Vec<f64>,
    pub(super) carry: Vec<[f64; 2]>,
    out: Vec<f64>,
    loss: Vec<f64>,
    pub(super) soak: Vec<f64>,
}

/// The numbers of a cell's changing outputs in its row: per output its value and its slopes (`put`).
fn changing(nx: usize) -> usize {
    (UP + nx) * (2 + nx)
}

/// The exchange of the ground `g` over the step under the air it sees, `air` (its temperature, K, then each active gas,
/// mol): into `work`, its parts (`put`) and what each mol of each gas carries (`carried`); returned, its fixed outputs.
/// Sunlight through the air of its node, clear and under its cloud, down and back up over what the surface and its
/// cover reflect (`Light::shares`), what the cover intercepts less the share its light reaction stores (set aside for it,
/// laws.md 1.4); the surface balance with what evaporates, by Newton's steps about its end
/// temperature, under the reflectance its surface has at the step's start; where a soft cover lies, under the
/// reflectance it ends the step with, as backward Euler takes it (`end_reflectance`): from the start's, each guess
/// followed to the end it gives, and halved between two once a guess and its end cross, until they agree, so a cover
/// that melts away darkens the ground in its step. The slopes come from the last balance on the branch it ended on
/// (`Air::moves`), held to what the cell holds (`hold`).
pub(super) fn exchange(cx: &Ctx, g: &Surf, air: &[f64], work: &mut Work) -> [f64; FIXED] {
    let (w, s, dt, act) = (cx.w, cx.s, cx.dt, cx.act);
    let (na, j, area) = (w.mesh.level(AIR).n(), g.j, g.area);
    let (adt, tl, top) = (area * dt, at_height(air[0], g.gam, g.head, g.zair), g.sun);
    let a = g.canopy;
    let cover = if a > 0.0 { one_minus_exp(a) } else { 0.0 };
    let caught = cover * g.absorb;
    // Under the reflectance `alb` of its ground, with the share its cover and ground reflect and its air's shares of
    // the light at the top, clear and under its cloud (`Light::shares`): [the sunlight that reaches it, in all its
    // passes (per unit of capture area the cover intercepts cover / a of it, all of it where it is thin); what it
    // absorbs; what is set aside for its cover], W/m², and the share of the light at the top its air absorbs.
    let lit = |alb: f64| {
        let [ground, in_air, _] = cx.light[j].shares((1.0 - cover) * alb + cover * (1.0 - g.absorb));
        let sun = top * ground;
        let kept = sun * caught * g.keep;
        [sun, sun * ((1.0 - cover) * (1.0 - alb) + caught) - kept, kept, in_air]
    };
    // The heat radiation its air absorbs of what it sends, at the shares of its temperature at the step's start.
    let [down, slope] = s.heat[j].glow(&w.chem, tl, DOWN);
    let rad = Air { tl, f: s.heat[j].absorbs(&w.chem, g.tg), down, slope };
    // The share of the air it sees that lies over it (`over_cell`).
    let over = area / g.seen_area;
    let Work { gas, ev, p, carry, out, loss, soak } = work;
    gas.clear();
    gas.extend(act.iter().enumerate().map(|(y, &ac)| Vapour::of(cx, g, ac, air[1 + y], super::per_mol(over, tl, area, s.hk[ac.gi * na + j]))));
    let gas = &*gas;
    // The balance with what evaporates linearised about `t0`.
    let balance = |t0: f64, sw: f64| {
        let (mut gain, mut slope) = (0.0, 0.0);
        for g in gas.iter().filter(|g| g.up) {
            let [m, dm_dt, _] = g.evaporate(s, t0, dt);
            (gain, slope) = (gain - g.lat(m) / adt, slope - g.lat(dm_dt) / adt);
        }
        let surface = Surface { sw, air: &rad, t0, gain, slope: slope.min(0.0) };
        surface.balance(g, dt, w.cfg.sensible)
    };
    // Newton's steps, kept between the temperatures the end has been shown to lie between: the net heat a node gets
    // falls as its end temperature rises, so a step down from `from` says the end lies below it. It may move with the
    // end temperature where a gas may evaporate (none may at the start and much at the end).
    let moves = gas.iter().any(|g| g.up);
    let solve = |sw: f64| {
        let (mut bal, mut from, (mut lo, mut hi)) = (balance(g.tg, sw), g.tg, (f64::MIN, f64::MAX));
        for _ in 1..SURFACE_ROUNDS {
            if !moves || (bal.ts - from).abs() < SURFACE_STILL_K {
                break;
            }
            (lo, hi) = if bal.ts < from { (lo, from) } else { (from, hi) };
            from = if bal.ts > lo && bal.ts < hi { bal.ts } else { 0.5 * (lo + hi) };
            bal = balance(from, sw);
        }
        bal
    };
    let (nx, stride) = (act.len(), 2 + act.len());
    // The parts (`TO_AIR`) where it absorbs the sunlight `sw`, what evaporates at the end temperature, with their
    // slopes: the air's temperature moves the air touching the cell and each gas's partial pressure, a gas's amount
    // its own.
    let parts = |sw: f64, p: &mut Vec<f64>, ev: &mut Vec<[f64; 3]>| {
        let bal = solve(sw);
        ev.clear();
        ev.extend(gas.iter().map(|g| g.evaporate(s, bal.ts, dt)));
        p.clear();
        p.resize((GAS + nx) * stride, 0.0);
        (p[TO_AIR * stride], p[TO_SPACE * stride]) = (bal.to_air, bal.to_space);
        ev.iter().enumerate().for_each(|(y, e)| p[(GAS + y) * stride] = e[0]);
        for v in 0..1 + nx {
            // How the air's change `v` moves each gas's partial pressure, Pa per K or per mol.
            let dpa = |y: usize| if v == 0 { gas[y].pa / tl } else if v == 1 + y { gas[y].per } else { 0.0 };
            let dgain = -gas.iter().zip(ev.iter()).enumerate().map(|(y, (g, e))| g.lat(e[2] * dpa(y))).sum::<f64>() / adt;
            let [dts, d_air, d_space] = rad.moves(g, dt, w.cfg.sensible, &bal, (if v == 0 { 1.0 } else { 0.0 }, dgain));
            (p[TO_AIR * stride + 1 + v], p[TO_SPACE * stride + 1 + v]) = (d_air, d_space);
            ev.iter().enumerate().for_each(|(y, e)| p[(GAS + y) * stride + 1 + v] = e[1] * dts + e[2] * dpa(y));
        }
        bal
    };
    carry.clear();
    carry.extend(act.iter().map(|ac| carried(cx, g, ac.k)));
    let (mut alb, mut lo, mut hi) = (g.alb, f64::NAN, f64::NAN);
    let mut light = lit(alb);
    let mut bal = parts(light[1], p, ev);
    if g.cover[2] > 0.0 {
        out.resize(Response::width(nx) - FIXED, 0.0);
        put(p, carry, out);
    }
    for _ in 1..if g.cover[2] > 0.0 { REFLECT_ROUNDS } else { 1 } {
        let end = end_reflectance(cx, g, (adt * light[1], out));
        if (end - alb).abs() < REFLECT_STILL {
            break;
        }
        (lo, hi) = if end > alb { (alb, hi) } else { (lo, alb) };
        alb = if lo.is_nan() || hi.is_nan() { end } else { 0.5 * (lo + hi) };
        light = lit(alb);
        bal = parts(light[1], p, ev);
        put(p, carry, out);
    }
    let [sun, sw, kept, in_air] = light;
    hold(cx, g, (air, adt * sw), carry, (p, loss));
    // Each bucket the step's rain wets as it falls (`Vapour::soak`), and per mol the rain adds to what it evaporates its
    // outputs: its latent heat moves the surface's balance (`Air::moves`) and so what the others evaporate.
    soak.clear();
    soak.resize(nx * cols(nx), 0.0);
    for (y, block) in soak.chunks_exact_mut(cols(nx)).enumerate().filter(|e| act[e.0].cond) {
        let Some([share, held, e]) = gas[y].soak(s, bal.ts, dt) else { continue };
        let [dts, d_air, d_space] = rad.moves(g, dt, w.cfg.sensible, &bal, (0.0, -gas[y].lat(1.0) / adt));
        block[..SOAK].copy_from_slice(&[g.share, share, held, gas[y].wet(), e]);
        let part = |r: usize| match r {
            TO_AIR => d_air,
            TO_SPACE => d_space,
            _ => ev[r - GAS][1] * dts + if r - GAS == y { 1.0 } else { 0.0 },
        };
        column(part, carry, |o, v| block[SOAK + o] = v);
    }
    let per_area = if cover > 0.0 { cover / a } else { 1.0 };
    [adt * sw, sun * per_area * dt, kept * adt, adt * top * in_air]
}


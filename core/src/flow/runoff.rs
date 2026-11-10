//! The liquid that may run off goes down the terrain's network to the bodies of standing liquid in one pass a month
//! (laws.md 1.2): each cell in no body sends it down the samples' ways from its samples to the bodies they reach
//! (`bodies::Bodies::ways`);
//! the bodies, each before the body its pass leads to, stand at one level, and what one holds over its pass runs on to
//! the next (`basin::share_out`); what each sample passes on is summed down the samples' ways (`discharge`), and the
//! head it loses there does work on the bed (`erode`). Every move goes from the cell it leaves to the cell it lands
//! in, with its heat, and its potential energy is booked from where it leaves (`potential::apply`).

use super::basin::{movable, share_out};
use super::bodies::Bodies;
use super::curve::{level_over, Curve};
use super::{bare_share, drain::liquid_heat, follow, land::{lift, solid}, liquid::book, potential::apply, year_weight, Scratch};
use crate::{math::one_minus_exp, space::network::NONE, store::{Class, LinkFlux, Move, Node}, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;

/// The moves of the pass, each from one cell to another: what crosses, a row of `kinds` (mol) then the heat (J).
struct Crossings {
    a: Vec<u32>,
    b: Vec<u32>,
    rows: Vec<f64>,
    width: usize,
}

impl Crossings {
    fn push(&mut self, from: u32, to: u32, row: impl Iterator<Item = (usize, f64)>, heat: f64) {
        let at = self.rows.len();
        self.rows.resize(at + self.width, 0.0);
        row.for_each(|(c, m)| self.rows[at + c] += m);
        self.rows[at + self.width - 1] = heat;
        self.a.push(from);
        self.b.push(to);
    }
    /// As the columns of a link flux: the mol of each kind over the links (`[c * links + l]`), then the heat.
    fn columns(&self) -> Vec<f64> {
        let links = self.a.len();
        let mut out = vec![0.0; self.width * links];
        for (l, row) in self.rows.chunks_exact(self.width).enumerate() {
            row.iter().enumerate().for_each(|(c, &x)| out[c * links + l] = x);
        }
        out
    }
}

/// What each cell holds that may move as the pass goes (`basin::movable`: rows of the liquids then the heat; and the
/// volume, m³), and what the pass sends into the samples' ways: mol and kg at each sample.
struct Held {
    q: Vec<f64>,
    vol: Vec<f64>,
    width: usize,
    into: Vec<[f64; 2]>,
}

/// One pass a month of `dt` seconds (the module's doc), of the liquids `liq` (what boils below the coldest node, `tmin`,
/// is a dissolved gas); `slope` and `runs` as the drain to the channels inside each cell found them (`drain::springs`).
pub(super) fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let n = w.mesh.cells().n();
    let all: Vec<u32> = (0..n as u32).collect();
    let (q, vol) = movable(w, s, liq, &all, tmin, (|k, i| super::land::wet(s, k, i), |i, mol| liquid_heat(w, s, liq, i, mol)));
    let b = std::mem::take(&mut s.bodies);
    let sol = s.solids(tmin);
    let kinds: Vec<usize> = liq.iter().chain(sol.iter().filter(|k| !liq.contains(k))).copied().collect();
    let mut x = Crossings { a: Vec::new(), b: Vec::new(), rows: Vec::new(), width: kinds.len() + 1 };
    let mut h = Held { q, vol, width: liq.len() + 1, into: vec![[0.0; 2]; b.cell_of.len()] };
    let mm: Vec<f64> = liq.iter().map(|&k| s.sp[k].mm).collect();
    send(w, s, &b, &mut h, &mut x, (slope, runs, &mm), dt);
    s.basin.level = stand(w, s, &b, &mut h, &mut x, &mm);
    let flux = discharge(w, &b, &h.into);
    let broke = erode(w, s, &b, &flux, (&sol, &kinds), &mut x);
    let col = x.columns();
    let at = (x.width - 1) * x.a.len();
    let ks: Vec<u16> = kinds.iter().map(|&k| k as u16).collect();
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &x.a, b: &x.b, ks: &ks, mol: &col[..at], heat: &col[at..] }]);
    apply(w, s, &[], &broke);
    follow_flux(w, &b, &flux, dt);
    s.bodies = b;
}

/// Each cell in no body sends the liquid that may run off (what stands over its pores, what came up from the store,
/// and the pores' drainage to the channels, a share 1 - exp(-`seep` × slope × time / (`porosity` × `hill`))) down the
/// samples' ways from each of its samples, by its share of the cell's area, to the cells the ways land in, with that
/// share of its heat (`Bodies::ways`; what reaches its own ground stays). All leave as the pass starts.
fn send(w: &World, s: &Scratch, b: &Bodies, h: &mut Held, x: &mut Crossings, (slope, runs, mm): (&[f64], &[f64], &[f64]), dt: f64) {
    let (area, ph, width) = (w.mesh.cells().area(), w.cfg.phys, h.width);
    let mut off = vec![0.0; area.len()];
    for i in (0..area.len()).filter(|&i| b.of[i] == NONE && h.vol[i] > 0.0 && !b.ways[i].is_empty()) {
        let (pores, own) = (s.pd[i] * area[i], h.vol[i] - runs[i]);
        let lp = own.min(pores);
        let free = if s.head[i] > s.z[i] { lp - ph.field * pores } else { 0.0 };
        let hill = if free > 0.0 && slope[i] > 0.0 { free.min(lp * one_minus_exp(ph.seep * slope[i] * dt / (ph.porosity * ph.hill))) } else { 0.0 };
        off[i] = (((own - pores).max(0.0) + runs[i] + hill) / h.vol[i]).min(1.0);
    }
    // What each cell sends, mol and kg, as the pass starts.
    let sent: Vec<[f64; 2]> = (0..area.len()).map(|i| {
        let row = &h.q[i * width..(i + 1) * width - 1];
        [off[i] * row.iter().sum::<f64>(), off[i] * row.iter().zip(mm).map(|(m, k)| m * k).sum::<f64>()]
    }).collect();
    for (k, into) in h.into.iter_mut().enumerate() {
        let i = b.cell_of[k] as usize;
        if off[i] > 0.0 && b.lands[k] as usize != i {
            (into[0], into[1]) = (into[0] + sent[i][0] * b.share[k], into[1] + sent[i][1] * b.share[k]);
        }
    }
    for i in (0..area.len()).filter(|&i| off[i] > 0.0) {
        let row: Vec<f64> = h.q[i * width..(i + 1) * width].iter().map(|m| m * off[i]).collect();
        for &(to, part) in &b.ways[i] {
            let to = to as usize;
            x.push(i as u32, to as u32, row[..width - 1].iter().map(|m| m * part).enumerate(), row[width - 1] * part);
            (0..width).for_each(|r| (h.q[i * width + r], h.q[to * width + r]) = (h.q[i * width + r] - row[r] * part, h.q[to * width + r] + row[r] * part));
            let v = h.vol[i] * off[i] * part;
            (h.vol[i], h.vol[to]) = (h.vol[i] - v, h.vol[to] + v);
        }
    }
}

/// The bodies in order (`Bodies::list`), each brought to one level: its cells' pores keep what they hold and the rest
/// stands at one level over their own ground (`curve::level_over`), no higher than its pass; each cell that holds more
/// gives the share of its liquid over it, mixed, to those that hold less (`share_out`), and what is over the pass
/// leaves for the cell its pass's way lands in, mixed, and into the samples' ways at the pass. Returns each body's level, m.
fn stand(w: &World, s: &Scratch, b: &Bodies, h: &mut Held, x: &mut Crossings, mm: &[f64]) -> Vec<f64> {
    let (area, width) = (w.mesh.cells().area(), h.width);
    let mut levels = Vec::with_capacity(b.list.len());
    for body in &b.list {
        let cells = &body.cells;
        let held: Vec<f64> = cells.iter().flat_map(|&i| h.q[i as usize * width..(i as usize + 1) * width].iter().copied()).collect();
        let vol: Vec<f64> = cells.iter().map(|&i| h.vol[i as usize]).collect();
        let pore: Vec<f64> = cells.iter().zip(&vol).map(|(&i, &v)| v.min(s.pd[i as usize] * area[i as usize])).collect();
        let parts: Vec<(&Curve, f64)> = cells.iter().map(|&i| (&s.curves[i as usize], s.z[i as usize])).collect();
        let top = level_over(&parts, vol.iter().zip(&pore).map(|(v, p)| v - p).sum());
        let (over, top) = (top > body.level, top.min(body.level));
        levels.push(top);
        let want: Vec<f64> = parts.iter().zip(&pore).map(|((c, z), p)| p + c.volume(top - z)).collect();
        let (hub, change, left) = share_out(cells, (&held, &vol), &want, (width, over));
        for (c, &i) in cells.iter().enumerate() {
            let i = i as usize;
            (0..width).for_each(|r| h.q[i * width + r] += change[c * width + r]);
            h.vol[i] = want[c];
            if c != hub {
                x.push(cells[hub], i as u32, change[c * width..(c + 1) * width - 1].iter().copied().enumerate(), change[(c + 1) * width - 1]);
            }
        }
        if over && left.iter().any(|&m| m > 0.0) {
            // (The hub's change, which `share_out` gives, holds what leaves over the pass.)
            let to = body.next.cell as usize;
            x.push(cells[hub], to as u32, left[..width - 1].iter().copied().enumerate(), left[width - 1]);
            (0..width).for_each(|r| h.q[to * width + r] += left[r]);
            h.vol[to] += vol.iter().sum::<f64>() - want.iter().sum::<f64>();
            let into = &mut h.into[body.pass as usize];
            (into[0], into[1]) = (into[0] + left[..width - 1].iter().sum::<f64>(), into[1] + left[..width - 1].iter().zip(mm).map(|(m, k)| m * k).sum::<f64>());
        }
    }
    levels
}

/// What runs through each sample in the pass, mol and kg: what the cells and the bodies sent into the samples' ways
/// (`into`), summed down them in one pass from the last sample the flood reached to the first, so each sample is summed
/// before the one its liquid runs to; a sample of a body takes what reaches it.
fn discharge(w: &World, b: &Bodies, into: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let s = w.terrain_samples();
    let mut flux = into.to_vec();
    for &k in s.order.iter().rev() {
        let (k, j) = (k as usize, s.to[k as usize]);
        if b.at[k] == NONE && j != NONE {
            let f = flux[k];
            let to = &mut flux[j as usize];
            (to[0], to[1]) = (to[0] + f[0], to[1] + f[1]);
        }
    }
    flux
}

/// The head the liquid loses down the samples' ways in each land cell in no body does work there (laws.md 1.3): g times
/// what runs through each of its samples, kg, times the fall to the sample it runs to. It picks up loose solids up to
/// `carry` m³ per J (at most all that lies loose), which go with the cell's runoff to where its ways land, with their
/// heat (what would land on its own ground stays);
/// the work its load leaves free (the cover effect, Sklar and Dietrich 2004) breaks bonded ground into loose (`wear`).
/// Returns the moves of the bed into loose ground.
fn erode(w: &World, s: &mut Scratch, b: &Bodies, flux: &[[f64; 2]], (sol, kinds): (&[usize], &[usize]), x: &mut Crossings) -> Vec<Move> {
    let samples = w.terrain_samples();
    let n = w.mesh.cells().n();
    let mut work = vec![0.0; n];
    for (k, f) in flux.iter().enumerate().filter(|p| p.1[1] > 0.0) {
        let (i, j) = (b.cell_of[k] as usize, samples.to[k]);
        if b.at[k] == NONE && j != NONE && b.of[i] == NONE && s.z[i] > 0.0 {
            work[i] += w.cfg.gravity * f[1] * (samples.h[k] - samples.h[j as usize]).max(0.0);
        }
    }
    let mut broke = Vec::new();
    for i in (0..n).filter(|&i| work[i] > 0.0) {
        let cap = w.cfg.carry * work[i] * bare_share(w, i);
        let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let f = if avail > 0.0 { (cap / avail).min(1.0) } else { 0.0 };
        if f > 0.0 {
            for &(to, part) in &b.ways[i] {
                let row = sol.iter().map(|&k| (kinds.iter().position(|&c| c == k).expect("a kind"), solid(w, s, k, i) * f * part));
                let heat = sol.iter().map(|&k| solid(w, s, k, i) * f * part * s.sp[k].cp * s.state.tg[i]).sum();
                x.push(i as u32, to, row, heat);
            }
        }
        let worn = work[i] * (1.0 - f * avail / cap).max(0.0);
        if worn > 0.0 {
            wear(w, s, i, worn, &mut broke);
        }
    }
    broke
}

/// The moving averages over about a year (`World::discharge`, `World::liquid_flux`, `World::off_land`), from what ran
/// through each sample in the pass of `dt` seconds: a cell's flux is the most through one of its samples, its main
/// channel; what leaves a land cell's samples for those of the open low ground runs off the land.
fn follow_flux(w: &mut World, b: &Bodies, flux: &[[f64; 2]], dt: f64) {
    let (ema, samples) = (year_weight(w, dt), w.terrain_samples());
    let n = w.mesh.cells().n();
    let (mut most, mut off) = (vec![0.0f64; n], vec![0.0f64; n]);
    for (k, f) in flux.iter().enumerate() {
        let i = b.cell_of[k] as usize;
        follow(&mut w.discharge[k], (f[0] / dt) as f32, ema);
        if b.at[k] == NONE {
            most[i] = most[i].max(f[0]);
            let j = samples.to[k];
            let low = j != NONE && b.at[j as usize] != NONE && b.list[b.at[j as usize] as usize].pass == NONE;
            if low && w.is_land(i) {
                off[i] += f[0];
            }
        }
    }
    for i in 0..n {
        follow(&mut w.liquid_flux[i], (most[i] / dt) as f32, ema);
        follow(&mut w.off_land[i], (off[i] / dt) as f32, ema);
    }
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

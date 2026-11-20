//! The liquid that may run off goes down the terrain's network to the bodies of standing liquid in one pass a month
//! (laws.md 1.2): each cell sends what stands over its pores and over what its parts of the bodies hold down the
//! samples' ways from its samples to the bodies they reach (`bodies::Bodies::ways`);
//! the bodies, each before the body its pass leads to, stand at one level, and what one holds over its pass runs on to
//! the next (`basin::share_out`); what each sample passes on is summed down the samples' ways (`discharge`), and the
//! head it loses there does work on the bed (`erode`). Every move goes from the cell it leaves to the cell it lands
//! in, with its heat, and its potential energy is booked from where it leaves (`potential::apply`).

use super::basin::{movable, share_out};
use super::bodies::Bodies;

mod edge;
use edge::come_in;
use super::{curve::Part, level::{level_with, Level}};
use super::{bare_share, drain::{liquid_heat, to_channels}, follow, land::{lift, solid, standing}, liquid::book, parts::{pores_first, under_liquid, Leaves}, potential::apply, year_weight, Scratch};
use crate::{space::{network::NONE, wear::STAND_M}, units::G_PER_KG, store::{Boundary, Class, LinkFlux, Move, Node}, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;

/// The moves of the pass, each from a node of the class `class[0]` to one of `class[1]` (a cell to another, or the
/// boundary for outside to a cell): what crosses, a row of `kinds` (mol) then the heat (J).
struct Crossings {
    a: Vec<u32>,
    b: Vec<u32>,
    rows: Vec<f64>,
    width: usize,
    class: [Class; 2],
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
    /// Its moves as columns: the mol of each kind over the links (`[c * links + l]`), then the heat.
    fn columns(&self) -> Vec<f64> {
        let links = self.a.len();
        let mut col = vec![0.0; self.width * links];
        for (l, row) in self.rows.chunks_exact(self.width).enumerate() {
            row.iter().enumerate().for_each(|(c, &x)| col[c * links + l] = x);
        }
        col
    }
}

/// The moves `moves` of the kinds `kinds` booked through the store together, as one set of link fluxes
/// (`liquid::book`): a cell may give in one what it gets in another (a cell at the open edge gives over its level what
/// came in from the boundary), so each cell's sum is booked at once, never a part of it that would ask it for more than
/// it holds.
fn book_all(w: &mut World, s: &mut Scratch, kinds: &[usize], moves: &[&Crossings]) {
    let ks: Vec<u16> = kinds.iter().map(|&k| k as u16).collect();
    let cols: Vec<Vec<f64>> = moves.iter().map(|x| x.columns()).collect();
    let fluxes: Vec<LinkFlux> = moves.iter().zip(&cols).filter(|(x, _)| !x.a.is_empty()).map(|(x, col)| {
        let at = (x.width - 1) * x.a.len();
        LinkFlux { class: x.class, a: &x.a, b: &x.b, ks: &ks, mol: &col[..at], heat: &col[at..] }
    }).collect();
    book(w, s, &fluxes, Leaves::Liquid);
}

/// What each cell holds that may move as the pass goes (`basin::movable`: rows of the liquids then the heat; and the
/// volume, m³), what each cell's part of each body holds of it (`State::held`), m³, and what the pass sends into the samples'
/// ways: mol and kg at each sample.
struct Held {
    q: Vec<f64>,
    vol: Vec<f64>,
    width: usize,
    part: Vec<Vec<f64>>,
    into: Vec<[f64; 2]>,
}

/// One pass a month of `dt` seconds (the module's doc), of the liquids `liq` (what boils below the coldest node, `tmin`,
/// is a dissolved gas); `slope` and `runs` as the drain to the channels inside each cell found them (`drain::springs`).
pub(super) fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let n = w.mesh.cells().n();
    let all: Vec<u32> = (0..n as u32).collect();
    let (q, vol) = movable(w, s, liq, &all, tmin, (|k, i| super::land::wet(s, k, i), |i, mol| liquid_heat(w, s, liq, i, mol)));
    let b = std::mem::take(&mut s.bodies);
    let mut x = Crossings { a: Vec::new(), b: Vec::new(), rows: Vec::new(), width: liq.len() + 1, class: [Class::Ground; 2] };
    let mut edge = Crossings { a: Vec::new(), b: Vec::new(), rows: Vec::new(), width: liq.len() + 1, class: [Class::Boundary, Class::Ground] };
    // (The room for what runs through each sample is kept from month to month, `spare::Spare`.)
    let mut into = std::mem::take(&mut s.spare.samples);
    into.clear();
    into.resize(b.cell_of.len(), [0.0; 2]);
    let mut h = Held { q, vol, width: liq.len() + 1, part: s.state.held.iter().map(|v| vec![0.0; v.len()]).collect(), into };
    let mm: Vec<f64> = liq.iter().map(|&k| s.sp[k].mm).collect();
    send(w, s, &b, &mut h, &mut x, (slope, runs, &mm), dt);
    let (levels, came) = stand(s, &b, &mut h, (&mut x, &mut edge), (&mm, w.edges_open()));
    s.basin.level = levels;
    s.state.held = std::mem::take(&mut h.part);
    discharge(w, &b, &mut h.into);
    let flux = h.into;
    book_all(w, s, liq, &[&x, &edge]);
    if !edge.a.is_empty() {
        let heat: f64 = edge.rows.chunks_exact(edge.width).map(|r| r[edge.width - 1]).sum();
        (w.counts.edge_liquid[0], w.counts.edge_liquid[1]) = (w.counts.edge_liquid[0] + came, w.counts.edge_liquid[1] + heat);
    }
    // What the liquid carries is booked apart from the liquid, so the heat of each lands with its own matter in a cell
    // of parts (`parts::to_liquid`).
    let sol = s.solids(tmin);
    let mut load = Crossings { a: Vec::new(), b: Vec::new(), rows: Vec::new(), width: sol.len() + 1, class: [Class::Ground; 2] };
    let (broke, loads) = erode(w, s, &b, &flux, &sol, &mut load);
    book_all(w, s, &sol, &[&load]);
    (w.counts.load_month, w.counts.plumes) = (loads, Vec::new());
    apply(w, s, &[], &broke);
    follow_flux(w, &b, &flux, dt);
    (s.bodies, s.spare.samples) = (b, flux);
}

/// What runs off each cell, as a share of the liquid that may move in it: what came up from the store; the pores'
/// drainage to the channels, a share 1 - exp(-conductivity × slope × time / (porosity × `hill`), `soil::Pores`), which what stands over
/// them refills first (pores first; `drain::to_channels`; of a cell of parts its land's drain to its liquid,
/// `drain::land_drains`, and under a body its pores lie full); and what stands over its pores over what its parts of the bodies held at the last
/// pass (`State::held`; they keep the rest, each by what it held, in `Held::part`).
fn runs_off(w: &World, s: &Scratch, h: &mut Held, (slope, runs): (&[f64], &[f64]), dt: f64) -> Vec<f64> {
    let (area, ph) = (w.mesh.cells().area(), w.cfg.phys);
    let mut off = vec![0.0; area.len()];
    for i in (0..area.len()).filter(|&i| h.vol[i] > 0.0) {
        let (pores, own) = (s.pd[i] * area[i], h.vol[i] - runs[i]);
        let (lp, stands) = (own.min(pores), (own - pores).max(0.0));
        let soil = w.pores(i);
        let free = if s.head[i] > s.z[i] { lp - soil.field * pores } else { 0.0 };
        let hill = if s.basin.is[i] || super::parts::has(s, i) { 0.0 } else { to_channels(&ph, soil, slope[i], dt, (free, lp)) };
        let held: f64 = s.state.held[i].iter().sum();
        let kept = held.min((stands - hill).max(0.0));
        h.part[i].iter_mut().zip(&s.state.held[i]).for_each(|(p, &was)| *p = if held > 0.0 { was * kept / held } else { 0.0 });
        off[i] = (((stands - kept).max(hill) + runs[i]) / h.vol[i]).min(1.0);
    }
    off
}

/// Each cell sends what runs off it (`runs_off`) down the samples' ways from each of its samples, by its share of the
/// cell's area, to the cells and the bodies' parts the ways reach, with that share of its heat (`Bodies::ways`; what
/// reaches its own ground stays, its own part's where it reaches one). All leave as the pass starts.
fn send(w: &World, s: &Scratch, b: &Bodies, h: &mut Held, x: &mut Crossings, (slope, runs, mm): (&[f64], &[f64], &[f64]), dt: f64) {
    let (n, width) = (w.mesh.cells().n(), h.width);
    let off = runs_off(w, s, h, (slope, runs), dt);
    // What each cell sends, mol and kg, as the pass starts, into the ways of its samples in no pond.
    let sent: Vec<[f64; 2]> = (0..n).map(|i| {
        let row = &h.q[i * width..(i + 1) * width - 1];
        [off[i] * row.iter().sum::<f64>(), off[i] * row.iter().zip(mm).map(|(m, k)| m * k).sum::<f64>()]
    }).collect();
    for &k in &b.land {
        let (k, i) = (k as usize, b.cell_of[k as usize] as usize);
        if off[i] > 0.0 {
            let into = &mut h.into[k];
            (into[0], into[1]) = (into[0] + sent[i][0] * b.share[k], into[1] + sent[i][1] * b.share[k]);
        }
    }
    // What each cell sends, its row and volume, as the pass starts: what it then gets does not leave with it.
    let leaving: Vec<(usize, Vec<f64>, f64)> = (0..n).filter(|&i| off[i] > 0.0).map(|i| (i, h.q[i * width..(i + 1) * width].iter().map(|m| m * off[i]).collect(), h.vol[i] * off[i])).collect();
    for (i, row, v) in leaving {
        for &(r, part) in &b.ways[i] {
            let to = r.cell as usize;
            if r.part != NONE {
                h.part[to][r.part as usize] += v * part;
            }
            if to == i {
                continue;
            }
            x.push(i as u32, to as u32, row[..width - 1].iter().map(|m| m * part).enumerate(), row[width - 1] * part);
            (0..width).for_each(|r| (h.q[i * width + r], h.q[to * width + r]) = (h.q[i * width + r] - row[r] * part, h.q[to * width + r] + row[r] * part));
            (h.vol[i], h.vol[to]) = (h.vol[i] - v * part, h.vol[to] + v * part);
        }
    }
}

/// A body's part in a cell as its level reads it: the part, its cell's mean surface, m, the area its liquid covers
/// now, m², and what the pores of the ground it comes to cover lack, m, and what the cell's own pores give them first,
/// m³ (`parts::pores_first`).
type Ground<'a> = (&'a Part, f64, f64, [f64; 2]);

/// The area of cell `i`'s part `q` of a body its liquid covers now, m²: of a cell of parts, the share of the ground
/// under its liquid as its last solve left it (`parts::Parts::wet`, which `moves::strip` moves from), by what each of its
/// parts covers at what stands now; of a node that is one, what the part covers at what stands now.
fn covered(s: &Scratch, i: usize, q: usize) -> f64 {
    let c = &s.curves[i];
    let now: Vec<f64> = c.parts.iter().zip(s.fill(i).shares(standing(s, i))).map(|(p, v)| p.under(p.level(v))).collect();
    let all: f64 = now.iter().sum();
    match under_liquid(s, i) {
        Some(wet) if all > 0.0 => wet * c.area() * now[q] / all,
        Some(wet) => wet * c.area() / now.len() as f64,
        None => now[q],
    }
}

/// The level of a body whose parts `parts` hold `vol` m³, m, no higher than `pass`; what the pores of the ground each
/// part newly covers take of it, m³, as they fill first (`parts::pores_first`); and whether it stands over its pass.
/// What its samples hold under a level and what those pores take both grow with the level: the level is where they
/// take `vol` (`level::level_with`); where it stands at a height of samples whose pores take more than is left, what is
/// left goes to those pores, by what each takes there.
fn body_level(parts: &[Ground], vol: f64, pass: f64) -> (f64, Vec<f64>, bool) {
    let over: Vec<(&Part, f64)> = parts.iter().map(|g| (g.0, g.1)).collect();
    let soaks = |q: usize, under: f64| soaks(&parts[q], under);
    let soak = |x: f64| over.iter().enumerate().map(|(q, &(p, z))| soaks(q, p.under(x - z))).collect::<Vec<f64>>();
    match level_with(&over, vol, soaks) {
        Level::At(x) | Level::Step(x, _) if x > pass => (pass, soak(pass), true),
        Level::At(x) => (x, soak(x), false),
        Level::Step(lo, hi) => {
            let (below, above) = (soak(lo), soak(hi));
            let stands: f64 = over.iter().map(|&(p, z)| p.volume(lo - z)).sum();
            let (left, step) = (vol - stands - below.iter().sum::<f64>(), above.iter().zip(&below).map(|(a, b)| a - b).sum::<f64>());
            let taken = below.iter().zip(&above).map(|(b, a)| b + if step > 0.0 { (a - b) * (left / step).clamp(0.0, 1.0) } else { 0.0 }).collect();
            (lo, taken, false)
        }
    }
}

/// What the pores of the ground part `g` newly covers take where its liquid covers `under` m², m³: what they lack over
/// what the cell's own pores give them first (`parts::pores_first`).
fn soaks(g: &Ground, under: f64) -> f64 {
    let (_, _, now, [dry, own]) = *g;
    ((under - now).max(0.0) * dry - own).max(0.0)
}

/// The bodies in order (`Bodies::list`), each brought to one level: what its parts hold (`Held::part`) stands at one
/// level over their samples, no higher than its pass, the pores of the ground it newly covers filling first
/// (`body_level`); each cell whose part holds more than its samples hold under that level gives the share of its liquid
/// over it, mixed, to those that hold less (`share_out`), and what is over the pass leaves for the cell and the part
/// its pass's way reaches, mixed, and into the samples' ways at the pass. Each part then holds what its samples hold
/// under its body's level. Where the map's edges are open (`open`), the part of the open low ground at the edge stands
/// at the planet's level: what it lacks comes in (`come_in`) and what stands over it leaves for the boundary for
/// outside, as over a pass (`edge`). Returns each body's level, m, and the volume that came in at the edges, m³.
fn stand(s: &Scratch, b: &Bodies, h: &mut Held, (x, edge): (&mut Crossings, &mut Crossings), (mm, open): (&[f64], bool)) -> (Vec<f64>, f64) {
    let width = h.width;
    let (mut levels, mut came) = (Vec::with_capacity(b.list.len()), 0.0);
    for body in &b.list {
        let cells = &body.cells;
        let ground = |(&i, &q): (&u32, &usize)| {
            let (i, p) = (i as usize, &s.curves[i as usize].parts[q]);
            (p, s.z[i], covered(s, i, q), pores_first(s, i))
        };
        let parts: Vec<Ground> = cells.iter().zip(&body.part).map(ground).collect();
        let held_open = open && body.pass == NONE && !body.edge.is_empty();
        if held_open {
            came += come_in(body, &parts, h, edge);
        }
        let held: Vec<f64> = cells.iter().flat_map(|&i| h.q[i as usize * width..(i as usize + 1) * width].iter().copied()).collect();
        let vol: Vec<f64> = cells.iter().map(|&i| h.vol[i as usize]).collect();
        let pass = if held_open { STAND_M } else { body.level };
        let (top, soak, over) = body_level(&parts, cells.iter().zip(&body.part).map(|(&i, &q)| h.part[i as usize][q]).sum(), pass);
        levels.push(top);
        let own: Vec<f64> = parts.iter().map(|&(p, z, ..)| p.volume(top - z)).collect();
        // Each cell wants what it holds apart from its part, what its part holds under the level, and what the pores of
        // the ground it newly covers take. What it holds apart from its part is at least what its other parts hold: where
        // this part holds nearly all its liquid, the difference would round away what another holds, and the cell would
        // be left holding less than its parts.
        let apart = |c: usize| {
            let (i, q) = (cells[c] as usize, body.part[c]);
            let others: f64 = h.part[i].iter().enumerate().filter(|&(r, _)| r != q).map(|(_, v)| v).sum();
            (vol[c] - h.part[i][q]).max(others)
        };
        let want: Vec<f64> = (0..cells.len()).map(|c| apart(c) + own[c] + soak[c]).collect();
        let (hub, change, left) = share_out(cells, (&held, &vol), &want, (width, over));
        for (c, &i) in cells.iter().enumerate() {
            let i = i as usize;
            (0..width).for_each(|r| h.q[i * width + r] += change[c * width + r]);
            (h.vol[i], h.part[i][body.part[c]]) = (want[c], own[c]);
            if c != hub {
                x.push(cells[hub], i as u32, change[c * width..(c + 1) * width - 1].iter().copied().enumerate(), change[(c + 1) * width - 1]);
            }
        }
        if held_open && over && left.iter().any(|&m| m > 0.0) {
            // (The hub's change, which `share_out` gives, holds what leaves over the planet's level.)
            edge.push(Boundary::Outside as u32, cells[hub], left[..width - 1].iter().map(|m| -m).enumerate(), -left[width - 1]);
            came -= vol.iter().sum::<f64>() - want.iter().sum::<f64>();
        } else if over && left.iter().any(|&m| m > 0.0) {
            // (The hub's change, which `share_out` gives, holds what leaves over the pass.)
            let (to, gone) = (body.next.cell as usize, vol.iter().sum::<f64>() - want.iter().sum::<f64>());
            x.push(cells[hub], to as u32, left[..width - 1].iter().copied().enumerate(), left[width - 1]);
            (0..width).for_each(|r| h.q[to * width + r] += left[r]);
            h.vol[to] += gone;
            if body.next.part != NONE {
                h.part[to][body.next.part as usize] += gone;
            }
            let into = &mut h.into[body.pass as usize];
            (into[0], into[1]) = (into[0] + left[..width - 1].iter().sum::<f64>(), into[1] + left[..width - 1].iter().zip(mm).map(|(m, k)| m * k).sum::<f64>());
        }
    }
    (levels, came)
}

/// What runs through each sample in the pass, mol and kg: what the cells and the bodies sent into the samples' ways
/// (`flux`), summed down them in one pass over the samples in no pond from the last the flood reached to the first
/// (`Bodies::down`), so each sample is summed before the one its liquid runs to; a sample of a body takes what reaches
/// it. A sample nothing runs through adds nothing.
fn discharge(w: &World, b: &Bodies, flux: &mut [[f64; 2]]) {
    let s = w.terrain_samples();
    for &k in &b.down {
        let (f, j) = (flux[k as usize], s.to[k as usize] as usize);
        if f != [0.0; 2] {
            (flux[j][0], flux[j][1]) = (flux[j][0] + f[0], flux[j][1] + f[1]);
        }
    }
}

/// The head the liquid loses down the samples' ways in each land cell does work there (laws.md 1.3): g times what runs
/// through each of its samples in no pond, kg, times the fall to the sample it runs to. It picks up loose solids up to
/// `carry` m³ per J times its loose ground's erodibility (`Texture::erodibility`; at most all that lies loose), which go with the cell's runoff to where its ways land, with their
/// heat (what would land on its own ground stays);
/// the work its load leaves free (the cover effect, Sklar and Dietrich 2004) breaks bonded ground into loose (`wear`).
/// Returns the moves of the bed into loose ground, and of each cell the load in its main channel, g per m³ of its liquid,
/// and the depth that settled on it, m (`Counters::load_month`: none settles on the way yet).
fn erode(w: &World, s: &mut Scratch, b: &Bodies, flux: &[[f64; 2]], sol: &[usize], x: &mut Crossings) -> (Vec<Move>, Vec<[f32; 2]>) {
    let samples = w.terrain_samples();
    let n = w.mesh.cells().n();
    let mut work = vec![0.0; n];
    for &k in b.land.iter().filter(|&&k| flux[k as usize][1] > 0.0) {
        let (k, f) = (k as usize, flux[k as usize]);
        let (i, j) = (b.cell_of[k] as usize, samples.to[k]);
        if j != NONE && s.z[i] > 0.0 {
            work[i] += w.cfg.gravity * f[1] * (samples.h[k] - samples.h[j as usize]).max(0.0);
        }
    }
    let mut broke = Vec::new();
    // What the main channel of each cell carried in the month, mol of liquid (the most through one of its samples).
    let mut main = vec![0.0f64; n];
    b.land.iter().for_each(|&k| main[b.cell_of[k as usize] as usize] = main[b.cell_of[k as usize] as usize].max(flux[k as usize][0]));
    let mut loads = vec![[0.0f32; 2]; n];
    for i in (0..n).filter(|&i| work[i] > 0.0) {
        let cap = w.cfg.carry * work[i] * bare_share(w, i) * w.soil[i].texture.erodibility();
        let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
        let f = if avail > 0.0 { (cap / avail).min(1.0) } else { 0.0 };
        let per_m3 = w.liquid_in(i, |k| w.g(k, i)).map_or(0.0, |l| l.1);
        if per_m3 > 0.0 && main[i] > 0.0 {
            let kg: f64 = sol.iter().map(|&k| solid(w, s, k, i) * f * s.sp[k].mm).sum();
            loads[i][0] = (G_PER_KG * kg / (main[i] / per_m3)) as f32;
        }
        if f > 0.0 {
            for &(r, part) in b.ways[i].iter().filter(|r| r.0.cell as usize != i) {
                let to = r.cell;
                let row = sol.iter().enumerate().map(|(c, &k)| (c, solid(w, s, k, i) * f * part));
                let heat = sol.iter().map(|&k| solid(w, s, k, i) * f * part * s.sp[k].cp * s.state.tg[i]).sum();
                x.push(i as u32, to, row, heat);
            }
        }
        let worn = work[i] * (1.0 - f * avail / cap).max(0.0);
        if worn > 0.0 {
            wear(w, s, i, worn, &mut broke);
        }
    }
    (broke, loads)
}

/// What ran through each cell's main channel in the month (`World::liquid_month`), and the moving averages over about a
/// year (`World::discharge`, `World::liquid_flux`, `World::off_land`), from what ran
/// through each sample in the pass of `dt` seconds (an average that is none stays none where none ran): a cell's flux
/// is the most through one of its samples in no pond, its main channel; what leaves a land cell's samples for those of
/// the open low ground runs off the land.
fn follow_flux(w: &mut World, b: &Bodies, flux: &[[f64; 2]], dt: f64) {
    let (ema, samples) = (year_weight(w, dt), w.terrain_samples());
    let n = w.mesh.cells().n();
    let (mut most, mut off) = (vec![0.0f64; n], vec![0.0f64; n]);
    for (avg, f) in w.discharge.iter_mut().zip(flux).filter(|p| *p.0 != 0.0 || p.1[0] != 0.0) {
        follow(avg, (f[0] / dt) as f32, ema);
    }
    // Whether each cell is land (`World::is_land`), found once for a cell, at its first sample that needs it.
    let mut land: Vec<Option<bool>> = vec![None; n];
    for &k in &b.land {
        let (k, f) = (k as usize, flux[k as usize]);
        let i = b.cell_of[k] as usize;
        most[i] = most[i].max(f[0]);
        let j = samples.to[k];
        let low = j != NONE && b.at[j as usize] != NONE && b.list[b.at[j as usize] as usize].pass == NONE;
        if low && *land[i].get_or_insert_with(|| w.is_land(i)) {
            off[i] += f[0];
        }
    }
    w.liquid_month.resize(n, 0.0);
    for i in 0..n {
        w.liquid_month[i] = (most[i] / dt) as f32;
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

//! A cell's parts (laws.md 1.1): the ground of a cell with liquid standing on it is a short list of parts under one
//! air, each with its own heat: its land, where its own ground lies above the liquid that stands over its pores
//! (`curve::Curve::wet`), the liquid's top layer down to `h_mix` under its level, and its deep below that. The land and
//! the top face the air, each with its own heat, phases, soft cover and reflectance: a mosaic of tiles (Avissar and
//! Pielke 1989; Koster and Suarez 1992); the deep swaps liquid with the top through the thermocline (`deep`), as the mixed
//! layer of a slab model does with the liquid under it (Hansen et al. 1984). The store holds the cell's one ground node;
//! Flow keeps how its matter and heat lie between the parts (`Parts`), finds each one's state from them (`solve`), and
//! from them the node's state that every other law reads (`row`). What other laws move is shared between the parts
//! (`moves::reconcile`); what Flow moves of one part it books there.

mod deep;
mod moves;
mod node;
mod view;

pub(super) use deep::mix;
pub(super) use moves::{after_air, to_liquid};
pub(super) use view::{amounts, pores_first, under_liquid};

use super::land::{column, layers, melts, node_state, take_row, thaw_at, NONE, STATE_ROW};
use node::{conductance, row};
use super::scratch::{Latent, Scratch};
use crate::{ground::{skin_cap, Melting}, store::Class, World};
use serde::{Deserialize, Serialize};

/// The land part, the liquid's top layer and its deep, in `Parts` and `Mixed`, and how many parts there are.
pub(super) const LAND: usize = 0;
pub(super) const TOP: usize = 1;
pub(super) const DEEP: usize = 2;
pub(super) const PARTS: usize = 3;

/// How a cell's matter and heat lie between its parts, saved with the world: the share of the cell's area under its
/// liquid, as last found (the land the rest); each part's heat, J, and mol of each substance (`amt[k]`), [land, top,
/// deep], as Flow last set them; and each part's temperature, K, and the substance on its plateau with its molten
/// share, as the last `solve` found them (what life reads, `World::thaw_last`), and the room the land's pores have
/// left, m³ (the liquid of the cell fills the pores under its liquid and what the land's hold before it stands, `room`,
/// `Scratch::dry`). A part that holds nothing is not there: a cell all under its liquid has no land, a liquid shallower
/// than `h_mix` no deep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Parts {
    pub(super) wet: f64,
    pub(super) heat: [f64; PARTS],
    pub(super) amt: Vec<[f64; PARTS]>,
    pub(super) t: [f64; PARTS],
    pub(super) on: [Option<(usize, f64)>; PARTS],
    pub(super) dry: f64,
}

impl Parts {
    /// Does it have land (its liquid covers less than all of the cell)?
    pub(super) fn land(&self) -> bool {
        self.wet < 1.0
    }
    /// Does its liquid have a deep (that part holds any matter)?
    pub(super) fn deep(&self) -> bool {
        self.amt.iter().any(|a| a[DEEP] > 0.0)
    }
    /// The part that holds the ground under the liquid: its deep where it has one, else its top.
    pub(super) fn bottom(&self) -> usize {
        if self.deep() { DEEP } else { TOP }
    }
    /// The share of the cell's area of part `q`: the liquid's top and deep lie over the same ground.
    pub(super) fn share(&self, q: usize) -> f64 {
        if q == LAND { 1.0 - self.wet } else { self.wet }
    }
    /// The share of the cell's ground (its bed's skin and fixed solids) part `q` holds: the land its area's, the
    /// liquid's bottom the rest.
    pub(super) fn ground(&self, q: usize) -> f64 {
        if q == LAND {
            1.0 - self.wet
        } else if q == self.bottom() {
            self.wet
        } else {
            0.0
        }
    }
}

/// A part as the air's step reads it (`air::surf::Surf::of_part`), as the last `solve` found it: its share of the
/// cell's area and of its ground (`Parts::ground`), its mol of each substance, its heat, J, and heat capacity, J/K; its
/// temperature, K, plateau's substance (or `NONE`) and molten share, and fusion enthalpy held (`Latent`); its soft cover
/// (`Scratch::cover`), reflectance, and pore space as a depth over its area, m; and the layers of the part under it,
/// which its reflectance sees through (the top's deep, `land::layers`).
#[derive(Clone, Debug)]
pub(super) struct Piece {
    pub(super) share: f64,
    pub(super) ground: f64,
    pub(super) amt: Vec<f64>,
    pub(super) heat: f64,
    pub(super) cap: f64,
    pub(super) tg: f64,
    pub(super) pk: usize,
    pub(super) phi: f64,
    pub(super) latent: Latent,
    pub(super) cover: [f64; 3],
    pub(super) alb: f64,
    pub(super) pd: f64,
    pub(super) below: [f64; 6],
}

impl Piece {
    /// The mol of substance `k` (`sp`) it holds liquid, by its temperature and plateau.
    pub(super) fn liquid(&self, k: usize, sp: &super::scratch::Sp) -> f64 {
        let molten = if self.pk == k { self.phi } else { (self.latent.molten >> k & 1) as f64 };
        if self.tg < sp.boil { self.amt[k] * molten } else { 0.0 }
    }
}

/// A cell's parts as the last `solve` found them (`Piece`), and the heat conductance between its land and its liquid's
/// top, W/K (`conductance`); the parts that solve left and the node's row it found (`row`), which a solve of the same
/// parts and node finds again.
#[derive(Clone, Debug)]
pub(super) struct Mixed {
    pub(super) piece: [Piece; PARTS],
    pub(super) link: f64,
    solved: Parts,
    row: [f64; STATE_ROW],
}

impl Mixed {
    /// The temperature of its liquid, K: its parts' by the liquid each holds (`Piece::liquid`), its top's where none
    /// holds any.
    pub(super) fn liquid_t(&self, sp: &[super::scratch::Sp]) -> f64 {
        let held = self.piece.each_ref().map(|p| (0..p.amt.len()).map(|k| p.liquid(k, &sp[k])).sum::<f64>());
        let all = held[LAND] + held[TOP] + held[DEEP];
        if all > 0.0 { (held[LAND] * self.piece[LAND].tg + held[TOP] * self.piece[TOP].tg + held[DEEP] * self.piece[DEEP].tg) / all } else { self.piece[TOP].tg }
    }
    /// Does it face the air with two parts, its land and its liquid's top (else the top alone)?
    pub(super) fn two_faces(&self) -> bool {
        self.piece[LAND].share > 0.0
    }
}

/// Does cell `i` stand as parts?
#[inline]
pub(super) fn has(s: &Scratch, i: usize) -> bool {
    s.state.parts.get(i).is_some_and(Option::is_some)
}

/// The nodes `anew` (each with whether it thaws, `node_state`) found anew, in parallel with `threads`: a node that is
/// one by `land::node_state`, one of parts from its parts, and with `moving` a node that `node_state` finds with liquid
/// over part of its own ground or deeper than `h_mix` made parts first (`create`) and one whose parts no longer stand
/// apart one again (`solve`). Without it (a build of the caches alone: a loaded world's, which an
/// unbroken world does not make) the parts only find their state, so a world goes on as an unbroken one. A node of
/// parts that nothing moved since its last solve (its parts as that solve left them, and the node's amounts and heat
/// theirs, `moves::unmoved`) takes that solve's row again, unless the build is `full`.
pub(super) fn solve_nodes(w: &World, s: &mut Scratch, anew: &[(u32, bool)], (full, moving): (bool, bool)) {

    let mut rows = std::mem::take(&mut s.spare.state);
    rows.resize(anew.len() * STATE_ROW, 0.0);
    let sr = &*s;
    crate::par::rows(&mut rows, STATE_ROW, crate::par::UNCOUNTED, || (), |_, q, row| {
        let (i, thaw) = (anew[q].0 as usize, anew[q].1);
        if !has(sr, i) {
            node_state(w, sr, i, (sr.cap[i], thaw, full), row);
        }
    });
    let same = |i: usize, p: &Parts| !full && sr.mixed(i).is_some_and(|m| m.solved == *p) && moves::unmoved(w, i, p);
    // The nodes to solve, each with its parts where they are new (`create`).
    let mixed: Vec<(usize, Option<Parts>)> = (anew.iter().zip(rows.chunks_exact(STATE_ROW)))
        .filter_map(|(&(i, _), r)| match &sr.state.parts[i as usize] {
            Some(p) if same(i as usize, p) => None,
            Some(_) => Some((i as usize, None)),
            None if moving && r[9] != 0.0 => create(w, sr, i as usize, r).map(|p| (i as usize, Some(p))),
            None => None,
        })
        .collect();
    let solved = crate::par::map(&mixed, |(i, made)| made.as_ref().or(sr.state.parts[*i].as_ref()).and_then(|p| solve(w, sr, *i, p, moving)));
    let mut solving = vec![false; s.state.parts.len()];
    mixed.iter().for_each(|m| solving[m.0] = true);
    for (&(i, _), r) in anew.iter().zip(rows.chunks_exact(STATE_ROW)) {
        match s.mixed(i as usize).map(|m| m.row) {
            _ if !has(s, i as usize) => take_row(s, i as usize, r),
            Some(row) if !solving[i as usize] => take_row(s, i as usize, &row),
            _ => (),
        }
    }
    s.spare.state = rows;
    for ((i, _), solved) in mixed.into_iter().zip(solved) {
        match solved {
            Some((p, m, row)) => {
                take_row(s, i, &row);
                (s.state.parts[i], s.mixed[i]) = (Some(p), Some(Box::new(m)));
            }
            None => {
                (s.state.parts[i], s.mixed[i]) = (None, None);
                let mut row = [0.0; STATE_ROW];
                node_state(w, s, i, (s.cap[i], true, true), &mut row);
                take_row(s, i, &row);
            }
        }
    }
}

/// A part as one solve finds it: its heat capacity, J/K, where it stands on its melting plateaus (`land::thaw_at`), and
/// its layers, solid surface and soft cover (`land::layers`).
struct Found {
    cap: f64,
    th: Melting,
    l: [f64; 6],
    z: f64,
    cover: [f64; 2],
}

impl Found {
    /// Part `q` of `p` in cell `i`, whose bed's skin holds `skin` J/K: its share of that and the heat capacity of its
    /// loose amounts; a part that is not there holds nothing (`none`).
    fn of(w: &World, s: &Scratch, (i, skin): (usize, f64), p: &Parts, q: usize) -> Found {
        let (share, ground) = (p.share(q), p.ground(q));
        if share <= 0.0 || (q == DEEP && !p.deep()) {
            return Found::none(w.base_z);
        }
        let amt = |k: usize| p.amt[k][q];
        let cap = skin * ground + s.state.loose.iter().map(|&k| amt(k) * s.sp[k].cp).sum::<f64>();
        let melt = melts(w, s, amt);
        let th = thaw_at(s, cap.max(1.0), p.heat[q], amt, &melt);
        let (l, z, cover) = layers(w, s, (i, share, ground), th.t, amt, |k| th.molten(k), |k| melt[k]);
        Found { cap, th, l, z, cover }
    }
    /// A part that is not there, its surface at `z`.
    fn none(z: f64) -> Found {
        let th = Melting { t: 0.0, on: None, held: 0.0, lo: f64::MIN, hi: f64::MAX, lat: [0.0; 2], bits: 0 };
        Found { cap: 0.0, th, l: [0.0; 6], z, cover: [0.0, f64::INFINITY] }
    }
    /// What its pores hold when full, m³ (porosity `por`).
    fn pores(&self, por: f64) -> f64 {
        por * self.l[4]
    }
    /// The liquid over its pores, m³ (none: under them).
    fn over(&self, por: f64) -> f64 {
        self.l[2] - self.pores(por)
    }
}

/// The parts of `p` in cell `i` (`Found::of`), the heat capacity of its bed's skin found once (`ground::skin_cap`).
fn found(w: &World, s: &Scratch, i: usize, p: &Parts) -> [Found; PARTS] {
    let n = w.mesh.cells().n();
    let skin = skin_cap(&w.chem, &w.cfg, w.mesh.cells().area()[i], s.state.bedk.iter().copied(), |k| w.store.amounts(Class::Bed)[k * n + i]);
    [LAND, TOP, DEEP].map(|q| Found::of(w, s, (i, skin), p, q))
}

/// What stands over the pores of the ground under the liquid of the parts found as `f` (its top and deep, porosity
/// `por`), liquid and soft solid, m³: what the share of the cell under the liquid is found from.
fn stands(f: &[Found; PARTS], por: f64) -> f64 {
    let (t, d) = (&f[TOP], &f[DEEP]);
    t.l[0] + d.l[0] + liquid(f, por)
}

/// The liquid that stands over those pores, m³.
fn liquid(f: &[Found; PARTS], por: f64) -> f64 {
    let (t, d) = (&f[TOP], &f[DEEP]);
    (t.l[2] + d.l[2] - t.pores(por) - d.pores(por)).max(0.0)
}

/// The parts of cell `i` found anew from `kept` (`Parts`): first what other laws moved since is shared between them
/// (`moves::reconcile`), each is found (`Found`); with `moving`, the liquid over the land's pores runs to the liquid
/// (`moves::settle`), then the share of the cell under the liquid follows what stands on it (`Curve::wet`), with the
/// ground it gains or gives (`moves::strip`), and the deep holds what stands deeper than `h_mix` (`deep::layer`). The
/// parts, as the air's step reads them, and the node's row (`row`). None where nothing stands, or it covers the whole
/// cell no deeper than `h_mix`: the cell is one.
pub(super) fn solve(w: &World, s: &Scratch, i: usize, kept: &Parts, moving: bool) -> Option<(Parts, Mixed, [f64; STATE_ROW])> {
    let por = w.cfg.phys.porosity;
    let mut p = kept.clone();
    moves::reconcile(w, s, i, &mut p);
    let mut f = found(w, s, i, &p);
    if moving && moves::settle(s, &mut p, &f, por) {
        f = found(w, s, i, &p);
    }
    let (to, deep) = if moving { (s.fill(i).wet(stands(&f, por)), deep::under(w, s.fill(i), liquid(&f, por))) } else { (p.wet, None) };
    if to <= 0.0 || (to >= 1.0 && deep.is_none() && (moving || !p.deep())) {
        return None;
    }
    let to = to.min(1.0);
    if to != p.wet {
        let b = p.bottom();
        moves::strip(s, i, &mut p, (&f[b], por), to);
        f = found(w, s, i, &p);
    }
    if moving && deep::layer(s, i, &mut p, &f, (deep, por)) {
        f = found(w, s, i, &p);
    }
    p.t = f.each_ref().map(|x| x.th.t);
    p.on = f.each_ref().map(|x| x.th.on);
    let piece = [LAND, TOP, DEEP].map(|q| piece(w, s, i, (&p, q), &f));
    let link = conductance(w, s, i, &p, &f);
    let row = row(w, s, i, &p, &f);
    p.dry = (-f[LAND].over(por)).max(0.0);
    Some((p.clone(), Mixed { piece, link, solved: p, row }, row))
}

/// The parts of cell `i` made from it as one node, its row `row` (`land::node_state`) with a new surface: where what
/// stands over its pores (or, where its pores are not full, what its parts of the bodies held at the last pass,
/// `State::held`) covers part of its ground (`Fill::wet`), land and liquid at its temperature and phases, the liquid
/// holding that and its area's share of the rest, its pores full; where it covers all of it deeper than `h_mix`, all of
/// it the liquid's, whose deep its solve makes (`deep::layer`). None elsewhere.
pub(super) fn create(w: &World, s: &Scratch, i: usize, row: &[f64]) -> Option<Parts> {
    let (n, area, por) = (w.mesh.cells().n(), w.mesh.cells().area()[i], w.cfg.phys.porosity);
    let over = Some((row[11] - row[16] - row[10]).max(0.0) * area).filter(|&v| v > 0.0).unwrap_or_else(|| s.fill(i).held());
    let wet = s.fill(i).wet(over);
    if wet <= 0.0 || (wet >= 1.0 && deep::under(w, s.fill(i), over).is_none()) {
        return None;
    }
    let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let (cap, e) = (s.cap[i], w.store.heat(Class::Ground)[i]);
    let melt = melts(w, s, amt);
    let th = thaw_at(s, cap.max(1.0), e, amt, &melt);
    if wet >= 1.0 {
        let amts = (0..w.chem.ns()).map(|k| [0.0, amt(k), 0.0]).collect();
        return Some(Parts { wet: 1.0, heat: [0.0, e, 0.0], amt: amts, t: [th.t; PARTS], on: [th.on; PARTS], dry: 0.0 });
    }
    let (l, ..) = layers(w, s, (i, 1.0, 1.0), th.t, amt, |k| th.molten(k), |k| melt[k]);
    // The liquid's volume: what stands over the pores and its area's share of what they hold; where the pores are not
    // full, what its parts of the bodies held over the pores under them, full.
    let (pores, stands) = (por * l[4], l[2] - por * l[4]);
    let top = if stands > 0.0 { stands + wet * (l[2] - stands) } else { (over + wet * pores).min(l[2]) };
    if top <= 0.0 || l[2] <= 0.0 {
        return None;
    }
    // Each liquid's share in the liquid, mixed, and its area's share of the rest.
    let liquid = |k: usize| if s.state.vary.contains(&k) && th.t < s.sp[k].boil { amt(k) * th.molten(k) } else { 0.0 };
    let amts: Vec<[f64; PARTS]> = (0..w.chem.ns())
        .map(|k| {
            let held = liquid(k) * top / l[2] + wet * (amt(k) - liquid(k));
            [amt(k) - held, held, 0.0]
        })
        .collect();
    let c = s.skin[i] * wet + s.state.loose.iter().map(|&k| amts[k][TOP] * s.sp[k].cp).sum::<f64>();
    let fused: f64 = s.state.vary.iter().map(|&k| amts[k][TOP] * s.sp[k].lf * th.molten(k)).sum();
    let heat = c * th.t + fused;
    Some(Parts { wet, heat: [e - heat, heat, 0.0], amt: amts, t: [th.t; PARTS], on: [th.on; PARTS], dry: 0.0 })
}

/// Part `q` of `p` in cell `i`, its parts found as `f`, as the air's step reads it (`Piece`): the top's reflectance
/// that of its column with its deep under it.
fn piece(w: &World, s: &Scratch, i: usize, (p, q): (&Parts, usize), f: &[Found; PARTS]) -> Piece {
    let (share, por, x) = (p.share(q), w.cfg.phys.porosity, &f[q]);
    let (th, c, area) = (&x.th, x.cap.max(1.0), w.mesh.cells().area()[i] * share);
    let (pk, phi) = th.on.unwrap_or((NONE, 0.0));
    // The span off the plateaus, as temperatures (as `land::node_state` finds it).
    let span = [if th.lo > f64::MIN { (th.lo - th.held) / c } else { f64::NEG_INFINITY }, if th.hi < f64::MAX { (th.hi - th.held) / c } else { f64::INFINITY }];
    let below = if q == TOP { f[DEEP].l } else { [0.0; 6] };
    let per = |v: f64| if area > 0.0 { v / area } else { 0.0 };
    Piece {
        share,
        ground: p.ground(q),
        amt: p.amt.iter().map(|a| a[q]).collect(),
        heat: p.heat[q],
        cap: x.cap,
        tg: th.t,
        pk,
        phi,
        latent: Latent { held: th.held, span, lat: th.lat, molten: th.bits },
        cover: [x.cover[0], x.cover[1], per(x.l[0])],
        alb: column(w, s, (i, share), &std::array::from_fn(|y| x.l[y] + below[y])),
        pd: per(por * x.l[4]),
        below,
    }
}

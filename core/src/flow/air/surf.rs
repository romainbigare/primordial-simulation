//! What a cell's exchange with the air reads of the ground it solves (`Surf`): one cell's (of a cell of parts, each
//! part that faces the air), or a body's of standing liquid under one air node as one surface (laws.md 1.4): its cells there that
//! mix as one (`basin::one_surface`) with the same phases (`patches`), solved once with their summed area, heat, heat
//! capacity and gases, at their mean temperature, reflectance and air, and the sunlight of their mean latitude; and
//! each of its active gases as its evaporation reads it (`Vapour`).

use super::super::land::{boil_off, column, layers, melts, reflectance, thaw_at, wet_share, NONE};
use super::super::{basin::{one_surface, phases}, parts::{amounts, Piece, LAND, TOP}, scratch::Latent, Scratch};
use super::cells::{exchange, put, rise, sunlight, Ctx, Response, Work, FIXED, LINK, SPACE, UP};
use super::radiation::on_slope;
use super::exchange::bucket;
use super::lerp;
use super::step::Active;
use crate::{store::Class, units::{MAX_SUBSTANCES, R_GAS}, World, AIR};

/// The ground an exchange solves (`cells::exchange`): its area, m², its air node, the area of the air it sees (`lerp`),
/// m², and its mean sunlight at the top of the air, W/m² of its plan (`radiation::on_slope`); that air's drop with height, K/m, and mean height, m, and
/// its head, m (`touching`); its temperature, K, plateau's substance and molten share; heat capacity, J/K; fusion
/// enthalpy held and the energies it holds it between (`Latent`); soft cover (`Scratch::cover`); reflectance; pore
/// space as a depth, m; heat, J; what it holds of each active gas, mol (`amt[k]`); the fusion enthalpy of its
/// plateau's substance, J; its cover of bodies (`World::canopy`) and the share of light they absorb and keep; and a
/// cell of it, whose other numbers (its layers, for a soft cover's end reflectance) it is, with the part of that cell it
/// is (`parts`; none: the whole cell) and that part's share of the cell's area.
pub(super) struct Surf {
    pub(super) area: f64,
    pub(super) j: usize,
    pub(super) seen_area: f64,
    pub(super) sun: f64,
    pub(super) gam: f64,
    pub(super) zair: f64,
    pub(super) head: f64,
    pub(super) tg: f64,
    pub(super) pk: usize,
    pub(super) phi: f64,
    pub(super) cap: f64,
    pub(super) latent: Latent,
    pub(super) cover: [f64; 3],
    pub(super) alb: f64,
    pub(super) pd: f64,
    /// The share of its area the liquid standing over its pores covers (`land::wet_share`)
    pub(super) wet: f64,
    pub(super) heat: f64,
    pub(super) amt: [f64; MAX_SUBSTANCES],
    pub(super) fusion: f64,
    pub(super) canopy: f64,
    pub(super) absorb: f64,
    pub(super) keep: f64,
    /// A second film over it beside the air's (`World::second_film`): its substance, mol K/(Pa s), and the share of
    /// that substance it holds which the film reaches
    pub(super) second: Option<(usize, f64, f64)>,
    pub(super) cell: usize,
    pub(super) part: Option<usize>,
    pub(super) share: f64,
}

impl Surf {
    /// Ground cell `i` under the mean sunlight `sun` of its latitude and slope, W/m² of its plan: its node, or its
    /// liquid's top where that alone faces the air (`Scratch::top_alone`).
    pub(super) fn of_cell(cx: &Ctx, i: usize, sun: f64) -> Surf {
        match cx.s.top_alone(i) {
            Some(top) => Surf::of_part(cx, i, sun, (TOP, top)),
            None => Surf::node(cx, i, sun),
        }
    }
    /// Ground node `i` as a whole under the mean sunlight `sun`, W/m² of its plan.
    fn node(cx: &Ctx, i: usize, sun: f64) -> Surf {
        let (w, s) = (cx.w, cx.s);
        let n = w.mesh.cells().n();
        let ground = w.store.amounts(Class::Ground);
        let mut amt = [0.0; MAX_SUBSTANCES];
        cx.act.iter().for_each(|ac| amt[ac.k] = ground[ac.k * n + i]);
        let pk = s.state.pk[i];
        Surf {
            area: w.mesh.cells().area()[i],
            j: w.mesh.cells().parent()[i] as usize,
            seen_area: s.seen[i],
            sun,
            gam: lerp(s, &s.gam, i),
            zair: lerp(s, &s.zair, i),
            head: s.head[i],
            tg: s.state.tg[i],
            pk,
            phi: s.state.phi[i],
            cap: s.cap[i],
            latent: s.latent[i],
            cover: s.cover[i],
            alb: s.alb[i],
            pd: s.pd[i],
            wet: wet_share(s, i),
            heat: w.store.heat(Class::Ground)[i],
            amt,
            fusion: if pk != NONE { ground[pk * n + i] * s.sp[pk].lf } else { 0.0 },
            canopy: w.canopy[i] as f64,
            absorb: crate::contract::stand(w, i).absorb(),
            keep: crate::contract::stand(w, i).keep(),
            second: w.second_film(i).map(|(k, film, reach)| (k, film * w.mesh.cells().area()[i], reach)),
            cell: i,
            part: None,
            share: 1.0,
        }
    }
    /// Part `q` of ground cell `i` (`parts::Piece`), one that faces the air, under the mean sunlight `sun`, W/m² of its
    /// plan: its own area, heat, phases, soft cover, reflectance and pores, each uniform over it (its liquid over none
    /// of its land, over all of its liquid's top); the cell's air, head and cover of bodies.
    pub(super) fn of_part(cx: &Ctx, i: usize, sun: f64, (q, p): (usize, &Piece)) -> Surf {
        let mut amt = [0.0; MAX_SUBSTANCES];
        cx.act.iter().for_each(|ac| amt[ac.k] = p.amt[ac.k]);
        let g = Surf::node(cx, i, sun);
        Surf {
            area: g.area * p.share,
            tg: p.tg,
            pk: p.pk,
            phi: p.phi,
            cap: p.cap,
            latent: p.latent,
            cover: p.cover,
            alb: p.alb,
            pd: p.pd,
            wet: if q == TOP { 1.0 } else { 0.0 },
            heat: p.heat,
            amt,
            fusion: if p.pk != NONE { p.amt[p.pk] * cx.s.sp[p.pk].lf } else { 0.0 },
            // The second film stands over its land: all of the cell's over its land part, none over its liquid's top.
            second: g.second.filter(|_| q == LAND),
            part: Some(q),
            share: p.share,
            ..g
        }
    }
    /// The cells `cells` of one patch (`patches`), as each faces the air `faces` (`of_cell`), as one surface under the
    /// mean sunlight `sun`, W/m², seeing the air of the nodes of its node's list by the weights `sees` (`Patch`): their
    /// sums, and their means by area (by heat capacity for the temperature and the bounds of its plateaus).
    fn of_patch(cx: &Ctx, (cells, faces): (&[u32], &[Surf]), sun: f64, sees: &[(u32, f64)]) -> Surf {
        let (w, s) = (cx.w, cx.s);
        let (area_of, first) = (w.mesh.cells().area(), cells[0] as usize);
        let sum = |f: &dyn Fn(usize) -> f64| cells.iter().map(|&i| f(i as usize)).sum::<f64>();
        let sum_faces = |f: &dyn Fn(&Surf) -> f64| faces.iter().map(f).sum::<f64>();
        let (area, cap) = (sum(&|i| area_of[i]), sum_faces(&|g| g.cap));
        let by_area = |v: &[f64]| sum(&|i| v[i] * area_of[i]) / area;
        let face_by_area = |f: &dyn Fn(&Surf) -> f64| faces.iter().zip(cells).map(|(g, &i)| f(g) * area_of[i as usize]).sum::<f64>() / area;
        let by_cap = |f: &dyn Fn(&Surf) -> f64| sum_faces(&|g| f(g) * g.cap) / cap.max(f64::MIN_POSITIVE);
        let seen = |v: &[f64]| s.corners.mean(sees, v);
        // A bound of the plateaus the cells are kept between: their mean by heat capacity, or none where one has none there.
        let bound = |b: usize| if faces.iter().any(|g| g.latent.span[b].is_infinite()) { faces[0].latent.span[b] } else { by_cap(&|g| g.latent.span[b]) };
        let mut amt = [0.0; MAX_SUBSTANCES];
        cx.act.iter().for_each(|ac| amt[ac.k] = sum_faces(&|g| g.amt[ac.k]));
        Surf {
            area,
            j: w.mesh.cells().parent()[first] as usize,
            seen_area: by_area(&s.seen),
            sun,
            gam: seen(&s.gam),
            zair: seen(&s.zair),
            head: by_area(&s.head),
            tg: by_cap(&|g| g.tg),
            pk: NONE,
            phi: 0.0,
            cap,
            latent: Latent {
                held: sum_faces(&|g| g.latent.held),
                span: [bound(0), bound(1)],
                lat: [sum_faces(&|g| g.latent.lat[0]), sum_faces(&|g| g.latent.lat[1])],
                molten: faces[0].latent.molten,
            },
            cover: faces[0].cover,
            alb: face_by_area(&|g| g.alb),
            pd: face_by_area(&|g| g.pd),
            wet: face_by_area(&|g| g.wet),
            heat: sum_faces(&|g| g.heat),
            amt,
            fusion: 0.0,
            canopy: 0.0,
            absorb: 0.0,
            keep: 0.0,
            second: None,
            cell: first,
            part: None,
            share: 1.0,
        }
    }
}

/// The patches of the bodies of standing liquid that each solve one surface balance: under each air node, the cells of
/// one body that mix as one (`basin::one_surface`), by what is molten in them (`basin::phases`); at least two cells
/// each, in the order of their cells.
pub(super) fn patches(w: &World, s: &Scratch) -> Vec<Vec<u32>> {
    groups(w, |i| one_surface(w, s, i), |i| (s.basin.of[i], phases(s, i)))
}

/// The cells `member` names under each air node, by their body and their phases (`key`): at least two cells each, in
/// the order of their cells.
pub(super) fn groups(w: &World, member: impl Fn(usize) -> bool, key: impl Fn(usize) -> (u32, u64)) -> Vec<Vec<u32>> {
    let parent = w.mesh.cells().parent();
    let mut by: std::collections::BTreeMap<(u32, (u32, u64)), Vec<u32>> = std::collections::BTreeMap::new();
    (0..w.mesh.cells().n()).filter(|&i| member(i)).for_each(|i| by.entry((parent[i], key(i))).or_default().push(i as u32));
    by.into_values().filter(|p| p.len() > 1).collect()
}

/// The answer of one patch to the air it sees (`cells::Response`): its cells, in order; its parts (`cells::put`) at the
/// air it sees at the step's start, `air`, with their slopes, what each mol of each gas carries, and its changing outputs
/// from them; its fixed outputs; per cell its share of the heat parts (by the heat capacity of what faces the air, so
/// they warm alike) and of each active gas (by what that holds of it; by area where none holds any), `[c * (1 + nx) +
/// v]`; and the air it sees: per entry of its node's list (`air::Corners::sees`), the weight of that node, its cells'
/// weights there by their areas.
pub(super) struct Patch {
    pub(super) cells: Vec<u32>,
    pub(super) air: Vec<f64>,
    pub(super) parts: Vec<f64>,
    pub(super) carry: Vec<[f64; 2]>,
    pub(super) out: Vec<f64>,
    pub(super) fixed: [f64; FIXED],
    pub(super) share: Vec<f64>,
    pub(super) sees: Vec<(u32, f64)>,
}

impl Patch {
    /// The patch of the cells `cells`, which solves one surface (`Surf::of_patch`, under the sunlight of their mean
    /// latitude) under the mean by area of the air they see.
    pub(super) fn of(cx: &Ctx, cells: &[u32]) -> Patch {
        let (s, nx, na) = (cx.s, cx.act.len(), cx.w.mesh.level(AIR).n());
        let (lats, area) = (cx.w.mesh.cells().lat(), cx.w.mesh.cells().area());
        let (all, sees) = (cells.iter().map(|&i| area[i as usize]).sum::<f64>(), s.corners.by_area(&s.wt, area, cells));
        // Its sunlight: at its mean latitude by area, on its liquid's level surface.
        let sun = sunlight(cx.w, cx.decl, cells.iter().map(|&i| lats[i as usize] * area[i as usize]).sum::<f64>() / all)[0];
        let faces: Vec<Surf> = cells.iter().map(|&i| Surf::of_cell(cx, i as usize, sun)).collect();
        let g = Surf::of_patch(cx, (cells, &faces), sun, &sees);
        let mean = |v: &[f64]| s.corners.mean(&sees, v);
        let air: Vec<f64> = std::iter::once(mean(cx.t0)).chain((0..nx).map(|y| mean(&cx.x0[y * na..(y + 1) * na]))).collect();
        let mut work = Work::default();
        let fixed = exchange(cx, &g, &air, &mut work);
        let mut out = vec![0.0; Response::width(nx) - FIXED];
        put(&work.p, &work.carry, &mut out);
        let mut share = Vec::with_capacity(cells.len() * (1 + nx));
        for (&i, face) in cells.iter().zip(&faces) {
            share.push(face.cap / g.cap);
            // Where none of its cells holds a gas, what condenses of it lands by area.
            share.extend(cx.act.iter().map(|ac| if g.amt[ac.k] > 0.0 { face.amt[ac.k] / g.amt[ac.k] } else { area[i as usize] / all }));
        }
        Patch { cells: cells.to_vec(), air, parts: work.p, carry: work.carry, out, fixed, share, sees }
    }
}

/// An active gas of a ground node, as its evaporation reads it (`Vapour::evaporate`): its partial pressure near the
/// ground, Pa, and that per mol of the gas in the air, Pa/mol; and what does not move with the node's end temperature.
pub(super) struct Vapour {
    k: usize,
    /// It may evaporate, and the node holds some.
    pub(super) up: bool,
    pub(super) pa: f64,
    pub(super) per: f64,
    /// What the node holds, mol; what it holds wet (`exchange::bucket`), mol; what its pores hold full, mol; the share
    /// of its area the liquid standing over them covers; the air's film, mol K/(Pa s).
    held: f64,
    wet: f64,
    pores: f64,
    share: f64,
    film: f64,
    /// A second film beside the air's (`Surf::second`), in series with the air's, mol K/(Pa s), and what of it lies
    /// behind that film, mol.
    second: f64,
    behind: f64,
    /// Over its boiling point, what the node's heat boils, mol (`land::boil_off`).
    boil: Option<f64>,
    /// Its vaporisation enthalpy, and that plus its fusion enthalpy, J/mol; the fusion enthalpy a mol of it holds in the
    /// node (`Scratch::fusion`), J/mol.
    lv: f64,
    lf_lv: f64,
    fusion: f64,
}

impl Vapour {
    /// The active gas `ac` of the ground `g` with `x` mol of it in the air, at `per` Pa per mol.
    pub(super) fn of(cx: &Ctx, g: &Surf, ac: Active, x: f64, per: f64) -> Vapour {
        let (w, sp, area) = (cx.w, cx.s.sp[ac.k], g.area);
        let held = g.amt[ac.k];
        let up = ac.up && held > 0.0;
        let boil = (up && g.tg >= sp.boil).then(|| held.min(boil_off(&sp, g.cap, g.tg)));
        let melted = g.latent.molten >> ac.k & 1 == 1;
        let pores = g.pd * area / sp.v[melted as usize];
        let (film, wet) = (w.cfg.evap / R_GAS * area, w.cfg.phys.field * pores);
        // The fusion enthalpy a mol of it holds in the ground: none where it is solid or on its plateau.
        let fusion = if g.pk != ac.k && melted { sp.lf } else { 0.0 };
        let (second, behind) = match g.second {
            Some((k, f, reach)) if k == ac.k && f > 0.0 => (f * film / (f + film), reach * held),
            _ => (0.0, 0.0),
        };
        Vapour { k: ac.k, up, pa: x * per, per, held, wet, pores, share: g.wet, film, second, behind, boil, lv: sp.lv, lf_lv: sp.lf + sp.lv, fusion }
    }
    /// The heat `m` mol take from the node as they leave it, J: their fusion and vaporisation enthalpies, less the fusion
    /// enthalpy the node holds for them.
    pub(super) fn lat(&self, m: f64) -> f64 {
        m * self.lf_lv - m * self.fusion
    }
    /// What evaporates over the step from the node, ending at `t`, mol, and its slopes with `t` and with the partial
    /// pressure: the saturation deficit through the air's film (`evap`), at the open rate while wet
    /// (`exchange::bucket`), over its own ground: the share of it the standing liquid covers holds what its pores hold
    /// and all that stands, the rest what its pores hold, each by its area; and through a second film beside the air's
    /// (`Surf::second`, in series with the air's), at its open rate while what lies behind it lasts, from what the
    /// first leaves; over the boiling point, all of it that the node's heat over that point boils.
    pub(super) fn evaporate(&self, s: &Scratch, t: f64, dt: f64) -> [f64; 3] {
        if !self.up {
            return [0.0; 3];
        }
        if let Some(m) = self.boil {
            return [m, 0.0, 0.0];
        }
        let ps = s.sat[self.k].at(t);
        let e = self.film * (ps - self.pa) / t;
        if e <= 0.0 {
            return [0.0; 3];
        }
        let one = |held: f64| bucket(held, 0.0, e, self.wet, dt);
        let (m, dm_de) = if self.share == 0.0 || self.share == 1.0 {
            one(self.held)
        } else {
            let pore = self.held.min(self.pores);
            let ((dry, d_dry), (wet, d_wet)) = (one(pore), one(pore + (self.held - pore) / self.share));
            ((1.0 - self.share) * dry + self.share * wet, (1.0 - self.share) * d_dry + self.share * d_wet)
        };
        let (m2, dm2_de2) = bucket(self.behind.min(self.held - m), 0.0, self.second * (ps - self.pa) / t, 0.0, dt);
        // The open rates' slopes with `t` (Clausius-Clapeyron) and with the partial pressure.
        let films = dm_de * self.film + dm2_de2 * self.second;
        [m + m2, films * (ps * self.lv / (R_GAS * t * t) - (ps - self.pa) / t) / t, -films / t]
    }
}

/// The rows of the parts of cell `i` that face the air (`parts::Mixed::two_faces`), [land, top], as `respond` lays a
/// cell's out, under the air it sees at the step's start `air`: each part its own surface (`Surf::of_part`), the land
/// under the sun of the cell's slope, the liquid's top, which lies level, under the flat sun.
pub(super) fn parts_respond(cx: &Ctx, i: usize, air: &[f64]) -> [Vec<f64>; 2] {
    let (m, q) = (cx.s.mixed(i).expect("a cell of parts"), sunlight(cx.w, cx.decl, cx.w.mesh.cells().lat()[i]));
    let sun = [on_slope(q, rise(cx, i)), q[0]];
    let mut work = Work::default();
    [LAND, TOP].map(|p| {
        let ground = Surf::of_part(cx, i, sun[p], (p, &m.piece[p]));
        let mut row = vec![0.0; Response::width(cx.act.len())];
        let fixed = exchange(cx, &ground, air, &mut work);
        put(&work.p, &work.carry, &mut row[FIXED..]);
        row[..FIXED].copy_from_slice(&fixed);
        row
    })
}

/// The reflectance of the surface of the ground `g` (a soft cover's: never a patch's, `surf::patches`; a cell's or a
/// part's, over the part under it) at the end of the step whose sunlight and outputs (`exchange`) are `(sun, out)`: its
/// amounts with what evaporates, its energy with the heat the store books for it, and its phases there.
pub(super) fn end_reflectance(cx: &Ctx, g: &Surf, (sun, out): (f64, &[f64])) -> f64 {
    let (w, s, stride, i) = (cx.w, cx.s, 2 + cx.act.len(), g.cell);
    let piece = g.part.and_then(|q| s.mixed(i).map(|m| &m.piece[q]));
    // Its amounts, read once: the melting points read them many times.
    let mut amt = amounts(w, i, piece);
    let (mut c, e) = (g.cap, g.heat + sun - out[LINK * stride] - out[SPACE * stride]);
    for (y, ac) in cx.act.iter().enumerate() {
        let m = out[(UP + y) * stride];
        (amt[ac.k], c) = (amt[ac.k] - m, c - m * s.sp[ac.k].cp);
    }
    // The melting points, found once: the plateaus and the cover read them.
    let melt = melts(w, s, |k| amt[k]);
    let th = thaw_at(s, c.max(1.0), e, |k| amt[k], &melt);
    match piece {
        Some(p) => {
            let l = layers(w, s, (i, g.share, p.ground), th.t, |k| amt[k], |k| th.molten(k), |k| melt[k]).0;
            column(w, s, (i, g.share), &std::array::from_fn(|y| l[y] + p.below[y]))
        }
        None => reflectance(w, s, i, &layers(w, s, (i, 1.0, 1.0), th.t, |k| amt[k], |k| th.molten(k), |k| melt[k]).0),
    }
}

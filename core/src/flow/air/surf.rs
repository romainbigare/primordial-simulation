//! What a cell's exchange with the air reads of the ground it solves (`Surf`): one cell's, or the low basin's under one
//! air node as one surface (laws.md 1.4): its cells there that mix as one (`basin::one_surface`) with the same phases
//! (`patches`), solved once with their summed area, heat, heat capacity and gases, at their mean temperature,
//! reflectance and air, and the sunlight of their mean latitude; and each of its active gases as its evaporation reads
//! it (`Vapour`).

use super::super::{basin::one_surface, land::{boil_off, NONE}, scratch::Latent, Scratch};
use super::cells::{exchange, put, sunlight, Ctx, Response, Work, FIXED};
use super::exchange::bucket;
use super::lerp;
use super::step::Active;
use crate::{store::Class, units::{MAX_SUBSTANCES, R_GAS}, World, AIR};

/// The ground an exchange solves (`cells::exchange`): its area, m², its air node, the area of the air it sees (`lerp`),
/// m², and its mean sunlight at the top of the air, W/m²; that air's drop with height, K/m, and mean height, m, and
/// its head, m (`touching`); its temperature, K, plateau's substance and molten share; heat capacity, J/K; fusion
/// enthalpy held and the energies it holds it between (`Latent`); soft cover (`Scratch::cover`); reflectance; pore
/// space as a depth, m; heat, J; what it holds of each active gas, mol (`amt[k]`); the fusion enthalpy of its
/// plateau's substance, J; its cover of bodies (`World::canopy`) and the share of light they absorb and keep; and a
/// cell of it, whose other numbers (its layers, for a soft cover's end reflectance) it is.
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
    pub(super) heat: f64,
    pub(super) amt: [f64; MAX_SUBSTANCES],
    pub(super) fusion: f64,
    pub(super) canopy: f64,
    pub(super) absorb: f64,
    pub(super) keep: f64,
    pub(super) cell: usize,
}

impl Surf {
    /// Ground cell `i` under the mean sunlight `sun` of its latitude, W/m².
    pub(super) fn of_cell(cx: &Ctx, i: usize, sun: f64) -> Surf {
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
            heat: w.store.heat(Class::Ground)[i],
            amt,
            fusion: if pk != NONE { ground[pk * n + i] * s.sp[pk].lf } else { 0.0 },
            canopy: w.canopy[i] as f64,
            absorb: w.life.absorb.get(i).map_or(0.0, |&x| x as f64),
            keep: w.life.keep.get(i).map_or(0.0, |&x| x as f64),
            cell: i,
        }
    }
    /// The cells `cells` of one patch (`patches`) as one surface under the mean sunlight `sun`, W/m², seeing the air
    /// of the nodes of its node's list by the weights `sees` (`Patch`): their sums, and their means by area (by heat
    /// capacity for the temperature and the bounds of its plateaus).
    fn of_patch(cx: &Ctx, cells: &[u32], sun: f64, sees: &[(u32, f64)]) -> Surf {
        let (w, s) = (cx.w, cx.s);
        let (n, area_of, first) = (w.mesh.cells().n(), w.mesh.cells().area(), cells[0] as usize);
        let sum = |f: &dyn Fn(usize) -> f64| cells.iter().map(|&i| f(i as usize)).sum::<f64>();
        let (area, cap) = (sum(&|i| area_of[i]), sum(&|i| s.cap[i]));
        let by_area = |v: &[f64]| sum(&|i| v[i] * area_of[i]) / area;
        let by_cap = |f: &dyn Fn(usize) -> f64| sum(&|i| f(i) * s.cap[i]) / cap.max(f64::MIN_POSITIVE);
        let near = &s.corners.sees;
        let seen = |v: &[f64]| sees.iter().map(|&(at, wt)| wt * v[near[at as usize] as usize]).sum::<f64>();
        // A bound of the plateaus the cells are kept between: their mean by heat capacity, or none where one has none there.
        let bound = |b: usize| if cells.iter().any(|&i| s.latent[i as usize].span[b].is_infinite()) { s.latent[first].span[b] } else { by_cap(&|i| s.latent[i].span[b]) };
        let ground = w.store.amounts(Class::Ground);
        let mut amt = [0.0; MAX_SUBSTANCES];
        cx.act.iter().for_each(|ac| amt[ac.k] = sum(&|i| ground[ac.k * n + i]));
        Surf {
            area,
            j: w.mesh.cells().parent()[first] as usize,
            seen_area: by_area(&s.seen),
            sun,
            gam: seen(&s.gam),
            zair: seen(&s.zair),
            head: by_area(&s.head),
            tg: by_cap(&|i| s.state.tg[i]),
            pk: NONE,
            phi: 0.0,
            cap,
            latent: Latent {
                held: sum(&|i| s.latent[i].held),
                span: [bound(0), bound(1)],
                lat: [sum(&|i| s.latent[i].lat[0]), sum(&|i| s.latent[i].lat[1])],
                molten: s.latent[first].molten,
            },
            cover: s.cover[first],
            alb: by_area(&s.alb),
            pd: by_area(&s.pd),
            heat: sum(&|i| w.store.heat(Class::Ground)[i]),
            amt,
            fusion: 0.0,
            canopy: 0.0,
            absorb: 0.0,
            keep: 0.0,
            cell: first,
        }
    }
}

/// The patches of the low basin that each solve one surface balance: under each air node, the cells of the low basin that
/// mix as one (`basin::one_surface`), by what is molten in them; at least two cells each, in the order of their cells.
pub(super) fn patches(w: &World, s: &Scratch) -> Vec<Vec<u32>> {
    let parent = w.mesh.cells().parent();
    let mut by: std::collections::BTreeMap<(u32, u64), Vec<u32>> = std::collections::BTreeMap::new();
    (0..w.mesh.cells().n()).filter(|&i| one_surface(w, s, i)).for_each(|i| by.entry((parent[i], s.latent[i].molten)).or_default().push(i as u32));
    by.into_values().filter(|p| p.len() > 1).collect()
}

/// The answer of one patch to the air it sees (`cells::Response`): its cells, in order; its parts (`cells::put`) at the
/// air it sees at the step's start, `air`, with their slopes, what each mol of each gas carries, and its changing outputs
/// from them; its fixed outputs; per cell its share of the heat parts (by heat capacity, so they warm alike) and of each
/// active gas (by what it holds of it), `[c * (1 + nx) + v]`; and the air it sees: per entry of its node's list
/// (`air::Corners::sees`), the weight of that node, its cells' weights there by their areas.
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
        let (n, lats, area) = (cx.w.mesh.cells().n(), cx.w.mesh.cells().lat(), cx.w.mesh.cells().area());
        let all: f64 = cells.iter().map(|&i| area[i as usize]).sum();
        let mut sees: Vec<(u32, f64)> = Vec::new();
        for &i in cells {
            for (&at, &wt) in s.corners.entry[i as usize].iter().zip(&s.wt[i as usize]).filter(|e| *e.1 > 0.0) {
                let wt = wt * area[i as usize] / all;
                match sees.iter_mut().find(|e| e.0 == at) {
                    Some(e) => e.1 += wt,
                    None => sees.push((at, wt)),
                }
            }
        }
        // Its sunlight: at its mean latitude by area.
        let g = Surf::of_patch(cx, cells, sunlight(cx.w, cx.decl, cells.iter().map(|&i| lats[i as usize] * area[i as usize]).sum::<f64>() / all), &sees);
        let near = &s.corners.sees;
        let mean = |v: &[f64]| sees.iter().map(|&(at, wt)| wt * v[near[at as usize] as usize]).sum::<f64>();
        let air: Vec<f64> = std::iter::once(mean(cx.t0)).chain((0..nx).map(|y| mean(&cx.x0[y * na..(y + 1) * na]))).collect();
        let mut work = Work::default();
        let fixed = exchange(cx, &g, &air, &mut work);
        let mut out = vec![0.0; Response::width(nx) - FIXED];
        put(&work.p, &work.carry, &mut out);
        let mut share = Vec::with_capacity(cells.len() * (1 + nx));
        let ground = cx.w.store.amounts(Class::Ground);
        for &i in cells {
            share.push(s.cap[i as usize] / g.cap);
            share.extend(cx.act.iter().map(|ac| if g.amt[ac.k] > 0.0 { ground[ac.k * n + i as usize] / g.amt[ac.k] } else { 0.0 }));
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
    /// What the node holds, mol; what it holds wet (`exchange::bucket`), mol; the air's film, mol K/(Pa s).
    held: f64,
    wet: f64,
    film: f64,
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
        let (film, wet) = (w.cfg.evap / R_GAS * area, w.cfg.phys.field * g.pd * area / sp.v[melted as usize]);
        // The fusion enthalpy a mol of it holds in the ground: none where it is solid or on its plateau.
        let fusion = if g.pk != ac.k && melted { sp.lf } else { 0.0 };
        Vapour { k: ac.k, up, pa: x * per, per, held, wet, film, boil, lv: sp.lv, lf_lv: sp.lf + sp.lv, fusion }
    }
    /// The heat `m` mol take from the node as they leave it, J: their fusion and vaporisation enthalpies, less the fusion
    /// enthalpy the node holds for them.
    pub(super) fn lat(&self, m: f64) -> f64 {
        m * self.lf_lv - m * self.fusion
    }
    /// What evaporates over the step from the node, ending at `t`, mol, and its slopes with `t` and with the partial
    /// pressure: the saturation deficit through the air's film (`evap`), at the open rate while wet
    /// (`exchange::bucket`); over the boiling point, all of it that the node's heat over that point boils.
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
        let (m, dm_de) = bucket(self.held, 0.0, e, self.wet, dt);
        // The open rate's slopes with `t` (Clausius-Clapeyron) and with the partial pressure.
        [m, dm_de * self.film * (ps * self.lv / (R_GAS * t * t) - (ps - self.pa) / t) / t, -dm_de * self.film / t]
    }
}

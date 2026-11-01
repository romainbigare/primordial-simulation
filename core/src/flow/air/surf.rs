//! What a cell's exchange with the air reads of the ground it solves (`Surf`): one cell's, or the low basin's under one
//! air node as one surface (laws.md 1.4): its cells there that mix as one (`basin::one_surface`) with the same phases
//! (`patches`), solved once with their summed area, heat, heat capacity and gases, at their mean temperature,
//! reflectance and air, and the sunlight of their mean latitude; and each of its active gases as its evaporation reads
//! it (`Vapour`).

use super::super::{basin::one_surface, land::{boil_off, NONE}, scratch::Latent, Scratch};
use super::cells::{exchange, put, sunlight, Ctx, Response, Work, FIXED, GAS, KEPT, LIGHT, SUN, TO_AIR, TO_SPACE};
use super::exchange::bucket;
use super::lerp;
use super::step::Active;
use crate::{store::Class, units::{MAX_SUBSTANCES, R_GAS}, World};

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
    /// The cells `cells` of one patch (`patches`) as one surface, each `Surf::of_cell`: their sums, and their means by
    /// area (by heat capacity for the temperature).
    pub(super) fn of_patch(cells: &[Surf]) -> Surf {
        let area: f64 = cells.iter().map(|c| c.area).sum();
        let cap: f64 = cells.iter().map(|c| c.cap).sum();
        let by_area = |f: fn(&Surf) -> f64| cells.iter().map(|c| f(c) * c.area).sum::<f64>() / area;
        let sum = |f: fn(&Surf) -> f64| cells.iter().map(f).sum::<f64>();
        // A bound of the plateaus the cells are kept between: their sum, or none where they have none there.
        let bound = |f: fn(&Latent) -> f64, none: f64| if cells.iter().any(|c| f(&c.latent) == none) { none } else { cells.iter().map(|c| f(&c.latent)).sum() };
        let first = &cells[0];
        let mut amt = [0.0; MAX_SUBSTANCES];
        cells.iter().for_each(|c| amt.iter_mut().zip(&c.amt).for_each(|(a, x)| *a += x));
        Surf {
            area,
            j: first.j,
            seen_area: by_area(|c| c.seen_area),
            sun: by_area(|c| c.sun),
            gam: by_area(|c| c.gam),
            zair: by_area(|c| c.zair),
            head: by_area(|c| c.head),
            tg: cells.iter().map(|c| c.tg * c.cap).sum::<f64>() / cap.max(f64::MIN_POSITIVE),
            pk: NONE,
            phi: 0.0,
            cap,
            latent: Latent {
                held: sum(|c| c.latent.held),
                lo: bound(|l| l.lo, f64::MIN),
                hi: bound(|l| l.hi, f64::MAX),
                lat: [sum(|c| c.latent.lat[0]), sum(|c| c.latent.lat[1])],
                molten: first.latent.molten,
            },
            cover: first.cover,
            alb: by_area(|c| c.alb),
            pd: by_area(|c| c.pd),
            heat: sum(|c| c.heat),
            amt,
            fusion: 0.0,
            canopy: 0.0,
            absorb: 0.0,
            keep: 0.0,
            cell: first.cell,
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

/// The rows of the response (`cells::Response`) of the cells `cells` of one patch, which solves one surface
/// (`Surf::of_patch`, under the sunlight of their mean latitude) under the mean by area of the air they see (`seen`, per
/// cell as `Response` has it): each cell's
/// share of its parts, of the heat by its heat capacity (so they warm alike) and of each gas by what it holds of it.
pub(super) fn patch_rows(cx: &Ctx, cells: &[u32], seen: &[f64]) -> Vec<f64> {
    let (nx, width, lats) = (cx.act.len(), Response::width(cx.act.len()), cx.w.mesh.cells().lat());
    let stride = 2 + nx;
    let members: Vec<Surf> = cells.iter().map(|&i| Surf::of_cell(cx, i as usize, 0.0)).collect();
    let mut g = Surf::of_patch(&members);
    // Its sunlight: at its mean latitude by area.
    g.sun = sunlight(cx.w, cx.decl, members.iter().zip(cells).map(|(m, &i)| lats[i as usize] * m.area).sum::<f64>() / g.area);
    let mut air = vec![0.0; 1 + nx];
    for (m, &i) in members.iter().zip(cells) {
        air.iter_mut().zip(&seen[i as usize * (1 + nx)..]).for_each(|(a, &x)| *a += x * m.area / g.area);
    }
    let mut work = Work::default();
    let fixed = exchange(cx, &g, &air, &mut work);
    let (p, carry) = (&work.p, &work.carry);
    let (mut rows, mut part) = (vec![0.0; cells.len() * width], Vec::with_capacity(p.len()));
    for (m, row) in members.iter().zip(rows.chunks_exact_mut(width)) {
        let heat = m.cap / g.cap;
        part.clear();
        part.extend_from_slice(p);
        part[TO_AIR * stride..(TO_SPACE + 1) * stride].iter_mut().for_each(|x| *x *= heat);
        for (y, ac) in cx.act.iter().enumerate() {
            let share = if g.amt[ac.k] > 0.0 { m.amt[ac.k] / g.amt[ac.k] } else { 0.0 };
            part[(GAS + y) * stride..(GAS + y + 1) * stride].iter_mut().for_each(|x| *x *= share);
        }
        put(&part, carry, &mut row[FIXED..]);
        (row[SUN], row[LIGHT], row[KEPT]) = (fixed[SUN] * heat, fixed[LIGHT], fixed[KEPT] * heat);
    }
    rows
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

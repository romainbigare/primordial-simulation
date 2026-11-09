//! Ground nodes: heat capacity and temperature on the melting plateaus (`ground_state`), phases, the surface and
//! heads, and heat conducted sideways (`conduct`).

use super::{on_land, parts, scratch::{Latent, Scratch, Sp}};
use crate::{chem::Phase, ground::{heat_above_floor, melting, skin_of, Melting}, solve::sweep::{by_size, Net}, store::{apply, Class, LinkFlux}, units::{MAX_SUBSTANCES, T_FLOOR}, World};

/// No substance on a melting plateau (`Scratch::pk`).
pub(super) const NONE: usize = usize::MAX;

/// Molten share of substance `k` in ground node `i` (liquid, or gas above boiling).
#[inline]
pub(super) fn molten(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.state.pk[i] == k { s.state.phi[i] } else { melted(s, k, i) as u8 as f64 }
}

/// Is substance `k` at or over its melting point in ground node `i` (`World::melt`, as of the last `ground_state`)?
#[inline]
pub(super) fn melted(s: &Scratch, k: usize, i: usize) -> bool {
    s.latent[i].molten >> k & 1 == 1
}

/// Liquid share of substance `k` in ground node `i`.
#[inline]
pub(super) fn wet(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.state.tg[i] < s.sp[k].boil { molten(s, k, i) } else { 0.0 }
}

/// Rise of the solid surface and of the head of a ground node for `m` mol of a substance (negative: fall).
#[inline]
pub(super) fn rise(p: &Sp, t: f64, liq: bool, m: f64, area: f64) -> [f64; 2] {
    if t >= p.boil {
        [0.0; 2]
    } else if !liq {
        let d = m * p.v[0] / area;
        [d, d]
    } else {
        [0.0, m * p.v[1] / area]
    }
}

/// Raise (or lower) the surface of ground node `i` by `[solid, head]` m, counted toward its next surface
/// (`Scratch::shift`).
#[inline]
pub(super) fn lift(s: &mut Scratch, i: usize, [dz, dh]: [f64; 2]) {
    s.shift[i] += dz.abs() + dh.abs();
    s.z[i] += dz;
    s.head[i] += dh;
}

impl Scratch {
    /// Ground node `i`, of `area` m², gained `m` mol of `k` (negative: lost): its heat capacity, fusion enthalpy and
    /// surface follow.
    #[inline]
    pub(super) fn note(&mut self, k: usize, i: usize, m: f64, area: f64) {
        self.hold(k, i, m);
        let d = rise(&self.sp[k], self.state.tg[i], melted(self, k, i), m, area);
        lift(self, i, d);
    }
    /// `note` but for the surface: for a gas dissolved in the liquid, which takes no room.
    #[inline]
    pub(super) fn hold(&mut self, k: usize, i: usize, m: f64) {
        self.cap[i] += m * self.sp[k].cp;
        self.latent[i].held += self.fusion(k, i, m);
    }
    /// Fusion enthalpy `m` mol of `k` hold in ground node `i`, J: none where `k` is solid or on its plateau.
    #[inline]
    pub(super) fn fusion(&self, k: usize, i: usize, m: f64) -> f64 {
        if self.state.pk[i] != k && melted(self, k, i) { m * self.sp[k].lf } else { 0.0 }
    }
}

/// Most mol of a substance `p` a ground node of heat capacity `cap`, J/K, at `t` K can boil with its heat over the
/// substance's boiling point, cooling to it: each takes its vaporisation enthalpy and its heat over that point.
#[inline]
pub(super) fn boil_off(p: &Sp, cap: f64, t: f64) -> f64 {
    cap.max(1.0) * (t - p.boil) / (p.cp * (t - p.boil) + p.lv)
}

/// Solid amount of `k` in ground node `i`, mol.
#[inline]
pub(super) fn solid(w: &World, s: &Scratch, k: usize, i: usize) -> f64 {
    w.store.amounts(Class::Ground)[k * w.mesh.cells().n() + i] * (1.0 - molten(s, k, i))
}

/// Temperature of the liquid of ground node `i` now, K: the node's, no warmer than at the last `ground_state`; of a
/// node of parts its parts' by the liquid each holds (`parts`), the liquid that leaves it.
#[inline]
pub(super) fn t_now(w: &World, s: &Scratch, i: usize) -> f64 {
    match s.mixed(i) {
        Some(m) => m.liquid_t(&s.sp).max(T_FLOOR),
        None => s.state.tg[i].min((w.store.heat(Class::Ground)[i] - s.latent[i].held) / s.cap[i].max(1.0)).max(T_FLOOR),
    }
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node: each takes its heat capacities from
/// the amounts now (`capacities`; other laws move matter between liquid steps, and a stale capacity would read a node
/// far too hot or cold) and its temperature by its energy alone (`steady`); a node off its plateaus whose temperature
/// left the span between them, or that is on one, is found anew, and one whose surface moved (`moved`) gets a new
/// surface (`parts::solve_nodes`, with `moving` as it says), in parallel with `threads`. A full build finds every node
/// anew, the basin (`basin::find`) and the gases that dissolve in the month (`air::henry`).
pub(super) fn ground_state(w: &World, s: &mut Scratch, (full, moving): (bool, bool)) {
    let n = w.mesh.cells().n();
    let caps = capacities(w, s, 0..n);
    let mut anew = Vec::new();
    for (i, &cap) in caps.iter().enumerate() {
        let thaw = !steady(w, s, i, cap) || full;
        if thaw || moved(w, s, i) {
            anew.push((i as u32, thaw));
        }
    }
    parts::solve_nodes(w, s, &anew, (full, moving));
    if full {
        s.basin = super::basin::find(w, s);
        s.henry = super::air::henry(w, s, super::range(&s.state.tg).0);
    }
}

/// The state of the ground nodes `cells` alone brought up to date, as `ground_state` does it.
pub(super) fn cells_state(w: &World, s: &mut Scratch, cells: &[u32]) {
    let caps = capacities(w, s, cells.iter().map(|&i| i as usize));
    let mut anew = Vec::new();
    for (&i, &cap) in cells.iter().zip(&caps) {
        let thaw = !steady(w, s, i as usize, cap);
        if thaw || moved(w, s, i as usize) {
            anew.push((i, thaw));
        }
    }
    parts::solve_nodes(w, s, &anew, (false, true));
}

/// A ground node's surface is found again once its solid surface and its head have moved since it was last found
/// (`Scratch::shift`) by this share of the depth that hides half of what lies under a soft solid (`hide`) and the liquid
/// standing on it: then its reflectance may have moved by about this share.
const SURFACE_SHARE: f64 = 1e-2;

/// Has the surface of ground node `i` moved enough to be found again (`SURFACE_SHARE`)?
fn moved(w: &World, s: &Scratch, i: usize) -> bool {
    s.shift[i] >= SURFACE_SHARE * (w.cfg.phys.hide[0] + (s.head[i] - s.pd[i] - s.z[i]).max(0.0))
}

/// The heat capacity of the skin of the bed of each ground node of `cells` and its own, J/K (`ground::skin_cap`, then
/// its loose amounts in order), summed substance by substance over the nodes.
fn capacities(w: &World, s: &Scratch, cells: impl Iterator<Item = usize> + Clone) -> Vec<[f64; 2]> {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    // Per node the heat capacity of its bonded amounts, J/K, and their volume, m³.
    let mut bed = vec![[0.0; 2]; cells.clone().count()];
    for &k in &s.state.bedk {
        let (cp, v, b) = (w.chem.subs[k].cp, w.chem.molar_volume(k, Phase::Solid), &w.store.amounts(Class::Bed)[k * n..(k + 1) * n]);
        cells.clone().zip(&mut bed).for_each(|(i, x)| (x[0], x[1]) = (x[0] + b[i] * cp, x[1] + b[i] * v));
    }
    let mut caps: Vec<[f64; 2]> = cells.clone().zip(&bed).map(|(i, x)| [skin_of(&w.cfg, area[i], x[0], x[1]); 2]).collect();
    for &k in &s.state.loose {
        let (cp, a) = (s.sp[k].cp, &w.store.amounts(Class::Ground)[k * n..(k + 1) * n]);
        cells.clone().zip(&mut caps).for_each(|(i, c)| c[1] += a[i] * cp);
    }
    caps
}

/// Ground node `i` takes its heat capacities `[skin, own]` (`capacities`) and its temperature by its energy and the
/// fusion enthalpy it holds (`Scratch::note` keeps it as matter moves). Does it stay off its plateaus, its temperature
/// in the span between the melting points of the last solve (`Latent::span`)?
fn steady(w: &World, s: &mut Scratch, i: usize, [skin, cap]: [f64; 2]) -> bool {
    let (e, l) = (w.store.heat(Class::Ground)[i], s.latent[i]);
    let t = (e - l.held) / cap.max(1.0);
    (s.skin[i], s.cap[i], s.state.tg[i]) = (skin, cap, t);
    s.state.pk[i] == NONE && t >= l.span[0] && t < l.span[1]
}

/// Ground node `i` takes its row of `node_state` (`STATE_ROW`).
pub(super) fn take_row(s: &mut Scratch, i: usize, r: &[f64]) {
    (s.state.tg[i], s.state.pk[i], s.state.phi[i]) = (r[0], r[1] as usize, r[2]);
    s.latent[i] = Latent { held: r[3], span: [r[4], r[5]], lat: [r[6], r[7]], molten: r[8].to_bits() };
    if r[9] != 0.0 {
        (s.z[i], s.head[i], s.cover[i], s.alb[i], s.pd[i]) = (r[10], r[11], [r[12], r[13], r[14]], r[15], r[16]);
        s.shift[i] = 0.0;
    }
}

/// The numbers of a node's row in `node_state`: its temperature, K; its plateau's substance (`NONE` as is) and molten
/// share, and its `Latent` (the bits of what is molten as an f64's bits); whether it has a new surface, then its solid
/// surface and head, m, its soft cover (`Scratch::cover`), its reflectance and its pore space as a depth, m.
pub(super) const STATE_ROW: usize = 17;

/// The row of ground node `i` of heat capacity `cap`, J/K (`STATE_ROW`): with `thaw`, where it stands on its plateaus
/// found anew (`thaw_at`), else as it is; with a new surface where its phases changed, its surface moved (`moved`), or
/// with `full` (`surface_of`).
pub(super) fn node_state(w: &World, s: &Scratch, i: usize, (cap, thaw, full): (f64, bool, bool), row: &mut [f64]) {
    let n = w.mesh.cells().n();
    let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let melt = melts(w, s, amt);
    let l = s.latent[i];
    let (t, pk, phi, l) = if thaw {
        let c = cap.max(1.0);
        let th = thaw_at(s, c, w.store.heat(Class::Ground)[i], amt, &melt);
        let (pk, phi) = th.on.unwrap_or((NONE, s.state.phi[i]));
        // The span off the plateaus, as temperatures: `melting` gives the energies at the points below and above.
        let span = [if th.lo > f64::MIN { (th.lo - th.held) / c } else { f64::NEG_INFINITY }, if th.hi < f64::MAX { (th.hi - th.held) / c } else { f64::INFINITY }];
        (th.t, pk, phi, Latent { held: th.held, span, lat: th.lat, molten: th.bits })
    } else {
        (s.state.tg[i], NONE, s.state.phi[i], l)
    };
    row[..10].copy_from_slice(&[t, pk as f64, phi, l.held, l.span[0], l.span[1], l.lat[0], l.lat[1], f64::from_bits(l.molten), 0.0]);
    if full || s.state.pk[i] != NONE || pk != NONE || l.molten != s.latent[i].molten || moved(w, s, i) {
        let molten = |k: usize| if k == pk { phi } else { (l.molten >> k & 1) as f64 };
        row[9] = 1.0;
        row[10..].copy_from_slice(&surface_of(w, s, i, t, molten, &melt));
    }
}

/// The melting point of each substance that varies in a ground node of amounts `amt(k)`, mol, lowered by what is
/// dissolved (`Chemistry::melt_in`, laws.md 2.5), K, by substance.
pub(super) fn melts(w: &World, s: &Scratch, amt: impl Fn(usize) -> f64) -> [f64; MAX_SUBSTANCES] {
    let (mut melt, solvents) = ([0.0; MAX_SUBSTANCES], w.chem.solvents());
    for &k in &s.state.vary {
        // Only a substance others dissolve in melts below its own point (`Chemistry::lowers_melt`).
        melt[k] = if solvents >> k & 1 == 1 { w.chem.melt_in(k, &amt) } else { s.sp[k].melt };
    }
    melt
}

/// Where a ground node of heat capacity `c`, J/K, energy `e`, J, and amounts `amt(k)`, mol, stands on the melting
/// plateaus of the substances that vary (`ground::melting`), each at its point `melt[k]`, K (`melts`).
pub(super) fn thaw_at(s: &Scratch, c: f64, e: f64, amt: impl Fn(usize) -> f64, melt: &[f64; MAX_SUBSTANCES]) -> Melting {
    // The substances by their melting points here; a substance's number fits a byte (`MAX_SUBSTANCES`).
    let (mut ord, m) = ([0u8; MAX_SUBSTANCES], s.state.vary.len());
    for (o, &k) in ord.iter_mut().zip(&s.state.vary) {
        *o = k as u8;
    }
    ord[..m].sort_by(|&a, &b| melt[a as usize].total_cmp(&melt[b as usize]));
    melting(c, e, ord[..m].iter().map(|&k| (melt[k as usize], k as usize, amt(k as usize) * s.sp[k as usize].lf)))
}

/// Heat conducts sideways through the ground's active layer over `dt` seconds: an explicit sweep on the cells' links,
/// sub-stepped where a cell is too small for the step (`sweep::by_size`; rock passes a 2 km cell's heat to its
/// neighbours in about 70,000 years, so a year is far below the limit there, laws.md 1.8), booked as heat-only link
/// fluxes. A link passes the harmonic mean of its nodes'
/// conductivity × thickness (two half cells in series, Patankar 1980, 4.2) times width over length. A node on a plateau
/// takes its share as if it warmed; the next `ground_state` sets its temperature from its energy.
pub(super) fn conduct(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, cells) = (w.mesh.cells().n(), w.mesh.cells());
    // Conductivity × thickness of each node's active layer, W/K: its bed's skin and its loose ground side by side; none
    // below 0 m (`on_land`), so heat crosses only links between two land cells.
    let on: Vec<bool> = (0..n).map(|i| on_land(s, i)).collect();
    let mut kt: Vec<f64> = s.state.bed.iter().zip(&on).map(|(b, &on)| if on { b.cond * w.cfg.skin_m } else { 0.0 }).collect();
    for &k in &s.state.loose {
        let p = s.sp[k];
        for (i, x) in kt.iter_mut().enumerate() {
            let a = w.store.amounts(Class::Ground)[k * n + i];
            if a > 0.0 && on[i] {
                *x += a * ((1.0 - molten(s, k, i)) * p.v[0] + wet(s, k, i) * p.v[1]) * p.cond / cells.area()[i];
            }
        }
    }
    let (mut a, mut b, mut g) = (Vec::new(), Vec::new(), Vec::new());
    for (l, (&i, &j)) in cells.a().iter().zip(cells.b()).enumerate() {
        let (ki, kj) = (kt[i as usize], kt[j as usize]);
        let gl = 2.0 * ki * kj / (ki + kj).max(f64::MIN_POSITIVE) * cells.width()[l] / cells.dist()[l];
        if gl > 0.0 {
            a.push(i);
            b.push(j);
            g.push(gl);
        }
    }
    let (cap, held) = (s.cap.iter().map(|&c| c.max(1.0)).collect::<Vec<f64>>(), w.store.heat(Class::Ground));
    // (In sub-steps a node's temperature follows the heat it gained, as one on a plateau takes its share here.)
    let again = |now: &[f64], us: &[usize]| ((0..n).map(|i| s.state.tg[i] + (now[i] - held[i]) / cap[i]).collect(), us.iter().map(|&u| g[u]).collect());
    let heat = by_size(&Net { a: &a, b: &b, g: &g, cap: &cap }, &s.state.tg, dt, (held, &[]), &|i, e| heat_above_floor(e, cap[i]), &again).flux;
    apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[]);
}

/// Solid surface, head, soft cover and reflectance of node `i` at temperature `t` with the share `molten(k)` of each
/// substance molten and its melting points `melt` (`melts`), from its layers (`layers`): its solid surface and head, m, its soft cover (`Scratch::cover`), its
/// reflectance and its pore space as a depth, m. The pores of the loose ground hold the first liquid. A soft cover holds
/// none: its pores close as it settles, as snow becomes ice (Cuffey and Paterson 2010), so the liquid on it stands and
/// runs off.
fn surface_of(w: &World, s: &Scratch, i: usize, t: f64, molten: impl Fn(usize) -> f64, melt: &[f64; MAX_SUBSTANCES]) -> [f64; 7] {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let (l, z, [rc, tc]) = layers(w, s, (i, 1.0, 1.0), t, amt, molten, |k| melt[k]);
    [z, z + l[2] / area, rc, tc, l[0] / area, reflectance(w, s, i, &l), w.cfg.phys.porosity * l[4] / area]
}

/// The layers of the share `part` of the area of ground node `i` holding the share `ground` of its bed and its fixed
/// solids (1 and 1: the node; a part of a mixed cell, `parts`) at temperature `t` with amounts `amt(k)`, of which the share `molten(k)`
/// is molten: the volume and volume × reflectance of each of [soft solid, liquid, loose solid] top down, m³; its solid
/// surface, m; and of its soft cover the heat resistance, m² K/W, and the melting point, K: its solids' melting points
/// `melt(k)` (lowered by what is dissolved, `Chemistry::melt_in`), a mean by volume; infinite with no cover.
pub(super) fn layers(w: &World, s: &Scratch, (i, part, ground): (usize, f64, f64), t: f64, amt: impl Fn(usize) -> f64, molten: impl Fn(usize) -> f64, melt: impl Fn(usize) -> f64) -> ([f64; 6], f64, [f64; 2]) {
    let (area, fix) = (w.mesh.cells().area()[i] * part, s.state.fix[i].map(|v| v * ground));
    let mut l = [0.0, 0.0, 0.0, 0.0, fix[0], fix[1]];
    let (mut z, mut cover) = (w.base_z + (s.state.bed[i].vol * ground + fix[0]) / area, [0.0; 2]);
    for &k in &s.state.vary {
        let (p, a) = (&s.sp[k], amt(k));
        if a <= 0.0 || t >= p.boil {
            continue;
        }
        let liq = molten(k);
        let vs = a * (1.0 - liq) * p.v[0];
        let o = if t > p.soft { 0 } else { 4 };
        (l[o], l[o + 1]) = (l[o] + vs, l[o + 1] + vs * p.refl[0]);
        (l[2], l[3]) = (l[2] + a * liq * p.v[1], l[3] + a * liq * p.v[1] * p.refl[1]);
        z += vs / area;
        if o == 0 {
            (cover[0], cover[1]) = (cover[0] + vs / area / p.cond, cover[1] + vs * melt(k));
        }
    }
    cover[1] = if l[0] > 0.0 { cover[1] / l[0] } else { f64::INFINITY };
    (l, z, cover)
}

/// Reflectance of ground node `i` with `layers` `l`, over its own ground: the share of it under the level of what stands
/// over its pores (`curve::Curve::wet`) a column of its layers with that liquid over the share, the rest one whose liquid
/// is what its pores hold (`column`), by area.
pub(super) fn reflectance(w: &World, s: &Scratch, i: usize, l: &[f64; 6]) -> f64 {
    let pores = w.cfg.phys.porosity * l[4];
    let (pore, stands) = (l[2].min(pores), (l[2] - pores).max(0.0));
    let wet = super::curve::of_cell(w, s, i).wet(stands);
    if wet == 0.0 || wet == 1.0 {
        return column(w, s, (i, 1.0), l);
    }
    // The liquid of each part, m³ over the cell's area, with its reflectance by volume.
    let with = |liquid: f64| {
        let mut x = *l;
        (x[2], x[3]) = (liquid, l[3] / l[2] * liquid);
        column(w, s, (i, 1.0), &x)
    };
    (1.0 - wet) * with(pore) + wet * with(pore + stands / wet)
}

/// Reflectance of one column of the share `part` of the area of ground node `i` with `layers` `l`: soft solid, liquid
/// and loose solid, top down, each cover depth / (depth + `hide`) of what lies below, and its bed the rest.
pub(super) fn column(w: &World, s: &Scratch, (i, part): (usize, f64), l: &[f64; 6]) -> f64 {
    let area = w.mesh.cells().area()[i] * part;
    let (mut a, mut seen) = (0.0, 1.0);
    for c in 0..3 {
        if l[2 * c] > 0.0 {
            let d = l[2 * c] / area;
            let cover = d / (d + w.cfg.phys.hide[c]);
            a += seen * cover * l[2 * c + 1] / l[2 * c];
            seen *= 1.0 - cover;
        }
    }
    a + seen * s.state.bed[i].refl
}

/// The level the liquid that stands on node `i` over what its pores hold reaches over its own ground (`curve::Curve`;
/// its lowest ground when none does), m.
#[inline]
pub(super) fn stand(s: &Scratch, i: usize) -> f64 {
    s.z[i] + s.curves[i].level(standing(s, i))
}

/// The share of node `i`'s area that the liquid standing over its pores covers, its own ground under its level
/// (`curve::Curve::wet`), 0..1.
#[inline]
pub(super) fn wet_share(s: &Scratch, i: usize) -> f64 {
    s.curves[i].wet(standing(s, i))
}

/// The volume of the liquid that stands on node `i` over what its pores hold, m³.
#[inline]
fn standing(s: &Scratch, i: usize) -> f64 {
    (s.head[i] - s.pd[i] - s.z[i]).max(0.0) * s.curves[i].area()
}

/// Whether the pores of node `i` (porosity `por`) are full: all its other liquid stands on it. Ground with no pores
/// always is (its head may round a hair under its surface).
#[inline]
pub(super) fn filled(s: &Scratch, i: usize, por: f64) -> bool {
    por <= 0.0 || s.head[i] - s.z[i] >= s.pd[i]
}

/// Height of the bottom of the loose ground of node `i` (porosity `por`), m: the depth its pores take as liquid under
/// its soft cover.
#[inline]
pub(super) fn bottom(s: &Scratch, i: usize, por: f64) -> f64 {
    s.z[i] - s.cover[i][2] - if por > 0.0 { s.pd[i] / por } else { 0.0 }
}

/// Head of the liquid table of node `i`, m: with pores not full, the bottom of its loose ground plus the depth they
/// fill; else the head that stands.
#[inline]
pub(super) fn table(s: &Scratch, i: usize, por: f64) -> f64 {
    if filled(s, i, por) { s.head[i] - s.pd[i] } else { bottom(s, i, por) + (s.head[i] - s.z[i]) / por }
}

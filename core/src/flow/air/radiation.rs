//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact
//! heat between the ground and the air touching it.

use super::super::land::NONE;
use super::super::Scratch;
use super::air_temp;
use crate::life::Life;
use crate::math::one_minus_exp;
use crate::store::{apply, Boundary, Class, LinkFlux};
use crate::units::SIGMA;
use crate::{Config, World, AIR};
use std::f64::consts::{PI, TAU};

/// Daily-mean sunlight at the top of the air, W/m².
fn insolation(s0: f64, lat: f64, decl: f64) -> f64 {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0)
}

/// Layers of the air over a cell in `column`.
pub(crate) const LAYERS: usize = 4;

/// The layers of the air over a cell that send heat radiation [down to the ground, up to space],
/// as how much colder than the air touching the ground each is, K: the grey absorber of optical
/// depth `x` lies like the air's mass (scale height `h`, in a column `l` deep), and the air's
/// temperature falls by `gam` per metre. Each layer sends that side the same share of what the
/// column sends it, from its height (Schwarzschild's integral, summed at `LAYERS` points: see
/// `glow`). More gas lifts the layers that reach space into colder air, so less leaves and the
/// ground warms, with no ceiling (one slab at the mean temperature has one: 2^(1/4) times the bare ground's).
pub fn column(x: f64, gam: f64, l: f64, h: f64) -> [[f64; LAYERS]; 2] {
    let (f, e, mut out) = (one_minus_exp(x), (-l / h).exp(), [[0.0; LAYERS]; 2]);
    // Drop to the height with a share `above` of the absorber above it.
    let drop = |above: f64| gam * h * (e + (1.0 - e) * above).ln();
    for m in (0..LAYERS).filter(|_| x > 0.0) {
        // Share of the absorber between the layer and the side it sends to.
        let near = -(-f * (m as f64 + 0.5) / LAYERS as f64).ln_1p() / x;
        (out[0][m], out[1][m]) = (drop(1.0 - near), drop(near));
    }
    out
}

/// What layers (`column`) over ground with air at `tl` that absorbs a share `f` send, W/m².
pub fn glow(f: f64, tl: f64, lay: &[f64; LAYERS]) -> f64 {
    lay.iter().map(|&d| (tl + d).max(1.0).powi(4)).sum::<f64>() * SIGMA * f / LAYERS as f64
}

/// What the radiation of one ground cell reads of the world.
struct Ground<'a> {
    cfg: &'a Config,
    /// Area (m²), latitude (degrees) and air node of each cell, from the mesh.
    area: &'a [f64],
    lat: &'a [f64],
    air_of: &'a [u32],
    /// Declination of the sun, radians.
    decl: f64,
    ground: &'a [f64],
    canopy: &'a [f32],
    life: &'a Life,
    dt: f64,
}

/// The numbers of one cell's radiation, in a row (see `cell`).
const ROW: usize = 6;

/// Sunlight by latitude and season, reflected by what covers the ground; the ground emits
/// σT⁴, of which the air absorbs its share and emits by its layers (`column`); and
/// contact heat with the air touching the ground (warmer than the air's mean by the drop
/// from the air's mean height). The ground temperature is solved implicitly. The store books it
/// in one call (`store::apply`), as link fluxes: the sunlight each ground node absorbs, from the sun
/// node; what it gives its air node (what that absorbs of it, less what it sends down, and what
/// leaves to space through it); and what leaves each air node to the space node.
pub(crate) fn radiation(w: &mut World, s: &mut Scratch, dt: f64) {
    let season = w.season();
    let World { cfg, mesh, ground, canopy, light, light_pool, life, .. } = w;
    let (n, cells) = (mesh.cells().n(), mesh.cells());
    light_pool.resize(n, 0.0);
    let decl = -cfg.tilt.to_radians() * (TAU * season).cos();
    let mut kept_all = 0.0;
    // Each cell's numbers depend on no other cell: computed in rows (in parallel, with `threads`),
    // applied in the order of the cells.
    let mut rows = std::mem::take(&mut s.rad);
    rows.resize(n * ROW, 0.0);
    let (sc, at) = (&*s, Ground { cfg, area: cells.area(), lat: cells.lat(), air_of: cells.parent(), decl, ground, canopy, life, dt });
    // Each task keeps the last latitude and its sunlight: neighbouring cells often share a latitude (a row of the
    // square map), and the sunlight is the same function of it.
    crate::par::rows(&mut rows, ROW, || (f64::NAN, 0.0), |last, i, row| {
        if at.lat[i] != last.0 {
            *last = (at.lat[i], insolation(at.cfg.sun, at.lat[i].to_radians(), at.decl));
        }
        *<&mut [f64; ROW]>::try_from(row).unwrap() = cell(&at, sc, i, last.1);
    });
    // The links: from the sun to each ground node, from each ground node to its air node, and from each air node to
    // space (`ids`: the nodes 0, 1, ... of the cells, and of the fewer air nodes).
    let (na, mut heat, mut out) = (mesh.level(AIR).n(), vec![0.0; 2 * n], vec![0.0; mesh.level(AIR).n()]);
    let (sun, ids) = (vec![Boundary::Sun as u32; n], (0..n as u32).collect::<Vec<_>>());
    for (i, row) in rows.chunks_exact(ROW).enumerate() {
        let &[tl, dl, kept, sw, to_air, to_space] = <&[f64; ROW]>::try_from(row).unwrap();
        s.tl[i] = tl;
        let adt = cells.area()[i] * dt;
        if canopy[i] as f64 > 0.0 {
            light[i] += dl;
            for &b in &life.at[i] {
                life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&f| f) - 1.0) * dl;
            }
        }
        light_pool[i] += adt * kept;
        kept_all += adt * kept;
        (heat[i], heat[n + i]) = (adt * sw, to_air + to_space);
        out[cells.parent()[i] as usize] += to_space;
    }
    s.rad = rows;
    // The light set aside for bodies is no heat of a node yet: it crosses from the sun as it falls.
    w.cross(Boundary::Sun, [], kept_all);
    let (space, air_of) = (vec![Boundary::Space as u32; na], w.mesh.cells().parent().to_vec());
    let fluxes = [
        LinkFlux { class: [Class::Boundary, Class::Ground], a: &sun, b: &ids, ks: &[], mol: &[], heat: &heat[..n] },
        LinkFlux { class: [Class::Ground, Class::Air], a: &ids, b: &air_of, ks: &[], mol: &[], heat: &heat[n..] },
        LinkFlux { class: [Class::Air, Class::Boundary], a: &ids[..na], b: &space, ks: &[], mol: &[], heat: &out },
    ];
    apply(w, &fluxes, &[], &[]);
    for j in 0..w.mesh.level(AIR).n() {
        s.ta[j] = air_temp(w.e_air[j], s.capa[j]);
    }
}

/// The radiation of ground cell `i` under the sunlight `q` of its latitude (W/m²) over the step: [temperature of the
/// air touching the ground, K; light each unit of capture area of its bodies gets, J/m²; light set
/// aside for them, W/m²; shortwave absorbed, W/m²; heat given to its air node, J; heat sent to
/// space, J].
fn cell(at: &Ground, s: &Scratch, i: usize, q: f64) -> [f64; ROW] {
    let (cfg, n, dt) = (at.cfg, at.area.len(), at.dt);
    let adt = at.area[i] * dt;
    // The air this cell sees, and its temperature where it touches the ground.
    let ([gam, zair, f, ta], c, t, j) = (s.seen[i], s.state.cloud[i], s.state.tg[i], at.air_of[i] as usize);
    let ta = ta + s.ta[j] - s.ta0[j];
    let tl = (ta - gam * (s.head[i] - zair)).max(1.0);
    let (down, emit) = (glow(f, tl, &s.lay[i][0]), glow(f, tl, &s.lay[i][1]));
    // Bodies with light-capture area `a` per ground area intercept a share 1 - exp(-a) of what
    // passes the clouds. What they absorb warms the surface now but for the share their light
    // reactions store (as at their last update, `Life::keep`): that is set aside, and each takes
    // its share when it updates (`body::live`). Taller members shade smaller ones (`survey::shade`).
    let (a, mut cover, mut caught, mut kept, mut dl) = (at.canopy[i] as f64, 0.0, 0.0, 0.0, 0.0);
    if a > 0.0 {
        cover = one_minus_exp(a);
        dl = q * (1.0 - c) * cover * dt / a;
        caught = cover * at.life.absorb.get(i).map_or(0.0, |&x| x as f64);
        kept = q * (1.0 - c) * caught * at.life.keep.get(i).map_or(0.0, |&x| x as f64);
    }
    let sw = q * (1.0 - c) * ((1.0 - cover) * (1.0 - s.alb[i]) + caught) - kept;
    // The surface temperature, implicit: heat reaches the surface from the node through its heat
    // capacity over the step (none on a melting plateau) in series with any soft solid cover.
    let t3 = SIGMA * t * t * t;
    let surface = |r: f64| if r > 0.0 { (t / r + sw + down + 3.0 * t3 * t + cfg.sensible * tl) / (1.0 / r + 4.0 * t3 + cfg.sensible) } else { t };
    let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), cfg.sensible * (ts - tl));
    let mut ts = surface(if s.state.pk[i] != NONE { 0.0 } else { adt / s.cap[i] } + s.rc[i]);
    // A plateau holds the node only while its fusion enthalpy lasts: a step that would freeze
    // (or melt) more than the node holds takes it off, and its heat capacity holds it back.
    if s.state.pk[i] != NONE {
        let ((up, hs), lat) = (flux(ts), at.ground[s.state.pk[i] * n + i] * s.sp[s.state.pk[i]].lf);
        let net = adt * (sw + down - up - hs);
        if net < -s.state.phi[i] * lat || net > (1.0 - s.state.phi[i]) * lat {
            ts = surface(adt / s.cap[i] + s.rc[i]);
        }
    }
    let (up, hs) = flux(ts);
    [tl, dl, kept, sw, adt * (f * up + hs - down - emit), adt * ((1.0 - f) * up + emit)]
}


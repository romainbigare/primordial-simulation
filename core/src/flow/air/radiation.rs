use super::super::land::NONE;
use super::super::Scratch;
use crate::math::one_minus_exp;
use crate::units::SIGMA;
use crate::World;
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

/// Sunlight by latitude and season, reflected by what covers the ground; the ground emits
/// σT⁴, of which the air absorbs its share and emits by its layers (`column`); and
/// contact heat with the air touching the ground (warmer than the air's mean by the drop
/// from the air's mean height). The ground temperature is solved implicitly. All the air
/// absorbs and emits is booked to its node.
pub(crate) fn radiation(w: &mut World, s: &mut Scratch, dt: f64) {
    let season = w.season();
    let World { cfg, grid, ground, e_ground, e_air, ledger, canopy, light, light_pool, life, .. } = w;
    light_pool.resize(grid.n(), 0.0);
    let (n, na, adt) = (grid.n(), grid.na(), s.area * dt);
    let decl = -cfg.tilt.to_radians() * (TAU * season).cos();
    let (mut sun, mut out) = (0.0, 0.0);
    s.node_tmp.fill(0.0);
    let suns: Vec<f64> = (0..grid.h).map(|y| insolation(cfg.sun, grid.lat(y, cfg.lat_south, cfg.lat_north).to_radians(), decl)).collect();
    // Each cell's numbers (they depend on no other cell) in a row: [air temperature at the ground,
    // light it gets, light set aside, shortwave in, heat gained, heat given to its air node, heat
    // sent to space, heat it could not pay], applied in the order of the cells.
    let mut rows = std::mem::take(&mut s.rad);
    rows.resize(n * 8, 0.0);
    let (sc, e_ground_now, canopy_now) = (&*s, &*e_ground, &*canopy);
    crate::par::rows(&mut rows, 8, || (), |_, i, row| {
        let q = suns[i / grid.w];
        // The air this cell sees, and its temperature where it touches the ground.
        let ([gam, zair, f, c, ta], t, j) = (sc.seen[i], sc.tg[i], sc.air_of[i] as usize);
        let ta = ta + sc.ta[j] - sc.ta0[j];
        let tl = (ta - gam * (sc.head[i] - zair)).max(1.0);
        let (down, emit) = (glow(f, tl, &sc.lay[i][0]), glow(f, tl, &sc.lay[i][1]));
        // Bodies with light-capture area `a` per ground area intercept a share 1 - exp(-a) of what
        // passes the clouds. What they absorb warms the surface now but for the share their light
        // reactions store (as at their last update, `Life::keep`): that is set aside, and each takes
        // its share when it updates (`body::live`). Taller members shade smaller ones (`body::shade`).
        let (a, mut cover, mut caught, mut kept, mut dl) = (canopy_now[i] as f64, 0.0, 0.0, 0.0, 0.0);
        if a > 0.0 {
            cover = one_minus_exp(a);
            dl = q * (1.0 - c) * cover * dt / a;
            caught = cover * life.absorb.get(i).map_or(0.0, |&x| x as f64);
            kept = q * (1.0 - c) * caught * life.keep.get(i).map_or(0.0, |&x| x as f64);
        }
        let sw = q * (1.0 - c) * ((1.0 - cover) * (1.0 - sc.alb[i]) + caught) - kept;
        // The surface temperature, implicit: heat reaches the surface from the node through its heat
        // capacity over the step (none on a melting plateau) in series with any soft solid cover.
        let t3 = SIGMA * t * t * t;
        let surface = |r: f64| if r > 0.0 { (t / r + sw + down + 3.0 * t3 * t + cfg.sensible * tl) / (1.0 / r + 4.0 * t3 + cfg.sensible) } else { t };
        let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), cfg.sensible * (ts - tl));
        let mut ts = surface(if sc.pk[i] != NONE { 0.0 } else { adt / sc.cap[i] } + sc.rc[i]);
        // A plateau holds the node only while its fusion enthalpy lasts: a step that would freeze
        // (or melt) more than the node holds takes it off, and its heat capacity holds it back.
        if sc.pk[i] != NONE {
            let ((up, hs), lat) = (flux(ts), ground[sc.pk[i] * n + i] * sc.sp[sc.pk[i]].lf);
            let net = adt * (sw + down - up - hs);
            if net < -sc.phi[i] * lat || net > (1.0 - sc.phi[i]) * lat {
                ts = surface(adt / sc.cap[i] + sc.rc[i]);
            }
        }
        let (mut up, mut hs) = flux(ts);
        // A node pays only what it holds above `T_FLOOR`; past that it emits and conducts less (as `take_heat`).
        let (room, lose, mut clamped) = (crate::ground::heat_above_floor(e_ground_now[i], sc.cap[i]), adt * (up + hs), 0.0);
        if lose > room + adt * (sw + down) {
            let lam = (room + adt * (sw + down)) / lose;
            clamped = lose * (1.0 - lam);
            (up, hs) = (up * lam, hs * lam);
        }
        *<&mut [f64; 8]>::try_from(row).unwrap() = [tl, dl, kept, sw, adt * (sw + down - up - hs), adt * (f * up + hs - down - emit), adt * ((1.0 - f) * up + emit), clamped];
    });
    for (i, row) in rows.chunks_exact(8).enumerate() {
        let &[tl, dl, kept, sw, gain, to_air, to_space, clamped] = <&[f64; 8]>::try_from(row).unwrap();
        s.tl[i] = tl;
        if canopy[i] as f64 > 0.0 {
            light[i] += dl;
            for &b in &life.at[i] {
                life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&f| f) - 1.0) * dl;
            }
        }
        light_pool[i] += adt * kept;
        sun += adt * kept;
        if clamped != 0.0 {
            ledger.clamped_heat += clamped;
        }
        e_ground[i] += gain;
        s.node_tmp[s.air_of[i] as usize] += to_air;
        sun += adt * sw;
        out += to_space;
    }
    s.rad = rows;
    for (j, e) in e_air.iter_mut().enumerate().take(na) {
        *e += s.node_tmp[j];
        s.ta[j] = *e / s.capa[j];
    }
    ledger.sun_in += sun;
    ledger.space_out += out;
}

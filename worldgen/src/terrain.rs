//! Terrain: an island with mountains, and the starting state of ground, low liquid and air.

use crate::chemistry::{chemistry0, Init};
use pcore::chem::{Chemistry, Phase};
use pcore::grid::Grid;
use pcore::rng::{Purpose, Rng};
use pcore::{Config, Diag, Ledger, World};

fn hash01(seed: u64, x: i64, y: i64, octave: u64) -> f64 {
    Rng::new(seed, octave, ((x as u64) << 32) ^ (y as u64 & 0xffff_ffff), Purpose::Terrain).f64()
}

/// Smooth value noise in [0, 1).
fn value_noise(seed: u64, x: f64, y: f64, octave: u64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let (ix, iy) = (x0 as i64, y0 as i64);
    let a = hash01(seed, ix, iy, octave);
    let b = hash01(seed, ix + 1, iy, octave);
    let c = hash01(seed, ix, iy + 1, octave);
    let d = hash01(seed, ix + 1, iy + 1, octave);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

/// Fractal noise in roughly [0, 1).
fn fbm(seed: u64, x: f64, y: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed, x * freq, y * freq, o as u64);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Height map in metres (zero is the flat reference level).
pub fn heights(seed: u64, w: usize, h: usize) -> Vec<f64> {
    let mut out = vec![0.0; w * h];
    let scale = 6.0 / w.max(h) as f64;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f64 * scale, y as f64 * scale);
            // Warp the domain so coasts and ridges bend.
            let wx = fx + 1.5 * (fbm(seed ^ 11, fx * 0.5, fy * 0.5, 3) - 0.5);
            let wy = fy + 1.5 * (fbm(seed ^ 13, fx * 0.5, fy * 0.5, 3) - 0.5);
            let base = fbm(seed, wx, wy, 6);
            let ridge = 1.0 - (2.0 * fbm(seed ^ 7, wx * 0.7, wy * 0.7, 5) - 1.0).abs();
            let dx = (x as f64 + 0.5) / w as f64 - 0.5;
            let dy = (y as f64 + 0.5) / h as f64 - 0.5;
            let d = (dx * dx + dy * dy).sqrt() * 2.0;
            let land = base * 0.9 + ridge.powi(3) * 0.6 - d.powf(1.6) * 1.0;
            out[y * w + x] = if land > 0.3 {
                (land - 0.3).powf(1.3) * 3600.0
            } else {
                (land - 0.3) * 1500.0
            }
            .max(-800.0);
        }
    }
    out
}

/// A new world from a seed: terrain, chemistry 0, ground, liquid and air.
pub fn new_world(seed: u64, w: usize, h: usize) -> World {
    let (chem, init) = chemistry0();
    build(seed, w, h, chem, &init, Config::default())
}

pub fn build(seed: u64, w: usize, h: usize, chem: Chemistry, init: &Init, cfg: Config) -> World {
    let grid = Grid { w, h, cell_m: 2000.0, air_scale: 4 };
    let (n, na, ns) = (grid.n(), grid.na(), chem.ns());
    let area = grid.area();
    let hz = heights(seed, w, h);
    let base_z = -3000.0;
    let loose_m = 1.0;
    let mut ground = vec![0.0; ns * n];
    let mut bed = vec![0.0; ns * n];
    let mv = |k: usize| chem.molar_volume(k, Phase::Solid);

    for i in 0..n {
        let (x, y) = grid.xy(i);
        let z = hz[i];
        // Patchy deposits, more common in high ground.
        let mut fr: Vec<(usize, f64)> = init.bed.clone();
        for (j, &(k, f)) in init.deposits.iter().enumerate() {
            let p = fbm(seed ^ (100 + j as u64), x as f64 * 0.15, y as f64 * 0.15, 3);
            let lift = (z / 1500.0).clamp(0.0, 1.0);
            let amt = f * ((p - 0.55) * 12.0).clamp(0.0, 3.0) * (0.3 + lift);
            fr.push((k, amt));
        }
        let tot: f64 = fr.iter().map(|p| p.1).sum();
        let bed_v = (z - loose_m - base_z) * area;
        for &(k, f) in &fr {
            bed[k * n + i] += f / tot * bed_v / mv(k);
        }
        let ltot: f64 = init.loose.iter().map(|p| p.1).sum();
        for &(k, f) in &init.loose {
            ground[k * n + i] += f / ltot * loose_m * area / mv(k);
        }
        if z < 0.0 {
            ground[init.low * n + i] += -z * area / chem.molar_volume(init.low, Phase::Liquid);
        }
    }

    // Air: a column per air node, thinner over high ground.
    let mut air = vec![0.0; ns * na];
    let mut zmean = vec![0.0; na];
    let mut cnt = vec![0.0; na];
    for i in 0..n {
        zmean[grid.air_of(i)] += hz[i].max(0.0);
        cnt[grid.air_of(i)] += 1.0;
    }
    let atot: f64 = init.air.iter().map(|p| p.1).sum();
    let m_air: f64 = init.air.iter().map(|&(k, f)| f / atot * chem.subs[k].molar_mass).sum();
    for j in 0..na {
        zmean[j] /= cnt[j];
        let col = 101_325.0 / (cfg.gravity * m_air) * (-zmean[j] / 8000.0).exp() * area * cnt[j];
        for &(k, f) in &init.air {
            air[k * na + j] = f / atot * col;
        }
    }

    let mut world = World {
        cfg,
        chem,
        grid,
        seed,
        day: 0,
        ground,
        bed,
        e_ground: vec![0.0; n],
        air,
        e_air: vec![0.0; na],
        base_z,
        diag: Diag { liquid_flux: vec![0.0; n], rain: vec![0.0; n] },
        ledger: Ledger::default(),
        events: Vec::new(),
        scratch: Default::default(),
    };
    world.ledger.imposed_elems = vec![0.0; world.chem.n_elements];

    // Starting temperatures: warm low latitudes, cooler poles and heights.
    let t_at = |w: &World, y: usize, z: f64| {
        let lat = w.grid.lat(y, w.cfg.lat_south, w.cfg.lat_north).to_radians();
        302.0 - 45.0 * lat.sin().powi(2) - 0.0065 * z.max(0.0)
    };
    for i in 0..n {
        let (_, y) = world.grid.xy(i);
        let t = t_at(&world, y, hz[i]);
        world.e_ground[i] = world.heat_cap_ground(i) * t;
    }
    for j in 0..na {
        let y = (j / world.grid.aw()) * world.grid.air_scale + world.grid.air_scale / 2;
        let t = t_at(&world, y.min(h - 1), zmean[j]);
        world.e_air[j] = world.heat_cap_air(j) * t;
    }
    world
}

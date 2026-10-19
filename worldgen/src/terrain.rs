//! Terrain: an island with mountains, and the starting state of ground, low liquid and air.

use crate::chemistry::{chemistry0, Init};
use pcore::change::Reactor;
use pcore::chem::{Chemistry, Phase};
use pcore::flow::scale_height;
use pcore::rng::{Purpose, Rng};
use pcore::space::square::Square;
use pcore::space::Mesh;
use pcore::store::{Boundary, Class, Links, Node, Store};
use pcore::units::P_REF;
use pcore::{Config, Counters, World, AIR};

/// Cell size, m, and ground cells per air node along a side.
const CELL_M: f64 = 2000.0;
const AIR_SCALE: usize = 4;
/// Height of the bottom of the bed, m.
const BASE_Z: f64 = -3000.0;
/// Depth of the loose ground every cell starts with, m.
pub const LOOSE_M: f64 = 1.0;
/// Starting temperatures, K: at the equator at 0 m, less `POLE_DROP` x sin² latitude, less `LAPSE` K per m of height.
const T_EQUATOR: f64 = 302.0;
const POLE_DROP: f64 = 45.0;
const LAPSE: f64 = 0.0065;
/// Low ground starts this many K above the melting point of its liquid, so it is molten.
const MOLTEN_MARGIN: f64 = 2.0;

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

/// Air over a m² at 0 m, mol: the reference pressure over the weight of a mol of the starting air (`Init::air`).
pub fn air_column(chem: &Chemistry, init: &Init, gravity: f64) -> f64 {
    let atot: f64 = init.air.iter().map(|p| p.1).sum();
    let m_air: f64 = init.air.iter().map(|&(k, f)| f / atot * chem.subs[k].molar_mass).sum();
    P_REF / (gravity * m_air)
}

/// Starting temperature at row `y` of `map` and height `z`, K: warm low latitudes, cooler poles and heights.
fn t_start((map, mesh): (&Square, &Mesh), y: usize, z: f64) -> f64 {
    let lat = mesh.cells().lat()[map.idx(0, y)].to_radians();
    T_EQUATOR - POLE_DROP * lat.sin().powi(2) - LAPSE * z.max(0.0)
}

/// Scale the heat-absorbing gases of the air by `factor` (`ew run --absorber`), each air node keeping its temperature:
/// the gas added comes from outside the world (an edit) with its heat at that temperature and its latent heat.
pub fn scale_absorbers(w: &mut World, factor: f64) {
    for j in 0..w.mesh.level(AIR).n() {
        let t = w.t_air(j);
        let more: Vec<(usize, f64)> = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0).map(|k| (k, w.a(k, j) * (factor - 1.0))).collect();
        let heat = more.iter().map(|&(k, m)| m * (w.chem.subs[k].cp * t + w.chem.subs[k].latent_gas())).sum();
        w.cross((Boundary::Outside, Node::air(j)), more, heat);
    }
}

/// A new world from a seed: terrain, chemistry 0, ground, liquid and air.
pub fn new_world(seed: u64, w: usize, h: usize) -> World {
    let (chem, init) = chemistry0();
    build(seed, w, h, chem, &init, Config::default())
}

pub fn build(seed: u64, w: usize, h: usize, chem: Chemistry, init: &Init, cfg: Config) -> World {
    let map = Square { w, h, side: CELL_M };
    let mesh = World::mesh_of(&map, AIR_SCALE, &cfg);
    let (n, na, ns) = (mesh.cells().n(), mesh.level(AIR).n(), chem.ns());
    let area = map.area();
    let hz = heights(seed, w, h);
    let mut ground = vec![0.0; ns * n];
    let mut bed = vec![0.0; ns * n];
    let mv = |k: usize| chem.molar_volume(k, Phase::Solid);

    for i in 0..n {
        let (x, y) = map.xy(i);
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
        let bed_v = (z - LOOSE_M - BASE_Z) * area;
        for &(k, f) in &fr {
            bed[k * n + i] += f / tot * bed_v / mv(k);
        }
        let ltot: f64 = init.loose.iter().map(|p| p.1).sum();
        for &(k, f) in &init.loose {
            ground[k * n + i] += f / ltot * LOOSE_M * area / mv(k);
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
        zmean[mesh.up(i, 0, AIR)] += hz[i].max(0.0);
        cnt[mesh.up(i, 0, AIR)] += 1.0;
    }
    // The column over high ground is thinner by exp(-z / H), H the scale height of the air at the node's starting
    // temperature (`flow::scale_height`, as Flow's law takes it).
    let (atot, column): (f64, f64) = (init.air.iter().map(|p| p.1).sum(), air_column(&chem, init, cfg.gravity));
    let m_air: f64 = init.air.iter().map(|&(k, f)| f * chem.subs[k].molar_mass).sum();
    let mut t_air = vec![0.0; na];
    for j in 0..na {
        zmean[j] /= cnt[j];
        let y = (j / w.div_ceil(AIR_SCALE)) * AIR_SCALE + AIR_SCALE / 2;
        t_air[j] = t_start((&map, &mesh), y.min(h - 1), zmean[j]);
        let col = column * (-zmean[j] / scale_height(t_air[j], atot, m_air, cfg.gravity)).exp() * area * cnt[j];
        for &(k, f) in &init.air {
            air[k * na + j] = f / atot * col;
        }
    }

    let links = Links::new(mesh.cells().a().len());
    let mut world = World {
        cfg,
        chem,
        map,
        air_scale: AIR_SCALE,
        mesh,
        seed,
        time: 0,
        sleep: Default::default(),
        store: Store::new(n, (ground, bed, vec![0.0; n]), (air, vec![0.0; na])),
        base_z: BASE_Z,
        counts: Counters::default(),
        rain: vec![0.0; n],
        liquid_flux: vec![0.0; n],
        life: Default::default(),
        parts: Default::default(),
        canopy: vec![0.0; n],
        links,
        light: vec![0.0; n],
        rained: vec![0.0; n],
        events: Vec::new(),
        scratch: Default::default(),
        batches: Default::default(),
        moves: Default::default(),
    };
    // The ground and the air at their starting temperatures. (The world is made: its store is built whole at each stage,
    // never written outside `store::apply` once it runs.)
    let (amounts, heat) = (|w: &World, c: Class| w.store.amounts(c).to_vec(), |w: &World, c: Class| w.store.heat(c).to_vec());
    let mut e_ground = vec![0.0; n];
    for (i, &z) in hz.iter().enumerate().take(n) {
        let (_, y) = world.map.xy(i);
        // Low ground starts with its liquid molten.
        let t = t_start((&world.map, &world.mesh), y, z).max(if z < 0.0 { world.chem.subs[init.low].melt + MOLTEN_MARGIN } else { 0.0 });
        e_ground[i] = world.energy_ground(i, t);
    }
    let e_air = t_air.iter().enumerate().map(|(j, &t)| world.heat_cap_air(j) * t).collect();
    world.store = Store::new(n, (amounts(&world, Class::Ground), amounts(&world, Class::Bed), e_ground), (amounts(&world, Class::Air), e_air));
    // The liquid starts at balance with the air: of each pair of reactions that undo each other, the
    // substance the first makes is set, in each cell, to the amount at which the pair goes at equal
    // rates at the cell's temperature and the air's start (`Reactor::balance`: from the chemistry's
    // own rate constants and barriers). A cell with no liquid holds none. The air stays as it is.
    // (The node takes the heat of its new amounts at its temperature: worked out once they are in the store.)
    let mut rc = Reactor::new(&world.chem, &world.cfg);
    let (mut ground, mut balanced) = (amounts(&world, Class::Ground), Vec::new());
    for i in 0..n {
        let (t, j, share) = (world.t_ground(i), world.mesh.up(i, 0, AIR), world.mesh.share(i, AIR));
        let mut amt: Vec<f64> = (0..ns).map(|k| world.g(k, i)).collect();
        let gas: Vec<f64> = (0..ns).map(|k| world.a(k, j) * share).collect();
        if !rc.balance(&world.chem, &mut amt, &gas, t).is_empty() {
            for (k, &a) in amt.iter().enumerate() {
                ground[k * n + i] = a;
            }
            balanced.push((i, t));
        }
    }
    let rebuild = |w: &World, ground: Vec<f64>, e_ground: Vec<f64>| Store::new(n, (ground, amounts(w, Class::Bed), e_ground), (amounts(w, Class::Air), heat(w, Class::Air)));
    world.store = rebuild(&world, ground, heat(&world, Class::Ground));
    let mut e_ground = heat(&world, Class::Ground);
    balanced.iter().for_each(|&(i, t)| e_ground[i] = world.energy_ground(i, t));
    world.store = rebuild(&world, amounts(&world, Class::Ground), e_ground);
    world
}

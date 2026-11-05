//! Terrain: an island or a globe with mountains, and the starting state of ground, low liquid and air.

use crate::chemistry::{chemistry0, Init};
use crate::drain::drain;

use pcore::change::Reactor;
use pcore::chem::{Chemistry, Phase};
use pcore::flow::scale_height;
use pcore::life::Life;
use pcore::rng::{Purpose, Rng};
use pcore::space::square::Square;
use pcore::space::{Level, Mesh};
use pcore::store::{Boundary, Class, Links, Node, Store};
use pcore::units::P_REF;
use pcore::{Config, Counters, World, AIR};
use std::f64::consts::{PI, TAU};

/// Cell size of an island, m, and ground cells per air node along a side.
const CELL_M: f64 = 2000.0;
const AIR_SCALE: usize = 4;
/// The squares below 0 m round the land that stay squares on an island (`coarse_low`), so its edges stay sharp.
const LAND_RING: i32 = 1;
/// A globe's radius, m, and the share of its area above the level 0 m: those of the one planet known (Earth: mean
/// radius 6,371 km, land 29% of the surface).
const RADIUS_M: f64 = 6.371e6;
const LAND_SHARE: f64 = 0.29;
/// Units of the coarsest noise across a map's longer side (a globe's equator).
const ACROSS: f64 = 6.0;
/// The keys mixed into the seed of the noise that warps the terrain along each axis.
const WARP: [u64; 3] = [11, 13, 17];
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

/// A draw in [0, 1) for the corner `at` of the noise's lattice (2 or 3 dimensions).
fn hash01(seed: u64, at: &[i64], octave: u64) -> f64 {
    let bits = 64 / at.len() as u32;
    let key = at.iter().fold(0u64, |k, &c| (k << bits) ^ (c as u64 & (u64::MAX >> (64 - bits))));
    Rng::new(seed, octave, key, Purpose::Terrain).f64()
}

/// Smooth value noise in [0, 1) at point `p`: the draws of the corners of its lattice cell, eased in along each axis.
fn value_noise(seed: u64, p: &[f64], octave: u64) -> f64 {
    let lo: Vec<f64> = p.iter().map(|v| v.floor()).collect();
    let ease = |t: f64| t * t * (3.0 - 2.0 * t);
    // Corner `c` lies at `lo` plus bit `d` of `c` along axis `d`; each axis in turn folds the corners in pairs.
    let mut v: Vec<f64> = (0..1usize << p.len())
        .map(|c| hash01(seed, &lo.iter().enumerate().map(|(d, &l)| l as i64 + (c >> d & 1) as i64).collect::<Vec<i64>>(), octave))
        .collect();
    for (&x, &l) in p.iter().zip(&lo) {
        let t = ease(x - l);
        v = v.chunks(2).map(|ab| ab[0] + (ab[1] - ab[0]) * t).collect();
    }
    v[0]
}

/// Fractal noise in roughly [0, 1).
fn fbm(seed: u64, p: &[f64], octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed, &p.iter().map(|v| v * freq).collect::<Vec<f64>>(), o as u64);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Where cell `i` of `map` lies in the noise at `per_cell` units a square: on a flat map its centre, in squares, less
/// half a square (a square's column and row); on a globe the point over its centre on a sphere `per_cell` units a
/// square round its equator (no seam where the rows wrap).
fn spot(map: &Square, cells: &Level, i: usize, per_cell: f64) -> Vec<f64> {
    if !map.wrap {
        return cells.centre()[i].iter().map(|&c| (c / map.side - 0.5) * per_cell).collect();
    }
    let (r, lat, lon) = (per_cell * map.w as f64 / TAU, cells.lat()[i].to_radians(), cells.lon()[i].to_radians());
    vec![r * lat.cos() * lon.cos(), r * lat.cos() * lon.sin(), r * lat.sin()]
}

/// The rise of the ground at point `p` of the noise: fractal noise with ridges, its domain warped so coasts and ridges
/// bend.
fn relief(seed: u64, p: &[f64]) -> f64 {
    let half: Vec<f64> = p.iter().map(|v| v * 0.5).collect();
    let warp: Vec<f64> = p.iter().zip(WARP).map(|(&v, key)| v + 1.5 * (fbm(seed ^ key, &half, 3) - 0.5)).collect();
    let base = fbm(seed, &warp, 6);
    let ridge = 1.0 - (2.0 * fbm(seed ^ 7, &warp.iter().map(|v| v * 0.7).collect::<Vec<f64>>(), 5) - 1.0).abs();
    base * 0.9 + ridge.powi(3) * 0.6
}

/// Height of ground whose rise lies `above` the level, m: land climbs steeply, low ground falls gently.
fn height(above: f64) -> f64 {
    if above > 0.0 { above.powf(1.3) * 3600.0 } else { above * 1500.0 }.max(-800.0)
}

/// Height of each cell of `cells` on `map`, m (zero is the reference level). An island's rise falls away from the
/// middle of the map to a level of 0.3; a globe's level leaves the share `LAND_SHARE` of its area above it.
fn heights(seed: u64, map: &Square, cells: &Level) -> Vec<f64> {
    let per_cell = ACROSS / map.w.max(map.h) as f64;
    let rise: Vec<f64> = (0..cells.n()).map(|i| relief(seed, &spot(map, cells, i, per_cell))).collect();
    if !map.wrap {
        // How far the centre `c`, m, lies from the middle of a side of `len` squares, in sides.
        let middle = |c: f64, len: usize| c / map.side / len as f64 - 0.5;
        return (rise.iter().enumerate())
            .map(|(i, &r)| {
                let (dx, dy) = (middle(cells.centre()[i][0], map.w), middle(cells.centre()[i][1], map.h));
                height(r - ((dx * dx + dy * dy).sqrt() * 2.0).powf(1.6) * 1.0 - 0.3)
            })
            .collect();
    }
    let mut order: Vec<usize> = (0..cells.n()).collect();
    order.sort_by(|&a, &b| rise[b].total_cmp(&rise[a]));
    let (total, mut above) = (cells.area().iter().sum::<f64>(), 0.0);
    let level = order.iter().find(|&&i| {
        above += cells.area()[i];
        above >= LAND_SHARE * total
    });
    let level = level.map_or(0.0, |&i| rise[i]);
    rise.iter().map(|&r| height(r - level)).collect()
}

/// Height of each cell of `mesh` on `map` (its rows between the latitudes of `cfg`), m: the mean by area of the heights
/// of its samples (`heights` on `Square::samples`), so a cell of many squares holds the ground and the low liquid its
/// squares would, and a cell finer than a square takes the height at its own centre.
fn cell_heights(seed: u64, map: &Square, mesh: &Mesh, cfg: &Config) -> Vec<f64> {
    let samples = map.samples();
    let sampled = World::mesh_of(&samples, cfg);
    let (z, area) = (heights(seed, map, sampled.cells()), sampled.cells().area());
    let mut hz = vec![0.0; mesh.cells().n()];
    for (s, leaf) in samples.leaves().iter().enumerate() {
        let c = mesh.tree().cell_at(leaf.corner());
        hz[c] += area[s] / mesh.cells().area()[c] * z[s];
    }
    hz
}

/// Air over a m² at 0 m, mol: the reference pressure over the weight of a mol of the starting air (`Init::air`).
pub fn air_column(chem: &Chemistry, init: &Init, gravity: f64) -> f64 {
    let atot: f64 = init.air.iter().map(|p| p.1).sum();
    let m_air: f64 = init.air.iter().map(|&(k, f)| f / atot * chem.subs[k].molar_mass).sum();
    P_REF / (gravity * m_air)
}

/// Starting temperature at latitude `lat`, degrees, and height `z`, K: warm low latitudes, cooler poles and heights.
fn t_start(lat: f64, z: f64) -> f64 {
    let lat = lat.to_radians();
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

/// The map of an island of `w × h` cells.
pub fn island(w: usize, h: usize) -> Square {
    Square::new(w, h, CELL_M, false, AIR_SCALE)
}

/// The map of a globe of `rows` rows from pole to pole, with twice as many columns, so a cell is square at the equator.
pub fn globe(rows: usize) -> Square {
    Square::new(2 * rows, rows, PI * RADIUS_M / rows as f64, true, AIR_SCALE)
}

/// A new world on `map` from a seed: terrain, chemistry 0, ground, liquid and air.
pub fn new_map(seed: u64, map: Square) -> World {
    let (chem, init) = chemistry0();
    build(seed, map, chem, &init, Config::default())
}

/// A new island from a seed, its low ground away from land in coarse cells (`coarse_low`): terrain, chemistry 0, ground,
/// liquid and air.
pub fn new_world(seed: u64, w: usize, h: usize) -> World {
    new_map(seed, coarse_low(seed, island(w, h)))
}

/// `map` (of squares) with its low ground away from land in coarse cells (`coarsen`): an air block whose squares all
/// lie below 0 m on the terrain of `seed`, none within `LAND_RING` squares of one that does not, is coarse.
fn coarse_low(seed: u64, map: Square) -> Square {
    let z = heights(seed, &map, World::mesh_of(&map, &Config::default()).cells());
    let ring = -LAND_RING..=LAND_RING;
    let open: Vec<bool> = (0..map.n()).map(|s| ring.clone().all(|dy| ring.clone().all(|dx| map.step(s, (dx, dy)).is_none_or(|t| z[t] < 0.0)))).collect();
    let w = map.w;
    coarsen(map, |bx, by| block(w, bx * AIR_SCALE, by * AIR_SCALE, AIR_SCALE).any(|s| !open[s]))
}

/// The squares of the block of side `k` squares from column `x` and row `y` of a map `w` squares wide.
fn block(w: usize, x: usize, y: usize, k: usize) -> impl Iterator<Item = usize> {
    (y..y + k).flat_map(move |ty| (x..x + k).map(move |tx| ty * w + tx))
}

/// `map` (of squares) with each whole air block that `fine(column, row)` (of the block) does not keep in squares one
/// cell, split into smaller cells where it borders finer ones until cells across a border differ by one depth at most
/// (`Square::balanced`).
fn coarsen(map: Square, fine: impl Fn(usize, usize) -> bool) -> Square {
    let (whole, square) = ((map.w / map.scale, map.h / map.scale), map.square_depth());
    map.by_blocks(|bx, by| if bx < whole.0 && by < whole.1 && !fine(bx, by) { 0 } else { square }).balanced()
}

/// A new globe of `rows` rows from a seed, every cell a whole air block but in the rectangle `fine` of air blocks
/// (column, row, columns, rows), in squares (`coarsen`): terrain, chemistry 0, ground, liquid and air.
pub fn new_globe(seed: u64, rows: usize, fine: [usize; 4]) -> World {
    let ([x, y, bw, bh], map) = (fine, globe(rows));
    assert!(x + bw <= map.w / AIR_SCALE && y + bh <= map.h / AIR_SCALE, "the fine rectangle lies inside the globe's {} x {} air blocks", map.w / AIR_SCALE, map.h / AIR_SCALE);
    new_map(seed, coarsen(map, |bx, by| (x..x + bw).contains(&bx) && (y..y + bh).contains(&by)))
}

/// A new world on `map` (an island or a globe, whose rows run from pole to pole).
pub fn build(seed: u64, map: Square, chem: Chemistry, init: &Init, mut cfg: Config) -> World {
    if map.wrap {
        (cfg.lat_south, cfg.lat_north) = (-90.0, 90.0);
    }
    let mesh = World::mesh_of(&map, &cfg);
    let (n, na, ns) = (mesh.cells().n(), mesh.level(AIR).n(), chem.ns());
    let mut hz = cell_heights(seed, &map, &mesh, &cfg);
    drain(mesh.cells(), &mut hz, map.wrap);
    let mut ground = vec![0.0; ns * n];
    let mut bed = vec![0.0; ns * n];
    let mv = |k: usize| chem.molar_volume(k, Phase::Solid);

    for i in 0..n {
        let (z, area) = (hz[i], mesh.cells().area()[i]);
        // Patchy deposits, more common in high ground.
        let mut fr: Vec<(usize, f64)> = init.bed.clone();
        for (j, &(k, f)) in init.deposits.iter().enumerate() {
            let p = fbm(seed ^ (100 + j as u64), &spot(&map, mesh.cells(), i, 0.15), 3);
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

    // Air: a column per air node, thinner over high ground (its cells' mean height, by area).
    let (mut air, mut zmean, air_area) = (vec![0.0; ns * na], vec![0.0; na], mesh.level(AIR).area());
    for i in 0..n {
        zmean[mesh.up(i, 0, AIR)] += hz[i].max(0.0) * mesh.cells().area()[i];
    }
    // The column over high ground is thinner by exp(-z / H), H the scale height of the air at the node's starting
    // temperature (`flow::scale_height`, as Flow's law takes it).
    let (atot, column): (f64, f64) = (init.air.iter().map(|p| p.1).sum(), air_column(&chem, init, cfg.gravity));
    let m_air: f64 = init.air.iter().map(|&(k, f)| f * chem.subs[k].molar_mass).sum();
    let mut t_air = vec![0.0; na];
    for j in 0..na {
        zmean[j] /= air_area[j];
        // At the latitude of the first cell of a row near the middle of the node's block.
        let y = mesh.tree().block_xy(j).1 * map.scale + map.scale / 2;
        let lat = mesh.cells().lat()[mesh.tree().cell_at(map.point(map.idx(0, y.min(map.h - 1))))];
        // A node's temperature is its air's at the column's mean height (`flow::air`), which lies higher the warmer the
        // air: it starts at the starting temperature there, the height taken twice from the scale height it gives.
        let at_mean = |t: f64| {
            let (h, l) = (scale_height(t, atot, m_air, cfg.gravity), cfg.air_top_m - zmean[j]);
            t_start(lat, zmean[j] + h - l / (l / h).exp_m1())
        };
        t_air[j] = at_mean(at_mean(t_start(lat, zmean[j])));
        let col = column * (-zmean[j] / scale_height(t_air[j], atot, m_air, cfg.gravity)).exp() * air_area[j];
        for &(k, f) in &init.air {
            air[k * na + j] = f / atot * col;
        }
    }

    let links = Links::new(mesh.cells().a().len());

    let life = Life::new(&mesh);
    let mut world = World {
        cfg,
        chem,
        map,
        mesh,
        seed,
        time: 0,
        sleep: Default::default(),
        store: Store::new(n, (ground, bed, vec![0.0; n]), (air, vec![0.0; na])),
        base_z: BASE_Z,
        counts: Counters::default(),
        rain: vec![0.0; n],
        liquid_flux: vec![0.0; n],
        evap: vec![0.0; n],
        off_land: vec![0.0; n],
        life,

        canopy: vec![0.0; n],
        links,
        joined: vec![None; n],
        watch: Vec::new(),
        light: vec![0.0; n],
        rained: vec![0.0; n],
        events: Vec::new(),
        scratch: Default::default(),
    };
    // The ground and the air at their starting temperatures. (The world is made: its store is built whole at each stage,
    // never written outside `store::apply` once it runs.)
    let (amounts, heat) = (|w: &World, c: Class| w.store.amounts(c).to_vec(), |w: &World, c: Class| w.store.heat(c).to_vec());
    let mut e_ground = vec![0.0; n];
    for (i, &z) in hz.iter().enumerate().take(n) {
        // Low ground starts with its liquid molten.
        let t = t_start(world.mesh.cells().lat()[i], z).max(if z < 0.0 { world.chem.subs[init.low].melt + MOLTEN_MARGIN } else { 0.0 });
        e_ground[i] = world.energy_ground(i, t);
    }
    let e_air = t_air.iter().enumerate().map(|(j, &t)| world.heat_cap_air(j) * t).collect();
    world.store = Store::new(n, (amounts(&world, Class::Ground), amounts(&world, Class::Bed), e_ground), (amounts(&world, Class::Air), e_air));
    // The liquid starts at balance with the air (`Reactor::balance`, laws.md 2.6); a cell with no liquid holds none. The
    // node takes the heat of its new amounts at its temperature, once they are in the store.
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

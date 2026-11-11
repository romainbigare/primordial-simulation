//! Terrain: an island or a globe with mountains, and the starting state of ground, low liquid and air.

use crate::chemistry::{chemistry0, Init};

use pcore::change::Reactor;
use pcore::chem::{Chemistry, Phase};
use pcore::flow::scale_height;
use pcore::life::Life;
use pcore::space::square::Square;
use pcore::space::samples::Samples;
use pcore::space::terrain::{self, Place, Terrain, ISLAND_LEVEL};
use pcore::space::Level;
use pcore::store::{Boundary, Class, Links, Node, Store};
use pcore::units::P_REF;
use pcore::{Config, Counters, World, AIR};
use std::f64::consts::PI;

/// Cell size of an island, m, and ground cells per air node along a side.
const CELL_M: f64 = 2000.0;
const AIR_SCALE: usize = 4;
/// The squares below 0 m round the land that stay squares on an island (`coarse_low`), so its edges stay sharp.
const LAND_RING: i32 = 1;
/// A globe's radius, m, and the share of its area above the level 0 m: those of the one planet known (Earth: mean
/// radius 6,371 km, land 29% of the surface).
const RADIUS_M: f64 = 6.371e6;
const LAND_SHARE: f64 = 0.29;
/// Height of the bottom of the bed, m.
const BASE_Z: f64 = -3000.0;
/// Starting temperatures, K: at the equator at 0 m, less `POLE_DROP` x sin² latitude, less `LAPSE` K per m of height.
const T_EQUATOR: f64 = 302.0;
const POLE_DROP: f64 = 45.0;
const LAPSE: f64 = 0.0065;
/// Ground that holds the starting liquid (of the low basin and the bodies) starts this many K above its melting point,
/// so it is molten.
const MOLTEN_MARGIN: f64 = 2.0;

/// The terrain of `seed` on `map` whose samples are `cells` (`space::terrain`): an island's level 0 m lies at the rise
/// `ISLAND_LEVEL`; a globe's leaves the share `LAND_SHARE` of the area of the samples above it.
fn terrain_of(seed: u64, map: &Square, cells: &Level, soft: f64) -> Terrain {
    if !map.wrap {
        return Terrain::new(seed, ISLAND_LEVEL, soft);
    }
    let rise: Vec<f64> = (0..cells.n()).map(|i| terrain::rise(seed, map, Place::of(cells, i))).collect();
    let mut order: Vec<usize> = (0..cells.n()).collect();
    order.sort_by(|&a, &b| rise[b].total_cmp(&rise[a]));
    let (total, mut above) = (cells.area().iter().sum::<f64>(), 0.0);
    let level = order.iter().find(|&&i| {
        above += cells.area()[i];
        above >= LAND_SHARE * total
    });
    Terrain::new(seed, level.map_or(0.0, |&i| rise[i]), soft)
}

/// The volume fractions of the bed where the share `patch` of it is of its patches (`terrain::patch`): the chemistry's
/// `bed` and, over that share, its `deposits`, each by its own fractions.
fn bed_of(init: &Init, patch: f64) -> Vec<(usize, f64)> {
    let whole = |fr: &[(usize, f64)], by: f64| fr.iter().map(move |&(k, f)| (k, by * f / fr.iter().map(|p| p.1).sum::<f64>())).collect::<Vec<_>>();
    if init.deposits.is_empty() {
        return whole(&init.bed, 1.0);
    }
    [whole(&init.bed, 1.0 - patch), whole(&init.deposits, patch)].concat()
}

/// How much faster flowing liquid wears the bed of the patches than the rest (`Terrain::soft`): as Flow's wear breaks a
/// bed (laws.md 1.3), by the sum of each substance's volume share over its bond strength.
pub fn soft_of(chem: &Chemistry, init: &Init) -> f64 {
    let weak = |fr: Vec<(usize, f64)>| fr.iter().map(|&(k, f)| f / chem.subs[k].strength).sum::<f64>();
    weak(bed_of(init, 1.0)) / weak(bed_of(init, 0.0))
}

/// Height of each cell of `map` by `terrain` (its rows between the latitudes `lat`), m: the terrain's mean over it
/// (`Terrain::inside`, on its worn samples), the one height of the ground of a cell, which its relief stands about
/// and its splits continue (laws.md 0).
fn heights(seed: u64, map: &Square, terrain: &Terrain, lat: (f64, f64)) -> Vec<f64> {
    map.leaves().iter().map(|&l| terrain.inside(seed, map, l, lat)).collect()
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
/// liquid and air. One terrain, so its worn samples are found once.
pub fn new_world(seed: u64, w: usize, h: usize) -> World {
    let ((squares, terrain), (chem, init)) = (island_ground(seed, w, h), chemistry0());
    let map = coarse_low(seed, squares, &terrain);
    build_on(seed, map, terrain, (chem, &init), Config::default())
}

/// The ground of the island of `w × h` squares `new_world` makes from `seed`: the map of its squares and its terrain
/// (how its cells are cut does not change the ground).
pub fn island_ground(seed: u64, w: usize, h: usize) -> (Square, Terrain) {
    let (chem, init) = chemistry0();
    (island(w, h), Terrain::new(seed, ISLAND_LEVEL, soft_of(&chem, &init)))
}

/// `map` (of squares) with its low ground away from land in coarse cells (`coarsen`): an air block whose squares all
/// lie below 0 m on `terrain` of `seed`, none within `LAND_RING` squares of one that does not, is coarse.
fn coarse_low(seed: u64, map: Square, terrain: &Terrain) -> Square {
    let cfg = Config::default();
    let z = heights(seed, &map, terrain, (cfg.lat_south, cfg.lat_north));
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
    let terrain = terrain_of(seed, &map, World::mesh_of(&map.samples(), &cfg).cells(), soft_of(&chem, init));
    build_on(seed, map, terrain, (chem, init), cfg)
}

/// `build` on `terrain` (its level found on the map's squares, `terrain_of`), each cell at its height (`heights`).
fn build_on(seed: u64, map: Square, terrain: Terrain, (chem, init): (Chemistry, &Init), cfg: Config) -> World {
    let mesh = World::mesh_of(&map, &cfg);
    let (n, na, ns) = (mesh.cells().n(), mesh.level(AIR).n(), chem.ns());
    let lat = (cfg.lat_south, cfg.lat_north);
    let hz = heights(seed, &map, &terrain, lat);
    let over = |x: &dyn Fn(&Samples, usize) -> f64| map.leaves().iter().map(|&l| terrain.over(seed, &map, l, lat, x)).collect::<Vec<f64>>();
    // A cell's ground node is the layer that exchanges with the surface within a year (`skin_m`): of each sample it takes
    // the top `skin_m` of its loose ground at most; what the wear left deeper lies under it, packed with the bed.
    let (patch, loose) = (over(&|s, k| s.patch[k]), over(&|s, k| s.loose[k].min(cfg.skin_m)));
    let mut ground = vec![0.0; ns * n];
    let mut bed = vec![0.0; ns * n];
    let mv = |k: usize| chem.molar_volume(k, Phase::Solid);

    for i in 0..n {
        let (z, area) = (hz[i], mesh.cells().area()[i]);
        let fr = bed_of(init, patch[i]);
        let tot: f64 = fr.iter().map(|p| p.1).sum();
        let bed_v = (z - loose[i] - BASE_Z) * area;
        for &(k, f) in &fr {
            bed[k * n + i] += f / tot * bed_v / mv(k);
        }
        let ltot: f64 = init.loose.iter().map(|p| p.1).sum();
        for &(k, f) in &init.loose {
            ground[k * n + i] += f / ltot * loose[i] * area / mv(k);
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
        discharge: Vec::new(),
        liquid_flux: vec![0.0; n],
        evap: vec![0.0; n],
        off_land: vec![0.0; n],
        life,

        canopy: vec![0.0; n],
        links,
        terrain,
        rise_south: Vec::new(),
        light: vec![0.0; n],
        rained: vec![0.0; n],
        events: Vec::new(),
        scratch: Default::default(),
    };
    let (amounts, heat) = (|w: &World, c: Class| w.store.amounts(c).to_vec(), |w: &World, c: Class| w.store.heat(c).to_vec());
    // The low basin's liquid at 0 m and each body of standing liquid full to its pass: each cell whose own ground dips
    // under them holds the liquid that fills its pores and stands over its ground up to them (`World::full_bodies`), as
    // Flow's pass stands it.
    let (por, v_low) = (world.cfg.phys.porosity, world.chem.molar_volume(init.low, Phase::Liquid));
    let low = world.full_bodies(0.0);
    let mut ground = amounts(&world, Class::Ground);
    for (i, &under) in low.iter().enumerate().filter(|p| *p.1 > 0.0) {
        ground[init.low * n + i] += (por * loose[i] * world.mesh.cells().area()[i] + under) / v_low;
    }
    world.store = Store::new(n, (ground, amounts(&world, Class::Bed), vec![0.0; n]), (amounts(&world, Class::Air), vec![0.0; na]));
    // The ground and the air at their starting temperatures. (The world is made: its store is built whole at each stage,
    // never written outside `store::apply` once it runs.)
    let mut e_ground = vec![0.0; n];
    for (i, &z) in hz.iter().enumerate().take(n) {
        // Ground that holds the starting liquid starts with it molten.
        let t = t_start(world.mesh.cells().lat()[i], z).max(if low[i] > 0.0 { world.chem.subs[init.low].melt + MOLTEN_MARGIN } else { 0.0 });
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
    // The way the ground of each cell faces (laws.md 0), and no liquid running yet down the terrain's samples.
    world.rise_south = world.rise_at_build();
    world.discharge = vec![0.0; world.terrain_samples().h.len()];
    world
}

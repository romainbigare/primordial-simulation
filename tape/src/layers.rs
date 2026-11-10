//! Surface fields, the layer table and byte quantisation for viewer frames.
//! To add a layer, add one line to `LAYERS` (and the data it needs to `Ctx`).

use pcore::par;
use pcore::body::Body;
use pcore::cover::{per_m2, Cover};
use pcore::chem::{Phase, Substance};
use pcore::{World, AIR};

/// Thinnest cover (m) that hides what lies below it.
pub(crate) const COVER_MIN: f64 = 0.01;
/// Melting points of the frozen cover, K: substances that are liquid somewhere on the surface.
const COVER_MELT: (f64, f64) = (200.0, 320.0);
/// Boiling point above which a gas of the air counts as vapour (it condenses on the surface), K.
const VAPOR_BOIL: f64 = 250.0;
/// Ground temperature at which the true colour starts to glow, K; the glow triples it over as many K again.
const GLOW_T: f64 = 500.0;

/// Frozen cover: substances that are liquid somewhere on the surface (`COVER_MELT`).
fn is_cover(s: &Substance) -> bool {
    s.melt > COVER_MELT.0 && s.melt < COVER_MELT.1
}

/// The melting point of the frozen cover, K: that of the substance of the cover (`is_cover`) the ground holds most of
/// (0 where it holds none).
pub(crate) fn cover_melt(w: &World) -> f64 {
    let held = |k: usize| (0..w.mesh.cells().n()).map(|i| w.g(k, i)).sum::<f64>();
    let covers = (0..w.chem.ns()).filter(|&k| is_cover(&w.chem.subs[k]));
    covers.map(|k| (held(k), w.chem.subs[k].melt)).max_by(|a, b| a.0.total_cmp(&b.0)).map_or(0.0, |(_, melt)| melt)
}

/// Per ground cell values.
pub struct Fields {
    /// Surface elevation, m
    pub z: Vec<f64>,
    /// Liquid depth, m
    pub depth: Vec<f64>,
    /// Frozen cover depth, m
    pub ice: Vec<f64>,
    /// Loose solid depth (without frozen cover), m
    pub loose: Vec<f64>,
    /// Ground temperature, K
    pub t: Vec<f64>,
    /// Visible colour of what covers the surface, 0..1
    pub rgb: Vec<[f64; 3]>,
    /// Colour of the ground itself, 0..1: its loose solids, else its bonded ones (no frozen cover, no liquid)
    pub ground: Vec<[f64; 3]>,
}

/// One cell's values: elevation, liquid depth, frozen cover depth, loose solid depth, temperature, colour of the
/// surface and of the ground.
type Cell = (f64, f64, f64, f64, f64, [f64; 3], [f64; 3]);

pub fn fields(w: &World) -> Fields {
    let cells = par::map_n(w.mesh.cells().n(), par::UNCOUNTED, |i| cell(w, i));
    let mut f = Fields { z: vec![], depth: vec![], ice: vec![], loose: vec![], t: vec![], rgb: vec![], ground: vec![] };
    for (z, depth, ice, loose, t, rgb, ground) in cells {
        f.z.push(z);
        f.depth.push(depth);
        f.ice.push(ice);
        f.loose.push(loose);
        f.t.push(t);
        f.rgb.push(rgb);
        f.ground.push(ground);
    }
    f
}

/// The values of cell `i` (`Cell`).
fn cell(w: &World, i: usize) -> Cell {
    let (t, pl) = w.thaw_node_last(i);
    // Kinds of cover in order of priority: frozen, liquid, loose, bonded. Each: volume, volume * colour.
    let mut acc = [[0.0f64; 4]; 4];
    let mut add = |kind: usize, vol: f64, refl: [f32; 3]| {
        acc[kind][0] += vol;
        for c in 0..3 {
            acc[kind][1 + c] += vol * refl[c] as f64;
        }
    };
    for k in 0..w.chem.ns() {
        let s = &w.chem.subs[k];
        let (melt, a) = (w.molten(k, i, t, pl), w.g(k, i));
        let solid = a * (1.0 - melt) * w.chem.molar_volume(k, Phase::Solid);
        add(if is_cover(s) { 0 } else { 2 }, solid, s.refl[0]);
        if t < s.boil {
            add(1, a * melt * w.chem.molar_volume(k, Phase::Liquid), s.refl[1]);
        }
        add(3, w.b(k, i) * w.chem.molar_volume(k, Phase::Solid), s.refl[0]);
    }
    let area = w.mesh.cells().area()[i];
    let top = acc[..3].iter().find(|a| a[0] / area >= COVER_MIN).unwrap_or(&acc[3]);
    let glow = 1.0 + 2.0 * ((t - GLOW_T) / GLOW_T).clamp(0.0, 1.0);
    let colour = |a: &[f64; 4]| [1, 2, 3].map(|c| if a[0] > 0.0 { (a[c] / a[0] * glow).min(1.0) } else { 0.0 });
    let ground = acc[2..].iter().find(|a| a[0] > 0.0).unwrap_or(&acc[3]);
    (w.surface_z(i), acc[1][0] / area, acc[0][0] / area, acc[2][0] / area, w.t_surface_last(i), colour(top), colour(ground))
}

/// Liquid deeper than this stands (a basin), m; shallower, it may flow as a river.
pub const STANDING_M: f64 = 1.0;
/// A river carries at least this many times the liquid its own area sheds (`World::catchment`): an accumulation threshold, how
/// drainage networks are counted (O'Callaghan and Mark 1984).
pub const RIVER_CATCH: f64 = 10.0;
/// A river carries at least this liquid flux, mol/s, so a dry cell with a trickle from upstream is none.
pub const RIVER_FLOOR: f64 = 9.0e3;
/// Whether cell `i` is a river: its flux is over `RIVER_FLOOR` and its catchment (`World::catchment`) over `RIVER_CATCH`.
pub fn is_river(w: &World, i: usize) -> bool {
    w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64 > RIVER_FLOOR && w.catchment(i) > RIVER_CATCH
}

/// The dry land cells (land under no more than `STANDING_M` of liquid) that are rivers (`is_river`).
pub fn river_count(w: &World, f: &Fields) -> usize {
    (0..w.mesh.cells().n()).filter(|&i| w.is_land(i) && f.depth[i] <= STANDING_M && is_river(w, i)).count()
}

/// Everything a layer, and a row of the series, may read.
pub(crate) struct Ctx<'a> {
    pub w: &'a World,
    pub f: Fields,
    /// Per air node: temperature K, column of condensable gases mol/m², heat-absorbing column (Σ n·ir_abs / area)
    pub t_air: Vec<f64>,
    pub vapor: Vec<f64>,
    pub absorber: Vec<f64>,
    /// Per ground cell: the cover's mass ("plants", `cover::per_m2`), its walls ("wood"), kg/m², and its capture area
    /// ("leaf"), m²/m²; the bodies' mass ("animals", all members of each node), kg/m²; bodies (nodes and individuals)
    pub plants: Vec<f64>,
    pub wood: Vec<f64>,
    pub leaf: Vec<f64>,
    pub animals: Vec<f64>,
    pub bodies: Vec<f64>,
    /// Per ground cell: how wet its ground is, 0..1 (`Physics::wetness` of the liquid in its pores and standing over
    /// them, `flow::means`; none before Flow first built its state)
    pub wet: Vec<f64>,
    /// Per ground cell: mass of made objects (parts), kg/m²
    pub parts: Vec<f64>,
    /// Per living body (`Life::alive`): its mass, kg
    pub mass: Vec<f64>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(w: &'a World) -> Self {
        let n = w.mesh.cells().n();
        let t_air = (0..w.mesh.level(AIR).n()).map(|j| w.t_air(j)).collect();
        let bodies: Vec<&Body> = w.life.alive().collect();
        let mass = par::map(&bodies, |b| b.mass(&w.chem));
        let f = fields(w);
        let means = pcore::flow::means(w, &(0..n).collect::<Vec<usize>>()).unwrap_or_else(|| vec![Default::default(); n]);
        let wet = means.iter().map(|m| w.cfg.phys.wetness(m.pore + m.stands)).collect();
        let mut c = Ctx { w, f, t_air, vapor: vec![], absorber: vec![], plants: per_m2(w, Cover::kg), wood: per_m2(w, Cover::walls), leaf: per_m2(w, Cover::leaf), animals: vec![0.0; n], bodies: vec![0.0; n], wet, parts: vec![0.0; n], mass };
        for p in w.life.parts() {
            let i = p.cell as usize;
            c.parts[i] += p.mass(&w.chem) / w.mesh.cells().area()[i];
        }
        for (b, &mass) in bodies.iter().zip(&c.mass) {
            let i = b.cell as usize;
            c.animals[i] += mass / w.mesh.cells().area()[i];
            c.bodies[i] += 1.0;
        }
        (c.absorber, c.vapor) = par::map_n(w.mesh.level(AIR).n(), par::UNCOUNTED, |j| air_column(w, j)).into_iter().unzip();
        c
    }
    fn air(&self, i: usize) -> usize {
        self.w.mesh.up(i, 0, AIR)
    }
    /// The channel of cell `i`'s own liquid flux (`World::channel`).
    pub(crate) fn channel(&self, i: usize) -> pcore::flow::Channel {
        self.w.channel(i, self.w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64)
    }
}

/// Air node `j`: its heat-absorbing column (Σ n·ir_abs / area) and the column of condensable gases, mol/m².
fn air_column(w: &World, j: usize) -> (f64, f64) {
    let (mut absorber, mut vapor, area) = (0.0, 0.0, w.mesh.level(AIR).area()[j]);
    for (k, s) in w.chem.subs.iter().enumerate() {
        let n = w.a(k, j);
        absorber += w.chem.depth(k, n / area);
        if s.boil > VAPOR_BOIL {
            vapor += n / area;
        }
    }
    (absorber, vapor)
}

pub struct Layer {
    pub name: &'static str,
    pub unit: &'static str,
    /// "lin", "log" or "raw" (0..1)
    pub scale: &'static str,
    pub min: f64,
    pub max: f64,
    pub(crate) val: fn(&Ctx, usize) -> f64,
}

const fn l(name: &'static str, unit: &'static str, scale: &'static str, min: f64, max: f64, val: fn(&Ctx, usize) -> f64) -> Layer {
    Layer { name, unit, scale, min, max, val }
}

pub const LAYERS: &[Layer] = &[
    l("z", "m", "lin", -800.0, 4000.0, |c, i| c.f.z[i]),
    l("depth", "m", "log", 0.01, 3000.0, |c, i| c.f.depth[i]),
    l("ice", "m", "log", 0.001, 100.0, |c, i| c.f.ice[i]),
    l("flux", "mol/s", "log", 1.0, 1e7, |c, i| c.w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64),
    l("catch", "1", "log", 1.0, 1e5, |c, i| c.w.catchment(i)),
    l("channel_w", "m", "log", 0.1, 1000.0, |c, i| c.channel(i).width),
    l("channel_d", "m", "log", 0.01, 30.0, |c, i| c.channel(i).depth),
    l("loose", "m", "log", 0.01, 100.0, |c, i| c.f.loose[i]),
    l("wet", "1", "lin", 0.0, 1.0, |c, i| c.wet[i]),
    l("t_ground", "K", "lin", 220.0, 330.0, |c, i| c.f.t[i]),
    l("t_air", "K", "lin", 220.0, 330.0, |c, i| c.t_air[c.air(i)]),
    l("rain", "mol/m2/s", "log", 1e-6, 1e-2, |c, i| c.w.rain.get(i).copied().unwrap_or(0.0) as f64),
    l("vapor", "mol/m2", "log", 1.0, 1e5, |c, i| c.vapor[c.air(i)]),
    l("absorber", "1", "lin", 0.0, 5.0, |c, i| c.absorber[c.air(i)]),
    l("plants", "kg/m2", "log", 0.03, 300.0, |c, i| c.plants[i]),
    l("wood", "kg/m2", "log", 0.01, 300.0, |c, i| c.wood[i]),
    l("leaf", "m2/m2", "log", 0.01, 30.0, |c, i| c.leaf[i]),
    l("animals", "kg/m2", "log", 1e-4, 10.0, |c, i| c.animals[i]),
    l("bodies", "1", "lin", 0.0, 32.0, |c, i| c.bodies[i]),
    l("traffic", "1/yr", "log", 1.0, 1e4, |c, i| c.w.counts.traffic.get(i).copied().unwrap_or(0.0) as f64),
    l("worn", "1", "log", 1.0, 100.0, |c, i| c.w.mesh.cells().around(i).0.iter().map(|&l| pcore::lanes::eased(c.w, l as usize)).fold(1.0, f64::max)),
    l("structures", "kg/m2", "log", 1e-6, 1.0, |c, i| c.parts[i]),
    l("true_r", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][0]),
    l("true_g", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][1]),
    l("true_b", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][2]),
    l("ground_r", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][0]),
    l("ground_g", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][1]),
    l("ground_b", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][2]),
];

/// Quantise a value to a byte as the frames format defines.
pub fn quantise(l: &Layer, v: f64) -> u8 {
    let b = match l.scale {
        "raw" => 255.0 * v,
        "log" if v < l.min => 0.0,
        "log" => 1.0 + 254.0 * (v.ln() - l.min.ln()) / (l.max.ln() - l.min.ln()),
        _ => 255.0 * (v - l.min) / (l.max - l.min),
    };
    b.round().clamp(0.0, 255.0) as u8
}

/// The cell each pixel of an `across` x `down` raster of `window` (column, row, width and height in squares) shows,
/// row by row: the one that holds its centre.
pub fn raster(w: &World, [x, y, wide, high]: [f64; 4], across: usize, down: usize) -> Vec<usize> {
    let tree = w.mesh.tree();
    let at = |k: usize, n: usize, from: f64, span: f64| from + (k as f64 + 0.5) * span / n as f64;
    (0..across * down).map(|p| tree.cell_at(w.map.point_at(at(p % across, across, x, wide), at(p / across, down, y, high)))).collect()
}

/// The cell each square of the map shows, row by row: the one that holds its centre (`Square::point`).
pub fn pixels(w: &World) -> Vec<usize> {
    raster(w, [0.0, 0.0, w.map.w as f64, w.map.h as f64], w.map.w, w.map.h)
}

/// Append one frame: every layer in table order, a byte for each square of the map, row-major (`pixels`).
pub(crate) fn frame(c: &Ctx, out: &mut Vec<u8>) {
    layers(c, &pixels(c.w), out);
}

/// Append every layer in table order, a byte for each of `cells` (the cell each pixel shows).
fn layers(c: &Ctx, cells: &[usize], out: &mut Vec<u8>) {
    for l in LAYERS {
        out.extend(cells.iter().map(|&i| quantise(l, (l.val)(c, i))));
    }
}

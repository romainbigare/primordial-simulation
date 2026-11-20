//! Surface fields, the layer table and byte quantisation for viewer frames.
//! To add a layer, add one line to `LAYERS` (and the data it needs to `Ctx`).

use pcore::par;
use pcore::cover::{lying, per_m2, Cover};
use pcore::chem::{Phase, Substance};
use pcore::{World, AIR};

/// Thinnest cover (m) that hides what lies below it.
pub(crate) const COVER_MIN: f64 = 0.01;
/// The remains of the cover lie on the ground's surface as litter, covering all but e^-1 of it at this much, kg/m² (a
/// forest floor's litter of a few tenths of a kg/m² hides the mineral soil; forests hold 0.5 to 5 kg/m² of it, from
/// memory).
const LITTER_KG: f64 = 0.3;
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
    /// Colour of the ground itself, 0..1: its loose solids, else its bonded ones (no frozen cover, no liquid), under the
    /// litter of the cover's remains on it (`LITTER_KG`)
    pub ground: Vec<[f64; 3]>,
}

/// One cell's values: elevation, liquid depth, frozen cover depth, loose solid depth, temperature, colour of the
/// surface and of the ground.
type Cell = (f64, f64, f64, f64, f64, [f64; 3], [f64; 3]);

/// Of each cell of `w`, its liquid in its pores and over them (`flow::means`; none before Flow first built its state).
fn liquid_of(w: &World) -> Vec<pcore::inside::Means> {
    let n = w.mesh.cells().n();
    pcore::flow::means(w, &(0..n).collect::<Vec<usize>>()).unwrap_or_else(|| vec![Default::default(); n])
}

pub fn fields(w: &World) -> Fields {
    fields_of(w, &liquid_of(w))
}

/// The fields of `w` whose cells hold the liquid `liquid` (`liquid_of`).
fn fields_of(w: &World, liquid: &[pcore::inside::Means]) -> Fields {
    let cells = par::map_n(w.mesh.cells().n(), par::UNCOUNTED, |i| cell(w, i, liquid[i].stands));
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

/// The values of cell `i` (`Cell`), over whose pores `stands` m of liquid stands: the liquid shows as the surface only
/// so far as it stands over the ground; the liquid in the pores is part of the ground (`flow::means`).
fn cell(w: &World, i: usize, stands: f64) -> Cell {
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
    }
    acc[3] = bonded(w, i);
    let area = w.mesh.cells().area()[i];
    let shows = |kind: usize, a: &[f64; 4]| (if kind == 1 { stands } else { a[0] / area }) >= COVER_MIN;
    let top = acc[..3].iter().enumerate().find(|&(kind, a)| shows(kind, a)).map_or(&acc[3], |(_, a)| a);
    let glow = 1.0 + 2.0 * ((t - GLOW_T) / GLOW_T).clamp(0.0, 1.0);
    let colour = |a: &[f64; 4]| [1, 2, 3].map(|c| if a[0] > 0.0 { (a[c] / a[0] * glow).min(1.0) } else { 0.0 });
    let ground = colour(acc[2..].iter().find(|a| a[0] > 0.0).unwrap_or(&acc[3]));
    (w.surface_z(i), acc[1][0] / area, acc[0][0] / area, acc[2][0] / area, w.t_surface_last(i), colour(top), litter(w, i, area, ground))
}

/// The bonded solids of cell `i` (its bed): their volume, m³, and that times their reflectance (red, green, blue).
fn bonded(w: &World, i: usize) -> [f64; 4] {
    let mut acc = [0.0; 4];
    for k in 0..w.chem.ns() {
        let (v, refl) = (w.b(k, i) * w.chem.molar_volume(k, Phase::Solid), w.chem.subs[k].refl[0]);
        acc[0] += v;
        (0..3).for_each(|c| acc[1 + c] += v * refl[c] as f64);
    }
    acc
}

/// Ground of colour `ground` in cell `i` (`area` m²) under the litter of the cover's remains in it: their colour by
/// volume, over the share 1 - e^-(their kg/m² / `LITTER_KG`) of it.
fn litter(w: &World, i: usize, area: f64, ground: [f64; 3]) -> [f64; 3] {
    let Some(p) = pcore::cover::Parts::of(&w.chem) else { return ground };
    let (mut kg, mut vol, mut refl) = (0.0, 0.0, [0.0; 3]);
    for &k in &p.sub {
        let (s, mol) = (&w.chem.subs[k], w.g(k, i));
        let v = mol * w.chem.molar_volume(k, Phase::Solid);
        (kg, vol) = (kg + mol * s.molar_mass, vol + v);
        (0..3).for_each(|c| refl[c] += v * s.refl[0][c] as f64);
    }
    if vol <= 0.0 {
        return ground;
    }
    let covers = -(-kg / area / LITTER_KG).exp_m1();
    std::array::from_fn(|c| ground[c] + (refl[c] / vol - ground[c]) * covers)
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
fn river_cells<'a>(w: &'a World, f: &'a Fields) -> impl Iterator<Item = usize> + 'a {
    (0..w.mesh.cells().n()).filter(move |&i| w.is_land(i) && f.depth[i] <= STANDING_M && is_river(w, i))
}

/// How many dry land cells are rivers (`river_cells`).
pub fn river_count(w: &World, f: &Fields) -> usize {
    river_cells(w, f).count()
}

/// The liquid flux of the dry land cells that are rivers (`river_cells`, `World::liquid_flux`), summed, mol/s.
pub fn river_flux(w: &World, f: &Fields) -> f64 {
    river_cells(w, f).map(|i| w.liquid_flux[i] as f64).sum()
}

/// Everything a layer, and a row of the series, may read.
pub(crate) struct Ctx<'a> {
    pub w: &'a World,
    pub f: Fields,
    /// Per air node: temperature K, column of condensable gases mol/m², heat-absorbing column (its lines' optical depth
    /// summed over the bands, `air_column`)
    pub t_air: Vec<f64>,
    pub vapor: Vec<f64>,
    pub absorber: Vec<f64>,
    /// Per ground cell: the cover's mass ("plants", `cover::per_m2`), its walls ("wood"), kg/m², and its capture area
    /// ("leaf"), m²/m²
    pub plants: Vec<f64>,
    pub wood: Vec<f64>,
    pub leaf: Vec<f64>,
    /// Per ground cell: the seeds lying on its ground, kg/m² (`cover::lying`), and what its cover built in the last whole
    /// year, kg of dry matter per m² a year (`Counters::grown_last`; none before a year has passed)
    pub seeds: Vec<f64>,
    pub grown: Vec<f64>,
    /// Per ground cell: the living mass of its takers per km² of its land, kg (`fauna::per_km2`)
    pub takers: Vec<f64>,
    /// Per ground cell: how wet its ground is, 0..1 (`Physics::wetness` of the liquid in its pores and standing over
    /// them, `flow::means`; none before Flow first built its state)
    pub wet: Vec<f64>,
    /// Per ground cell: the share of its loose ground's pores its liquid fills, 0..1 (`flow::means`; none (NaN) where it
    /// holds no pores or before Flow first built its state)
    pub filled: Vec<f64>,
    /// Per ground cell: the share of its area that is land, 0..1: its land (`contract::lands`, which its cover lives
    /// on) over its area
    pub land: Vec<f64>,
    /// Per ground cell: the dead matter, minerals and rot of its ground (`soil::holdings`; none before Flow first built
    /// its state)
    pub dead: Option<Vec<pcore::soil::Holdings>>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(w: &'a World) -> Self {
        let n = w.mesh.cells().n();
        let t_air = (0..w.mesh.level(AIR).n()).map(|j| w.t_air(j)).collect();
        let means = liquid_of(w);
        let f = fields_of(w, &means);
        let wet = means.iter().map(|m| w.cfg.phys.wetness(m.pore + m.stands)).collect();
        let filled = means.iter().map(|m| if m.pores > 0.0 { m.pore / m.pores } else { f64::NAN }).collect();
        let area = w.mesh.cells().area();
        let grown = (0..n).map(|i| w.counts.grown_last.get(i).map_or(0.0, |kg| kg / area[i])).collect();
        let land = pcore::contract::lands(w).iter().zip(area).map(|(m2, a)| m2 / a).collect();
        let mut c = Ctx { w, f, t_air, vapor: vec![], absorber: vec![], plants: per_m2(w, Cover::kg), wood: per_m2(w, Cover::walls), leaf: per_m2(w, Cover::leaf), seeds: lying(w), grown, takers: pcore::fauna::per_km2(w), wet, filled, land, dead: pcore::soil::holdings(w) };
        (c.absorber, c.vapor) = par::map_n(w.mesh.level(AIR).n(), par::UNCOUNTED, |j| air_column(w, j)).into_iter().unzip();
        c
    }
    /// Of the dead matter, minerals and rot of cell `i`'s ground (`dead`), the number `of`; none (NaN) before Flow
    /// first built its state.
    fn dead(&self, i: usize, of: fn(&pcore::soil::Holdings) -> f64) -> f64 {
        self.dead.as_ref().map_or(f64::NAN, |d| of(&d[i]))
    }
    fn air(&self, i: usize) -> usize {
        self.w.mesh.up(i, 0, AIR)
    }
    /// The channel of cell `i`'s own liquid flux (`World::channel`).
    pub(crate) fn channel(&self, i: usize) -> pcore::flow::Channel {
        self.w.channel(i, self.w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64)
    }
}

/// An amount per m² of a cell of which the share `land` is land, as per m² of that land: what lives on land only (the
/// cover), as a viewer draws it on the land only; none (NaN) where the cell holds no land.
pub(crate) fn per_land(per_m2: f64, land: f64) -> f64 {
    if land > 0.0 { per_m2 / land } else { f64::NAN }
}

/// What cell `i` of `w` sheds, mol/(m² s): its rain less what rises from it (`World::rain`, `World::evap`; their
/// moving averages), the liquid its ground passes on to runoff and to the ground below over a year; less than none
/// where more rises than falls.
fn sheds(w: &World, i: usize) -> f64 {
    w.rain.get(i).copied().unwrap_or(0.0) as f64 - w.evap.get(i).copied().unwrap_or(0.0) as f64
}

/// Air node `j`: its heat-absorbing column (the optical depth of its gases' lines, `Chemistry::depth`, summed over the
/// bands of heat radiation) and the column of condensable gases, mol/m².
fn air_column(w: &World, j: usize) -> (f64, f64) {
    let (mut absorber, mut vapor, area) = (0.0, 0.0, w.mesh.level(AIR).area()[j]);
    for (k, s) in w.chem.subs.iter().enumerate() {
        let n = w.a(k, j);
        absorber += (0..pcore::chem::HEAT_BANDS).map(|b| w.chem.depth(k, b, n / area)).sum::<f64>();
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
    l("filled", "1", "lin", 0.0, 1.0, |c, i| c.filled[i]),
    l("coarse", "1", "lin", 0.0, 1.0, |c, i| c.w.soil[i].texture.coarse),
    l("fine", "1", "lin", 0.0, 1.0, |c, i| c.w.soil[i].texture.fine),
    l("organic", "1", "log", 3e-4, 0.05, |c, i| c.w.soil[i].texture.organic),
    l("litter", "kg/m2", "log", 0.01, 30.0, |c, i| c.dead(i, |g| g.litter)),
    l("slow", "kg/m2", "log", 0.1, 10.0, |c, i| c.dead(i, |g| g.slow)),
    l("mineral_free", "kg/m2", "log", 1e-5, 0.1, |c, i| c.dead(i, |g| g.minerals[0])),
    l("mineral_held", "kg/m2", "log", 1e-5, 0.3, |c, i| c.dead(i, |g| g.minerals[1])),
    l("rot", "1/yr", "log", 0.01, 10.0, |c, i| c.dead(i, |g| g.rot)),
    l("conduct", "m/s", "log", 1e-9, 1e-4, |c, i| c.w.pores(i).seep),
    l("avail", "m", "log", 0.01, 0.3, |c, i| c.w.pores(i).for_roots(c.f.loose[i])),
    l("roots", "1", "lin", 0.0, 1.0, |c, i| c.w.pores(i).available(c.filled[i])),
    l("t_ground", "K", "lin", 220.0, 330.0, |c, i| c.f.t[i]),
    l("t_air", "K", "lin", 220.0, 330.0, |c, i| c.t_air[c.air(i)]),
    l("rain", "mol/m2/s", "log", 1e-6, 1e-2, |c, i| c.w.rain.get(i).copied().unwrap_or(0.0) as f64),
    l("rises", "mol/m2/s", "log", 1e-5, 1e-2, |c, i| c.w.evap.get(i).copied().unwrap_or(0.0) as f64),
    l("sheds", "mol/m2/s", "lin", -4e-3, 4e-3, |c, i| sheds(c.w, i)),
    l("vapor", "mol/m2", "log", 1.0, 1e5, |c, i| c.vapor[c.air(i)]),
    l("absorber", "1", "lin", 0.0, 5.0, |c, i| c.absorber[c.air(i)]),
    l("plants", "kg/m2", "log", 0.03, 300.0, |c, i| per_land(c.plants[i], c.land[i])),
    l("wood", "kg/m2", "log", 0.01, 300.0, |c, i| per_land(c.wood[i], c.land[i])),
    l("leaf", "m2/m2", "log", 0.01, 30.0, |c, i| per_land(c.leaf[i], c.land[i])),
    l("seeds", "kg/m2", "log", 1e-5, 0.1, |c, i| per_land(c.seeds[i], c.land[i])),
    l("grown", "kg/m2/yr", "log", 0.01, 10.0, |c, i| per_land(c.grown[i], c.land[i])),
    l("takers", "kg/km2", "log", 1.0, 1e4, |c, i| c.takers[i]),
    l("true_r", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][0]),
    l("true_g", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][1]),
    l("true_b", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][2]),
    l("ground_r", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][0]),
    l("ground_g", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][1]),
    l("ground_b", "", "raw", 0.0, 1.0, |c, i| c.f.ground[i][2]),
];

/// Quantise a value to a byte as the frames format defines: none (NaN) is 0, as nothing.
pub fn quantise(l: &Layer, v: f64) -> u8 {
    let b = match l.scale {
        _ if v.is_nan() => 0.0,
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

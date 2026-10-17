//! Surface fields, the layer table and byte quantisation for viewer frames.
//! To add a layer, add one line to `LAYERS` (and the data it needs to `Ctx`).

use pcore::chem::{Phase, Substance};
use pcore::World;

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
}

pub fn fields(w: &World) -> Fields {
    let (n, ns, area) = (w.grid.n(), w.chem.ns(), w.grid.area());
    let mut f = Fields { z: vec![0.0; n], depth: vec![0.0; n], ice: vec![0.0; n], loose: vec![0.0; n], t: vec![0.0; n], rgb: vec![[0.0; 3]; n] };
    for i in 0..n {
        let (t, pl) = w.thaw(i);
        // Kinds of cover in order of priority: frozen, liquid, loose, bonded. Each: volume, volume * colour.
        let mut acc = [[0.0f64; 4]; 4];
        let mut add = |kind: usize, vol: f64, refl: [f32; 3]| {
            acc[kind][0] += vol;
            for c in 0..3 {
                acc[kind][1 + c] += vol * refl[c] as f64;
            }
        };
        for k in 0..ns {
            let s = &w.chem.subs[k];
            let (melt, a) = (w.molten(k, i, t, pl), w.g(k, i));
            let solid = a * (1.0 - melt) * w.chem.molar_volume(k, Phase::Solid);
            add(if is_cover(s) { 0 } else { 2 }, solid, s.refl[0]);
            if t < s.boil {
                add(1, a * melt * w.chem.molar_volume(k, Phase::Liquid), s.refl[1]);
            }
            add(3, w.b(k, i) * w.chem.molar_volume(k, Phase::Solid), s.refl[0]);
        }
        f.t[i] = t;
        f.z[i] = w.surface_z(i);
        (f.ice[i], f.depth[i], f.loose[i]) = (acc[0][0] / area, acc[1][0] / area, acc[2][0] / area);
        let top = acc[..3].iter().find(|a| a[0] / area >= COVER_MIN).unwrap_or(&acc[3]);
        let glow = 1.0 + 2.0 * ((t - GLOW_T) / GLOW_T).clamp(0.0, 1.0);
        f.rgb[i] = [1, 2, 3].map(|c| if top[0] > 0.0 { (top[c] / top[0] * glow).min(1.0) } else { 0.0 });
    }
    f
}

/// Everything a layer, and a row of the series, may read.
pub(crate) struct Ctx<'a> {
    pub w: &'a World,
    pub f: Fields,
    /// Per air node: temperature K, column of condensable gases mol/m², heat-absorbing column (Σ n·ir_abs / area)
    pub t_air: Vec<f64>,
    pub vapor: Vec<f64>,
    pub absorber: Vec<f64>,
    /// Per ground cell: body mass of light-eaters ("plants") and of the others ("animals"), kg/m²
    /// (all members of each node); bodies (nodes and individuals)
    pub plants: Vec<f64>,
    pub animals: Vec<f64>,
    pub bodies: Vec<f64>,
    /// Per ground cell: mass of made objects (parts), kg/m²
    pub parts: Vec<f64>,
    /// Per body slot: a live light-eater (`body::light_eater`)
    pub light: Vec<bool>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(w: &'a World) -> Self {
        let (na, ns) = (w.grid.na(), w.chem.ns());
        let mut cells = vec![0.0; na];
        for i in 0..w.grid.n() {
            cells[w.grid.air_of(i)] += 1.0;
        }
        let n = w.grid.n();
        let (t_air, vapor, absorber) = ((0..na).map(|j| w.t_air(j)).collect(), vec![0.0; na], vec![0.0; na]);
        let light = w.life.bodies.iter().map(|b| b.alive && pcore::body::light_eater(&w.chem, &b.genes)).collect();
        let mut c = Ctx { w, f: fields(w), t_air, vapor, absorber, plants: vec![0.0; n], animals: vec![0.0; n], bodies: vec![0.0; n], parts: vec![0.0; n], light };
        for p in w.parts.alive() {
            c.parts[p.cell as usize] += p.mass(&w.chem) / w.grid.area();
        }
        for (s, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            let i = b.cell as usize;
            let m = b.mass(&w.chem) / w.grid.area();
            *if c.light[s] { &mut c.plants[i] } else { &mut c.animals[i] } += m;
            c.bodies[i] += 1.0;
        }
        for (j, &count) in cells.iter().enumerate() {
            let area = count * w.grid.area();
            for k in 0..ns {
                let (s, n) = (&w.chem.subs[k], w.a(k, j));
                c.absorber[j] += w.chem.depth(k, n / area);
                if s.boil > VAPOR_BOIL {
                    c.vapor[j] += n / area;
                }
            }
        }
        c
    }
    fn air(&self, i: usize) -> usize {
        self.w.grid.air_of(i)
    }
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
    l("loose", "m", "log", 0.01, 100.0, |c, i| c.f.loose[i]),
    l("t_ground", "K", "lin", 220.0, 330.0, |c, i| c.f.t[i]),
    l("t_air", "K", "lin", 220.0, 330.0, |c, i| c.t_air[c.air(i)]),
    l("rain", "mol/m2/s", "log", 1e-6, 1e-2, |c, i| c.w.rain.get(i).copied().unwrap_or(0.0) as f64),
    l("vapor", "mol/m2", "log", 1.0, 1e5, |c, i| c.vapor[c.air(i)]),
    l("absorber", "1", "lin", 0.0, 5.0, |c, i| c.absorber[c.air(i)]),
    l("plants", "kg/m2", "log", 0.03, 300.0, |c, i| c.plants[i]),
    l("animals", "kg/m2", "log", 1e-4, 10.0, |c, i| c.animals[i]),
    l("bodies", "1", "lin", 0.0, 32.0, |c, i| c.bodies[i]),
    l("traffic", "1/yr", "log", 1.0, 1e4, |c, i| c.w.diag.traffic.get(i).copied().unwrap_or(0.0) as f64),
    l("worn", "1", "log", 1.0, 100.0, |c, i| (0..4).map(|f| pcore::lanes::eased(c.w, i, f)).fold(1.0, f64::max)),
    l("structures", "kg/m2", "log", 1e-6, 1.0, |c, i| c.parts[i]),
    l("true_r", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][0]),
    l("true_g", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][1]),
    l("true_b", "", "raw", 0.0, 1.0, |c, i| c.f.rgb[i][2]),
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

/// Append one frame: every layer in table order, row-major.
pub(crate) fn frame(c: &Ctx, out: &mut Vec<u8>) {
    for l in LAYERS {
        out.extend((0..c.w.grid.n()).map(|i| quantise(l, (l.val)(c, i))));
    }
}

//! Quick PNG pictures of a world: map, temperature, rain, the cover and the depth of the cells side by side, and
//! one panel of a window of the map (`window`).

use pcore::World;
use tape::{fields, is_river, pixels, raster, Fields, STANDING_M};

type Rgb = [u8; 3];

/// Ice thicker than this covers the map, m.
const ICE_M: f64 = 0.05;
/// Pixels per cell, the panels side by side and the gap between them.
const PX: usize = 3;
/// The panels of the pictures of a world, in the order they are drawn side by side.
const SIDE_BY_SIDE: usize = 5;
/// The panels `ew view` draws: those of the pictures, the land with no river drawn over it, and of the loose ground
/// (`GROUND`): its conductivity, the liquid it holds for roots, what the year's rain leaves after what rises (its
/// yield), the share of its pores its liquid fills, what rises from it, its texture, the cell's (soil), its organic
/// share, the minerals it holds, how easily flowing liquid carries it off, its slow organic matter, how fast its dead
/// frame rots, and the terrain's sample's texture under each pixel (texture, `SAMPLED`).
pub const PANELS: [&str; 18] = ["map", "temperature", "rain", "life", "depth", "land", "conductivity", "water", "yield", "wetness", "rise", "soil", "organic", "minerals", "erodibility", "slow", "rot", "texture"];
/// The panel drawn by the terrain's samples, not by the cells: the last.
const SAMPLED: usize = PANELS.len() - 1;
/// The first of the loose ground's panels in `PANELS`.
const GROUND: usize = 6;
const GAP: usize = 4;
/// Seconds of a year for the rain panel (1e4 mol/(m² year) is about 0.2 m of liquid).
const YEAR_S: f64 = 3.15e7;

fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|c| (a[c] as f64 + (b[c] as f64 - a[c] as f64) * t) as u8)
}

fn ramp(stops: &[(f64, Rgb)], v: f64) -> Rgb {
    if v <= stops[0].0 {
        return stops[0].1;
    }
    for p in stops.windows(2) {
        if v <= p[1].0 {
            return lerp(p[0].1, p[1].1, (v - p[0].0) / (p[1].0 - p[0].0));
        }
    }
    stops[stops.len() - 1].1
}

/// What the panels colour: the fields of the world, the life's mass per area of each cell, the share of each cell's
/// pores its liquid fills, and the volume of a mol of the key liquid, m³ (for the depths of the rain and the rise).
struct Look<'a> {
    w: &'a World,
    f: Fields,
    /// Mass per area of the cover (green), kg/m².
    green: Vec<f64>,
    wet: Vec<f64>,
    liquid_m3: f64,
    /// What each cell's ground holds of dead matter and minerals, and how fast its frame rots (`soil::grounds`).
    ground: Vec<pcore::soil::Ground>,
}

impl<'a> Look<'a> {
    fn new(w: &'a World) -> Self {
        let cells: Vec<usize> = (0..w.mesh.cells().n()).collect();
        let wet = pcore::flow::means(w, &cells).map_or(vec![0.0; cells.len()], |m| m.iter().map(|m| m.pore / m.pores.max(1e-9)).collect());
        let liquid = pcore::life::Parts::of(&w.chem).map_or(0, |p| p.liquid);
        let liquid_m3 = w.chem.molar_volume(liquid, pcore::chem::Phase::Liquid);
        let ground = pcore::soil::grounds(w).unwrap_or_else(|| vec![Default::default(); cells.len()]);
        Look { w, f: fields(w), green: pcore::cover::per_m2(w, pcore::cover::Cover::kg), wet, liquid_m3, ground }
    }

    /// The minerals the ground of cell `i` holds (`Chemistry::minerals`), mol/m².
    fn minerals(&self, i: usize) -> f64 {
        self.w.chem.minerals().iter().map(|m| self.w.g(m.0, i)).sum::<f64>() / self.w.mesh.cells().area()[i]
    }

    /// The depth of liquid a year of a flux of `mol` mol/(m² s), mm.
    fn mm_a_year(&self, mol: f64) -> f64 {
        mol * self.liquid_m3 * YEAR_S * 1e3
    }

    /// The colour of the loose ground's panel `panel` (index in `PANELS`, from `GROUND`) of cell `i`: a blue grey where
    /// liquid stands over the cell.
    fn ground(&self, panel: usize, i: usize) -> Rgb {
        let (w, f, soil) = (self.w, &self.f, self.w.pores(i));
        if f.depth[i] > STANDING_M {
            return [70, 80, 95];
        }
        match panel - GROUND {
            // Saturated conductivity, m/s, on a log scale: a clay's 1e-7 dark, a sand's 1e-4 pale.
            0 => ramp(&[(-7.0, [60, 30, 20]), (-6.0, [170, 90, 40]), (-5.0, [240, 200, 90]), (-4.0, [250, 250, 220])], soil.seep.log10()),
            // The liquid the loose ground holds between the wilting point and field capacity, mm.
            1 => ramp(&[(0.0, [240, 230, 200]), (40.0, [200, 210, 140]), (120.0, [90, 170, 120]), (300.0, [30, 90, 160])], soil.for_roots(f.loose[i]) * 1e3),
            // Rain less what rises, mm a year: what runs off and sinks.
            2 => ramp(&[(-200.0, [190, 60, 40]), (0.0, [240, 235, 220]), (300.0, [110, 170, 220]), (1000.0, [20, 50, 150])], self.mm_a_year(w.rain[i] as f64 - w.evap[i] as f64)),
            // The share of the pores the liquid fills.
            3 => ramp(&[(0.0, [200, 150, 90]), (0.3, [230, 220, 160]), (0.6, [110, 180, 120]), (1.0, [30, 70, 160])], self.wet[i]),
            // The texture.
            5 => texture(&w.soil[i].texture),
            // What rises, mm a year.
            4 => ramp(&[(0.0, [240, 235, 220]), (300.0, [200, 210, 120]), (700.0, [70, 160, 90]), (1200.0, [20, 90, 70])], self.mm_a_year(w.evap[i] as f64)),
            // Its slow organic matter, kg/m², on a log scale: 0.01 pale, 10 black-brown.
            9 => ramp(&[(-2.0, [235, 225, 200]), (-1.0, [180, 150, 100]), (0.0, [100, 70, 40]), (1.0, [30, 20, 10])], self.ground[i].slow.max(1e-9).log10()),
            // How fast its dead frame rots, a year, on a log scale: 0.01 blue (peat), 1 orange.
            10 => ramp(&[(-2.0, [30, 50, 120]), (-1.0, [100, 150, 170]), (-0.5, [220, 210, 150]), (0.0, [230, 120, 40])], self.ground[i].rot.max(1e-9).log10()),
            // Its erodibility against a loam's, on a log scale: a thirtieth dark, a loam's yellow, four times red.
            8 => ramp(&[(-1.5, [40, 40, 80]), (-0.5, [90, 140, 170]), (0.0, [240, 220, 120]), (0.6, [200, 40, 30])], w.soil[i].texture.erodibility().max(1e-9).log10()),
            // The minerals the ground holds, mol/m², on a log scale: 1e-3 pale, 1 dark green.
            7 => ramp(&[(-3.0, [240, 230, 210]), (-2.0, [200, 200, 140]), (-1.0, [110, 160, 80]), (0.0, [30, 90, 40])], self.minerals(i).max(1e-9).log10()),
            // The organic share of the solids' volume, on a log scale: 1e-5 pale, 1e-2 dark.
            _ => ramp(&[(-5.0, [235, 225, 200]), (-4.0, [190, 160, 110]), (-3.0, [120, 85, 50]), (-2.0, [50, 30, 15])], w.soil[i].texture.organic.max(1e-9).log10()),
        }
    }

    /// The colour of panel `panel` (index in `PANELS`) for cell `i`; `beside` are the cells west and north of it, `step`
    /// the distance to them, m (the map panel shades by the slope to them).
    fn colour(&self, panel: usize, i: usize, beside: (usize, usize), step: f64) -> Rgb {
        let (w, f) = (self.w, &self.f);
        match panel {
            0 => self.map(i, beside, step, true),
            5 => self.map(i, beside, step, false),
            GROUND.. => self.ground(panel, i),
            1 => ramp(&[(240.0, [40, 40, 160]), (273.0, [200, 220, 255]), (288.0, [250, 220, 120]), (305.0, [200, 40, 30])], f.t[i]),
            // Condensed substance falling, mol/(m² year), on a log scale (1e4 is about 0.2 m of liquid).
            2 => ramp(&[(3.0, [235, 225, 200]), (4.0, [190, 200, 140]), (4.5, [110, 180, 110]), (5.0, [40, 110, 170]), (5.5, [20, 40, 120])], (w.rain[i] as f64 * YEAR_S).max(1.0).log10()),
            3 => {
                let base = if f.depth[i] > STANDING_M { [40, 50, 70] } else { [90, 85, 80] };
                ramp(&[(-1.5, base), (0.0, [60, 120, 40]), (1.5, [30, 200, 40])], self.green[i].max(1e-9).log10())
            }
            // The depth of the cell at the square's centre against a square's: coarser dark, a square light grey, each
            // depth finer warmer.
            _ => {
                let finer = w.map.leaves()[i].depth as f64 - w.map.square_depth() as f64;
                ramp(&[(-2.0, [60, 60, 70]), (0.0, [200, 200, 200]), (1.0, [250, 210, 80]), (2.0, [230, 110, 30]), (3.0, [170, 20, 20])], finer)
            }
        }
    }

    /// The map's colour of cell `i`: ice, standing liquid, a river, or land by height, shaded by the slope to the cells
    /// `beside` it (west and north), `step` m away.
    fn map(&self, i: usize, (west, north): (usize, usize), step: f64, rivers: bool) -> Rgb {
        let (w, f) = (self.w, &self.f);
        let river = rivers && is_river(w, i);
        let shade = (1.0 - (((f.z[west] - f.z[i]) + (f.z[north] - f.z[i])) / step) * 6.0).clamp(0.6, 1.3);
        let c = if f.ice[i] > ICE_M {
            [240, 244, 250]
        } else if f.depth[i] > STANDING_M {
            // Standing liquid: shallow light, deep dark.
            ramp(&[(1.0, [90, 160, 215]), (30.0, [50, 110, 190]), (200.0, [25, 60, 140]), (800.0, [10, 30, 80])], f.depth[i])
        } else if river {
            // Flowing liquid, mol/s, on a log scale.
            ramp(&[(4.5, [120, 175, 225]), (5.5, [50, 110, 215]), (6.5, [20, 50, 170])], (w.liquid_flux[i] as f64).log10())
        } else {
            ramp(
                &[(0.0, [120, 160, 90]), (400.0, [150, 170, 100]), (1000.0, [170, 150, 110]), (1800.0, [140, 120, 105]), (2600.0, [200, 200, 200])],
                f.z[i],
            )
        };
        let shade = if f.depth[i] > STANDING_M || river { 1.0 } else { shade };
        c.map(|v| (v as f64 * shade).clamp(0.0, 255.0) as u8)
    }
}

/// The colour of texture `t`: of sand, silt and clay by their shares, darker as it holds more organic matter.
fn texture(t: &pcore::soil::Texture) -> Rgb {
    let (sand, silt, clay) = ([235.0, 200.0, 110.0], [175.0, 175.0, 160.0], [150.0, 60.0, 55.0]);
    let dark = (1.0 - 2.0 * t.organic).clamp(0.3, 1.0);
    [0, 1, 2].map(|c| ((t.coarse * sand[c] + t.middle() * silt[c] + t.fine * clay[c]) * dark).clamp(0.0, 255.0) as u8)
}

fn write_png(path: &str, (across, down): (usize, usize), img: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), across as u32, down as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(img).unwrap();
}

pub fn save(w: &World, path: &str) {
    let look = Look::new(w);
    let (gw, gh) = (w.map.w, w.map.h);
    let (s, panels) = (PX, SIDE_BY_SIDE);
    let (pw, ph) = (gw * s * panels + (panels - 1) * GAP, gh * s);
    let mut img = vec![255u8; pw * ph * 3];
    // A pixel block for each square of the map, of its cell.
    let cells = pixels(w);
    for sq in 0..w.map.n() {
        let ((x, y), i) = (w.map.xy(sq), cells[sq]);
        let beside = (cells[w.map.step(sq, (-1, 0)).unwrap_or(sq)], cells[w.map.step(sq, (0, -1)).unwrap_or(sq)]);
        for p in 0..panels {
            let c = look.colour(p, i, beside, w.map.side);
            for dy in 0..s {
                for dx in 0..s {
                    let px = p * (gw * s + GAP) + x * s + dx;
                    let o = ((y * s + dy) * pw + px) * 3;
                    img[o..o + 3].copy_from_slice(&c);
                }
            }
        }
    }
    write_png(path, (pw, ph), &img);
}

/// Draw panel `panel` (a name of `PANELS`) of `window` (column, row, width and height in squares) `px` pixels wide and as high as the window's shape asks,
/// each the colour of the cell that holds its centre, so cells finer than a square are seen; with `edges`, a pixel
/// whose east or south neighbour is another cell is darkened.
pub fn window(w: &World, window: [f64; 4], px: usize, panel: &str, edges: bool, path: &str) -> Result<(), String> {
    let panel = PANELS.iter().position(|p| *p == panel).ok_or(format!("unknown layer {panel}; the layers are {}", PANELS.join(", ")))?;
    let down = ((px as f64 * window[3] / window[2]).round() as usize).max(1);
    let (look, cells) = (Look::new(w), raster(w, window, px, down));
    // The distance between two pixels, m.
    let step = w.map.side * window[2] / px as f64;
    let sampled = (panel == SAMPLED).then(|| (pcore::soil::samples(w), w.terrain.samples(w.terrain.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north))));
    let mut img = Vec::with_capacity(px * down * 3);
    for p in 0..px * down {
        let (x, y) = (p % px, p / px);
        let beside = (cells[if x > 0 { p - 1 } else { p }], cells[if y > 0 { p - px } else { p }]);
        let edge = edges && ((x + 1 < px && cells[p + 1] != cells[p]) || (y + 1 < down && cells[p + px] != cells[p]));
        let c = match &sampled {
            // The sample under the pixel's centre, standing liquid over its cell as the cell's panels draw it.
            Some((tex, s)) if look.f.depth[cells[p]] <= STANDING_M => {
                let per = s.across as f64 / w.map.w as f64;
                let at = |o: usize, along: f64, n: usize| (((window[o] + (along + 0.5) * window[o + 2] / [px, down][o] as f64) * per) as usize).min(n - 1);
                texture(&tex[at(1, y as f64, s.down) * s.across + at(0, x as f64, s.across)])
            }
            _ => look.colour(panel.min(SAMPLED - 1), cells[p], beside, step),
        };
        img.extend(if edge { c.map(|v| v / 2) } else { c });
    }
    write_png(path, (px, down), &img);
    Ok(())
}

//! Quick PNG pictures of a world: map, temperature, rain, life, traffic and the depth of the cells side by side, and
//! one panel of a window of the map (`window`).

use pcore::World;
use tape::{fields, pixels, raster, Fields, STANDING_M};

type Rgb = [u8; 3];

/// Liquid flux over which a dry land cell is a river in these pictures, mol/s (the web viewer's is `tape::RIVER_WEB`).
pub const RIVER_PNG: f64 = 3.0e4;
/// Ice thicker than this covers the map, m.
const ICE_M: f64 = 0.05;
/// Pixels per cell, the panels side by side and the gap between them.
const PX: usize = 3;
/// The panels of the pictures of a world, in the order they are drawn side by side.
const SIDE_BY_SIDE: usize = 6;
/// The panels `ew view` draws: those of the pictures and the land with no river drawn over it (on a coarse cell every
/// land square is over the river flux, so the map shows no relief there).
pub const PANELS: [&str; 7] = ["map", "temperature", "rain", "life", "traffic", "depth", "land"];
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

/// What the panels colour: the fields of the world and the life's mass per area of each cell.
struct Look<'a> {
    w: &'a World,
    f: Fields,
    /// Mass per area of the cover (green) and of the bodies (orange dots), kg/m².
    green: Vec<f64>,
    other: Vec<f64>,
    /// Cells holding made objects.
    made: Vec<bool>,
}

impl<'a> Look<'a> {
    fn new(w: &'a World) -> Self {
        let mut other = vec![0.0; w.mesh.cells().n()];
        for b in w.life.alive() {
            other[b.cell as usize] += b.mass(&w.chem) / w.mesh.cells().area()[b.cell as usize];
        }
        let mut made = vec![false; w.mesh.cells().n()];
        for p in w.life.parts() {
            made[p.cell as usize] = true;
        }
        Look { w, f: fields(w), green: pcore::cover::per_m2(w), other, made }
    }

    /// The colour of panel `panel` (index in `PANELS`) for cell `i`; `beside` are the cells west and north of it, `step`
    /// the distance to them, m (the map panel shades by the slope to them).
    fn colour(&self, panel: usize, i: usize, beside: (usize, usize), step: f64) -> Rgb {
        let (w, f) = (self.w, &self.f);
        match panel {
            0 => self.map(i, beside, step, true),
            6 => self.map(i, beside, step, false),
            1 => ramp(&[(240.0, [40, 40, 160]), (273.0, [200, 220, 255]), (288.0, [250, 220, 120]), (305.0, [200, 40, 30])], f.t[i]),
            // Condensed substance falling, mol/(m² year), on a log scale (1e4 is about 0.2 m of liquid).
            2 => ramp(&[(3.0, [235, 225, 200]), (4.0, [190, 200, 140]), (4.5, [110, 180, 110]), (5.0, [40, 110, 170]), (5.5, [20, 40, 120])], (w.rain[i] as f64 * YEAR_S).max(1.0).log10()),
            3 => {
                let base = if f.depth[i] > STANDING_M { [40, 50, 70] } else { [90, 85, 80] };
                ramp(&[(-1.5, base), (0.0, [60, 120, 40]), (1.5, [30, 200, 40])], self.green[i].max(1e-9).log10())
            }
            // Bodies moving into the cell per year, on a log scale, over the map in grey.
            4 => {
                let t = w.counts.traffic.get(i).copied().unwrap_or(0.0) as f64;
                let g = self.map(i, beside, step, true).map(|v| v / 3 + 40);
                if t < 1.0 { g } else { ramp(&[(0.0, [90, 70, 120]), (1.0, [200, 60, 160]), (2.0, [255, 160, 40]), (3.0, [255, 255, 200])], t.log10()) }
            }
            // The depth of the cell at the square's centre against a square's: coarser dark, a square light grey, each
            // depth finer warmer (refinement, `World::refine`).
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
        let flux = if rivers { w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64 } else { 0.0 };
        let shade = (1.0 - (((f.z[west] - f.z[i]) + (f.z[north] - f.z[i])) / step) * 6.0).clamp(0.6, 1.3);
        let c = if f.ice[i] > ICE_M {
            [240, 244, 250]
        } else if f.depth[i] > STANDING_M {
            // Standing liquid: shallow light, deep dark.
            ramp(&[(1.0, [90, 160, 215]), (30.0, [50, 110, 190]), (200.0, [25, 60, 140]), (800.0, [10, 30, 80])], f.depth[i])
        } else if flux > RIVER_PNG {
            // Flowing liquid, mol/s, on a log scale.
            ramp(&[(4.5, [120, 175, 225]), (5.5, [50, 110, 215]), (6.5, [20, 50, 170])], flux.log10())
        } else {
            ramp(
                &[(0.0, [120, 160, 90]), (400.0, [150, 170, 100]), (1000.0, [170, 150, 110]), (1800.0, [140, 120, 105]), (2600.0, [200, 200, 200])],
                f.z[i],
            )
        };
        let shade = if f.depth[i] > STANDING_M || flux > RIVER_PNG { 1.0 } else { shade };
        c.map(|v| (v as f64 * shade).clamp(0.0, 255.0) as u8)
    }
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
                    let dot = p == 3 && look.other[i] > 1e-5 && dx == 1 && dy == 1;
                    let c = &if dot { ramp(&[(-5.0, [255, 170, 60]), (-2.0, [255, 30, 0])], look.other[i].log10()) } else { c };
                    // Made objects: a white dot in the corner.
                    let c = &if p == 3 && look.made[i] && dx == 0 && dy == 0 { [255, 255, 255] } else { *c };
                    let px = p * (gw * s + GAP) + x * s + dx;
                    let o = ((y * s + dy) * pw + px) * 3;
                    img[o..o + 3].copy_from_slice(c);
                }
            }
        }
    }
    write_png(path, (pw, ph), &img);
}

/// Draw panel `panel` (a name of `PANELS`) of `window` (column, row, width and height in squares) `px` pixels wide and as high as the window's shape asks,
/// each the colour of the cell that holds its centre, so cells finer than a square are seen; with `edges`, a pixel
/// whose east or south neighbour is another cell is darkened. Bodies and made objects are not marked.
pub fn window(w: &World, window: [f64; 4], px: usize, panel: &str, edges: bool, path: &str) -> Result<(), String> {
    let panel = PANELS.iter().position(|p| *p == panel).ok_or(format!("unknown layer {panel}; the layers are {}", PANELS.join(", ")))?;
    let down = ((px as f64 * window[3] / window[2]).round() as usize).max(1);
    let (look, cells) = (Look::new(w), raster(w, window, px, down));
    // The distance between two pixels, m.
    let step = w.map.side * window[2] / px as f64;
    let mut img = Vec::with_capacity(px * down * 3);
    for p in 0..px * down {
        let (x, y) = (p % px, p / px);
        let beside = (cells[if x > 0 { p - 1 } else { p }], cells[if y > 0 { p - px } else { p }]);
        let edge = edges && ((x + 1 < px && cells[p + 1] != cells[p]) || (y + 1 < down && cells[p + px] != cells[p]));
        let c = look.colour(panel, cells[p], beside, step);
        img.extend(if edge { c.map(|v| v / 2) } else { c });
    }
    write_png(path, (px, down), &img);
    Ok(())
}

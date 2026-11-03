//! Quick PNG pictures of a world: map, temperature, rain and life side by side.

use pcore::World;
use tape::{fields, Fields, STANDING_M};

type Rgb = [u8; 3];

/// Liquid flux over which a dry land cell is a river in these pictures, mol/s (the web viewer's is `tape::RIVER_WEB`).
pub const RIVER_PNG: f64 = 3.0e4;
/// Ice thicker than this covers the map, m.
const ICE_M: f64 = 0.05;
/// Pixels per cell, the panels side by side and the gap between them.
const PX: usize = 3;
const PANELS: usize = 5;
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

/// The colour of square `s` of the map: its cell's, shaded by the slope to the cells of the squares west and north.
fn map_pixel(w: &World, f: &Fields, s: usize) -> Rgb {
    let i = w.mesh.cell_of(s);
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let shade = {
        let zl = f.z[w.mesh.cell_of(w.map.step(s, (-1, 0)).unwrap_or(s))];
        let zu = f.z[w.mesh.cell_of(w.map.step(s, (0, -1)).unwrap_or(s))];
        let s = ((zl - f.z[i]) + (zu - f.z[i])) / w.map.side;
        (1.0 - s * 6.0).clamp(0.6, 1.3)
    };
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

pub fn save(w: &World, path: &str) {
    let f = fields(w);
    let (gw, gh) = (w.map.w, w.map.h);
    let (s, panels) = (PX, PANELS);
    // Life: mass per area of the cover (green) and of the bodies (orange dots), kg/m².
    let (green, mut other) = (pcore::cover::per_m2(w), vec![0.0; w.mesh.cells().n()]);
    for b in w.life.alive() {
        other[b.cell as usize] += b.mass(&w.chem) / w.mesh.cells().area()[b.cell as usize];
    }
    let mut made = vec![false; w.mesh.cells().n()];
    for p in w.life.parts() {
        made[p.cell as usize] = true;
    }
    let (pw, ph) = (gw * s * panels + (panels - 1) * GAP, gh * s);
    let mut img = vec![255u8; pw * ph * 3];
    // A pixel block for each square of the map, of its cell.
    for s in 0..w.map.n() {
        let ((x, y), i) = (w.map.xy(s), w.mesh.cell_of(s));
        let cols = [
            map_pixel(w, &f, s),
            ramp(&[(240.0, [40, 40, 160]), (273.0, [200, 220, 255]), (288.0, [250, 220, 120]), (305.0, [200, 40, 30])], f.t[i]),
            // Condensed substance falling, mol/(m² year), on a log scale (1e4 is about 0.2 m of liquid).
            ramp(&[(3.0, [235, 225, 200]), (4.0, [190, 200, 140]), (4.5, [110, 180, 110]), (5.0, [40, 110, 170]), (5.5, [20, 40, 120])], (w.rain[i] as f64 * YEAR_S).max(1.0).log10()),
            {
                let base = if f.depth[i] > STANDING_M { [40, 50, 70] } else { [90, 85, 80] };
                ramp(&[(-1.5, base), (0.0, [60, 120, 40]), (1.5, [30, 200, 40])], green[i].max(1e-9).log10())
            },
            // Bodies moving into the cell per year, on a log scale, over the map in grey.
            {
                let t = w.counts.traffic.get(i).copied().unwrap_or(0.0) as f64;
                let g = map_pixel(w, &f, s).map(|v| v / 3 + 40);
                if t < 1.0 { g } else { ramp(&[(0.0, [90, 70, 120]), (1.0, [200, 60, 160]), (2.0, [255, 160, 40]), (3.0, [255, 255, 200])], t.log10()) }
            },
        ];
        for (p, c) in cols.iter().enumerate() {
            for dy in 0..s {
                for dx in 0..s {
                    let dot = p == 3 && other[i] > 1e-5 && dx == 1 && dy == 1;
                    let c = &if dot { ramp(&[(-5.0, [255, 170, 60]), (-2.0, [255, 30, 0])], other[i].log10()) } else { *c };
                    // Made objects: a white dot in the corner.
                    let c = &if p == 3 && made[i] && dx == 0 && dy == 0 { [255, 255, 255] } else { *c };
                    let px = p * (gw * s + GAP) + x * s + dx;
                    let o = ((y * s + dy) * pw + px) * 3;
                    img[o..o + 3].copy_from_slice(c);
                }
            }
        }
    }
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), pw as u32, ph as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&img).unwrap();
}

//! Quick PNG pictures of a world: map, temperature, rain and life side by side.

use pcore::World;
use tape::{fields, Fields};

type Rgb = [u8; 3];

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

fn map_pixel(w: &World, f: &Fields, i: usize) -> Rgb {
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let shade = {
        let zl = f.z[w.grid.step(i, (-1, 0)).unwrap_or(i)];
        let zu = f.z[w.grid.step(i, (0, -1)).unwrap_or(i)];
        let s = ((zl - f.z[i]) + (zu - f.z[i])) / w.grid.cell_m;
        (1.0 - s * 6.0).clamp(0.6, 1.3)
    };
    let c = if f.ice[i] > 0.05 {
        [240, 244, 250]
    } else if f.depth[i] > 1.0 {
        // Standing liquid: shallow light, deep dark.
        ramp(&[(1.0, [90, 160, 215]), (30.0, [50, 110, 190]), (200.0, [25, 60, 140]), (800.0, [10, 30, 80])], f.depth[i])
    } else if flux > 3.0e4 {
        // Flowing liquid, mol/s, on a log scale.
        ramp(&[(4.5, [120, 175, 225]), (5.5, [50, 110, 215]), (6.5, [20, 50, 170])], flux.log10())
    } else {
        ramp(
            &[(0.0, [120, 160, 90]), (400.0, [150, 170, 100]), (1000.0, [170, 150, 110]), (1800.0, [140, 120, 105]), (2600.0, [200, 200, 200])],
            f.z[i],
        )
    };
    let shade = if f.depth[i] > 1.0 || flux > 3.0e4 { 1.0 } else { shade };
    c.map(|v| (v as f64 * shade).clamp(0.0, 255.0) as u8)
}

pub fn save(w: &World, path: &str) {
    let f = fields(w);
    let (gw, gh) = (w.grid.w, w.grid.h);
    let s = 3; // pixels per cell
    let panels = 5;
    // Life: mass per area of light-eaters (green) and of the other bodies (orange dots), kg/m².
    let (mut green, mut other) = (vec![0.0; w.grid.n()], vec![0.0; w.grid.n()]);
    for b in w.life.alive() {
        let m = b.mass(&w.chem) / w.grid.area();
        *if pcore::body::light_eater(&w.chem, &b.genes) { &mut green[b.cell as usize] } else { &mut other[b.cell as usize] } += m;
    }
    let mut made = vec![false; w.grid.n()];
    for p in w.parts.alive() {
        made[p.cell as usize] = true;
    }
    let (pw, ph) = (gw * s * panels + (panels - 1) * 4, gh * s);
    let mut img = vec![255u8; pw * ph * 3];
    let n = w.grid.n();
    for i in 0..n {
        let (x, y) = w.grid.xy(i);
        let cols = [
            map_pixel(w, &f, i),
            ramp(&[(240.0, [40, 40, 160]), (273.0, [200, 220, 255]), (288.0, [250, 220, 120]), (305.0, [200, 40, 30])], f.t[i]),
            // Condensed substance falling, mol/(m² year), on a log scale (1e4 is about 0.2 m of liquid).
            ramp(&[(3.0, [235, 225, 200]), (4.0, [190, 200, 140]), (4.5, [110, 180, 110]), (5.0, [40, 110, 170]), (5.5, [20, 40, 120])], (w.rain[i] as f64 * 3.15e7).max(1.0).log10()),
            {
                let base = if f.depth[i] > 1.0 { [40, 50, 70] } else { [90, 85, 80] };
                ramp(&[(-1.5, base), (0.0, [60, 120, 40]), (1.5, [30, 200, 40])], green[i].max(1e-9).log10())
            },
            // Bodies moving into the cell per year, on a log scale, over the map in grey.
            {
                let t = w.diag.traffic.get(i).copied().unwrap_or(0.0) as f64;
                let g = map_pixel(w, &f, i).map(|v| v / 3 + 40);
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
                    let px = p * (gw * s + 4) + x * s + dx;
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

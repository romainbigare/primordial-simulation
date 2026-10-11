//! Quick PNG pictures of a world: map, temperature, rain side by side.

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
    let (x, y) = w.grid.xy(i);
    let flux = w.diag.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let shade = {
        let zl = f.z[w.grid.step(i, (-1, 0)).unwrap_or(i)] + f.depth[w.grid.step(i, (-1, 0)).unwrap_or(i)];
        let zu = f.z[w.grid.step(i, (0, -1)).unwrap_or(i)] + f.depth[w.grid.step(i, (0, -1)).unwrap_or(i)];
        let s = ((zl - f.z[i] - f.depth[i]) + (zu - f.z[i] - f.depth[i])) / w.grid.cell_m;
        (1.0 - s * 6.0).clamp(0.6, 1.3)
    };
    let _ = (x, y);
    let c = if f.ice[i] > 0.05 {
        [240, 244, 250]
    } else if f.depth[i] > 3.0 {
        ramp(&[(3.0, [70, 130, 190]), (100.0, [30, 70, 140]), (800.0, [10, 30, 80])], f.depth[i])
    } else if flux > 2.0e3 {
        ramp(&[(2.0e3, [110, 170, 220]), (1.0e5, [40, 110, 210])], flux)
    } else if f.depth[i] > 0.3 {
        [90, 150, 205]
    } else {
        ramp(
            &[(0.0, [120, 160, 90]), (400.0, [150, 170, 100]), (1000.0, [170, 150, 110]), (1800.0, [140, 120, 105]), (2600.0, [200, 200, 200])],
            f.z[i],
        )
    };
    c.map(|v| (v as f64 * shade).clamp(0.0, 255.0) as u8)
}

pub fn save(w: &World, path: &str) {
    let f = fields(w);
    let (gw, gh) = (w.grid.w, w.grid.h);
    let s = 3; // pixels per cell
    let panels = 3;
    let (pw, ph) = (gw * s * panels + (panels - 1) * 4, gh * s);
    let mut img = vec![255u8; pw * ph * 3];
    let n = w.grid.n();
    let maxrain = w.diag.rain.iter().cloned().fold(1e-12f32, f32::max) as f64;
    for i in 0..n {
        let (x, y) = w.grid.xy(i);
        let cols = [
            map_pixel(w, &f, i),
            ramp(&[(240.0, [40, 40, 160]), (273.0, [200, 220, 255]), (288.0, [250, 220, 120]), (305.0, [200, 40, 30])], f.t[i]),
            ramp(&[(0.0, [235, 225, 200]), (0.3, [120, 190, 120]), (1.0, [20, 60, 160])], w.diag.rain[i] as f64 / maxrain),
        ];
        for (p, c) in cols.iter().enumerate() {
            for dy in 0..s {
                for dx in 0..s {
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

//! PROBE (never committed): the frost test's island (core/tests/cover.rs) by row of latitude: the land cells, the coldest
//! ground of 60 years, the most of the tender kind a month's frost kills, how many months it kills any, and the mean
//! frost gene at the end. frostprobe SOUTH NORTH
use pcore::cover::{self, FROST};
use worldgen::life::{Place, KINDS};

fn main() {
    let a: Vec<f64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: a[0], lat_north: a[1], ..Default::default() };
    let mut w = worldgen::terrain::build(1, worldgen::terrain::island(16, 16), chem, &init, cfg);
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let Place::Field(f) = &KINDS[0].place else { unreachable!() };
    let (mut g, mut low) = (f.genes, f.genes);
    (g[FROST], low[FROST]) = (276.0, 261.0);
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
    for &i in &land {
        cover::place(&mut w, i, g, 0, 0.05);
        cover::place(&mut w, i, low, 0, 0.05);
    }
    let parts = cover::Parts::of(&w.chem).expect("cover parts");
    let lat: Vec<f64> = land.iter().map(|&i| (w.mesh.cells().lat()[i] * 10.0).round() / 10.0).collect();
    let mut rows = lat.clone();
    rows.sort_by(|x, y| y.total_cmp(x));
    rows.dedup();
    let (mut coldest, mut kill, mut months) = (vec![f64::MAX; land.len()], vec![0.0f64; land.len()], vec![0u32; land.len()]);
    for _ in 0..60 * 12 {
        pcore::step(&mut w);
        for (k, &i) in land.iter().enumerate() {
            let act = cover::activity(&w.cfg.phys, &g, cover::wet(&w, &parts, i, w.g(parts.liquid, i)).1);
            let fz = cover::frozen(&w.cfg.phys, w.chem.subs[parts.liquid].melt, g[FROST] as f64, (w.t_ground_last(i), act));
            coldest[k] = coldest[k].min(w.t_ground_last(i));
            kill[k] = kill[k].max(fz);
            months[k] += (fz > 0.0) as u32;
        }
    }
    let mm = parts.mm(&w.chem);
    println!("island {} to {}: lat  cells  coldest K (least, mean)  most killed  months that kill (mean a cell)  mean frost gene K", a[0], a[1]);
    for &r in &rows {
        let at: Vec<usize> = (0..land.len()).filter(|&k| lat[k] == r).collect();
        let n = at.len() as f64;
        let (frost, built) = at.iter().flat_map(|&k| {
            let c = &w.life.cell_at(land[k]).cover;
            c.held().map(move |x| (c.built(&mm, x), c.genes[x][FROST] as f64)).collect::<Vec<_>>()
        }).fold((0.0, 0.0), |a, (b, f)| (a.0 + b * f, a.1 + b));
        println!("  {r:5.1}  {:3}  {:7.1} {:7.1}  {:5.3}  {:6.1}  {:6.1}", at.len(), at.iter().map(|&k| coldest[k]).fold(f64::MAX, f64::min), at.iter().map(|&k| coldest[k]).sum::<f64>() / n, at.iter().map(|&k| kill[k]).fold(0.0, f64::max), at.iter().map(|&k| months[k] as f64).sum::<f64>() / n, frost / built);
    }
}

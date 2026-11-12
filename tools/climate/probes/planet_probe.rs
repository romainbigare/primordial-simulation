//! Probe (not committed): build the planet's air (worldgen::planet), time it, print per row the year's mean air
//! temperature, warmth, coldest and warmest month, vapour share, and the warmth's gradient.
fn main() {
    let seed: u64 = std::env::args().nth(1).map_or(1, |s| s.parse().unwrap());
    pcore::par::adopt_thread();
    let t = std::time::Instant::now();
    let p = worldgen::planet::planet(seed);
    eprintln!("planet in {:.2} s", t.elapsed().as_secs_f64());
    let ns = p.share.len() / (p.lat.len() * p.months);
    let chem = worldgen::chemistry0().0;
    let r0 = p.lat.len() / 2;
    for k in 0..ns { let v = p.share[(r0 * p.months) * ns + k]; if v > 0.0 { eprintln!("share k {k} {v:.3e} lv {}", chem.subs[k].lv); } }
    let water = (0..ns).filter(|&k| chem.subs[k].lv > 0.0 && p.share[(r0 * p.months) * ns + k] > 0.0 && p.share[(r0 * p.months) * ns + k] < 0.1).max_by(|&a, &b| p.share[(r0 * p.months) * ns + a].total_cmp(&p.share[(r0 * p.months) * ns + b])).unwrap_or(0);
    println!("lat t_mean t_min t_max warmth mol_m2 vapour_share t_dec");
    for r in 0..p.lat.len() {
        let v = |x: &[f64]| x[r * p.months..(r + 1) * p.months].to_vec();
        let (t, wm, m) = (v(&p.t), v(&p.warmth), v(&p.mol));
        let mean = |x: &[f64]| x.iter().sum::<f64>() / x.len() as f64;
        let vap = (0..p.months).map(|k| p.share[(r * p.months + k) * ns + water]).sum::<f64>() / p.months as f64;
        println!("{:6.1} {:6.1} {:6.1} {:6.1} {:6.1} {:8.0} {:.5} {:6.1}", p.lat[r], mean(&t), t.iter().cloned().fold(f64::MAX, f64::min), t.iter().cloned().fold(f64::MIN, f64::max), mean(&wm), mean(&m), vap, t[p.months - 1]);
    }
}

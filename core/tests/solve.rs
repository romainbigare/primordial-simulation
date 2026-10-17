//! The shared maths of `solve/`: each solver against an exact answer, big steps against small ones, the books.

// ---- lines: the implicit step along one line (the wind) ----

use pcore::solve::lines::along_line;

/// A line of 40 nodes with uneven amounts and shares that span four orders of magnitude.
fn line_system() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = 40;
    let share = |p: usize, k: f64| 10f64.powf(4.0 * ((p as f64 * 1.7 + k).sin() * 0.5 - 0.5));
    let to_next = (0..len - 1).map(|p| share(p, 0.3)).collect();
    let to_prev = (0..len - 1).map(|p| share(p, 2.1)).collect();
    let x = (0..len).map(|p| 1.0 + 5.0 * ((p as f64 * 0.9).cos() + 1.0)).collect();
    (to_next, to_prev, x)
}

#[test]
fn line_step_solves_its_system_and_keeps_the_total() {
    let (to_next, to_prev, before) = line_system();
    let len = before.len();
    let mut after = before.clone();
    along_line(&to_next, &to_prev, &mut after, &mut vec![0.0; len]);
    // Put the answer back in: x' + (out - in) must give the amounts before.
    let mut flux = vec![0.0; len - 1];
    for p in 0..len - 1 {
        flux[p] = to_next[p] * after[p] - to_prev[p] * after[p + 1];
    }
    let mut worst: f64 = 0.0;
    for p in 0..len {
        let out = if p + 1 < len { flux[p] } else { 0.0 };
        let into = if p > 0 { flux[p - 1] } else { 0.0 };
        worst = worst.max((after[p] + out - into - before[p]).abs());
    }
    assert!(worst < 1e-12, "residual {worst}");
    let (sum_before, sum_after): (f64, f64) = (before.iter().sum(), after.iter().sum());
    assert!((sum_after - sum_before).abs() < 1e-12 * sum_before, "total {sum_before} became {sum_after}");
    assert!(after.iter().all(|&v| v > 0.0), "an amount went negative");
}

/// One implicit step along a line with shares s is close to 100 steps with shares s / 100 (the same total time), and
/// both keep the total. On the slowest mode of a line (equal shares s both ways, so it decays at rate r = 2 s (1 -
/// cos(pi / n)) per unit share), one step keeps 1 / (1 + r) and 100 steps keep (1 + r / 100)^-100, about exp(-r); the
/// two differ by less than r^2 / 2, the first error term of backward Euler.
#[test]
fn line_big_step_is_close_to_small_steps() {
    let n = 40;
    let mode: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos()).collect();
    let before: Vec<f64> = mode.iter().map(|m| 10.0 + m).collect();
    let rate = 0.2;
    let s = rate / (2.0 * (1.0 - (std::f64::consts::PI / n as f64).cos()));
    let step = |share: f64, x: &mut [f64]| along_line(&vec![share; n - 1], &vec![share; n - 1], x, &mut vec![0.0; n]);
    let (mut big, mut small) = (before.clone(), before.clone());
    step(s, &mut big);
    (0..100).for_each(|_| step(s / 100.0, &mut small));
    let total = |x: &[f64]| x.iter().sum::<f64>();
    assert!((total(&big) - total(&before)).abs() < 1e-12 * total(&before));
    assert!((total(&small) - total(&before)).abs() < 1e-12 * total(&before));
    let (keep_big, keep_small) = (1.0 / (1.0 + rate), (1.0 + rate / 100.0).powi(-100));
    for i in 0..n {
        assert!((big[i] - 10.0 - keep_big * mode[i]).abs() < 1e-12 && (small[i] - 10.0 - keep_small * mode[i]).abs() < 1e-12);
        assert!((big[i] - small[i]).abs() < rate * rate / 2.0 * mode[i].abs());
    }
}

// ---- spread: the implicit step along links (backward Euler, conjugate gradient) ----

mod spread {
    use pcore::solve::spread::{Net, Spread};

    /// One step of `spread` on a network; returns the link fluxes and the iterations made.
    fn solve(a: &[u32], b: &[u32], g: &[f64], cap: &[f64], x: &mut [f64], dt: f64, iters: usize) -> (Vec<f64>, usize) {
        let mut flux = vec![0.0; a.len()];
        let made = Spread::default().step(&Net { a, b, g, cap }, x, dt, iters, &mut flux);
        (flux, made)
    }

    /// One step of `spread` on a network; returns the link fluxes.
    fn run(a: &[u32], b: &[u32], g: &[f64], cap: &[f64], x: &mut [f64], dt: f64, iters: usize) -> Vec<f64> {
        solve(a, b, g, cap, x, dt, iters).0
    }

    /// A line of nodes: link p joins nodes p and p + 1.
    fn line(n: usize) -> (Vec<u32>, Vec<u32>) {
        ((0..n as u32 - 1).collect(), (1..n as u32).collect())
    }

    /// The same implicit step on a line, solved directly: (C + dt L) x' = C x is tridiagonal.
    fn direct(g: &[f64], cap: &[f64], x: &[f64], dt: f64) -> Vec<f64> {
        let n = x.len();
        let off = |p: usize| -dt * g[p];
        let main = |p: usize| cap[p] + dt * (if p > 0 { g[p - 1] } else { 0.0 } + if p + 1 < n { g[p] } else { 0.0 });
        let (mut c, mut d) = (vec![0.0; n], vec![0.0; n]);
        for p in 0..n {
            let lo = if p > 0 { off(p - 1) } else { 0.0 };
            let rhs = cap[p] * x[p] - if p > 0 { lo * d[p - 1] } else { 0.0 };
            let m = main(p) - if p > 0 { lo * c[p - 1] } else { 0.0 };
            c[p] = if p + 1 < n { off(p) / m } else { 0.0 };
            d[p] = rhs / m;
        }
        for p in (0..n - 1).rev() {
            d[p] -= c[p] * d[p + 1];
        }
        d
    }

    /// Numbers in [0, 1) from a seed (splitmix64), for irregular networks.
    fn noise(seed: &mut u64) -> f64 {
        *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }

    /// What the nodes hold: the sum of capacity times potential.
    fn held(cap: &[f64], x: &[f64]) -> f64 {
        cap.iter().zip(x).map(|(c, v)| c * v).sum()
    }

    /// Two nodes relax as backward Euler says for one step, toward the exact exponential over many small steps, and to
    /// their mean for a step far longer than their time constant.
    #[test]
    fn two_nodes_relax() {
        let (cap, g, x0) = ([2.0, 3.0], [1.5], [10.0, 0.0]);
        let mean = held(&cap, &x0) / 5.0;
        let rate = g[0] * (1.0 / cap[0] + 1.0 / cap[1]);
        let (dt, gap) = (1.0, x0[0] - x0[1]);
        let mut x = x0;
        let flux = run(&[0], &[1], &g, &cap, &mut x, dt, 2);
        let euler = gap / (1.0 + rate * dt);
        assert!((x[0] - (mean + euler * 3.0 / 5.0)).abs() < 1e-12 && (x[1] - (mean - euler * 2.0 / 5.0)).abs() < 1e-12);
        assert!((flux[0] - dt * g[0] * euler).abs() < 1e-12);
        // Many small steps: the error of backward Euler is (rate dt)^2 / (2 steps) of the exact gap, to first order.
        let steps = 1000;
        let mut x = x0;
        for _ in 0..steps {
            run(&[0], &[1], &g, &cap, &mut x, dt / steps as f64, 2);
        }
        let exact = gap * (-rate * dt).exp();
        assert!(((x[0] - x[1]) - exact).abs() < (rate * dt).powi(2) / steps as f64 * exact);
        // A step a million time constants long lands on the backward Euler value, next to the mean. Its rounding
        // grows with the stiffness rate dt, and a stiff solve needs one iteration more than the two nodes.
        let (dt, mut x) = (1e6, x0);
        run(&[0], &[1], &g, &cap, &mut x, dt, 3);
        let (euler, tol) = (gap / (1.0 + rate * dt), 4.0 * f64::EPSILON * rate * dt * gap);
        assert!((x[0] - (mean + euler * 3.0 / 5.0)).abs() < tol && (x[1] - (mean - euler * 2.0 / 5.0)).abs() < tol);
    }

    /// On a line of unlike nodes and links, a stiff step matches the direct tridiagonal solve of the same system.
    #[test]
    fn line_matches_direct_solve() {
        let n = 12;
        let (a, b) = line(n);
        let cap: Vec<f64> = (0..n).map(|i| 1.0 + (i % 4) as f64 * 3.0).collect();
        let g: Vec<f64> = (0..n - 1).map(|p| 0.5 + (p % 3) as f64 * 2.0).collect();
        let x0: Vec<f64> = (0..n).map(|i| if i < n / 2 { 300.0 } else { 250.0 + i as f64 }).collect();
        let dt = 20.0;
        let want = direct(&g, &cap, &x0, dt);
        let mut x = x0.clone();
        let (flux, made) = solve(&a, &b, &g, &cap, &mut x, dt, 4 * n);
        assert!(made < 4 * n, "a converged solve stops before its cap");
        for (i, (&got, &w)) in x.iter().zip(&want).enumerate() {
            assert!((got - w).abs() < 1e-10 * w.abs(), "node {i}: {got} against {w}");
        }
        for (p, &f) in flux.iter().enumerate() {
            assert!((f - dt * g[p] * (want[p] - want[p + 1])).abs() < 1e-9);
        }
    }

    /// One big step of a smooth profile is close to 100 small ones: on the slowest mode of a line, which decays at
    /// rate r, one step keeps 1 / (1 + r T) and 100 steps keep about exp(-r T); the two differ by less than
    /// (r T)^2 / 2, the first error term of backward Euler.
    #[test]
    fn big_step_is_close_to_small_steps() {
        let n = 40;
        let (a, b) = line(n);
        let (cap, g) = (vec![1.0; n], vec![1.0; n - 1]);
        let mode: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos()).collect();
        let rate = 2.0 * (1.0 - (std::f64::consts::PI / n as f64).cos());
        let span = 0.2 / rate;
        let mut big = mode.clone();
        run(&a, &b, &g, &cap, &mut big, span, n);
        let mut small = mode.clone();
        for _ in 0..100 {
            run(&a, &b, &g, &cap, &mut small, span / 100.0, n);
        }
        let (keep_big, keep_small) = (1.0 / (1.0 + rate * span), (1.0 + rate * span / 100.0).powi(-100));
        for i in 0..n {
            assert!((big[i] - keep_big * mode[i]).abs() < 1e-9 && (small[i] - keep_small * mode[i]).abs() < 1e-9);
            assert!((big[i] - small[i]).abs() < (rate * span).powi(2) / 2.0 * mode[i].abs());
        }
    }

    /// With only 2 iterations on an irregular network (capacities over six orders), the solve is far from converged,
    /// yet what the nodes hold is kept to 1e-12 of it, and each node moves by exactly its fluxes.
    #[test]
    fn books_are_exact_unconverged() {
        let (n, links, seed) = (30, 70, &mut 7u64);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        while a.len() < links {
            let (i, j) = ((noise(seed) * n as f64) as u32, (noise(seed) * n as f64) as u32);
            if i != j {
                a.push(i);
                b.push(j);
            }
        }
        let cap: Vec<f64> = (0..n).map(|_| 10f64.powf(6.0 * noise(seed) - 3.0)).collect();
        let g: Vec<f64> = (0..links).map(|_| 0.1 + 10.0 * noise(seed)).collect();
        let x0: Vec<f64> = (0..n).map(|_| 250.0 + 100.0 * noise(seed)).collect();
        let dt = 1.0;
        let mut solved = x0.clone();
        run(&a, &b, &g, &cap, &mut solved, dt, 10 * n);
        let mut x = x0.clone();
        let flux = run(&a, &b, &g, &cap, &mut x, dt, 2);
        assert!(x.iter().zip(&solved).any(|(p, q)| (p - q).abs() > 1.0), "two iterations do not solve this network");
        let scale = held(&cap, &x0);
        assert!((held(&cap, &x) - held(&cap, &x0)).abs() < 1e-12 * scale);
        for (i, (&xi, &x0i)) in x.iter().zip(&x0).enumerate() {
            let into: f64 = (0..links)
                .map(|k| if b[k] as usize == i { flux[k] } else if a[k] as usize == i { -flux[k] } else { 0.0 })
                .sum();
            assert!((cap[i] * (xi - x0i) - into).abs() < 1e-12 * scale, "node {i} moves by its fluxes");
        }
    }
}

use pcore::chem::Phase;
use pcore::space::square::DIRS;
use pcore::solve::downhill::{self, Adjacency, Order};
use pcore::units::DAY;
use pcore::World;
use std::collections::BTreeMap;
use worldgen::testkit::LIQUID;

// ---- downhill: the order of cells by head, and the pass down it ----

/// One liquid step.
const LIQUID_STEP: f64 = 15.0 * DAY;

/// The links of a square grid to all 8 neighbours, each once, as plain slices.
#[derive(Default)]
struct GridLinks {
    /// The first entry of each cell; the link and the other cell of each entry (in the order of `DIRS`).
    start: Vec<u32>,
    link: Vec<u32>,
    other: Vec<u32>,
    /// The first cell of each link, and its length, m.
    a: Vec<u32>,
    dist: Vec<f64>,
}

/// The links of the grid of `w`.
fn grid_links(w: &World) -> GridLinks {
    let (mut g, mut ids) = (GridLinks { start: vec![0], ..Default::default() }, BTreeMap::new());
    for i in 0..w.grid.n() {
        for d in DIRS {
            if let Some(j) = w.grid.step(i, d) {
                let l = *ids.entry((i.min(j), i.max(j))).or_insert_with(|| {
                    g.a.push(i as u32);
                    g.dist.push(if d.0 != 0 && d.1 != 0 { w.grid.cell_m * std::f64::consts::SQRT_2 } else { w.grid.cell_m });
                    g.a.len() as u32 - 1
                });
                g.link.push(l);
                g.other.push(j as u32);
            }
        }
        g.start.push(g.link.len() as u32);
    }
    g
}

/// Every ground node of `w` brought to 330 K, so all its liquid is liquid through a step of radiation.
fn warm(w: &mut World) {
    for i in 0..w.grid.n() {
        w.e_ground[i] = w.energy_ground(i, 330.0);
    }
}

/// On bare ground with rain on it (no pores, nothing sinking into the bed below), one pass down the order moves the
/// liquid as Flow's sweep does (`flow::liquid`, whose rule for the share that leaves is written here as the pass's
/// rule): the same amount on every cell to rounding. Every amount is kept, and the link fluxes account for every
/// change.
#[test]
fn downhill_moves_surface_liquid_as_the_sweep_does() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (n, area, v, ph) = (w.grid.n(), w.grid.area(), w.chem.molar_volume(LIQUID, Phase::Liquid), w.cfg.phys);
    // Only the liquid on the bed; the exchange with the air and the wind wait for a later step than the next.
    (w.cfg.phys.porosity, w.cfg.phys.seep, w.cfg.carry, w.cfg.wear) = (0.0, 0.0, 0.0, 0.0);
    (w.cfg.exchange_days, w.cfg.air_days) = (30, 30);
    let clear = |w: &mut World| (0..w.chem.ns()).filter(|&k| k != LIQUID).for_each(|k| w.ground[k * n..(k + 1) * n].fill(0.0));
    clear(&mut w);
    w.deep.fill(0.0);
    w.scratch = Default::default();
    warm(&mut w);
    // The first step runs every part of Flow.
    pcore::flow::step(&mut w, LIQUID_STEP);
    // 5 mm of rain on every cell; then only the liquid moves at the next step.
    clear(&mut w);
    w.ground[LIQUID * n..(LIQUID + 1) * n].iter_mut().for_each(|m| *m += 0.005 * area / v);
    warm(&mut w);
    let z: Vec<f64> = (0..n).map(|i| w.surface_z(i)).collect();
    let q0 = w.ground[LIQUID * n..(LIQUID + 1) * n].to_vec();
    pcore::flow::step(&mut w, LIQUID_STEP);
    let sweep = &w.ground[LIQUID * n..(LIQUID + 1) * n];

    let GridLinks { start, link, other, a, dist } = grid_links(&w);
    let adj = Adjacency { start: &start, link: &link, other: &other, a: &a };
    let head = |q: &[f64], i: usize| (z[i] + q[i] * v / area).max(z[i]);
    // The sweep orders heads in steps of 0.1 m down from 20 km, ties by cell.
    let mut order = Order::default();
    order.update(&(0..n).map(|i| -((((2e4 - head(&q0, i)).max(0.0)) * 10.0) as u64 as f64)).collect::<Vec<_>>());
    let (mut q, mut flux) = (q0.clone(), vec![0.0; a.len()]);
    downhill::pass(&order, &adj, &mut q, &mut flux, |i, q, share| {
        // Outlets: lower by more than `still`, weighted by slope; the head may fall to a cell still to come (or to its
        // own bed), and half way to one passed. Outlets much less steep than the steepest get nothing.
        let (hd, zi, es) = (head(q, i), z[i], start[i] as usize..start[i + 1] as usize);
        let (mut out, mut wsum, mut rmax, mut qmax) = ([(0.0, 0.0); 8], 0.0, 0.0_f64, 0.0_f64);
        for (e, o) in es.clone().zip(out.iter_mut()) {
            let (j, dh) = (other[e] as usize, hd - head(q, other[e] as usize));
            if hd > zi && dh > ph.still {
                *o = (dh / dist[link[e] as usize], if order.before(j, i) { 0.5 * dh } else { hd - head(q, j).max(zi) });
                (wsum, rmax, qmax) = (wsum + o.0, rmax.max(o.1), qmax.max(o.0));
            }
        }
        for o in out.iter_mut().filter(|o| o.0 > 0.0 && o.0 < ph.split * qmax) {
            (wsum, o.0) = (wsum - o.0, 0.0);
        }
        // The liquid over the pores (all of it), up to how far the head may fall, shared by weight.
        let lvol = q[i] * v;
        if wsum <= 0.0 || lvol <= 0.0 {
            return;
        }
        let x = lvol.min(area * rmax) / wsum;
        let vols = out.map(|o| (x * o.0).min(area * o.1));
        let qsum: f64 = vols.iter().sum();
        let part = (qsum / lvol).min(1.0);
        share.iter_mut().zip(&vols).for_each(|(s, &vol)| *s = part * vol / qsum);
    });

    // The same amount to 1e-12 of it, or to the rounding of the heads: a share is taken from a difference of heads
    // (a sheet on high ground leaves by the drop of its head to its bed), known to a few EPSILON x the highest head.
    let top = (0..n).map(|i| head(&q0, i).abs()).fold(0.0, f64::max);
    let mut moved = 0;
    for i in 0..n {
        let (s, d) = (sweep[i], q[i]);
        let tol = 1e-12 * s.abs().max(d.abs()) + 4.0 * f64::EPSILON * top * area / v;
        assert!((s - d).abs() <= tol, "cell {i}: the sweep leaves {s} mol, the pass {d}");
        moved += ((d - q0[i]).abs() > 0.01 * q0[i]) as usize;
        let net: f64 = (start[i] as usize..start[i + 1] as usize).map(|e| if a[link[e] as usize] as usize == i { -flux[link[e] as usize] } else { flux[link[e] as usize] }).sum();
        assert!((q0[i] + net - d).abs() <= 1e-12 * q0[i].max(d), "cell {i}: the fluxes bring {net} mol, it changed by {}", d - q0[i]);
    }
    assert!(moved > n / 4, "only {moved} of {n} cells changed");
    let (before, after) = (q0.iter().sum::<f64>(), q.iter().sum::<f64>());
    assert!((after - before).abs() <= 1e-14 * before, "the pass holds {after} mol of {before}");
}

/// Rain is the same whether it falls at once or in 4 parts, each followed by a pass, when the rule is linear in the
/// amounts: here all that a cell holds leaves in equal shares to its lower neighbours (by ground height), so one pass
/// carries it all the way to the cells with no lower neighbour. The cells with a lower neighbour end empty, the
/// sinks hold the same amounts, the link fluxes add up to those of the one pass, and every amount is kept.
#[test]
fn downhill_one_rain_in_one_pass_or_in_parts() {
    let w = worldgen::new_world(1, 32, 32);
    let n = w.grid.n();
    let z: Vec<f64> = (0..n).map(|i| w.surface_z(i)).collect();
    let GridLinks { start, link, other, a, .. } = grid_links(&w);
    let adj = Adjacency { start: &start, link: &link, other: &other, a: &a };
    let mut order = Order::default();
    order.update(&z);
    let rule = |i: usize, _: &mut [f64], share: &mut [f64]| {
        let es = start[i] as usize..start[i + 1] as usize;
        let lower = es.clone().filter(|&e| z[other[e] as usize] < z[i]).count();
        es.zip(share.iter_mut()).filter(|(e, _)| z[other[*e] as usize] < z[i]).for_each(|(_, s)| *s = 1.0 / lower as f64);
    };
    let rain: Vec<f64> = (0..n).map(|i| 1.0 + (i * 37 % 11) as f64).collect();
    let total: f64 = rain.iter().sum();
    let (mut whole, mut whole_flux) = (rain.clone(), vec![0.0; a.len()]);
    downhill::pass(&order, &adj, &mut whole, &mut whole_flux, rule);
    let (mut parts, mut flux, mut part_flux) = (vec![0.0; n], vec![0.0; a.len()], vec![0.0; a.len()]);
    for _ in 0..4 {
        parts.iter_mut().zip(&rain).for_each(|(q, r)| *q += r / 4.0);
        downhill::pass(&order, &adj, &mut parts, &mut part_flux, rule);
        flux.iter_mut().zip(&part_flux).for_each(|(f, p)| *f += p);
    }
    let sinks = (0..n).filter(|&i| (start[i]..start[i + 1]).all(|e| z[other[e as usize] as usize] >= z[i])).count();
    assert!(sinks < n / 2 && whole.iter().filter(|&&q| q > 1e-12 * total).count() == sinks, "only the sinks hold liquid");
    for i in 0..n {
        assert!((whole[i] - parts[i]).abs() <= 1e-12 * total, "cell {i}: {} against {}", whole[i], parts[i]);
    }
    for (l, (f, g)) in whole_flux.iter().zip(&flux).enumerate() {
        assert!((f - g).abs() <= 1e-12 * total, "link {l}: {f} against {g}");
    }
    for q in [&whole, &parts] {
        assert!((q.iter().sum::<f64>() - total).abs() <= 1e-14 * total);
    }
}

/// The order is the order of the heads now, highest first and ties by cell (heads of both signs, many equal), also
/// after some heads change.
#[test]
fn the_order_is_the_order_of_the_heads() {
    let n = 4096u32;
    let sorted = |h: &[f64]| {
        let mut c: Vec<u32> = (0..n).collect();
        c.sort_by(|&a, &b| h[b as usize].total_cmp(&h[a as usize]).then(a.cmp(&b)));
        c
    };
    let mut head: Vec<f64> = (0..n).map(|i| (i * 7919 % 1000) as f64 * 0.1 - 50.0).collect();
    let mut order = Order::default();
    order.update(&head);
    assert_eq!(order.cells(), sorted(&head));
    for i in (0..n as usize).step_by(97) {
        head[i] += 0.35;
    }
    order.update(&head);
    assert_eq!(order.cells(), sorted(&head));
    let c = order.cells();
    assert!(order.before(c[0] as usize, c[1] as usize) && !order.before(c[1] as usize, c[0] as usize));
}

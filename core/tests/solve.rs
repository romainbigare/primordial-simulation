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

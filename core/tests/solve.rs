//! The shared maths of `solve/`: each solver against an exact answer, big steps against small ones, the books.

// ---- band: the implicit step of amounts carried along links, solved directly (the air) ----

use pcore::solve::band::{fluxes, Band, Carry};

/// The ends of the links (`a`, `b`), `fwd`, `back` and the amounts of a test grid.
type Grid = (Vec<u32>, Vec<u32>, Vec<f64>, Vec<f64>, Vec<f64>);

/// A grid of 7 x 5 nodes, each row closed on itself (a ring, as round the globe), with uneven amounts and coefficients
/// that span four orders of magnitude each way.
fn grid() -> Grid {
    let (w, h) = (7, 5);
    let (mut a, mut b) = (vec![], vec![]);
    for y in 0..h {
        for x in 0..w {
            let j = (y * w + x) as u32;
            a.push(j);
            b.push((y * w + (x + 1) % w) as u32);
            if y + 1 < h {
                a.push(j);
                b.push(j + w as u32);
            }
        }
    }
    let share = |l: usize, k: f64| 10f64.powf(4.0 * ((l as f64 * 1.7 + k).sin() * 0.5 - 0.5) + 1.0);
    let (fwd, back) = ((0..a.len()).map(|l| share(l, 0.3)).collect(), (0..a.len()).map(|l| share(l, 2.1)).collect());
    let x = (0..w * h).map(|p| 1.0 + 5.0 * ((p as f64 * 0.9).cos() + 1.0)).collect();
    (a, b, fwd, back, x)
}

/// The amounts after a step put back in: `x' + (out - in) + diag x'`, from the link fluxes.
fn put_back(c: &Carry, after: &[f64], diag: &[f64]) -> Vec<f64> {
    let mut flux = vec![0.0; c.a.len()];
    fluxes(c, after, (1, 0), &mut flux);
    let mut before: Vec<f64> = after.iter().zip(diag).map(|(x, d)| x * (1.0 + d)).collect();
    for (l, &f) in flux.iter().enumerate() {
        (before[c.a[l] as usize], before[c.b[l] as usize]) = (before[c.a[l] as usize] + f, before[c.b[l] as usize] - f);
    }
    before
}

/// The step on a grid with rings solves its system: the answer put back in gives the amounts before; with nothing lost
/// at the nodes the total is kept, and no amount goes negative.
#[test]
fn band_step_solves_its_system_and_keeps_the_total() {
    let (a, b, fwd, back, before) = grid();
    let c = Carry { a: &a, b: &b, fwd: &fwd, back: &back };
    let zero = vec![0.0; before.len()];
    let mut after = before.clone();
    Band::step(before.len(), &c).solve(&mut after);
    let again = put_back(&c, &after, &zero);
    let worst = before.iter().zip(&again).map(|(p, q)| (p - q).abs()).fold(0.0, f64::max);
    assert!(worst < 1e-10, "residual {worst}");
    let (sum_before, sum_after): (f64, f64) = (before.iter().sum(), after.iter().sum());
    assert!((sum_after - sum_before).abs() < 1e-12 * sum_before, "total {sum_before} became {sum_after}");
    assert!(after.iter().all(|&v| v > 0.0), "an amount went negative");
}

/// One implicit step along a line with shares s is close to 100 steps with shares s / 100 (the same total time). On
/// the slowest mode of a line (equal shares s both ways, so it decays at rate r = 2 s (1 - cos(pi / n)) per unit share),
/// one step keeps 1 / (1 + r) and 100 steps keep (1 + r / 100)^-100, about exp(-r); the two differ by less than r^2 / 2,
/// the first error term of backward Euler.
#[test]
fn band_big_step_is_close_to_small_steps() {
    let n = 40;
    let mode: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos()).collect();
    let before: Vec<f64> = mode.iter().map(|m| 10.0 + m).collect();
    let rate = 0.2;
    let s = rate / (2.0 * (1.0 - (std::f64::consts::PI / n as f64).cos()));
    let (a, b): (Vec<u32>, Vec<u32>) = ((0..n as u32 - 1).collect(), (1..n as u32).collect());
    let step = |share: f64, x: &mut [f64]| {
        let v = vec![share; n - 1];
        Band::step(n, &Carry { a: &a, b: &b, fwd: &v, back: &v }).solve(x)
    };
    let (mut big, mut small) = (before.clone(), before.clone());
    step(s, &mut big);
    (0..100).for_each(|_| step(s / 100.0, &mut small));
    let (keep_big, keep_small) = (1.0 / (1.0 + rate), (1.0 + rate / 100.0).powi(-100));
    for i in 0..n {
        assert!((big[i] - 10.0 - keep_big * mode[i]).abs() < 1e-12 && (small[i] - 10.0 - keep_small * mode[i]).abs() < 1e-12);
        assert!((big[i] - small[i]).abs() < rate * rate / 2.0 * mode[i].abs());
    }
}

/// Two fields side by side per node, each carried along the grid, the first losing at each node what the second gives
/// it back (a coupling of the kind of rain and heat), solve their system together: the answer put back in gives the
/// right-hand side.
#[test]
fn band_solves_two_coupled_fields() {
    let (a, b, fwd, back, x) = grid();
    let (n, c) = (x.len(), Carry { a: &a, b: &b, fwd: &fwd, back: &back });
    let reach = a.iter().zip(&b).map(|(&p, &q)| p.abs_diff(q) as usize).max().unwrap();
    let mut band = Band::zero(2 * n, 2 * reach + 1);
    band.carry(&c, 2, 0, 1.0);
    band.carry(&c, 2, 1, 2.0);
    for j in 0..n {
        band.add(2 * j, 2 * j, 3.0);
        band.add(2 * j + 1, 2 * j + 1, 2.0);
        band.add(2 * j, 2 * j + 1, -0.5);
        band.add(2 * j + 1, 2 * j, -1.0);
    }
    let rhs: Vec<f64> = (0..2 * n).map(|r| x[r / 2] * (1.0 + (r % 2) as f64)).collect();
    let mut z = rhs.clone();
    band.factor();
    band.solve(&mut z);
    let field = |f: usize| (0..n).map(|j| z[2 * j + f]).collect::<Vec<f64>>();
    let (u, v) = (field(0), field(1));
    let (pu, pv) = (put_back(&c, &u, &vec![2.0; n]), put_back(&c, &v, &vec![0.0; n]));
    for j in 0..n {
        let (ru, rv) = (pu[j] - 0.5 * v[j], 2.0 * pv[j] - u[j]);
        assert!((ru - rhs[2 * j]).abs() < 1e-9 * rhs[2 * j] && (rv - rhs[2 * j + 1]).abs() < 1e-9 * rhs[2 * j + 1], "node {j}: {ru} {rv}");
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

use pcore::solve::downhill::{self, Adjacency, Crossed, Order};

// ---- downhill: the order of a priority flood, and the pass down it ----

/// Rain is the same whether it falls at once or in 4 parts, each followed by a pass, when the rule is linear in the
/// amounts: here all that a cell holds leaves in equal shares to its lower neighbours (by ground height) still to come,
/// so one pass carries it all the way to the cells with none. The cells with a lower neighbour end empty, the
/// sinks hold the same amounts, the link fluxes add up to those of the one pass, and every amount is kept.
#[test]
fn downhill_one_rain_in_one_pass_or_in_parts() {
    let w = worldgen::new_world(1, 32, 32);
    let n = w.mesh.cells().n();
    let z: Vec<f64> = (0..n).map(|i| w.surface_z(i)).collect();
    let cells = w.mesh.cells();
    let (start, other, a) = (cells.start(), cells.adj_cell(), cells.a());
    let adj = Adjacency { start, link: cells.adj_link(), other, a };
    let mut order = Order::default();
    order.update(&adj, &z, |i| z[i]);
    let down = |i: usize, e: usize| z[other[e] as usize] < z[i] && order.before(i, other[e] as usize);
    let rule = |i: usize, _: &mut [f64], share: &mut [f64]| {
        let es = start[i] as usize..start[i + 1] as usize;
        let lower = es.clone().filter(|&e| down(i, e)).count();
        es.zip(share.iter_mut()).filter(|(e, _)| down(i, *e)).for_each(|(_, s)| *s = 1.0 / lower as f64);
    };
    let rain: Vec<f64> = (0..n).map(|i| 1.0 + (i * 37 % 11) as f64).collect();
    let total: f64 = rain.iter().sum();
    // What crossed each link, added up from the crossings of a pass (one row).
    let add = |flux: &mut [f64], c: &Crossed| c.links.iter().zip(&c.amount).for_each(|(&l, m)| flux[l as usize] += m);
    let (mut whole, mut whole_flux, mut crossed) = (rain.clone(), vec![0.0; a.len()], Crossed::default());
    downhill::pass(&order, &adj, &mut whole, &mut crossed, rule);
    add(&mut whole_flux, &crossed);
    let (mut parts, mut flux) = (vec![0.0; n], vec![0.0; a.len()]);
    for _ in 0..4 {
        parts.iter_mut().zip(&rain).for_each(|(q, r)| *q += r / 4.0);
        downhill::pass(&order, &adj, &mut parts, &mut crossed, rule);
        add(&mut flux, &crossed);
    }
    let sinks = (0..n).filter(|&i| (start[i]..start[i + 1]).all(|e| !down(i, e as usize))).count();
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

/// The order of a valley: a slope, a level stretch, a pit behind a sill, another level stretch, and the lowest cell,
/// dry and then wet up to 1.5 m. Higher spill levels come first; the pit's liquid stands and leaves it before the sill;
/// a level stretch is passed from its far end toward its way out; liquid stands below the lowest cell's head.
#[test]
fn the_order_follows_the_spill_levels() {
    let floor = [9.0, 7.0, 7.0, 7.0, 3.0, 5.0, 2.0, 2.0, 1.0, 0.0];
    let n = floor.len() as u32;
    // A line of cells: link k joins cells k and k + 1.
    let (mut start, mut link, mut other) = (vec![0], vec![], vec![]);
    for i in 0..n {
        for j in [i.wrapping_sub(1), i + 1].into_iter().filter(|&j| j < n) {
            link.push(i.min(j));
            other.push(j);
        }
        start.push(other.len() as u32);
    }
    let a: Vec<u32> = (0..n - 1).collect();
    let adj = Adjacency { start: &start, link: &link, other: &other, a: &a };
    let mut order = Order::default();
    for (wet, standing) in [(0.0, vec![4, 9]), (1.5, vec![4, 8, 9])] {
        order.update(&adj, &floor, |i| floor[i].max(wet));
        assert!((1..n as usize).all(|i| order.before(i - 1, i)), "the pass goes down the valley");
        assert_eq!((0..n as usize).filter(|&i| order.stands[i]).collect::<Vec<_>>(), standing);
        assert_eq!([1, 2, 3, 4, 6].map(|i| order.outlet[i] as usize), [2, 3, 4, 5, 7], "the way out of each level stretch and of the pit");
    }
}

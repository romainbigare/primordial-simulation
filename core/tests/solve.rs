//! The shared maths of `solve/`: each solver against an exact answer, and the books.

// ---- sweep: the explicit step of what moves along links and fills nodes (the liquid under the ground, its heat) ----

use pcore::solve::sweep::{sweep, Net};

/// A sweep moves each link by explicit Euler, `dt g (x_a - x_b)`, where the step is small; a node that holds less than
/// its links would take gives all it holds, in their shares; and a step far past the explicit limit moves no link past
/// half of what would level its two nodes. On a line of five nodes with uneven capacities.
#[test]
fn a_sweep_steps_explicitly_and_overdraws_no_node() {
    let (a, b) = (vec![0u32, 1, 2, 3], vec![1u32, 2, 3, 4]);
    let (g, cap) = (vec![1.0, 2.0, 0.5, 1.0], vec![10.0, 1.0, 5.0, 2.0, 8.0]);
    let x = [5.0, 1.0, 3.0, 0.0, 2.0];
    let net = Net { a: &a, b: &b, g: &g, cap: &cap };
    let plenty = [1e9; 5];
    let small = sweep(&net, &x, 0.01, &plenty);
    for l in 0..4 {
        let want = 0.01 * g[l] * (x[a[l] as usize] - x[b[l] as usize]);
        assert!((small[l] - want).abs() < 1e-15, "link {l}: {} against {want}", small[l]);
    }
    // Node 2 would send to 1 and 3 over 0.2 s far more than it holds: it gives all it holds, in their shares.
    let held = [1e9, 1e9, 0.3, 1e9, 1e9];
    let cut = sweep(&net, &x, 0.2, &held);
    let out = -cut[1] + cut[2];
    assert!((out - 0.3).abs() < 1e-12 && cut[1] < 0.0 && cut[2] > 0.0, "node 2 gives {out}");
    // A step a hundred times its limit: no link carries past half of what levels its two nodes.
    let long = sweep(&net, &x, 100.0, &plenty);
    for l in 0..4 {
        let (i, j) = (a[l] as usize, b[l] as usize);
        let level = (x[i] - x[j]) * cap[i] * cap[j] / (cap[i] + cap[j]);
        assert!((long[l] - 0.5 * level).abs() < 1e-12, "link {l}: {} against {}", long[l], 0.5 * level);
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
    order.update(&adj, &z, |i| z[i], &[]);
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
        order.update(&adj, &floor, |i| floor[i].max(wet), &[]);
        assert!((1..n as usize).all(|i| order.before(i - 1, i)), "the pass goes down the valley");
        assert_eq!((0..n as usize).filter(|&i| order.stands[i]).collect::<Vec<_>>(), standing);
        assert_eq!([1, 2, 3, 4, 6].map(|i| order.outlet[i] as usize), [2, 3, 4, 5, 7], "the way out of each level stretch and of the pit");
    }
}

// ---- line: the implicit step along paths and rings ----

use pcore::solve::line::{factor, lines, solve};

/// A line step is backward Euler: on a ring of five nodes and a path of three, with uneven flows (some links named
/// from their far end) and a step a hundred times past any explicit limit, the end values meet
/// y = x + dt (in - out) at those values, so what the nodes hold is kept, and none goes below zero.
#[test]
fn a_line_step_is_backward_euler_and_keeps_the_books() {
    // Ring 0-1-2-3-4-0, path 5-6-7; link 3 runs from 4 back to 3.
    let (a, b) = (vec![0u32, 1, 2, 4, 4, 5, 7], vec![1u32, 2, 3, 3, 0, 6, 6]);
    let (ca, cb) = (vec![2.0, 0.5, 1.0, 3.0, 0.2, 1.5, 0.7], vec![0.3, 1.0, 2.5, 0.1, 1.2, 0.4, 2.0]);
    let k = [0.1, -0.2, 0.0, 0.3, 0.05, -0.1, 0.2];
    let x = [5.0, 1.0, 3.0, 0.0, 2.0, 4.0, 0.5, 1.0];
    let ls = lines(&a, &b, &(0..7).collect::<Vec<_>>(), 8);
    let dt = 100.0;
    for k in [None, Some(&k[..])] {
        let mut y = [0.0; 8];
        solve(&ls, &factor(&ls, &a, (&ca, &cb), dt), &a, k, &x, &mut y);
        let flow = |l: usize| ca[l] * y[a[l] as usize] - cb[l] * y[b[l] as usize] + k.map_or(0.0, |k| k[l]);
        for (j, (&x, &y)) in x.iter().zip(&y).enumerate() {
            let net: f64 = (0..7).map(|l| if b[l] as usize == j { flow(l) } else if a[l] as usize == j { -flow(l) } else { 0.0 }).sum();
            assert!((y - x - dt * net).abs() < 1e-9 * (1.0 + x.abs()), "node {j}: {y} from {x}");
        }
        assert!((y.iter().sum::<f64>() - x.iter().sum::<f64>()).abs() < 1e-9, "the books");
        assert!(k.is_some() || y.iter().all(|&v| v >= 0.0), "no value below zero: {y:?}");
    }
}

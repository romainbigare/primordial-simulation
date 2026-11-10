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

//! The shared solvers of `core/src/solve/`: each solves its system, and keeps what it moves.

// lines

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

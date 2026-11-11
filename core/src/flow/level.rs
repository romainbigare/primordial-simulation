//! One level of liquid over floors of heights and areas (laws.md 1.2): over floors in any order (`level_of`), and over
//! the parts of a body of standing liquid together, each over its cell's mean surface (`level_over`).

use super::curve::Part;

/// The level at which `vol` m³ fill floors of heights and areas `floors` (m, m²), sorted here: Σ area × (level -
/// floor) over the floors under it is `vol`, m.
pub fn level_of(floors: &mut [(f64, f64)], vol: f64) -> f64 {
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    level_sorted(floors.len(), |k| floors[k], vol)
}

/// `level_of` on the `n` floors `floor(k)`, lowest first: summed in that order up to the first the level does not pass.
fn level_sorted(n: usize, floor: impl Fn(usize) -> (f64, f64), vol: f64) -> f64 {
    let (mut area, mut volume_under) = (0.0, 0.0);
    for k in 0..n {
        let (z, a) = floor(k);
        (area, volume_under) = (area + a, volume_under + a * z);
        let level = (vol + volume_under) / area;
        if k + 1 == n || level <= floor(k + 1).0 {
            return level;
        }
    }
    f64::NAN
}

/// Steps of `level_over` before it takes the parts' floors in order.
const STEPS_MOST: usize = 64;
/// A step of `level_over` this small, a share of the level (of a metre near 0 m), is the rounding of its sums: the
/// level is found.
const LEVEL_ROUNDING: f64 = 1e-13;

/// The level at which `vol` m³ stand over the parts `parts` together, each over its cell's mean surface (m), m: Σ of
/// their volumes under it is `vol`; their lowest floor for none. Newton's steps on that sum, which is convex and
/// piecewise linear in the level, from over all the floors: each lies over the level, and the step from a point over
/// the floors' segment that holds it lands on it, so they reach it exactly; where they take more than `STEPS_MOST`,
/// the floors in order (`in_order`, from the order they took at the body's last level, `order`).
pub(super) fn level_over(parts: &[(&Part, f64)], vol: f64, order: &mut Vec<u32>) -> f64 {
    let spans: Vec<[f64; 2]> = parts.iter().map(|&(p, z)| p.span(z)).collect();
    let (low, high) = spans.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), s| (l.min(s[0]), h.max(s[1])));
    if vol <= 0.0 {
        return low;
    }
    let at = |x: f64| {
        parts.iter().fold((0.0, 0.0), |(v, a), &(p, z)| {
            let (pv, pa) = p.volume_under(x - z);
            (v + pv, a + pa)
        })
    };
    let all = at(f64::INFINITY).1;
    let mut x = high + vol / all;
    for _ in 0..STEPS_MOST {
        let (v, a) = at(x);
        let next = x - (v - vol) / a;
        if (next - x).abs() <= LEVEL_ROUNDING * x.abs().max(1.0) {
            return next;
        }
        x = next;
    }
    in_order(parts, vol, order)
}

/// `level_of` over the floors of `parts`, each part's floors over its cell's mean surface (m), sorted from `order`: the
/// order they took at the body's last level, which the cells' surfaces move a little in a month, so a sort that keeps
/// the runs it finds in order has less to do from there. Where two floors at one height differ in area, the order a
/// sort gives them is not one (and the sums differ in their last bit): there the floors are sorted as `level_of` sorts
/// them. Else every sort gives the one order, so the level is `level_of`'s.
fn in_order(parts: &[(&Part, f64)], vol: f64, order: &mut Vec<u32>) -> f64 {
    let mut floors: Vec<(f64, f64)> = parts.iter().flat_map(|&(p, z)| p.floors().iter().map(move |&(d, a)| (z + d, a))).collect();
    if order.len() != floors.len() {
        *order = (0..floors.len() as u32).collect();
    }
    order.sort_by(|&p, &q| floors[p as usize].0.total_cmp(&floors[q as usize].0));
    let floor = |k: usize| floors[order[k] as usize];
    let one = (1..order.len()).all(|k| floor(k).0.total_cmp(&floor(k - 1).0).is_ne() || floor(k).1.to_bits() == floor(k - 1).1.to_bits());
    if one {
        level_sorted(order.len(), floor, vol)
    } else {
        level_of(&mut floors, vol)
    }
}


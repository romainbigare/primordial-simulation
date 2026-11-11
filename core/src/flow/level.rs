//! One level of liquid over floors of heights and areas (laws.md 1.2): over floors in any order (`level_of`), and over
//! the parts of a body of standing liquid together, each over its cell's mean surface (`level_with`).

use super::curve::Part;

/// The level at which `vol` m³ fill floors of heights and areas `floors` (m, m²), sorted here: Σ area × (level -
/// floor) over the floors under it is `vol`, m.
pub fn level_of(floors: &mut [(f64, f64)], vol: f64) -> f64 {
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    let (mut area, mut volume_under) = (0.0, 0.0);
    for (k, &(z, a)) in floors.iter().enumerate() {
        (area, volume_under) = (area + a, volume_under + a * z);
        let level = (vol + volume_under) / area;
        if floors.get(k + 1).is_none_or(|next| level <= next.0) {
            return level;
        }
    }
    f64::NAN
}

/// Where a level stands over the parts of a body (`level_with`): between two of their floors, where what they take is
/// the volume (`At`); or at a height of floors where what they take steps over it (`Step(at, over)`: under it at `at`,
/// over it at `over`, the next number up), as the room of pores the level reaches there may.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Level {
    At(f64),
    Step(f64, f64),
}

/// What the floors of parts under a level sum to (`level_with`): of each part, how many of its floors lie under it; over
/// all parts, their area, m², their Σ area × height, m³, and what `extra` takes, m³.
struct Under {
    count: Vec<usize>,
    area: f64,
    moment: f64,
    extra: f64,
}

/// The level at which the parts `parts`, each over its cell's mean surface (m), take `vol` m³: what stands over the
/// floors under it, Σ area × (level - floor), and what `extra(q, a)` takes over part `q` where `a` m² of it lie under
/// the level (none for none, and growing with `a`). Their lowest floor for none.
///
/// What they take grows with the level, linearly between two heights of floors; so the level is exact, found as one
/// line through the floors under a level `lo` under it (where that line meets `vol` before the next floor, the level is
/// there; else that floor lies under it too), with a halving of the span between `lo` and a level over it at each turn:
/// the line through the floors under `lo` meets `vol` over the level, as what they take only grows faster above.
pub(super) fn level_with(parts: &[(&Part, f64)], vol: f64, extra: impl Fn(usize, f64) -> f64) -> Level {
    let (low, high) = parts.iter().map(|&(p, z)| p.span(z)).fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), s| (l.min(s[0]), h.max(s[1])));
    let under = |x: f64| {
        let mut u = Under { count: Vec::with_capacity(parts.len()), area: 0.0, moment: 0.0, extra: 0.0 };
        for (q, &(p, z)) in parts.iter().enumerate() {
            let (k, s) = p.below(x - z);
            u.count.push(k);
            (u.area, u.moment, u.extra) = (u.area + s[0], u.moment + s[1] + z * s[0], u.extra + extra(q, s[0]));
        }
        u
    };
    let takes = |u: &Under, x: f64| x * u.area - u.moment + u.extra;
    if vol <= 0.0 || parts.is_empty() {
        return Level::At(low);
    }
    let (mut lo, mut at_lo) = (low, under(low));
    let mut hi = high + vol / under(f64::INFINITY).area;
    loop {
        // The lowest level at which a floor not under `lo` comes under (`entry`): the line through the floors under `lo`
        // holds up to there.
        let next = parts.iter().zip(&at_lo.count).filter_map(|(&(p, z), &k)| p.floors().get(k).map(|f| entry(z, f.0))).fold(f64::INFINITY, f64::min);
        if at_lo.area > 0.0 {
            let x = (vol - at_lo.extra + at_lo.moment) / at_lo.area;
            if x < next {
                return Level::At(x.max(lo));
            }
            hi = hi.min(x);
        }
        // The floors that come under at `next` taken in: where what they take steps over `vol` there, the level stands
        // at them.
        let at_next = under(next);
        if takes(&at_next, next) > vol {
            return Level::Step(next.next_down(), next);
        }
        (lo, at_lo) = (next, at_next);
        let mid = 0.5 * (lo + hi);
        if mid > lo && mid < hi {
            let at_mid = under(mid);
            if takes(&at_mid, mid) <= vol {
                (lo, at_lo) = (mid, at_mid);
            } else {
                hi = mid;
            }
        }
    }
}

/// The lowest level at which a floor `d` over a cell's mean surface at `z` lies under it (`Part::below`: `d < x - z`
/// as rounded), m.
fn entry(z: f64, d: f64) -> f64 {
    let mut x = z + d;
    while x - z <= d {
        x = x.next_up();
    }
    while (x.next_down() - z) > d {
        x = x.next_down();
    }
    x
}

/// `level_with` where only what stands over the floors counts: a level between floors, or at a height of floors.
pub(super) fn level_over(parts: &[(&Part, f64)], vol: f64) -> f64 {
    match level_with(parts, vol, |_, _| 0.0) {
        Level::At(x) | Level::Step(x, _) => x,
    }
}

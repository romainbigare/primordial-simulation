//! Two parents: the partner of a mature body (`partner`), and the genes and born rows its children take from each
//! (`mix`, `mix_rows`).

use crate::body::{distance, spread_at, Body, Layout};
use crate::rng::{Purpose, Rng};
use crate::World;
use crate::{body, brain};

/// Up to this many children, `share` draws one by one.
const FEW: f64 = 32.0;
/// From this many children taking from the rarer parent (on average) `share` draws a normal, below it a Poisson.
const NORMAL_FROM: f32 = 10.0;

/// The partner of body `b` (slot `slot`, taken out of it) for its children: another body of its cell that is mature
/// (`Body::adult`) and alike to it (`body::alike`), the most alike of them (the least squared gap in steps,
/// `body::distance`), a keyed random pick among equals; never itself. None: the children are copies of one parent. It
/// takes nothing from the partner. `Config::two_parents` off: none.
pub(super) fn partner(w: &World, slot: usize, b: &Body, r: &mut Rng) -> Option<usize> {
    if !w.cfg.two_parents {
        return None;
    }
    let (lay, lim) = (Layout::of(&w.chem), w.cfg.phys.alike);
    // Among equals the one with the least key (a draw keyed by the call and the slot, not by the order of the cell's list).
    let (mut best, mut pick, draw) = (f32::INFINITY, None, r.u64());
    for &s in &w.life.at[b.cell as usize] {
        let (s, t) = (s as usize, &w.life.bodies[s as usize]);
        if s == slot || !t.alive {
            continue;
        }
        // (The genes first, as they end at the first one unlike; the size of the body last.)
        let Some(d) = distance(&lay, &b.genes, &t.genes, |_| lim).filter(|&d| d <= best) else { continue };
        if !t.adult(&w.chem) {
            continue;
        }
        let key = Rng::new(draw, 0, s as u64, Purpose::Partner).u64();
        if d < best || pick.is_some_and(|(_, k)| key < k) {
            (best, pick) = (d, Some((s, key)));
        }
    }
    pick.map(|p| p.0)
}

/// The share of `m` children that take a thing from their second parent, whose share of the members of both is `p` (a
/// half for two individuals; a node's partner of a few members is a few of the partners it would meet, so its children
/// take little from it, and the mean over all members is kept, as a merge keeps it). One child, or less than one, takes
/// it from one parent or the other (0 or 1, 1 with chance `p`); a group of many takes about `p` of it, a binomial of `m`
/// draws.
fn share(m: f64, p: f32, r: &mut Rng) -> f32 {
    if m <= 1.0 {
        return (r.f64() < p as f64) as u8 as f32;
    }
    if m <= FEW {
        let n = m.round();
        return (0..n as usize).filter(|_| r.f64() < p as f64).count() as f32 / n as f32;
    }
    // Many: the number taking it from the less likely parent (mean m q), a normal when m q is large, else a Poisson (a
    // normal clamped at 0 would give the rare parent more than its due).
    let (q, m) = (p.min(1.0 - p), m as f32);
    let k = m * q;
    let c = if k >= NORMAL_FROM {
        (k + (k * (1.0 - q)).sqrt() * body::noise(r)).clamp(0.0, m)
    } else {
        let (mut c, mut t, e) = (0.0f32, r.f64(), (-k as f64).exp());
        while t > e {
            (c, t) = (c + 1.0, t * r.f64());
        }
        c.min(m)
    };
    if p <= 0.5 { c / m } else { 1.0 - c / m }
}

/// The share of the members of `a` and `t` that are `t`'s (`share`).
fn weight(a: &Body, t: &Body) -> f32 {
    (t.n() / (a.n() + t.n())) as f32
}

/// The genes and the spread of `m` children of `a` and its partner `t`: each gene from one of them, drawn gene by gene
/// (`share`; `t`'s with the chance `weight`); for a group of many the mean between them by the share drawn, and the
/// spread what a merge pools (`nodes::take_in`). One child gets one parent's gene and that parent's spread. The
/// generation's mutation (`body::vary`) is added after, as for one parent.
pub(super) fn mix(a: &Body, t: &Body, m: f64, r: &mut Rng) -> (Vec<f32>, Vec<f32>) {
    let p = weight(a, t);
    (0..a.genes.len())
        .map(|g| {
            let (x, d, f) = (a.genes[g], t.genes[g] - a.genes[g], share(m, p, r));
            (x + f * d, (1.0 - f) * spread_at(&a.spread, g) + f * spread_at(&t.spread, g) + f * (1.0 - f) * d * d)
        })
        .unzip()
}

/// The born rows of the children of `a` and its partner `t` (`t`'s carried over to the sizes of `a`'s brain if they
/// differ): each row from one of them, drawn row by row (`share`; a row is one reader of the hidden layer, so a row kept
/// whole stays coherent, and a weight drawn alone would not), the group's mean between them as in `mix`; and the spread
/// of the rows pooled as a merge pools it. None where a brain has no rows alike (a brain of no size, or rows of other
/// lengths): the children keep `a`'s.
pub(super) fn mix_rows(w: &World, a: &Body, t: &Body, m: f64, r: &mut Rng) -> Option<(Vec<f32>, f32)> {
    let (sa, st) = (w.life.shape_of(a.lineage)?, w.life.shape_of(t.lineage)?);
    let (ra, rt) = (w.life.born(a), w.life.born(t));
    let moved;
    let rt = if sa == st { rt } else {
        moved = brain::rerow(rt, st, sa, r, w.cfg.phys.new_noise);
        &moved
    };
    if ra.len() != rt.len() || ra.len() != sa.rows() * sa.r() {
        return None;
    }
    let (mut mean, mut gap, mut f_sum, p) = (Vec::with_capacity(ra.len()), 0.0, 0.0, weight(a, t));
    for (x, y) in ra.chunks_exact(sa.r()).zip(rt.chunks_exact(sa.r())) {
        let f = share(m, p, r);
        f_sum += f;
        for (&p, &q) in x.iter().zip(y) {
            mean.push(p + f * (q - p));
            gap += f * (1.0 - f) * (q - p) * (q - p);
        }
    }
    let f = f_sum / (ra.len() / sa.r()) as f32;
    Some((mean, (1.0 - f) * a.row_spread + f * t.row_spread + gap / ra.len() as f32))
}

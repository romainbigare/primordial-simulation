//! Two parents: the partner of a mature body (`partner`), and the genes and born rows its children take from each
//! (`mix`, `mix_rows`).

use crate::body::{distance, spread_at, Body, Layout};
use crate::rng::{Purpose, Rng};
use crate::{life::{Chunk, Handle}, World};
use crate::brain;

/// The partner of body `b` (taken out of its slot) for its children: another body of its cell that is mature
/// (`Body::adult`) and alike to it (`body::alike`), the most alike of them (the least squared gap in steps,
/// `body::distance`), a keyed random pick among equals; never itself. None: the children are copies of one parent. It
/// takes nothing from the partner. `Config::two_parents` off: none.
pub(super) fn partner(w: &World, d: &Chunk, b: &Body, r: &mut Rng) -> Option<Handle> {
    if !w.cfg.two_parents {
        return None;
    }
    let (lay, lim) = (Layout::of(&w.chem), w.cfg.phys.alike);
    // Among equals the one with the least key (a draw keyed by the call and its id, not by the order of the cell's list).
    let (mut best, mut pick, draw) = (f32::INFINITY, None, r.u64());
    for s in d.cell(b.cell as usize) {
        let t = &d[s];
        if !t.alive || t.leaving {
            continue;
        }
        // (The genes first, as they end at the first one unlike; the size of the body last.)
        let Some(d) = distance(&lay, &b.genes, &t.genes, |_| lim).filter(|&d| d <= best && t.adult(&w.chem)) else { continue };
        let key = Rng::new(draw, 0, t.id, Purpose::Partner).u64();
        if d < best || pick.is_some_and(|(_, k)| key < k) {
            (best, pick) = (d, Some((s, key)));
        }
    }
    pick.map(|p| p.0)
}

/// The share of `m` children (at least one) that take a thing from their second parent: each child takes each thing from
/// one parent or the other, by even chances (the partner is one parent, whether it is an individual or a node: it gives
/// the genes of one of its members, its mean), so a group of many takes about half of it, a binomial of `m` draws.
fn share(m: f64, r: &mut Rng) -> f32 {
    let n = m.round().max(1.0) as usize;
    (0..n).filter(|_| r.f64() < 0.5).count() as f32 / n as f32
}

/// The genes and the spread of `m` children of `a` and its partner `t`: each gene from one of them, drawn gene by gene
/// (`share`); for a group of many the mean between them by the share drawn, and the
/// spread what a merge pools (`nodes::add`). One child gets one parent's gene and that parent's spread. The
/// generation's mutation (`body::vary`) is added after, as for one parent.
pub(super) fn mix(a: &Body, t: &Body, m: f64, r: &mut Rng) -> (Vec<f32>, Vec<f32>) {
    (0..a.genes.len())
        .map(|g| {
            let (x, d, f) = (a.genes[g], t.genes[g] - a.genes[g], share(m, r));
            (x + f * d, (1.0 - f) * spread_at(&a.spread, g) + f * spread_at(&t.spread, g) + f * (1.0 - f) * d * d)
        })
        .unzip()
}

/// The born rows of the children of `a` and its partner `t` (`t`'s carried over to the sizes of `a`'s brain if they
/// differ): each row from one of them, drawn row by row (`share`; a row is one reader of the hidden layer, so a row kept
/// whole stays coherent, and a weight drawn alone would not), the group's mean between them as in `mix`; and the spread
/// of the rows pooled as a merge pools it. None where a brain has no rows alike (a brain of no size, or rows of other
/// lengths): the children keep `a`'s.
pub(super) fn mix_rows(w: &World, d: &Chunk, a: &Body, t: &Body, m: f64, r: &mut Rng) -> Option<(Vec<f32>, f32)> {
    let (sa, st) = (d.shape_of(w, a.lineage)?, d.shape_of(w, t.lineage)?);
    let (ra, rt) = (d.born(w, a), d.born(w, t));
    let moved;
    let rt = if sa == st { rt } else {
        moved = brain::rerow(rt, st, sa, r, w.cfg.phys.new_noise);
        &moved
    };
    if ra.len() != rt.len() || ra.len() != sa.rows() * sa.r() {
        return None;
    }
    let (mut mean, mut gap, mut f_sum) = (Vec::with_capacity(ra.len()), 0.0, 0.0);
    for (x, y) in ra.chunks_exact(sa.r()).zip(rt.chunks_exact(sa.r())) {
        let f = share(m, r);
        f_sum += f;
        for (&p, &q) in x.iter().zip(y) {
            mean.push(p + f * (q - p));
            gap += f * (1.0 - f) * (q - p) * (q - p);
        }
    }
    let f = f_sum / (ra.len() / sa.r()) as f32;
    Some((mean, (1.0 - f) * a.row_spread + f * t.row_spread + gap / ra.len() as f32))
}

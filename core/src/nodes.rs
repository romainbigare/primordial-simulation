//! The monthly merge of alike nodes in a crowded cell.

use crate::body::{light_eater, Layout};
use crate::World;

/// The speed shortcut of plan 7.1, not a law: it coarsens only how members are represented (as
/// population nodes), never how many there are or where. Monthly, in each cell, among the bodies
/// that hold and are inside nothing: a node joins the first larger one alike in genes
/// (`copy::alike`, on the means only, so a split's halves stay apart for selection; never for
/// being of the same lineage) and of its size (member bodies within `alike` steps of the
/// copy-mass gene, and both mature or both immature, `copy::class`); then, while more than
/// `cfg.crowd` remain, the two most alike (squared distance of their genes in mutation steps,
/// `copy::steps`) merge, the smaller into the larger, if they are alike: unlike nodes are not
/// merged, unless the cell holds more than twice `crowd`: then the most alike merge however
/// unlike, down to that, a numerical safeguard (with no ceiling one 64x64 world gathered about
/// 146,000 bodies in one cell within a few years). Stores and heat add up, genes and born rows
/// become the means by count and the spread of both is kept in the merged node's (`copy::join`),
/// the born rows carried over to the larger's brain where its sizes differ (`copy::merge`);
/// lineage, brain group and memory are the larger's. A node that grew too varied splits again at
/// its next update (`copy::split`). With `cfg.nodes` off it leaves animals out of the list: they
/// never merge and never count toward `crowd`.
/// A body's genes in mutation steps, its spread in steps and its size class (`crowd`).
type Likeness = (Vec<f32>, Vec<f32>, (f64, bool));

pub(crate) fn crowd(w: &mut World) {
    let (max, lay) = (w.cfg.crowd.max(1), Layout::of(&w.chem));
    for i in 0..w.grid.n() {
        if w.life.at[i].len() <= 1 {
            continue;
        }
        let b = &w.life.bodies;
        let mut list: Vec<(u64, usize)> = w.life.at[i].iter().map(|&s| s as usize).filter(|&s| b[s].hold == 0 && b[s].inside == 0 && (w.cfg.nodes || light_eater(&w.chem, &b[s].genes))).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
        list.sort_unstable();
        let mut head: Vec<usize> = (0..list.len()).collect();
        // Each one's class once (`copy::class`), tested before the hundred genes.
        let class: Vec<(f64, bool)> = list.iter().map(|&(_, s)| crate::copy::class(&w.chem, &b[s])).collect();
        for x in 0..list.len() {
            let a = &b[list[x].1];
            if a.n() > 1.0 {
                let lim = w.cfg.phys.alike;
                head[x] = (0..x).find(|&y| head[y] == y && class[y].1 == class[x].1 && crate::copy::alike_log(&lay, lim, class[y].0, class[x].0) && crate::copy::alike(&lay, lim, &b[list[y].1].genes, &a.genes)).unwrap_or(x);
            }
        }
        for x in (0..list.len()).filter(|&x| head[x] != x) {
            crate::copy::merge(w, list[x].1, list[head[x]].1);
        }
        let mut left: Vec<usize> = (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect();
        if left.len() <= max {
            continue;
        }
        // The most alike pair first (ties: the earlier, larger ones), until `max` are left or no
        // pair is alike: here each within the other's spread, as children join a node
        // (`copy::within`); past twice `max`, unlike pairs too (the most alike of them first). A
        // pair's key: unlike or not, then its squared distance in steps.
        let (m, lim) = (left.len(), w.cfg.phys.alike);
        let steps = |w: &World, s: usize| (crate::copy::steps(&lay, &w.life.bodies[s].genes), crate::copy::spread_steps(&lay, &w.life.bodies[s]), crate::copy::class(&w.chem, &w.life.bodies[s]));
        let mut v: Vec<Likeness> = left.iter().map(|&s| steps(w, s)).collect();
        let reach = lim as f64 * crate::copy::size_step(&lay) as f64;
        let dist = |p: &Likeness, q: &Likeness| {
            let gap = |g: usize| (p.0[g] - q.0[g]).abs() > lim + crate::copy::WITHIN * (p.1[g] + q.1[g]).sqrt();
            (p.2 .1 != q.2 .1 || (p.2 .0 - q.2 .0).abs() > reach || (0..p.0.len()).any(gap), p.0.iter().zip(&q.0).map(|(x, y)| (x - y) * (x - y)).sum::<f32>())
        };
        let mut d = vec![(true, f32::INFINITY); m * m];
        for p in 0..m {
            for q in p + 1..m {
                d[p * m + q] = dist(&v[p], &v[q]);
            }
        }
        for n in (max..m).rev() {
            let k = (0..m * m).fold(0, |k, x| if d[x] < d[k] { x } else { k });
            if d[k].1 == f32::INFINITY || d[k].0 && n < 2 * max {
                break;
            }
            let (p, q) = (k / m, k % m);
            let (s, t) = if w.life.bodies[left[q]].n() > w.life.bodies[left[p]].n() { (p, q) } else { (q, p) };
            // Count the members of the smaller node when the pair is unlike (a tally, read by the tape).
            if d[k].0 {
                w.diag.unlike += w.life.bodies[left[s]].n();
            }
            crate::copy::merge(w, left[s], left[t]);
            (left[s], v[t]) = (usize::MAX, steps(w, left[t]));
            for o in 0..m {
                let (lo, hi) = (o.min(s), o.max(s));
                d[lo * m + hi] = (true, f32::INFINITY);
                if o != t && left[o] != usize::MAX {
                    let (lo, hi) = (o.min(t), o.max(t));
                    d[lo * m + hi] = dist(&v[o], &v[t]);
                }
            }
        }
    }
}

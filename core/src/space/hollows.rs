//! The hollows the high stand's lobes leave (laws.md 0, `space::delta`): the liquid a lobe cuts off from the rest of
//! its body of liquid, and the valleys behind the lobes, filled to where their liquid would spill out of them, as the
//! fine load fills a lagoon or a dammed valley over the stand; each hollow by the mouth whose lobe shut it.

use super::delta::Liquid;
use super::fill::Step;
use super::grid::Grid;
use super::network::NONE;
use std::collections::BinaryHeap;

/// A raise the hollows take: the sample, by how much, m, and the mouth whose lobe shut the hollow.
pub(super) type Lift = (usize, f64, u32);

/// The hollows the lobes leave on the heights `built` over the heights before them `before`, each sample's lobe in
/// `shut_by` (`NONE` for none): a flood from the open liquid of `liquid` still joined to the most of its own body (each
/// part of the open low ground before the lobes keeps its largest part after them; another body of liquid is not cut
/// off) over what the lobes raised, the liquid they cut off and the ground behind them (`behind`), each sample up to the
/// lowest height its liquid could leave it by (Barnes, Lehman and Mulla 2014), charged to the lobe of most load the
/// flood's way to it crossed; one the flood does not reach (shut in by higher ground), up to the lowest ground of a lobe round it, charged
/// to that lobe. Raises `built` and returns the raises; a hollow no lobe shut is left.
pub(super) fn hollows(g: &Grid, (built, before): (&mut [f64], &[f64]), shut_by: &[u32], (liquid, behind): (&Liquid, &[bool])) -> Vec<Lift> {
    let n = g.n();
    let wet: Vec<bool> = (0..n).map(|k| liquid.open[k] && built[k] < liquid.level[k]).collect();
    let main = kept(g, liquid.open, &wet);
    let laid: Vec<bool> = (0..n).map(|k| built[k] > before[k] || (wet[k] && !main[k]) || behind[k]).collect();
    // (Of each sample, the lowest height the flood reached it at so far, and the lobe it came over last on that way.)
    let (mut done, mut best, mut from) = ((0..n).map(|k| !laid[k]).collect::<Vec<bool>>(), vec![f64::INFINITY; n], vec![NONE; n]);
    let mut heap = BinaryHeap::new();
    for k in (0..n).filter(|&k| !done[k] && g.around(k).any(|(j, _)| main[j])) {
        (best[k], from[k]) = (built[k].max(liquid.level[k]), shut_by[k]);
        heap.push(Step(best[k], k));
    }
    let mut lifts = Vec::new();
    while let Some(Step(at, k)) = heap.pop() {
        if done[k] || at > best[k] {
            continue;
        }
        done[k] = true;
        // (Of the lobes the way crossed, the one of the most load: the lobes are numbered largest first.)
        let by = shut_by[k].min(from[k]);
        lift((k, at, by), built, &mut lifts);
        for (j, _) in g.around(k).filter(|&(j, _)| !done[j]) {
            let to = built[j].max(at);
            if to < best[j] {
                (best[j], from[j]) = (to, by);
                heap.push(Step(to, j));
            }
        }
    }
    for k in (0..n).filter(|&k| !done[k]) {
        let rim = g.around(k).filter(|&(j, _)| !laid[j] && shut_by[j] != NONE).map(|(j, _)| (built[j], shut_by[j])).min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((at, by)) = rim {
            lift((k, at.max(liquid.level[k]), by), built, &mut lifts);
        }
    }
    lifts
}

/// Sample `k` raised to `at`, m, where it stands lower and a lobe `by` shut its hollow: on `built`, noted in `lifts`.
fn lift((k, at, by): (usize, f64, u32), built: &mut [f64], lifts: &mut Vec<Lift>) {
    if at > built[k] && by != NONE {
        lifts.push((k, at - built[k], by));
        built[k] = at;
    }
}

/// Of the samples `wet` of grid `g`, those in the largest part (joined through their sides and corners, the least index
/// where two tie) of each part of `open` they lie in: the liquid each body keeps joined.
fn kept(g: &Grid, open: &[bool], wet: &[bool]) -> Vec<bool> {
    let (body, _) = parts(g, open);
    let (piece, sizes) = parts(g, wet);
    let mut best: Vec<(usize, u32)> = vec![(0, NONE); g.n()];
    for k in (0..g.n()).filter(|&k| wet[k]) {
        let (b, p) = (body[k] as usize, piece[k]);
        let size = sizes[p as usize];
        if size > best[b].0 || (size == best[b].0 && p < best[b].1) {
            best[b] = (size, p);
        }
    }
    (0..g.n()).map(|k| wet[k] && best[body[k] as usize].1 == piece[k]).collect()
}

/// The parts of the samples `of` of grid `g` joined through their sides and corners: of each sample its part (`NONE`
/// where not of them), and the samples of each part.
fn parts(g: &Grid, of: &[bool]) -> (Vec<u32>, Vec<usize>) {
    let n = g.n();
    let mut part = vec![NONE; n];
    let mut sizes = Vec::new();
    for k in (0..n).filter(|&k| of[k]) {
        if part[k] != NONE {
            continue;
        }
        let id = sizes.len() as u32;
        let (mut todo, mut size) = (vec![k], 0);
        part[k] = id;
        while let Some(i) = todo.pop() {
            size += 1;
            for (j, _) in g.around(i) {
                if of[j] && part[j] == NONE {
                    part[j] = id;
                    todo.push(j);
                }
            }
        }
        sizes.push(size);
    }
    (part, sizes)
}

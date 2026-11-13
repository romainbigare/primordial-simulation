//! The rule by which what lands in a cell finds its place among the few entries the cell keeps (laws.md 3): the slots
//! of its cover (`seed::lodge`, `stand::merge_alike`) and the lots of the seeds lying on its ground (`lying`). An entry
//! is its genes and its starting kind (`NO_ROOT`: empty); alike entries of a starting kind are one (`APART` ×
//! `Physics::alike` steps in every gene, `distance`), and when all are held the two nearest join, since the entries are
//! a limit of the bookkeeping and the two hold the same niche to within what it resolves.

use super::{distance, APART, NG, NO_ROOT};

/// Of the entries of genes `genes` and starting kinds `root`, the most alike of starting kind `r` to genes `g` within
/// `APART` × `alike` steps.
pub(super) fn kin(genes: &[[f32; NG]], root: &[u32], (g, r): (&[f32; NG], u32), alike: f32) -> Option<usize> {
    let of_kind = (0..root.len()).filter(|&x| root[x] == r);
    of_kind.map(|x| (distance(&genes[x], g), x)).filter(|e| e.0 <= APART * alike).min_by(|a, b| a.0.total_cmp(&b.0)).map(|e| e.1)
}

/// The first empty entry.
pub(super) fn empty(root: &[u32]) -> Option<usize> {
    root.iter().position(|&r| r == NO_ROOT)
}

/// Whether entries `a` and `z` are both held and alike (within `APART` × `alike` steps), so they are one.
pub(super) fn together(genes: &[[f32; NG]], root: &[u32], (a, z): (usize, usize), alike: f32) -> bool {
    root[a] != NO_ROOT && root[z] != NO_ROOT && distance(&genes[a], &genes[z]) <= APART * alike
}

/// The two entries nearest each other by their genes, of any starting kind.
pub(super) fn nearest(genes: &[[f32; NG]]) -> (usize, usize) {
    let pairs = (0..genes.len()).flat_map(|a| (a + 1..genes.len()).map(move |z| (a, z)));
    pairs.min_by(|p, q| distance(&genes[p.0], &genes[p.1]).total_cmp(&distance(&genes[q.0], &genes[q.1]))).expect("two entries")
}

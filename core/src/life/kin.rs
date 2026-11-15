//! The rule by which what lands in a cell finds its place among the few entries the cell keeps (laws.md 3): the slots
//! of its cover (`cover::lodge`) and the lots of the seeds lying on its ground (`cover::Lying`). An entry is its genes
//! and its starting kind (`NO_ROOT`: empty); alike entries of a starting kind are one (within a distance its life sets,
//! by its own measure of how far apart two gene vectors are), and the cover lets the two nearest join when all are held,
//! since its entries are a limit of the bookkeeping and the two hold the same niche to within what it resolves.

/// The root of an empty entry.
pub const NO_ROOT: u32 = u32::MAX;
/// The share of `Physics::alike` within which what lands joins an entry of its kind and entries of a cell are one;
/// what lands farther from every entry of its kind founds one of its own, so a variant keeps its genes apart (a rule of
/// the model).
pub const APART: f32 = 0.5;

/// Genes `g` after one generation's mutation of size `size` (`Physics::mut_size`): each moves by noise of its step
/// (`range`: its step, least and most) times that, within its range.
pub fn mutate<const N: usize>(g: &[f32; N], range: &[(f32, f32, f32); N], size: f32, r: &mut crate::rng::Rng) -> [f32; N] {
    std::array::from_fn(|q| (g[q] + r.near_normal() * range[q].0 * size).clamp(range[q].1, range[q].2))
}

/// Of the entries of genes `genes` and starting kinds `root`, the most alike of starting kind `r` to genes `g` within
/// `within` by `distance`.
pub fn kin<G>(genes: &[G], root: &[u32], (g, r): (&G, u32), within: f32, distance: impl Fn(&G, &G) -> f32) -> Option<usize> {
    let of_kind = (0..root.len()).filter(|&x| root[x] == r);
    of_kind.map(|x| (distance(&genes[x], g), x)).filter(|e| e.0 <= within).min_by(|a, b| a.0.total_cmp(&b.0)).map(|e| e.1)
}

/// Of the entries of genes `genes` and starting kinds `root`, all of starting kind `r` within `within` of genes `g` by
/// `distance`.
pub fn kins<'a, G>(genes: &'a [G], root: &'a [u32], (g, r): (&'a G, u32), within: f32, distance: impl Fn(&G, &G) -> f32 + 'a) -> impl Iterator<Item = usize> + 'a {
    (0..root.len()).filter(move |&x| root[x] == r && distance(&genes[x], g) <= within)
}

/// The first empty entry.
pub fn empty(root: &[u32]) -> Option<usize> {
    root.iter().position(|&r| r == NO_ROOT)
}

/// Whether entries `a` and `z` are both held and alike (within `within` by `distance`), so they are one.
pub fn together<G>(genes: &[G], root: &[u32], (a, z): (usize, usize), within: f32, distance: impl Fn(&G, &G) -> f32) -> bool {
    root[a] != NO_ROOT && root[z] != NO_ROOT && distance(&genes[a], &genes[z]) <= within
}

/// The two entries nearest each other by their genes (`distance`), of any starting kind.
pub fn nearest<G>(genes: &[G], distance: impl Fn(&G, &G) -> f32) -> (usize, usize) {
    let pairs = (0..genes.len()).flat_map(|a| (a + 1..genes.len()).map(move |z| (a, z)));
    pairs.min_by(|p, q| distance(&genes[p.0], &genes[p.1]).total_cmp(&distance(&genes[q.0], &genes[q.1]))).expect("two entries")
}

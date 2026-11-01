//! Which air nodes each ground cell sees and which cells see each air node (`air::lerp`), found once from the map.

use crate::space::Level;

/// The cells that see each air node, and the nodes each node's cells see, from the four nearest air nodes of each
/// ground cell and their weights (`Scratch::near`, `wt`; a corner of no weight sees nothing).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Corners {
    /// Per air node, the cells that see it and the corner they see it at, in the order of the cells: entries
    /// `by_at[q]..by_at[q + 1]` of `by` (`rain::wet`).
    pub(super) by_at: Vec<usize>,
    pub(super) by: Vec<(u32, u8)>,
    /// Per air node, the nodes its cells see, each where the first of them (in the order of the cells and their
    /// corners) sees it: entries `sees_at[j]..sees_at[j + 1]` of `sees`; and per cell and corner the entry of its node's
    /// list it sees there (`cells::Response::ground`).
    pub(super) sees_at: Vec<usize>,
    pub(super) sees: Vec<u32>,
    pub(super) entry: Vec<[u32; 4]>,
}

impl Corners {
    /// From the four nearest air nodes of each ground cell, `near`, and their weights, `wt`, on the air level `air`.
    pub(in crate::flow) fn of(air: &Level, near: &[[u32; 4]], wt: &[[f64; 4]]) -> Corners {
        let (n, na) = (near.len(), air.n());
        let corners = |i: usize| (0..4).filter(move |&c| wt[i][c] > 0.0);
        let mut by_at = vec![0; na + 1];
        (0..n).for_each(|i| corners(i).for_each(|c| by_at[near[i][c] as usize + 1] += 1));
        (0..na).for_each(|q| by_at[q + 1] += by_at[q]);
        let (mut by, mut next) = (vec![(0u32, 0u8); by_at[na]], by_at.clone());
        for (i, ns) in near.iter().enumerate() {
            for c in corners(i) {
                let q = ns[c] as usize;
                by[next[q]] = (i as u32, c as u8);
                next[q] += 1;
            }
        }
        let (mut sees_at, mut sees, mut entry) = (vec![0], Vec::new(), vec![[0u32; 4]; n]);
        for j in 0..na {
            let first = sees.len();
            for &i in air.children(j) {
                for c in corners(i as usize) {
                    let q = near[i as usize][c];
                    let at = sees[first..].iter().position(|&e| e == q).map_or_else(|| sees.len(), |at| first + at);
                    if at == sees.len() {
                        sees.push(q);
                    }
                    entry[i as usize][c] = at as u32;
                }
            }
            sees_at.push(sees.len());
        }
        Corners { by_at, by, sees_at, sees, entry }
    }
}

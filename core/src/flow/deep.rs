//! The store of the bed under each cell (`Class::Deep`): its liquid flows between cells by Darcy's law (laws.md 1.2),
//! one explicit step of the liquid's, sub-stepped inside it only where a cell is too small for it (`flow`).

use super::drain::{carried, store_table, Carried};
use super::Scratch;
use crate::{solve::sweep::{by_size, Net}, store::Class, World};

/// The links of a net: the borders they cross (`Level` links) and their two cells.
type Links<'a> = (&'a [usize], &'a [u32], &'a [u32]);

/// The store's net over the links `links` for the store volumes `vol`, m³: each cell's table, m (`store_table`), and its
/// capacity, its volume per metre of head, m², and each link's conductance, m²/s: `bed_seep` × the depth the store of
/// its higher end fills × width / distance.
fn net(w: &World, s: &Scratch, (border, a, b): Links, vol: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (p, cells) = (w.cfg.phys, w.mesh.cells());
    let area = cells.area();
    let x: Vec<f64> = vol.iter().enumerate().map(|(i, &v)| store_table(w, s, i, v)).collect();
    let cap = area.iter().map(|&a| p.bed_porosity * a).collect();
    let g = (border.iter().zip(a.iter().zip(b)))
        .map(|(&l, (&i, &j))| {
            let up = (if x[i as usize] >= x[j as usize] { i } else { j }) as usize;
            p.bed_seep * vol[up] / (p.bed_porosity * area[up]) * cells.width()[l] / cells.dist()[l]
        })
        .collect();
    (x, cap, g)
}

/// Darcy's law through the stores over `dt` seconds, across the links `links`: each store holds `vol` m³ and moves its
/// share of each substance of `ks` and of its heat; where a cell is too small for the step (a deep store: 125 m filled
/// more than 5 m, 250 m more than 20 m; laws.md 1.8), it and its links take sub-steps (`sweep::by_size`). Returns what
/// crosses: the links stepped once, then the others.
pub(super) fn flow(w: &World, s: &Scratch, links: Links, (ks, vol): (&[usize], &[f64]), dt: f64) -> Vec<Carried> {
    let (n, nk, (border, a, b)) = (w.mesh.cells().n(), ks.len(), links);
    let (x, cap, g) = net(w, s, links, vol);
    let deep = w.store.amounts(Class::Deep);
    let rows: Vec<f64> = ks.iter().flat_map(|&k| deep[k * n..(k + 1) * n].iter().copied()).chain(w.store.heat(Class::Deep).iter().copied()).collect();
    let again = |now: &[f64], us: &[usize]| {
        let pick = |v: &[u32]| us.iter().map(|&u| v[u]).collect::<Vec<u32>>();
        let (x, _, g) = net(w, s, (&us.iter().map(|&u| border[u]).collect::<Vec<usize>>(), &pick(a), &pick(b)), now);
        (x, g)
    };
    let moved = by_size(&Net { a, b, g: &g, cap: &cap }, &x, dt, (vol, &rows), &|_, v| v, &again);
    let fast: Vec<bool> = (0..a.len()).map(|u| moved.fast.binary_search(&u).is_ok()).collect();
    let (sa, sb): (Vec<u32>, Vec<u32>) = (0..a.len()).filter(|&u| !fast[u]).map(|u| (a[u], b[u])).unzip();
    let flux: Vec<f64> = (0..a.len()).filter(|&u| !fast[u]).map(|u| moved.flux[u]).collect();
    let amount = |q: usize, i: usize| deep[ks[q] * n + i];
    let mut out = vec![carried((&sa, &sb), &flux, (ks, amount), vol, |i, share, _| w.store.heat(Class::Deep)[i] * share)];
    if !moved.fast.is_empty() {
        let (fa, fb): (Vec<u32>, Vec<u32>) = moved.fast.iter().map(|&u| (a[u], b[u])).unzip();
        let (mol, heat) = moved.rows.split_at(nk * moved.fast.len());
        out.push(Carried { a: fa, b: fb, mol: mol.to_vec(), heat: heat.to_vec() });
    }
    out
}

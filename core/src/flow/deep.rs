//! The store of the bed under each cell (`Class::Deep`): its liquid flows between cells by Darcy's law (laws.md 1.2),
//! one explicit step of the liquid's (`flow`).

use super::drain::{carried, store_table, Carried};
use super::liquid::reached;
use super::Scratch;
use crate::{solve::sweep::{sweep, Net}, store::Class, World};

/// The links of a net: the borders they cross (`Level` links) and their two cells.
type Links<'a> = (&'a [usize], &'a [u32], &'a [u32]);

/// The store's net over the links `links` for the store volumes `vol`, m³: each cell's table, m (`store_table`), and its
/// capacity, its volume per metre of head, m², and each link's conductance, m²/s: `bed_seep` × the depth the store of
/// its higher end fills × width / distance, between the stores under the ground the laws under the surface reach
/// (`liquid::reached`: the store under a body is not drained).
fn net(w: &World, s: &Scratch, (border, a, b): Links, vol: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (p, cells) = (w.cfg.phys, w.mesh.cells());
    let area = cells.area();
    let x: Vec<f64> = vol.iter().enumerate().map(|(i, &v)| store_table(w, s, i, v)).collect();
    let cap = area.iter().map(|&a| p.bed_porosity * a).collect();
    let g = (border.iter().zip(a.iter().zip(b)))
        .map(|(&l, (&i, &j))| {
            let up = (if x[i as usize] >= x[j as usize] { i } else { j }) as usize;
            p.bed_seep * vol[up] / (p.bed_porosity * area[up]) * cells.width()[l] / cells.dist()[l] * reached(s, i as usize, j as usize)
        })
        .collect();
    (x, cap, g)
}

/// Darcy's law through the stores over `dt` seconds, across the links `links`: each store holds `vol` m³ and moves its
/// share of each substance of `ks` and of its heat (`sweep::sweep`). Returns what crosses.
pub(super) fn flow(w: &World, s: &Scratch, links: Links, (ks, vol): (&[usize], &[f64]), dt: f64) -> Carried {
    let (n, (_, a, b)) = (w.mesh.cells().n(), links);
    let (x, cap, g) = net(w, s, links, vol);
    let deep = w.store.amounts(Class::Deep);
    let flux = sweep(&Net { a, b, g: &g, cap: &cap }, &x, dt, vol);
    let amount = |q: usize, i: usize| deep[ks[q] * n + i];
    carried((a, b), &flux, (ks, amount), vol, |i, share, _| w.store.heat(Class::Deep)[i] * share)
}

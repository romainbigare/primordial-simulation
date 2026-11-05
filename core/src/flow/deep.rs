//! The store of the bed under each cell (`Class::Deep`): its liquid flows between cells by Darcy's law (laws.md 1.2),
//! one explicit step of the liquid's, sub-stepped inside it only where a cell is too small for it (`flow`).

use super::drain::{carried, store_table, Carried};
use super::Scratch;
use crate::{solve::sweep::{steps, sweep, Net}, store::Class, World};

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
/// share of each substance of `ks` and of its heat. A cell whose step is within the explicit limit (`sweep::steps`)
/// steps once. Where a cell's is not (a small cell with a deep store: 125 m filled more than 5 m, 250 m more than 20 m;
/// laws.md 1.8), it and its links take that many sub-steps (`sub_steps`), the cells beside it held as the step starts
/// and each link's sum booked once (a flux register, Berger and Colella 1989); then the other links step once from the
/// step's start, no cell giving more than its sub-stepped links left it. Returns what crosses: the links stepped once,
/// then the others.
pub(super) fn flow(w: &World, s: &Scratch, links: Links, (ks, vol): (&[usize], &[f64]), dt: f64) -> Vec<Carried> {
    let (n, (border, a, b)) = (w.mesh.cells().n(), links);
    let (x, cap, g) = net(w, s, links, vol);
    let rate = steps(&Net { a, b, g: &g, cap: &cap }, dt);
    let fast: Vec<bool> = rate.iter().map(|&r| r > 1).collect();
    let quick: Vec<bool> = a.iter().zip(b).map(|(&i, &j)| fast[i as usize] || fast[j as usize]).collect();
    let pick = |on: bool| -> (Vec<usize>, Vec<u32>, Vec<u32>) {
        let us: Vec<usize> = (0..a.len()).filter(|&u| quick[u] == on).collect();
        (us.iter().map(|&u| border[u]).collect(), us.iter().map(|&u| a[u]).collect(), us.iter().map(|&u| b[u]).collect())
    };
    let most = rate.iter().copied().max().unwrap_or(1);
    let (mut out, gave) = match most {
        1 => (Vec::new(), vec![0.0; n]),
        _ => {
            let (ob, oa, ou) = pick(true);
            let (register, gave) = sub_steps(w, s, (&ob, &oa, &ou), (ks, vol), &fast, (most, dt));
            (vec![register], gave)
        }
    };
    let (_, sa, sb) = pick(false);
    let sg: Vec<f64> = (0..a.len()).filter(|&u| !quick[u]).map(|u| g[u]).collect();
    let have: Vec<f64> = vol.iter().zip(&gave).map(|(&v, &d)| v - d).collect();
    let flux = sweep(&Net { a: &sa, b: &sb, g: &sg, cap: &cap }, &x, dt, &have);
    let amount = |q: usize, i: usize| w.store.amounts(Class::Deep)[ks[q] * n + i];
    out.insert(0, carried((&sa, &sb), &flux, (ks, amount), vol, |i, share, _| w.store.heat(Class::Deep)[i] * share));
    out
}

/// `most` sub-steps of `dt / most` of the links `links`, each of which has a cell that needs them (`fast`): each from
/// the stores as the sub-steps so far leave the fast cells, a cell beside them as the step starts, each link moving the
/// sender's share of what it holds then. Returns what crossed each link over them all, and the volume each other cell
/// gave over them, m³.
fn sub_steps(w: &World, s: &Scratch, links: Links, (ks, vol): (&[usize], &[f64]), fast: &[bool], (most, dt): (u32, f64)) -> (Carried, Vec<f64>) {
    let (n, nk, (_, a, b)) = (w.mesh.cells().n(), ks.len(), links);
    let deep = w.store.amounts(Class::Deep);
    // What each store holds as the sub-steps go: its volume, each substance (`[q * n + i]`) and its heat.
    let (mut now, mut amt, mut heat) = (vol.to_vec(), ks.iter().flat_map(|&k| deep[k * n..(k + 1) * n].iter().copied()).collect::<Vec<f64>>(), w.store.heat(Class::Deep).to_vec());
    let (mut gave, mut mol, mut e) = (vec![0.0; n], vec![0.0; nk * a.len()], vec![0.0; a.len()]);
    let (mut dv, mut da, mut dh) = (vec![0.0; n], vec![0.0; nk * n], vec![0.0; n]);
    for _ in 0..most {
        let (x, cap, g) = net(w, s, links, &now);
        let have: Vec<f64> = (0..n).map(|i| if fast[i] { now[i] } else { vol[i] - gave[i] }).collect();
        let flux = sweep(&Net { a, b, g: &g, cap: &cap }, &x, dt / most as f64, &have);
        for (u, &f) in flux.iter().enumerate().filter(|p| *p.1 != 0.0) {
            let (from, to) = if f > 0.0 { (a[u] as usize, b[u] as usize) } else { (b[u] as usize, a[u] as usize) };
            let share = f.abs() / now[from];
            for q in 0..nk {
                let m = amt[q * n + from] * share;
                (mol[q * a.len() + u], da[q * n + from], da[q * n + to]) = (mol[q * a.len() + u] + f.signum() * m, da[q * n + from] - m, da[q * n + to] + m);
            }
            let h = heat[from] * share;
            (e[u], dh[from], dh[to], dv[from], dv[to]) = (e[u] + f.signum() * h, dh[from] - h, dh[to] + h, dv[from] - f.abs(), dv[to] + f.abs());
            if !fast[from] {
                gave[from] += f.abs();
            }
        }
        // The fast cells take what moved; what a cell beside them gets lands with the step's end.
        for i in (0..n).filter(|&i| fast[i]) {
            (now[i], heat[i]) = (now[i] + dv[i], heat[i] + dh[i]);
            (0..nk).for_each(|q| amt[q * n + i] += da[q * n + i]);
        }
        dv.fill(0.0);
        da.fill(0.0);
        dh.fill(0.0);
    }
    (Carried { a: a.to_vec(), b: b.to_vec(), mol, heat: e }, gave)
}

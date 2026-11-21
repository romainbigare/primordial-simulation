//! The flow the weather's lift draws (laws.md 1.6): air that rises must be fed from below and leave aloft, so where the
//! weather lifts a node's air its lower half gathers air from its neighbours and its upper half gives it back, and
//! where it sinks the low air spreads out (the divergent wind of the weather systems, whose low convergence feeds their
//! rain: P - E = -∇·(q v), Holton 2004, 3.5 and 8.4; Trenberth 1999). The flow on each link is the difference across
//! it of a velocity potential whose convergence at each block is the block's lift (the divergent part of the wind,
//! Helmholtz; Lynch 1989): a Poisson problem on the air's blocks, solved each day by conjugate gradients. It does not
//! cross the map's open edges: the planet's columns beyond them are a held table of its climate with no weather of their
//! own, and a weather system's convergence and the sinking that pays for it close within a few thousand km, inside the
//! map's air (laws.md 1.6).

use super::super::Scratch;
use super::daily::temps;
use super::wind::{column, End, Links};
use crate::{solve::sweep::HALF_WAY, space::Level, Config};

/// The conjugate gradients stop where the residual is below this share of the lift's (`Draw::flows`): far below what
/// the books or the rain could feel, as the flows are booked whatever they are.
const RESIDUAL: f64 = 1e-12;

/// The air's blocks (`daily::blocks`) as one Poisson problem (`Draw::flows`): per link between blocks (`border`) its
/// blocks and its conductance, mol/s per unit of the potential's difference (the lower half's cross-section over its
/// length); per block the sum of its conductances, its group of blocks joined by links (`group`) and its area, m².
pub(super) struct Draw {
    block: Vec<usize>,
    a: Vec<usize>,
    b: Vec<usize>,
    c: Vec<f64>,
    diag: Vec<f64>,
    group: Vec<usize>,
    area: Vec<f64>,
}

impl Draw {
    /// The problem of the blocks (`block`, per air node its block) and the links between them (`border`, of `links`),
    /// from the nodes' ends (`ends`).
    pub(super) fn of(cfg: &Config, air: &Level, (links, border, block): (&Links, &[usize], &[usize]), ends: &[End]) -> Draw {
        let nb = block.iter().max().map_or(0, |&b| b + 1);
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        for &l in border {
            let (ia, ib, id) = (links.a[l] as usize, links.b[l] as usize, links.id[l]);
            a.push(block[ia]);
            b.push(block[ib]);
            c.push(conductance(cfg, (&ends[ia], &ends[ib]), air.width()[id] / air.dist()[id]));
        }
        let mut diag = vec![0.0; nb];
        a.iter().zip(&b).zip(&c).for_each(|((&i, &k), &g)| [i, k].iter().for_each(|&m| diag[m] += g));
        let mut area = vec![0.0; nb];
        block.iter().enumerate().for_each(|(j, &m)| area[m] += air.area()[j]);
        let group = groups(nb, &a, &b);
        Draw { block: block.to_vec(), a, b, c, diag, group, area }
    }

    /// The flows the lift `lift` (per air node, mol/s, rising above 0) draws: per link between blocks the lower half's
    /// flow from its `a` to its `b`, mol/s (the upper half's as much back). Their convergence at each block is its lift:
    /// a group of blocks can only lift as much as it lets sink, so its mean lift per m² sinks over it all, as the
    /// compensating subsidence does.
    pub(super) fn flows(&self, lift: &[f64]) -> Vec<f64> {
        let nb = self.diag.len();
        let mut rhs = vec![0.0; nb];
        lift.iter().zip(&self.block).for_each(|(l, &m)| rhs[m] -= l);
        let (mut sum, mut area) = (vec![0.0; nb], vec![0.0; nb]);
        for m in 0..nb {
            sum[self.group[m]] += rhs[m];
            area[self.group[m]] += self.area[m];
        }
        for m in 0..nb {
            rhs[m] -= sum[self.group[m]] * self.area[m] / area[self.group[m]].max(f64::MIN_POSITIVE);
        }
        let phi = self.solve(&rhs);
        (0..self.a.len()).map(|p| self.c[p] * (phi[self.a[p]] - phi[self.b[p]])).collect()
    }

    /// The Laplacian of the blocks times `phi`: per block what its lower half sends out over its links for the potential.
    fn apply(&self, phi: &[f64]) -> Vec<f64> {
        let mut out: Vec<f64> = self.diag.iter().zip(phi).map(|(d, p)| d * p).collect();
        for p in 0..self.a.len() {
            out[self.a[p]] -= self.c[p] * phi[self.b[p]];
            out[self.b[p]] -= self.c[p] * phi[self.a[p]];
        }
        out
    }

    /// The potential whose Laplacian (`apply`) is `rhs`: conjugate gradients with the diagonal as preconditioner
    /// (Hestenes and Stiefel 1952; Saad 2003, 9.2), at most as many steps as blocks (exact in that many), ending once the
    /// residual is below `RESIDUAL` of `rhs`. A block with no links is left at none.
    fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let nb = rhs.len();
        let dot = |u: &[f64], v: &[f64]| u.iter().zip(v).map(|(x, y)| x * y).sum::<f64>();
        let pre = |r: &[f64]| r.iter().zip(&self.diag).map(|(r, d)| if *d > 0.0 { r / d } else { 0.0 }).collect::<Vec<f64>>();
        let (mut phi, mut r) = (vec![0.0; nb], rhs.to_vec());
        let stop = RESIDUAL * dot(rhs, rhs).sqrt();
        let mut z = pre(&r);
        let (mut p, mut rz) = (z.clone(), dot(&r, &z));
        for _ in 0..nb {
            if dot(&r, &r).sqrt() <= stop {
                break;
            }
            let q = self.apply(&p);
            let alpha = rz / dot(&p, &q).max(f64::MIN_POSITIVE);
            phi.iter_mut().zip(&p).for_each(|(x, p)| *x += alpha * p);
            r.iter_mut().zip(&q).for_each(|(r, q)| *r -= alpha * q);
            z = pre(&r);
            let next = dot(&r, &z);
            p = z.iter().zip(&p).map(|(z, p)| z + next / rz.max(f64::MIN_POSITIVE) * p).collect();
            rz = next;
        }
        phi
    }
}

/// The heat that the gases the weather's drawn flow moves over a day carry across the links between blocks (`gases`,
/// per gas and link, mol, + from its `a` to its `b`), J, + from `a` to `b`, from the air `x` (`[gi * na + j]`) and heat
/// `e`: each mol at its gas's heat per mol at the link's mean temperature, the mean of its ends' (as the air holds a
/// gas's heat, cp T, `air_temp`; the latent heat of the vapour is the air's to hold, so it moves with the vapour's mol).
/// The drawn flow is the weather's divergent circulation, whose sign follows the random lift: as many mol of its halves
/// cross each way, so a reversible circulation carries no heat of its own, only that of the net gas that converges.
/// No node gives more than `HALF_WAY` of its heat so: where it would, its own outgoing links are cut in proportion.
pub(super) fn heat(s: &Scratch, (x, e): (&[f64], &[f64]), (a, b): (&[u32], &[u32]), gases: &[Vec<f64>]) -> Vec<f64> {
    let t = temps(s, x, e);
    let mut heat: Vec<f64> = (0..a.len())
        .map(|p| {
            let mean = (t[a[p] as usize] + t[b[p] as usize]) / 2.0;
            s.state.gas.iter().zip(gases).map(|(&k, mol)| mol[p] * s.sp[k].cp * mean).sum()
        })
        .collect();
    let sender = |p: usize, q: f64| if q > 0.0 { a[p] as usize } else { b[p] as usize };
    let mut gives = vec![0.0; e.len()];
    heat.iter().enumerate().for_each(|(p, &q)| gives[sender(p, q)] += q.abs());
    for (p, q) in heat.iter_mut().enumerate() {
        let j = sender(p, *q);
        if gives[j] > HALF_WAY * e[j] {
            *q *= HALF_WAY * e[j].max(0.0) / gives[j];
        }
    }
    heat
}

/// The conductance of a link between ends `a` and `b` for the drawn flow (`Draw`), its width over its length `shape`:
/// the lower half's cross-section, the mol per m³ times half the column's depth (`wind::column`), times `shape`, so the
/// potential's difference drives a flow as a speed through that section would.
pub(super) fn conductance(cfg: &Config, (a, b): (&End, &End), shape: f64) -> f64 {
    let (rho, depth) = column(cfg, a, b);
    rho * depth / 2.0 * shape
}

/// Each of `n` blocks' group: the least block it is joined to by the links `a`, `b`.
fn groups(n: usize, a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut group: Vec<usize> = (0..n).collect();
    let root = |g: &mut Vec<usize>, mut m: usize| {
        while g[m] != m {
            m = g[m];
        }
        m
    };
    for (&i, &k) in a.iter().zip(b) {
        let (ri, rk) = (root(&mut group, i), root(&mut group, k));
        group[ri.max(rk)] = ri.min(rk);
    }
    (0..n).map(|m| root(&mut group, m)).collect()
}

#[cfg(test)]
mod tests {
    use super::Draw;

    /// The flows the lift draws converge at each block to its lift less the mean lift, which sinks over all: on a row of
    /// three blocks of one area.
    #[test]
    fn the_low_air_gathers_where_the_weather_lifts() {
        let row = Draw { block: vec![0, 1, 2], a: vec![0, 1], b: vec![1, 2], c: vec![2.0, 2.0], diag: vec![2.0, 4.0, 2.0], group: vec![0; 3], area: vec![1.0; 3] };
        let lift = [3.0, -1.0, 0.5];
        let mut got = vec![0.0; 3];
        row.flows(&lift).iter().enumerate().for_each(|(p, f)| (got[row.a[p]], got[row.b[p]]) = (got[row.a[p]] - f, got[row.b[p]] + f));
        let mean = lift.iter().sum::<f64>() / 3.0;
        assert!(got.iter().zip(&lift).all(|(g, l)| (g - (l - mean)).abs() < 1e-9), "{got:?}");
    }
}

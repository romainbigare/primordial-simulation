//! How each block's air moves as one through a day, as the eddies even it (`Together`, read by the air's days,
//! `daily`).

use super::super::Scratch;
use super::daily::Ground;

/// How each block's air moves as one through a day, as the eddies even it (`exchange`): per node its share of a change
/// of its block's vapour (`Pool::v`, by volume); per active gas and block (`[y * blocks + b]`) the slope of what the
/// block's ground gives of the gas with its vapour so shared (`Pool::s`), mol/mol; and per block the slope of the heat
/// its ground gives it with its temperature, all its nodes warming alike, J/K. And, for `outs` = 1 + nx outputs, per
/// node each block whose air its ground reads (`Ground::near`), entries `seen_at[j]..seen_at[j + 1]` of `seen`, with
/// the slopes of what the ground gives the node with that block's warming and with a change of its vapour so shared
/// (`by`, `[outs²]` per entry: per output its slope with the warming, then with each active gas), so the day's later air
/// is the day's start's moved by its blocks (`warmed`, `shared`).
pub(super) struct Together {
    pub(super) v: Vec<f64>,
    pub(super) gas: Vec<f64>,
    pub(super) heat: Vec<f64>,
    outs: usize,
    seen_at: Vec<usize>,
    seen: Vec<u32>,
    by: Vec<f64>,
}

impl Together {
    pub(super) fn of(s: &Scratch, ground: &Ground, (blocks, block): (&[Vec<u32>], &[usize]), nx: usize) -> Together {
        let (na, outs, nb) = (block.len(), 1 + nx, blocks.len());
        let mut v = vec![0.0; na];
        for nodes in blocks {
            let vol: f64 = nodes.iter().map(|&j| s.vol[j as usize]).sum();
            nodes.iter().for_each(|&j| v[j as usize] = s.vol[j as usize] / vol);
        }
        let (mut gas, mut heat) = (vec![0.0; nx * nb], vec![0.0; nb]);
        let (mut seen_at, mut seen, mut by) = (vec![0], Vec::new(), Vec::new());
        for j in 0..na {
            for e in ground.at[j]..ground.at[j + 1] {
                let (q, slope) = (ground.near[e] as usize, &ground.slope[e * outs * outs..(e + 1) * outs * outs]);
                if block[q] == block[j] {
                    (0..nx).for_each(|y| gas[y * nb + block[j]] += slope[(1 + y) * outs + 1 + y] * v[q]);
                    heat[block[j]] += slope[0];
                }
                let k = seen[seen_at[j]..].iter().position(|&b| b as usize == block[q]).map_or_else(|| seen.len(), |k| seen_at[j] + k);
                if k == seen.len() {
                    seen.push(block[q] as u32);
                    by.resize(by.len() + outs * outs, 0.0);
                }
                for (o, row) in by[k * outs * outs..(k + 1) * outs * outs].chunks_exact_mut(outs).enumerate() {
                    row[0] += slope[o * outs];
                    (0..nx).for_each(|y| row[1 + y] += slope[o * outs + 1 + y] * v[q]);
                }
            }
            seen_at.push(seen.len());
        }
        Together { v, gas, heat, outs, seen_at, seen, by }
    }
    /// What the ground gives each node (`got`, `[j * (1 + nx) + o]`), moved by its blocks' warming `warm`, K per block.
    pub(super) fn warmed(&self, got: &mut [f64], warm: &[f64]) {
        self.shift(got, |row, b| row[0] * warm[b]);
    }
    /// What the ground gives each node (`got`), moved by a change `dx` of its blocks' vapour (`[y * nb + b]`, mol),
    /// shared by volume.
    pub(super) fn shared(&self, got: &mut [f64], dx: &[f64]) {
        let nb = self.heat.len();
        self.shift(got, |row, b| row[1..].iter().enumerate().map(|(y, s)| s * dx[y * nb + b]).sum::<f64>());
    }
    /// `got` shifted per node, output and block seen by `by(slopes of the output, block)`.
    fn shift(&self, got: &mut [f64], by: impl Fn(&[f64], usize) -> f64) {
        let outs = self.outs;
        for (j, row) in got.chunks_exact_mut(outs).enumerate() {
            for k in self.seen_at[j]..self.seen_at[j + 1] {
                let slopes = &self.by[k * outs * outs..(k + 1) * outs * outs];
                row.iter_mut().zip(slopes.chunks_exact(outs)).for_each(|(g, s)| *g += by(s, self.seen[k] as usize));
            }
        }
    }
}

//! Band: one implicit step of amounts carried along links, solved directly. Link `l` carries `fwd[l] x'[a] - back[l]
//! x'[b]` over the step, `x'` the amounts after it: (I + L) x' = x, backward Euler, stable for any coefficients
//! (Patankar 1980, ch. 5). A caller may add slopes (a Newton step) and couple fields. A transport alone is dominant by
//! columns, where elimination swaps no rows (Wilkinson 1961); with slopes it pivots (Press et al. 2007, 2.4). The band
//! is the farthest pair of a link; the cost is unknowns times the band squared. On a globe whose rows are numbered in a
//! folded order (`space::square`), the link that closes a row joins two nodes numbered side by side, so the wrapped
//! system is solved as it is, exactly, in a band of one row.

/// A banded system: per row the entries from `w` left of the diagonal to `2 w` right (room for pivoting); once factored,
/// its factors, the swaps and how far right each row reaches.
#[derive(Clone, Debug, Default)]
pub struct Band {
    w: usize,
    rows: Vec<f64>,
    swap: Vec<usize>,
    last: Vec<usize>,
}

/// Coefficients of a transport over a step (at least 0), per link: it carries `fwd x'[a] - back x'[b]` from `a` to `b`.
pub struct Carry<'a> {
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub fwd: &'a [f64],
    pub back: &'a [f64],
}

impl Band {
    /// No entry yet in `n` rows, room for `w` entries either side of the diagonal.
    pub fn zero(n: usize, w: usize) -> Band {
        Band { w, rows: vec![0.0; n * (3 * w + 1)], swap: Vec::new(), last: (0..n).map(|i| (i + w).min(n.max(1) - 1)).collect() }
    }
    /// The step of one field of `n` nodes: the identity with the transport `c` added, factored.
    pub fn step(n: usize, c: &Carry) -> Band {
        let mut band = Band::zero(n, reach(c));
        band.carry(c, 1, 0, 1.0);
        (0..n).for_each(|j| band.add(j, j, 1.0));
        band.factor();
        band
    }
    /// Where the entry of row `i`, column `j` is kept.
    #[inline]
    fn at(&self, i: usize, j: usize) -> usize {
        i * (3 * self.w + 1) + self.w + j - i
    }
    /// Add `v` to the entry of row `i`, column `j`.
    #[inline]
    pub fn add(&mut self, i: usize, j: usize, v: f64) {
        let at = self.at(i, j);
        self.rows[at] += v;
    }
    /// Add the transport `c` of field `f` of `fields` side by side per node, its rows scaled by `scale`.
    pub fn carry(&mut self, c: &Carry, fields: usize, f: usize, scale: f64) {
        for (l, (&p, &q)) in c.a.iter().zip(c.b).enumerate() {
            let (p, q) = (p as usize * fields + f, q as usize * fields + f);
            let (out, back) = (scale * c.fwd[l], scale * c.back[l]);
            self.add(p, p, out);
            self.add(q, p, -out);
            self.add(q, q, back);
            self.add(p, q, -back);
        }
    }
    /// Factor the system in place: LU with partial pivoting.
    pub fn factor(&mut self) {
        let (w, n) = (self.w, self.last.len());
        self.swap = (0..n).collect();
        // The part of the pivot's row right of the diagonal, which each row below takes its share of.
        let mut right = vec![0.0; 2 * w];
        for k in 0..n {
            let below = n.min(k + w + 1);
            let p = (k..below).fold(k, |p, i| if self.rows[self.at(i, k)].abs() > self.rows[self.at(p, k)].abs() { i } else { p });
            if p != k {
                let end = self.last[k].max(self.last[p]);
                for j in k..=end {
                    let (a, b) = (self.at(k, j), self.at(p, j));
                    self.rows.swap(a, b);
                }
                (self.swap[k], self.last[k], self.last[p]) = (p, end, end);
            }
            let (pivot, m) = (self.rows[self.at(k, k)], self.last[k] - k);
            let from = self.at(k, k) + 1;
            right[..m].copy_from_slice(&self.rows[from..from + m]);
            for i in k + 1..below {
                let ik = self.at(i, k);
                let f = self.rows[ik] / pivot;
                self.rows[ik] = f;
                if f != 0.0 {
                    let from = self.at(i, k + 1);
                    self.rows[from..from + m].iter_mut().zip(&right[..m]).for_each(|(v, &r)| *v -= f * r);
                    self.last[i] = self.last[i].max(k + m);
                }
            }
        }
    }
    /// Solve the factored system in place: `x` holds the right-hand side, then the unknowns.
    pub fn solve(&self, x: &mut [f64]) {
        let (n, w) = (x.len(), self.w);
        for k in 0..n {
            x.swap(k, self.swap[k]);
            (k + 1..n.min(k + w + 1)).for_each(|i| x[i] -= self.rows[self.at(i, k)] * x[k]);
        }
        for i in (0..n).rev() {
            let s: f64 = (i + 1..=self.last[i]).map(|j| self.rows[self.at(i, j)] * x[j]).sum();
            x[i] = (x[i] - s) / self.rows[self.at(i, i)];
        }
    }
}

/// How far from the diagonal the links of `c` reach: the farthest pair of a link.
pub fn reach(c: &Carry) -> usize {
    c.a.iter().zip(c.b).map(|(&p, &q)| p.abs_diff(q) as usize).max().unwrap_or(0)
}

/// What each link of `c` carries of field `f` over the step from the amounts after it, `x`, into `flux`.
pub fn fluxes(c: &Carry, x: &[f64], (fields, f): (usize, usize), flux: &mut [f64]) {
    flux.iter_mut().enumerate().for_each(|(l, v)| *v = c.fwd[l] * x[c.a[l] as usize * fields + f] - c.back[l] * x[c.b[l] as usize * fields + f]);
}

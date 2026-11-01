//! A link flux gathered link by link, as a law finds what crosses each link (`Batch`), for `apply`.

use super::moves::LinkFlux;
use super::nodes::Class;

/// What crosses links between nodes of two classes, gathered link by link: ends, mol of each of `ks` (+ from `a` to
/// `b`) and the heat leaving `a`, J.
#[derive(Clone, Debug)]
pub struct Batch {
    class: [Class; 2],
    a: Vec<u32>,
    b: Vec<u32>,
    ks: Vec<u16>,
    /// Per substance of `ks`, per link
    mol: Vec<Vec<f64>>,
    heat: Vec<f64>,
    /// `mol`, one substance after another, as `flux` gives it
    flat: Vec<f64>,
}

impl Batch {
    /// No link yet between nodes of `class`, of the substances `ks` (more join as they cross).
    pub fn new(class: [Class; 2], ks: &[u16]) -> Batch {
        Batch { class, a: Vec::new(), b: Vec::new(), ks: ks.to_vec(), mol: vec![Vec::new(); ks.len()], heat: Vec::new(), flat: Vec::new() }
    }
    /// A new link from node `a` to node `b`, carrying `heat` J and nothing else yet; its index.
    pub fn link(&mut self, (a, b): (usize, usize), heat: f64) -> usize {
        self.a.push(a as u32);
        self.b.push(b as u32);
        self.heat.push(heat);
        self.mol.iter_mut().for_each(|v| v.push(0.0));
        self.a.len() - 1
    }
    /// Link `l` carries `m` mol of substance `k` more.
    pub fn carry(&mut self, l: usize, k: u16, m: f64) {
        let q = self.ks.iter().position(|&x| x == k).unwrap_or_else(|| {
            self.ks.push(k);
            self.mol.push(vec![0.0; self.a.len()]);
            self.ks.len() - 1
        });
        self.mol[q][l] += m;
    }
    /// From `a` to `b`, on the last link if it joins them (else a new one): `m` mol of `k` and `heat` J more.
    pub fn add(&mut self, (a, b): (usize, usize), k: u16, (m, heat): (f64, f64)) {
        let l = match self.a.len().checked_sub(1) {
            Some(l) if (self.a[l], self.b[l]) == (a as u32, b as u32) => {
                self.heat[l] += heat;
                l
            }
            _ => self.link((a, b), heat),
        };
        self.carry(l, k, m);
    }
    /// The batch as a link flux, for `apply`.
    pub fn flux(&mut self) -> LinkFlux<'_> {
        self.flat.clear();
        self.mol.iter().for_each(|m| self.flat.extend_from_slice(m));
        LinkFlux { class: self.class, a: &self.a, b: &self.b, ks: &self.ks, mol: &self.flat, heat: &self.heat }
    }
}

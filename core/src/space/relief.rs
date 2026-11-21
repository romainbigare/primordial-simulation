//! What the running world laid on the terrain's samples (laws.md 0 and 1.3): the samples' ground is the seed's
//! (`Terrain::samples`) plus the relief, the solid volume per area the load laid on each sample, m (as
//! `World::surface_z`), never under none. Only the load writes it (`flow::runoff`): what settles at a sample lies on
//! it, and what a cell's runoff picks up comes first from its samples' relief. The relief says where in a cell its
//! matter lies; the matter is booked in the cell's ground node.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The relief of the samples (the module's doc): of each sample what the load laid on it, m, now and when the samples
/// were last built from it (`built`; none at all before the first lay or build).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relief {
    pub laid: Vec<f64>,
    pub built: Vec<f64>,
}

impl Relief {
    /// `m` m laid on sample `k` of `n` samples.
    pub fn lay(&mut self, k: usize, m: f64, n: usize) {
        if self.laid.len() != n {
            self.laid.resize(n, 0.0);
        }
        self.laid[k] += m;
    }
    /// At most `m` m taken from sample `k`'s relief, never under none: what was taken, m.
    pub fn take(&mut self, k: usize, m: f64) -> f64 {
        let Some(at) = self.laid.get_mut(k) else { return 0.0 };
        let taken = m.min(*at).max(0.0);
        *at -= taken;
        taken
    }
    /// What lies on sample `k`, m.
    pub fn at(&self, k: usize) -> f64 {
        self.laid.get(k).copied().unwrap_or(0.0)
    }
    /// What was laid on sample `k` since the samples were last built, m (less what was taken).
    pub fn since(&self, k: usize) -> f64 {
        self.at(k) - self.built.get(k).copied().unwrap_or(0.0)
    }
}

/// The relief as saved: of each list the samples that hold some, each with what it holds, and the number of samples.
#[derive(Serialize, Deserialize)]
struct Sparse {
    n: u32,
    laid: Vec<(u32, f64)>,
    built: Vec<(u32, f64)>,
}

/// The samples of `v` that hold some, in their order.
fn sparse(v: &[f64]) -> Vec<(u32, f64)> {
    v.iter().enumerate().filter(|p| *p.1 != 0.0).map(|(k, &m)| (k as u32, m)).collect()
}

/// `n` samples, those of `v` holding what it says and the rest none (none at all where `v` is empty).
fn dense(n: usize, v: &[(u32, f64)]) -> Vec<f64> {
    if v.is_empty() {
        return Vec::new();
    }
    let mut d = vec![0.0; n];
    v.iter().for_each(|&(k, m)| d[k as usize] = m);
    d
}

impl Serialize for Relief {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        Sparse { n: self.laid.len().max(self.built.len()) as u32, laid: sparse(&self.laid), built: sparse(&self.built) }.serialize(to)
    }
}

impl<'de> Deserialize<'de> for Relief {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Relief, D::Error> {
        let s = Sparse::deserialize(from)?;
        Ok(Relief { laid: dense(s.n as usize, &s.laid), built: dense(s.n as usize, &s.built) })
    }
}

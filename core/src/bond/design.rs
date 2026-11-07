//! The design key: how alike parts and structures find each other (laws.md 3.1). A part's label holds its material
//! band (the hardness and the bond strength of its walls) and its shape band (its hollow share and the elongation of
//! its sides), each cut into `BANDS` bands, and its mass band (`MASS_STEP`); a structure's design is the hash of its bond graph with those labels, by
//! `WL_ROUNDS` rounds of neighbour hashing (Weisfeiler and Lehman 1968; Shervashidze et al. 2011): two structures with
//! the same bond graph and the same bands have the same design, wherever they lie. A design is how alike objects are
//! found, as a lineage is for bodies; it is never copied.

use super::Part;
use crate::{chem::Chemistry, life::Cell};
use std::collections::{BTreeMap, HashMap};

/// Bands a part's hardness, bond strength (0..1), hollow share (0..1) and elongation are cut into.
pub(super) const BANDS: usize = 4;
/// A part's mass band is this factor wide: alike parts weigh within it, so their sizes are within its cube root (26%).
const MASS_STEP: f64 = 2.0;
/// Elongation (log10 of longest over shortest side) that fills the last band.
const ELONGATION_DECADES: f64 = 2.0;
/// Rounds of neighbour hashing in a structure's design.
const WL_ROUNDS: usize = 3;

/// FNV-1a over the bytes of `words`: 64-bit offset basis and prime.
pub(super) fn fnv(words: &[u64]) -> u64 {
    words.iter().flat_map(|w| w.to_le_bytes()).fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// The band of `x` in 0..1.
pub(super) fn band(x: f64) -> i64 {
    ((x * BANDS as f64) as i64).clamp(0, BANDS as i64 - 1)
}

/// The bands of a part's label (`label`), in the order a cap of lots joins them (`lots::cap`, the last first): its
/// walls' hardness and bond strength, its hollow share, the elongation of its sides, and the mass of the walls of one
/// part (of a lot, `MASS_STEP`).
pub(super) fn bands(chem: &Chemistry, p: &Part) -> [i64; 5] {
    let walls = p.walls(chem);
    let elongation = (p.size[0] as f64 / (p.size[2] as f64).max(1e-9)).max(1.0).log10() / ELONGATION_DECADES;
    let mass = ((walls.mass / p.count).ln() / MASS_STEP.ln()).floor() as i64;
    [band(walls.hardness), band(walls.strength), band(p.hollow as f64), band(elongation), mass]
}

/// A part's label: its material band (hardness, bond strength of its walls), its shape band (hollow share, elongation
/// of its sides) and its mass band (`bands`).
pub(super) fn label(chem: &Chemistry, p: &Part) -> u64 {
    label_of(bands(chem, p))
}

/// The label of a part of bands `b` (`bands`).
pub(super) fn label_of(b: [i64; 5]) -> u64 {
    let n = BANDS as i64;
    fnv(&[(b[0] * n + b[1]) as u64, (b[2] * n + b[3]) as u64, b[4] as u64])
}

/// The design of each structure of cell `c`, by its root's id: `WL_ROUNDS` rounds of neighbour hashing of its parts'
/// labels over its bonds, then the hash of its parts' labels sorted.
pub fn designs(chem: &Chemistry, c: &Cell) -> BTreeMap<u64, u64> {
    let at: HashMap<u64, usize> = c.parts.iter().enumerate().map(|(k, p)| (p.id, k)).collect();
    let mut near = vec![vec![]; c.parts.len()];
    for (a, b) in c.bonds.a.iter().zip(&c.bonds.b) {
        if let (Some(&a), Some(&b)) = (at.get(a), at.get(b)) {
            near[a].push(b);
            near[b].push(a);
        }
    }
    let mut labels: Vec<u64> = c.parts.iter().map(|p| label(chem, p)).collect();
    for _ in 0..WL_ROUNDS {
        labels = (0..labels.len())
            .map(|k| {
                let mut round: Vec<u64> = near[k].iter().map(|&q| labels[q]).collect();
                round.sort_unstable();
                round.insert(0, labels[k]);
                fnv(&round)
            })
            .collect();
    }
    let mut by_root: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (k, p) in c.parts.iter().enumerate() {
        by_root.entry(p.root).or_default().push(labels[k]);
    }
    by_root
        .into_iter()
        .map(|(r, mut own)| {
            own.sort_unstable();
            (r, fnv(&own))
        })
        .collect()
}

//! Repeated structures: the parts of the snapshots grouped by structure (`Part::root`), each structure hashed by its bond
//! graph with its parts' materials and shapes cut into bands, and the copies of each hash counted.

use crate::{BANDS, ELONGATION_DECADES, REPEAT_MIN, WL_ROUNDS};
use pcore::bond::{Part, NOBODY};
use pcore::World;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;
use tape::Kept;
use worldgen::schedule::Schedule;

/// FNV-1a over the bytes of `words`: 64-bit offset basis and prime.
fn fnv(words: &[u64]) -> u64 {
    words.iter().flat_map(|w| w.to_le_bytes()).fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// The band of `x` in 0..1.
fn band(x: f64) -> u64 {
    ((x * BANDS as f64) as u64).min(BANDS as u64 - 1)
}

/// A part's first label: its material band (hardness, bond strength of its walls) and its shape band (hollow share,
/// elongation of its sides).
fn label(w: &World, p: &Part) -> u64 {
    let walls = p.walls(&w.chem);
    let material = band(walls.hardness) * BANDS as u64 + band(walls.strength);
    let elongation = (p.size[0] as f64 / (p.size[2] as f64).max(1e-9)).max(1.0).log10() / ELONGATION_DECADES;
    let shape = band(p.hollow as f64) * BANDS as u64 + band(elongation);
    fnv(&[material, shape])
}

/// What one structure is: its hash, and its numbers.
struct Found {
    hash: u64,
    parts: usize,
    mass: f64,
    hollow: f64,
    enclosed: bool,
    held: bool,
}

/// The structures of cell `i` of `w`, each hashed by `WL_ROUNDS` rounds of neighbour hashing over its bonds.
fn cell_structures(w: &World, i: usize) -> Vec<Found> {
    let cell = w.life.cell_at(i);
    let at: HashMap<u64, usize> = cell.parts.iter().enumerate().map(|(k, p)| (p.id, k)).collect();
    let mut near = vec![vec![]; cell.parts.len()];
    for (a, b) in cell.bonds.a.iter().zip(&cell.bonds.b) {
        if let (Some(&a), Some(&b)) = (at.get(a), at.get(b)) {
            near[a].push(b);
            near[b].push(a);
        }
    }
    let mut labels: Vec<u64> = cell.parts.iter().map(|p| label(w, p)).collect();
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
    let mut by_root: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (k, p) in cell.parts.iter().enumerate() {
        by_root.entry(p.root).or_default().push(k);
    }
    by_root
        .values()
        .map(|members| {
            let mut own: Vec<u64> = members.iter().map(|&k| labels[k]).collect();
            own.sort_unstable();
            let parts: Vec<&Part> = members.iter().map(|&k| &cell.parts[k]).collect();
            let root = parts.iter().find(|p| p.id == p.root).unwrap_or(&parts[0]);
            Found {
                hash: fnv(&own),
                parts: parts.len(),
                mass: parts.iter().map(|p| p.mass(&w.chem)).sum(),
                hollow: parts.iter().map(|p| p.hollow as f64).sum::<f64>() / parts.len() as f64,
                enclosed: root.s.encl.closed >= tape::ENCLOSED,
                held: parts.iter().any(|p| p.held != NOBODY),
            }
        })
        .collect()
}

/// The cards of the structures in the snapshots of `dir` (`snap_*.bin.gz`, oldest first), and the years of the snapshots.
/// A card is of a hash with `REPEAT_MIN` copies or more in the last snapshot; its count and numbers are the last
/// snapshot's, its first seen is the earliest snapshot with the hash.
pub fn read(dir: &Path) -> io::Result<(Vec<Value>, Vec<f64>)> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("snap_") && n.ends_with(".bin.gz")))
        .collect();
    files.sort();
    let (mut years, mut first, mut last): (Vec<f64>, HashMap<u64, usize>, Vec<Found>) = (vec![], HashMap::new(), vec![]);
    for path in &files {
        let (w, _): (World, (Schedule, Kept)) = tape::load_snapshot(path)?;
        last = (0..w.mesh.cells().n()).flat_map(|i| cell_structures(&w, i)).collect();
        for s in &last {
            first.entry(s.hash).or_insert(years.len());
        }
        years.push(w.year());
    }
    let mut copies: BTreeMap<u64, Vec<&Found>> = BTreeMap::new();
    for s in &last {
        copies.entry(s.hash).or_default().push(s);
    }
    let share = |c: &[&Found], f: fn(&Found) -> bool| c.iter().filter(|s| f(s)).count() as f64 / c.len() as f64;
    let cards = copies
        .iter()
        .filter(|(_, c)| c.len() >= REPEAT_MIN)
        .map(|(hash, c)| {
            let n = c.len() as f64;
            let seen = first[hash];
            let mut card = json!({
                "pattern_id": format!("struct-{hash:016x}"),
                "category": "structure",
                "count": c.len(),
                "parts": c[0].parts,
                "mass": c.iter().map(|s| s.mass).sum::<f64>() / n,
                "hollow": c.iter().map(|s| s.hollow).sum::<f64>() / n,
                "enclosed_share": share(c, |s| s.enclosed),
                "held_share": share(c, |s| s.held),
                "first_seen_year": years[seen],
                "first_seen_exact": seen > 0,
                "evidence_refs": [format!("snap:{}", years[seen]), format!("snap:{}", years[years.len() - 1])],
            });
            if years.len() == 1 {
                card["note"] = json!("only the end snapshot: first_seen_year is the end of the run, not the first appearance");
            }
            card
        })
        .collect();
    Ok((cards, years))
}

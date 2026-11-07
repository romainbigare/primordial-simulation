//! Repeated structures: the parts of the snapshots grouped by structure (`Part::root`), each structure with its design
//! (`bond::designs`: the hash of its bond graph with its parts' materials and shapes cut into bands), and the copies of
//! each design counted.

use crate::REPEAT_MIN;
use pcore::bond::{self, Part, NOBODY};
use pcore::World;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;
use tape::Kept;
use worldgen::schedule::Schedule;

/// What one structure is: its hash, its copies (a lot's count) and the numbers of each copy.
struct Found {
    hash: u64,
    copies: f64,
    parts: usize,
    mass: f64,
    hollow: f64,
    enclosed: bool,
    held: bool,
}

/// The structures of cell `i` of `w`, each with its design (`bond::designs`).
fn cell_structures(w: &World, i: usize) -> Vec<Found> {
    let cell = w.life.cell_at(i);
    let mut by_root: BTreeMap<u64, Vec<&Part>> = BTreeMap::new();
    for p in &cell.parts {
        by_root.entry(p.root).or_default().push(p);
    }
    let designs = bond::designs(&w.chem, cell);
    by_root
        .into_iter()
        .map(|(r, parts)| {
            let root = parts.iter().find(|p| p.id == p.root).unwrap_or(&parts[0]);
            Found {
                hash: designs[&r],
                copies: root.count,
                parts: parts.len(),
                mass: parts.iter().map(|p| p.mass(&w.chem) / p.count).sum(),
                hollow: parts.iter().map(|p| p.hollow as f64).sum::<f64>() / parts.len() as f64,
                enclosed: root.s.encl.closed >= tape::ENCLOSED,
                held: parts.iter().any(|p| p.held != NOBODY),
            }
        })
        .collect()
}

/// The cards of the structures in the snapshots of `dir` (`snap_*.bin.gz`, oldest first), and the years of the snapshots.
/// A card is of a hash with `REPEAT_MIN` copies or more in the last snapshot (a lot of alike parts counts its parts);
/// its count and numbers (means by copies) are the last snapshot's, its first seen is the earliest snapshot with the
/// hash.
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
    let all = |c: &[&Found]| c.iter().map(|s| s.copies).sum::<f64>();
    let mean = |c: &[&Found], f: fn(&Found) -> f64| c.iter().map(|s| s.copies * f(s)).sum::<f64>() / all(c);
    let share = |c: &[&Found], f: fn(&Found) -> bool| c.iter().filter(|s| f(s)).map(|s| s.copies).sum::<f64>() / all(c);
    let cards = copies
        .iter()
        .filter(|(_, c)| all(c) >= REPEAT_MIN as f64)
        .map(|(hash, c)| {
            let seen = first[hash];
            let mut card = json!({
                "pattern_id": format!("struct-{hash:016x}"),
                "category": "structure",
                "count": all(c),
                "parts": c[0].parts,
                "mass": mean(c, |s| s.mass),
                "hollow": mean(c, |s| s.hollow),
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

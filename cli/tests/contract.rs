//! The contract between the layers (`docs/foundation/plan.md` 5.2): a law changes the world's amounts and heat only by
//! handing moves and link fluxes to the store (`store::apply`); no code of `core/src` outside the store writes the
//! arrays of `World` itself, but the places named in `EXEMPT`.

use std::path::Path;

/// The arrays of `World` that hold the amounts and heat of the store's nodes (`store::Class`), and the bed.
const ARRAYS: [&str; 10] = ["ground", "e_ground", "air", "e_air", "deep", "e_deep", "e_bodies", "bed", "boundary", "e_boundary"];
/// The names a `World` goes by before a `.` (a field of anything else, as `Scratch::bed`, is not the world's).
const WORLD: [&str; 3] = ["w", "ww", "self"];

/// The folder and the files of `core/src` that may write the arrays, each with its reason. Each must still write them
/// (an exemption no longer needed goes).
const EXEMPT: [(&str, &str); 1] = [("store", "the store itself: `apply`, the boundary's books (`World::cross`), and the bed's one change, `World::loosen` (until plan F6)")];

/// Does `line` (code, without its comment) write one of `ARRAYS`: an index of it on the left of an assignment, the
/// array lent as `&mut`, or a method that changes it?
fn writes(line: &str) -> bool {
    let line = line.split("//").next().unwrap_or("");
    let b = line.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    // Where a tuple on the left of an assignment ends, if the line starts with one.
    let tuple_end = if line.trim_start().starts_with('(') { line.find(") = ") } else { None };
    for name in ARRAYS {
        for (at, _) in line.match_indices(name) {
            let end = at + name.len();
            if end < b.len() && word(b[end]) || at > 0 && word(b[at - 1]) {
                continue;
            }
            // Its owner: the world, or a binding of the world's field (`let World { ground, .. } = w`).
            let mut start = at;
            if at > 0 && b[at - 1] == b'.' {
                let owner = line[..at - 1].rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("");
                if !WORLD.contains(&owner) {
                    continue;
                }
                start = at - 1 - owner.len();
            }
            let rest = &line[end..];
            let lent = line[..start].trim_end().ends_with("&mut");
            let changed = [".iter_mut(", ".fill(", ".copy_from_slice(", ".swap(", ".resize("].iter().any(|m| rest.starts_with(m));
            let in_tuple = tuple_end.is_some_and(|e| at < e);
            if lent || changed || in_tuple || rest.starts_with('[') && assigned(rest) {
                return true;
            }
        }
    }
    false
}

/// Is the index that `rest` starts with (`[...]`) followed by an assignment (`=`, `+=`, `-=`, `*=`, `/=`)?
fn assigned(rest: &str) -> bool {
    let mut depth = 0;
    for (i, c) in rest.char_indices() {
        depth += match c {
            '[' => 1,
            ']' => -1,
            _ => 0,
        };
        if depth == 0 {
            let after = rest[i + 1..].trim_start();
            return ["+=", "-=", "*=", "/="].iter().any(|op| after.starts_with(op)) || after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>");
        }
    }
    false
}

/// Every line under `path` (a folder or a file) that writes the world's arrays: the place it is in (its path from
/// `root`), and the line.
fn scan(root: &Path, path: &Path, hits: &mut Vec<(String, String)>) {
    let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
    if path.is_dir() {
        for e in std::fs::read_dir(path).unwrap().flatten() {
            scan(root, &e.path(), hits);
        }
    } else if rel.ends_with(".rs") {
        for (i, line) in std::fs::read_to_string(path).unwrap().lines().enumerate().filter(|(_, l)| writes(l)) {
            hits.push((rel.clone(), format!("{rel}:{}: {}", i + 1, line.trim())));
        }
    }
}

/// The exemption `rel` falls under, if any: a file named, or a file in a folder named.
fn exempt(rel: &str) -> Option<&'static str> {
    EXEMPT.iter().map(|e| e.0).find(|&p| rel == p || rel.starts_with(&format!("{p}/")))
}

#[test]
fn laws_on_the_store_write_no_array_of_the_world() {
    let caught = ["w.ground[k * n + i] += m;", "(w.e_ground[i], w.potential) = (a, b);", "let g = &mut w.air;", "e_air[j] -= q;", "w.deep[ks[q] * n + i] = 0.0;"];
    let passed = ["let a = w.ground[k * n + i];", "if w.air[j] == 0.0 {", "s.bed[i].vol -= m;", "let (x, y) = (w.e_ground[i], 1.0);", "airs[l] = j;"];
    assert!(caught.iter().all(|l| writes(l)) && !passed.iter().any(|l| writes(l)), "the scanner");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/src");
    let mut hits = Vec::new();
    scan(&root, &root, &mut hits);
    let broken: Vec<&str> = hits.iter().filter(|h| exempt(&h.0).is_none()).map(|h| h.1.as_str()).collect();
    assert!(broken.is_empty(), "code that writes the world's arrays (book it through store::apply):\n{}", broken.join("\n"));
    let stale: Vec<&str> = EXEMPT.iter().map(|e| e.0).filter(|&p| !hits.iter().any(|h| exempt(&h.0) == Some(p))).collect();
    assert!(stale.is_empty(), "exemptions no longer needed (delete them): {stale:?}");
}

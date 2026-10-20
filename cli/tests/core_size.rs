//! The code-size test (handover 14.3): `core/src` holds at most `CEILING` lines of code. A code line is one that is
//! not blank and does not start with `//` after its spaces, so comments are free.

use std::collections::BTreeMap;
use std::path::Path;

/// The most code lines `core/src` may hold: 9,000, and 700 more for running the bodies on threads (decision 42).
const CEILING: usize = 9_700;

/// Is `line` a line of code (not blank, not a comment)?
fn is_code(line: &str) -> bool {
    let t = line.trim_start();
    !t.is_empty() && !t.starts_with("//")
}

/// Add the code lines of every `.rs` file under `dir` to `by_folder`, keyed by the folder under `core/src` (`.` for
/// the files directly in it).
fn count(dir: &Path, folder: &str, by_folder: &mut BTreeMap<String, usize>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            count(&p, if folder == "." { &name } else { folder }, by_folder);
        } else if name.ends_with(".rs") {
            let lines = std::fs::read_to_string(&p).unwrap().lines().filter(|l| is_code(l)).count();
            *by_folder.entry(folder.to_string()).or_default() += lines;
        }
    }
}

#[test]
fn core_stays_under_its_code_ceiling() {
    assert!(is_code("let a = 1; // b") && !is_code("   // a") && !is_code("  ") && !is_code("/// doc"), "the rule");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/src");
    let mut by_folder = BTreeMap::new();
    count(&src, ".", &mut by_folder);
    let total: usize = by_folder.values().sum();
    for (folder, n) in &by_folder {
        println!("{folder:>8} {n:>6}");
    }
    println!("{:>8} {total:>6} (ceiling {CEILING})", "core");
    assert!(total <= CEILING, "core/src holds {total} code lines, above the ceiling of {CEILING}");
}

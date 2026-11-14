//! `docs/numbers.md` has one row for every field of `Physics` and `Config`, and no row for a field that does not
//! exist: a number cannot be added without saying what it is.

use std::collections::BTreeSet;
use std::path::Path;

const MARKS: [&str; 7] = ["physical", "anchored", "tuned", "switch", "container", "speed", "setting"];

/// The headings whose tables have the rows of the fields.
const FIELD_HEADINGS: [&str; 3] = ["Config", "Speed settings", "Physics"];

/// A table row of the file: the "## " heading it is under and its cells.
struct Row {
    heading: String,
    cells: Vec<String>,
}

/// Every table row of `docs/numbers.md` but the header rows and the rules (`|---`).
fn rows() -> Vec<Row> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/numbers.md");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (mut heading, mut out) = (String::new(), Vec::new());
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            heading = h.trim().to_string();
        } else if line.starts_with('|') && !line.starts_with("|---") {
            let cells: Vec<String> = line.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect();
            out.push(Row { heading: heading.clone(), cells });
        }
    }
    out
}

/// The rows of the fields (under the `FIELD_HEADINGS`): name and mark.
fn field_rows() -> Vec<(String, String)> {
    let rows = rows().into_iter().filter(|r| FIELD_HEADINGS.contains(&r.heading.as_str()) && r.cells[0].starts_with('`'));
    rows.map(|r| {
        assert!(r.cells.len() == 5, "a row has five cells (name, unit, fact, anchor, mark): {:?}", r.cells);
        (r.cells[0].trim_matches('`').to_string(), r.cells[4].clone())
    })
    .collect()
}

fn names<T: serde::Serialize>(x: &T) -> Vec<String> {
    match serde_json::to_value(x).expect("serialises") {
        serde_json::Value::Object(m) => m.keys().cloned().collect(),
        _ => panic!("a struct serialises to a map"),
    }
}

fn fields() -> BTreeSet<String> {
    let mut f: BTreeSet<String> = names(&pcore::Physics::default()).into_iter().collect();
    f.extend(names(&pcore::Config::default()));
    f
}

#[test]
fn every_field_has_a_row() {
    let have: BTreeSet<String> = field_rows().into_iter().map(|r| r.0).collect();
    let missing: Vec<String> = fields().into_iter().filter(|f| !have.contains(f)).collect();
    assert!(missing.is_empty(), "docs/numbers.md has no row for: {missing:?}");
}

#[test]
fn no_row_is_for_a_field_that_does_not_exist() {
    let want = fields();
    let stale: Vec<String> = field_rows().into_iter().map(|r| r.0).filter(|n| !want.contains(n)).collect();
    assert!(stale.is_empty(), "docs/numbers.md has rows for fields that do not exist: {stale:?}");
}

#[test]
fn every_row_has_one_name_and_a_known_mark() {
    let mut seen = BTreeSet::new();
    for (name, mark) in field_rows() {
        assert!(seen.insert(name.clone()), "two rows for {name}");
        assert!(MARKS.iter().any(|m| mark.starts_with(m)), "{name}: the mark is one of {MARKS:?}, not {mark:?}");
    }
}

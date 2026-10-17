//! `docs/numbers.md` has one row for every field of `Physics` and `Config`, and no row for a field that does not
//! exist: a number cannot be added without saying what it is. Its tuned numbers and the rules chosen for a run stay
//! within their budgets.

use std::collections::BTreeSet;
use std::path::Path;

/// Rows marked tuned in the whole file: it may not grow. A new number must be physical or anchored, or replace a tuned
/// one. Lower this when a tuned number is anchored or removed; do not raise it without asking.
const TUNED_BUDGET: usize = 61;

/// Rules chosen for a run (what was picked so a run came out a certain way, not a number of `Physics` or `Config`): the
/// table "Rules chosen for a run" lists them, and it may not grow.
const RULES_BUDGET: usize = 3;

const MARKS: [&str; 5] = ["physical", "anchored", "tuned", "switch", "container"];

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

/// The rows of the fields (under "## Config" and "## Physics"): name and mark.
fn field_rows() -> Vec<(String, String)> {
    let rows = rows().into_iter().filter(|r| (r.heading == "Config" || r.heading == "Physics") && r.cells[0].starts_with('`'));
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

#[test]
fn the_rules_chosen_for_a_run_stay_within_their_budget() {
    let n = rows().into_iter().filter(|r| r.heading == "Rules chosen for a run" && r.cells[0] != "rule").count();
    assert!(n > 0, "the table of rules chosen for a run is missing");
    assert!(n <= RULES_BUDGET, "{n} rules chosen for a run; the budget is {RULES_BUDGET}");
}

#[test]
fn the_tuned_numbers_stay_within_their_budget() {
    let tuned = rows().into_iter().filter(|r| r.cells.len() == 5 && r.cells[4].starts_with("tuned")).count();
    assert!(tuned <= TUNED_BUDGET, "{tuned} rows are marked tuned; the budget is {TUNED_BUDGET}");
}

//! `docs/numbers.md` has one row for every field of `Physics` and `Config`, and no row for a
//! field that does not exist (review 5, drift 1, fix 1): a number cannot be added without saying
//! what it is.

use std::collections::BTreeSet;
use std::path::Path;

/// Fields that rows are written for ahead of the change that adds them (review 5, rows 1 and 2).
/// Remove a name from here when its field exists: then a stale row for it is an error again.
const AHEAD: [&str; 0] = [];

/// Rows marked tuned in the whole file (handover 14.3): it may not grow. A new number must be
/// physical or anchored, or replace a tuned one. Lower this when a tuned number is anchored or
/// removed; do not raise it without asking (handover 14.3).
const TUNED_BUDGET: usize = 62;

const MARKS: [&str; 5] = ["physical", "anchored", "tuned", "switch", "container"];

fn names<T: serde::Serialize>(x: &T) -> Vec<String> {
    match serde_json::to_value(x).expect("serialises") {
        serde_json::Value::Object(m) => m.keys().cloned().collect(),
        _ => panic!("a struct serialises to a map"),
    }
}

/// The rows of the tables under the headings "## Config" and "## Physics" of the file (up to the
/// next "## " heading that is neither): (field name, mark).
fn rows() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/numbers.md");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (mut on, mut out) = (false, Vec::new());
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            on = h.trim() == "Config" || h.trim() == "Physics";
        } else if on && line.starts_with("| `") {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert!(cells.len() == 5, "a row has five cells (name, unit, fact, anchor, mark): {line}");
            let name = cells[0].trim_matches('`').to_string();
            out.push((name, cells[4].to_string()));
        }
    }
    out
}

fn fields() -> BTreeSet<String> {
    let mut f: BTreeSet<String> = names(&pcore::Physics::default()).into_iter().collect();
    f.extend(names(&pcore::Config::default()));
    f
}

#[test]
fn every_field_has_a_row() {
    let have: BTreeSet<String> = rows().into_iter().map(|r| r.0).collect();
    let missing: Vec<String> = fields().into_iter().filter(|f| !have.contains(f)).collect();
    assert!(missing.is_empty(), "docs/numbers.md has no row for: {missing:?}");
}

#[test]
fn no_row_is_for_a_field_that_does_not_exist() {
    let want = fields();
    let stale: Vec<String> = rows().into_iter().map(|r| r.0).filter(|n| !want.contains(n) && !AHEAD.contains(&n.as_str())).collect();
    assert!(stale.is_empty(), "docs/numbers.md has rows for fields that do not exist: {stale:?}");
}

#[test]
fn every_row_has_one_name_and_a_known_mark() {
    let mut seen = BTreeSet::new();
    for (name, mark) in rows() {
        assert!(seen.insert(name.clone()), "two rows for {name}");
        assert!(MARKS.iter().any(|m| mark.starts_with(m)), "{name}: the mark is one of {MARKS:?}, not {mark:?}");
    }
}

/// Rules chosen for a run (review 6, finding 9): what was picked so a run came out a certain way
/// and is not a number of `Physics` or `Config`. The table "Rules chosen for a run" lists them; it
/// may not grow (handover 14.3).
const RULES_BUDGET: usize = 3;

#[test]
fn the_rules_chosen_for_a_run_stay_within_their_budget() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/numbers.md");
    let text = std::fs::read_to_string(&path).expect("docs/numbers.md");
    let (mut on, mut n) = (false, 0);
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            on = h.trim() == "Rules chosen for a run";
        } else if on && line.starts_with("| ") && !line.starts_with("| rule |") && !line.starts_with("|---") {
            n += 1;
        }
    }
    assert!(n > 0, "the table of rules chosen for a run is missing");
    assert!(n <= RULES_BUDGET, "{n} rules chosen for a run; the budget is {RULES_BUDGET} (handover 14.3)");
}

#[test]
fn the_tuned_numbers_stay_within_their_budget() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/numbers.md");
    let text = std::fs::read_to_string(&path).expect("docs/numbers.md");
    let tuned = text
        .lines()
        .filter(|l| l.starts_with('|'))
        .filter(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            cells.len() == 5 && cells[4].starts_with("tuned")
        })
        .count();
    assert!(tuned <= TUNED_BUDGET, "{tuned} rows are marked tuned; the budget is {TUNED_BUDGET} (handover 14.3)");
}

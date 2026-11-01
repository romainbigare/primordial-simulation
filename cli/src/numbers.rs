//! `ew numbers A B`: the numbers that differ between two runs' `physics.json` files.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Every leaf of a document with its path (`physics.take`, `config.phys_days`).
fn leaves(prefix: &str, v: &Value, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, x)| leaves(&if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") }, x, out)),
        _ => {
            out.insert(prefix.to_string(), v.clone());
        }
    }
}

/// A run's directory or its `physics.json`.
fn load(arg: &str) -> Result<BTreeMap<String, Value>, String> {
    let path = if std::path::Path::new(arg).is_dir() { format!("{arg}/physics.json") } else { arg.to_string() };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let mut out = BTreeMap::new();
    leaves("", &doc, &mut out);
    Ok(out)
}

/// The lines (name, old, new) of the fields that differ; a field only one file has reads `-` in the other.
fn diff(a: &str, b: &str) -> Result<Vec<[String; 3]>, String> {
    let (a, b) = (load(a)?, load(b)?);
    let show = |v: Option<&Value>| v.map_or("-".to_string(), |v| v.to_string());
    let names: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    Ok(names.into_iter().filter(|k| a.get(*k) != b.get(*k)).map(|k| [k.clone(), show(a.get(k)), show(b.get(k))]).collect())
}

/// Print the differing fields as an aligned table, or say that the numbers are the same.
pub fn run(args: &[String]) {
    let (Some(a), Some(b)) = (args.get(2), args.get(3)) else {
        eprintln!("usage: ew numbers OLD NEW   (each a run's directory or its physics.json)");
        std::process::exit(2);
    };
    match diff(a, b) {
        Err(e) => {
            eprintln!("numbers: {e}");
            std::process::exit(1);
        }
        Ok(rows) if rows.is_empty() => println!("the numbers are the same"),
        Ok(rows) => {
            let w = rows.iter().fold([4, 3, 3], |w, r| [0, 1, 2].map(|i| w[i].max(r[i].len())));
            println!("{:<a$}  {:<b$}  new", "name", "old", a = w[0], b = w[1]);
            for r in rows {
                println!("{:<a$}  {:<b$}  {}", r[0], r[1], r[2], a = w[0], b = w[1]);
            }
        }
    }
}

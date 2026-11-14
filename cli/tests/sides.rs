//! The natural world names no module of the bodies (laws.md 0, "The contract"): the ground, the air, the cover and
//! Change run, build and are tested with no body code, and bodies reach them only through the contract (`contract`)
//! and the cell's view of the store. Like the word ban, a scan of the source.
//!
//! Files that may name both sides: `schedule.rs` (it lists the passes of both sides in their order), `lib.rs` (it
//! declares every module), `world.rs` (the world holds the bodies' registry and their counters), `survey.rs` and
//! `life/` (the passes of a cell's life and its registry, which the cover's chunks share), `params.rs` (the numbers of
//! both).
//!
//! The other way round, the takers (`fauna/`) reach the land only through the contract: they name no module of the
//! natural world's laws, only the contract, the shared life (`life`) and plain data (the chemistry, the units, the
//! numbers, the draws).

use std::path::Path;

/// The natural world under `core/src`: its folders and its files, and the contract between it and the bodies.
const NATURAL: [&str; 14] = ["flow", "change", "cover", "space", "store", "chem", "solve", "clock", "inside", "ground.rs", "check.rs", "math.rs", "units.rs", "contract"];

/// The modules of the bodies.
const BODIES: [&str; 9] = ["body", "choose", "copy", "bond", "nodes", "brain", "lineage", "lanes", "fauna"];

/// The modules of the natural world's laws, which the takers reach only through the contract.
const LAND_LAWS: [&str; 10] = ["flow", "change", "cover", "space", "store", "solve", "clock", "inside", "ground", "check"];

/// A line without its comment: the code before `//`.
fn code(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// The first module of each path that starts with `crate::` in `code`, also in a group (`crate::{a, b::{c}}`).
fn crate_modules(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in code.match_indices("crate::") {
        let rest = &code[at + "crate::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            // The items of the group at its own depth, each up to `::`, `,` or the group's end.
            let (mut depth, mut item) = (1, String::new());
            for c in group.chars() {
                match c {
                    '{' => depth += 1,
                    '}' if depth == 1 => break,
                    '}' => depth -= 1,
                    ',' if depth == 1 => item.clear(),
                    c if depth == 1 && (c.is_alphanumeric() || c == '_') => item.push(c),
                    ':' if depth == 1 && !item.is_empty() => {
                        out.push(std::mem::take(&mut item));
                    }
                    _ => {}
                }
            }
            if !item.is_empty() {
                out.push(item);
            }
        } else {
            out.push(rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect());
        }
    }
    out
}

/// Every line under `p` whose code names one of the modules `named`.
fn scan(p: &Path, named: &[&str], hits: &mut Vec<String>) {
    if p.is_dir() {
        for e in std::fs::read_dir(p).unwrap().flatten() {
            scan(&e.path(), named, hits);
        }
        return;
    }
    let text = std::fs::read_to_string(p).unwrap();
    for (i, line) in text.lines().enumerate() {
        for m in crate_modules(code(line)).iter().filter(|m| named.contains(&m.as_str())) {
            hits.push(format!("{}:{}: crate::{m}", p.display(), i + 1));
        }
    }
}

#[test]
fn the_natural_world_names_no_module_of_the_bodies() {
    let found = |s: &str| crate_modules(s);
    assert_eq!(found("use crate::{body::Body, store::{Class, Node}, life::{self}, World};"), ["body", "store", "life", "World"]);
    assert_eq!(found("crate::lanes::regrow(w, dt); crate::cover::step(w, d)"), ["lanes", "cover"]);
    assert!(found(code("x // crate::body")).is_empty(), "a comment names nothing");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/src");
    let mut hits = Vec::new();
    for n in NATURAL {
        let p = src.join(n);
        assert!(p.exists(), "{} is gone: say what the natural world is", p.display());
        scan(&p, &BODIES, &mut hits);
    }
    assert!(hits.is_empty(), "the natural world names the bodies:\n{}", hits.join("\n"));
}

#[test]
fn the_takers_reach_the_land_only_through_the_contract() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/src/fauna");
    assert!(p.exists(), "{} is gone: say where the takers are", p.display());
    let mut hits = Vec::new();
    scan(&p, &LAND_LAWS, &mut hits);
    assert!(hits.is_empty(), "the takers name a law of the land:\n{}", hits.join("\n"));
}

//! The three detectors find what they should and nothing else, on tapes made by hand (frames) and from a small world
//! (snapshots).

use flate2::{write::GzEncoder, Compression};
use pcore::bond;
use pcore::store::Node;
use pcore::{step, World};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use tape::{Recorder, LAYERS};
use worldgen::life::Seeding;
use worldgen::schedule::Schedule;
use worldgen::testkit::{dry_lands, part_of, set_amount, small_world, ROCK};

/// A folder of this test process for `name`.
fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ew_analyze_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Side of the frames' squares and the years of the frames: a frame every ten years.
const SIDE: usize = 16;
const YEARS: [f64; 6] = [0.0, 10.0, 20.0, 30.0, 40.0, 50.0];

/// A viewer file in `dir` of `YEARS.len()` frames in which `set(frame, layer name, square)` gives the value of a square
/// (zero where it gives none).
fn write_frames(dir: &std::path::Path, set: impl Fn(usize, &str, usize) -> f64) {
    let frames: Vec<Value> = YEARS.iter().map(|&y| json!({"day": 0, "year": y})).collect();
    let layers: Vec<Value> = LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect();
    let head = serde_json::to_vec(&json!({"w": SIDE, "h": SIDE, "globe": false, "layers": layers, "frames": frames})).unwrap();
    let mut z = GzEncoder::new(std::fs::File::create(dir.join("world.ewf.gz")).unwrap(), Compression::fast());
    z.write_all(b"EWF2").unwrap();
    z.write_all(&(head.len() as u32).to_le_bytes()).unwrap();
    z.write_all(&head).unwrap();
    for t in 0..YEARS.len() {
        for l in LAYERS {
            let bytes: Vec<u8> = (0..SIDE * SIDE).map(|s| tape::quantise(l, set(t, l.name, s))).collect();
            z.write_all(&bytes).unwrap();
        }
    }
    z.finish().unwrap();
}

fn cards(dir: &std::path::Path, category: &str) -> Vec<Value> {
    let report = analyze::analyze(dir).unwrap();
    report["cards"].as_array().unwrap().iter().filter(|c| c["category"] == category).cloned().collect()
}

/// The squares of the `side`-wide block at (x, y).
fn block(x: usize, y: usize, side: usize) -> impl Fn(usize) -> bool {
    move |s| (x..x + side).contains(&(s % SIDE)) && (y..y + side).contains(&(s / SIDE))
}

/// A dense block that stays is a group; one that is dense for two frames only, and a dense spot of one square, are not.
#[test]
fn a_dense_block_that_stays_is_a_group() {
    let dir = tmp("groups");
    let (stays, brief, spot) = (block(3, 3, 3), block(10, 10, 3), block(12, 2, 1));
    write_frames(&dir, |t, layer, s| match layer {
        "animals" if stays(s) || spot(s) || (brief(s) && t < 2) => 1.0,
        "animals" => 1e-3,
        "bodies" if stays(s) => 2.0,
        _ => 0.0,
    });
    let found = cards(&dir, "group");
    assert_eq!(found.len(), 1, "{found:?}");
    let g = &found[0];
    assert_eq!((g["cells"].as_u64(), g["years"].as_f64(), g["first_seen_year"].as_f64(), g["peak_bodies"].as_f64()), (Some(9), Some(50.0), Some(0.0), Some(18.0)));
    assert_eq!(g["centre"], json!([4.0, 4.0]));
    assert_eq!(g["pattern_id"], json!(format!("group-{}", 3 * SIDE + 3)));
    assert!(cards(&dir, "route").is_empty() && cards(&dir, "structure").is_empty());
}

/// A worn line that stays is a route with its two ends and its traffic; a short one and one that wears for a single frame
/// are not.
#[test]
fn a_worn_line_that_stays_is_a_route() {
    let dir = tmp("routes");
    let line = |s: usize| s / SIDE == 8 && (2..8).contains(&(s % SIDE));
    let short = |s: usize| s / SIDE == 2 && (2..4).contains(&(s % SIDE));
    let brief = |s: usize| s / SIDE == 12 && (2..8).contains(&(s % SIDE));
    write_frames(&dir, |t, layer, s| match layer {
        "worn" if line(s) || short(s) || (brief(s) && t == 1) => 3.0,
        "worn" => 1.0,
        "traffic" if line(s) => 100.0,
        _ => 0.0,
    });
    let found = cards(&dir, "route");
    assert_eq!(found.len(), 1, "{found:?}");
    let r = &found[0];
    assert_eq!((r["cells"].as_u64(), r["years"].as_f64()), (Some(6), Some(50.0)));
    assert_eq!(r["ends"], json!([[2, 8], [7, 8]]));
    assert!((r["mean_traffic"].as_f64().unwrap() / 100.0 - 1.0).abs() < 0.05, "{r}");
    assert!(cards(&dir, "group").is_empty());
}

/// Snapshot of `w` in `dir` as `ew run` writes it.
fn snapshot(w: &World, dir: &std::path::Path, name: &str) {
    let plan = (Schedule::new(None, false, Seeding::default()), Recorder::new(w, 0, 0).kept());
    let mut bytes = vec![];
    tape::snapshot_bytes(w, &plan, &mut bytes).unwrap();
    tape::write_snapshot(&bytes, dir.join(name)).unwrap();
}

/// A part of the common rock in cell `i`, flat or hollow.
fn rock(w: &mut World, i: usize, hollow: f32) -> u64 {
    set_amount(w, Node::ground(i), ROCK, 1e6);
    let mut wall = vec![0.0; w.chem.ns()];
    wall[ROCK] = 10.0;
    part_of(w, i, wall, vec![0.0; w.chem.ns()], -1.0, hollow)
}

/// Structures of the same bond graph and bands count as copies, however far apart; a structure of one kind stands once is no
/// card; and the first snapshot with a structure gives its first seen.
#[test]
fn copies_of_a_structure_are_counted() {
    let dir = tmp("structures");
    let mut w = small_world();
    let cells = dry_lands(&w);
    // A pair of flat rock plates in the first cell, and a lone hollow part.
    let pair = |w: &mut World, i: usize| {
        let (a, b) = (rock(w, i, 0.0), rock(w, i, 0.0));
        bond::link(w, a, b, 1e5);
    };
    pair(&mut w, cells[0]);
    rock(&mut w, cells[3], 0.8);
    let first = w.year();
    snapshot(&w, &dir, "snap_00001.bin.gz");
    assert!(cards(&dir, "structure").is_empty(), "one copy is not repeated");
    for _ in 0..6 {
        step(&mut w);
    }
    pair(&mut w, cells[1]);
    pair(&mut w, cells[2]);
    rock(&mut w, cells[4], 0.0);
    rock(&mut w, cells[5], 0.0);
    snapshot(&w, &dir, "snap_00002.bin.gz");
    let found = cards(&dir, "structure");
    assert_eq!(found.len(), 2, "{found:?}");
    let (pairs, singles) = (found.iter().find(|c| c["parts"] == 2).unwrap(), found.iter().find(|c| c["parts"] == 1).unwrap());
    assert_eq!((pairs["count"].as_u64(), singles["count"].as_u64()), (Some(3), Some(2)));
    assert_eq!((pairs["held_share"].as_f64(), pairs["enclosed_share"].as_f64()), (Some(0.0), Some(0.0)));
    assert!(pairs["mass"].as_f64().unwrap() > 0.0);
    // The pair stood at the first snapshot; the singles are new.
    assert_eq!((pairs["first_seen_year"].as_f64(), pairs["first_seen_exact"].as_bool()), (Some(first), Some(false)));
    assert_eq!(singles["first_seen_exact"], json!(true));
    assert!(pairs.get("note").is_none());
    // With the end snapshot alone, the cards say so.
    std::fs::remove_file(dir.join("snap_00001.bin.gz")).unwrap();
    assert!(cards(&dir, "structure").iter().all(|c| c["note"].is_string() && c["first_seen_exact"] == json!(false)));
}

use flate2::read::GzDecoder;
use pcore::{check::hash, step, World};
use std::io::Read;

fn world() -> World {
    worldgen::new_world(1, 32, 32)
}

fn run(w: &mut World, months: u32) {
    for _ in 0..months {
        step(w);
    }
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("ew_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn same_seed_same_hash() {
    let (mut a, mut b) = (world(), world());
    run(&mut a, 3);
    run(&mut b, 3);
    assert_eq!(hash(&a), hash(&b));
}

#[test]
fn snapshot_round_trip() {
    let dir = tmp("snap");
    let (mut a, mut whole) = (world(), world());
    run(&mut a, 2);
    tape::save_snapshot(&a, dir.join("s.bin.gz")).unwrap();
    let mut b = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    assert_eq!(hash(&a), hash(&b));
    run(&mut b, 2);
    run(&mut whole, 4);
    assert_eq!(hash(&b), hash(&whole));
}

/// Save and load in the middle of the slow cycles of Flow (refresh, work, creep) and with life
/// about: the resumed run must give the same history as the unbroken one.
#[test]
fn snapshot_round_trip_mid_cycle() {
    let dir = tmp("snap_mid");
    let (mut a, mut whole) = (world(), world());
    for w in [&mut a, &mut whole] {
        worldgen::life::seed(w, 0);
        worldgen::life::seed(w, 1);
    }
    run(&mut a, 17);
    tape::save_snapshot(&a, dir.join("s.bin.gz")).unwrap();
    let mut b = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    assert_eq!(hash(&a), hash(&b));
    run(&mut b, 14);
    run(&mut whole, 31);
    assert_eq!(hash(&b), hash(&whole));
}

#[test]
fn frames_file_round_trip() {
    let dir = tmp("frames");
    let mut w = world();
    let mut rec = tape::Recorder::new(&w, 1, 1);
    rec.record_month(&mut w);
    run(&mut w, 1);
    rec.record_month(&mut w);
    rec.finish(&dir).unwrap();

    let mut raw = vec![];
    GzDecoder::new(std::fs::File::open(dir.join("world.ewf.gz")).unwrap()).read_to_end(&mut raw).unwrap();
    assert_eq!(&raw[..4], b"EWF1");
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&raw[8..8 + n]).unwrap();
    let (cells, layers, frames) = (32 * 32, meta["layers"].as_array().unwrap().len(), meta["frames"].as_array().unwrap().len());
    assert_eq!((meta["w"].as_u64(), frames), (Some(32), 2));
    assert_eq!(raw.len() - 8 - n, cells * layers * frames);
    // Layer "z" of frame 0 is the first layer: check one cell.
    let z = &tape::LAYERS[0];
    assert_eq!(z.name, meta["layers"][0]["name"].as_str().unwrap());
    let first = tape::quantise(z, worldgen::new_world(1, 32, 32).surface_z(100));
    assert_eq!(raw[8 + n + 100], first);
    assert!(meta["events"].as_array().unwrap().iter().any(|e| e["text"].as_str().unwrap().contains("first appears")));
}

#[test]
fn yearly_series_names_gases_and_catalysts() {
    let dir = tmp("series");
    let mut w = world();
    let mut rec = tape::Recorder::new(&w, 0, 0);
    for _ in 0..w.cfg.months_per_year {
        run(&mut w, 1);
        rec.record_month(&mut w);
    }
    rec.finish(&dir).unwrap();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let head: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
    let gases = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0).count();
    assert!(gases > 0 && head.contains(&"absorber"));
    assert_eq!(head.iter().filter(|h| h.starts_with("air_")).count(), gases);
    assert_eq!(head.iter().filter(|h| h.starts_with("cat_")).count(), w.chem.reactions.len());
}

/// The per-kind columns (`root_<k>_*`) are in the series, deaths and births of the kinds add up to
/// the totals of the year, and the land's seasonal swing is there and not negative.
#[test]
fn yearly_series_has_the_measures_per_starting_kind() {
    let dir = tmp("per_kind");
    let mut w = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let mut rec = tape::Recorder::new(&w, 0, 0);
    rec.record_month(&mut w);
    for _ in 0..3 * w.cfg.months_per_year {
        run(&mut w, 1);
        rec.record_month(&mut w);
    }
    rec.finish(&dir).unwrap();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').map(|v| v.parse().unwrap()).collect()).collect();
    assert_eq!(rows.len(), 4);
    let col = |n: &str, r: &Vec<f64>| r[head.iter().position(|h| *h == n).unwrap_or_else(|| panic!("no column {n}"))];
    let kinds: Vec<usize> = (0..8).filter(|k| head.contains(&format!("root_{k}_members").as_str())).collect();
    assert!(!kinds.is_empty());
    for name in ["births", "life_real", "mass", "copy_mass", "gas", "leaf", "copy_log", "interval", "hidden", "died_age"] {
        assert!(head.contains(&format!("root_{}_{name}", kinds[0]).as_str()), "{name}");
    }
    for r in &rows {
        for (kind_col, total) in [("died_lack", "died_lack"), ("died_phase", "died_phase"), ("died_spent", "died_spent"), ("died_taken", "died_taken"), ("died_broken", "died_broken"), ("died_age", "died_age"), ("births", "births")] {
            let sum: f64 = kinds.iter().map(|k| col(&format!("root_{k}_{kind_col}"), r)).sum();
            let all = col(total, r);
            assert!((sum - all).abs() <= 1e-9 * all.max(1.0), "{total}: kinds {sum} total {all}");
        }
    }
    let last = rows.last().unwrap();
    assert!(col("t_land_swing", last) >= 0.0 && col("t_land_summer", last) >= col("t_land_winter", last));
    // A catalyst gene on in a kind keeps its columns, with the share of members that carry it.
    assert!(head.iter().any(|h| h.starts_with("root_0_cat_") && h.ends_with("_on")));
    // The kind's mass in kg is its members times its mean member's mass; the heaviest member has a kind; the merge tally is there.
    for k in &kinds {
        let (m, mean, kg) = (col(&format!("root_{k}_members"), last), col(&format!("root_{k}_mass"), last), col(&format!("root_{k}_kg"), last));
        assert!(m == 0.0 && kg == 0.0 || (kg - m * mean).abs() <= 1e-9 * kg, "root_{k}_kg {kg} members {m} mean {mean}");
    }
    assert!(kinds.contains(&(col("mass_max_root", last) as usize)));
    assert!(col("merged_unlike", last) >= 0.0);
    // The run's numbers are written beside the series.
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("physics.json")).unwrap()).unwrap();
    assert_eq!(doc["physics"]["alike"].as_f64().map(|x| x as f32), Some(w.cfg.phys.alike));
    assert_eq!(doc["config"]["crowd"].as_u64(), Some(w.cfg.crowd as u64));
    assert!(doc["config"].get("phys").is_none());
}

/// What a tape folder holds: the viewer file's header and frame bytes, the series rows (as text)
/// and the (year, month) of each months.csv row.
fn read_tape(dir: &std::path::Path) -> (serde_json::Value, usize, Vec<String>, Vec<(u64, u64)>) {
    let mut raw = vec![];
    GzDecoder::new(std::fs::File::open(dir.join("world.ewf.gz")).unwrap()).read_to_end(&mut raw).unwrap();
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let meta = serde_json::from_slice(&raw[8..8 + n]).unwrap();
    let series = std::fs::read_to_string(dir.join("series.csv")).unwrap().lines().map(String::from).collect();
    let months = std::fs::read_to_string(dir.join("months.csv")).unwrap();
    let ym = |l: &str| (l.split(',').next().unwrap().parse().unwrap(), l.split(',').nth(1).unwrap().parse().unwrap());
    (meta, raw.len() - 8 - n, series, months.lines().skip(1).map(ym).collect())
}

/// A run written for 3 years and resumed from its year 2 snapshot into the same folder keeps
/// the years 0..2 it had, replays 3 and goes on to 4: series, months, frames and events with no
/// gap and no repeat.
#[test]
fn resume_into_the_same_folder_continues_the_tape() {
    let dir = tmp("resume");
    let mut w = world();
    let (mpy, dpy) = (w.cfg.months_per_year as u64, w.days_per_year());
    let mut rec = tape::Recorder::new(&w, 1, 0);
    rec.record_month(&mut w);
    for m in 1..=3 * mpy {
        run(&mut w, 1);
        rec.record_month(&mut w);
        if m == 2 * mpy {
            tape::save_snapshot(&w, dir.join("snap.bin.gz")).unwrap();
        }
    }
    rec.finish(&dir).unwrap();
    let (first, ..) = read_tape(&dir);
    let first_series = std::fs::read_to_string(dir.join("series.csv")).unwrap();

    let mut w = tape::load_snapshot(dir.join("snap.bin.gz")).unwrap();
    let mut rec = tape::Recorder::new(&w, 1, 0);
    rec.resume(&mut w, &dir).unwrap();
    rec.record_month(&mut w);
    for _ in 0..2 * mpy {
        run(&mut w, 1);
        rec.record_month(&mut w);
    }
    rec.finish(&dir).unwrap();

    let (meta, bytes, series, months) = read_tape(&dir);
    let years: Vec<&str> = series[1..].iter().map(|r| r.split(',').next().unwrap()).collect();
    assert_eq!(years, ["0", "1", "2", "3", "4"]);
    // The row of the snapshot's year is the one the first run wrote, not a new one.
    assert_eq!(series[..4], first_series.lines().take(4).map(String::from).collect::<Vec<_>>());
    assert_eq!(months, (0..=4 * mpy).map(|i| (i / mpy, i % mpy)).collect::<Vec<_>>());
    let days: Vec<u64> = meta["frames"].as_array().unwrap().iter().map(|f| f["day"].as_u64().unwrap()).collect();
    assert_eq!(days, (0..5).map(|y| y * dpy).collect::<Vec<_>>());
    assert_eq!(bytes, 5 * 32 * 32 * tape::LAYERS.len());
    assert_eq!(meta["events"], first["events"]);
    for col in meta["series"].as_object().unwrap().values() {
        assert_eq!(col.as_array().unwrap().len(), 5);
    }
}

/// A null run: with life about and copies made, nobody learns (the bodies' own adjustments stay
/// zero) and every born row is a mean of the starting ones (inside their range, as a node's rows
/// are the means of its members'); in the real run of the same world they are neither.
#[test]
fn null_run_freezes_the_brain_rows() {
    let go = |null: bool| {
        let mut w = world();
        w.cfg.null = null;
        worldgen::life::seed(&mut w, 0);
        worldgen::life::seed(&mut w, 1);
        let start: Vec<&Vec<f32>> = w.life.genomes.iter().filter(|g| !g.rows.is_empty()).map(|g| &g.rows).collect();
        let (lo, hi): (Vec<f32>, Vec<f32>) = (0..start[0].len()).map(|k| start.iter().fold((f32::MAX, f32::MIN), |m, r| (m.0.min(r[k]), m.1.max(r[k])))).unzip();
        run(&mut w, 24);
        let out = |r: &Vec<f32>| r.len() != lo.len() || r.iter().enumerate().any(|(k, &x)| x < lo[k] - 1e-5 || x > hi[k] + 1e-5);
        let new = w.life.genomes.iter().filter(|g| !g.rows.is_empty() && out(&g.rows)).count();
        let moved = w.life.alive().filter(|b| b.adj.iter().any(|&x| x != 0.0)).count();
        (new, moved, w.life.updates)
    };
    let (real, null) = (go(false), go(true));
    assert!(null.2 > 100 && real.2 > 100, "life ran: {real:?} {null:?}");
    assert!(real.0 > 0 && real.1 > 0, "the real run learns and mutates rows: {real:?}");
    assert_eq!((null.0, null.1), (0, 0), "the null run does not: {null:?}");
}

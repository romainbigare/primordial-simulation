//! The tape: snapshots (at each month's start through the slow cycles, refused without their header, carrying the
//! schedule), the viewer file, the yearly series and the months, and resuming into the same folder.

use flate2::read::GzDecoder;
use pcore::{step, World};
use std::io::Read;
use std::sync::OnceLock;
use tape::fingerprint;

mod common;

/// The side of the tests' worlds, cells.
const SIDE: usize = 16;

/// The world of the tape tests: seed 1, `SIDE` x `SIDE` cells.
fn world() -> World {
    worldgen::testkit::one_rise(1, SIDE, SIDE)
}

/// `months` months of `w`.
fn run(w: &mut World, months: u32) {
    for _ in 0..months {
        step(w);
    }
}

/// `w` with two kinds of cover, none of it fixing: the small fast ones and the light-eaters (rows 0 and 1).
fn fed(w: &mut World) {
    let how = worldgen::life::Seeding { fixers: 0.0, ..Default::default() };
    worldgen::life::seed_with(w, 0, &how);
    worldgen::life::seed_with(w, 1, &how);
}

/// The world with two kinds of cover (rows 0 and 1).
fn two_kinds() -> World {
    let mut w = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    w
}

/// Seed the third kind of cover (row 2), late in a run, and the label of its columns (`root_<k>_*`).
fn late(w: &mut World) -> String {
    worldgen::life::seed(w, 2);
    String::from("root_2_")
}

/// Save the world and the shell's state `plan` to `path`.
fn save_snapshot<P: serde::Serialize>(w: &World, plan: &P, path: std::path::PathBuf) -> std::io::Result<()> {
    let mut bytes = vec![];
    tape::snapshot_bytes(w, plan, &mut bytes)?;
    tape::write_snapshot(&bytes, path)
}

/// A folder of this test process for `name`.
fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("ew_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A file without the snapshot header (here: a bare bincode body) is refused with a clear error.
#[test]
fn a_file_without_the_snapshot_header_is_refused() {
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    let path = tmp("snap_bare").join("s.bin.gz");
    let mut z = GzEncoder::new(std::fs::File::create(&path).unwrap(), Compression::fast());
    z.write_all(&bincode::serialize(&world()).unwrap()).unwrap();
    z.finish().unwrap();
    let e = tape::load_snapshot::<()>(&path).expect_err("refused");
    assert!(e.to_string().contains("not a snapshot"), "{e}");
}

/// Months saved at, from the start: past a creep (once a year) and four refreshes (once in three months) of Flow.
const SAVED: usize = 14;
/// Months each resumed run goes on for: through a creep and the refreshes.
const RESUMED: usize = 12;

/// Save and load at each month's start through the slow cycles of Flow (refresh, creep) with life about: the loaded
/// world is the saved one, and each resumed run gives the same history as the unbroken one, month by month.
#[test]
fn snapshot_round_trip_at_every_month() {
    let mut w = two_kinds();
    let (mut saved, mut whole) = (vec![], vec![]);
    for m in 0..SAVED + RESUMED {
        if m < SAVED {
            let mut bytes = vec![];
            tape::snapshot_bytes(&w, &(), &mut bytes).unwrap();
            saved.push((fingerprint(&w), bytes));
        }
        step(&mut w);
        whole.push(fingerprint(&w));
    }
    for (m, (at, bytes)) in saved.iter().enumerate() {
        let (mut b, ()) = tape::read_snapshot(&bytes[..]).unwrap();
        assert_eq!(fingerprint(&b), *at, "the world loaded at month {m}");
        for (k, unbroken) in whole[m..m + RESUMED].iter().enumerate() {
            step(&mut b);
            assert_eq!(fingerprint(&b), *unbroken, "saved at month {m}, resumed for {} months", k + 1);
        }
    }
}

/// A roster run resumed from a snapshot taken before some of its kinds are due places them as the unbroken run does:
/// the snapshot carries the schedule, and both runs end with the same world.
#[test]
fn a_resumed_run_places_the_kinds_still_due() {
    use worldgen::life::{Seeding, ROSTER};
    use worldgen::schedule::Schedule;
    let dir = tmp("resume_plan");
    // The roster from year 0, its second half due at year 1, after the snapshot.
    let mut plan = Schedule::new(Some(0), &ROSTER, Seeding::default());
    plan.kinds.iter_mut().enumerate().for_each(|(n, k)| k.1 = (2 * n >= ROSTER.len()) as u64);
    let late: Vec<usize> = plan.kinds.iter().filter(|k| k.1 > 0).map(|k| k.0).collect();
    let placed = |w: &World, k: usize| pcore::cover::sums(w).get(k).is_some_and(|s| s.slots > 0.0);
    let years = |w: &mut World, plan: &Schedule, n: u64| {
        for _ in 0..n {
            plan.place(w);
            run(w, w.cfg.months_per_year);
        }
    };
    let (mut a, mut whole) = (world(), world());
    years(&mut whole, &plan, 2);
    years(&mut a, &plan, 1);
    assert!(late.iter().all(|&k| !placed(&a, k)), "a later kind placed before the snapshot");
    save_snapshot(&a, &plan, dir.join("s.bin.gz")).unwrap();
    let (mut b, plan): (World, Schedule) = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    // (Checked as they are placed: a kind placed in so small a world may die out within the year.)
    plan.place(&mut b);
    assert!(late.iter().all(|&k| placed(&b, k)), "the resumed run placed the later kinds");
    let months = b.cfg.months_per_year;
    run(&mut b, months);
    assert_eq!(fingerprint(&b), fingerprint(&whole));
}

/// The viewer file holds its header (the layers, the frames, the events) and a byte per cell, layer and frame.
#[test]
fn frames_file_round_trip() {
    let dir = tmp("frames");
    let mut w = world();
    let mut rec = tape::Recorder::new(&w, 1, 1);
    common::record(&mut rec, &mut w);
    run(&mut w, 1);
    common::record(&mut rec, &mut w);
    rec.finish(&dir).unwrap();

    let mut raw = vec![];
    GzDecoder::new(std::fs::File::open(dir.join("world.ewf.gz")).unwrap()).read_to_end(&mut raw).unwrap();
    assert_eq!(&raw[..4], b"EWF2");
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&raw[8..8 + n]).unwrap();
    let (cells, layers, frames) = (SIDE * SIDE, meta["layers"].as_array().unwrap().len(), meta["frames"].as_array().unwrap().len());
    assert_eq!((meta["w"].as_u64(), frames, meta["globe"].as_bool()), (Some(SIDE as u64), 2, Some(false)));
    assert_eq!(raw.len() - 8 - n, cells * layers * frames);
    // Layer "z" of frame 0 is the first layer: check one cell.
    let z = &tape::LAYERS[0];
    assert_eq!(z.name, meta["layers"][0]["name"].as_str().unwrap());
    let first = tape::quantise(z, world().surface_z(100));
    assert_eq!(raw[8 + n + 100], first);
    assert!(meta["events"].as_array().unwrap().iter().any(|e| e["text"].as_str().unwrap().contains("first appears")));
}

/// A window of the map is a raster of the cell that holds each pixel's centre (`tape::raster`): over the whole map at
/// one pixel a square it is the squares of the frames (`tape::pixels`).
#[test]
fn a_window_of_the_whole_map_shows_the_frames_squares() {
    let mut w = world();
    run(&mut w, 1);
    w.drain_events();
    let (across, down) = (w.map.w, w.map.h);
    assert_eq!(tape::raster(&w, [0.0, 0.0, across as f64, down as f64], across, down), tape::pixels(&w), "the whole map is the frames' squares");
}

/// The viewer file says whether the world is a globe and the latitudes of its edges, and the world's means of a globe are
/// by area (its rows narrow toward the poles): `t_ground` is the area-weighted mean of the cells' ground temperature.
#[test]
fn a_globe_file_names_its_shape_and_its_means_are_by_area() {
    let dir = tmp("globe");
    let (rows, mut w) = (8, worldgen::terrain::new_globe(1, 8, [0; 4]));
    let mut rec = tape::Recorder::new(&w, 1, 0);
    common::record(&mut rec, &mut w);
    for _ in 0..w.cfg.months_per_year {
        step(&mut w);
        common::record(&mut rec, &mut w);
    }
    rec.finish(&dir).unwrap();
    let mut raw = vec![];
    GzDecoder::new(std::fs::File::open(dir.join("world.ewf.gz")).unwrap()).read_to_end(&mut raw).unwrap();
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&raw[8..8 + n]).unwrap();
    assert_eq!((meta["globe"].as_bool(), meta["w"].as_u64(), meta["h"].as_u64()), (Some(true), Some(2 * rows), Some(rows)));
    assert_eq!((meta["lat_north"].as_f64(), meta["lat_south"].as_f64()), (Some(90.0), Some(-90.0)));
    let (t, area) = (tape::fields(&w).t, w.mesh.cells().area());
    let by_area = t.iter().zip(area).map(|(t, a)| t * a).sum::<f64>() / area.iter().sum::<f64>();
    assert!((rec.last("t_ground") - by_area).abs() < 1e-9, "{} against {by_area}", rec.last("t_ground"));
}

/// The recording the series tests read: `world` with the soil and the life of `fed`, recorded month by month for three
/// years, with the reaction that breaks the fixed mineral down (`MINERAL_BREAKS`) off, so that the mineral in all stays
/// what the soil gave; its folder, and the area of the land and of the low ground at each yearly row, m².
struct Recording {
    w: World,
    dir: std::path::PathBuf,
    areas: Vec<[f64; 2]>,
}

fn recording() -> &'static Recording {
    static RECORDING: OnceLock<Recording> = OnceLock::new();
    RECORDING.get_or_init(|| {
        let dir = tmp("series");
        let mut w = world();
        w.chem.reactions[worldgen::testkit::MINERAL_BREAKS].k = 0.0;
        worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
        // (No cover fixes: the mineral's whole is then kept.)
        fed(&mut w);
        let areas = |w: &World| {
            let area = w.mesh.cells().area();
            let land: f64 = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| area[i]).sum();
            [land, area.iter().sum::<f64>() - land]
        };
        let mpy = w.cfg.months_per_year;
        let mut rec = tape::Recorder::new(&w, 0, 0);
        common::record(&mut rec, &mut w);
        let mut at = vec![areas(&w)];
        for m in 1..=3 * mpy {
            run(&mut w, 1);
            common::record(&mut rec, &mut w);
            if m % mpy == 0 {
                at.push(areas(&w));
            }
        }
        rec.finish(&dir).unwrap();
        Recording { w, dir, areas: at }
    })
}

/// The series has a column for each heat-absorbing gas of the air.
#[test]
fn yearly_series_names_the_gases() {
    let Recording { w, dir, .. } = recording();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let head: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
    let gases = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0).count();
    assert!(gases > 0 && head.contains(&"absorber"));
    assert_eq!(head.iter().filter(|h| h.starts_with("air_")).count(), gases);
}

/// The per-kind columns of the cover (`root_<k>_*`) are in the series, the cover of the kinds adds up to the cover of
/// the world, the land's summer is not colder than its winter, and the run's numbers are written beside the series.
#[test]
fn yearly_series_has_the_measures_per_starting_kind() {
    let Recording { w, dir, .. } = recording();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').map(|v| v.parse().unwrap()).collect()).collect();
    assert_eq!(rows.len(), 4);
    let col = |n: &str, r: &Vec<f64>| r[head.iter().position(|h| *h == n).unwrap_or_else(|| panic!("no column {n}"))];
    let kinds: Vec<usize> = (0..16).filter(|k| head.contains(&format!("root_{k}_slots").as_str())).collect();
    assert!(!kinds.is_empty());
    for k in &kinds {
        assert!(["slots", "cover_kg", "cover_area", "cover_light", "cover_frost"].iter().all(|c| head.contains(&format!("root_{k}_{c}").as_str())));
    }
    let last = rows.last().unwrap();
    let sum: f64 = kinds.iter().map(|k| col(&format!("root_{k}_cover_kg"), last)).sum();
    assert!((sum - col("cover_kg", last)).abs() <= 1e-9 * sum, "the kinds' cover {sum}, the world's {}", col("cover_kg", last));
    assert!(col("t_land_summer", last) >= col("t_land_winter", last));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("physics.json")).unwrap()).unwrap();
    assert_eq!(doc["physics"]["alike"].as_f64().map(|x| x as f32), Some(w.cfg.phys.alike));
    assert_eq!(doc["config"]["days_per_month"].as_u64(), Some(w.cfg.days_per_month as u64));
    assert!(doc["config"].get("phys").is_none());
}

/// What a tape folder holds: the viewer file's header and frame bytes, the series rows (as text) and the (year, month) of
/// each of its months.
fn read_tape(dir: &std::path::Path) -> (serde_json::Value, usize, Vec<String>, Vec<(u64, u64)>) {
    let mut raw = vec![];
    GzDecoder::new(std::fs::File::open(dir.join("world.ewf.gz")).unwrap()).read_to_end(&mut raw).unwrap();
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&raw[8..8 + n]).unwrap();
    let series = std::fs::read_to_string(dir.join("series.csv")).unwrap().lines().map(String::from).collect();
    let months = meta["months"].as_array().unwrap().iter().map(|m| (m["year"].as_u64().unwrap(), m["month"].as_u64().unwrap())).collect();
    (meta, raw.len() - 8 - n, series, months)
}

/// A run with life written for 4 years and resumed from its year 1 snapshot into the same folder keeps the years 0..1
/// it had and replays 2 to 4 as the unbroken run wrote them: series rows, months, frames and events, with no gap and no
/// repeat (the snapshot carries what the recorder keeps across years: the substances seen). A kind seeded after the
/// snapshot has no columns until then, as in the unbroken run.
#[test]
fn resume_into_the_same_folder_continues_the_tape() {
    let dir = tmp("resume");
    let mut w = two_kinds();
    let (mpy, dpy) = (w.cfg.months_per_year as u64, w.days_per_year());
    let mut rec = tape::Recorder::new(&w, 1, 0);
    common::record(&mut rec, &mut w);
    // The third kind comes at the end of year 2, after the snapshot of year 1, in both runs.
    for m in 1..=4 * mpy {
        run(&mut w, 1);
        common::record(&mut rec, &mut w);
        if m == mpy {
            save_snapshot(&w, &rec.kept(), dir.join("snap.bin.gz")).unwrap();
        }
        if m == 2 * mpy {
            late(&mut w);
        }
    }
    rec.finish(&dir).unwrap();
    let whole = read_tape(&dir);

    let (mut w, kept) = tape::load_snapshot(dir.join("snap.bin.gz")).unwrap();
    let mut rec = tape::Recorder::new(&w, 1, 0);
    rec.resume(&w, kept, &dir).unwrap();
    common::record(&mut rec, &mut w);
    for m in mpy + 1..=4 * mpy {
        run(&mut w, 1);
        common::record(&mut rec, &mut w);
        if m == 2 * mpy {
            late(&mut w);
        }
    }
    rec.finish(&dir).unwrap();

    let (meta, bytes, series, months) = read_tape(&dir);
    let years: Vec<&str> = series[1..].iter().map(|r| r.split(',').next().unwrap()).collect();
    assert_eq!(years, ["0", "1", "2", "3", "4"]);
    for (row, (resumed, unbroken)) in series.iter().zip(&whole.2).enumerate() {
        let diff: Vec<&str> = series[0].split(',').zip(resumed.split(',').zip(unbroken.split(','))).filter(|(_, (a, b))| a != b).map(|(h, _)| h).collect();
        assert_eq!(resumed, unbroken, "series row {row}: {diff:?}");
    }
    assert_eq!(months, (0..=4 * mpy).map(|i| (i / mpy, i % mpy)).collect::<Vec<_>>());
    let days: Vec<u64> = meta["frames"].as_array().unwrap().iter().map(|f| f["day"].as_u64().unwrap()).collect();
    assert_eq!(days, (0..5).map(|y| y * dpy).collect::<Vec<_>>());
    assert_eq!(bytes, 5 * SIDE * SIDE * tape::LAYERS.len());
    assert_eq!((meta["events"].clone(), meta["series_units"].clone()), (whole.0["events"].clone(), whole.0["series_units"].clone()));
}

/// A tape that stops before the snapshot it is resumed from (a run that ended between the two writes) is refused, not
/// continued with a gap.
#[test]
fn a_tape_that_stops_before_the_snapshot_is_refused() {
    let dir = tmp("stale");
    let mut w = two_kinds();
    let mpy = w.cfg.months_per_year as u64;
    let mut rec = tape::Recorder::new(&w, 1, 0);
    common::record(&mut rec, &mut w);
    for m in 1..=2 * mpy {
        run(&mut w, 1);
        common::record(&mut rec, &mut w);
        if m == mpy {
            rec.finish(&dir).unwrap();
        }
    }
    save_snapshot(&w, &rec.kept(), dir.join("snap.bin.gz")).unwrap();
    let (w, kept) = tape::load_snapshot(dir.join("snap.bin.gz")).unwrap();
    let e = tape::Recorder::new(&w, 1, 0).resume(&w, kept, &dir).expect_err("refused");
    assert!(e.to_string().contains("no month at the snapshot"), "{e}");
}

/// A tape written at the end of each year (the CSV file then gets only its new rows) leaves the same CSV file as one
/// written once at the end, also when a kind that comes late gives it new columns.
#[test]
fn csv_files_grown_year_by_year_are_those_written_at_once() {
    let (by_year, at_once) = (tmp("csv_by_year"), tmp("csv_at_once"));
    let mut label = String::new();
    for (dir, each_year) in [(&by_year, true), (&at_once, false)] {
        let mut w = two_kinds();
        let mpy = w.cfg.months_per_year as u64;
        let (mut rec, mut written) = (tape::Recorder::new(&w, 0, 0), tape::Written::default());
        common::record(&mut rec, &mut w);
        for m in 1..=3 * mpy {
            run(&mut w, 1);
            common::record(&mut rec, &mut w);
            if each_year && m % mpy == 0 {
                rec.tape().write(dir, &mut written).unwrap();
            }
            if m == mpy {
                label = late(&mut w);
            }
        }
        if each_year {
            rec.tape().write(dir, &mut written).unwrap();
        } else {
            rec.finish(dir).unwrap();
        }
    }
    let (a, b) = (std::fs::read_to_string(by_year.join("series.csv")).unwrap(), std::fs::read_to_string(at_once.join("series.csv")).unwrap());
    assert!(a == b, "series.csv differs");
    assert!(a.lines().next().unwrap().contains(&label), "the late kind has columns");
}

/// The minerals: the series has each mineral (`min0_`, the mineral of the fixing reaction's gas, the only one) in the ground
/// per m² of land and of sea, and where it is in all (dead matter on land and sea, the cover, the bed's stores), which
/// with the ground's stays what the soil gave the world (no cover fixes, and the reaction that breaks it down is off).
#[test]
fn yearly_series_counts_the_minerals() {
    let Recording { w, dir, areas: at } = recording();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').map(|v| v.parse().unwrap()).collect()).collect();
    assert_eq!(rows.len(), 4);
    let col = |n: &str, r: &Vec<f64>| r[head.iter().position(|h| *h == n).unwrap_or_else(|| panic!("no column {n}"))];
    // The soil: 0.357 mol of the mineral (80 g) in a m2 is 28.6 g; the cells under standing liquid have none.
    let first = &rows[0];
    assert!(col("min0_land", first) > 0.015 && col("min0_land", first) <= 0.0286 + 1e-6, "{}", col("min0_land", first));
    assert_eq!(col("min0_sea", first), 0.0);
    assert!(!head.iter().any(|h| h.starts_with("min1_")), "one mineral");
    let mm = w.chem.subs[w.chem.minerals()[0].0].molar_mass;
    let all = |r: &Vec<f64>, a: [f64; 2]| {
        let ground = (col("min0_land", r) * a[0] + col("min0_sea", r) * a[1]) / mm / 1e9;
        ground + ["min0_dead_land", "min0_dead_sea", "min0_cover", "min0_deep"].iter().map(|c| col(c, r)).sum::<f64>()
    };
    for (r, &c) in rows.iter().zip(at) {
        assert!((all(r, c) / all(first, at[0]) - 1.0).abs() < 1e-6, "the mineral in all: {} against {}", all(r, c), all(first, at[0]));
    }
    let last = rows.last().unwrap();
    assert!(col("min0_land", last) > 0.0 && col("min0_dead_land", last) > 0.0 && col("min0_cover", last) > 0.0);
}


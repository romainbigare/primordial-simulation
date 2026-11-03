//! The tape: snapshots (in the middle of the slow cycles, refused without their header, carrying the schedule), the
//! viewer file, the yearly series and the monthly rows, resuming into the same folder, and the null run.

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
    worldgen::new_world(1, SIDE, SIDE)
}

/// `months` months of `w`.
fn run(w: &mut World, months: u32) {
    for _ in 0..months {
        step(w);
    }
}

/// Cover added on every dry land cell for the eaters of `fed`, kg per m²: with it they live through the first years and
/// breed, their numbers flat or growing, not kept by the luck of a few founders.
const FOOD_KG: f64 = 2.0;

/// `w` with two kinds of life that live on: the light-eaters (none of them fixing) with `FOOD_KG` more of their cover
/// on the dry land, and the eaters with the hand-set brains of `ew run`.
fn fed(w: &mut World) {
    let how = worldgen::life::Seeding { fixers: 0.0, ..Default::default() };
    worldgen::life::seed_with(w, 0, &how);
    worldgen::testkit::cover_lands(w, FOOD_KG);
    worldgen::life::seed_with(w, 1, &how);
}

/// The world with two kinds of life: light-eaters and eaters with neutral brains.
fn two_kinds() -> World {
    let mut w = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
    w
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

/// Save and load in the middle of the slow cycles of Flow (refresh, work, creep) with life and a cleared lane about:
/// the tape can read the lanes before the resumed run's first survey, and the resumed run must give the same history
/// as the unbroken one.
#[test]
fn snapshot_round_trip_mid_cycle() {
    let dir = tmp("snap_mid");
    let (mut a, mut whole) = (world(), world());
    for w in [&mut a, &mut whole] {
        worldgen::life::seed(w, 0);
        worldgen::life::seed_with(w, 1, &worldgen::life::Seeding::neutral());
        w.links.cleared[0] = 1.0;
    }
    run(&mut a, 17);
    save_snapshot(&a, &(), dir.join("s.bin.gz")).unwrap();
    let (mut b, ()) = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    assert_eq!(fingerprint(&a), fingerprint(&b));
    assert!(pcore::lanes::eased(&b, 0) >= 1.0, "the lane's ease, from the rebuilt sums");
    run(&mut b, 14);
    run(&mut whole, 31);
    assert_eq!(fingerprint(&b), fingerprint(&whole));
}

/// Parts lie in the list of their cell, each with an id that never changes (laws.md 3.1): two bonded parts keep their
/// ids and their bond through a snapshot and when their structure moves to a neighbour cell, and a part made after them
/// gets an id of its own, so an old id names nothing once its part is gone.
#[test]
fn parts_keep_their_ids_and_bonds_in_their_cell() {
    use pcore::bond;
    let dir = tmp("snap_part");
    let mut a = worldgen::testkit::small_world();
    let i = worldgen::testkit::dry_land(&a);
    let (p, q) = (worldgen::testkit::plate(&mut a, i, 10.0, false), worldgen::testkit::plate(&mut a, i, 10.0, false));
    bond::link(&mut a, p, q, 1e9);
    save_snapshot(&a, &(), dir.join("s.bin.gz")).unwrap();
    let (mut b, ()) = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    let bonded = |w: &pcore::World, c: usize| w.life.cell_at(c).bonds.of(p).any(|l| l.0 == q);
    assert!((b.life.part_cell(p), b.life.part_cell(q)) == (Some(i), Some(i)) && bonded(&b, i), "the parts or their bond changed in the snapshot");
    // Their structure (its root is the least id) moves to a neighbour cell with its bond.
    let (j, root) = (b.mesh.cells().around(i).1[0] as usize, b.life.part(q).expect("a part").root);
    bond::moved(&mut b, (bond::NOBODY, Some(root)), (i, j), 0.0);
    assert!((b.life.part_cell(p), b.life.part_cell(q)) == (Some(j), Some(j)) && bonded(&b, j) && !bonded(&b, i), "the structure did not move whole");
    // Both fall apart in the month, as lighter than the least a part may weigh; a new part has a new id.
    b.cfg.phys.min_kg = 1e3;
    bond::step(&mut b);
    assert!(b.life.part(p).is_none() && b.life.part(q).is_none(), "an old id still names a part");
    let r = worldgen::testkit::plate(&mut b, i, 10.0, false);
    assert!(r != p && r != q && b.life.part_cell(r) == Some(i), "the new part {r} took an old id");
}

/// A roster run resumed from a snapshot taken before some of its kinds are due places them as the unbroken run does:
/// the snapshot carries the schedule, and both runs end with the same world.
#[test]
fn a_resumed_run_places_the_kinds_still_due() {
    use worldgen::life::{Seeding, KINDS, ROSTER};
    use worldgen::schedule::Schedule;
    let dir = tmp("resume_plan");
    // The roster from year 0, its later kinds due at year 1, after the snapshot.
    let mut plan = Schedule::new(Some(0), true, Seeding::default());
    plan.kinds.iter_mut().for_each(|k| k.1 = k.1.min(1));
    let late: Vec<u32> = ROSTER.iter().filter(|&&k| KINDS[k].year > 0).map(|&k| k as u32).collect();
    let years = |w: &mut World, plan: &Schedule, n: u64| {
        for _ in 0..n {
            plan.place(w);
            run(w, w.cfg.months_per_year);
        }
    };
    let (mut a, mut whole) = (world(), world());
    years(&mut whole, &plan, 2);
    years(&mut a, &plan, 1);
    assert!(a.life.brains.iter().flatten().all(|l| !late.contains(&l.root)), "a later kind placed before the snapshot");
    save_snapshot(&a, &plan, dir.join("s.bin.gz")).unwrap();
    let (mut b, plan): (World, Schedule) = tape::load_snapshot(dir.join("s.bin.gz")).unwrap();
    // (Checked as they are placed: a kind placed in so small a world may die out within the year.)
    plan.place(&mut b);
    assert!(late.iter().all(|k| b.life.brains.iter().flatten().any(|l| l.root == *k)), "the resumed run placed the later kinds");
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
    assert!(!dir.join("months.csv").exists(), "months.csv is written only when asked for");

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

/// The viewer file says whether the world is a globe and the latitudes of its edges, and the world's means of a globe are
/// by area (its rows narrow toward the poles): `t_ground` is the area-weighted mean of the cells' ground temperature.
#[test]
fn a_globe_file_names_its_shape_and_its_means_are_by_area() {
    let dir = tmp("globe");
    let (rows, mut w) = (8, worldgen::terrain::new_globe(1, 8));
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
        let mut rec = tape::Recorder::new(&w, 0, 0).with_months_csv();
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

/// The series has a column for each heat-absorbing gas of the air and each reaction's catalysts.
#[test]
fn yearly_series_names_gases_and_catalysts() {
    let Recording { w, dir, .. } = recording();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let head: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
    let gases = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0).count();
    assert!(gases > 0 && head.contains(&"absorber"));
    assert_eq!(head.iter().filter(|h| h.starts_with("air_")).count(), gases);
    assert_eq!(head.iter().filter(|h| h.starts_with("cat_")).count(), w.chem.reactions.len());
}

/// The per-kind columns (`root_<k>_*`) are in the series, deaths and births of the kinds add up to
/// the totals of the year, and the land's summer is not colder than its winter.
#[test]
fn yearly_series_has_the_measures_per_starting_kind() {
    let Recording { w, dir, .. } = recording();
    let csv = std::fs::read_to_string(dir.join("series.csv")).unwrap();
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').map(|v| v.parse().unwrap()).collect()).collect();
    assert_eq!(rows.len(), 4);
    let col = |n: &str, r: &Vec<f64>| r[head.iter().position(|h| *h == n).unwrap_or_else(|| panic!("no column {n}"))];
    let kinds: Vec<usize> = (0..8).filter(|k| head.contains(&format!("root_{k}_members").as_str())).collect();
    assert!(!kinds.is_empty());
    for name in ["births", "life_real", "mass", "copy_mass", "matter", "body", "copy_log", "hidden", "repair", "died_age", "alloc", "child", "brood"] {
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
    assert!(col("t_land_summer", last) >= col("t_land_winter", last));
    // A catalyst gene on in a kind keeps its columns, with the share of members that carry it.
    assert!(head.iter().any(|h| h.starts_with("root_1_cat_") && h.ends_with("_on")));
    // The kind of cover keeps its columns: its slots, mass and genes.
    assert!(["slots", "cover_kg", "cover_area", "cover_light", "cover_frost"].iter().all(|c| head.contains(&format!("root_0_{c}").as_str())));
    // The kind's mass in kg is its members times its mean member's mass.
    for k in &kinds {
        let (m, mean, kg) = (col(&format!("root_{k}_members"), last), col(&format!("root_{k}_mass"), last), col(&format!("root_{k}_kg"), last));
        assert!(m == 0.0 && kg == 0.0 || (kg - m * mean).abs() <= 1e-9 * kg, "root_{k}_kg {kg} members {m} mean {mean}");
    }
    // The body matter's share of a kind's mass is between 0 and 1, and the mean body of a member (matter and the liquid it keeps
    // with it) is no more than the mean member's mass.
    for k in &kinds {
        let (m, mean, matter, body) = (col(&format!("root_{k}_members"), last), col(&format!("root_{k}_mass"), last), col(&format!("root_{k}_matter"), last), col(&format!("root_{k}_body"), last));
        assert!(m == 0.0 || (matter > 0.0 && matter < 1.0 && body > 0.0 && body <= mean * (1.0 + 1e-9)), "root_{k}: matter {matter}, body {body}, mass {mean}");
    }
    // The genes of reproduction: the share of building that goes to offspring, a child's body as log10 of its share of the
    // copy mass, and the brood as a share of the members' mass.
    for k in &kinds {
        let m = col(&format!("root_{k}_members"), last);
        let (alloc, child, brood) = (col(&format!("root_{k}_alloc"), last), col(&format!("root_{k}_child"), last), col(&format!("root_{k}_brood"), last));
        assert!(m == 0.0 || ((0.0..=1.0).contains(&alloc) && (-6.0..=-0.3).contains(&child) && (0.0..1.0).contains(&brood)), "root_{k}: alloc {alloc}, child {child}, brood {brood}");
    }
    // The share of a kind's births in the year that had two parents: between 0 and 1, none where there were no births.
    for k in &kinds {
        let (births, mixed) = (col(&format!("root_{k}_births"), last), col(&format!("root_{k}_mixed"), last));
        assert!(if births > 0.0 { (0.0..=1.0).contains(&mixed) } else { mixed.is_nan() }, "root_{k}: births {births}, mixed {mixed}");
    }
    // months.csv carries the births of each kind by month; the twelve months before the last yearly row add up to its births.
    let months = std::fs::read_to_string(dir.join("months.csv")).unwrap();
    let mut ml = months.lines();
    let mh: Vec<&str> = ml.next().unwrap().split(',').collect();
    let mrows: Vec<Vec<&str>> = ml.map(|l| l.split(',').collect()).collect();
    assert!(mrows.iter().all(|r| r.len() == mh.len()));
    for k in &kinds {
        let at = mh.iter().position(|h| *h == format!("root_{k}_births")).unwrap_or_else(|| panic!("months.csv has no births of kind {k}"));
        let sum: f64 = mrows[mrows.len() - 12..].iter().map(|r| r[at].parse::<f64>().unwrap()).sum();
        let year = col(&format!("root_{k}_births"), last);
        assert!((sum - year).abs() <= 1e-9 * year.max(1.0), "root_{k}: months {sum} year {year}");
    }
    // and so do the deaths of each kind by cause.
    for k in &kinds {
        let sum = |c: &str| {
            let at = mh.iter().position(|h| *h == format!("root_{k}_died_{c}")).unwrap_or_else(|| panic!("months.csv has no deaths of kind {k} by {c}"));
            mrows[mrows.len() - 12..].iter().map(|r| r[at].parse::<f64>().unwrap()).sum::<f64>()
        };
        for c in ["lack", "phase", "spent", "taken", "broken", "age"] {
            let year = col(&format!("root_{k}_died_{c}"), last);
            assert!((sum(c) - year).abs() <= 1e-9 * year.max(1.0), "root_{k} died of {c}: months {} year {year}", sum(c));
        }
    }
    // The heaviest member has a kind.
    assert!(kinds.contains(&(col("mass_max_root", last) as usize)));
    // The run's numbers are written beside the series.
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("physics.json")).unwrap()).unwrap();
    assert_eq!(doc["physics"]["alike"].as_f64().map(|x| x as f32), Some(w.cfg.phys.alike));
    assert_eq!(doc["config"]["days_per_month"].as_u64(), Some(w.cfg.days_per_month as u64));
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

/// A run with life written for 4 years and resumed from its year 1 snapshot into the same folder keeps the years 0..1
/// it had and replays 2 to 4 as the unbroken run wrote them: series rows, months, frames and events, with no gap and no
/// repeat (the snapshot carries what the recorder keeps across years: where bodies were, what they took, worn edges).
/// A kind seeded after the snapshot has no columns until then, as in the unbroken run.
#[test]
fn resume_into_the_same_folder_continues_the_tape() {
    let dir = tmp("resume");
    let mut w = two_kinds();
    let (mpy, dpy) = (w.cfg.months_per_year as u64, w.days_per_year());
    let mut rec = tape::Recorder::new(&w, 1, 0).with_months_csv();
    common::record(&mut rec, &mut w);
    // The third kind comes at the end of year 2, after the snapshot of year 1, in both runs.
    let late = |w: &mut World| worldgen::life::seed_with(w, 2, &worldgen::life::Seeding::neutral());
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
    let whole_months = std::fs::read_to_string(dir.join("months.csv")).unwrap();

    let (mut w, kept) = tape::load_snapshot(dir.join("snap.bin.gz")).unwrap();
    let mut rec = tape::Recorder::new(&w, 1, 0).with_months_csv();
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
    assert!(series[0].contains("eat_animals") && series[0].contains("worn_new"));
    for (row, (resumed, unbroken)) in series.iter().zip(&whole.2).enumerate() {
        let diff: Vec<&str> = series[0].split(',').zip(resumed.split(',').zip(unbroken.split(','))).filter(|(_, (a, b))| a != b).map(|(h, _)| h).collect();
        assert_eq!(resumed, unbroken, "series row {row}: {diff:?}");
    }
    assert_eq!(std::fs::read_to_string(dir.join("months.csv")).unwrap(), whole_months);
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
    let mut rec = tape::Recorder::new(&w, 1, 0).with_months_csv();
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

/// A tape written at the end of each year (the CSV files then get only their new rows) leaves the same CSV files as one
/// written once at the end, also when a kind that comes late gives both files new columns.
#[test]
fn csv_files_grown_year_by_year_are_those_written_at_once() {
    let (by_year, at_once) = (tmp("csv_by_year"), tmp("csv_at_once"));
    for (dir, each_year) in [(&by_year, true), (&at_once, false)] {
        let mut w = two_kinds();
        let mpy = w.cfg.months_per_year as u64;
        let (mut rec, mut written) = (tape::Recorder::new(&w, 0, 0).with_months_csv(), tape::Written::default());
        common::record(&mut rec, &mut w);
        for m in 1..=3 * mpy {
            run(&mut w, 1);
            common::record(&mut rec, &mut w);
            if each_year && m % mpy == 0 {
                rec.tape().write(dir, &mut written).unwrap();
            }
            if m == mpy {
                worldgen::life::seed_with(&mut w, 2, &worldgen::life::Seeding::neutral());
            }
        }
        if each_year {
            rec.tape().write(dir, &mut written).unwrap();
        } else {
            rec.finish(dir).unwrap();
        }
    }
    for file in ["series.csv", "months.csv"] {
        let (a, b) = (std::fs::read_to_string(by_year.join(file)).unwrap(), std::fs::read_to_string(at_once.join(file)).unwrap());
        assert!(a == b, "{file} differs");
    }
    let series = std::fs::read_to_string(by_year.join("series.csv")).unwrap();
    assert!(series.lines().next().unwrap().contains("root_2_"), "the late kind has columns");
}

/// A null run: with life about and copies made, nobody learns (the bodies' own adjustments stay
/// zero) and every born row is a mean of the starting ones (inside their range, as a node's rows
/// are the means of its members'); in the real run of the same world they are neither. The eaters
/// (`fed`) breed within its months, so that many copies carry rows.
#[test]
fn null_run_freezes_the_brain_rows() {
    let go = |null: bool| {
        let mut w = world();
        w.cfg.null = null;
        fed(&mut w);
        let start: Vec<&Vec<f32>> = w.life.genomes.iter().filter(|g| !g.rows.is_empty()).map(|g| &g.rows).collect();
        let (lo, hi): (Vec<f32>, Vec<f32>) = (0..start[0].len()).map(|k| start.iter().fold((f32::MAX, f32::MIN), |m, r| (m.0.min(r[k]), m.1.max(r[k])))).unzip();
        run(&mut w, 72);
        // (Rows of the starting shape only: genes still mutate in a null run, and a lineage whose size genes moved a whole unit
        // has rows of another length, carried over from the starting ones.)
        let out = |r: &Vec<f32>| r.len() == lo.len() && r.iter().enumerate().any(|(k, &x)| x < lo[k] - 1e-5 || x > hi[k] + 1e-5);
        let new = w.life.genomes.iter().filter(|g| !g.rows.is_empty() && out(&g.rows)).count();
        let moved = w.life.alive().filter(|b| w.life.rows_of(b) != w.life.born(b)).count();
        (new, moved, w.counts.updates)
    };
    let (real, null) = (go(false), go(true));
    assert!(null.2 > 100 && real.2 > 100, "life ran: {real:?} {null:?}");
    assert!(real.0 > 0 && real.1 > 0, "the real run learns and mutates rows: {real:?}");
    assert_eq!((null.0, null.1), (0, 0), "the null run does not: {null:?}");
}

/// The minerals: the series has each mineral (`min0_`, the mineral of the fixing reaction's gas, the only one) in the ground
/// per m² of land and of sea, and where it is in all (dead matter on land and sea, the bodies, the cover, the parts, the bed's
/// stores), which with the ground's stays what the soil gave the world (no body fixes, and the reaction that breaks it
/// down is off).
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
        ground + ["min0_dead_land", "min0_dead_sea", "min0_bodies", "min0_cover", "min0_parts", "min0_deep"].iter().map(|c| col(c, r)).sum::<f64>()
    };
    for (r, &c) in rows.iter().zip(at) {
        assert!((all(r, c) / all(first, at[0]) - 1.0).abs() < 1e-6, "the mineral in all: {} against {}", all(r, c), all(first, at[0]));
    }
    let last = rows.last().unwrap();
    assert!(col("min0_land", last) > 0.0 && col("min0_dead_land", last) > 0.0 && col("min0_bodies", last) > 0.0);
}

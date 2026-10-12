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

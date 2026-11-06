//! `ew view`: a window of a snapshot as a PNG of the size asked for, and a refusal of a layer that does not exist.

use std::process::{Command, Output};

fn ew(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ew")).args(args).output().expect("start ew")
}

#[test]
fn a_window_of_a_snapshot_is_drawn_at_the_pixels_asked_for() {
    let dir = std::env::temp_dir().join(format!("ew_cli_view_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (dir, png) = (dir.to_str().unwrap(), |d: &str| format!("{d}/window.png"));
    assert!(ew(&["run", "--size", "12", "--years", "1", "--frames-every", "1000", "--out", dir]).status.success());
    let (snapshot, file) = (format!("{dir}/snap_00001.bin.gz"), png(dir));
    let view = |layer: &str| ew(&["view", "--from", &snapshot, "--window", "2.5,3,4,4", "--px", "40", "--edges", "--layer", layer, "--out", &file]);
    assert!(view("depth").status.success());
    let info = png::Decoder::new(std::fs::File::open(&file).unwrap()).read_info().unwrap();
    assert_eq!((info.info().width, info.info().height), (40, 40));
    assert!(!view("nothing").status.success());
}

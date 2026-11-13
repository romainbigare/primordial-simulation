//! The writer thread of `ew run`: the files of a run that was stopped at a snapshot and resumed into the same folder
//! are those of the run that was not stopped.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `ew <command> <args> --out dir` and say if it ended well.
fn ew(command: &str, args: &[&str], dir: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_ew")).arg(command).args(args).arg("--out").arg(dir).output().expect("start ew");
    assert!(out.status.success(), "ew {command}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A new folder of this test process for `name`.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ew_cli_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The text of `series.csv` without the columns of the clocks (`ms_*`), which differ from run to run.
fn series_without_clocks(dir: &Path) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(dir.join("series.csv")).expect("series.csv");
    let rows: Vec<Vec<&str>> = text.lines().map(|l| l.split(',').collect()).collect();
    let keep: Vec<usize> = (0..rows[0].len()).filter(|&i| !rows[0][i].starts_with("ms_")).collect();
    rows.iter().map(|r| keep.iter().map(|&i| r[i].to_string()).collect()).collect()
}

#[test]
fn a_run_resumed_into_its_folder_leaves_the_files_of_the_unbroken_run() {
    let (whole, broken) = (folder("whole"), folder("broken"));
    let common = ["--size", "12", "--seed", "1", "--frames-every", "1000"];
    ew("run", &[&common[..], &["--years", "12"]].concat(), &whole);
    ew("run", &[&common[..], &["--years", "10"]].concat(), &broken);
    ew("resume", &["--from", broken.join("snap_00010.bin.gz").to_str().unwrap(), "--years", "2", "--frames-every", "1000"], &broken);
    let snap = "snap_00012.bin.gz";
    assert!(std::fs::read(whole.join(snap)).unwrap() == std::fs::read(broken.join(snap)).unwrap(), "the snapshot differs");
    assert_eq!(series_without_clocks(&whole), series_without_clocks(&broken));
    let parts = std::fs::read_dir(&broken).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".part")).count();
    assert_eq!(parts, 0, "files left half written");
}

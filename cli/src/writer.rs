//! The writer: one thread that compresses and writes the tape, and a snapshot when one is due, while the world goes on.

use pcore::World;
use std::io;
use std::path::Path;
use std::thread::JoinHandle;
use tape::{Kept, TapeFiles, Written};
use worldgen::schedule::Schedule;

/// What the writer thread gives back when a job ends: what the CSV files hold, and the buffer of the snapshot's bytes
/// (kept for the next snapshot: a new buffer that grows to its size costs more than filling it).
struct Idle {
    written: Written,
    bytes: Vec<u8>,
}

/// The thread that writes, at most one job at a time. The files are the same as if the main thread had written them.
pub struct Writer {
    idle: Option<Idle>,
    job: Option<JoinHandle<io::Result<Idle>>>,
}

impl Writer {
    pub fn new() -> Self {
        Writer { idle: Some(Idle { written: Written::default(), bytes: vec![] }), job: None }
    }

    /// Wait for the job in flight. A write that failed stops the run.
    pub fn wait(&mut self) {
        if let Some(job) = self.job.take() {
            self.idle = Some(job.join().expect("the writer thread ended").expect("write the snapshot and the tape"));
        }
    }

    /// Stop the run now if the job in flight has ended with an error; never wait for a job that is still going.
    pub fn check(&mut self) {
        if self.job.as_ref().is_some_and(|job| job.is_finished()) {
            self.wait();
        }
    }

    /// Take a copy of the tape and, if a `snapshot` file is given, make the bytes of a snapshot of `w` and the shell's
    /// state, and start writing them into the folder `dir` and to that file, once the last job has ended. The tape goes
    /// first and the snapshot last, so that a tape is never behind a snapshot: a run that stops between the two files
    /// leaves a snapshot that resumes.
    pub fn start(&mut self, w: &World, shell: &(&Schedule, Kept), snapshot: Option<String>, tape: TapeFiles, dir: String) {
        self.wait();
        let Idle { mut written, mut bytes } = self.idle.take().expect("the last job gave back its buffers");
        if snapshot.is_some() {
            tape::snapshot_bytes(w, shell, &mut bytes).expect("make the snapshot");
        }
        let job = move || {
            tape.write(Path::new(&dir), &mut written)?;
            if let Some(file) = snapshot {
                tape::write_snapshot(&bytes, &file)?;
            }
            Ok(Idle { written, bytes })
        };
        self.job = Some(std::thread::Builder::new().name("writer".into()).spawn(job).expect("start the writer thread"));
    }
}

/// A run that stops while a job is going (a panic of the main thread) lets it end first, so that no file is cut.
impl Drop for Writer {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            let _ = job.join();
        }
    }
}

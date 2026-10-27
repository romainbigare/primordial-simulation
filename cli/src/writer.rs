//! The writer: one thread that compresses and writes a decade's snapshot and tape while the world goes on.

use pcore::World;
use serde::Serialize;
use std::io;
use std::path::Path;
use std::thread::JoinHandle;
use tape::{Tape, Written};

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

    /// Make the bytes of a snapshot of `w` and `plan` and a copy of `tape`, and start writing them to the file `snapshot`
    /// and into the folder `dir`, once the last job has ended.
    pub fn start<P: Serialize>(&mut self, w: &World, plan: &P, snapshot: String, tape: Tape, dir: String) {
        self.wait();
        let Idle { mut written, mut bytes } = self.idle.take().expect("the last job gave back its buffers");
        tape::snapshot_bytes(w, plan, &mut bytes).expect("make the snapshot");
        let job = move || {
            tape::write_snapshot(&bytes, &snapshot)?;
            tape.write(Path::new(&dir), &mut written)?;
            Ok(Idle { written, bytes })
        };
        self.job = Some(std::thread::Builder::new().name("writer".into()).spawn(job).expect("start the writer thread"));
    }
}

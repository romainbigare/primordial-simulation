# Idea 1: cheaper saves (lane F)

Steps: PR1, F1, F2, F3 (`plan.md` section 4). Decisions: L8, L13.

## 1. What the code does today

`cli/src/main.rs`, `drive`: after each simulated year,
- every `SNAP_EVERY` (10) years and at the run's end: `tape::save_snapshot(&w, &(&plan, rec.kept()), snap_NNNNN.bin.gz)`;
- every 10 years (not at the end): `rec.finish(&out)`, "the tape so far, so a stopped run keeps its series and
  frames"; at the run's end `rec.finish` once more.

Only `rec.finish` is timed into the tape's column (`clock.spent[TAPE]`). **`save_snapshot` is timed nowhere**: the
"per year" a run prints, and the last report's 1,000-year times, leave the snapshots out (globe 18: 313 s timed, 319 s
wall; globe 28: 773 s against 791 s).

`tape/src/snapshot.rs`, `save_snapshot`: a `BufWriter` (1 MB) over a `GzEncoder` at `Compression::fast()` over the file;
the tag and version, then `bincode::serialize_into` of the `World` and of the shell's state. A save of `benchZ`'s
island22 is about 6.3 MB on disk. In the last round's profile of one year (`docs/speed-x10/measures.md`, "Z: the final
checks" 4) the write of the last snapshot was **608 M instructions** (`miniz_oxide::deflate` 601 M), about 10% of the
year's 6,104 M, so roughly 0.15 to 0.2 s a save, 15 to 20 ms a year at one save every 10 years. The load (`World`
deserialize) was 329 M.

`tape/src/rec/mod.rs`, `Recorder::finish`: writes the whole tape again each time:
- `write_ewf` (`rec/ewf.rs`): a JSON head of **every** series value of every year, every event and every month row,
  then the frames' bytes, through a `GzEncoder` at `Compression::default()` (level 6);
- `series.csv`: built in one `String` by `format!` per value;
- `months.csv` (`months::csv`) and `physics.json`.
Each of these grows with the run, so the run's total grows with the square of its length. The tape's column was 37 /
38 / 47 ms a year in the 1,000-year runs against about 6 ms on the benches (the recorder's monthly work, G8): the
difference, about 30 to 40 ms a year, is this rewrite.

**What is promised** (`docs/tape.md`, handover 7.3 and 8.3): a snapshot every 10 years and at the end, "and before every
god action" (M6); "`ew resume --from` goes on from it, and a resumed tape equals the unbroken one" (`Recorder::resume`
reads `world.ewf.gz` up to the world's day, `read_ewf`). The comment in `drive` promises that a stopped run keeps its
series and frames up to its last snapshot.

## 2. What changes

**F1 (level 1).** Time `save_snapshot` into `clock.spent[TAPE]`, as `rec.finish` is. The world does not change; the
tape's `ms_tape` column now holds the saves. The last report's long-run numbers are set beside the new ones with this
said (L8).

**F2 (level 1).** One writer thread in `cli` (a `std::thread` handle kept in `drive`), at most one job in flight:
- the snapshot: the main thread turns the world and the shell's state into bytes (`bincode::serialize` into a `Vec`,
  the tag and version first); the thread compresses and writes them to `snap_NNNNN.bin.gz.part`, then renames it to
  its name (a crash never leaves a half file under the real name);
- the tape: the main thread copies what `finish` reads (`Recorder` gets a method that returns an owned value with the
  series, months, events and frames, or `finish` takes the copy); the thread builds the JSON and the CSV and writes
  them, the same way (`.part`, then rename);
- before a new job and at the run's end, the main thread waits for the last one (`join`), and a write error stops the
  run as today (`expect`).
The files are the same bytes as today (the same coders, the same levels, the same order). `tape/tests/tape.rs`
(snapshots, resuming, the viewer's file) pass unchanged. `pcore` and the wasm build do not see it (the thread is in
`cli` only).

**F3 (level 1 for the world; the tape's files hold the same rows).** Only if PR1 shows the writer thread still busy
for more than 20% of a long run (it then competes with the body batches for the 4 cores):
- the viewer's file at `Compression::fast()` (as the snapshot; the viewer reads any gzip level);
- `series.csv` and `months.csv` appended with the rows since the last write (opened once; a resumed run truncates them
  to the world's day, as `read_ewf` keeps rows up to it), not rebuilt;
- if the JSON head is still the cost: the viewer's file written every 100 years and at the end, the CSVs every 10. This
  breaks "a resumed tape equals the unbroken one" for a resume from a snapshot between two viewer writes unless
  `read_ewf` rebuilds the missing rows from the CSVs, so it comes only with that, and the owner told.

**Not chosen:** saving less often (L13: the cadence is the spec's; the alternative if F2 and F3 fail); another coder
(zstd, lz4): PR1 measures them, and one is taken only if it halves the thread's work at the same size, since a new
crate is a build cost and the viewer must read the file (the viewer's file stays gzip).

## 3. Risks

- A copy of the world's bytes in memory: about 20 to 40 MB for the size-28 globe, one at a time. Fine on 15 GB.
- The thread competes with the pool for the 4 cores. A compression of 0.2 s every 10 years of a 0.3 to 0.8 s year is
  small; PR1 says how much the tape's rewrite adds.
- A run killed while a job is in flight loses that save; the `.part` and the rename keep the last good one.
- The tape's files must stay the same bytes (F2) or the same rows (F3): the resume test and a byte compare of a
  50-year run's files (F2) or of their parsed rows (F3) check it.

## 4. What PR1 must show first

On a branch, timers (no world change) around each part, in a 300-year island run (`ew run --size 22 --seed 1 --years
300`, default frames and snapshots) and a 100-year globe run, written to a CSV per save:
1. the snapshot: `bincode` into memory, the compression (`fast`; and levels 0 and 1 of zstd and lz4 on the same bytes,
   for scale), the write; the bytes before and after;
2. `Recorder::finish` by file: the JSON head, its compression, the frames' bytes, `series.csv`, `months.csv`;
3. how each grows with the year (a line fit), carried to 1,000 years; the share of a long run's wall time;
4. the gap between the run's timed year and its wall time.

The gate: F2 goes ahead if the saves and the tape on the main thread are over 3% of a long run's wall time (expected
about 10%). F3 only as section 2 says.

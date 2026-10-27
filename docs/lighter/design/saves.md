# Idea 1: cheaper saves (lane F)

Steps: PR1, F1, F3, then F2 only if needed (`plan.md` section 4). Decisions: L8, L13, L21.

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
year's 6,104 M (its wall time was not measured). The wall times bound it: the 1,000-year globe 18 ran 313 s timed
and 319 s wall, the size-28 globe 773 s and 791 s (`measures.md` Z 3), so all 100 snapshots with the start and the end
cost **at most 6 and 18 ms a year, about 2%**. The load (`World` deserialize) was 329 M.

`tape/src/rec/mod.rs`, `Recorder::finish`: writes the whole tape again each time:
- `write_ewf` (`rec/ewf.rs`): a JSON head of **every** series value of every year, every event and every month row,
  then the frames' bytes, through a `GzEncoder` at `Compression::default()` (level 6);
- `series.csv`: built in one `String` by `format!` per value;
- `months.csv` (`months::csv`) and `physics.json`.
Each of these grows with the run, so the run's total grows with the square of its length. The tape's column was 37 /
38 / 47 ms a year in the 1,000-year runs against about 6 ms on the benches (the recorder's monthly work, G8): the
difference, about 30 to 40 ms a year, is this rewrite (the bench runs of `benchZ` with default flags show 11 / 15 / 19
ms a year over 200 to 300 years: it grows with the run). **This is the cost, about 8 to 10% of a long run's year.**
`render::save` (the pictures every `--every` years) and `check::broken` are untimed too.

**What is promised** (`docs/tape.md`, handover 7.3 and 8.3): a snapshot every 10 years and at the end, "and before every
god action" (M6); "`ew resume --from` goes on from it, and a resumed tape equals the unbroken one" (`Recorder::resume`
reads `world.ewf.gz` up to the world's day, `read_ewf`). The comment in `drive` promises that a stopped run keeps its
series and frames up to its last snapshot.

## 2. What changes

**F1 (level 1).** Time `save_snapshot` and `render::save` into `clock.spent[TAPE]`, as `rec.finish` is. The world
does not change. The year's row is recorded before the save (`cli/src/main.rs`: `record` in the month loop, the save
after it), so the save lands in the **next** year's `ms_tape`; `docs/tape.md` says so. The last round's `measures.md`
Z 3 sentence ("holds the snapshot") is corrected in the same commit.

**F3 (level 1 for the world; the tape's files hold the same rows).** The rewrite stops growing with the run:
- `series.csv` and `months.csv` appended with the rows since the last write, not rebuilt (a resumed run cuts them to
  the world's day first, as `read_ewf` keeps rows up to it);
- the viewer's file at `Compression::fast()` (as the snapshot; any gzip reader reads it);
- what PR1 names in the JSON head (it holds every series value and month row again): if it is still the cost, the
  viewer's file written every 100 years and at the end, the CSVs every 10, **only with** `read_ewf` rebuilding the
  missing rows from the CSVs, so a resumed tape still equals the unbroken one; else not.

**F2 (level 1), only if the saves and the tape are still over 2% of a long run's wall time after F3.** One writer
thread in `cli` (a handle kept in `drive`), at most one job in flight: the main thread turns the world and the shell's
state into bytes (`bincode` into a `Vec`, the tag and version first) and copies what `finish` reads; the thread
compresses and writes to `<name>.part`, then renames (a crash never leaves a half file under the real name); before a
new job and at the run's end the main thread waits for the last one, and a write error stops the run as today. The
files are the same bytes. A writer thread does not remove work: `ew` already runs 4 threads on 4 cores, so it only
uses the pool's idle time.

**Not chosen:** saving less often (L13: the cadence is the spec's; the alternative if F3 and F2 fail); another coder
(zstd, lz4) for the snapshot: at most 2% to win, a new crate to build.

## 3. Risks

- Appending: a resumed run must cut the CSVs to its day before it appends, or rows repeat; a stopped run keeps the rows
  up to its last write (as today). The resume test and a compare of the parsed rows of a 50-year run, unbroken against
  stopped and resumed, check it.
- The viewer's file at gzip `fast` is larger on disk (PR1 says how much).
- F2 (if built): a copy of the world's bytes in memory, 20 to 40 MB for the size-28 globe; a run killed while a job is
  in flight loses that save (the `.part` keeps the last good one).

## 4. What PR1 must show first

On a branch, timers (no world change) around each part, in a 300-year island run (`ew run --size 22 --seed 1 --years
300`, default frames and snapshots) and a 100-year globe run, written to a CSV per save:
1. the snapshot: `bincode` into memory, the compression, the write; `render::save`;
2. `Recorder::finish` by file: the JSON head, its compression, the frames' bytes, `series.csv`, `months.csv`;
3. how each grows with the year (a line fit), carried to 1,000 years; the share of a long run's wall time;
4. the gap between the run's timed year and its wall time.

The gates: F3 goes ahead (expected: the tape about 8 to 10% of a long run's year). F2 only if, after F3, the saves and
the tape are still over 2% of a long run's wall time.

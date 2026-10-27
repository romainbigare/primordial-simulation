# Lighter world: measures

One section per step, in the order of the work. Wall times are on 4 threads, one run at a time under the timing lock,
on a machine of 4 cores that other builds share. Two runs of the same build differ by about 1 to 2 s in 65 s, so a
difference under 3% of a run is read with its noise in mind. `$L` is the scratch folder of the round.

## PR1: what a save costs

**How it was measured.** A scratch build (not kept) with timers around each part, written to stderr: the snapshot
(the world and the shell's state to bytes, the compression, the write), `Recorder::finish` file by file, the pictures,
the yearly check, and the run's wall time against its own timed total. Runs: `ew run --size 22 --seed 1 --years 300`
(island, the default flags: a snapshot, the tape so far and a picture every 10 years) and `ew run --globe --size 18
--seed 1 --years 100`. The base build is `$L/ew-base` (`9656b89`). One real run of the kept code (F1) times the whole
snapshot in one piece, as the code writes it (bincode into a 1 MB buffer into gzip level 1 into the file).

**The snapshot (island, 30 saves).** The world is 1.7 MB of bincode at year 10, 5.3 MB at year 50, and 8.3 to 9.0 MB
from year 100; the file is 0.8, 3.8 and 6.4 to 6.8 MB. The cost grows until about year 60 and then stays flat.

| Part | Year 10 | Year 50 | Years 100 to 300 | 300 years in all |
|---|---|---|---|---|
| the real save, in one piece (F1's clock) | 6.6 ms | 25 ms | 39 to 48 ms | **1.13 s** |
| bincode to a `Vec` (probe, apart) | 8 ms | 24 ms | 31 to 46 ms | 0.81 s |
| gzip level 1, the code's level (probe, first pass on a new buffer) | 13 ms | 35 ms | 53 to 87 ms | 1.62 s |
| gzip level 1 again, warm buffer | 6 ms | 20 ms | 33 to 38 ms | |
| gzip level 3 / level 6 (default) / none | 26 / 53 / 12 ms | 106 / 145 / 30 ms | 180 to 207 / 233 to 273 / 32 to 88 ms | |
| the write of 6 MB | 0.3 ms | 8 ms | 3 to 25 ms | 0.51 s |

The parts measured apart (bincode, then gzip, then write) add up to more than the real save (about 120 ms against
42 ms late in the run): the probe copies the world into a `Vec` that grows, and its first pass over a new buffer pays
for the page faults (on a warm buffer the world to bytes takes 3.5 to 4.5 ms, see "F2: the gate"). The real save
streams the bytes through a small buffer. **A save is about 42 ms late in a 300-year island run,
4 ms a year, 1.7% of the run.** Gzip at level 6 would cost 7 times the level the code uses for 1.5% to 3% less size, so the
level is right. The write is a small part. Another coder (zstd, lz4) was not measured: no crate for it is in this
build, and the architect's order (F1, F3, F2) does not need it.

**`Recorder::finish` (island).** It writes the whole tape again at every decade.

| Part | Year 10 | Year 100 | Year 200 | Year 300 | Growth of a write |
|---|---|---|---|---|---|
| `world.ewf.gz` in all | 5 ms | 34 ms | 73 ms | 114 ms | +0.39 ms for each year of the run |
| of it: gzip level 6 of the JSON head | 1 ms | 24 ms | 57 ms | 91 ms | |
| of it: the head built as a JSON value, then bytes | 1 ms | 4 ms | 8 ms | 13 ms | |
| of it: gzip of the frames | 2 ms | 4 ms | 6 ms | 8 ms | |
| `series.csv` built (`format!` of each value) | 0.2 ms | 2 ms | 4.5 ms | 6 ms | +0.021 ms a year of the run |
| `months.csv` built | 0.1 ms | 9 ms | 5 ms | 8 ms | +0.027 ms a year of the run (noisy) |
| the three files written to disk, `physics.json` | 0.1 ms | 1.3 ms | 1.4 ms | 1.6 ms | |

The JSON head is 1.7 MB at year 230 and 2.2 MB at year 300: every value of the 231 series columns (one number a year),
and 12 rows a year of `months.csv`. The frames are 0.5 MB. The file is 0.97 MB. **The head's compression is three
quarters of the tape's writing, and it grows with the length of the run:** the total of all `finish` calls in 300 years
is 2.0 s (1.78 s the file, 0.12 s and 0.14 s the two CSV builds). The tape's column is 11.8 ms a year over the run
(17 ms in years 200 to 300) and holds the months recorded by the recorder (about 6 ms) besides these writes.

**The globe (size 18, 100 years).** The snapshot is 6.9 MB raw, 5.0 MB on disk, and costs less in bincode (3 ms
against 40 on the island: few bodies, large arrays) and about the same in gzip (30 ms). The tape's file is 35 ms at year
100 (head 0.68 MB, frames 0.42 MB). The saves and the files are 2% of this run.

**To 1,000 years (island).** A save stays near 45 ms after year 100 (the world holds its size while life grows slowly;
the last report's years 900 to 1000 hold more bodies, so a little more): 100 saves are about 4.5 s. The writes of the
tape grow in a line, so the sum of all writes is the line's slope times the sum of the years of the writes: for the file
0.391 ms x 10 x 5,050 = 19.7 s, for the two CSV builds 1.1 s and 1.4 s, for the disk 0.3 s. **About 22 s in all.** Set
against the last report's 334 s for 1,000 years of the island: the saves 1.3%, the tape's writes 6.6%, together 8%
(and the last report's tape column, 37 ms a year, agrees: 22 ms a year of writes and 6 to 11 of monthly work).

**The gap between the timed total and the wall time.** Base build, 300 years: wall time 66.6 s, own total 65.0 s,
a gap of 1.6 s. The start (making the world) is 0.2 s, the saves are 1.1 s, the pictures (0.6 ms each) and the yearly
check (0.3 to 1.3 ms) are 0.05 s; the rest is printing and the placing of what the plan says. The saves were the gap.

**Verdict.** The saves' cost is the sum of two parts that PR1 separates: the snapshot (flat, about 1.3% of a 1,000-year
run by this estimate; the 1,000-year run of F3 measured 6.6 s, 2.0%, as saves grow in the last centuries) and the tape's
files (a line in the run's length, 6.6%; measured 24.9 s, 7.5% of 334 s). The plan counted the snapshot as the larger.
It is the smaller. The order F1, F3, F2 is right. A writer thread (F2) could take away the compression and the write
of the snapshot (about 90% of a save) and the tape's files; the gate for F2 comes after F3.

## F1: the snapshot is timed with the tape

`cli/src/main.rs`: the snapshot's time is added to `clock.spent[TAPE]`, as the files of the tape are. The world does
not change. Check: the same `hash` for 300 years (`50d3eca3d09d9cc5`), the same `snap_00300.bin.gz` byte for byte as
the base build.

| 300 years, island22, seed 1 | base `ew-base` | F1 |
|---|---|---|
| the tape's column, mean of the 300 years | 11.75 ms | 15.46 ms |
| the tape's column, years 201 to 300 | 17.2 ms | 21.7 ms |
| the run's own total and its wall time | 65.0 s and 66.6 s | 64.2 s and 64.6 s; 65.3 s and 65.7 s (a second run) |

The column grows by the 3.7 ms a year of the snapshot (the PR1 number 1.1 s over 300 years). The own total of F1 is
within 0.4 s of the wall time, the start (0.2 s) included. The year's own row is written before its snapshot, so a
snapshot lands in the next row (`docs/tape.md`). The pictures and the yearly check are not timed (0.05 s in 300 years).
The long-run times of `docs/reports/speed-x10.md` leave the snapshots out; from here a run's "per year" holds them
(1.7% more on the island).

## F3: the tape's files written cheaper

**What it does** (`tape/src/rec/`). The viewer's file is compressed at gzip level 1 (it was level 6); its header is
written straight from the recorder's data (a struct, `serde_json::to_vec`), not through a JSON value tree; `series.csv`
and `months.csv` get only the rows added since the last write (`csv.rs`), and are written whole only when their header
line changed or at a resumed run's first write. `Recorder::finish` takes `&mut self` for the bookkeeping. The files
hold the same rows. New test `csv_files_grown_year_by_year_are_those_written_at_once` (a kind that comes late changes
the header of both files between two writes); the resume test passes unchanged.

**Same tape, same world** (1,000 years, island22, seed 1, the build before F3 against F3): the same hash
(`1d1e6532df96b27d`), the same `snap_01000.bin.gz` byte for byte, the same `months.csv` byte for byte, the same
`series.csv` in every column but the clocks (`ms_*`), and the same frames, events, months and series in the viewer's
file once read (the file is 8% larger: 3.49 MB against 3.23 MB at year 1,000).

**What the writes cost** (the timers of the run itself; the machine was loaded by other builds, 4.6 to 8.3 on 4 cores,
so the wall times of the two runs cannot be compared, but the parts of one run can):

| 1,000-year island run | before F3 | F3 |
|---|---|---|
| all `finish` calls (99 writes) | 24.9 s | **5.6 s** |
| growth of a write, ms for each year of the run | 0.52 | 0.116 |
| one write at year 500 | 255 ms | 41 ms |
| all snapshots (100 saves, same code in both) | 6.6 s | 7.3 s (noise) |
| CPU time of the process (user + sys) | 713 s | **660 s (-7.4%)** |
| wall time (loaded machine, not comparable) | 468 s | 540 s |

One write at year 300, by part, F3 against the PR1 table: the file 22 ms against 114 ms (the header's compression 13 ms
against 91, the header built and made bytes 5 ms against 13, the frames' compression 2 ms against 8), the three small
files 2 ms against 15. In 300 years all writes cost 0.39 s against 2.0 s.

**Verdict against the kill line (the long run's wall time a year under -1%): passed.** The writes fall by 19 s of a
1,000-year run. The last report's quiet run of this island is 334 s, so that is -5.8% of the wall time; the CPU time
of the process falls by 7.4% in the measure above. A wall-time pair on a quiet machine was not possible. What is left
after F3, in a 1,000-year run: the snapshots 6.6 to 7.3 s and the tape's writes 5.6 s, about 12 s, which is 2.7%
to 3.7% of a quiet run (334 s, or 12 s of the loaded runs' 468 s to 540 s: 2.2% to 2.6%). The gate of F2 is 2%.

## F2: the gate

The gate asks whether the saves and the tape are still over 2% of a long run after F3. They are: 2.2% to 3.7% in a
1,000-year island run (above), 2.4% in the 300-year run (snapshots 1.15 s, writes 0.4 s, of 65 s). A direct measure of
the parts of one snapshot of the bench save `island22` (8.6 MB of bincode, 4 passes): the world to bytes 3.5 to 4.5 ms
(into a sink or a `Vec`), gzip level 1 of those bytes 36 to 38 ms, the same into a file 40 to 43 ms (one pass of four
took 188 ms: the disk). So 90% of a save is compression and the write, which a writer thread can take; the world to
bytes stays on the main thread.

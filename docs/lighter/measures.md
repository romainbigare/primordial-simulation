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

## Phase P: the probes (short)

The key table and the verdict of each probe. The full tables, the methods and the scripts stay on the probe branches
(never merged): `probe/r0` (R0, PR6, PR8), `probe/c0` (C0), `probe/pr3` (PR3), `probe/pr7` (PR7), `probe/pa` (PA).
All: one year from the `benchZ` saves (snapshot 49), one thread, callgrind, shares of `schedule::run_for`; the probes
change nothing (each ends on `$L/ew-base`'s fingerprints).

### R0: the start

Core code at the start: 10,444 lines.

| Share of `run_for` | island22 | globe36 | region24 |
|---|---|---|---|
| the year, M instructions | 5,019 | 5,516 | 10,557 |
| animal updates | 40.2% | 32.1% | 42.7% |
| stand updates | 21.7% | 18.5% | 24.1% |
| landing of moves and litters | 6.8% | 4.8% | 6.3% |
| the survey | 5.3% | 5.0% | 5.5% |
| the air / the liquid | 13.4% / 2.6% | 20.6% / 3.5% | 11.0% / 2.0% |
| Change / parts | 7.8% / 0 | 7.8% / 5.4% | 6.1% / 0.2% |
| a stand update / an animal update, thousand instructions | 87.6 / 159.2 | 88.5 / 154.5 | 89.7 / 161.3 |

**Verdict.** The start is not the old P0 table: animals are the largest part; stands all included are about 21 to 30%
of the year, not 37 / 19 / 60%. The write of a snapshot is 11 to 13% of a one-year run.

### PR6: a decision and the take

| Stage (island22) | per decision | share of the year |
|---|---|---|
| perceive / inputs / options / walk | 6.0 / 1.6 / 5.3 / 0.4 k | 0.77 / 0.21 / 0.68 / 0.05% |
| think / learn | 8.0 / 3.5 k | 1.03 / 0.45% |
| spread and its acts | 39.4 k | 5.03% |
| a habit (the takes again) | 18.5 k | 2.27% |
| the take (`take_node`, 16.1 k a call) / of it the payer | | 4.02% / 0.62% |

**Verdict.** Perception is about 6 of a decision's 70 thousand instructions (0.55 to 0.91% of the year); the stands
are 80% of the faces tested. D1 (the payer, 0.6%) and D2 (`perceive`, under 1%) are not built (L14).

### PR8: the parts' reactor (globe36)

| | a part, 6 months | a ground node, 10 days |
|---|---|---|
| instructions a call | 191 k | 18 to 24 k |
| sub-steps a call / halvings a call | 14.3 / 110 | 1.0 / 0 |
| the highest temperature inside a call | over 1e6 K in every call (up to 2.8e8 K) | |

1,186 calls a year, 4.2% of globe36's year. The part reacts its cell's whole air share for 6 months on its own 2.8e5 J/K.
**Verdict.** G1 (the search starts where the heat allows): -1.21%, the hash unchanged (built, below). A part reacting
as a ground node does (G2, a law fix): about -3.5% (L20).

### PR3: rot inside living bodies

| island22, one year | year | body reactor (share of the year) |
|---|---|---|
| base | 4,946 M | 322 M (6.5%) |
| the skip alone | **-2.0%** | -18% |
| (b0): decomposer reactions only by a body's own catalyst | +4.2% | -16% |
| (b0) and the skip | +1.2% | -33% |
| (d') (data in `open`, two-state unfolding) | +4.1% | 0 |

32 x 32, seeds 1 to 6, 30 years, paired with the base: the skip keeps every number within its noise (the land +0.01 K); every law
variant gives land +0.3 K, gas -22 ppm, light-eaters x1.7, **eaters x35**, updates a year +38%, animal nodes x7, S50
+0.02. Key numbers of (b0) with the skip: every climate key in range, eaters 6.67 log10 kg (out). The eaters' bare
self-burning is 0.21 to 0.30 of their upkeep. Globe36 and region24: the skip -0.9 / -2.1%.
**Verdict.** The law makes the world heavier and costs time: not built this round, an open item for the owner (L9).
The skip is built (B1).

### PR7: the air on globes

| Counted, wide (6 x 4 / 8 x 4) against 4 x 4 | globe36 | size 28, year 50 | size 28, year 100 |
|---|---|---|---|
| the air's instructions | -22.5% | -12.2% | -14.6% |
| share of the year | -4.7% | -4.2% | -3.8% |
| rain on land (each seed, 30 years) | +5.8% to +9.7% | +11% | |

The globe test passes (rain on land +12.5%); zonal temperatures move under 1 K; the busiest domain stays under 0.15.
**Verdict.** The gain passes the 3% line, but the land is wetter on every seed and world (13 to 20 times the
year-to-year spread) while the globe's rain does not move: E1 is not built (L12). Code on `probe/pr7`.

### C0: plants per cell today

| | island22 | globe36 | region24 |
|---|---|---|---|
| stands a planted cell: adult / young / all | 2.19 / 3.97 / 6.17 | 1.96 / 3.60 / 5.56 | 2.37 / 4.48 / 6.85 |
| adult mass in the 3 largest form groups | 100% | 100% | 100% |
| an adult / a young stand update, thousand instructions | 105.2 / 75.2 | 106.7 / 75.8 | 105.9 / 78.0 |
| stand updates, share of the year | 21.7% | 18.6% | 24.0% |
| the cover's gain, all included: at today's kinds / at the full bound | -13% / -5% | -11% / -2% | -15% / -9% |

**Verdict.** `cell_kinds` 3 holds; the adult update is under 120k; the old gate is out of reach; lane C is restated
(L35, L36).

### PA: the animal update by stage

| Share of the year | island22 | globe36 | region24 |
|---|---|---|---|
| animal updates | 40.2% | 32.7% | 43.0% |
| the body's day / the decision / the habit | 16.6 / 9.0 / 2.3% | 12.8 / 9.5 / 2.0% | 17.3 / 9.7 / 2.8% |
| children (`copy::copy`) / maturing | 7.3 / 4.0% | 4.8 / 2.7% | 7.8 / 4.1% |
| landing of their moves and litters | 4.9% | 1.3% | 4.2% |
| `nodes::add` (all callers; 21 k a call) | 10.1% | 6.3% | 9.9% |

Every animal's `INTERVAL` is at or under 10 days (36 updates a year); a young node's update 144 k, an adult's 185 k.
Levers (estimates): children in lumps -5.8 / -3.8 / -6.5%; maturing in lumps -2.6 / -1.8 / -2.7%; decisions every 40
days -3.3 / -3.7 / -3.4%.
**Verdict.** The bookkeeping of nodes is 16 / 9 / 16% of the year, more than the decisions: lane H (H1, H2; H3 not
built; H4 a probe) (L39).

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

## F1, the pictures

The pictures every `--every` years (`render::save`) are timed in the tape's column too, so that a run's "per year" and
its wall time agree. A picture costs 0.6 ms on the island (PR1), 0.05 s in 300 years: under 0.1% of a run. The yearly
check (0.3 to 1.3 ms) is not timed.

## F3: the tape's files written cheaper

**What it does** (`tape/src/rec/`). The viewer's file is compressed at gzip level 1 (it was level 6); its header is
written straight from the recorder's data (a struct, `serde_json::to_vec`), not through a JSON value tree; `series.csv`
and `months.csv` get only the rows added since the last write (`csv.rs`), and are written whole only when their header
line changed or at a resumed run's first write (the bookkeeping is a `Written` that the caller keeps; F2 gives it to
the writer thread, `Recorder::finish` writes the whole tape). The files hold the same rows. New test `csv_files_grown_year_by_year_are_those_written_at_once` (a kind that comes late changes
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
| CPU time of the process (user + sys), one pair on a loaded machine | 713 s | 660 s (noise: the writes fell by 19 s, this by 53 s) |
| wall time (loaded machine, not comparable) | 468 s | 540 s |

One write at year 300, by part, F3 against the PR1 table: the file 22 ms against 114 ms (the header's compression 13 ms
against 91, the header built and made bytes 5 ms against 13, the frames' compression 2 ms against 8), the three small
files 2 ms against 15. In 300 years all writes cost 0.39 s against 2.0 s.

**Verdict against the kill line (the long run's wall time a year under -1% in the brief, under -4% in the plan): passed
on an estimate.** The writes fall by 19.3 s of a 1,000-year run, read from the run's own timers. The last report's
quiet run of this island is 334 s, so that is about -5.8% of the wall time. This is an estimate, not a wall-time
pair: the machine was loaded, and the CPU pair above is noise. A quiet wall pair belongs to phase Z. What is left
after F3, in a 1,000-year run: the snapshots 6.6 to 7.3 s and the tape's writes 5.6 s, about 12 s, which is 2.7%
to 3.7% of a quiet run (334 s, or 12 s of the loaded runs' 468 s to 540 s: 2.2% to 2.6%). The gate of F2 is 2%.

## F2: the gate

The gate asks whether the saves and the tape are still over 2% of a long run after F3. They are: 2.2% to 3.7% in a
1,000-year island run (above), 2.4% in the 300-year run (snapshots 1.15 s, writes 0.4 s, of 65 s). A direct measure of
the parts of one snapshot of the bench save `island22` (8.6 MB of bincode, 4 passes): the world to bytes 3.5 to 4.5 ms
(into a sink or a `Vec`), gzip level 1 of those bytes 36 to 38 ms, the same into a file 40 to 43 ms (one pass of four
took 188 ms: the disk). So 90% of a save is compression and the write, which a writer thread can take; the world to
bytes stays on the main thread.

## F2: the writer thread

**What it does** (`cli/src/writer.rs`, `tape/src/snapshot.rs`, `tape/src/rec/mod.rs`). At each snapshot year the main
thread fills a buffer with the snapshot's bytes (bincode, the buffer is kept for the next time) and copies the tape
(`Recorder::tape`), and one writer thread compresses and writes them: the tape's files, then the snapshot (so that a
tape is never behind a snapshot). The snapshot and the viewer's file go to a `.part` file that takes its name when
whole (`gzfile::write_gzip`). One job at a time: the run waits for the last one before it starts the next
and at its end. `Recorder::finish` is the same call as before for one whole write (the tests and the exit on a broken
state use it); the thread uses `TapeFiles::write`, which keeps what the CSV files hold between jobs (`Written`). The files
are the same: a 45-year run gives the same `snap_*.bin.gz` byte for byte, the same `months.csv`, the same `series.csv`
(but the clocks) and the same viewer's file once read as the build before; a run resumed from its year 40 into the same
folder gives the same files again.

**The first try, a new buffer for each snapshot, was no gain.** The main thread's CPU time (read from
`/proc/thread-self/schedstat`, so that the load of the machine does not count) for the bytes of a snapshot into a new
`Vec` was 1.19 s in 300 years, as much as the whole save before (1.15 s): a growing buffer pays for its page faults and
copies (5 to 45 ms a time). The same bytes streamed through gzip into nothing cost 1.07 s, and gzip of the finished bytes
alone 0.94 s, so the serialising is 0.1 s in all. The kept buffer takes the bytes to 4 ms a time.

**The main thread's part, 300-year island run** (CPU time of the thread, and its wall time, over the 30 jobs):

| | F3 (the main thread writes) | F2 |
|---|---|---|
| the snapshot: bytes, compression, write | 1.15 s | 0.15 s (bytes only) |
| the tape's files | 0.4 s | 0.00 s (the copy: 0.5 ms a time) |
| together, CPU time | 1.55 s | **0.15 s** |
| together, wall time of the main thread (machine at load 6 to 9) | 1.55 s | 0.27 s |
| share of the run (65 s on a quiet machine) | 2.4% | **0.4%** |

The process's CPU time does not change (the work moved to the thread: 150 s against 149 s in 300 years). Carried to
1,000 years: the main thread's part falls from about 12 s to about 1 s, 3.3% of a quiet run (334 s).

**Wall time.** A pair of builds, 3 rounds of 300 years interleaved under the lock, gave medians of 150.3 s (F3) and
148.2 s (F2); the machine was at load 8 to 15 on 4 cores from other agents' builds, and the runs differ by 30% among
themselves, so this cannot tell 2%.

**Verdict.** F2 is judged on the count (the architect logs this): the main thread's part of the saves and the tape in
a 300-year island run falls from 1.55 s to 0.15 s of CPU time, 2.4% to 0.4% of the run. This is an upper bound of the
gain in wall time: the writer's 1.4 s of CPU now runs beside the four pool threads, and when it takes a core from a
domain's thread the barrier of that batch waits. The process uses 2.4 cores of 4 on average, so a gain is likely, but
it is not shown. A quiet wall pair (5 runs interleaved, 300 years) belongs to phase Z.

## G1: the parts' reactor starts its halving where the heat allows

Step: `Reactor::sub_step` (`core/src/change/kernel.rs`). The sub-step is halved while the heat of the running reactions
would move the node by more than `MAX_DT_K`. The heat is concave in the length (each reaction's share used is
`1 - e^-z` or its power form), so it falls by less than half when the length halves: no fewer than
`ceil(log2(heat / bound))` halvings can be enough. The first try now jumps there (not past the shortest sub-step),
then halves one at a time as before. It finds the same sub-step, so the world is the same bit for bit (level 1). The
search has one call site of `reach`; with two the compiler outlines the closure and the other reactors lose 20 M on
globe36.

Callgrind, one year, one thread (`RAYON_NUM_THREADS=1`), from the `benchZ` saves; base = `speed/lighter` at `07897b6`,
same machine, fresh out folders. Instructions in millions.

| | base | G1 | change |
|---|---|---|---|
| globe36, all | 6,601.8 | 6,538.2 | -63.6 (-0.96%) |
| globe36, `run_for` (the year) | 5,393.1 | 5,329.5 | **-63.6 (-1.18%)** |
| globe36, parts (`bond::month::step`) | 257.4 | 194.0 | -63.3 (-24.6%) |
| globe36, Change | 412.9 | 412.9 | 0 |
| island22, `run_for` | 4,932.1 | 4,931.9 | -0.2 (0.00%) |
| region24, `run_for` | 10,365.1 | 10,362.3 | -2.8 (-0.03%) |

The parts' reactor alone (the part of `bond::step` that is `Reactor::react`, 226.6 M, 191 k a call) falls to about
163 M, -28%; the same sub-steps are found with 59% fewer evaluations of the reactions' heat (60,798 against 147,289
by the count of PR8). Same fingerprint of the year on all three saves (`63c9e0c7cc7c87e5`, `0baf9b2e8c7657ec`,
`f528d300f41d6ab3`).

Checks: the golden test passes unchanged with threads and with `RAYON_NUM_THREADS=1`, and in the plain debug build;
`cargo test --release --workspace` passes; the same with debug assertions; clippy has no warnings; the wasm build
passes. Core code lines: 10,450 (+8 -2).

Verdict: kept. -1.18% of globe36's year for 8 lines passes L2's line for a level-1 step of 20 lines or less (1%). It is
under the plan row's 1.5% kill line; the architect decided for L2's line. The 4-thread year was not timed.
One section per step, in the order of the work. `$L` is the scratch folder of the round.

## The long-step test judges six worlds

`the_long_step_keeps_the_climate` judges the mean over seeds 1 to 6 of what the default steps move the climate from
the short ones (its own list, `STEP_SEEDS`; the other tests keep `SEEDS`, 1 to 3), with the limit unchanged, 0.3 K for
a temperature. One seed's offset of the land spans 0.10 to 0.40 K, so a mean of three moved by about 0.01 K with any
change of rounding, and the test had 2.5% of room. Six halve the variance. On the base (`07897b6`):

| | seeds 1 to 6 | mean | limit |
|---|---|---|---|
| land, K | 0.398, 0.103, 0.377, 0.261, 0.210, 0.207 | **0.259** | 0.3 |
| summer, K | 0.251, -0.017, 0.269, 0.187, 0.179, -0.003 | 0.144 | 0.3 |
| winter, K | 0.274, -0.042, 0.196, 0.011, -0.019, 0.097 | 0.086 | 0.3 |

Rain, gas 3 and rivers stay far within their year-to-year spread. It passes.

## B1: the skip of tiny bare reactions

**What.** In a body's reactor (`change/kernel.rs`, `Reactor::rates`; stands and animals alike, and the sample member's
day), a bare reaction (catalyst factor at most 1) whose rate, contact and orders included, would use less than
`TOO_SLOW` = 3.6e-5 of its scarcest input over the world's year is skipped. It books nothing, so the books stay exact.
It counts over the year, like the frozen rule beside it, so it does not depend on the length of a body's update (its
`INTERVAL` gene, node or individual, `batch_days`): the first form counted over the call (1e-6 a call), which made the
same body run another chemistry by its interval (review of B1, finding 1). 3.6e-5 a year is that first form at a
10-day call. Ground, air and parts' nodes pass no catalysts and are not touched. It replaces the law "living bodies do
not rot", which probe PR3 found makes the world heavier (branch `probe/pr3`, its `measures.md`).

**Why a `const`, not a `Physics` number.** It is a speed tolerance like `NEGLIGIBLE`: it says how exactly the reactor
runs, not how large a law is (named in `numbers.md` among the safeguards). At living temperatures it skips per call
about 3.4 to 4.6 of the 11.4 to 11.9 reactions a stand runs and 5.7 to 6.6 of an animal's 12.1 to 13.3: the frame
rotting and splitting (26, 27), the mineral breaking down hot (25), charring where cool (4, in part), the light and
fixing reactions where they run bare, and 2, 3, 6 (probe PR3, its section 2).

**Cost** (callgrind, one thread, one year from the `benchZ` saves; base `$L/ew-base`; calls from the call tree):

| | island22 | globe36 | region24 |
|---|---|---|---|
| the year (`run_for`), base / B1, M | 4,932 / 4,823 (**-2.2%**) | 5,393 / 5,348 (**-0.8%**) | 10,365 / 10,120 (**-2.4%**) |
| a body reactor call, base / B1, k (calls) | 12.06 / 9.77 (**-19.0%**; 26,307 / 26,220) | 11.82 / 9.40 (**-20.5%**; 22,908 / 22,923) | 12.10 / 9.66 (**-20.2%**; 59,688 / 59,655) |
| a body's day `to_growth`, base / B1, k (calls) | 53.5 / 50.0 (-6.5%; 27,161 / 27,090) | 52.7 / 49.0 (-6.9%; 24,764 / 24,816) | 54.0 / 50.1 (-7.2%; 60,925 / 60,876) |
| the cheaper day times the base's days, share of the year | -1.9% | -1.7% | -2.3% |
| `Reactor::react` in all (bodies, ground, air), M | 551 / 489 (-11.2%) | 755 / 697 (-7.7%) | 1,114 / 968 (-13.1%) |

About a third of the day's gain is outside the reactor: a skipped reaction moves no gas, so the day's later stages skip
those moves. The rest of the year's change is the slightly different world after a year (hashes `83fe7c125bb6f508`,
`d54531013bf7aec3`, `f645b07365c67e1d`): -0.3% on island22, +0.8% on globe36, -0.1% on region24.

**Kill line** (under -1% of the year on island22 and region24): passed, -2.2% and -2.4% (the cheaper day alone: -1.9%
and -2.3%).

**Level 3.** The skipped fluxes are real, if tiny (at most 3.6e-5 of an input a year, a one-sided bias that ends at
death, when the matter goes to the ground), and life's choices amplify them, so the world moves by more than rounding.
Golden re-pinned. Probe PR3's paired runs of the first form (32 x 32, 30 years, seeds 1 to 6) moved the land +0.012
± 0.004 K; the eaters' life -1.7 ± 0.8 years (2 standard errors) is noise, as the key numbers below have it +0.07
years. The key numbers (64 x 64, seeds 1 to 3):

| Number | B1 | today (speed x10, "Z: the final checks" 1) | reference |
|---|---|---|---|
| land | 292.17 K | 292.16 | 291.79 ± 1 |
| swing | 15.30 K | 15.33 | 15.13 ± 1.5 |
| rain on land | 0.001818 | 0.001819 | 0.001714 x/ 1.3 |
| gas 3 | 412.4 ppm | 412.3 | 388.8 ± 40 |
| river cells | 699 | 695 | 712.4 x/ 1.5 |
| land worn down | 0.1344 mm/yr | 0.1337 | 0.141 x/ 2 |
| light-eaters | 10.25 log10 kg | 10.25 | 10.38 ± 0.3 |
| eaters | 5.21 log10 kg | 5.24 | 5.354 ± 0.5 |
| life of light-eaters | 1.42 yr | 1.38 | 1.81 x/ 2 |
| life of eaters | 13.33 yr | 13.26 | 10.47 x/ 2 |
| S50 | 0.558 (known out) | 0.558 | under 0.5 |

**The batch test** (2 against 10 days, mean of seeds 1 to 3) passes: plant members +0.4%, light-eaters' kg -0.06%,
births +1.2%, their life -0.03 yr, gas 3 -3e-8, each far within its noise. The globe's climate test passes.

**The long-step test passes** on six worlds (the step before this one): the land 0.262 K off, against 0.259 K on the
base, limit 0.3 K. Per seed, default steps against short ones, K:

| reactor | seeds 1, 2, 3 | seeds 4, 5, 6 | mean of six |
|---|---|---|---|
| base (`07897b6`) | 0.398, 0.103, 0.377 | 0.261, 0.210, 0.207 | 0.259 |
| B1 | 0.424, 0.164, 0.329 | 0.230, 0.193, 0.231 | **0.262** |

Single seeds move by up to ±0.06 K between reactors that differ by under 1e-6 of a flux a call, in both directions, so
the skip does not warm the land. On seeds 1 to 3 alone B1 would read 0.305 K: that is why the test went to six.

**Tests.** `a_body_skips_the_same_tiny_reactions_whatever_its_calls` (`core/tests/change.rs`) states the skip: a year
in 360 calls and in 9 makes the same products and moves each starting store by the same amount to within 1e-3 (the
rest is the reactions run one after the other in a call), the frame not at all, each call exact in elements and energy;
it fails with no skip (the frame rots) and would fail with the per-call form (charring runs in 40-day calls, not in
1-day ones). `catalysts_fail_when_hot` takes its bare reference outside a body (`None`) and asserts it is above 0.

## H1: children in lumps

**What.** A population node makes its children at most once in `lump_days` (30): at its first update `lump_days` or
more after its last children, all that its brood covers then (`copy::due`). The node keeps the first batch at which it
may make them again (`Body::lump_at`), so a changed interval gene can neither make them twice in 30 days nor skip a
turn, and a part split off a node keeps the node's turn. A node that never made children waits for the first batch of
its own phase (a hash of its id): without it, nodes placed together (founders) would make theirs at the same batches
ever after (the pulse test caught this). A node whose children would all wait (under one member, all staying, no young
node) makes none and keeps its turn. The brood fills at every update as before and leaves with members who die.
Individuals make children at every update. The rule holds for every node; a node without a brain updates every 40
days, so by default each of its updates may make children (the guard `lump_days <= node_days` of decision 43 is not
built: it would refuse the pace test's settings 0 and 20). The dispersers of an update go one way (one direction draw
an update, not one a child: a direction a child would need an `emit`, a join, a child).

**Cost** (callgrind, one thread, one year; base: `speed/lighter` at `195bbc2`; the `benchZ` saves read by an H1 build
that keeps snapshot 49, the new field and setting skipped on load, so both years start from one world).

| | island22 | globe36 | region24 |
|---|---|---|---|
| the year (`run_for`), base / H1, M | 4,823 / 4,485 (**-7.0%**) | 5,286 / 4,974 (**-5.9%**) | 10,118 / 9,398 (**-7.1%**) |
| of it `copy::copy`, M | 521 / 283 (-4.9 points) | 390 / 216 (-3.3) | 1,172 / 641 (-5.3) |
| of it the landing after the batch (`outbox::land`), M | 331 / 270 (-1.3 points) | 273 / 227 (-0.9) | 648 / 497 (-1.5) |
| the rest of a body update, M | 2,420 / 2,395 (-0.5 points) | 2,269 / 2,229 (-0.8) | 5,495 / 5,484 (-0.1) |
| the rest of the year, M | 1,551 / 1,537 (-0.3 points) | 2,354 / 2,302 (-1.0) | 2,803 / 2,776 (-0.3) |
| groups of children made (`copy::emit` calls) | 12,982 / 8,127 (-37%) | 9,814 / 6,426 (-35%) | 28,780 / 18,316 (-36%) |
| joins (`nodes::add` calls) | 23,270 / 18,951 | 15,761 / 12,881 | 49,642 / 39,721 |
| animal updates (`choose::act` calls) | 12,548 / 12,494 | 10,706 / 10,584 | 27,742 / 27,898 |
| a group made (`emit`), k | 39.5 / 34.1 | 39.1 / 32.8 | 40.1 / 34.3 |

**Why more than T1's bound (-4.4% on the island).** T1 took half of the children (6.9%) and half of the litters'
landing (1.8%). The children fall by 46% here (521 to 283 M), and their landing by 18% of all landing: an adult animal
node made children at about 24 updates a year and now at 12, and each group made is 14% cheaper. That is the mix of
groups, not a cheaper group: per call the brain rows' shake falls from 10.3 k to 6.4 k (`brain::shake` in `emit`) and
the join from 19.2 k to 16.4 k (`copy::start`), because the groups of animals (with brain rows to shake) fall by about
half while the groups of stands (no rows) stay. About 1 point of the year (the rest of the update and of the year) is
the world moving within the year.

**Kill line** (under -3% of the year on island22 and region24, counted): passed, -7.0% and -7.1%.

**Do the animals move?** (Review of lane H, item 1.) Base `195bbc2` against H1 (and H5), with counters in a probe
build (never merged): litters that land and found a node, join one, or go back; stayer groups that found a node; the
young share. Seeds 1 to 3 each.

The key-number island (64 x 64, 100 years: the eaters there settle by year 70 to 100; "Reading the eaters", handover
13.2): eaters' log10 kg over years 91 to 100, the rest over years 71 to 100, a year.

| | base | H1 | H1 and H5 |
|---|---|---|---|
| eaters, log10 kg | 8.29, 8.35, 8.07 (8.24) | 8.31, 8.23, 8.11 (8.22) | 8.50, 8.31, 8.04 (8.28) |
| eater members, millions | 4.57, 5.33, 2.81 | 4.89, 4.09, 2.91 | 7.17, 4.77, 2.57 |
| eater births a year, millions | 1.06, 1.25, 0.64 | 1.07, 1.03, 0.68 | 1.41, 1.13, 0.63 |
| (births - deaths) / births | -0.13, -0.04, -0.08 | -0.08, -0.09, -0.06 | -0.03, -0.06, -0.09 |
| young share of animal members | 0.820, 0.813, 0.808 | 0.809, 0.814, 0.794 | 0.807, 0.818, 0.810 |
| animal nodes; cells with animals | 2,922; 1,445 (means) | 2,953; 1,441 | 3,425; 1,442 |
| litters that found a node | 722, 1,136, 595 | 475, 542, 378 | 530, 556, 374 |
| litters that join a node | 31,685, 26,871, 28,232 | 11,797, 9,358, 10,715 | 13,996, 9,838, 10,639 |
| litters sent back | 1,307, 764, 1,256 | 358, 175, 354 | 961, 210, 401 |
| stayer groups that found a node | 66, 7, 127 | 60, 10, 80 | 220, 8, 99 |
| died of lack | 0, 5, 0 | 0, 0, 0 | 0, 0, 0 |

island22 (22 x 22, 300 years, the bench's world; log10 kg over years 271 to 300, the rest over 241 to 300): eaters
7.57, 7.07, 6.80 (base), 7.40, 7.11, 6.62 (H1), 7.53, 6.91, 6.87 (H1 and H5); animal members at year 300 8.3e5, 3.0e5,
2.1e5 (base), 4.3e5, 2.2e5, 1.0e5 (H1), 6.5e5, 1.3e5, 1.8e5 (H1 and H5); litters that found a node 296, 135, 70 (base),
121, 112, 34 (H1); cells with animals 154, 155, 163 (means).

globe36 (an 18-cell-side globe, 150 years; eaters there spread late: log10 kg over years 141 to 150, the rest 121 to
150): eaters 12.52, 12.55, 12.30 (base), 12.71, 12.68, 12.47 (H1), 12.47, 12.39, 12.31 (H1 and H5); cells with animals
150, 81, 112 (base), 154, 101, 106 (H1), 136, 65, 107 (H1 and H5); litters that found a node 684, 178, 415 (base), 286,
133, 181 (H1); died of lack 4, 0, 0 (base), 8.0e6 (0.04% of births), 1,167, 0 (H1), 19, 3,211, 3 (H1 and H5).

- **The warning sign is the world of B1, not H1.** The remade island22 bench holds 6.55e5 animal members against 0.9e5
  to 2.5e5 on the earlier bench builds, but the base `195bbc2` itself (after B1 and G1) holds 8.3e5 on the same seed
  and year: the earlier builds were made before B1.
- **The animals stay within their seeds' noise** on all three worlds: mass, members, births, births less deaths, the
  young share, nodes and cells. On the globe H1's eaters lie 0.16 decades above the base (seeds spread 0.25), and H1
  with H5 0.07 below it: the gap between two steps that do not change timing (H1, H1 and H5) is as large, so it is
  noise.
- **Larger lumps found fewer nodes, not more.** Litters are fewer and larger (a third as many land), so more of them
  pass one member, but in all fewer found a node (-43% on the 64 x 64 island, -53% on the globe) and far fewer go
  back (-73%). The cells the animals hold stay the same: the range does not depend on the founding rate at this level.
- **Lack:** one globe seed shows 8e6 deaths of lack a year with H1 (0.04% of births; the base 4). Larger lumps take
  larger provisions at once (review item 7); watch it.

**Births inside a month** (the 64 x 64 island, years 71 to 100, node births of animals by batch): the share of birth
updates in the first, second and third batch of a month is 0.331, 0.333, 0.335 (base) and 0.328, 0.326, 0.346 (H1);
of members born 0.336, 0.329, 0.335 and 0.328, 0.312, 0.360. By batch of the year the largest and smallest share of
birth updates are 0.0362 and 0.0188 (base, the seasons) and 0.0357 and 0.0207 (H1); an even share is 0.0278. No pulse;
the third batch of a month holds 4% more than an even third.

**Level 4.** Golden re-pinned. The key numbers (64 x 64, seeds 1 to 3) against B1: land 292.15 K (B1
292.17), swing 15.30 (15.30), rain on land 0.001819 (0.001818), gas 3 412.2 ppm (412.4), river cells 696 (699), land
worn down 0.1335 mm/yr (0.1344), light-eaters 10.25 (10.25), eaters 5.18 (5.21), life of light-eaters 1.43 yr (1.42),
life of eaters 13.34 yr, seeds 14.8, 11.8, 13.5 (13.33), S50 0.559 (known out). All in their ranges.

**Tests.** The nodes check, the pace and batch tests, the long-step test on six worlds (land 0.273 K off, limit 0.3 K; B1
0.262), the globe's climate test, `a_century_keeps_its_books` and `the_eaters_live_and_stop_growing` pass. New: `a_node_makes_its_children_at_most_once_in_lump_days` (updates at gaps of one to three batches:
no two births under 30 days apart, none skipped at an update 30 days or more after the last, as many as at every update
less what waits in the brood; a node that joins it and the half split off it make none until 30 days after its births,
then some), `no_batch_holds_a_pulse_of_births` (thirty nodes placed together, a year: each batch within 25% of its even
share; one shared phase gives 200%, phases 2:1 give 33%). The node tests that are not about lumps set `lump_days` to
one batch; the settings test refuses a `lump_days` that is not whole batches.

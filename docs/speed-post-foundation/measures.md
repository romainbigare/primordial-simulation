# Measures: what each step of the speed work gained

One row for each step that touches speed. The plan is `plan.md`; `scripts/bench.sh` takes the numbers.

## How to read the table

- **Benches.** `island22` is a 22 x 22 island (`ew run --size 22 --seed 1`) saved at year 300. `globe36` is a globe of
  36 x 18 cells (`ew run --globe --size 18 --seed 1`) saved at year 200. Both are settled worlds. The saves live in
  `~/.cache/ew-bench`, outside the repository.
- **How the numbers were taken.** The bench goes on from the save for 5 years (`ew resume --years 5`), twice, and keeps
  the run with the lower total. The machine has 4 cores and nothing else was running. Each number is the mean of the 5
  years. Two repeats of the same code differed by about 1 to 8% in the total (the globe more than the island).
- **Columns.** `s/yr` is the sum of the five stage times, in seconds for one simulated year. `flow`, `change`,
  `bodies`, `parts`, `tape` are the `ms_*` columns of `series.csv` in ms a year: the wall time in Flow, in Change, in
  the bodies (the survey and the body days), in the parts, and in the tape itself. `alive` is the column `bodies`
  (bodies alive, nodes and individuals). `updates` is the column `updates` (body updates in the year). `core lines`
  are the code lines of `core` (`cargo test --release -p cli --test core_size -- --nocapture`; the ceiling is 9,700, decision S3).

## The rows

| Step | Commit | island22 s/yr | flow | change | bodies | parts | tape | alive | updates/yr | globe36 s/yr | flow | change | bodies | parts | tape | alive | updates/yr | core lines |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline | `16e6a4e` | 0.857 | 159 | 49 | 638 | 0 | 11 | 2,744 | 27,762 | 1.347 | 229 | 51 | 1,010 | 43 | 15 | 1,955 | 25,626 | 8,998 |
| A2 | A2 | 0.702 | 128 | 40 | 525 | 0 | 10 | 2,744 | 27,762 | 1.220 | 210 | 49 | 905 | 43 | 12 | 1,955 | 25,626 | 8,998 |
| E2.1 | E2.1 | 0.728 | 135 | 46 | 538 | 0 | 10 | 2,744 | 27,762 | 1.274 | 225 | 55 | 940 | 40 | 14 | 1,932 | 25,410 | 8,996 |
| D3 | D3 | 0.730 | 122 | 44 | 553 | 0 | 11 | 2,744 | 27,762 | 1.215 | 188 | 48 | 922 | 43 | 13 | 1,955 | 25,626 | 9,001 |
| E2.2 | E2.2 | 0.735 | 126 | 45 | 552 | 0 | 12 | 2,769 | 27,974 | 1.214 | 200 | 54 | 906 | 40 | 14 | 1,904 | 25,345 | 9,008 |
| D2.1a (benches remade, `/tmp/d21-bench`) | D2.1a | 1.267 | 184 | 52 | 931 | 82 | 18 | 2,836 | 32,886 | 1.641 | 233 | 57 | 1,202 | 125 | 23 | 2,142 | 27,570 | 9,107 |
| D2.1b (the same saves) | D2.1b | 1.281 | 150 | 55 | 981 | 75 | 19 | 2,836 | 32,886 | 1.609 | 222 | 54 | 1,185 | 125 | 23 | 2,142 | 27,570 | 9,149 |
| D2.3 (benches remade, `/tmp/d23-bench`) | D2.3 | 1.395 | 141 | 50 | 1,153 | 33 | 19 | 3,026 | 39,306 | 1.530 | 226 | 53 | 1,187 | 43 | 21 | 2,412 | 29,278 | 9,364 |
| D2.4a (benches remade, `/tmp/d24-bench`) | D2.4a | 1.442 | 149 | 51 | 1,201 | 20 | 21 | 2,839 | 36,465 | 1.174 | 205 | 54 | 892 | 9 | 15 | 1,512 | 22,280 | 9,577 |
| D2.4b (island22 remade at `eec6f1d`, `/tmp/d24b-bench`; `eec6f1d` there: 2.093 s/yr) | D2.4b | 2.108 | 141 | 50 | 1,885 | 3 | 28 | 3,679 | 53,648 | — | — | — | — | — | — | — | — | 9,684 |
| D2.5 (benches remade, `d25-bench` in a scratch folder; island22 the same world as D2.4b's) | D2.5 | 1.187 | 137 | 51 | 971 | 3 | 25 | 3,679 | 53,648 | 0.858 | 204 | 51 | 571 | 18 | 14 | 1,648 | 23,373 | 9,678 |
| D2.5b, the largest domains first (the same saves) | D2.5b | 1.179 | 143 | 51 | 956 | 3 | 27 | 3,679 | 53,648 | 0.801 | 203 | 50 | 515 | 18 | 14 | 1,648 | 23,373 | 9,679 |
| D2.6, the survey on threads (the same saves) | D2.6 | 1.062 | 143 | 48 | 841 | 3 | 27 | 3,679 | 53,648 | 0.777 | 203 | 50 | 493 | 18 | 14 | 1,648 | 23,373 | 9,691 |
| E2.3, the repeated reads of one update (the same saves) | E2.3 | 1.044 | 147 | 48 | 821 | 3 | 25 | 3,679 | 53,648 | 0.796 | 209 | 49 | 505 | 20 | 13 | 1,648 | 23,373 | 9,689 |
| E2.3, the recorder on the pool, not kept (medians of 7 runs on 4 threads, the saves of the profile after D2; branch `speed/e23-tape-not-kept`) | `7e21f1f` | 1.096 | 146 | 50 | 879 | 3 | 20 | 3,679 | 53,648 | 0.826 | 213 | 52 | 530 | 19 | 12 | 1,648 | 23,373 | 9,689 |
| E2.3 (1), the booking once per number (the same saves; another history, level 2) | E2.3 (1) | 1.056 | 142 | 51 | 833 | 3 | 26 | 3,634 | 53,515 | 0.795 | 210 | 50 | 505 | 16 | 15 | 1,620 | 23,160 | 9,688 |

### A2: compiler settings

Each setting alone, then together, with `scripts/bench.sh` (lower of the runs of each bench; 2 to 4 runs per setting).
The machine drifted by about 7% between hours, so the baseline was measured again between the settings: its best runs
were 0.783 s (island22) and 1.341 s (globe36). Gains are against those.

| Setting | island22 s/yr | gain | globe36 s/yr | gain | Kept |
|---|---|---|---|---|---|
| a. `lto = "fat"` | 0.786 | 0% | 1.323 | 1% | no (under 2%) |
| b. `codegen-units = 1` | 0.727 | 7% | 1.284 | 4% | yes |
| c. `target-cpu=native` | 0.808 | -3% | 1.379 | -3% | no (slower) |
| d. a + b + c | 0.759 | 3% | 1.429 | -7% | no |
| a + b | 0.748 | 4% | 1.304 | 3% | no (a adds under 1% to b) |
| b + c | 0.752 | 4% | 1.365 | -2% | no |
| e. `panic = "abort"` | not measured | | | | no: `core/tests/store.rs` has a `should_panic` test |
| PGO on b (`-Cprofile-generate`, 2 years of both benches, `llvm-profdata merge`, `-Cprofile-use`) | 0.690 | 12% | 1.233 | 8% | no |

Kept: `codegen-units = 1` (4 before). Not kept: fat LTO and `target-cpu=native` (no gain on the cores that matter);
PGO gains 5% and 4% on top of b (about 12% and 8% against the baseline), under the 8% bar on both, and the final
measure of the kept build (row A2) is 0.702 and 1.220, so PGO adds 1 to 2% over it at the same quiet hour (S35). Build
time of `cargo build --release -p cli` after `touch core/src/lib.rs`: 25.6 s before, 36.4 s with b, 53.9 s with fat LTO
and b. The golden fingerprints move (level 2): the bench worlds are bit for bit the same for 5 years from their saves,
but fresh 16 x 16 worlds differ at rounding from year 3 (4 years, seed 1: 360 bodies against 356, `t_air` 246.664
against 246.662 K). The builds with and without threads give the same new fingerprints.

Stage times are in ms a year. The plan's guess of 1.5 and 2.1 s a year (from `docs/reports/foundation-l.md`) was
higher than today's 0.86 and 1.35 s: the worlds here are the two benches, and the code is the one after the
foundation. These are the numbers the later rows are compared with.

### A3: many worlds at once

Three runs of `ew resume --from island22.bin.gz --years 5` (island22 at `target/release/ew`), wall time of the whole
set, the lower of two sets. One run alone takes about 3.9 s. All three runs end with the same hash.

| Way | Wall time, three 5-year runs | Worlds per minute |
|---|---|---|
| a. one after another (4 threads each) | 11.33 s | 15.9 |
| b. all at once, default threads (4 each, 12 in all) | 3.94 s | 45.7 |
| c. all at once, `RAYON_NUM_THREADS=1` each | 3.98 s | 45.2 |
| d. all at once, `RAYON_NUM_THREADS=2` each | 3.99 s | 45.1 |

Running side by side is 2.9 times faster than one after another. The three pool settings differ by 1%, which is
inside the noise (the other set of each way was 3.99, 4.07 and 4.01 s). Bodies are three quarters of a year and run on
one thread, so a world uses about one core and the pools hardly compete. In one process (`tape/tests/ranges.rs`) the
seeds are `std::thread`s that all call the one global rayon pool in `core/src/par.rs` (no other pool is made), so the
four workers are shared and the cores are not oversubscribed.

Advice: start the seeds as separate `ew` runs at the same time (`ew run ... --seed 1 & ew run ... --seed 2 & ... wait`),
up to the number of cores. `RAYON_NUM_THREADS` does not matter on 4 cores with 3 runs; set it to 1 only with more runs
than cores.

### E2.1: members of a node that stay in their cell

Three runs of `scripts/bench.sh` each, the code before (`b75d5b0`) and after taken in turn: globe36 bodies 1,022, 992
and 971 ms against 940, 933 and 960 ms (best against best -4%, per update -3%; the island has no brains and does not
change). Under callgrind, one year of globe36 takes 10.03 billion instructions against 10.41, the bodies
(`survey::day`) 5.02 against 5.40 billion (-7%), and `spread` 1.17 against 1.51 billion: taking members back
(`nodes::rejoin`) costs 48 million where `nodes::take_in` cost 363 million, and `nodes::part` (now only for moves and
bonds) 71 against 124 million. Level 3: a node's row spread is no longer pooled with itself and capped again when its
members watch, and the split and the means are worked from the acting side (other rounding); golden re-pinned. Key
numbers: eaters OUT of the old range, inside after S49: 5.391 log10 kg (seeds 5.291, 5.474, 5.409) against 5.354 at
`b75d5b0` (seeds 5.295, 5.414, 5.352), old range 4.86 +- 0.5 (up to 5.36); a variant with only E2.1's rounding (the old
cap kept) gives 5.379 (seeds 5.305, 5.446, 5.385). The light-eaters' stored share is 0.557 before and after (S50).
### D3: the physics on threads

**The measure before D3** (callgrind, one simulated year, share of all instructions, island22 / globe36). Already on
worker threads: the air's `pass` (5.45 island) and Change's reactor rows (4.3 island). Serial on the main thread and
independent per cell or per air node, the candidates: the air's `gather` (with `solve` 1.3 / 2.3), the air state
`nodes` and `sums` (0.45 / 0.39), `dissolve` (0.86 island), `fixed` and `room` (tiny), liquid `slopes` (0.25 / 0.04),
land `ground_state` (0.98 / 0.80), Change's `Batches::add` (0.60 island) and the row work inside `air::book` (2.4 / 2.6,
but most of that is `store::apply`, which stays serial). Together about 3% of the instructions, so the loops alone
could not give 1.2 times on Flow (S48).

**Each loop was judged** (S45) in wall time with threads (the stage timed inside the code, the mean of 4 years, 4 cores;
the machine was shared with another job, so the ranges are wide) against its serial time. The cost of a call into the
pool decided it: a loop that takes under about 200 µs
a call gains nothing, because handing the rows out and waiting for them costs 30 to 55 µs, and a loop that reads its
cells several times (`gather`) pays that again.

| Loop (built as `par::rows`, results bit for bit) | Serial, ms a year (island / globe) | One call, µs | On threads, ms a year | Kept |
|---|---|---|---|---|
| air `gather`, per air node over its cells in cell order | 7.0 / 16 | 88 / 110 | 6.5 to 8.5 / 20 to 25 | no |
| air `fixed` and `room` | 1.5 / 1.8 (`fixed`) | 42 / 50 | 2.5 to 3.5 / 3.0 to 4.2 | no (slower) |
| liquid `slopes` | 2.2 to 2.5 / 4.2 to 5.0 | 190 / 400 | 2.0 to 2.2 / 3.0 to 4.2 | no (under 1 ms, under the noise) |

Not built, because each takes under 200 µs a call and the table shows that does not pay: the air state's loops
(`nodes`, `sums`, `seen`: 4.8 / 6.2 ms a year together, 133 / 172 µs a call for the three), `dissolve` (8.0 / 10.4 ms,
of which 5 ms is `apply`), `ground_state` (4.8 / 10 ms, 57 / 120 µs), `Batches::add` (about 58 µs) and the row work of
`book` (27 of 161 million instructions on the island, the rest is `apply`).

**What did gain** is in `par.rs`, and it is for every loop that uses `par::rows`:

- `par::rows` had a least size of task (4,096 numbers). The rows of the air's `pass` and of the reactor are heavy (an
  iteration to a balance each), so a pass was cut into only a few tasks. Now the rows split as far as other threads are
  idle. Alone this made Flow 1.14 times faster on the island (146 to 128 ms) and 1.17 times on the globe
  (242 to 207 ms; 5 runs of each build, interleaved).
- `ew` makes its main thread one of the pool's threads (`par::adopt_thread`). Before, the main thread handed each
  loop to the workers and slept until they were done, which costs a wake-up each way. Now it starts the loop at once and
  works on it with the others. On top of the first change: Flow 135 to 125 ms on the island (5 runs, interleaved).
  `RAYON_NUM_THREADS=1` still works (the main thread alone).

**The result** (7 runs of each build, interleaved in the same hour; a run during which another process used more than
0.2 core was thrown away and made again; medians, ms a year):

| | island22 flow | island22 change | globe36 flow | globe36 change |
|---|---|---|---|---|
| before D3 | 150.1 | 48.1 | 229.1 | 53.7 |
| after D3 | 121.9 | 44.4 | 188.1 | 48.1 |
| faster by | 1.23 times | 1.08 times | 1.22 times | 1.12 times |

The air's `pass`, timed inside, went from about 53 to 31 ms a year on the island and from about 102 to 60 ms on the
globe. The whole year (row D3 above) went from 0.792 to 0.730 s on the island and from 1.313 to 1.215 s on the globe
(same-hour medians). One simulated year costs the same instructions as before (6.288 to 6.293 billion on the island,
10.410 to 10.411 billion on the globe): the work is the same, spread over the four threads. The golden
fingerprints are unchanged with and without threads, and the 5-year runs of both benches from their saves give the same
`series.csv` (outside the `ms_*` columns) as before D3. The S48 target (Flow 1.2 times faster on 4 cores) is reached on both
benches, by `par.rs` and not by the loops of the list. Core code: 9,002 lines (+4).

### E2.2: powers and exponentials in the body update

Judged on callgrind instructions of one year on one thread (S45; `RAYON_NUM_THREADS=1`, so idle threads add no
instructions), before at D3, which keeps E2.1's bits:

| | island22 before | island22 after | globe36 before | globe36 after |
|---|---|---|---|---|
| all instructions, million | 6,269.8 | 6,138.4 (-2.10%) | 9,996.4 | 9,855.4 (-1.41%) |
| body updates in the year (`updates`) | 27,672 | 27,691 | 25,267 | 25,277 |
| `survey::day` (inclusive) per body update | 125,372 | 121,056 (-3.44%) | 197,935 | 193,627 (-2.18%) |
| outside `survey::day`, million | 2,800.5 | 2,786.2 | 4,995.2 | 4,961.1 |
| `pow` (exclusive, million) | 200.1 | 138.1 | 234.9 | 160.7 |
| `exp` | 169.3 | 140.6 | 220.4 | 202.7 |
| `expm1` | 79.2 | 67.9 | 133.5 | 118.9 |
| `cbrt` | 34.2 | 33.1 | 48.2 | 47.2 |
| `log` | 38.2 | 38.3 | 57.5 | 57.6 |
| `log1p` | 10.4 | 7.8 | 26.8 | 22.4 |

The other rounding changes the history, so the instructions of a year could move with the number of bodies. The
measure per body update cannot be moved by that: the saving of the body day per update, times the updates of the year
before, is 119.4 million on the island (1.91% of the year's instructions) and 108.9 million on the globe (1.09%). The
work outside the body day (Change's reactor, `sub_step`) saves 14.3 and 34.1 million more (0.23% and 0.34%). By the
measure per update the step is under its kill line of 2% on both benches (the island at 1.91%, 2.14% with the work
outside the bodies; the globe at 1.09%, 1.43% with it). In the first year the history moves the updates by under 0.1%,
so the whole-year measure is not inflated by it. The wall times in the row are of another hour than the rows before
(the machine drifts by about 7%), so they are not compared. What was cheaper:

- Values that follow from the genes and the chemistry, worked out once in `Body::prepare` (`Body::k`): the rate of the
  main burning at `t_cat` that `pace` and `upkeep_pace` divide by (two `exp` and a `pow` per call before), the copy
  mass (an `exp` per `dec(COPY_MASS)`, in eight places) and the tissue factor of upkeep (a `pow` per upkeep). This part
  gives the same bits as before (the golden test did not move with it): island22 -52 million.
- Fixed exponents: the power 2/3 of a cube's surface and of the coat's conductance as a cube root (one `cbrt` in
  `conductance`, which had two), the power 1/4 of `Physics::think` and of the height as two square roots, `10^WET` as
  `dec`; the film's leaf area is taken from the one `film` of `physics` (it was worked out twice).
- `Reactor::sub_step` worked out `gone` (an `expm1` and a `ln_1p`) twice for each running reaction, once for the heat
  guard and once for `far`; now once (same bits).

Not done: `equiv` (`mass^intake_exp`, 338,000 `pow` calls a year on the island, 44 million instructions) because
`intake_exp` is a free number of `Physics`; as the constant 3/4 it would be two square roots. The same for
`activity` (`dormancy`), the Newton steps of `exchange::through` and `denature`. Level 2 (other rounding): golden
re-pinned. Five years from the bench saves, island22 against E2.1: `t_land` differs by 1e-5 to 1.2e-4 (relative) in
each year, `bodies` by up to 2.4% and `root_0_kg` by up to 2.7% (year 1: 0.2% and 0.3%); globe36 `t_land` by 2e-4,
`bodies` -4.7%, `root_0_kg` -2.5% at year 5. Life's choices amplify rounding, as in A2 (the first year of the island
differs in the bodies by 0.2%). The level-2 world check (the key numbers) is covered by the next key-number run (B1
with B3); if that run falls out of range, the key numbers are run at E2.2 alone to tell the causes apart.

### D2.1a: stable body ids

Snapshot version 46, so both benches were remade with D2.1a's code (`BENCH_DIR=/tmp/d21-bench scripts/bench.sh
--remake`, saves of commit `ff48805`). Remade worlds are other worlds (S65): the island now has parts (82 ms a year)
and 2,836 bodies, so its row is not compared with the rows above. The row is the lowest of six runs (island) and two
runs (globe) of the bench, interleaved with D2.1b's.

Cost per body update, callgrind on one thread (`RAYON_NUM_THREADS=1`), one year: `survey::day` inclusive over the
year's updates. `main` is `5a04c54`.

| Build and saves | island22 updates | per update | globe36 updates | per update |
|---|---|---|---|---|
| `main` on `~/.cache/ew-bench` (made at the baseline) | 27,691 | 121,059 | 25,277 | 193,592 |
| `main` on saves remade with `main` | 31,132 | 147,068 | 26,353 | 198,703 |
| D2.1a on its remade saves | 33,527 | 152,243 | 27,633 | 210,957 |

D2.1a against `main` on saves each remade with its own code: +3.5% (island) and +6.2% (globe) per update. The new
rng keys give other worlds, and the difference is spread over the laws (on the globe `bond::month::step` and
`choose::act` cost 2 times and 1.3 times as much: more parts and more bodies with a brain), not in the code D2.1a
adds: the id table (a `BTreeMap`) costs 0.2% of the island's instructions in that run, and the walks of the cells'
lists (`Life::alive`) about 0.1%. D2.1's own cost is measured exactly by D2.1b against D2.1a, on one world.

### D2.1b: domains own their bodies

Level 1 against D2.1a: the golden fingerprints do not move (with and without threads), and 3 years from each of
D2.1a's saves give the same fingerprint (`ew resume` prints it as `hash`: island22 `cd5d9b3a6ee9bfe9`, globe36
`c49613dcff20132e`), the same series and the same last snapshot with both builds. In the golden runs 314 bodies change
domain (298 already filed in their new domain's ring, 16 placed parts whose due update moves with them).

Cost per body update, callgrind on one thread, one year from the same saves (the same history):

| | island22 | globe36 |
|---|---|---|
| D2.1a, `survey::day` per update | 152,243 | 210,957 |
| D2.1b | 153,605 (+0.89%) | 212,909 (+0.93%) |
| all instructions of the year | +0.69% | +0.52% |

Under the 3% allowed for the handles. Most of it is the extra index of a body through its domain where a cell's
bodies are walked often: `exchange::cover_of` (the cover of the other bodies of a cell, at each update) is about +19 and
+18 million of the +68 and +66 million, then perception and the survey's sums. Wall time agrees: the bodies' stage on
island22, six interleaved runs of each build, median 1,008 ms (D2.1a) and 1,017 ms (D2.1b), +0.9%; on globe36, two
runs each, 1,202 and 1,264 against 1,185 and 1,234 ms.

### D2.2: batches of two days

Level 3 (the golden fingerprints are re-pinned, the same with and without threads; snapshot version 47). Both benches
were remade with D2.2's code (`BENCH_DIR=/tmp/d22-bench scripts/bench.sh --remake`, saves of commit `3cdc57d` plus
D2.2), so the rows are other worlds than D2.1b's (S65).

**The work per batch** (D2.5's unit; 180 batches a year), from the bench (lowest of two runs, mean of the last 5 years):

| | island22 | globe36 |
|---|---|---|
| bodies' updates a year | 32,105 | 25,915 |
| updates per batch | 178 | 144 |
| bodies' stage a year (survey included), ms | 891 | 992 |
| ms per batch | 4.95 | 5.51 |
| µs per update (wall) | 27.7 | 38.3 |

A batch holds 4.95 and 5.51 ms of body work against a pool's call of 30 to 55 µs (about 100 times as much).

Cost per body update, callgrind on one thread, one year from each remade save (the body batch inclusive,
`processes::closure#9`, over the year's updates):

| | island22 | globe36 |
|---|---|---|
| updates in the year | 32,122 | 26,659 |
| D2.1b, per update (its own saves) | 153,605 | 212,909 |
| D2.2, per update | 151,533 (-1.3%) | 205,525 (-3.5%) |
| all instructions of the year, million | 9,467 | 10,965 |

The two rows come from other worlds, so the change in the code itself is no more than this: an update costs about what
it cost, as the batch only moves when an update runs (S65).

Key numbers (`the_key_numbers_stay_in_their_ranges`): all in their ranges; the eaters 5.350 (seeds 5.298, 5.401, 5.351)
against 5.354 (S49); the light-eaters' stored share fails as before (S50), 0.558 (seeds 0.563, 0.556, 0.554) against
0.552 to 0.557. The equivalence test (`the_batches_of_bodies_keep_life`, batches of 1 day against 2): each mean moves
by 9% of its noise or less (the move as a share of the mean, then of the noise): `plant_members` +1.3% (1.4%),
`root_0_kg` +0.6% (0.8%), `births` +1.5% (2.0%), `root_0_life_real` -0.1% (2.6%), `air_3` -0.02% (8.8%).

### D2.3: faces and the store's view

Level 3 (the golden fingerprints are re-pinned, the same with and without threads; no change of the snapshot). Bodies
see each other by the faces taken at the batch's start and the store through their domain's view; the batch's moves
are booked by one `apply` per domain. The benches were remade with D2.3's code (`BENCH_DIR=/tmp/d23-bench
scripts/bench.sh --remake`, saves of commit `e15c30c` plus D2.3), with the third bench `region24` (S41):

| Bench | s/yr | flow | change | bodies | parts | tape | alive | updates/yr | busiest domain |
|---|---|---|---|---|---|---|---|---|---|
| island22 | 1.395 | 141 | 50 | 1,153 | 33 | 19 | 3,026 | 39,306 | 0.170 |
| globe36 | 1.530 | 226 | 53 | 1,187 | 43 | 21 | 2,412 | 29,278 | 0.228 |
| region24 | 3.185 | 207 | 72 | 2,859 | 1 | 46 | 5,407 | 83,017 | 0.274 |

The busiest domain holds 17%, 23% and 27% of a batch's updates (the year's mean, column `busiest_domain`): four
threads could at best share a batch 4 times on the island, 4 times on the globe and 3.6 times on `region24`.

Cost per body update, callgrind on one thread, one year, the body batch inclusive (`survey::batch`) over the year's
updates. The same save gives the same start to both builds (D2.3 does not change the snapshot), so these compare the
code; the histories then part (level 3):

| Saves | Build | island22 | globe36 | region24 |
|---|---|---|---|---|
| D2.2's (`/tmp/d22-bench`) | `main` (D2.2) | 151,528 | 205,549 | — |
| | D2.3 | 152,062 (+0.35%) | 205,820 (+0.13%) | — |
| D2.3's (`/tmp/d23-bench`) | `main` (D2.2) | 156,458 | 204,785 | 177,148 |
| | D2.3 | 156,230 (-0.15%) | 205,800 (+0.50%) | 169,319 (-4.4%) |
| | all instructions of the year, D2.3 against `main` | +0.07% | +0.50% | -1.68% |

The view and the faces cost under 1% an update (the plan allows 5%). What it costs and saves: a move is gathered
through the view (`World::gather_move`, about 3.4% of island22's instructions, against 1.6% for the three nodes of one
update before); one `apply` a domain and batch in place of one an update books as many moves (`apply` itself costs the same);
a cell's nodes are taken into the view when a move first touches them, not three nodes at every update; the faces of
every living body each batch. A first build that filled the whole view of a domain at once cost 7 to 8% an update
more: its reads went through a look-up that the compiler did not inline, and Bond's monthly step and `wear` (outside
the batches, one view a part or a cell) filled all of a domain's cells each time. A ground node's heat capacity and
melting read its amounts through one look-up of the view (`World::amounts_now`), not one a substance: a closure that
read the view per substance was not inlined and cost 3% an update.

Key numbers (`the_key_numbers_stay_in_their_ranges`): all in their ranges; the eaters 5.364 (seeds 5.271, 5.403, 5.419)
against 5.354 (S49); the light-eaters' stored share fails as before (S50), 0.555 (seeds 0.560, 0.554, 0.549).

### D2.4a: the outbox and the serial phase

Level 3 (the golden fingerprints are re-pinned, the same with and without threads; no change of the snapshot). Every
effect of an update outside its domain goes to the domain's outbox and is applied in the serial phase; the bodies
that hold or are inside a part run there; the update code still takes `&mut World` and the domains still run one
after another (S69). The benches were remade with D2.4a's code (`BENCH_DIR=/tmp/d24-bench scripts/bench.sh
--remake`, saves of commit `17344b2` plus D2.4a):

| Bench | s/yr | flow | change | bodies | parts | tape | alive | updates/yr | busiest domain |
|---|---|---|---|---|---|---|---|---|---|
| island22 | 1.442 | 149 | 51 | 1,201 | 20 | 21 | 2,839 | 36,465 | 0.174 |
| globe36 | 1.174 | 205 | 54 | 892 | 9 | 15 | 1,512 | 22,280 | 0.240 |
| region24 | 2.760 | 239 | 89 | 2,308 | 79 | 45 | 5,165 | 64,404 | 0.271 |

The globe's save holds fewer bodies (1,512 against 2,412 in D2.3's) and fewer parts; the worlds differ at level 3.

Cost per body update, callgrind on one thread, one year from D2.4a's saves, the body batch inclusive
(`survey::batch`) over the year's updates (D2.4a does not change the snapshot, so `main` resumes the same saves):

| Build | island22 | globe36 |
|---|---|---|
| `main` (D2.3) | 156,765 | 186,744 |
| D2.4a | 157,521 (+0.48%) | 186,019 (-0.39%) |
| all instructions of the year, D2.4a against `main` | +1.26% | -0.01% |

**The serial phase's share of the bodies' instructions** (it limits D2.5), from a build of D2.4a whose phases are
kept out of line for the measure only, same saves, share of `survey::batch` inclusive:

| Part | island22 | globe36 |
|---|---|---|
| the parallel phase (the bodies that touch no part) | 89.6% | 87.5% |
| the serial phase, all | 9.8% | 12.0% |
| of it: the domains' moves booked (one `apply` per domain) | 4.5% | 3.8% |
| of it: the outboxes applied (`outbox::deliver`; litters landing are most of it on the island) | 4.2% | 6.0% |
| of it: the bodies that hold or are inside a part | 0.6% | 1.4% |
| of it: ids made global, holders counted, the serial moves booked | 0.5% | 0.8% |
| before the phases (the faces, the list of updates) | 0.6% | 0.5% |

So at most 1 / (0.104 + 0.896 / 4) = 3.0 times (island22) and 1 / (0.125 + 0.875 / 4) = 2.9 times (globe36) faster
bodies on four threads, before the busiest domain's share and the threads' own cost.

Key numbers (`the_key_numbers_stay_in_their_ranges`): all in their ranges; the eaters 5.377 (seeds 5.326, 5.434, 5.372)
against 5.354 (S49); the light-eaters' stored share fails as before (S50), 0.555 (seeds 0.562, 0.551, 0.551). The
nodes check (F7, `tape/tests/nodes.rs`): differences past the noise in the rules not held, 8 against 12 for `main`
(D2.3) on the same machine (default plan 0 against 2, roster 8 against 10); rules (b) and (c) hold everywhere, rule (a)
breaks once past the noise (roster, seed 1, kind 4 mass, broken but within the noise on `main`), the rest are rule
(d)'s rarer deaths, as on `main`.

### D2.4b: the context `(&World, &mut Domain)`

Level 2 (S72; the golden fingerprints are re-pinned, the same with and without threads; no change of the snapshot).
Body code takes the world read only and its domain writable; the domains are taken out of the world for the parallel
phase, and every serial step runs on the domain of its cell taken out the same way (`World::in_domain`). Lineages and
born rows are always provisional and the changes of holders always deferred, settled after the parallel phase and
after each serial step (outside a batch, the holders at once; in a batch, at its end); let-goes and anchors are part
acts. The island bench was remade with the code of `eec6f1d` (D2.4a and its review; `/tmp/d24b-bench`, 300 years).

**The same world.** Two years from that save: all 229 columns of `series.csv` but the timings, and `months.csv`, are
the same to the last digit (plant members 7,041,668,247.591711, bodies 3,630, updates 54,298, land 293.50516462337765
K, `air_3` 0.0004036156847595971, births 9,909,766,397.899576 in the second year, both builds). The two end saves
differ only in the ids of born rows: the store, the parts, the counters, the lineages and every body are the same, bit
for bit, but for its `genome` (1,927 of 3,630 bodies), and the rows each body was born with are the same. The ids
differ because outside a batch (the monthly merge, `nodes::crowd`) the changes of holders are now added after each
serial step: a merge's new born rows take a free slot before the rows it leaves with no holder are freed. No random
draw reads a lineage's or born rows' id, so the history does not depart. S72's level-3 parts, which change a history where they
happen (not in these two years): a body that holds a part and lets go of it or anchors it in its own update now does
so when its outbox is applied, after its update (and pays to anchor then); a body that dies or joins another while the
outboxes are applied, or in `wear` or `crowd`, lets go after the rest of that pass (in golden seed 2, month 16, a
`BondBroken` event moves after a later `Died` of the same cell and day: the same set of events), and a later part act
of the same pass could in principle find the part still held. A 16 x 16 world of seed 3 ends four years on the
same fingerprint in both builds.

Cost, callgrind on one thread, one year from that save (54,486 updates in both builds):

| | `eec6f1d` | D2.4b |
|---|---|---|
| the body batch (`survey::batch`, inclusive), per update | 168,902 | 170,150 (+0.74%) |
| of it: the updates (`survey::update`) | 8,208 M | 8,142 M (-0.81%) |
| of it: the outboxes applied (`outbox::deliver`; a serial step per entry) | 472 M | 561 M (+19%) |
| all instructions of the year | 13,930 M | 13,903 M (-0.20%) |

The updates are cheaper (the store's view by domain, no `opened` list); the serial phase costs more, as each of its
steps takes its domain out and settles it.

### D2.5: the domains on threads

Level 1 against D2.4b (`929d0ef`): the golden fingerprints do not move, with and without threads. The parallel phase
runs its domains on rayon's pool, one task a domain (`par::each`). The order test runs 30 batches on pools of 1 and 4
threads, in order and reversed, and the four worlds are the same (the saved world, events and counters). The three
benches were remade with D2.5's code (`BENCH_DIR=<scratch>/d25-bench`, saves of `929d0ef` plus D2.5). island22 gives the
numbers of D2.4b's save (3,679 bodies, 53,648 updates a year; 54,486 updates in the year under callgrind): D2.4b's
code and D2.5's make the same island in 300 years.

| Bench | s/yr | flow | change | bodies | parts | tape | alive | updates/yr | busiest domain |
|---|---|---|---|---|---|---|---|---|---|
| island22 | 1.187 | 137 | 51 | 971 | 3 | 25 | 3,679 | 53,648 | 0.146 |
| globe36 | 0.858 | 204 | 51 | 571 | 18 | 14 | 1,648 | 23,373 | 0.198 |
| region24 | 1.249 | 201 | 73 | 942 | 5 | 28 | 4,086 | 51,695 | 0.269 |

**The gain (S41, S65).** Wall time: 7 runs of 2 years from each save with `RAYON_NUM_THREADS=1` and with 4 threads,
interleaved in the same hour, the medians (the range in brackets). Cost per update: callgrind on one thread, one year,
the body batch inclusive (`survey::day` for `main`, `survey::batch` for D2.5) over the year's updates, each build on its
own remade save. The numbers of `main` (`5a04c54`, before D2.1) for island22 and globe36 are D2.1a's (the same
definition); region24 was remade with `main` for this row.

| | island22 | globe36 | region24 |
|---|---|---|---|
| bodies' stage (survey included), 1 thread, ms a year | 1,954 (1,845 to 2,000) | 982 (933 to 1,022) | 1,700 (1,674 to 1,728) |
| bodies' stage, 4 threads | 1,092 (1,043 to 1,121) | 589 (526 to 668) | 968 (911 to 1,046) |
| **gain of the threads** | **1.79** | **1.67** | **1.76** |
| whole year, 1 thread and 4 threads, ms | 2,208 and 1,321 (1.67) | 1,328 and 878 (1.51) | 2,076 and 1,271 (1.63) |
| busiest domain's share | 0.147 | 0.199 | 0.269 |
| `main` per update (updates in the year) | 147,068 (31,132; D2.1a) | 198,703 (26,353; D2.1a) | 146,560 (54,021) |
| D2.5 per update, 1 thread (updates in the year) | 169,747 (54,486) | 203,921 (23,155) | 152,783 (51,514) |
| overhead factor, `main` over D2.5 | 0.866 | 0.974 | 0.959 |
| **gain against `main`** | **1.55** | **1.63** | **1.69** |

island22 is under the kill line (1.6), globe36 and region24 between it and the target (2.2). The island's overhead
factor compares two worlds far apart (`main`'s island has 31,132 updates a year, D2.5's 54,486; S65 says such a
comparison measures the worlds). On one world the steps D2.1b to D2.4b each cost +0.4 to +0.9% an update (the rows
above): about +3% together, a factor of 0.97, which would give 1.74 on the island. The plan's kill decision comes after
D2.6 (S41), which puts the survey on threads.

**What limits it.** In instructions (the same callgrind runs), the parallel phase is 81.1%, 78.8% and 81.7% of the
bodies' stage (survey included), so four threads give at most 2.55, 2.46 and 2.58 times. In wall time, from a build
with timers around the phases (kept off `main`; one year, the mean of two runs):

| ms a year | island22, 1 thread | 4 threads | globe36, 1 | 4 | region24, 1 | 4 |
|---|---|---|---|---|---|---|
| bodies' stage | 1,956 | 1,074 | 1,018 | 566 | 1,724 | 1,025 |
| before the phases (the list, the faces) | 55 | 49 | 26 | 22 | 66 | 60 |
| the parallel phase | 1,490 | 605 | 761 | 318 | 1,317 | 613 |
| of it, the domains' work summed | 1,485 | 1,882 | 757 | 919 | 1,312 | 1,685 |
| of it, the busiest domain of each batch summed | 212 | 293 | 148 | 182 | 322 | 414 |
| the serial phase | 242 | 251 | 161 | 161 | 191 | 200 |
| the survey (and the rest of the stage) | 168 | 169 | 71 | 66 | 150 | 153 |

- The parallel phase is 2.46, 2.40 and 2.15 times faster on 4 threads.
- A domain's work costs 21 to 28% more when four threads run at once (the domains' work summed), with the same
  instructions: the threads share the memory and the last cache.
- The parts that stay on one thread (before the phases, the serial phase, the survey) are 44%, 44% and 40% of the
  stage on 4 threads.
- On region24 the busiest domain of each batch is 68% of the parallel phase on 4 threads.

**Ideas** (not built):

1. The largest domains first: sort the tasks by their number of updates before `par::each` (one line; the order changes
   nothing). With a build that does it, 5 interleaved runs on 4 threads: the parallel phase -7.0% (island22), -2.5%
   (globe36), -7.7% (region24); the bodies' stage -9.6%, +0.5%, -2.7% (noisy).
2. Less on one thread: D2.6 puts the survey on threads (16%, 12% and 15% of the stage on 4 threads); the serial phase
   (23%, 28%, 20%) is mostly the domains' moves booked and the outboxes applied (D2.4a): a domain's moves that touch
   only its own nodes could be booked by its own task.
3. A cheaper update on threads: the 21 to 28% more each domain costs on 4 threads; a fresh `Reactor`, `LiveTmp` and
   `ActTmp` per domain and batch, and the allocations of an update, are where to look first (a profile of cache misses,
   as E1's gate).

### D2.5b: the largest domains first

Level 1: the golden fingerprints do not move, with and without threads, and the bench gives the same bodies and updates.
The tasks of the parallel phase are sorted by their number of updates, largest first (ties by domain index), and the
pool's idle threads take them in that order (`par::each` hands the items out through `par_bridge`; before, rayon cut
the list in halves, so a large domain could start last). So no batch waits on a large domain started late (longest
processing time first, Graham 1969). With `reversed` (the order test) the sorted list is run backwards.

The bodies' stage on 4 threads, ms a year: 7 runs of 2 years from each D2.5 save, the three builds interleaved in the
same hour, the medians (the range in brackets). "Sorted only" sorts the list but keeps rayon's halving.

| | island22 | globe36 | region24 |
|---|---|---|---|
| D2.5 | 1,041 (963 to 1,087) | 577 (518 to 609) | 964 (866 to 1,088) |
| sorted only | 989 (970 to 1,083), -5.1% | 590 (541 to 638), +2.4% | 923 (860 to 1,095), -4.3% |
| **sorted, taken in order (kept)** | **959 (904 to 1,014), -7.9%** | **552 (474 to 594), -4.3%** | **836 (727 to 853), -13.3%** |

`scripts/bench.sh` (row above): island22 1.179 s/yr, globe36 0.801, region24 1.103 (bodies 956, 515 and 809 ms).

### D2.6: the survey on threads

Level 2 (golden re-pinned, the same with and without threads). The survey's wear of movers and its monthly merge run
per domain on the pool, through the same helper as the batch's parallel phase (`survey::on_domains`: the domains taken
out, the most work first, put back, their moves booked, the world settled, the outboxes applied), in the cells where no
part is and no body holds or is inside one; then those cells one by one, each a serial step (`World::in_domain`) with
its outbox applied after it. The shade runs per domain on the pool, the sums per cell (`par::rows`). Deaths and merges
change the holders of lineages and born rows by the domains' deltas, and the born rows a merge makes are provisional,
made global in domain order when the world settles (S72's mechanism, no second one). `counts.clamped_heat` is added
domain by domain in order (`World::book_moves`). The Draws of the air step and the marks of light stay serial (S73).
The order test runs a survey before each batch on pools of 1 and 4 threads, in order and reversed. Its first survey, at
a month's start, kills a body by wear in a cell with no part (a `Died` of cause `BROKEN`), kills a holder in its cell
with a part, which lets go of it (`BondBroken`), and merges two alike nodes with different born rows into new born rows;
the test asserts all three, and that the four runs end the same with the books kept.

**The same world.** The three golden worlds end with the same events in the same order, and the same store, parts,
counters, lineages and bodies, bit for bit, but for the ids of born rows (104 of 434 bodies in seed 1, 88 of 400 in seed
2, 71 of 304 in seed 3; each body's rows are the same): the slots a merge's rows leave free are now freed after the
domain's pass, so new rows take other slots. No random draw reads these ids. Two years from each D2.5 save give the
same `series.csv`, all 224 columns but the timings, to the last digit (island22: plant members 7,041,668,247.591711,
bodies 3,630, updates 54,298, `air_3` 0.0004036156847595971 in the second year; globe36 1,648 bodies; region24 3,997).

**The gain.** The bodies' stage on 4 threads, ms a year: 7 runs of 2 years from each save, the two builds interleaved
in the same hour, the medians (the range in brackets).

| | island22 | globe36 | region24 |
|---|---|---|---|
| D2.5b | 938 (856 to 996) | 543 (494 to 566) | 818 (746 to 899) |
| **D2.6** | **854 (768 to 896), -8.9%** | **506 (491 to 553), -6.9%** | **782 (711 to 832), -4.4%** |

The survey's instructions (`survey::survey` inclusive; callgrind, one thread, one year), D2.5b then D2.6: 800 and 801
million on island22, 315 and 331 on globe36, 651 and 655 on region24. They are the same but on globe36, which costs 5%
more: its survey does little work, and the two `on_domains` rounds of each survey (the domains taken out and put back,
their moves booked, the world settled, the outboxes applied) weigh more there. The gain is the threads.
`scripts/bench.sh` (row above): island22 1.062 s/yr, globe36 0.777, region24 1.080 (bodies 841, 493 and 768 ms).

### After D2.6: two steps not kept, and the gain (S41, S65)

**Scratch kept per domain, not kept.** Each domain kept its `Reactor`, `LiveTmp` and `ActTmp` from batch to batch (not
saved, empty in a copy, made again when absent). Level 1 (golden held). The bodies' stage on 4 threads, 7 interleaved
runs of 2 years, medians against D2.6: -1.6% (island22, 895 to 881 ms), +3.1% (globe36, 509 to 525), +3.9% (region24,
756 to 785): no gain above the noise. A fresh scratch costs little against a domain's batch.

**Booking in the task, not built.** Three measures:

- A build that books each domain's moves on the pool after the parallel phase (each `apply` writing the shared store
  through a raw pointer, the domains' numbers disjoint; a probe only, never for `main`) keeps golden but is slower on 4
  threads: +6.3%, +5.7%, +7.1% (7 interleaved runs). The store's arrays go by substance, then by cell, so the numbers
  of neighbouring domains share cache lines, and threads writing them fight over the lines.
- The whole booking after the parallel passes (batch and survey) takes 44, 21 and 46 ms a year on 4 threads (a probe
  build, 3 runs of one year): 5.3%, 4.2% and 5.9% of the bodies' stage. That is the most this step could save. Settling
  and applying the outboxes after it takes 150, 93 and 115 ms.
- A safe design must not write the store in the parallel phase (the others read it as it was at the batch's start): the
  task works out the booked numbers itself and the serial phase only copies them. The store's view would then sum as
  `apply` does (the rounding carried for the ground and the air, the numbers that fall below zero counted and set to
  zero, all of it undone with a checkpoint), the lift to the potential would stay a serial `apply`, and `book_moves`
  would copy each domain's view. About +30 core lines; the core had 9 left (S3). Not built (S74).

**The gain** (S41, S65; the same definitions and the same `main` numbers as D2.5's). Wall time: 7 runs of 2 years from
each D2.5 save with `RAYON_NUM_THREADS=1` and with 4 threads, interleaved in the same hour, the medians (the range in
brackets), build `2bbcf39` (D2.6). Cost per update: callgrind on one thread, one year, `survey::batch` inclusive over
the year's updates (the build of D2.6; D2.5's were 169,747, 203,921 and 152,783: the batch costs what it cost).

| | island22 | globe36 | region24 |
|---|---|---|---|
| bodies' stage (survey included), 1 thread, ms a year | 1,855 (1,786 to 1,959) | 980 (894 to 1,004) | 1,604 (1,585 to 1,791) |
| bodies' stage, 4 threads | 883 (846 to 977) | 535 (487 to 587) | 776 (714 to 800) |
| **gain of the threads** | **2.10** | **1.83** | **2.07** |
| whole year, 1 thread and 4 threads, ms | 2,100 and 1,105 (1.90) | 1,347 and 834 (1.61) | 1,960 and 1,088 (1.80) |
| busiest domain's share | 0.147 | 0.199 | 0.269 |
| `main` per update | 147,068 | 198,703 | 146,560 |
| D2.6 per update, 1 thread (updates in the year) | 169,753 (54,486) | 203,880 (23,155) | 152,795 (51,514) |
| overhead factor, `main` over D2.6 | 0.866 | 0.975 | 0.959 |
| **gain against `main`** | **1.82** | **1.79** | **1.98** |
| D2.5's gain against `main`, for comparison | 1.55 | 1.63 | 1.69 |

All three are above the kill line (1.6) and under the target (2.2); the headline for island22 is 1.82. (A side note:
with the island's overhead factor taken on one world, about 0.97 by D2.5's note, it would be about 2.04; S65 compares
the two remade benches, so 1.82 is the number that counts.)

**What is left on one thread** (a probe build timing the parallel calls, `par::each` in `on_domains` and `par::rows`
in `sums`; 3 runs of one year, the middle run):

| | island22 | globe36 | region24 |
|---|---|---|---|
| bodies' stage, 1 thread: in parallel calls, on one thread anyway (ms) | 1,707, 329 | 730, 190 | 1,501, 282 |
| share on one thread, 1 thread | 16% | 21% | 16% |
| bodies' stage, 4 threads: in parallel calls, on one thread (ms) | 590, 331 | 316, 210 | 511, 241 |
| **share on one thread, 4 threads** (after D2.5: 44%, 44%, 40%) | **36%** | **40%** | **32%** |
| the parallel calls, 4 threads against 1 | 2.9 | 2.3 | 2.9 |

The Amdahl ceiling of four threads from the shares on one thread: 2.70, 2.45 and 2.74. What stays on one thread is
mostly the serial phase of a batch: settling the world and applying the outboxes (150, 93 and 115 ms a year on 4
threads), the booking (44, 21, 46 ms), the faces and the list of updates, and the bodies that hold or are inside a part.

### After D2: profiles and the gates of E1, E3 and D1 (b)

At 5fa02b0 (the code of 485faa3), on D2.5's saves; callgrind one year on one thread; raw files in the scratch of that
run. Share of the simulation's instructions (load, save and pictures left out):

| Stage | island22 | globe36 | region24 |
|---|---|---|---|
| bodies: batch | 76.6% | 58.2% | 68.2% |
| bodies: survey | 6.6% | 4.1% | 5.7% |
| Flow | 10.8% | 27.6% | 17.2% |
| Change | 4.0% | 6.4% | 6.7% |
| parts | 0.2% | 2.0% | 0.3% |
| tape recorder | 1.7% | 1.7% | 2.0% |
| of the bodies' stage, on one thread | 12.9% | 18.6% | 12.6% |
| the batch's serial phase: `outbox::deliver`, `book_moves`, `settle` | 4.5%, 3.5%, 0.9% | 5.0%, 2.8%, 0.7% | 3.2%, 3.3%, 0.6% |

**E1 gate** (callgrind `--cache-sim=yes`, cycles estimated as Ir + 10 L1 misses + 100 LL misses): the body update
loses 21.8% (island22) and 22.2% (globe36) of its estimated cycles to misses, over the 10% of the gate. Of that, the
instruction cache is 12.0% and 11.9%, which columns do not change; the data cache is 9.7% and 10.3%; the last level
is 0.0% and 0.1%. The worst data misses are in `nodes::add`, `outbox::deliver` and `copy::young::mature`.

**E3 gate:** work an update of a body without a brain repeats with the same inputs (`Body::activity` about 11 calls
for 3 to 4 states, `Body::thermal`, `cover_of` and `depth`, the prologue of `plan`) is about 3% of the bodies'
instructions (range 2 to 5%; island22 300 million of 10,051), under the 10% of the gate.

**D1 (b) gate:** a sampler of the threads' states every millisecond (2 and 5 years from one save, the difference,
3 interleaved repeats on a quiet machine): with 4 threads, one thread runs alone 41.2% of a year (island22) and
53.2% (globe36), over the 25% of the gate. The physics that D1 (b) could hide behind the bodies (Flow, Change, parts,
tape) is 223 ms of a 1,079 ms year (21%) and 304 of 842 ms (36%); the rest of the time on one thread is the bodies'
own serial phase, which D1 (b) does not touch.

### E2.3: the repeated reads of one update

Judged on callgrind instructions of one year on one thread (S45; `RAYON_NUM_THREADS=1`) from the bench saves of the gates
(D2.5's), before at `f3c3c97`. The updates of the year are the same (54,486 and 23,155), and the world is the same bit
for bit (two years of the three benches: the series without its timing columns and the last snapshot hash equal; the
golden test holds with and without threads).

| | island22 before | island22 after | globe36 before | globe36 after |
|---|---|---|---|---|
| all instructions, million | 13,968.2 | 13,639.5 (-2.35%) | 9,169.5 | 9,036.8 (-1.45%) |
| `Body::activity` calls | 639,897 | 287,212 | | |
| `cover_of` calls | 303,374 | 178,367 | | |
| `Body::thermal` calls | 745,171 | 703,584 | | |
| `Reaction::clone` calls | 171,588 | 0 | | |

What changed:

- A `Payer` (a body about to pay, valid until its stores or heat change) holds the body's activity (`Payer::dry`) and is
  made with the cover of other bodies over it. The update reads its activity once as it begins (the first `buildable`,
  the upkeep, both plans of `settle_upkeep`, and the payer), once after the pay (`react`), once for `grow` (its
  allowance, its payer and the upkeep and plan of `convert`; `convert` keeps the payer while no conversion changes the
  body), and once for the last `buildable`: 4 reads where there were about 11. The cover is read once in `site`
  and given to the payers of `settle_upkeep`, `keep_warm` and `grow` (others do not change in an update).
- `starved` works out the plateau state only if it loses a share, so `shed` no longer repeats it.
- `grow` no longer clones `Chemistry::build` and `builds` (171,588 reaction copies a year, with their lists): the world
  is only read there. This part alone is 97 million (0.69%) on the island and 42 million (0.45%) on the globe; the rest
  of the step gives 234 million (1.67%) and 92 million (1.01%).

Not done: `visible` repeats the plateau state of the end of `shed` in about 90% of updates, but between the two
the body goes through `select`, `mature` and `split`, which can change it; knowing when it has not would be a cache of
the state (S31). The depth of liquid over a body is the same two comparisons on dry land, and in `Payer::at` it is
read again at the mass of that moment (a body afloat changes with its mass), so it is not given along. The prologue and
the burnings' rates of `plan` are repeated in its second call (an estimate of about 70 million, 0.5%) and would need arrays in
`Payer`; they are left for a later spot.

### E2.3: the recorder on the pool, and PGO again (both not kept)

Both were measured on 4 threads, 7 runs of 5 years from the saves of the profile after D2, the four builds interleaved
(`main`, the recorder build, `main` without PGO, `main` with PGO) on a machine with no other job (a run was made again
when another process used the cores); the medians, the range in brackets. "Year" is the sum of the five stage times.
The results of all builds are the same: the same world hash, and for the recorder build the same `series.csv` (all
columns but the timings), `months.csv` and viewer file for 2 years of both benches with a frame every month.

**The recorder on the pool (`tape/` only, level 1, branch `speed/e23-tape-not-kept`).** The monthly pass over the
bodies (a row of numbers for each body made on the pool with `par::rows`, then summed in the order of `Life::alive`), the
yearly read of each body (mass, size, activity, life, matter, brood, learned offset: one `Seen` for each body, read
once, then summed by the rows of the series) and of each ground cell (`layers::fields`) run on the pool. Every sum keeps
its order, so nothing changes. The land cells come once from `Fields::z` (they were found by `is_land`, which works out
`surface_z`, three more times in a yearly row).

| | island22 | globe36 |
|---|---|---|
| tape stage, `main`, ms a year | 27.9 (24.0 to 32.4) | 14.5 (13.4 to 16.6) |
| tape stage, recorder on the pool | 20.1 (18.7 to 22.4) | 11.9 (10.9 to 12.4) |
| **gain** | **7.8 ms, 0.7% of the year** | **2.6 ms, 0.3% of the year** |
| year, `main` and recorder on the pool | 1,136 (1,032 to 1,177) and 1,096 (1,016 to 1,201) | 828 (797 to 878) and 826 (794 to 880) |
| the tape stage, share of the year | 2.5% | 1.8% |

Under the kill line (1% of a 4-thread year on both): not kept. The year's own spread (5 to 10%) hides these gains; the
tape stage column is the fair one. The recorder is 2.5% and 1.8% of the year, so a gain of 1% would have to take away 40%
and 57% of it; even a recorder that cost nothing in its event counts (2 to 3 ms a year, one thread) would not have
reached it on globe36 (0.3% + 0.3%).

Why it does not go further, from timers inside a probe build and callgrind (one thread, one year):

- The reading of a body is little work (about 54 instructions for the monthly row of a body) but many misses: a body's
  genes, signals and known bodies are in separate allocations. With 4 threads the monthly pass of island22 fell from
  about 1.2 to 0.45 ms, and that of globe36 (1,600 bodies) from 0.42 to 0.29 ms: a call into the pool costs 30 to 55 µs
  (D3), a large part of 0.3 ms.
- What stays on one thread: the sums in order (about 1.6 ms a year on island22), the event counts (`Recorder::count`,
  130,000 events a year, 62,000 of them deaths, 3 ms a year on island22, 2.2 on globe36; the tallies are floats summed
  in event order and `Took` fills a `BTreeMap` of the snapshot, so they cannot be split without a level 2 change),
  the row's other columns and `finish`.
- Found on the way: the first record after a resume works out `surface_z` for every body (`Life::env` is empty until
  the first survey), 45 million instructions once; the file writing (`finish`) is called twice at the end of a run
  (`cli/src/main.rs`, once in the snapshot block, once after the loop): neither is in the bench's tape stage.

**PGO again** on the code after D2 (S35's bar to keep it is 8%; the flow of A2: `-Cprofile-generate` on a build of
`main`, 2 years of island22, globe36 and region24 from the saves, `llvm-profdata merge` (the one of the toolchain),
`-Cprofile-use` on the same code; the control is the same code built the same way without it). The year on 4 threads:

| | island22 | globe36 |
|---|---|---|
| without PGO, ms | 1,080 (1,045 to 1,267) | 836 (793 to 902) |
| with PGO, ms | 1,068 (1,027 to 1,130) | 804 (767 to 886) |
| **gain** | **1.1%** | **3.8%** |

Under the bar on both (and under the 5% and 4% of A2, which were on one thread and the older code): not kept. The
instruction cache loses 12% of the update's cycles, but the profile does not turn that into a gain above 4%. S35 stands:
no PGO, no `scripts/pgo.sh`.

### E2.3 (1): the booking once per number

A domain's view keeps beside each number its change since it was taken from the store; the booking adds that change
to the store once (`Moves::book`, with `Store::carry` for the ground's and the air's amounts, as `apply` adds), in
place of one `apply` over the list of the domain's moves. The list goes. Callgrind, one year on one thread, D2.5's
saves, before at f3c3c97 (million instructions):

| | island22 before | island22 after | globe36 before | globe36 after |
|---|---|---|---|---|
| all instructions | 13,967.6 | 13,546.1 (-3.02%) | 9,169.6 | 8,983.8 (-2.03%) |
| body updates in the year | 54,486 | 54,382 | 23,155 | 23,190 |
| booking (`World::book_moves`, inclusive, serial) | 417.8 | 70.1 | 222.4 | 50.8 |
| gathering (`Moves::gather`, inclusive, on the pool) | 530.7 | 491.6 | 244.5 | 226.3 |
| the batch (`survey::batch`) per body update | 169,753 | 163,483 (-3.7%) | 203,844 | 197,404 (-3.2%) |

The other rounding changes the history, so a little of the total is not the step's (the tape's compression -24 and
+4 million). Over the kill line of 2% on both benches. The bodies' stage on 4 threads, 6 interleaved runs of 3 years
each on a quiet machine (`ms_bodies`, median, min to max): island22 840.9 (777 to 902) -> 795.6 (732 to 820) ms a
year, -5.4%; globe36 508.8 (469 to 523) -> 483.8 (460 to 502), -4.9%. Level 2: golden re-pinned. Two years from the
saves against f3c3c97: `t_land` within 4e-5, `t_air` within 4e-5, the bodies within 1.1%, the updates within 0.7%,
the rain within 2e-4 (relative); the elements are kept to 1e-16. The order test's scenario cells moved (the old ones
no longer gave a dispersing litter that lands in another cell with this history).

## Profile of the baseline

One year from the same saves under `valgrind --tool=callgrind` (`scripts/bench.sh profile island22` and `... globe36`),
at `16e6a4e`. Shares are of all instructions in that run. The run also loads the save at the start and writes a save
and the tape at the end; that is about 10 to 15% of the instructions (in the tables: the miniz, bincode and
`serialize` rows) and is not part of the bench time. Inclusive shares of nested functions overlap, so they do not add
up. Flow and Change run on worker threads (the rayon rows); callgrind counts the instructions of all threads.

**island22: 6.654 billion instructions in one year**

| Top by exclusive instructions | Share | Top by inclusive instructions | Share |
|---|---|---|---|
| `miniz_oxide::deflate::compress_inner` (save) | 2.69% | `ew::drive` | 86.31% |
| `pow` (libm) | 2.37% | `schedule::run_for` | 78.51% |
| `change::kernel::Reactor::react` | 2.36% | `survey::day` (bodies) | 56.03% |
| `store::apply::apply` closure 3 | 1.78% | `body::live::live` | 39.74% |
| `body::genes::vary` (gene draw) | 1.65% | `body::live::to_growth` | 32.98% |
| `store::apply::apply` closure 3, inner | 1.54% | `body::physics::physics` | 16.49% |
| `exp` (libm, `__ieee754_exp_fma`) | 1.47% | rayon worker threads (Flow, Change) | 10.69% |
| slice indexing inside `Reactor::react` | 1.24% | `Reactor::react` | 8.95% |
| `expm1` (libm) | 1.19% | `copy::copy` | 8.25% |
| `nodes::crowd` | 1.19% | `store::apply::apply` | 7.85% |
| `memcpy` (libc) | 1.15% | `flow::air` | 7.62% |
| `free` (`_int_free`, libc) | 1.13% | `survey::survey` | 7.23% |
| `store::apply::apply` | 1.07% | `copy::select::sample` | 6.39% |
| `miniz_oxide::deflate::flush_block` (save) | 1.01% | `copy::grow::grow` | 6.10% |
| `nodes::add` fold | 0.95% | `nodes::crowd` | 6.02% |

**globe36: 10.942 billion instructions in one year**

| Top by exclusive instructions | Share | Top by inclusive instructions | Share |
|---|---|---|---|
| `miniz_oxide::deflate::compress_inner` (save) | 4.18% | `ew::drive` | 83.80% |
| `change::kernel::Reactor::react` | 2.38% | `schedule::run_for` | 74.32% |
| `memcpy` (libc) | 1.88% | `survey::day` (bodies) | 52.83% |
| `pow` (libm) | 1.81% | `body::live::live` | 25.26% |
| `miniz_oxide::deflate::flush_block` (save) | 1.60% | `body::live::to_growth` | 18.94% |
| `store::apply::apply` closure 3, inner | 1.36% | `choose::act` | 18.92% |
| `miniz_oxide::inflate::decompress_fast` (load) | 1.32% | `choose::spread::spread` | 15.44% |
| `store::apply::apply` closure 3 | 1.30% | rayon worker threads (Flow, Change) | 12.32% |
| `expm1` (libm) | 1.24% | `Reactor::react` | 8.78% |
| `flow::air::step::step` | 1.23% | `body::physics::physics` | 8.30% |
| slice indexing inside `Reactor::react` | 1.22% | `nodes::add` | 7.99% |
| `exp` (libm, `__ieee754_exp_fma`) | 1.18% | `flow::air` | 7.85% |
| `free` (`_int_free`, libc) | 1.03% | `World::serialize` (save) | 6.50% |
| `memset` (libc) | 1.01% | `nodes::take_in` | 6.47% |
| `body::genes::vary` (gene draw) | 0.93% | `store::apply::apply` | 6.06% |

## Where the time goes

- Bodies are about three quarters of a year: 638 of 857 ms on the island, 1,010 of 1,347 ms on the globe. Flow is
  about 18 and 17%; Change about 6 and 4%; the parts and the tape are small.
- The profile is flat. No single function holds more than about 2.5% of the instructions apart from the save. The
  weight is spread over the whole body update (`live`, with growth and physics inside it) and the copy and spread
  work around it.
- On the island, `to_growth` alone is a third of the instructions and `physics` another sixth. On the globe, the
  choice of what to do (`choose::act`, mostly `spread`) is a fifth. `nodes::add` (8%) and `nodes::take_in` (6%), the
  work of moving a body into and out of nodes, come next.
- A body update costs about 23 µs on the island (638 ms for 27,762 updates) and about 39 µs on the globe. That is the
  number B, C and E have to cut.
- The libm calls (`pow`, `exp`, `expm1`) together hold about 5% of the instructions, a small but easy target for a
  faster compiler setting or a cheaper formula.

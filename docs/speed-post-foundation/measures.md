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

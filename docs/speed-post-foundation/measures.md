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

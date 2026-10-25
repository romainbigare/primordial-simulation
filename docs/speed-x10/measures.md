# Speed x10: measures

What each step of the round measured and gained. The plan is `plan.md`; the designs are in `design/`. One section for
each step that measures. The formats follow `docs/speed-post-foundation/measures.md`.

## R2: the gates at the branch's start

R2 measures every gate of the plan on one set of benches, with the code of `speed/x10` at `4cc7b81` (R1 changed rules
and docs only, so this is the code of the last round's Z1). No law and no line of `core` changed. The counters and
timers ran on throw-away branches that are never merged (see "How the numbers were taken").

### The benches

Made again with this code in 14 minutes (11:39 to 11:53), with the commands of `scripts/bench.sh` (`ew run` with
`--monthly-years 0 --frames-every 100000 --every 100000`, then the last snapshot kept), snapshot version 47. They live
outside the repository, for the later steps to use:

`/tmp/claude-0/-home-user-primordial-simulation/f8e69c10-3d92-5fa9-8e05-bbc4e367226f/scratchpad/x10/r2-bench`
(`island22.bin.gz`, `globe36.bin.gz`, `region24.bin.gz`, each with its `.stamp`). Run them with `ew resume --from <file>
--years N --out <dir>`, as the script does (`BENCH_DIR` set to that folder).

| Bench | Made with | Saved at year | Hash of the world at that year | Bodies in the save (plant nodes, animal nodes and individuals) |
|---|---|---|---|---|
| island22 | `--size 22 --seed 1 --years 300` | 300 | `3099d973a77cb4d9` | 1,974 and 376 |
| globe36 | `--globe --size 18 --seed 1 --years 200` | 200 | `e2e09f65ee242973` | 1,083 and 434 |
| region24 | `--size 24 --seed 1 --region 8,8,8,8 --years 200` | 200 | `efd25fa63c04ec30` | 4,737 and 185 |

These are the same worlds as Z1's: one year of island22 and of globe36 costs 7,951,587,962 and 8,281,669,234
instructions here, the same numbers as Z1 (`docs/speed-post-foundation/measures.md`, section Z), and the updates and
bodies of a year are the same (32,499 and 22,847 updates). So Z1's numbers and these can be set side by side. Only the
machine differs (below).

The raw outputs (logs, `series.csv` files, callgrind files, the key-number log) are small and stay beside the benches, in
`.../scratchpad/x10/raw` (`raw/keys.log` is the key-number test). They are not in the repository.

### How the numbers were taken

- **Wall time.** `ew resume --years 5` from the bench saves, the mean of the 5 years, 5 sets in turns (each set: every
  bench, 4 threads then 1 thread; `RAYON_NUM_THREADS`), medians of the 5. Every timed run ran under the lock (`flock
  timing.lock`) and was thrown away and made again if a foreign `cargo`, `rustc`, `valgrind` or `ew` started during it.
  The machine has 4 cores. **It was not quiet:** other lanes ran their tests beside it. The table below is the second
  made set of 5 sets (no run thrown away; mean load before a run 1.6). A first set of 5 sets, made an hour earlier
  with 12 runs thrown away and a mean load of 1.9, was 8% to 20% slower and wider (island22 4T 0.738 s, globe36 0.892
  s, region24 1.119 s; 1T 1.390, 1.438, 1.907 s); it is not used. Read the wall times with the Z1 column beside them.
- **Instructions.** `valgrind --tool=callgrind`, one year, one thread, from the same saves. The shares are of
  `schedule::run_for` (the simulation); the loading and saving of the save is left out, as in Z1. The details (plants
  and animals apart, `choose::act`, the serial phase, Flow) come from a build where those functions are
  `#[inline(never)]` (branch `r2-split`, +0.3% instructions); the totals above are from the clean build.
- **Counters** (branch `r2-probe`, `EW_PROBE=counts`): one thread, 3 years from the save; the first year is not
  counted (the habit of MI0 needs a year to be known); counts are of the 2 years after it, per year or per batch.
  The world is not changed by them (the habit and the counts are on the side).
- **Timers** (branch `r2-probe`, `EW_PROBE=timers`): the same, with clocks around the parts. 3 sets in turns, medians,
  on 1, 2, 3 and 4 threads. The clocks cost a few percent (not measured apart).
- The branches: `r2-probe` (d762ba0, from `r2`) holds `core/src/probe.rs` and the hooks of the counters and timers;
  `r2-split` (600290e) holds only `#[inline(never)]` marks and two update wrappers. Neither is merged. What each hook
  measured is said in the sections below.

### The year on this machine

Medians of 5 sets in turns, ms a year (the total is the median of the totals; the range of the totals in brackets).
The last two columns set them against Z1 (a quiet hour).

| Threads | Bench | s/yr | flow | change | bodies | parts | tape | alive | updates/yr | busiest domain | Z1 s/yr | median / Z1 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 4 | island22 | 0.679 (0.660 to 0.712) | 141 | 51 | 470 | 3 | 16 | 2,361 | 32,499 | 0.157 | 0.614 | 1.11 |
| 4 | globe36 | 0.841 (0.787 to 0.853) | 211 | 52 | 521 | 38 | 15 | 1,542 | 22,847 | 0.189 | 0.738 | 1.14 |
| 4 | region24 | 0.921 (0.903 to 1.073) | 209 | 72 | 607 | 9 | 29 | 4,857 | 52,839 | 0.305 | 0.888 | 1.04 |
| 1 | island22 | 1.222 (1.181 to 1.312) | 160 | 58 | 985 | 3 | 16 | 2,361 | 32,499 | 0.157 | 1.072 | 1.14 |
| 1 | globe36 | 1.299 (1.160 to 1.336) | 250 | 60 | 938 | 36 | 15 | 1,542 | 22,847 | 0.189 | 1.183 | 1.10 |
| 1 | region24 | 1.751 (1.633 to 1.785) | 270 | 95 | 1,345 | 10 | 32 | 4,857 | 52,839 | 0.305 | 1.589 | 1.10 |

The machine ran 4% to 14% slower than in Z1's hour. The best of the 5 sets is within 2% to 7% of Z1 at 4 threads. The
goal's measure (a year on 4 threads, 0.091 s island, 0.149 s globe) is therefore still 7.5 times (island) and 5.6
times (globe) away on this machine, against Z1's 6.7 and 5.0. The bodies' stage per update (wall): 14.5, 22.8 and 11.5
µs on 4 threads; 30.3, 41.1 and 25.5 µs on 1 thread.

### The year in instructions (callgrind, one thread)

Million instructions of `schedule::run_for`: island22 6,731, globe36 7,163, region24 9,921 (whole runs 7,976, 8,301
and 11,425 with the load and save of the file). Shares of `run_for`. These agree with Z1's profile to a tenth of a
point (bodies' batches 68.3 / 58.0 / 62.2%, `choose::act` 11.6 / 17.8 / 3.5%).

| Stage | island22 | globe36 | region24 |
|---|---|---|---|
| bodies: the batches (`survey::batch`) | 68.4% | 58.0% | 62.2% |
| of it: plant updates | 30.0% | 14.7% | 48.0% |
| of it: animal updates (serial bodies with parts: 0.3 / 1.1 / 0.0) | 32.2% | 35.9% | 8.7% |
| of it: `outbox::deliver` | 3.4% | 5.0% | 3.0% |
| of it: `book_moves`, `settle`, `Faces::make` | 0.7, 0.4, 0.5% | 0.5, 0.6, 0.3% | 0.7, 0.3, 0.7% |
| of it: the list of due bodies, sorting, the rest | 1.5% | 2.1% | 0.9% |
| bodies: the survey (`survey::survey`) | 5.8% | 4.2% | 8.5% |
| Flow: the air (`flow::air`) | 13.6% | 22.0% | 15.3% |
| Flow: the liquid | 4.0% | 5.3% | 4.4% |
| Flow: the state outside the survey, creep, refresh | 1.1% | 1.4% | 1.2% |
| Change | 7.1% | 6.7% | 7.8% |
| parts (Bond's month) | 0.3% | 2.9% | 1.0% |
| the tape's recorder | 1.7% | 1.5% | 1.8% |

Inside the updates (shares of `run_for`): the body's day `body::live::to_growth` 31.4 / 20.3 / 36.1%, of it
`Reactor::react` 10.0 / 9.3 / 11.7%; `choose::act` 11.8 / 17.8 / 3.6%; `copy::copy` 8.1 / 5.3 / 6.3%;
`copy::grow::grow` 6.2 / 4.3 / 6.5%; `nodes::add` 6.3 / 5.1 / 4.3%; `copy::young::mature` 4.4 / 3.1 / 2.9%;
`nodes::merge` 3.1 / 3.3 / 1.8%; `store::apply` (everything that books) 4.7 / 5.2 / 5.2%.

**Inside a plant update** (island22 / globe36, share of the update): the body's day (`live`) 77% / 77%; the sample member
(`copy::select::sample`, a second body day for a node without a brain) 13% / 13%; maturing (`copy::young::mature`)
5.5% / 5.8%; the rest 4% / 4%. **Inside an animal update:** the body's day 52% / 42%; the decision 37% / 50%; maturing
8.5% / 6.3%; the rest 2.5%.

### P0: plants and animals

**Kill line: if plants are under 20% of island22's year on one thread, P3 is not built. Verdict: P3 stays.** Plant
updates alone are 30.0% of the island's year in instructions. All included (plant updates; their share of faces, of
the numbers changed in the booking, of the outbox litters; and their bodies' share of the survey) the plants are about
37% (island22), 19% (globe36) and 60% (region24). The plan's 38% and 21% stand.

| | island22 | globe36 | region24 |
|---|---|---|---|
| plant updates, share of a year (instructions, 1 thread) | 30.0% | 14.7% | 48.0% |
| animal updates | 32.2% | 35.9% | 8.7% |
| plants, all included | about 37% | about 19% | about 60% |
| plants' share of the updates (counts) | 62.9% | 47.3% | 89.6% |
| plants' share of the faces made | 87.0% | 76.9% | 96.9% |
| plants' share of the number changes in the booking views | 63.7% | 45.5% | 89.7% |
| plants' share of the litters in the outbox | 62.1% | 56.4% | 94.4% |
| plant update, 1 thread / 4 threads, µs (timers) | 18.0 / 21.9 | 16.7 / 21.8 | 16.8 / 20.9 |
| thread time of plant updates a year, 4 threads, ms | 456 | 234 | 992 |
| stands (plant nodes) per cell that has plants: mean, most | 8.2, 26 | 4.6, 14 | 10.3, 28 |
| plant lineages in the world / per cell | 1 / 1.0 | 1 / 1.0 | 1 / 1.0 |
| young and adult plant nodes | 1,315 and 659 | 706 and 377 | 3,144 and 1,593 |

In the reactor, per call of a body's reactions (the calls with catalyst factors): reactions with all their inputs
11.7 / 10.8 / 11.5 for plants (13.8 / 13.2 / 13.8 looked at); **the share that ran with no catalyst (the bare rate):
89.9% / 90.4% / 90.8%**. Animals: 12.2 / 12.0 / 13.1, all bare (100%: their reactor work is the burnings on demand).
One sub-step a call. So nine in ten of the reactions that run in a plant are bare ones; they are the ones the month law
would not run in stands (plants 2.3, stage 5).

**`|T_plant - T_ground|`** (a plant node's temperature at the start of its update, less its ground's at the last Flow
step; counted at every plant update of 2 years):

| | island22 | globe36 | region24 |
|---|---|---|---|
| mean of the absolute value, K | 3.69 | 3.84 | 3.37 |
| signed mean, K | +0.74 | +1.29 | +0.65 |
| updates over 1 K / 3 K / 5 K | 84.6 / 54.5 / 25.8% | 81.6 / 50.0 / 28.1% | 84.7 / 50.7 / 21.3% |

On the saved day (one date, node by node, as the save holds them) the plants were 2.1 K (island), 3.1 K (globe) and 2.2
K (region) from their ground on average, and over 3 K in 26%, 39% and 24% of the nodes (10%, 12%, 23% by members). So
over a plant's interval the difference is often above 3 K, in half of the updates: **P1.3's rule "if it is often over 3
K, stage 6 keeps the old relaxation" is met.** The plan's P1.3 (stands at their ground's temperature) must say how it
pays this difference (the heat of 3.7 K on a stand's heat capacity, booked with the ground), and its gate "a climate
test or a key number fails" is the check.

**A plant's liquid time `1 / (LIQ x wetness)` against a tenth of the interval** (counted at every plant update): the
time is over a tenth of the plant's own interval in 27.9% (island), 18.2% (globe) and 24.9% (region) of the updates;
over a tenth of 30 days in 30.1 / 22.6 / 27.3%, of 36 days 27.9 / 18.2 / 24.9%, of 60 days 23.7 / 12.2 / 21.0%, of 90
days 21.4 / 11.5 / 18.8%. The geometric mean of the time is 3.0, 4.2 and 2.7 days. So the closed form of the mean of the
exponential approach is needed in about a quarter of the stand updates (a fifth at 90 days).

**A plant's stock over 36 days** (the energy its burnings can release, against the reserve of 36 days of upkeep,
`R_res`; the straight line of the update's own net change carried to 36 days): below `R_res` at the start of the update
in 17.5 / 14.4 / 17.5% of the updates; empty at the start in 0% (none); its net change is negative in 71 / 74 / 77%; the
line hits zero within 36 days in 0.5 / 1.4 / 0.6%; it crosses `R_res` in 10.1 / 10.0 / 10.3% (the stock is far above
its reserve in most: the mean of log10(stock / `R_res`) is +0.81, +0.85, +0.84). So phase 3 (an empty stock and deaths
by the rate) is rare; phase 2 (growth at the reserve) is the case for one update in six.

### E0: the engine

**(a) Outbox entries per batch by kind, and which would be S2** (a year has 180 batches; the entries applied in the
batches; the survey's own entries are few: 68 acts a year on the island, 332 on the globe, 0 on the region):

| Per batch | island22 | globe36 | region24 |
|---|---|---|---|
| all entries | 52.7 | 167.4 | 70.4 |
| moves (`Go`) | 0.9 | 7.5 | 7.3 |
| litters (children in another cell) | 41.2 | 27.0 | 59.9 |
| watches | 5.6 | 85.8 | 2.7 |
| part acts (`Act`; of them let-go or torn: 0.38 / 1.85 / 0.00) | 5.0 | 47.1 | 0.5 |
| **S2 candidates**: part acts + watches + litters that go back to the parent's brood or the ground (2.13 / 0.35 / 3.79) + moves into a cell with parts (0.02 / 1.50 / 0.78) | 12.7 | 134.7 | 7.8 |
| S2 candidates with the lane clears (a move that clears a lane: 82 / 75 / 78% of moves) | 13.5 (25% of the entries and the clears) | 140.4 (81%) | 13.4 (18%) |
| moves that stay in their domain | 68.5% | 86.3% | 78.3% |
| litters that start in their parent's domain | 64.5% | 69.3% | 80.6% |
| litters that join a node or start a body there (the rest go back) | 94.8% | 98.7% | 93.7% |

The cost of the entries is not by count. In instructions the litters are most of `outbox::deliver` (`copy::start` and
`copy::kin`: 78% of it on the island, 31% on the globe, 81% on the region); on the globe `bond::verbs::act` (14%),
`Body::visible` (12%) and `nodes::merge` (9%) follow.

**(b) Watches and part acts per batch:** in the table: 5.6 and 5.0 (island), 85.8 and 47.1 (globe), 2.7 and 0.5
(region).

**(c) Share of updates by serial bodies** (bodies that hold or are inside a part, S52): **0.5% (island22), 2.1%
(globe36), 0.0% (region24)** of the updates (158, 485 and 1 a year); in instructions 0.3%, 1.1% and 0.0% of a year. As a
share of the animal updates: 1.3%, 4.1%, 0.0%. Time on 4 threads (timers): the serial bodies take 7.4, 24.0 and 1.0 ms a
year. (The design said 8.5% of the updates on the globe at year 200; these worlds have 2.1%.) **E5's gate (serial-body
updates at least 10% on a bench, or S2 at least 15% of the 4-thread bodies' stage): not met today.** An upper bound of
S2's time is `deliver` plus the serial bodies, 14.8% (island), 25.0% (globe) and 12.6% (region) of the bodies' stage on
4 threads; the time of the S2 kinds alone was not measured. The gate is measured again after the plants and G2 (X23).

**(d) The busiest share of a batch**, as `Counters::busiest_share` counts it (the most updates of one task over the
updates of the batch; the mean over the batches; the bench column `busiest domain` reproduces: 0.157 / 0.189 / 0.305),
and the share the busiest of 4 threads would have with the longest task first (LPT, 4 threads; the ideal is 0.25).
Tiles are 1, 4 and 16 consecutive cells of a domain's cell list. Cost-weighted: an animal update counts 2.2 plant
updates (the measured ratio of their times is 2.1, 2.5 and 2.2).

| | island22 | globe36 | region24 |
|---|---|---|---|
| domains: busiest share, tasks a batch | 0.160, 16.6 | 0.188, 16.5 | 0.302, 12.5 |
| tiles of 16: busiest share | 0.160 | 0.188 | 0.155 |
| **tiles of 4: busiest share, tasks a batch** | **0.069**, 40.4 | **0.102**, 32.2 | **0.077**, 54.5 |
| tiles of 1: busiest share, tasks a batch | 0.036, 76.1 | 0.060, 57.2 | 0.039, 119.8 |
| **tiles of 4 over domains** (gate: at most 0.6 on two benches) | **0.43** | **0.55** | **0.26** |
| LPT-4 share, domains / tiles of 4 | 0.2557 / 0.2535 | 0.2590 / 0.2548 | 0.3158 / 0.2537 |
| the same, cost-weighted | 0.2559 / 0.2533 | 0.2587 / 0.2535 | 0.3133 / 0.2529 |

**E2's gate is met on all three benches.** The 4-thread balance, though, is already close to the ideal by counts:
domains cost 2.3% (island) and 3.6% (globe) over 0.25, and tiles of 4 cost 1.4% and 1.9%, so tiles take only 0.9%
(island) and 1.6% (globe) off the parallel phase there (3 and 4 ms a year); on region24 the domains are 26% over the
ideal, tiles of 4 are 1.5% over, and tiles take 20% off its parallel phase (about 78 ms of its 397 ms). The gain of tiles on island22 and globe36 at 4 threads is in the other things they change
(smaller tasks, more threads), not in the balance.

**(e) The bodies' stage on 4 threads and on 1** (timers; wall ms a year; medians of 3 sets):

| ms a year | island22 4T | 1T | globe36 4T | 1T | region24 4T | 1T |
|---|---|---|---|---|---|---|
| P1 (the parallel phase of the batches, `par::each`) | 306 | 854 | 268 | 673 | 397 | 1,023 |
| booking (`book_moves`) | 11.2 | 10.4 | 10.5 | 7.7 | 15.7 | 13.8 |
| settle | 8.7 | 7.0 | 5.6 | 4.1 | 9.3 | 8.2 |
| deliver (`outbox::deliver`) | 66.8 | 64.8 | 102.9 | 91.7 | 85.1 | 79.9 |
| faces (`Faces::make`) | 22.3 | 24.0 | 15.0 | 14.7 | 48.8 | 55.5 |
| the list of due bodies (take, find, sort) | 10.0 | 10.2 | 8.2 | 7.9 | 15.8 | 16.6 |
| serial bodies | 7.4 | 6.8 | 24.0 | 21.5 | 1.0 | 0.9 |
| tail (settle, `settle_moved`, clear) | 8.9 | 8.3 | 10.7 | 9.0 | 6.3 | 5.8 |
| **the batches' serial parts (all but P1)** | **135** | 132 | **177** | 157 | **182** | 181 |
| the survey (total) | 55 | 87 | 57 | 60 | 95 | 183 |
| of it the parallel calls (wear and merge, shade) | 36 | 64 | 22 | 27 | 63 | 140 |
| of it the serial cells (cells with parts, one by one) | 4.6 | 4.5 | 22.6 | 20.0 | 10.7 | 9.2 |
| of it sums, state, cells, book/settle/deliver | 15 | 19 | 12 | 13 | 21 | 34 |

The serial parts do not shrink with threads (they are the same on 1 thread): on 4 threads they are 27% (island), 35%
(globe) and 27% (region) of the bodies' stage, and 19%, 22% and 18% of the year. `deliver` alone is 66.8 / 102.9 /
85.1 ms.

**(f) A domain's work on 1, 2, 3 and 4 threads (c(P))**, the thread time of an update against its time on one thread
(timers; the same updates, the same world):

| | island22 plants / animals | globe36 plants / animals | region24 plants / animals |
|---|---|---|---|
| 1 thread, µs | 18.0 / 37.9 | 16.7 / 41.8 | 16.8 / 36.4 |
| 2 threads | 1.05 / 1.04 | 1.18 / 1.20 | 1.16 / 1.12 |
| 3 threads | 1.11 / 1.10 | 1.25 / 1.30 | 1.24 / 1.24 |
| 4 threads | 1.22 / 1.25 | 1.31 / 1.31 | 1.24 / 1.22 |
| P1 wall, 1 -> 4 threads: speed-up | 2.79 | 2.51 | 2.58 |

G15's target (an update on 4 threads at most 1.2 times its cost on 1 thread) is 0.02 to 0.11 away. The four-worlds test
(G0) says how much of it the machine owns.

**E4's gate** (`store::apply` on the main thread at least 3% of a year at 4 threads, after the air section): today the
four booking pieces on one thread (Change's booking, the air's `book`, the liquid's `book`, `book_moves`) are 22.3 +
22.9 + 8.9 + 11.2 = 65 ms (island, 9.0% of its 4-thread year of 722 ms) and 22.2 + 28.3 + 11.4 + 10.5 = 72 ms (globe,
8.9% of 812 ms). In instructions `store::apply` is 4.7 / 5.2 / 5.2% of a year.

### AIR0: the air

**The table of 1.4**, wall ms a year (timers; 1 thread, then 4 threads):

| Part | island22 1T | 4T | globe36 1T | 4T | region24 1T | 4T | runs on |
|---|---|---|---|---|---|---|---|
| `pass` (cell rows) | 61.9 | 31.5 | 97.1 | 51.2 | 95.3 | 42.4 | pool |
| `book` (the air's booking) | 22.2 | 22.9 | 26.0 | 28.3 | 34.3 | 38.0 | one core |
| `gather` | 9.1 | 9.2 | 14.8 | 15.9 | 12.4 | 14.2 | one core |
| `solve` + `alone` | 8.5 | 8.7 | 17.4 | 19.5 | 7.9 | 8.8 | one core |
| `dissolve` | 8.5 | 8.9 | 10.9 | 11.8 | 12.3 | 13.7 | one core |
| setup (`active`, `Start`, `Drive`, `fixed`) | 4.3 | 4.2 | 4.7 | 5.1 | 5.5 | 6.0 | one core |
| `Draws::settle` + `owe` | 8.6 | 9.1 | 5.5 | 5.9 | 19.3 | 21.0 | one core |
| `air::state` + `ground_now` | 6.6 | 6.8 | 8.4 | 8.3 | 9.5 | 10.0 | one core |
| the liquid (12 steps a year) | 36.4 | 33.4 | 43.8 | 47.0 | 50.4 | 55.7 | mostly one core |
| Flow's state process | 9.2 | 9.1 | 11.3 | 12.4 | 14.0 | 12.9 | one core |
| **Flow, the stage (series)** | 175 | 148 | 240 | 209 | 258 | 225 | |

On 4 threads the parts that run on one core are 114 of 148 ms (77%) on the island and 156 of 209 (75%) on the globe: the
design's 68% to 76% stands. In instructions (callgrind, share of a year, 1 thread; island / globe / region): `pass`
6.1 / 10.9 / 7.2%; the air's booking `store::apply` 2.0 / 2.5 / 2.2%; `gather` 1.4 / 2.6 / 1.5%; the band `solve`
0.7 / 1.9 / 0.5%; `dissolve` 0.8 / 1.0 / 0.9%; the liquid 4.0 / 5.3 / 4.4% (`surface` 2.5 / 3.0 / 2.7%, `under` 1.1 /
1.4 / 1.2%). The cell rows `pass` are 6.1% to 10.9% of the instructions of a year, not the 24% to 43% that summing every
function that contains `pass` would say.

**Rounds a step:** island22 2.26 (2 rounds in 74% of the steps, 3 in 26%), globe36 3.97 (4 rounds in 94%), region24
2.22 (2 in 78%, 3 in 22%), 36 steps a year.

**Cells that are calm per round** (a ground cell whose seen air temperature moved under 0.01 K and whose active gases
moved under 1e-4 of their sum since its last row):

| Round | island22 | globe36 | region24 |
|---|---|---|---|
| 2 | 0.0% | 0.0% | 0.0% |
| 3 | 19.0% (of the 26% of steps that have one) | 13.2% | 3.8% (of 22%) |
| 4 | none | 96.8% | none |
| all rounds from the second on | **4.0%** | **36.1%** | **0.7%** |
| by temperature alone / by gas alone | 8.9 / 10.2% | 45.4 / 38.7% | 5.1 / 4.7% |

Round 2 never has a calm cell: the first solve moves the air of every node by more than the thresholds. Counted on the
cell rows of all rounds, calm cells would skip **2.2%** (island), **27.0%** (globe) and 0.4% (region) of the rows.
**AIR1's gate "cell rows -25%" is met only on the globe; the island and the region get under 3%.**

**The share of sea cells that would group (AIR6)** (open sea: below 0 m, liquid standing, no ice cover, no bodies, no
draws; a group is the cells under one air node, in one row of latitude, whose ground is within 0.2 K; the first of each
group keeps its row):

| | island22 | globe36 | region24 |
|---|---|---|---|
| open sea cells, share of all cells | 51.2% | 62.5% | 39.9% |
| **sea cells that would share a row** (not the first of their group) | **54.2%** | **41.3%** | **56.3%** |
| the same, share of all cells | 27.8% | 25.8% | 22.5% |
| groups a step | 113 | 238 | 134 |

**AIR6's gate ("40% or more of the cells group")**, and G11's reserve gate: met if "cells" are sea cells (54 / 41 /
56%), not met if they are all cells (28 / 26 / 22%). The second is the one that bounds the gain: `pass` cannot lose more
than 28% (island), 26% (globe) and 23% (region), before the cost of correcting each member, against AIR6's need of 30%.

### MI0: the minds

Brained updates (decisions, `choose::act` calls) a year: 12,200 (island22), 11,180 (globe36), 5,466 (region24); nodes
make 97.8%, 98.3% and 78.7% of them. The habit of minds 2.1 was kept on the side for each body (rules 1 to 5, `wake`
0.25, `sleep_most` 90 days) while the world ran as it does; "would sleep" is what the rules say at that update.

**The sleep share** (MI3a's gate: at least 40% on island and globe, measured again after G2, X36):

| Share of the decisions that would sleep | island22 | globe36 | region24 |
|---|---|---|---|
| **rules 1 to 5 as designed** (nodes / individuals) | **13.9%** (13.2 / 42.6) | **10.9%** (10.6 / 30.2) | **28.7%** (26.3 / 37.4) |
| `others` keyed by lineage (a set of other lineages) | 18.6% | 11.8% | 40.3% |
| rule 5 left out | 14.6% | 13.2% | 28.7% |
| a merge that brings under 5% new members does not wake it | 38.2% | 25.6% | 35.9% |
| that, and `others` keyed by lineage | 49.2% | 27.9% | 50.7% |

**The gate fails as designed on all three benches** (14 / 11 / 29%, against 40%). The plan's 5 to 8 ms for MI3a after G2
rests on a sleep share of about 50%; at 14% / 11% it would be a third of that or less.

**Wake causes** (the first rule that fails, in the order of the rules; share of all decisions):

| Cause | island22 | globe36 | region24 |
|---|---|---|---|
| no habit yet (first thought) | 0.1% | 0.1% | 1.0% |
| rows changed by a **watch** applied | 3.1% | 27.4% | 3.1% |
| rows changed by a **merge** in `nodes::add` (children or maturing members join the node) | **65.2%** | **48.1%** | **31.1%** |
| rows changed by a resize | 0.0% | 0.0% | 0.0% |
| `sleep_most` passed | 0.0% | 0.0% | 0.1% |
| a sense moved over 0.25 | 12.1% | 10.0% | 24.3% |
| `others` changed (ids of other lineages' faces in the cell) | 4.9% | 1.2% | 11.7% |
| the habit is no longer possible (rule 5) | 0.8% | 2.3% | 0.0% |
| **sleeps** | 13.9% | 10.9% | 28.7% |

Which rule alone keeps a decision awake (all the others would hold): the sense rule 8.2 / 7.4 / 10.1%; `others` by ids
4.7 / 0.9 / 11.7% (keyed by lineage: **0.0% on all three**: the set of other lineages in a cell hardly ever changes);
rule 5 0.8 / 2.3 / 0.0%. Senses over 0.25 (a decision may have several): the body's warmth against the ground 14.8 /
11.7 / 16.2%, light 13.9 / 16.8 / 7.3%, the ground's temperature 8.5 / 5.2 / 11.3%, crowding 1.9 / 2.4 / 11.6%, loose
substances 3.0 / 0.9 / 7.2%, own stores 2.8 / 1.2 / 2.0%, the rest under 2.1%. The merges that wake it, by the share of
members that joined: under 5% 10,908 / 7,083 / 1,518, 5% to 25% 3,753 / 2,729 / 1,139, 25% or more 1,241 / 935 / 747
(in the 2 years). **The rule "any merge with other rows makes it think" is what stops the habit**; seasons (warmth,
light, ground temperature) come second.

**Decisions per (cell, lineage, batch):** 1.72 (island22), 1.94 (globe36), 1.33 (region24) decisions for each distinct
(cell, lineage) of a batch (nodes only: 1.69 / 1.90 / 1.33).

**Place sharing, decisions per distinct (cell, sight) between two surveys** (MI4's gate: at least 1.5): **1.95
(island22), 2.17 (globe36), 1.48 (region24)**: 333, 305 and 149 decisions on 171, 141 and 101 places per survey. The
gate is met on island and globe, and missed by 0.02 on the region.

**Parts of a node's decision that move or bond** (MI5): parts placed per node decision 0.05 (island), **1.96 (globe)**,
0.19 (region); on the globe 54.5% of the parts came back to the node, 45.5% left; of the parts placed for a move 88.9%
came back on the globe, and none on the island and the region (so a move that is paid leaves, as one body).

**The breakdown of `choose::act`** (callgrind, share of a year; island22 / globe36 / region24):

| | island22 | globe36 | region24 |
|---|---|---|---|
| `choose::act` | 11.8% | 17.8% | 3.6% |
| perception of bodies (`perceive`: the faces loop, the cover) | 2.2% | 1.2% | 0.8% |
| perception of cells (`walk` and `options`: scores, direction features) | 1.1% | 1.3% | 0.3% |
| the brain's inputs, `pick`, `noise`, the rest | 1.3% | 1.2% | 0.4% |
| the network (`brain::think`) | 1.5% | 1.3% | 0.4% |
| learning (`learn::learn`, of it `brain::learn` 0.5 / 0.5 / 0.2) | 0.6% | 0.6% | 0.2% |
| acting (`spread::spread`: parts placed, `act_as`, the traces; includes the node's trace) | 5.0% | 12.2% | 1.2% |

In shares of `choose::act`: acting 42% / 68% / 34%; perception 28% / 14% / 31%; the network and learning 18% / 11% /
17%; the rest 11%. (Z1's 11.6 / 17.8 / 3.5% hold; the design's 16% does not.)

### G0: the gap

**The update split** (timers; µs per update, one thread then 4 threads; the design's probe world beside it):

| | island22 1T / 4T | design 1T / 4T | globe36 1T / 4T | design 1T / 4T |
|---|---|---|---|---|
| plant update | 18.0 / 21.9 | 16.1 / 21.9 | 16.7 / 21.8 | 17.2 / 23.1 |
| animal update | 37.9 / 47.2 | 34.9 / 45.9 | 41.8 / 54.8 | 42.3 / 56.4 |
| of it: the body's day | 18.0 | 15.4 | 15.6 | 15.2 |
| of it: the decision | 14.1 | 13.5 | 21.2 | 21.3 |
| of it: the rest | 5.8 | 6.0 | 5.0 | 5.8 |
| updates a year: plants, animals | 20,856 / 12,293 | 18,072 / 8,846 | 10,728 / 11,945 | 10,573 / 11,840 |

region24: plant 16.8 / 20.9 µs, animal 36.4 / 44.4 µs (decision 16.4, body's day 16.3, rest 3.7); 47,419 plant and
5,522 animal updates a year. The animal updates are by nodes in 97.7% (island22), 98.1% (globe36) and 78.4% (region24) of the cases.
**The animal node's interval:** the mean is 9.07 days (island22), 9.21 (globe36), 9.67 (region24). A floor of 20 / 30 /
36 / 60 days keeps this share of the node updates: island 0.45 / **0.30** / 0.25 / 0.15; globe 0.46 / **0.31** / 0.26 /
0.15; region 0.48 / **0.32** / 0.27 / 0.16. The design's 0.28 and 0.31 stand (island 0.30).

**Change's sleep** (`clock/sleep.rs`; per year, ground nodes): due nodes 11,502 of 17,424 (66%, island), 13,584 of
23,328 (58%, globe), 19,859 of 26,112 (76%, region). Woken by a gain (`Sleep::watch`): 23.0% / 25.1% / 13.9% of the due
ones. Rests a node takes after reacting: **under 10 days 71.2% / 66.8% / 81.1%**, 10 to 30 days 10.7 / 9.1 / 6.1%, 30 to
89 days 18.1 / 16.1 / 12.8%, at 90 days 0 / 8.0 / 0%. (The design: 75% under 10 days.) Air nodes: 618, 986 and 582 due a
year. Change on 4 threads: the rows 26.4 / 26.5 / 34.2 ms (wall) and the booking 22.3 / 22.2 / 38.9 ms; on 1 thread
40.4 / 36.7 / 54.7 and 21.0 / 18.5 / 34.8 ms. Rows over 50 µs: 8, 4 and 12 a year (1.7, 0.8 and 2.0 ms); the longest row
1.2, 0.7 and 0.8 ms. In instructions the rows are 4.8 / 4.6 / 5.2%, the booking `store::apply` 1.4 / 1.2 / 1.6% of a
year.

**The clock settings, cost only** (timers build, 4 threads, 2 years from the same save, medians of 5 sets, ms a year; the
world drifts, so this is cost and not a check of the world; the bodies' due batches follow the new batch length):

| Setting | island22 | globe36 | region24 | What moved (island / globe) |
|---|---|---|---|---|
| today (`phys_days` 10, batch 2 days, node 36, animal nodes by gene) | 737 | 780 | 1,082 | |
| `phys_days` 30 | 648 (0.88) | 807 (1.03) | 1,010 (0.93) | Change 55 to 21 and 51 to 22; bodies 506 to 468 and 490 to 523 |
| animal node floor 30 days | 491 (0.67) | 569 (0.73) | 1,016 (0.94) | animal updates about 12,300 to about 3,000 (all updates 33,024 to 23,871) and 11,900 to 2,900 (22,414 to 13,594); bodies 506 to 290 and 490 to 285 |
| `batch_days` 10, `node_days` 40 | 607 (0.82) | 743 (0.95) | 909 (0.84) | bodies 506 to 381 and 490 to 421 |
| all of them (`node_days` 60) | 379 (0.51) | 427 (0.55) | 608 (0.56) | bodies 506 to 181 and 490 to 173; Change 55 to 23 |

The globe's totals moved by the noise of the machine (its Flow rose from 203 to 219 ms with `phys_days` 30, which does not
change Flow); the stages that the setting changes are the ones to read: Change x0.38 / x0.43 / x0.36, the bodies' stage
with the floor x0.57 / x0.58 / x0.91, with the batch of 10 days x0.75 / x0.86 / x0.77, with all x0.36 / x0.35 /
x0.47. The design's year ratios were 0.89 / 0.74 / 0.66 / 0.55 (island) and 0.96 / 0.69 / 0.75 / 0.52 (globe). The
floor gains less on the region (0.94): its animals are 8.7% of its year.

**The four-worlds test** (timers; one world on 1 thread alone, against four separate 1-thread worlds at the same
time; 3 sets, medians; the update's thread time):

| | island22 | globe36 | region24 |
|---|---|---|---|
| plant update alone / in four, µs (ratio) | 17.3 / 18.3 (0.99) | 17.3 / 17.7 (1.06) | 16.1 / 17.5 (1.08) |
| animal update alone / in four, µs (ratio) | 36.7 / 38.5 (0.99) | 43.7 / 45.1 (1.09) | 35.3 / 38.3 (1.09) |
| updates' thread time a year alone / in four, ms | 813 / 855 | 707 / 729 | 956 / 1,043 |

Four worlds at once cost 0% to 9% more per update than one alone (the design measured 10% to 30%, mean 18%). One world
on 4 threads costs 22% to 31% more per update than on 1 thread (table (f)). So of the 22% to 31%, the machine owns 0%
to 9% and our process the rest, 12% (region24) to 25% (island22, globe36): G15's room is larger than the design
thought, on this machine at this hour.

The code on one core while the 4 threads are on, against 1 thread (timers; the design saw 25% to 30%): the liquid
33.4 / 47.0 / 55.7 ms on 4 threads against 36.4 / 43.8 / 50.4 on 1 thread (-8% / +7% / +11%); Change's booking 22.3 /
22.2 / 38.9 against 21.0 / 18.5 / 34.8 (+6% / +20% / +12%); the air's `book` 22.9 / 28.3 / 38.0 against 22.2 / 26.0 /
34.3 (+3% / +9% / +11%). Less than the design's 25% to 30%, and not the same on all benches.

### The key-number references

`cargo test --release -p tape --test ranges the_key_numbers -- --ignored --nocapture`, run once on the clean code of
`4cc7b81` (a copy of the tree, 314 s). These are this round's fixed references (X5: no re-centring between steps).
Seeds 1, 2 and 3 in brackets.

| Key number | Value (seeds) | Reference | Verdict |
|---|---|---|---|
| ground temperature on land | 292.157676 K (291.499583, 292.636619, 292.336827) | 291.79 +-1 | in range |
| seasonal swing on land | 15.464015 K (16.054563, 15.061449, 15.276034) | 15.13 +-1.5 | in range |
| rain on land | 0.001814 mol/m2/s (0.001753, 0.001844, 0.001843) | 0.001714 x/ 1.3 | in range |
| heat-absorbing gas | 411.242700 ppm (398.417830, 419.466952, 415.843319) | 388.8 +-40 | in range |
| river cells | 695.566667 (729.300000, 620.800000, 736.600000) | 712.4 x/ 1.5 | in range |
| land worn down | 0.145772 mm/yr (0.142174, 0.146392, 0.148749) | 0.141 x/ 2 | in range |
| light-eaters | 10.239688 log10 kg (10.295646, 10.214008, 10.209410) | 10.38 +-0.3 | in range |
| eaters | 5.411112 log10 kg (5.340904, 5.455988, 5.436442) | 5.354 +-0.5 | in range |
| life of light-eaters | 1.241509 yr (1.315172, 1.305659, 1.103698) | 1.81 x/ 2 | in range |
| life of eaters | 13.908324 yr (14.886267, 12.952336, 13.886369) | 10.47 x/ 2 | in range |
| rain on land (m/yr) | 1.015 | Earth's 0.76 x/ 3 | in range |
| **light-eaters' stored share (S50)** | **0.556** (0.557, 0.556, 0.554) | under 0.5 | **out (known failure)** |

The test ends with its one known failure, S50. S84 (the sleep bias on `air_3`) is a different test and was not run.
The raw output is in `raw/keys.log`.

### The gates at a glance

| Gate (step) | Number | Verdict |
|---|---|---|
| P0: plants under 20% of island22's year (1 thread) kills P3 | plant updates 30.0%; all included about 37% | P3 stays |
| P1.3: `|T_plant - T_ground|` often over 3 K? | over 3 K in 54 / 50 / 51% of plant updates | yes: stage 6 must book the difference |
| E0 (c) / E5: serial-body updates at least 10% on a bench | 0.5 / 2.1 / 0.0% | not met today (measured again after P3, G2) |
| E2: busiest share at 4 cells at most 0.6 x today's on two benches | 0.43 / 0.55 / 0.26 | met on all three |
| E4: `apply` on the main thread at least 3% of a year at 4T (after the air section) | booking pieces 9.0% / 8.9% | met today |
| AIR1: cell rows -25% | calm cells would skip 2.2% / 27.0% / 0.4% of the rows | globe only |
| AIR6 / G11 reserve: 40% or more of the cells group | sea cells 54 / 41 / 56%; all cells 28 / 26 / 22% | met on sea cells, not on all cells |
| MI3a: sleep share at least 40% on island and globe | 13.9 / 10.9 / 28.7% | **fails** as designed (49% / 28% / 51% with softer rules) |
| MI4: sharing at least 1.5 | 1.95 / 2.17 / 1.48 | met on island and globe |
| G15: an update on 4T at most 1.2 x its 1T cost | 1.22 to 1.31 | 0.02 to 0.11 away |
| the key numbers | all but S50 in range | S50 stays a known failure |

### What is measured and what is not yet

All gates named for R2 have a number above. These parts are weaker than the rest, or not done:

| Item | State |
|---|---|
| P0, E0 (a) to (f), AIR0, MI0, G0, the minds breakdown, the key-number references | measured |
| Plants' share of booking and outbox | a proxy: counts of number changes and of litters, not time per kind |
| E0 (d), the 4-thread gain of tiles | counted (busiest share, longest-task-first share), not timed on a tiled build |
| E5's S2 time (part acts, watches, litter returns) | not measured per kind; only an upper bound (`deliver` plus serial bodies) |
| MI0, `others` keyed by lineage, and merges under 5% | what-if variants on the side; the world ran as it does |
| S84 (sleep bias on `air_3`) | not run (a different test) |
| Wall times | made on a busy machine (4% to 14% above Z1); judge steps by instructions (S45) |
| PR-A, PR-B, PR-M, PR-S, G14-P1, AIR4-R1, R3 | not part of R2; not done here |

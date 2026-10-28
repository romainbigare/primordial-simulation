# Lighter world: the final check

Watch report for the owner (handover 12.4; `docs/lighter/plan.md`, Phase Z). The numbers behind it are in
`docs/lighter/measures.md` (the sections "Z: the year in instructions", "Z: the slow suite", "Z: the year in wall
time", "Z: 1,000 years", "Z: the largest globe" and one section for each step), the decisions in
`docs/lighter/decisions.md` (L1 to L47), and what changed and when in `docs/history.md`, section 7. The last round's
report is `docs/reports/speed-x10.md`.

## In short

- **This round changed what we compute, not how fast we compute it.** It kept five steps: cheaper saves (F), the skip
  of tiny bare reactions in bodies (B1), a faster start for the parts' reactor (G1), children in lumps (H1) and the
  liquid's sink at every physics step (L1, a law fix that costs 0.3%). The big ideas (plants as a cover, living bodies
  that do not rot, a coarser air, the parts' law) were measured and not kept: each changed the world more than it saved.
- **A year on one thread costs 12% / 22% / 10% less** (island22 / globe36 / region24, counted instructions, each build
  on its own remade world). The kept steps alone give about 9% / 8% / 9%; the rest is the worlds (globe36's start
  holds 573 parts, the end's 14). **An animal update is 13% to 22% cheaper**, a stand update 1% to 6%.
- **On 4 cores a settled year is x1.10 / x1.30 / x1.02 faster** (246, 247, 548 ms). The region's end world holds more
  animals (20% more animal updates a year), so its cheaper updates are spent on more of them.
- **1,000 years are 1.2 to 1.4 times faster:** island 6.1 to 4.3 minutes, globe 5.1 to 3.7, refined region 10.6 to
  8.7 (wall time, default pictures and saves, the same hour).
- **The largest globe that runs 1,000 years in 15 minutes:** **size 30, 1,800 cells** (the last
  round: size 28, 1,568 cells). It takes 12.9 minutes; size 32 (2,048 cells) takes 17.1 minutes.
- **The books are exact and the world is in range.** Element drift under 6e-16, energy under 2e-14 in every long run.
  Every key number is in range but S50 (known, 0.559). The sleep test (S84) passes now: it states the bias you
  accepted.
- **The core has 10,473 code lines** (the round's start: 10,444). The ceiling is now 10,673 (L4).

## Where the speed stands

**One thread, instructions of a year** (callgrind, one year from the benches; the start is `ew-base`, the code of
`cb6db47`, on `benchZ`; the end is `1abddae` on benches made with its own code, `benchZ50`). Millions:

| | island22 | globe36 | region24 |
|---|---|---|---|
| start | 4,932 | 5,393 | 10,365 |
| **end** | **4,339** | **4,183** | **9,284** |
| change | -12.0% (x1.14) | -22.4% (x1.29) | -10.4% (x1.12) |
| the kept steps, each on its own base | about -8.9% | about -7.6% | about -9.3% |
| an animal update, thousand instructions | 156 to 130 (-17%) | 152 to 131 (-13%) | 158 to 124 (-22%) |
| a stand update | 86 to 81 (-6%) | 87 to 86 (-1%) | 88 to 83 (-6%) |

Where the end's year goes (one thread, share of the year): animal updates 39% / 32% / 45%; stand updates 22% / 22% /
22%; the landing after a batch 3.6% / 3.0% / 3.8% (it was 7.0% / 4.9% / 6.5%); the survey about 5%; the air 16% / 24% /
13%; the liquid 3.4% / 4.5% / 2.6%; Change 8% / 8.5% / 7%; parts 0.1% / 0.1% / 0.6% (globe36 was 4.8% with 573
parts); the tape 2%.

**Four threads and one, wall time** (6 interleaved sets, each run alone under the lock, the machine quiet; the mean of
years 2 and 3 of a 3-year run; medians, ms a year):

| | island22 | globe36 | region24 |
|---|---|---|---|
| start, 4 threads | 271 | 321 | 561 |
| **end, 4 threads** | **246** | **247** | **548** |
| **faster** | **x1.10** | **x1.30** | **x1.02** |
| start, 1 thread | 617 | 662 | 1,447 |
| end, 1 thread | 542 | 491 | 1,220 |
| faster, 1 thread | x1.14 | x1.35 | x1.19 |
| bodies / Flow at the end, 4 threads | 141 / 79 | 131 / 94 | 362 / 139 |

The one-thread runs spread by up to 30% inside one build, so read them to about 10%; they agree with the counted
instructions. On four threads the bodies' stage falls 17% / 29% / 7%; Flow (mostly the air) does not fall, as its code
did not change.

**1,000 years** (seed 1, `ew run --years 1000`, default pictures and saves, 4 threads, one run at a time; wall time of
the command):

| Run | Cells | Start | End | Faster | Per year at the end | Years 901 to 1000 at the end |
|---|---|---|---|---|---|---|
| island 22 x 22 | 484 | 6.1 min (366 s) | 4.3 min (260 s) | x1.41 | 259 ms | 250 ms |
| globe 36 x 18 | 648 | 5.1 min (306 s) | 3.7 min (224 s) | x1.37 | 223 ms | 232 ms |
| refined region 24 x 24, 8 x 8 split | 768 | 10.6 min (635 s) | 8.7 min (521 s) | x1.22 | 517 ms | 604 ms |

Long runs gain more than a settled year because the saves are cheaper (the tape's column 35 to 41 ms a year at the
start, 5.5 to 13 ms at the end with the snapshots in it) and because the worlds part: the start's island grows to
1,275 animal nodes in its last century, the end's stays near 390, so its years 901 to 1000 cost 250 ms against 563.
One seed each: that part is the world's path, not a promise.

**The largest globe in 15 minutes** (`ew run --globe --size N`, 1,000 years, the end's build):

| `--size` | Cells | Per year | 1,000 years (wall) | Years 901 to 1000 | Bodies | Element / energy drift |
|---|---|---|---|---|---|---|
| 18 | 648 | 223 ms | 224 s | 232 ms | 1,424 | 3.1e-16 / 4.0e-15 |
| 28 | 1,568 | 701 ms | 706 s | 783 ms | 7,078 | 2.3e-16 / 2.5e-15 |
| **30** | **1,800** | 768 ms | **773 s** | 939 ms | 11,009 | 4.3e-16 / 5.9e-15 |
| 32 | 2,048 | 1,018 ms | 1,025 s | 1,712 ms | 29,308 | 5.8e-16 / 4.0e-15 |

A globe of size 30 has 30 rows of 60 columns, 2.8 times the cells of the bench globe, and fits with 14% to spare. At
the last round's end size 30 missed by 22 s. The cost grows faster than the cells, because the light-eater bodies do:
size 32 holds 28,567 of them at year 1,000 (4.5 times size 28's for 1.3 times the cells), and its last century costs 1.7
s a year. A longer run fits a smaller planet.

## The 1,000-year runs

Seed 1. Everything ran to year 1,000, with the end's build:

| Run | Wall time | Bodies at year 1,000 | Land K (years 951 to 1,000) | Gas, ppm | Element drift | Energy drift |
|---|---|---|---|---|---|---|
| island 22 x 22 | 4.3 min | 2,119 | 293.7 | 407 | 1.8e-16 | 3.2e-16 |
| globe 36 x 18 | 3.7 min | 1,424 | 290.8 | 376 | 3.1e-16 | 4.0e-15 |
| region 24 x 24 | 8.7 min | 6,011 | 294.1 | 406 | 1.5e-16 | 5.9e-15 |

Heat not given: 0 in all. The three worlds are alive and steady (light-eaters 9.6 to 15.1 log10 kg, eaters 7.3 to
11.4 at year 1,000). Against the start's runs of the same seeds: the island is 0.7 K cooler (L1 cools an island's land a little, -0.13 K in
its study; the rest is the world's path), the region 0.3 K warmer, the globe 0.5 K cooler with 3% more rain on land (one seed; a globe swings by
up to 0.2 K a year; a watch item). The gas is 8 to 19 ppm higher, in its range.

## What was built and kept

Gains are those of the step on its own base (`measures.md`); "1T" is instructions on one thread. Levels are from
`plan.md` 2: 1 bit for bit, 2 the same to rounding, 3 the same world, 4 a new law for a part of the world.

| Step | What | Gain | Level |
|---|---|---|---|
| F1, F3, F2 | The saves: the snapshot and the pictures timed with the tape; the tape's CSVs appended and the viewer's file at gzip fast; the compression and the write on a writer thread | a 1,000-year island run's writes -19.3 s (about -6%); the main thread's saves 2.4% to 0.4% of a 300-year run; the tape's column of a long run 35 to 41 ms a year to 5.5 to 13 ms | 1 |
| G1 | The parts' reactor starts its halving search where the heat allows | globe36 1T -1.2% | 1 |
| B1 | The skip: a bare reaction in a body that would use under 3.6e-5 of its scarcest input over a year is skipped | 1T -2.2% / -0.8% / -2.4% | 3 |
| H1 | Children in lumps: a population node makes its children at most once in `lump_days` (30), its brood filling at every update | 1T -7.0% / -5.9% / -7.1% | 4 |
| L1 | The liquid's sink into the bed's store at every physics step (a law fix) | 1T +0.3% | 3 |
| Tests | The long-step test judges six worlds; the sleep test states the gas bias it accepts; the drift tests' scene makes children at every update | none | 1 |

The core code went from 10,444 to 10,473 lines.

**The law changes** (each its own commit, with its effect; `laws.md` and `numbers.md` say them as they are):

| Change | What it does | Effect |
|---|---|---|
| B1, the skip | A bare reaction (no catalyst) inside a living body that would use under `TOO_SLOW` (3.6e-5) of its scarcest input over the world's year is not run. Counted over a year, so a body's chemistry does not depend on its interval | Key numbers within their noise; the long-step test 0.259 to 0.262 K (six worlds) |
| H1, children in lumps | A node makes its children at most once in 30 days, at its first update that may; its next turn is its own field; a node's first lump waits for a batch of its own phase, so no world-wide pulse. Individuals as before | Groups of children made -35% to -37%; the animals within their seeds' noise on three worlds; births by month, the nodes check, the pace and batch tests hold; snapshot 50 |
| L1, the sink | Rain over field capacity sinks into the bed's store right after each air step, not up to 60 days later. Rain held in the pores had darkened the land (+1.7 W/m², +0.2 K against a 2-day liquid) | The long-step test 0.273 to 0.191 K; the key numbers' land -0.05 K, land worn 0.134 to 0.147 mm/yr; +0.3% of a year |

## What was tried and not kept

Each was built or measured, judged against its line, and taken out (or not built). The branches keep the code.

| Idea | What happened | Alternative |
|---|---|---|
| 3, living bodies do not rot (the law) | Not built (L9). Probe PR3: every form makes the world heavier: eaters x35, animal nodes x7, updates +38%, land +0.3 K | B1, the skip (kept) |
| 5, plants as a cover: C1a and C1, the bound on kinds | Not kept (L45; `lighter/c-not-kept`). -1.2% of island22's year against a -3% line; light-eater splits x20 to x90; a better kind spreads under 2 cells a decade against 29; light-eaters' copy mass -0.7 to -1.0 decades. C2 (the young inside) not built: C1 + C2 about -7% on island22, under the -10% line. C3, C4 and the gate went with them | none for speed (A3, A4 under their worth lines) |
| 2 and 4, fewer stands a cell and a cheaper sample member | Not built: they waited for the cover (L15, L18), and A3 and A4 are under their worth lines after it (L45) | none |
| 6, animals look around less (D1, D2) | Not built (L14). PR6: perception is 0.6% to 0.9% of a year, the take's payer 0.6% | lane H: H1 (kept) |
| 7, a coarser air on globes (E1) | Not built (L12). PR7: -4.7% of globe36's year, but rain on land +6% to +12% on every seed and world | lane G: G1 (kept) |
| G2, the parts' reactor held at its ground's temperature | Not kept (L43; `lighter/g2-not-kept`). The reactor works (a call 136 to 5.7 thousand instructions), but without the 1e6 K burn parts no longer end: 26,087 after 30 years on globe36 against 660 | none beyond G1 |
| H5, one draw per birth | Not kept (L41; `lighter/h5-not-kept`): -1.5% / -0.5% / -1.7%, under a law step's 3% | none |
| H2, H3, H4 | Not built: maturing in lumps under a law step's line; `think_days` 40 costs young eaters (X72); a cheaper `nodes::add` at the law-step line, and the objects round rebuilds that code next | none |
| HP1, HP2, the hard plant law (your request: tall and short plants side by side) | HP1 not kept (L47; `lighter/hp1-not-kept`): plants stiffen with lime (17% to 34% of dry mass) because chemistry 0 makes lime cheaper than frame; island22's year +40%. HP2 not built | first chemistry 0's price of lime against frame (a life round) |
| Small nodes act whole (open item 8 of the last round) | Not kept (`study/small-nodes-not-kept`): two fixes lower the young eaters (5.18 to 4.89 log10 kg) and move nodes away from individuals in the nodes check | an open item |

## Why not more

1. **The big savings changed the world.** The cover saved little on today's world (C0: at best 13% / 11% / 15% at
   today's kinds, 5% / 2% / 9% at the full bound) and what it saved came with wider kinds, lighter plants and slower
   invasions. The law of living bodies and the coarser air made the world heavier or wetter.
2. **Animals are the largest part** (32% to 45% of a year), and in them the bookkeeping of nodes (children, maturing,
   joins) costs more than the decisions. H1 took its largest piece. The next pieces (maturing in lumps, a cheaper join)
   are each 2% to 3%, at the line of a law step.
3. **The air did not move** (13% to 24% of a year on one thread). A wider air node made the land wetter; the air's
   loops are short for the pool (last round's AIR2).
4. **Busier worlds eat the gain.** The region's end world holds 20% more animal updates; its year on four cores is
   about the same as at the start.

## The slow suite and the debug checks (Z)

- **The slow tests** (`cargo test --release --workspace --no-fail-fast -- --ignored`, 17 tests, 16 minutes): 16 pass,
  1 fails: the key numbers, only on **S50** (the light-eaters' stored share 0.559 against "under 0.5"; 0.558 at the
  start). The other 10 key numbers are in range: land 292.10 K, swing 15.30 K, rain 0.001809, gas 412.0 ppm, river
  cells 697, land worn 0.1471 mm/yr, light-eaters 10.24, eaters 5.18, life of light-eaters 1.36 yr, of eaters 12.85
  yr.
- **The sleep test passes** (S84 no longer fails: it holds `air_3` to the bias you accepted).
- The nodes check, the pace and batch tests, the long-step test (six worlds) and the globe's climate test, the century
  of books, `individuals_mode_merges_no_animal` and the other slow tests pass.
- **The debug checks pass:** the books, overdraw and views with debug assertions (199 passed, 17 slow ignored); the
  golden test without threads and in a plain debug build.

## Open items for you

From the plan's list and the decisions:

1. **Merging.** The branch `speed/lighter` is yours to merge. It has three law changes (B1, H1, L1), a new golden and
   snapshot 50.
2. **Living bodies do not rot** (handover 13.2). You said leave as is. The law (b0: a decomposer reaction runs in a
   living body only by the body's own catalyst, about 15 lines) is ready, but it makes the world heavier (eaters x35,
   updates +38%). It belongs to a round about life, with new eater references (L9, L19).
3. **How walls end and the parts' reactor** (G2, L43). A part reacts its cell's whole air share for 6 months and passes
   1e6 K in every call; that burn is what ends walls. You said yes to the loose-residue rule, rot through the outer
   skin, part nodes, built solids of bodies as building matter, a pieces store per cell, mud by properties and stone
   that lasts centuries; the designs run in the objects round.
4. **S50, plants store too much.** 0.559 against "under 0.5". A study is queued.
5. **Plant forms side by side.** C1 is not kept; the hard plant law waits for chemistry 0's price of lime against frame
   (a life round), then HP1 (L47).
6. **Small nodes act whole.** The two fixes made the world worse; the deficit they uncover is open.
7. **The plants' copy-mass gene at long paces** (PR-C2 of the last round): kept in the notes.
8. **A coarser air:** left as is (E1 not built, L12).
9. **The sleep's gas bias:** accepted as small; the test states it. **The 60-day liquid warms the land:** answered by
   L1.
10. **Watch items from the long runs** (one seed each): the end's globe is 0.5 K colder over its last 50 years with 3%
    more rain on land; the end's island holds 390 animal nodes in its last century against the start's 1,275 (2
    lineages against 9). Neither is out of a range; three seeds would say whether either is more than the path of one
    world.

## Surprises

- The kept steps add to about 9% of a year, but the remade worlds differ by more than that: globe36's start bench held
  573 parts (4.8% of its year) against 14, and the region's end bench 20% more animal updates. Across remade worlds a
  speed number is a world number too.
- The law fix (L1) came from a study asked for by you, not from the speed plan, and it cost only 0.3%.
- The cover, the largest idea, failed on its first step: the bound on kinds made kinds wider, so they split again (x20
  to x90 splits), and the saving was 1.2% instead of the 3% needed.

## How the numbers were taken

- **Instructions:** callgrind, one year, one thread (`RAYON_NUM_THREADS=1`), from the benches. The start is
  `$L/ew-base` (the code of `cb6db47`) on `benchZ` (snapshot 49); the end is `1abddae` on `benchZ50` (snapshot 50, made
  with its own build by `make_benches.sh`). The parts come from measuring builds of both (the same code with the parts
  not inlined; the same hashes; 0.1% more instructions).
- **The year on 4 and 1 threads:** the same benches; 6 interleaved sets; each run alone under `flock`; the machine
  quiet (load 1.2 to 1.8); the mean of years 2 and 3 of a 3-year run, the sum of the stages' timers; medians.
- **Long runs:** one `ew run` at a time on 4 threads, wall time of the command, default pictures and saves, start and
  end in the same hour. The machine restarted twice during phase Z; the size-28 globe was cut at year 300 and run
  again whole; no other run was cut.
- **Steps that change the world** were judged on one world (the base's save, or same-seed counted work), not on remade
  benches (PR-S noise: 19% island, 11% globe on three seeds). The end against the start is across remade worlds, so
  its numbers mix the steps and the worlds; the per-update costs are the steadier part.
- **The machine drifts** by 5% to 10% between hours (the start build's year on 4 threads: 260 / 361 / 529 ms in the
  last report, 271 / 321 / 561 now). Each comparison above is inside one hour.
- **No pictures** in this report. The numbers are in `measures.md`.

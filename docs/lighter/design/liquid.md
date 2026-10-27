# The liquid's column every physics step (a fix for the long liquid step's warming)

> **Status:** a proposal from the study "why the long liquid step warms the land" (`measures.md`). Not built. The
> prototype is on the branch `study/liquid-warm` (switch `LIQ_FAST=1`, never merged).

## 1. The fault

The air step books the rain of its 10 days into each ground node. Liquid over field capacity leaves a node only in the
liquid step (`flow::liquid`, every `flow_days`, 60): it sinks into the bed's store (`drain::under`), drains down the
hillslope and runs off (`liquid::surface`). So the rain of up to 60 days waits in the pores and on the surface. The
stock it makes is a saw tooth: its mean grows with the step, about half the net rain of one step.

On the long-step test's world (32 x 32, years 11 to 20, 6 seeds, each against itself with a 2-day liquid, the air as
today), the liquid over field capacity on land is +9, +31, +65 and +135 mm at 10, 30, 60 and 120 days.

- **Evaporation does not follow it.** Ground at field capacity already evaporates at the open rate (the bucket's
  `wet` is `field` x the pores, Manabe 1969). The latent cooling grows only +0.58 W/m² at 60 days.
- **The reflectance does.** `land::reflectance` covers the ground with all its liquid, depth d over d + `hide` (0.5 m
  for a liquid), and the liquid is darker than the loose ground. The land's reflectance falls by 0.0069 at 60 days; the
  sunlight the land absorbs grows by +1.70 W/m².
- The rest, +1.1 W/m², leaves as heat radiation and sensible heat from a warmer ground: **+0.20 K**. The budget closes
  on these two terms (sun less latent +1.12 W/m², heat out +1.10 W/m²), so moving liquid carries no heat that counts.

It is a first-order splitting error: the warming grows in proportion to the step (+0.09, +0.11, +0.20, +0.43 K at 10,
30, 60, 120 days), in every season (winter +0.18 K, summer +0.25 K at 60 days).

## 2. The fix: split the liquid by its time scales

The liquid holds two kinds of process.

| part | what | time scale | feeds on |
|---|---|---|---|
| the column (fast) | what is over field capacity sinks into the bed's store (the vertical link of `drain::under`: at most `seep` a second and no more than the store's room) | `seep` 3.6e-6 m/s passes 3.1 m in 10 days: at once | the rain of each air step |
| across cells (slow) | springs, the downhill pass (hillslope drainage, runoff, load, wear), Darcy's law between cells in pores and stores, what a full store gives back | weeks to years | the store and what stands |

The column runs **every physics step, right after the air step** (its own process of the clock, period `phys_days`).
The liquid step keeps the rest, every `flow_days`. This is the same law (the same rate, the same room, the same heat
carried and the same head lost into the store), only booked at the step of the process that feeds it. No new rule,
no new number. A Strang split does not help here: it runs the liquid twice a step (the cost of a 30-day liquid) and
still leaves rain waiting a quarter of the step on average.

**Code** (about +25 / -15 core lines):
- `drain.rs`: the sink loop of `under` becomes `fn sinks(w, s, (mob, held), liq, dt)` (the cells, their shares and
  the head lost into each store); a new `pub(super) fn column(w, s, dt)` books it (`carry`, `Batch::fallen`, `book`,
  as `under` does today). `under` no longer sinks.
- `flow/mod.rs`: `pub fn column(w, dt)` through `with`. `schedule.rs`: a process `(physics, "column")` after `air`.
- `Config::validate`: `flow_days` must be a multiple of `phys_days`, so each liquid step falls on a physics step,
  after its column (today 60 and 10; the long-step test's short run 2 and 2).
- `laws.md` 0 (the schedule) and 1.2 (Pore water): the sentence "Liquid above it sinks to the deep store" says it does
  so every physics step. `history.md` one line. No `numbers.md` row. No field of `World` or `Config` changes: the
  snapshot version stays.

## 3. What the prototype shows

| | today | the column each physics step |
|---|---|---|
| probe, land against a 2-day liquid, 60 days | +0.200 ± 0.046 K | +0.052 ± 0.042 K |
| probe, 30 / 120 days | +0.114 / +0.432 K | +0.074 / +0.093 K |
| probe, reflectance of land, 60 days | -0.0069 | -0.0013 |
| long-step test, land (limit 0.3 K) | +0.273 K | **+0.191 K** |
| long-step test, summer / winter | +0.174 / +0.092 K | +0.118 / +0.017 K |
| long-step test, rain / rivers (limits 3.2e-4 / 27.7) | +1.8e-5 / +1.2 | +1.1e-5 / +0.15 |

The warming no longer grows with the liquid's step (30 to 120 days within the noise). What is left (+0.05 K) is the
standing liquid over full pores and full stores, which waits for the pass but darkens little, and the 10-day column.
The default world cools by about 0.14 K (probe -0.148 K; in the long-step test the default runs move -0.14 K and the
short runs -0.06 K, as their 2-day column now runs before the pass).

## 4. Cost (callgrind, one thread, one year from the `benchO50` saves; millions)

| | island22 | globe36 | region24 |
|---|---|---|---|
| the liquid today | 132.5 | 185.6 | 212.3 |
| the liquid with the column apart, plus the column | 126.2 + 23.3 | 173.4 + 33.0 | 201.1 + 42.4 |
| added, share of the year | +17.0 (+0.29%) | +20.8 (+0.38%) | +31.2 (+0.26%) |

A column call costs 0.65 M on island22 (36 a year), most of it in reading the mobile liquid and the stores of every
cell and booking; a column that reads only the cells that rained over field capacity would cost less.

## 5. Check level and what must hold

Level 3 (the order of a law changes; the world moves by about -0.14 K of land). Golden re-pinned, with and without
threads. Must hold:
- the long-step test, unchanged (expected about 0.19 K of 0.3: room 36% instead of 9%);
- the globe's climate test (`tape/tests/ranges.rs`);
- the key numbers: land, swing, rain on land, rivers, land worn, the heat-absorbing gas. A first run with the
  prototype (`measures.md`): all in range but S50; land -0.05 K, rain -0.5%, rivers the same, land worn 0.134 to 0.147
  mm/yr (back to the 30-day liquid's 0.147);
- the books (debug checks): the column books both sides through `carry` and `book`, as `under` does.

## 6. Then

With the column apart, the liquid's step no longer warms the land (120 days: +0.09 K against +0.43 K today). A
120-day liquid would halve the liquid's cost again (about -1% of the island's year); rivers and land worn would need
the key numbers first. The architect decides.

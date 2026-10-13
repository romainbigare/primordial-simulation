# Watch report: the fix list before M4

13 October 2026. Code: `main` at 7de55c0 (the runs) and later. Review that started this work:
`review-m0-m3.md`.

## Short answer

We did all 16 items of the review. The physics gaps are closed: cold now costs energy, walking
wears paths, bodies are real-sized, plant cover holds real mass per m², brains inherit, and
learning works in its test. We found and fixed six real bugs on the way. The new runs are stable
for 300 years and keep energy and matter exact.

But most of the M2 and M3 watch-for items still do not appear: no seasonal travel, no focused
trails, no shelters, no tools, no new kinds. Two new problems are large: eaters wipe out plant
cover for 40 to 65 years, and the world is 18 to 37 times slower than the speed goal. Neutral
eaters (no hand-set brain) die out every time.

So the review's question ("do kinds, a food chain, trails and shelter now appear?") gets this
answer: a food chain yes, a network of worn paths yes, kinds and shelter no. By the review's own
rule, the next step is learning and planning (option 2 of the check point), together with M4,
because population nodes now hide the bodies that would build shelters and tools.

## What we built

| # | Review item | Done | What it gave |
|---|---|---|---|
| 1 | Debug check after each law part; `take_heat` | yes | Found the negative heat: big bodies evaporating liquid took more heat than the ground held. All heat takes are now limited. |
| 2 | Choose test: a body learns to move toward what it can take | yes | Found 3 learner bugs (see below). Now: 0.88 of time on the substance with learning, 0.43 without. |
| 3 | Learning and variety numbers in the tape | yes | `learned`, `greedy`, `kinds`, mass range, trophic share, deaths and births split into plants and animals. |
| 4 | Forgetting as a gene | yes | Gene `FORGET`, 0 to 0.02 per month. |
| 5 | Brain rows are genes | yes | Every child inherits its parent's brain rows with small noise. A group's brain is the mean of its members plus what it learned. |
| 6 | Metabolism depends on temperature | yes | A body can release only what its reactions give at its own temperature. At 270 K it can pay about half its upkeep. |
| 7 | No limit on bodies per cell | yes | `max_bodies_per_cell` is gone. Crowding costs energy instead. |
| 8 | Real-sized bodies; plants as population nodes | yes | Members of 1 g to 10 t. Plants are one node per lineage per cell, with real kg/m². |
| 9 | Cover feeds back into the laws | yes | Cover shades, slows walking, holds soil against rain and creep, and burns when hot. |
| 10 | Size physics | yes | Heat loss through the surface (mass^2/3); force is strength × mass; a quicker prey can escape; prey can hide in cover. |
| 11 | Edges that remember use | yes | Walking clears a lane that makes the edge cheaper; cover grows back over decades. |
| 12 | Neutral seeding with a spray of variants | yes | Default now. Neutral eaters die out (see below). `--eater-bias` gives the old start. |
| 13 | Brain size as genes, with a cost | yes | Hidden units, memory, bodies seen, sight range. `--clever-year` seeds a big brain. |
| 14 | Rivers | yes | The review's guess (sweep order) was wrong. The cause was evaporation in 15-day steps. Fix: a physical cap and air at 3 days: river cells 240 → 484. |
| 15 | Profile, cheap speed wins | yes | Cheap wins gave about 2%. Bodies are 67% of the time; inside them the brain is half. |
| 16 | 300-year runs on 3 seeds and this report | yes | This file. |

Also from the review's smaller list: the tuned constants are now in one `Config::phys` section
(57 numbers, for the M7 world search); the core no longer labels bodies as "plant" or "animal";
grid tables are no longer saved in snapshots; a test that a hot node reacts at once (it found a
bug); the dead tape code is gone; shared group brains were compared with one brain per body
(`--own-brains`: similar patterns, 20-35% slower). Reports are now kept in `docs/reports/`.

### Bugs found and fixed

1. Ground heat below zero (big bodies evaporating).
2. Learner: the error spread started 30 times too large; signal noise used up the step limit;
   and a body felt as good after dumping liquid as after eating.
3. Bodies grew without limit (one body of 3 million tonnes).
4. A node that reacts "far" skipped its first turn after a load.
5. A body that gave away everything stayed alive with nothing; this later made a NaN and crashed
   a run at year 221.
6. Plants handed all the light of 5 to 60 days to the ground at once. Cells reached 400-1,000 K,
   rock charcoal caught fire, and the greenhouse gas rose 4 times. Now the ground gets the light
   as it falls.

## The runs

128 × 128 island (256 km), 300 years, seeds 1, 5 and 7. Light-eaters placed in year 2 (neutral
brain, 20 rounds of mutation). **Eaters placed in year 22 with the old hand-set brain
(`--eater-bias`)**, because neutral eaters died out in 8 of 8 test runs. The eaters are a
starting condition, not an emergence.

| At year 300 | seed 1 | seed 5 | seed 7 |
|---|---|---|---|
| Plant members | 4.8e11 | 1.3e11 | 2.4e11 |
| Animal members | 3.4e10 | 3.3e12 | 3.1e10 |
| Animal bodies (nodes) | 16,821 | 12,824 (+199 single) | 12,449 |
| Cover on land, median | 3.4 kg/m² | 3.3 kg/m² | 3.1 kg/m² |
| Kinds | 2 | 3 | 2 |
| Brain lineages alive / ever | 6 / 2,617 | 12 / 3,001 | 8 / 4,854 |
| Ground temperature | 294.7 K | 295.6 K | 295.1 K |
| Land edges worn | 91% | 82% | 88% |
| Seconds per year (mean / at year 300) | 15.7 / 22.0 | 10.5 / 14.7 | 12.7 / 17.0 |

Energy drift at year 300 was 3e-13 or less, elements 4e-16 or less. No heat was clamped.

## Pictures

Each picture has five panels: map, temperature, rain, life (green plants, red animals), and
traffic.

- `watch-pre-m4/seed1_year0000.png`: the start, bare island.
- `watch-pre-m4/seed1_year0050.png`: year 50, during the cover collapse: plants on about 40%
  of land.
- `watch-pre-m4/seed7_year0070.png`: year 70, the worst collapse: plants on 15% of land.
- `watch-pre-m4/seed1_year0120.png`: year 120, plants back over most land.
- `watch-pre-m4/seed1_year0300.png`: year 300: lakes and rivers in the valleys, plants on
  about 76% of land. The traffic panel is all one colour: almost every cell is crossed.
- `watch-pre-m4/seed5_year0300.png`: year 300 of seed 5, for comparison.

## What emerged, and what did not

| Watch-for item | Seen? | Numbers |
|---|---|---|
| Rivers and lakes | **yes** | Water on land grows to 253 / 83 / 79 km³. 1,400-2,100 river cells. Low ground floods. |
| Climate bands | weak | North-south difference falls from 26 K to 3-4 K by year 100 (see "Decisions"). Almost no ice. |
| Light-eaters spread | **yes** | 20-35% of land before eaters, 71-84% by year 300. |
| Eaters of eaters | **yes**, in 3 of 3 | 8-15% of animal bodies take more from animals than from plants. 10-26% of animal deaths are "taken". |
| Kinds differ by climate | no | `kinds` stays 2. A third kind (small bodies) appears in seed 5 after year 207. |
| Populations rise, fall, cycle | rise and fall, no cycle | Animal members grow 100 to 10,000 times after year 75. No regular rhythm. |
| Extinctions | lineages yes | Over 99.7% of brain lineages die out. Both placed kinds live. |
| Trails | a net, not routes | 82-91% of land edges are worn and stay worn. The busiest 2% of cells carry 10-38% of crossings: no clear routes. |
| Seasonal movement | **no** | Seasons are strong (14-18 K) but animals move less than one cell. They live only where it is warm. |
| Learned behaviour spreads by imitation | partly | Watching reaches up to 39% of groups. Nothing shows that the behaviour is adopted. `learned` peaks at year 100, then falls. |
| Signals settle | weak | 0.07-0.16 bit of information. |
| Giving and exchange | giving yes, exchange no | Up to 47,000 gives a year (seed 1, late). At most 166 pairs give both ways. |
| Held objects as tools | a trace | Holding raises the take by 11-36%, but in under 0.1% of takes. Parts vanish after year 115. |
| Shelters in cold | **no** | No body is ever inside in the cold. |
| Floating structures | a trace | At most 2 things afloat. |

## Current problems

1. **Eaters wipe out plant cover.** After year 22, median cover stays near 0 for 40 to 65 years
   in all three seeds, then comes back (above 1 kg/m² by year 109-154). This is overgrazing:
   eaters kill plants 3 to 5 times faster than plants are born, then keep them down. It is a
   result of the physics as it stands, not a rule. The cold and size physics did not change it.
2. **Neutral eaters die out every time.** With no hand-set brain, eaters eat too little to pay
   for their brain before they can learn. So the eaters in these runs are a starting condition.
3. **Population nodes hide individuals.** Almost all animals are merged nodes (0 to 199 single
   animals at year 300). A node cannot hold, build or enter. This is why tools, shelters and
   floating things disappeared (the old code had up to 1,491 parts and 34 structures). Merging
   also averages genes, so plants barely evolve. Nodes never split back into individuals. This
   is the M4 work.
4. **Speed.** 10 to 16 s per simulated year (goal 0.6 s). Bodies are 93-95% of the time at year
   300. The cheap fixes are done; only population nodes that really skip work, or faster brains,
   can help.
5. **No seasonal travel and no shelter.** Animals live only in places that stay at 287-305 K, so
   cold never pushes them. Thin walls cannot keep a body warm; only thick walls can (+3.3 K in
   the test).
6. **Few kinds.** Two kinds most of the time. The second rung of the chemistry (fat) is never
   used: the fat reaction is 11 decades too slow for selection to find. Making it fast made the
   ecology worse, so we did not change it.
7. **Learning in the world.** `learned` rises to year 100, then falls by half. Brain size genes
   never change (always 12 hidden units).
8. **Code size.** Core is 5,983 lines (goal 5,000 through M3, 8,000 through M7). Flow is 1,212
   lines (goal 100-400 per law; it works best, so we kept it).
9. **Old snapshots do not load.** The saved format changed several times.

## Decisions for the owner

1. **Climate bands.** The island spans 50° of latitude in only 256 km. Air mixing (2e4 m²/s)
   then makes temperatures almost equal from north to south. This is correct physics for the
   numbers we chose, not a bug (the 15-day step had hidden it by mixing too little). For clear
   bands: a narrower latitude range, or less mixing. Which do you prefer?
2. **Methane.** The Change law makes a reaction "second order" in a scarce gas, so methane in the
   air lasts about 2 million years at today's amount (about 4 years with the usual first-order
   law). Methane is now 54-71% of the greenhouse gas and keeps rising. A fix changes every
   reaction. Should we change the rate law?
3. **Neutral eaters.** Keep the neutral start as default and report every run that uses
   `--eater-bias`? Or make neutral eaters viable (for example, a lower brain cost, or more
   starting stores)?
4. **Next step.** We recommend M4 (splitting nodes back into individuals where something happens,
   and the check that nodes give the same patterns), together with better learning and planning
   (option 2), and not new physics. Do you agree?

## Numbers in detail

The full analysis is in `watch-pre-m4/analysis.md`: tables by decade per seed, every watch
item with numbers, and a comparison with the runs before the last two bug fixes and with the old
code. The tapes themselves are not in the repository (about 1 GB each). To make them again:
`ew run --seed N --years 300 --every 10 --frames-every 5 --eater-bias --out DIR`.

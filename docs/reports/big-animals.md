# Why the roster's big animals die out

15 October 2026. Code: `main` at 81ad832 (law 2 smooth rate, rock nutrient, anchored erosion). Runs: 64 x 64,
60 years, `--every 1000`, roster and normal, seeds 1 to 3. The kinds: 8 (150 kg eaters of light-eaters, placed
at year 22), 9 (100 kg eaters of eaters) and 10 (600 kg eaters of eaters), both placed at year 26.

## Short answer

- **They starve, as founders, in their first year.** `SPENT` is 80 to 94% of the founders of kind 10, 62 to 77%
  of kind 9, 29 to 32% of kind 8. Few are eaten (`TAKEN` under 5% of deaths), none die of age early.
- **The main cause is a cost that grows faster than the body across kinds.** Tissue costs the same per kg of body
  at every size (the energy ceiling, m^1.0). That is right inside one body as it grows (West, Brown and Enquist
  2001). But it makes a kind's *adult* upkeep go with M^1.0 across kinds, while what it can take goes with M^0.75.
  Real adults pay Kleiber's M^0.75. The energy-ceiling row said that Kleiber "follows by itself across kinds": it
  does only for a kind at its ceiling, and no kind is near it (3.1 t).
- **Two smaller causes.** Founders are placed as juveniles: their body is 0.44 to 0.5 of their copy mass, so
  the first child needs 2 to 4 years of building at the best rate (kind 10 had no birth at all on seeds 1 and 2).
  And the rock element holds back 39 to 48% of the building tries of kinds 8 and 10 in their first year (animals
  get it only from the matter they digest).
- **Not the cause:** building speed. At `c_build` 1e4 a 481 kg adult makes 0.22 to 0.25 of its body a year
  (bison 0.15 to 0.25), a 44 kg eater 0.43 (deer 0.3 to 0.5). The "1,450" anchor of R6 would make it 0.03.
- **The node machinery** mixes kinds (below), which makes the tape of a kind that lives long hard to read.
  It is not why 9 and 10 die: their members are not lost to merges in the base runs.

## Founders, measured (32 x 32 world, two months after placement, means by body)

| kind | mass, kg | body built | copy mass | intake max / upkeep | build allowance / upkeep | fuel at placement lasts |
|---|---|---|---|---|---|---|
| 1 eaters (30 kg) | 28 | 18.5 | 50 | 3.23 | 3.8% | 115 days |
| 8 (150 kg) | 121 | 52 | 136 | 2.86 | 3.3% | 476 days |
| 9 (100 kg) | 87 | 52 | 151 | 2.27 | 2.8% | 111 days |
| 10 (600 kg) | 516 | 238 | 547 | 1.90 | 2.2% | 330 days |

Upkeep per kg of body at 290 K: the 238 kg founder pays 1.32 W/kg, 1.5 times Kleiber's resting rate for its
size (0.87 W/kg); the 18.5 kg eater 1.42 W/kg, 0.87 times Kleiber's (1.64). So the big kind pays 1.75 times too
much against the eater. With intake at 1.9 times upkeep, a founder must take at full rate on more than half of
its decisions only to stay even. The first-year `SPENT` share follows this ratio (and the runway of its fuel).

## Real animals (from memory, review 7, and the orchestrator's check)

- P/B: bison 500 kg 0.15 to 0.25 a year, deer 50 to 70 kg 0.3 to 0.5. Adult bison survive 0.92 to 0.95 a year
  (12 to 20 years of remaining life). Production goes with mass^0.755 (Ernest et al. 2003, 305 mammals).
- Generation in the model (child 0.3 of the copy mass to maturity, at pace 0.75): 4.5 years for 481 kg,
  then 0.6 births a year (bison: maturity 2 to 3 years, 0.5 to 0.8 calves a year). The model is a little slow,
  but in range: generation time is not what kills them.

## R4, R5, R6 of review 7

- **R5 (allowance and upkeep by body) is done:** the energy-ceiling row put supply (`Body::supply`, so intake and
  the `c_build` allowance), tissue upkeep and wear on `Body::built`. Only the brain's cost per decision still
  reads the whole mass.
- **R4 (the liquid shortcut) is only half closed.** The liquid is bounded at h / (1 - h) of the matter, and size,
  built body, supply and upkeep all read the body. But the keep gene h itself is free up to 0.95, and selection
  takes it there: from 0.75 at the start to 0.93 to 0.95 in light-eaters and small animals by year 30 to 60
  (roster seed 1, snapshots). A body at h 0.95 is 20 times its matter, not 4, so its maturity and its children
  cost a fifth of the matter. The big founders stayed at 0.72 to 0.80 in their few generations, so it is not why
  they die; it helps the small kinds that replace them. Real water shares (from memory): mammals 0.6 to 0.7,
  herbs 0.8 to 0.9, jellyfish 0.95. What is left: a choice of law (maturity and a child counted in matter, or
  building the liquid's room costing work), the owner's.
- **R6 (`c_build` to 1,450) is not built, and `c_build` is anchored at 1e4 instead.** P/B of real mammals from
  20 g to 500 kg gives 5,800 to 18,000 per kg^0.75 (`docs/numbers.md`), Ernest's maximum 4,400 to 26,000 at 50 kg.
  The 1,450 counted only a newborn's matter at birth. Tuned numbers 63 -> 62.

## Experiments (roster; kg of each kind at year 60, and the last year with members)

| run | change | seed | kind 8 | kind 9 | kind 10 | kind 10, first-year `SPENT` |
|---|---|---|---|---|---|---|
| base | main 81ad832 | 1 / 2 / 3 | 0 (39) / 0 (58) / 0 (47) | 0 (35) / 0 (31) / 0 (32) | 0 (30) / 0 (30) / 0 (32) | 80 / 89 / 94% |
| K | Kleiber across kinds (below) | 1 / 2 / 3 | 1.7e6 / 5.5e4 / 7.8e7 | 0 (39) / 0 (59) / 0 (40) | 0 (48) / 0 (42) / 0 (37) | 41 / 49 / 57% |
| A | adult founders: copy-mass gene = placed body, every row | 1 | 4.9e5 | 0 (35) | 0 (35) | 82% |
| U | no unlike merges in the monthly merge | 1 | 0 (39) | 8.8e7 (as 1.7 kg animals) | 0 (33) | 85% |
| A+U | | 1 | 2.1e8 | 0 (30) | 0 (29) | 91% |
| K+A+U | | 1 | 1.5e7 | 3.3e6 | 0 (53) | 46% |

- **K**, the one law tested: tissue costs (basal, catalysts, strength, learning, signal) per kg of body times
  (copy mass / 44 kg)^-0.25, 44 kg being the eater founder's copy mass where the costs are anchored. One line in
  `body::upkeep`, no new number (the exponent is 1 - `intake_exp`). Inside a body costs still go by its kg (the
  ceiling stays); across kinds adults then pay Kleiber's M^0.75 (WBE: the maintenance per kg is a trait of the
  kind, smaller for a bigger kind). It halves the founders' starvation and keeps kind 8 on all three seeds, but
  kind 10 still dies, 7 to 18 years later. It also makes a 1.4 kg plant pay 2.4 times more per kg: kind 0 of the
  roster 1.3e9 / 4.6e9 / 2.1e9 kg against 6.5e9 / 4.8e9 / 7.5e9, and its copy mass rises (1.3 to 5 kg).
- **A** gives births in the first year (kind 10: 22 against 0) but does not stop the starving.
- **U**: the monthly merge joins unlike nodes past twice `crowd` (8 nodes in a cell). In the roster that is
  3.3e10 members a year (base, seed 1): nodes of different kinds pool their genes and the larger one keeps its
  label. With A, root 9 gained 1.6e6 members by merges in one year while its copy mass fell from 52 to 2 kg.
  Kinds 9 and 1 are also alike within `alike` in every gene, so they interbreed and join each other's nodes.
- Speed: base 6.0 s a year (seed 1), K 5.2, U 5.2, K+A+U 4.8.

## Roster at year 60 and normal runs (code of this change = main: nothing in the laws changed)

Roster, kg at year 60, seeds 1 / 2 / 3 (kinds 2, 3 not placed or gone): 0: 6.5e9 / 4.8e9 / 7.5e9; 1: 6.0e8 /
1.0e7 / 0; 3: 0 / 0 / 1.4e8; 4: 8.2e10 / 6.1e10 / 6.5e10; 5: 2.8e8 / 2.2e9 / 5.1e8; 6: 1.8e8 / 0 / 0;
7: 1.8e9 / 2.2e9 / 4.5e9; 8, 9, 10: 0. Carbon gas 300 / 299 / 309 ppm. The same after (no law changed).

Normal runs (means of years 56 to 60, seeds 1 / 2 / 3): plants 3.7e10 / 3.8e10 / 5.0e10 kg, adult life 1.27 /
1.02 / 0.98 y; eaters 2.3e9 / 1.9e9 / 1.3e9 kg, adult life 2.08 / 1.26 / 1.53 y; carbon gas 334 / 330 / 307 ppm
(the same as the law-2 report's runs on this code). With K (seed 1): plants 3.9e10 kg, adult 0.83 y; eaters
4.2e9 kg, adult 2.58 y, body 12.6 kg (20.7); carbon gas 336 ppm.

## For the owner

1. **Kleiber across kinds (K)** is the physics the cause asks for, and it is one line. It changes every kind's
   costs, plants included, so it is your choice. If taken, `c_basal` may come back to its review-5 form
   (2e4 per kg^0.75 at 1 kg, Kleiber's shape) instead of the tuned 12,208.
2. **Founders as adults (A)**, set from their own genes: an initial condition, honest, but it changes the
   normal run's founders too (plants mature at 0.7 kg, eaters at 21 kg).
3. **No unlike merges (U)**: nodes then never pool unlike kinds, as individuals never do; not slower here.
4. **The liquid keep gene (R4)** and **the rock element for animals** are open.
None of these, alone or together, kept the 600 kg kind for 60 years on seed 1. Not tuned, as asked.

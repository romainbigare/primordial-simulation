# A rock nutrient in body matter

15 October 2026. Branch `worktree-agent-a03759468385e6516`, from main `03123d8`. The owner asked for
two nutrients, built from plain chemistry with no hard-coded mechanics: nitrogen (done, on main)
and a rock nutrient. This report is about the rock nutrient. Runs: 64x64, 60 years, `--every 1000`,
seeds 1, 2, 3 and `--roster` on seed 1, main `03123d8` against the branch. Numbers are the mean of
years 56 to 60 unless said otherwise.

## Short answer

**Update, after the owner's decision to anchor erosion (section "Erosion anchored" below):** with `carry`,
`wear` and `creep` set to real sizes, the rock element holds back 0 to 7% of the plants' building (it was 53 to
62%), the lime stays on the land (median 2,690 mol/m2 at year 10, 2,570 to 2,600 at year 60), the eaters live in
all three seeds, and the island loses 0.003 to 0.07 mm of ground a year (it lost 7 to 12 mm). The older
text below describes the run with the first-guess erosion.

- **Body matter now holds the rock element** (element 5 of chemistry 0), one atom in every
  molecule of matter. Growth takes it from substance 23, the salt that lives only dissolved in
  liquid. Roots take it up the way they take the nitrogen mineral (24). Digestion gives it back.
  Rotting gives it back as lime (substance 10).
- **The amount is 1.16% of the dry mass.** Real land plants hold 0.5 to 2%. To get there I made
  the unit of matter three times bigger (C120H196N6O106Ca, 3,456 g/mol). With the old unit the share
  would be 3.5%.
- **The code got smaller, not bigger.** Everything that was written for "the" nitrogen mineral is
  now written for a list of minerals (`Chemistry::minerals`). Core went from 8,549 to 8,548 lines.
  No code names the element.
- **Life does not collapse.** Plants are about as heavy as in main (kg within -14% to +3%).
  They live longer. They are fewer in number and bigger. Eaters are 3 to 5 times lighter in
  all three seeds, and in seed 2 they die out. Seeds differ in main by 100 times for eaters, so
  I cannot say the rock element did this. In the roster the first kind of eater lives and in
  main it does not.
- **The rock element limits building a lot, more and more with the years.** It holds back 4% of
  the plants' building attempts at year 5, 27% at year 10, 63% at year 60 (seed 1; means of years 56 to 60,
  seeds 1 to 3: 53 to 62%). Nitrogen holds back 10 to 13% (main: 19 to 52%). I traced the cause (see
  "Follow-up" below). It is on the land and it is already in main. **Running liquid erodes the loose
  ground** (the tuned, first-guess numbers `carry`, `wear`, `creep`, which are 1,000 times or more above real
  erosion). The median land cell holds 1,715 mol of lime a m2 at year 10 and 103 at year 60. Below about
  110 mol/m2 of lime, **the reaction that makes the salt (22) goes 6,000 times slower**, because of how the
  rate law counts a gas that is not the scarcest input. With erosion cut 100 times (a diagnostic, not
  a fix) the rock element holds back only 1 to 3% and eaters live in all three seeds.
- **Speed is the same.** 3.05 seconds a year against 3.13 (seed 1, 30 years).
- **A surprise, and a rule it needed.** The salt does not last inside a body. It falls out as
  lime in hours to days. Without a rule, plants filled up with lime (2% of their mass at year
  6 to 20, 16 times the rock element their matter holds). Now a body gives lime back to the ground
  when it updates. This is the same rule that already sent the extra nitrogen mineral back.

## What land pores hold (the first question)

Land starts with no salt 23. At the start land has no liquid, and the rule of worldgen
(`Reactor::balance`) gives a cell with no liquid no salt. This is right: salt needs liquid.

After one year of rain the pores of every wet land cell hold salt. The same two reactions as the sea's
(22 and 23) make it from the lime of the loose ground (a tenth of it is lime), the carbon gas
over the cell and the liquid. A test checks it: 93 wet land cells, the salt is 0.91, 1.10 and
1.40 times the balance of the cell at the 10th, 50th and 90th percent. No dry cell holds any.

So land does not need the salt at the start. By year 2, when life starts, the land holds 0.72
kg of the salt on each m2 (4.4 mol). It rises to 1.5 kg at year 60. The sea holds about 70 to 80
kg on each m2.

| salt 23, kg/m2 | year 1 | year 3 | year 10 | year 30 | year 60 |
|---|---|---|---|---|---|
| land, main | 0.53 | 0.73 | 0.82 | 1.33 | 1.48 |
| land, rock nutrient | 0.53 | 0.73 | 0.86 | 1.31 | 1.52 |
| sea floor cells, main | 79.6 | 79.3 | 73.8 | 67.7 | 69.3 |
| sea floor cells, rock nutrient | 79.6 | 79.3 | 74.3 | 68.6 | 69.1 |

(Seed 1. The plants hardly change the salt in the ground: they hold very little of it.)

## What was built

### The chemistry (`worldgen/src/chem0.toml`)

| part | before | now |
|---|---|---|
| body matter, 22 | C40H66N2O34, 1,118 g/mol | **C120H196N6O106Ca, 3,456 g/mol** |
| growth | 40 of 5 + 1 of 24 -> 1 of 22 + 9 of 0 | **118 of 5 + 3 of 24 + 1 of 23 -> 1 of 22 + 27 of 0** |
| hf of 22 | -6,271 kJ/mol | **-20,315.8 kJ/mol** |
| burning matter, 21 | 22 + 40 O2 -> 40 CO2 + 31 H2O + 24 | **22 + 118 O2 -> 119 CO2 + 92 H2O + 3 of 24 + 1 of 10** |
| energy of burning | 16.7 MJ/kg | 15.9 MJ/kg |
| carbon to nitrogen | 20 | 20 |
| rock element | none | 1 atom, 1.16% of the mass |

Checks (all in `core/tests/nitrogen.rs`):

- Every reaction and growth balance in elements (`Chemistry::validate`).
- Growth is thermoneutral. hf(22) = 118 x -212 + 3 x -365 + (-1,926.8) - 27 x -286 = -20,315.8.
  Matter holds the energy of what it was made of.
- The salt's two carbons go into the matter (118 + 2 = 120 carbons). Growth gives off only liquid.
  No gas has to go anywhere.
- Rotting (21) gives back three of the nitrogen mineral and one of lime. A year at 290 K: 195 of
  1,000 mol rot, 586 mol of the mineral and 195 mol of lime come back. Hot (1000 K) the nitrogen
  leaves as gas (3,000 mol of N2) and all 1,000 mol of the rock element stay in solids.
- The rate of rotting did not change. It is per mol of matter, so per kg it is the same.

### Why lime and not the salt, when rotting

The task allowed lime (10) or salt (23, if liquid is an input). I chose lime. If liquid were an
input, dead matter in dry places would not rot, and fires would burn slower in dry air. That would
change two things nobody asked to change. Lime meets liquid and gas in the ground and becomes the
salt by reaction 22, the sea carbon's own rule.

### Why three times the unit

One rock atom in each molecule of matter gives:

| unit of matter | mass | rock element share |
|---|---|---|
| 1 (as it was) | 1,160 g | 3.5% |
| 2 | 2,300 g | 1.7% |
| **3** | **3,456 g** | **1.16%** |

The anchor: dry matter of land plants holds 0.5 to 2% of this element (Marschner, *Mineral Nutrition
of Higher Plants*: about 0.5% is enough for most plants, broad-leaved plants hold 1 to 2%; **from
memory, not checked**). The unit 3 is the one nearest the middle of the range. It also keeps the
nitrogen in whole counts (6 atoms, 3 of the mineral). One matter serves all bodies, so this is the
plants' figure. Animals hold more (bone).

### The code (generalised, not copied)

| place | what changed |
|---|---|
| `core/src/chem.rs` | `mineral()` (one) became `minerals()` (a list: the inputs of growth that no reaction burns with a gas: 24, then 23) |
| `core/src/body.rs`, `shed` | for each mineral, what is held past the level goes back to the ground. New: anything else a body holds that it does not burn, is not matter, not gas, not its key liquid, goes back, all of it (this is the lime) |
| `core/src/choose.rs` | digestion room ignores the minerals (a list now) |
| `core/src/copy.rs`, `core/src/lib.rs` | the tally of what held building back is [tried, any mineral, the first, the second] |
| `worldgen/src/life.rs` | founders hold and keep every mineral, light-eaters have the uptake gene on for each; `fixing` looks at the list |
| `tape/src/rec.rs` | columns for each mineral: `min0_*`, `min1_*`, `nlim0`, `nlim1`, `nstore0`, `nstore1` |

Roots and `Body::digest` needed no change: they already loop over the uptake genes and the
inputs of growth. Fixing (reaction 24) and the founders are not changed. A tenth of the light-eaters
still carry the fixing gene at 8.6. The gene is for the nitrogen mineral only, as before.

Core lines: 8,549 before, 8,548 after.

## What happens to the salt inside a body

This was the part I did not expect, so I measured it.

Reaction 23 (3 salt -> 3 lime + 3 CO2 + 3 water) has no partner in a body. In the ground, lime and
gas make the salt back (22), so the ground keeps a balance. In a body there is no lime. The
salt just falls out. Its rate goes with the salt's share of the body's liquid, squared. A full
store (twice the rock element of the body's matter) loses half in about 5 hours. A store of a
tenth of that loses half in about 3 weeks.

What this means for life:

- A plant cannot keep a store of the salt. Roots refill it at each update (every 5 to 20
  days). Growth uses what roots just brought.
- Before I added the rule below, the lime stayed in the bodies. At year 6 to 20 (seed 1) plants
  held 2% of their mass as lime, which is 16 times the rock element in their matter. The worst body held 16 to 33%
  of its mass as lime.
- Now `body::shed` sends it back to the ground at the update. After the change plants hold
  1.5 x 10^6 kg of lime in 3.1 x 10^10 kg (0.005%). A small test world grew 25% more plant
  members in a year.

The nitrogen mineral does not have this problem (it is stable in a body).

## Results (years 56 to 60, main > rock nutrient)

### Plants (kind 0)

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| members | 3.0e10 > 2.5e10 | 4.0e10 > 1.8e10 | 4.4e10 > 3.2e10 |
| kg | 3.15e10 > 3.09e10 | 4.52e10 > 4.66e10 | 4.25e10 > 3.64e10 |
| bodies | 3,365 > 2,926 | 3,046 > 4,747 | 3,519 > 3,114 |
| matter share of mass | 5.8% > 4.7% | 4.9% > 4.1% | 5.3% > 5.1% |
| life, real / adult (years) | 0.46/0.89 > 0.69/1.19 | 0.41/0.78 > 0.53/0.77 | 0.38/0.90 > 0.56/1.07 |
| deaths, % by cause (main > rock) | spent 64, taken 13, broken 15, age 8 > spent 62, taken 2, broken 22, age 13 | spent 70, taken 4, broken 19, age 7 > spent 71, taken 2, broken 16, age 12 | spent 72, taken 2, broken 19, age 6 > spent 64, taken 2, broken 23, age 10 |
| births by month, % | 8 7 8 10 10 10 9 8 8 8 7 6 > 8 7 9 11 11 11 10 8 7 7 7 6 | 8 6 8 10 11 11 10 9 8 8 7 7 > 7 7 8 9 10 11 10 8 9 8 7 6 | 8 7 8 10 11 10 9 9 8 8 7 6 > 8 7 9 10 11 11 9 9 7 6 6 6 |
| share with the fixing gene, year 60 | 62% > 54% | 74% > 35% | 69% > 47% |

Plants are about as heavy, a little fewer in number and a little bigger. They live longer (more
die of age, fewer are taken). The spring peak of births is the same.

### Eaters (kind 1)

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| members | 3.6e7 > 9.5e6 | 1.95e5 > 4 (no body) | 2.8e7 > 5.4e6 |
| kg | 2.9e9 > 6.1e8 | 2.5e7 > 197 | 1.4e9 > 2.6e8 |
| bodies | 2,291 > 1,411 | 158 > 0 | 1,009 > 401 |
| life, real / adult (years) | 0.74/1.13 > 1.29/2.47 | 1.32/1.34 > 1.18/2.73 | 0.54/1.34 > 0.79/1.36 |
| deaths, % by cause | spent 69, taken 1, broken 8, age 22 > spent 54, taken 2, broken 14, age 30 | spent 50, taken 1, broken 16, age 33 > spent 57, phase 5, age 38 | spent 73, taken 1, broken 8, age 18 > spent 56, taken 4, broken 11, age 29 |
| births by month, % | 5 4 4 7 11 13 13 11 10 9 7 5 > 6 4 4 6 9 11 12 12 11 10 9 7 | 4 2 3 4 6 11 17 15 13 11 9 6 > none | 5 3 4 6 9 12 13 12 12 11 8 5 > 5 4 4 6 10 12 16 12 10 8 7 6 |
| rock nutrient only: share of their building attempts held back by the rock salt | 0.13 | 0.07 | 0.15 |

Eaters grow slower from the start. Members at year 40 in seed 1: main 2.2e6, rock 1.1e5. At year
60 they are still growing in seeds 1 and 3. In seed 2 they die out (main's eaters in seed 2 are
only 2.8e5 members at year 60 and were at 370 at year 35, so that seed is on the edge in both).

**I do not know if the rock element did this.** Three things push the same way: the matter
gives 4.6% less fuel when digested (the salt's carbon and the minerals are not fuel); the salt
is lost inside a body before use (eaters are held back by the rock salt in 7 to 15% of their
attempts); plants are a little fewer. Against this: in main the same seed gives 100 times more eaters than the next seed,
so a 3 to 5 times drop is inside the spread of seeds; the roster shows the opposite for kind 1. I did not run a control
that differs in nothing physical (the sea-carbon report found such runs differ as much as main
and a branch).

### Carbon gas, ppm, years 10 to 60

| | 10 | 20 | 30 | 40 | 50 | 60 |
|---|---|---|---|---|---|---|
| seed 1, main | 266 | 282 | 294 | 305 | 326 | 338 |
| seed 1, rock | 284 | 297 | 308 | 316 | 320 | 322 |
| seed 2, main | 246 | 264 | 279 | 283 | 285 | 286 |
| seed 2, rock | 285 | 291 | 298 | 307 | 306 | 296 |
| seed 3, main | 268 | 286 | 297 | 306 | 311 | 323 |
| seed 3, rock | 294 | 292 | 302 | 306 | 308 | 311 |
| roster, main | 263 | 248 | 303 | 308 | 307 | 304 |
| roster, rock | 284 | 265 | 276 | 295 | 310 | 315 |

The air falls less in the first decade (+18 to +39 ppm at year 10). Plants build slower at first
because the rock salt limits them (see below). Later the two agree within the spread of seeds.

### How often each mineral holds back building (plants)

The tape shows the share of building attempts that a mineral held back. "Any" is held back by the
nitrogen mineral, the rock salt, or both.

| plants, years | 5 | 10 | 20 | 40 | 60 |
|---|---|---|---|---|---|
| seed 1: any / nitrogen / rock salt | .64 / .63 / .04 | .39 / .18 / .27 | .52 / .16 / .44 | .60 / .12 / .55 | .66 / .10 / .63 |
| seed 2 | .63 / .60 / .07 | .45 / .29 / .20 | .37 / .03 / .35 | .54 / .06 / .51 | .63 / .16 / .57 |
| seed 3 | .78 / .74 / .08 | .47 / .27 / .24 | .49 / .18 / .38 | .55 / .12 / .49 | .57 / .09 / .53 |

In main (nitrogen only) the figure at years 56 to 60 is 52, 19 and 42%. With the rock salt, nitrogen
holds back 10, 13 and 10%, and the rock salt 62, 56 and 53% (means of years 56 to 60, seeds 1 to 3). At first (year 5) nitrogen is the limit, as in main. By year 10 the
rock salt takes over.

Why so often? I looked at the snapshots of seed 1 (every 10 years), main and branch alike.

| land, seed 1 | year 10 | year 20 | year 40 | year 60 |
|---|---|---|---|---|
| median land cell, lime in mol/m2 (main) | 1,715 | 382 | 136 | 103 |
| median land cell, lime in mol/m2 (rock nutrient) | 1,717 | 367 | 133 | 101 |
| land cells with under 0.001 mol/m2 of the salt (main) | 18% | 31% | 43% | 58% |
| land cells with under 0.001 mol/m2 of the salt (rock nutrient) | 22% | 37% | 48% | 53% |
| plant bodies standing on such cells (main) | 18% | 31% | 45% | 61% |
| plant bodies standing on such cells (rock nutrient) | 22% | 37% | 48% | 53% |

(At the balance a wet cell holds about 1 mol of the salt a m2.) The limit follows the loss: in the
branch it is 0.04, 0.27, 0.44, 0.55 and 0.63 of the building attempts at years 5, 10, 20, 40 and 60.
Nothing needed the rock element in main, so nobody saw the loss. The next section says what takes
the lime and why the salt does not form. (An earlier version of this report said "rain washes the
lime out". That was wrong: it is erosion, not solution.)

In the cells with 1 to 3 mol of the salt per m2, plants hold 56% of their level.

### Roster (seed 1, members, main > rock nutrient)

| kind | main | rock nutrient |
|---|---|---|
| 0 light-eaters | 1.0e10 | 5.3e9 |
| 3 small fast light-eaters | 7.4e9 | 2.9e10 |
| 4 big slow light-eaters | 2.7e9 | 2.3e9 |
| 1 eaters | none | 1.4e5 |
| 5 small eaters of remains | 5.0e9 | 3.4e8 |
| 6 big eaters of remains | 6.4e7 | none |
| 7 small fast eaters of light-eaters | 4.2e8 | 6.1e8 |
| 8 big slow eaters of light-eaters | none | 620 |
| carbon gas, year 60 | 304 ppm | 315 ppm |

Kinds 2, 9 and 10 are gone in both.

## Follow-up: why the land has no salt, and where the lime goes

The coordinator asked three questions after the first report. No code fix came out of them. The
answers are below, with the numbers. All from seed 1; the snapshots were remade (same hash as before).

### 1. Why do land cells with liquid and lime hold almost no salt?

It is not the scheduler, and not the roots. I took cell 549 of the year-60 snapshot (285 K, 1,380 mol/m2 of
liquid, 11.8 mol/m2 of lime, salt 1e-5 mol/m2) and called the reactor on it directly for one month, with
the air's gas as the pool. Then I raised the lime and did it again:

| lime in the cell (mol/m2) | salt made in a month (mol/m2) | per mol of lime and month |
|---|---|---|
| 0.6 | 2.7e-6 | 4.6e-6 |
| 11.8 | 5.4e-5 | 4.6e-6 |
| 58.9 | 2.7e-4 | 4.6e-6 |
| **235.6** | **1.59** | 6.7e-3 |
| 1,178 | 1.59 | 1.4e-3 |

The salt made is **in proportion to the lime, at a very small rate, until the lime in the cell passes the
carbon gas of the air column over it** (4.5e8 mol, 110 mol/m2). Above that it jumps by a factor of 6,000 and
stops at the balance (1.6 mol/m2). A month in one go, so the scheduler is out of it. Why: Change's rate is
"per unit of the scarcest input, times the share of the other gases". If the gas is the scarcest, its
share is not counted (the sea's anchor: 0.19 a year per unit of the gas over it). If the lime is the
scarcest, the gas share is counted, and carbon gas is 4 parts in 10,000 of the air: 4e-4 (2,500 times less).
Two regimes, with a jump between them. A lime-rich cell (the sea floor, the young land) is in the first.
A cell that has lost its lime is in the second, and there the salt cannot form as fast as the liquid carries
it away or the roots take it. In the world: of the salt made on land in a year (3.1 mol/m2 on average at
year 10), the roots take 1.9 and give back as lime (see "What happens to the salt inside a body"), the
liquid takes 1.1 to the lower cells.

I did **not** change the law. A continuous version (compare the lime with the whole gas of the cell, not
with the carbon gas) puts the sea floor cells (about 1e10 mol of lime, against 1.1e12 of gas) in the slow
regime too, about 100 times slower than the sea carbon anchor, and the burning of litter is anchored to the
second regime (the 0.21 of the oxygen-like gas). So it needs a new anchor, not a bug fix. It is the
owner's call. A smaller point: `Reactor::balance` skips a substance if the air holds even a trace of it
(1e-34 mol of the salt, as vapour, by year 60); worldgen calls it only at the start, so it is harmless.

### 2. What takes the lime from the land?

Erosion by running liquid, not solution. Per m2 of land per year (seed 1; lime in mol):

| | year 10 | year 40 |
|---|---|---|
| Flow: lime carried by liquid and by creep, net on all land cells | +10.9 | +27.3 |
| Change (reaction 22 and 23): lime dissolved, all land | -3.0 | -2.0 |
| bodies (roots take the salt, give lime back, `shed`) | +1.7 | +1.0 |
| bed lime worn into loose ground (Flow) | 27.7 | 45.9 |

The whole land gains lime (the bed wears into loose ground faster than the loose ground is carried to
the low cells). The cells that have less than the median (a half of the land) lose it. I ran one year of that
half with a thing switched off each time (mol/m2 of lime in the year):

| below-median land cells | year 10 | year 40 |
|---|---|---|
| all as it is | -29.0 | -6.2 |
| creep off | -24.4 | -6.4 |
| liquid-borne solids off (`carry` 0) | **-3.0** | **-0.05** |
| both off | -1.6 | +0.1 |
| the Change step off | -27.6 | -6.9 |

So 89% (year 10) to 99% (year 40) of the loss is loose solids carried by running liquid; creep is
a sixth at year 10; chemistry is 1.4 mol/m2 at year 10 and about nothing later.

Is it plausible? The chemistry is. 3.0 and 2.0 mol/m2 a year are 0.11 and 0.075 mm a year of pure lime
(27 mol/m2 for a mm), which is 110 and 75 mm per thousand years, at the top of real chemical denudation of
limestone, 10 to 100 mm per thousand years (and this ground is lime-rich). The erosion is not. The
below-median cells lose 29 mol/m2 of lime a year, 1.1 mm a year of pure lime; with the ground a tenth lime,
that is about 10 mm of ground a year, 1 m in a century. The bed lime lost under the land (27.7 mol/m2 a year)
is the same size again. Real soils lose 0.1 to 1 mm in a century, so the world erodes 1,000 to 10,000
times faster. `carry`, `wear` and `creep` are in `docs/numbers.md` as "tuned, first guess"
(the creep row says itself that 1e-6 m/s is 30 m a year at a slope of 1, far above soil creep). It is not a
bug in the lime: it is the tuned erosion, seen by a nutrient for the first time.

### 3. Rerun

No fix changed the code, so I did not rerun the branch. I ran **a diagnostic, not a fix**: the same
branch with `carry`, `wear` and `creep` 100 times smaller (compiled for the run, not committed; closer to
real, and the landscape changes less). 64x64, 60 years, seeds 1 to 3, years 56 to 60 (main > branch > erosion / 100):

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| plants, members | 3.0e10 > 2.5e10 > 4.6e10 | 4.0e10 > 1.8e10 > 1.7e10 | 4.4e10 > 3.2e10 > 5.3e10 |
| plants, kg | 3.15e10 > 3.09e10 > 4.86e10 | 4.52e10 > 4.66e10 > 4.87e10 | 4.25e10 > 3.64e10 > 5.17e10 |
| plant matter share | 5.8 > 4.7 > 5.2% | 4.9 > 4.1 > 6.2% | 5.3 > 5.1 > 6.2% |
| plant life real / adult (y) | .46/.89 > .69/1.19 > .52/1.10 | .41/.78 > .53/.77 > .45/.70 | .38/.90 > .56/1.07 > .43/.96 |
| eaters, members | 3.6e7 > 9.5e6 > 5.2e7 | 1.95e5 > 4 > 2.2e7 | 2.8e7 > 5.4e6 > 2.6e7 |
| eaters, kg | 2.9e9 > 6.1e8 > 3.1e9 | 2.5e7 > 197 > 1.5e9 | 1.4e9 > 2.6e8 > 1.4e9 |
| plants held back: any / nitrogen / rock salt | (.52 N only) > .66/.10/.63 > .08/.07/.01 | (.19) > .61/.13/.56 > .08/.06/.03 | (.42) > .58/.10/.53 > .16/.15/.02 |
| eaters held back by the rock salt | - > .13 > .01 | - > .07 > .13 | - > .15 > .08 |
| salt on land, kg/m2, years 1, 10, 30, 60 | main .53 .82 1.33 1.48 > .53 .86 1.31 1.52 > .53 .94 1.47 1.82 | - > .51 .79 1.22 1.44 > .51 .92 1.38 1.63 | - > .52 .60 .80 1.11 > .52 .67 .88 1.17 |
| carbon gas, years 10 to 60 (ppm) | 266 ... 338 > 284 ... 322 > 260 259 270 272 282 308 | 246 ... 286 > 285 ... 296 > 293 284 293 299 302 317 | 268 ... 323 > 294 ... 311 > 265 270 273 278 284 295 |

With the erosion near real the rock element holds back 1 to 3% of the plants' building, the eaters live
in all three seeds (also seed 2, 2.2e7 members), and plants are heavier than in main (4.9 to 5.2e10 kg).
The rock nutrient therefore does not limit life when the lime stays on the slopes. It limits it here
because of the world's fast erosion. I did not retune anything. The owner may want to anchor `carry`,
`wear` and `creep` (the rows say "none found"): that is a change to how fast valleys form.

## Erosion anchored

The coordinator asked for this after the follow-up, and the owner agreed: `carry`, `wear` and `creep`
become real sizes and leave the tuned numbers. The Law 2 jump (reaction 22 going 6,000 times slower
below 110 mol/m2 of lime) is another study. I did not touch `change.rs`.

### The three numbers

| number | what it does in the code | before | now | where it comes from |
|---|---|---|---|---|
| `creep` | creeping solids move a volume of creep x softness x thickness x slope per width and time, so the flux is D x slope with D = creep x softness x thickness | 1e-6 m/s (D = 48 m2/yr) | **6.3e-11 m/s (D = 3e-3 m2/yr)** | Hillslope soil diffusivity: 1e-3 to 1e-2 m2/yr (Roering et al. 1999, Oregon, 3 to 5e-3; Fernandes and Dietrich 1997; Hanks 2000; **from memory**). I took 3e-3, the middle in the logarithm. The starting loose ground is 1.00 m thick and has softness 1.53 (a test measures it) |
| `carry` | the volume running liquid carries off a cell is carry x the J it dissipates | 1e-7 m3/J | **2e-11 m3/J** | The Earth's rivers carry 15 to 20 Gt of sediment a year (Milliman and Syvitski 1992; **from memory**), 7e9 m3. Their runoff, 4e13 m3 a year, falls a mean 840 m: 3.4e20 J. So 2e-11 m3 per J (1 to 3e-11) |
| `wear` | the bed breaks wear x (volume share / bond strength) per J | 3e-9 | **1.4e-11** | The same ratio, 2e-11 m3/J, for the starting bed: its shares over strengths add to 1.39, so 2e-11 / 1.39 |

Two things to know. (1) The most a flow can carry is more than its yield. Bagnold's relation (volume =
e x energy / (L x 1,650 x 9.81 x 0.63), e = 0.1, L = 2 km, from memory) gives 5e-9 m3/J, 250 times the
yield. I took the yield, because this model has loose ground everywhere and nothing that holds it. With the
capacity, a flow strips loose ground at about 8 mm a year. A real river carries far less than it could,
because its load is bound. (2) The old numbers were 5,000 times (`carry`), 200 times (`wear`) and
16,000 times (`creep`) the new ones. All three are in `docs/numbers.md` as **anchored**, and the tuned count
fell from 66 to 63 (`TUNED_BUDGET` in `core/tests/numbers.rs`). `hide`, `still`, `split` and `hold` are
settings of the routing and of the cover, not rates of erosion. They stay tuned.

### Check, not tune: how fast does the island lose ground?

I measured the rock-making solids (the loose ground and the bed) that leave the land cells, from the
snapshots, as a depth over the land. Mean since the start, mm a year:

| | year 10 | year 30 | year 60 |
|---|---|---|---|
| first-guess numbers, seed 1 | 9.6 | 7.9 (year 20: 8.5) | - |
| first-guess numbers (main), seed 2, seed 3 | 12.3, 11.8 | - | 6.7, 6.8 |
| **anchored, seed 1** | 0.007 | 0.023 | 0.044 |
| **anchored, seed 2** | 0.014 | 0.007 | 0.033 |
| **anchored, seed 3** | 0.003 | 0.021 | 0.045 |

The rate in the late decades is higher than in the first: seed 1 lost 0.007 mm a year in years 0 to 10, about 0.03 in
years 10 to 30 and about 0.065 in years 30 to 60. So **the island lands at 0.003 to 0.07 mm a year: inside the
real range of 0.01 to 1 mm a year from year 10 on, and below it in the first decade of seeds 1 and 3**. The
Earth's mean is about 0.05. I did not look at why the first decade is slow. The lowest and highest cells (the surface
change per cell, 99th percentile and the maximum): 0.3 to 0.4 and 0.4 to 0.7 mm a year. The mountains (2,200 to 2,500 m)
do not reach the few mm a year of the fastest real ones. The first-guess numbers gave 90 to 150 mm a year at the 99th percentile and up to 240 at the maximum.

A small island (a 32x32 map, 10 years, a test in `core/tests/erosion.rs`) loses 0.1 mm a year.

### What it does to the rock nutrient and to life (64x64, 60 years, years 56 to 60)

Main (03123d8) > rock nutrient with the first-guess erosion > rock nutrient with the anchored erosion.

| plants | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| members | 3.0e10 > 2.5e10 > 3.7e10 | 4.0e10 > 1.8e10 > 2.3e10 | 4.4e10 > 3.2e10 > 4.8e10 |
| kg | 3.15e10 > 3.09e10 > 4.27e10 | 4.52e10 > 4.66e10 > 7.72e10 | 4.25e10 > 3.64e10 > 4.92e10 |
| bodies | 3,365 > 2,926 > 2,586 | 3,046 > 4,747 > 3,628 | 3,519 > 3,114 > 3,004 |
| matter share | 5.8 > 4.7 > 6.2% | 4.9 > 4.1 > 4.2% | 5.3 > 5.1 > 5.6% |
| life real / adult (y) | .46/.89 > .69/1.19 > .61/1.16 | .41/.78 > .53/.77 > .72/1.13 | .38/.90 > .56/1.07 > .46/1.01 |
| deaths, % (last column) | spent 63, taken 1, broken 26, age 9 | spent 55, taken 3, broken 29, age 12 | spent 67, taken 1, broken 26, age 6 |
| births by month, % (last column) | 8 7 8 10 12 10 9 9 7 8 7 6 | 7 6 6 8 9 10 10 10 9 9 8 7 | 9 7 9 10 11 10 8 8 8 7 7 6 |
| held back by any mineral / nitrogen / rock salt (last two columns) | main: nitrogen only .52; old .66/.10/.63; new **.05/.05/.00** | main .19; old .61/.13/.56; new **.59/.57/.07** | main .42; old .58/.10/.53; new **.11/.11/.01** |
| share with the fixing gene, year 60 | 62 > 54 > 63% | 74 > 35 > 15% | 69 > 47 > 55% |

| eaters | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| members | 3.6e7 > 9.5e6 > 5.7e6 | 1.95e5 > 4 > 1.15e7 | 2.8e7 > 5.4e6 > 7.9e6 |
| kg | 2.9e9 > 6.1e8 > 6.5e8 | 2.5e7 > 197 > 7.9e8 | 1.4e9 > 2.6e8 > 6.8e8 |
| bodies | 2,291 > 1,411 > 1,162 | 158 > 0 > 775 | 1,009 > 401 > 1,043 |
| life real / adult (y) | .74/1.13 > 1.29/2.47 > 1.89/2.21 | 1.32/1.34 > 1.18/2.73 > 1.05/1.65 | .54/1.34 > .79/1.36 > 1.43/1.97 |
| deaths, % (last column) | spent 32, taken 2, broken 29, age 37 | spent 49, taken 5, broken 18, age 27 | spent 34, taken 2, broken 31, age 33 |
| births by month, % (last column) | 5 3 4 5 7 10 13 13 13 11 9 7 | 6 4 4 7 10 13 12 11 11 8 7 7 | 6 4 4 5 8 12 14 13 11 10 8 7 |
| held back by the rock salt (last column) | .02 | .23 | .04 |

What I see:

- **The rock nutrient no longer limits plants** (0 to 7% of the attempts; it was 53 to 62%). The
  lime stays where it was, and the salt forms on the land.
- **Eaters live in all three seeds** (5.7e6 to 1.2e7 members, 6.5 to 7.9e8 kg). They are lighter than main's in seeds 1 and 3
  (a fifth and a half of main's kg), and much heavier in seed 2 (main's eaters hang by a thread there).
- **Plants are heavier than in main** (+36%, +71%, +16% in kg). Their number of members is higher in seeds 1 and 3 and lower in seed 2.
- **Nitrogen binds in seed 2** (57% of the attempts; main 19%). The plants of that seed are the heaviest (7.7e10 kg), and the
  nitrogen mineral of the land's ground is lower (0.25 kg/m2 at year 60; main 0.72). The fixers fall to 15% there.
  This is an effect of the heavy plants, not of the rock element (the rock salt holds back 7%).
- Seeds still differ a lot, as in main. Four runs cannot say more.

**Roster (seed 1, members, main > rock nutrient with anchored erosion):** light-eaters 1.0e10 > 4.8e9; small fast light-eaters
7.4e9 > 8.5e9; big slow light-eaters 2.7e9 > 6.4e9; eaters none > 6.8e5; small eaters of remains 5.0e9 > 3.3e9; big eaters of
remains 6.4e7 > none; small fast eaters of light-eaters 4.2e8 > 1.1e9; big slow eaters of light-eaters none > 365.
Carbon gas at year 60: 304 > 292. The rock salt holds back 0 to 7% of the plants and 5 to 14% of the animals.

**Salt 23 on land, kg/m2, years 1, 3, 10, 30, 60** (main, first-guess erosion, seed 1: .53 .73 .82 1.33 1.48): anchored seed 1: .53 .80 .96 1.50
1.84; seed 2: .51 .77 .90 1.20 1.48; seed 3: .52 .70 .66 .88 1.15. The sea: 79.6 to 66.5, 77.5 to 61.5, 74.5 to 62.3.

**Lime on land, mol/m2: the 10th percent, the median, the 90th percent of the land cells**

| | main, year 10 | main, year 60 | anchored, year 10 | anchored, year 60 |
|---|---|---|---|---|
| seed 1 | 59, 1,715, 6,471 | 27, 103, 11,127 | 2,648, 2,687, 2,721 | 2,349, 2,569, 2,793 |
| seed 2 | 70, 2,189, 5,428 | 27, 146, 9,643 | 2,646, 2,689, 2,717 | 2,433, 2,605, 2,776 |
| seed 3 | 76, 2,236, 6,373 | 33, 191, 9,868 | 2,644, 2,691, 2,722 | 2,363, 2,593, 2,791 |

The start is 2,700. The land cells with under 0.001 mol/m2 of the salt: main 18% (year 10) and 58% (year 60) in seed 1, 12% and 42%
in seed 2, 15% and 35% in seed 3; anchored 0 to 1% in all.

**Carbon gas, ppm, years 10 to 60** (main > anchored):

| | 10 | 20 | 30 | 40 | 50 | 60 |
|---|---|---|---|---|---|---|
| seed 1, main | 266 | 282 | 294 | 305 | 326 | 338 |
| seed 1, anchored | 275 | 276 | 283 | 283 | 284 | 293 |
| seed 2, main | 246 | 264 | 279 | 283 | 285 | 286 |
| seed 2, anchored | 280 | 272 | 266 | 264 | 251 | 248 |
| seed 3, main | 268 | 286 | 297 | 306 | 311 | 323 |
| seed 3, anchored | 276 | 281 | 286 | 287 | 291 | 297 |
| roster, main | 263 | 248 | 303 | 308 | 307 | 304 |
| roster, anchored | 254 | 233 | 233 | 270 | 287 | 292 |

The air ends lower than main's in all four runs (more plant mass, as above).

**Speed** (seed 1, 30 years, two runs at once on 4 cores): main 3.35 s a year, anchored 3.22; 52.0 and 51.0 us a body update;
2,722 bodies against 3,146 at year 30. No cost.

### Tests

New: `core/tests/erosion.rs` (3): the starting loose ground has the diffusivity 3e-3 m2/yr (1.00 m thick, softness 1.53); `carry` is
the measured yield per J (within a fifth) and the starting bed breaks at the same; a 32x32 island loses 0.1 mm a year of rock, between
0.003 and 1. No older test assumed the fast erosion: all passed unchanged. `cargo test --release --workspace` passes.
`TUNED_BUDGET` is 63.

### What I am not sure of (erosion)

1. **The sources are from memory.** The ranges are right to my knowledge, the exact figures need a check.
2. **`carry` is the yield, not the capacity** (above). A reader could choose the capacity; it would strip the loose ground fast.
3. **The first decade is slow** (0.003 to 0.014 mm a year) and later decades are 5 to 10 times faster. I do not know why.
4. **`carry` depends on the cell size** (Bagnold's L). The yield anchor does not, but the model's stream power does: at another
   cell size the check must be done again.
5. **The island keeps its shape.** With real erosion, rivers carve almost nothing in 60 years, as in real life. If the owner wants
   valleys to show in a run, that needs a longer run, not bigger numbers.
6. **The Law 2 jump is still there.** With the lime kept on the land it no longer matters in these runs, but a cell that loses its lime still
   forms its salt 6,000 times slower.

## Speed

Seed 1, 30 years, two runs at once on 4 cores, nothing else running:

| | s per year | us per body update | bodies at year 30 |
|---|---|---|---|
| main | 3.13 | 48.6 | 3,146 |
| rock nutrient | 3.05 | 47.9 | 2,978 |

The extra cost is a list of two minerals where there was one. It is not measurable. (The 8 long runs
ran at once on 4 cores, 8 to 14 s a year; they are not a speed test.)

## Tests

`cargo test --release --workspace` passes. New or rewritten, in `core/tests/nitrogen.rs`:

- The chemistry: counts, the rock element at 1.16% (between 0.5 and 2%), growth thermoneutral,
  burning gives 15.9 MJ/kg, rotting leaves 3 of the mineral and 1 of lime, the salt is a liquid that never boils.
- Rotting and hot burning give back both elements (above).
- Growth stops without either mineral (whichever is missing, not a gram of matter) and the tally names
  the one that held it back. With all of them, each falls by its count for the matter made (3 and 1).
  With half of one, half the matter.
- Dead matter on the ground gives back the nitrogen (3 for 1, exact to 10^-4) and the rock element
  (1 for 1, to a tenth in the cell: the salt moves with liquid).
- Roots take each mineral to the level and no more, a body whose gene is off takes none, the
  excess goes back to the ground, no lime stays in the body.
- An eater digests matter into the fuel and both minerals and gives back the excess.
- The pores of the land hold the salt (above).
- Tape: columns for each mineral.

Older tests I changed, and why (no law was tuned to pass):

- `build.rs`, growth: the counts are 118, 3, 1 and 27. The oxidation test: 55,124.7 kJ a mol. The digestion test:
  10 mol of matter give 1,180 of the solid, 30 and 10 of the minerals, and use 270 of the liquid.
- `build.rs`, heat of the work: the body keeps 5.1% of it, was 4.7% (I measured both). The bound
  moved from 5% to 6%.
- `build.rs`, the immature body and the growth ceiling: these bodies had a one-off store of the
  salt and no roots. The salt falls out of the liquid in days, so a store given from outside is
  gone at the next update. They now have roots that find the minerals (uptake gene 0.01, so a
  meal is still nearly all fuel) and ground full of them.
- `mix.rs`: 1999.9999999999998 children for 2000: a float. Now a tolerance of 10^-9.
- `tape.rs`: the columns have new names (`min0_land` for `n_land`, ...).

## What I am not sure of

1. **The salt cannot be stored in a body.** This is a result of the chemistry (reaction 23 in a
   liquid with no lime). In real plants calcium sits bound in cell walls and crystals and is not
   lost. Here a plant has to find liquid at every update. I did not change the reaction, as the
   sea carbon depends on it.
2. **The rock element limits building in 53 to 62% of attempts.** Is it "a little"? The owner
   should judge. It comes from the tuned erosion (`carry`, `wear`, `creep`: 1,000 times or more above
   real) that strips the lime from the slopes, and from the rate law of reaction 22 that goes 6,000 times
   slower in a cell with under 110 mol/m2 of lime ("Follow-up"). With the erosion cut 100 times it is
   1 to 3%. The rock element has no input of its own (the bed becomes loose ground only where moving
   liquid wears it). Changing the rate law would move the sea carbon's anchor, so I left it.
3. **Eaters are fewer.** See above. With the erosion cut 100 times they live in all three seeds, so
   the rock limit is a likely cause, through the plants they eat.
4. **The shed rule is general.** Anything a body holds that it does not burn, is not matter, not gas and not
   its key liquid goes back. Today that is lime only. If a future chemistry gives bodies a solid
   store (bone, shell), this rule must be looked at again: such a store would need a level.
5. **One matter for all bodies.** 1.16% is the plants' figure. Animals get what they digest.
6. **Doubling the matter was another choice** (1.7%). It is inside the range too. I chose 1.16%
   because it is near the middle and because 3 keeps the nitrogen in whole counts.
7. **The unit of matter is now 3 times bigger.** Anything that counts matter in mol and not in
   kg changes. The tests that did are listed above. The code does not count in mol.
8. **Founders and fixing are not changed**, as asked. Fixers are 35 to 54% of the plants at year 60 (main: 62
   to 74%). The fixing gene is only for the nitrogen mineral.

## Files

- Code: `core/src/chem.rs`, `core/src/body.rs`, `core/src/choose.rs`, `core/src/copy.rs`,
  `core/src/lib.rs`, `worldgen/src/life.rs`, `worldgen/src/chem0.toml`, `tape/src/rec.rs`.
- Tests: `core/tests/nitrogen.rs`, `core/tests/build.rs`, `core/tests/mix.rs`,
  `tape/tests/tape.rs`, `worldgen/tests/flow.rs`.
- Docs: `docs/handover.md` (a row in Section 4, 6.0, 6.4, 8.3, 13.2 item 4), `docs/numbers.md` (rows of
  chemistry 0).
- Raw runs were in `/tmp/ca-*` and are deleted.

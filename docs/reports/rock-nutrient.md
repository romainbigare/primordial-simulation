# A rock nutrient in body matter

15 October 2026. Branch `worktree-agent-a03759468385e6516`, from main `03123d8`. The owner asked for
two nutrients, built from plain chemistry with no hard-coded mechanics: nitrogen (done, on main)
and a rock nutrient. This report is about the rock nutrient. Runs: 64x64, 60 years, `--every 1000`,
seeds 1, 2, 3 and `--roster` on seed 1, main `03123d8` against the branch. Numbers are the mean of
years 56 to 60 unless said otherwise.

## Short answer

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
  seeds 1 to 3: 53 to 62%). Nitrogen holds back 10 to 13% (main: 19 to 52%). The reason is on the land,
  and it is already in main: rain washes the lime out of the land. The median land cell holds 1,715 mol of
  lime a m2 at year 10 and 103 at year 60. The share of land cells whose pores hold almost no salt rises
  from 18% to 58%. More than half of the plants stand on those cells.
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

(At the balance a wet cell holds about 1 mol of the salt a m2.) So rain and rivers wash the
rock out of the slopes. It was true in main too, and nobody saw it, because nothing needed the rock
element. Now plants need it, and the limit follows the loss: in the branch it is 0.04, 0.27, 0.44, 0.55 and 0.63 of
the building attempts at years 5, 10, 20, 40 and 60. This is what the owner expected from the
nutrient ("erosion and rivers move it, so fertility differs from place to place").

Two things I could not explain. (1) The cells that have lost their salt still hold liquid (hundreds to
thousands of mol/m2, a few cm) and 25 to 110 mol/m2 of lime. The balance would give about 1 mol/m2 of the
salt. In a probe that stepped the year-60 snapshot of main for six months, 4 of 5 such cells stayed
under 0.001. I did not find out why the salt does not form there. (2) Whether this washing is too
strong: the sea gets the rock, and a real land is also given rock by weathering of the bed. Here the
bed becomes loose ground only where moving liquid works on it (Flow), so slopes are not
replenished. I did not look at how much comes back. This is main's ground, not my change.

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
   should judge. It comes from the land losing its lime to the rain (above). The nitrogen mineral is
   washed to the sea too, but the fixers put it back, and the rock element has no input of its
   own (the bed becomes loose ground only where moving liquid wears it). A weathering law would be
   a new rule.
3. **Eaters are fewer.** See above. I cannot separate this from the spread of seeds without
   more runs.
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

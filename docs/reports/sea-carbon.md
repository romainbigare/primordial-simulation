# The sea's store of carbon, from the reaction rates

15 October 2026. Branch `sea-carbon`, from `main` at d3819ac. The owner asked for a carbon store in
the sea, built from plain chemistry, with the sea starting at balance with the air. This report says
what was built, what it does and what is still wrong. Runs: 64 x 64, 60 years, seeds 1 to 3 and the
roster on seed 1, `main` against the branch.

## Short answer

- **It works as chemistry.** One new substance and two new reactions. No new law. No special
  code for the sea. The same reactions also run in any wet cell on land.
- **The sea starts at balance.** Worldgen finds the amount of salt in each wet cell from the
  reaction rates. The air starts at 400 ppm as before.
- **The sea now holds 4.3 times the air's carbon** at the start (Earth: about 50 times, but its
  sea is 3,700 m deep and this map's is 540 m).
- **The air falls less.** Carbon gas at year 60, seeds 1 to 3 and the roster: main 247, 265, 251,
  297 ppm; branch 299, 361, 356, 364 ppm. The sea gives back 1.0 to 1.4 x 10^12 mol of carbon.
- **Life does not collapse and does not clearly improve.** Plants and eaters change as much between
  two runs that differ in nothing physical (the control below) as between main and the branch.
- **The books close:** element drift 1 to 2 x 10^-15, energy drift 6 to 9 x 10^-14 at year 60.
- **Speed:** the Change law costs 55 ms a year more in the first 20 years (54 to 109), about 5% of
  the whole year. Later the cost is lost in the noise.
- **One test needed a new limit** (a statistical test that sat at the mean of its effect). See
  "Tests".
- **Weak point, said plainly:** the store grows with the sea's depth to the power 2/3, not 1,
  and the number that anchors it (500 m) is this map's, not the Earth's. See "What is not right".

## What was built

Chemistry 0 (`worldgen/src/chem0.toml`), data only:

| # | what | numbers |
|---|---|---|
| substance 23 | Ca(HCO3)2, 162.1 g/mol. A salt that exists only dissolved. | hf -1,926.8 kJ/mol; melts at 271 K (freezing point of a salty liquid); never boils, so evaporation leaves it behind |
| reaction 22 | CaCO3 + CO2 + H2O -> salt | dH -40.3 kJ/mol; k 1.0e-4 /s; Ea 2,800 K |
| reaction 23 | 3 salt -> 3 CaCO3 + 3 CO2 + 3 H2O | k 1.8e12 /s; Ea 7,647 K |

`library/chem0.toml` has the Earth names (display only).

**Why reaction 23 has a count of three.** The real way back takes one Ca2+ and two HCO3-: three
particles. Their shares in the liquid multiply. In the Change law a reaction is first order in its
scarce input and the other inputs count by their share, so a count of three gives a rate per unit
of salt that goes with the salt's share of the liquid, squared. No new code and no separate order
number were needed. (The proposal said two. Three is the real count, and it gives the real
exponent for the gas, 1/3. Two would give 1/2.)

**What the balance is.** At balance, dissolving equals settling:

    salt^3 = (k22 / k23) x exp(4,847 / T) x (gas over the cell) x (liquid in the cell)^2

- A cell with no liquid holds no salt. Salt left in a dry cell settles to rock within a day.
- A deeper column holds more salt (to the power 2/3).
- More gas over it holds more salt (to the power 1/3). Warmer holds less (dH says so).
- Buffer: when the air's gas halves, the sea gives back about 3 units for each unit the air loses.
  The Revelle factor is 3. (Earth: about 10.)

**The anchors.**

| number | value | where it comes from | mark |
|---|---|---|---|
| dH | -40.3 kJ/mol | formation enthalpies of Ca2+(aq) -542.8 and HCO3-(aq) -692.0 kJ/mol (real dH -40.1) | physical |
| Ea(23) - Ea(22) | 4,847 K | dH / R | physical |
| k22 | 0.19 a year per unit of gas over the cell, at 288 K | Earth's gross air-sea exchange: 80 of 589 PgC a year (Ciais et al. 2013) over the 71% of the planet that is sea | anchored |
| Ea(22) | 2,800 K | gas transfer is 1.4 times faster per 10 K | anchored |
| k23 | 1.8e12 /s | the balance at 288 K, 400 ppm and 500 m of liquid is 1.0 mol of salt per m3 (2.0 mol/m3 of dissolved carbon, the measured amount in the sea) | anchored, with one choice: the 500 m |
| cp, density of the salt | 194 J/(mol K), 30 cm3/mol | Kopp's rule; the volume of dissolved ions | anchored (rough) |

Cross-check: pure calcite, water and the gas give 0.6 mol of salt per m3 (from Ksp 10^-8.48,
K1 10^-6.35, K2 10^-10.33, KH 10^-1.47). The sea holds more because of the other salts.
Nothing here counts as a tuned number: the budget stays at 66 and 2.

**Worldgen** (`terrain.rs`, `Reactor::balance` in `change.rs`). The code finds, from the
chemistry alone, every pair of reactions where one undoes the other. For each wet cell it sets
the substance the first makes to the amount where both go at the same rate, at the cell's
temperature and the air's start. It takes the rock and liquid that the salt would have used. `ew chem`
prints the balance for three depths and three temperatures. `ew run` prints a start line.

**Flow needed no change.** A liquid substance already moves with the liquid by its share, and
evaporation leaves it behind (it never boils). Tests show both. The nitrogen patch's idea of
"dissolved solids" is not needed for this.

**Speed gate** (`change.rs`). A cell reacts every step if one of its reactions "may go far".
The test took the salt's share of the liquid as 1, so every wet cell reacted every step. Now an
input that counts more than once, in a liquid, goes by its share to the power `nu - 1`. A
test proves it: in a world with only these two reactions, a cell reacts about once in 39 steps
(13,845 changes of salt in 39 steps over 355 cells without the gate, 245 with it).

## Results

Control ("ctl"): the same code with the two reactions switched off (k = 0). It has the new
substance, so the brains and genes are one row longer and every random draw differs. It shows how far
two runs drift apart for no physical reason. Read "new" against "ctl" as well as against "main".

### Carbon gas, ppm, years 10, 20, 30, 40, 50, 60 (main > ctl > new)

| run | 10 | 20 | 30 | 40 | 50 | 60 |
|---|---|---|---|---|---|---|
| seed 1 | 247 > 301 > 318 | 230 > 268 > 285 | 226 > 256 > 295 | 226 > 254 > 292 | 237 > 250 > 298 | 247 > 250 > 299 |
| seed 2 | 279 > 297 > 327 | 262 > 290 > 337 | 257 > 294 > 346 | 254 > 296 > 354 | 257 > 298 > 352 | 265 > 302 > 361 |
| seed 3 | 278 > 298 > 314 | 251 > 281 > 325 | 247 > 283 > 332 | 246 > 282 > 338 | 248 > 280 > 350 | 251 > 270 > 356 |
| roster | 258 > 273 > 306 | 230 > 245 > 303 | 273 > 283 > 348 | 288 > 308 > 363 | 291 > 319 > 357 | 297 > 334 > 364 |

New is above ctl in every run and every decade: by 30 to 90 ppm at year 60. Main and ctl were
already apart by 3 to 37 ppm (litter decay, chaos).

### Carbon by store at year 60 (10^11 mol of carbon; main > ctl > new)

| run | air | dissolved gas | salt | dead matter, sea floor | dead matter, land | life | all |
|---|---|---|---|---|---|---|---|
| seed 1 | 14.1 > 14.2 > 17.0 | 0.6 > 0.6 > 0.7 | 0 > 0 > 85.5 | 2.5 > 3.0 > 5.3 | 3.0 > 3.1 > 4.3 | 3.0 > 2.2 > 2.8 | 23.1 > 23.1 > 115.7 |
| seed 2 | 15.3 > 17.4 > 20.9 | 0.6 > 0.7 > 0.8 | 0 > 0 > 95.1 | 3.5 > 2.1 > 2.6 | 2.3 > 1.8 > 2.5 | 1.7 > 1.3 > 1.6 | 23.4 > 23.4 > 123.5 |
| seed 3 | 14.4 > 15.5 > 20.5 | 0.5 > 0.6 > 0.7 | 0 > 0 > 87.9 | 2.5 > 1.8 > 2.3 | 3.5 > 3.0 > 2.4 | 2.5 > 3.9 > 2.5 | 23.4 > 24.8 > 116.3 |
| roster | 17.0 > 19.0 > 20.8 | 0.7 > 0.7 > 0.8 | 0 > 0 > 89.7 | 1.4 > 1.0 > 1.5 | 1.1 > 0.6 > 1.2 | 3.2 > 2.0 > 3.9 | 23.3 > 23.3 > 118.0 |

- "Salt" counts 2 carbon atoms per salt. At the start it held 9.96, 10.5, 9.8 and 9.96 x 10^12 mol of
  carbon (seeds 1, 2, 3, roster), 4.2 to 4.5 times the air's 2.3 x 10^12. The world starts with that
  much more carbon in total. This is the owner's decision (the sea starts at balance, the air stays
  at 400 ppm).
- By year 60 the salt gave back 1.4, 1.0, 1.0 and 1.0 x 10^12 mol of carbon.
- At year 60 the salt of each wet cell is within a few percent of the balance for the air then
  (mean 0.98 to 0.99 of it; cells from 0.57 to 1.46).
- Salt on land: 1 to 2 x 10^8 mol (nothing). Salt in bodies: nothing.
- Dead matter on the sea floor is bigger in new on seed 1 and the roster, about the same on
  seeds 2 and 3.

### Ground temperature (K, mean of years 56 to 60; main > ctl > new)

Seed 1: 290.05 > 289.21 > 289.91. Seed 2: 290.29 > 290.50 > 290.92. Seed 3: 290.30 > 289.92 >
290.75. Roster: 289.33 > 289.90 > 290.57. More gas in the air, a warmer ground (0.4 to 0.8 K over
ctl in all four).

### Plants and eaters, mean of years 56 to 60 (main > ctl > new)

| run | kind | bodies | members | real life, y | adult life, y |
|---|---|---|---|---|---|
| seed 1 | plants | 2,562 > 1,331 > 1,555 | 3.1e10 > 2.6e10 > 3.4e10 | 0.36 > 0.58 > 0.47 | 1.76 > 1.00 > 0.60 |
| seed 1 | eaters | 1,461 > 785 > 387 | 9.4e6 > 1.2e6 > 2.0e6 | 0.72 > 0.92 > 0.71 | 1.54 > 2.75 > 1.89 |
| seed 2 | plants | 1,262 > 1,140 > 1,247 | 1.8e10 > 1.2e10 > 1.4e10 | 0.45 > 0.56 > 0.50 | 0.81 > 0.90 > 0.89 |
| seed 2 | eaters | 1,204 > 609 > 599 | 8.3e6 > 1.5e6 > 2.9e6 | 0.67 > 1.26 > 2.16 | 1.48 > 2.96 > 2.99 |
| seed 3 | plants | 1,628 > 1,788 > 1,406 | 2.8e10 > 1.8e10 > 2.5e10 | 0.41 > 0.52 > 0.47 | 0.75 > 1.43 > 1.06 |
| seed 3 | eaters | 1,266 > 787 > 1,706 | 9.9e6 > 5.3e6 > 5.8e7 | 0.68 > 1.35 > 0.66 | 1.37 > 2.32 > 2.03 |
| roster | plants (kind 0) | 1,836 > 0 > 409 | 4.7e9 > 0 > 1.0e9 | 0.49 > - > 0.24 | 1.13 > - > 0.44 |
| roster | eaters (kind 1) | 27 > 442 > 0.6 | 3.3e3 > 3.1e6 > 8 | 1.72 > 0.83 > 0.41 | 2.99 > 3.05 > 3.74 |

All kinds live in every seed run. The eaters of the branch are fewer than main's on seeds 1 and 2
and more on seed 3. The control is fewer than main on all three seeds, so a part of that is the extra
substance (a longer brain) and chaos, not the salt. In the roster, kind 0 plants are gone in the
control and 409 bodies in the branch, 1,836 in main; the roster is fragile in any run (watch report
5 saw the same). I did not find a sign that the salt helps or harms life. I cannot rule out a small
effect.

### Deaths by cause, share of all deaths, years 51 to 60 (%, main > ctl > new)

| run | kind | lack | spent | taken | broken | age |
|---|---|---|---|---|---|---|
| seed 1 | plants | 1 > 5 > 15 | 82 > 52 > 45 | 2 > 2 > 2 | 9 > 36 > 34 | 5 > 6 > 4 |
| seed 1 | eaters | 0 > 0 > 0 | 63 > 59 > 64 | 5 > 8 > 1 | 5 > 5 > 6 | 27 > 29 > 29 |
| seed 2 | plants | 4 > 5 > 4 | 68 > 61 > 59 | 1 > 3 > 7 | 21 > 21 > 22 | 6 > 9 > 8 |
| seed 2 | eaters | 0 > 0 > 0 | 63 > 56 > 33 | 1 > 3 > 1 | 6 > 6 > 16 | 30 > 35 > 49 |
| seed 3 | plants | 6 > 4 > 5 | 61 > 67 > 59 | 2 > 5 > 7 | 24 > 19 > 22 | 7 > 6 > 6 |
| seed 3 | eaters | 0 > 0 > 1 | 55 > 58 > 69 | 12 > 3 > 1 | 5 > 7 > 10 | 28 > 32 > 19 |
| roster | plants | 5 > 0 > 4 | 47 > 37 > 50 | 34 > 55 > 36 | 10 > 5 > 7 | 3 > 3 > 3 |
| roster | eaters | 0 > 0 > 0 | 31 > 43 > 58 | 29 > 20 > 2 | 3 > 10 > 13 | 37 > 27 > 25 |

("phase", at most 2%, is left out.) The causes are the same set in all runs: starving
("spent") is the biggest, then breaking and age for eaters.

### Births by month (peak month, 0 to 11; main > ctl > new)

Plants: 4 > 4 > 4 on all three seeds (spring). Eaters: 6 > 7 > 6, 8 > 7 > 7, 6 > 8 > 7 (summer).
Roster: plants 5 > 6 > 6, eaters 9 > 6 > 6. The seasons of breeding are kept.

### Speed

| | main | control | branch |
|---|---|---|---|
| seed 2, 20 years, idle machine: ms a year in Change | 54 | 52 | 109 |
| same: ms a year in all | 1,322 | 1,261 | 1,385 |
| 60 years, four runs together, seeds 1, 2, 3, roster: s a year | 5.1, 4.3, 4.8, 8.0 | 5.1, 4.9, 5.5, 10.8 | 3.9, 3.6, 4.2, 9.0 |
| same: ms a year in Change | 157, 178, 185, 162 | 236, 250, 221, 183 | 238, 242, 231, 202 |

The branch's whole year is lower in the 60-year runs because it has fewer bodies (the cost of
bodies dominates), not because the sea is cheaper. The clean number is the 20-year seed 2 pair:
Change costs 55 ms more a year (the wet cells react every 40 steps in place of every 320), total
about +5%. The control's Change time over 60 years is as high as the branch's (236 against 157 for main:
the extra substance and reactions cost something even switched off), so I cannot show a cost of
the salt itself after the first years.

## Tests

- `core/tests/dissolved.rs` (new, 9, 0.2 s): the pair is found from the chemistry alone; elements
  and energy close from both sides; the anchor (1.0 mol/m3 at 500 m, gas to the power 1/3, warmth as
  dH says); a dry cell holds none and a deeper column holds more (4.64 for 10 times the depth),
  salt left in a dry cell settles at once; a long run of the reactions in 120-day lumps with the air
  held at 400 ppm settles to the worldgen balance within 5%, from none and from twice as much;
  the sea gives back more than 2 units for each unit the air loses; a world starts with salt in every
  wet cell and none on dry land and keeps its books for two years; the salt runs off with the liquid
  by its share and stays when the liquid evaporates; a cell does not react every step.
- `core/tests/choose.rs`: `body_learns_to_move_toward_what_it_can_take` failed at 1.7 points over
  chance against a limit of 2. It is a statistical test. On `main` the same measure is 2.7 points on
  seeds 1 to 800 and 1.5 on seeds 801 to 1600, so the limit sat at the mean of the effect. The new
  substance changes the brain's input size and so every random draw. I set the limits to 1 and 1.5
  points (comment in the test). This is a change to a test, and I say so.
- Whole workspace: `cargo test --release --workspace --no-fail-fast`, all pass, including the
  word ban and the numbers test.

## What is not right

1. **The store goes with the liquid to the power 2/3, not 1.** The reaction takes its gas from the
   whole air column over the cell, so the air's amount, not the liquid's, sets the supply. Real
   chemistry gives salt in proportion to the liquid. The sea's salt per m3 falls as the depth to the
   power -1/3 (1.7 mol/m3 at 100 m, 1.0 at 500 m, 0.5 at 3,700 m for the same air).
2. **The 500 m anchor is this map's** (median 660 m, mean 540 m). Anchored at the Earth's mean
   depth, 3,700 m, the same sea would hold about twice as much per m3. This is an anchor to the
   map, not to the Earth. I chose it so the sea of this map has the measured amount of carbon per
   m3. The owner may prefer the other.
3. **The way to remove 1 and 2 is known.** If gas reached the ground's reactions only through the
   liquid cover (a cell under deep liquid would use the gas dissolved in it, which the Henry law
   already tracks, and not the air above), the same two reactions would give salt in proportion to
   the liquid and to the gas to the power 1/3, with no 500 m. That is also review 8's doubt about the
   sea floor using the air's oxygen. It changes how dead matter rots on the sea floor, so it needs the
   owner's decision and its own measurement. I did not do it.
4. **The sea is one layer to its floor** (the known limit of 6.1). Here the whole 800 m answers the
   air in a few years. On Earth the deep sea takes centuries. So this sea gives back carbon faster
   than a real one.
5. **The Revelle factor is 3** (Earth about 10), as far as one salt can go.
6. **The world starts with 4.3 times more carbon** in total. It is the owner's decision, but the
   ledger's "all" column is not comparable with main's.
7. **Lime runs short in some cells.** In a few wet cells the loose rock has been carried away; there
   the lime is equal to the salt, and the salt is below its balance (the lowest cells are at 0.57 to
   0.70 of it).
8. **Adding a substance changes everything random.** Brains and genes get one more row. The control
   shows how much two runs drift for that alone; the life results cannot be read finer than that.
9. **Speed:** Change costs 55 ms a year more in the first years (about 5% of the year).

## Where things are

- `worldgen/src/chem0.toml`, `library/chem0.toml` (names)
- `core/src/change.rs`: `reversible`, `Reactor::balance`, and the share test in the candidate check
- `worldgen/src/terrain.rs`: the start at balance
- `cli/src/chem.rs`, `cli/src/main.rs`: the balance table and the start line
- `core/tests/dissolved.rs`, `core/tests/choose.rs`
- `docs/handover.md` (a row in Section 4; the known limit of 6.1), `docs/numbers.md`

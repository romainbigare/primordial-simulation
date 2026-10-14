# Watch report 7: review 6's fixes, and one law that did not work

14 October 2026. Code: `main` after review 6 (`review-6-adversarial.md`, `review-6-life-size.md`).
All runs are 64 x 64, 60 years, seeds 1 to 3, normal and roster. Tables are in
`watch-7-before-m4/runs-normal60.md` and `runs-roster60.md` (columns "new" = this code, "main" =
the code before the scaling and start changes).

## Short answer

Two Opus agents looked at the world. One did an adversarial review; the other found why lives are
short and why animals shrank. We applied their fixes, with one exception.

**What changed in the world (seeds 1 to 3, year 60):**

- **Animals no longer shrink.** Eaters keep 45 to 65 kg of copy mass. Before, they went down to
  0.04 to 11 kg in most roster runs.
- **Lives are longer:**
  - eaters 0.4 to 0.9 years (before 0.12 to 0.5);
  - eaters of remains 0.2 to 0.4 years (before 0.06 to 0.12);
  - big eaters of eaters up to 1.3 and 3.2 years.
- **The big hunters live.** Eaters of eaters survive in all three roster seeds, the big slow ones
  at 400 to 1,290 kg. Before, they died within 1 to 3 years in every run.
- **Fewer deaths by starving.** In the roster, `SPENT` is 8 to 77% of animal deaths, against 77 to
  95%. Age now kills 20 to 40% of the eaters of remains.
- **The start is no longer a lottery.** 33 to 164 founder bodies are left at the low point, against
  2 to 16.

**What did not work:** the law "a body is built, not just filled" (growth paid like building a
child, satiety at every size, size by dry mass). Under it every animal kind died in all six runs,
and the plants fell to 5 to 30 times less mass. It is **not merged**. The code is saved as
`watch-7-before-m4/built-mass-not-merged.patch`. Why it failed is explained below.

## What we built

| # | Change | Why (review 6) |
|---|---|---|
| 1 | `copy::WITHIN` 2 → 0.5. Children and merges join only nodes of their own kind (5.9 steps instead of 11.6) | Nodes mixed kinds. The many small eaters of remains flooded the eaters' nodes (51 kg to 3 kg in 4 years). Individuals, which copy asexually, never mix |
| 2 | Strength, catalysts, learning and signals cost per kg^0.75, like income, set at the 30 kg founders (each number x 30^0.25 = 2.34) | Income went with mass^0.75 but these costs with mass, so a 0.18 kg body had twice the surplus of a 44 kg one |
| 3 | Moving costs `c_move` x (m / 30 kg)^-0.32 per kg per m (Taylor's cost of transport) | A flat cost per kg let small bodies move 6 times too cheaply |
| 4 | Animal founders get the hand-set brain biases by default. `--neutral-eaters` gives the old start. `c_build` goes back to 10⁴ | 97 to 99% of neutral-brained founders died in their first year, and `c_build` had been raised to 3 x 10⁴ to win that lottery. Fix the start, not the laws |
| 5 | `docs/numbers.md`: `c_build` and `c_basal` marked tuned; `take` and `c_move` anchored; a new count of "rules chosen for a run" (4) with a test | The tuned budget had been sidestepped by a label |
| 6 | `c_build` is paid at most once per update, satiety has an upper limit, three tests fixed, `ew`'s rain timing is the default, a `merged_wide` column | Smaller findings 6, 7, 10, 11, 13 |
| 7 | A fair big-brain test: copies of living eaters, one set with a 48-unit brain, one unchanged | The old test compared a hand-set start against a neutral one |
| 8 | Corrections in report 6 | Five claims did not match the data |

## Key numbers (year 60, mean of years 56 to 60)

**Normal runs** (new / before):

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| eaters: copy mass | 61 / 45 kg | 64 / 61 kg | 61 / 58 kg |
| eaters: real life | 0.72 / 0.19 y | 0.78 / 0.15 y | 0.90 / 0.52 y |
| eaters: births per member a year | 1.4 / 6.5 | 1.3 / 7.3 | 1.2 / 2.2 |
| plants: real life | 0.30 / 0.39 y | 0.53 / 0.44 y | 0.93 / 0.70 y |
| fewest founder bodies | 82 / 11 | 104 / 3 | 164 / 3 |
| carbon gas | 254 / 223 ppm | 243 / 255 ppm | 245 / 232 ppm |

**Roster runs:** the kinds alive at year 60 in seeds 1, 2 and 3, new against before:

| kind | new | before |
|---|---|---|
| eaters | 3 of 3, 45 to 65 kg, life 0.43 to 0.67 y | 2 of 3, 11 and 71 kg, life 0.12 to 0.16 y |
| eaters of remains (small, big) | 6 of 6, 2.3 to 31 kg | 4 of 6 |
| eaters of light-eaters (small, big) | 4 of 6 | 4 of 6, mostly small |
| eaters of eaters (fast, big) | 6 of 6, 42 to 1,290 kg, life up to 3.2 y | 1 of 6 |
| small fast light-eaters | 0 of 3 | 2 of 3 |

The small fast light-eaters (0.05 kg) now die out. Their strength and catalysts cost them 2.3 times
more per kg than before, which is the intended scaling, and they cannot live on it.

## The big-brain test

- The fair test places copies of 300 living eaters twice: one set with a 48-unit brain, one
  unchanged (the control).
- Run once (seed 1, before the scaling change):
  - The big-brain kind grew to about 1.7 times the ordinary eaters' mass by year 70.
  - Its real life was shorter (0.17 against 0.24 years).
- The control is not clean. It has the ordinary eaters' genes, so their nodes absorb it and its own
  count fell to 0 by year 55. The big-brain kind cannot merge with them, because likeness counts
  brain size.
- So the result is "big brain against the ordinary eaters", not against a matched control. It is
  one seed. **No verdict yet.**

## The law that did not work: "a body is built, not just filled"

**The idea (review 6, finding 5).**

- A child grew to adult size by eating, and paid nothing for it. 70% of every adult body was built
  for free.
- So:
  - a body's own growth should be paid like a child's: `c_copy` per kg of dry mass, from the same
    daily `c_build` allowance;
  - satiety could then apply at every size;
  - size (the copy trigger, leaf, height) should be the dry mass the body has built, so liquid
    gives no free size.

**What happened (six runs, 60 years):**

- **Every animal kind died.** About 97% of the founders died of `SPENT` in their first months, with
  no births. The low point was 1 to 4 bodies.
- **The plants fell to 5 to 30 times less mass,** and the carbon gas rose to 360 to 380 ppm.
- **The plants that lived had long lives,** 2 to 16 years. That is the only sign that the law
  lengthens lives as the arithmetic said.

**Why it failed:**

1. **A body here has no structure apart from its food store.**
   - A body's dry mass is mostly its sugar.
   - "Pay to build dry mass" therefore charges building work for storing fuel.
   - In life, storing sugar or fat is cheap, and building body matter (protein, cell walls) is
     dear. This model cannot tell them apart.
   - A plant then pays about 2.6 x 10⁷ J for each kg it keeps: 1.6 x 10⁷ of sugar and 10⁷ of work.
     Its surplus is about 1.4 x 10⁷ J per kg a year.
2. **`c_build` is anchored to how fast animals make young.**
   - 10⁴ J a day per kg^0.75 lets a 1 kg plant build about 0.4 kg a year.
   - Real plants can grow 10 to 100 times faster.
   - So the plants starved first, and the animals with them.
3. **Burning leaves liquid that nothing sends out.**
   - The few eaters that lived for years swelled from 30 to 85 kg of mostly liquid.
   - Their upkeep, which goes by the whole mass, doubled while their dry body stayed flat.

**What it would take.** Chemistry 0 needs a body-matter substance apart from the fuel store, made
from food at a real cost and broken down at death. Building would then cost work for body matter
only, and storing fuel would stay cheap. That is a change to the world's chemistry, and so a
decision for the owner. Until then, satiety stays limited to bodies past `full` (a rule chosen for
a run, counted in `docs/numbers.md`), and a child's growth stays free.

## Problems still open

1. **Growth is free and satiety acts only past `full`** (above).
2. **The plants' births come from the brain's copy chance, not from a law.** Plant nodes almost
   never pass `full`, so the kept copy logit (a hand-set number) sets how often they copy
   (review 6, finding 4).
3. **The nodes-against-individuals check** still needs a world where everything can be an
   individual. Review 6 proposes small cells (about 20 m), so a 32 x 32 world holds about 4 x 10⁴
   plants as individuals.
4. **The starting repair genes and the stores of the roster's big kinds** were set for the old
   costs, and they now give different lives than their comments say.
5. **One seed per run, 60 years.** The 300-year runs at 128 x 128 are still to do.

## What next

1. **Decide on a body-matter substance in chemistry 0.** It is what lets growth cost time without
   starving the plants.
2. Build the small-cell check world and run the nodes-against-individuals check.
3. Run the three 300-year seeds.
4. Then M4.

Code size: `core` is 7411 lines (budget 8,000 through M7).

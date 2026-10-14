# Review 6, second part: why lives are short and why animals evolve small

14 October 2026. An analysis with experiments: 64 x 64 roster runs, seeds 1 to 4, 60 to 80 years,
each with one number or one line changed. The full tables are in `review-6-life-size/tables.md`.
This file is the orchestrator's summary of the agent's report. The agent could not write files, so
the text was saved from its answer.

## Short answer

**Lives.** The real life is members over deaths a year. In a steady population deaths equal
births, so **life = 1 / (births per member a year)**. Members are born 3 to 25 times a year, so they
live 2 weeks to 4 months. Age is only 1 to 4% of deaths. The causes, ranked:

1. **The animals are small.** At 20 to 80 g, a life of 2 to 4 weeks is about a wild mouse's mean
   life at birth.
2. **A new member is cheap and grows up for free.**
   - A child's work counts only what is not liquid, and the roster animals became 86 to 93% liquid.
   - Growing up is not "building": one meal makes a child an adult in 2 to 4 weeks.
   - `c_build` was at the top of its anchor (3 x 10⁴).
3. **Animals live less than one interval from starving.** A roster eater holds fuel for about 0.9
   intervals above the line where it dies. So 85 to 95% of animal deaths are `SPENT`. When births
   are slowed and feeding is no surer, the eaters die out.
4. **Age never arrives,** so the repair gene evolved down. When lives get longer, age deaths
   appear at once.

**Size**, ranked:

1. **The nodes mixed the kinds** (`copy::WITHIN` 2). A node at its spread cap took children up to
   11.6 mutation steps away in every gene. Eaters and eaters of remains are only 6 to 9 steps apart,
   so the many small eaters of remains flooded the eaters' nodes. In seed 1 the eaters went from
   51 kg to 3 kg in the 4 years after they were placed, and by year 100 all four animal kinds had
   the same genes. Copying is asexual, so individuals of two kinds never mix. This was a fault in
   the speed tool.
2. **The start keeps small founders.** Of 340 neutral-brained founders, 9 were left a year later.
   They were the ones whose copy mass (25 kg) was below the 30 kg they were placed at, so they could
   copy at once. With hand-set brains 96 survived and kept 57 to 64 kg.
3. **Per kg, small bodies breed faster and pay nothing extra for it.** Intake and building go with
   m^0.75, so per kg they go with m^-0.25. Strength, catalysts and learning cost by the kg at every
   size. Food per meal never binds (the take limit is 7 to 17 times upkeep), so births are set by
   building, and building per kg favours small bodies. The brain is the only brake. This pressure
   is slower than cause 1, but it acts even where nothing mixes.

## Key experiments (year 60, eaters' copy mass in kg)

| seed | `WITHIN` 2 (as it was) | `WITHIN` 0.5 |
|---|---|---|
| 1 | 4.9; 1.0 (a run that differs only in rounding) | 12.5; 53.4 (the rounding variant) |
| 2 | 50.7 (no mixing happened) | 54.8 |
| 3 | 0.042 | 51.4 |
| 4 | 1.31 | 51.6 |

Other levers, tested from year 30 of a `WITHIN` 0.5 world (seed 1, eaters' real life, years):

| change | eaters' life | note |
|---|---|---|
| control | 0.168 | |
| `c_build` 10⁴ | 0.196 | from the start, the neutral-brained eaters die; with hand-set brains they live |
| `c_build` 3 x 10³ | 0.317 | eaters die on seed 2 |
| a child's work counts the whole adult body | 0.214 | eaters die on seed 2 and from the start |
| `take` 0.025 | 0.237 | |
| strength, catalysts, learning by m^0.75 | 0.204 | big hunters survive to year 60 for the first time (eaters of eaters 92 to 99 kg; the 600 kg kind at 912 kg on seed 2); mixed effect on eaters of remains |

## What the agent recommended

- **Keep:** `copy::WITHIN` 2 to 0.5. Done on `main`.
- **Promising, but needs a start that works:** `c_build` back into its anchor's middle, costs by
  m^0.75, and growth counted as building.
- **Rejected:** removing the special case where copying past `full` pays out all the body can
  release. Eaters shrank to about 3 kg.
- **Next:** copying triggered by built (non-liquid) mass, with growth limited by the build rate. A
  start that does not need `c_build` at its top. A likeness test over several genes together.

## Warnings from the agent

- **The world is chaotic.** Two runs that differed only in the rounding of one product gave eaters
  of 1.0 against 4.9 kg at year 60. Trust only effects with the same sign across seeds.
- **`WITHIN` 0.5 costs speed:** 6 to 58% more bodies in the roster, and up to 1.5 times the time
  for bodies.
- **The figures for real animals are from memory:** births per head and mean life at birth, about
  0.05 to 0.1 years for mice and 2 to 4 years for 50 kg deer.

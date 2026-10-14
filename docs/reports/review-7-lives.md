# Review 7, lives: why plants and animals live months, not years

14 October 2026. Code: `main` at 316bf79 (after watch report 8). Written by an Opus agent with
16 variants and about 70 runs (64 x 64, 60 years, seeds 1 to 3). Saved by the orchestrator from the
agent's answer. Its tested code is `review-7-lives.patch`: R1 to R3, applied on `main`.

## Short answer

**1. Errors? Some, but no single bug sets the lives.**

- **Bug: digested body matter went past the store limit.**
  - It was checked against the room for matter, then turned into the solid.
  - The heaviest eaters held 3 times the solid their body allows, and 10 to 10,000 times when
    building was slower.
  - Fixed (R1).
- **Stale starting repair genes.** Three roster kinds started with lives of 0.7 to 1.4 years, not
  the 2.5 to 21 years their comment says, and age was 49 to 65% of their deaths. Fixed (R2).
- **Two lines set the starvation margin.**
  - Growth turns all fuel into body except one update of upkeep.
  - Burning is first order in the store, so a body can burn only 20 to 37% of its store in an
    update.
- **The fuel's safe minimum was a death line.** Eaters died of `LACK` (45 to 55% of their deaths)
  while they still held body matter they could burn. Fixed: it is now a reserve (R3).
- **Units:** the growth allowance and upkeep go by the whole mass, fuel included, though fuel is not
  body.
- **Upkeep does not follow temperature, but burning does.** So 80% of plant deaths fall in the five
  winter months.
- **Held liquid is a cheap way to a big body:** size counts liquid, but building is paid per kg of
  matter.
- **Nodes:** a birth splits members into halves, and deaths ignore age. This is not a bug, but it
  decides which real numbers we may compare with.
- **The tape counts right.**

**2. More laws? No.** Changed numbers and allocation inside the five laws already give lives of 1
to 3 years (some kinds up to 9).

**3. How lives can emerge without a lifespan rule.**

- In a steady population, life = 1 / (births per member a year).
- Births are body built a year over half a copy mass.
- So lives get long when:
  - bodies build at real rates for their size;
  - they keep a reserve;
  - held liquid is no free shortcut;
  - the small and the young die first;
  - costs slow in the cold.
- Age deaths then come by themselves: 11 to 29% of plant deaths in the longest-lived runs.

**4. Existing laws:**

- **The best lever is a reserve growth keeps (R3).** With R1 and R2: plants 0.65 to 0.74 years
  (before 0.37 to 0.49), eaters 0.80 to 0.98.
- **A slower `c_build` doubles eater lives,** but only while the digestion bug let eaters hoard.
  Close the liquid shortcut (R4) first.
- **"Upkeep follows temperature"** is Change's own equation applied to upkeep, not a sixth law. Alone
  it removed the winter die-off, but did not lengthen lives, and it cut eaters 6 to 40 times,
  because summer upkeep doubled. Re-anchor `c_basal` (Kleiber's value is for bodies at 310 K) when
  it is added.

## Compare like with like

- **A node has no ages.** Its deaths take a random share of members, so its life at birth is also
  an adult's remaining life.
- **Its newborn is half a member,** like a weaned young animal, not a newborn mouse or a seed.
- **So compare with these real values,** not with real mean life at birth (most real newborns die
  in weeks):
  - the remaining life of an adult;
  - the life from independence;
  - P/B, body made a year over standing body.
- **Real values (from memory):**

| real kind | remaining life of an adult | P/B a year |
|---|---|---|
| mouse, 20 g | 0.3 to 0.5 y | 3 to 5 |
| rabbit, 1.5 kg | 1 to 2 y | 1 to 2 |
| deer, 50 to 70 kg | 4 to 7 y | 0.3 to 0.5 |
| bison, 500 kg | 8 to 12 y | 0.15 to 0.25 |
| perennial herb, 1 kg | 3 to 20 y | about 1 |

- **The model against these:**
  - Eaters: P/B about 0.75, so 1.5 to 2.5 times too fast; adult life 0.85 years, so 5 to 8 times
    too short.
  - The 600 kg kind: about 5 times too short.
  - The small roster animals: near real values.
  - Plants: 10 to 50 times too short.
- **Eaters are 10 times too dense:** 240 t/km² against about 20 t/km² for real large plant-eaters.

## Experiments (normal runs, seeds 1 to 3, years 56 to 60)

| run | one change | plants: life (y) | eaters: life (y) | note |
|---|---|---|---|---|
| base | `main` | 0.37 to 0.49 | 0.81 to 0.94 | |
| A | upkeep follows temperature | 0.40 to 0.51 | 0.62 to 0.83 | winter die-off gone; eaters 6 to 40 times fewer |
| B | growth allowance by body | 0.33 to 0.42 | 0.85 to 1.06 | |
| C | `c_build` 3 x 10³ | 0.62 to 0.70 | 1.50 to 2.02 | eaters hoard 4 to 5 times their body as fuel (the bug) |
| D | upkeep by body | 0.34 to 0.59 | 0.92 to 1.06 | |
| E | burning saturates | 0.40 to 0.56 | 0.82 to 0.96 | twice the plant members |
| G | growth keeps 3 updates of upkeep | 0.67 to 1.06 | 0.87 to 0.99 | |
| H2 | fuel minimum a reserve, 0.25 at start | 0.62 to 0.69 | 0.83 to 1.13 | |
| **XHF** | R1 + R2 + R3 | **0.65 to 0.74** | **0.80 to 0.98** | roster: 7, 7, 6 of 7 animal kinds |
| P2 | XHF + slower building + allowance by body + adult founders | 0.79 to 1.27 | dead on 2 of 3 seeds | eaters became about 95% liquid |

**On `main`, with copying by physics** (review 7 on plants) **plus R1 to R3:** plants 0.62 to 0.68
years, eaters 0.94 to 1.15, and all 7 roster animal kinds alive (seed 1).

## Proposed fixes and their state

| # | Fix | Size | State |
|---|---|---|---|
| R1 | Digested matter fills the room of what it becomes | S | applied |
| R2 | Set the stale repair genes again | S | applied |
| R3 | A fuel's safe minimum is a reserve, not a death line; starting minimum 0.25 | S | applied |
| R4 | Close the liquid shortcut: things that need living matter go by matter, or holding water costs building | M | not built; do before R6 |
| R5 | Building allowance and upkeep by body, not whole mass | S | not built; with R1, and adult founders |
| R6 | `c_build` toward its anchor (1,450 to 1,500 per kg^0.75 for 50 kg to 1 t animals) | S | after R1, R4 and R5, on more seeds |
| R7 | Upkeep follows temperature, with `c_basal` re-anchored | M | queued (task 2), with M4's cold seasons |
| R8 | Burning saturates (Michaelis-Menten) | M | not a lever now |
| R9 | Tape: P/B per kind, deaths by month, adult age at death for individuals | M | partly done (adult and young deaths, births by month) |
| R10 | Size or age structure in nodes, or the individuals check first | L | size classes built, off |
| R11 | Liquid made in the body above its limit drains out | S to M | not tested |

## What not to do

- **No lifespan, age of maturity, death rate, "adults stop growing" rule, or dormancy switch.**
- **Do not raise repair genes to lengthen lives.** Hunger deaths simply replace age deaths.
- **Do not lower `c_build` alone.** The eaters died, and bodies became 95% liquid.
- **Do not turn satiety on.**
- **Do not compare `life_real` with real mean life at birth.**
- **Do not trust one seed,** and judge nothing before the 300-year runs.

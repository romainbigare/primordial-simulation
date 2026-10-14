# Review 7, plants: how the light-eaters reproduce

14 October 2026. Code: `main` at 316bf79 (after watch report 8). Written by an Opus agent with
experiments (64 x 64, 60 years, seeds 1 to 3, one change at a time). Saved by the orchestrator from
the agent's answer. Its code is `review-7-plants.patch` (not merged).

## Short answer

**1. Errors in our code? Yes. One real bug, and node machinery that sets the plants' demography.**

- **Bug: copying when full never happens for a body that has finished growing.**
  - The trigger reads `Body::size`, which is capped by the liquid the body holds now.
  - The liquid that goes with new matter comes by soaking and rain, and each update closes only part
    of the gap.
  - So a grown body sits 0.04 to 0.12% under `full` for years, and never copies by this path.
- **Every plant birth comes from the brain's copy chance.**
  - Only 0 to 0.09% of plant members are past `full`.
  - With that chance turned off, the plants nearly died out on all three seeds.
  - The kind of start for the plants' brains alone moved their births from 2.2 to 1.25 per member a
    year.
- **Node machinery:**
  - Children who stay in a node take nothing from its stores, so the CHILD gene does nothing to
    them.
  - One copy decision doubles a whole node of about 10⁷ members at once.
  - A parent cannot shade its own young.
  - Deaths take members of average size, so the small do not die first.
- **Smaller errors:**
  - About 38% of plant decisions are a copy attempt that does nothing.
  - A newborn's leaf was computed before its liquid was counted.
  - Plants walk: a plant member moves to a neighbour cell 1.3 times a year, 5 times more often than
    it sends a child there.

**2. More laws? Not for reproduction itself.** With the trigger fixed, copying by physics alone works
for plants and animals on all three seeds. Real plant life cycles (seeds that wait, germination,
annual and perennial plants) also need:

- **size classes in nodes** (a speed-tool fix);
- **perhaps one refinement of Change:** a body's chemistry runs in its liquid, so a dry body is
  nearly dormant;
- **real seasons with frost and a dry season** (M4: the island's winter is 283 to 285 K).

**3. How to make it emerge without hardcoding:**

1. A body copies when it has built its body (the trigger fix).
2. Size classes in nodes, so parents shade their young, the young die first, and seed size against
   seed number becomes a real trade-off.
3. A dry body is dormant, and a child may get less liquid than its parent, so dry children wait in
   the ground and grow when it is wet.
4. Frost and a dry season (M4).

Nothing names a plant, a seed or a season.

**4. Existing laws or a new one:**

- **Now (S):**
  - the trigger fix;
  - copying by physics only (this drops the kept copy logits, one of the three "rules chosen for a
    run");
  - later, fold `full` into the COPY_MASS gene (one tuned number fewer).
- **Then (M):** size classes in nodes.
- **New law (L, needs the owner, handover 14.2): activity needs liquid.**
  - The equation: activity = (liquid / the level it keeps)^n, with n about 5 (Harrington's rule for
    seeds).
  - Three unrelated uses: dormant seeds, spores and eggs; drought survival by drying out; dry places
    that preserve dead matter.

## Runs (plants are kind 0, eaters kind 1; values per seed 1 / 2 / 3)

| Run | Change | Plant births per member a year | Plant life (y) | Plant body (kg) | Eater life (y) | Carbon gas (ppm) |
|---|---|---|---|---|---|---|
| base | nothing | 2.2 / 2.9 / 3.5 | 0.49 / 0.44 / 0.37 | 1.0 to 1.3 | 0.81 to 0.94 | 199 to 229 |
| P | hand-set plant brains | 1.25 (seed 1) | 0.71 | 1.2 | 1.01 | 220 |
| A | plants never copy by the brain | 0.5 to 0.56 | collapse (3 to 12 nodes) | | | 387 to 389 |
| AF | A plus the trigger fix | 4.3 / 4.0 / 3.4 | 0.40 / 0.45 / 0.50 | 1.7 to 2.3 | 0.56 to 0.95 | 233 to 248 |
| F | trigger fix only | 4.0 to 4.9 | 0.31 to 0.39 | 0.8 to 1.7 | 0.85 to 0.95 | 125 to 208 |
| NC | trigger fix, no body copies by the brain | 2.6 to 4.1 | 0.51 / 0.44 / 0.51 | 1.7 to 2.35 | 0.85 to 1.07 (body 51 to 59 kg) | 233 to 256 |
| **Y** | NC plus the newborn-leaf fix | 2.2 / 3.5 / 1.9 | **0.58 / 0.48 / 0.58** | 1.6 to 2.5 | **0.89 to 1.01** (body 53 to 55 kg) | 212 to 285 |
| C | size-class nodes only | 2.4 to 4.1 | 0.25 to 0.39 | 1.3 to 1.5 | 0.73 to 0.90 | 172 to 196 |

**Roster runs:**

- **NC, seed 1:** all three plant kinds live, including the small fast ones (gone in every base
  roster run). The big slow plants keep 23 kg (base: 3 kg). All 7 animal kinds live.
- **Y, seed 2:** all 7 animal kinds live, and the big slow plants hold 3.65 x 10¹⁰ kg (base 5.9 x 10⁹).

## Findings in short

1. **The copy-when-full bug,** described above. Seen in snapshots: grown nodes 1 to 20 years old sit
   at 3.9 x 10⁻⁴ to 8.6 x 10⁻⁴ under `full`. Fix: `Body::built` (matter / (1 - h)), the body the
   matter makes, read by the trigger.
2. **The brain's chance sets the pace.** For seed 1:
   - 15 decisions a year, at a copy chance of 0.44, means about 55 days of waiting;
   - rebuilding 0.072 kg of matter at 1.2 g a day takes 61 days;
   - so a cycle is about 116 days, at most 3.1 births a year. Measured: 2.2 to 3.5.
3. **Node machinery** (above): children who stay ignore CHILD; births are synchronised across a
   node; there is no shade inside a node; deaths take the average member; merges compare genes only.
4. **About 38% of plant decisions are wasted copy attempts** below copy mass.
5. **Plants walk.**
   - A move costs about 0.36 days of a plant's upkeep and needs no force.
   - With hand-set brains, walking falls to 0.49 moves per member a year.
6. **Newborn leaf bug.** A newborn's leaf area was computed before its liquid was counted, so it was
   4.9 times too small until its first update. Fixed in the patch.
7. **"Real life" cannot show a plant's life.**
   - It equals 1 / births. A herb that makes 100 seeds a year has a life at birth of weeks, yet its
     adults live years.
   - Plant nodes persist: the median member is in a node 2.6 years old.
   - The tape needs deaths split into young and adult members.
8. **Why plant lives are months:** everything a grown plant makes becomes children, and those
   children starve in a full canopy.
   - `SPENT` is 69 to 85% of plant deaths.
   - Eaters take only 0.17% of standing plants a year. Real herbivores take about 10 to 50% of what
     plants make (from memory).
9. **Lower:**
   - growth's allowance uses the whole mass, not the body;
   - stale comments;
   - no season to adapt to;
   - `--individuals` keeps plants as nodes, so it cannot check them.

## Proposed fixes, ranked

| # | Fix | Size | Status |
|---|---|---|---|
| 1 | The copy trigger reads the body built (`Body::built`) | S | in the patch |
| 2 | Copy by physics only: drop the brain's copy chance (`COPY_BY_CHOICE = false`), then fold `full` into COPY_MASS and delete the kept copy logits | S | in the patch (switch), owner to decide |
| 3 | Newborn leaf computed after its liquid | S | in the patch |
| 4 | Tape: share of members past copy mass and past `full`, births by path (brain or physics), moves per member, monthly births, deaths of young against adult members | S | not built |
| 5 | Size-class nodes (`COHORTS`, off): children far smaller than their parents form their own group, and merges need members within a factor `full` in size | M | in the patch, off; 20 to 30% slower |
| 6 | Activity needs liquid (dormancy), with a gene for the child's share of liquid and frost that kills in proportion to activity | L | sketch; a new law, owner to decide |
| 7 | Plants walking: measure first (fix 4), then either "moving needs force" or roots as a bond to the ground; never a "plants cannot move" rule | M | not built |
| 8 | Seasons with frost and a dry season | M4 | |

## What not to do

- **No named stages:** no seed substance, flowering or germination action, "copy in spring" rule,
  or minimum child size.
- **No plant-only rules:** the core cannot know what a plant is.
- **Do not tune for a target life:** not `c_build`, `full`, `H_LIGHT` or the start.
- **Do not judge plant life by `life_real` alone:** add adult deaths first.
- **Do not check plants with `--individuals`:** it keeps them as nodes.
- **Do not turn size classes on by default** before the tape can show adult survival.

## Limits

- Three seeds, 60 years, 64 x 64; one roster seed per variant.
- Adult survival was not measured.
- The dormancy law was not built.
- The real-plant figures and Harrington's rule are from memory.

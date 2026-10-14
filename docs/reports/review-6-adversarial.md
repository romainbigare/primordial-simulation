# Adversarial review: `main` after watch report 6, before M4

14 October 2026. Code: `main` at 7e1075a (watch report 6). I read handover 2, 3, 4, 6, 7, 12,
13.2 and 14, `docs/numbers.md`, review 4, review 5, watch reports 5 and 6, and the code (`body.rs`,
`copy.rs`, `choose.rs`, `lib.rs`, `worldgen/src/life.rs`, `tape/src/rec.rs`, the tests). I read the
series of `/tmp/claude-0/r5/*` (this code) and `/tmp/claude-0/runs/*` (report 5's code).

**I also ran things.** I did not touch the repo. A scratch crate outside it (same `core`,
`worldgen` and `tape`) did three jobs:

- It read the year-50 and year-100 snapshots and listed each kind's members by mass and copy mass.
- It reran the eaters' start: 6 seeds, 64 x 64, 32 years, `c_build` 3 x 10⁴ and 10⁴, and 3 seeds
  with hand-set eater biases. It reproduces `ew run --seed 1 --size 64` exactly: 340 eaters
  placed, 177 members in 2 bodies at year 25, 3.596 x 10⁶ members at year 30, as in `real.log`.
- It reran the big-brain test from the year-40 snapshot, with a control kind beside it. It
  reproduces the clever run exactly: 2.93 x 10⁷ members and 3.20 x 10⁹ kg at year 60.

Run commands and outputs are in my scratchpad. Every other number has its arithmetic written out.

## Short answer

The books still balance (element drift 10⁻¹⁵, energy drift 10⁻¹³). The new tools are real gains:
the numbers table, `physics.json`, the per-kind mass columns, and brain sizes in likeness. But
four things make M4 unsafe to start now.

1. **The eaters' start is a lottery, and two laws were bent to win it.** In every run with
   neutral-brained founders, 97 to 99% of the placed eaters die in their first year. The kind survives through 2 to 14 bodies.
   - `c_build` was raised to the top of its range so that seeds 1 and 2 pass this bottleneck.
     That makes every life 3 times shorter.
   - "Satiety only past `full`" is a rule picked for the same reason.
   - With hand-set eater biases (an initial condition, already in the code), the bottleneck is 25
     to 77 bodies at `c_build` 10⁴.

   Fix the start, not the laws.

2. **"Every animal evolves small" is caused by a scaling mismatch, not by `take`.** Intake goes
   with mass^0.75. Strength, catalysts, learning and moving cost by the kg. So a 0.18 kg eater
   has 2 times the surplus per joule of upkeep of a 44 kg one (6.7 against 3.3). Watch report 6
   and handover 13.2 name `take` as the knob. Tuning `take` would tune an outcome.

3. **The big-brain verdict is wrong.** The 48-unit kind starts with hand-set biases. The
   ordinary eaters start with a neutral brain. I placed a 12-unit kind with the same hand-set
   start. It did better than the 48-unit kind: 3.40 x 10⁹ kg against 3.20 x 10⁹ at year 60. So
   the start won, not the brain.

4. **The population node sets the plants' demography, and the planned check cannot see it.**
   - Almost no plant member is past `full` (0.1% at year 50, 0.0% at year 100). So plants never
     copy by the physics path. Their births are set by the kept copy logit (a tuned number)
     times their decisions.
   - The nodes-against-individuals check keeps light-eaters as nodes in both modes.
   - Animals cannot be individuals at real numbers either.

Five claims of watch report 6 do not stand up to its own data (finding 8). The tuned-number
budget was sidestepped by a label (finding 9). Three tests pass for the wrong reason (finding 10).

**Do before M4:** fixes 1 to 6 of the list at the end, about a week. Then the 300-year seeds,
on at least three seeds, with the start fixed.

## Findings, by severity

### 1. Critical: the start is a lottery, and laws were tuned to win it

**What.** Founders with neutral brains starve within weeks. A kind lives only if a handful of
them breed before they die. Two decisions of 14 Oct were taken to make seeds 1 and 2 come out
alive:

- `c_build` was set at 3 x 10⁴ (the top of 1,500 to 30,000), not review 5's 10⁴.
- Satiety acts only past `full` (`choose.rs:604`).

**Evidence (my runs: 64 x 64, `ew`'s settings, eaters placed at year 22).**

| seed | `c_build` 3 x 10⁴: placed, bodies after 1 year, fewest bodies, members at year 30 | 10⁴ | 10⁴ with hand-set eater biases |
|---|---|---|---|
| 1 | 340, 9, **2**, 3.6 x 10⁶ | 484, 7, 3, 27 | 484, 82, 25, 1.6 x 10⁴ |
| 2 | 350, 5, **5**, 1.4 x 10⁵ | 412, 2, 0, **dead** | 412, 125, 77, 8.5 x 10⁴ |
| 3 | 516, 15, 14, 2.8 x 10⁶ | 488, 9, 0, **dead** | 488, 151, 52, 1.3 x 10⁵ |
| 4 | 302, 7, 7, 2.3 x 10⁶ | 384, 7, 0, **dead** | |
| 5 | 360, 14, 14, 6.3 x 10⁵ | 396, 4, 0, **dead** | |
| 6 | 478, 10, 10, 9.0 x 10⁶ | 494, 7, 1, 8.2 x 10³ | |

- At 3 x 10⁴, seed 1 keeps 9 of 340 eaters after one year (97% dead), then 2 bodies at year 25.
  `real.log` shows the same: 177 members in 2 bodies.
- With `Config::default`'s `exchange_days` 9 (the shell sets 3; finding 11), the same 3 x 10⁴
  loses the eaters on seeds 1, 3 and 5. A change in the rain path flips survival.
- The roster's three biggest animal kinds die outright under the current laws:
  - 150 kg kind: 219 placed at year 22, 0 at year 25.
  - 100 kg kind: 215 placed at year 26, 0 at year 29.
  - 600 kg kind: 224 placed at year 26, 0 at year 28.

  (`roster/series.csv`, `root_8/9/10_members`.) Watch report 6 does not mention this. The
  roster has had no eater of eaters since year 29. The 60% starting stores (`Kind::store`),
  tuned for exactly these kinds, no longer help.

**Why it matters.**

- `c_build` sets the life of every animal. At steady state a life is one over the birth rate, and
  an eater near `full` breeds as fast as `c_build` lets it.
  - At 3 x 10⁴: a 57 kg eater needs 82 days per child (handover's own number), and the measured
    life is 0.25 years.
  - At 10⁴: 245 days per child.

  So the choice made lives 3 times shorter. The owner asked for longer lives. Report 6 says
  "lives a little longer, not years" and does not connect the two.
- Every result after year 25 descends from 2 to 14 founders on one seed.
- Handover 2's lesson applies: the number was set so a run came out a certain way.

**Fix (M).**

1. Fix the start with initial conditions, which handover 7.1 allows when they are reported.
   - Place the founders with `--eater-bias` by default. Or place founders whose born rows come
     from a 10-year pre-run.
   - Place animals in two or three waves, years apart, each printed by its seeding line.
   - Do the same for the roster's big kinds.
2. Put `c_build` back to a central value, decided once, before any run.
   - Review 5's 10⁴ is fine.
   - Better: one coefficient from one consistent set of data. The review's own 50 kg and 1 t
     points give 1,450 and 1,500. Only the 1 kg point gives 27,000, an 18 times spread for a law
     that is meant to be one coefficient.
3. Rule: a law or number may not be changed because of how a start comes out. Any change decided
   by a run needs at least 3 seeds, and the report gives the bottleneck size: the fewest founder
   bodies, as in my table.

### 2. Critical: "small wins" comes from mixed scaling

**What.** Income goes with mass^0.75. Most upkeep goes with mass.

- `body.rs:1380`: strength, catalysts, learning and signal are `mass x (...)`. Basal and leaf
  go with the mass that works, to the 0.75.
- `choose.rs:467`: moving costs `c_move` per kg per m, flat.
- Intake (`Body::equiv`) and building (`c_build`) go with mass^0.75.

So the ratio of what a body can take to what it pays rises as the body shrinks. The brain's cost
per kg (mass^-0.75) is the only brake.

**Evidence (arithmetic: a strength-0.6 eater, burn gene 2.45, LEARN -1.0, repair -0.4, sugar
15.6 MJ/kg, 77% of what it takes is sugar).**

| per kg per day | 44 kg (normal run, interval 5.9 days) | 0.18 kg (roster kind 1 at year 100, 7.5 days) |
|---|---|---|
| basal 2 x 10⁴ x m^-0.25 | 7,760 | 30,700 |
| strength 10⁵ x 0.6 | 60,000 | 60,000 |
| catalysts (10^2.45 x 4,300 x e^-26.76 x 1.5 x 10⁹) | 4,350 | 4,350 |
| learning 3 x 10⁴ x 0.1 | 3,000 | 3,000 |
| brain 10⁵ x m^0.25 / interval / m | 990 | 48,200 |
| upkeep, x 0.939 (repair) | **71,500** | **137,000** |
| most it can take (0.05 x m^-0.25 x 0.77 x 15.6 MJ) | **233,000** | **920,000** |
| ratio | **3.3** | **6.7** |
| building per kg (3 x 10⁴ x m^-0.25) | 11,600 | 46,000 (3.95 times) |

With the same 3.3-to-6.7 gap, a small body pays for children 4 times faster per kg. That is the
whole story of the roster (`root_1_mass` 79.8 kg at year 25, 0.18 kg at year 100).

Moving gives small bodies another edge. Real cost of transport is about 10.7 m^-0.32 J per kg
per m: 3.6 at 30 kg (so `c_move` = 3 fits a founder), but 18.5 at 0.18 kg (6 times what the code
charges) and 1.4 at 600 kg.

The numbers table makes the clash visible, though no one has fixed it: `c_basal` and
`c_strength` are both anchored to "resting metabolism, Kleiber". One scales by kg^0.75, the
other by kg.

Handover 13.2 says "intake, building and basal upkeep all go with mass^0.75, and `take` ... feeds
a small body enough" and asks to "look at `take`". If every cost went with mass^0.75, size would
be neutral but for the brain. The cause is the terms that do **not** go with it.

**Why it matters.** Body size is the first thing M4's planet should select: cold, seasons, bands.
Now it is decided in the core by a unit choice. Tuning `take` down to stop it would tune an
outcome.

**Fix (S code, M with runs).**

- Charge strength, catalysts, learning and signal by `Body::equiv` (per kg-equivalent), like
  basal.
- Re-anchor each at the 30 kg founder so the start is unchanged at 30 kg: `c_strength` x
  30^0.25 = 2.34 times, and the same for `c_cat`, `c_learn`, `c_signal`.

With that:

- 44 kg: per-kg upkeep 65,700, ratio 3.5.
- 0.18 kg: 301,000, ratio 3.1.

Size becomes nearly neutral in surplus. Small bodies keep only their real edge, a shorter
generation (building ∝ m^-0.25).

- Charge moving as `c_move` x m^-0.32 per kg per m, anchored at 30 kg, so moving is about
  constant per kg^0.68.
- Rerun normal and roster at 3 seeds. Read `root_<k>_mass` at year 100.
- Leave `take` alone.

### 3. High: the big-brain test is confounded, and report 6's verdict does not hold

**What.** `worldgen/src/life.rs:227`: `hand = ... || which == 2`. The 48-unit kind always gets
the old hand-set biases, for example take-from-a-body +2 and take-from-the-ground +0.5
(`H_EATER`, line 43). It also gets genes sprayed at one generation's step, not 3. The ordinary
eaters start neutral.

So the two kinds differ in:

- the hand-set biases
- the brain sizes
- LEARN (-1.5 against -1.0)
- copy mass: 60 kg kept, against 25 kg evolved

Report 6 says the 48-unit kind "finds food" and "a big brain pays something here". It also says
404 were placed. The log says **216**, with "hand-set brain biases (old start)" (`clever.log`
line 8). 404 is watch report 5's number.

**Evidence (my control run).** I started from the same year-40 snapshot. I placed the ordinary
eaters' row (12 hidden) with the same hand-set biases, and tagged it as a separate kind.

| year 60 | ordinary (neutral start) | placed kind: members, kg, mean mass |
|---|---|---|
| 48-unit kind placed (as in report 6) | 2.3 x 10⁷, 1.18 x 10⁹ kg | 2.93 x 10⁷, **3.20 x 10⁹ kg**, 109 kg |
| 12-unit kind placed, hand-set | 1.2 x 10⁶, 6.1 x 10⁷ kg | 4.08 x 10⁷, **3.40 x 10⁹ kg**, 83 kg |

The 12-unit control:

- had more members: 4.1 against 2.9 x 10⁷
- had a little more mass: 3.40 against 3.20 x 10⁹ kg
- pushed the ordinary eaters down 19 times further

The control is not perfect. It placed 446 founders, not 216. It has smaller senses (6 memory, 3
seen, sight 1). Its LEARN is -1.0, not -1.5. Still, the hand-set start alone gives all of the
effect report 6 credited to the brain.

**Why it matters.** This is the one experiment about intelligence before M4. Its verdict goes
into the owner's check-point decision (handover 13.1).

**Fix (S).**

- Make `--clever-year` place a kind that differs from a control in brain size only:
  - Copy N living ordinary eaters: their genes and born rows, carried over by `brain::rerow`.
  - Set HIDDEN, MEMORY, SEEN and SIGHT.
  - Place the same N copies with 12 units as a control.
- Correct report 6 (216, hand-set, verdict withdrawn) and handover 13.2.

### 4. High: the node sets the plants' life, and the planned check cannot see it

**What.** A node copies by physics only when its *mean* member is past `full` (`body.rs:1358`).
Below that it copies by its brain's copy chance (`choose.rs:214`). Each birth stays in the node
and pulls the mean down.

**Evidence (snapshots, normal run, by members).**

| kind | year | share of members past `full` | share past copy mass | mean member / copy mass |
|---|---|---|---|---|
| plants | 50 | 0.1% | 58% | 1.09 |
| plants | 100 | **0.0%** | 45% | 0.98 |
| eaters | 50 | 42% | 99.8% | 1.93 |
| eaters | 100 | 47% | 99.9% | 1.94 |

- 100% of plant members live in nodes, and almost none is ever past `full`. So plant births per
  member a year are, at most, decisions a year x the copy chance: 360 / 21.9 days x 1 / (1 +
  e^1.0) = 16.4 x 0.27 = **4.4**, if the logit is still near its start.
- The measured rate is 2.9 a year, life 0.37 years.
- The build law does not bind for plants. A 1.8 kg plant owes 1 x 10⁷ x 0.35 (CHILD) x 0.18 kg
  dry = 6.2 x 10⁵ J. One 22-day interval of building is 3 x 10⁴ x 1.8^0.75 x 22 = 1.0 x 10⁶ J.
- So the plants' life, about 90% of the biomass, comes from a tuned behaviour (the kept copy
  logit, `H_LIGHT`) and from where the node's mean sits. No law sets it.
- As individuals, a plant at twice its copy mass would copy by physics at every update.

The planned check (handover 7.1) has two holes:

- `--individuals` keeps light-eaters as nodes, so this cannot show up in it.
- Animals as individuals reach 69,135 bodies by year 25 at 48 x 48 (report 6). So the check can
  only run with about 10⁵ animals, while the runs it is meant to vouch for have 10⁷ to 10¹¹
  animal members.

Also, every mass^0.75 law (intake, basal, build, leaf) is evaluated at the node's mean member.
With members of mixed size, n x f(mean) is more than the sum of f(m_i). Example: half at 0.3 m
and half at 1 m gives 0.724 against 0.703 (+3%). Small, but always in the node's favour.

**Why it matters.** Lives, generation times and the plants' response to anything on the planet
would be properties of the node rule. M4 adds more node machinery (coarse regions).

**Fix (M).**

1. A check world where everything can be an individual. Add a cell-size parameter to
   `worldgen::new_world`; it is fixed at 2,000 m in `terrain.rs:75`. Turn `Config::nodes` into
   three settings: all nodes, plants only, none.
   - At 20 m cells (400 m²) a cell holds 40 plants of 1 kg at 0.1 kg/m².
   - A 32 x 32 world then has about 4 x 10⁴ plants and a few thousand animals: individuals for
     both.
2. Run the 7.1 criteria there, with plants included.
3. A convergence check in the real world: vary `crowd` (1, 4, 16) and `alike` (2, 4, 8). A
   pattern that moves with them is the node's.
4. A tape column `root_<k>_past_full`: the share of members past `full`. It shows the regime at a
   glance.

### 5. High: building a body costs nothing; only the child's share is charged

**What.**

- `copy.rs:246`: `owe = c_copy x s x (mass - wet)`. The work is paid only on the share given at
  birth.
- A child then grows to its copy mass by eating. That pays only `c_handle` (10³ J/kg) and upkeep.
- `c_build` caps only payments into the child store (`copy.rs:251`).

**Evidence (arithmetic, normal-run eater).**

- At year 100, CHILD is 0.29 and the mean member 44 kg. The child is about 13 kg, and it grows
  31 kg by eating.
- The dry share is 0.29 (liquid share 0.71, from the snapshot). So the eater builds 9 kg of dry
  body.
- At `c_copy` that would be 9 x 10⁷ J. It pays 3.1 x 10⁴ J of handling, 3,000 times less.
- So about 70% of every adult body is built for free and at no limited rate.

The anchor review 5 used, von Bertalanffy, is the growth law of a body itself, not of what a
parent hands over.

**Why it matters.**

- "Building takes time" binds only on the parent's share. A kind can evade it by giving smaller
  children.
- The "satiety only past `full`" rule exists because a growing body below `full` must eat more
  than upkeep plus building, since its growth was not counted as building.

**Fix (M).**

- Count a member's own growth of dry mass as building. While its dry mass is below its copy
  mass, pay `c_copy` per kg of dry mass gained, from the same `c_build` allowance per day.
- Then satiety can be one law at every size: take no more than upkeep plus the building
  allowance. The "past `full`" exception goes away, and `full` can go back to being the copy
  trigger only.
- This makes the start harder. Do it with fix 1's start, never instead of it.

### 6. Medium: `c_build` can be paid twice in one update

**What.** `body::day` runs `live` and then `choose::act` in the same update (`body.rs:1003-1008`).

- `live` copies when the mean member is past `full` (`body.rs:1358`) and pays up to `c_build` x
  equiv x interval (`copy.rs:251`).
- If that made births, the member falls below `full`: an individual keeps (1 - CHILD) x `full` x
  copy mass, which is 0.7 x 2 = 1.4 copy masses at CHILD 0.3.
- Then the brain's copy (`choose.rs:214`, gated by `pm < full`) calls `copy` again. It pays
  another full interval's allowance.

**Why it matters.** The documented cap is "never more than `c_build` x equiv x the days". The
eaters sit at `full`: mean ratio 1.94 and 47% past it. With an eater's copy chance of about 17%
(logit -1.6), their real build rate is up to about 17% above the number in the table. No test
covers two calls in one update.

**Fix (S).**

- Keep `Body::paid_day`, the day of the last payment.
- Cap each payment by `c_build` x equiv x (day - paid_day), at most one interval.
- Add a test: one update with a physics copy, then a brain copy, pays at most one interval.

### 7. Medium: `merged_unlike` = 0 reassures about the wrong event

**What.** `merged_unlike` counts only merges past twice `crowd` (`body.rs:941`). The crowded
merge before that, and every child's join (`copy.rs:104`), accept pairs whose means differ by up
to `alike` + 2 sqrt(s1 + s2) steps (`body.rs:924`).

- With both spreads at their cap, `most` = step² x (16π/8 + 1) = 7.28 step² per node.
- That is 4 + 2 sqrt(14.6) = **11.6 steps** in every gene.
- For the copy-mass gene (0.2 per step) that is 2.3 decades: copy masses 200 times apart.
- For a catalyst it is 3.5 decades.

These merges are not counted.

**Why it matters.**

- Spreads reach the cap where cells are full, because a node cannot split there
  (`copy.rs:525`). The roster run has up to 8 nodes per cell.
- Report 6 and handover 7.1 read `merged_unlike` 0 as "the safeguard did not write biology". It
  does not show that.

**Fix (S).** Add a column `merged_wide`: members merged or joined whose means differ by more than
`alike` steps in some gene. Report it next to `merged_unlike`.

### 8. Medium: claims of watch report 6 the data do not support

| claim (report 6) | data | verdict |
|---|---|---|
| "The eaters live 6 months at year 50, then 3 months at year 100, because they evolved smaller" (line 33) | normal run: copy mass 25.7 to 22.7 kg, mean 49.6 to 44 kg, a 3% change of m^0.25. Life fell 0.52 to 0.25 years. In report 5's run life fell the same way, 0.30 to 0.14, at a constant 57 kg | not "because smaller"; it falls as eaters grow 2.2 x 10⁷ to 4.0 x 10⁷ |
| "In the roster run every animal kind shrank" | 3 of 5 animal kinds (150, 100, 600 kg) died within 1 to 3 years of placement and are not in the report's table | misleading |
| "The normal run is 3 times faster" (line 41, under what changed) | flow, whose code did not change since report 5, went from 1,079 to 296 ms a year; bodies from 5,066 to 1,491 | machine load, not the laws |
| "404 eaters with a 48-unit brain" (line 121) | `clever.log`: 216, hand-set | wrong number, start not said |
| "a big brain pays something here: it finds food" | finding 3 | not supported |
| "The eaters of remains went down to 50 to 60 g" | its own table: 75 and 83 g | small slip |
| "`c_build` ... anchored" (table, `numbers.md:156`) | the handover's own words: "a tuned choice within the anchor" | mislabelled (finding 9) |
| "The plant giants are about 300 times smaller" | true (2.7 t), but the giants are 2 to 3.8 times their copy mass (680 to 830 kg) and mostly liquid | swelling past `full` is still there (finding 12) |

**Fix (S):** a short correction section at the top of report 6, as report 4 got.

### 9. Medium: the tuned-number budget was sidestepped

**What.**

- `numbers.md` defines tuned as "set so a run came out a certain way". `c_build` was set so seeds
  1 and 2 kept their eaters, but it is marked **anchored**. The budget test counts rows whose mark
  starts with "tuned", so the count stayed at 67.
- `c_basal` = 2 x 10⁴ is 1/15 of Kleiber's 2.9 x 10⁵ (3.4 W x 86,400 s). Review 5 picked it so
  that "a 1 kg plant still keeps a spare of over twice its upkeep". That is a run-shaped choice
  too.
- The rules picked for outcomes are outside any count:
  - satiety only past `full`
  - the kept copy logits `H_LIGHT` and `H_EATER`, which set the plants' demography (finding 4)
  - the hand-set clever start
  - the roster stores
- The `full` row still says "It goes away with satiety". It did not. It now also gates satiety.

**Why it matters.** Handover 14.3's rule ("the count may not grow") is the main guard against
lesson 3 of version 1. It already failed once, silently, the day it was written.

**Fix (S).**

- Mark `c_build` and `c_basal` tuned. The count becomes 69. Keep `TUNED_BUDGET` at 67 and retire
  two tuned numbers to get back under it (for example, anchor `c_move` per fix 2).
- Add a table "rules chosen for a run" to `numbers.md`, with its own count and test, listing the
  four above.
- Fix the `full` row.

### 10. Medium: tests that pass for the wrong reason

- `core/tests/nodes.rs:432` `individuals_mode_merges_no_animal`: "over three years no animal
  becomes a node". I ran the same scene. Animals alive at months 6, 12, 18, 24, 30, 36 were
  7, 2, 1, 1, 1, **0**. The `all(count == 1)` check is empty for most of the window.
  **Fix (S):** place hand-set eaters or feed them. Assert that animals are alive at the end, and
  that animal births in the window are more than 0.
- `core/tests/life.rs:483` `a_body_takes_no_more_than_it_can_use`: with `c_build` 3 x 10⁴ and 0,
  both cases take the upkeep's worth within 2%. The body holds fuel for many intervals, so `can`
  is negative and the floor `.max(up)` decides. The new formula, `(up + build) x fuel / rel -
  fuel` (`choose.rs:611`), is never exercised. **Fix (S):** add a body past `full` with a store
  below the target, and assert that what it takes rises with `c_build` as the formula says.
- `core/tests/life.rs:675`: the doc comment says 10⁴ (`c_build` is 3 x 10⁴), and it does not
  cover finding 6. **Fix (S).**

### 11. Medium-low: the shell runs a different world than `Config::default`

`cli/src/main.rs:41` sets `exchange_days` to 3. `Config::default` has 9 (`lib.rs:455`). Tests,
the web build and any driver that uses the default run the other rain path. It matters: at
`c_build` 3 x 10⁴ the eaters survive on 6 of 6 seeds with 3, and on 3 of 6 with 9 (finding 1).
`physics.json` records it, but nobody reads it for this.

**Fix (S):** make the default 3 (or make `ew` stop overriding it), so tests run the world the
reports describe.

### 12. Low: plants swell with free liquid

- Leaf (`body.rs:282`, `1166`) and height (`shade`, by member mass) count the whole mass,
  liquid included. The copy work counts dry mass only (`copy.rs:246`).
- Satiety does not cover light or liquid.
- Every plant kind ends 90 to 93% liquid (snapshot).
- The 12 heaviest roster members are 1.6 to 2.7 t, at 2.0 to 3.8 times their copy mass (680 to
  830 kg). Eleven of them are 57 to 96% liquid; one is 17%.

**Fix (S/M):** count leaf and height by the dry mass that works. Or let copying past `full` move
liquid out with the children at no work. Decide with fix 5.

### 13. Low: satiety assumes the release is first order in the fuel

`can = (up + build) x fuel / rel - fuel` (`choose.rs:611`) is right only when the fuel is the
scarce input of the burning. When the gas is scarce (under liquid, `gas_depth`) or the catalysts
unfold, `rel` is small for a large `fuel`. Then `can` grows without bound: no satiety where
burning is limited.

**Fix (S):** cap `can` at `take` x days x equiv, or compute the target from `plan`'s scarce input.

### 14. Low: doc drift

- Handover 8.3 says `root_<k>_gas` is "mol of air per kg per day" and `_leaf` "m² per kg". They
  are per m² of open surface, and per kg-equivalent.
- `numbers.md`, handover 14.3 and three code comments are dated 15 Oct 2026. The commits are of
  14 Oct.
- Handover 13.2's "look at `take`" should point to finding 2.

## What is fine

- **The books.** Element drift is 10⁻¹⁵ and energy drift 10⁻¹³ over 100 years in every run.
  `copy::open` conserves exactly and draws from the spread (its test is sound).
- **Repeatability.** My scratch driver reproduced the runs exactly (seed 1 eaters, clever kind),
  so results here are checkable.
- **Brain sizes in likeness by whole units.** It works: `root_2_hidden` stays 48.0 for 60 years.
- **Leaf by mass^0.75.** It removed the 3,400 t members (now 2.7 t).
- **Basal upkeep.** It ended the strength-0 free lunch. Its scaling is right; only its siblings'
  scaling is wrong (finding 2).
- **Transparency.** The numbers table, its test, `physics.json`, `ew numbers`, `root_<k>_kg`,
  `mass_max_root` and `life_real` all did their job. Most findings here came from them.
- **Honest prose.** The handover's own text says `c_build` is "a tuned choice within the anchor".
  Only the label disagrees.
- **Review 5's section 3.** A body-size cause instead of a maturity rule was the right call.
- **Report 6's own record.** It says plainly that the individuals check could not run.

## Fixes in order

| # | Do | Size | Finding |
|---|---|---|---|
| 1 | Start: founders placed with hand-set biases (or born rows from a pre-run), in waves, for every animal kind including the roster's big ones; seeding lines print it; every report gives the bottleneck (fewest founder bodies) on 3 seeds | M | 1 |
| 2 | `c_build` back to one central value (10⁴, or one consistent fit of the review's data) and marked tuned if any run chose it; no law changed for a start | S | 1, 9 |
| 3 | Strength, catalysts, learning and signal upkeep by `Body::equiv`, re-anchored at 30 kg (x 30^0.25); `c_move` x m^-0.32 anchored at 30 kg; rerun normal and roster on 3 seeds; leave `take` alone | S + runs | 2 |
| 4 | Big-brain test with a control that differs only in brain size (copies of living eaters, `rerow`); correct report 6 and handover 13.2 | S | 3 |
| 5 | Check world with small cells and all bodies as individuals (plants too); `root_<k>_past_full` column; `crowd` 1/4/16 and `alike` 2/4/8 convergence run | M | 4 |
| 6 | Pay-day cap so `c_build` cannot be paid twice in one update, with a test | S | 6 |
| 7 | Tests: animals alive in `individuals_mode_merges_no_animal`; a satiety case below the target that depends on `c_build`; stale comment | S | 10 |
| 8 | `Config::default` `exchange_days` = what `ew` runs | S | 11 |
| 9 | `numbers.md`: `c_build`, `c_basal` tuned; table of rules chosen for a run, counted; `full` row fixed; budget lowered by retiring two | S | 9 |
| 10 | `merged_wide` column | S | 7 |
| 11 | Correction section in report 6 | S | 8 |
| 12 | Own growth counted as building; satiety at every size; `full` only as the copy trigger | M | 5 |
| 13 | Leaf and height by dry mass; `can` capped for gas-limited burning; doc drift | S | 12, 13, 14 |
| 14 | Then the 300-year seeds (3 seeds, 128 x 128), then M4 with the individuals check (row 5) as its first job | | |

Rows 1 to 6 decide what the next runs mean, so do them before any long run. Row 12 changes the
start again, so do it together with row 1 or right after it. Do not do it alone.

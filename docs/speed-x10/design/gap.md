# Speed x10, section "gap": what closes the road to 10 times on 4 cores

> **After P1.2** (X62 to X69): G2 is in `lane-a.md` 3 (the brain floored on today's law, `think_days`; no P1b); G3 is
> dropped (X68, its alternative the measure GB); G9a runs on the day law (`lane-a.md` 2.3).
>
> **Note: what the plan kept** (`plan.md` is the single source of the steps; this design keeps its numbering).
> G0: in R2. Kept: G1, G2 (needs P1b, `animals.md`), G3, G4, G5, G6, G7, G8, G10 (= AIR4), G11, G12 (its clock fix
> is in R3), G13, G14, G15, G16, GM. **G9: split** into G9a (`field_days` 90) and G9b (compact stands, gated, level 1,
> X26). **G14-P runs twice:** G14-P1 in phase P on today's code, G14-P2 before G14. The ceiling is 12,000 (X22).
> The budget of section 2 is the plan's section 5, restated after R2; the plan counts each step once in landing
> order (X33). **After the second review** (the plan holds the corrected numbers): G14's bound counts the serial parts
> and the lumpy month windows (island 79 to 98 ms, 9.8 to 12.2 times; globe 101 to 126 ms, 13.1 to 16.3 times; the
> 75 to 88 and 95 to 112 below are the first bound), and the plan adds a "likely" column (island about 8.5 to 10.5
> times, globe about 12 to 15). G2 does not need MI3a; MI3a and MI4 are gated again after G2 (X36). G3 keeps only the
> closed form of the heat: keep-warm is a drain in P1b's balance (X14). G1's rows are booked after the call. G14 cuts
> each reaction as a whole (X21). G8 is level 1 for the world and level 2 for the tape. G5's fallback is expected
> (X18); G12's gain is a guess until R3 (X19). `field_days` is 30, 60 or 90 from P1.2 on (X17).

Design section of the x10 plan (`plan.md` section 1: "the plan also carries a gap design"). It takes the four designs
(plants, engine, air, minds) as they are, adds the strategies that close the gap, and gives the arithmetic. Written in
simple words for the owner. Code at `0f891fe` (branch `speed/x10`).

## 0. In short

- **The goal, strictly:** a settled year on 4 cores 10 times faster than the start of the last round: island22
  0.910 s to 0.091 s, globe36 1.490 s to 0.149 s. Today 0.614 s and 0.738 s (Z1). So from today we need 6.7 times
  (island) and 5.0 times (globe).
- **The plan's kept steps give about 1.7 times (island) and 1.6 times (globe)** in my budget (2.5 and 3.3 times the
  baseline). The gap left is **4.0 times (island) and 3.1 times (globe)**.
- **New measures (a probe in scratch, this work):**
  1. An animal node updates every **8.5 days** (island) and **9.2 days** (globe) on average. A floor of 30 days keeps
     only **28% and 31%** of their updates. Measured with today's code: the bodies' stage falls to **0.65 and 0.53** of
     itself. This is the biggest lever left.
  2. Change's nodes almost never sleep: **75%** of ground nodes rest under 10 days. Change costs 54 ms on 4 threads,
     of which **24 ms is a booking on one core** and 27 ms are rows in tiny tasks that 4 threads speed up only 1.3 times.
     A 30-day physics step cuts Change to **0.38 to 0.41** of itself (measured, cost only).
  3. One batch per 10 days (not 2): the per-update wall time in the parallel phase falls to **0.77 to 0.83**, faces to
     0.22 (measured).
  4. The machine: one world's update costs **31 to 36% more on 4 threads** than on 1. Four separate one-thread worlds
     each run **18% slower** than one alone. So about half of the slowdown is the machine, half is sharing inside our
     process. Code on one core runs **25 to 30% slower** while the 4 threads are on (liquid 30 to 39 ms, Change's book
     19 to 24 ms). On this machine 4 cores give at most about **3 times**, even for perfect parallel work.
- **What this means:** a year of 91 ms on 4 cores needs the one-thread work of a year cut to about **200 to 250 ms**
  (island; today 1,000 ms in the probe world) and **350 to 420 ms** (globe; today 1,230), *and* about 85% of what is left
  must run in parallel. Work must fall **4 to 5 times** (island) and **3 to 3.5 times** (globe).
- **The strategies (G1 to G16), with their gain on top of the plan (4 threads, island / globe, ms of a year in the
  probe world; the target there is 96 / 165 ms):**

| Step | What | Island | Globe | Law change? |
|---|---|---|---|---|
| G1 | Change per domain: its rows and its booking in one pool call | -32 | -32 | no (level 2) |
| G7 | Bond's monthly part turns per domain on the pool | -4 | -15 | no (level 2) |
| G8 | The tape's recorder reads on the pool | -8 | -9 | no (level 1) |
| G6 | The whole survey per domain on the pool | -14 | -20 | no (level 2) |
| G15 | Remove the sharing between threads inside the process | -13 | -17 | no (level 1) |
| G13 | One heat draw per cell for stands | -3 | -3 | no (level 2) |
| G4 | One batch per 10 days | -26 | -39 | small (level 3) |
| G5 | A physics step of 30 days (the air stays at 10) | -23 | -23 | yes (level 3) |
| **G2** | **Animal nodes update at least every 30 days; every rate follows the pace** | **-59** | **-101** | **yes (level 3)** |
| G3 | The animal month: P1b's closed form also makes the animal's body cheap | -9 | -12 | with P1b |
| G9 | Stands at a season's pace, compact stands | -10 | -6 | yes (level 3) |
| G12 | The liquid every 60 days | -13 | -14 | yes (level 3) |
| G10 | AIR4 calm months (in the plan as research; counted here) | -32 | -38 | yes (level 3) |
| G11 | f32 and SIMD for the air's cell rows | -6 | -9 | no (level 2) |
| **G14** | **Physics beside the bodies: Flow and Change run on the pool at the same time as the batch** | **-48 to -60** | **-55 to -70** | **yes (level 3)** |
| G16, GM | Well-being once per decision; the cost of a thought measured | -1 | -2 | small |

- **Result** (Z1 scale, against the baseline): all steps but G14: island **7.4 times**, globe **10.0 times**. With G14:
  island **10.9 to 12.8 times**, globe **14.7 to 17.4 times**. Without AIR4 (G10) but with G14: island **8.9 to 10.6**,
  globe **12.3 to 14.5**. Without G2: island at most **6 to 7 times**.
- **Plainly:** 10 times on 4 cores **is reachable on paper, but only with the whole set**, and two of its steps are
  hard: **G2** (animal nodes at a 30-day pace, which needs P1b first) and **G14** (physics beside the bodies). The globe
  reaches 10 times with G2 and either AIR4 or G14. The island needs G2 *and* G14, and is at the edge unless AIR4 passes
  too. If G14 or AIR4 fail, expect about **6 to 7.5 times** on the island and **8 to 10 times** on the globe.
- **Main rule changes:** (1) every population node updates at least every `node_days` (30), plants and animals alike,
  and every rate of an update, also the share of members that move, follows its pace; (2) `batch_days` 10 and
  `phys_days` 30 (the air step stays 10 days); (3) the physics of a window is worked out from the store at the window's
  start and booked after the bodies, each node's outgoing moves cut to what it then holds; (4) the core ceiling 12,000
  lines, or 11,500 if E5 and AIR6 give way to G7 and G11.

---

## 1. What exists today, and what I measured

### 1.1 The probe

A copy of `0f891fe` in scratch (`x10/gap-work`, not in the repository), with timers around the update, its body day,
its decision, the batch's phases, the survey, Change's rows and booking, Flow's parts and the liquid's parts. It
resumes the year-200 snapshots of fresh runs made with today's code (`x10/plants-run/{island22,globe36}`, snapshot
version 47; the Z1 bench saves are not on this machine), 2 years, 4 threads and 1 thread. Its year is 646 ms (island)
and 818 ms (globe) on 4 threads, near Z1's 614 and 738, so I use its shares and scale the target the same way:
**96 ms (island) and 165 ms (globe) in the probe world** stand for Z1's 91 and 149. The machine drifts by about 10%
between hours: read ratios inside one set of runs.

### 1.2 Where a year goes now (4 threads, probe world, ms)

| Part | island22 | globe36 | Code |
|---|---|---|---|
| plant updates (their share of the parallel phase) | 136 | 80 | `survey::update` (`core/src/survey.rs:262`) |
| animal updates (their share) | 140 | 220 | same; `body::live`, `choose::act` |
| the batch's serial phase: booking, settling, outboxes (75 / 120), faces (16 / 14), bodies with parts (14 / 32) | 105 | 166 | `survey::batch` (`survey.rs:209`), `on_domains` (`survey.rs:245`) |
| the survey (the part not inside `on_domains`) | 25 | 44 | `survey::survey` (`survey.rs:28`) |
| Flow: the air step | 100 | 136 | `flow::air` (`core/src/flow/mod.rs:46`) |
| Flow: the bodies' draws and the air's state | 13 | 13 | `survey::Draws::settle` (`core/src/survey/draws.rs:72`) |
| Flow: the liquid (slopes and springs 5, surface 24, under 10) | 39 | 48 | `flow::liquid` (`flow/mod.rs:56`, `flow/liquid.rs`) |
| Flow: the state process | 10 | 11 | `flow::state` (`flow/mod.rs:29`) |
| Change: rows 27, booking 24 | 54 | 54 | `change::step` (`core/src/change/mod.rs:26`) |
| parts (Bond's month) | 7 | 28 | `bond::step` (`core/src/bond/month.rs:12`) |
| tape | 17 | 17.5 | `Recorder::record_month` (`tape/src/rec/mod.rs:160`) |
| **year** | **646** | **818** | |

**One update** (1 thread / 4 threads, µs):

| | island22 | globe36 |
|---|---|---|
| plant update | 16.1 / 21.9 (body day 11.2) | 17.2 / 23.1 |
| animal update | 34.9 / 45.9 | 42.3 / 56.4 |
| of it: the body's day (`live`) | 15.4 | 15.2 |
| of it: the decision (`choose::act`) | 13.5 | 21.3 |
| of it: the rest (children, select, mature, split) | 6.0 | 5.8 |
| updates a year: plants, animals | 18,072, 8,846 | 10,573, 11,840 |
| animal updates by nodes / by individuals | 96% / 4% | 98% / 2% |
| mean interval of an animal node | 8.5 days | 9.2 days |
| animal node updates kept by a floor of 20 / 30 / 36 / 60 days | 0.42 / 0.28 / 0.24 / 0.14 | 0.46 / 0.31 / 0.26 / 0.15 |

**Change's sleep** (`clock/sleep.rs`, `change/mod.rs:18`): due ground nodes 11,307 a year of 17,424 (65%) on the
island, 13,291 of 23,328 (57%) on the globe. Of the rests a node takes after reacting, **75% are under 10 days**
(island: 16,857 under 10 days, 1,971 from 10 to 30, 3,786 from 30 to 89, none at 90). Only 19% of the due nodes were
woken by a gain (`Sleep::watch`); the rest are due by their own alarm: their reactions use 5% of an input (`CALM`) in
under 10 days. A row costs about 2.3 µs; the longest row 0.4 ms. The rows are 314 tiny tasks a step: their thread time
on 4 threads (51 ms) is 1.55 times their one-thread time (33 ms), so 4 threads gain only 1.3 times.

**The clock's settings, cost only** (2 years from the same snapshot, 4 threads; the world drifts, so read these as
cost, not as a check of the world):

| Setting (today: phys 10, batch 2, node 36, animal nodes by gene) | island22 year | globe36 year | What moved |
|---|---|---|---|
| today (same set of runs) | 729 | 853 | |
| `phys_days` 30 | 646 (0.89) | 821 (0.96) | Change 61 to 23 and 60 to 25; survey 55 to 34 and 60 to 45 |
| animal node floor 30 days | 539 (0.74) | 589 (0.69) | animal updates 8,846 to 2,124 and 11,840 to 3,002; bodies 461 to 299 and 528 to 278; parts on the globe 28 to 10.5 |
| `batch_days` 10, `node_days` 40 | 483 (0.66) | 641 (0.75) | faces 20 to 4.4; serial phase 80 to 46; parallel-phase wall per update 11.6 to 8.9 µs and 13.7 to 11.4 µs |
| all of them, `node_days` 60 | **399 (0.55)** | **444 (0.52)** | bodies 461 to 182 and 528 to 169; Change 61 to 23 and 60 to 26 |

**A warning in the same runs:** with today's laws, the animals lose about half their members in 2 years at a 30 to 40
day pace (island members 3.0e5 to 0.98e5; globe 2.7e10 to 1.3e10). This is C2's finding again (upkeep paid before the
interval's income). **So G2 must wait for P1b** (the month law for animals). The cost numbers stand; the world must be
checked after P1b.

**A bug found on the way:** `flow_days` 60 does not make the liquid run every 60 days. `clock::Every` caps a period at
its `within` (`core/src/clock/mod.rs:47`), and the liquid's `within` is the month (`schedule.rs:54`), so it still runs
monthly for a 30-day span. `Config::validate` does not refuse it. G12 fixes this.

### 1.3 The machine (4 threads)

| Measure | Value |
|---|---|
| one world: an update on 4 threads against 1 thread | plants 1.36 times, animals 1.31 times dearer |
| four separate one-thread worlds at once: an update against one world alone | 1.10 to 1.30 (mean 1.18) times dearer |
| code on one core while the 4 threads are on, against 1 thread | liquid 1.30, Change's booking 1.27 times slower |
| the threads busy in the parallel phase of a batch | 2.9 (island), 3.0 (globe) of 4 |

So 4 cores give at most about 3.4 times for separate worlds, and about 3 times for one world's perfect parallel work.
The other half of the slowdown (one world dearer than four separate ones) is ours: it is G15.

---

## 2. The budget

### 2.1 After the plan's kept steps

The plan's sure steps: P1, P2 (`field_days` 60), P3; E1a to E1c, E2; AIR1 to AIR3; MI1, MI3, MI4, MI5. Each line uses
the designs' own numbers (4 threads, probe world, ms):

| Part | island today | island after plan | how | globe today | globe after plan | how |
|---|---|---|---|---|---|---|
| stands (plants) | 136 | 27 | 16.1 to 6 µs (x0.37), 60 days (x0.6), tiles x0.88 | 80 | 16 | same, tiles x0.9 |
| animals | 140 | 102 | decisions -45% (minds 6), tiles | 220 | 154 | decisions -43%, tiles |
| batch serial phase | 105 | 35 | engine 6.1 (63% moved), plants' litters | 166 | 65 | engine 6.1 (54% moved), MI5 |
| survey | 25 | 30 | P3 adds the stands' field step | 44 | 44 | |
| Flow (air step, draws, liquid, state) | 162 | 110 | air 6: x0.68 | 208 | 125 | air 6: x0.60 |
| Change | 54 | 54 | nothing in the plan | 54 | 54 | nothing |
| parts | 7 | 7 | nothing | 28 | 28 | nothing (E5 gated) |
| tape | 17 | 17 | nothing | 17.5 | 17.5 | nothing |
| **year** | **646** | **382** | **1.69 times; 2.51 times the baseline** | **818** | **504** | **1.62 times; 3.28 times the baseline** |
| target | | **96** | gap **4.0 times** | | **165** | gap **3.1 times** |

This agrees with the review (1.6 to 2.2 and 1.46 to 1.72 times). Note: after the plan, **Flow, Change, parts and tape
alone are 188 ms (island) and 225 ms (globe)**, about twice the island's whole target.

### 2.2 Step by step (4 threads, probe world, ms; "x" is against the baseline, Z1 scale)

Order: first what changes no law, then the law changes, then the research. The script is in scratch
(`x10/gap-work/py/budget.py`).

| Step | island: saves, year, x | globe: saves, year, x |
|---|---|---|
| after the plan | 382, x2.51 | 504, x3.28 |
| G1 Change per domain | -32, 350, x2.74 | -32, 472, x3.50 |
| G7 parts per domain | -4, 346, x2.77 | -15, 456, x3.62 |
| G8 tape on the pool | -8, 338, x2.83 | -9, 448, x3.69 |
| G6 survey on the pool | -14, 325, x2.95 | -20, 428, x3.86 |
| G15 sharing between threads | -13, 312, x3.07 | -17, 411, x4.02 |
| G13 one draw per cell | -3, 309, x3.10 | -3, 408, x4.05 |
| G4 one batch per 10 days | -26, 283, x3.39 | -39, 369, x4.48 |
| G5 physics step 30 days | -23, 259, x3.69 | -23, 346, x4.78 |
| **G2 animal nodes at 30 days** | **-59, 200, x4.79** | **-101, 245, x6.74** |
| G3 the animal month (cost) | -9, 191, x5.02 | -12, 233, x7.10 |
| G9 stands at a season, compact | -10, 181, x5.30 | -6, 227, x7.29 |
| G12 liquid every 60 days | -13, 168, x5.70 | -14, 213, x7.76 |
| G10 AIR4 calm months (research) | -32, 136, x7.06 | -38, 175, x9.45 |
| G11 f32 and SIMD cell rows | -6, 130, x7.38 | -9, 166, x9.96 |
| **G14 physics beside the bodies** | **75 to 88, x10.9 to 12.8** | **95 to 112, x14.7 to 17.4** |

**What is left before G14** (island / globe, ms): stands 10 / 6, animals 16 / 27, batch serial 21 / 34, survey 10 /
18, air step 31 / 36, draws and state 4 / 3, liquid 14 / 15, state 3 / 3, Change 9 / 9, parts 2 / 5, tape 9 / 9.
The parts run one after the other, and many of them on one core.

**How G14 is counted.** It does not cut work; it fills idle cores. The bound is the thread time of all parts over 4
cores, times an imbalance of 1.1 to 1.3. Thread time = wall time times the threads busy in that part (stands and
animals 3.3; Change 2.8; the survey, parts and air 1.8 to 2.0; the batch's serial phase, liquid and tape 1.5). Island:
271 thread-ms over 4 = 68 ms, so 75 to 88 ms. Globe: 345 over 4 = 86 ms, so 95 to 112 ms.

### 2.3 Which steps carry the goal

| Set | island22 | globe36 |
|---|---|---|
| plan only | 2.5 times | 3.3 times |
| plan + G1, G4, G6, G7, G8, G13, G15 (no world change but G4) | 3.4 | 4.5 |
| all but G14 | 7.4 | 10.0 |
| all but AIR4 (G10), with G14 | 8.9 to 10.6 | 12.3 to 14.5 |
| all but AIR4 and G11, with G14 | 8.7 to 10.2 | 11.8 to 14.0 |
| all but G5, with G14 | 9.0 to 10.6 | 12.7 to 15.0 |
| all but G2 and G3, with G14 | 6.1 to 7.2 | 7.5 to 8.8 |
| **all** | **10.9 to 12.8** | **14.7 to 17.4** |

So: **G2 cannot be missed.** The island also needs **G14**; AIR4 gives the margin. G5 is worth about one step of the
range. Every other step is small alone, but together they are a third of the road.

---

## 3. The strategies

Each strategy: what it does, the rule or law it changes (exact words), why it stays honest, the check, the gain, the
lines, the risks. The steps table is in section 9.

### G1. Change per domain

**What.** `change::step` (`change/mod.rs:26`) makes one row per due node with `par::rows` (314 tiny tasks a step),
then books all rows on one core (`Batches::book`, `change/mod.rs:114`, through `store::apply`). New: one pool task per
domain (the cells of one air node, or a tile). The task reacts its due ground nodes in cell order and makes their rows;
it books nothing. After the call the rows are booked by the engine's `book_by_array` (E1c), each array by one task;
the gas each cell gives its air node and the lift are summed per domain, in cell order, and added to the air node in
domain order. Air nodes react in their own short call as now. One shared helper with the engine's booking by array
(E1c) and E4. Because the rows exist before they are booked, G14 can book the same rows after the bodies (with its
cut) and does not redo G1.

**Rule or law.** None. The order of sums into an air node changes (per domain, then across domains).

**Honest.** Each ground number has one writer (its array's booking task); the air node gets the sum of its cells' rows, cut as now
(a node gives no more heat than it holds, `react_ground`, `change/mod.rs:124`). `store::apply`'s rule ("only adds, sets
rounding below zero to zero and counts it") is kept by the helper.

**Check.** Level 2: golden re-pinned; two years of each bench give the same series to rounding; books at 1e-12.

**Gain.** Change's one-thread work is 52 ms (rows 33, booking 19). In domain tasks on 4 threads at about 2.6 times:
about 20 ms, plus 2 ms for the air nodes. **54 to 22 ms on both benches.**

**Lines.** About +45 and -25 (`Batches` loses its per-class gas batches).

**Risks.** Per-domain tasks are uneven (sea domains react more): the domains sorted by last step's cost, largest first
(the LPT rule of `on_domains`).

### G2. Animal nodes at the field's pace

**What.** Today `Body::interval` (`core/src/body/mod.rs:245`) floors only plant nodes at `node_days`; an animal node
updates by its `INTERVAL` gene, every 8.5 to 9.2 days on the benches. New: **every population node** (a body of more
than one member) updates at least every `node_days` = 30 days. An individual keeps its gene's interval (they are 2 to
4% of animal updates). Every rate of an update follows its pace (`Body::paces`, `body/mod.rs:250`), as intake, takes
and deaths already do (`choose/feed.rs:42,104`, `body/live.rs:227`, `body/physics.rs:257,294`); new: the share of
members that move (`choose/spread.rs:14`) and learning's discount.

**Why it is physics.** A population node is a mean field: what its members do over 30 days is what they would do over
three 10-day steps, if every rate follows the length of the step. Movement is a diffusion: a share f of members that
moves one cell per interval I spreads members at a rate f L² / I. At interval `paces x I` the same spread comes from a
share `min(1, f x paces)`. So the movers of one decision become `count x min(1, p[d] x paces)`. This is the same rule
as `over_span` for deaths. Event scheduling of slow agents: Zeigler 1976; Grimm and Railsback 2005.

**It needs P1b.** The probe shows today's law starves animal nodes at a 30 to 40 day pace (members -50% in 2 years):
upkeep is paid at the start of an interval before its income (C2, S62). P1b pays upkeep and income together across the
interval; only then is G2 honest. **G2 replaces MI6** (calm animals update less often): one rule instead of a doubling
on calm, and its gate. **G2 does not need MI3a:** it carries learning's discount `discount^paces` itself; it needs P1b
and MI5 (both change `spread`). MI3a and MI4 are gated again after G2 (X36). The nodes check (30-day nodes against
individuals at their gene) is a strong test of G2.

**Law (exact new words).**
- `laws.md` 0, intervals: "A population node, of plants or of animals, updates at least every `node_days`; an
  individual by its `INTERVAL` gene. Every rate of an update follows its pace (`paces`, the update's length over its
  gene's interval): intake, takes, deaths, the share of members that move, and learning's discount."
- `laws.md` 4.4, "Pace": "A node updates at least every `node_days`. Its update stands for that whole time, so the pace
  is a speed setting, checked by the pace test."
- `laws.md` 5.4: "the critic's estimate is discounted by `discount` raised to the paces since the last thought".
- handover 7.3, the population nodes line: "Nodes update at least every `node_days` (plants: `field_days`); their
  rates follow the length of their update, so the length does not change the result (the pace test)."
- `numbers.md`, row `node_days`: "speed; days; 30; a population node of animals updates at least this often; anchor:
  the pace test for animals at 10, 30 and 60 days".

**Check.** Level 3. A new slow pace test for animals: eaters with `node_days` 10, 30, 60, three seeds, the eaters'
mass at year 30, members, births less deaths of years 71 to 100 within the seed noise (the same form as
`the_pace_of_nodes_keeps_life`, `tape/tests/ranges.rs:327`). A fast test: a node at paces 3 sends three times the
movers of a node at paces 1 (to its cap), and its members' spread over cells grows at the same rate. The key numbers;
the learning scene (`core/tests/choose.rs:164`; if its body is a node, it runs with `node_days` 10, at its gene's pace); `--full-minds` agrees by the nodes check.

**Gain.** Measured: animal node updates x0.28 (island), x0.31 (globe); with individuals x0.31 and x0.32; an update
about 5% dearer (more members act). Animals after the plan 102 to 33 ms (island), 154 to 52 (globe); the serial phase
loses most landings of moves (measured 80 to 45 and 123 to 60 ms today; after E1 x0.8 and x0.7); parts on the globe
x0.4 (measured 28 to 10.5). **-59 ms (island), -101 ms (globe).** With a floor of 20 days: about -45 and -80. With 60
days (a stretch, if its pace test passes): about -10 and -15 more.

**Lines.** About +30 and -10 (the brainless special case of `interval` becomes one rule; MI6's branch is never
written).

**Risks.** (1) The animals' world moves (predators and prey meet less often; a node eats 30 days of food in one take,
bounded by what the stand holds). The pace test and the key numbers decide; a floor of 20 days is the fallback.
(2) Learning: fewer decisions a year per node. Nodes learn from the whole interval (the discount `discount^paces`,
carried by G2). (3) A
predator in its cell wakes nothing until the node's update: habits' wake rules (MI3) can shorten an interval back to
the gene's when `others` changes, if the pace test asks for it.

### G3. The animal month (the cost side of P1b)

**What.** The plan builds P1b as a law step (one energy law). Its cost gain is not counted. An animal's body day is
15.4 µs of 34.9 (island) and 15.2 of 42.3 (globe): settle upkeep, the reactor's sub-steps on its stores, gas, water,
the heat network, keep warm, growth. P1b's month is a closed form per phase (the stock equation of `plants.md` 2.3),
and keep-warm is already a steady drain in its upkeep (`animals.md` 3.1, X14). G3 keeps only the heat: for a
population node with a brain, its own temperature by the closed form of `physics::relax_heat`, in place of the day
law's heat network.
Digestion of what it ate is booked by its reactions' extents at the next update (`apply_extent`). Target: 4 to 6 µs.

**Law.** That of P1b (X9, X14): "A body pays its upkeep, keep-warm included, and gets its income together over its
interval; the stock equation is solved exactly per phase." G3 adds only the heat's closed form for population nodes.
Individuals and full minds keep the day law's heat (rule 3 for smart bodies: two kinds of body code).

**Check.** Level 3, with P1b. The splitting test of P1 (one update of 60 days against six of 10 within 2%) for an
animal node.

**Gain.** An animal update after the plan 28.8 to 18.4 µs (island, x0.64), 33.1 to 22.9 (globe, x0.69). **-9 and
-12 ms** (after G2; it would be -37 and -48 before G2).

**Lines.** About +100 in `body/month.rs` beyond P1.

**Risks.** The heat in a closed form for animals that live near their limits (cold globe cells): the splitting test
must hold at the poles; if not, keep the day law's heat step with P1b's balance (half the gain).

### G4. One batch per 10 days

**What.** `batch_days` 2 to 10 (`core/src/params.rs:428`), so a batch is one air step. Bodies due in a 10-day window
update at its first day, as they do now in a 2-day window; what leaves a domain lands at the window's end. Intervals
round to whole batches (10 days); population nodes are at 30 (G2) or 60 days (stands) anyway.

**Rule (exact words).** S64 is replaced: "`batch_days` is 10, one air step: a batch holds 30 to 60 ms of work on the
benches against 30 to 55 µs for a pool call. Intervals are whole batches. Individuals with a shorter gene interval than
a batch update once a batch; smart bodies run sub-batches inside a batch (plan 2.2, rule 2: the step per place)."
`numbers.md`, row `batch_days`: 10. `Config::validate` already asks that `batch_days` divides `phys_days` and the
month, and refuses a `node_days` that is not a multiple of `batch_days` (`params.rs:392`): so from P1.2 on
`field_days` is 30, 60 or 90 (G5 and P3 also need a multiple of `phys_days`), and `node_days` 30 or 20.

**Honest.** Only the order and timing of updates inside 10 days change; books as now. Perception reads faces up to 10
days old (today up to 2).

**Check.** Level 3. The equivalence test of batches (today 1 against 2) becomes 2 against 10. Key numbers.

**Gain.** Measured: faces x0.22, the serial phase x0.58, the parallel phase's wall per update x0.77 (island) and
x0.83 (globe) (bigger batches balance better). After E1 and E2 I count x0.85 on the parallel parts and x0.75 on the
serial phase: **-26 and -39 ms.** (The measure also cut updates by rounding up; that is not counted here, G2 does it.)

**Lines.** About +5.

**Risks.** Fast individuals lose resolution: intervals round to whole batches, so gene intervals from 5 to 15 days all
become 10 days (a small law effect; 2 to 4% of animal updates); smart bodies later get sub-batches. Moves land up to
10 days late: crowding and merges a little later.

### G5. A physics step of 30 days

**What.** `phys_days` 10 to 30 (`params.rs:422`): the survey, Flow's state and Change run once a month. The air step
stays 10 days (`air_hours` 240): its climate needs it (X10). Change's reactor already integrates a node over any span
(its sleep does up to 90 days, `change/mod.rs:22-60`).

**Law (exact words).** `laws.md` 0: "The survey, Flow's state and Change run every `phys_days` (30 days); the air every
`air_hours` (10 days); bodies in batches of `batch_days` (10 days)." `numbers.md`, row `phys_days`: "speed; days; 30;
anchor: the long-step test (air 6 hours, physics 2 days) within its limits, and the sleep test". Handover 7.3, the step
loop line, the same.

**Honest.** Each law integrates its whole span; books as now. Bodies read the shade and the cell sums of the month's
start (stands change once a season anyway).

**Check.** Level 3. The long-step test (`tape/tests/ranges.rs`) and the globe climate test with `phys_days` 30 against
the short steps; the sleep test (S84's known bias on `air_3` must not grow beyond its seeds' noise); key numbers.

**Gain.** Measured: Change x0.38 (island) and x0.41 (globe); survey x0.63 and x0.75; Flow's state process x0.4. After
G1 and G6: **-23 ms on both.**

**Lines.** About +10 (validation; any place that reads "10 days").

**Risks.** S84: a reaction pair that undoes itself over a long span is the sleep's known bias; 75% of Change's rests
are under 10 days (1.2), so a 30-day span for every node will likely grow it. **Expect the fallback:** Change keeps 10
days (a `change_days`), and the survey and state take 30 (about half the gain). The plan's "likely" column counts only
that half.

### G6. The whole survey on the pool

**What.** `survey::survey` (`survey.rs:28`) runs wear and merge per domain, but the cells with parts one by one
(`in_domain` and `deliver` each), then `sum_cells`, `traffic`, a full `flow::state` once a month (which the `state`
process right after it does again, `schedule.rs:52`), the cell caches and `choose::cells`, mostly on one core
(the globe's survey is as slow on 4 threads as on 1: 58.9 and 58.5 ms). New: one per-domain pass that does wear,
merge, shade, the stands' field step (P3) and the cell sums of its cells; cells with parts go through the engine's S2
or E5; the monthly full state is done once (by the `state` process).

**Rule.** None. **Check.** Level 2 (sum order of the cell caches). **Gain.** x0.55: **-14 and -20 ms.** **Lines.**
+40 and -15. **Risks.** Cells with parts on the globe: their wear stays serial until E5; measured share small.

### G7. Bond's monthly turns per domain

**What.** `bond::step` (`bond/month.rs:12`) gives each part a serial turn (`in_domain`, then `book_moves` over every
domain, line 39): 2,421 turns a year on the globe, about 10 µs each. New: the parts grouped by the domain of their
cell, each domain's parts in slot order on the pool (each part's moves go to its domain's view), booked by domain;
structures that span domains stay serial (as E5 plans for the batch).

**Rule.** None. **Check.** Level 2. **Gain.** parts x0.45: **-4 and -15 ms** (after G2 cuts the part acts). **Lines.**
+40 and -10. It can replace E5's +150 if the batch's serial bodies are small after G2 (E5's gate measures it).

### G8. The tape's recorder on the pool

**What.** `Recorder::record_month` (`tape/src/rec/mod.rs:160`) counts events, detects, and walks the world for the
monthly row, and once a year for the series (`push_series`). New: the walks over cells and bodies in chunks on the
pool, summed in chunk order; the monthly row reads the survey's per-cell sums (`Life::sum`) where it can instead of the
bodies. The earlier probe "recorder on the pool" (branch `speed/e23-tape-not-kept`) gave 0.7% and 0.3% of a year: its
kill line was 1% of a year; against a year of 91 ms, 8 ms is 9%.

**Rule.** None (tape crate; not core lines). **Check.** Level 1 for the world (golden unchanged: the tape does not
write the world); level 2 for the tape (`series.csv` and `months.csv` the same to rounding), or level 1 there too if
each sum keeps its order.
**Gain.** 17 to 9 ms. **Risks.** Small.

### G9. Stands at a season's pace, and compact stands

**What.** `field_days` 60 to 90 or 120 if the stands' pace test passes (the month law is exact over its interval,
`plants.md` 2.3); and P4 (compact stands: a `Stand` without brain fields, stored per cell) built now, not gated,
because after G2 and G3 the 0.7 KB `Body` values and their clones are a large share of a stand's 6 µs (the review's
item e).

**Law (exact words).** `numbers.md`, row `field_days`: "speed; days; 90; a stand updates every this many days;
anchor: the pace test of the field at 10, 45, 90 and 180 days".

**Check.** Level 3 for the pace; level 1 for compact stands (same laws).

**Gain.** Stands x0.5: **-10 and -6 ms.** **Lines.** +150 and -50 (P4). **Risks.** Births and maturing wait for the
stand's update (S62's reason at 140 days): the pace test decides; 60 days is the fallback.

### G10. AIR4 counted (calm months by TR-BDF2)

**What.** As in `air.md` 2.4. I count it in the budget because the goal needs it on the island unless G14 is very good.
Its research step (AIR4-R1: why 30-day steps warm the winters) comes first.

**Gain.** Air step x0.55 and the draws x0.7: **-32 and -38 ms.** Kill line as in `air.md`.

### G11. f32 and SIMD for the air's cell rows (replaces AIR6)

**What.** `pass` (`flow/air/step.rs:197`) works cell by cell with branches (the surface balance with up to 8 Newton
steps, the end reflectance with up to 8 guesses). New: cells in blocks of 8, structure of arrays in f32 for the rates
(sunlight, emission, exchange coefficients), a fixed number of Newton steps per block with a mask, `exp` by a short
polynomial. The rows (what moves) are widened to f64 once and booked on both sides (X4: 32 bits hold nothing).

**Rule.** X4 already allows it. **Check.** Level 2: the climate tests; golden re-pinned. **Gain.** `pass` x0.6 after
AIR1 and AIR4: **-6 and -9 ms** (without AIR4: -11 and -16). **Lines.** +120 and -40. **Risks.** A fixed Newton count
that is too small biases the surface temperature: the block falls back to the scalar path when its residue is not
under the tolerance. It replaces AIR6 (calm sea shares one row), which works on the same `pass`.

### G12. The liquid every 60 days

**What.** The liquid runs from the start of the world every `flow_days` = 60 (`clock::Every` with `within` = its own
period, as creep does, `schedule.rs:55`), and `Config::validate` refuses a `flow_days` over a month unless it is a whole
number of months. The drainage shares are exact over the step (`1 - exp(-seep x slope x time / ...)`, `liquid.rs`), and
the downhill pass routes all water that may run off.

**Law (exact words).** `numbers.md`, row `flow_days`: "speed; days; 60; anchor: the rivers and land rain of the
long-step test and of the globe test within their limits".

**Check.** Level 3: rivers, rain on land, land worn (key numbers). **Gain.** The liquid x0.6 (it is not halved: a
step with more water does more work): **-13 and -14 ms.** Not measured (the probe's run was monthly by the clock bug
above): a guess until R3's clock fix lands; the plan's "likely" column counts half. **Lines.** +15. **Risks.** Puddles stand longer between steps, so more evaporates: the key numbers decide; 30 is
the fallback.

### G13. One heat draw per cell for stands

**What.** As the air design asks (`air.md` 10): stands give one `Draw` per cell, so `Draws::settle` and `owe`
(`survey/draws.rs:72,114`) and the light marks after each air step (`flow/air/book.rs:62`, a loop over all bodies)
loop over cells for stands. **Check.** Level 2. **Gain.** -3 ms. **Lines.** +30 and -20. Needs P3.

### G14. Physics beside the bodies

**What.** This is D1 (b) again (the review's item a), in a simpler form that the new rules allow. Today, at each time,
the processes run one after the other (`schedule.rs:45-61`): survey, Flow's state, air, liquid, Change, Bond, batch.
New, for each 10-day window (one batch, G4):

1. **Before (serial, short):** what is left of the survey when due (G5: once a month); `Draws::settle` (per cell
   after G13). The survey's per-domain part (G6) runs at the head of the bodies' lane, not here.
2. **Two lanes at once** (`rayon::join` on one pool, so each lane uses idle cores):
   - **physics lane:** the air step of the window, and at the month's lines the state, the liquid and Change. It
     *reads* the world as it was at the window's start and *writes* only its own `Scratch` (a second one) and its
     rows: what it would move. No store number is written.
   - **bodies lane:** the batch, as the engine's pipeline runs it (P1, S1, P2, S2). It reads the store at the window's
     start through its views, the faces, and Flow's state of the last window (the old `Scratch`).
3. **After (join):** the bodies' views are booked (E1c); then the physics rows are booked, **each node's outgoing
   moves cut to what the node holds then** (one shared helper, the generalisation of the air's `hold_air`,
   `flow/air/book.rs`); then `Draws::owe` and the light marks; then the scratches swap. **The cut is whole per
   reaction:** for each node, each substance's share is what it holds over what all rows ask of it (at most 1); a
   reaction's share is the least of its inputs' shares, and it is applied to all its inputs, outputs and heat; a move
   of matter goes with its heat (as `hold_air`). Never substance by substance: that would break a reaction's
   proportions, so the books of elements and energy. G1's rows (made in the domain tasks, booked after the call) are
   these rows: G14 books them after the bodies and does not redo G1.

Bond's month and the tape stay at the month's line; the tape reads a frozen copy beside the next window (G8), so it
is not in the serial part.

**Law (exact words).**
- `CLAUDE.md` 3.4, a new sentence: "When the physics runs beside the bodies, it works out its moves from the store at
  the window's start and books them after the bodies; each node's outgoing moves are cut to what the node holds then,
  one share for each reaction or move as a whole, so no order of the two lanes can overdraw it."
- handover 7.3, the step loop: "In each window of `batch_days`, the air (and at a month's line Flow's other parts and
  Change) and the bodies' batch run at once, both from the world at the window's start; the bodies' moves are booked
  first, then the physics, cut to what each node holds. The bodies read the physics of the window before."
- `laws.md` 0: the order of processes at one time says the same.

**Honest.** Books: every move still goes both sides through `store::apply`; the cut keeps both sides (a move cut by a
share takes its heat by the same share, as `hold_air` does). No overdraw: the bodies' views never take more than the
view holds (as today); the physics never gives more than the node holds after the bodies. The physics error is an
operator-splitting lag of one window (10 days), the same kind as today's order (bodies see the air after its step,
physics sees the bodies before theirs); its size is checked by the key numbers.

**What S81 found, and why it is smaller now.** S81's design reserved a share for each body by its last takes, kept two
reserve nodes, and cloned the world at each air step (170 to 280 lines, 110 free). Here nothing is reserved: the cut
at booking is the only guard, and it already exists for the air. What is needed: Flow and Change split into "work out"
(`&World`, own `Scratch`, rows) and "book" (`&mut World`); a second `Scratch` for the bodies to read; the cut helper
for Change and the liquid (Change already counts "only what a node holds at the end of the call", `change/mod.rs:78`).
AIR2 and AIR3 do most of the split for the air anyway.

**Check.** Level 3. The order test gets a case: the two lanes in both orders give the same world. Books at 1e-12 each
month in golden. A debug check: after the cut, no physics row books on a node more than it holds. A counter (tape only)
of the moves cut, and how much: if cuts are more than 1e-3 of the moved amount on the benches, the lag is too big.
Climate tests; key numbers. The debug check also holds that every reaction is booked with one share on all its
substances and its heat (its proportions kept).

**Gain.** The first bound of section 2.2 (island 75 to 88 ms instead of 130; globe 95 to 112 instead of 166) left out
the serial parts outside the two lanes (the survey's "before", about 10 / 18 ms; Bond's month and the tape at month
lines, about 11 / 14 ms; the draws) and the lumpy windows (every third window carries Change, the state, the liquid
and AIR4's 30-day step). Counted: **island 79 to 98 ms (9.8 to 12.2 times), globe 101 to 126 ms (13.1 to 16.3
times)**. G6 in the bodies' lane and the tape's frozen copy are what bring it near the good end. G14-P1 and G14-P2
time month windows and other windows apart and report the serial part.

**Lines.** About +280 and -30.

**Risks.** (1) The physics sees the bodies 10 days late: large in crowded cells that drink a dry cell; the cut keeps
the books, the counter shows the size. (2) The two lanes fight for memory: the imbalance of 1.1 to 1.3 is a guess; a
probe before the full build (section 9, G14-P). (3) Determinism: each lane is fixed by its data; the join's order is
fixed; the order test checks it.

### G15. The sharing between threads inside one process

**What.** One world's update costs 31 to 36% more on 4 threads, but four separate worlds only 18% more: about 13 points
are ours. Candidates, each measured before any change: (a) cache lines shared by neighbouring domains (the `Domain`
headers, counters and view vectors side by side; pad hot fields to 64 bytes); (b) the allocator (bodies allocated by
one thread and freed by another after a move: glibc frees across arenas with a lock; a per-thread arena for
`Vec<f64>` of bodies, or `Body::store` in a fixed array); (c) `par::each` uses `par_bridge` (`core/src/par.rs:38`),
which takes a lock for every item; (d) the 25 to 30% slower serial code while the pool's threads spin (try a pool that
parks idle threads at once between phases). **Rule.** None. **Check.** Level 1. **Gain.** the parallel parts x0.9:
**-13 and -17 ms** (range 0 to -25). **Lines.** about +10. **Risks.** None for the world; it may find nothing.

### G16. Well-being once per decision; GM, the cost of a thought

- **G16** (the review's item d; with G2, the alternative to MI3a and MI4 if their gates fail after G2, X36): `spread` measures well-being before and after each part's act (`choose/spread.rs`)
  for the learning advantage; measure it once before the decision and once after all parts, and share the change by
  members. 126 and 168 million instructions a year today (1.1% and 2.1%); after MI3 and G2 about 1 to 2 ms. Level 3
  (the advantage of each part is now a mean). Sonnet, small.
- **GM** (the review's item f): a measure, no code on the branch: the cost of a thought at 12 and 64 hidden units with
  `--full-minds` (instructions per decision, think, learn, trace), for the smart bodies later. Haiku.

---

## 4. Ideas checked and dropped

| Idea | Why not |
|---|---|
| Climate "on repeat" (replay last year's fluxes, wake on change) | The fluxes would not answer this year's ground water, ice or plants: a cache of past results, not physics. Books would hold (cuts), the world would not. Most cells would wake anyway: the interannual spread of a 10-day mean temperature is far above the air's `STILL_K` (0.05 K) |
| Last year's air as Newton's first guess | The air design tried the step before as a guess: 2.4 rounds instead of 2.1 (`air.md` 1.5). The year before is further from the answer than the step before. At most a probe |
| Change asleep by the net change, not by the use of inputs | 75% of nodes are due by their own alarm because their reactions use 5% of an input in under 10 days. A net rule would sleep reaction pairs that undo themselves: exactly S84's open bias, whose fix costs 20%. G1 and G5 get more, with no new law |
| A cap of bodies per cell, or a crowding rule (S66) | On the benches the most bodies in a cell are 24 to 35, and animal nodes are few per cell (222 over 484 cells). After P1 to P3 and G9 a crowded cell of 623 stands costs about 623 x 4 updates x 3 µs = 7.5 ms a year. Not a speed tool any more; it stays the owner's world choice for long runs |
| Fewer substances or reactions, merged layers | Change's rows cost 2.3 µs a node: cheap. The cost is in the booking and the clock (G1, G5). Fewer substances would change the world's chemistry, the heart of the project |
| A stage-by-stage update for the instruction cache (12% of an update's cycles) | E3's gate found 3% repeated work; stage order changes the order of moves (level 3); and after G2, G3 and G9 the update code is far smaller (closed forms replace the reactor's sub-steps), which itself cuts the misses |
| A linear-time air solver (multigrid or Krylov; the review's item c) | No gain on the benches (the band solve is 6 to 8% of the air step). It stays AIR5, gated on worlds of 2,000 cells or more |
| The 16-core model as reaching the goal | Not used (the review's item 1). On this machine 4 threads give at most about 3 times |
| Batched networks, 32-bit store, order-free threads, grid growth table, coarser air grid | Already dropped by the plan (X3, X4, X8, X10, X12) |

## 5. Rule changes, all in one place

1. **`laws.md` 0, intervals and the clock:** "A population node, of plants or of animals, updates at least every
   `node_days` (stands every `field_days`); an individual by its `INTERVAL` gene. Every rate of an update follows its
   pace: intake, takes, deaths, the share of members that move, and learning's discount. The survey, Flow's state and
   Change run every `phys_days` (30 days); the air every `air_hours` (10 days); the liquid every `flow_days` (60 days,
   counted from the world's start); bodies in batches of `batch_days` (10 days). In each batch window the air (and at
   a month's line the other physics) runs beside the bodies, from the world at the window's start, and is booked after
   them." (G2, G4, G5, G12, G14)
2. **`CLAUDE.md` 3.4, a new sentence:** as in G14.
3. **`CLAUDE.md` 2.11, the ceiling:** "`core` holds at most 12,000 lines of code (`docs/speed-x10/decisions.md`)". Or
   11,500 if E5 and AIR6 give way to G7 and G11 (section 7).
4. **Handover 7.3:** the population nodes line and the step loop line (G2, G14). **Handover 6.5:** none beyond MI3.
5. **Plan 2.2, rule 2 (the step per place):** "A batch is 10 days. Individuals and smart bodies may run sub-batches
   inside it; population nodes update at least every `node_days`."
6. **`numbers.md`:** rows `node_days` (30, animals and the floor of all nodes), `field_days` (90), `batch_days` (10),
   `phys_days` (30), `flow_days` (60), each a speed setting with its pace or climate test as anchor.
7. **Decisions to log:** S64 replaced (G4); MI6 replaced by G2; AIR6 replaced by G11; E5 after G7's measure; the goal
   needs G2 and G14 (this section).

## 6. How we check: "the same world" now

- **Level 1** (G15, compact stands in G9; G8 for the world, level 2 for the tape unless each sum keeps its order):
  bit for bit.
- **Level 2** (G1, G6, G7, G11, G13): golden re-pinned, the series to rounding where life does not amplify it.
- **Level 3** (G2, G3, G4, G5, G9 pace, G12, G14, G16): the key numbers (`tape/tests/ranges.rs`) in their ranges, the
  nodes check, and the pace tests; plus:
  - a pace test for animals (G2), at `node_days` 10, 30 and 60;
  - the pace test of the field at 10, 45, 90, 180 days (G9);
  - the long-step test and the globe climate test with `phys_days` 30 and `flow_days` 60 (G5, G12), and the sleep test
    (S84 must not grow past its noise);
  - the equivalence test of batches at 2 against 10 (G4);
  - the order test with the two lanes in both orders, and the cut counter under 1e-3 (G14).
- Each level-3 step re-pins golden in its commit and runs the key-number test once (X5).
- **What must hold** (on top of X5): animal members and births less deaths within noise of the plan's world; land
  temperature and swing; rain and rivers; the light-eaters; books at 1e-12 every month.

## 7. Line cost (core code lines)

| Step | Added | Removed | Net |
|---|---|---|---|
| G1 | 45 | 25 | +20 |
| G2 | 30 | 10 | +20 (and MI6's +25 not written) |
| G3 | 100 | 0 | +100 |
| G4, G5, G12 | 30 | 0 | +30 |
| G6 | 40 | 15 | +25 |
| G7 | 40 | 10 | +30 (E5's +150 may not be needed) |
| G9 (P4 built) | 150 | 50 | +100 |
| G11 | 120 | 40 | +80 (AIR6's +70 not written) |
| G13 | 30 | 20 | +10 |
| G14 | 280 | 30 | +250 |
| G15, G16 | 15 | 10 | +5 |
| G8 | (tape crate) | | 0 |
| **total** | | | **about +670** |

The four designs need about +1,250 to +1,600 with all their steps. With G2, G7 and G11 replacing MI6, E5 and AIR6
(about -245), the round needs about 9,591 + 1,400 + 670 - 245 = **about 11,400 to 11,700**. So: keep 11,500 if E5 and
AIR6 are not built; else raise to 12,000. Each step names what it deletes.

## 8. Risks and answers

| Risk | Answer |
|---|---|
| G2 moves the animals' world (the probe saw members halve under today's law) | G2 waits for P1b; the pace test for animals; 20 days is the fallback (about 75% of G2's gain) |
| G14 is big and its gain depends on how two lanes share 4 cores | A probe first (G14-P: the two lanes timed together with fake booking on a branch), with a kill line; the cut counter shows the lag's size |
| AIR4 fails its climate test | The island then needs G14 at its good end (8.9 to 10.6 times); the globe still passes with G14 |
| G5 grows S84's bias | Change keeps 10 days; the survey and state take 30 |
| G12 dries or floods rivers | 30 days stays; the gain is lost (-13 ms) |
| The steps that change no law are small one by one | Together they are -100 ms (island) and -135 ms (globe) on top of the plan: a third of the road; their kill lines are per step |
| This machine's 4 vCPUs (31 to 36% dearer updates on 4 threads) | G15 takes our half; the other half is the hardware; a machine with 4 real cores and its own caches would give more, the plan does not count on it |
| The sum stays under 10 times | Said plainly in section 0: the likely outcome without G14 or AIR4 is 6 to 7.5 times (island) and 8 to 10 times (globe). Each step is judged on its own kill line |

## 9. Steps

"4T" = wall time on 4 threads (5 or more interleaved runs, medians); "instr." = callgrind on one thread (S45). Gains
are of a year on the bench.

| ID | What | Level | Done when | Kill line | Needs | Parallel with | Agent |
|---|---|---|---|---|---|---|---|
| G0 | The probe of section 1 on the remade benches (R2): the update split, Change's sleep, the clock settings, the four-worlds test; rows in `measures.md` | none | the rows are written | none | R2 | anything | Haiku |
| G1 | Change per domain (rows in the domain tasks, booked after the call by `book_by_array`) | 2 | golden re-pinned; Change on 4T -50% | under -30% of Change on 4T: revert | E1c's booking helper | G6, G7, G8 | Sonnet |
| G7 | Bond's turns per domain | 2 | parts on 4T -40% on globe36 | under -25%: revert | E1 | G1, G6, G8 | Sonnet |
| G8 | The recorder on the pool | 1 for the world, 2 for the tape (1 if each sum keeps its order) | golden unchanged; tape files as the level says; tape on 4T -40% | under -25%: revert | none | all | Sonnet |
| G6 | The survey per domain | 2 | survey on 4T -35% on both | under -20%: revert | P3 (the field step lives there), E1 | G1, G7 | Sonnet |
| G15 | Find and remove the sharing between threads: (a) to (d) of G15, each measured | 1 | an update on 4T at most 1.2 times its 1T cost | nothing found above 3% of the bodies' stage: stop | E1, E2 | all | Sonnet (measures), Opus (a fix) |
| G13 | One draw per cell for stands | 2 | `Draws` and the marks loop over cells for stands | under 2 ms: revert | P3 | G6 | Sonnet |
| G4 | `batch_days` 10 (`field_days` 30, 60 or 90; gene intervals 5 to 15 days become 10) | 3 | equivalence test 2 vs 10; key numbers; bodies' stage on 4T -10% | under -6%: back to 2 | E1 (pipeline), G2 not needed | G5 | Sonnet, Opus review |
| G5 | `phys_days` 30 (fallback expected: Change at 10) | 3 | long-step and globe tests; sleep test within noise; key numbers; year -4% on 4T | a climate test fails or under -3%: back to 10 (or Change at 10, the rest at 30) | G4 (`batch_days` divides it), G1 | G4 | Opus |
| **G2** | **Nodes at least every `node_days` 30; movers and discount by paces; MI6 dropped** | 3 | pace test for animals (10, 30, 60) passes; key numbers; nodes check (a strong test of G2); bodies' stage on 4T -25% on both | pace test fails at 30: try 20; fails at 20, or under -15%: revert | **P1b**, MI5 (not MI3a) | G9 | Opus, adversarial review |
| G3 | The animal month's heat (the closed form of `relax_heat` for population nodes with a brain; keep-warm is already in P1b's balance) | 3 | splitting test for an animal node; animal update -30% (instr.) | under -15%: keep P1b's law only | P1b, G2 | G2 | Opus |
| G9 | `field_days` 90 (pace test) and compact stands (P4) | 3, 1 | pace test of the field; stands -40% (instr.) | pace fails at 90: 60; compact under -10% of stands: not kept | P3 | G2 | Opus |
| G12 | The liquid every 60 days from the start; validate (gain a guess until R3's fix) | 3 | rivers, land rain, worn land in range; liquid -35% | a key number leaves its range: 30 | AIR2, R3 | G9 | Sonnet |
| G10 | = AIR4 (with AIR4-R1 first) | 3 | as `air.md` | as `air.md` | AIR1 to AIR3 | G11 | Opus |
| G11 | f32 and SIMD cell rows (instead of AIR6) | 2 | climate tests; `pass` -35% | under -20%: revert | AIR1, AIR2 | G10 | Opus |
| G14-P | Probe on a branch: the two lanes timed together (physics rows not booked, bodies as now); month windows and other windows apart; the serial part reported. Twice: G14-P1 on today's code, G14-P2 after G4, G5, AIR2, AIR3 | none | 4T year of the two lanes against one after the other; the serial part | the overlap saves under 10% of the year (X38): G14 not built | G4, G5, AIR2 (G14-P2) | anything | Sonnet |
| **G14** | **Physics beside the bodies (work out, then book after the bodies with the cut, whole per reaction)** | 3 | order test with lanes reversed; books; debug check (no overdraw, each reaction's proportions kept); cut counter under 1e-3; climate tests; key numbers; year -25% on 4T | under -10% on either bench (X38), or the cut counter over 1e-3: revert | G14-P, E1, AIR2, AIR3, G1 | none | Opus, adversarial review |
| G16 | Well-being once per decision | 3 | learning scene; nodes check | under 1% of the bodies' stage: revert | G2, MI5 | any | Sonnet |
| GM | The cost of a thought at 12 and 64 hidden units, `--full-minds` (if MI3a is built; else today's year) | none | a row in `measures.md` | none | G2 (MI3a if built) | any | Haiku |

**Order.** G0 with R2. Then the safe lane (G1, G7, G8, G6, G13, G15) beside the plan's lanes: it touches `change/`,
`bond/`, `tape`, `survey.rs` (after P3), `par.rs`. G4 then G5 after E1. G2 and G3 right after P1b (they replace MI6 in
phase 2). G9 after P3. G12 after AIR2. G10 and G11 in the air lane's phase 2. G14-P after G4 and G5; G14 last, after
AIR2 and AIR3 have split the air into "work out" and "book".

## 10. Interactions with the other designs

- **Plants:** P1b is now on the critical path (G2 needs its law, G3 its closed form for animals). P4 is built (G9),
  not gated. `field_days` may go to 90. P3's field step lives in the per-domain survey (G6).
- **Engine:** G1, G6 and G7 use the booking-by-array helper of E1c and the per-domain runner; G7 may make E5
  unnecessary (E5's gate measures the serial bodies after G2). G4 makes S64's 2-day batch 10 days; the engine's
  `SERIAL_BELOW` matters less. G14 runs the engine's pipeline as one lane of two. G15 is the engine's open question
  ("a domain's work on 4T +21 to 28%").
- **Air:** AIR2 and AIR3 must split the air step into "work out" (`&World`) and "book" (`&mut World`): G14 needs it.
  G11 replaces AIR6. G12 changes the liquid's clock. G5 changes `phys_days` but not `air_hours`. AIR4 is counted in the
  budget.
- **Minds:** G2 replaces MI6, and comes before MI3a and MI4, which are gated again after it (X36). The habits' wake
  rules (if MI3a is built) may shorten a node's interval back to its gene's when danger comes (G2 risk 3). G16
  touches `spread` after MI5 and G2.
- **Plan:** section 1's "10 times on 4 cores is not in sight" becomes: "10 times on 4 cores is in sight only with the
  gap's G2 and G14 (and AIR4 for margin on the island)". The X decisions get: G2 replaces MI6; S64 replaced; AIR6 and E5
  give way to G11 and G7; the ceiling (X2) stays 11,500 under that condition.

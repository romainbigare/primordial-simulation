# Air: fewer air steps where the air is calm, and a coarser air grid (owner's choice 6)

> **Note: what the plan kept** (`plan.md` is the single source of the steps; this design keeps its old numbering).
> AIR0: in R2. AIR-T: in R3 (with the `flow_days` fix). AIR1: kept, **level 3** with the long-step and globe
> climate tests (X34). AIR3: kept, written through the engine's `book_by_array` (E1c); E4 does not touch the air.
> AIR2: kept, **level 2**, after E1c, on its domain runner, domains cut by air node, partials summed in node order:
> a fixed order, not a free one (X3). AIR4-R1: a probe of phase P. AIR4: kept and counted (= G10); a calm-month
> variant is built and measured whatever R1 finds (X32). AIR5: gated on worlds of 2,000+ cells, not in this round's
> benches. **AIR6: dropped**, replaced by G11; it is G11's alternative (X27). The liquid moves to 60 days in G12.
> Code names say "open liquid", never "sea" (X35).

Design section for the speed x10 plan. Written for the owner: short sentences, common words.
All numbers below were measured for this section at `f63c06a` (the bench worlds made again with this code: island
22 x 22 at year 300, globe 36 x 18 at year 200), with a timing probe and knobs on a copy of the code outside the repo.
"1 thread" means `RAYON_NUM_THREADS=1`. The machine drifts by about 10% between hours, so read small differences
with care.

## 0. In short

- **The air grid is already coarse.** One air node covers a block of 4 x 4 ground cells (`AIR_SCALE = 4`,
  `worldgen/src/terrain.rs:21`). The island has 36 air nodes, the globe 45. The work done per air node (the implicit
  solve) is only **6 to 8% of the air step**. Almost all the cost is **per ground cell**. So a coarser air grid saves
  **at most about 5% of Flow** on our benches, and it costs climate and threads (the domains of the bodies are the
  cells under one air node). **I do not propose it for the benches.** For much larger worlds it matters (section 6.5).
- **The air already takes long steps.** One air step is 10 days (`air_hours` 240): 36 steps a year. I tried longer
  steps with today's method (backward Euler): 15 days, 30 days, and 30 days only in "calm" months. **All break the
  climate test** (the land warms by +0.30 to +0.67 K against short steps; the limit is 0.3 K; winters warm by up to
  +1.2 K). A step of first order cannot be longer than about 10 days. This agrees with foundation decisions 6 and 22.
- **What "calm" can honestly mean here** (my proposal):
  1. **Calm cells inside a step:** a ground cell whose air did not move since its last round keeps its row (no new
     surface balance). Measured on a 64 x 32 globe: 36% fewer cell rows, the climate moves by 0.03 K.
  2. **A calm solver:** the stop test of the rounds asks too much of the gas (1e-5 of the air). At 1e-4 the globe needs
     3.5 rounds instead of 4.4; the climate moves by 0.01 K.
  3. **Calm months, the honest way:** only with a step of second order (TR-BDF2) and its own error estimate, which
     chooses 10 or 30 days each month. This is research with a strict kill line.
  4. **Calm sea:** alike open-sea cells under one air node share one surface balance, each corrected by its slopes.
     Research, gated by a measure.
- **On 4 threads about 70 to 76% of Flow's time runs on one core** (booking, gathering, dissolving, the bodies' draws,
  the state, the liquid). The owner's choice 4 (thread order free) lets these loops run per domain on all cores.
  This is the largest sure gain for Flow.
- **Expected gain for Flow (4 threads):** 1.5 to 1.65 times from the sure steps (AIR1 to AIR3), up to 2.5 to 2.9
  times if the research steps pass (AIR4, AIR6). Flow alone cannot give the year 10 times: even with Flow at the best
  case, Flow plus Change is still more than the whole year of the aim on the island (section 7).

## 1. What exists today

### 1.1 The grid

- The mesh has two levels. Level 0: the ground cells. Level 1 (`AIR = 1`, `core/src/world.rs:109`): the air nodes.
  `World::mesh_of` (`world.rs:113`) builds it from the map with `Square::mesh(&[air_scale], ...)`
  (`core/src/space/square.rs:58`). Each air node is a block of `air_scale`² = 16 squares (`square.rs:67`), fewer at
  the south and east edges. On a globe the blocks of a row are numbered in a folded order, so the banded solve stays
  one row wide.
- `air_scale` is saved in `World` (`world.rs:63`) and is 4 for every world (`worldgen/src/terrain.rs:21`).
- Island 22 x 22: 6 x 6 = 36 air nodes. Globe 36 x 18: 9 x 5 = 45 air nodes (its last row of blocks has 2 rows of
  cells). Refined region 24 x 24: 36 air nodes, one holds 64 cells.
- **The domains of the bodies are the cells under one air node** (`Life::new`, `core/src/life/mod.rs:206`; handover
  7.4). A body's view of the store holds its domain's air node (`store/gather.rs:115`).
- A ground cell sees four air nodes, bilinearly (`air::lerp`, `flow/air/mod.rs:36`; tables in `flow/scratch.rs`,
  `near` and `wt`). Its rain comes out of these four nodes by their shares (`air::shares`, `mod.rs:44`).

### 1.2 The clock

- `schedule::processes` (`core/src/schedule.rs:45`): Flow's `state` and Change and the survey every `phys_days`
  (10); `air` every `air_hours` (240 h = 10 days, `schedule.rs:53`); `liquid` every `flow_days` (30); creep once a
  year; refresh every 3 months.
- `Config::validate` (`core/src/params.rs:384`): the air's step must divide the physics step, because the air step is
  booked whole when it is solved.

### 1.3 One air step (`flow::air`, `core/src/flow/mod.rs:46`)

1. `survey::Draws::settle` (`core/src/survey/draws.rs:72`): per cell, the heat the bodies may still draw; the rest
   of the bodies' node heat goes back to the ground. A loop over all bodies, on one core.
2. `ground_now` and `air::state` (`flow/air/mod.rs:70`): the state of the air nodes (`nodes`, `heights`) and the air
   each cell sees (`seen`).
3. `air::step` (`flow/air/step.rs:130`):
   - `dissolve` (`flow/air/exchange.rs:87`): gases that cannot condense go into the ground's liquid (Henry), booked.
   - setup: `active`, `Start::of`, `Drive::of` (the wind's pushes from the step's start), `fixed`, `room`.
   - **rounds** (Newton), at most 16 (`ROUNDS`, `step.rs:21`):
     - `pass` (`step.rs:197`): for **every ground cell**, its row (`cell`, `step.rs:214`): sunlight past clouds,
       rain (`rain`), the surface balance solved for the end temperature (`exchange`, `step.rs:280`, with up to 8
       Newton steps) under the reflectance at the step's end (`end_reflectance`, `step.rs:344`, up to 8 guesses).
       On the pool (`par::rows`).
     - `gather` (`newton.rs:39`): the rows summed per air node, with their slopes. One core.
     - `solve` (`newton.rs:129`): one implicit step of the air level, a banded LU (`solve/band.rs`). One core.
     - stop when no air node's temperature moved by more than 0.05 K (`STILL_K`, `step.rs:17`) and no gas by more
       than 1e-5 of the node's gas (`STILL_SHARE`, `step.rs:19`).
   - `alone`, then `book` (`flow/air/book.rs:17`): the last round's rows and wind are booked in one `store::apply`,
     cut where an air node would give more than it holds (`hold_air`, `book.rs:110`). So **the books are exact
     whether or not the rounds converged.** Then `Draws::owe` (`draws.rs:114`).

### 1.4 Where the air's time goes (measured)

One settled year, 1 thread, ms (island22 / globe36). Flow in these runs was 174 / 261 ms (Z1 measured 149 / 229 in
another hour; the shares are what matters).

| Part | island22 | globe36 | runs on | grows with |
|---|---|---|---|---|
| `pass` (cell rows) | 62 (36%) | 112 (43%) | pool | cells x rounds x steps |
| `book` (mostly `store::apply`) | 22 (13%) | 26 (10%) | one core | cells x gases x steps |
| `gather` | 7.9 | 14.6 | one core | cells x rounds x steps |
| `solve` + `alone` (the air level) | 7.1 (4%) | 16.1 (6%) | one core | air nodes x band² x rounds |
| `dissolve` | 8.1 | 10.8 | one core | cells x steps |
| setup (`Start`, `Drive`, `fixed`, `room`) | 4.3 | 5.2 | one core | cells x steps |
| `Draws::settle` + `owe` | 8.5 | 6.1 | one core | bodies x steps |
| `air::state` + `ground_now` | 6.3 | 8.3 | one core | cells x steps |
| liquid (12 steps a year) | 33 (19%) | 46 (18%) | mostly one core | cells x steps |
| the rest (Flow's `state` process) | about 14 | about 16 | one core | cells x steps |

- **Rounds:** island 2.26 a step (2 rounds in 74% of steps, 3 in 26%); globe 3.98 (4 in 94%). On the globe the 4th
  round is needed only by the gas test: after round 3 the temperature has moved by 0.003 to 0.04 K, but the gas by 2
  to 9e-5 of the air (over 1e-5).
- **Inside a cell row:** 1.1 to 1.2 surface balances per cell and 1.7 to 2.1 Newton steps per balance; the end
  reflectance is about 30% of `pass`. There is no big waste inside a row: about 1.3 µs per cell and round.
- **On 4 threads** (D3's measure and Z1): `pass` 31 / 60 ms, Flow 130 / 189 ms. So **99 / 129 ms of Flow (76% /
  68%) run on one core.** Cutting rounds helps the wall time less than the instructions say.
- **The air level is small:** the band solve is 0.08 ms a call on the globe. On a 72 x 36 globe (162 air nodes, no
  life, year 3) it was 0.4 ms a call, and the rounds grew to 6.9: there `solve` was 99 ms a year and becomes a wall
  (section 6.5).

### 1.5 What I tried (on a copy, not kept)

The long-step test's world (`tape/tests/ranges.rs`): 32 x 32 with life, seeds 1, 2, 3, 20 years, means of years
11 to 20, each seed against itself at short steps (air 6 hours, physics 2 days). Limit: 0.3 K for a temperature.

| Variant | air steps a year | t_land | summer | winter | verdict |
|---|---|---|---|---|---|
| today: 10-day air step | 36 | +0.15 | +0.16 | +0.03 | holds |
| 15-day air step | 24 | +0.30 | +0.12 | +0.57 | fails (winter) |
| 30-day air step (physics 10 days) | 12 | +0.65 | +0.28 | +1.19 | fails |
| 30-day air and physics step | 12 | +0.64 | +0.30 | +0.91 | fails |
| 30 days in the 4 solstice months, else 10 | 25 | +0.40 | -0.17 | +1.00 | fails |
| 30 days in 8 months (all but the equinox months) | 14 | +0.67 | +0.24 | +1.22 | fails |
| one round only (linearly implicit Euler) | 36 | | | | NaN in year 1 (a fresh world moves too much for one linearisation) |
| first guess extrapolated from the last step | 36 | +0.03 | +0.03 | -0.08 | holds, but 2.4 rounds instead of 2.1 (the air's change from step to step is not smooth): slower |
| calm cells (`CALM_K` 0.01 K) on the island | 36 | +0.00 | +0.00 | +0.00 | holds, but no gain: with 2 rounds the air moves a whole step between round 1 and 2 |

On a 64 x 32 globe without life (the globe test's world, 20 years, means of years 11 to 20, against today):

| Variant | rounds | cell rows a step | `pass` | `solve` | t_land | ice |
|---|---|---|---|---|---|---|
| today | 4.38 | 8,966 | 226 ms/yr | 48 | 285.94 K | 0.1025 |
| gas test 1e-4 (`STILL_SHARE`) | 3.50 | | -22% | -24% | -0.01 | +0.0002 |
| calm cells 0.01 K | 4.22 | 5,955 | -26% | -2% | -0.03 | +0.0005 |
| both | 3.44 | 5,771 | -24% | -15% | -0.03 | +0.0006 |

What it says: the climate's error at long steps is **seasonal and everywhere** (warm winters on every seed), not a
storm in one place. The air is never "calm" in a way a first-order step can use. The real waste is in the solver's
rounds and in work that runs on one core.

## 2. The new design

Five pieces, each its own step (section 9). Two are sure (AIR1, AIR2), one is plain code (AIR3), three are research
or gated (AIR4, AIR5, AIR6).

### 2.1 AIR1: calm cells and a calm solver (inside one air step)

**Calm cells.** In round 2 and later, a ground cell recomputes its row only if the air it sees moved since its row
was made: its temperature (`lerp` of `it.t`) by more than `CALM_K` (0.01 K) or its active gases (the sum of `lerp`
of `it.x`) by more than `CALM_SHARE` (1e-4 of that sum). Otherwise it keeps its row: the exchange stays linearised
about the air of its last round, and its slopes still enter the Newton matrix. This is inexact Newton (Dembo,
Eisenstat and Steihaug 1982): the answer converges to the same point within the tolerance.

Data: `Scratch` gets one new cache (not saved): `seen_at: Vec<[f64; 2]>`, per cell the air temperature and gas sum
its row was made with. Code: in `step::pass` (`step.rs:197`), a test before `cell`; the round number passed in.
Two named consts next to `STILL_K`:

```rust
/// A cell keeps its row from the round before when the air it sees moved less than this, K,
const CALM_K: f64 = 1e-2;
/// and its active gases less than this share of their sum.
const CALM_SHARE: f64 = 1e-4;
```

**Calm solver.** `STILL_SHARE` goes from 1e-5 to 2e-4 (`step.rs:19`; 1e-4 left the globe at 3.69 rounds, X39). Reason: the gas test is the one that binds;
1e-4 of the air is about 0.5% of its vapour, and the temperature has already stopped moving (0.003 to 0.04 K).
The rain of a step moves by about that share, far under the year-to-year spread of rain on land.

Books: unchanged. `book` books the rows of the last round as they are, cut by `hold_air`. A kept row is a row like
any other: what it moves is booked both sides, and `hold_air` stops any overdraw of the air.

### 2.2 AIR2: the air's loops on all cores (needs the owner's choice 4)

Today the air's per-cell loops run in a fixed order on one core so the sums are bit for bit. With choice 4 (thread
order free, books exact), they can run per domain on the pool. A domain is the 16 cells under one air node, so a
domain's cells feed mostly their own air node.

- **One task per domain per phase**, not one per loop. D3 measured that a pool call costs 30 to 55 µs and that
  loops under about 200 µs a call do not pay. So the small loops are fused into a few phases, each one call:
  1. *before the rounds:* `Draws::settle` per cell (its claims, its pool), the `dissolve` rows (not the apply),
     `fixed`, `room`, `air::seen`, `ground_state` (in `ground_now`).
  2. *each round:* `pass` (already on the pool) and `gather` fused: a domain makes its cells' rows, then adds them
     into a **per-domain partial** of `Gathered` (its own node and the slopes to the nodes within `far`). The
     partials are added in domain order at the end of the phase (36 to 45 small arrays: cheap). The band solve stays
     on one core: it is small.
  3. *after the rounds:* the row work of `book` (the links per cell, the clouds per cell) and `Draws::owe` per cell.
     `store::apply` stays one call (its speed is the engine's work, AIR3).
  4. *the liquid:* `mobile`, `slopes`, the per-cell part of `springs` and `under` per domain. The pass down the cells
     (`solve::downhill`, a priority flood: upstream before downstream) is a physical order and stays on one core.
  5. *Flow's `state` process:* `ground_state` and `sums` per domain.
- Where it lives: a helper `flow::on_domains(w, |domain, cells| ...)` next to `par::rows` (`core/src/par.rs`), or
  the engine's general domain runner if the engine builds one (one place for each idea).
- The sums change order, so the world changes in the last digits (check level 2; the golden test is re-pinned).

### 2.3 AIR3: the air's book as columns

`book` (`book.rs:17`) rebuilds each step the list of links from each cell to the air: its own node and the four it
sees (`ends`, `from`: 5 n entries), per active gas, then calls the general `store::apply` with 8 kinds of links. It
is 10 to 13% of Flow. Make it a **column move**: per cell and gas, what goes up into its own node, and what rains out
of each of its four nodes (a fixed 4-wide array, the shares). One dense pass books both sides: the cell's ground,
the four air nodes, the heat, the lift. `hold_air` stays as it is (it cuts the rain shares and the wind).

This is a pure speed change. If the engine (choice 5) gives a cheaper apply for "many moves between two classes of
nodes", the air uses it instead of its own pass (one place for each idea). If the order of the sums is kept, check
level 1; else level 2.

### 2.4 AIR4 (research): calm months by a step of second order

The only honest way to take fewer air steps is a method whose error is smaller at long steps. Backward Euler is
first order and damps the seasons (warm winters, section 1.5). BDF2 was tried in the foundation and swung a polar
node (decision 22): it is a two-step method, and its history and the explicit pushes of the step's start fed each
other.

**Proposal: TR-BDF2** (Bank et al. 1985; Hosea and Shampine 1996). It is a one-step method (no history from the step
before), second order, and L-stable (stiff modes, like the land with a 3-day relaxation, are damped, not rung). It has
two implicit stages: a trapezoid to `gamma dt` (gamma = 2 - sqrt 2), then a BDF2 stage to `dt`. Each stage is one
of today's Newton solves (rounds of `pass`, `gather`, `solve`). It reuses the rates at the step's start: the rows of
the step before, which `Scratch::rad` already holds.

- **Books:** TR-BDF2 is stiffly accurate: what the step moves is `dt (w f0 + w f1 + d f2)` with `w = sqrt 2 / 4`,
  `d = 1 - sqrt 2 / 2`, all weights positive. So `book` books one weighted sum of the three row sets, once. `hold_air`
  cuts that sum where an air node would overdraw, so no node is ever overdrawn.
- **Calm, by the method's own error estimate** (Hosea and Shampine 1996 give an embedded estimate for TR-BDF2): at
  each month's start, the air takes the month in one 30-day step if the estimate of the last month, scaled to 30
  days, is under `CALM_TOL` in every air node (0.1 K, and 1% of its vapour); else it takes three 10-day steps. Only
  steps that fit the clock: 10 or 30 days, aligned to the month (Gustafsson 1991 for the control, with a fixed grid
  of steps).
- **Clock:** the air process still runs every 10 days. When the month is calm, its first run takes 30 days and books
  them; the next two runs see `Scratch::air_until` and do nothing. (I tested this mechanism with Euler: it works.)
- **What must be checked first (R1, before any code of TR-BDF2):** why 30-day Euler steps warm the winters. Switch
  one explicit part at a time at 30 days (the clouds of the step before; the wind's pushes from the step's start;
  the end reflectance; the evaporation bucket integrated over the step; the gas of the bodies' node held a month).
  If an explicit part, not the integrator, makes most of the error, fix that part instead (it is cheaper and safer).
- **Risks that kill it:** the rows are amounts over the step, not rates (the bucket of evaporation is integrated
  exactly over `dt`, `exchange.rs`), so `f0`, `f1`, `f2` must be read as mean rates of their stages; the explicit
  clouds and pushes are 30 days old; two stages cost two solves, so a month in one step costs about 2/3 of three
  Euler steps in rounds, but only 1/3 in the per-step work (book, dissolve, setup, draws, state).

### 2.5 AIR5 (gated, for large worlds): an air grid sized by the planet

Not for the benches. For a world of 2,000 cells or more, the band solve grows as air nodes x (fields x row)², and
the rounds grow too (72 x 36 globe: 6.9 rounds, `solve` 99 ms a year). Two honest answers:

1. **Keep the air nodes about the same count**: `air_scale` from the map size (for example 8 for a 72 x 36 globe),
   and **give domains their own level** (level 1 = domain blocks of 4 x 4, level 2 = air), so the bodies keep enough
   domains for the threads. Then several domains share one air node. A domain may take from a shared air node at most
   its share by area (`Mesh::share`), so two domains on two threads can never overdraw it together.
2. **Or a solve that grows only linearly with the air nodes** (a preconditioned Krylov solve, or multigrid on the
   air level), with the band solve kept for small worlds. Only if (1) loses too much climate.

**What a coarser air loses** (why not on the benches): the globe 36 x 18 would have 3 air rows from pole to pole
instead of 5 (the only scales that divide 36 columns are 6 and 12), so the heat carried to the poles and the rain
belts would be lost; the island would have 9 air nodes, so the coast-to-inland contrast of rain smooths out. The
orographic rain stays (each cell takes its air at its own height, `step.rs:217`). The bodies would have 9 domains on
the island instead of 36: worse balance on 4 threads.

### 2.6 AIR6 (research): calm sea cells share one surface balance

Open sea is the most "calm" surface: under one air node, cells of the same row of latitude that are open liquid at
the top, with no ice, no bodies and no draws, and ground temperatures within 0.2 K, have nearly the same surface
balance. Make **one row** for a group (its first cell), and for each other member **correct the row by the row's own
slopes** with its own air (`tl`, its gas column): `row_i = row_g + slope_t (tl_i - tl_g) + slope_x (col_i - col_g)`;
its end temperature by `Balance::follow`. If a member's air differs by more than 0.5 K, it gets its own full row.
This is the tile idea of land-surface schemes (Avissar and Pielke 1989; Koster and Suarez 1992) used the other way:
alike surfaces share one solve.

Books: each cell's own row is booked from its own store (`row[o].min(a + r)` already guards evaporation); `hold_air`
guards the air. Gate: in AIR0, count the share of cells that would group (I expect about half on the island and two
thirds on the globe, to be measured). Build only if `pass` can lose 30% or more.

## 3. The law changes in physical terms

- **AIR1:** no law changes. The air's step solves the same equations to a slightly looser tolerance (the gas within
  1e-4 of the air), and cells whose air did not move keep their linearised exchange. The books are exact as before.
- **AIR2, AIR3:** no law changes. The sums run in another order.
- **AIR4:** the air's step becomes a second-order step and is 10 or 30 days long, chosen each month by its error
  estimate. The air books a calm month at its start, ahead of two physics steps: Change, the survey and the bodies
  read the air of the month's end during that month (today they read the air of the next 10 days' end). That is an
  order effect of up to 20 days, checked by the key numbers.
- **AIR5:** the air nodes are larger on large worlds. Bodies in several domains share one air node by area.
- **AIR6:** alike sea cells share one surface balance, each corrected to first order for its own air.

Why it stays honest: every move is still booked both sides through `store::apply`; `hold_air` still cuts the wind
and rain where an air node would give more than it holds; the ground evaporates at most what it holds and gets; no
law reads a counter.

## 4. Which rules change (proposed wording)

1. **`CLAUDE.md` 6 (Working), new item:** "6.5 The air's solver tolerances (`STILL_K`, `STILL_SHARE`, `CALM_K`,
   `CALM_SHARE` in `flow/air/step.rs`) are speed numbers, like a time step. A change re-pins the golden test and
   must pass the long-step test and the globe's climate test, in the same commit."
2. **`docs/numbers.md`, row `air_hours` (only if AIR4 is kept):** "the air's step in a month that is not calm; a
   calm month (the error estimate of TR-BDF2 under `CALM_TOL`) takes one step of the whole month (`flow::air`)."
   New row `calm_tol`: "K and share of vapour; the largest error a calm month's single step may make; 0.1 K and 1%,
   anchored on the long-step test (0.3 K); speed".
3. **`Config::validate` (`params.rs:384`), only with AIR4:** "The air's step (`air_hours`) divides the physics step;
   a calm month's single step is the month, a whole number of physics steps."
4. **Handover 7.3, the "As built" paragraph:** today "Backward Euler is first order in the step, so a longer step
   needs a step of second order." With AIR4: "The air's step is TR-BDF2 (second order, L-stable): 10 days, or the
   whole month when its error estimate says the month is calm."
5. **Handover 7.4 (only with AIR5):** "The cells are cut into domains: the cells of one block of the domain level
   (4 x 4 cells). On large worlds an air node covers several domains; each takes from it at most its share by area."
6. **Foundation decisions 6 and 22** stay true for Euler. With AIR4, a new decision in this round's decisions file
   says why TR-BDF2 does not have BDF2's problem (one step, no history).
7. **`docs/laws.md` 1.4:** the sentence "A long step gives the world of short ones" gets the method's name and, with
   AIR4, the calm month.

## 5. How we check it

| Step | Level | Must hold |
|---|---|---|
| AIR1 | 3 (same world) | golden re-pinned; `the_long_step_keeps_the_climate` and `a_globe_without_life_keeps_an_earth_like_climate` pass; the key numbers; books 1e-12 |
| AIR2 | 2 (same to rounding; life amplifies it) | golden re-pinned; 5-year runs of the benches: climate columns of `series.csv` within 0.05 K and 1%; books |
| AIR3 | 1 if the order is kept, else 2 | golden unchanged (level 1) or re-pinned |
| AIR4 | 3 | long-step test with the calm months against short steps within 0.3 K; the globe test; the key numbers; the sleep test; books |
| AIR5 | 3 | the globe test at the large size, against the same world at `air_scale` 4 |
| AIR6 | 3 | the long-step test and the globe test |

**"The same world" for the air:** the six climate columns of the long-step test (`t_land`, summer, winter,
`rain_land`, `air_3`, `rivers`), each seed against itself: within 0.3 K for a temperature and within the
year-to-year spread for the others; the globe test's bounds (land, drift, ice, surface, vapour, rain); books within
1e-12.

**Tests:**
- Fix, before anything: the long-step test's short run uses `phys_days` 3 (`ranges.rs:284`), which
  `Config::validate` refuses since `batch_days` is 2 (the test does not call `validate`, so it runs a world the CLI
  would refuse). Use 2 days.
- No new test for AIR1 to AIR3 (the existing climate tests state the law).
- AIR4: the long-step test runs the default (calm months on) against the short steps. No test of the chooser
  itself (that would be a story, not a law).
- AIR5: the globe test at the large size.
- Watching: a yearly counter `air_rounds` (mean rounds a step) and, with AIR4, `air_steps` in `Counters` (tape only;
  no law reads them).

## 6. Expected gain

Base (Z1, 4 threads, ms a year): Flow 130 (island22) and 189 (globe36). `pass` on 4 threads 31 and 60 (D3); the
rest of Flow on one core: 99 and 129.

| Step | island22 Flow | globe36 Flow | Arithmetic |
|---|---|---|---|
| now | 130 | 189 | |
| AIR1 | 128 | 163 | globe: rounds 4.0 to about 3.1 and 36% fewer rows: `pass` 60 to 40, `gather` 13 to 10, `solve` 14 to 11. Island: already 2.26 rounds, little to cut |
| AIR2 | 98 | 126 | serial per-cell work on one core today: about 55 ms (island) and 69 ms (globe): dissolve, gather, setup, draws, state, book rows, half the liquid, Flow's state. At about 2.2 times on 4 cores (memory bound, as the bodies got 2 times): -30 and -37 |
| AIR3 | 88 | 114 | book 19 and 23 ms to about 9 and 11 |
| AIR6 (if built) | 78 | 98 | `pass` -30 to -45% |
| AIR4 (if kept) | 52 | 66 | most months in one step: the per-step work / 3, the rounds x 2/3 |
| **total, sure steps (AIR1 to AIR3)** | **1.48 x** | **1.66 x** | range 1.3 to 1.8 |
| **total, all** | **2.5 x** | **2.9 x** | range 1.8 to 3.2 |

**The year (Amdahl).** If the other sections make the bodies 10 times cheaper (42 and 46 ms) and nothing else
changes (Change 46 and 46, parts 3 and 33, tape 15 and 13):

| | island22 | globe36 |
|---|---|---|
| year with today's Flow | 236 ms | 327 ms |
| with AIR1 to AIR3 | 194 ms | 252 ms |
| with all air steps | 158 ms | 204 ms |
| the aim (10 times) | 86 ms | 135 ms |

So **after the bodies get cheaper, Flow is the wall**, and the air work alone does not reach 10 times: even Flow at
its best case plus Change (46 ms) is more than the whole aim on the island. 10 times also needs Change cut by 3 to 4
times and the liquid cut. On the globe the parts (Bond's month, 33 ms) also count.

**Large worlds.** A settled cell's Flow costs about 0.27 to 0.29 ms a year now; after AIR1 to AIR3 about 0.18; with
all about 0.10 to 0.11. Without AIR5, worlds over about 2,000 cells lose part of that to the band solve and to more
rounds.

## 7. Line cost (core code lines, today 9,591, ceiling 9,700)

| Step | Added | Removed | Net | Deletes |
|---|---|---|---|---|
| AIR1 | 12 | 0 | +12 | nothing (a number changes) |
| AIR2 | 50 | 15 | +35 | the per-loop calls of `par::rows` in `step` replaced by phases |
| AIR3 | 30 | 40 | -10 | the rebuilt `ends`/`from` lists and the 5-link `LinkFlux` of the air's book |
| AIR4 | 180 | 40 | +140 | the plain backward Euler path (the method becomes TR-BDF2 everywhere, also in non-calm months: one method) |
| AIR5 | 120 | 10 | +110 | only if gated in |
| AIR6 | 70 | 0 | +70 | only if gated in |

Sure steps: about +37 lines. All: about +360. The ceiling must rise (the engine section, choice 7).

## 8. Risks and answers

| Risk | Answer |
|---|---|
| Calm cells keep a row whose air moved a little: a small bias in rain or warmth | The tolerance is 0.01 K; measured -0.03 K on a 64 x 32 globe; the climate tests hold it; the books stay exact |
| The looser gas test changes rain | Measured -0.05% of rain on land; under the year-to-year spread |
| AIR2: the threads fight over memory, the gain is small | Fused phases (few calls); kill line 1.15 times on Flow on 4 cores |
| AIR2: free order makes runs on different thread counts differ | Accepted by the owner's choice 4; books exact |
| AIR4: the explicit clouds and pushes, 30 days old, make a node swing (as BDF2 did) | R1 first finds the cause of the warm winters; TR-BDF2 is one-step and L-stable; the chooser falls back to 10 days where the estimate is large; kill line |
| AIR4: the rows are amounts over the step, not rates | Read each stage's row as its mean rate; the evaporation bucket is integrated per stage |
| AIR4: the bodies, Change and the survey read the month's end air for 20 days | An order effect; the key numbers and the sleep test check it |
| AIR5: two domains overdraw a shared air node | Each takes at most its area's share (`Mesh::share`); `store::apply` counts any rounding below zero |
| AIR6: grouped cells drift apart | Strict group test (same row, open liquid, 0.2 K); a member whose air differs by more than 0.5 K gets its own row |
| Everything: Flow stays the wall | Said plainly in section 6; the plan's 10 times needs Change and the liquid too |

## 9. Steps

| ID | What | Level | Done when | Kill line | Needs | Parallel with | Agent |
|---|---|---|---|---|---|---|---|
| AIR0 | Measure on a branch, on the new benches: the table of 1.4 (1 and 4 threads), rounds, the share of cells that are calm per round, the share of sea cells that would group (AIR6) | none | the table in this round's measures | none | none | all | Haiku |
| AIR-T | Fix the long-step test's short physics step (3 to 2 days) | 1 (a test) | the test runs a world `validate` accepts and passes | none | none | all | Haiku |
| AIR1 | Calm cells (`CALM_K`, `CALM_SHARE`) and `STILL_SHARE` 2e-4 (X39) | 3 | rounds globe36 at most 3.3, cell rows -25% or more, the climate tests pass, golden re-pinned | Flow on 1 thread (callgrind) under 8% better on globe36 | AIR-T | AIR3, AIR4-R1 | Sonnet |
| AIR2 | The air's and the liquid's per-cell loops in phases per domain on the pool (free order) | 2 | Flow on 4 threads 1.25 times faster on both benches | under 1.15 times on either | the engine's rule for choice 4; AIR1 | AIR3 | Opus |
| AIR3 | The air's book as column moves | 1 or 2 | `book` -40% | under -25% | engine's apply design (choice 5), or none | AIR1, AIR2 | Sonnet |
| AIR4-R1 | Find what makes 30-day steps warm the winters (switch the explicit parts one by one) | none (branch) | a table: each part's share of the +1.2 K | none | AIR-T | AIR1 to AIR3 | Opus |
| AIR4 | TR-BDF2 with the calm-month chooser | 3 | long-step test within 0.3 K, globe test, key numbers, books; at least 6 of 12 months calm on both benches | Flow on 4 threads under 1.4 times faster than after AIR3, or any climate test fails | AIR4-R1 says the integrator is the cause; AIR1 to AIR3 | AIR6 | Opus |
| AIR5 | Air sized by the planet; domains on their own level; area quotas | 3 | the globe test at 72 x 36; `solve` under 10% of Flow | the globe test fails, or the bodies' stage loses more than 5% | gate: a world of 2,000+ cells is a target of this round | none | Opus |
| AIR6 | Alike sea cells share one row | 3 | `pass` -30%, the climate tests pass | `pass` under -20% | AIR0 shows 40%+ cells group; AIR1 | AIR4 | Opus |

Order: AIR0 and AIR-T first; then AIR1 and AIR3 and AIR4-R1 side by side; AIR2 when the engine's choice-4 rule is
in; AIR4 and AIR6 last. A step that misses its kill line is reverted and kept on a branch.

## 10. Interactions with the other sections

**What I need:**
- **Engine:** (a) the rule of choice 4 (any sum order, books exact) before AIR2; one helper to run a phase per domain
  on the pool (if the engine builds one, AIR2 uses it). (b) With choice 5 (32-bit moves inside a month, exact books
  each month): **the air step must read exact 64-bit amounts at its start** (its implicit solve starts from them, and
  `hold_air` needs the true holdings to stop overdraws). So either the air's classes stay 64-bit, or each air step
  (3 a month) is a closing point of the books. (c) New code should not assume "a domain is the cells under one air
  node" (AIR5 may change it). (d) The ceiling (choice 7): this section needs about +40 lines sure, +360 at most.
- **Plants (as a field):** per cell, the same three numbers the air reads today: light-capture area (`w.canopy`),
  the share absorbed (`life.absorb`) and the share their light reactions store (`life.keep`). The field's heat drawn
  from the ground as **one `Draw` curve per cell** (`survey/draws.rs`), not one per body: then `Draws::settle` and
  `owe` loop over cells, not bodies (about 8 ms to under 1 ms a year). The marks of light after each air step
  (`book.rs:62`, a loop over all bodies) become a per-cell add for fields. Choice 3 (upkeep and light together across
  the interval): the air keeps booking light per cell per step into `w.light`; the plants integrate it over their
  interval. No change in the air.
- **Minds:** nothing needed.

**What I give:**
- **Engine:** the measure that on 4 threads 68 to 76% of Flow runs on one core, and the list of its loops (2.2).
  The air step as a natural booking point (3 a month).
- **Plants:** light per cell per air step (`w.light`), rain per cell (`w.rained`), the ground and air temperature;
  unchanged.
- **Minds (choice 8, think on change):** a cheap wake signal per cell: the change of the air it sees over the last
  air step (the step already has it: `next.t` against the step's start), and the rain of the step. A mind can wake
  when its cell's air moved by more than a threshold or the season turned.
- **Change:** its sleep is not touched. With AIR4, Change still reacts every 10 days on the air the month's step left;
  the sleep test (S84's open bias on `air_3`) must be read with that in mind.
- **Liquid:** stays monthly (`flow_days` 30, a whole number of air steps also with AIR4); its per-cell loops join
  AIR2; its downhill pass stays on one core.

# Lighter world: the plan

Status: **planned, not started.** Branch `speed/lighter` (from `speed/x10` at `9656b89`). This file is the one source
of the steps: each row has its files, check level, done-when and kill line. The designs are in `design/` (one note per
idea; `cover.md` for idea 5 comes from its own designer and is merged here by the architect). Decisions: `decisions.md`
(L1 and on). Measures: `measures.md` (made by the first step that measures). History: `docs/history.md`, section 7.

## 1. The goal, and what to expect

**The goal:** compute less. The last round ended with this finding: most of a year goes to the background (plants 37% /
19% / 60% of a year on one thread on island22 / globe36 / region24, the air 11% / 17% / 9%, the saves of a long run 35
to 50 ms of a 313 to 709 ms year), not to the story (animals that choose). This round changes **what** we compute. The
owner said yes to all seven ideas, the law changes too (L3). What must stay is the goal of the game (handover 1, 6,
13): animals with brains, trails, later people, settlements and trade; a climate with seasons, rain and rivers; plants
that are food, cover and fuel and that adapt to their places.

**What the round can give** (estimates; the review after phase P restates them from the probes). One thread,
instructions of a settled year (`schedule::run_for`) on the start's benches (`benchZ`: the saves made at `cb6db47`,
snapshot 49); "x" is faster by.

| Idea | island22 | globe36 | region24 | How it is counted |
|---|---|---|---|---|
| 1 saves (lane F) | 0 | 0 | 0 | benches write no save; **long runs -8 to -12%** of a year (wall, with the saves) |
| 2 fewer stands a cell (A2) | -7 to -14% | -4 to -8% | -7 to -14% | stand updates are about 28% / 19% / 29% of the year after PL1; stands a cell -25 to -50% |
| 3 living bodies do not rot (B) | -3 to -7% | -3 to -7% | -2 to -4% | the reactor of a stand update falls by about half; an animal's reactor runs only bare reactions today, so it has none to run |
| 4 a cheaper sample (A1) | -1.5 to -2% | -1 to -1.5% | -1.5 to -2% | the sample is about 13% of a stand update; `sample_every` 4 halves it |
| 5 the cover (C) | x1.5 (owner's guess) | x1.2 (plants are 19%) | x2 (owner's guess) | from `design/cover.md`; it replaces A1 and A2 |
| 6 animals look around less (D) | -2 to -4% | -2 to -5% | about -1% | `choose::act` is 11% of the year (island22, globe36), the take `take_node` a third of it |
| 7 a coarser air on globes (E) | 0 | -1 to -3% | 0 | the Newton part of the air is about 17% of the globe's air; more on large globes |

| Case | island22 | globe36 | region24 | long runs | the largest globe in 15 minutes |
|---|---|---|---|---|---|
| **everything works** (C replaces A) | **about x1.5 to x1.7** | **about x1.3 to x1.4** | **about x1.9 to x2.1** | about 10% more | size 28 now; about 32 |
| **C fails its gate** (A1, A2 and their fallbacks A3, A4 instead) | about x1.2 to x1.3 | about x1.2 | about x1.2 to x1.3 | about 10% more | about 30 |
| only the safe part (F, A1, B, D1) | about x1.1 | about x1.1 | about x1.05 | about 10% more | 28 to 30 |

On 4 threads the gain is smaller where the part cut ran on the pool (the bodies run x2.4 / x1.75 / x2.6 faster on 4
threads than on 1). **Plainly:** without the cover, this round gives about a fifth; with it, about a half on the island
and the region and a third on the globe.

## 2. Rules for this round

All of `CLAUDE.md` and of the round's brief (kept outside the repository, beside the scratch files) holds. The rules
that change land with the step that makes them true (section 7).

- **Check levels** (the brief): 1 bit for bit; 2 the same to rounding; 3 the same world (key numbers, pace and batch
  tests, the nodes check when a step touches nodes or minds); **4 a new law for a part of the world**: each level-4 row
  below says what must still hold and which tests change. Never loosen a tolerance; a test of a deleted mechanism is
  deleted; a test of a changed law is rewritten to state the new law.
- **Kill lines ask for at most 10% of the year** (L2; for lane F, of a long run's year): a line on a part (a stand
  update, the take) asks no more than that of the year. A step under its kill line is reverted and kept on a branch
  `lighter/<id>-not-kept`, and its alternative (section 3) starts.
- **References** (L10): the key numbers of the start are those of `docs/speed-x10/measures.md` "Z: the final checks" 1
  (the same world as `9656b89`). After each level-4 law lands, the key-number test runs once more and its values become
  the reference for the steps after it. S50 and S84 stay the known failures; a step must not move them past their seeds'
  noise unless it says why.
- **How a gain is judged** (L11). One thread: callgrind instructions of a year (`RAYON_NUM_THREADS=1`). Four threads:
  wall time under `flock $L/timing.lock`, 5 or more interleaved runs, medians, the load said. A step that keeps the
  snapshot runs `benchZ` as it is. A step that changes the snapshot is judged on one world: the base's save turned to the
  new snapshot, instructions per update or per stand-year (X50); across remade worlds only large cuts count (PR-S: 19%
  island22, 11% globe36). **Long runs count wall time, the saves included** (L8).
- **Agents.** Law steps (level 4) and every adversarial review go to the strongest agents; normal code to normal
  agents; measures to measuring agents. Each step in its own worktree and branch; the architect merges one at a time;
  a step that re-pins golden rebases and re-pins on `speed/lighter`; `SNAPSHOT_VERSION` goes to the highest plus one at
  each merge.
- **The three rules for smart bodies hold:** individuals stay real; the step per place (each domain owns its bodies and
  its ring); two kinds of body code.
- **Core lines** (L4): the ceiling is 10,643. A step may pass it only with a decision that says why. Each step names
  what it deletes. The cover should delete more than it adds.

## 3. The owner's seven ideas: what we build, and the alternative if it is dropped

| # | Idea | What we build | If it fails: the alternative |
|---|---|---|---|
| 1 | Cheaper saves | F1 the save timed with the tape; F2 the saves and the tape written on a writer thread; F3 the tape so far written cheaper (gzip fast, csv appended). Snapshots stay every 10 years (L13) | Snapshots every 20 years and at the end, the owner told (handover 7.3 and 8.3 change) |
| 2 | Fewer stands a cell | A2: past `crowd`, stands merge their most alike pairs whatever their genes, or the smallest rule PR2 names (L6) | A3 after C's gate fails (a wider rule PR2 names); else nothing more for stands but C |
| 3 | Living bodies do not rot | B1: a living body runs only the reactions its own catalysts run (L9); B2: a body with nothing to react skips the reactor | B1 for stands only, if the eaters break (two kinds of body code, said in `laws.md`); else not built and the owner told |
| 4 | A cheaper sample member | A1: `sample_every` 4 | A4 after C's gate fails: the sample runs only the stages its gene reads; else `sample_every` stays 2 |
| 5 | Plants as a cover | C1 to Cn, from `design/cover.md` | A3 and A4 (stands stay bodies) |
| 6 | Animals look around less | D1 a cheaper take (level 1); D2 a cheaper view of the bodies seen, by PR6 (L14) | the part of perception C takes away (the stands' faces); else nothing more |
| 7 | A coarser air on globes | E1: air nodes wider than tall on globes (rows of latitude kept), gated by PR7 (L12) | E2: a solve that grows linearly with the air nodes, for large globes only; else nothing |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. The design note of each idea holds
the detail (what the code does today, what changes, the risks, what its probe must show).

### Phase P: probes (in parallel; branches `probe/<id>`, never merged)

| ID | What | Done when | Gate it decides | Detail |
|---|---|---|---|---|
| R0 | The start: `benchZ` profiled again with `#[inline(never)]` on the parts the probes name (`physics`, `react`, `copy::sample`, `perceive`, `options`, `learn`, `take_node`, `spread::repeat`, `flow::air::step`'s Newton and band solve, `save_snapshot`, `Recorder::finish`); the 4T year of `$L/ew-base` on `benchZ` (5 sets) | a table in `measures.md` "R0" | the base of every step | this plan |
| PR1 | What a save costs, by part: bincode into memory, gzip at `fast` and at levels 0 to 1 of other coders, the write; `Recorder::finish` by file (the viewer's JSON and gzip, `months.csv`, `series.csv`); how both grow over a 300-year island run (extrapolated to 1,000); the gap between the run's timed year and its wall time | its table | F2 and F3 first or not; whether F3 is needed | `design/saves.md` |
| PR2 | Why a cell holds 8 stands (island22; up to 26): stands by stage, by kin group, left after `join_alike` and after `merge_closest` and why (unlike), children that start a young node because they are not `within`; then with A2's rule on a branch: stands a cell, stand updates, the mean spread of each gene at year 30, key numbers once | its table | A2's rule and its line | `design/stands.md` |
| PR3 | Bare reactions in living bodies: by reaction, how often and how much they run in stands and in animals (matter, fuel, gas and heat a year against the world's flows); with a switch on a branch, variants (a) no bare reaction in any living body, (b) only the rot (reactions with `open` > 1) stops, (c) (a) for stands only: the cost of a stand and an animal update, the key numbers (3 seeds), the land's temperature, gas, evaporation and light-eaters over years 0 to 30 on 32 x 32 (P1.2's setting); **why P1.2 moved the land -0.78 K** (each reaction switched alone until the cause is named) | its table; the cause of any land shift over 0.3 K named | B1's variant (L9) | `design/no-rot.md` |
| PR4 | The sample member at `sample_every` 2, 4, 8: the cost of a stand update; `selection_inside_a_node_moves_its_mean_as_its_members_would` and `genes_do_not_drift_without_selection`; the gene means of light-eaters at year 30 (3 seeds) | its table | A1's setting | `design/sample.md` |
| PR6 | What a decision and a habit cost, by stage (perceive, inputs, think, learn, walk, options, pick, spread, the take); the faces looked at per decision by kind (stands, animals); the take's cost by part (`intake`, `take_node`'s payer, the booking) | its table | D1 and D2; which waits for C | `design/perception.md` |
| PR7 | The air on globe36 and on the size-28 globe: Newton rounds, the band solve, the per-node and per-cell work; on a branch, air blocks of 6 x 4 (globe36) and 8 x 4 (size 28 and the 64 x 32 test globe): rounds, the air's instructions, the busiest domain's share, the globe climate test, zonal rain and land temperature by row over 30 years (3 seeds) | its table | E1 (shape, gate) or E2 | `design/air.md` |
| C0 | The cover's own probes, from `design/cover.md` | as there | C's build | `design/cover.md` |

**Review after phase P** (an adversarial review, then a fix, until accepted): section 1 restated from R0 and the
probes; each gate said go or no go; a dropped idea gets its alternative started.

### Phase 1: six lanes side by side

Each lane is its own part of the code, its own agent and worktree. Lane B lands before C builds its chemistry (L5).

**Lane F: the saves** (`tape/src/snapshot.rs`, `tape/src/rec/`, `cli/src/main.rs`; `design/saves.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| F1 | The snapshot's write is timed in the tape's column (`clock.spent[TAPE]`), so a run's "per year" holds it | 1 | golden unchanged; a run prints its year with the save in it | none |
| F2 | The saves off the main thread: the world turned to bytes (bincode into memory) on the main thread, the compression and the write on one writer thread; the tape's files the same way; one write in flight; the run waits for it before the next and at its end | 1 | golden unchanged; tape tests (resume equals unbroken); saves and tape on the main thread under 10 ms a year in a 300-year island run | a 300-year island run's wall time a year under -4% |
| F3 | The tape so far cheaper: `series.csv` and `months.csv` appended, not written again; the viewer's file at gzip `fast`. Only if PR1 shows the writer thread busy over 20% of the run | 1 (files hold the same rows) | tape tests; the writer's work in a 1,000-year run -50% | the long run's wall time a year under -1% |

**Lane B: living bodies do not rot** (`core/src/body/physics.rs`, `body/metab.rs`, `change/kernel.rs`; `design/no-rot.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| B1 | **A law:** a living body runs only the reactions its own catalysts run (the variant PR3 chose, L9); the catalysed burnings run on demand only (`body::pay`), not again at the bare rate in `react`. `laws.md` 2.1, 2.4, 6.x; handover 13.2's "inside bodies" item closed; `numbers.md` rows of reactions 1, 21, 26 | 4 | what must hold, below; golden re-pinned; a stand update 1T -5% on `benchZ` | a stand update 1T under -3%; or the numbers that must hold fail with no cause named: variant (c), else not built |
| B2 | A body with no reaction to run skips the reactor; the reactor's setup only over the reactions a body runs | 1 on B1's world | an animal update 1T -3% | under -1% of the year: revert B2 (B1 stays) |

**B1 must still hold:** every key number but the eaters' two (land, swing, rain, gas, rivers, worn land, light-eaters,
their life) in range; the eaters' two in range, or out with the cause said and the architect's decision logged (the
bare burning of an eater's own matter is 5 to 8 times its upkeep today, PR-A, so they may rise); S50 reported (no bare
sugar burning may raise it); the long-step and globe climate tests; the pace and batch tests; the nodes check;
`the_eaters_live_and_stop_growing`; `a_century_keeps_its_books`; watch-for: eaters do not eat most of the plants'
production (handover 13.2), no boom and crash. **Tests that change:**
`a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses` (its heat made counts no bare burning);
`upkeep_and_supply_follow_the_rate_law_of_the_reactions` if it reads the bare rate in a body; a new test
`a_living_body_runs_only_its_own_catalysed_reactions` (a body with every catalyst off keeps its stores over a year at
300 K; the same matter on moist ground rots at its litter constant).

**Lane A: stands, the cheap forms** (`core/src/copy/select.rs`, `core/src/nodes.rs`, `core/src/params.rs`; `design/sample.md`, `design/stands.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A1 | `Config::sample_every` 4 (PR4's setting); `numbers.md` row | 3 | PR4's two tests pass; key numbers; a stand update 1T -4% on a `benchZ` save with its `Config` set to 4 at load | under -3% of a stand update |
| A2 | Past `crowd`, stands of one lineage and stage merge their most alike pairs whatever their genes (or PR2's smaller rule), at most 30 core lines (L6). `laws.md` 4.4; `numbers.md` `crowd` | 4 | what must hold, below; stands a cell -25%; island22 1T -5% | island22 1T under -4% |

**A2 must still hold:** the key numbers; `selection_inside_a_node_moves_its_mean_as_its_members_would`,
`genes_do_not_drift_without_selection`, `a_node_spreads_with_its_children_and_splits_when_too_varied`; the nodes check;
the mean spread of the light-eaters' genes at year 30 no less than half of today's (PR2 measures today's); light-eaters
still differ by climate (watch-for M1: kinds differ by climate). **Tests that change:** `crowd_bounds_the_nodes_of_a_cell`
(stands of a full cell merge however unlike; animals as today).

**Lane C: plants as a cover** (from `design/cover.md`; its designer writes this table, the architect merges it)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| C1 to Cn | from `design/cover.md` | 4 | from `design/cover.md` | from `design/cover.md` |
| **G-C** | **The cover's gate** (placeholder; from `design/cover.md`): its own checks pass and the plants' cost falls by the amount it states | | | **fails:** C is not merged; A3 and A4 start (phase 2) |

C builds its chemistry after B1 lands (a living cover runs only its catalysed reactions). If C lands, it deletes A1's
and A2's code with the stands, and D's steps that read the stands' faces become moot.

**Lane D: animals look around less** (`core/src/choose/`; `design/perception.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| D1 | A cheaper take: `take_node` pays its handling through the body's own payer, worked out once per update; what PR6 names in `intake` and the booking | 1 | golden unchanged with and without threads; the take 1T -30% | the year 1T under -1% |
| D2 | A cheaper view of the bodies seen, the cut PR6 names (for example faces of one cell read once a batch for the cover chance and the room, or the seen bodies' features worked out once a batch); if it is a law (fewer bodies seen), level 4. **Waits for G-C** if most of the faces looked at are stands (L14) | 2 or 4 | `choose::act` 1T -20% | the year 1T under -2% |

**D2 at level 4 must still hold:** the key numbers; the nodes check; `body_learns_to_move_toward_what_it_can_take`;
`selection_moves_a_nodes_brain_size`; watch-for M2: trails, imitation, signals (the `social` columns of the tape within
their seeds' noise).

**Lane E: the air on globes** (`core/src/space/square.rs`, `worldgen/src/terrain.rs`, `core/src/world.rs`, `core/src/flow/air/`; `design/air.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| E1 | Air nodes wider than tall on a globe: blocks of `sx x sy` squares, `sy` 4 as today, `sx` about 2 `sy` and a divisor of the row (globe36 6 x 4, the size-28 and the test globe 8 x 4; PR7's shape; the island and regions keep 4 x 4): the air keeps its rows of latitude and has about half the nodes, each Newton band about half as wide. `World::air_scale` becomes a pair; snapshot bumped; handover 7.2, `laws.md` 1.4 | 4 | what must hold, below; globe36 1T -1.5%; the size-28 globe 1T -3% | globe36 1T under -1% and size 28 under -2% |
| E2 | Only if E1 fails its climate checks: a linear-time solve of the air level for globes of 1,500 cells and more (`solve/`), PR7's size-28 numbers first | 2 | size-28 globe 1T -3% | under -2% |

**E1 must still hold:** `a_globe_without_life_keeps_an_earth_like_climate`; the long-step test (island: unchanged); on
globe36, 3 seeds, years 21 to 30: land temperature and rain on land within their seeds' noise, zonal rain and land
temperature by row within 1 K and 15%, ice share within its noise; the busiest domain's share on 4 threads at most
0.3; watch-for M0: rain belts, deserts behind mountains, ice at the poles. **Tests that change:**
`core/tests/space.rs` gains blocks of two sizes; `core/tests/flow.rs:179` reads the air's width from the mesh.

**Lane reviews.** After each lane's last step: an adversarial review, then a fixer, until accepted (section 6).

### Phase 2: after the cover's gate

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A3 | Only if G-C fails: the wider merge rule PR2 names (for example young and adult of one kind in one node) | 4 | as A2; stands a cell -40% | island22 1T under -5% |
| A4 | Only if G-C fails: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6% | under -3% |
| D2 | If it waited for G-C (see lane D) | | | |

### Phase Z: the end

1. The year at the round's end against its start: `$L/ew-base` on `benchZ` and the end's build on benches made with
   its own code (`make_benches.sh`), 6 interleaved sets, 4 threads and 1 thread, under the lock; medians. And 1T
   instructions of a year on both.
2. 1,000-year runs (seed 1; island 22, globe 18, region 24 with `--region 8,8,8,8`), one at a time, wall time **with
   the saves** and the default pictures; set beside the last report's.
3. The largest globe that runs 1,000 years in 15 minutes of wall time (start at size 28, then up).
4. The slow suite (`cargo test --release --workspace --no-fail-fast -- --ignored`), the golden test without threads,
   the debug checks (books, overdraw, views; the plain debug golden).
5. The report `docs/reports/lighter.md` (as `docs/reports/speed-x10.md`: in short, where the speed stands, what was
   kept and not, the law changes and their effects, the open items).
6. The docs at the round's end: the `CLAUDE.md` row "Lighter world (done)"; handover 7.1, 7.3, 13.2, 14.3 (the core
   ceiling: the lines then plus 200, or lower if the cover deleted lines, L4); `laws.md`, `numbers.md` and `tape.md`
   as they are; every row of this plan says kept, not kept or not built.
7. The final adversarial review, then fixes, until accepted.

## 5. Order and lanes

```
Phase P   R0 ─ PR1, PR2, PR3, PR4, PR6, PR7, C0 (side by side) ─► review P (section 1 restated, gates)
Phase 1   F: F1 ─ F2 ─ (F3) ──────────────────────────────────────────────┐
          B: B1 ─ B2 ─────────────────┐                                    │
          A: A1 ─ A2 (tiny forms) ────┤                                    ├─► lane reviews
          C: C (probes, prototype) ───┴─ (after B1) C build ─ G-C ─────────┤
          D: D1 ─ (D2, or after G-C) ──────────────────────────────────────┤
          E: E1 (or E2) ───────────────────────────────────────────────────┘
Phase 2   G-C failed: A3, A4 ;  G-C passed: C deletes stands (A1, A2 with them) ; D2 if it waited
Phase Z   the end (section 4) ─► final review
```

**Where steps collide, this order holds:**
- B1 lands before C writes its chemistry: the cover follows the law of living bodies (L5).
- A1 and A2 land before C's gate only in their tiny forms (L6); C rebases on them and deletes them.
- B1 and A2 are both level 4 and both move the plants: they merge one at a time; the second re-runs the key numbers
  against the reference the first set (L10).
- D1 and C both touch `choose/feed.rs` (C changes what eaters take from): D1 lands first (level 1); C rebases.
- D2 waits for G-C if PR6 finds that most of the faces looked at are stands (L14).
- E touches no file of the others but `world.rs` (`air_scale`) and the snapshot version.
- F touches only `tape/` and `cli/`; it lands any time (level 1).
- Merges into `speed/lighter` one at a time; golden re-pinned on the branch by the step that changes it; the snapshot
  version the highest plus one. If `main` moves, it is merged in at the next lane review (X1's rule, L1).

## 6. Reviews

The cycle is: plan, adversarial review, implement, adversarial review, fix, until accepted. Review rounds sit:
1. **After this plan** (with `design/cover.md` merged in): before any probe starts.
2. **After phase P:** section 1 restated, gates go or no go.
3. **After each lane's last step** (F, B, A, D, E): the lane's diffs, tests and measures against its rows; each review
   of a level-4 lane runs the key-number test against the reference.
4. **At C's design** (before C builds, after C0) **and at G-C:** the cover's law, its checks and its measure.
5. **After phase Z:** the report and the docs.

Each review is adversarial (it looks for what is wrong: books, overdraw, honesty of the law, rules of `CLAUDE.md`,
measures that do not prove the claim) and ends with a verdict; a fixer answers each finding; the review runs again
until it says accepted.

## 7. Rule and doc changes, and the step that writes each

| Rule or doc | Lands with |
|---|---|
| `CLAUDE.md` table row "Lighter world (in progress)"; `docs/history.md` section 7 | the first merged step |
| `docs/tape.md`: a run's year holds its saves; how the tape's files are written | F1, F2, F3 |
| `laws.md` 2.1 ("no decomposer works in a body: it runs only its own catalysed reactions"), 2.4, 6.x; handover 13.2 "Inside bodies" closed; `numbers.md` reactions 1, 21, 26 | B1 |
| `laws.md` 4.4 (merging of stands); `numbers.md` `crowd`, `sample_every` | A1, A2 |
| handover 7.1, 7.3; `laws.md` 4, 6 (what a plant is) | C (from `design/cover.md`) |
| `laws.md` 5.2 (what a body perceives), if D2 is a law | D2 |
| handover 7.2 and 7.4 (air nodes on globes, domains); `laws.md` 1.4 | E1 |
| `CLAUDE.md` row "Lighter world (done)"; handover 14.3 and `CLAUDE.md` 2.11 (the ceiling) | Z |

## 8. Risks

| Risk | Answer |
|---|---|
| B1 changes the eaters' energy budget (their bare burning of matter is 5 to 8 times their upkeep, PR-A) | PR3 measures variant (a) and (c) apart; the eaters' key numbers are read and reported; variant (c) is the fallback |
| B1 moves the land as P1.2 did (-0.78 K, not explained) | P1.2 changed many things at once, and its prototype put its own shift on its heat stage (`design/no-rot.md` 3); PR3 says whether the bare reactions move the land and names the cause of any shift over 0.3 K before B1 builds; the climate keys must hold |
| S50 (stored share) rises when stored sugar no longer burns bare | reported in PR3 and B1; a known failure, its cause said |
| A2 loses the gene variety that lets plants adapt to places | its check: the genes' spread at year 30 and kinds by climate; A2 is a small rule, deleted if C lands |
| C fails its gate late, after A's tiny forms | A3 and A4 are written in phase 2; the round still gives about a fifth (section 1) |
| E1 loses the rain belts or the heat carried to the poles | air nodes stay one block tall (the rows of latitude kept); zonal checks; E2 as the alternative |
| E1 makes too few domains for 4 threads | PR7 measures the busiest share; its line 0.3 |
| D2 at level 4 slows learning or trails | the learning scene and the social columns must hold |
| The machine is shared: wall times swing | 1T instructions judge most steps; wall time under the lock, interleaved, the load said |
| Lanes collide in golden and the snapshot | section 5's order; one merge at a time |

## 9. Lines

Core at the round's start: 10,443 code lines (X88), ceiling 10,643. Estimates: F 0 (tape and cli only); B1 about -5,
B2 about +10; A1 0, A2 about +20; D1 about -10 to +10, D2 about +20; E1 about +40 (blocks of two sizes, the snapshot);
C from `design/cover.md`, expected to delete more than it adds (stands, `copy::select`, much of `nodes.rs` for plants).
Without C about +90 lines; within the ceiling. A step that must pass it asks the architect (L4).

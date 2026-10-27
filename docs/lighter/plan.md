# Lighter world: the plan

Status: **planned, after the first review** (`decisions.md`, "Answers to the first review"). Branch `speed/lighter`
(from `speed/x10` at `9656b89`). This file is the one source of the steps: each row has its files, check level,
done-when and kill line. The designs are in `design/` (one note per idea; `cover.md` for idea 5 is its designer's, and
lane C's table below waits for its new version). Decisions: `decisions.md` (L1 and on; the owner's words are L17 and
L18). Measures: `measures.md` (made by the first step that measures). History: `docs/history.md`, section 7.

## 1. The goal, and what to expect

**The goal:** compute less. The last round ended with this finding: most of a year goes to the background (plants 37% /
19% / 60% of a year on one thread on island22 / globe36 / region24 by the old benches, the air 11% / 17% / 9%, the
tape of a long run 37 to 47 ms of a 313 to 709 ms year), not to the story (animals that choose). This round changes
**what** we compute. The owner said yes to all seven ideas, the law changes too (L3), and then said: several plant
kinds per cell that do not mix are a must for the cover (L17), and idea 2 waits for the cover (L18). What must stay is
the goal of the game (handover 1, 6, 13): animals with brains, trails, later people, settlements and trade; a climate
with seasons, rain and rivers; plants that are food, cover and fuel and that adapt to their places.

**What the round can give** (estimates; R0 measures the shares again on `benchZ`, the saves made at `cb6db47`, snapshot
49, and the review after phase P restates this table). One thread, instructions of a settled year
(`schedule::run_for`); "x" is faster by. On `benchZ`, `survey::update` (every body's update) is 61.7% / 50.8% / 66.7%
of `run_for`; the bodies' reactor called from their day 6.4% / 5.0% / 7.0%; the parts' reactor 0 / 4.2% / about 0.

| Idea | island22 | globe36 | region24 | How it is counted |
|---|---|---|---|---|
| 1 saves (lane F) | 0 | 0 | 0 | the benches write no tape; **long runs: about -8 to -10% from the tape's rewrite (F3), at most -2% from the snapshots (F2)** |
| 2 fewer stands a cell | 0 now | 0 now | 0 now | waits for the cover (L18); A3 only if the cover fails |
| 3 living bodies do not rot (B1, variant (d)) | -3 to -5% | -3 to -4% | -3 to -5% | most of the bodies' reactor runs bare reactions (P0: 90% of a stand's, all of an animal's); under (d) the frozen rule skips the decay in a living body |
| 4 a cheaper sample | 0 now | 0 now | 0 now | the cover sets the sample's cadence (L15); A4 only if the cover fails |
| 5 the cover (C) | from `cover.md` | from `cover.md` | from `cover.md` | its designer restates the cost (an adult-node update a cover, K covers a cell); the gate is on the whole year |
| 6 animals look around less (D) | about -0.5% (D1, likely not built) | the same | about 0 | `take_node`'s payer is about 16% of a take (2.5 of 16.0 thousand); D2 after the cover |
| 7 a coarser air on globes (E1) | 0 | -1 to -2% (1T) | 0 | the band solve is 1.4% of globe36's year, the rest of the Newton part 2.3%; more on large globes; 4 threads may lose it (fewer domains) |
| parts' reactor (lane G, the review's find) | 0 | -2 to -3% | about 0 | 191 thousand instructions a call, about 10 times a ground node's; PR8 says why |

| Case | island22 | globe36 | region24 | long runs |
|---|---|---|---|---|
| without the cover (B1, E1, G1, F) | about x1.03 to x1.05 | about x1.06 to x1.1 | about x1.03 to x1.05 | about 10% more |
| with the cover | the cover's own estimate (`cover.md`), times about x1.03 | the same, times about x1.08 | the same, times about x1.03 | about 10% more |

In the "with the cover" row B1 counts on the animals only: the cover cuts the stands' reactor itself. On 4 threads the
gains are smaller where the work cut ran on the pool. **Plainly:** without the cover this round gives a few percent on
the benches and about 10% on long runs; the cover is the round.

## 2. Rules for this round

All of `CLAUDE.md` and of the round's brief (kept outside the repository, beside the scratch files) holds. The rules
that change land with the step that makes them true (section 7).

- **Check levels:** 1 bit for bit; 2 the same to rounding; 3 the same world (key numbers, pace and batch tests, the
  nodes check when a step touches nodes or minds); **4 a new law for a part of the world**: each level-4 row says what
  must still hold and which tests change. Never loosen a tolerance; a test of a deleted mechanism is deleted; a test of
  a changed law is rewritten to state the new law.
- **Kill lines and worth** (L2): a kill line asks for at most 10% of the year (for lane F, of a long run's year). A
  level-1 step of 20 core lines or less is kept at 1% of the year or more; a law step (level 3 or 4) needs about 3% on
  its target worlds, as each costs a key-number cycle and new references for every lane. A step under its kill line is
  reverted and kept on a branch `lighter/<id>-not-kept`, and its alternative (section 3) starts.
- **References** (L10): the start's key numbers are those of `docs/speed-x10/measures.md` "Z: the final checks" 1.
  After each level-4 law lands, the key-number test runs once more and its values become the reference. **Every
  level-4 step rebased on a moved `speed/lighter` runs the key numbers again** before it merges. S50 and S84 stay the
  known failures; a step must not move them past their seeds' noise unless it says why. If B1 moves the eaters out of
  their range, the architect takes new eater references with the cause said (L19).
- **How a gain is judged** (L11). One thread: callgrind instructions of a year (`RAYON_NUM_THREADS=1`). Four threads:
  wall time under `flock $L/timing.lock`, 5 or more interleaved runs, medians, the load said. A step that keeps the
  saved world runs `benchZ` as it is; one that changes it is judged on one world (a turned save, per update; X50) or by
  counted work on same-seed worlds (E1); across remade worlds only large cuts count (PR-S: 19% island22, 11% globe36).
  **Long runs count wall time** (L8).
- **Agents.** Law steps (level 4) and every adversarial review go to the strongest agents; normal code to normal
  agents; measures to measuring agents. Each step in its own worktree and branch; the architect merges one at a time;
  a step that re-pins golden rebases and re-pins on `speed/lighter`; `SNAPSHOT_VERSION` goes to the highest plus one at
  each merge.
- **The three rules for smart bodies hold:** individuals stay real; the step per place (each domain owns its bodies and
  its ring); two kinds of body code (code paths, not two physics).
- **Core lines** (L4): the ceiling is 10,643; each lane has a budget (section 9). A step may pass the ceiling only with a
  decision that says how many lines and why.

## 3. The owner's seven ideas: what we build, and the alternative if it is dropped

| # | Idea | What we build | If it fails: the alternative |
|---|---|---|---|
| 1 | Cheaper saves | F1 the save timed; F3 the tape's rewrite cheaper (csv appended, viewer file at gzip fast); F2 a writer thread only if the saves are still over 2% of a long run's wall time (L13) | Snapshots every 20 years and at the end, the owner told (handover 7.3, 8.3) |
| 2 | Fewer stands a cell | Nothing before the cover (L18); the cover bounds the kinds a cell holds | A3 if the cover fails its gate (PR2's counts are in C0) |
| 3 | Living bodies do not rot | B1, variant (d) (L9): reactions 1 and 21 written as 26 already is (a negligible bare rate, the litter decay in `open`); no body code changes | Variant (b) (only the rot stops, by body code with a stated heat-damage law); else not built and the owner told |
| 4 | A cheaper sample member | Nothing before the cover (L15): the cover sets the sample's cadence | A4 if the cover fails its gate (the sample runs only the stages its gene reads) |
| 5 | Plants as a cover | C0, then C1 to Cn from `design/cover.md`: the bound on kinds first, then the stages | A3 and A4 (stands stay bodies) |
| 6 | Animals look around less | D1 a cheaper take, only if PR6 finds its payer over 1% of the year (L14); D2 a cheaper view, after the cover's gate | D2; the stands' faces go with the cover |
| 7 | A coarser air on globes | E1: air nodes wider than tall on globes (rows of latitude kept), gated by PR7 on 1 and 4 threads (L12) | Lane G (the parts' reactor), the globe's other background cost |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. The design note of each idea holds
the detail.

### Phase P: probes (in parallel; branches `probe/<id>`, never merged)

| ID | What | Done when | Gate it decides | Detail |
|---|---|---|---|---|
| R0 | The start on `benchZ`: P0's table again (plant updates alone and all included, stands a cell, young and adult, the bare share, `nodes::add` and merges), with `#[inline(never)]` on the parts the probes name; the untimed parts of a run (`save_snapshot`, `render::save`, `check::broken`); the 4T year of `$L/ew-base` (5 sets) | a table in `measures.md` "R0"; section 1 restated | the base of every step | this plan |
| PR1 | The tape and the saves by part: `Recorder::finish` by file (the viewer's JSON, its gzip, the frames, `series.csv`, `months.csv`) and the snapshot (bincode, gzip, write), how each grows over a 300-year island run (carried to 1,000); the share of a long run's wall time | its table | F3's parts; whether F2 is needed | `design/saves.md` |
| PR3 | Bare reactions in living bodies on `benchZ` at the gene's pace: by reaction, in stands and animals, the moles, heat and gas a year against the world's flows; the eaters' self-burning against their upkeep (PR-A's number again); variants (d), (a), (b) on 3 seeds with the key numbers; `denature` against each variant's factors; dry litter's rot under (d); dormant bodies; the cost of a stand and an animal update | its table | B1's variant and numbers (L9) | `design/no-rot.md` |
| PR6 | A decision and a habit by stage (perceive, inputs, think, learn, walk, options, pick, spread, the take); `take_node` by part (`wanted`, `intake`, `body::pay`: `Payer::at` and `spend`, the ground's heat); faces walked per decision by kind | its table | D1 (payer over 1% of the year), D2 | `design/perception.md` |
| PR7 | The air on globe36 and the size-28 globe, 4 x 4 against 6 x 4 and 8 x 4: the air's instructions by part, Newton rounds a step, the band solve; the 4T year and the busiest domain's share; the globe test; globe36 with life, 3 seeds, 30 years: zonal rain and land temperature by row | its table | E1 | `design/air.md` |
| PR8 | The parts' reactor on globe36: per call the sub-steps, the halvings (`MAX_DT_K`), the reactions that run and the one that limits, what a part holds; calls a year | its table | G1 | `design/parts.md` |
| C0 | The cover's probe, from `design/cover.md`, with PR2's counts folded in (stands a cell by stage, alike groups, growth forms that live side by side, merges and `nodes::add`'s instructions) | as there | the cover's kinds and stages | `design/cover.md` |

**Review after phase P** (adversarial, then fixed, until accepted): section 1 restated from R0 and the probes; each
gate said go or no go; a dropped idea gets its alternative started.

### Phase 1: six lanes side by side

Each lane is its own part of the code, its own agent and worktree; the lanes merge one at a time.

**Lane F: the saves** (`tape/src/rec/`, `tape/src/snapshot.rs`, `cli/src/main.rs`; `design/saves.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| F1 | `save_snapshot` and `render::save` timed in the tape's column (they land in the next year's row, `docs/tape.md` says so); `measures.md` Z 3's sentence corrected | 1 | golden unchanged; a run's "per year" holds its saves | none |
| F3 | The tape's rewrite cheaper: `series.csv` and `months.csv` appended (a resumed run cuts them to the world's day), the viewer's file at gzip `fast`, the rest PR1 names | 1 (the files hold the same rows) | tape tests (resume equals unbroken); the tape's column of a 300-year island run, carried to 1,000 years, -60% | a 1,000-year island run's wall time a year under -4% |
| F2 | Only if the saves and the tape are still over 2% of a long run's wall time after F3: the compression and the write on one writer thread (bytes made on the main thread, one job in flight, `.part` then rename) | 1 | as F3; the main thread's share under 1% | under -1% of a long run's wall time a year |

**Lane B: living bodies do not rot** (`worldgen/src/chem0.toml`, `worldgen/src/life/make.rs`; `design/no-rot.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| B1 | **A law, variant (d):** reactions 1 and 21 written as 26 is: their bare `k` divided by `X` (PR3's number; 26 uses 1e6) and their `open` times `X`, so the open world's decay is unchanged and a living body's decay falls under the frozen rule; the founders' catalyst genes of 1 and 21 raised by log10 `X` (`LITTER_X`'s rule), so a body's catalysed burning is unchanged. No body code. `numbers.md` rows of 1, 21 and their `open`; `laws.md` 2.1, 2.4, 6.8, 6.9; handover 13.2's "inside bodies" item closed; snapshot bumped (the chemistry is saved) | 4 | what must hold, below; golden re-pinned; a stand update and an animal update 1T -10% on a turned save | the year 1T under -3% on island22, or a climate key out: variant (b) or not built |
| B2 | Only if PR3 shows the reactor's setup on a body with nothing to run still over 1% of the year after B1: `Reactor::react` returns at once when no reaction can run (inside the reactor, not a second end of `physics::react`) | 1 on B1's world | an animal update 1T -3% | under -1% of the year |

**B1 must still hold:** the climate keys (land, swing, rain, gas, rivers, worn land) and the light-eaters' two in range;
the eaters' two in range, or new references taken with the cause said (L19: today their bare self-burning is 5 to 8
times their Kleiber upkeep, an artifact of a litter constant run inside them); S50 reported with its cause; the
long-step test; the pace and batch tests; the nodes check; `the_eaters_live_and_stop_growing`;
`a_century_keeps_its_books`; globe36 with life (3 seeds, 30 years) for the climate. **Watch:** eaters do not eat most
of the plants' production (handover 13.2); bodies with activity under 1% that outlive a few of their kind's lives
(counted at year 30 and in the 1,000-year runs); dry litter (moisture 0) now decays slowly: the gas 3 from dry land.
**Tests that change:** `a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses` (its heat made counts no
bare burning); `chemistry0.rs` and `numbers.rs` where they read the rows of 1 and 21; a new test
`a_living_body_runs_no_decay_but_its_own_catalysts` (a body with its burning genes off keeps its matter and solid
over a year at 300 K; the same matter on moist ground rots at its litter constant; at fire's heat it burns).

**Lane C: plants as a cover** (from `design/cover.md`; its designer is rewriting it, and the architect sends the new
table. Order: C0, then **the bound on kinds first** (on today's young-adult pairs), **then the stages**, each its own
law step with its gate on the whole year)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| C0 | The cover's probe (phase P) | none | from `design/cover.md` | none |
| C1 to Cn | from `design/cover.md` | 4 | from `design/cover.md` | from `design/cover.md` |
| **G-C** | **The cover's gate** (placeholder, from `design/cover.md`): its checks pass; the whole year and the animal updates judged | | | **fails:** C is not merged; A3 and A4 start (phase 2) |

B1 and the cover are independent (L5): they land in any order; the second re-runs the key numbers against the first's
reference. The cover's tests of its own chemistry are written after B1 if B1 lands first.

**Lane D: animals look around less** (`core/src/choose/`; `design/perception.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| D1 | Only if PR6 finds the take's payer and repeats over 1% of the year: the part of `take_node`'s payment that does not change since the upkeep (the cover over the body, its depth) read once per update; what PR6 names in `intake` | 1 | golden unchanged with and without threads | the year 1T under -1% |
| D2 | After G-C (most faces are stands today, L14): the cheaper view PR6 names (a face's own features once a batch; or, if the cover fails, stands seen through the cell's features) | 2 or 4 | `choose::act` 1T -20% | the year 1T under -2% |

**D2 at level 4 must still hold:** the key numbers; the nodes check; `body_learns_to_move_toward_what_it_can_take`;
`selection_moves_a_nodes_brain_size`; the `social` columns of the tape within their seeds' noise (watch-for M2).

**Lane E: the air on globes** (`core/src/space/square.rs`, `worldgen/src/terrain.rs`, `core/src/world.rs`; `design/air.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| E1 | Air nodes wider than tall on a globe: blocks of `sx x 4` squares, `sx` the largest divisor of the row from 5 to 8 (globe36 6, size 28 and the test globe 8), else 4 x 4; the island and regions keep 4 x 4. `World::air_scale` a pair; snapshot bumped; handover 7.2, 7.4; `laws.md` 1.4 | 4 | what must hold, below; counted work on same-seed worlds: the air's instructions a year -8% on globe36, -15% on the size-28 globe; the 4T year not slower on globe36 | globe36 4T slower, or the size-28 globe's air under -5% |

**E1 must still hold:** `a_globe_without_life_keeps_an_earth_like_climate`; globe36 with life, 3 seeds, years 21 to 30:
land temperature and rain on land within their seeds' noise, zonal rain and land temperature by row within 1 K and 15%,
ice within its noise; the busiest domain's share on 4 threads at most 0.3; watch-for M0 (rain belts, deserts behind
mountains, ice at the poles). **Tests that change:** `core/tests/space.rs` gains blocks of two sizes;
`core/tests/flow.rs:179` reads the air level's width from the mesh.

**Lane G: the parts' reactor** (`core/src/bond/month.rs`, `core/src/change/kernel.rs`; `design/parts.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| G1 | The cut PR8 names (for example a part's heat stage that halves its sub-steps far less often, or its six months run as Change runs a ground node); level by the probe | 1 to 3 | the parts' reactor 1T -50% on globe36 | globe36's year 1T under -1.5% (a law form: under -3%) |

**Lane reviews.** After each lane's last step: an adversarial review, then a fixer, until accepted (section 6).

### Phase 2: after the cover's gate

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A3 | Only if G-C fails: stands of a full cell merge past `crowd` whatever their genes (or C0's smaller rule); merges and `nodes::add` counted | 4 | as `design/stands.md` 4; island22 1T -5% | island22 1T under -3% |
| A4 | Only if G-C fails: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6% | under -3% |
| D2 | As lane D, after G-C | | | |

### Phase Z: the end

1. The year at the round's end against its start: `$L/ew-base` on `benchZ` and the end's build on benches made with
   its own code (`make_benches.sh`), 6 interleaved sets, 4 threads and 1 thread, under the lock; medians. And 1T
   instructions of a year on both.
2. 1,000-year runs (seed 1; island 22, globe 18, region 24 with `--region 8,8,8,8`), one at a time, wall time, the
   default pictures and saves, **for both `$L/ew-base` and the end's build** (the last report has wall times for the
   globe only).
3. The largest globe that runs 1,000 years in 15 minutes of wall time (start at size 28, then up).
4. The slow suite (`cargo test --release --workspace --no-fail-fast -- --ignored`), the golden test without threads,
   the debug checks (books, overdraw, views; the plain debug golden).
5. The report `docs/reports/lighter.md` (as `docs/reports/speed-x10.md`).
6. The docs at the round's end: the `CLAUDE.md` row "Lighter world (done)"; handover 7.1, 7.3, 13.2, 14.3 (the core
   ceiling: the lines then plus 200, L4); `laws.md`, `numbers.md` and `tape.md` as they are; every row of this plan says
   kept, not kept or not built.
7. The final adversarial review, then fixes, until accepted.

## 5. Order and lanes

```
Phase P   R0 ─ PR1, PR3, PR6, PR7, PR8, C0 (side by side) ─► review P (section 1 restated, gates)
Phase 1   F: F1 ─ F3 ─ (F2) ──────────────────────────────────┐
          B: B1 ─ (B2) ───────────────────────────────────────┤
          C: C1 to Cn: the bound on kinds, then the stages ─ G-C ┤
          D: (D1) ────────────────────────────────────────────┼─► lane reviews
          E: E1 ──────────────────────────────────────────────┤
          G: G1 ──────────────────────────────────────────────┘
Phase 2   G-C failed: A3, A4, D2 (stands seen through the cell) ;  G-C passed: D2
Phase Z   the end (section 4) ─► final review
```

**Where steps collide, this order holds:**
- B1 and the cover's steps are all level 4 and move the plants: they merge one at a time, in any order; the second
  re-runs the key numbers against the reference the first set (L5, L10).
- B1 changes the chemistry's data and the founders' genes (worldgen); the cover changes the bodies' code: no shared
  file but `numbers.md`, `laws.md` and golden.
- D1 and the cover both touch `choose/feed.rs`: D1 (level 1) lands first if it is built; the cover rebases.
- E1 and the cover both bump the snapshot; G1 and B2 both touch `change/kernel.rs`: one at a time.
- F touches only `tape/` and `cli/`; it lands any time (level 1).
- Merges into `speed/lighter` one at a time; golden re-pinned by the step that changes it; the snapshot version the
  highest plus one. If `main` moves, it is merged in at the next lane review (L1).

## 6. Reviews

The cycle is: plan, adversarial review, implement, adversarial review, fix, until accepted. Review rounds sit:
1. **After this plan** (done once: `decisions.md`, "Answers to the first review"; again when `cover.md` is merged).
2. **After phase P:** section 1 restated, gates go or no go.
3. **After each lane's last step** (F, B, D, E, G): the lane's diffs, tests and measures against its rows; each review
   of a level-4 lane runs the key-number test against the reference.
4. **At the cover's design** (after C0) **and at G-C:** the cover's law, its checks and its measure.
5. **After phase Z:** the report and the docs.

Each review is adversarial (books, overdraw, honesty of the law, `CLAUDE.md`'s rules, measures that do not prove the
claim) and ends with a verdict; a fixer answers each finding; the review runs again until it says accepted.

## 7. Rule and doc changes, and the step that writes each

| Rule or doc | Lands with |
|---|---|
| `CLAUDE.md` table row "Lighter world (in progress)" | the first merged step |
| `docs/tape.md` (a run's year holds its saves and pictures, in the next year's row; how the tape is written); `measures.md` of the last round, Z 3's sentence | F1, F3, F2 |
| `laws.md` 2.1 (a living body runs no decay but its own catalysts), 2.4, 6.8 (heat burns matter at fire's heat), 6.9; handover 13.2 "Inside bodies" closed; `numbers.md` reactions 1, 21 and their `open`, `LITTER_X` | B1 |
| handover 7.1, 7.3; `laws.md` 4, 6 (what a plant is) | C (from `design/cover.md`) |
| `laws.md` 5.2 (what a body perceives), if D2 is a law | D2 |
| handover 7.2 and 7.4 (air nodes on globes, domains); `laws.md` 1.4 | E1 |
| `laws.md` 3 (parts' reactions), if G1 changes them | G1 |
| `CLAUDE.md` row "Lighter world (done)"; handover 14.3 and `CLAUDE.md` 2.11 (the ceiling) | Z |

## 8. Risks

| Risk | Answer |
|---|---|
| B1 changes the eaters' energy budget (their bare self-burning is 5 to 8 times their upkeep, PR-A, an artifact) | PR3 measures it again on `benchZ`; the eaters' references may be re-taken with the cause said (L19); the plants must not be eaten out |
| B1 moves the land as P1.2 did (-0.78 K, not explained) | PR-M's prototype ran stands with no bare reactions and moved the land +0.13 K (v1); PR3 reads 3 seeds and hunts reaction by reaction only if the mean moves past the seeds' noise |
| B1 under (d) leaves dry litter rotting slowly (the open world's bare rate falls too) | PR3 reads the gas 3 from dry land; if it matters, (d) is set with a smaller `X` |
| A catalyst factor `X` times larger unfolds more steeply under `denature` (it works on log f) | PR3 checks a hot body's catalysed rate under (d) against today's; a heat-killed enzyme now leaves almost nothing, not the bare rate |
| A dormant body (activity near 0) never dies once nothing burns it bare | the watch in B1's checks |
| S50 rises when stored solid no longer burns bare | reported in PR3 and B1, with its cause |
| The cover fails its gate late | A3, A4, D2 in phase 2; the round then gives a few percent (section 1) |
| E1 loses the rain belts or the heat to the poles | the rows of latitude are kept; zonal checks on globe36 with life |
| E1 costs more on 4 threads (45 domains become 30 on globe36) than it saves | E1's done-when asks the 4T year not slower; PR7 measures it first |
| The machine is shared: wall times swing | 1T instructions judge most steps; wall time under the lock, interleaved, the load said |
| Lanes collide in golden and the snapshot | section 5's order; one merge at a time; key numbers again after a rebase |

## 9. Lines

Core at the round's start: 10,443 code lines (X88; the cover's designer counted 10,444; R0 counts it), ceiling 10,643.
Budgets by lane: F 0 (tape and cli only); B1 0 (data and worldgen), B2 at most +10; D1 at most +10, D2 at most +20; E1
at most +40; G1 at most +30; the cover from `design/cover.md` (its first version said net about +75 to +95; it deletes
little, as `copy::select` stays on each cover). Without the cover about +100; with it about +190, past the ceiling: the
cover may pass it only with a decision that says how many lines and why (L4), and the lanes that land after it must
fit what is left or ask.

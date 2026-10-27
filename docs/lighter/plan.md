# Lighter world: the plan

Status: **planned, after the first review** (`decisions.md`, "Answers to the first review"). Branch `speed/lighter`
(from `speed/x10` at `9656b89`). This file is the one source of the steps: each row has its files, check level,
done-when and kill line. The designs are in `design/` (one note per idea; `cover.md` for idea 5 is its designer's; lane C's
table below is its section 7). Decisions: `decisions.md` (L1 and on; the owner's words are L17 and L18; the cover's decisions D1 to D11 are in `cover.md` 9 and logged as L23
to L28 and L30). Measures: `measures.md` (made by the first step that measures). History: `docs/history.md`, section 7.

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
| 3 living bodies do not rot (B1, by PR3 among (b) and (d')) | 0 to -5% | 0 to -4% | 0 to -5% | most of the bodies' reactor runs bare reactions (P0: 90% of a stand's, all of an animal's); a gain needs (b)'s body code or (d')'s frozen test on the real rate: today's frozen rule skips nothing in a living body at any `X` that fits the genes (L9) |
| 4 a cheaper sample | 0 now | 0 now | 0 now | the cover sets the sample's cadence (L15); A4 only if the cover fails |
| 5 the cover (C1, C2) | **x1.20 to x1.28** (C1 alone x1.05 to x1.12) | **x1.03 to x1.09** (C1 about x1.0) | **x1.49 to x1.58** (C1 alone x1.14 to x1.23) | `cover.md` 5: `cell_kinds` 3, so at most 4 kinds a cell; a cover update is an adult-node update (about 100k instructions), 3 to 4 covers a planted cell on the island (2 to 3 globe, 3.5 to 4 region) against 8.2 / 4.6 / 10.3 stands; the outbox litters go by copying updates and rise on the globe; plants all included 37% / 19% / 60% of the year fall to 15 to 21% / 11 to 16% / 23 to 27% |
| 6 animals look around less (D) | about -0.5% (D1, likely not built) | the same | about 0 | `take_node`'s payer is about 16% of a take (2.5 of 16.0 thousand); D2 after the cover |
| 7 a coarser air on globes (E1) | 0 | -1 to -2% (1T) | 0 | built only if it gives 3% of the year on the size-28 globe (L12); the band solve is 1.4% of globe36's year, the rest of the Newton part 2.3%; 4 threads may lose it (fewer domains) |
| parts' reactor (lane G, the review's find) | 0 | -2 to -3% | about 0 | 191 thousand instructions a call, about 10 times a ground node's; PR8 says why |

| Case | island22 | globe36 | region24 | long runs |
|---|---|---|---|---|
| without the cover (B1, E1, G1, F) | about x1.0 to x1.05 | about x1.03 to x1.1 | about x1.0 to x1.05 | about 10% more |
| C1 alone (C2 fails its lines) | about x1.08 to x1.17 | about x1.06 to x1.1 | about x1.17 to x1.28 | about 10% more |
| **with the cover (C1 and C2)** | **about x1.21 to x1.33** | **about x1.07 to x1.18** | **about x1.50 to x1.62** | about 10% more |

In the "with the cover" row B1 counts on the animals only (about 9% of the region's year), and B1's part is 0 if PR3
finds no candidate; the cover cuts the stands' reactor itself. On 4 threads the
gains are smaller where the work cut ran on the pool. **Plainly:** without the cover this round gives a few percent on
the benches and about 10% on long runs; with it about a quarter to a third on the island, a tenth to a fifth on the
globe and about 50 to 60% on the region. The next lever after the cover is the cost of making children, a third of a cover
update (probe C4, phase 2, L30).

## 2. Rules for this round

All of `CLAUDE.md` and of the round's brief (kept outside the repository, beside the scratch files) holds. The rules
that change land with the step that makes them true (section 7).

- **Check levels:** 1 bit for bit; 2 the same to rounding; 3 the same world (key numbers, pace and batch tests, the
  nodes check when a step touches nodes or minds); **4 a new law for a part of the world**: each level-4 row says what
  must still hold and which tests change. Never loosen a tolerance; a test of a deleted mechanism is deleted; a test of
  a changed law is rewritten to state the new law.
- **Kill lines and worth** (L2): a kill line asks for at most 10% of the year (for lane F, of a long run's year). A
  level-1 or level-2 step of 20 core lines or less is kept at 1% of the year or more, of 21 to 40 lines at 1.5%; a law
  step (level 3 or 4) needs about 3% on its target worlds, as each costs a key-number cycle and new references for every lane. A step under its kill line is
  reverted and kept on a branch `lighter/<id>-not-kept`, and its alternative (section 3) starts.
- **References** (L10): the start's key numbers are those of `docs/speed-x10/measures.md` "Z: the final checks" 1.
  After each level-4 law lands, the key-number test runs once more and its values become the reference. **Every
  level-4 step rebased on a moved `speed/lighter` runs the key numbers again** before it merges. S50 and S84 stay the
  known failures; a step must not move them past their seeds' noise unless it says why. If B1 moves the eaters out of
  their range, the architect takes new eater references with the cause said (L19).
- **How a gain is judged** (L11). One thread: callgrind instructions of a year (`RAYON_NUM_THREADS=1`). Four threads:
  wall time under `flock $L/timing.lock`, 5 or more interleaved runs, medians, the load said. A step that keeps the
  saved world runs the newest benches as they are; one that changes it is judged on one world (a turned save, per
  update; X50) or by counted work on same-seed worlds (E1, the cover); across remade worlds only large cuts count
  (PR-S: 19% island22, 11% globe36). **Long runs count wall time** (L8).
- **Benches** (L11): the step that bumps `SNAPSHOT_VERSION` remakes the three benches with its own build
  (`make_benches.sh`, `$L/bench<version>`, about 7 minutes) as part of its merge and names the folder in `measures.md`;
  later steps use the newest set, base build and step build both run from it. Until the first bump it is `benchZ`.
- **Agents.** Law steps (level 4) and every adversarial review go to the strongest agents; normal code to normal
  agents; measures to measuring agents. Each step in its own worktree and branch; the architect merges one at a time;
  a step that re-pins golden rebases and re-pins on `speed/lighter`; `SNAPSHOT_VERSION` goes to the highest plus one at
  each merge.
- **The three rules for smart bodies hold:** individuals stay real; the step per place (each domain owns its bodies and
  its ring); two kinds of body code (code paths, not two physics).
- **Core lines** (L4, L31): the ceiling is 10,643; each lane has a budget (section 9). The step that would pass it
  raises `CEILING` in `cli/tests/core_size.rs`, `CLAUDE.md` 2.11 and handover 14.3 in the same commit, by the lines it
  needs, with a decision logged that gives the number; phase Z sets the round's ceiling.

## 3. The owner's seven ideas: what we build, and the alternative if it is dropped

| # | Idea | What we build | If it fails: the alternative |
|---|---|---|---|
| 1 | Cheaper saves | F1 the save timed; F3 the tape's rewrite cheaper (csv appended, viewer file at gzip fast); F2 a writer thread only if the saves are still over 2% of a long run's wall time (L13) | Snapshots every 20 years and at the end, the owner told (handover 7.3, 8.3) |
| 2 | Fewer stands a cell | Nothing before the cover (L18); the cover bounds the kinds a cell holds | A3 if the cover fails its gate (PR2's counts are in C0) |
| 3 | Living bodies do not rot | B1's law chosen by PR3 among (b) (the decomposers' reactions do not run in a living body, by body code, with heat damage kept by a stated law) and (d') (the decay in `open`, a frozen test on the real rate, the two-state `denature`), then a design review of lane B (L9) | "Not built": the owner told, handover 13.2's item stays open; the reactor made cheaper for what bodies run (B2, G1) |
| 4 | A cheaper sample member | Nothing before the cover (L15): the cover sets the sample's cadence | A4 if the cover fails its gate (the sample runs only the stages its gene reads) |
| 5 | Plants as a cover | C0, then C1 (the bound on kinds, on today's young-adult pairs), then C2 (the young inside the cover), C3 the checks, gate G-C (`design/cover.md` 7) | C1 alone if C2 fails its lines; A3 and A4 if C1 fails too (stands stay bodies) |
| 6 | Animals look around less | D1 a cheaper take, only if PR6 finds its payer over 1% of the year (L14); D2 a cheaper view, after the cover's gate | D2; the stands' faces go with the cover |
| 7 | A coarser air on globes | E1: air nodes wider than tall on globes (rows of latitude kept), built only if it gives 3% of the year on the size-28 globe (counted) and the 4T year is not slower (L12) | Lane G (the parts' reactor), the globe's other background cost |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. The design note of each idea holds
the detail.

### Phase P: probes (in parallel; branches `probe/<id>`, never merged)

| ID | What | Done when | Gate it decides | Detail |
|---|---|---|---|---|
| R0 | The start on `benchZ`: P0's table again (plant updates alone and all included, stands a cell, young and adult, the bare share, `nodes::add` and merges), with `#[inline(never)]` on the parts the probes name; the untimed parts of a run (`save_snapshot`, `render::save`, `check::broken`); the 4T year of `$L/ew-base` (5 sets) | a table in `measures.md` "R0"; section 1 restated | the base of every step | this plan |
| PR1 | The tape and the saves by part: `Recorder::finish` by file (the viewer's JSON, its gzip, the frames, `series.csv`, `months.csv`) and the snapshot (bincode, gzip, write), how each grows over a 300-year island run (carried to 1,000); the share of a long run's wall time | its table | F3's parts; whether F2 is needed | `design/saves.md` |
| PR3 | Bare reactions in living bodies (running now): on `benchZ` at the gene's pace, by reaction, in stands and animals, the moles, heat and gas a year; the eaters' self-burning against their upkeep (PR-A's number again); **the frozen points** of 1, 21, 26 under today's rule and under (d')'s test on the real rate, `X` from 1e6 up; **`denature`**: today's form against the two-state form on the eaters (at 310 K, at `COOL_T`, the share of eater updates above `DENATURE - 10 K`); **rot by moisture class** (the ground's dead matter and the mineral 24 on land for m 0 to 0.2, 0.2 to 0.5, 0.5 to 1; the parts' walls' life on globe36); **bodies with genes of 1 or 21 between 0 and log10 `X`**; the cost of a stand and an animal update; (b) and (d') on 3 seeds with the key numbers, dormant bodies, S50; globe36 with life | its table | B1's law: (b), (d') or not built, then the design review of lane B (L9) | `design/no-rot.md` |
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

**Lane B: living bodies do not rot** (`design/no-rot.md`): **B1's law chosen by PR3 among (b) and (d'), then a
design review** of lane B (L9). Files by the choice: (b) `core/src/body/physics.rs`; (d') `worldgen/src/chem0.toml`,
`worldgen/src/life/make.rs`, `core/src/change/kernel.rs`.

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| B0 | The design review of lane B after PR3: it reads PR3's table and chooses (b), (d') or "not built", with `X`, the skip share and the heat-damage law where they apply; `no-rot.md` rewritten to the choice | none | the review's verdict; L9 rewritten to the choice | none |
| B1 | **A law, as B0 chose.** (b): in `body::physics::react` a decomposers' reaction (`open > 1`: 1, 21, 26) gets factor 0 when its gene is off, a catalysed burning 0 (it runs on demand); 22, 23, 25 as today; heat damage by the stated law. (d'): 1 and 21 written as 26 (`k / X`, `open · X`, every body's genes of 1 and 21 raised by log10 `X`), the frozen test on the real rate against a named skip share (a level-2 rule for every node), the two-state `denature` (`1 + (f - 1) / (1 + e^z)`, L32); snapshot bumped (L29). `laws.md` 2.1, 2.4, 6.8, 6.9; handover 13.2's "Inside bodies" item closed; `numbers.md` rows of what changes | 4 | what must hold, below; golden re-pinned; a stand update and an animal update 1T -10% on a turned save | the year 1T under -3% on island22, or a climate key out: not built |
| B2 | Only if worth 1% of the year after B1 (and after G1, L34): `Reactor::react` returns at once when no reaction can run (inside the reactor, not a second end of `physics::react`) | 1 on B1's world | an animal update 1T -3% | under -1% of the year |

**B1 must still hold:** the climate keys (land, swing, rain, gas, rivers, worn land) and the light-eaters' two in range;
the eaters' two in range, or new references taken with the cause said (L19: today their bare self-burning is 5 to 8
times their Kleiber upkeep, an artifact of a litter constant run inside them); S50 reported with its cause; the
long-step test; the pace and batch tests; the nodes check; `the_eaters_live_and_stop_growing`;
`a_century_keeps_its_books`; globe36 with life (3 seeds, 30 years) for the climate. **Watch:** eaters do not eat most
of the plants' production (handover 13.2); bodies with activity under 1% that outlive a few of their kind's lives
(counted at year 30 and in the 1,000-year runs); under (d'): the ground's dead matter and the mineral 24 on land that is
not fully moist, and the parts' walls. **Tests that change:**
`a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses` (its heat made counts no bare burning); a new
test `a_living_body_runs_no_decay_but_its_own_catalysts` (a body with its burning genes off keeps its matter and solid
over a year at 300 K; the same matter on moist ground rots at its litter constant); under (d') `chemistry0.rs`,
`numbers.rs` where they read 1 and 21, and a test of the two-state unfolding; under (b) a test of the stated heat-damage
law.

**Lane C: plants as a cover** (`design/cover.md`, its section 7; decisions L23 to L28, L30). "Counted" = callgrind on one
thread, one year from the benches remade at the step's snapshot (`make_benches.sh`), seeds 1 to 3, with
`--toggle-collect` on the update of a body without a brain: the year, the plant work per planted cell-year, the
planted cells, the animal updates a year, bodies per planted cell (L24). The order: C0 (phase P), **C1 the bound on
kinds on today's young-adult pairs, then C2 the stages**, then C3 the checks and the gate. A number in brackets in the
table is a section of `cover.md`; tests 1 to 3 are its 6.3.

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| C0 | Probe (branch `probe/cover-c0`, not merged), on the three bench saves and one year of each: stands per planted cell by stage; alike groups (adult and young by `body::within`); **growth forms side by side** (groups unlike in the form genes: `COPY_MASS`, `LEAF`, the keep genes of the frame and lime, `STRENGTH`; and the heights `Body::height`); share of adult mass in the largest 1 to 4 groups; litters a planted cell-year that land where no node of their kind is, and the adult mass a year later that came from them; animal nodes per planted cell, animal splits and `merge_closest` runs a year; the spread of the form genes' cell means across the land cells of globe36 (kinds by climate, C1's reference); callgrind of a stand update split by `b.adult()` | a test file on the branch | probe | the numbers in `docs/lighter/measures.md` C0; `cell_kinds` is 3 (decided); if the forms side by side need 4, the architect restates G-C before C1 (4 fails G-C as priced, 5) | none (if the adult update is over 120k, the architect is told before C1) |
| C1 | **The bound on kinds** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the cell holds fewer than `cell_kinds` + 1 kinds, else the litter joins the most alike; `nodes::split` of a light-eater node needs two free places; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (`merge_closest` without the `alike` limit for bodies without a brain); `crowd` unchanged for animals; snapshot 50; test 1 (two invasions) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (and the mean per planted cell measured); test 1 passes; animal nodes per planted cell and animal updates a year within their seeds' noise; key numbers in range (S50 within noise); the climate and life tests of 6 pass; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's); the counted year measured, not gated | **no speed line** (it is the base of C2; G-C judges the speed): animal updates a year or animal nodes per planted cell past their seeds' noise, a key number out of range or S50 past its noise, a climate test failing, test 1 failing, or kinds by climate lost: C stops; G-C fails |
| C2 | **The young inside** (2): `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth split by each stage's energy balance (light share less mass share of upkeep); maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood falling with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage for covers; founders and bodies that lose their brain all adult; snapshot 51; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4 (one law, the flux by `Pass`, with two holders: node pairs for animals, stages for covers), 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; counted plant work per planted cell-year x1.4 against C1 on island22 and region24 (`outbox::deliver` counted with it); light-eater members per planted cell within a factor of 2 of C1's; test 2 passes; animals, key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 (else stop and find why: P1.2's -0.78 K was never explained) | the counted year falls less than 5% against C1 on island22 or less than 8% on region24, or the animal, key-number, climate or kinds lines of C1 fail: C1 stays, C2 is not merged |
| C3 | **The checks** (no code): counted runs of the three benches, plants all included as in P0; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **The cover's gate** (after C3, `cell_kinds` 3): the whole year, one thread, counted, at least -15% on island22 and -30% on region24, and not worse on globe36 (estimate: -16 to -22%, -33 to -37%, -3 to -8%); animal nodes per planted cell and animal updates a year within their seeds' noise of today; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 alone may stay if it passed its own lines); A3 and A4 start (phase 2) |

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
| E1 | Air nodes wider than tall on a globe: blocks of `sx x 4` squares, `sx` the largest divisor of the row from 5 to 8 (globe36 6, size 28 and the test globe 8), else 4 x 4; the island and regions keep 4 x 4. `World::air_scale` a pair; snapshot bumped; handover 7.2, 7.4; `laws.md` 1.4. **Built only if PR7 shows it worth 3% of the size-28 globe's year** (L12) | 4 | what must hold, below; counted work on same-seed size-28 worlds: the year 1T -3%; the 4T year not slower on globe36 and on size 28 | the size-28 globe's year 1T under -3% (counted), or a 4T year slower: not built, lane G is the alternative |

**E1 must still hold:** `a_globe_without_life_keeps_an_earth_like_climate`; globe36 with life, 3 seeds, years 21 to 30:
land temperature and rain on land within their seeds' noise, zonal rain and land temperature by row within 1 K and 15%,
ice within its noise; the busiest domain's share on 4 threads at most 0.3; watch-for M0 (rain belts, deserts behind
mountains, ice at the poles). **Tests that change:** `core/tests/space.rs` gains blocks of two sizes;
`core/tests/flow.rs:179` reads the air level's width from the mesh.

**Lane G: the parts' reactor** (`core/src/bond/month.rs`, `core/src/change/kernel.rs`; `design/parts.md`)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| G1 | The cut PR8 names (for example a part's heat stage that halves its sub-steps far less often, or its six months run as Change runs a ground node); level by the probe; on B1's world if B1 lands first (its walls rot by B1's law, L34) | 1 to 3 | the parts' reactor 1T -50% on globe36 | globe36's year 1T under -1.5% (a law form: under -3%) |

**Lane reviews.** After each lane's last step: an adversarial review, then a fixer, until accepted (section 6).

### Phase 2: after the cover's gate

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A3 | Only if G-C fails: stands of a full cell merge past `crowd` whatever their genes (or C0's smaller rule); merges and `nodes::add` counted | 4 | as `design/stands.md` 4; island22 1T -5% | island22 1T under -3% |
| A4 | Only if G-C fails: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6% | under -3% |
| D2 | As lane D, after G-C | | | |
| C4 | After G-C passes: a probe of the cost of making children (two `vary` calls, the join, the litter: about a third of a cover update), and a law step by it if it is worth 3% (L2, L30) | none, then 4 | its table | none |

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
          B: B0 (design review after PR3) ─ B1 ─ (B2) ─────────┤
          C: C1 (the bound on kinds) ─ C2 (the stages) ─ C3 ─ G-C ┤
          D: (D1) ────────────────────────────────────────────┼─► lane reviews
          E: E1 ──────────────────────────────────────────────┤
          G: G1 ──────────────────────────────────────────────┘
Phase 2   G-C failed: C1 alone if it passed; else A3, A4, D2 (stands seen through the cell) ;  G-C passed: C4, D2
Phase Z   the end (section 4) ─► final review
```

**Where steps collide, this order holds:**
- B1 and the cover's steps are all level 4 and move the plants: they merge one at a time, in any order; the second
  re-runs the key numbers against the reference the first set (L5, L10).
- B1 changes the chemistry's data and the founders' genes; C2 changes the bodies' code and the founders (all adult):
  both touch `worldgen/src/life/make.rs`, `numbers.md`, `laws.md` and golden; the second rebases.
- C1 then C2, each its own law step with its gate on the whole year; C2 takes new key-number references (L26).
- D2 comes after G-C, on the world the cover leaves.
- B1, C1 (snapshot 50 in its design), C2 (51) and E1 bump the snapshot: the number is the highest plus one at each merge.
- `change/kernel.rs` (L34): B1, if it edits the kernel ((d')), lands before G1; B2 after both; each re-pins golden.
  B1 is the base of G's walls: if B1 lands first, PR8's walls' numbers run again on its world before G1 is judged.
- F touches only `tape/` and `cli/`; it lands any time (level 1).
- Merges into `speed/lighter` one at a time; golden re-pinned by the step that changes it; the snapshot version the
  highest plus one. If `main` moves, it is merged in at the next lane review (L1).

## 6. Reviews

The cycle is: plan, adversarial review, implement, adversarial review, fix, until accepted. Review rounds sit:
1. **After this plan** (done twice: `decisions.md`, "Answers to the first review" and "... second review"; `cover.md`
   reviewed and rewritten twice). Lanes C, D, E, F, G and phase P accepted; lane B after its own review (B0).
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
| `laws.md` 2.1 (the decomposers' decay does not run in a living body), 2.4 (under (d'): the two-state unfolding, the genes of 1 and 21 useful from log10 `X`), 6.8 (heat damage: (b)'s stated law, or under (d') ended until fire's own reaction at M4), 6.9; handover 13.2 "Inside bodies" closed; `numbers.md` rows of what changes | B1 |
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
| Under (d') dead matter rots slower on land that is not fully moist (the factor is `1 + (open - 1) m`), which moves the mineral 24 and lime on land and the parts' walls | PR3 reads them by moisture class; B0 weighs it against (b), which keeps the open world as it is |
| A catalyst factor `X` times larger would unfold more steeply under today's `denature` (it works on log f), for every warm body | (d') comes with the two-state `denature`, which does not depend on `f`'s size (L32); PR3 measures it on the eaters |
| A dormant body (activity near 0) never dies once nothing burns it bare | the watch in B1's checks |
| S50 rises when stored solid no longer burns bare | reported in PR3 and B1, with its cause |
| The cover fails its gate late | C1 alone if it passed its own lines; else A3, A4, D2 in phase 2; the round then gives a few percent (section 1) |
| A newcomer kind that never matures holds the newcomer place and blocks invasions | the invasion test and the long runs watch it; if it shows, the place frees when the newcomer has not matured within its own gene's life span (L23) |
| Reproduction genes are selected only between kinds and cells under the cover | accepted (L27); C0's number and the 1,000-year watch |
| E1 loses the rain belts or the heat to the poles | the rows of latitude are kept; zonal checks on globe36 with life |
| E1 costs more on 4 threads (45 domains become 30 on globe36) than it saves | E1's done-when asks the 4T year not slower; PR7 measures it first |
| The machine is shared: wall times swing | 1T instructions judge most steps; wall time under the lock, interleaved, the load said |
| Lanes collide in golden and the snapshot | section 5's order; one merge at a time; key numbers again after a rebase |

## 9. Lines

Core at the round's start: 10,443 code lines (X88; the cover's designer counted 10,444; R0 counts it), ceiling 10,643.
Budgets by lane: the cover C1 about +23 net, C2 about +113 net (`cover.md` 7: net about +136, to about 10,580; it
cannot delete more than it adds, as the node code stays for the animals, L4); F 0 (tape and cli only); B1 (b) about
+10, (d') about +25 (the frozen test, the two-state unfolding), B2 at most +10; D1 at most +10, D2 at most +20; E1 at
most +40; G1 at most +30; C4 by its probe. **The whole plan comes to about +260, about 10,700, past the ceiling by
about 60, and the crossing comes from the lanes after the cover, not from the cover** (about 10,580). The step that
would cross raises `CEILING` (`cli/tests/core_size.rs`), `CLAUDE.md` 2.11 and handover 14.3 in its own commit, by the
lines it needs, with a decision that gives the number; E1 and G1 shrink first where they can; phase Z sets the round's
ceiling (the lines then plus 200, X88's rule) (L31).

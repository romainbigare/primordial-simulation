# Lighter world: the plan

Status: **restated after phase P and the third review, with the cover and G2 after their designs** (the probes R0, PR1, PR3, PR6, PR7, PR8, C0, PA;
`measures.md`). Branch `speed/lighter` at `195bbc2`: lane F, G1, the six-seed long-step test and B1 (the skip) merged.
This file is the one source of the steps: each row has its files, check level, done-when and kill line. The designs are
in `design/`; decisions in `decisions.md` (L1 to L43; answers to three reviews at its end); measures in `measures.md`;
history in `docs/history.md`, section 7.

## 1. The goal, and where the year goes

**The goal:** compute less. The owner said yes to all seven ideas, the law changes too (L3); several plant kinds per
cell that do not mix are a must (L17); idea 2 waits for the cover (L18). What must stay is the goal of the game
(handover 1, 6, 13): animals with brains, trails, later people, settlements and trade; a climate with seasons, rain and
rivers; plants that are food, cover and fuel and that adapt to their places.

**Where the year goes** (R0 and PA on `benchZ`, one thread, share of `run_for`; island22 / globe36 / region24):

| Part | island22 | globe36 | region24 |
|---|---|---|---|
| **animal updates** | **40.2%** | **32.1%** | **42.7%** |
| of it: the body's day (physics, upkeep, growth) | 16.6% | 12.8% | 17.3% |
| of it: children (`copy::copy`) / maturing (`copy::young::mature`) | 7.3% / 4.0% | 4.8% / 2.7% | 7.8% / 4.1% |
| of it: the decision / the habit | 9.0% / 2.3% | 9.5% / 2.0% | 9.7% / 2.8% |
| landing of the animals' moves and litters | 4.9% | 1.3% | 4.2% |
| **stand updates** | **21.7%** | **18.5%** | **24.1%** |
| the air | 13.4% | 20.6% | 11.0% |
| Change / the liquid / the survey | 7.8 / 2.6 / 5.3% | 7.8 / 3.5 / 5.0% | 6.1 / 2.0 / 5.5% |
| parts (Bond's whole month, before G1; of it the parts' reactor 4.2% on globe36, PR8) | 0 | 5.4% | 0.2% |

**Animals are now the largest part** (L37), and inside them the bookkeeping of nodes (children, maturing, the
landing, much of it `nodes::add` at about 21 thousand instructions a call) is 16% / 9% / 16% of the year, more than the
decision and the habit together. Stands are 19% to 24%, all included about 27% / 24% / 30% (C0). Every animal updates
every batch (its `INTERVAL` gene at or under 10 days, PA).

**What the round can give** (one thread, instructions of a settled year; estimates from the probes; "x" is faster by):

| Step | island22 | globe36 | region24 | Status |
|---|---|---|---|---|
| F the saves (long runs) | 0 | 0 | 0 | done: long runs about -6% (F3, estimate) and the main thread's saves 2.4% to 0.4% (F2, counted) |
| B1 the skip | -2.2% | -0.8% | -2.4% | **done** (`195bbc2`, measured) |
| G1 the parts' search start | 0 | -1.18% | -0.1% | **done** (measured) |
| G2 a part reacts its walls with the air over its footprint, held at its ground's temperature | 0 | about -2.6% | about -0.05% | design `parts-law.md` (L43), review, step |
| H1 children in lumps | up to -4.4% | up to -2.9% | up to -4.9% | design (`design/animals.md`), then the step; upper bounds (births at 67% of an adult's updates: the calls fall by at most half) |
| H5 one draw per birth | about -1.2% after H1 (-2.3% alone) | about -0.8% (-1.5%) | about -1.2% (-2.4%) | after H1 |
| H2 maturing in lumps | (up to -2.1%) | (-1.4%) | (-2.1%) | **not built** (under a law step's 3%; L39) |
| H4 a cheaper join | to be counted | to be counted | to be counted | probe after H1 and H5 |
| C the cover: C1a and C1 / with C2 | -1 to -7.5% / -9 to -13.5% | -2.5 to -6.5% / -8.5 to -11% | -3 to -8% / -11.5 to -15% | C0 done; `design/cover.md` after its third review (counted estimates; C1's range depends on the fill of kinds); the entry condition L42 |

| Case | island22 | globe36 | region24 | long runs |
|---|---|---|---|---|
| without the cover (B1, G, H1, H5) | about x1.08 to x1.10 | about x1.09 | about x1.09 to x1.10 | about 6 to 8% more |
| with C1a and C1 | about x1.10 to x1.18 | about x1.12 to x1.17 | about x1.13 to x1.20 | the same |
| **with C1 and C2** | **about x1.20 to x1.27** | **about x1.20 to x1.24** | **about x1.25 to x1.31** | the same |

(The cases add the steps' shares of today's year; each step's own measure restates them.) **Plainly:** the cover is no
longer the whole round: on `benchZ` the stands are a fifth to a quarter of the year, and the
animals two fifths. The round's largest sure gain is in the animals' bookkeeping (lane H), the parts' reactor on the
globe (G) and the long runs' saves (F, done). Built and not built so far: section 3.

## 2. Rules for this round

All of `CLAUDE.md` and of the round's brief (kept outside the repository, beside the scratch files) holds.

- **Check levels:** 1 bit for bit; 2 the same to rounding; 3 the same world (key numbers, pace and batch tests, the
  nodes check when a step touches nodes or minds); **4 a new law for a part of the world**: each level-4 row says what
  must still hold and which tests change. Never loosen a tolerance; a test of a deleted mechanism is deleted; a test of
  a changed law is rewritten to state the new law.
- **Kill lines and worth** (L2): a kill line asks for at most 10% of the year (for lane F, of a long run's year). A
  level-1 or level-2 step of 20 core lines or less is kept at 1% of the year or more, of 21 to 40 lines at 1.5%; a law
  step (level 3 or 4) needs about 3% on its target worlds. A step under its kill line is reverted and kept on a branch
  `lighter/<id>-not-kept`, and its alternative (section 3) starts.
- **References** (L10): the start's key numbers are those of `docs/speed-x10/measures.md` "Z: the final checks" 1.
  After each level-4 law lands, the key-number test runs once more and its values become the reference. Every level-3
  or level-4 step rebased on a moved `speed/lighter` runs the key numbers again before it merges. S50 and S84 stay the
  known failures. The long-step test judges the mean of seeds 1 to 6, its 0.3 K limit unchanged (L38).
- **How a gain is judged** (L11): 1T callgrind instructions of a year; 4T wall time under `flock $L/timing.lock`, 5 or
  more interleaved runs, medians, the load said; a step that changes the saved world is judged on one world (a turned
  save) or by counted work on remade benches; across remade worlds only large cuts count (PR-S). **Benches:** the step
  that bumps `SNAPSHOT_VERSION` remakes the three benches with its own build (`make_benches.sh`) at its merge and names
  the folder in `measures.md`; later steps use the newest set (`benchZ` until the first bump). Long runs count wall
  time (L8).
- **Agents.** Law steps and every adversarial review go to the strongest agents; normal code to normal agents;
  measures to measuring agents. Each step in its own worktree and branch; the architect merges one at a time;
  `SNAPSHOT_VERSION` goes to the highest plus one at each merge.
- **The three rules for smart bodies hold:** individuals stay real; the step per place; two kinds of body code (code
  paths, not two physics). **No calendar rule:** a window of days is a speed number judged against the per-update law
  (the nodes check against individuals), never a law of its own, a season or a date every body shares (L39).
- **Core lines** (L4, L31): the ceiling is 10,643 (10,444 at the start, R0; 10,457 now). The step that would pass it raises
  `CEILING` in `cli/tests/core_size.rs`, `CLAUDE.md` 2.11 and handover 14.3 in the same commit, by the lines it needs,
  with a decision logged; phase Z sets the round's ceiling.

## 3. The owner's seven ideas: what is built, what is not, and the alternatives

| # | Idea | Status | Alternative taken |
|---|---|---|---|
| 1 | Cheaper saves | **done** (F1, F3, F2; L13): the snapshot and the pictures timed, the tape's files written cheaper, the saves on a writer thread | |
| 2 | Fewer stands a cell | not built before the cover (L18) | the cover's bound on kinds (C1); A3 only if the cover fails |
| 3 | Living bodies do not rot | **the law is not built this round** (PR3: every form makes the world heavier, eaters x35, updates +38%): an open item for the owner (L9) | **B1, the skip (done)**: a bare reaction in a body too slow to matter over a year is skipped (-2.2 / -0.8 / -2.4%) |
| 4 | A cheaper sample member | not built before the cover (L15) | A4 only if C2 is not built and its gate holds |
| 5 | Plants as a cover | **restated from C0** (L35, L40): C1a (`crowd` counts bodies with a brain only), C1 (the bound on kinds and the newcomer's clock) kept at 3% of the year; C2 built only if C1 cuts at least 6.5% (island22) and 6% (region24), kept only if C1 + C2 reach -10% on both with the animals unchanged | A4 (the sample's stages) or nothing |
| 6 | Animals look around less | **D1, D2 not built** (PR6: perception is 0.6% to 0.9% of the year, the take's payer 0.6%) (L14) | **lane H**: the animal update by stage (PA): children in lumps (H1), one draw per birth (H5), a cheaper join (H4, a probe); H2 and H3 not built |
| 7 | A coarser air on globes | **E1 not built** (PR7: -4.7% of globe36 but the land 6% to 12% wetter on every seed) (L12) | **lane G**: G1 (done) and G2, the parts' reactor |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. "Counted": 1T on remade benches,
seeds 1 to 3, the parts the step names.

### Phase P (done)

R0, PR1, PR3, PR6, PR7, PR8, C0, PA: the key tables and verdicts are in `measures.md`; the full tables stay on the
probe branches (`probe/r0`, `probe/pr3`, `probe/pr7`, `probe/c0`, `probe/pa`). PR2 and PR4 were not run (L22).

### Lane F: the saves (done)

| ID | What | Level | Status |
|---|---|---|---|
| F1 | `save_snapshot` and `render::save` timed in the tape's column (the next year's row) | 1 | done |
| F3 | `series.csv` and `months.csv` appended, the viewer's file at gzip `fast` | 1 | done: a 1,000-year island run's writes -19.3 s, about -5.8% (estimate) |
| F2 | The compression and the write on a writer thread | 1 | done: judged on the count, the main thread's saves 2.4% to 0.4% of a 300-year run (L13); a quiet wall pair in phase Z |

### Lane B: the skip (done; `core/src/change/kernel.rs`; `design/no-rot.md`)

| ID | What | Level | Status |
|---|---|---|---|
| B1 | **The skip** (L9, L33): in a body's reactor, a bare reaction (factor at most 1) whose rate, contact and orders included, would use under `TOO_SLOW` (3.6e-5) of its scarcest input over the world's year (`kap * year`) is skipped; the per-year form, so a body's chemistry does not depend on its interval; `TOO_SLOW` a speed tolerance beside `NEGLIGIBLE` | 3 | **done** (`195bbc2`): -2.2 / -0.8 / -2.4% of the year against its kill line of -1% (L2's line for 20 lines or less); key numbers within their noise; judged by the six-seed long-step test, landed before it (`00899f9`: base 0.259 K, B1 0.262 K; on seeds 1 to 3 alone it would read 0.305 K, L38) |

**The law "living bodies do not rot" is an open item for the owner** (L9): its best form is (b0) (a decomposer reaction
runs in a living body only by its own catalyst; about +15 lines; no data change); it changes the eaters x35 and the
updates +38%, so it belongs to a round about life, with new eater references.

### Lane G: the parts' reactor (`core/src/bond/month.rs`, `core/src/change/kernel.rs`; `design/parts.md`)

| ID | What | Level | Done when | Kill line | Status |
|---|---|---|---|---|---|
| G1 | The halving search starts at `ceil(log2(heat / bound))` | 1 | golden unchanged; globe36 1T -1.18% | under 1% (L2) | **done** (`feceae7`) |
| G2 | **A law fix (level 4 for parts; `design/parts-law.md`, L43):** a part reacts only its walls (`Part::wall`) with the air column over its own footprint (the air node's amounts x the cell's share x footprint / cell area; about 1e-6 of the ground node's claim, so no second full claim), with the decomposers' catalyst as far as its ground is moist, held at its ground's temperature by its surface: the reactor gets the heat capacity `G · dt`, `G` its still-air conductance (one helper shared with a body's film, `body/exchange.rs`); one call a 6 months as today (`REACT_MONTHS`); `settle` and `send_gas` as today. The seasonal bias of one temperature for 6 months is checked; its named fix (the mean ground temperature since the last call) is built only if the check finds it over 30%. `laws.md` 3.1, 2.2; no state change, no snapshot bump | 4 (parts) | globe36's year 1T -2% counted on the `benchZ` save (estimate -2.6%); tests `a_part_reacts_at_its_grounds_temperature` (within `MAX_DT_K` of its ground, one sub-step, the books) and `a_walls_frame_rots_at_its_litter_constant` (a wall and the same dead matter on the ground agree to 10%); globe36 with life, 3 seeds, 30 years: the walls' rot against the litter constants at their ground's temperature, zonal land temperature and rain, the air's gases, the parts and shelter columns (M3) within their seeds' noise; golden re-pinned; the books | under -1.5% of globe36's year, or a check fails | design reviewed next, then the step |

### Lane H: animals, the largest part (`core/src/copy/`, `core/src/nodes.rs`, `core/src/params.rs`; `design/animals.md`)

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| H1 | **Children in lumps:** a node's brood fills at every update as today; it makes children at most once in a window of `Config::lump_days` (30, a speed number like `batch_days`, `numbers.md` row; snapshot bumped). The window is the world's batch index with the node's phase from its id (no world-wide pulse); the node keeps the index of its last lump (one field), so a changed interval gene can neither lump twice in a window nor skip one it updates in. `Config` validation: `lump_days` a multiple of `batch_days` and at most `node_days`. One direction draw per birth call, as today (so a lump's dispersers go one way: `animals.md` 2). Individuals and stands as today. `laws.md` 4.2 | `copy/mod.rs`, `params.rs`, `body/mod.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | what must hold, below; counted: the year 1T -3% on island22 and region24 (upper bound -4.4 / -4.9%) | under -3% on island22 (L2) |
| H5 | **One draw per birth** (L41): `copy::copy` draws the children's genes and brain rows once per birth and splits the members between dispersers and stayers (the gap of the two groups' means into their spread), instead of two `emit` calls that each draw | `copy/mod.rs`, tests, docs | 2 or 3 | golden re-pinned; the key numbers; the nodes check; the year 1T -1% after H1 (about -1.2 / -0.8 / -1.2%; -2.3% alone) | under -1% of the year (a step of 20 lines or less, L2) |
| H4 | **Probe first, after H1 and H5:** a cheaper `nodes::add` for joiners of the same lineage (the group that moved, the part that matured): the share of joins whose genes equal or lie within rounding of the node's; the cost of a join's per-gene pooling, `Body::prepare`, the rows' blend and `see`; then a step at level 1 or 2 if it clears L2. Before C (both edit `nodes.rs`) | `nodes.rs` | probe, then 1 or 2 | the probe's table; a step only at 1% of the year (20 lines) or 1.5% (40) | as L2 |

**H1 must still hold:** the key numbers, the eaters within their range (5.354 ± 0.5; life 10.47 yr x/ 2); the nodes
check (individuals keep per-update births); the pace and batch tests; `the_eaters_live_and_stop_growing`;
`a_century_keeps_its_books` and the books of every lump (the brood leaves with dead members, `Body::lose`, as today);
births a year, **births by month** (`months.csv`) and the young nodes' members within their seeds' noise; litters
landing in cells with no node of their kind and the eaters' range on globe36 (the coarser dispersal); watch-for M1 and
M2. **Tests that change or are new:** the copy tests that make children in one update state the window
(`a_mature_nodes_children_gather_in_its_young_node`, `reproducing_updates_keep_elements_and_energy`); new: a node's
births over 90 days are the same in lumps as at every update, within the brood held; over N windows a node lumps at
most N times, also when its interval gene changes; **the pulse test**: no batch holds more than its share of the
year's lumps (about one in `lump_days / batch_days`, within a stated margin).

**Not built in lane H:** **H2** (maturing in lumps: its upper bound -2.1 / -1.4 / -2.1% is under a law step's 3%, and
its field needs two new rules, T4); **H3** (`think_days` 40: X72 measured -0.32 decades for the young world's eaters).
Both L39.

### Lane C: plants as a cover (`design/cover.md` 7; L17, L23 to L28, L35, L36, L40, L42)

C0 is done (`measures.md` C0). "Counted" = callgrind on one thread, one year from the remade benches, seeds 1 to 3, with
`--toggle-collect` on the update of a body without a brain: the year, the plant work per planted cell-year, the planted
cells, the animal updates a year, bodies per planted cell, `outbox::deliver`. A number in brackets is a section of
`cover.md`; tests 1 to 3 are its 6.3. The order: C1a, C1, then C2 only at its entry condition, C3, G-C; after lane H in
`copy/` (L35). **The entry condition (L42, `cover.md` D13):** C1 is built and measured; it is kept alone if it gives at
least 3% of the year (counted) with its law checks; C2 is built if the estimate of C1 + C2 from C1's measured world
reaches -10% on island22 and region24; C1 and C2 are kept together only if their measured sum reaches -10% with the
animals unchanged; else C1 alone if it passed 3%, else neither (A4 or nothing).

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| C1a | **`crowd` counts bodies with a brain only, set to 1; an animal node splits only while it is the only body with a brain in its cell** (3): `nodes::crowd` and `nodes::split`; `numbers.md`'s `crowd` row; `laws.md` 4.4; handover 7.1 (the check's pair `--crowd 1` and `2`); `tape/tests/nodes.rs` (`NOISE_CROWD` 2); snapshot | `nodes.rs`, `params.rs`, `tape/src/snapshot.rs`, `tape/tests/nodes.rs`, docs | 4 (animals) | animal nodes per planted cell, animal updates a year and animal splits and merges a year (planted and unplanted cells) within their seeds' noise of the world after lane H (counted, 3 seeds), and in a young world (32 x 32, year 30) splits as today's; the eaters' key numbers; the nodes check holds its rules; its own effect on the counted year reported apart | those move past their seeds' noise: option (a) stays, and C1 and C2 are gated on the animals as they are |
| C1 | **The bound on kinds and the clock** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the newcomer place is free, else the litter joins the most alike; a light-eater node splits only while its cell holds fewer than `cell_kinds` kinds with adults; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (no `alike` limit for bodies without a brain), and every kind without adults older than its life span at its ground's temperature now joins its most alike kind (`Body::life_span`); snapshot; test 1 (two invasions, the speed against today's code) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `body/tissue.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (kinds with adults and newcomers per planted cell measured apart); test 1; animals within their seeds' noise of the world after C1a and H; key numbers in range (S50 within noise); the climate and life tests of 6; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's); the counted year against the world after C1a and H (estimate -1% to -7.5% island22, -3% to -8% region24) | **kept if** the counted year falls at least 3% on island22 and region24 with all its law checks (decision 37), **or, if D13 is taken,** if its law checks pass and C1 + C2's estimate from its counted world reaches -10%; else C stops (A4 or nothing) |
| C2 | **The young inside** (2), **built only if C1 is kept and C1 + C2's estimate from C1's counted world (kinds with adults and newcomers per planted cell, the prices of 5) reaches -10% on island22 and region24**: `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth by each stage's balance, a young deficit only killing young (2.5); maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage; founders and bodies that lose their brain all adult; `Young::held` a day, read by the clock; snapshot; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4, 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; **C1 + C2 at least -10% of the year counted on island22 and region24 against the world after C1a and H** (decision 37; estimate -10% to -14.5%, -12.5% to -16%) with animal nodes and animal updates within their seeds' noise; light-eater members per planted cell within a factor of 2 of C1's; test 2; key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 | C1 + C2 under -10% on island22 or region24, or the animal, key-number, climate or kinds lines fail: C2 is not merged, C1 stays if it passed its own lines |
| C3 | **The checks** (no code): counted runs of the three benches, plants all included; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **The cover's gate** (decisions 37, 46), against the world after C1a and lane H: C1 at least -3% of the year counted on island22 and region24 with its law checks (or D13); with C2, C1 + C2 at least -10% on both, globe36 not worse; animal nodes per planted cell and animal updates a year within their seeds' noise; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 stays if it passed its own lines); A4 or nothing |

### Lanes not built

| Lane | Why | Alternative |
|---|---|---|
| D (D1, D2) | PR6: perception 0.55 to 0.91% of the year, the take's payer 0.6%, the per-substance repeats in the take 0.6% (L14) | lane H |
| E (E1) | PR7: -4.7% of globe36, -3.8 to -4.2% of the size-28 globe, but rain on land +6% to +12% on every seed and world, 13 to 20 times its year-to-year spread (L12); code on `probe/pr7` | lane G (G2) |
| A (A3, A4) | phase 2 only (L6, L18) | A4 if C2 is not built and it is worth L2's line |

### Phase 2 (after the gate)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A4 | Only if C2 is not built: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6%; the year 1T -1.5% | under -1.5% of the year (L2) |
| C4 | After G-C if C2 is built: a probe of the cost of making a cover's children (L30) | probe | its table | none |

### Phase Z: the end (unchanged)

1. The year at the round's end against its start: `$L/ew-base` on `benchZ` and the end's build on benches made with
   its own code (`make_benches.sh`), 6 interleaved sets, 4 threads and 1 thread, under the lock; medians. And 1T
   instructions of a year on both.
2. 1,000-year runs (seed 1; island 22, globe 18, region 24 with `--region 8,8,8,8`), one at a time, wall time, the
   default pictures and saves, for both `$L/ew-base` and the end's build (L21); the quiet wall pair of F2 (L13).
3. The largest globe that runs 1,000 years in 15 minutes of wall time (start at size 28, then up).
4. The slow suite, the golden test without threads, the debug checks.
5. The report `docs/reports/lighter.md`, with the owner's open items (the law of living bodies, L9).
6. The docs at the round's end: the `CLAUDE.md` row "Lighter world (done)"; handover 7.1, 7.3, 13.2, 14.3 (the core
   ceiling: the lines then plus 200, L4); every row of this plan says kept, not kept or not built.
7. The final adversarial review, then fixes, until accepted.

## 5. Order and lanes

```
Phase P   done
Phase 1   F: done ;  B: B1 done ;  G: G1 done ─ G2 (design note, review, step) ──┐
          H: H1 ─ H5 ─ H4 (probe, then maybe a step) ──────────────────────────┼─► lane reviews
          C (after H): C1a ─ C1 ─ (C2 at its entry) ─ C3 ─ G-C ─────────────────┘
Phase 2   C2 not built: A4 ;  C2 built: C4
Phase Z   the end ─► final review
```

**Where steps collide, this order holds** (L35, L39, third review T8):
- **H before C** in `copy/` and `nodes.rs`: H1 and H5 change `copy/mod.rs`, H4 `nodes.rs` (`add`), as C1a, C1 and C2
  do (`add`, `rejoin`, `Lot`, `part`). H1, then H5, then H4's probe and step; C rebases on H. "The animals as today" in
  C1a, C1 and C2, and the cover's gates, mean the world after C1a and lane H (L10).
- G2 (in `bond/month.rs`, the kernel only through its call) comes after B1, which touches a body's reactor only: no
  clash. G2's walls' check uses the world after B1.
- H1, H5, C1a, C1, C2 and G2 are level 2 to 4: they merge one at a time; each re-runs the key numbers against the
  reference the one before set (L10).
- H1, C1a, C1 and C2 bump the snapshot: the number is the highest plus one at each merge, and the benches are remade
  (L11).

## 6. Reviews

The cycle is: plan, adversarial review, implement, adversarial review, fix, until accepted. Done: the plan (three rounds),
`cover.md` (three rounds), lane F, lane B (two rounds). To come: G2's design note before its step; H1's
step against `animals.md`; each lane after its last step (G, H, C), each level-3 or level-4 lane with the
key-number test against the reference; the cover's restated design and G-C; phase Z.

## 7. Rule and doc changes, and the step that writes each

| Rule or doc | Lands with |
|---|---|
| `docs/tape.md` (a run's year holds its saves and pictures; how the tape is written) | F (done) |
| `laws.md` 2.1 or 2.4 (a tiny bare reaction in a body is skipped, over a year) | B1 (done) |
| `laws.md` 3 (a part reacts its own share of the air, held with its ground's heat) | G2 |
| `laws.md` 4.2 (children in lumps, at most once in a body's window); `numbers.md` `lump_days` | H1 |
| `laws.md` 4.2 (one draw of genes and rows per birth) | H5 |
| handover 7.1 (the nodes check's noise pair: the default and the next value, `--crowd 1` and `2`); `laws.md` 4.4, `numbers.md` `crowd` (bodies with a brain only, 1) | C1a |
| handover 7.1, 7.3; `laws.md` 4.2, 4.4, 6 (what a plant is); `numbers.md` `cell_kinds` | C1, C2 (from `design/cover.md`) |
| `CLAUDE.md` row "Lighter world (done)"; handover 13.2 (the law of living bodies left to the owner), 14.3 and `CLAUDE.md` 2.11 (the ceiling) | Z |

## 8. Risks

| Risk | Answer |
|---|---|
| H1 changes the timing of births (up to a window later) and the brood's exposure to deaths | the brood fills as today and leaves with dead members (`Body::lose`); births a year and the young node within their seeds' noise; the nodes check |
| H1 makes a world-wide pulse (every node at the same batch) | each body's window has its own phase (its id); the pulse test; no date shared by every body (L39) |
| H1 moves births into colder weather (about 10 to 15 days later on average) and spreads litters in fewer, larger steps | births by month and litters landing where no kind of theirs is, within their seeds' noise |
| G2 changes the parts' walls and their gas turnover (today what a wall can burn burns every 6 months) | its checks against the litter constants on globe36 with life, not against today's runaway; the books |
| The cover's gain on the restated world is small | C1 judged at 3%, C2 at -10% with the animals unchanged; the alternative A4 or nothing |
| A step moves the long-step test by chance (one seed spans 0.10 to 0.40 K) | the test judges the mean of seeds 1 to 6 (L38); the cause of the small room (the 60-day liquid, X80) stays an open item for the owner |
| Lanes collide in `copy/`, golden and the snapshot | section 5's order; one merge at a time; key numbers again after a rebase |

## 9. Lines

Core at the round's start 10,444 (R0); now 10,457 (G1, B1); ceiling 10,643. Budgets: G2 about +6 net (`parts-law.md`); H1 about +25 (the
window, its field and the guard); H5 about +15; H4 by its probe (at most +20); the cover by `design/cover.md` 7 (C1a
about +2, C1 about +29 net, C2 about +115 net). Without C2 about +95 (to about 10,555), within the ceiling; with C2
about +210 (to about 10,670): C2 (or the step that crosses) raises the ceiling in its commit, with a decision (L31).

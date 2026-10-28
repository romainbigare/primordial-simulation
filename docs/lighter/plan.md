# Lighter world: the plan

Status: **phase Z measured; the final review to come.** The report is `docs/reports/lighter.md`. Branch `speed/lighter`: lane F, G1, the six-seed long-step test,
B1 (the skip), H1 (children in lumps) and L1 (the liquid's sink every physics step) kept; G2, H5, C1 and HP1 not
kept (their branches named below). This file is the one source of the steps: each row has its files, check level,
done-when and kill line, and its status. The designs are in `design/`; decisions in `decisions.md` (L1 to L47; answers
to three reviews at its end); measures in `measures.md`; history in `docs/history.md`, section 7.

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

**What was kept** (one thread, instructions of a settled year, each step measured on its own base; "x" is faster by):

| Step | island22 | globe36 | region24 | Status |
|---|---|---|---|---|
| F the saves (long runs) | 0 | 0 | 0 | **kept**: long runs about -6% (F3, estimate) and the main thread's saves 2.4% to 0.4% of a 300-year run (F2, counted); a quiet wall pair in phase Z |
| B1 the skip | -2.2% | -0.8% | -2.4% | **kept** (`195bbc2`) |
| G1 the parts' search start | 0 | -1.18% | -0.1% | **kept** (`feceae7`) |
| H1 children in lumps | -7.0% | -5.9% | -7.1% | **kept** (`060341f`) |
| L1 the liquid's sink every physics step (a law fix, L46) | +0.3% | +0.3% | +0.3% | **kept** (`bcef8fd`) |
| **all kept, added** | **about -8.9% (x1.10)** | **about -7.6% (x1.08)** | **about -9.3% (x1.10)** | phase Z, across remade worlds: 1T -12.0 / -22.4 / -10.4%; 4T x1.10 / x1.30 / x1.02; 1,000 years x1.41 / x1.37 / x1.22 |

**Not kept or not built:** G2 (parts no longer end, L43), H5 (-1.5% at most, L41), H2 and H3 (L39), C1 (-1.2% on
island22, and it changed the plants, L45), C2 (C1 + C2 about -7% on island22, under -10%), HP1 and HP2 (L47), the law
of living bodies (L9), D1, D2 (L14), E1 (L12), A3, A4 (under their worth lines). **Plainly:** the kept steps give about a
tenth of the year on the benches; phase Z measured x1.02 to x1.30 for a settled year on 4 threads and x1.22 to x1.41
for 1,000 years, partly from the remade worlds; the cover did not pay on today's world.

### The owner's open choices, as they stand

1. **Living bodies do not rot** (handover 13.2): the owner says leave as is; the law (b0) is ready if wanted (L9).
2. **How walls end and the parts' reactor** (G2, L43): the owner said yes to the loose-residue rule (Pilling-Bedworth),
   rot through the outer skin, part nodes now, built solids of bodies (shells) as building matter, a pieces store per
   cell, mud by properties only, and stone that lasts centuries; designs running (`design/objects-scale`,
   `design/objects-materials`); for the next steps, not this round's speed.
3. **S50, plants store too much:** a study queued.
4. **The 60-day liquid warms the land:** answered by L1 (the long-step test's room 0.191 of 0.3 K).
5. **The sleep's gas bias:** accepted as small; the sleep test states it.
6. **Plant forms side by side:** C1 not kept (L45); the hard plant law waits for chemistry 0's price of lime against
   frame (a life round), then HP1 (L47).
7. **The plants' copy-mass gene at long paces** (PR-C2): kept in the notes.
8. **Small nodes act whole:** not kept; an open item.
9. **A coarser air:** leave (E1 not built, L12).

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
- **Core lines** (L4, L31): the ceiling was 10,643 (10,444 at the start, R0; 10,473 at the end). The step that would pass it raises
  `CEILING` in `cli/tests/core_size.rs`, `CLAUDE.md` 2.11 and handover 14.3 in the same commit, by the lines it needs,
  with a decision logged; phase Z set the round's ceiling: **10,673** (10,473 + 200).

## 3. The owner's seven ideas: what is built, what is not, and the alternatives

| # | Idea | Status | Alternative taken |
|---|---|---|---|
| 1 | Cheaper saves | **kept** (F1, F3, F2; L13): the snapshot and the pictures timed, the tape's files written cheaper, the saves on a writer thread | |
| 2 | Fewer stands a cell | **not built** (L18; the cover not kept, L45) | the cover's bound on kinds (C1); A3 only if the cover fails |
| 3 | Living bodies do not rot | **the law is not built this round** (PR3: every form makes the world heavier, eaters x35, updates +38%): an open item for the owner (L9) | **B1, the skip (done)**: a bare reaction in a body too slow to matter over a year is skipped (-2.2 / -0.8 / -2.4%) |
| 4 | A cheaper sample member | **not built** (L15; A4 under its worth line, L45) | A4 only if C2 is not built and its gate holds |
| 5 | Plants as a cover | **not kept** (C1 not kept, C2 not built, L45); **restated from C0** (L35, L40): C1a (`crowd` counts bodies with a brain only), C1 (the bound on kinds and the newcomer's clock) kept at 3% of the year; C2 built only if C1 cuts at least 6.5% (island22) and 6% (region24), kept only if C1 + C2 reach -10% on both with the animals unchanged | A4 (the sample's stages) or nothing |
| 6 | Animals look around less | **D1, D2 not built**; its alternative H1 kept (PR6: perception is 0.6% to 0.9% of the year, the take's payer 0.6%) (L14) | **lane H**: the animal update by stage (PA): children in lumps (H1), one draw per birth (H5), a cheaper join (H4, a probe); H2 and H3 not built |
| 7 | A coarser air on globes | **E1 not built**; its alternative G1 kept, G2 not kept (PR7: -4.7% of globe36 but the land 6% to 12% wetter on every seed) (L12) | **lane G**: G1 (done) and G2, the parts' reactor |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. "Counted": 1T on remade benches,
seeds 1 to 3, the parts the step names.

### Phase P (done)

R0, PR1, PR3, PR6, PR7, PR8, C0, PA: the key tables and verdicts are in `measures.md`; the full tables stay on the
probe branches (`probe/r0`, `probe/pr3`, `probe/pr7`, `probe/c0`, `probe/pa`). PR2 and PR4 were not run (L22).

### Lane F: the saves (done)

| ID | What | Level | Status |
|---|---|---|---|
| F1 | `save_snapshot` and `render::save` timed in the tape's column (the next year's row) | 1 | kept |
| F3 | `series.csv` and `months.csv` appended, the viewer's file at gzip `fast` | 1 | kept: a 1,000-year island run's writes -19.3 s, about -5.8% (estimate) |
| F2 | The compression and the write on a writer thread | 1 | kept: judged on the count, the main thread's saves 2.4% to 0.4% of a 300-year run (L13); a quiet wall pair in phase Z |

### Lane B: the skip (done; `core/src/change/kernel.rs`; `design/no-rot.md`)

| ID | What | Level | Status |
|---|---|---|---|
| B1 | **The skip** (L9, L33): in a body's reactor, a bare reaction (factor at most 1) whose rate, contact and orders included, would use under `TOO_SLOW` (3.6e-5) of its scarcest input over the world's year (`kap * year`) is skipped; the per-year form, so a body's chemistry does not depend on its interval; `TOO_SLOW` a speed tolerance beside `NEGLIGIBLE` | 3 | **kept** (`195bbc2`): -2.2 / -0.8 / -2.4% of the year against its kill line of -1% (L2's line for 20 lines or less); key numbers within their noise; judged by the six-seed long-step test, landed before it (`00899f9`: base 0.259 K, B1 0.262 K; on seeds 1 to 3 alone it would read 0.305 K, L38) |

**The law "living bodies do not rot" is an open item for the owner** (L9): its best form is (b0) (a decomposer reaction
runs in a living body only by its own catalyst; about +15 lines; no data change); it changes the eaters x35 and the
updates +38%, so it belongs to a round about life, with new eater references.

### Lane G: the parts' reactor (`core/src/bond/month.rs`, `core/src/change/kernel.rs`; `design/parts.md`)

| ID | What | Level | Done when | Kill line | Status |
|---|---|---|---|---|---|
| G1 | The halving search starts at `ceil(log2(heat / bound))` | 1 | golden unchanged; globe36 1T -1.18% | under 1% (L2) | **kept** (`feceae7`) |
| G2 | **Not kept (L43): parts no longer end without the 1e6 K burn (`measures.md` "G2: not kept"); branch `lighter/g2-not-kept`; an open item for the owner.** **A law fix (level 4 for parts; `design/parts-law.md`, L43):** a part reacts only its walls (`Part::wall`) with the air column over its own footprint (the air node's amounts x the cell's share x footprint / cell area; about 1e-6 of the ground node's claim, so no second full claim), with the decomposers' catalyst as far as its ground is moist, held at its ground's temperature by its surface: the reactor gets the heat capacity `G · dt`, `G` its still-air conductance (one helper shared with a body's film, `body/exchange.rs`); one call a 6 months as today (`REACT_MONTHS`); `settle` and `send_gas` as today. The seasonal bias of one temperature for 6 months is checked; its named fix (the mean ground temperature since the last call) is built only if the check finds it over 30%. `laws.md` 3.1, 2.2; no state change, no snapshot bump | 4 (parts) | globe36's year 1T -2% counted on the `benchZ` save (estimate -2.6%); tests `a_part_reacts_at_its_grounds_temperature` (within `MAX_DT_K` of its ground, one sub-step, the books) and `a_walls_frame_rots_at_its_litter_constant` (a wall and the same dead matter on the ground agree to 10%); globe36 with life, 3 seeds, 30 years: the walls' rot against the litter constants at their ground's temperature, zonal land temperature and rain, the air's gases, the parts and shelter columns (M3) within their seeds' noise; golden re-pinned; the books | under -1.5% of globe36's year, or a check fails | not kept |

### Lane H: animals, the largest part (`core/src/copy/`, `core/src/nodes.rs`, `core/src/params.rs`; `design/animals.md`)

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| H1 | **Kept** (`060341f`: -7.0 / -5.9 / -7.1%; the guard `lump_days <= node_days` dropped, the window applies to every node, L39). **Children in lumps:** a node's brood fills at every update as today; it makes children at most once in a window of `Config::lump_days` (30, a speed number like `batch_days`, `numbers.md` row; snapshot bumped). The window is the world's batch index with the node's phase from its id (no world-wide pulse); the node keeps the index of its last lump (one field), so a changed interval gene can neither lump twice in a window nor skip one it updates in. `Config` validation: `lump_days` a multiple of `batch_days` and at most `node_days`. One direction draw per birth call, as today (so a lump's dispersers go one way: `animals.md` 2). Individuals and stands as today. `laws.md` 4.2 | `copy/mod.rs`, `params.rs`, `body/mod.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | what must hold, below; counted: the year 1T -3% on island22 and region24 (upper bound -4.4 / -4.9%) | under -3% on island22 (L2) |
| H5 | **Not kept** (-1.54 / -0.45 / -1.68%, under 3%; L41; branch `lighter/h5-not-kept`). **One draw per birth** (L41): `copy::copy` draws the children's genes and brain rows once per birth and splits the members between dispersers and stayers (the gap of the two groups' means into their spread), instead of two `emit` calls that each draw | `copy/mod.rs`, tests, docs | 2 or 3 | golden re-pinned; the key numbers; the nodes check; the year 1T -1% after H1 (about -1.2 / -0.8 / -1.2%; -2.3% alone) | under -1% of the year (a step of 20 lines or less, L2) |
| H4 | **Not built** (the architect, before phase Z: PA's estimate of 2% to 3% is at the law-step line, and `nodes::add` is the code the objects round's part nodes rebuild next; alternative: none in this round). **Probe first, after H1 and H5:** a cheaper `nodes::add` for joiners of the same lineage (the group that moved, the part that matured): the share of joins whose genes equal or lie within rounding of the node's; the cost of a join's per-gene pooling, `Body::prepare`, the rows' blend and `see`; then a step at level 1 or 2 if it clears L2. Before C (both edit `nodes.rs`) | `nodes.rs` | probe, then 1 or 2 | the probe's table; a step only at 1% of the year (20 lines) or 1.5% (40) | as L2 |

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
| C1a | **Not kept, with C1** (L45; branch `lighter/c-not-kept`). **`crowd` counts bodies with a brain only, set to 1; an animal node splits only while it is the only body with a brain in its cell** (3): `nodes::crowd` and `nodes::split`; `numbers.md`'s `crowd` row; `laws.md` 4.4; handover 7.1 (the check's pair `--crowd 1` and `2`); `tape/tests/nodes.rs` (`NOISE_CROWD` 2); snapshot | `nodes.rs`, `params.rs`, `tape/src/snapshot.rs`, `tape/tests/nodes.rs`, docs | 4 (animals) | animal nodes per planted cell, animal updates a year and animal splits and merges a year (planted and unplanted cells) within their seeds' noise of the world after lane H (counted, 3 seeds), and in a young world (32 x 32, year 30) splits as today's; the eaters' key numbers; the nodes check holds its rules; its own effect on the counted year reported apart | those move past their seeds' noise: option (a) stays, and C1 and C2 are gated on the animals as they are |
| C1 | **Not kept** (L45; `700f203` on `lighter/c-not-kept`: -1.2% of island22's year against the -3% line; splits of light-eaters x20 to x90; the second invasion of test 1 fails; light-eaters' copy mass -0.7 to -1.0 decades). **The bound on kinds and the clock** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the newcomer place is free, else the litter joins the most alike; a light-eater node splits only while its cell holds fewer than `cell_kinds` kinds with adults; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (no `alike` limit for bodies without a brain), and every kind without adults older than its life span at its ground's temperature now joins its most alike kind (`Body::life_span`); snapshot; test 1 (two invasions, the speed against today's code) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `body/tissue.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (kinds with adults and newcomers per planted cell measured apart); test 1; animals within their seeds' noise of the world after C1a and H; key numbers in range (S50 within noise); the climate and life tests of 6; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's); the counted year against the world after C1a and H (estimate -1% to -7.5% island22, -3% to -8% region24) | **kept if** the counted year falls at least 3% on island22 and region24 with all its law checks (decision 37), **or, if D13 is taken,** if its law checks pass and C1 + C2's estimate from its counted world reaches -10%; else C stops (A4 or nothing) |
| C2 | **Not built** (C1 + C2 about -7% on island22, under -10%; L45). **The young inside** (2), **built only if C1 is kept and C1 + C2's estimate from C1's counted world (kinds with adults and newcomers per planted cell, the prices of 5) reaches -10% on island22 and region24**: `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth by each stage's balance, a young deficit only killing young (2.5); maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage; founders and bodies that lose their brain all adult; `Young::held` a day, read by the clock; snapshot; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4, 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; **C1 + C2 at least -10% of the year counted on island22 and region24 against the world after C1a and H** (decision 37; estimate -10% to -14.5%, -12.5% to -16%) with animal nodes and animal updates within their seeds' noise; light-eater members per planted cell within a factor of 2 of C1's; test 2; key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 | C1 + C2 under -10% on island22 or region24, or the animal, key-number, climate or kinds lines fail: C2 is not merged, C1 stays if it passed its own lines |
| C3 | **Not built** (C not kept; not run). **The checks** (no code): counted runs of the three benches, plants all included; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **Not kept: failed at C1** (L45). **The cover's gate** (decisions 37, 46), against the world after C1a and lane H: C1 at least -3% of the year counted on island22 and region24 with its law checks (or D13); with C2, C1 + C2 at least -10% on both, globe36 not worse; animal nodes per planted cell and animal updates a year within their seeds' noise; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 stays if it passed its own lines); A4 or nothing |

### Lane L: the liquid (the owner's study request; `design/liquid.md`)

| ID | What | Level | Done when | Kill line | Status |
|---|---|---|---|---|---|
| L1 | **A law fix:** the liquid's sink into the bed's store (`drain::sink`) runs every physics step right after the air step; the rest of the liquid every `flow_days` (60). Rain over field capacity no longer waits in the pores for up to 60 days, darkening the land (+1.7 W/m² absorbed, +0.2 K in the study); `Config::validate` wants each liquid step on a physics step | 3 | the long-step test's land 0.273 -> 0.191 K; the key numbers in range but S50 (land 292.155 -> 292.103 K, rain on land -0.5%, land worn 0.1335 -> 0.1471 mm/yr); golden re-pinned | the year +1% | **kept** (`bcef8fd`; +0.3% of the year; L46) |

### Lane HP: hard plant matter (the owner's request: plants tall and short side by side)

| ID | What | Level | Status |
|---|---|---|---|
| HP1 | The band rule and the sample's height credit (`c45730b`, `c2a49f8`; one-commit version `lighter/hp1-v1`) | 4 | **not kept** (L47; branch `lighter/hp1-not-kept`): plants stiffen with lime (17 to 34% of dry mass), as chemistry 0 makes lime cheaper than frame; island22's year +40%; S50's pass is arithmetic |
| HP2 | The second step of the hard plant law | 4 | **not built** (L47) |

### Studies and test fixes (no law change, or not kept)

| What | Status |
|---|---|
| The sleep test states the gas bias it accepts (S84: `air_3` 0.3% high in six of six seeds, +4 ppm and +0.04 K over a century, flat after year 21; held to `GAS_SLEEP_BIAS` 3e-6, twice the measured) | **kept** (`c9f263a`; the owner accepts it as small) |
| The drift tests' scene makes children at every update (`churn`: `lump_days` one batch, as the other node tests since H1; `genes_do_not_drift_without_selection` and `selection_moves_a_nodes_brain_size` pass again) | **kept** (`335e1c1`) |
| Small nodes act whole (open item 8 of speed x10): two fixes lower the young eaters (5.18 to 4.89 log10 kg) and move nodes away from individuals in the nodes check | **not kept** (`5d226db`; branch `study/small-nodes-not-kept`); an open item |
| Why the long liquid step warms the land (X80) | **kept** (a study, `4a6039a`), then L1 |

### Lanes not built

Every row: **not built**.

| Lane | Why | Alternative |
|---|---|---|
| D (D1, D2) | PR6: perception 0.55 to 0.91% of the year, the take's payer 0.6%, the per-substance repeats in the take 0.6% (L14) | lane H |
| E (E1) | PR7: -4.7% of globe36, -3.8 to -4.2% of the size-28 globe, but rain on land +6% to +12% on every seed and world, 13 to 20 times its year-to-year spread (L12); code on `probe/pr7` | lane G (G2) |
| A (A3, A4) | phase 2 only (L6, L18); with C not kept, A3 and A4 are under their worth lines (L45) | none for speed |

### Phase 2 (after the gate)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A4 | **Not built** (L45: under its worth line). Only if C2 is not built: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6%; the year 1T -1.5% | under -1.5% of the year (L2) |
| C4 | **Not built** (C2 not built). After G-C if C2 is built: a probe of the cost of making a cover's children (L30) | probe | its table | none |

### Phase Z: the end

Items 1 to 6: **kept** (done; the "Z" sections of `measures.md`, the report). Item 7: to come.

1. The year at the round's end against its start: `$L/ew-base` on `benchZ` and the end's build on benches made with
   its own code (`make_benches.sh`), 6 interleaved sets, 4 threads and 1 thread, under the lock; medians. And 1T
   instructions of a year on both.
2. 1,000-year runs (seed 1; island 22, globe 18, region 24 with `--region 8,8,8,8`), one at a time, wall time, the
   default pictures and saves, for both `$L/ew-base` and the end's build (L21); the quiet wall pair of F2 (L13: the
   tape's column of these runs, the snapshots in it at the end).
3. The largest globe that runs 1,000 years in 15 minutes of wall time (start at size 28, then up).
4. The slow suite, the golden test without threads, the debug checks.
5. The report `docs/reports/lighter.md`, with the owner's open items (the law of living bodies, L9).
6. The docs at the round's end: the `CLAUDE.md` row "Lighter world (done)"; handover 7.1, 7.3, 13.2, 14.3 (the core
   ceiling: the lines then plus 200, L4); every row of this plan says kept, not kept or not built.
7. The final adversarial review, then fixes, until accepted.

## 5. Order and lanes

```
Phase P   done
Phase 1   F: done ;  B: B1 done ;  G: G1 done, G2 not kept ;  L: L1 done
          H: H1 done, H5 not kept, H4 not built ;  C (after H): C1a and C1 not kept, C2 not built
          HP: HP1 not kept, HP2 not built
Phase 2   not built (A4 and C4 under their conditions)
Phase Z   done (the year, 1,000 years, the largest globe, the slow suite, the report) ─► final review
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
`cover.md` (three rounds), lane F, lane B (two rounds), G2's design note, H1 against `animals.md`, lanes G, H and C
after their last steps with the key numbers, the cover's restated design and G-C, and phase Z (`review-z`, accepted
after fixes).

## 7. Rule and doc changes, and the step that writes each

| Rule or doc | Lands with |
|---|---|
| `docs/tape.md` (a run's year holds its saves and pictures; how the tape is written) | F (kept) |
| `laws.md` 2.1 or 2.4 (a tiny bare reaction in a body is skipped, over a year) | B1 (kept) |
| `laws.md` 3 (a part reacts its own share of the air, held with its ground's heat) | G2 (not kept: not written) |
| `laws.md` 4.2 (children in lumps, at most once in a body's window); `numbers.md` `lump_days` | H1 (kept) |
| `laws.md` 4.2 (one draw of genes and rows per birth) | H5 (not kept: not written) |
| handover 7.1 (the nodes check's noise pair: the default and the next value, `--crowd 1` and `2`); `laws.md` 4.4, `numbers.md` `crowd` (bodies with a brain only, 1) | C1a (not kept: not written) |
| handover 7.1, 7.3; `laws.md` 4.2, 4.4, 6 (what a plant is); `numbers.md` `cell_kinds` | C1, C2 (not kept, not built: not written) |
| `CLAUDE.md` row "Lighter world (done)"; handover 13.2 (the law of living bodies left to the owner), 14.3 and `CLAUDE.md` 2.11 (the ceiling); handover 5.4, 7.3, 12.6 (the speed and the steps) | Z (kept) |

## 8. Risks

| Risk | Answer |
|---|---|
| H1 changes the timing of births (up to a window later) and the brood's exposure to deaths | the brood fills as today and leaves with dead members (`Body::lose`); births a year and the young node within their seeds' noise; the nodes check |
| H1 makes a world-wide pulse (every node at the same batch) | each body's window has its own phase (its id); the pulse test; no date shared by every body (L39) |
| H1 moves births into colder weather (about 10 to 15 days later on average) and spreads litters in fewer, larger steps | births by month and litters landing where no kind of theirs is, within their seeds' noise |
| G2 changes the parts' walls and their gas turnover (today what a wall can burn burns every 6 months) | its checks against the litter constants on globe36 with life, not against today's runaway; the books |
| The cover's gain on the restated world is small | C1 judged at 3%, C2 at -10% with the animals unchanged; the alternative A4 or nothing |
| A step moves the long-step test by chance (one seed spans 0.10 to 0.40 K) | the test judges the mean of seeds 1 to 6 (L38); the cause of the small room (the 60-day liquid, X80) is answered by L1 (0.273 to 0.191 K, L46) |
| Lanes collide in `copy/`, golden and the snapshot | section 5's order; one merge at a time; key numbers again after a rebase |

## 9. Lines

Core at the round's start 10,444 (R0); at its end 10,473 (G1, B1, H1, L1); the ceiling 10,643, now 10,673 (phase Z, L4). The budgets below were for the steps as planned; G2, H5, H4 and the
cover were not kept or not built. Budgets: G2 about +6 net (`parts-law.md`); H1 about +25 (the
window, its field and the guard); H5 about +15; H4 by its probe (at most +20); the cover by `design/cover.md` 7 (C1a
about +2, C1 about +29 net, C2 about +115 net). Without C2 about +95 (to about 10,555), within the ceiling; with C2
about +210 (to about 10,670): C2 (or the step that crosses) raises the ceiling in its commit, with a decision (L31).

# Speed after the foundation: the plan

**Status:** the owner asked for this work on 24 October 2026: build ideas A to E of the speed list (group F waits).
The owner asked the architect to take every decision and log it: they are in `decisions.md` next to this file (S1,
S2, ...). What was built and measured goes in `docs/history.md` (section 5) and in `measures.md` next to this file.
Where this file and `docs/handover.md` differ, the handover wins until it is updated.

This file is for the agents who do the work and for the owner. It is written in simple English (handover 0).

---

## 0. In short

- **The aim:** a settled world runs **10 times faster** on the owner's 4-core machine, with the same laws and the same
  honest books. That is the owner's aim, and the targets of 1.1 are set by it.
- **What we expect, honestly:** with groups A to E, **about 3 to 5 times** overall. Bodies can become about 8 to 15
  times cheaper; the rest of a year (Flow, Change, parts, tape) about 2 times. The rest alone is already more than the
  whole budget of 10 times (section 1.3). Reaching 10 times needs group F (S1) or new physics algorithms; 1.3 says what
  each would add.
- **Where the time goes today** (`measures.md`, the baseline): bodies are about three quarters of a year (638 of 857
  ms on the island, 1,010 of 1,347 ms on the globe) and run on one core; one body update costs about 23 µs on the
  island and 39 µs on the globe; Flow is about 18%, Change about 5%; three cores of four sit idle outside Flow's and
  Change's threads. The profile is flat: no function but the save holds more than about 2.5%.
- **What we build**, in the order of their measured share (S32):
  - **A, measure and free wins:** the bench kit (done), compiler settings, several seeds side by side.
  - **E2, hot spots:** the few places the profile names (`spread::part_acts` placing and taking back bodies,
    `copy::select::sample`, `nodes::crowd`, one `apply` per update, `pow`), each its own commit.
  - **D3, the physics and the survey on threads:** a parallel lane beside B and C.
  - **B, fewer bodies:** one node per kind per cell for nodes without a brain, with its young inside; light that
    reads the sizes inside a node; cells that cannot fill without end, by a milder merge.
  - **C, fewer updates:** a closer step over a node's interval, so nodes update half as often; dormant bodies sleep.
  - **D2, all four cores for the bodies:** the world is cut into **domains** (the cells under one air node); each
    domain runs its bodies on its own thread in **batches** of days; every move to another cell and every effect
    outside the domain waits in an **outbox** booked in a fixed order. Built on a branch; merged only if it pays.
  - **D1:** the tape's writing off the main thread (about 1 to 2% of a year).
  - **E1 and E3, gated:** columns, and a stage path for bodies without a brain, only if a measure shows they can pay.
- **Each step measures its gain.** A step that does not pay is taken out again and logged.

---

## 1. The goal and how we measure it

### 1.1 The baseline and the targets

The baseline is `measures.md`, commit `16e6a4e`, ms a simulated year (the mean of 5 years):

| Bench | s a year | Flow | Change | bodies | parts | tape | bodies alive | updates a year | µs an update |
|---|---|---|---|---|---|---|---|---|---|
| `island22` (22 × 22, year 300) | 0.857 | 159 | 49 | 638 | 0 | 11 | 2,744 | 27,762 | about 23 |
| `globe36` (36 × 18, year 200) | 1.347 | 229 | 51 | 1,010 | 43 | 15 | 1,955 | 25,626 | about 39 |

| Target | Today | The aim (10 times) | What A to E likely give (3 to 5 times) |
|---|---|---|---|
| `island22`, s a year | 0.857 | **0.086** | 0.17 to 0.29 |
| `globe36`, s a year | 1.347 | **0.135** | 0.27 to 0.45 |
| 1,000 years of the 36 × 18 globe (bodies grow over centuries) | 36.6 min (`docs/reports/foundation-l.md`) | under 15 min | about 7 to 12 min |
| Largest planet in 15 minutes (the cost model of the foundation, plan 5.7) | about 290 cells | about 3,000 cells | about 900 to 1,500 cells |

A settled world is the world after its life has settled: its bodies per cell are what a long run has, not what year 10
has. "Faster" counts every gain: fewer bodies (B), fewer updates (C), more cores (D), cheaper updates (A, E2, E1, E3).

### 1.2 How we measure (A1 built this)

- **Bench worlds.** `scripts/bench.sh` makes `island22` (`ew run --size 22 --seed 1`, year 300) and `globe36` (`ew run
  --globe --size 18 --seed 1`, year 200) once for each snapshot version and world-changing step, and keeps them
  outside the repository (`$BENCH_DIR`, default `~/.cache/ew-bench`). It runs `ew resume` for 5 years from each, twice,
  keeps the lower, and prints per simulated year: Flow, Change, bodies, parts, tape (the `ms_*` columns), the bodies
  alive, the body updates and the seconds a year. Nothing else runs on the machine.
- **From D2.3 on**, the bench also prints the **busiest domain's share** of a batch's work (its updates over all
  updates of the batch, the year's mean; a counter of the tape, `docs/tape.md`), so an imbalance shows.
- **A refined region.** D2 gains less in a refined region: one air node holds 64 cells there and the densest life, so
  one domain holds much of the work. A refined-region bench (`region24`) is added in phase Z only (Z1), to report it.
- **Profile.** `scripts/bench.sh profile <bench>` runs one year under `valgrind --tool=callgrind` and prints the top
  functions by inclusive and exclusive instructions. Instruction counts do not depend on what else runs, so they
  compare two builds exactly. E1's gate adds `--cache-sim=yes` (section 4, E1).
- **The record.** Each step that touches speed adds one row to `measures.md`: the commit, both benches (s a year and
  each stage), bodies, updates, core code lines.
- **Few long runs.** The benches take seconds. Long runs (1,000 years) wait for phase Z.

### 1.3 The arithmetic of 10 times

The budget of 10 times is 86 ms a year on the island and 135 ms on the globe. The rest of a year (Flow, Change,
parts, tape) is **219 ms** on the island and **338 ms** on the globe: alone it is 2.5 times the whole budget. So even
infinitely fast bodies give at most 3.9 times (island) and 4.0 times (globe). For 10 times with bodies 15 times
cheaper (43 and 67 ms), the rest would have to fall to about 45 and 70 ms: 5 times cheaper.

What each group can give (the plan's guesses; `measures.md` holds the real numbers):

| Group | On the bodies | On the rest |
|---|---|---|
| A2, compiler settings | 1.1 to 1.2 | 1.1 to 1.2 |
| E2, hot spots | 1.2 to 1.4 (the spots hold about a third of the instructions; part of each can go) | — |
| B, fewer bodies | depends on B1's measure (section 4, B1); B2 matters on long runs | — |
| C, fewer updates | 1.3 to 2 (C2 halves the updates of nodes without a brain if it passes) | — |
| D2, four cores | 2.5 to 3 | — |
| D3, the physics on threads | — | Flow about 1.8; Change 1.2 to 1.5 (its reactor already runs on threads) |
| D1, the tape off the main thread | — | about 1 to 2% of a year |
| E1, E3 (gated) | unknown | — |

So bodies become **about 8 to 15 times cheaper** (B and C together 2.5 to 3.5, D2 2.5 to 3, E2 1.2 to 1.4), and the
rest **about 2 times** (A2, D3, D1; the parts' monthly step and the serial parts of Flow stay). The year:

| | bodies 8 times cheaper | bodies 15 times cheaper |
|---|---|---|
| island: rest halved (110 ms) | 80 + 110 = 190 ms: 4.5 times | 43 + 110 = 153 ms: 5.6 times |
| island: rest only 1.5 times cheaper (146 ms) | 226 ms: 3.8 times | 189 ms: 4.5 times |
| globe: rest halved (169 ms) | 126 + 169 = 295 ms: 4.6 times | 67 + 169 = 236 ms: 5.7 times |
| globe: rest only 1.5 times cheaper (225 ms) | 351 ms: 3.8 times | 292 ms: 4.6 times |

Hence "about 3 to 5 times" (S20). What would add more, beyond this plan:

- **Group F** (S1): *climate on repeat* (a settled climate replayed instead of solved) and *slow stores sped up*
  would cut Flow and Change, the part A to E cannot; *life as a field* (common kinds as densities per cell) would cut
  the bodies further; *a GPU* would take Flow's per-cell work and Change's reactors; *threads that may change the
  history* would put Flow's serial loops (the priority flood, the conjugate gradient's sums) on all cores.
- **New physics algorithms:** a cheaper solve of the air or of the liquid (fewer iterations, a better preconditioner)
  would cut the serial part of Flow that D3 cannot share.
- **More cores:** D2 and D3 gain more on 8 cores; the serial parts stay.

---

## 2. The rules of this work

### 2.1 The repository's rules hold

`CLAUDE.md` holds for every step: one place for each idea, no magic numbers, no dead code, no compatibility code, no
experiment switches, clear names, no Earth words in `core` and `worldgen`, clippy clean, exact books, comments that say
what and why (with a reference), no history in code, docs that describe the present.

The main rule of the foundation holds too (`docs/foundation/plan.md`, "The main rule"): **a general piece replaces the
special code it makes useless, in the same step.** No new pool, cache or catch-up for timing. Two functions of the same
shape become one. Every commit message ends with its line count: `Lines: +X -Y (core code N)`.

**The docs move with the code, in the same commit:**

- `docs/laws.md` changes in the same commit as each law change (B1, B3, B2, C2, C1, D2.2, D2.3, D2.4, E3).
- `docs/numbers.md` gets a row for each new setting in the step that adds it (`batch_days` in D2.2, the dormancy
  threshold in C1), and the rows of `crowd` and `node_days` change where B2, C2 and D2.2 change them.
- `SNAPSHOT_VERSION` (in `tape`) is bumped at every step that changes the saved world: B1, C1 (the body alarms), D2.1,
  D2.2, E1, and any step that adds a saved field.
- `docs/tape.md` changes when events or counters carry new ids or columns (D2.1's body ids, D2.3's busiest-domain
  share).
- `CLAUDE.md`'s table "Where things are" lists this folder (the architect adds it with this plan); Z4 checks it points
  to the final state.

### 2.2 The three rules for future smart bodies (the owner's wish)

The work must not close the door to smart, people-like bodies later (handover 13.2, "No people-like kind"):

1. **One place decides who may join whom.** Every merge path (`nodes::join_alike`, `nodes::merge_closest`, the kin
   joins of `copy`, `spread::part_acts`, the maturing of a young node into its adult node) asks one predicate,
   `nodes::may_join`, built in B2 with no change of behaviour beyond B2's rule (S7). Today's animal nodes are
   population nodes by handover 7.1 and keep merging as they do. When people-like minds come, "a mind that stays
   individual" is one test in that predicate (S30). A node can always be split back into individuals (`nodes::open`
   stays and is tested).
2. **The step length is chosen per place.** The batch length (D2.2) and the domain clock are built so that a domain
   could later run with a shorter batch (a focus zone, handover 7.3), without a new design. Today one value holds for
   all domains.
3. **Two kinds of body code.** B1 and E3 are for bodies without a brain. Bodies with a brain keep their young nodes
   and a per-body path. Nothing in B1 or E3 may assume that every body takes the same branches.

### 2.3 Checks for each step

| Check | When |
|---|---|
| `cargo build --release --workspace` | every step |
| `cargo test --release --workspace` (the normal suite, under about 30 s, with the golden test) | every step |
| `cargo clippy --release --workspace --all-targets` with no warning | every step |
| `cargo build -p pcore --target wasm32-unknown-unknown` | every step |
| `cargo test --release -p tape --test golden` (the golden test without threads) | every step that touches threads or order |
| `scripts/bench.sh` (both benches, 5 years each) | every step that touches speed |
| The key-number test (`tape/tests/ranges.rs`, about 10 minutes today, faster later) | once in each level-3 step that changes a law or a speed setting: B1, B3, B2, C2, C1, D2.2, D2.3, D2.4, E3 (S33) |
| The equivalence test of a new speed setting (× 2 and ÷ 2) | in the step that adds or changes it |
| The slow suite (`cargo test --release --workspace -- --ignored --no-fail-fast`) | at the end of each phase |

**Tests are few and fast** (`CLAUDE.md` 5). A new test states a law or an invariant. A step that deletes a mechanism
deletes its tests.

**Check levels** are those of `docs/foundation/plan.md` 6.1: level 1 (bit for bit), level 2 (the same to rounding,
for speed work that changes the order of sums), level 3 (the same world: key numbers in their ranges). Each step names
its level. A level-2 or level-3 step re-pins the golden test in the same commit and says why.

**Merging and the golden test.** Steps merge one at a time. A step that re-pins the golden test rebases on the `main`
it merges into, runs again and re-pins there, never on a stale base. B3 and B2 both re-pin it, so they run one after
the other.

### 2.4 The code ceiling

`core` holds 8,998 code lines; the ceiling is 9,000 (`docs/foundation/decisions.md` 42). Never pack lines to save
count; comments are not counted (`CLAUDE.md` 2.11). The rules (S3):

- **Before D2.1 everything lives under 9,000:** A, E2, D3, B and C. B and C may balance between them: their net is at
  most zero together. Every other step pays for its own lines.
- **The estimates of B and C:** B1 about +60 (two parts in one body and the size rules by part; nothing of young nodes
  goes, as nodes with a brain keep them); B3 about +20; B2 about -10 (one predicate replaces the tests of `kin_of`,
  `merge_closest`'s distance, `copy::kin` and `young::linked`; the milder rule adds a few); C2 about +30 to +50 (by
  the method chosen); C1 about +20 (a class of alarms in `clock::Sleep`, the wake check in the survey). About +120 to
  +140 in all: B and C must find that much to simplify in the files they touch. A step that would pass the ceiling
  stops and reports before it is merged.
- **D2 on a branch.** D2 is built on the branch `speed/d2`, each sub-step committed and pushed there. The D2.1 commit
  on that branch raises the ceiling to 9,700 (decision 42's allowance). The branch merges into `main` only when D2.5
  shows its gain. If D2 is killed, the branch is not merged and the ceiling stays 9,000. The estimates: D2.1 about +110
  (domains, ids, the id table; `Life::genomes` goes), D2.2 about +20, D2.3 about +200 (the context, the store view, the
  faces), D2.4 about +150 (the outbox and the serial phase), D2.5 about +20, D2.6 about +40: about 540.
- **E has no extra allowance.** E1 and E3 live under 9,700 and simplify as much as they add.

### 2.5 How a step is worked and closed (the agents)

The architect plans, splits the work, reviews and merges. For each step:

1. **Context:** a Haiku agent gathers facts (call sites, numbers, profiles) when the step needs a wide search or a
   measure before it is built (B1, C1, C2).
2. **Build:** an Opus agent for complex code (B1, B3, C2, D2.1, D2.3 to D2.5, E1, E3, the hot spot of `part_acts`), a
   Sonnet agent for normal code (A, the other hot spots, B2, C1, D1, D2.2, D2.6, D3, docs) (S19). Steps on different
   files run in parallel, each in its own git worktree.
3. **Check:** the builder runs the checks of 2.3 and adds the bench row.
4. **Review:** at the end of each phase (and of D2's big sub-steps), an adversarial Opus agent reviews the diff against
   this plan, `CLAUDE.md` and the handover. A fixing Opus agent answers every finding (fix it, or say why not). They
   loop until the reviewer approves.
5. **Close:** the architect re-reads the diff, then commits to `main` and pushes (D2.1 to D2.5: to `speed/d2`). One
   line per change goes in `docs/history.md`; a decision taken goes in `decisions.md`.

### 2.6 When a step fails its goal

A step that does not reach its "done when" stops. If it does not pay (its gain is under its kill line), it is taken
out again and gets one line in `docs/history.md` under "tried and not kept". If it pays but breaks a check, the cause is
found and fixed; a range or a tolerance is never loosened to pass (`CLAUDE.md` 5.5).

---

## 3. The architecture after this work

### 3.1 The picture

```
  world step (one month)
  ├─ survey            every phys step: per domain on threads (wear, crowd, shade, sums)        [D2.6]
  ├─ Flow, Change      per-cell work on threads, sums in a fixed order                           [D3]
  ├─ Parts (Bond)      once a month, serial over all domains, as now                             [S23]
  └─ bodies            every batch of `batch_days` days                                          [D2.2]
        ├─ batch start: a face of every body, the ids in every cell                              [D2.3]
        ├─ parallel:    each domain runs its due bodies in its fixed order, on its own bodies    [D2.5]
        │               and parts, its own store view and its own outbox
        │                  └─ a body without a brain: the stage path, if E3 is built             [E3]
        │                  └─ a body with a brain: the per-body path
        └─ serial:      each domain's moves booked by one `apply`, then the outboxes,            [D2.4]
                        in a fixed order (3.6)
```

### 3.2 Domains: what each one owns (D2)

- A **domain** is one air node and the ground cells under it (4 × 4 cells today; 64 in a refined region) (S17).
  Domains are fixed when the world is made. Their order is the order of the air nodes.
- **What a domain owns and writes in the parallel phase** (its state; nothing else is written there):
  - its **bodies** (slots in the domain), its cells' lists of body ids, and its **parts**: the parts of its cells,
    with their per-cell lists (S23);
  - its **ring of updates due** (and, after C1, the alarms of its sleeping bodies); a body that moves to another
    domain is taken out of its ring and filed in the new domain's ring, at the same due day, in the serial phase;
  - its **store view** and its moves, in place of `World::moves` (3.4);
  - its scratch: a `Reactor`, a `LiveTmp`, an `ActTmp`;
  - its **events** and its **counters** (f64 sums, merged into `World::counts` in domain order);
  - `Life::shade` (per body) and `Life::stale` (per cell) for its own bodies and cells;
  - its counters of new body ids and part ids, its **provisional lineages** with their brains, and its deltas of
    lineage member counts (3.3);
  - its **outbox** (3.6).
- **What is written only in the serial phase:** the id table (id → domain and slot), `Life::brains` and
  `Life::members`, `Life::pushed`, `Links::cleared`, `World::counts.crossings`, the bodies and parts that change
  domain, and the world's event list.
- **What the parallel phase reads, unchanged during the batch:** the faces and the cells' id lists (3.5), the survey's
  caches (`Life::sum`, `env`, `seen`, `cellf`, `absorb`, `keep`, `World::canopy`), `World::light` and `rained`,
  `Links::cleared`, Flow's last state (`scratch`), the global lineages, the chemistry and the config.
- **Flow writes bodies** (`mark`, `draw.owed` through `survey::Draws::owe`, the bodies' node's heat): those loops go
  over the domains, outside the batches, and `Draws::owe` writes each domain's bodies.
- **Same history with or without threads.** The parallel phase reads a world that does not change, each domain is
  sequential inside, and the serial phase has one fixed order, so the result does not depend on which thread ran a
  domain or when. The golden test checks it with and without threads.

### 3.3 Ids: bodies, parts, lineages

- **Bodies.** A body gets an id at birth that never changes: its domain of birth and that domain's counter (S10). A
  body that moves keeps its id. The id table finds where it lives; it is written only in the serial phase, so a body
  born in the parallel phase is found through its own domain until then. Memory of individuals (`Body::known`) and the
  rng keys read the id, so a move between domains changes nothing for them.
- **Parts.** A part gets a stable id the same way (domain of birth and a counter). `Part::held` keeps the holder's
  body id (S23).
- **Provisional lineages.** A lineage made in the parallel phase (`lineage::resize`, `copy::child_lineage`) gets a
  provisional id of its domain; the domain keeps its brain, and every read of a lineage's brain looks there first. The
  serial phase maps provisional ids to global ids, in domain order and then in the order made, and rewrites the ids
  its bodies and its outbox hold. Lineage member counts and a lineage dying out are settled only in the serial phase,
  so a brain stays readable for the whole batch (S24).

### 3.4 The store view (D2.3)

- **What it holds** (S26): the Ground, Bodies and Bed nodes of every cell of the domain and the domain's air node, as
  the moves of this batch leave them (as `store::Moves` keeps one cell's nodes for one update today), and a per-domain
  delta of the Potential node (the lift paid when gas comes down from the air, `Moves::gas`).
- **Every direct read of the store in body code goes through the view.** Today these are:
  - `World::g`, `World::a`, `World::a_seen` and `World::seen_now`, `World::heat_now`, `World::take_from_ground`
    (through `store::Moves` today);
  - `World::b` and the skin in `World::heat_cap_ground` (the Bed: the body's physics and `feed::take_node` read
    it), and `World::thaw` (the ground's heat, read straight from `w.store`, where `World::t_ground_last` falls back
    to it before Flow has run);
  - `bond::anchor` and `bond::dig` (the Bed's amounts, `bond/verbs.rs`).
  The builder of D2.3 lists them again with `grep -n "w\.store\|self\.store\|\.store\.amounts\|\.store\.heat"`
  over `body/`, `choose/`, `copy/`, `bond/verbs.rs`, `nodes.rs`, `lineage.rs`, `ground.rs` and `world.rs`, and the list
  goes in the commit message. Reads outside the batches (`survey::Draws::settle`, `bond::month`) stay as they are.
- **No overdraw.** A domain's bodies take from its nodes in order, each seeing what the bodies before it left. A debug
  check holds that a take from the view never passes what the view holds.
- **Booking.** The batch's moves of a domain are booked by one `apply` per domain in the serial phase (D2.3, level 3:
  one `apply` for a batch changes the order of sums and what rounding sets to zero).

### 3.5 Faces (D2.3)

- **Each batch starts with a face of every body** (S14): id, cell, members, lineage, visible traits (`Body::vis`),
  signal, well-being, and the root and closedness of the room it is in (`bond::room`); about 64 to 80 bytes a body.
  With them, for every cell, the list of the ids in it at the batch's start. Faces are made serially or per domain;
  either way before any update of the batch.
- **Perception reads only faces and those lists** (`choose::perceive`, `inputs`' signals heard, `options`' features
  of a body seen, `Body::known` by id), whether the other body is in the same domain or not. `Choice::seen` holds ids,
  not slots.
- **Acts on another body.** Taking and giving (`take_body`, `feed::give`) are in the same cell, so the same domain:
  they resolve the id in the domain and act on the live body; if it is gone, the act fails as today. The partner of a
  mating (`copy::mix::partner`) and the others of a cell in `body::exchange` are in the same cell too and are read
  live.
- **Watching** (`learn::watch`) reads the other's whole brain rows, so it always goes to the outbox and is applied in
  the serial phase on the live target. No rows are copied into faces.

### 3.6 The outbox and the serial phase (D2.4)

**Every relocation of a body or of members to another cell goes through the outbox and lands at the batch's end,
inside the domain as well as across its edge** (S22): one path, so a cell at a domain's edge behaves like any other.
Until it lands, the body stays in its old cell: the rest of its update (selection, `lineage::resize`, `copy::mature`,
`nodes::split`, `World::see`) runs in its old cell, in its own domain. `choose::go` pays its cost and returns that it
queued a move; `spread::part_acts` treats a part that queued a move as leaving the node (today it reads `m.cell`).

The kinds, in the order the serial phase applies them:

| Kind | Made by | Carries | Applied in the serial phase | If its target is gone |
|---|---|---|---|---|
| Relocation | `choose::go` (a body, or a part placed by `part_acts`) | body id, from cell, to cell, the structure it carries (root), the energy spent pushing, the lane area to clear | the body leaves the old cell's list for the new one's (id table, rings); `cell`, `mark`, `rained` of the new cell, `inside` none; the carried structure moves and the parts there are pushed (`bond::moved`); `counts.crossings[j] += n`; `Life::pushed[j] +=` the energy; `lanes::clear` on `Links::cleared` | the body died later in the batch: the relocation is dropped whole; its cost was paid and spent; a structure it let go of when it died stays where it was |
| Arriving members | `spread::part_acts` (members that move) | a relocation, marked "look for kin" | after the relocation lands: the kin search in the new cell (`copy::kin`, through `nodes::may_join`) and the merge (`nodes::merge`); no kin: it stays a body of its own, as today | the part is gone: nothing lands, nothing joins |
| Litter | `copy::emit`, children who start in another cell | the cell the parent was in, the target cell, members, stores and heat (taken from the parent by `give`), genes, spread, size, rows, lineage, parent id, whether it may be a node, whether it is mixed | the kin search in the target cell; it joins (`nodes::add`) or is placed (`place_child`); the `Born` event is made here (S25) | no kin and less than one member: back into the parent's brood if the parent lives (by id: stores, heat and brood back), else to the ground node of the cell the parent was in, as a move |
| Watch | `learn::watch` | watcher id, the share of its members who watched (1 for an individual, x / count for a node's part), target id | if both live and their brains have one shape: the watcher's rows move by `blend` × alike × that share toward the target's (alike from the target's `vis` and the watcher's `visible`); the `Watched` event. The blend is linear, so this is the mean by members that `nodes::take_in` makes today | either is gone: nothing (the turn was spent) |
| New lineage | `lineage::resize`, `copy::child_lineage` | provisional id and its `Lineage` | mapped to a global id; ids rewritten (3.3) | cannot fail |
| Lineage counts | every body placed or removed | per lineage, the change of its bodies | added; a lineage at zero dies out (`Life::brains` none) | cannot fail |
| Events and counters | all body code | the domain's list and sums | appended and added in domain order | cannot fail |

**The serial phase, in this fixed order:**

1. Each domain's moves are booked by one `apply`, in domain order.
2. Provisional lineages get global ids, in domain order and then in the order made; ids are rewritten.
3. The entries of the outboxes (relocations, arriving members, litters, watches), domain by domain in domain order,
   each domain's in the order they were made.
4. The moves the serial phase made (litters given to the ground) are booked by one `apply`.
5. Lineage counts are added in domain order; a lineage at zero dies out.
6. Events and counters are merged in domain order, then the serial phase's own events in the order made.
7. The id table and the rings are final; the next batch starts with new faces.

**What changes in the world (level 3):** bodies are seen as they were at the batch's start; every move to another
cell and every effect of a watch land at the batch's end. This is the same kind of lag a body already has when it sees
far cells "as the last survey saw them" (handover 5.6 of the foundation). A fractional litter that disperses is now
made and may come back (S25).

### 3.7 Batches (D2.2)

Bodies update in batches of `Config::batch_days` days. The full description is in D2.2 (section 4), the one place
for it (S8).

### 3.8 One node per kind per cell, for nodes without a brain (B1, B3)

- Today a kind in a cell is an adult node and a young node joined by a link. After B1, **for nodes without a brain**,
  a kind in a cell is **one node** with two parts: the adult part (its members at the adult size) and the young part
  (its members below the copy mass, with the size moments the young node keeps today: `Body::size_spread`,
  `size_lag`). **One store**: the parts share the node's stores, heat and brood; they differ only in members and sizes
  (S29). Nodes with a brain keep their young nodes (rule 3 of 2.2).
- Children who stay join the young part and maturing moves members from the young part to the adult part (the same
  `math::Pass` as `copy::young::mature`); with one store, neither moves any matter.
- What depends on a member's size reads the part's member size, where the difference matters (Jensen's inequality on
  a power law). The list is in B1.
- B3: the light of a cell passes the young part and the adult part of each node as two layers by their heights
  (Purves et al. 2008, the perfect plasticity approximation; Moorcroft et al. 2001).

### 3.9 Who may join whom (B2)

One predicate, `nodes::may_join`, answers for every merge path (rule 1 of 2.2). Past `crowd`, the most alike pair of
one lineage merges when the merged node's pooled spread stays within its cap on every gene (S7).

### 3.10 The physics and the survey on threads (D3, D2.6)

Per-cell work that today runs in a serial loop runs through `par::rows` (rows made in parallel, then summed in the
fixed order of the cells). Sums across cells keep their order, so the history is the same bit for bit (level 1). D3
does it for Flow and Change; D2.6 does it for the survey, per domain.

### 3.11 Columns and the stage path (E1, E3: gated)

- E1, only if its gate passes: inside a domain, bodies are columns (the scalars of all bodies in arrays, the stores in
  one array of bodies × substances, the genes and spreads in one array each, the variable parts per body). Code reads
  a body through `BodyRef` and `BodyMut`; field writes become methods where the columns need them, so the laws' code
  does change. Same arithmetic in the same order: level 1.
- E3, only if its gate passes: the update of the due bodies without a brain of a domain runs stage by stage over all of
  them, each stage a short loop. Bodies with a brain keep the per-body path (rule 3).

---

## 4. The work list

Each step has an ID, a check level, a "done when", a kill line (when it does not pay), its line estimate and the
agent that builds it. "Parallel with" names the steps that can run at the same time in their own worktrees. The steps
are listed in the order they are built (section 5).

### Phase A: measure, and the free wins

#### A1. The bench and profile kit (done)

- **What:** `scripts/bench.sh` as in 1.2; `measures.md` with the baseline of both benches and a callgrind profile of
  each; `docs/history.md` section "5. Speed after the foundation". Done at `0a3d1e6`.
- **Kept for later steps:** the bench remakes its snapshots only when asked (`--remake`) after a snapshot version or a
  world-changing commit; it runs each bench twice and keeps the lower.

#### A2. Compiler settings

- **What:** in `Cargo.toml`'s release profile, `lto = "fat"` and `codegen-units = 1`; in `.cargo/config.toml`,
  `target-cpu=native` for the native target only (not wasm). Then profile-guided optimisation (PGO) tried with
  `cargo pgo` or `llvm-profdata`, trained on the two benches.
- **Why:** the compiler can inline across crates and use the CPU's newer instructions.
- **Challenges:** the golden fingerprints may move (other instructions, same maths in most places); then they are
  re-pinned at level 2 and the commit says why. `target-cpu=native` makes the fingerprints depend on the machine:
  handover 12.5 asks the same history only on the same machine and build, so this is allowed; the golden test is
  pinned on the owner's machine (S34). PGO needs a tool and a second build: it stays only if it gains at least 8% on
  the benches and its script is short (`scripts/pgo.sh`); else it is logged as not kept.
- **Check level:** 1 or 2. **Done when:** each setting's gain is in `measures.md`; the kept ones are on `main`.
- **Kill line:** a setting that gains under 2% is not kept. **Code:** no core line.
- **Agent:** Sonnet.

#### A3. Many worlds at once

- **What:** inside one process rayon's pool is shared, so the seeds of `tape/tests/ranges.rs` (one `std::thread` each)
  already share Flow's and Change's four workers and do not oversubscribe the cores; nothing changes there. For
  separate `ew` runs side by side, A3 measures three seeds both ways: each run with its own pool, and each run with
  `RAYON_NUM_THREADS=1` (which loses Flow's and Change's threads). It writes the better way, with the numbers, in
  `measures.md` (S34).
- **Why:** the owner's work runs three seeds; on four cores that can be nearly three times faster for free.
- **Check level:** none. **Done when:** the measure and the advice are in `measures.md`. **Agent:** Sonnet (small).
  **Parallel with:** everything.

#### A4. A faster machine

Not code: the report tells the owner what a newer CPU would give (from the profile's instruction counts and the
bench's instructions per second) (S5).

### E2. Hot spots (right after phase A)

- **What:** the places the baseline profile names, each its own commit with its measured gain (S31). Each is built
  after A2, on the profile of the A2 build:
  1. **`spread::part_acts`** places a body for each option part and takes it back (`nodes::take_in` with
     `nodes::add`: about 14% of the globe's instructions). Where the part stays (same cell, holding nothing, inside
     nothing), act on the members without placing a body and without taking them back through `add`'s gene, size
     and row pooling. Level 1 where the sums stay, else 2 (the mean of equal genes is exact; size moments may round).
     Agent: Opus.
  2. **`copy::select::sample`** (6.4% on the island): the sample member's update; find the work it repeats from the
     node's own update in the profile and do it once. Level 1.
  3. **`nodes::crowd`** (6% on the island): find what each month rebuilds for every cell (its kin, its pairs) and
     rebuild only what changed. Level 1 (the same merges in the same order).
  4. **One world `apply` per update** (6 to 8%, `World::book_moves`): a direct path for an update's few moves that
     makes the same sums in the same order. Level 1, else 2. D2.3 later books one `apply` per domain and replaces this
     path; it is built anyway because D2 may be killed, and it must stay small.
  5. **`pow`** (2.4% on the island): powers with a fixed exponent (mass^0.75, `10^x`) as cheaper forms. Level 2 (other
     rounding).
- **Why:** these spots hold about a third of the instructions (inclusive shares, which overlap); the profile is flat
  elsewhere.
- **Done when:** each spot's gain is in `measures.md`. **Kill line:** a spot that gains under 2% of a bench is taken
  out again.
- **Code:** each spot pays for its own lines (under 9,000). **Agent:** Sonnet, but Opus for `part_acts`.
  **Parallel with:** A3, D3.

### The parallel lane: D3

#### D3. The physics and the survey on threads

- **What** (S21):
  - **Flow:** the per-cell loops that run serially today run through `par::rows`, their cross-cell sums kept in the
    fixed order of the cells: the air step's `fixed`, `room`, `seen`; the air's `gather` per air node over its cells in
    cell order; land's `ground_state` and `surface`; liquid's `slopes`, `mobile`, `surface`'s rates and `follow`. The
    priority-flood order, the downhill pass and the conjugate gradient's dot products stay serial.
  - **Change:** its reactor already runs on threads; its serial parts (noting each node's row in `Batches::add`,
    booking) go on threads where their sums keep their order; and a profile-driven look at `Reactor::react` (2.4%
    exclusive, about 9% inclusive; slice indexing alone 1.2%), keeping results bit for bit where possible. A change of
    the reactor's arithmetic is level 2 and its own commit.
  - **The survey's** per-cell loops are done in D2.6, per domain.
- **Why:** Flow is 17 to 19% of a year and uses one core outside the air's `pass`; Change is 4 to 6%. The rest of a
  year must shrink too (1.3).
- **Challenges and answers:** a sum across cells must keep its order to stay bit for bit: each parallel task writes its
  own row; the sums are made afterwards in the old order. The golden test runs with and without threads.
- **Check level:** 1 (bit for bit), but 2 for a change of the reactor's arithmetic. **Done when:** the golden test
  keeps its fingerprints with and without threads; Flow is at least 1.8 times faster on 4 cores on the benches.
  **Kill line:** a loop that gains under 1% of Flow (or of Change) stays serial.
- **Code:** pays for its own lines (under 9,000). **Agent:** Sonnet. **Parallel with:** E2, B, C (it touches `flow/`,
  `solve/` and `change/` only).

### Phase B: fewer bodies

#### B1. One node per kind per cell, its young inside it (nodes without a brain)

- **Measure first** (a Haiku agent, both benches): the share of bodies that are young nodes without a brain, the share
  of updates they take, and how many of them have no adult node of their kind in their cell (S29). Call `s` the share
  of bodies that are young nodes without a brain **with** an adult node of their kind in their cell, and `u` their
  share of updates. B1's target is set from these: those young nodes go into their adult node; young nodes without an
  adult of their kind become the node of their kind holding only young (no fewer bodies there).
- **What:** as in 3.8, for nodes without a brain only. A node has an adult part (members at the adult size) and a
  young part (members below the copy mass, with the density of `copy/young.rs`: log10 size, `e^(-k y)` fitted to its
  mean and variance). One store, one heat, one brood. Children who stay join the young part (members and size moments;
  their matter leaves the brood for the body inside the same store). Maturing moves members from the young part to
  the adult part by `math::Pass`. Brood is made from the adult part's growth only (`ALLOC`).
- **Why:** young nodes update as often as adults; one node per kind halves the bodies of a kind where both live.
- **Design points:**
  - `Body` gains the adult part's members (or the young part's); `size_spread` and `size_lag` describe the young part.
    For nodes without a brain, `copy::young::{linked, link}`, `Grown::Joins` and the stage in `nodes::Kin` are no
    longer used; they stay for nodes with a brain.
  - Growth (`copy::grow`) splits what the node builds between its parts by their supply (members × mass^0.75 at each
    part's size, Kleiber), so the young grow at a young member's pace and the adults fill their brood.
  - Upkeep, costs by kg, the store checks and death by wear run on the node's totals as now (they are linear in mass).
  - **The size rules that must read a part's member size:** `Body::adult()`, `Body::member_body`, `live::starved`,
    `copy::copy`'s `BROOD_HELD * n` (adult members only), supply by mass^0.75 (`Body::supply`), `take_body`'s
    per-member force and reach, `Body::height`, `Body::visible`. The builder lists every use of `member_body` and
    `size` (a Haiku agent) and assigns each to "the node's totals" or "a part's member"; the list goes in the commit
    message.
  - The selection sample (`copy::sample`) is drawn from one of the two parts with a chance by members, at that part's
    size, so selection still sees both stages.
  - Children who start in another cell, and children of individuals without a brain, join the node of their kind
    there (its young part) when they are alike (`copy::kin`, which B2 folds into `nodes::may_join`), else start a
    node.
- **References:** Lefkovitch 1965 and Caswell 2001 (stage-structured populations); McKendrick 1926 and von Foerster
  1959 (the flow across a size); Kleiber 1932; Weiner 1990 (size hierarchies in plant stands).
- **Challenges and answers:**
  - *The books.* With one store, births that stay and maturing move no matter, so they cannot break the books; the
    tests that hold the books of young and adult nodes without a brain are rewritten for one node.
  - *The nodes check (F7).* It may move. It is run once at the end of phase B and its result written in the report; a
    rule that newly fails is a bug to find, not a range to widen.
- **Check level:** 3. **Done when:** on both benches the bodies alive fall by at least 0.8 `s` and the updates by at
  least 0.8 `u`; the key numbers hold; the pace test holds.
- **Kill line:** if the bodies' time on the benches falls by less than half of `u`, stop and report.
- **Code:** about +60. Snapshot version bumped; `docs/laws.md` (4.4, the nodes) changes. **Agent:** Opus.
  **Parallel with:** D3.

#### B3. Light and selection read the sizes in a node

- **What:** the survey's shade (`survey::shade`) puts each node's young part and adult part as two layers at their own
  heights (`Body::height` at each part's member size); a part's light per capture area is that of its layer. The
  selection sample reads the light at the sample member's height from the cell's sorted layers, so a gene that makes
  a member taller or shorter changes the light it gets (proposal 10.3 #2 of the foundation plan).
- **Why:** B1 puts young and adults in one body; without this, a node's young would get the adults' light. With it,
  the frame (height) is seen by selection, which today sees only its cost (decision 52).
- **References:** Purves et al. 2008 and Strigul et al. 2008 (the perfect plasticity approximation); Moorcroft et al.
  2001 (the ecosystem demography model).
- **Check level:** 3. **Done when:** a test holds that the shorter part of a node gets less light per area than its
  taller part under a closed canopy; the key numbers hold.
- **Code:** about +20. **Agent:** Opus. **Depends on:** B1. **Then:** B2 (both re-pin the golden test).

#### B2. Cells that cannot fill without end

- **What** (S7):
  1. **One predicate.** `nodes::may_join(w, a, b)` answers "may these join?" for every merge path: `join_alike` and
     `merge_closest` (the monthly crowd), the kin joins of `copy` (`copy::kin`), `spread::part_acts`'s members that
     move, and the maturing of a young node with a brain into its adult node (`copy::young::linked`). It holds the
     tests that are spread over `kin_of`, `merge_closest`'s distance, `copy::kin` and `young::linked` today, with no
     change of behaviour (level 1 for this part).
  2. **The milder rule.** Past `Config::crowd` nodes at a month's start, the most alike pair of one lineage in a cell
     merges when the merged node's pooled spread (the variance of a mixture: within plus between, `nodes::add`) stays
     within its cap (`body::most`) on every gene. Unlike nodes beyond that stay apart.
- **Why:** on long runs, nodes of one lineage that differ in their genes pile up (up to 73 in a cell; the refined
  region's bodies grew from 2,891 to 11,536 over 1,000 years, and a year's cost with them). A merge under the cap
  destroys no difference beyond what one node can hold (handover 7.1's "similar states").
- **What it does not do** (S30): it does not exempt bodies that learned. Today's animal nodes are population nodes by
  handover 7.1 and keep merging as they do; "a mind that stays individual" is one test in `may_join` when people-like
  minds come. With `--individuals`, no body with a brain merges, as today.
- **Challenges and answers:**
  - *Merge, split, merge.* `nodes::split` already needs room in its cell; a merged node cannot split again while the
    cell is full. A test holds that after the month's crowd, no pair of one lineage in a cell past `crowd` could still
    merge under the rule.
  - *The world must not change past its noise:* the `crowd` equivalence (× 2 and ÷ 2, handover 7.1's noise).
- **Check level:** 3. **Done when:** measured on the benches (the most bodies in a cell, the bodies alive) and in phase
  Z on a 1,000-year run of the refined region. If the milder rule does not bound the bodies, B2 reports with numbers;
  a stronger rule needs a new decision.
- **Code:** about -10. `docs/laws.md` 4.4 and the `crowd` row of `docs/numbers.md` change. **Agent:** Sonnet.
  **Depends on:** B3 (one after the other).

### Phase C: fewer updates

#### C2. A closer step over a node's interval (node_days 36 → 72)

- **What:** find what makes nodes that update rarely differ from nodes that update often (the pace test fails at 144
  days: births -55%), and approximate those terms better over the interval. Then double `node_days`.
- **Why:** a node without a brain updates every `node_days` (36). At 72 its updates halve.
- **How** (S28):
  1. **Measure: a gate.** A Haiku agent runs the pace test's worlds at 36, 72 and 144 days and gathers births, deaths
     by cause, growth and capture area per update, against the same nodes updated every `MIN_INTERVAL` days, and
     writes where the error comes from. **The method is chosen after this measure**, by the architect, and logged.
  2. **The method: a closer approximation**, for the term the measure names. For example:
     - growth that compounds through the capture area during the interval (a node that grows catches more light
       before its next update): an exponential integrator `(e^z − 1)/z` on the node's last specific growth rate
       (exponential time differencing; Cox and Matthews 2002). Then **the survey's set-aside must project each node's
       area over its interval**: the light the survey sets aside for a node (`survey::claims`) is made for the area
       the node will have, so a claim of the integrator stays inside what was set aside and the bodies' node is never
       overdrawn. A test holds it;
     - births and maturing during the interval: the node's two-stage flow over its interval (Caswell 2001), in closed
       form for the linear part.
     Its error is measured at 36, 72 and 144 days and written in `measures.md`. It is not an exact solution: the node
     is not linear.
  3. Set `node_days` to 72; the pace test then runs its × 2 and ÷ 2 (36 and 144).
- **Check level:** 3. **Done when:** the pace test passes at 36 and 144 around `node_days` 72; the key numbers hold.
- **Kill line:** if the pace test cannot pass at 144, `node_days` stays 36 and the method stays only if it makes the
  pace test closer at 72; the rest is logged.
- **Code:** about +30 to +50. `docs/laws.md` and the `node_days` row change. **Agent:** Opus. **Depends on:** B1 (the
  node's stages live in one body).

#### C1. Dormant bodies sleep

- **Measure first** (S12): a Haiku agent counts, on both benches, the share of updates of bodies without a brain
  whose activity (`Body::activity`, laws.md 6.5) is under 0.05. **Kill line:** under 5% of updates, C1 is not built
  and is logged.
- **What** (S27): a node without a brain whose activity at its update is under the threshold (dry or frozen) sleeps.
  C1 extends `clock::Sleep`, the one sleep and wake of the world, to body nodes: an alarm per sleeping body, at most
  `MAX_INTERVAL` ahead. At each survey, a sleeping body is woken through `Sleep`'s wake when its cell's ground
  temperature and liquid (`Life::env`) would raise its activity past the threshold, as store nodes wake by
  `wake_share`. A woken or due body is filed in the ring for the next batch and integrates the whole time since its
  last update, as every update does. No second mechanism.
- **The threshold** is a new speed setting (working name `Config::dormant`, a share of full activity) with its row in
  `docs/numbers.md` and its test (× 2 and ÷ 2 against no sleep), in this step.
- **Why:** a dry or frozen body's chemistry is nearly stopped (handover 4, "a dry body is nearly dormant"); updating it
  every interval costs time and changes nothing.
- **Check level:** 3. Snapshot version bumped (the alarms of bodies are saved). **Code:** about +20.
  **Agent:** Sonnet. **Depends on:** C2.

C3 (batches of days) is built as D2.2, its first use (section 4, D2.2).

### Phase D: all four cores

#### D2. Domains: the bodies on all cores (on the branch `speed/d2`)

Built in sub-steps on the branch `speed/d2`, each committed and pushed there when it passes its checks (S3). The
adversarial review runs after D2.3 and after D2.5. The branch merges into `main` only when D2.5 shows its gain;
D2.6 is then built on `main`.

- **D2.1 Domains own their bodies and parts (sequential).** `Life` holds a `Domain` per air node: its bodies, its
  cells' lists, its due ring (and the body alarms of C1), and the parts of its cells (S23). Stable ids for bodies and
  parts (3.3) with the id table. Each body with a brain keeps its own born rows (`Life::genomes` and its counts go,
  S11). Bodies still run one after another, in the order of their cells. Level 3 (new ids key the random draws).
  Snapshot version bumped; `docs/tape.md` (the ids in events). **This commit raises the ceiling to 9,700.**
- **D2.2 Batches (C3).**
  - **What** (S8): bodies update in **batches** of `Config::batch_days` days (5, the shortest interval
    `MIN_INTERVAL`). A batch runs, at its first day, every body due in it. `survey::day` runs once a batch.
  - **A body's interval is a whole number of batches:** its `INTERVAL` gene's days rounded to the nearest multiple of
    `batch_days`, at least one batch (`Body::interval` and the gene constant `Body::k.interval`). So every due day is a batch's start,
    no body runs early, and every law that reads `b.interval(w)` reads the real gap; there is no jitter.
  - `node_days` must be a multiple of `batch_days`: this step sets it to the nearest multiple (70 if C2 kept 72, else
    35). `MAX_INTERVAL` (360) is one already.
  - `Config::validate` refuses a `batch_days` that does not divide `phys_days` and the month, and a `node_days` that is
    not a multiple of it.
  - A body placed during a batch (`World::add`, `World::place`, children, splits, parts placed by `part_acts`) is first
    due at the next batch's start or later. The ring of updates (`life::RING`) is kept by batch.
  - **Why:** a day holds 1.5 to 15 ms of body work: too little to share between threads (decision 59 of the
    foundation). A batch of 5 days holds five times more, and D2 needs one batch as its unit of work.
  - **Cost:** short intervals lose resolution (6 and 7 days both become 5); the equivalence test checks it.
  - **Check level:** 3, with its equivalence test: `batch_days` 1 (today), 2 (÷ 2, as 2.5 is not a whole day), 5 and 10
    (× 2), each with `node_days` at its nearest multiple, give the same key numbers to the noise. Snapshot version bumped; `batch_days` gets its row in
    `docs/numbers.md`; `docs/laws.md` says intervals are whole batches.
- **D2.3 The update runs on its domain.** The update code (`body/`, `choose/`, `copy/`, `nodes.rs`, `lineage.rs`, the
  body side of `bond/`) takes a context that holds the world read-only and its domain writable: its bodies and parts,
  its store view (3.4), its scratch, its events and counters, its per-domain state (3.2). Each batch starts with the
  faces (3.5); perception reads them and `Choice::seen` holds ids. The batch's moves of a domain are booked by one
  `apply` per domain after the batch. The bench prints the busiest domain's share (1.2). Writes outside the domain
  still happen at once here (one thread). Level 3 (faces from the batch's start; one `apply` a batch).
- **D2.4 The outbox, whole.** Every kind of 3.6 with its fixed order and its failure rule; every relocation goes
  through it (S22); provisional lineages and lineage counts settled in the serial phase (S24); dispersing litters as
  in S25. **Tests:** one makes each outbox kind fail once (a relocation of a body that died, arriving members that
  are gone, a litter with no kin back to a living parent and to the ground of a dead one's cell, a watch of a body
  that died) and checks the books; one checks that the remap gives the same lineages with and without threads. Level 3
  (a fractional litter that disperses is now made; moves land at the batch's end). `docs/laws.md` changes.
- **D2.5 Threads.** With the feature `threads`, the domains of a batch run on rayon's pool (`par_iter_mut`); without
  it, in order. The golden test gives the same fingerprints both ways. If the benches show the gain, the branch merges
  into `main`.
- **D2.6 The survey on threads (on `main`).** Wear, the monthly crowd, shade and sums run per domain in parallel; the
  Draws of the air step and the marks of light are filled per domain. Level 1.
- **Why:** bodies are three quarters of a year on one core. The waves of blocks of the foundation failed (decision
  59) because a day held too little work and each block was copied onto a world of its thread's own; here a batch holds
  five days of work, and nothing is copied: each domain owns its bodies.
- **References:** the domain decomposition of agent models (FLAME GPU, Richmond et al. 2010; Repast HPC, Collier and
  North 2013); deterministic parallel order (Bocchino et al. 2009, "Parallel programming must be deterministic by
  default").
- **Challenges and answers:**
  - *A body reads other bodies:* only through faces (3.5); acts in its own cell are live; watching goes to the outbox.
  - *Load balance:* sea domains hold few bodies, a refined region's one domain holds many. Rayon's work stealing takes
    domains one by one; the bench prints the busiest domain's share; `region24` reports it in phase Z.
  - *The store view must not overdraw:* 3.4.
  - *The tape and the viewer read bodies:* `Life::alive()` and the per-cell lists keep their meaning through the
    domains.
  - *wasm:* without `threads`, the domains run in order, the same history.
- **Done when:** the golden test is the same with and without threads; on the benches the bodies' stage is at least
  2.5 times faster on 4 cores than D2.4 on one; the key numbers hold.
- **Kill line:** if the bodies' stage gains under 1.8 times on 4 cores, stop and report: the branch is not merged and
  the ceiling stays 9,000.
- **Code:** about +540 in all (2.4). **Agent:** Opus for D2.1, D2.3, D2.4, D2.5; Sonnet for D2.2 and D2.6. Sequential
  (they touch the same files).

#### D1. Work off the main thread

- **What:** (a) the tape's file writing (snapshot compression, series rows, viewer frames) runs on a thread of the
  `cli` while the world runs on; (b) measure the cores' idle time in a year after D2 and D3; if more than 25% of a year
  leaves three cores idle, build the physics of the next step to run beside the bodies through the bodies' node (S9).
- **Why:** the tape is 11 to 15 ms a year: (a) gains about 1 to 2% of a year today, more of a faster one.
- **Check level:** 1 (the world is the same; only the shell's timing moves). **Code:** no core line for (a).
  **Agent:** Sonnet. **Parallel with:** D2 (it touches `cli/` and `tape/` only; on `main`).

### Phase E: a cheaper update, gated

#### E1. Bodies as columns (gated)

- **Gate:** a callgrind run with `--cache-sim=yes` on both benches after D2. E1 is built only if the body update
  loses at least 10% of its estimated cycles to cache misses (callgrind's estimate: instructions plus the misses
  weighted by their cost). Else it is logged as not built (S31).
- **What:** as in 3.11. A domain stores its bodies as columns; the code reads a body through `BodyRef` and `BodyMut`;
  field writes become methods where the columns need them.
- **Why:** each body is a struct with its own heap lists; columns keep a domain's bodies together in memory.
- **Check level:** 1 (only the layout changes). **Done when:** the golden test is unchanged; the bodies' stage is
  faster by at least half the share the gate measured. **Kill line:** under 3% faster, it is taken out again.
- **Code:** no extra allowance: under 9,700, it simplifies as much as it adds. Snapshot version bumped.
  **Agent:** Opus.

#### E3. A stage path for bodies without a brain (gated)

- **Gate:** a profile after D2 (and E1 if built) shows a stage of the updates of bodies without a brain repeating work
  worth at least 10% of the bodies' time (the thermal state, sums over stores). Else it is logged as not built.
- **What:** that stage runs as a loop over a domain's due bodies without a brain: what it needs is computed once per
  body, and the loop is short. Only auto-vectorisation: no `std::simd` (nightly) and no new crates. Deaths and
  children inside a stage are kept as marks and settled after it, in the bodies' order. Bodies with a brain keep the
  per-body path (rule 3): the code with a brain (`choose::act`, 19% of the globe) is not touched by E3.
- **Check level:** 3 (S15). **Gain:** unknown until the gate's profile. **Done when:** the bodies' stage is faster by at
  least 5% of the bodies' time; the key numbers hold.
- **Kill line:** a stage that gains under 5% of the bodies' time keeps the per-body path.
- **Code:** no extra allowance (under 9,700). **Agent:** Opus. **Depends on:** D2 (and E1 if built).

### Phase Z: the final check

- **Z1:** both benches and their profiles; the refined-region bench `region24` (1.2), with the busiest domain's
  share; the cost model of a cell and of a body again; the largest planet that fits 15 minutes.
- **Z2:** the slow suite (key numbers, equivalence tests, the nodes check), the golden test without threads.
- **Z3:** 1,000 years of the island 22 × 22, the globe 36 × 18, the refined region (B2's done-when) and the largest
  planet that fits; books, bodies and time.
- **Z4:** the watch report `docs/reports/speed.md` with pictures (S2), saying plainly where the speed stands against
  the aim of 10 times and what group F would add; the handover (sections 5.4, 7, 12.6), `docs/laws.md`,
  `docs/numbers.md`, `docs/tape.md` and `CLAUDE.md` (its table "Where things are") describe the present.
- **Agent:** Sonnet for the runs and the report draft, Opus for the final review.

---

## 5. Order and parallel lanes

```
A1 (done) ──► A2 ──► E2 hot spots (1 ── 2 ── 3 ── 4 ── 5) ──► B1 ──► B3 ──► B2 ──► (phase B review) ──► C2 ──► C1 ──┐
               │                                                                                                    │
               ├──► A3 (any time)                                                                                   │
               └──► D3 (flow/, solve/, change/) ── a parallel lane beside E2, B and C ──────────────────────────────┤
                                                                                                                    ▼
   branch speed/d2:  D2.1 ──► D2.2 (C3) ──► D2.3 ──► (review) ──► D2.4 ──► D2.5 ──► (review; merge if it pays)
   main:                                                                                       └──► D2.6 ──► (review)
   main, beside D2:  D1 (a); D1 (b) after D2.6 and D3
                                                                       E1 (gated) ──► E3 (gated) ──► (review) ──► Z
```

- **A1 is done:** every other step reads its baseline.
- **A2 next:** it changes every profile; the hot spots are found on its build.
- **E2 before B:** the hot spots are cheap and measured; B and C change the same body code afterwards.
- **D3** touches only `flow/`, `solve/` and `change/`: it runs from A2 on, beside E2, B and C, in its own worktree.
- **B1 before B3 and B2:** both read the node's parts that B1 makes. B3 and B2 run one after the other: both re-pin
  the golden test (2.3).
- **C after B:** C2 approximates the stages that B1 puts in one body; C1 after C2 (both touch the timing of nodes).
- **D2 after C, on a branch:** D2.3 touches every file of the update; it waits until B and C are merged.
- **E1 and E3 after D2:** the columns live inside the domains; the gates read profiles after D2. If D2 is killed, the
  gates read profiles of `main` and E1 is built on today's layout.
- **Reviews:** after phase A and E2 (light), after phase B, after C, after D2.3, after D2.5, after D2.6, after E3, and
  the final review.

---

## 6. Risks and what to do

| Risk | What to do |
|---|---|
| Ten times is not reached | Expected (1.3, S20): `measures.md` says where it stands at every step; the report says it plainly, with what group F would add |
| The refactor of D2.3 never ends | Sub-steps that each run the island and pass the checks; D2.3 changes the reads, D2.4 the writes; all on a branch that `main` never waits for |
| D2 does not pay | Its kill line: the branch is not merged, the ceiling stays 9,000, E's gates read profiles of `main` |
| A speed tool changes the world (B2, C1, D2.2, `node_days`) | Its equivalence test and the key-number test are part of its "done when"; a change past noise stops the step |
| B1 removes fewer bodies than hoped | Its target comes from its own measure; its kill line stops it |
| The milder merge of B2 does not bound the bodies | B2 reports with numbers; a stronger rule needs a new decision |
| The nodes check (F7) breaks more | Run at the end of phase B and of phase D; a new failure is a bug to find |
| Threads give a different history | The golden test with and without threads at every D step; no step closes until it holds |
| The physics' threads change sums | D3 is level 1: the fingerprints must not move (a reactor change is level 2, its own commit) |
| The code ceiling | 2.4: before D2.1 B and C balance, other steps pay; D2's lines open only on its branch; E simplifies as much as it adds |
| A long run finds a books error late | The benches print the drift every year; phase Z runs 1,000 years on four worlds |

---

## 7. Words used in this plan

| Word | Meaning |
|---|---|
| bench | a settled world kept as a snapshot, run 5 years to measure speed |
| hot spot | a function the profile shows holding a large share of the instructions, made cheaper in its own commit (E2) |
| domain | the cells under one air node, with the bodies and parts in them, run as one unit |
| batch | the bodies due in `batch_days` days, run together at the batch's first day |
| parallel phase | the part of a batch where each domain runs its due bodies, on threads |
| serial phase | the part of a batch after it, where the moves and the outboxes are booked in a fixed order |
| relocation | a body, or members of a node, going to another cell; it lands at the batch's end |
| outbox | what a domain's bodies did that lands at the batch's end, applied in a fixed order |
| store view | a domain's store nodes as the moves of its batch leave them |
| face | what others see of a body (id, cell, members, lineage, visible traits, signal, well-being, room), kept from the batch's start |
| provisional id | the id a lineage made in the parallel phase holds in its domain until the serial phase gives it a global id |
| part (of a node) | the adult or the young members of a node without a brain after B1; they share one store |
| column | one array holding one field of all bodies of a domain |
| kill line | the gain under which a step is taken out again |

# Speed after the foundation: the plan

**Status:** the owner asked for this work on 24 October 2026: build ideas A to E of the speed list (group F waits).
The owner asked the architect to take every decision and log it: they are in `decisions.md` next to this file (S1,
S2, ...). What was built and measured goes in `docs/history.md` (section 5) and in `measures.md` next to this file.
Where this file and `docs/handover.md` differ, the handover wins until it is updated. Phase A is done.

This file is for the agents who do the work and for the owner. It is written in simple English (handover 0).

---

## 0. In short

- **The aim:** a settled world runs **10 times faster** than the baseline on the owner's 4-core machine, with the same
  laws and the same honest books. That is the owner's aim, and the targets of 1.1 are set by it.
- **What we expect, honestly:** with groups A to E, **about 3 to 5 times** overall; about 2 times if D2 is killed and
  C2 fails (S20). After phase A, bodies can become about 6 to 12 times cheaper, the rest of a year (Flow, Change,
  parts, tape) about 1.4 to 1.6 times. The rest alone is more than the whole budget of 10 times (section 1.3). Reaching
  10 times needs group F (S1) or new physics algorithms; 1.3 says what each would add.
- **Where we stand** (`measures.md`): phase A is done. One codegen unit (A2, S35) took a year from 0.857 to 0.702 s on
  the island and from 1.347 to 1.220 s on the globe. Three worlds side by side run 2.9 times faster than one after
  another (A3). Bodies are three quarters of a year (525 of 702 ms on the island, 905 of 1,220 ms on the globe) and run
  on one core; one body update costs about 19 µs on the island and 35 µs on the globe. The profile is flat.
- **What we build next**, in the order of their measured share (S32):
  - **E2.1 and E2.2, two hot spots:** `spread::part_acts` cloning and pooling a body for each option part, and `pow`.
  - **D3, the physics and the survey on threads:** a parallel lane beside B and C.
  - **B, fewer bodies:** one node per kind per cell for every population node, its young inside it; light that reads
    the sizes inside a node; cells that cannot fill without end, by a milder merge.
  - **C, fewer updates:** a closer step over a node's interval, so nodes update half as often; dormant bodies rest.
  - **D2, all four cores for the bodies:** the world is cut into **domains** (the cells under one air node); each
    domain runs its bodies on its own thread in **batches** of days; every move to another cell and every effect
    outside the domain waits in an **outbox** booked in a fixed order; bodies near parts run in the serial phase. If
    it does not pay, its commits are reverted.
  - **D1:** the tape's writing off the main thread (about 1 to 2% of a year).
  - **E2.3, hot spots after D2; E1 and E3, gated:** a new profile picks again; columns and a stage path for bodies
    without a brain only if a measure shows they can pay.
- **Each step measures its gain.** A step that does not pay is taken out again and logged.

---

## 1. The goal and how we measure it

### 1.1 Where we stand and the targets

From `measures.md`, ms a simulated year (the mean of 5 years):

| Bench | Row | s a year | Flow | Change | bodies | parts | tape | bodies alive | updates a year | µs an update |
|---|---|---|---|---|---|---|---|---|---|---|
| `island22` (22 × 22, year 300) | baseline | 0.857 | 159 | 49 | 638 | 0 | 11 | 2,744 | 27,762 | about 23 |
| | A2 (now) | 0.702 | 128 | 40 | 525 | 0 | 10 | 2,744 | 27,762 | about 19 |
| `globe36` (36 × 18, year 200) | baseline | 1.347 | 229 | 51 | 1,010 | 43 | 15 | 1,955 | 25,626 | about 39 |
| | A2 (now) | 1.220 | 210 | 49 | 905 | 43 | 12 | 1,955 | 25,626 | about 35 |

The machine drifts by about 7% between hours (`measures.md`, A2); a gain is read against a measure of the same hour.

| Target | Baseline | The aim (10 times) | What A to E likely give (about 3 to 5 times) |
|---|---|---|---|
| `island22`, s a year | 0.857 | **0.086** | 0.16 to 0.20 |
| `globe36`, s a year | 1.347 | **0.135** | 0.29 to 0.37 |
| 1,000 years of the 36 × 18 globe (bodies grow over centuries) | 36.6 min (`docs/reports/foundation-l.md`) | **3.7 min**; under 15 min is a waypoint | about 7 to 10 min |
| Largest planet in 15 minutes (the cost model of the foundation, plan 5.7) | about 290 cells | about 2,900 cells | about 1,000 to 1,600 cells |

A settled world is the world after its life has settled: its bodies per cell are what a long run has, not what year 10
has. "Faster" counts every gain: fewer bodies (B), fewer updates (C), more cores (D), cheaper updates (A, E).

### 1.2 How we measure (A1 built this)

- **Bench worlds.** `scripts/bench.sh` makes `island22` (`ew run --size 22 --seed 1`, year 300) and `globe36` (`ew run
  --globe --size 18 --seed 1`, year 200) and keeps them outside the repository (`$BENCH_DIR`, default
  `~/.cache/ew-bench`). It runs `ew resume` for 5 years from each, twice, keeps the lower, and prints per simulated
  year: Flow, Change, bodies, parts, tape (the `ms_*` columns), the bodies alive, the body updates and the seconds a
  year. Nothing else runs on the machine.
- **Remaking the benches.** A step that changes the snapshot version or the world remakes them with `--remake`: about 8
  minutes today (the years to settle them), less as the work speeds up. The script never remakes silently.
- **A third bench from D2.3 on:** `region24`, a 24 × 24 island with an 8 × 8 refined region (`ew run --size 24 --seed 1
  --region 8,8,8,8`, year 200), added to `scripts/bench.sh` in D2.3 and measured in every row from then on (S41). One
  air node holds 64 cells there and the densest life, so one domain holds much of the work: D2 gains least there.
- **From D2.3 on**, the bench also prints the **busiest domain's share** of a batch's work (its updates over all
  updates of the batch, the year's mean; a counter of the tape, `docs/tape.md`), so an imbalance shows.
- **Profile.** `scripts/bench.sh profile <bench>` runs one year under `valgrind --tool=callgrind` and prints the top
  functions by inclusive and exclusive instructions. Instruction counts do not depend on what else runs, so they
  compare two builds exactly. E1's gate adds `--cache-sim=yes`.
- **The record.** Each step that touches speed adds one row to `measures.md`: the commit, the benches (s a year and
  each stage), bodies, updates, core code lines.
- **Few long runs.** The benches take seconds. Long runs (1,000 years) wait for phase Z.

### 1.3 The arithmetic of 10 times

The budget of 10 times is 86 ms a year on the island and 135 ms on the globe. After A2, the rest of a year (Flow,
Change, parts, tape) is **178 ms** on the island and **314 ms** on the globe: alone it is about twice the whole budget.
For 10 times with bodies 12 times cheaper (44 and 75 ms), the rest would have to fall to about 40 and 60 ms: 4 to 5
times cheaper.

What each group can give from here (the plan's guesses; `measures.md` holds the real numbers):

| Group | On the bodies | On the rest |
|---|---|---|
| A (done) | 1.2 and 1.1 (A2) | 1.2 and 1.06 (A2) |
| E2.1, E2.2, hot spots | 1.05 to 1.15 (`nodes::add` is 8% inclusive on the globe; `pow` 2.4% on the island) | — |
| B, fewer bodies | depends on B1's measure (section 4, B1); B2 matters on long runs | — |
| C, fewer updates | 1.3 to 2 (C2 halves the updates of population nodes without a brain if it passes) | — |
| D2, four cores | 2.2 to 3 (against `main` before D2.1) | — |
| D3, the physics on threads | — | Flow about 1.5; Change 1.2 to 1.5 (its reactor already runs on threads) |
| D1, the tape off the main thread | — | about 1 to 2% of a year |
| E2.3, E1, E3 | unknown until profiled after D2 | — |

So bodies become **about 6 to 12 times cheaper** than now (E2 1.05 to 1.15, B and C together 2.5 to 3.5, D2 2.2 to
3), and the rest **about 1.4 to 1.6 times** (D3, D1; the parts' monthly step and the serial parts of Flow stay): about
115 ms on the island and 220 ms on the globe. The year, against the baseline:

| Case | island | globe |
|---|---|---|
| bodies 6 times cheaper | 88 + 115 = 203 ms: 4.2 times | 151 + 220 = 371 ms: 3.6 times |
| bodies 12 times cheaper | 44 + 115 = 159 ms: 5.4 times | 75 + 220 = 295 ms: 4.6 times |
| **floor:** D2 killed and C2 failed (bodies about 1.7 times cheaper: E2, B, C1) | 309 + 115 = 424 ms: 2.0 times | 532 + 220 = 752 ms: 1.8 times |
| bodies free (the ceiling of A to E) | 115 ms: 7.5 times | 220 ms: 6.1 times |

Hence "about 3 to 5 times", and about 2 at the floor (S20). What would add more, beyond this plan:

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

- `docs/laws.md` changes in the same commit as each law change (E2.1, B1, B3, B2, C2, C1, D2.2, D2.3, D2.4, E3).
- `docs/numbers.md` gets a row for each new setting in the step that adds it (`merge_cap` in B2, `rest_activity` in
  C1, `batch_days` in D2.2), and the rows of `crowd` and `node_days` change where B2, C2 and D2.2 change them.
- `SNAPSHOT_VERSION` (in `tape`) is bumped at every step that changes the saved world: B1, B2 (`Config::merge_cap`),
  C1 (`Body::due`, `Body::gap`), D2.1, D2.2, E1, and any step that adds a saved field.
- `docs/tape.md` changes when events or counters carry new ids or columns (D2.1's body ids, D2.3's busiest-domain
  share).
- `CLAUDE.md`'s table "Where things are" lists this folder; Z4 checks it points to the final state.

### 2.2 The three rules for future smart bodies (the owner's wish)

The work must not close the door to smart, people-like bodies later (handover 13.2, "No people-like kind"):

1. **One place decides who may join whom.** Every merge path (`nodes::join_alike`, `nodes::merge_closest`, the kin
   joins of `copy`, `spread::part_acts`) asks one predicate on data, `nodes::may_join` (lineage, genes, spread, and
   a hook for a mind that stays individual), built in B2 with no change of behaviour beyond B2's rule (S7). Today's
   animal nodes are population nodes by handover 7.1 and keep merging as they do. When people-like minds come, "a
   mind that stays individual" is one test in that predicate (S30). A node can always be split back into individuals
   (`nodes::open` stays and is tested).
2. **The step length can later be chosen per place.** A domain owns its bodies and its ring of updates, so a zone
   domain (a focus zone, handover 7.3) can later run several sub-batches inside one global batch, its edge effects
   still landing at the global batch's end. Nothing in D2 blocks that; today one batch length holds for all domains.
3. **Two kinds of body code.** E3, if built, is for bodies without a brain. Bodies with a brain keep a per-body path,
   and nothing in E3 may assume that every body takes the same branches. (B1 is one mechanism for every node; young
   animals act through their node's spread of actions, S36.)

### 2.3 Checks for each step

| Check | When |
|---|---|
| `cargo build --release --workspace` | every step |
| `cargo test --release --workspace` (the normal suite, under about 30 s, with the golden test) | every step |
| `cargo clippy --release --workspace --all-targets` with no warning | every step |
| `cargo build -p pcore --target wasm32-unknown-unknown` | every step |
| `cargo test --release -p tape --test golden` (the golden test without threads) | every step that touches threads or order |
| `scripts/bench.sh` (the benches, 5 years each) | every step that touches speed |
| The key-number test (`tape/tests/ranges.rs`, about 10 minutes today, faster later) | once in each level-3 step that changes a law or a speed setting: E2.1; B1 and B3 together (one run after B3); B2, C2, C1, D2.2 (its run also checks D2.1's new rng keys), D2.3, D2.4, E3 (S33) |
| The nodes check (F7, `tape/tests/nodes.rs`, about 5 minutes) | before B1 merges, against today's result (S36); at the end of phases B and D |
| The equivalence test of a new speed setting (× 2 and ÷ 2) | in the step that adds or changes it |
| The slow suite (`cargo test --release --workspace -- --ignored --no-fail-fast`) | at the end of each phase |

**Tests are few and fast** (`CLAUDE.md` 5). A new test states a law or an invariant. A step that deletes a mechanism
deletes its tests.

**Check levels** are those of `docs/foundation/plan.md` 6.1: level 1 (bit for bit), level 2 (the same to rounding,
for speed work that changes the order of sums), level 3 (the same world: key numbers in their ranges). Each step names
its level. A level-2 or level-3 step re-pins the golden test in the same commit and says why.

**Merging and the golden test.** Steps merge one at a time. A step that re-pins the golden test rebases on the `main`
it merges into, runs again and re-pins there, never on a stale base. B1, B3 and B2 run one after the other.

### 2.4 The code ceiling

The 700 lines of decision 42 of the foundation serve the whole speed work (B, C, D, E), not only threads (S3): the core
ceiling is **9,700** code lines on `main` from the commit of this plan (`cli/tests/core_size.rs`, `CLAUDE.md` 2.11).
`core` held 8,998 lines after phase A. Never pack lines to save count; comments are not counted (`CLAUDE.md` 2.11).

- **Each step names its deletions and reports its lines** in its commit message. The work aims to end at or below
  **9,500**.
- **D2 goes to `main`** sub-step by sub-step. If D2 is killed, its commits are reverted.
- **The estimates:**

| Step | Adds | Deletes | Net |
|---|---|---|---|
| E2.1, E2.2 | about 15 | about 15 | about 0 |
| D3 | about 40 | about 10 | about +30 |
| B1 | about 160 (the parts' state as shares, the parts in `Lot` and in `add`/`rejoin`, the growth split, maturing inside a node, `open`'s draw of a part, the per-part helper and its about 15 uses, the two starvations, a gain per part, one think per part for nodes with a brain) | about 70 (`copy/young.rs`, 50 code lines; `Body::link` and its uses; `Grown::Joins` and the merge after maturing in `survey::update`; the stage in `nodes::Kin` and in `merge_closest`'s key; `copy::emit`'s young-node branches) | about +90 |
| B3 | about 20 | — | about +20 |
| B2 | about 25 | about 35 (one predicate replaces the tests of `kin_of`, `merge_closest`'s distance and `copy::kin`) | about -10 |
| C2 | about 30 to 50 (by the method chosen) | — | about +40 |
| C1 | about 35 (`Body::due`, `Body::gap`, the wake check) | about 10 (the reads of `interval` it replaces) | about +25 |
| D2.1 to D2.6 | about 560 | about 40 | about +520 |
| D1 (b), if built | about 80 | — | about +80 |
| E2.3, E1, E3 | each simplifies as much as it adds | | about 0 |

That is about 9,790 with everything built, over the ceiling; about 9,710 without D1 (b). Both are above the aim of
9,500. So each step looks for what it can simplify in the files it touches, and the reviews ask for it; D1 (b) is built
only if it pays and the lines are there. **A step that would cross 9,700** simplifies first; if it cannot, it stops,
and the architect decides (logged in `decisions.md`).

### 2.5 How a step is worked and closed (the agents)

The architect plans, splits the work, reviews and merges. For each step:

1. **Context:** a Haiku agent gathers facts (call sites, numbers, profiles) when the step needs a wide search or a
   measure before it is built (D3, B1, C1, C2, the serial share of D2).
2. **Build:** an Opus agent for complex code (E2.1, B1, B3, C2, D2.1, D2.3 to D2.5, E1, E3), a Sonnet agent for normal
   code (E2.2, E2.3, B2, C1, D1, D2.2, D2.6, D3, docs) (S19). Steps on different files run in parallel, each in its
   own git worktree.
3. **Check:** the builder runs the checks of 2.3 and adds the bench row.
4. **Review:** at the end of each phase (and of D2's big sub-steps), an adversarial Opus agent reviews the diff against
   this plan, `CLAUDE.md` and the handover. A fixing Opus agent answers every finding (fix it, or say why not). They
   loop until the reviewer approves.
5. **Close:** the architect re-reads the diff, then commits to `main` and pushes (S16). One line per change goes in
   `docs/history.md`; a decision taken goes in `decisions.md`.

### 2.6 When a step fails its goal

A step that does not reach its "done when" stops. If it does not pay (its gain is under its kill line), it is taken
out again and gets one line in `docs/history.md` under "tried and not kept". If it pays but breaks a check, the cause is
found and fixed; a range or a tolerance is never loosened to pass (`CLAUDE.md` 5.5).

**How a gain is judged against its kill line** (S45). The machine drifts by about 7% between hours, so:

- a step on one thread (E2, B, C, E3, a serial loop of D3) is judged on callgrind instructions,
  which are exact;
- a gain from threads (D2, D3's threads, D1) and a gain from memory layout (E1) is judged on wall time, 5 or more runs
  of each build interleaved in the same hour, compared by their medians;
- "a gain above the noise" means above the spread of those runs (or any gain in instructions, which have none).

---

## 3. The architecture after this work

### 3.1 The picture

```
  world step (one month)
  ├─ survey            every phys step: per domain on threads (wear, crowd, shade, sums)        [D2.6]
  ├─ Flow, Change      per-cell work on threads, sums in a fixed order                           [D3]
  ├─ Parts (Bond)      once a month, serial, as now                                              [S23]
  └─ bodies            every batch of `batch_days` days                                          [D2.2]
        ├─ batch start: a face of every body, the ids in every cell                              [D2.3]
        ├─ parallel:    each domain, taken out of `World`, runs its due bodies that touch no     [D2.5]
        │               part, in its fixed order, with its own store view and its own outbox
        │                  └─ a body without a brain: the stage path, if E3 is built             [E3]
        │                  └─ a body with a brain: the per-body path
        └─ serial:      the domains' moves booked, the outboxes applied, then the bodies that    [D2.4]
                        touch parts, all in a fixed order (3.6)
```

### 3.2 Domains: what each one owns (D2)

- A **domain** is one air node and the ground cells under it (4 × 4 cells today; 64 in a refined region) (S17).
  Domains are fixed when the world is made. Their order is the order of the air nodes.
- **For the parallel phase the domains are taken out of `World`** (`std::mem::take`), so the compiler refuses a live
  read of another domain; the world left behind is read-only (S42).
- **What a domain owns and writes in the parallel phase** (its state; nothing else is written there):
  - its **bodies** (slots in the domain) and its cells' lists of body ids;
  - its **ring of updates due**; a body that moves to another domain is taken out of its ring and filed in the new
    domain's ring, at the same due batch, in the serial phase;
  - its **store view** and its moves, in place of `World::moves` (3.4);
  - its scratch: a `Reactor`, a `LiveTmp`, an `ActTmp`;
  - its **events** and its **counters** (f64 sums, merged into `World::counts` in domain order);
  - `Life::shade` (per body) and `Life::stale` (per cell) for its own bodies and cells;
  - its counter of new body ids, its **provisional lineages** and **provisional born rows**, and its integer deltas of
    lineage member counts and of born-row holders (3.3);
  - its **outbox** (3.6).
- **Parts stay one global registry, touched only in the serial phase** (S23, S52). A due body that holds a part or is
  inside one (`hold` or `inside` set) is a **serial body**: it runs its whole update in the serial phase, so its
  exchanges with the part it holds or is in stay immediate and no part id changes. Every other body runs in the
  parallel phase and reads parts only (perception, `bond::possible`, depth and walls, which do not change there); an
  action it takes on a part (any of Bond's verbs, FORM among them) is a **part act** of the outbox, run whole in the
  serial phase on the part and the body as they are then. `bond::step` stays serial. Measured on `globe36` (S52):
  bodies that hold or are inside a part take 8.5% of the updates; bodies in a cell with a part, 37%.
- **What is written only in the serial phase:** parts, the id table (id → domain and slot), `Life::brains`,
  `Life::members`, `Life::genomes` and their counts, `Life::pushed`, `Links::cleared`, `World::counts.crossings`, the
  bodies that change domain, and the world's event list.
- **What the parallel phase reads, unchanged during the batch:** the faces and the cells' id lists (3.5), the store as
  it was at the batch's start for cells of other domains (3.4), the survey's caches (`Life::sum`, `env`, `seen`,
  `cellf`, `absorb`, `keep`, `World::canopy`), `World::light` and `rained`, `Links::cleared`, Flow's last state
  (`scratch`), the parts, the global lineages and born rows, the chemistry and the config.
- **Flow writes bodies** (`mark`, `draw.owed` through `survey::Draws::owe`, the bodies' node's heat): those loops go
  over the domains, outside the batches, and `Draws::owe` writes each domain's bodies.
- **Same history with or without threads.** The parallel phase reads a world that does not change, each domain is
  sequential inside, and the serial phase has one fixed order, so the result does not depend on which thread ran a
  domain or when. The golden test checks it with and without threads, and D2.5's order test with pools of 1 and 4
  threads and the domains in reversed order (S42).

### 3.3 Ids: bodies, lineages, born rows

- **Bodies.** A body gets an id at birth that never changes: its domain of birth and that domain's counter (S10). A
  body that moves keeps its id. The id table finds where it lives; it is written only in the serial phase, so a body
  born in the parallel phase is found through its own domain until then. A body placed in the serial phase (a litter,
  the members `nodes::open` draws in `place_child`) takes its id from the target domain's counter in serial order, and
  its random draws are keyed by its parent's id and the purpose (S42). Memory of individuals (`Body::known`) and the
  rng keys read the id, so a move between domains changes nothing for them.
- **Parts** keep today's slots and ids: they are only touched in the serial phase. `Part::held` keeps the holder's
  body id.
- **Provisional lineages.** A lineage made in the parallel phase (`lineage::resize`, `copy::child_lineage`) gets a
  provisional id of its domain; the domain keeps its brain, and every read of a lineage's brain looks there first. The
  serial phase maps provisional ids to global ids, in domain order and then in the order made, and rewrites the ids
  its bodies and its outbox hold. Lineage member counts and a lineage dying out are settled only in the serial phase,
  as integer deltas merged in domain order (also after the deaths of D2.6's parallel `wear`), so a brain stays
  readable for the whole batch (S24).
- **Born rows stay shared** (`Life::genomes`, S11). Their holders' counts change by per-domain integer deltas, merged in
  the serial phase; born rows made in the parallel phase get provisional ids, remapped like lineages.

### 3.4 The store view (D2.3)

- **What it holds** (S26): the Ground, Bodies and Bed nodes of every cell of the domain and the domain's air node, as
  the moves of this batch leave them (as `store::Moves` keeps one cell's nodes for one update today), and a per-domain
  delta of the Potential node (the lift paid when gas comes down from the air, `Moves::gas`).
- **Every direct read of the store in body code goes through the view.** Today these are:
  - `World::g`, `World::a`, `World::a_seen` and `World::seen_now`, `World::heat_now`, `World::take_from_ground`
    (through `store::Moves` today);
  - `World::b` and the skin in `World::heat_cap_ground` (the Bed: the body's physics and `feed::take_node` read it),
    and `World::thaw` (the ground's heat, read straight from `w.store`, where `World::t_ground_last` falls back to it
    before Flow has run);
  - `bond::dig` (the Bed's amounts, `bond/verbs.rs`): DIG is open to every body (`choose::options` masks only ANCHOR
    and RELEASE), so it reads the Bed through the view; `bond::anchor` reads the Bed too, but only serial bodies hold
    a part to anchor.
- **Reads of another domain's cells read the store as it was at the batch's start** (S42): `bond::afloat` reads the
  liquid of the target cell through `w.g` (`bond/sense.rs`, called by `choose::go`), and `choose::go` reads the target
  cell's survey sums. The builder lists every such read and every read of the first list again with
  `grep -n "w\.store\|self\.store\|\.store\.amounts\|\.store\.heat\|w\.g(\|w\.b(\|w\.a("` over `body/`, `choose/`,
  `copy/`, `bond/`, `nodes.rs`, `lineage.rs`, `ground.rs` and `world.rs`; the list goes in the commit message. Reads
  outside the batches (`survey::Draws::settle`, `bond::month`) stay as they are.
- **No overdraw.** A domain's bodies take from its nodes in order, each seeing what the bodies before it left. A debug
  check holds that a take from the view never passes what the view holds.
- **Checkpoint and roll back** (S38). The view can be checkpointed (the length of its move list, the values of its kept
  nodes, its events and its counters) and rolled back to it. `copy::sample` runs inside a checkpoint and rolls back, as
  today a sample member's update drops its moves. A test holds that a sampled update leaves the view as an unsampled
  one would.
- **Booking.** The batch's moves of a domain are booked by one `apply` per domain in the serial phase (level 3: one
  `apply` for a batch changes the order of sums and what rounding sets to zero). This replaces today's `apply` per
  update (`World::book_moves`); `store::apply` is 6 to 8% of the instructions, counting every `apply`, Flow's and
  Change's too.

### 3.5 Faces (D2.3)

- **Each batch starts with a face of every body** (S14): id, cell, members, lineage, visible traits (`Body::vis`),
  signal, well-being, and the root and closedness of the room it is in (`bond::room`, read from the parts, which do not
  change in the parallel phase); about 64 to 80 bytes a body. With them, for every cell, the list of the ids in it at
  the batch's start. Faces are made serially or per domain; either way before any update of the batch.
- **Perception reads only faces and those lists** (`choose::perceive`, `inputs`' signals heard, `options`' features
  of a body seen, `Body::known` by id), whether the other body is in the same domain or not. `Choice::seen` holds ids,
  not slots. In its own cell, perception skips the ids its domain has marked leaving (3.6).
- **Acts on another body.** Taking and giving (`take_body`, `feed::give`) are in the same cell, so the same domain:
  they resolve the id in the domain and act on the live body; if it is gone or leaving, the act fails as today. The
  partner of a mating (`copy::mix::partner`) and the others of a cell in `body::exchange` are in the same cell too and
  are read live.
- **Watching** (`learn::watch`) reads the other's whole brain rows, so it always goes to the outbox and is applied in
  the serial phase on the live target. No rows are copied into faces.

### 3.6 The outbox and the serial phase (D2.4)

**Every relocation of a body or of members to another cell goes through the outbox and lands at the batch's end,
inside the domain as well as across its edge** (S22): one path, so a cell at a domain's edge behaves like any other.
`choose::go` pays its cost and returns that it queued a move.

**A body with a queued relocation is leaving** (S39). It stays in its old cell until the move lands, but:

- it does not split (`nodes::split`) in this update; the rest of its update (selection, `lineage::resize`, maturing
  inside the node, `World::see`) runs in its old cell, in its own domain;
- `copy::kin`, `copy::mix::partner`, `nodes::merge`, the targets of `take_body` and `feed::give`, and perception's
  own-cell lists skip it;
- `spread::part_acts` and the whole-node branch of `spread::spread` both return "left" (the loop breaks), so no
  second relocation is queued.

**A node's part with a queued part act** (S44, S52) stays a body of its own until the serial phase, as a moving part does
(`part_acts` treats it as leaving the node). After its part act is applied, the part body rejoins its node
(`nodes::rejoin`) if it holds nothing and both are in one cell; else it stays a body of its own.

The kinds, in the order the serial phase applies them:

| Kind | Made by | Carries | Applied in the serial phase | If its target is gone |
|---|---|---|---|---|
| Relocation | `choose::go` (a body, or a part placed by `part_acts`) | body id, from cell, to cell, the energy spent pushing, the lane area to clear | the body leaves the old cell's list for the new one's (id table, rings); `cell`, `mark`, `rained` of the new cell, `inside` none; the parts there are pushed (`bond::moved`); `counts.crossings[j] += n`; `Life::pushed[j] +=` the energy; `lanes::clear` on `Links::cleared` | the body died later in the batch: the relocation is dropped whole; its cost was paid and spent |
| Arriving members | `spread::part_acts` (members that move) | a relocation, marked "look for kin" | after the relocation lands: the kin search in the new cell (`copy::kin`, through `nodes::may_join`) and the merge (`nodes::merge`); no kin: it stays a body of its own, as today | the part is gone: nothing lands, nothing joins |
| Litter | `copy::emit`, children who start in another cell | the cell the parent was in, the target cell, members, stores and heat (taken from the parent by `give`), genes, spread, size, rows, lineage, parent id, whether it may be a node, whether it is mixed | the kin search in the target cell; it joins the young part of a node there (`nodes::add`) or is placed (`place_child`, ids as in 3.3); the `Born` event is made here (S25) | no kin and less than one member: back into the parent's brood if the parent lives (by id: stores, heat and brood back), else to the ground node of the cell the parent was in, as a move |
| Watch | `learn::watch` | watcher id, its member count and lineage at watch time, the share of its members who watched (1 for an individual, x / count for a node's part), target id | if both live, the watcher is still of that lineage and their brains have one shape: the watcher's rows move toward the target's by `blend` × alike × the share × (count then / count now) (alike from the target's `vis` and the watcher's `visible`); the `Watched` event. The blend is linear, so this is the mean by members that `nodes::rejoin` makes today (S40) | either is gone, or the watcher's lineage changed: nothing (the turn was spent) |
| Part act | any of Bond's verbs (FORM, take from or put into a part, grab, join, push...) chosen in the parallel phase, by a body or by a node's part placed by `part_acts` | body id, the verb, the part's id (or for FORM the shape drawn), and for a node's part the id of its node | run whole as today on the part and the body as they are then (`bond::act`, `bond::form`): the body pays, the part changes or is made; a node's part then rejoins its node if it holds nothing and both are in one cell | the body or the part is gone, the act is no longer possible (`bond::possible`), or the body can no longer pay: nothing (a node's part rejoins its node by the same rule) |
| New lineage, new born rows | `lineage::resize`, `copy::child_lineage`, `copy::place_child` | provisional id and its `Lineage` or rows | mapped to a global id; ids rewritten (3.3) | cannot fail |
| Counts | every body placed or removed | per lineage and per born rows, the change of their holders | added; a lineage at zero dies out (`Life::brains` none), born rows at zero are freed | cannot fail |
| Events and counters | all body code | the domain's list and sums | appended and added in domain order | cannot fail |

**The serial phase, in this fixed order:**

1. Each domain's moves are booked by one `apply`, in domain order.
2. Provisional lineages and born rows get global ids, in domain order and then in the order made; ids are rewritten.
3. The entries of the outboxes (relocations, arriving members, litters, watches, forms), domain by domain in domain
   order, each domain's in the order they were made.
4. The serial bodies run their whole update, in domain order and then in body order, each with its domain's store view
   opened fresh and booked when its update ends; their own outbox entries are applied right after their update, by the
   same code.
5. The moves the serial phase made otherwise (litters given to the ground, forms) are booked by one `apply`.
6. Counts are added in domain order; a lineage at zero dies out.
7. Events and counters are merged in domain order, then the serial phase's own events in the order made.
8. The id table and the rings are final; the next batch starts with new faces.

**What changes in the world (level 3):** bodies are seen as they were at the batch's start; every move to another
cell, every watch and every new part land at the batch's end; a leaving body does not split in that update.
This is the same kind of lag a body already has when it sees far cells "as the last survey saw them" (handover 5.6 of
the foundation). A fractional litter that disperses is now made and may come back (S25).

### 3.7 Batches (D2.2)

Bodies update in batches of `Config::batch_days` days. The full description is in D2.2 (section 4), the one place
for it (S8).

### 3.8 One node per kind per cell (B1, B3)

- Today a kind in a cell is an adult node and a young node joined by a link. After B1, **for every population node,
  with or without a brain** (S29), a kind in a cell is **one node** with two parts. **One store**: the parts share the
  node's stores, heat and brood; they differ only in members and sizes. `copy/young.rs`, `Body::link` and
  `Grown::Joins` go.
- **The state:** `count` (members, n), the adult members `n_a`, **the adults' share of the built matter** (not the
  adult member's body in kg), and the young part's log10 moments for its `n_y = n − n_a` members: mean, variance, and
  `size_lag` as today but defined against the young part's mean member. The adult member's body is the adults' share
  of the built matter over `n_a`; the young mean member's body is the rest over `n_y`.
- **The invariant** (the store's built matter = adults' matter + young matter; the brood apart, the adults' only)
  holds by construction: the parts are shares of one built matter, so it cannot drift. Every uniform change of the
  body matter needs no code: `brood_share` and its callers (`body/physics.rs`, `body/pay.rs`), `Body::lose`,
  `Body::digest`, transfers between bodies, burning and upkeep leave the shares as they are.
- **What changes the parts:**
  - growth is split by supply (members × mass^0.75 at each part's size, Kleiber): the shares move;
  - maturing moves members and their matter from the young part to the adult part by `math::Pass`, as
    `copy::young::mature` does today between two bodies; it runs only for `count` ≥ 2;
  - deaths that are not by size (`death::thin`, `live::lose_share`, `death::wound`) remove the same share of members
    from each part: the shares stay;
  - growth starvation (`live::starved`) is a death in the young part only, followed by a uniform rescale;
  - `live::starve` (members without their short stores) is a death of the same share of each part, and the matter it
    leaves behind is a uniform rescale;
  - children who stay join the young part (members and moments); their matter leaves the brood for the body inside the
    same store.
- **The parts rules** (S43):
  - a `nodes::Lot` carries the members of each part; any split (`Lot::split`, `nodes::part`, `nodes::split`,
    `nodes::open`) takes the same share of each part, and stores taken beyond that share (`fa − f`) are a uniform
    rescale of both parts' bodies;
  - `nodes::add` and `nodes::rejoin` pool adults with adults and young with young (each part's moments pooled as
    today's one set is);
  - an individual (`count` 1) is one part, chosen by its size: it moves to the adult part when it passes the copy mass;
  - `nodes::open` and its `draw` draw a part by a chance by members, and a size from that part;
  - every law that is not linear per member is evaluated per part and summed by members through **one helper**
    (for example `Body::per_part(|member_body, members| ...)`); linear laws run on the totals. The B1 audit lists the
    `\.n()` sites (about 50) and assigns each to "totals" or "per part": leaf area through `equiv`
    (`body/exchange.rs`), the skin area's member mass (`body/exchange.rs`, `skin_area` and its other use), the
    thinking cost (`body/pay.rs`, `upkeep`: not linear per member, so upkeep is not linear as a whole), `walk`
    (`choose/go.rs`, `bond/verbs.rs`), perception's `pm` and `vol`, `fuel / n` in well-being, and `take_body`'s
    `fb`, `fo` and `per`;
  - a part's gain for the selection sample is ln(that part's buildable matter after / before), the shared reserves
    counted to each part by its share of the built matter (S37).
- **Height and visible traits:** one composition (one store), so a part's height is that of a member of its size with
  the node's material shares. `Body::vis` keeps its meaning (the node's mean member) for perception, the survey's sums,
  faces and watching; the shade reads each part's height and capture area through the per-part helper (two layers,
  B3).
- **Why E11 cannot come back.** When sizes were pooled across the copy mass, variances ran to 1e16 and NaN
  (`docs/history.md`, E11). Here young and adults keep separate moments, never pooled across the copy mass: the adults
  sit at one size, children enter only the young moments, `add` pools each part with its own kind, and maturing moves
  members across by `math::Pass`.
- **Nodes with a brain** (S36): a node with both parts thinks once per part, with the inputs at that part's member
  size, and that part's members spread over its options (`choose::spread`). So young members act on their own state.
  Such a node thinks twice an update: its thinking cost is paid per part's members (it is per member), and the bench
  counts the extra think. The node keeps one brain state (S47): its trace and well-being come from one member drawn
  across both parts by members, as `spread` draws one member across the options today; its memory and signal come
  from that member's part's think.
- B3: the light of a cell passes the young part and the adult part of each node as two layers by their heights
  (Purves et al. 2008, the perfect plasticity approximation; Moorcroft et al. 2001).

### 3.9 Who may join whom (B2)

One predicate on data, `nodes::may_join`, answers for every merge path (rule 1 of 2.2), so `copy::kin` can ask it for
children before they are a body. Past `crowd`, the most alike pair of one lineage merges when the merged node's pooled
spread stays within its cap on every gene (S7).

### 3.10 The physics and the survey on threads (D3, D2.6)

Per-cell work that today runs in a serial loop runs through `par::rows` (rows made in parallel, then summed in the
fixed order of the cells). Sums across cells keep their order, so the history is the same bit for bit (level 1). D3
does it for Flow and Change; D2.6 does it for the survey, per domain (level 2: its parallel crowd changes the order of
merges and of the remap of new born rows).

### 3.11 Columns and the stage path (E1, E3: gated)

- E1, only if its gate passes: inside a domain, bodies are columns (the scalars of all bodies in arrays, the stores in
  one array of bodies × substances, the genes and spreads in one array each, the variable parts per body). Code reads
  a body through `BodyRef` and `BodyMut`; field writes become methods where the columns need them, so the laws' code
  does change. Same arithmetic in the same order: level 1.
- E3, only if its gate passes: the update of the due bodies without a brain of a domain runs stage by stage over all of
  them, each stage a short loop. Bodies with a brain keep the per-body path (rule 3).

---

## 4. The work list

Each step has an ID, a check level, a "done when", a kill line (when it does not pay), its line estimate (2.4) and the
agent that builds it. "Parallel with" names the steps that can run at the same time in their own worktrees. The steps
are listed in the order they are built (section 5).

### Phase A: measure, and the free wins (done)

- **A1, the bench and profile kit:** `scripts/bench.sh`, the baseline and the callgrind profiles in `measures.md`
  (`0a3d1e6`).
- **A2, compiler settings:** only `codegen-units = 1` is kept (S35): 0.702 s a year on the island and 1.220 s on the
  globe. Fat LTO, `target-cpu=native`, `panic = "abort"` and PGO are not kept (each under its bar; `measures.md`). The
  golden fingerprints moved at level 2.
- **A3, many worlds at once:** three worlds side by side run 2.9 times faster than one after another. Inside one
  process the seeds share rayon's one pool; separate `ew` runs side by side need no `RAYON_NUM_THREADS` on 4 cores
  with 3 runs. The advice is in `measures.md`.
- **A4, a faster machine:** not code; the report tells the owner what a newer CPU would give (S5).

### E2.1 and E2.2: two hot spots (right after phase A)

Each its own commit with its measured gain (S31). **Kill line:** a spot that gains under 2% of a bench's
instructions (callgrind, 2.6) is taken out again.

- **E2.1 `spread::part_acts`.** For each option part, `nodes::part` made a full clone of the node (`b.clone()`, with
  its genes, spreads, rows and memory) and `nodes::take_in` put it back through `nodes::add` (sizes, the rows of a
  watcher, the row spread pooled and capped, `see`). The measured inclusive share of `nodes::add` was 8% of the globe's
  instructions (`nodes::take_in`'s 6.5% overlapped it). Where the part stays (any action but a move or a bond), the
  node acts itself with the part's share while the others wait aside, and they come back by `nodes::rejoin` (amounts,
  sizes with the gap term of their variance, the rows of a watcher; S51). It drops the pool and cap of the row spread
  at a watcher's return, so it is **level 3**, with the key-number test (S49, S50). `docs/laws.md` (4.4) says it.
  Agent: Opus.
- **E2.2 `pow`** (2.4% exclusive on the island): powers with a fixed exponent (mass^0.75, `10^x`) as cheaper forms.
  Level 2 (other rounding). Agent: Sonnet.
- What the first plan also named is handled elsewhere: the sample (`copy::select::sample`) in B1 (S37); `nodes::crowd`
  is not cached (caches are banned; B2 rewrites the crowd); the `apply` per update is replaced by D2.3's store view.
- **Code:** each pays for its own lines. **Parallel with:** D3.

### The parallel lane: D3

#### D3. The physics and the survey on threads

- **Measure first** (a Haiku agent): callgrind on both benches, Flow's serial share per loop, written in
  `measures.md`. The loops worth it are chosen from that.
- **What** (S21):
  - **Flow:** the per-cell loops that run serially today run through `par::rows`, their cross-cell sums kept in the
    fixed order of the cells. Candidates: the air step's `fixed`, `room`, `seen`; the air's `gather` per air node over
    its cells in cell order; land's `ground_state` and `surface`; liquid's `slopes`, `mobile`, `surface`'s rates and
    `follow`. The priority-flood order, the downhill pass and the conjugate gradient's dot products stay serial.
  - **Change:** its reactor already runs on threads; its serial parts (noting each node's row in `Batches::add`,
    booking) go on threads where their sums keep their order; and a profile-driven look at `Reactor::react` (2.4%
    exclusive, about 9% inclusive; slice indexing alone 1.2%), keeping results bit for bit where possible. A change of
    the reactor's arithmetic is level 2 and its own commit.
  - **The survey's** per-cell loops are done in D2.6, per domain.
- **Why:** Flow is 17 to 18% of a year and uses one core outside the air's `pass`; Change is 4 to 6%. The rest of a
  year must shrink too (1.3).
- **Challenges and answers:** a sum across cells must keep its order to stay bit for bit: each parallel task writes its
  own row; the sums are made afterwards in the old order. The golden test runs with and without threads.
- **Check level:** 1 (bit for bit), but 2 for a change of the reactor's arithmetic. **Done when:** the golden test
  keeps its fingerprints with and without threads; Flow is at least 1.5 times faster on 4 cores on the benches.
  **Kill line:** a loop stays serial unless it gains above the noise (2.6): in instructions for its serial share, in
  interleaved wall time for its threads.
- **Code:** about +30. **Agent:** Sonnet. **Parallel with:** E2, B, C (it touches `flow/`, `solve/` and `change/`
  only).

### Phase B: fewer bodies

#### B1. One node per kind per cell, its young inside it

- **Measure first** (a Haiku agent, both benches): the share of bodies that are young nodes, the share of updates they
  take, and how many of them have no adult node of their kind in their cell (S29). Call `s` the share of bodies that
  are young nodes **with** an adult node of their kind in their cell, and `u` their share of updates. Those young
  nodes go into their adult node; a young node without an adult of its kind becomes the node of its kind holding only
  young (`n_a` = 0; no fewer bodies there).
- **What:** as in 3.8, for every population node: the state (`n_a`, the adults' share of the built matter, the young
  moments), the parts rules, growth split by supply, maturing inside the node, the two starvations, children who stay
  in the young part. `copy/young.rs`, `Body::link`, `Grown::Joins`, the stage in `nodes::Kin` and in `merge_closest`'s
  key, and `copy::emit`'s young-node branches go. Brood is made from the adult part's growth only (`ALLOC`).
- **Why:** young nodes update as often as adults; one node per kind removes a body wherever both live, and one
  mechanism replaces two (the main rule).
- **The audit** (a Haiku agent): every `\.n()` site (about 50), every use of `member_body` and `size`, each assigned
  to "totals" (laws linear per member: costs by kg, the store checks, death by wear) or "per part" through the one
  per-part helper (3.8): among them `Body::adult()`, `Body::member_body`, `live::starved`, `copy::copy`'s
  `BROOD_HELD * n` (adult members only), supply by mass^0.75 (`Body::supply`), leaf area through `equiv`, the skin's
  member mass, the thinking cost in `upkeep` (so upkeep is not linear as a whole), `walk`, perception's `pm` and `vol`,
  `fuel / n` in well-being, `take_body`'s `fb`, `fo` and `per`, `Body::height`. `Body::vis` stays the node's mean
  member. The list goes in the commit message.
- **The selection sample** (S37): drawn from one part (a chance by members), at that part's size, and compared with
  that same part's gain in the node's own update: ln(the part's buildable matter after / before), the shared reserves
  counted to each part by its share of the built matter. So a young member is compared with the young, not with the
  whole node.
- **Nodes with a brain** think once per part (3.8, S36).
- **The nodes check first** (S36): F7 (`tape/tests/nodes.rs`, `nodes_and_individuals_make_the_same_world`, about 5
  minutes) runs **before B1 merges**, against today's result. **The fallback, decided now:** if a rule of F7 that holds
  today breaks because of nodes with a brain, B1 is restricted to nodes without a brain, `copy/young.rs` stays for
  nodes with a brain, and that two-mechanism exception is logged as a decision.
- **Children** who start in another cell, and children of individuals, join the young part of the node of their kind
  there when they are alike (`copy::kin`, which B2 folds into `nodes::may_join`), else start a node.
- **References:** Lefkovitch 1965 and Caswell 2001 (stage-structured populations); McKendrick 1926 and von Foerster
  1959 (the flow across a size); Kleiber 1932; Weiner 1990 (size hierarchies in plant stands).
- **Challenges and answers:**
  - *The books.* With one store, births that stay and maturing move no matter, so they cannot break the books; the
    tests that hold the books of young and adult nodes are rewritten for one node. A test holds the invariant of 3.8
    after each kind of matter change.
  - *E11.* Separate moments for the young and the adults, never pooled across the copy mass (3.8).
  - *The nodes check (F7).* It may move. It runs before B1 merges (above) and again at the end of phase B, its result
    written in the report; a rule that newly fails is a bug to find, not a range to widen.
- **Check level:** 3, checked with B3 (two commits, one key-number run after B3; S33). **Done when:** on both benches
  the bodies alive fall by at least 0.8 `s` and the updates by at least 0.8 `u`; the pace test holds; the key numbers
  hold after B3.
- **Kill line:** if the bodies' instructions on the benches (callgrind, 2.6) fall by less than half of `u`, stop and
  report.
- **Code:** about +160 −70 (2.4). Snapshot version bumped; `docs/laws.md` (4.4, the nodes) changes. **Agent:** Opus.
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
- **Check level:** 3, with B1: the key-number run of both comes after this commit. **Done when:** a test holds that the
  shorter part of a node gets less light per area than its taller part under a closed canopy; the key numbers hold.
- **Code:** about +20. **Agent:** Opus. **Depends on:** B1. **Then:** B2.

#### B2. Cells that cannot fill without end

- **What** (S7):
  1. **One predicate on data.** `nodes::may_join` takes the data of both sides (lineage, genes, spread, and the hook
     for a mind that stays individual), not two bodies, so `copy::kin` can ask it for children before they are a body.
     Every merge path asks it: `join_alike` and `merge_closest` (the monthly crowd), the kin joins of `copy`, and
     `spread::part_acts`'s members that move. It holds the tests that are spread over `kin_of`, `merge_closest`'s
     distance and `copy::kin` today, with no change of behaviour (level 1 for this part).
  2. **The milder rule.** Past `Config::crowd` nodes at a month's start, the most alike pair of one lineage in a cell
     merges when the merged node's pooled spread (the variance of a mixture: within plus between, `nodes::add`) stays
     within its cap on every gene: `body::most` times `Config::merge_cap` (1). Unlike nodes beyond that stay apart.
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
  - *The world must not change past its noise:* `merge_cap` is a speed setting with its row in `docs/numbers.md` and
    its × 2 and ÷ 2 test; the `crowd` equivalence (handover 7.1's noise) runs again.
- **Check level:** 3. **Done when:** measured on the benches (the most bodies in a cell at a month's start, before and
  after B2) and in phase Z on a 1,000-year run of the refined region. If the milder rule does not bound the bodies, B2
  reports with numbers; a stronger rule needs a new decision.
- **Code:** about -10. Snapshot version bumped (`Config::merge_cap` is a new saved field). `docs/laws.md` 4.4, the
  `crowd` row and a new `merge_cap` row change. **Agent:** Sonnet. **Depends on:** B3.

### Phase C: fewer updates

#### C2. A closer step over a node's interval (node_days 36 → 70)

- **What:** find what makes nodes that update rarely differ from nodes that update often (the pace test fails at 144
  days: births -55%), and approximate those terms better over the interval. Then set `node_days` to 70.
- **Why:** a population node without a brain updates every `node_days` (36). At 70 its updates nearly halve. 70 is a
  multiple of `batch_days` (5), which D2.2 needs (S8).
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
     It is not an exact solution: the node is not linear. Its error is measured at 40, 70 and 140 days and written in
     `measures.md`.
  3. Set `node_days` to 70. The pace test (`the_pace_of_nodes_keeps_life` in `tape/tests/ranges.rs`) then runs at 40
     and 140 days around it: ÷ 1.75 and × 2, as 35 is not a multiple of 5; 40 and 140 are multiples of 5 and of 10.
- **Check level:** 3. **Done when:** the pace test passes at 40 and 140 around `node_days` 70; the key numbers hold.
- **Kill line:** if the pace test cannot pass at 140, C2 is killed: `node_days` stays 36 until D2.2 sets it to 40,
  and the method stays only if it makes the pace test closer at 70; the rest is logged.
- **Code:** about +40. `docs/laws.md` and the `node_days` row change. **Agent:** Opus. **Depends on:** B1 (the node's
  parts live in one body).

#### C1. Dormant bodies rest

- **Measure first** (S12): a Haiku agent counts, on both benches, the share of updates of bodies without a brain
  whose activity (`Body::activity`, laws.md 6.5) is under 0.05. **Kill line:** under 5% of updates, C1 is not built
  and is logged; once built, it is judged on the benches' instructions (2.6).
- **What** (S27): a body without a brain whose activity at its update is under `Config::rest_activity` files its
  next update in the ring at `MAX_INTERVAL`. `Body::activity` measures the key liquids held, whatever their phase, so
  this is a **dry** body (little liquid held); frozen bodies are not singled out. A new field `Body::due` (its due day;
  its due batch after D2.2) makes ring entries that no longer match it skip; `due` is checked when an entry is
  processed, not when the batch's list is built. At each month's survey, a wake check reads the cell (its ground
  temperature and liquid, the monthly `Life::env`) **and** the body's own activity, since a resting node can take in
  active members by kin joins and arrivals; when either would raise its activity past the threshold, it moves its due
  day earlier (a new ring entry; the old entry then skips). The body integrates the whole time since its last update
  when it runs, as every update does. `clock::Sleep` is not used: it stays the sleep of store nodes.
- **One place sets `due` and `gap`:** `Life::add` (and the filing of an update after it ran). `nodes::part` clones a
  body, so it resets them, and the part gets them from `Life::add` when placed.
- **The real gap.** Laws that read the time until the next update read a stored real gap, `Body::gap` (days: the due
  day minus the last update's day, set when the update is filed and again after a wake), not `interval(w)`: `feed`'s intake days and the chance to get away (`take_body`'s `away`),
  `copy/grow.rs` (its allowances), `survey::Draws::settle`, `body::pay` and the span of `live::settle_upkeep`, and
  `keep_warm`'s span; the callers of `body::pay` that pass `b.interval(w)` as an act's span (`choose::go`, `feed`,
  `bond::verbs`) read it too. The builder lists every read of `interval` and assigns each to "the gap" or "the gene's
  pace" (`Body::paces`); the list goes in the commit message.
- **The threshold** `Config::rest_activity` (not `dormant`: `Physics::dormancy` exists) is a speed setting with its
  row in `docs/numbers.md` and its test (× 2 and ÷ 2 against no rest), in this step.
- **Why:** a dry body's chemistry is nearly stopped (handover 4, "a dry body is nearly dormant"); updating it
  every interval costs time and changes nothing.
- **Check level:** 3. Snapshot version bumped (`Body::due`, `Body::gap`). **Code:** about +25. `docs/laws.md` (6.5)
  changes. **Agent:** Sonnet. **Depends on:** C2.

C3 (batches of days) is built as D2.2, its first use.

### Phase D: all four cores

#### D2. Domains: the bodies on all cores

Built in sub-steps on `main`, each committed when it passes its checks (S3, S16). If D2 is killed, its commits are
reverted. The adversarial review runs after D2.3 and after D2.6.

- **D2.1 Domains own their bodies (sequential).** `Life` holds a `Domain` per air node: its bodies, its cells' lists
  and its due ring. Stable body ids (3.3) with the id table. `Life::alive` and the per-cell iteration stay as accessors,
  so the tests, the tape and `worldgen` change little (about 256 uses of `life.bodies` and `life.at`). Born rows stay
  shared (S11). Bodies still run one after another, in the order of their cells. Level 3 (new ids key the random
  draws). Snapshot version bumped; `docs/tape.md` (the ids in events).
- **D2.2 Batches (C3).**
  - **What** (S8): bodies update in **batches** of `Config::batch_days` days (5, the shortest interval
    `MIN_INTERVAL`). A batch runs, at its first day, every body due in it. `survey::day` runs once a batch.
  - **A body's interval is a whole number of batches:** its `INTERVAL` gene's days rounded to the nearest multiple of
    `batch_days`, at least one batch (the gene constant `Body::k.interval`; `Body::prepare` gets `batch_days` from the
    `Config`). So every due day is a batch's start, no body runs early, and there is no jitter.
  - **The ring** (`life::RING`) is indexed by batch, with at least `MAX_INTERVAL / batch_days + 1` slots.
  - `node_days` must be a multiple of `batch_days`: 70 after C2; if C2 was killed, this step sets it to 40.
    `MAX_INTERVAL` (360) is one already.
  - `Config::validate` refuses a `batch_days` that does not divide `phys_days` and the month, and a `node_days` that is
    not a multiple of it.
  - A body placed during a batch (`World::add`, `World::place`, children, splits, parts placed by `part_acts`) is first
    due at the next batch's start or later.
  - **Why:** a day holds 1.5 to 15 ms of body work: too little to share between threads (decision 59 of the
    foundation). A batch of 5 days holds five times more, and D2 needs one batch as its unit of work.
  - **Cost:** short intervals lose resolution (6 and 7 days both become 5).
  - **The equivalence test** compares `batch_days` 1 and 5 only (10 would change the shortest interval itself, a law):
    on the pace test's world (32 × 32, 8 years, through the `yearly(&LIFE, ...)` machinery of
    `tape/tests/ranges.rs`) and its columns (`plant_members`, `root_0_kg`, `births`, `root_0_life_real`, `air_3`),
    each mean moves by less than its noise.
  - **Check level:** 3. Snapshot version bumped; `batch_days` gets its row in `docs/numbers.md`; `docs/laws.md` says
    intervals are whole batches.
- **D2.3 The update runs on its domain.** First the Haiku count of serial bodies (3.2; over 10%, the architect
  revisits). The update code (`body/`, `choose/`, `copy/`, `nodes.rs`, `lineage.rs`) takes a context that holds the
  world read-only and its domain writable: its bodies, its store view (3.4, with checkpoints for `copy::sample`), its
  scratch, its events and counters, its per-domain state (3.2). Each batch starts with the faces (3.5); perception
  reads them and `Choice::seen` holds ids. The batch's moves of a domain are booked by one `apply` per domain after
  the batch. `region24` joins the bench, and the bench prints the busiest domain's share (1.2). Writes outside the
  domain still happen at once here (one thread). **Tests:** a sampled update leaves the view as an unsampled one would.
  Level 3 (faces from the batch's start; one `apply` a batch).
- **D2.4 The outbox and the serial phase, whole.** Every kind of 3.6 with its fixed order and its failure rule; every
  relocation goes through it and a leaving body follows S39; every action on a part is a part act (S52); the serial bodies run in the serial
  phase (S23); provisional lineages, born rows and counts settled there (S24, S11); dispersing litters as in S25;
  watches as in S40. **Tests:** one makes each outbox kind fail once (a relocation of a body that died, arriving
  members that are gone, a litter with no kin back to a living parent and to the ground of a dead one's cell, a watch
  of a body that died and of one whose lineage changed, a part act that can no longer be paid or whose part is gone, and a node's part with a
  queued part act that rejoins its node after it and one that cannot rejoin) and checks the books; one
  checks that the remap gives the same lineages and born rows with and without threads. Level 3. `docs/laws.md`
  changes.
- **D2.5 Threads.** With the feature `threads`, the domains of a batch, taken out of `World`, run on rayon's pool
  (`par_iter_mut`); without it, in order. The golden test gives the same fingerprints both ways. **The order test**
  (S42): in one test binary, a small world forced to have many cross-edge moves, litters and watches runs with rayon
  pools of 1 and of 4 threads and with the domains in reversed order inside the parallel phase, and gives the same
  fingerprint each time. The reversed order is an argument of the batch runner that only the test passes, not a
  `Config` switch (`CLAUDE.md` 2.6).
- **D2.6 The survey on threads.** Wear, the monthly crowd, shade and sums run per domain in parallel; the Draws of the
  air step and the marks of light are filled per domain. Cells whose bodies hold or are inside a part, or that hold a
  part, run their wear and crowd serially after the domains (a death or a merge may let go of a part). Deaths in the
  parallel `wear` change lineage counts by per-domain deltas merged in order. A merge in the parallel crowd can make
  new born rows (`nodes::add` → `Life::reborn` → `Life::new_genome`): they get provisional ids of the domain and are
  remapped after the domains, in domain order and then in the order made, as in 3.3; then the cells with parts run.
  `counts.clamped_heat` (saved) stays summed in cell order. Level 2 (the order of merges and of the remap changes
  rounding): the golden test is re-pinned. Built before the kill decision.
- **Why:** bodies are three quarters of a year on one core. The waves of blocks of the foundation failed (decision
  59) because a day held too little work and each block was copied onto a world of its thread's own; here a batch holds
  five days of work, and nothing is copied: each domain owns its bodies.
- **References:** the domain decomposition of agent models (FLAME GPU, Richmond et al. 2010; Repast HPC, Collier and
  North 2013); deterministic parallel order (Bocchino et al. 2009, "Parallel programming must be deterministic by
  default").
- **Challenges and answers:**
  - *A body reads other bodies:* only through faces (3.5); acts in its own cell are live; watching goes to the outbox.
  - *A body reads other cells:* the store as it was at the batch's start (3.4); the compiler refuses a live read of
    another domain (3.2).
  - *Parts:* only serial bodies and part acts touch them, in the serial phase (3.2).
  - *Load balance:* sea domains hold few bodies, a refined region's one domain holds many. Rayon's work stealing takes
    domains one by one; the bench prints the busiest domain's share; `region24` shows the worst case.
  - *The store view must not overdraw:* 3.4.
  - *wasm:* without `threads`, the domains run in order, the same history.
- **Done when** (S41): the golden test and the order test hold; the key numbers hold; on the benches **the bodies'
  stage (survey included) is at least 2.2 times faster on 4 cores than in the last `measures.md` row before D2.1**.
- **Kill line:** under 1.6 times (wall time, 5 or more interleaved runs, 2.6), stop and report; D2's commits are
  reverted.
- **Code:** about +520 (2.4). **Agent:** Opus for D2.1, D2.3, D2.4, D2.5; Sonnet for D2.2 and D2.6. Sequential
  (they touch the same files).

#### D1. Work off the main thread

- **D1 (a): not built** (S56). The bench's tape stage is the recorder reading the world on the main thread (its month
  about 6.5 ms and its yearly row about 5.5 ms a year on `island22`); the file writing a thread could take is outside
  it and about 0.5% of a year in a real run. The recorder's reading is a hot spot for E2.3 (`tape/`, no core line).
- **D1 (b):** after D2 and D3, measure the cores' idle time in a year. If more than 25% of a year leaves three cores
  idle, build the physics of the next step to run beside the bodies through a double bodies' node (S9). **Check
  level:** 3 (bodies take from what the last air step filled for them). **Done when:** the measure is in
  `measures.md`; if built, a year falls by at least half the idle share measured, and the key numbers hold.
  **Kill line:** under 5% of a year (interleaved wall time, 2.6), taken out. **Code:** about +80.
- **Agent:** Sonnet.

### Phase E: a cheaper update, after D2

#### E2.3. Hot spots after D2

- **What:** profile both benches and `region24` after D2 and pick the new hot spots (among them the tape's recorder,
  about 12 ms a year of reading the world on the main thread: S56), each its own commit with its
  measured gain, as E2.1 and E2.2. **Kill line:** 2% of a bench's instructions (2.6). **Code:** each simplifies as much as it adds.
  **Agent:** Sonnet (Opus for a spot in the update's structure).

#### E1. Bodies as columns (gated)

- **Gate:** a callgrind run with `--cache-sim=yes` on both benches after D2. E1 is built only if the body update loses
  at least 10% of its estimated cycles to cache misses (callgrind's estimate: instructions plus the misses weighted by
  their cost). Else it is logged as not built (S31).
- **What:** as in 3.11. A domain stores its bodies as columns; the code reads a body through `BodyRef` and `BodyMut`;
  field writes become methods where the columns need them.
- **Why:** each body is a struct with its own heap lists; columns keep a domain's bodies together in memory.
- **Check level:** 1 in what it computes (only the layout changes), but the snapshot version is bumped and the
  fingerprint hashes the saved world, so **the check is: the golden test re-pinned, and the series of the golden worlds
  bit for bit equal for 2 years** before and after. **Done when:** the bodies' stage is faster by at least half the
  share the gate measured. **Kill line:** under 3% faster (interleaved wall time, 2.6), it is taken out again.
- **Code:** simplifies as much as it adds. Snapshot version bumped. **Agent:** Opus.

#### E3. A stage path for bodies without a brain (gated)

- **Gate:** a profile after D2 (and E1 if built) shows a stage of the updates of bodies without a brain repeating work
  worth at least 10% of the bodies' time (the thermal state, sums over stores). Else it is logged as not built.
- **What:** that stage runs as a loop over a domain's due bodies without a brain: what it needs is computed once per
  body, and the loop is short. Only auto-vectorisation: no `std::simd` (nightly) and no new crates. Deaths and
  children inside a stage are kept as marks and settled after it, in the bodies' order. Bodies with a brain keep the
  per-body path (rule 3): the code with a brain (`choose::act`, 19% of the globe) is not touched by E3.
- **Check level:** 3 (S15). **Gain:** unknown until the gate's profile. **Done when:** the bodies' stage is faster by at
  least 5% of the bodies' time; the key numbers hold.
- **Kill line:** a stage that gains under 5% of the bodies' instructions (2.6) keeps the per-body path.
- **Code:** simplifies as much as it adds. **Agent:** Opus. **Depends on:** D2 (and E1 if built).

### Phase Z: the final check

- **Z1:** the three benches and their profiles, with the busiest domain's share; the cost model of a cell and of a body
  again; the largest planet that fits 15 minutes.
- **Z2:** the slow suite (key numbers, equivalence tests, the nodes check), the golden test without threads.
- **Z3:** 1,000 years of the island 22 × 22, the globe 36 × 18, the refined region (B2's done-when) and the largest
  planet that fits; books, bodies and time.
- **Z4:** the watch report `docs/reports/speed.md` with pictures (S2), saying plainly where the speed stands against
  the aim of 10 times and what group F would add; the handover (sections 5.4, 7, 12.6), `docs/laws.md`,
  `docs/numbers.md`, `docs/tape.md` and `CLAUDE.md` (its table "Where things are" and 2.11) describe the present.
- **Agent:** Sonnet for the runs and the report draft, Opus for the final review.

---

## 5. Order and parallel lanes

```
A (done) ──► E2.1 ──► E2.2 ──► B1 ──► B3 ──► (key numbers) ──► B2 ──► (phase B review, F7) ──► C2 ──► C1 ──┐
   │                                                                                                         │
   └──► D3 (flow/, solve/, change/): measure, then a parallel lane beside E2, B and C ────────────────────────┤
                                                                                                             ▼
   main:  D2.1 ──► D2.2 (C3) ──► D2.3 ──► (review) ──► D2.4 ──► D2.5 ──► D2.6 ──► (review; kill decision) ──┐
          D1 (a) beside D2; D1 (b) after D2.6 and D3                                                         │
                                                                                                             ▼
                                              E2.3 ──► E1 (gated) ──► E3 (gated) ──► (review) ──► Z
```

- **Phase A is done:** every other step reads its rows.
- **E2.1 and E2.2 first:** they are small and measured; B and C change the same body code afterwards.
- **D3** touches only `flow/`, `solve/` and `change/`: it runs beside E2, B and C, in its own worktree.
- **B1, B3, B2 one after the other:** B3 and B2 read the node's parts that B1 makes; B1 and B3 share one key-number
  run; B2 re-pins the golden test after them.
- **C after B:** C2 approximates the parts that B1 puts in one body; C1 after C2 (both touch the timing of nodes).
- **D2 after C:** D2.3 touches every file of the update; it waits until B and C are merged. D2.6 comes before the kill
  decision, as its gain counts (S41).
- **E after D2:** the columns live inside the domains; the gates read profiles after D2. If D2 is killed, they read
  profiles of `main` and E1 is built on today's layout.
- **Reviews:** after E2, after phase B, after C, after D2.3, after D2.6, after E3, and the final review.

---

## 6. Risks and what to do

| Risk | What to do |
|---|---|
| Ten times is not reached | Expected (1.3, S20): `measures.md` says where it stands at every step; the report says it plainly, with what group F would add |
| The refactor of D2.3 never ends | Sub-steps that each run the island and pass the checks; D2.3 changes the reads, D2.4 the writes |
| D2 does not pay | Its kill line: its commits are reverted; E's gates read profiles of `main` |
| Too many bodies run serially (near parts) | The Haiku count before D2.3; over 10%, the architect revisits (S23) |
| A speed tool changes the world (B2, C1, D2.2, `node_days`) | Its equivalence test and the key-number test are part of its "done when"; a change past noise stops the step |
| B1 removes fewer bodies than hoped, or changes animals | Its target comes from its own measure; its kill line stops it; F7 runs before B1 merges, with the fallback to nodes without a brain written in S36 |
| The size moments blow up again (E11) | Separate moments for young and adults, never pooled across the copy mass; a test holds the invariant (3.8) |
| The milder merge of B2 does not bound the bodies | B2 reports with numbers; a stronger rule needs a new decision |
| The nodes check (F7) breaks more | Run at the end of phase B and of phase D; a new failure is a bug to find |
| Threads give a different history | The golden test with and without threads at every D step, and D2.5's order test; no step closes until they hold |
| The physics' threads change sums | D3 is level 1: the fingerprints must not move (a reactor change is level 2, its own commit) |
| The code ceiling | 9,700 (S3); each step names its deletions; the work aims at 9,500 (2.4) |
| A long run finds a books error late | The benches print the drift every year; phase Z runs 1,000 years on four worlds |

---

## 7. Words used in this plan

| Word | Meaning |
|---|---|
| bench | a settled world kept as a snapshot, run 5 years to measure speed |
| hot spot | a function the profile shows holding a large share of the instructions, made cheaper in its own commit (E2) |
| domain | the cells under one air node and the bodies in them, run as one unit |
| batch | the bodies due in `batch_days` days, run together at the batch's first day |
| parallel phase | the part of a batch where each domain runs its due bodies, on threads |
| serial phase | the part of a batch after it: the moves and the outboxes booked, then the serial bodies, in a fixed order |
| serial body | a due body that holds or is inside a part: it runs in the serial phase |
| part act | an action of a body in the parallel phase on a part, run whole in the serial phase |
| relocation | a body, or members of a node, going to another cell; it lands at the batch's end |
| leaving | a body with a queued relocation (or a node's part with a queued part act): it does not split, and others' searches skip it |
| outbox | what a domain's bodies did that lands at the batch's end, applied in a fixed order |
| store view | a domain's store nodes as the moves of its batch leave them, with checkpoints |
| face | what others see of a body (id, cell, members, lineage, visible traits, signal, well-being, room), kept from the batch's start |
| provisional id | the id a lineage or born rows made in the parallel phase hold in their domain until the serial phase gives a global one |
| part (of a node) | the adult or the young members of a population node after B1; they share one store |
| column | one array holding one field of all bodies of a domain |
| kill line | the gain under which a step is taken out again |

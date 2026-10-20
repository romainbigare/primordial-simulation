# The foundation plan: a stronger base before M4

**Status:** approved by the owner on 17 October 2026. All decisions are in Section 3. Where this file and
`docs/handover.md` differ, the handover wins until it is updated. What was built, and its numbers, is in
`docs/history.md`.

This file is for the agents who do the work, and for the owner. It is written in simple English (handover 0,
"Working with the owner").

---

## The main rule: the foundation REPLACES the code, it is not one more patch

**Read this before anything else. It is the owner's first wish for this work.**

During M0 to M3 every review added one more special piece of code. Each piece was right, but together they are a pile
of small rules. **The foundation exists to replace that pile with a few general pieces.** It must not sit on top of
the old code as one more layer.

The idea in one sentence: **a body, a patch, a wall, an air layer are all just nodes; everything that passes between
them goes along links; a gene or a material only says how easily a link passes.**

### What the special code became

| Special code | The general thing | Status |
|---|---|---|
| How a body trades heat, gas, liquid and salts with its cell | The formulas stay (they are the physics); each trade is a `Move` booked by `apply` at the body's update | Built (E5) |
| Pools that held light and heat for bodies between updates | One bodies' node per cell, settled in the air's step | Built (E5, J3) |
| The walls of a shelter keep a body warm | Links that pass heat badly | Built (D7) |
| The chemistry's own lazy timer | The clock's one sleep and wake | Built (D6) |
| Flow's own timers | The clock's one list of processes (`schedule.rs`) | Built (E8) |
| Merging, splitting and opening population nodes | One helper that moves members, amounts, heat and gene moments by shares | Built (E6, F1) |
| Ground cleared along edges | A field of the links (`store::Links`) | Built (D8) |
| Heat sideways between cells, pore liquid between cells | `spread` on links | Built (D1, D2) |
| The surface liquid's sorted sweep; erosion sampled every 7 months | The `downhill` pass, every step | Built (D2, D5) |
| Taking heat from a node with a guard in each place | A law never asks a node for more than it holds; `apply` only adds | Built (D11) |
| Caches saved with the state | A small saved state; caches rebuilt on load | Built (E8, E10) |

### How to work by the main rule

1. **Before you write code, find the special code your task replaces.** Delete it in the same branch. A task that adds
   the general piece and leaves the old one is not done.
2. **A law says *what*, the base does *how*.** A law gives a conductance, a capacity, a rate, a share. The loops, the
   time steps, the solving and the books live in `space/`, `store/`, `clock/` and `solve/`, once.
3. **No special path by node class inside a law.** If you write `if` on whether a node is a body, a patch, a part or a
   cell, stop. Ask which link or conductance says the same thing. Only where the physics really differs is a class
   test allowed, with a one-line comment saying why.
4. **No new pool, cache or catch-up for timing.** The clock and the moves handle time.
5. **Two functions of the same shape become one.**
6. **If the general way does not fit, do not add a special one.** Stop and tell the owner, with the case.
7. **Every commit message ends with its line count:** `Lines: +X -Y (core code N)`.

### The line budget (decision D2)

- **`core` may hold at most 8,000 lines of code, not counting comments and blank lines** (handover 14.3). Comments
  help the next agent: do not delete them to save lines.
- The new base has a fixed allowance of code lines. It must not grow past it without the owner's agreement:

| New part | Allowance (code lines) |
|---|---|
| `space/` (meshes, levels, stacks, the globe, a refined region) | 500 |
| `store/` (nodes, amounts, moves, links, the split and join helper) | 400 |
| `clock/` (processes, sleep and wake) | 250 |
| `solve/` (`spread`, `downhill`, `band`) | 350 |
| **Total** | **1,500** |

- The test `cli/tests/core_size.rs` fails above the ceiling: **9,000 until the end of the foundation**, **8,000 at its
  end (Phase L)**. The 8,000 is never a reason to build a task that costs speed or honesty.
- A phase may not end with more code than it started with, unless its report gives the reason.

### The time budget (the owner's rule, decision D12)

**The foundation aims at time, not at a size. A run between phases never waits hours.**

| Run | Budget, on the owner's machine (4 cores) |
|---|---|
| The normal test suite | under 30 seconds |
| The key-number test (3 seeds × 30 years, 64 × 64) | under 10 minutes |
| **A planet run of 1,000 years** | **under 15 minutes** (about 0.9 s per simulated year) |

- **The planet's size is not fixed.** It is the largest number of cells (on the globe) whose cost fits the budget, by
  the cost model (5.7). It is measured again at the end of each phase. Faster code buys a bigger planet.
- Speed never comes from removing a law or from a rule named after an outcome (handover 12.6). It comes from doing
  less work for the same physics: longer steps where the physics allows them, fewer updates where nothing changes,
  cheaper bookkeeping, cheap cells where little happens.

---

## 0. How to use this plan

1. Read, in this order: **the main rule above**, `docs/handover.md` (Sections 0 to 3, 5, 7, 8, 12, 14), the rest of
   this file, then `docs/laws.md`.
2. Every task has an **ID** (for example `B5`) and a **check level** (6.1). One task per branch:
   `claude/foundation-<id>`.
3. **The island must run at the end of every task.** `cargo test --release --workspace` passes, and the golden runs
   either keep their fingerprints (level 1) or are re-pinned on purpose with one line in the commit that says why.
4. **Each phase ends with a watch report**, `docs/reports/foundation-<letter>.md`, in simple English, with pictures
   and the numbers of Section 6 (handover 12.4). The owner reads it before the next phase starts.
5. When code moves, update `docs/laws.md` in the same change.
6. A new free number gets a row in `docs/numbers.md`. The count of tuned numbers may not grow (handover 14.3). Speed
   settings follow decision D4.
7. **The word ban holds** in `core/` and `worldgen/` (handover 14.4).
8. When a task cannot reach its "done when", stop and tell the owner with numbers. Never call a partial result done.

---

## 2. What stays the same (must)

- **The five laws and their equations.** This plan changes *where* the laws read and write, not *what* they say.
- **Chemistry 0** (`worldgen/src/chem0.toml`).
- **No Earth words** in `core/` and `worldgen/`.
- **Exact books.** Every element and the energy are counted to rounding.
- **Same seed, same build, same machine: same history** (handover 12.5). With and without threads.
- **No new body mechanics** during this plan (decision D6), only bug fixes. Moving body physics onto the new base is
  part of the plan.
- **Handover 8.2:** the step is a pure function; random numbers are keyed; fixed order; no IO in `core`; `core` builds
  for WebAssembly.

---

## 3. The owner's decisions

All decided on 17 October 2026. Later decisions are in `docs/foundation/decisions.md`.

| # | Decision | Status |
|---|---|---|
| D1 | The M3 check point choice (handover 13.1) | **Decided:** a foundation phase (this plan), then M4. Written in handover 13.1. |
| D2 | The `core` code budget (handover 14.3) | **Decided:** at most 8,000 lines of code in `core`, not counting comments and blank lines, at the end of the foundation (9,000 until then). The foundation replaces the special code, it does not sit on top of it. The new base has fixed allowances; a test keeps the ceiling (A6). |
| D3 | The cell shape for the planet | **Decided:** a latitude-longitude grid of squares that wraps east to west (5.5). It keeps today's square laws, the brain's 8 directions, the exact split of a cell into 4, and the viewer (the grid is the map). |
| D4 | Speed settings (when a node sleeps, how many nodes a cell keeps, the node pace) | **Decided:** a table "speed settings" in `docs/numbers.md`, outside the tuned budget. Each must pass an equivalence test (6.4): changing it 2× must not change the key numbers beyond their noise. |
| D5 | Old snapshots | **Decided:** no backwards compatibility; snapshots carry a format version (`tape/src/snapshot.rs`). |
| D6 | Body physics during this plan | **Decided:** frozen, bug fixes only. Moving existing body physics onto the new base (same physics, new code) is part of the plan, not new physics. |
| D7 | How the work merges | **Decided:** phase by phase into `main`, after the owner reads the phase report. No long-lived branch. |
| D8 | The safeguard that merges unlike nodes in a full cell | **Decided:** removed once nodes are cheap (F3). If a cap is still needed for speed, it becomes a speed setting (D4). |
| D9 | Who keeps amounts at or above zero | **Decided:** the law. A law never asks a node for more than it holds; `store::apply` only adds, clamps rounding and counts it. |
| D10 | The check level of speed work | **Decided:** level 2 (the same to rounding) is the normal level for speed work: it may change the order of sums, and it re-pins the golden runs in the same commit. Level 1 stays for pure clean-ups. |
| D11 | Bodies and Flow | **Decided:** a body is not a node in Flow's steps (every Flow step would touch every body through stiff heat links: a 1 kg body against a ground of gigajoules). Bodies book their trades as moves at their own updates, and one body node per cell holds what Flow brings them between updates (E5). |
| D12 | The size of the planet | **Decided:** set by the time budget, not fixed. |
| D13 | How often a population node updates | **Decided:** at most about 10 times a year (the speed setting `node_days`). An individual keeps its `INTERVAL` gene. Checked by the speed-setting test (× 2 and ÷ 2) and by F7. |
| D14 | What waits until after the foundation | **Decided:** focus zones, automatic zoom while running, patches, more memories on links, threads over bodies, the chunk layout, a query layer and snapshots of changes only (Section 10). Each adds realism or speed later, but costs speed or code before M4 needs it. |

---

## 5. The architecture

### 5.1 The layers

```
            ┌──────────────────────────────────────────┐
  tape,  ←──│  READERS: the tape, the CLI, the viewer   │   they read `&World` only
  viewer    └──────────────────────────────────────────┘
            ┌──────────────────────────────────────────┐
            │  THE FIVE LAWS                            │   core/src/{flow, change, bond, copy, choose}/
            │  they compute; they change the world only │   and body/
            │  through moves and link fluxes            │
            └──────────────────────────────────────────┘
            ┌─────────────────────┬────────────────────┐
            │ SOLVE               │ LIFE               │   core/src/solve/; life.rs, nodes.rs, body/
            │ shared maths        │ bodies and nodes   │
            └─────────────────────┴────────────────────┘
            ┌──────────────────────────────────────────┐
            │  CLOCK: who runs when; sleep and wake     │   core/src/clock/
            └──────────────────────────────────────────┘
            ┌──────────────────────────────────────────┐
            │  STORE: nodes, links, amounts, moves,     │   core/src/store/
            │  split and join, the books                │
            └──────────────────────────────────────────┘
            ┌──────────────────────────────────────────┐
            │  SPACE: the shape of the world            │   core/src/space/
            └──────────────────────────────────────────┘
```

### 5.2 The contract between layers (must)

| Layer | May read | May write | Must not |
|---|---|---|---|
| SPACE | nothing else | its own mesh, when it is built or rebuilt (a refined region, G4) | know about substances, heat or bodies |
| STORE | SPACE | amounts, heat, links' state, through `apply` and the split and join helper only | decide anything physical |
| CLOCK | STORE (wake marks) | the time, the queue of what is due | change amounts |
| SOLVE | inputs given to it | its outputs (fluxes, new values) | touch `World` directly |
| LAWS | everything below | moves and link fluxes, given to STORE | write an amount or a heat directly |
| LIFE | everything below | through STORE and CLOCK, as the laws do | hold a second copy of anything the store holds |
| READERS | everything, through `&World` | nothing (they take the events and counters through two small methods) | be used by the laws |

The compiler checks it: the arrays are private fields of `store::Store`, written only by `apply`.

### 5.3 The ideas, in short

How each works now is in `docs/laws.md` (section 0 and the laws).

1. **Everything is a node.** A fixed node is named by its class and index; only parts and bodies have a slot and a
   generation.
2. **Moves, transforms and link fluxes** are the one way to change the world. A law never asks a node for more than
   it holds; `apply` only adds. Sun, space and outside are boundary nodes, so one check covers every law's books.
3. **One small split and join**, by shares; what differs by class stays with the class.
4. **A mesh with levels and stacks**; detail in places is a refined region built into the mesh.
5. **Links are real**; a new link field comes only with the law that needs it.
6. **One clock** in whole seconds, one list of processes, one sleep and wake for nodes (by `wake_share`).
7. **Shared maths** in `solve/`: `spread`, `downhill`, `band`.
8. **Columns**; chunks and the chunk layout wait (D14); no sparse amounts.
9. **A fast life engine**: cheap bookkeeping, fewer updates, no brain where nothing is decided, selection inside
   nodes by sample members.
10. **Readers read `&World`.**

### 5.4 Focus zones (after the foundation, D14)

A **zone** is a set of cells taken down to about 30 m with its own fast clock: single members out of population nodes,
positions and speeds, day and night, actions that take time. It opens and closes on physical facts only, and **the
coarse rule outside must be the average of the fine rule inside** (a zone-against-no-zone test). After M4, with the
mind.

### 5.5 The shape of cells (decision D3)

A lat-lon grid of squares that wraps east to west: the square code carries over, the brain's 8 directions are the same
everywhere, the split into 4 is exact and the grid is the map. Cells near the poles are narrow and no link crosses a
pole; rows near the poles can be merged later if they cost too much.

### 5.6 Threads over bodies (after the foundation, D14)

The design, for when a planet profile shows one native thread for the bodies limits a long run: square chunks wider
than the longest sight, in 4 colours (2 × 2); a day's bodies run in 4 passes, the chunks of a pass on threads. Writes
across a chunk's border, new ids, counters and events go to per-chunk lists applied after the pass in chunk order;
random numbers are keyed by body and time. A run without threads runs the same passes, so the history is the same
with and without threads (level 1 after one level-3 change).

### 5.7 The cost model

The time of a run is the cost of one cell and of one body node per simulated year, times their numbers. Measured at
review round 2 (one thread, the CLI's stage times):

| Run | One cell, Flow and Change | One body node |
|---|---|---|
| 64 × 64 island, no life | 0.15 ms | |
| 64 × 64 island, life, years 26-30 | 0.43 ms | 350 µs |
| 64 × 32 globe, light-eaters | 0.29 ms | 320 µs |

At one body node per cell, about 1,100 cells fit the budget of 0.9 s a year. Threads give little today: the air's cell
pass and Change's reactor run on threads (Flow about 5%, Change about 20% on 4 shared cores); the solves, liquid and
the bodies run on one thread. Threads over bodies (5.6) are the first lever after the foundation if the planet is too
small to watch. Each phase report measures these numbers again and gives the planet size they allow.

---

## 6. How we check (the safety net)

### 6.1 Check levels

| Level | Meaning | How it is checked |
|---|---|---|
| **1: the same** | Bit for bit the same history | The golden fingerprints do not change |
| **2: the same to rounding** | Only the order of sums changed: the normal level for speed work (D10) | After 2 years of the golden worlds, every series column within 1e-9 of the old value (relative), books exact; where life's choices grow a rounding into a different history, the key numbers of level 3 instead. The golden runs are re-pinned in the same commit |
| **3: the same world** | The physics or the speed tool changed on purpose | The key numbers stay inside their ranges (6.3); every change larger than the seed noise is explained in the phase report |

### 6.2 Reference runs

A2's runs in `docs/reports/foundation-a/`: seeds 1, 2, 3 × 60 years × 64 × 64, normal and `--roster`; seeds 1, 5, 7 ×
300 years × 128 × 128.

### 6.3 Key numbers and their ranges

One slow test (`tape/tests/ranges.rs`, `#[ignore]`) on 3 seeds × 30 years × 64 × 64:

| Number | Range |
|---|---|
| ground temperature on land, mean of years 21 to 30 | the reference ±1 K |
| seasonal swing on land | the reference ±1.5 K |
| rain on land | the reference ×/÷ 1.3 |
| absorbing gas | the reference ±40 ppm |
| light-eater kg, eater kg (log10 at year 30) | the reference ±0.3 (a factor of 2: handover, "reading the eaters"); eaters ±0.5, the owner's choice |
| adult life of light-eaters and of eaters | the reference ×/÷ 2 (light-eaters ×/÷ 3, the owner's choice) |
| land worn down per year | the reference ×/÷ 2 |
| river cells (over the viewer's threshold) | the reference ×/÷ 1.5 |
| books | elements and energy to 1e-12 |

The references are the means over three seeds at the default steps, set again by the owner when the laws change
(decision 27). If a range is too tight for the seeds' own noise, widen it and say so in the report.

### 6.4 Equivalence tests

A speed tool must not make the world. Each one gets a test that runs a small world with and without it:

| Test | Compares | Task |
|---|---|---|
| nodes against individuals | handover 7.1, exactly as written there (`tape/tests/nodes.rs`) | F7 |
| coarse against fine | the same island on its squares and in quarters (`tape/tests/scale.rs`) | G5 |
| long step against short step | each seed at the 10-day steps against itself at 6-hour air steps and 3-day physics steps (`tape/tests/ranges.rs`) | J1 |
| sleep against no sleep | sleep on, and every node awake every step | B8, D12 |
| threads against no threads | fingerprints equal | every phase (the golden runs, by hand before the last push of a phase) |
| a speed setting ×2 and ÷2 | key numbers within their noise | D4, each setting (`wake_share`, `node_days`, ...) |

### 6.5 Speed

The benchmark `speed` (`tape/tests/golden.rs`), the CLI's per-stage times and the cost model (5.7) are recorded at the
end of each phase, on the same machine, with the planet size the time budget then allows, and the code lines of
`core` per folder. Goals: **the time budget** on the largest planet that fits; a body node at least 3 times cheaper
than at A1; a cell's physics at least 5 times cheaper than at A1 without threads.

---

## 7. The work list

Each phase ends with its watch report. Status in a few words; what each task did and its numbers are in
`docs/history.md`.

### Phases A to D (done)

- **A, prepare:** A1 profile, A2 reference runs, A3 key-number test, A4 two suspected bugs, A5 decisions in the
  handover, A6 code-size test. All built. Report `docs/reports/foundation-a.md`.
- **B, the base layers:** B1 mesh, B2 stacks, B3 node names, B5 moves and `apply`, B6 boundary nodes, B7 clock, B8
  sleep and wake, B9 keyed draws: built. B4 sparse amounts: measured, dropped. Report `docs/reports/foundation-bc.md`.
- **C, the shared maths:** C1 `spread`, C2 `downhill`, C3 the wind's lines (now `band`), C4 big-step tests: built.
- **D, the physics laws on the new base:** D1 ground heat, D2 liquid, D3 air on the mesh, D4 radiation, D5 erosion
  every step, D6 Change sleeps, D7 parts as nodes, D8 links, D9 every law through the store, D10 speed, D11 the store
  only adds, D12 wake by gain, D13 work spaces kept: built. The phase check missed its speed and size goals. Report
  `docs/reports/foundation-d.md`.

### Phase E: the life engine

| ID | What | Status |
|---|---|---|
| E1 | The bookkeeping of a body update | Built (a body update about 15% cheaper) |
| E2 | A list of active reactions per gene set, only if the reactor passes 5% of the bodies' time | Open (not needed so far) |
| E3 | Lookup tables for the rate's temperature factor, on the same condition | Open (not needed so far) |
| E4 | The pace of population nodes (`node_days`, D13) | Built for nodes without a brain |
| E5 | Bodies book through moves; one bodies' node per cell; a private `Store` | Built |
| E6 | Cheap child placement | Built, not cheaper per call |
| E7 | Members split off a node | Dropped (few after E9) |
| E8 | One clock, state apart from cache, fixed nodes by class and index, one wind solver | Built |
| E9 | Light-eaters without a brain | Built |
| E10 | Culls: shared group brains, `FORGET`, old flags, the shade sort, repeated solves, saved caches | Built |
| E11 | Young that grow up (an adult and a young node joined by a link) | Built |
| E12 | Selection inside a node by sample members | Built for nodes without a brain |

### Phase F: population nodes

| ID | What | Status |
|---|---|---|
| F7 | The nodes-against-individuals check (handover 7.1), slow test | Built; prints its failing rules (decision 25) |
| F1 | The small split and join helper | Built (with E6) |
| F2 | The node operations use F1 | Built (with E6) |
| F3 | No merge of unlike nodes in a full cell (D8) | Built |
| F5 | More age classes inside nodes, only if F7 shows they matter | Open |

### Phase J: the long physics step

| ID | What | Status |
|---|---|---|
| J1 | Liquid every 30 days; the air in one backward-Euler step every 10 days | Built; converged against 6-hour steps |
| J2 | Flow and Change keep their threads by rows on the globe | Built |
| J3 | Physics speed after J1 | Built: fewer rounds, the bodies' node in the air's step; BDF2 and a 30-day step dropped (decision 22) |

### Phase G: the globe

| ID | What | Status |
|---|---|---|
| G1 | The lat-lon grid that wraps | Built (`ew run --globe`) |
| G2 | Life on the mesh's links | Built |
| G3 | Cheap sea cells | Measured, not built (decision 20) |
| G4 | A refined region built into the mesh | Built (`ew run --region`) |
| G5 | The coarse-against-fine test | Built; river length fails (decision 31) |
| G6 | Remove old limits | Dropped (decision 28) |

### Phase K: recording and viewer

| ID | What | Status |
|---|---|---|
| K1 | The tape takes `&World` | Built |
| K2 | The tape and viewer file for the globe and a refined region | Built |
| K3 | The viewer: the globe, the region, the link layer, charts grouped and searchable | Built |
| K4 | Time per process in the tape | Built |

### Phase L: the final check

| ID | What | Done when |
|---|---|---|
| L1 | A2's reference runs on the new engine | Ranges hold; every larger difference explained |
| L2 | Speed | The time budget met on the largest planet that fits, with the cost model's numbers |
| L3 | Books | Elements and energy exact over 1,000 years, with threads and a refined region |
| L4 | Show worlds: the globe, a refined region | Each one runs, with pictures |
| L5 | The foundation watch report; then the owner decides M4 | The owner's choice is written in the handover |

## 8. What is left

The open findings of review round 2, F5 if F7 asks for it, `core` down to 8,000 code lines, a watch report for
Phases E to K, then Phase L. Work like this usually takes longer than planned: report the real days in each phase
report.

---

## 9. Risks and stop rules

| Risk | What to do |
|---|---|
| **The rebuild never ends and nothing is watched** (the version 1 trap: handover 2, lesson 4) | The island runs after every task. A phase more than 50% over its days stops, and the owner decides. |
| The new engine gives a different world | Level-3 differences are allowed only when the report explains them. A range broken without a reason stops the phase. |
| A speed tool makes the world | Its equivalence test (6.4) is part of its "done when". |
| The new base becomes one more layer, or code grows past the budget | Each task deletes the special code it replaces; simplify before adding; a new part past its allowance needs the owner's agreement. |
| Speed work aimed at the wrong half | Each speed task starts from a profile. |
| The planet that fits the time budget is too small to watch | Report it with the cost model; the owner chooses a longer budget, threads over bodies (5.6), or a change of the laws' cost. |
| New tuned numbers creep in | The tuned budget test (`core/tests/numbers.rs`) stays; speed settings go in their own table (D4). |
| Body physics changes sneak in | D6: bug fixes only, each its own branch. |

---

## 10. After the foundation

### 10.1 Waiting on purpose (D14)

| Idea | Why it waits | When it comes back |
|---|---|---|
| Focus zones (5.4) | Realism (chases, actions that take time), not speed; finer cells and minutes cost more | After M4, with the mind |
| Automatic zoom while running | A refined region built into the mesh (G4) gives detail in places with much less code | When a player must zoom anywhere, at any time |
| Patches: parts of a cell's ground with their own surface | Their first use is a burning share of a cell | With fire (M4) |
| More memories on links | A channel would repeat what erosion does to a cell's height | With the law that needs one |
| Threads over bodies (5.6) and the chunk layout | The game runs on one thread; batches run one world per core | When a planet profile shows one native thread for the bodies limits a long run |
| Snapshots of changes only | In 10 years almost everything changes | If snapshots are still too big |

### 10.2 What M4 still needs

The foundation gives the machinery. M4 then adds the content (handover 13):

- **The planet:** real latitudes, continents and basins on the globe.
- **Air in 2 or 3 layers** with momentum and the planet's turning; clouds that travel with the wind.
- **The sea in 2 layers** with a salt substance (owner's choice), currents from wind, heat and salt, thin ice on top.
- **Snow and ice that flow**; a slow carbon source like volcanoes; slow uplift.
- **A burning share of a cell** that spreads along links, with sparks from strong storms booked like the sun (handover
  13.2, "Fire and disease").
- **The mind** (handover 13.1, option 2): memory of places, a short look-ahead, learning in the hidden layer, used in
  zones.
- **Long watch runs** on the planet, 1,000 years and more, several seeds, against null runs (handover 18.3).

---

## 11. Words used in this plan

| Word | Meaning |
|---|---|
| mesh | the set of cells and the links between them: the shape of the world |
| level | a coarser layer of the mesh (the air, the sea) or a refined region: a cell of one level holds 4 cells of the next |
| stack | the nodes above and below a cell: pore liquid, the deep store |
| patch | a part of a cell's ground with its own soil, liquid, heat and cover |
| zone | a small area taken to the finest level, with its own fast clock (after the foundation) |
| refined region | a region of finer cells built into the mesh at the start or at a god action |
| time budget | how long a run may take on the owner's machine; it sets the planet's size |
| cost model | the cost of a cell and of a body node per simulated year, times their numbers |
| move, link flux | amounts and heat from one node to another; what crosses each link of a field at once |
| boundary node | the sun, space or outside: where energy and matter enter or leave the world |
| implicit step | a way to do a time step that stays stable for any step length |
| chunk | a block of cells that can run on its own thread (after the foundation) |
| fingerprint | one number made from the whole saved state (`tape::fingerprint`); equal fingerprints mean the same history |
| equivalence test | a test that a speed tool gives the same world as running without it |

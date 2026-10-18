# The foundation plan: a stronger base before M4

**Written:** 17 October 2026, from a review of the whole project with the owner.
**Starts from:** branch `claude/sanitise-m0-m3` at `43efbc5` (the M3 clean-up). `main` was still at `154554a`.
**Status:** approved by the owner on 17 October 2026. All decisions are taken (Section 3). Where this file and
`docs/handover.md` differ, the handover wins until it is updated.

This file is for the agents who will do the work, and for the owner. It is written in simple English (handover 0,
"Working with the owner"). When a technical word is needed, it is explained the first time.

---

## The main rule: the foundation REPLACES the code, it is not one more patch

**Read this before anything else. It is the owner's first wish for this work.**

During M0 to M3 every review added one more special piece of code: one for how a body loses heat, one for how it
breathes, one for rain on a leaf, one for roots, one for the chemistry's own timer, one for merging groups, one for worn
lanes. Each piece was right, but together they are a pile of small rules. **The foundation exists to replace that pile
with a few general pieces.** It must not sit on top of the old code as one more layer.

The idea in one sentence: **a body, a patch, a wall, an air layer are all just nodes; everything that passes between
them goes along links; a gene or a material only says how easily a link passes.** Then Flow moves heat, gas, liquid and
light between a body and its cell exactly as it moves them between two cells. Many separate pieces of code each become
**one line**: the conductance of a link.

### What today's special code becomes

| Today: a special piece of code | Where it is now | In the foundation: one general thing |
|---|---|---|
| How a body trades heat, gas, liquid and salts with its cell (coat, skin, still air, radiation, pores, evaporation, soaking, roots, rain caught) | `body/exchange.rs`, `body/physics.rs` | **The formulas stay: they are the physics** (each gives a conductance or a rate from the `HEAT`, `GAS` and `LIQ` genes, the size, the wind, the wetness). What goes is the way they write the world: each trade is a `Move` between the body and its cell's nodes, booked by `apply` at the body's update (E5). |
| Light caught and stored, and the pools that hold light and heat for bodies between their updates | `survey.rs` (`hold_heat`), `World::light_pool`, `heat_pool`, `light_keep`, `heat_keep`, `PerSlot` | **One body node per cell** for all its bodies (the collapsed form of handover 5.2): Flow passes light and heat into it at every step, as into any node (work in the number of cells, not of bodies); each body settles with it at its own update by moves. The pools become a real node of the store with exact books (E5). |
| The walls of a shelter keep a body warm | `bond/` (enclosure conductance, `walls_around`) | Links that pass heat badly. Nothing else. |
| The chemistry's own lazy timer: which nodes react now | `change/screen.rs` | The clock's one sleep and wake. |
| Flow's own timers (`pending`, `pending_liq`, `pending_exc` and the yearly and monthly turns) | `flow/mod.rs`, `flow::Scratch` | The clock's one list of processes (`schedule.rs`). The bodies' due ring stays: it is the clock's queue for bodies and costs only the bodies due. |
| Merging, splitting and opening population nodes | `nodes.rs` (`merge`, `join`, `take_in`, `split`, `one`, `open`) | One small helper that splits and joins amounts and heat by shares; what differs by class (genes, born rows) stays with the class (F1). |
| Ground cleared along edges | `lanes.rs`, `World::pass` | A field of the links (`store::Links`, built). A new field comes only with the law that needs it. |
| Heat moving sideways between cells once a year | `flow/creep.rs` | `spread` on links (built). |
| Pore liquid between cells (Darcy's law) | `flow/liquid.rs` (`pore_flow`), `flow/drain.rs` | `spread` on links; the deep store is a node in the cell's stack. |
| The surface liquid's sorted sweep, and erosion sampled every 7 months | `flow/liquid.rs`, `WORK_MONTHS` | The `downhill` tree, every step. |
| Taking heat from a node with a guard in each place | `ground.rs` (`draw`, `take_paid`, `take_heat`) | A law never asks a node for more than it holds; `apply` only adds (Idea 2). |
| Caches the laws keep between steps, saved with the state | `flow/scratch.rs`, `Life` (`sum`, `env`, `seen`), `Body::k` | A small saved state, named as state; a cache that is never saved and is rebuilt on load (E8). |

This table is the plan's promise. Each row is a task (Section 7). When the task is done, **the special code on the left
is deleted in the same change.**

### How to work by the main rule

1. **Before you write code, find the special code your task replaces.** Delete it in the same branch. A task that adds
   the general piece and leaves the old one is not done.
2. **A law says *what*, the base does *how*.** A law gives a conductance, a capacity, a rate, a share. The loops, the
   time steps, the solving and the books live in `space/`, `store/`, `clock/` and `solve/`, once.
3. **No special path by node class inside a law.** If you write `if` on whether a node is a body, a patch, a part or a
   cell, stop. Ask which link or conductance says the same thing. Only where the physics really differs is a class
   test allowed, with a one-line comment saying why.
4. **No new pool, cache or catch-up for timing.** The clock and the moves handle time.
5. **Two functions of the same shape become one.** A new function with a single caller, shaped like an existing
   one, is merged into it.
6. **If the general way does not fit, do not add a special one.** Stop and tell the owner, with the case.
7. **Every commit message ends with its line count:** `Lines: +X -Y (core code N)`.

### The line budget (decision D2)

- **`core` may hold at most 8,000 lines of code, not counting comments and blank lines** (handover 14.3). A comment is a
  line that starts with `//` after its spaces. Comments help the next agent: do not delete them to save lines.
- On `43efbc5` `core` holds 9,962 lines: **7,236 of code**, 2,142 of comments, 584 blank.
- The new base gets a fixed allowance of code lines. It must not grow past it without the owner's agreement:

| New part | Allowance (code lines) |
|---|---|
| `space/` (meshes, levels, stacks, the globe, a refined region) | 500 |
| `store/` (nodes, amounts, moves, links, the split and join helper) | 400 |
| `clock/` (processes, sleep and wake) | 250 |
| `solve/` (`spread`, `downhill`, `lines`) | 350 |
| **Total** | **1,500** |

- Most of the old laws' "moving code" turned out to be physics (rates, conductances, heads), so the laws shrink less
  than the base grows. The savings come from keeping the base small (the store only adds, Idea 2) and from deleting
  the special code the main rule's table names.
- A test (task A6) counts the code lines and fails above the ceiling: **9,000 until the end of the foundation**,
  **8,000 at its end (Phase L)**. The 8,000 is never a reason to build a task that costs speed or honesty (it is why
  a body is not a node in Flow's steps, Phase E).
- A phase may not end with more code than it started with, unless its report gives the reason.

### The time budget (the owner's rule, decision D12)

**The foundation aims at time, not at a size. A run between phases never waits hours.**

| Run | Budget, on the owner's machine (4 cores) |
|---|---|
| The normal test suite | under 30 seconds |
| The key-number test (3 seeds × 30 years, 64 × 64) | under 10 minutes |
| **A planet run of 1,000 years** | **under 15 minutes** (about 0.9 s per simulated year; threads for Flow and Change) |

- **The planet's size is not fixed.** It is the largest number of cells (on the globe of Phase G) whose cost fits the
  budget, by the cost model (5.7). It is measured again at the end of each phase. Faster code buys a bigger planet.
- Speed comes first among the goals of the foundation: a faster world means more runs watched and more work done in
  the same time. A task that costs speed says how much, and the phase report gives the cost per cell and per body
  node.
- Speed never comes from removing a law or from a rule named after an outcome (handover 12.6). It comes from doing
  less work for the same physics: longer steps where the physics allows them, fewer updates where nothing changes,
  cheaper bookkeeping, cheap cells where little happens.

---

## 0. How to use this plan (agents, read this first)

1. Read, in this order: **the main rule above**, `docs/handover.md` (Sections 0 to 3, 5, 7, 8, 12, 14), the rest of
   this file, then `docs/laws.md`. Every task's "done when" includes deleting the special code it replaces.
2. Every task has an **ID** (for example `B5`), a **check level** (Section 6.1), the files it touches and a
   "done when". Do one task per branch: `claude/foundation-<id>` (for example `claude/foundation-b5`).
3. **The old island must run at the end of every task.** `cargo test --release --workspace` passes, and the golden
   runs (`tape/tests/golden.rs`) either keep their fingerprints (check level 1) or are updated on purpose with one
   line in the commit message that says why (check levels 2 and 3).
4. **Each phase ends with a watch report**, `docs/reports/foundation-<letter>.md`, in simple English, with pictures
   and the numbers of Section 6 (handover 12.4). The owner reads it before the next phase starts.
5. When code moves, update `docs/laws.md` in the same change, so it keeps naming the right file and function.
6. A new free number gets a row in `docs/numbers.md`. The count of tuned numbers may not grow (handover 14.3). Speed
   settings follow decision D4 (Section 3).
7. **The word ban still holds** in `core/` and `worldgen/` (handover 14.4, test `cli/tests/word_ban.rs`). Names used in
   this file that are banned there: river, sea, lake, trail, route, road, fire, water, tool, house. In code, use
   neutral names: `downhill`, `liquid`, `lane`, `channel`, `zone`, `patch`, `burn`.
8. When a task cannot reach its "done when", stop and tell the owner with numbers. Never call a partial result done.

---

## 1. Why this plan

A review of M0 to M3 found that many limits come from the **structure** of the code, not from the laws. The laws are
good. The base they run on holds them back.

| Problem found | Where it comes from | Fixed in |
|---|---|---|
| Too slow: about 15 s per year with life on 128 × 128, against a goal of 0.6 s; it gets slower as life grows | Each body update costs about 36 µs; bodies are about 80% of the time | Phases E, J, G3 |
| Heat moves sideways between cells once a year, so a burning front cannot spread | `flow/creep.rs` runs lateral heat yearly | C1, D1 |
| Erosion is a 15-day sample every 7 months, multiplied | `WORK_MONTHS` in `flow/mod.rs` | D5 |
| Water crosses the map in one sweep; it is sorted again every time | `flow/liquid.rs` | C2, D2 |
| Links between cells hold almost nothing; only cleared lanes remember use | `World::pass`, `lanes.rs` | D8; more after the foundation (10) |
| No levels of detail: the plan's "rough far away, detailed where it matters" (handover 5.2, 7.2) is not built | One flat grid | G3, G4 |
| Population nodes change biology: selection inside a node runs at about 3%; unlike nodes are forced to merge in full cells | `nodes.rs` | F2 to F5 |
| One decision per 5 days or more, one cell per move: chases, building and tool use are almost impossible | Bodies live only at the world's coarse time | after the foundation (zones, 10) |
| A flat grid cannot become a planet | `grid.rs` is rows and columns | B1, G1 |
| Every law writes straight into the arrays; heat bugs came from this (19 unguarded places at M3) | No single way to change the world | B5, B6 |
| Each speed trick has its own code: the chemistry's lazy screen, the bodies' ring, node merges | No shared clock or split/join | B7, B8, F1 |

---

## 2. What stays the same (must)

- **The five laws and their equations.** This plan changes *where* the laws read and write, not *what* they say.
- **Chemistry 0** (`worldgen/src/chem0.toml`).
- **No Earth words** in `core/` and `worldgen/`.
- **Exact books.** Every element and the energy are counted to rounding. This gets *stronger* (Idea 2).
- **Same seed, same build, same machine: same history** (handover 12.5). With and without threads.
- **The body physics of M3** is moved, not rewritten. **No new body mechanics** during this plan (decision D6), only
  bug fixes.
- **Handover 8.2:** the step is a pure function; random numbers are keyed; fixed order; no IO in `core`; `core` builds
  for WebAssembly.

---

## 3. The owner's decisions

All decided on 17 October 2026.

| # | Decision | Status |
|---|---|---|
| D1 | The M3 check point choice (handover 13.1) | **Decided:** a foundation phase (this plan), then M4. Written in handover 13.1. |
| D2 | The `core` code budget (handover 14.3) | **Decided:** at most 8,000 lines of code in `core`, not counting comments and blank lines, at the end of the foundation (9,000 until then). The foundation replaces the special code, it does not sit on top of it (see "The main rule" at the top). The new base has fixed allowances; a test keeps the ceiling (A6). |
| D3 | The cell shape for the planet | **Decided:** a latitude-longitude grid of squares that wraps east to west (5.5). It keeps today's square laws, the brain's 8 directions, the exact split of a cell into 4, and the viewer (the grid is the map). |
| D4 | Speed settings (when a zone opens, when a node sleeps, when a patch splits, how many nodes a cell keeps) | **Decided:** a new table "speed settings" in `docs/numbers.md`, outside the tuned budget. Each must pass an equivalence test (6.4): changing it 2× must not change the key numbers beyond their noise. |
| D5 | Old snapshots | **Decided:** no backwards compatibility; snapshots carry a format version (`tape/src/snapshot.rs`). |
| D6 | Body physics during this plan | **Decided:** frozen, bug fixes only. Moving existing body physics onto the new base (same physics, new code) is part of the plan, not new physics. |
| D7 | How the work merges | **Decided:** phase by phase into `main`, after the owner reads the phase report. No long-lived branch. |
| D8 | The safeguard that merges unlike nodes in a full cell (`nodes::crowd`) | **Decided:** removed once nodes are cheap (F3). If a cap is still needed for speed, it becomes a speed setting (D4). |
| D9 | Who keeps amounts at or above zero | **Decided:** the law. A law never asks a node for more than it holds; `store::apply` only adds, clamps rounding and counts it (Idea 2). |
| D10 | The check level of speed work | **Decided:** level 2 (the same to rounding) is the normal level for speed work: it may change the order of sums, and it re-pins the golden runs in the same commit. Level 1 stays for pure clean-ups. |
| D11 | Bodies and Flow | **Decided:** a body is not a node in Flow's steps (that would make every Flow step touch every body, 120 times a year, through stiff heat links: a 1 kg body against a ground of gigajoules). Bodies book their trades as moves at their own updates, and one body node per cell holds what Flow brings them between updates (E5). |
| D12 | The size of the planet | **Decided:** set by the time budget (top of this file), not fixed. |
| D13 | How often a population node updates | **Decided:** at most about 10 times a year (the speed setting `node_days`; handover 7.3 suggests one month for population nodes). An individual keeps its `INTERVAL` gene. Checked by the speed-setting test (× 2 and ÷ 2) and by F7. |
| D14 | What waits until after the foundation | **Decided:** focus zones, automatic zoom while running, patches, more memories on links, threads over bodies, the chunk layout, a query layer and snapshots of changes only (Section 10). Each adds realism or speed later, but costs speed or code before M4 needs it. |

---

## 4. Where we start: the code today

On `claude/sanitise-m0-m3`. About 9,000 lines in `core`, 1,500 in `worldgen`, 1,500 in `tape`, 600 in `cli`,
600 in `web`.

| Part | Files | What it is |
|---|---|---|
| The world's state | `core/src/world.rs` | `World`: `ground`, `bed`, `e_ground` (per cell, `[k * n + i]`), `air`, `e_air` (per air node), `deep`, `e_deep`, `rain`, `liquid_flux`, `canopy`, `light`, `pass`, `light_pool`, `heat_pool`, `life`, `parts`, `boundary`, `e_boundary`, `counts`, `scratch` |
| Space | `core/src/grid.rs` | `Grid`: `w × h` squares of `cell_m` (2,000 m); 8 neighbours (`DIRS`), 4 forward edges (`FORWARD`); air nodes on a coarser grid (`air_scale` 4); latitude from the row |
| The month | `core/src/schedule.rs` | `step` = 30 days; every `phys_days` (3): `survey`, `flow::step`, `change::step`; day 0: `bond::step`; every day: `survey::day` (bodies due) |
| Flow | `core/src/flow/` | `land`, `liquid`, `drain`, `creep`, `scratch`; `air/{radiation, exchange, wind}` |
| Change | `core/src/change/` | `kernel` (the `Reactor`), `screen` (which ground nodes react now), `balance`, `gas` |
| Bond | `core/src/bond/` | parts, structures, loads |
| Bodies | `core/src/body/`, `life.rs`, `survey.rs`, `nodes.rs`, `lineage.rs` | `Body` is a struct per body with its own `Vec`s; `Life` is the registry with a ring of updates due (`RING` 384 days); `nodes.rs` merges and splits population nodes |
| Copy, Choose | `core/src/copy/`, `core/src/choose/`, `brain.rs` | children, two parents; perception, actions, learning |
| Lanes | `core/src/lanes.rs` | ground cleared along each forward edge (`World::pass`) |
| Books | `core/src/check.rs` | totals, drift, `broken` |
| Threads | `core/src/par.rs` | rows in parallel, applied in fixed order (flow and change only) |
| Tape | `tape/src/` | `rec/` (series, months, viewer file), `snapshot.rs` (format version, `fingerprint`) |
| Tests | `core/tests/{flow, change, bond, copy, choose, body, chemistry0, numbers}.rs`, `tape/tests/{tape, golden}.rs`, `worldgen/tests/*` | Golden runs pin 3 fingerprints; slow tests are `#[ignore]` and run in CI |

Already done on that branch (they were on the review's list): golden runs, a century run that keeps its books, a
speed benchmark (`tape/tests/golden.rs`, `speed`), a clear error for bad time steps (`cli/src/args.rs`,
`check_steps`), tests in one file per law, one fingerprint, one random-number purpose per draw, the docs split.

---

## 5. The target architecture

### 5.1 The picture

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

Today a text test (`cli/tests/contract.rs`) checks that no law outside `store/` writes the store's arrays. With E5 the
compiler checks it instead: the arrays become private fields of a `Store`, written only by `apply`, and the text test
goes.

### 5.3 The ten ideas

**Idea 1. Everything is a node; nodes live inside nodes.**
A node is anything that holds amounts and heat: the ground of a cell, a soil layer, an air layer, a deep store, the
bodies' node of a cell, a body (one member or a population), a part. Each node has a **class** (how it is stored:
`Ground`, `Air`, `Deep`, `Body`, `Part`, ...; a storage word, not an Earth word). A fixed node (ground, air, deep, the
boundary nodes) is named by its class and index; only what comes and goes (parts, bodies) has a slot and a generation
(E8). This is handover 5.2 ("graph inside a graph"), kept as simple as the data allows: "everything is a node" means
one way to book amounts and heat, not one table for every kind of thing.

**Idea 2. One way to change the world: moves and link fluxes.**
A law never writes an amount or a heat. It hands the store one of:
- a **move**: "from node A to node B, this many mol of substance k, and this heat";
- a **reaction**: "in node A, these amounts change, and this heat" (the books check that it keeps elements);
- a **link flux**: for a whole field at once, one number per link and substance (what crosses it). Bulk solvers give
  this; it keeps the books by its form, because what leaves one end enters the other.

The store applies them in a fixed order. **A law never asks a node for more than it holds**: `spread` keeps amounts
positive by its implicit step, `downhill` by its pass, the reactor by integrating to the scarcest input, and a law that
can still overdraw limits its own outgoing links, node by node (never a whole field: one dry cell must not slow the
liquid everywhere). So `apply` only adds; it sets what rounding leaves just below zero to zero and counts it (handover
12.1). A real overdraw breaks the books, and the books check finds it. **Sun, space and outside are three boundary nodes**: sunlight is a
move from the sun node, radiation a move to the space node, a god action a move from the outside node. The `Ledger`
becomes the totals of these three nodes. The books then need one check for every law.

Suggested shapes (agents may change them, and say so in the phase report):

```rust
pub struct NodeId { slot: u32, gen: u32 }
pub struct Move { from: NodeId, to: NodeId, k: u16, mol: f64, heat: f64 }
pub struct Reaction { at: NodeId, dmol: SmallVec<[(u16, f64); 8]>, heat: f64 }
pub struct LinkFlux<'a> { k: u16, per_link: &'a [f64], heat: &'a [f64] }   // + = from a to b
```

**Idea 3. One small split and join.**
Population nodes split and merge, and a refined region (G4) splits cells when the mesh is rebuilt. What is the same in
every case is small: **each amount and the heat are split by shares, and the last child gets the remainder** (parent
minus the others), so the sum is exact to rounding; heat is split so every child has the parent's temperature unless a
share says otherwise; a join is a sum. That is one helper of a few tens of lines (F1). What differs by class stays with
the class: a body's genes and spread join as the member-weighted mean with the pooled spread (`nodes::take_in`), its
born rows as E6 decides; a cell's links, stack and air parent come from the rebuilt mesh. No framework knows every
class.

**Idea 4. Space is any mesh, with levels and stacks.**
The laws see only:
- cells, with area, centre (and latitude, longitude), height;
- links, with the two cells, the distance between centres, and the **width** (the length of the shared border);
- each cell's parent and children (levels: the air today, later the sea, G3);
- each cell's stack: the nodes above and below (air layers, soil layers, deep store).

The **width** keeps today's square results: a diagonal link has width 0 (squares touch only at a corner), so heat,
creep and pore liquid cross only real borders, while the downhill route of surface liquid may still use all 8. The air
mesh is a coarser level of the same mesh: `Grid::air_of` becomes "the parent at the air level".

Levels serve fixed coarse layers (the air, then the sea). Detail in places is a **refined region built into the mesh**
at the start, or rebuilt at a god action (G4); cells do not split and join by themselves while the world runs (D14).

Suggested storage: compressed lists (one array of all links, and for each cell the start of its links in it), built
once per mesh change.

**Idea 5. Links are real.**
Each link holds its geometry and its fields. Today one field, the ground cleared by movers (`store::Links`), does what
lanes need. This is handover 5.1: "a route is simply an edge whose conductance became high because it was used a lot".
A new field (a channel worn by liquid, an ease for signals) comes only with the law that needs it (Section 10).

**Idea 6. Every process has its own clock.**
- Time is a whole number of **seconds** (`u64`). Days and months are derived (the tape still writes months).
- Each process (a law, or a part of one, at a level) has a period, for example: radiation and heat 3 days, liquid
  15 days, bodies by their `INTERVAL` gene (a population node at most about 10 times a year, D13). A fast process runs
  several times inside a slow one. The order inside one time is fixed and written once, in `schedule.rs`: every part
  of Flow is a process of that one list, with no timer of its own (E8).
- **One sleep and wake** for all nodes. A calm node sleeps until an alarm time, or until one of its amounts has grown
  by more than a share of what it held when it fell asleep (`Config::wake_share`, a speed setting). The look is a cheap
  scan at the node's own law's step, not part of `apply`, so the store does no work for the sleep, and a node wakes by
  whatever brought the gain (Flow, bodies, parts). This replaces
  `change/screen.rs`. The bodies keep their due ring (`life.rs`, `RING`): it is the clock's queue for bodies, and it
  costs only the bodies due, where a look over all alarms would cost every body.

**Idea 7. Shared maths that stays stable with big steps.**
`core/src/solve/`:
- `spread`: an implicit step (one that stays stable for any step length) for anything that moves from high to low
  along links with a conductance and fills nodes with a capacity: heat, pore liquid, dissolved salts, gases, the
  pressure evening of the wind. It returns **link fluxes**, so the books are exact even when the solve is not.
  Suggested: conjugate gradient (a standard iterative solver) with a fixed number of iterations and a fixed order.
- `downhill`: the order of cells from high head to low, as a tree, kept up to date when heights change; surface
  liquid and what it carries move down it in one pass.
- `lines`: the tridiagonal solve of the wind along the rows and columns of squares (`flow/air/wind.rs`), in its cyclic form on a row that wraps (G1). It is the wind's only solver.
- The reaction step stays in `change/kernel.rs`; it already integrates the rate law over the step.

**Idea 8. Columns and chunks.**
- **Columns:** each quantity of a node class in one array (as the ground already is). Bodies too, where threads need
  them (Idea 9, E7).
- **Chunks:** blocks of about 64 cells. A chunk can sleep and run on a thread. Threads run chunks in colour passes
  (5.6). Same history with and without threads. **Not built in the foundation** (D14).
- **The chunk layout of amounts:** today an amount lies at `[k * n + i]`. Flow likes that (one substance over all
  cells), but Change and the bodies read one node at a time, so each node read touches 26 cache lines for 208 bytes.
  The layout `[chunk][k][cell in chunk]` keeps a chunk's amounts in one block (about 13 KB for 64 cells and 26
  substances) that fits in the first cache: a law that goes substance by substance (Flow) or node by node (Change,
  bodies) works in the cache, and a thread owns one block. Its chunks are the chunks of the thread design (5.6), so
  it waits with it (D14). Before any chunk layout, measure the plain node-major layout `[cell][k]` (a node's amounts
  side by side), which is simpler and helps Change and the bodies most.
- **No sparse amounts:** at 26 substances a dense column costs little, and a sparse one adds code.

**Idea 9. A fast life engine.**
- **The bookkeeping first.** The profile of a living year (Phase A, and an outside review at the end of Phase D) puts
  most of a body update in bookkeeping, not in the brain or the reactor: re-summing a group's brain rows when a count
  changes, a temperature solve run again and again, the survey recomputing every body's height. These go first (E1).
- Each lineage or gene set gets its **list of active reactions** once, when its genes change, not at every update.
- Lookup tables for the slow maths: the rate's temperature factor, vapour pressure.
- **Fewer updates where nothing needs them**: a population node of millions of members does not need an update every
  20 days. It updates at most about 10 times a year (D13); its update integrates its physics, growth and children over
  the span, as a body's update already does for intervals up to 360 days.
- **Cheap born rows and a cheap child join** (E6): today each body carries its own born brain rows (about 6 KB, almost
  never shared), and placing a child is about 25% of the time of body updates.
- The brain's own maths (`think`) is a small share of the time: it stays as it is.
- **Selection inside population nodes at full speed** by sample members (E12): a node runs its update also for two
  sample members, one step of spread above and below its mean on one gene (a different gene each update, by the keyed
  random numbers), and moves its mean by the spread times the difference in what they gain. This is the breeder's
  equation (the change of a mean is the spread times the strength of selection), not a new law. It must pass the
  nodes-against-individuals check (handover 7.1).

**Idea 10. Readers read `&World`.**
The tape, the viewer, the CLI report and the tests take `&World`, which Rust keeps read only. The events and counters
they use are taken through two small methods (`World::drain_events`, `World::take_counters`). No query layer is built.

### 5.4 Focus zones (after the foundation, D14)

Zones are not built in the foundation: they add realism (chases, actions that take time), not speed, and M4's
watch-for does not need them. They are the target for after M4, with the mind (handover 13.1, option 2). The design:

A **zone** is a set of cells taken down to a fine level (about 30 m) with its own fast clock (minutes to an hour for
bodies). Inside a zone: population nodes give out single members, and take them back when the zone closes; a body has
a position inside its cell and a speed, its top speed from the power it can pay; the sun moves (day and night); actions
take time; catching and escaping come from speed and distance. A zone opens and closes on physical facts only. What
crosses its border is booked as moves on the `Outside` node. **The coarse rule outside zones must
be the average of the fine rule inside**: a zone-against-no-zone test will measure that.

### 5.5 The shape of cells (decision D3)

| | A lat-lon grid of squares that wraps east to west | Squares on a cube-shaped ball |
|---|---|---|
| Today's code | carries over: `Square` with a wrap flag; the area and width of each row from its latitude | a new mesh builder, seams between 6 faces |
| The brain's 8 directions | the same everywhere | turned at each face's edge; 7 neighbours at the 8 corners |
| Split into 4 | exact | exact |
| Viewer | the grid is the map | the faces must be unfolded |
| Weak point | cells near the poles are narrow (3 times longer than wide above about 70°); no link crosses the pole at first | some distortion at the 8 corners |

**Decided: the lat-lon grid.** The implicit solvers do not mind narrow cells, and life near the poles is rare. If the
narrow cells cost too much, rows near the poles can be merged later.

### 5.6 Threads over bodies (after the foundation, D14)

Not built in the foundation. The game runs as WebAssembly in one browser thread, so for the game all speed must come
from one thread. For batches of experiments (seeds, checks, the M7 world search), one world per core gives the same
throughput with no code. Threads over bodies come when a profile of the planet shows that one native thread for the
bodies limits a long run. Flow and Change keep their threads by rows (`par.rs`). The design, for then:

Bodies are most of a living year, and one thread can make a body update about 2.5 to 3 times faster (E1 to E3).
The rest of the plan's 10 times (6.5) must come from threads. A body update is not local to its cell: a body sees up to
4 cells away in 8 directions (`choose/perceive.rs`, the sight gene), takes from bodies and the ground of its cell, steps
to a neighbour cell (`choose/go.rs`), wears the bodies there and clears a lane, places children, moves the shared rows
of its brain group, and opens new slots, groups and lineages. So:

1. **Chunks and colours.** The cells are cut into square chunks of side `C` cells, with `C` at least the longest sight
   plus one (today 5; 8 gives the 64 cells of Idea 8). The chunks get 4 colours in a 2 × 2 pattern, so two chunks of
   one colour are always at least one whole chunk apart. A day's bodies run in 4 passes, one colour each. Inside a
   pass, the chunks run on threads at the same time; inside a chunk, its bodies run one by one in their fixed order.
2. **Reads.** A body reads the world as it stands, within its sight. No other chunk of its colour writes there during
   the pass (they are a whole chunk away and write only inside themselves), so what it reads does not depend on the
   threads.
3. **Writes inside the chunk** (its own cell's ground, air, bodies and parts, a neighbour cell inside the same chunk)
   are made at once, as today.
4. **Writes across the chunk's border become moves**: a step into a cell of another chunk, a child placed there, the
   wear of the bodies there, a lane on a border link, what is taken from or given to a body there. Each chunk keeps a
   list of them; after the pass the lists are applied in chunk order, each in its own order. A step that the target
   cell no longer allows when its move is applied does not happen, and the body stays (its cost is refunded by the same
   move). Effects that the body sees at once today (a cell it stepped into) it sees at its next update instead.
5. **Shared books** (new slots, groups and lineages, the group sums of born rows and the counts of members, the learned
   rows a decision moves, the counters and events of the tape) are written to per-chunk lists and merged after the
   pass in chunk order. New ids are given out in that merge, so they do not depend on the threads.
6. **Random numbers** are keyed by the body and the time (handover 8.2), never drawn from a shared stream.
7. **The same history with and without threads**: a run without threads runs the same passes in the same order, one
   chunk after another. Moving to this order changes the world once (check level 3, the key-number test); after that,
   threads against no threads is check level 1 (fingerprints equal).
8. **Population nodes** live in one cell, so a node is never split between two chunks. A brain group may span chunks:
   its rows are read as they stood at the start of the pass, and what its members learn in the pass is merged after
   it (rule 5).
9. **Later meshes.** On any mesh, a chunk is a set of cells under one parent at a coarser level (Idea 4); the colours
   come from a colouring of the chunks' neighbour graph. The rule stays: two chunks of one colour are never within a
   sight of each other.

### 5.7 The cost model

The time of a run is the cost of one cell and of one body node for one simulated year, times their numbers. This
sets the planet's size under the time budget (D12).

| | Today (64 × 64, seed 1, year 20, main after E1) |
|---|---|
| One cell, Flow and Change, per simulated year | about 290 µs |
| One body node, per simulated year | about 650 µs (about 18 updates of about 36 µs) |
| A planet of 512 × 256 cells (handover 5.4; about 134,000 body nodes) | about 130 s per year |
| The budget | 0.9 s per year |

What closes the gap, with rough estimates:

| Lever | Task | Estimate |
|---|---|---|
| A longer physics step (15 to 30 days, implicit column) | J1 | about 5 times on the physics |
| Cheap sea cells (the sea on a coarser level, as the air) | G3 | about 2 to 3 times on a planet's physics (70% of it is sea) |
| Light-eaters without a brain, and a population node updates at most about 10 times a year | E9, E4 | about 2.5 times on the bodies (prototype: a plant node-year 676 to 263 µs) |
| The culls (shade, the temperature solve, groups) and cheap child placement | E10, E6 | about 1.3 to 1.5 times on the bodies |
| Threads for Flow and Change (native) | as today | up to about 3.5 times on the physics |

With all of these, a cell costs about 20 µs a year and a body node about 250 µs: the bodies set the size. At about
one body node per cell, the budget holds a planet of about 3,000 to 5,000 cells (cells of about 300 to 400 km); every
later gain on the bodies grows it. Threads over bodies (5.6) would give up to about 3.5 times more on a native run:
they are the first lever after the foundation if the planet is too small to watch. Each phase report measures these
numbers again and gives the planet size they allow.

---

## 6. How we check (the safety net)

### 6.1 Check levels

Every task says which level it must meet.

| Level | Meaning | How it is checked |
|---|---|---|
| **1: the same** | Bit for bit the same history | The golden fingerprints do not change |
| **2: the same to rounding** | Only the order of sums changed: the normal level for speed work (D10) | After 2 years of the golden worlds, every series column within 1e-9 of the old value (relative), books exact; where life's choices grow a rounding into a different history, the key numbers of level 3 instead. The golden runs are re-pinned in the same commit |
| **3: the same world** | The physics or the speed tool changed on purpose | The key numbers stay inside their ranges (6.3); every change larger than the seed noise is explained in the phase report |

### 6.2 Reference runs (task A2)

Made once on today's code, saved under `docs/reports/foundation-a/` (series, `physics.json`, pictures):
- seeds 1, 2, 3 × 60 years × 64 × 64, normal and `--roster`;
- seeds 1, 5, 7 × 300 years × 128 × 128, normal (with `--features threads`).

### 6.3 Key numbers and their ranges (task A3)

One slow test (`tape/tests/ranges.rs`, `#[ignore]`, run in CI) on 3 seeds × 30 years × 64 × 64:

| Number | Range |
|---|---|
| ground temperature on land, mean of years 21 to 30 | the reference ±1 K |
| seasonal swing on land | the reference ±1.5 K |
| rain on land | the reference ×/÷ 1.3 |
| carbon gas | the reference ±40 ppm |
| plant kg, animal kg (log10 at year 30) | the reference ±0.3 (a factor of 2: handover, "reading the eaters"); animals ±0.5, the owner's choice: the reference animals shared their group's brain, E10 |
| adult life of plants and of eaters | the reference ×/÷ 2 (plants ×/÷ 3, the owner's choice: the reference plants had a brain, E9) |
| land worn down per year | the reference ×/÷ 2 |
| river cells (over the viewer's threshold) | the reference ×/÷ 1.5 |
| books | elements and energy to 1e-12 |

The reference is the mean over the three seeds of A2's 64 × 64 normal runs. If a range is too tight for the seeds'
own noise, widen it and say so in the report.

### 6.4 Equivalence tests

A speed tool must not make the world. Each one gets a test that runs a small world with and without it:

| Test | Compares | Task |
|---|---|---|
| nodes against individuals | handover 7.1, exactly as written there | F7 |
| coarse against fine | the same island at one level and one level finer everywhere | G5 |
| long step against short step | the key numbers at the long physics step and at 3 days | J1 |
| sleep against no sleep | sleep on, and every node awake every step | B8, D12 |
| threads against no threads | fingerprints equal | every phase (the golden runs, by hand before the last push of a phase) |
| a speed setting ×2 and ÷2 | key numbers within their noise | D4, each setting (`wake_share`, `node_days`, ...) |

### 6.5 Speed

The benchmark `speed` (`tape/tests/golden.rs`), the CLI's per-stage times and the cost model (5.7: the cost of a cell
and of a body node per simulated year) are recorded at the end of each phase, on the same machine, in the phase
report, with the planet size the time budget then allows. So are the code lines of `core` (A6), per folder, with
what the phase added and deleted. Goals, against A1:
- **the time budget** (top of this file), on the largest planet that fits;
- by the end of Phase E: a body node at least **3 times** cheaper per simulated year (E1, E4, E9, E10, E6);
- by the end of Phase J: a cell's physics at least **5 times** cheaper per simulated year without threads (J1),
  more on the planet with cheap sea cells (G3).

---

## 7. The work list

Days are rough guesses for one agent. Each phase ends with its watch report.

### Phase A: prepare (about 10 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| A1 | Measure where the time goes: per stage, and inside one body update (perception, brain, reactions, exchange, heat, growth, survey, merges) | a profiler run (`perf`) on a 64 × 64 run with life | — | A table in `foundation-a.md` |
| A2 | The reference runs (6.2) | `docs/reports/foundation-a/` | — | Saved, with pictures |
| A3 | The key-number test (6.3) | `tape/tests/ranges.rs` | — | It passes on today's code and runs in CI |
| A4 | Check two possible bugs: (a) plant members die "taken" in years 3 to 5 of a normal run, before any animal exists (light-eaters may take from each other); (b) the rain picture shows stripes on the 4 × 4 air grid | — | — | Each is explained or fixed (a fix is its own branch) |
| A5 | The owner's decisions of Section 3 written in the handover | `docs/handover.md` 13.1 and 14.3 | — | Done on 17 October 2026 |
| A6 | **The code-size test:** counts the code lines of `core/src` (not blank, not starting with `//`), prints them per folder, and fails above the ceiling of the line budget (9,000 while the foundation is built, 8,000 at its end). It replaces the `wc -l` count of handover 14.3. | `cli/tests/core_size.rs` (beside `word_ban.rs`) | — | It passes on today's code (7,236) and runs in CI |

### Phase B: the base layers (about 45 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| B1 | **SPACE:** a `Mesh` with cells (area, centre, height, latitude), links (cells, distance, width), levels (parent, children) and compressed link lists. A square builder that gives today's grid: 8 links per cell, diagonal width 0, the air grid as a coarser level. `Grid` stays as a thin view on it until D9. | `core/src/space/{mod, square}.rs`, `grid.rs` | 1 | Tests: areas sum to the map; every link appears once; `air_of` equals the parent at the air level for every cell |
| B2 | Stacks: nodes above and below a cell (for later air, soil and sea layers), with vertical links | `core/src/space/` | 1 | A test column of 3 layers |
| B3 | **STORE:** a node table: `NodeId` (slot, generation), class, parent, cell; the ground, air and deep nodes registered in it (their arrays stay where they are) | `core/src/store/{mod, nodes}.rs` | 1 | Every ground, air and deep node has an id; ids are stable through save and load |
| B5 | **Moves, reactions, link fluxes** and `apply`: it adds them, sets rounding below zero to zero and counts it | `core/src/store/moves.rs` | 1 (the machinery alone) | A random test: 10⁶ random moves, reactions and fluxes that do not overdraw keep every element and the energy to rounding |
| B6 | Boundary nodes sun, space and outside; `Ledger` becomes their totals; `check::drift` reads them | `world.rs`, `check.rs` | 1 | The drift printed is the same as before |
| B7 | **CLOCK:** time in seconds; processes with periods; the month of `schedule.rs` rebuilt on it in the same order | `core/src/clock/`, `schedule.rs` | 1 | Golden fingerprints unchanged; `World::day` and `season` derived from the seconds |
| B8 | One sleep and wake for nodes: an alarm time, and a wake when an amount of a node has grown by more than a share of what it held when it fell asleep | `core/src/clock/sleep.rs` | — (used in D6 and E4) | Tests: a calm node sleeps; a gain wakes it; a sleeping node's catch-up equals an awake node's steps within the solver's tolerance |
| B9 | Random numbers keyed by `NodeId` and seconds for new draws; old draws keep their keys until their task changes them | `rng.rs` | 1 | No two purposes share a stream (as today) |

**Deleted in or by the end of this phase:** the `Ledger`'s own fields (now the boundary nodes' totals). `grid.rs`
stays only as a thin view until D9 removes it.

### Phase C: the shared maths (about 25 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| C1 | `spread`: an implicit step on any mesh (conductance per link, capacity per node), fixed iterations and order, returns link fluxes | `core/src/solve/spread.rs` | — | Matches the exact answer for two nodes and for a line; one big step is close to 100 small ones; books exact |
| C2 | `downhill`: the high-to-low order as a tree, updated where heights change; one pass moves surface liquid and its load | `core/src/solve/downhill.rs` | — | On test terrain: the same end state as today's sweep (`flow/liquid.rs`) to rounding for one step; faster |
| C3 | `lines`: the wind's tridiagonal solve moved here | `core/src/solve/lines.rs`, `flow/air/wind.rs` | 1 | Fingerprints unchanged |
| C4 | A "big step against small steps" test for every solver | `core/tests/solve.rs` | — | All pass |

### Phase D: the physics laws on the new base (about 45 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| D1 | Ground heat between cells by `spread`, every flow step (no longer yearly in `creep.rs`) | `flow/land.rs`, `flow/creep.rs` | 3 | Ranges hold; a hot spot spreads in a test at the right speed |
| D2 | Liquid: pore liquid by `spread`, surface liquid by `downhill`, the deep store as a stack node with a vertical link | `flow/liquid.rs`, `flow/drain.rs` | 3 | Ranges hold (river cells, rain, lakes); faster |
| D3 | Air: air nodes as the coarser level; exchange through the parent relation; wind on links (`lines` on squares, `spread` on any mesh) | `flow/air/` | 2 on squares | Ranges hold; the wind also runs on a mesh that is not square (test) |
| D4 | Radiation reads the mesh (latitude per cell) | `flow/air/radiation.rs` | 1 | Fingerprints unchanged |
| D5 | Erosion on every liquid step, along `downhill` (no `WORK_MONTHS` sample) | `flow/liquid.rs`, `flow/mod.rs` | 3 | The land wears 0.01 to 0.07 mm a year (test `an_island_loses_ground_at_a_real_rate`) |
| D6 | Change: `change/screen.rs` replaced by the clock's sleep and wake | `change/` | 3 | Ranges hold; the sleep-against-no-sleep test passes; carbon, nitrogen and lime as before |
| D7 | Bond: parts are store nodes; bonds are links with a strength; **walls are links that pass heat, light and rain badly** (the enclosure conductance and `walls_around` become link conductances) | `bond/` | 3 | Bond tests pass, including `walls keep a body warm` |
| D8 | `World::pass` and `lanes.rs` become the first link field | `lanes.rs`, `world.rs` | 1 | Fingerprints unchanged |
| D9 | Every law moves through the store: no law writes the arrays directly (the grep test of 5.2); `Grid` removed from the laws | all laws | 2 | The contract test passes |
| — | **Phase check:** the island without life, against A2 | | 3 | Ranges hold; flow and change at least 2 times faster |
| D11 | **The store only adds** (D9): each law that could ask a node for more than it holds stops doing so in its own code (a node limits only its own outgoing links); `apply` adds, sets rounding below zero to zero and counts it. Its copies, restores, rescans and the scaling of whole fluxes go | `store/`, the laws that call `apply` | 3 | Books exact; the golden runs re-pinned; `store/` within its allowance (400 code lines) |
| D12 | **Wake by what a node gained since it fell asleep**, looked at by a scan at its law's step, not by `apply` (Idea 6); what bodies bring wakes a node too | `clock/sleep.rs`, `change/` | 3 | The sleep-against-no-sleep test passes, also with `wake_share` × 2 and ÷ 2 (D4) |
| D13 | The work spaces kept between steps: `Spread` (land, pore liquid, wind), the vapour-pressure tables (one per gas, on a fixed grid of temperatures), Change's batches | `flow/`, `change/` | 2 | Same to rounding of the tables |

**Deleted in this phase:** `change/screen.rs`; most of `flow/scratch.rs`; the sorted sweep, `pore_flow` and the
sampled work (`WORK_MONTHS`) of `flow/liquid.rs`; the movers of `flow/drain.rs`; `ground.rs`'s `take_heat`; `grid.rs`;
the scaling, copies and restores of `store::apply` (D11). D1 to D10 are reported in `docs/reports/foundation-d.md`.

### Phase E: the life engine (about 40 days)

The tasks follow the profile of a living year (Idea 9): the bookkeeping first, then light-eaters without a brain and
fewer updates, then the culls and the child join. Order: E1, E8 beside the others, E4, E9, E10, E6, E11, E12, then E5.

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| E1 | **The bookkeeping of a body update.** A change of a group's members changes its count at once and waits in the group by born rows; the sum of born rows and the rows follow in one pass when the rows are read (`Life::settle`). Each body's height is kept with its mass and leaf area (`Body::vis`). The temperature solve (`Body::thermal`) runs once per stage of an update. The upkeep plan (`pay::Payer`) shares the body's air. The chemistry's mineral list is made once | `life.rs`, `survey.rs`, `body/`, `chem/roles.rs` | 2 | **Built:** about 15% less per body update (64 × 64, seed 1, year 20: 42.5 to 35.5 µs) |
| E2 | A list of active reactions per gene set, made when the genes change, **only if** a profile of a living year puts the reactor above 5% of the bodies' time | `body/metab.rs`, `change/kernel.rs` | 2 | Same to rounding; faster |
| E3 | Lookup tables for the rate's temperature factor, on the same condition | `chem/thermo.rs` | 2 | Within 1e-6; faster |
| E4 | **The pace of population nodes** (D13): a node updates at most every `node_days` (about 36 days, a speed setting); an individual by its `INTERVAL` gene. The update must give the same rates per unit of time whatever its span: a prototype found light-eaters living 3.5, 5.5 and 8.0 years at spans of 18, 36 and 72 days, a step error of the update (the order of upkeep, light, deaths and children inside one span) that E4 finds and fixes first | `life.rs`, `body/live.rs`, `copy/` | 3 | Ranges hold; `node_days` × 2 and ÷ 2 within noise (the light-eaters' life too); the bodies' time per simulated year falls about 2 times. **Built** for nodes without a brain (a body with a brain keeps its gene's pace: its actions and learning go per decision): body updates 97,200 to 54,200 a year, the bodies' time 1.6 times less; nine rules that counted updates are rates over the span. A step error is left (light-eaters' life +14 to +24% at 36 days against no floor), for E6 and E11 to remove |
| E9 | **Light-eaters without a brain.** A light-eater's brain mostly makes noise: its decisions are random moves that fail, watches that do nothing for a body that does not learn, and random gifts; decisions are about 30% of its update, and its born rows most of a snapshot. Everything a light-eater really does is already the body's physics (`body::live`: light, roots, breath, growth, brood, children spread by `DISPERSE`, shedding, dormancy, frost, ageing, death), which a body with the `HIDDEN` gene at 0 already runs, paying nothing for a brain. So the light-eaters start with `HIDDEN` 0. No new law, no path for light-eaters in core, no new tuned number; a lineage may evolve a brain back through the same gene if a brain pays. "May be a node" becomes "a body without a brain" (the light-eater test leaves core); the hand-set brain biases of the light-eaters' start and `--plant-bias` go | `worldgen/src/life/`, `nodes.rs`, `cli/`, tests | 3 | Ranges hold (the light-eaters' life checked against a reference the owner sets, never a widened range); a plant node-year at about 250 µs or less with E4 (measured on a prototype: 676 to 263 µs). **Built**, with no born rows and no brain group for a body without a brain: a body update about 2 times cheaper; born-row sets at year 30 from 5,455 to 134; snapshot 12.9 to 10.6 MB; members split off from 238,734 to 488 in 30 years. Nine key numbers in range; the light-eaters' life 4.07 years against 1.895 (more and smaller members, no brain to pay, no random gifts): the owner widened its range to ×/÷ 3 |
| E10 | **Culls: what costs code or time and changes nothing measured** (an ablation of each, 64 × 64, 32 and 60 years, two seeds). (a) **Shared group brains** (after E9 only the animals are in groups) (`life::Group`, `settle`, `weigh`, `resum`, the joins of `regroup`, `lineage::own`, `Body::adj/near/since`, the tuned `leave`, `join`, `stay`, the switch `--own-brains`): one brain per body gives the same world at the same cost per decision, and a population node already makes its members share one brain; each body or node keeps its own rows. (b) The `FORGET` gene and the fading of what a group learned: every animal stays at "off". (c) The experiment flags of old questions: `--clever-year`, `--clever-n` and the big-brain test, `--spray-size`, `--spray-genes`, `--regrow`. (d) The shade sorted once a month (or when a body of the cell updates), not every 3 days: the same world, shade 5% of the bodies' time to 0.4%. (e) The body's temperature solve (`Body::thermal`, still about 12 per update, 7% of the bodies' time) solved once per stage and passed on. (f) Caches out of the snapshot: `Body::k` filled whenever the genes change, and not saved; no spread kept for a gene that is off | `life.rs`, `choose/`, `lineage.rs`, `body/`, `survey.rs`, `worldgen/`, `cli/`, tests | 3 | Ranges hold; about 300 code lines and 4 tuned numbers fewer; snapshots smaller. **Built:** about 226 code lines and 3 tuned numbers fewer (tuned 61 to 58); the shade worked out again only for cells whose bodies changed (bodies 3 to 8% faster, the same history); 12.6 to 7.6 temperature solves per update (a body update 4 to 13% faster); snapshot at year 30 14.7 to 10.0 MB. One brain per body is not the same world for the eaters: they learn at their full rate and weigh 1.6 to 2 times more (as the old `--own-brains` gave); the eaters' mass 4.88 against 4.497: the owner widened its range to ± 0.5 |
| E5 | **Bodies book through moves; one body node per cell; a private `Store`** (D11). What a body takes from and gives to the ground and the air, its heat, its breath, its liquid, its death and its growth's gas are `Move`s, gathered in one buffer per update and booked by one `apply`; the formulas of `body/exchange.rs` and `body/physics.rs` stay (they are the physics). One body node per cell holds the light and the heat Flow brings its bodies between their updates (handover 5.2's collapsed form: Flow touches one more node per cell, never each body). The store's arrays become private fields of a `Store` written only by `apply`; the bed becomes a class like the others | `body/`, `choose/feed.rs`, `copy/grow.rs`, `survey.rs`, `world.rs`, `ground.rs`, `store/` | 3 | Ranges hold; the heat-balance, pores, dormancy and frost tests pass; books exact; `cli/tests/contract.rs` deleted (the compiler checks the contract) |
| E6 | **Cheap child placement** (the owner's choice, from a review of reproduction). Placing children is about 16% of a living year: a light-eater node builds each group of children as a whole body (128 genes drawn with noise), searches its cell for a young node, and pools the genes back, 73,000 times a year for 3,400 new bodies, and searches for a second parent each time (1.3% find one, and a node's members mix inside it anyway). (A) Children that join a node are booked on it directly: members, matter, heat, and the gene means and spreads by formulas (the mutation variance of one generation added to the spread, drift kept); a body is built only for children that found a new node. (F) A node parent seeks no second parent; individuals keep two parents (handover 6.4). (D) Children that leave the cell no longer need to make one whole member, so dispersal does not depend on the update's span. One helper moves members, stores, heat and gene moments between nodes, built with F1 | `copy/`, `nodes.rs`, `life.rs` | 2 (F, D: 3) | Child placement about a third cheaper per call; the same world (key numbers) **Built** (`nodes::Lot`, `nodes::add`: one helper for every join, merge, split and booking, which is F1). Not cheaper per call: the cost is in drawing and pooling the genes and finding the node, not in building a body; `copy` 21% to 19% of the bodies' time. |
| E11 | **Young that grow up** (C). Today 91% of light-eater members sit in young nodes and never mature: a young node matures only when its mean member reaches the copy mass, but newborns keep joining and pull the mean down, and inside a node all members share light equally, so none can outgrow the others (real stands have size hierarchies: Weiner 1990). Each kind in a cell has an adult node and a young node joined by a link: births flow into the young node, and members mature one by one along the link at the rate their growth gives. The size classes (`class`, `alike_log`, `LEAST_KG`, `CLASS_SLACK`) and the young-into-adult part of the monthly merge go | `nodes.rs`, `copy/`, `body/` | 3 | Ranges hold; the share of members in young nodes and their median size against the copy mass reported; the light-eaters' life measured against its reference **Built** (an adult and a young node per kind and cell, a spread of member sizes, maturing at the rate growth gives; the size classes gone). Young members stay about 92% of light-eater members: most newborns starve within about 2 months, which is real, not an artefact. Members counted from when they join and unpaid upkeep not compounded within an update make the pace runs flat (the step error of E4 gone). |
| E12 | **Selection inside a node**: members are born and die in proportion whatever their genes, and splits are rare (345 in 35 years for 5,500 nodes), so a node's genes barely evolve. A node runs its update for two sample members, one step of spread above and below its mean on one gene (a different gene each update, by keyed random numbers), and moves its mean by the spread times the difference in what they gain (the breeder's equation and the Price equation, not a new law) | `nodes.rs`, `copy/` | 3 | Selection inside nodes measured near the individuals' rate; `genes_do_not_drift_without_selection` passes **Built** with one sample member per update (the node's own update is the other side; the side drawn), for nodes without a brain; genes that act only when children are made are selected between nodes only. It costs about 35% of the bodies' time; the sample now draws only among genes that change the gain within one update. |
| E7 | ~~Members split off a node~~: **dropped**. After E9 only about 500 members split off in 30 years | — | — | — |
| E8 | **One clock, state apart from cache, fixed nodes by class and index, one wind solver.** Flow's parts become processes of the clock's list; Flow's scratch, `Life`'s monthly sums and `Body::k` split into a small saved state and an unsaved cache; ground, air, deep, boundary and potential nodes are named by class and index (only what comes and goes keeps a slot and generation); the wind keeps one solver if the other costs little | `schedule.rs`, `flow/`, `store/`, `clock/`, `life.rs` | 1 (the wind: 3) | Fingerprints unchanged; snapshots smaller. **Built**, but: one clock changed the world a little (the old timers' first steps were uneven); `Body::k` stays saved (making it a cache means filling it whenever genes change, a change of the laws: with E5); the wind keeps one solver, `lines`: its form on any mesh (`spread`) was 5 times slower and lost the vapour the lower half of a column carries (rain on land −28%), and the globe's rows are square and wrap (G1), so it goes |
| — | **Phase check:** the island with life, against A2 | | 3 | Ranges hold; a body node at least 3 times cheaper per simulated year; the cost model updated |

**Deleted in this phase:** `survey.rs`'s `hold_heat`; `World::light_pool`, `heat_pool`, `light_keep`, `heat_keep` and
`PerSlot`; `ground.rs`'s `draw` and `take_paid`; `cli/tests/contract.rs`; the direct writes of `Sky::cross` by bodies;
Flow's own timers; the node table for fixed nodes. The code that only moves things in `body/physics.rs` goes; the
formulas stay.

### Phase F: population nodes (about 20 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| F7 | **First: the nodes-against-individuals check** (handover 7.1, as written there), as a slow test and a report, with a census of young and adult members. It runs during Phase E, after E12, and decides F3 and F5 | `tape/tests/`, report | — | The modes agree by the handover's three rules, or the difference is found and fixed |
| F1 | **The small split and join helper** (Idea 3): amounts and heat by shares, the last child takes the remainder; used by `nodes.rs` and by the mesh rebuild (G4) | `store/` | — | A random test: 10⁵ random splits and joins keep the books to rounding; split then join gives back the node |
| F2 | `nodes::{merge, join, split, one, open}` use F1 | `nodes.rs` | 2 | Copy tests pass |
| F3 | No merge of unlike nodes in a full cell (decision D8), after F7; if a cap stays, it is a speed setting | `nodes.rs` | 3 | No unlike merges are counted; ranges hold |
| F5 | More age classes inside nodes than young and adult (E11), **only if F7 shows** they matter | `nodes.rs` | 3 | Measured, kept or dropped with a reason |

**Deleted in this phase:** the amount and heat splitting of `nodes.rs` (it calls F1); the unlike-merge safeguard of
`nodes::crowd`.

### Phase J: the long physics step (about 25 days; right after Phase E)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| J1 | **A longer physics step.** Liquid every 30 days is built (Flow about 25% cheaper; the long-against-short test passes). **Heat, air and rain are not converged at 3 days**: from a step of 6 hours to 3 days, rain on land falls from about 4e-3 to 1.2e-3 mol/m²/s, river cells from about 800 to 500, the seasonal swing from 12.1 to 9.9 K; at 15 days rain falls to 0.4e-3, and at 30 days the run fails. The causes: radiation takes the air's temperature from the start of the step (the air settles in about a week); the wind's limits `EVEN` and `OVER` are per call, not per second; the exchange moves at most what a node holds per call; exchange and wind run one after the other. The honest long step is one implicit solve of the air's heat (radiation implicit in the air's temperature too), vapour and wind together, with the exchange inside it, and `EVEN` and `OVER` as rates per second or gone. Since the 3-day world is not the converged one, **the key numbers follow the converged world** (the owner's choice): the air's tuned numbers are anchored again to real sizes, and new reference runs replace A2's for the climate | `flow/air/`, `flow/land.rs`, `solve/` | 3 | The world the same at 6 hours and at the long step (the long-against-short test against a 6-hour run); a cell's physics at least 5 times cheaper without threads |
| J2 | Flow and Change keep their threads by rows (`par.rs`) on the globe's wrapping rows | `par.rs` | 1 between threads and none | Fingerprints equal |

### Phase G: the globe (about 35 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| G1 | **The lat-lon grid** (D3): squares that wrap east to west; the area and the border widths of each row from its latitude; latitude and longitude per cell; the wind's lines wrap (the cyclic tridiagonal solve); worldgen samples its noise on the sphere | `space/square.rs`, `flow/air/wind*`, `worldgen/src/terrain.rs` | — | A test globe without life runs 10 years; books exact; the sun is right at every latitude |
| G2 | **Life on the mesh's links**: Choose, Copy, Bond's drift and the brain groups step on the mesh's links (each link a bearing for the brain's 8 direction slots), not on `Square::step` and `DIRS`; the air's four-point mix reads the mesh | `choose/`, `copy/`, `bond/`, `life.rs`, `flow/scratch.rs` | 2 on the island | The same world on the island |
| G3 | **Cheap sea cells**: the sea on a coarser level (as the air), so that a sea cell costs a small share of a land cell | `flow/`, `space/` | 3 | The key numbers hold on the island; the cost of a sea cell measured |
| G4 | **A refined region built into the mesh**: finer cells in one region, at the start or rebuilt at a god action; amounts split by F1 on a rebuild, summed back | `space/`, `store/` | 3 | Books exact through a rebuild; liquid and heat cross the border of the region correctly (tests) |
| G5 | **The coarse-against-fine test** | `tape/tests/` | 3 | Key numbers agree within their ranges |
| G6 | Remove old limits: the 20-bit cell key of the liquid sort, the 64-substance bit masks (`u64`), the sea level fixed at 0 in the land and sea counts | `flow/`, `body/`, `tape/` | 1 | Tests with more than 10⁶ cells and more than 64 substances |

### Phase K: recording and viewer (about 20 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| K1 | The tape takes `&World` everywhere (Idea 10): `World::drain_events` and `World::take_counters` | `tape/`, `cli/`, `world.rs` | 1 | No reader borrows `World` mutably |
| K2 | The tape and the viewer file for the globe and a refined region | `tape/src/rec/` | — | `docs/tape.md` updated |
| K3 | The viewer: the globe as a map, the refined region, the link layer (lanes); charts grouped and searchable | `web/src/` | — | You can go from the whole globe to a refined region |
| K4 | Time per process in the tape | `tape/`, `cli/src/report.rs` | — | Speed problems visible in the viewer |

### Phase L: the final check (about 15 days)

| ID | What | Done when |
|---|---|---|
| L1 | A2's reference runs on the new engine | Ranges hold; every larger difference explained |
| L2 | Speed | The time budget met on the largest planet that fits, with the cost model's numbers |
| L3 | Books | Elements and energy exact over 1,000 years, with threads and a refined region |
| L4 | Show worlds: the globe, a refined region | Each one runs, with pictures |
| L5 | The foundation watch report; then the owner decides M4 | The owner's choice is written in the handover |

## 8. Order, parallel work and size

```
E (E1 to E8) ─► J (the long step) ─► G1 to G3 (the globe) ─► F1 to F3, F5 (as F7 asks) ─► G4, G5 ─► K ─► L
F7 runs during E, after E4.   E8 can run beside E4 to E7.
```

| Phase | Days (rough guess) |
|---|---|
| A to D (done) | 125 |
| E life engine | 40 |
| F population nodes | 20 |
| J long step | 25 |
| G globe | 35 |
| K recording and viewer | 20 |
| L final check | 15 |
| **Left after D** | **about 155 days** |

Work like this usually takes longer than planned. Report the real days in each phase report.

---

## 9. Risks and stop rules

| Risk | What to do |
|---|---|
| **The rebuild never ends and nothing is watched** (the version 1 trap: handover 2, lesson 4) | The island runs after every task. A phase more than 50% over its days stops, and the owner decides. |
| Exact books across split, join and a mesh rebuild | F1's random test is the gate: G4 does not start while it fails. |
| The new engine gives a different world | Level-3 differences are allowed only when the report explains them. A range broken without a reason stops the phase. |
| A speed tool makes the world (nodes, the node pace, sleep, the long step, cheap sea cells, a refined region) | Its equivalence test (6.4) is part of its "done when". |
| The new base becomes one more layer on top of the old code (the opposite of the main rule) | Each phase's "Deleted in this phase" list is part of its "done when". The code-size test (A6) fails above the ceiling. Each phase report gives the lines added and removed per folder. |
| Code size grows past the budget (D2: 8,000 code lines at the end) | Simplify before adding (handover 14.3). A new part past its allowance needs the owner's agreement. The ceiling never justifies a task that costs speed or honesty. |
| Speed work aimed at the wrong half | Each speed task starts from a profile of a living year; the order of Phase E follows it. |
| The planet that fits the time budget is too small to watch | Report it with the cost model; the owner chooses a longer budget, threads over bodies (5.6), or a change of the laws' cost. |
| New tuned numbers creep in | The tuned budget test (`core/tests/numbers.rs`) stays; speed settings go in their own table (D4). |
| Body physics changes sneak in | D6: bug fixes only, each its own branch. |

---

## 10. After the foundation

### 10.1 Waiting on purpose (D14)

These ideas add realism, or speed later, but before M4 they cost speed or code more than they give. Each comes back
with the work that needs it.

| Idea | Why it waits | When it comes back |
|---|---|---|
| Focus zones (5.4): fine cells and a fast clock where things happen | Realism (chases, actions that take time), not speed; finer cells and minutes cost more | After M4, with the mind |
| Automatic zoom while running | A refined region built into the mesh (G4) gives detail in places with much less code | When a player must zoom anywhere, at any time |
| Patches: parts of a cell's ground with their own surface | Their first use is a burning share of a cell | With fire (M4) |
| More memories on links (a channel worn by liquid, an ease for signals) | A channel would repeat what erosion already does to a cell's height | With the law that needs one |
| Threads over bodies (5.6) and the chunk layout (Idea 8) | The game runs on one thread; batches run one world per core | When a planet profile shows one native thread for the bodies limits a long run (the first lever if the planet is too small) |
| Snapshots of changes only | In 10 years almost everything changes; saving less (E6, E8) gives more | If snapshots are still too big after E6 and E8 |

### 10.2 What M4 still needs

The foundation gives the machinery. M4 then adds the content (handover 13), which becomes much smaller:

- **The planet:** real latitudes, continents and basins on the globe (G1 gives the mesh).
- **Air in 2 or 3 layers** with momentum and the planet's turning; clouds that travel with the wind and keep heat in
  (uses B2 stacks, C1).
- **The sea in 2 layers** with a salt substance (owner's choice), currents from wind, heat and salt, thin ice on top
  (B2, C1, G3).
- **Snow and ice that flow**; a slow carbon source like volcanoes; slow uplift.
- **A burning share of a cell** that spreads along links, with sparks from strong storms booked like the sun (patches,
  D1; handover 13.2, "Fire and disease").
- **The mind** (handover 13.1, option 2): memory of places, a short look-ahead, learning in the hidden layer, used in
  zones.
- **Long watch runs** on the planet, 1,000 years and more, several seeds, against null runs (handover 18.3).

---

## 11. Words used in this plan

| Word | Meaning |
|---|---|
| mesh | the set of cells and the links between them: the shape of the world |
| level | a coarser layer of the mesh (the air, the sea) or a refined region: a cell of one level holds 4 cells of the next |
| stack | the nodes above and below a cell: air layers, soil layers, the deep store |
| patch | a part of a cell's ground with its own soil, liquid, heat and cover |
| zone | a small area taken to the finest level, with its own fast clock (after the foundation) |
| refined region | a region of finer cells built into the mesh at the start or at a god action |
| time budget | how long a run may take on the owner's machine; it sets the planet's size |
| cost model | the cost of a cell and of a body node per simulated year, times their numbers |
| move | a change the store applies: amounts and heat from one node to another |
| link flux | what crosses each link for one substance, for a whole field at once |
| boundary node | the sun, space or outside: where energy and matter enter or leave the world |
| implicit step | a way to do a time step that stays stable for any step length |
| columns | each quantity in one array for all nodes of a class (fast for the computer) |
| chunk | a block of about 64 cells that can sleep or run on its own thread (after the foundation) |
| fingerprint | one number made from the whole saved state (`tape::fingerprint`); equal fingerprints mean the same history |
| equivalence test | a test that a speed tool gives the same world as running without it |

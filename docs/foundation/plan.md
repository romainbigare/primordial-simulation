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
| The bodies' own ring of updates due | `life.rs` (`RING`, `due`) | The clock. |
| Merging, splitting and opening population nodes | `nodes.rs` (`merge`, `join`, `take_in`, `split`, `one`, `open`) | The store's one `split` and `join`. |
| Ground cleared along edges | `lanes.rs`, `World::pass` | The memory of a link. |
| Heat moving sideways between cells once a year | `flow/creep.rs` | `spread` on links, every step. |
| Pore liquid between cells (Darcy's law) | `flow/liquid.rs` (`pore_flow`), `flow/drain.rs` | `spread` on links; the deep store is a node in the cell's stack. |
| The surface liquid's sorted sweep, and erosion sampled every 7 months | `flow/liquid.rs`, `WORK_MONTHS` | The `downhill` tree, every step. |
| Taking heat from a node with a guard in each place | `ground.rs` (`draw`, `take_paid`, `take_heat`) | A law never asks a node for more than it holds; `apply` only adds (Idea 2). |
| Caches the laws keep between steps | `flow/scratch.rs` (most of it) | The mesh and the store hold what is needed. |

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
| `space/` (meshes, levels, stacks, zoom) | 500 |
| `store/` (nodes, amounts, moves, links, split and join) | 400 |
| `clock/` (processes, sleep and wake) | 250 |
| `solve/` (`spread`, `downhill`, `lines`) | 350 |
| `zone/` | 400 |
| `query/` | 150 |
| **Total** | **2,050** |

- Most of the old laws' "moving code" turned out to be physics (rates, conductances, heads), so the laws shrink less
  than the base grows. The savings come from keeping the base small (the store only adds, Idea 2) and from deleting
  the special code the main rule's table names.
- A test (task A6) counts the code lines and fails above the ceiling: **9,000 until the end of the foundation**,
  **8,000 at its end (Phase L)**. The 8,000 is never a reason to build a task that costs speed or honesty (it is why
  a body is not a node in Flow's steps, Phase E).
- A phase may not end with more code than it started with, unless its report gives the reason.

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
| Too slow: about 15 s per year with life on 128 × 128, against a goal of 0.6 s; it gets slower as life grows | Each body update costs about 36 µs; bodies are about 80% of the time | Phases E, J |
| Heat moves sideways between cells once a year, so a burning front cannot spread | `flow/creep.rs` runs lateral heat yearly | C1, D1 |
| Erosion is a 15-day sample every 7 months, multiplied | `WORK_MONTHS` in `flow/mod.rs` | D5 |
| Water crosses the map in one sweep; it is sorted again every time | `flow/liquid.rs` | C2, D2 |
| Links between cells hold almost nothing; only cleared lanes remember use | `World::pass`, `lanes.rs` | H |
| No levels of detail: the plan's "rough far away, detailed where it matters" (handover 5.2, 7.2) is not built | One flat grid | F, G |
| Population nodes change biology: selection inside a node runs at about 3%; unlike nodes are forced to merge in full cells | `nodes.rs` | F2 to F5 |
| One decision per 5 days or more, one cell per move: chases, building and tool use are almost impossible | Bodies live only at the world's coarse time | I |
| A flat grid cannot become a planet | `grid.rs` is rows and columns | B1, G2 |
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
| D3 | The cell shape for the planet and for zoom | **Decided:** squares on a cube-shaped ball (see 5.5). They nest exactly, so zoom keeps exact books, and today's square laws carry over. |
| D4 | Speed settings (when a zone opens, when a node sleeps, when a patch splits, how many nodes a cell keeps) | **Decided:** a new table "speed settings" in `docs/numbers.md`, outside the tuned budget. Each must pass an equivalence test (6.4): changing it 2× must not change the key numbers beyond their noise. |
| D5 | Old snapshots | **Decided:** no backwards compatibility; snapshots carry a format version (`tape/src/snapshot.rs`). |
| D6 | Body physics during this plan | **Decided:** frozen, bug fixes only. Moving existing body physics onto the new base (same physics, new code) is part of the plan, not new physics. |
| D7 | How the work merges | **Decided:** phase by phase into `main`, after the owner reads the phase report. No long-lived branch. |
| D8 | The safeguard that merges unlike nodes in a full cell (`nodes::crowd`) | **Decided:** removed once nodes are cheap (F3). If a cap is still needed for speed, it becomes a speed setting (D4). |
| D9 | Who keeps amounts at or above zero | **Decided:** the law. A law never asks a node for more than it holds; `store::apply` only adds, clamps rounding and counts it (Idea 2). |
| D10 | The check level of speed work | **Decided:** level 2 (the same to rounding) is the normal level for speed work: it may change the order of sums, and it re-pins the golden runs in the same commit. Level 1 stays for pure clean-ups. |
| D11 | Bodies and Flow | **Decided:** a body is not a node in Flow's steps (that would make every Flow step touch every body, 120 times a year, through stiff heat links: a 1 kg body against a ground of gigajoules). Bodies book their trades as moves at their own updates, and one body node per cell holds what Flow brings them between updates (E5). |

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
  tape,  ←──│  QUERY: read only                         │   tape/, cli/, web/, tests read only this
  viewer    └──────────────────────────────────────────┘
            ┌──────────────────────────────────────────┐
            │  THE FIVE LAWS                            │   core/src/{flow, change, bond, copy, choose}/
            │  they compute; they change the world only │   and body/
            │  through moves and link fluxes            │
            └──────────────────────────────────────────┘
            ┌──────────────┬──────────────┬────────────┐
            │ SOLVE        │ LIFE         │ ZONE       │   core/src/solve/, core/src/life/, core/src/zone/
            │ shared maths │ bodies as    │ fine space │
            │              │ columns      │ and time   │
            └──────────────┴──────────────┴────────────┘
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
| SPACE | nothing else | its own mesh, when cells zoom | know about substances, heat or bodies |
| STORE | SPACE | amounts, heat, links' state, through `apply`, `split`, `join` only | decide anything physical |
| CLOCK | STORE (wake marks) | the time, the queue of what is due | change amounts |
| SOLVE | inputs given to it | its outputs (fluxes, new values) | touch `World` directly |
| LAWS | everything below | moves and link fluxes, given to STORE | write an amount or a heat directly |
| LIFE, ZONE | everything below | through STORE and CLOCK, as the laws do | hold a second copy of anything the store holds |
| QUERY | everything | nothing | be used by the laws |

A test checks the contract where it can: a grep test that no law outside `store/` writes `ground[`, `e_ground[`,
`air[` or `e_air[` with `=`, `+=` or `-=`, like the word-ban test.

### 5.3 The ten ideas

**Idea 1. Everything is a node; nodes live inside nodes.**
A node is anything that holds amounts and heat: the ground of a cell, a patch of it, a soil layer, an air layer, a deep
store, a body (one member or a population), a part, a zone. Each node has a **class** (how it is stored: `Ground`,
`Air`, `Deep`, `Patch`, `Body`, `Part`, `Zone`; a storage word, not an Earth word), a **parent** and a stable
`NodeId` (slot + generation, as bodies have today). This is handover 5.2 ("graph inside a graph").

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

**Idea 3. One split and one join for everything.**
Zoom (a cell splits into children), population nodes (a node splits, nodes merge), zones (cells split down to the
finest level and come back), patches (a cell's ground splits by its kinds of surface): they are all one operation.

```rust
fn split(store, node: NodeId, shares: &[Share]) -> Vec<NodeId>  // Share: area or members, per substance if needed
fn join(store, nodes: &[NodeId]) -> NodeId
```

Rules: each substance is split by its shares, and the **last child gets the remainder** (parent minus the others), so
the sum is exact to rounding. Heat is split so every child has the parent's temperature, unless a share says
otherwise. Genes join as the member-weighted mean, with the pooled spread (today's `nodes::take_in`). Links split by the
length of border each child gets, and join by length-weighted means. This is the most important code of the plan: it
gets the most tests (F1).

**Idea 4. Space is any mesh, with levels and stacks.**
The laws see only:
- cells, with area, centre (and latitude, longitude), height;
- links, with the two cells, the distance between centres, and the **width** (the length of the shared border);
- each cell's parent and children (levels of zoom);
- each cell's stack: the nodes above and below (air layers, soil layers, deep store).

The **width** keeps today's square results: a diagonal link has width 0 (squares touch only at a corner), so heat,
creep and pore liquid cross only real borders, while the downhill route of surface liquid may still use all 8. The air
mesh is a coarser level of the same mesh: `Grid::air_of` becomes "the parent at the air level".

Suggested storage: compressed lists (one array of all links, and for each cell the start of its links in it), built
once per mesh change.

**Idea 5. Links are real and have memory.**
Each link holds, besides its geometry: an ease of passing for liquid, for bodies, and (later) for signals; the wear
that raised them; what was carried along it in the last year. Use raises an ease; physics (creep, regrowth) lowers it
slowly. `World::pass` and `lanes.rs` become the first of these fields. This is handover 5.1: "a route is simply an edge
whose conductance became high because it was used a lot".

**Idea 6. Every process has its own clock.**
- Time is a whole number of **seconds** (`u64`). Days and months are derived (the tape still writes months).
- Each process (a law, or a part of one, at a level) has a period, for example: radiation and heat 3 days, liquid
  15 days, bodies by their `INTERVAL` gene, a zone's bodies minutes. A fast process runs several times inside a slow
  one. The order inside one time is fixed and written once (as `schedule.rs` does now).
- **One sleep and wake** for all nodes. A calm node sleeps until an alarm time, or until one of its amounts has grown
  by more than a share of what it held when it fell asleep (`Config::wake_share`, a speed setting). The look is a cheap
  scan at the node's own law's step, not part of `apply`, so what Flow moves does not make Change slower. This replaces
  `change/screen.rs` and the bodies' due ring (`life.rs`, `RING`).

**Idea 7. Shared maths that stays stable with big steps.**
`core/src/solve/`:
- `spread`: an implicit step (one that stays stable for any step length) for anything that moves from high to low
  along links with a conductance and fills nodes with a capacity: heat, pore liquid, dissolved salts, gases, the
  pressure evening of the wind. It returns **link fluxes**, so the books are exact even when the solve is not.
  Suggested: conjugate gradient (a standard iterative solver) with a fixed number of iterations and a fixed order.
- `downhill`: the order of cells from high head to low, as a tree, kept up to date when heights change; surface
  liquid and what it carries move down it in one pass.
- `lines`: today's tridiagonal solve of the wind (`flow/air/wind.rs`), kept as the fast form on square meshes.
- The reaction step stays in `change/kernel.rs`; it already integrates the rate law over the step.

**Idea 8. Columns and chunks.**
- **Columns:** each quantity of a node class in one array (as the ground already is). Bodies too, where threads need
  them (Idea 9, E7).
- **Chunks:** blocks of about 64 cells. A chunk can sleep and run on a thread. Threads run chunks in colour passes
  (5.6). Same history with and without threads.
- **The chunk layout of amounts:** today an amount lies at `[k * n + i]`. Flow likes that (one substance over all
  cells), but Change and the bodies read one node at a time, so each node read touches 26 cache lines for 208 bytes.
  The layout `[chunk][k][cell in chunk]` keeps a chunk's amounts in one block (about 13 KB for 64 cells and 26
  substances) that fits in the first cache: a law that goes substance by substance (Flow) or node by node (Change,
  bodies) works in the cache, and a thread owns one block. This is the memory idea that pays most. Its chunks are the
  chunks of the thread design (5.6), so it is built with it (E7).
- **No sparse amounts:** at 26 substances a dense column costs little, and a sparse one adds code.

**Idea 9. A fast life engine.**
- **The bookkeeping first.** The profile of a living year (Phase A, and an outside review at the end of Phase D) puts
  most of a body update in bookkeeping, not in the brain or the reactor: re-summing a group's brain rows when a count
  changes, a temperature solve run again and again, the survey recomputing every body's height. These go first (E1).
- Each lineage or gene set gets its **list of active reactions** once, when its genes change, not at every update.
- Lookup tables for the slow maths: the rate's temperature factor, vapour pressure.
- Bodies as columns (today `Body` holds its own `store`, `genes`, `spread` vectors), where the threads need them.
- The brain's own maths (`think`) is a small share of the time: it stays as it is.
- **Selection inside population nodes at full speed** by sample members (F4): a node runs its update also for two
  sample members, one step of spread above and below its mean on one gene (a different gene each update, by the keyed
  random numbers), and moves its mean by the spread times the difference in what they gain. This is the breeder's
  equation (the change of a mean is the spread times the strength of selection), not a new law. It must pass the
  nodes-against-individuals check (handover 7.1).

**Idea 10. A read-only window.**
`core/src/query/` (or a small crate): what the tape, the viewer, the CLI report and the tests read. They never borrow
`World` mutably. Today the tape reads the core's internals directly.

### 5.4 Focus zones (Phase I)

A **zone** is a node that holds a set of cells taken down to the finest level (about 30 m: 2,000 m halved 6 times) with
its own fast clock (minutes to an hour for bodies). Inside a zone:
- population nodes give out single members (`nodes::open` today), and take them back when the zone closes;
- a body has a position inside its cell and a speed; how fast it can go comes from the power it can pay (the cost of
  transport already in Choose);
- the sun moves (day and night) for the zone's cells;
- actions take time, and a body can keep doing one action;
- catching and escaping come from speed and distance, instead of today's chance from the two intervals
  (`choose/feed.rs`, `take_body`).

A zone opens on **physical facts only**: a body that takes from bodies is near bodies it can take; a body holds a part;
many members are in one cell; heat is spreading fast; the player looks there. It closes when these facts are gone for a
while. What crosses its border is booked as moves, applied at the outside's step. **The coarse rule outside zones must
be the average of the fine rule inside**: the zone-against-no-zone test (I9) measures that.

### 5.5 The shape of cells (decision D3)

| | Squares on a cube-shaped ball | Hexagons on an icosahedral ball |
|---|---|---|
| Zoom | A square splits into 4 exactly; books exact by area | Hexagons do not nest; children only cover their parent approximately |
| Neighbours | 4 by border, 4 more by corner | 6, all equal |
| Today's laws | carry over (they are square) | need new neighbour handling everywhere |
| Distortion | some at the cube's 8 corners | small everywhere |
| Viewer | easy (6 square faces) | harder |

Suggestion: squares on a cube-shaped ball, with a flat square island as the first mesh (it reproduces today).

### 5.6 Threads over bodies (the design of E6; written before the columns of E7, which follow it)

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

Flow and Change already run cells in parallel by rows (`par.rs`); J1 moves them onto the same chunks.

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
| plant kg, animal kg (log10 at year 30) | the reference ±0.3 (a factor of 2: handover, "reading the eaters") |
| adult life of plants and of eaters | the reference ×/÷ 2 |
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
| coarse against fine | the same island at one level and zoomed one level finer everywhere | G5 |
| zone against no zone | the same world with zones allowed and with none | I9 |
| sleep against no sleep | sleep on, and every node awake every step | B8, D12, E4 |
| threads against no threads | fingerprints equal | every phase (CI already does it for the golden runs) |
| a speed setting ×2 and ÷2 | key numbers within their noise | D4, each setting |

### 6.5 Speed

The benchmark `speed` (`tape/tests/golden.rs`) and the CLI's per-stage times are recorded at the end of each phase,
on the same machine, in the phase report. So are the code lines of `core` (A6), per folder, with what the phase
added and deleted. Goals for the end of this plan, against A1:
- the island without life: flow and change at least **3 times** faster. At today's step they are within 2 to 3 times
  of a tight one-thread floor (about 1 µs per cell per step), so this needs threads (J1) or a longer step (J2);
- the island with life: at least **10 times** faster per year, at the same body count. One thread can give about 2.5
  to 3 times (E1 to E3); the rest needs threads over bodies (E6: on 4 cores about 3.5 times).

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
| A6 | **The code-size test:** counts the code lines of `core/src` (not blank, not starting with `//`), prints them per folder, and fails above the ceiling of the line budget (9,500 until the end of Phase D, 8,000 after Phase E). It replaces the `wc -l` count of handover 14.3. | `cli/tests/core_size.rs` (beside `word_ban.rs`) | — | It passes on today's code (7,236) and runs in CI |

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
| D11 | **The store only adds** (D9): each law that could ask a node for more than it holds stops doing so in its own code (a node limits only its own outgoing links); `apply` adds, sets rounding below zero to zero and counts it. Its copies, restores, rescans and the scaling of whole fluxes go | `store/`, the laws that call `apply` | 3 | Books exact; the golden runs re-pinned; `store/` about 300 code lines or fewer |
| D12 | **Wake by what a node gained since it fell asleep**, looked at by a scan at its law's step, not by `apply` (Idea 6); what bodies bring wakes a node too | `clock/sleep.rs`, `change/` | 3 | The sleep-against-no-sleep test passes, also with `wake_share` × 2 and ÷ 2 (D4) |
| D13 | The work spaces kept between steps: `Spread` (land, pore liquid, wind), the vapour-pressure tables, Change's batches | `flow/`, `change/` | 1 | Fingerprints unchanged |

**Deleted in this phase:** `change/screen.rs`; most of `flow/scratch.rs`; the sorted sweep, `pore_flow` and the
sampled work (`WORK_MONTHS`) of `flow/liquid.rs`; the movers of `flow/drain.rs`; `ground.rs`'s `take_heat`; `grid.rs`;
the scaling, copies and restores of `store::apply` (D11). D1 to D10 are reported in `docs/reports/foundation-d.md`.

### Phase E: the life engine (about 50 days)

The tasks are in the order of the profile (Idea 9): the bookkeeping first, then the reactor, then threads.

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| E1 | **The bookkeeping of a body update.** A group keeps its learned rows and the sum of its members' born rows apart, and forms the mean only when it thinks (a change of count no longer re-sums every row: the hottest function of a living year, about 9%). Each body's height is kept with its mass and leaf area (`Body::vis`), so the survey's shade sort does not solve every body's temperature again. The temperature solve (`Body::thermal`) runs once per stage of an update and is passed along. The upkeep plan (`pay::plan`) shares the breath, cover and depth that the body's physics computes. The chemistry's mineral list is made once | `life.rs`, `survey.rs`, `body/`, `chem/roles.rs` | 2 | The same world (key numbers); the time per body update falls |
| E2 | A list of active reactions per gene set, made when the genes change | `body/metab.rs`, `change/kernel.rs` | 2 | Same to rounding; the time of reactions in A1's table falls |
| E3 | Lookup tables for the rate's temperature factor and vapour pressure | `chem/thermo.rs` | 2 | Within 1e-6; faster |
| E4 | Calm bodies sleep through the clock (instead of the due ring) | `life.rs`, `survey.rs` | 3 | The sleep-against-no-sleep test passes |
| E5 | **Bodies book through moves; one body node per cell** (D11). What a body takes from and gives to the ground and the air, its heat, its breath, its liquid, its death and its growth's gas are `Move`s booked by `apply` at its update; the formulas of `body/exchange.rs` and `body/physics.rs` stay (they are the physics). One body node per cell holds the light and the heat Flow brings its bodies between their updates (the collapsed form of handover 5.2: Flow touches one more node per cell, never each body); each body settles with it at its update | `body/`, `choose/feed.rs`, `copy/grow.rs`, `survey.rs`, `world.rs`, `ground.rs` | 3 | Ranges hold; the heat-balance, pores, dormancy and frost tests pass; books exact; the contract test has no exemption for bodies |
| E6 | **Bodies on threads by chunks in colour passes**, as designed in 5.6 | `par.rs`, `survey.rs`, `life.rs` | 3 once for the new order, then 1 between threads and none | Fingerprints equal with and without threads; all cores used |
| E7 | Columns where E6 needs them: the chunk layout of amounts `[chunk][k][cell in chunk]` for the ground, air and deep nodes, and the bodies' hot numbers as columns by chunk (Idea 8) | `store/`, `life.rs`, `body/` | 2 | Same to rounding; Change and the body update faster |
| — | **Phase check:** the island with life, against A2 | | 3 | Ranges hold; a body update at least 2.5 times faster on one thread; with threads, a living year about as many times faster again as there are cores, up to 4 |

**Deleted in this phase:** `survey.rs`'s `hold_heat`; `World::light_pool`, `heat_pool`, `light_keep`, `heat_keep` and
`PerSlot`; `ground.rs`'s `draw` and `take_paid`; every exemption of bodies in `cli/tests/contract.rs`; the direct
writes of `Sky::cross` by bodies; the due ring of `life.rs`. The code that only moves things in `body/physics.rs` goes;
the formulas stay.

### Phase F: split, join, population nodes and patches (about 45 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| F1 | **The one `split` and `join`** for every class (5.3, Idea 3). Designed for cost as much as for exact books: a child joining a node is about 17% of a living year today (`copy::emit`, `nodes::take_in`, `Life::reborn`, `brain::shake` of every weight for each child, `copy::target` searching the cell), so a join must not touch more than the numbers it changes | `core/src/store/split.rs` | — | A random test: 10⁵ random splits and joins of every class keep the books to rounding; split then join gives back the node; a join of one child into a node costs less than today's |
| F2 | `nodes::{merge, join, split, one, open}` use F1 | `nodes.rs` | 2 | Copy tests pass |
| F3 | No merge of unlike nodes in a full cell (decision D8): nodes are cheap now, so a cell may hold more; if a cap stays, it is a speed setting | `nodes.rs` | 3 | No unlike merges are counted; ranges hold |
| F4 | Selection inside a node at full speed by sample members (5.3, Idea 9) | `nodes.rs`, `copy/` | 3 | `genes_do_not_drift_without_selection` and `selection_moves_a_nodes_brain_size` pass; selection inside nodes measured near the individuals' rate |
| F5 | Age classes inside nodes beyond young and adult, if F7 shows they matter | `nodes.rs` | 3 | Measured, kept or dropped with a reason |
| F6 | **Patches:** a cell's ground splits into 2 to 4 patch nodes when its surface differs a lot (cover, liquid, what was burnt), and joins back when they are alike; Flow and bodies treat patches as nodes | `store/`, `flow/land.rs`, `survey.rs` | 3 | A test cell holds two kinds of ground; ranges hold |
| F7 | **The nodes-against-individuals check** (handover 7.1, as written there), as a slow test and a report | `tape/tests/`, report | — | The modes agree by the handover's three rules, or the difference is found and fixed |

**Deleted in this phase:** the merge, join, take-in, split, one and open code of `nodes.rs` (each becomes a call to
the store's `split` or `join` with its shares); the unlike-merge safeguard of `nodes::crowd`. Patches use `split` and
`join`, with no code of their own for creating or merging.

### Phase G: new shapes and zoom (about 40 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| G1 | **Zoom on the flat island:** a cell splits into 4 and joins back through F1; links between cells of different levels (a big cell beside two small ones) with the right widths | `space/`, `store/split.rs` | 3 | Liquid and heat cross a level border correctly (tests); books exact |
| G2 | The cube-shaped ball (or hexagons, D3): a mesh builder with latitude and longitude per cell; worldgen samples its noise at cell centres on any mesh | `space/cube.rs`, `worldgen/src/terrain.rs` | — | A test ball without life runs 10 years; books exact; the sun is right at every latitude |
| G3 | Fine heights kept small: rebuilt from the seed's noise plus a list of changes (erosion, edits), so a cell can zoom anywhere | `worldgen/`, `space/` | — | Memory measured; zooming twice gives the same heights |
| G4 | When to zoom, from physical facts only: fast change, many moves, many members, the player's view | `space/zoom.rs` | — (speed settings) | Zoom follows activity in a test |
| G5 | **The coarse-against-fine test** | `tape/tests/` | 3 | Key numbers agree within their ranges |
| G6 | Remove old limits: the 20-bit cell key of the liquid sort, the 64-substance bit masks (`u64`), the sea level fixed at 0 in the land and sea counts | `flow/`, `body/`, `tape/` | 1 | Tests with more than 10⁶ cells and more than 64 substances |

### Phase H: links with memory (about 20 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| H1 | Link fields: ease for liquid, ease for bodies, wear, carried last year | `store/links.rs` | 1 | Saved and loaded |
| H2 | Use raises an ease: liquid's lost power wears a channel on the link; movers' pushing clears a lane (today's `lanes::clear`); creep and regrowth lower them slowly | `flow/liquid.rs`, `lanes.rs` | 3 | A test: repeated use makes a lasting easy link, which fades without use |
| H3 | Links through zoom: a split gives a link's memory to the child links along its border; a join takes their width-weighted mean | `store/split.rs` | 3 | A channel survives zoom out and in |

**Deleted in this phase:** `lanes.rs` and `World::pass` (the link's memory does it). A channel, a lane and any later
"easy link" are the same field rule, not separate code.

### Phase I: focus zones (about 45 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| I1 | A zone node: cells zoomed to the finest level (about 30 m), its own clock, opening and closing through F1 and G1 | `core/src/zone/` | — | A zone opens and closes; books exact |
| I2 | What crosses a zone's border is booked as moves and applied at the outside's step | `zone/` | — | Books exact with a zone open for a year |
| I3 | Single members out of nodes, and back in (`nodes::open`, `nodes::join`) | `nodes.rs`, `zone/` | — | No member, gene or matter lost |
| I4 | A position inside the cell and a speed for bodies in a zone; the top speed from the power the body can pay | `body/`, `choose/go.rs` | — | A body walks and runs smoothly in a test |
| I5 | Day and night for the zone's cells | `flow/air/radiation.rs`, `zone/` | — | The sun moves in a zone; the day's mean equals today's light |
| I6 | Actions that take time, and keeping an action going | `choose/`, `brain.rs` | — | A long action finishes over many zone steps |
| I7 | Catching and escaping by speed and distance inside zones (instead of the chance from intervals in `choose/feed.rs`) | `choose/feed.rs` | — | Fast prey escapes slow takers in a test |
| I8 | A computer-time budget: zones wait in a queue when the budget is full (a speed setting) | `zone/` | — | Speed stays inside the budget |
| I9 | **The zone-against-no-zone test** | `tape/tests/` | 3 | Key numbers agree; what differs is reported |

**Rule for this phase:** a zone is the same laws on finer nodes with a faster clock. It may not hold its own copy of a
law. The only zone code is opening, closing and the border (within the `zone/` allowance of 400 code lines).

### Phase J: speed on the machine (about 30 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| J1 | Every law on threads by the chunks of 5.6 in colour passes | `par.rs`, laws | 1 between threads and none | All cores used; fingerprints equal |
| J2 | **A longer physics step.** Heat, air and rain run every 3 days, about 10 times more steps than handover 7.3 suggests (one month). Radiation already solves the surface implicitly; the air's side and the exchange hold the step at 3 days. An implicit step of each cell's ground-and-air column (with `spread` on the stack, 5.3 Idea 7) lets them step 15 to 30 days. It is the only way to about 5 times on the physics without threads | `flow/air/`, `flow/land.rs`, `solve/` | 3 | The key numbers hold (6.3) at the longer step; Flow at least 3 times faster without threads |
| J3 | Speed numbers written by CI on every run, and compared each night | `.github/workflows/ci.yml` | — | A slower change shows up |

### Phase K: recording, saving, viewer (about 30 days)

| ID | What | Files | Level | Done when |
|---|---|---|---|---|
| K1 | **QUERY:** the read-only window; the tape, CLI report and viewer frames use only it | `core/src/query/`, `tape/`, `cli/` | 1 | The tape builds without access to the store's internals |
| K2 | Snapshots: format version kept (D5); only the changes since the last snapshot | `tape/src/snapshot.rs` | 1 | Under 30 MB per snapshot at 128 × 128 |
| K3 | The tape for levels, patches, links and zones; the viewer file carries the mesh | `tape/src/rec/` | — | Every level recorded; `docs/tape.md` updated |
| K4 | The viewer: any mesh (mixed levels, the ball as a map), the link layer (channels, lanes), zones; charts grouped and searchable | `web/src/` | — | You can go from the whole world down to a zone |
| K5 | Time per process, per level and per zone in the tape | `tape/`, `cli/src/report.rs` | — | Speed problems visible in the viewer |

### Phase L: the final check (about 15 days)

| ID | What | Done when |
|---|---|---|
| L1 | A2's reference runs on the new engine | Ranges hold; every larger difference explained |
| L2 | Speed | The goals of 6.5 met, or the real numbers reported |
| L3 | Books | Elements and energy exact over 1,000 years, with threads, zoom, patches and zones |
| L4 | Show worlds: the ball, a zoom, a zone with a chase, a channel that lasts | Each one runs, with pictures |
| L5 | The foundation watch report; then the owner decides M4 | The owner's choice is written in the handover |

---

## 8. Order, parallel work and size

```
A ─► B ─► C ─► D ─► E ─► F ─► G ─► H ─► I ─► L
          (C can start during B)    (H can run with G)
                         J (from E on, beside the others)
                         K (from F on, beside the others)
```

| Phase | Days (rough guess) |
|---|---|
| A prepare | 10 |
| B base layers | 45 |
| C shared maths | 25 |
| D physics laws | 45 |
| E life engine | 50 |
| F split, join, nodes, patches | 45 |
| G shapes and zoom | 40 |
| H links with memory | 20 |
| I focus zones | 45 |
| J machine speed | 30 |
| K recording and viewer | 30 |
| L final check | 15 |
| **Total** | **about 400 days** |

Work like this usually takes longer than planned. Report the real days in each phase report.

---

## 9. Risks and stop rules

| Risk | What to do |
|---|---|
| **The rebuild never ends and nothing is watched** (the version 1 trap: handover 2, lesson 4) | The island runs after every task. A phase more than 50% over its days stops, and the owner decides. |
| Exact books across split, join and zoom | F1's random test is the gate: no later phase starts while it fails. |
| The new engine gives a different world | Level-3 differences are allowed only when the report explains them. A range broken without a reason stops the phase. |
| A speed tool makes the world (nodes, zones, sleep, zoom, patches) | Its equivalence test (6.4) is part of its "done when". |
| The new base becomes one more layer on top of the old code (the opposite of the main rule) | Each phase's "Deleted in this phase" list is part of its "done when". The code-size test (A6) fails above the ceiling. Each phase report gives the lines added and removed per folder. |
| Code size grows past the budget (D2: 8,000 code lines at the end) | Simplify before adding (handover 14.3). A new part past its allowance needs the owner's agreement. The ceiling never justifies a task that costs speed or honesty. |
| Speed work aimed at the wrong half | Each speed task starts from a profile of a living year; the order of Phase E follows it. |
| New tuned numbers creep in | The tuned budget test (`core/tests/numbers.rs`) stays; speed settings go in their own table (D4). |
| Body physics changes sneak in | D6: bug fixes only, each its own branch. |

---

## 10. After the foundation: what M4 still needs

The foundation gives the machinery. M4 then adds the content (handover 13), which becomes much smaller:

- **The planet:** real latitudes, continents and basins on the ball (G2 gives the mesh).
- **Air in 2 or 3 layers** with momentum and the planet's turning; clouds that travel with the wind and keep heat in
  (uses B2 stacks, C1).
- **The sea in 2 layers** with a salt substance (owner's choice), currents from wind, heat and salt, thin ice on top
  (B2, C1).
- **Snow and ice that flow**; a slow carbon source like volcanoes; slow uplift.
- **A burning share of a patch** that spreads along links, with sparks from strong storms booked like the sun (F6, H,
  D1; handover 13.2, "Fire and disease").
- **The mind** (handover 13.1, option 2): memory of places, a short look-ahead, learning in the hidden layer, used in
  zones (I).
- **Long watch runs** on the planet, 1,000 years and more, several seeds, against null runs (handover 18.3).

---

## 11. Words used in this plan

| Word | Meaning |
|---|---|
| mesh | the set of cells and the links between them: the shape of the world |
| level | a zoom level: a cell of one level holds 4 cells of the next |
| stack | the nodes above and below a cell: air layers, soil layers, the deep store |
| patch | a part of a cell's ground with its own soil, liquid, heat and cover |
| zone | a small area taken to the finest level, with its own fast clock |
| move | a change the store applies: amounts and heat from one node to another |
| link flux | what crosses each link for one substance, for a whole field at once |
| boundary node | the sun, space or outside: where energy and matter enter or leave the world |
| implicit step | a way to do a time step that stays stable for any step length |
| columns | each quantity in one array for all nodes of a class (fast for the computer) |
| chunk | a block of about 64 cells that can sleep or run on its own thread |
| fingerprint | one number made from the whole saved state (`tape::fingerprint`); equal fingerprints mean the same history |
| equivalence test | a test that a speed tool gives the same world as running without it |

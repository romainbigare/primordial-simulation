# Engine: threads, numbers and the line ceiling (owner's choices 4, 5, 7)

Design section of the speed x10 plan. Scope: how the bodies' stage runs on many cores, what the books need, which
numbers may be 32-bit, and the code ceiling. The other sections (plants, air, minds) change *what* is computed; this
one changes *how the work is shared out* and makes sure their work can use 8, 16 or more cores.

Code at `f63c06a`. Measures from `docs/speed-post-foundation/measures.md` (sections "D2.4a", "D2.5", "After D2.6",
"After D2", "E2.3 (1)", "Z1") unless named. "4T" = 4 threads, "1T" = 1 thread.

## 0. In short

1. **The one-core time of the bodies is not the price of determinism.** It is the price of one thread doing all the
   writes that cross a domain's edge (landing moves and litters, settling ids and counts, faces, booking). These writes
   can be shared out to the threads **by destination**, with an order fixed by the data (domain, cell, id). Then the
   world stays the same with any number of threads, at no cost. So we adopt the owner's choice 4 as a *permission*,
   but this design does not need to use it.
2. **New batch pipeline:** P1 (updates, per domain, as today) → S1 (a tiny serial step: new ids, outboxes sorted by
   destination) → P2 (per destination domain: land moves and litters, make the next faces; at the same time per store
   array: book the views) → S2 (serial, small: part acts, watches, rare returns, bodies that hold a part).
3. **Domains become tiles** of 4 cells, not one air node (16 cells; 64 in a refined region). A tile takes at most its
   cells' share of their air node in a batch (the "air column"). This fixes the load balance (the busiest domain is
   15.7% / 18.9% / 30.5% of a batch today, which caps any number of cores at 6.4 / 5.3 / 3.3 times on the bodies) and
   it is required by the air section's coarser air grid (else domains would grow to 64 cells).
4. **32-bit books do not pay and would break the books.** Every number that holds matter or energy stays 64-bit. 32-bit
   is allowed for numbers that hold nothing (tables, brain rows, faces, rates) where a measure shows a gain. The data
   of a world is 2 to 5 MB and the last-level cache misses are 0.0% to 0.1% of the cycles, so memory is not the limit.
   The reactor needs 64 bits (exact sums, a series exact to 2e-7, constants beyond 1e38).
5. **Ceiling: 11,500 code lines** for this round (today 9,591 of 9,700). The four sections need about +1,250 to
   +1,600 net; 11,500 is 20% over the old 9,700, the margin handover 14.3 already calls fine with a reason.
6. **Gain of this section alone:** the bodies' stage on 4 cores 16% to 19% faster (a year 11% to 14%). The real value
   is scaling: on 16 cores the bodies' stage is about 2.0 times (island), 2.1 times (globe), 3.1 times (region) faster
   than today's design on 16 cores, which is stuck at about 300 / 350 / 600 ms whatever the cores. After the plants
   section removes most bodies, the serial part would be about 40% of what is left without this section.

---

## 1. What exists today

### 1.1 The batch (`core/src/survey.rs:209`, `batch`)

1. All due ids are taken from every domain's ring (`Life::take_due`, `life/mod.rs:223`), looked up, sorted by
   (domain, cell, id) and split: bodies that hold or are inside a part are "serial bodies" (S52).
2. **Faces** of every living body are made on the main thread (`Faces::make`, `survey.rs:178`).
3. **Parallel phase** (`on_domains`, `survey.rs:245`): the domains are taken out of `World` (`std::mem::take`), sorted
   by work (largest first, Graham 1969), and run on the pool (`par::each`, `par.rs:38`). Each runs its bodies in order
   with its own store view (`store::Moves`, `store/gather.rs`), its own outbox (`outbox::Entry`), provisional lineages
   and born rows (`life/registry.rs:14`, `PROVISIONAL`) and integer deltas of holders.
4. **Serial phase**, on the main thread: `World::book_moves` (`gather.rs:309`, every domain's view added to the store,
   domain by domain), `World::settle` (`life/domain.rs:159`: events and counters in domain order; `Life::remap`,
   `registry.rs:87`, gives global ids and rewrites them in every body of a domain that made one), then
   `outbox::deliver` (`outbox.rs:39`): every entry in global order, each one a serial step (`World::in_domain`,
   `domain.rs:148`, takes one domain out, runs, puts it back and settles it); then the serial bodies, each a serial step
   followed by its own `deliver`; then `Life::count_all` (`registry.rs:101`) and `settle_moved`.
5. The survey (`survey.rs:28`) runs `on_domains` twice per physics step (wear and merge, then shade), each with the
   same booking, settling and delivering, and runs the cells with parts one by one.

What one landing does (`choose::land_move`, `choose/go.rs:49`; `copy::land_litter`, `copy/mod.rs:193`): a kin search
in the target cell, `Life::relocate` (unstore in the source domain, store in the target one, move the ring entry),
pushes parts in the new cell (`bond::moved`), adds to `Counters::crossings[j]` and `Life::pushed[j]` (two world-wide
vectors), clears a lane (`lanes::clear`, writes `Links::cleared[link]`), and maybe a merge (`nodes::merge`). A litter
joins a node (`nodes::add`) or is placed (`place_child`, an id from the target domain's counter), or, if it is less than
one member and finds no kin, goes back to its parent's brood (a body in any domain) or to the ground where its parent
was.

### 1.2 Where the one-core time is (measured)

| | island22 | globe36 | region24 |
|---|---|---|---|
| bodies' stage, 1T / 4T, ms a year (Z1) | 857 / 421 | 854 / 456 | 1,220 / 582 |
| share of the 4T stage on one thread (Z1) | 32% | 38% | 30% |
| so: one-thread time S at 4T, ms a year | 135 | 173 | 175 |
| the parallel part: 1T work, 4T time, speed-up | 722, 286, 2.52 | 681, 283, 2.41 | 1,045, 407, 2.57 |
| busiest domain's share of a batch (Z1) | 0.157 | 0.189 | 0.305 |
| a domain's work on 4T against 1T (D2.5) | +27% | +21% | +28% |

What S is made of (probe of "After D2.6", 4T; booking scaled by E2.3 (1), which cut it from 418 to 70 million
instructions on the island):

| Part of S | island22 | globe36 | region24 | Why it is serial today |
|---|---|---|---|---|
| landing the outboxes and settling (`deliver`, `settle`) | ~51% | ~48% | ~56% | it writes other domains' storage and world-wide vectors |
| booking (`book_moves`) | ~3% | ~2% | ~4% | one writer for the store (a parallel probe was 6% slower, S74: false sharing) |
| faces and the list of due bodies | ~17% | ~11% | ~29% | built for the whole world before any update |
| the rest: serial bodies (parts), the survey's serial cells and loops | ~30% | ~38% | ~10% | the parts registry is global (S23, S52) |

In instructions (Z1, one year, 1T) the serial pieces are small: `outbox::deliver` 3.5% / 5.1% / 3.1% of the year,
faces, booking and settling 2.5% / 2.3% / 2.3%. They cost much more wall time than their instructions, because the
main thread then touches every domain's data cold (the worst data misses of the update are in `outbox::deliver` and
`nodes::add`, "After D2", E1 gate), and the three other cores wait.

### 1.3 Which mechanisms are there because of bit-for-bit order

| Mechanism | File | Is a fixed order the cost? | Fate |
|---|---|---|---|
| outbox and serial delivery | `outbox.rs:39`, `go.rs:49`, `copy/mod.rs:193` | **No.** The cost is that one thread writes everything. Each destination can land its own entries; an order per destination (source domain, then order made) is free | P2, by destination |
| `World::settle`, `Life::remap`, `count_all` | `domain.rs:159`, `registry.rs:87,101` | **Barely.** Giving global ids in domain order is a short loop; rewriting ids in bodies can be per domain; integer counts commute | S1 (ids), P2 (rewrite) |
| `in_domain` for every serial step | `domain.rs:148` | No: it exists because entries touch any domain | only in S2, for the rare entries |
| fixed-order booking (`Moves::book`, `book_moves`) | `gather.rs:255,309` | **No.** Each store number has one writer (its domain). The probe was slow because threads wrote neighbouring cells of one array | P2, one task per store array |
| provisional ids and remap | `registry.rs` | No: the registries are read by all domains, so new entries wait. The ids stay deterministic | kept, rewrite in parallel |
| `par::rows` fixed-order sums | `par.rs:17` | **No.** The rows are already made in parallel; the serial part is `store::apply` writing the store | unchanged; E4 (apply by array) |
| survey's `on_domains` (2 rounds a survey) | `survey.rs:245` | same as the batch | same pipeline |
| faces and the due list | `survey.rs:178,209` | No | per domain |
| the parts' serial path | `survey.rs:236`, `bond/month.rs:12` | No: one writer per structure would do | E5 (gated) |

**Finding:** nothing above needs the order to depend on the threads to run in parallel. A data-fixed order costs at
most a sort of small lists. So "accept a different history with threads" frees no measurable time in today's code.
Where it would really pay (concurrent takes inside one crowded cell, section 2.6) is kept as a reserve.

### 1.4 Numbers today

- Store: `Store::amounts`, `heat`, `carry` (`store/mod.rs:25`), f64, `[k * len + i]` per class (substance-major).
  Ground and air amounts carry their rounding (Neumaier 1974, `apply.rs`, `add_carried`). 26 substances (chemistry 0,
  `docs/laws.md`), 648 cells on globe36: about 0.7 MB.
- Bodies: `Body::store: Vec<f64>`, `e`, `count`, `brood`, `last`, `mark`, `rained` f64; genes, spreads, brain rows,
  `vis`, `sig` already f32 (`body/mod.rs:52`). About 0.7 KB a body, 1.5 to 3.5 MB for 2,361 to 4,857 bodies.
- The reactor (`change/kernel.rs`) sums some changes exactly (`CANCEL`), uses a series exact to 2e-7 below 1e-3
  (`SERIES_BELOW`), and rate constants over a wide range.
- Books: `check::drift` (`check.rs:93`), global, Neumaier sums; drift in 1,000-year runs 5.6e-16 (elements) and
  7.3e-14 (energy), limit 1e-12.

---

## 2. The new design

### 2.1 One writer for each number

The rule that replaces "one thread does the crossing writes": **in a parallel phase, each number of the world has one
task that may write it.** A domain (tile) writes its bodies, its cells' lists, its ring, its view, its outbox, the
per-cell numbers of its cells. A booker writes one array of the store. Everything a task does to another task's
numbers waits in its outbox and lands in the next phase, in the task that owns the target, in an order fixed by the
data. So the world does not depend on how many threads ran or which ended first.

### 2.2 The batch pipeline

```
batch (every batch_days):
  P1  per domain, on the pool        take own due ids from own ring, sort by (cell, id), run the updates (as today)
                                     at the end: take each leaving body out of own storage into its Go entry,
                                     and sort own outbox into buckets by destination domain
  S1  serial, short                  give global ids to the lineages and born rows made in P1 (domain order, then
                                     order made: as today); hand each destination its buckets (source order)
  P2  on the pool, two kinds of task
      - per destination domain       rewrite provisional ids in own bodies and incoming entries (S1's table);
                                     land own Go and Litter entries (source order, then order made);
                                     make own faces for the next batch
      - per store array              book every domain's view into that array (Ground/Bed/Bodies/Air amounts per
                                     substance, and each heat array)
  S2  serial, small                  ids made in P2; events and counters merged in domain order; holder counts
                                     (`count_all`); `away` table; then the entries that touch shared things
                                     (part acts, watches, litters going back to a parent or the ground, lane clears,
                                     moves into a cell with parts), then the serial bodies, as today
```

Survey: `on_domains` runs its work as P1, then S1, P2, S2. One function for both (one place for the idea).

**Why P2 may book while it lands.** A Go or a Litter landing gathers no move of the store: the body carries its stores
by value; a merge moves matter between two bodies (both sides bodies). The entries that move matter into the store (a
litter's remains to the ground, `body::remains`) go to S2. So landing tasks write bodies and lists, booking tasks write
store arrays: disjoint memory, one pool call. The views are taken out of the domains before P2 (`Vec<Moves>`), so the
compiler checks it. A debug check holds that no landing opens a view.

**Booking by array.** Each booking task owns one array, for example "Ground amounts of substance k" (the numbers
`[k * n + i]` for all cells i). It walks the domains in order and adds each held cell's change (`add_carried` for
ground and air, `zero_rounding` as today). Different tasks write different arrays, so no two threads share a cache
line except at an array's end. The lift into the potential and the heat a ground node could not give are summed per
domain and added in domain order in S2 (as today). `counts.rounded` is summed per array and added in array order.
About 4 × 26 + 4 = 108 small jobs, grouped into chunks of a few arrays.

**What stays ordered (and is free):** the order of updates in a domain (cell, id); the order of landing in a
destination (source domain, then order made); the order of ids given in S1 (domain, then order made); the order of
events and counters (domain order); the S2 entries (domain order, then order made).

**Faces made at the end of landing.** A domain's faces for the next batch are made at the end of its P2 task, from its
bodies as they then are. A domain changed later (S2, a survey's serial cell, an edit through `World::add`) marks its
faces stale and they are made again before the next batch's P1 (usually one or two domains, on the main thread).
Flow writes only `draw.owed` of bodies (`survey/draws.rs:114`), no number of a face.

**Small phases run on the calling thread.** A pool call costs 30 to 55 µs (D3). A P1 or P2 with less work than
`SERIAL_BELOW` updates (a named `const` beside `par::each`, about 8) runs on the calling thread. The results are the
same by construction. This matters once the plants section removes most bodies.

**Scratch per thread, not per domain.** With tiles there are about 4 times as many tasks. `par::each` becomes
`each_init` (rayon's `for_each_init`): a `Reactor`, `LiveTmp` and `ActTmp` are made once per thread and reused, not
once per domain and batch (`survey.rs:233` today).

### 2.3 Data structures

| What | Today | New |
|---|---|---|
| `Domain` (`life/domain.rs:14`) | the cells under one air node | the cells of one **tile** (`TILE_CELLS` = 4 cells of one air node, in `Level::children` order; a named `const` in `life/mod.rs` with a one-line doc) |
| `Domain::outbox` | `out: Vec<Entry>` | `out` as now while P1 runs; at P1's end `post: Vec<(u32, Vec<Entry>)>` (destination, entries in order made) |
| `Entry::Go` (`outbox.rs:13`) | body id | the `Body` itself, taken out of the source's storage at P1's end (source unstores it and takes its ring entry out) |
| incoming entries | — | `Domain::inbox: Vec<Entry>`, filled in S1 in source order |
| `Life::pushed`, `Counters::crossings` | world-wide `Vec<f64>`, `Vec<f32>` per cell | per domain, per own cell (`Domain::pushed`, `Domain::crossings`); `traffic` reads them monthly |
| `Faces` (`survey.rs:162`) | one list for the world, built serially | per domain: `(Vec<Face>, Vec<u32>)` built in P2 and moved into `Life::faces` (no copy); `Faces::of(i)` reads the cell's domain |
| `Moves` (view) | the domain's air node whole | the tile's **air column** (2.4) |
| `book_moves` | loops over every domain | books only the domains opened since the last booking (a list), also in the parts' monthly step (`bond/month.rs:39` calls it once per part) |
| `deliver` | takes every domain's outbox | delivers the outbox of the domain just stepped (S2 steps) |
| ids | body ids per domain counter; lineage and born-row ids provisional, remapped | unchanged; S1 gives global ids, P2 rewrites them per domain (in parallel) |
| `away` (id to storage domain) | written during `relocate` | each destination lists its arrivals in P2; merged in S2 (sorted) |
| `Links::cleared` | written by `lanes::clear` at landing | lane clears go to S2 as a list (a link joins two cells that may be in two tiles) |

Snapshot: the domains (tiles) are saved as today (`Saved`, `life/mod.rs:85`), with their `pushed` and `crossings`;
`SNAPSHOT_VERSION` is bumped in E1a and E2. Tape: `Counters::busiest_share` becomes the busiest tile's share
(`docs/tape.md`). Viewer: nothing changes.

### 2.4 Tiles and the air column

**Why tiles.** The busiest domain bounds each batch. Today a domain is an air node (S17) because every update touches
its air node, and only one task may write it. Tiles of 4 cells give about 121 tasks on island22 (484 cells), 162 on
globe36 (648) and 192 on region24 (768), against about 36, 45 and 36 air nodes of 4 × 4 ground cells. The refined
region's air nodes over the refined area hold 64 cells each, which is why its busiest domain is 30% of a batch.

**The air column (law change, physical reason).** In a batch, a tile's bodies breathe the air above their own cells,
not the air above all 16 (or 64) cells of the air node. A tile's view takes, when it opens:

- of each substance k of air node j: `A_T,k = s_T × A_j,k`, with `s_T` the sum of `Mesh::share(i, AIR)` over the
  tile's cells (`space/mod.rs:210`; the parts' monthly reaction already uses this share, `bond/month.rs:57`);
- of the air node's heat: `s_T × E_j`;
- what bodies see (`a_seen`): `s_T × (A_j,k + sum of the bodies' nodes under j)`, then its own changes.

Takes from the column are bounded by the column (the debug check in `Moves::alter`, `gather.rs:65`, reads the view);
gives add to it. At booking, the air node's change is the sum of its tiles' changes, in tile order (one booking task
owns the air array). As the shares sum to one, the tiles together can never take more than the node holds: no order
of tasks can overdraw it. The temperature and the gas shares a body reads are ratios (`flow::air_temp`, `gas_pool`),
the same at the batch's start as today; within a batch they move by the tile's own changes only.

Is this honest? The air over one cell is its column; winds mix the node's air within each air step (every 10 days).
For the main gases the column is far more than a batch's breathing, so nothing changes. For a scarce gas, a crowded
cell can now take at most its column's share in one batch (a sixteenth on a 4 × 4 node), and the rest of the node's
gas is left to the other tiles. That is more local, not less. Level 3, checked by the key numbers.

**Tile size.** `TILE_CELLS = 4`, chunks of each air node's children list (2 × 2 on a square grid; rows of 4 if the
children are listed by rows: any chunk works, the column share is exact either way). One cell per tile would cut
the busiest share more but makes about 1 update per task per batch (island22 has about 180 updates per batch over 484
cells): the per-task cost (opening a view, scratch) would grow. The gate E0(d) measures the busiest share for 1, 4
and 16 cells from the bench saves before E2 is built.

### 2.5 The parts (gated, E5)

Today a body that holds or is inside a part runs in the serial phase (S52; 8.5% of the updates on globe36 at year 200),
and every act on a part is a serial entry. After the plants section, plants are no longer bodies, so this share of the
remaining updates could rise 2 to 3 times (to 20% or more on globe36). Then it is the main serial cost.

Design if its gate passes: a **structure** (a root and the parts bonded to it) is **local** when all its parts lie in
the cells of one tile. For a batch, each tile owns the local structures of its cells: their parts are taken out of the
world's registry into the tile by slot (as bodies are), before P1, and put back in P2. The bodies that hold or are
inside a local part run in P1 in their tile, with immediate exchanges as today. FORM in P1 makes a part in the tile with
a provisional part id, given a global id in S1 like a lineage. Structures that span tiles keep today's serial path.
Bond's monthly step (`bond/month.rs:12`) runs the local parts per tile on the pool. Order: by the tile and the part's
slot, so it is fixed by the data. This revisits S23 and S52 ("parts owned per domain" was refused for the part ids that
change domain; provisional ids now exist for lineages and born rows, the same mechanism serves).

### 2.6 The permission to let the history depend on the threads (choice 4)

We take the owner's choice as a rule with a price: a step may make the history depend on the number of threads or on
their timing only if it gains at least 5% of a year on 4 cores and no fixed-order design gets that gain (logged in
`decisions.md`). Nothing in this section uses it. Where it could pay later:

- **Takes inside one crowded cell.** If a cell holds hundreds of bodies (623 in one globe cell at year 410), its bodies
  could run on several threads, each take from the cell's nodes an atomic compare-and-swap clamped to what the node
  holds (no overdraw; the same number added to the body as taken from the node, so the books hold). The order of takes
  would then depend on timing. Built only if a crowded cell is still the busiest task after the plants section.
- **The air's solves** (air section): an iterative solver whose sums follow the work split. Note: a fixed-shape
  reduction (fixed chunk size) gives the same speed with a fixed order; a level-2 re-pin already allows a changed order.

**Reproducibility.** With this design the same seed, code and machine give the same world with 1, 4 or 64 threads.
That is kept while it is free, not built as proof machinery (handover 14.5).

### 2.7 Numbers: what "exact books per cell and per month" means (choice 5)

Precisely, for this design:

1. **Per move:** every transfer books both sides with one f64 number (as today). The precision of the *rate* that made
   the move does not matter for the books: a move worked out in f32 is widened to f64 once and booked on both sides.
2. **Per cell:** each number of a cell is written by one task per phase (2.1), so no update is lost and no sum
   depends on timing. A tile's takes from a shared node are bounded by its share (2.4).
3. **Per month:** the golden test checks the books at every month's end (today: at the end of its 2 years), limit
   1e-12, and a run checks them at its snapshot lines (as today).

**Why held numbers stay 64-bit.** A 32-bit float keeps 24 bits: adding a body's move to a ground node holding
10^6 times more loses the move whole, and each add rounds by up to 6e-8 of the node. The limit is 1e-12 over 1,000
years. An exact 32-bit scheme would need 64-bit accumulators beside every 32-bit number: more memory, not less.
Integer (fixed-point) books would be exact and order-free, but this world spans about 1e-9 mol (a trace a small body
takes) to more than 1e18 mol (an air node's main gas): 27 decades, more than a 64-bit integer holds (19 decades); the
`carry` of ground and air amounts exists for exactly this range. 128-bit integers would work but double the memory and
slow every add.

**Where 32-bit could pay, and what to expect:**

| Candidate | Holds books? | Expected | Verdict |
|---|---|---|---|
| store amounts and heat; a body's stores, heat, members, brood; the views' changes | yes | breaks the books | stays f64 |
| the reactor's internals (`change/kernel.rs`) | no | exact sums (`CANCEL`), a series exact to 2e-7 (f32's step is 1.2e-7), rate constants beyond 1e38 overflow f32 | stays f64 (f32 harms) |
| libm `pow`, `exp`, `expm1` in body code (1.54 + 1.45 + 1.07% of the island's instructions, Z1) | no | `powf`/`expf` take about half the instructions: at most about 2% of a year | probe E6 on a branch; kill line 2% |
| memory and cache | — | data 2 to 5 MB; last-level misses 0.0% to 0.1% of the update's cycles; L1 data misses 9.7% to 10.3% (E1 gate); a probe of the body layout gained nothing (S82) | no gain to expect |
| SIMD | — | the hot code branches per body; no long loop of one operation over many numbers but the reactor rows (already parallel) | no gain to expect |
| growth tables (plants), brain rows and batched networks (minds), Flow coefficients (air) | no | memory per table, wider SIMD for matrix products | allowed, each section measures |

So the honest version of choice 5: **books stay 64-bit; 32-bit is a tool for numbers that hold nothing, used where a
section measures a gain.** A per-cell ledger of every element (a monthly check of each cell's change against what
crossed its edges) would be a stronger check, not a speed tool; it would cost an add per element per move. Not
proposed; possible later as a debug check.

### 2.8 `store::apply` on the pool (gated, E4)

`store::apply` (`apply.rs:165`) books Flow's and Change's link fluxes, moves and transforms on the main thread: 4.7%
to 5.2% of a year's instructions (Z1, inclusive). It can run per array on the pool with the order of changes of each
number kept: cut the arrays by substance (`[k * len .. (k + 1) * len]` are disjoint), each task applies the changes
that fall in its range in their given order, and `low` and the debug check per task. Same order per number: level 1,
golden unchanged. It is the same idea as booking by array (2.2), so the two share one helper. Built after the air
section's steps, if `apply` is still at least 3% of a year on the main thread at 4T.

---

## 3. The law changes in physical terms

| Change | What a body or cell does differently | Why it is still honest |
|---|---|---|
| Landing by destination (E1) | Part acts, watches and returns of litters are applied after all moves and litters of the batch have landed (today: mixed, in the order of the source domains). Events of a batch come in another order | every entry is still applied once, by the same law, on the bodies as they then are; books by the same moves |
| Booking by array (E1) | the store gets each number's change once, as today; only the order of the rounding counter's sum changes | one writer per number; `add_carried` and `zero_rounding` as today |
| The air column (E2) | in a batch, a tile's bodies take at most their cells' share of their air node, and see the air's gas shares and temperature of the node at the batch's start moved by their own tile only | the tiles' shares sum to one: no overdraw whatever the order; every take is booked both sides |
| Tiles (E2) | body ids carry the tile (ids change; random draws are keyed by ids, so histories change) | ids stay unique (tile index and its count, `life/mod.rs:28`) |
| Parts per tile (E5, gated) | bodies that hold a part in a local structure act in the parallel phase, at once on the part | one writer per structure; parts of spanning structures stay serial |

---

## 4. Which rules change, and the exact new wording

### 4.1 `CLAUDE.md` section 2, item 11 (the ceiling)

> 11. **Size.** `core` holds at most 11,500 lines of code (`docs/speed-x10/decisions.md`), not counting comments and
>     blank lines (`docs/handover.md` 14.3; the test `cli/tests/core_size.rs` counts them). Simplify before you add.
>     Never delete useful comments to save lines: they are not counted. Never pick a slower or less honest design to
>     save lines. Each step reports its lines; the round aims to end at or below 11,200, and at its end the ceiling is
>     set to the lines then plus 200.

(The folder name `docs/speed-x10/` is the architect's choice.)

### 4.2 `CLAUDE.md` section 3 (new text; items 2, 3, 5 unchanged)

> 1. **Everything is kept.** Each element, the energy and the members of a node. Every transfer books both sides
>    through a shared helper. A drift above about 1e-12 is a bug: find it, do not hide it. Every number that holds
>    matter, energy or members (the store, bodies, parts, plant fields) is a 64-bit float.
> 4. **A law never overdraws a node.** It never asks a node for more than it holds; where it could, it limits that
>    node's own outgoing links, never a whole field. A task that shares a node with other tasks takes at most its share
>    of it (a tile takes at most its cells' share of their air node, `Mesh::share`), so no order of the tasks can
>    overdraw it. `store::apply` only adds, sets rounding below zero to zero and counts it. A real overdraw breaks the
>    books.
> 6. **One writer for each number.** In a parallel phase each number has one task that may write it: a tile its
>    bodies, its cells and its view; a booker one array of the store. What a task does to another task's numbers waits
>    in its outbox and lands in the next phase, in the task that owns it, in an order fixed by the data (tile, cell,
>    id), never by which thread ends first.
> 7. **32-bit numbers hold nothing.** A 32-bit float may be a rate, a table, a brain row, a face or a picture, never
>    an amount, a heat or a count of members. A move worked out in 32 bits is booked on both sides as one 64-bit
>    number. Use 32 bits only where a measure shows a gain.

### 4.3 `CLAUDE.md` section 6, item 1

> 1. **A pure clean-up keeps the results bit for bit.** Check the golden test, and compare a few short runs (same
>    fingerprint, same `series.csv`). Keep the order of floating-point sums; `a*b*c` is not `a*(b*c)`.
>    **Speed work may change the order of sums** (check level 2 of `docs/foundation/plan.md` 6.1): it re-pins the
>    golden test and shows the same world (a few series columns, or the key numbers where life's choices amplify
>    rounding). **The number of threads does not change the world:** every order is fixed by the data. A step may give
>    this up only by a decision in `decisions.md`, when it gains at least 5% of a year on 4 cores and no fixed-order
>    design gets that gain; the golden test then runs on one thread, and runs with threads are checked by the key
>    numbers.

Item 3 ("Before you push...") is unchanged; section 5.3 keeps "the golden test also without threads".

*Fallback wording, if the owner wants the weaker promise now:* "The same seed gives the same world with the same
number of threads; runs with different numbers of threads may differ. The golden test runs on one thread."

### 4.4 `docs/handover.md`

**12.1 Numbers** (replaces the first bullet):

> - Every number that holds matter, energy or members (the store's amounts and heat, a body's stores and heat, a
>   part's walls and fill, a plant field) is f64: these are the books. Numbers that hold nothing (rates, tables worked
>   out once, brain rows, what a body shows others, pictures) may be f32 where a measure shows a gain; a move worked out
>   from them is booked on both sides as one f64 number.

**12.2** adds one sentence: "The golden test checks the books at every month's end."

**12.3**, the repeatability bullet:

> - **Repeatability:** run the same seed twice on the same machine, with any number of threads, and the state must be
>   the same, bit for bit.

**12.5 Repeatability** (first bullet):

> - Same seed, same code version and same machine/build give the same history, with any number of threads. (A decision
>   may narrow this to the same number of threads, `CLAUDE.md` 6.1.)

**14.3 Code budgets**, the `core` row and the counting paragraph: "≤ 11,500 lines of code through M7
(`docs/speed-x10/decisions.md`)" and "fails above its ceiling: 11,500 lines".

**14.5 Do not**, one bullet added:

> - Do not let a law read which thread ran first. A law reads data; the order of work is fixed by the data.

### 4.5 Decisions of the speed work that change

| Decision | Change |
|---|---|
| S3 (ceiling 9,700) | replaced by 11,500 (X-ceiling) |
| S17 (a domain is one air node) | replaced: a domain is a tile of `TILE_CELLS` cells of one air node, with an air column (E2) |
| S4, S22 (outbox, one path for every relocation) | kept: every relocation still lands at the batch's end; now by destination, in P2 |
| S23, S52 (parts serial) | amended by E5 if its gate passes (local structures per tile) |
| S24, S11 (provisional ids, remap in a fixed order) | kept; the rewrite runs per domain in P2 |
| S42 (order test, reversed order) | kept and widened: P1 and P2 reversed, and the booking jobs reversed |
| S74, S79 (booking in the task; landing own moves in the task: not built) | replaced: booking by array in P2; landing by destination in P2 (one path) |
| S64 (`batch_days` 2) | unchanged |

---

## 5. How we check it

**Check levels.** E1a: level 1 in the series (the snapshot format changes, so the fingerprint is re-pinned; the
series of two years from each bench bit for bit). E1b, E1c: level 3 (the order of S2 entries and of events changes),
expected to be nearly level 1 for the store (each destination lands in the same order as before). E2: level 3 (a law
change). E4: level 1. E5: level 3. E6: on a branch only.

**"The same world" now.** For E1: two years from each of the three bench saves give the same series columns, or
differ only within the run-to-run noise of the key numbers where an S2 entry acted. For E2 and E5: the key-number test
(`tape/tests/ranges.rs`) once, all numbers in their ranges as before (S50 and S84 stay known failures); land
temperature, gas ppm, light-eater and eater mass, bodies per cell at year 30 within seed noise.

**Tests.**

- **The order test** (`core/tests/outbox.rs:221`) stays an equality test: pools of 1 and 4 threads, P1's and P2's
  task orders reversed, the booking jobs reversed; the four worlds equal (saved world, events, counters). Its scenario
  gains: two litters from two tiles landing in one cell (fixed order by source), a move that merges on landing and
  makes new born rows in P2, a watch and a part act in S2, a tile next to a crowded tile under one air node (the
  column bound holds: the air node never below zero).
- **The golden test** (`tape/tests/golden.rs`): same form; re-pinned in E1b, E1c, E2 (and E5); the same with and
  without threads; books checked at each month's end (`Books::assert_kept` per month, 1e-12).
- **The drift test** (the century, `a_century_keeps_its_books`) unchanged.
- **No new test file.** One assertion in the store tests: a tile's view holds `s_T` of its air node when it opens.
- Delete the tests of mechanisms deleted: none expected (the serial `deliver` loop stays for S2).

**Key numbers that must hold:** books 1e-12 (elements and energy) in the golden worlds every month and in the century;
the order test's four worlds equal; key numbers in range.

---

## 6. Expected gain (arithmetic from the measures)

### 6.1 On 4 cores

Moved off the one thread by E1: landing (85% of it; S2 keeps the rest), booking, faces and the due list: 63% of S on
island22, 54% on globe36, 81% on region24. Left serial (S_res): 50, 80 and 33 ms a year. The moved work runs at the
parallel part's speed-up (2.52, 2.41, 2.57). One more pool call per batch and per survey round: about
(180 + 72) × 45 µs ≈ 11 ms a year.

| ms a year, bodies' stage, 4T | island22 | globe36 | region24 |
|---|---|---|---|
| today (Z1) | 421 | 456 | 582 |
| E1: 286 + moved/speed-up + S_res + 11 | 286 + 34 + 50 + 11 = **381** (-9.5%) | 283 + 39 + 80 + 11 = **413** (-9.4%) | 407 + 55 + 33 + 11 = **506** (-13%) |
| E1 + E2 (imbalance 1.27 to 1.15; region no longer bound by its busiest domain) | **351** (-17%) | **383** (-16%) | **471** (-19%) |
| a year, today → E1 + E2 | 614 → 544 (1.13x) | 738 → 665 (1.11x) | 888 → 777 (1.14x) |

Honest range: E1 -5% to -12%; E2 -3% to -10% on island22 and globe36 (depends on how spread the work is inside a busy
air node, E0(d)), -8% to -20% on region24. Tiles cost: about 4 times the view openings (each takes its air column:
26 numbers, plus the bodies' nodes under the air node for `a_seen`) and more tasks; kept small by scratch per thread.

### 6.2 On more cores

Model: bodies' stage T(P) = S + W × c(P) × max(imb / P, b), with W the parallel work at 1T, c(P) the cost of
threads sharing memory (measured 1.21 to 1.28 at 4T; 1.4, 1.6 and 2.0 at 8, 16, 64 are guesses, E7 measures what it
can), imb the imbalance and overhead of a batch (1.27 fits today's 4T times), b the busiest task's share. Today:
S = 135 / 173 / 175, W = 722 / 681 / 1,045, b = 0.157 / 0.189 / 0.305. New: S_res + 11, W plus the moved work,
imb 1.15, b with tiles estimated 0.06 / 0.07 / 0.08 (E0(d) measures it).

| ms a year, bodies' stage | P = 4 | 8 | 16 | 64 |
|---|---|---|---|---|
| island22 today's design | 421 | 296 | 316 | 362 |
| island22 new | 351 | 224 | **154** | 158 |
| globe36 today's design | 443 | 353 | 379 | 430 |
| globe36 new | 369 | 247 | **180** | 199 |
| region24 today's design | 573 | 621 | ~640 | ~700 |
| region24 new | 471 | 283 | **196** | 234 |

- **Today's design stops at about 4 to 6 cores.** The busiest domain bounds each batch: 1 / b = 6.4, 5.3 and 3.3
  times on the parallel part at best, and the serial S does not shrink. More cores even cost (memory sharing).
- **The new design scales to about 16 cores** for today's worlds (2.0 to 3.1 times today's design on 16 cores). At 64
  cores the busiest tile and S_res bound it (Amdahl: island S_res 61 of 868 ms of 1T work is 7%, a ceiling of 14
  times over 1T before memory sharing).
- **After the plants section** (bodies' parallel work cut by about 50 to 65%, and the island's landing too, since
  plant litters are most of it, D2.4a): for example island22 W ≈ 320, S_res ≈ 40 ms; P = 4: 40 + 320 × 1.25 × 0.2875
  ≈ 155 ms; P = 16: about 77 ms. Without E1, S ≈ 80 ms would be over half of the stage on 4 cores.
- **The whole year** also needs Flow: today Flow and Change gain only 1.15 to 1.2 times from 4 cores (130 against 149
  ms on the island). With this section alone, a year on 16 cores would be about 176 + 154 + 18 ≈ 350 ms on island22
  (1.8 times today's 4T year). The air section decides the rest; E4 helps its booking.

---

## 7. Line cost

| Step | Adds | Deletes | Net | What it deletes or replaces |
|---|---|---|---|---|
| E1a (per-domain cell numbers, booking and delivering only what was touched) | ~25 | ~15 | +10 | world-wide `pushed`/`crossings` writes; loops over all domains in `book_moves` and `take_queued` |
| E1b (P1 end, S1, P2 landing, S2, faces per domain) | ~110 | ~45 | +65 | `Faces::make`'s world loop, the global due list, the serial `deliver` of all entries (it serves S2 only), `in_domain` per landing |
| E1c (booking by array, scratch per thread, serial below a size) | ~45 | ~25 | +20 | `Moves::book`'s per-domain walk (becomes per array), scratch per domain in `batch` |
| E2 (tiles, air column) | ~30 | ~5 | +25 | the domain = air node assumption in `Life::new`, `Moves::open` |
| E4 (apply by array, gated) | ~40 | ~10 | +30 | shares the booking-by-array helper |
| E5 (parts per tile, gated) | ~180 | ~30 | +150 | serial path for local structures |
| **total** | | | **+120 (E1, E2)**, up to +300 with E4 and E5 | |

---

## 8. Risks and answers

| Risk | Answer |
|---|---|
| P2's extra pool call costs more than it saves on small worlds | `SERIAL_BELOW`: small phases run on the calling thread; kill line on the three benches |
| Booking by array is slow (each task reads every view, strided) | 108 jobs grouped by chunks; if slower than serial, book serially in S2 (it is 3% of S) and keep the rest of E1 |
| A landing secretly gathers a store move | the views are out of the domains in P2: the code cannot; a debug assertion names the entry |
| An S2 entry reordered changes the world more than noise | the key numbers once (level 3); S2 entries are a few per batch (E0(a) counts them) |
| The air column starves a crowded tile of a scarce gas | that is the physical column; key numbers; E0 reports how often a body's gas pool is bound by the column on the benches |
| Tiles add per-task costs (view openings, rings of 384 slots per domain: 9 KB each) | E0(d) gates E2; tile of 4 not 1; scratch per thread |
| The busiest task is one crowded cell (623 bodies on the globe at year 410) | no tile splits a cell; the plants section should remove it; the reserve (2.6) is a decision with a price |
| Memory sharing grows with cores (c(P) unknown above 4) | E7 measures what the machine allows; the table says "guess" |
| Determinism breaks by accident (a map iterated by hash, an atomic) | the order test with pools 1 and 4 and reversed orders; no `HashMap` iteration in body code (today: `BTreeMap`) |
| f32 used for a book by a section | rule 3.7; reviews check it |

---

## 9. Steps

| ID | What | Level | Done when | Kill line | Depends on | Parallel with | Agent |
|---|---|---|---|---|---|---|---|
| X-E3 | Rules and ceiling: `CLAUDE.md` 2.11, 3, 6.1; handover 12.1, 12.2, 12.3, 12.5, 14.3, 14.5; `cli/tests/core_size.rs` `CEILING` 11,500; decisions logged | 1 | tests pass; the owner approved the wording | — | the owner's yes | E0, all others | Sonnet |
| X-E0 | Measures (no code on main): (a) outbox entries per batch by kind, and which would be S2; (b) watches and part acts per batch; (c) share of updates by serial bodies on the three benches; (d) the busiest share per batch for tiles of 1, 4, 16 cells, from the saves; (e) a timer build (branch): P1, deliver, settle, booking, faces and list, serial bodies, survey serial parts at 4T; (f) a domain's work at 1, 2, 3, 4 threads (c(P)) | — | a table in `measures.md` | — | — | X-E3, plants and minds work | Haiku (a-d, f), Sonnet (e) |
| X-E1a | `pushed`, `crossings` per domain; `book_moves` and `deliver` only over the domains touched | 1 (series bit for bit; fingerprint re-pinned for the snapshot format) | golden with and without threads; 2 years of each bench same `series.csv` | parts stage or bodies' stage not slower (instructions) | X-E3 | X-E0 | Sonnet |
| X-E1b | The pipeline: P1 end (leaving bodies out, buckets), S1, P2 landing by destination, S2, faces per domain; survey on the same pipeline | 3 | order test (widened) equal; golden re-pinned, same without threads; key numbers once; books each month | bodies' stage at 4T -6% on two benches and not worse on the third (7 interleaved runs, medians) | X-E1a, X-E0 (a, b) | plants step 1 (other files) | Opus |
| X-E1c | Booking by array inside P2's call; per-domain partial sums in order; `each_init` scratch per thread; `SERIAL_BELOW` | 2 | as E1b | bodies' stage at 4T -2% on top of E1b on two benches | X-E1b | — | Sonnet |
| X-E2 | Tiles of `TILE_CELLS` = 4 with the air column | 3 | order test; golden re-pinned; key numbers once; per-update instructions on 1T at most +3% | gate: E0(d) busiest share at 4 cells ≤ 0.6 × today's on two benches; kill: bodies' stage at 4T -5% on two benches or -8% on region24 | X-E1c; the air section's grid step if it lands first (tiles must cut the new air nodes) | minds steps | Opus |
| X-E4 | `store::apply` per array on the pool, order per number kept | 1 | golden unchanged with and without threads | gate: `apply` on the main thread ≥ 3% of a year at 4T after the air section; kill: year -2% at 4T | X-E1c, air section | X-E5 | Sonnet |
| X-E5 | Parts per tile for local structures | 3 | order test with parts; golden re-pinned; key numbers once | gate: after the plants section, serial-body updates ≥ 10% on a bench or S2 ≥ 15% of the 4T bodies' stage; kill: bodies' stage at 4T -8% on globe36 | X-E2, plants section | X-E4 | Opus |
| X-E6 | f32 probe (branch only): `powf`/`expf` for gene factors in `body::live`, `copy::grow`; books untouched | — | a row in `measures.md` | 2% of a year's instructions on two benches, else not merged | — | anything | Sonnet |
| X-E7 | Scaling check: 1, 2, 4 threads (and 8, 16 if a larger machine is available) on the three benches after X-E2; the table of 6.2 with measures | — | a table in the report | — | X-E2 | — | Haiku |

Order: X-E3 and X-E0 first (in parallel); X-E1a, X-E1b, X-E1c in a row; X-E2 after the air section's grid change if
that comes early (else before it, and the air section keeps tiles inside its new nodes); X-E4 and X-E5 at the end, each
behind its gate; X-E6 any time; X-E7 last.

---

## 10. Interactions with the other sections

**What I give them**

- **All:** the pipeline (P1, S1, P2, S2) for any per-tile work; one writer per number; ids that stay unique (body ids
  per tile counter; provisional ids for registries); the order test as the check that a new parallel piece did not
  make the world depend on threads; the 11,500 ceiling with an allotment (below).
- **Plants:** a plant field per cell is a set of numbers that hold matter: f64, booked like the store (a store class
  or a field owned by the tile), never f32. Its growth table per genome may be f32 (it holds nothing). The field's own
  update can run per tile in P1 (before the animals of the tile, so they eat the new growth in order) or per cell in a
  `par::rows` pass like Change. Seed spread between cells becomes a flux between neighbour cells (a link flux booked by
  `apply`), not outbox litters: this removes most of the island's landing work. Plants breathe from the tile's air
  column (2.4), so their uptake is bounded by it.
- **Air:** domains no longer depend on the air grid: a coarser air node simply holds more tiles, each with its column
  share. The air section may change the order of its sums (level 2) or use a solver whose sums follow a fixed work split
  without any permission; E4 puts `store::apply` on the pool for its booking.
- **Minds:** a group or a herd that decides together lives in one cell or one tile (a group across tiles would read
  another tile's live bodies). Batched networks run per task: the pipeline lets one task run several tiles (the LPT
  list holds groups of tiles), so a batch of brains can be large enough. Think-on-change cuts updates, which makes the
  serial share larger: E1 is what keeps it small. Smart bodies with their own step length: a tile can run sub-batches
  inside a batch (plan 2.2, rule 2), unchanged.

**What I need from them**

- Plants: when plants stop being bodies, the share of updates by bodies that hold parts (E5's gate) and the new
  busiest share (E2's gate) are measured again.
- Air: the date of the air grid step (E2 must cut tiles inside the new air nodes, or come after it); whether `apply`
  stays on Flow's path (E4's gate).
- Minds: groups kept within a tile; brains' state written only by their own task.

**Line allotment for the round (proposal for the architect):** plants about +550, minds about +450, air about +250,
engine about +120 to +300 (E4, E5 gated), reserve about 250. With 9,591 today that is about 11,200 to 11,400, under
11,500.

# Emergent World: the spec

What we build, why, and how the code is laid out. It holds the principles and the owner's decisions. Where it
speaks of how a law works, `docs/laws.md` and the code are the truth. The plan and the priorities are in
`review/roadmap.md`. The rules of work are in `CLAUDE.md`; how we build, test and look is in `docs/workflow.md`.

When this document says **must**, the owner decided it. When it says **suggested**, you may change it if you
measure a better way; tell the owner. When the owner changes a decision, change this file in the same commit.

---

## 1. What we are building

A **game** in which a whole world lives and develops by itself over thousands of years, and the player watches it
on a map they understand at once.

The world starts with physics, chemistry and simple life. Everything else must **emerge**; nobody writes it in the
code. The eventual goal includes:

- climate and its changes, ice ages, deserts, rain forests, rivers carving valleys
- plants and animals adapting to places; food chains, herds, extinctions
- people spreading across the land, with cultures shaped by it; **agriculture** and **domestication**
- **settlements** and cities of different forms; **trade routes** by land and sea; discovery of far lands
- tools, metal-like materials, new **energy sources** and **transport** (carts, boats, ships, maybe car-like and
  plane-like machines)
- **conquest**, kingdoms, empires, dynasties; medicine (**penicillin-like**), gathering halls (**temple-like**),
  houses, walls, workshops

None of these needs to match Earth. They can be new mixtures of physical and chemical properties. **After** they
appear, a separate translation layer recognises them and shows them with the closest Earth concept, for example
"≈ bronze" (Section 9).

### What the player can do

1. **Watch.** A 2D map seen from above, from the whole world down to single plants. Speed controls and a timeline
   over thousands of years.
2. **Understand.** Things look like what they are. Clicking anything says what it is, what it is made of, its
   history, and why it got its name.
3. **Play god.** Set land on fire, raise or lower land, make a lake, plant or clear, later cause a drought or change
   the climate. Then go on and see what happens over the next 200 years. **There is only one timeline** (Section 11).

**What is built today:** the natural world (ground, air, liquid, chemistry, plant cover, fire, soil), a first field
of animals (the takers, not yet placed by default), the live page and its first god tools. People, the pattern
finder and the translation layer are not built.

---

## 2. Lessons: what not to do

An earlier attempt with the same goal grew very large and nobody ever watched anything emerge. Its code is not
available and must not be rebuilt. Its lessons:

1. **Do not write rules at the level of results.** No hand-written market, organisation, land lease or farming
   practice. Write causes.
2. **Do not script choices.** No functions that tell a living thing when to sow or store. It must work that out.
3. **Do not let realism beat emergence.** Field data and calibrated species reward detail, not emergence. A number
   set so that a run comes out a certain way is a rule in a smaller coat (14.3).
4. **Do not build machinery before results.** No cross-machine replay, receipts or proof campaigns before the laws
   are shown to make history.
5. **Do not pick a scale that cannot reach the goal.** One valley at one step a day never reaches empires.
6. **Do not drown in process.** Reviews are useful; process that eats most of the effort is not.

**The most important lesson:**
> Write the **laws of the world**, never its **history**. A law is about causes (how matter, energy and choice
> behave), never about a result we expect (a route, a farm, a market, a house).

**A test for any rule:** *can you list all its uses in advance?* If yes, it is a named system in disguise. A good
law has uses you cannot list: "moving matter wears what it moves through" gives valleys, trails, paths and roads.

---

## 3. Principles (must)

1. **Laws, not outcomes.** No code in `core` or `worldgen` describes a human or Earth concept. The word ban checks it
   (14.4).
2. **Five laws.** The world runs on Flow, Change, Bond, Copy and Choose (Section 6). A sixth law only under 14.2.
3. **One graph.** The world is nodes that hold stuff and links between them (Section 5).
4. **Watch first, measure second, polish last.** Build the smallest thing that can be watched. Look at it. Then
   decide.
5. **Physics makes things possible; it never makes them happen.** If an Earth-like thing cannot appear, add the
   physics it needs (9.4). Never add the thing.
6. **The translation layer is one-way.** Names and stories are computed from the history and never flow back.
7. **One timeline.** No branches, no alternative futures.
8. **Cheap repeatability only.** Same seed, same code, same machine: same history (12.5). Nothing more.
9. **Small code.** Keep `core` small by computing less, never by cramming lines.
10. **Things emerge from general laws, and we do not compute what is not needed.** Rough far away, detailed where it
    matters (Section 7).

---

## 4. Decisions already made

The decisions on size, visuals, god mode, translation, repeatability and speed are in their own sections (5.3, 10.1,
11, 9, 12.5, 12.6).

| Topic | Decision |
|---|---|
| Institutions | Coordination, trade, property and institutions are **not** primitives. They must emerge from the five laws |
| Life as fields | Plants are a field per cell (the cover); animals are a field per cell (the takers). No life is a list of single bodies. People will need a thin layer of single agents on top of the field, after the animal layer (`review/roadmap.md`) |
| Real sizes | Living things are real-sized: a member weighs what a real one weighs, set by its genes. The cover holds real mass per m². **No limit on how much a cell holds**: density is an outcome of light, crowding and food |
| Every trait has a cost | Leaves, stems, roots, frost hardiness, enzymes, coat, repair: each has a price, or nothing sorts |
| Children | By physics only: a mature slot puts a share of what it builds into seeds or young, never by a choice |
| Size and ageing | Size is set by energy (upkeep by the kg, supply grows more slowly), never by a cap. Wear by the rate of living; repair is a gene with a cost |
| Temperature and seasons | Every rate of life follows its temperature by the rate law. **No calendar rule, ever**: seasons of growth and breeding must emerge |
| Soil | Fertility differs by place: one mineral (nitrogen) comes from plain chemistry; texture and organic matter come from laws |
| The start | What is placed at the start is an initial condition, printed with every run (e.g. a tenth of the starting light-eaters carry the fixing gene; the sea's carbon starts at balance with the air) |
| One law each | One rate law for every reaction; one freezing law for ground, lakes, sea and life; fire is a law of the remains on the ground, lit by strikes |
| World chemistry | One hand-made, Earth-like "chemistry 0", numbers only in the core (6.0). No random chemistry |
| Language | Rust for the core (native and WebAssembly), TypeScript for the page, WGSL for the shaders |

---

## 5. The world model

### 5.1 Nodes and links

The world is a graph. A **node** holds stuff: amounts of each substance (non-negative, 64-bit) and heat, from which
its temperature follows. A **link** joins two nodes and lets stuff pass. There are no link types like "road" or
"river": every link is the same kind of thing, and only its numbers differ.

The fixed nodes live in the store (`core/src/store/`), by class: each cell's **ground** (its loose ground and the
heat of its active layer), its **bed** (the bonded ground under it), its **deep** store (the liquid in the bed), its
**aside** node (light and gas life holds for a moment), its **life** (its cover and takers), and the **air** nodes.
Three **boundary** nodes (sun, space, outside) book what crosses the world's edge, so the world and they together keep
every element and the energy. The store's arrays are private; only `store::apply` and `store::book_by_array` write
them.

Space is a mesh of cells and links held as plain arrays, one per quantity (`core/src/space/`): squares of 2 km on
land, cells of 2 or 4 squares a side in the open low ground, the air on blocks of 4 x 4 squares. Life is plain arrays
per cell too: a few **slots** per cell, each one kind with its amounts, its genes and a real count.

### 5.2 Three layers, and the law inside a cell

```
Layer 3  THE PICTURE  per pixel, on the graphics card, only where one looks. It reads layers 1 and 2 and writes
                      nothing back.
Layer 2  THE GROUND   fixed samples of 125 m, made once at the world's start by the erosion law, then rebuilt
                      slowly from what the running world lays down. It is state.
Layer 1  THE BOOKS    slow physics and life as fields on fixed cells. Every element and the energy are kept exactly.
```

- **Fixed cells.** The cells are made once with the world and stay for its whole life. The camera is not an input of
  the run; zoom is drawing. A field is a number per cell (kg per m²), so the cost of a cell is fixed whatever lives
  in it.
- **A ground with a history.** When the world is made, the erosion law (stream power, creep, rebound) runs over the
  samples for 2 million years: valleys, ponds with one level, one network of channels, rock in folded layers, a
  drowned shelf. While the world runs, the load the liquid lays down rebuilds the samples every few years
  (`docs/laws.md` 0 and 1.3).
- **The law inside a cell.** A cell holds means (its cover, the liquid in its soil, its snow). One rule shares each
  mean over the cell's own ground from its shape alone: how much liquid gathers at a point (upslope area over slope,
  Beven and Kirkby 1979), slope, facing, height over the cell's liquid level, and how wet a place each plant kind's
  genes like. The shares keep each mean exactly (`core/src/inside/`). The page draws with the same shares, so what one
  sees is what the world goes on with. A cell's cover may be cut into **patches** (a cleared or burnt part).
- **Detail is derived, not simulated.** Whoever needs detail calls the law inside a cell at a point: the picture
  today, single animals later. The count, size and place of plants follow from the cover's kg per m². Nobody stores
  a tree.

### 5.3 World sizes

A world is one square map of 128 x 128 squares of 2 km (256 km a side). 64 x 64 is the size to judge by in tests and
pictures. A world of 512 costs about 64 times a world of 64 per year, so sizes above 128 wait until speed is looked
at. There is no globe as a world to play (a globe stays only as the planet's strip that gives an island its air).
Worlds should differ as much as they can: from its seed an island gets one to several lands (`--shape drawn`, the
default), or one land in the middle (`--shape one`, the tests' worlds).

---

## 6. The five laws

Each law is one equation, or a small set, applied everywhere. A law lives in its own folder (`core/src/<law>/`).
This section gives the design in short. How each one works now, with the code, is in `docs/laws.md`.

### 6.0 The world's chemistry

The chemistry is not a law. It is the **data** the laws read: **chemistry 0**, hand-made and roughly Earth-like
(`worldgen/src/chem0.toml`; 11 elements, 25 substances, 24 reactions; the list is in `docs/laws.md` 2.6).

- In `core` and `worldgen`, substances are **numbers only**. Their Earth names live only in `library/chem0.toml`.
- A substance is a vector of element counts, so every reaction balances its elements. Elements are never made or
  destroyed.
- Each substance has rough physical numbers (molar mass, heat capacity, melting and boiling points, densities,
  strength, reflectance, absorption of radiation, solubility, conductivity), never precise field data.
- A reaction has inputs and outputs, a speed, an energy barrier and a heat. Some take light instead of heat.

What chemistry 0 offers, as possibilities only: a common liquid that rains, flows and wears; an air with a gas that
holds heat (climate and climate change); a light reaction that stores energy in a solid (plants); fuels that burn in
the oxidising gas; body matter, a frame and a mineral that life needs; rock of several makes; clay that bakes; a
solid of carbon (charcoal-like, coal-like) and two metal oxides it frees only in a flame's or kiln's heat.

### 6.1 Law 1: Flow (`core/src/flow/`)

**Stuff moves along links from high push to low push, with loss.**

`J = g · (Φ_i − Φ_j) · share`: the flow of a substance is the link's conductance for its phase, times the difference
of push, times the substance's share of the moving phase. The push is pressure for gas, head (height plus depth) for
liquid, concentration for what is dissolved or carried, temperature for heat. Liquid in pores flows by Darcy's law.

- **Loss does work.** Moving liquid loses head, and that energy carries loose ground and wears the bed where it is
  lost. So valleys, deltas and filled basins form by themselves. The sizes are anchored to real erosion rates.
- **Radiation is Flow** between the surface, the air and the space node: the sun by latitude and season,
  reflectance, emission by σT⁴, absorption of outgoing heat by the air's gases through a few grey layers. More of the
  heat-holding gas warms the ground with no ceiling. **Climate change needs no climate-change rule.**
- **Air and weather.** Air moves heat and vapour between blocks; rain, evaporation and snow follow. An island stands
  in the planet's air by its latitude band (`--band`), so its years differ.
- **Stability (must):** never let an amount go negative (a node limits its own outgoing links); respect a stable
  step; check the books.

### 6.2 Law 2: Change (`core/src/change/`)

**Substances react inside nodes, faster when hot, by the chemistry's table.**

The rate is mass action with an energy barrier (`k · exp(−Ea/T)`), times the contact of the phases, times a catalyst
and light where they apply; the heat of each reaction goes into the node. A catalyst is 1 in the open world. Inside
life, genes raise it for a few reactions: **this is metabolism**. Keeping a catalyst costs energy; it is slowed by its
own product. Melting, freezing and boiling follow from temperature and the substance's points; a liquid freezes at
its mixture's melting point. Burning, rot, weathering, growth and digestion are all this one law.

### 6.3 Law 3: Bond

**Nodes can be joined; joins break when pushed too hard.** A bond has a strength and a stiffness; a structure is
nodes held by bonds and moves as one; load passes down to the ground; a closed structure makes a space inside with
low-conductance walls (containment is not a rule of its own); a structure in liquid floats by what it displaces.
Geometry comes in stages: size and hollowness (tools, shelters, floats), then turning joints (wheels), then lift
(sails, planes), each only when the world needs it (9.4).

**Not built.** Today the only bonded matter is the bed, which wear breaks into loose ground. Bond for made things
waits for the people stage: make no object before a kind exists that wants one.

### 6.4 Law 4: Copy

**A piece of the world is copied, with small random changes.** Matter is never copied for free.

**Built in the fields:** a slot of cover puts a share of what it builds into seeds; a slot of takers grows young.
Both carry the parent's genes after a generation's mutation, land by one dispersal kernel (`core/src/life/disperse.rs`),
and join the most alike slot of the cell they reach (`life::kin`). Genes hold the body plan and its materials, the
reactions a kind speeds, its stores, its seeds or young, its frost hardiness or the heat it keeps, its repair.
**Not built:** imitation (copying learned habits), which is how culture spreads.

### 6.5 Law 5: Choose

**Some life senses a little, acts, and learns what helps it.** Perception is local (its own stores and heat, the cell
and its neighbours). Acts are general and physical (move, take, put, join or break a bond, push, signal, watch,
rest), each paid by physics. There is no "farm", "trade" or "attack" act: those are patterns of these. Learning
raises the body's own well-being; rewards are never shaped for a behaviour. Signals have no meaning at first.

**Not built.** The plan (`review/roadmap.md`): first habits as inherited weights on what a kind senses (the null run,
all weights zero, is what to compare with), then habits learned and passed on as a field.

### 6.6 What is NOT a law

These must emerge. If one does not, improve physics, perception, memory or learning. **Never add it as a law.**

> farming, herding, domestication, trade, money, markets, prices, property, law, councils, kingdoms, empires, war,
> religion, houses, villages, cities, roads, routes, rivers, tools, metals, fuels, engines, vehicles, medicine,
> families, dynasties

### 6.7 Life as fields (built)

- **The cover** (`core/src/cover/`): each cell holds up to `Config::variety` kinds of plant cover (8 by default), each
  slot its amounts (fuel, body matter, frame, two solids that slow a reaction), 12 genes and its plants by size. Once a month it
  catches light, runs its light reaction, pays upkeep, fixes nitrogen, builds, dies and sends seeds, which lie on the
  ground until it lets them grow (`docs/laws.md` 3).
- **The takers** (`core/src/fauna/`): animals as a field. Up to 8 slots a cell, each one kind in adults and young,
  with real counts, stores, body matter, heat and 8 genes. They take the cover by the height its plants stand at and
  the seeds lying, digest by their enzyme, pay upkeep by Kleiber's law at their body's pace, keep heat by their coat,
  and die of hunger, wear, frost and thirst (`docs/laws.md` 3b). A run places them with `--fauna-year`.
- **Flames** (`core/src/flame/`): the remains of life on the ground burn where they are dry and joined enough, by the
  chemistry's burning reactions, at a speed set by the heat balance of the bed (Rothermel 1972; `docs/laws.md` 3c).
- **Soil** (`core/src/soil/`): the texture, organic matter, pores and minerals of the loose ground (`docs/laws.md` 4).
- **The contract** (`core/src/contract/`): what lies between the natural world and what takes from it. A taker reads
  a cell and moves matter only through it; the natural world names no module of a taker (`cli/tests/sides.rs`).

---

## 7. Speed: fields, multi-rate steps, threads

Speed tools are not laws. They must give the same patterns as the detailed form. **Never remove a law to get
speed.**

### 7.1 Fields

Life is fields of slots per cell (6.7), so the cost is fixed by the cells and the slots, never by the number of
living things. A slot of one member is an individual in the maths: one set of rules for both. Counts are real and
never rounded up; a taker slot's few members are whole numbers, so a kind can truly die out.

### 7.2 Fixed cells

The cells are made once (5.2); nothing depends on where one looks.

### 7.3 Time steps (multi-rate)

One world step is one month, a fixed list of passes written once in `core/src/schedule.rs`. Each process steps at
its own pace:

| Process | Step |
|---|---|
| The air (heat, vapour, rain, the sun's season) | 15 days, two a month |
| Liquid, its load and wear; the cover, the takers, the flames; Change; the soil | 1 month |
| Heat sideways and creep | 1 year |
| The slow ground and the substances present, rechecked | every 3 months |
| The terrain's samples rebuilt from the load laid | every `build_years` years |
| The ground's long wear | once, when the world is made |

There is one world clock (whole seconds). Seasons exist because the air steps twice a month and the sun moves.

### 7.4 Threads and chunks

The history is the same with and without threads, on any number of threads (the feature `threads` of `pcore`; none
in wasm) and for any count of chunks (`core/tests/threads.rs`).

- The cells are cut into `Config::chunks` runs of neighbouring cells. Life's month runs one task per chunk; a chunk
  writes only its own cells and its view of the store. What leaves a chunk (seeds, young) waits and lands after, in
  an order fixed by the data, never by which thread ends first.
- Flow's air step, Change and the flames use the same pool where a loop is large enough to pay for it (`pcore::par`).
- The tape's recorder uses the same pool, and every sum is added in the order of the data.

---

## 8. Code architecture

### 8.1 The workspace

```
core/        crate pcore: the laws, the world state, keyed random numbers. No IO, no wall clock, no Earth words.
  src/flow/     Law 1: the ground of a cell and its parts, liquid and runoff, erosion and creep, the air (air/:
                radiation, exchange, rain, wind, weather, the planet's strip), the build of the samples
  src/change/   Law 2: the rate law, the reactor, the step over ground and air nodes
  src/cover/    the plant cover: growth, upkeep, seeds, patches, stands
  src/fauna/    the takers: animals as a field
  src/flame/    fire in the remains on the ground
  src/soil/     the loose ground's texture, organic matter, pores, minerals
  src/life/     each cell's life (cover and takers), the chunks it runs on, dispersal, the slot rule, its books
  src/contract/ what a taker may read of a cell and how it takes and gives
  src/inside/   the law inside a cell (read by the core, the tape and the page)
  src/edit/     edits from outside (god mode, a run's plan): brush, heat, basin, ground shape
  src/space/    the mesh, the square map, the terrain and its worn samples, strata, ponds, the shelf, deltas
  src/store/    the node names, the store's private arrays, `apply` and `book_by_array`, the boundary nodes
  src/chem/     the chemistry: phases, melting, heat capacity, the roles of reactions
  src/clock/    time in whole seconds, the calendar, the sleep and wake of nodes
  src/solve/    shared maths: explicit sweep on links, implicit line solve
  src/*.rs      schedule (the month), world, params (Config, Physics), ground (heat and melting plateaus), check
                (the books), units (constants), math, rng, event, par (threads)
  tests/        one file per law or part
worldgen/    the starting world: terrain, chemistry 0 (chem0.toml) and its loader, first life (life/), the run's plan
             (schedule.rs), the planet's strip for an island's air (planet.rs, planet.bin); the test kit
             (worldgen::testkit, behind the `testkit` feature)
tape/        what a run records and what a shell draws from: the recorder (rec/), snapshots, map layers, the cells'
             table, the stands of cover, the things in a window, the ground in tiles (ground/), measures, the
             fingerprint of a world
cli/         `ew`, the headless shell: run, resume, view, cells, card, planet, chem, numbers (flags in args.rs);
             PNG pictures; the writer thread
wasm/        crate ew-wasm: the core as WebAssembly for the live page, plain `extern "C"` exports (no wasm-bindgen)
web/         the page (TypeScript in src/, shaders in shaders/, textures/, build.mjs, index.html)
  shade/     crate ew-shade: checks the WGSL shaders and translates them to GLSL with naga
  look/      the picture tool: pictures and numbers of the page in a headless browser (web/look/README.md)
library/     the recognition library; today only the Earth names of chemistry 0
docs/        the spec, the laws, the numbers, the tape, the workflow
review/      the roadmap: priorities and the owner's decisions
```

### 8.2 Core rules (must)

1. **The step is a pure function:** `next_state = step(state)` (`pcore::step`). Edits enter only at a step boundary.
2. **Random numbers are keyed** by seed, time, node and purpose (`core/src/rng.rs`, counter-based), so parts of the
   world can run in any order.
3. **Fixed order.** Never iterate a hash map to decide anything.
4. **No IO, no clocks, no threads required in `core`.** Threads are optional (7.4).
5. **The same `core` compiles to native and to WebAssembly.** Keep the wasm build working.
6. **Few dependencies.**

### 8.3 The history tape (`tape/`, details in `docs/tape.md`)

- **Snapshots:** the whole world and the run's plan, compressed (bincode, gzip), at the end of a run and when asked.
  `ew resume` goes on from one; a resumed tape equals the unbroken one. A change of the saved `World` bumps
  `SNAPSHOT_VERSION`; old snapshots are refused.
- **Raw events:** only physical facts: the first time a substance appears, and each edit. "A city was founded" is
  not a raw event.
- **The series** (`series.csv`): a row a year; columns named by index, never by Earth names.
- **The run's numbers** (`physics.json`): every field of `Physics` and `Config`; `ew numbers OLD NEW` shows what
  differs.
- **The viewer's file** (`world.ewf.gz`): map layers every few years, with the series and the events.

### 8.4 How data flows

```
core (pcore) ──step──► World ──► tape: layers, cells' table, stands, things, ground tiles, snapshots, series
                                   │                                   │
                       cli `ew` writes files               wasm (ew-wasm) hands bytes to the page
                                   │                                   │
                                   └──────────► web: the page ◄────────┘
                                                 │
                                 WebGL2 draws the map per pixel (shaders in WGSL → GLSL by web/shade)
```

- **Headless:** `ew run` steps a world and writes its tape and snapshots; `ew view` and `ew cells` read a snapshot.
- **Live:** the core runs as WebAssembly in a Web Worker, so the screen never freezes. It posts, a few times a
  second, a tape frame, the cells' table (`tape::Table`), the samples' discharge when it changed, and the cover's
  stands and things where the view is close. One or two more workers, each with its own core, make the ground's
  tiles (`tape::Ground`). A snapshot opens and steps on; "Save the world" writes one back.
- **A tape:** the page also plays a run's `world.ewf.gz`; its map is drawn from its squares, with no ground tiles.

### 8.5 The page and its renderer

- The page is plain TypeScript modules joined by `web/build.mjs` into one `dist/index.html`, with the workers held as
  text. Textures are copied beside it.
- The map is drawn on the graphics card with **WebGL2** at the screen's own pixels (`web/src/gpu.ts`,
  `gpu-webgl2.ts`). The shaders are written once in **WGSL** (`web/shaders/`, shared parts in `web/shaders/lib/`);
  at build time `web/shade` checks them and writes GLSL ES 3.00 with naga. A program may be made again with detail
  compiled out for views whose pixels are large (`LEAST_E`), the same picture with less work.
- The page hands the card fields, not pictures: the ground's tiles (height, wetness index, nearest channel), the
  cells' table, the cover's kinds and plants. The card applies the laws inside a cell at each pixel and turns them
  into colours. The page decides what counts as sea, river or forest; the core never does.
- The page never waits for the card, and the world never waits for a picture.

---

## 9. Later stages: from the history to things a player understands

**Not built.** The plan, one-way from the world to the player:

```
WORLD (5 laws, no names) → TAPE → PATTERN FINDER → FACT CARDS → MATCHER + RECOGNITION LIBRARY → the page
                                                                  (leftovers) → LLM TRANSLATOR (cached)
```

- **9.1 Pattern finder** (normal code, in the background, never inside the step): repeated structures (a hash of
  each bond graph, e.g. Weisfeiler–Lehman), their use, kinds and family trees, lasting groups, routes, relations
  between kinds, new substances, cultures, one-way flows of power, sudden changes. Each pattern becomes a **fact
  card** (JSON with a stable id, what it is made of, its shape, use, count, first year, evidence).
- **9.2 Recognition library** (`library/`): one data entry per Earth concept, the only place they live. An entry has
  a parent concept, a **fingerprint** (tests on fact-card fields), the tests for each match level, the physics it
  needs, its look and naming notes. Grow it only from what appears.
- **9.3 Matcher** (normal code, the same answer every time): it walks down the concept tree only as far as the
  evidence allows. Levels: **same**, **similar** ("≈" badge), **analogy** (the parent's look, a local name), **none**
  (a generic look and a description). A name stays unless the evidence changes much.
- **9.4 The library is a physics checklist.** Each concept lists the physics it needs: no lift, no planes; no turning
  joints, no wheels; no life inside bodies, no medicine. **When a wanted concept cannot appear, add the missing
  physics. Never add the concept.**
- **9.5 LLM translator:** a small model, cached by pattern hash, for unmatched patterns (closest concept or "none",
  a local name, a line of description), stories of important events and "what happened since your action", and
  offline drafts of library entries that people review. Input is fact cards, never the raw world; output is strict
  JSON checked by code. **"No Earth equivalent" is always allowed.**
- **9.6 Naming and hard themes.** Keep apart what the world makes possible, what interpretation recognises, and what
  history explains. Up to three names: a physical description, a local name, an Earth comparison ("≈ copper").
  Names keep their history; earlier history is never rewritten. Religions, nations and movements get original names,
  never real ones. A label needs its mechanism to be really there. War, epidemics and hunger are shown neutrally: no
  celebration, no scores, no ranking of groups; a setting hides distressing detail and changes only the presentation.

---

## 10. Visualisation

### 10.1 A 2D map, not 3D (must)

A **flat plan seen from above, like a good atlas map**, that shows matter, not symbols: water coloured by its depth,
shores from dry ground through mud to shallows, rivers as channels of their true width and depth (Leopold and
Maddock 1953), the ground's materials, single plants placed by the cover and the shares inside a cell (5.2).
Everything comes from the world's numbers and the ground's shape, seeded by place, at true size. **Goals:** a player
understands what they see without reading, and what is drawn is **what is really there**.

### 10.2 The Earth view and its layers

The **Earth view** is the map in one hand-painted style over the physical colours (`web/shaders/lib/style.wgsl`,
`web/src/style.ts`); every look still means what the model holds. **Layers** show one number of the model at a time
as a colour ramp (`tape/src/layers.rs`).

### 10.3 The base map

Drawn per pixel on the graphics card from the ground's tiles and the cells' table, with light on the slopes. The
page, never the core, names each square's **map class** (sea, lake, river, snow, rock, desert, grassland, forest:
`web/src/earth.ts`). Plants are drawn plant by plant where a plant takes a few pixels, and as a field of the same
colours where it takes less, so near and far agree (`web/src/vegetation.ts`, `web/shaders/cover.wgsl`). Later,
farmland and settlements come from the pattern finder, with match badges.

### 10.4 Drawn from numbers

Each thing is drawn from its own numbers: a tree's form from its kind's genes (`web/src/form.ts`), its height from
the cover's law, its colour from a measured leaf moved by its genes, with randomness seeded by place. **Planned:**
animals from the takers' field (far away nothing, then a herd as a soft blob, then single animals); small
axonometric sprites for made things, one style and angle, tinted by their materials.

### 10.5 Zoom

From the whole world to 80 pixels a metre. Detail fades in as it grows past a few pixels; nothing depends on the
view, so a place keeps its colour after any pan or zoom.

### 10.6 Time

Play, pause and speeds; a tape has a timeline with its events and charts of its series. **Planned:** event markers
in plain words ("Year 3,410: first metal tools (≈ bronze)").

### 10.7 Planned layers

Cultures as colours (alike cultures, alike colours), flows as moving lines, borders, routes, buildings.

### 10.8 Inspect panel

Click a square: its class and each layer's value. Close up, hover a cell: each slot of its cover, its numbers and
genes. **Planned:** the Earth name with its badge, the local name, what it is made of, its history, "why this name"
(the fingerprint tests that passed) and "show evidence".

---

## 11. God mode

Each action is a **physics edit** made at a step boundary (`core/src/edit/`, `World::edit`) through the law it
belongs to, and recorded as a raw event. What an edit adds comes from the outside node, so the books close.
Disasters are never scripted. The player paints with a round brush (`web/src/god.ts`, `wasm/src/edit.rs`).

### 11.1 Actions are physics edits

| Action | What really changes | Built |
|---|---|---|
| Set on fire | heat added to the remains under the brush; what is dry and joined enough lights and burns by the flames' law | yes |
| Make a lake (add liquid) | the ground dug to a floor and the key liquid poured in to its pass | yes |
| Raise or lower land | a terrain brush; the ground comes to rest no steeper than loose ground rests | yes |
| Plant or clear | a share of each cell made a new patch; a grown stand of a starting kind placed (trees planted show as trees) | yes |
| Move or dry up a river | ground lowered along a new path and raised on the old; or its source taken away | no |
| Flood, drought | liquid added; less rain over an area for some years | no |
| Volcano, meteor | heat, rock and gas at one spot | no |
| Change the climate | the sun's strength, or the air's heat-holding gas | only at a run's start (`--sun`, `--absorber`) |
| Add or remove animals | takers of a kind placed or taken | no |

Where the world cannot do it (nothing to burn), the tool says why in plain words, from the core's own reasons.

### 11.2 One timeline

An action lands at the next step boundary and is saved in the tape as an edit event. A stroke can be undone. To see
"200 years later" the player goes on and watches. **Planned:** a save before each action (a file, not a second
timeline); a "then and now" view against the snapshot of the action's year; "what happened since" as a short story.

---

## 12. Engineering

### 12.1 Numbers

64-bit for every number that holds matter or energy, each move booked on both sides with one 64-bit number; 32-bit
only for numbers that hold nothing (genes, rates). Rounding below zero is set to zero and counted; if the count
grows, there is a bug. Every free number has a row in `docs/numbers.md` (14.3).

### 12.2 Conservation

The sun, space and outside nodes book what crosses the boundary, so one check (`check::drift`) covers every law. In
debug builds each pass of the month is checked (`check::broken`) and the run stops at the first that breaks the
world. A drift above about 1e-9 is a bug.

### 12.3 Tests (few, fast and meaningful)

A test states a law or an invariant, never the story of a bug. The tests cover:

- **conservation** of every element and the energy; **no NaN, no infinity, no negative amounts**;
- **repeatability:** the same seed twice gives the same world, bit for bit, with any number of threads and chunks;
- **snapshot round-trip:** save, load, go on; the result equals an unbroken run;
- **a small scene per law:** liquid flows downhill, heat spreads, a reaction runs faster when hot, the cover grows by
  what its budget books and dies in frost, what a taker takes the cover loses;
- **key numbers in real ranges** (climate, land, life), and the main guard that a world lives and keeps its books
  (`tape/tests/ranges.rs`);
- **guards of the design:** the word ban (14.4), the sides of the contract (`cli/tests/sides.rs`), a row for every
  number (`core/tests/numbers.rs`).

The tests share one kit, `worldgen::testkit` (the `testkit` feature, never in a run). Slow tests are `#[ignore]`.
How to run them is in `docs/workflow.md`.

### 12.4 Watch reports

Each stage ends with a short report for the owner in simple English: what was built, **pictures**, key numbers,
**what emerged and what did not**, what comes next and why.

### 12.5 Repeatability (cheap kind only)

Same seed, same code, same machine and build: the same history, whatever the threads and chunks (every order is
fixed by the data). **Do not** build identical results across machines, CPU/GPU agreement, receipts or replay
proofs.

### 12.6 Speed

**No speed target for now.** Build the most true, most interesting and best-made system; cut back when it becomes
too slow. Each merge writes down its cost per year, so the day of cutting back has numbers. Never remove a law for
speed: use the tools of Section 7, profile, or move field laws to the graphics card.

---

## 13. Risks

| Risk | What to do |
|---|---|
| Emergence stalls, or long behaviours (taming, storing, making new substances) never appear | improve sensing, memory, learning, imitation; seasons and bad years that give reasons; check the physics allows it (9.4). Never a named recipe |
| Plants and grazers do not hold each other (a zone eaten bare, a crash) | physical brakes: hard-to-digest food, thin bites, plants out of reach, eaters |
| Named rules "just this once" | the word ban; review against Section 3 |
| Results are strange, not Earth-like | allowed: local names and layers; grow the library only from what appears |

---

## 14. Guards on the laws

### 14.1 What may emerge

Patterns to look for, **never to build**: tending that changes plants, predator and prey cycles, domestication,
shared upkeep of a channel, over-use of a shared resource, cultures blending, different city forms, burning that
changes the climate, recovery after a collapse. Each has an equally valid other outcome; a world that only tells the
familiar story was tuned too much. An item counts as emerged only where it clearly beats a null run (Choose with all
its weights at zero).

### 14.2 Adding a sixth law (rare)

All must be true: a stage's watch-for failed and you can show why the five laws cannot give it; the new law is **one
equation** on nodes or links; it has **at least three unrelated uses**; it is under about 300 lines; the owner agrees.

### 14.3 Tuned numbers

`docs/numbers.md` has one row for every field of `Physics` and `Config` and for the free constants outside them,
marked physical, anchored or tuned (`core/tests/numbers.rs` fails when a field has no row). A tuned number is one set
so that a run came out a certain way (lesson 3), or one with no real size behind it. **The count of tuned rows and of
the rules chosen for a run (things picked for a run that are not numbers) may not grow.** A new number is physical or
anchored, or it replaces a tuned one.

### 14.4 The Earth-word ban in `core` and `worldgen`

`cli/tests/word_ban.rs` fails if a banned word (or its simple endings) appears in an identifier, comment or string in
`core/` or `worldgen/`: Earth outcomes (farm, herd, house, city, road, river, lake, sea, market, king, war, temple,
tool, boat, metal, wood, stone, fire, food, water, culture, ...) and the Earth names of chemistry 0's substances.
Extend it as needed. Earth words are allowed in `library/`, `web/`, the docs and LLM prompts. Physical and structural
words are fine (liquid, gas, heat, body, bond, flow, node, kind, state, field, path).

### 14.5 Do not

- write a rule, act or system named after an Earth outcome, or shape rewards for a behaviour;
- add realism (field data, calibrated species) unless something needs it to emerge;
- build proof machinery (cross-machine determinism, receipts, replay proofs); orders fixed by the data are not that;
- build branching timelines, or let the LLM or the library influence the world.

---

## 15. Glossary

| Word | Meaning here |
|---|---|
| **cell** | a square of 2 km, or a block of 2 x 2 or 4 x 4 of them in the open low ground; fixed for a world's life |
| **sample** | a point of the ground, 125 m apart on the island, where its shape is held |
| **node, link** | what holds stuff; what joins two nodes and lets stuff pass |
| **push** | what drives a flow: pressure, height of liquid, concentration, temperature |
| **the books** | the accounts of every element and the energy |
| **field, slot** | a number per cell in place of a list of things; one kind of life in a cell (amounts, genes, a real count) |
| **cover, taker** | the plant field; a slot of the animal field, or anything that takes from a cell through the contract |
| **patch** | a share of a cell whose cover has its own history (cleared, burnt) |
| **kind, starting kind** | a cluster of alike genes; the row of the start table a slot descends from |
| **edit** | a physics change from outside at a step boundary (a god action, a run's plan) |

---

## 16. References (ideas, not rules)

Epstein and Axtell, *Growing Artificial Societies* (1996); Turchin et al., PNAS (2013), polities from simple rules
on a map; Arthur and Polak (2006), technology from combining parts; Kiyotaki and Wright (1989), money-like goods
among learning traders; Yaeger, Polyworld (1994); Baker et al. (2019), emergent tool use; Tero et al. (2010), used
paths get stronger; Chan, Lenia (2019); Boyd and Richerson (1985), culture as inherited habits.

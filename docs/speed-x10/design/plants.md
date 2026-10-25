# Speed x10, section "plants": a field of stands, a card per genome, upkeep and light together

Owner's choices covered here: **1** (plants as a field), **2** (a growth table per genome), **3** (pay upkeep and get
light together across the interval). Other sections: engine (threads, 32-bit, ceiling), air, minds.

## 0. In short

- **A plant is a body of a lineage with no brain** (`Life::brainless`, `core/src/life/registry.rs:61`). Since
  decision 72 such a lineage can never get a brain, and a lineage with a brain never loses it. So the set is closed:
  no body ever moves between "plant" and "body". Today all plants are one lineage (the light-eaters' founders).
- **Plants become stands of a field.** A stand is what a plant node already is: "how much plant of this kind lives in
  this cell" (members, stores, genes mean and spread, sizes, young or adult). What changes is **the law and the
  cadence**: a stand is updated by **the plant month**, a closed form over its whole interval, not by the body's day
  (`body::live`). Animals stay bodies.
- **Choice 3 is the heart of it.** The plant month pays upkeep and gets light **together**, as one stock equation over
  the interval (a labile carbon pool, Thornley 1972; LPJ, Sitch et al. 2003). This removes the splitting error that
  C2 found (upkeep paid before the light, a reserve of "one interval"), so a stand can update every 36 to 60 days
  with the same result as many short updates.
- **Choice 2 in its honest form: a card per genome, not a grid table.** What a genome fixes is worked out once, at
  every change of the genes (`Body::prepare`, already there as `Consts`), plus a few numbers for the month. A grid
  table of the body law over light, heat, water and stores does not fit: 6 or more inputs, 20 to 200 million
  instructions to build one, and the genes of every stand move at every update (selection), so it would be rebuilt
  every time. The closed form costs about 1.5 µs and is exact in its books. Details in 2.4.
- **Stands stay `Body` values in the domains' cell lists** (at least at first). So perception (faces), biting
  (`take_body`), cover, shade, sums, the air's light marks, the tape and the viewer need no change. E1 showed that the
  layout of bodies gives no speed; the cost is in the law.
- **Measured** (this work, one thread, year 200 of fresh runs, a loaded machine): plants are about **38%** of a year
  on the island (22 x 22) and **21%** on the globe (36 x 18). A plant update costs 17 to 19 µs, of which the body's day
  and the selection sample are 13 µs.
- **Expected gain from this section alone:** island 1.25 to 1.45 times, globe 1.1 to 1.2 times (one thread). Plants
  for free would give at most 1.6 and 1.27 times. **Plants are needed for 10 times, but they are not where most of it
  is**: after them, animals (30 to 40%) and Flow (16 to 21%) are the year.
- **Lines:** about +300 to +450 core code lines net (P1 +230, P3 +170, minus about 60). It needs the ceiling raised
  (engine section, choice 7).
- **Steps:** P0 (measure, Haiku), P1 (the plant month, Opus), P2 (the cadence, Sonnet), P3 (the field step per cell,
  Opus, gated), P4 (compact stands, gated, likely not built), P5 (a grid table with discrete strains: only on a branch,
  if the owner still wants it tried).

---

## 1. What exists today

### 1.1 Who is a plant today

- `Life::brainless(b)` = the lineage's brain has no hidden units (`core/src/life/registry.rs:51-63`). Light-eater kinds
  are founded with `HIDDEN 0` (`worldgen/src/life/kinds.rs:52-66`; roster rows 2 and 3 too). A body with no brain does
  not mutate its brain-size genes (`body::vary`, decision 72), so its lineage never changes (`lineage::resize`).
- Such a body never decides (`survey.rs:276`): it never moves, takes, gives, watches or acts on a part. So it never
  holds a part and is never inside one (measured: 0 of 1,820 and 0 of 1,083).
- It is always a population node: `nodes::may_be_node` is true for it (`nodes.rs:97-100`). Measured: no plant has one
  member or fewer.
- The tape's "light-eater" is another test (`body::light_eater`, the light catalyst over 5, `body/genes.rs:150-160`);
  no law reads it. Today both sets are the same bodies.

### 1.2 The update of a plant body today

`survey::batch` (`core/src/survey.rs:209-239`) runs the bodies due in each 2-day batch, domain by domain on the pool.
A plant is due every `Config::node_days` (36) days (`Body::interval`, `body/mod.rs:245-248`). Its update
(`survey::update`, `survey.rs:262-297`):

1. `copy::sample` (`copy/select.rs:211-235`): on one update in `sample_every` (2), a clone with one gene one standard
   deviation off runs the whole update as far as growth (`gain_to_growth`, `body/live.rs:122-124`), in a checkpoint of
   the store view, rolled back.
2. `body::live` (`body/live.rs:81-91`) → `to_growth` (`live.rs:97-117`):
   - `settle_upkeep` (`live.rs:143-170`): **upkeep of the time is paid from the stock first** (`pay_at`, line 149),
     the unpaid share dies (`SPENT`), the wear share dies (`AGE`).
   - `physics` (`body/physics.rs:43-73`): rain caught, light caught (`catch_light`, 85-90), the air it can breathe
     (`gas_pool`, 94-106), **the reactor** on its stores (`react` → `Reactor::react`, 111-129, all 28 reactions in
     sub-steps), gas with latent heat (`settle_gas`), the light set aside taken from the bodies' node (`store_light`,
     153-161), evaporation (`evaporate`, 181-234), soaking (`soak`, 256-269), roots (`roots`, 293-306), the heat
     network and the ground's heat (`relax_heat`, 312-333, sets the next `survey::Draw`).
   - `keep_warm` (190-202), `check_stores` (206-233: minimums, frost, boiling), `copy::grow` (`copy/grow.rs:13-39`;
     `convert` 87-139 keeps **a reserve of one interval's upkeep**, line 107), `starved` (131-139).
   - then children (`copy::copy`, `copy/mod.rs:57-103`; dispersing ones go to the outbox as a `Litter`, 168-171) and
     `shed` (254-272).
3. `copy::select` (`select.rs:240-249`): the breeder's equation moves one gene's mean by the sample's gain.
4. `copy::mature` (`copy/young.rs:386-412`): young members past the copy mass join the adult node.
5. `nodes::split` (`nodes.rs:206-238`), `Domain::see` (`life/domain.rs:126-129`), `Domain::schedule`.

Around the updates, per survey (every `phys_days`, 10): wear and the monthly merge (`nodes::crowd`, `nodes.rs:15-20`),
the shade (`survey::shade`, `survey.rs:127-149`), the sums per cell (`survey::sums`, 92-104), and per batch the faces
of every body (`Faces::make`, 178-189).

### 1.3 What other code reads plant bodies

| Reader | Where | What it needs of a plant |
|---|---|---|
| Perception | `choose/perceive.rs:70-110` | its face (`survey::Face`: id, members, visible traits) |
| Biting, giving | `choose/feed.rs:92-129` (`take_body`), 134-156 (`give`); `body::wound` | a `Body` by handle: stores, members, strength, interval |
| Cover over a body | `body/exchange.rs:32-36` (`cover_of`) | mass and members of the cell's bodies |
| Shade and canopy | `survey.rs:127-149`, `sums` 92-104, `World::canopy` | height `vis[6]`, leaf `vis[1]`, members, keep share |
| The air step | `flow/air/step.rs:114-127` (`fixed`: canopy, absorb, keep per cell); `air/book.rs:61-62` (light marks moved by shade); `survey/draws.rs` (`Draw`, `claims`, `Draws::settle`) | per cell sums; per body `mark`, `shade`, `keep`, `draw` |
| Rain on bodies | `physics.rs:77-81` (`rain_caught`) | `rained` mark, leaf, shade |
| Movers pushing through | `choose/go.rs:76` (`wear`) | the cell's bodies |
| The tape and viewer | `tape/src/layers.rs:91-116`, `rec/tally.rs`, `rec/cols/world.rs:59-64` | living bodies, genes, mass |
| Snapshot | `life/mod.rs:96-124` | living bodies (and puts each in the ring by `due`, line 118) |

### 1.4 Measured now (this work)

Fresh runs with today's code (`ew run --size 22 --seed 1 --years 200`, `ew run --globe --size 18 --seed 1 --years
200`), then one or two years from the year-200 snapshot on **one thread** in a copy of `core` with timers around the
body update (a probe outside the repository; the machine had other loads, so read the shares, not the last digit).

| | island 22 x 22, year 200 | globe 36 x 18, year 200 |
|---|---|---|
| bodies, of them plants, animals | 2,074, 1,820, 254 | 1,517, 1,083, 434 |
| cells with plants; plant nodes per such cell, mean and most | 240; 7.6 and 26 | 236; 4.6 and 14 |
| young and adult plant nodes | 1,189 and 631 | 706 and 377 |
| plant lineages; plants with one member or fewer; holding or inside a part | 1; 0; 0 | 1; 0; 0 |
| plant members, median per node | 9.6e4 | 1.5e9 |
| genes whose node means differ by over half a step across all plant nodes | 24 of 127 (copy mass 5.8 steps, the fixing catalyst 16) | 22 of 127 |
| a year, s | 1.03 | 1.31 |
| Flow, Change | 0.168, 0.057 | 0.273, 0.064 |
| plant updates a year; µs each; s | 18,369; 17.0; **0.313 (30%)** | 10,590; 18.8; **0.199 (15%)** |
| of it: the body's day (`to_growth`), the sample, children (`copy`), the rest | 0.183, 0.046, 0.035, 0.048 | 0.112, 0.029, 0.026, 0.032 |
| animal updates a year; µs each; s | 8,903; 35.3; 0.314 (31%) | 11,850; 44.0; 0.521 (40%) |
| bodies' stage outside the updates (survey, faces, booking, outbox) | 0.172 (faces 0.016, survey 0.065) | 0.226 (faces 0.016, survey 0.065) |
| **plants, all included (their updates and about half the rest of the bodies' stage)** | **about 0.39 s, 38%** | **about 0.28 s, 21%** |

Plants are 63% (island) and 47% (globe) of the updates, as the brief says (55 to 70%), but **an animal update costs
twice a plant update**, so plants are a smaller share of the time than of the updates. In the profile of `measures.md`
Z1, `to_growth` is 31.5% and 20.3% of a year's instructions (about 52,000 instructions a call, samples included).

### 1.5 The C2 finding

`docs/history.md` (C2, S62): a node that updates rarely pays a whole interval's upkeep from its stock before the light
of that interval comes in (`live.rs:107-108`), and growth keeps a reserve of one interval's upkeep (`grow.rs:107`). So
the stock it needs grows with the interval: at 144 days adult deaths of unpaid upkeep are 13 times as many. Worse, the
law has no limit for short steps: a starving node loses the unpaid share (1 − P/U) at each update, so over a time T
its survivors are (P/U)^(T/Δ): **the death rate goes with 1/Δ, the length of its updates.** This is why the cadence of
plants could not be changed.

---

## 2. The new design

### 2.1 Who becomes field, and what stays a body

**Rule:** a body of a lineage with no brain (`Life::brainless`) is a **stand** of the field. Every other body stays a
body. Founders of such a lineage are placed as stands (`World::add`); children of a stand are stands.

What the field keeps, item by item (the brief's list):

| A plant today | In the field |
|---|---|
| disperses children | yes: the share `DISPERSE` of births goes to a neighbour cell (the `Litter` of the outbox, as today; P3 may make it a per-cell flux) |
| holds parts, is inside parts | never today; never in the field (a stand has no actions) |
| is eaten | yes: a stand is a `Body` in its cell's list; `take_body` and `wound` work unchanged |
| is perceived by animals | yes: a stand has a face (id, members, visible traits) |
| shades others, is shaded | yes: `survey::shade` reads the cell's list; heights and leaf area as today |
| takes water by roots, soaks, transpires | yes, in the plant month (closed forms of `physics::soak`, `roots`, and transpiration through its pores) |
| stores | yes: its stores (all substances) as today |
| dies, rots | yes: deaths by cause (`SPENT`, `AGE`, `LACK`, `PHASE`, `TAKEN`, `BROKEN`); remains to the ground (`body::remains`, `to_ground`); litter rots there by Change, as today |
| has its own temperature | **no**: a stand stands at its ground's temperature (2.3) |
| keeps warm or cool by burning or sweating (`KEEP_T`, `COOL_T`) | **no**: these genes are not read for stands (they are off in every light-eater today: 250 K and 320 K) |

What stays a body: every body of a lineage with a brain, node or individual (all eaters of every kind). Their law
(`body::live`, `choose::act`) does not change in this section.

### 2.2 The stand: its state and where it lives

A stand is a `Body` value (`body/mod.rs:52-132`), stored in its domain and in its cell's list, with its id, as today.
The fields it uses: `id, alive, cell, count, lineage, parent, born, last, mark, rained, shade, keep, draw, e, store,
genes, spread, size_spread, size_lag, link, genome, wet, brood, vis, k`. The brain fields stay empty, as they are
today for a plant (`rows`, `ms`, `mem`, `trace` are empty vectors; `known` is 4 pairs).

- **Members and biomass:** `count` (a real number, never rounded) and its stores, mol of each substance (body matter,
  fuel, key liquid, minerals, frame, lime and the rest).
- **Genes:** their mean (`genes`) and the variance of each among its members (`spread`), as a node today.
- **Age structure:** a young stand and an adult stand of each kind in a cell, linked (`link`), and in a young stand the
  spread of log sizes (`size_spread`, `size_lag`): the two-stage population of `copy/young.rs` (Lefkovitch 1965,
  McKendrick 1926). Kept as it is: B1 (one node for both stages) changed the world and was killed (S59).
- **The card** (2.4): `k: Consts`, worked out from the genes at every change of the genes.
- **Its marks:** `mark` (light, J/m²), `rained`, `keep` and `draw`: the light and rain it caught since `last` are exact
  totals, whatever the length of its interval.

Why not a new compact struct now: E1's probe (S77, S82) stored bodies in update order and got no gain; the misses come
from the update's own working set. The cost of a plant is its law (13 of 17 µs), not its storage. A compact stand
(P4) is built only if a profile after the other sections says so.

### 2.3 The plant month (choice 3)

A new law for stands, in a new file `core/src/body/month.rs` (it is a law of bodies; `body/` is its folder). It runs a
stand over the `Δ` days since its last update (`Δ = day − last`, at least one), in stages. Forcing over the interval is
read as an exact total where the world keeps one (light, rain), and at its value now where it does not (temperature,
the air's shares, the ground's liquid).

**Stage 1. Where it is** (`live::site`, reused): its cell, air node, the ground's temperature `T` (`t_ground_last`),
depth of liquid, walls (none for a stand in practice), the cover over it.

**Stage 2. What came in** (reused helpers):
- light caught `L`, J (`physics::catch_light`): leaf area × (light − mark) × (1 − reflectance) × what liquid over it
  lets through. Exact over the interval, as the air step books light into `w.light` and moves each mark by its shade
  at each air step (`air/book.rs:61-62`);
- rain caught (`physics::rain_caught`);
- the air it can breathe, by its pores, no more than its air node holds (`physics::gas_pool`).

**Stage 3. Water and minerals.** The key liquid's balance (soaking in at `LIQ` × wetness, out through the pores)
relaxes in `1/(LIQ × wet)` days, far less than a month for a plant (`LIQ` gene about 0, so about a day). The month
takes the liquid at its level for the interval: where soaking meets what the pores pass, capped by the ground's
liquid (the exact form of `physics::soak`, and `exchange::through`). The activity `a` (`Body::activity`) follows from
it. Minerals come by roots up to their room (`physics::roots`, already exact over the time). P0 measures how often the
liquid's time is over a tenth of the interval; there the mean of the exponential approach is used (closed form).

**Stage 4. Income, upkeep and growth together** (new, `month::balance`). Per member and per day, constant over the
interval:

- **Income P**, J of fuel a day, is the light reaction's rate: the least of three limits (Blackman 1905), the same three
  that the reactor meets today (laws.md 2.4):
  - what the light caught can store: `light_yield × L / Δ`;
  - what the pores bring of the carbon gas: the gas pool of stage 2 over `Δ`, times the energy stored per mol;
  - what the catalyst runs at `T` and activity `a` (`Body::rate_at` on the light reaction).
  With **product inhibition** (`Body::inhibited`, decision 63): `P(R) = P₀ × max(0, 1 − R / R*)`, `R*` the energy of
  the fuel store at its keep level, `R` the fuel stock.
- **Upkeep U**, J a day: `pay::upkeep` for one day at `T` and `a` (unchanged law, laws.md 6.2).
- **Growth G**, J of fuel a day: the allowance `c_build × supply / m_ref × a` (`grow::allowance`), as fuel burnt for the
  work plus fuel built in, limited by the minerals it holds (`grow::reach`). Growth uses only the fuel above a
  **reserve** `R_res = U × D`, `D = Physics::reserve_days` (new; 36 by default, today's reserve of a node's interval;
  see 4 and numbers).
- **The stock R** (J its burnings can release, `Body::fuel`) follows `dR/dt = P(R) − U − G(R)`:
  - phase 1, `R > R_res`: `G` is the allowance;
  - phase 2, `R = R_res`: `G = min(allowance, max(0, P(R_res) − U))`: growth eats exactly the net income;
  - phase 3, `R = 0` and `P < U`: the stock is empty. **Members die at the rate `μ = (1 − P/U) / D`** (cause
    `SPENT`): the share `1 − e^(−μ t₃)` over the `t₃` days spent there. This rate does not depend on the length of
    the update: it is the limit of today's rule at today's pace (one unpaid share per `D` = 36 days).
  Each phase is a linear equation `dR/dt = α − βR`, with the exact solution `R(t) = R_eq + (R₀ − R_eq) e^(−βt)` (or a
  straight line when `β = 0`). The times where `R` meets `R_res` or 0 have closed forms. At most three phases. The
  integrals of `P`, `U` paid and `G` over the interval come out of the same forms. This is the labile pool of a plant
  carbon balance (Thornley 1972; Thornley and Johnson 1990; the labile store of LPJ, Sitch et al. 2003) with
  Blackman's limits.
- **Age:** the share `1 − e^(−wear × Δ)` dies (`AGE`), wear as today (`live.rs:154`): already independent of the step.
- Members that die take their share of every store (`live::lose_share` → `thin`): the per-member stock above does not
  change by deaths, so deaths apply as shares at the end.
- Growth and the capture area within the interval are not compounded (C2 measured that this does nothing measurable).

**Stage 5. Book what the integrals say.** Each integral is an **extent of a reaction**, booked by its stoichiometry:
- the light reaction: extent = ∫P / its energy; it takes the carbon gas and liquid, gives the fuel and the oxidising
  gas; the light it stores is taken from the bodies' node by the keep share, as `physics::store_light` does today;
- the burnings: extent = ∫U paid / their energy (`Consts::burn`, the order of `pay::plan`);
- growth: extents of `Chemistry::builds` then `Chemistry::build` (`copy::grow`'s order and shares, `ALLOC` to the
  brood past the copy mass);
- any other catalysed reaction that is on (fixing, splitting the frame): its rate over `Δ` by `change::frac` on its
  inputs at the start, capped at its products' level (`Body::inhibited`). Reactions with no catalyst (bare rates) are
  not run in stands (P0 measures that they are slow inside bodies: no decomposers' factor `open` there).
- Gas goes to and from the air node with its heat and latent heat (`Moves::gas`, as `pay::pay_at`), solids and
  liquids to the stand's stores (`body::add_store`).
One shared helper books an extent of reaction `r` on a body (`body::apply_extent`): it is cut out of `pay::pay_at`
(lines 134-145), which then calls it. **Every element is kept by the stoichiometry, whatever the closed form says.**

**Stage 6. Heat.** A stand stands at its ground's temperature (a leaf's heat capacity is small and its conductance
large: it follows the air and ground within hours, Campbell and Norman 1998). Its heat after the month is
`Body::energy_at(T)`. What that takes or gives beyond the heat of the reactions and of the liquids that moved is booked
first with what its bodies' node holds for it (`Draw::owed`), then with the ground (`Moves::take_from_ground`, never
below `T_FLOOR`), as `physics::relax_heat` does for a body with a very large conductance; if the ground cannot pay, the
stand ends colder, as a body does today. The latent heat of what it transpired is its next draw (`Draw::after`, its
stop at the dew point of the air it breathes): the air step keeps drawing it from the ground between its updates
(laws.md 1.4), unchanged.

**Stage 7. The checks that end an update** (reused, unchanged): `check_stores` (minimums kill the poorest, frost × activity
over the span, boiling), `starved`, `shed`. Then, as today: children (`copy::copy`), `copy::mature`, `nodes::split`,
`Domain::see`.

**What the month does not do** (law changes, listed in 3): no own temperature, no keep-warm or cool-by-sweat, no bare
reactions, no reactor sub-steps.

### 2.4 The card per genome (choice 2), and why not a grid table

**What is computed once per genome.** Every number of the month that depends only on the genes: `Consts` already holds
most of them (`body/mod.rs:136-162`: the powers of ten of `GAS`, `LIQ`, `HEAT`, `LEAF`, `COPY_MASS`, the catalyst
factors, the burnings, the uptakes, the catalysts' capacity, the interval, the tissue factor, the reference rates). The
card adds a few: upkeep per kg of living body per day at `t_cat` (with repair), leaf upkeep per m², the allowance per
kg-equivalent, the light reaction's energy per mol of carbon gas, the build's fuel per kg. It is worked out in
`Body::prepare`, which runs at every change of the genes (birth, `select`, `add`, `split`). Cost: a few thousand
instructions, the cost of `prepare` today. **It is never stale**: there is no cache to invalidate.

**The grid table the owner asked about** (a table per genome over light, heat, water, stores; values read and
interpolated) was worked out and is **not proposed**:

| Point | Number |
|---|---|
| inputs the month depends on | light per leaf area, temperature, ground wetness, carbon gas share, fuel fill, mineral fill (2), member size over copy mass: 7 or more |
| a small grid | 5 light × 8 temperature × 3 wetness × 3 fuel fill = 360 points, without carbon gas, minerals and size |
| cost of one point | one run of the body law, about 50,000 instructions (`to_growth`, `measures.md` Z1) |
| cost of one table | 18 million instructions (360 points) to 200 million (with 3 more inputs) |
| how often a genome changes | every update of every stand: selection moves a gene mean (`select.rs:247`), joins pool the genes (`nodes::add`), splits shift them. A table per stand would be rebuilt at each update: 18 million instructions instead of 50,000 |
| a table per lineage | all plants are one lineage: one table for 1,820 stands with genes that differ by up to 16 steps: wrong for most |
| accuracy | interpolation in 4 to 7 inputs over `min()` limits (light, gas, water) and the reserve's corners makes errors of several percent where the stand sits at a limit; the books stay exact (the moves are extents), but the law would be a fit, not the physics |
| a rule of the repository | a table of past results is a cache; the main rule of the foundation forbids new caches for timing |

A table only works with **discrete strains**: genes on a lattice, each strain fixed for life, mutation as a flux of
births into a neighbour strain, selection by the growth of strains (no breeder's equation). That is a new evolution law
with its own speed setting (how many strains a cell may hold), which is the kind of world choice that failed B2 (S66).
It is step P5: only on a branch, only if the owner still wants it tried.

**Where a grid of the body law is used:** as a one-time check in P1. The body law still exists (animals use it). A
probe on a branch runs a light-eater body through `body::live` and the same stand through the month on a grid of
inputs (light, temperature, wetness, fuel fill) at short intervals (2 and 6 days, where the old law's splitting error is
small), and writes the differences in `measures.md`. It is a measure, not a kept test.

### 2.5 The field step (P3, gated): one formula per cell

P1 and P2 keep stands in the ring of updates (they update in the batches like a body, with interval `field_days`).
P3 makes the field a field:

- **Stands leave the ring.** `survey::survey` (`survey.rs:28-69`), in its per-domain pass on the pool
  (`on_domains`), after the wear and the merge and before the shade, runs `field::step` on the cells whose turn it is:
  cell `i` updates at a survey when `(survey number + i) mod (field_days / phys_days) = 0`. So a third (at 30 days) or
  a sixth (at 60) of the cells update at each survey, spread over the period. `Config::validate` refuses a
  `field_days` that is not a multiple of `phys_days`.
- **One site per cell.** Stage 1, the soil (`physics::soil`) and the air's shares are worked out once per cell.
- **Fair shares of what is scarce.** Stage 3's uptakes (soaking and roots: cheap) run first for every stand of the cell.
  If the stands together ask a node (the ground's liquid, a mineral, the light set aside in the bodies' node) for more
  than it holds, every stand's ask of that node is cut by the same share: one cut per node and substance, the node's
  own outgoing links (`CLAUDE.md` 3.4). Then each stand's month runs with its cut supply. Today the stand that updates
  first takes first; in a monthly field the first stand of every cell would be favoured every month.
- **Faces of stands** are made when they update (and when a bite changes them), not at every batch.
- **What a stand does not need** is skipped: `lineage::resize` (no brain), `Domain::schedule` (no ring).
- **Children:** the stayers join the cell's young stand of their kind (`young::linked`, as today); the dispersers go to
  the outbox as a `Litter` (unchanged path; `on_domains` applies the outboxes).
- **Snapshot:** `Life`'s reader puts a body in the ring only if its lineage has a brain (`life/mod.rs:118`); `Body::due`
  stays for bodies.

### 2.6 Evolution in the field

Unchanged in law, cheaper in cost:
- **Mutation** at birth (`body::vary`): the litter's genes are the parent's mean with noise of the litter's size, and
  its spread gets the mutation variance (laws.md 4.1).
- **Selection inside a stand**: the breeder's equation (`copy::select`; Lande 1976; Robertson 1966), its sample
  evaluated by **the plant month** for the genes one standard deviation off in one drawn gene, on a checkpoint of the
  view rolled back (`Domain::checkpoint`). A sample costs about 1.5 µs instead of 10 µs. So **`sample_every` is dropped
  (a sample at every update)**: one speed setting, its test and its row fewer.
- **Selection between stands**: stands of one kind grow at different rates in the same cell; splits along the widest
  gene (`nodes::split`) and the monthly merge of alike stands (`nodes::crowd`) as today.
- **Dispersal** to neighbour cells: the share `DISPERSE` of the births, as a litter (today's law).

### 2.7 How animals deal with stands, and how stands deal with light and heat

- **Eating:** `take_body` (`feed.rs:92-129`) on a stand, unchanged: Holling's take on the members its area holds,
  the bite tears matter and kills members (`body::wound`). The stand's `paces` (its interval over its gene's, used for
  rooms, `feed.rs:104`) is `field_days / gene interval`, as today with `node_days`.
- **Seeing:** faces as today (P3: refreshed at the stand's update and after a bite).
- **Cover:** `cover_of` (`exchange.rs:32-36`) counts stands' mass, as today.
- **Shade and light:** `survey::shade` per cell over all bodies, stands included; the air step's per-cell canopy,
  absorb and keep come from `survey::sums` as today (`step.rs:114-127`); each stand's mark moves by its shade at each
  air step (`book.rs:61-62`), so its light over a long interval is exact.
- **Heat drawn:** `survey::Draw` per stand, set by stage 6; `Draws::settle` and `claims` (`draws.rs:11-95`) unchanged.
- **Litter:** members that die and what a stand sheds go to the ground node with their heat (`remains`, `to_ground`),
  and rot there by Change.

### 2.8 Step loop, snapshot, tape, viewer

- **Step loop** (`schedule::processes`, `schedule.rs:43-60`): P1 and P2 add nothing (stands are due in the batches).
  P3: the field runs inside the survey (every `phys_days`), before the shade and the sums, so the air step reads
  the field as just updated.
- **Snapshot:** `Config::node_days` becomes `Config::field_days`; `sample_every` goes; `Physics::reserve_days` is new.
  Bump `SNAPSHOT_VERSION` (`tape/src/snapshot.rs:13`) in P1. P3 changes no saved field.
- **Tape:** no column changes. `updates` counts stand updates too (`docs/tape.md` says so). The light-eater columns,
  `plants`, `plant_members`, `cover_*`, `root_<K>_*` read living bodies, which stands are.
- **Viewer:** no change (layers "plants" and "bodies" read living bodies).

### 2.9 Files

| File | Change |
|---|---|
| `core/src/body/month.rs` (new, about 250 lines) | the plant month: stages 3 to 6, `balance` (the stock's phases), the card's extra numbers |
| `core/src/body/pay.rs` | `apply_extent` cut out of `pay_at` (one place to book a reaction's extent on a body) |
| `core/src/body/live.rs` | `gain_to_growth` goes; `lose_share`, `check_stores`, `starved`, `shed`, `site` become `pub(super)` for the month |
| `core/src/body/mod.rs` | `interval` reads `field_days` for a stand; `Consts` gets the card's numbers |
| `core/src/survey.rs` | `update` calls the month for a stand (P1); the field step in `survey` (P3) |
| `core/src/copy/select.rs` | the sample runs the month; `sample_every` goes |
| `core/src/params.rs` | `node_days` → `field_days`; `sample_every` goes; `reserve_days` new; validation |
| `core/src/life/mod.rs`, `domain.rs` | P3: stands not in the ring |

---

## 3. The law changes, in physical terms

1. **A plant pays its upkeep and gets its light together, over its whole interval.** Its fuel is a stock fed by the
   light reaction and drawn by upkeep and growth at the same time. It keeps a reserve of `reserve_days` of upkeep
   (a fixed time, not "one interval"). When the stock is empty and the light does not pay the upkeep, its members die
   at a rate (`(1 − P/U) / reserve_days`), not by a share per update. **Effect:** the result no longer depends on how
   often a stand is updated (C2's splitting error is gone), so plants can update every 36 to 60 days.
2. **A plant stands at its ground's temperature.** It no longer has its own heat network, keep-warm or cooling by
   sweating. Its reactions' heat and its transpiration's latent heat go to and from the ground (through its bodies'
   node, as today).
3. **Reactions without a catalyst do not run inside a plant** (they are slow there: P0 measures it).
4. **The light reaction's rate is the least of three limits** (light, carbon gas, catalyst), as the reactor gives it
   in practice for a plant, with product inhibition kept.
5. **(P3) The stands of a cell share a scarce liquid or mineral alike** instead of first come, first served.
6. **Selection inside a stand samples at every update** (`sample_every` goes).

**Why it is still honest.**
- *Elements:* every change of a stand's stores is an extent of a reaction booked by its stoichiometry
  (`apply_extent`), or a move between nodes booked both sides by `store::Moves`. The closed form only says how far
  each reaction runs.
- *Energy:* each extent carries its reaction's enthalpy; light stored comes from the bodies' node (set aside by the
  air step); the stand's heat is set to its value at the ground's temperature and the difference is booked with the
  bodies' node and the ground. Drift stays at rounding (`check::drift`, 1e-12).
- *No overdraw:* the gas pool is capped by what the air node holds (the view); the liquid and minerals by what the
  ground holds (`take_from_ground`, P3's cut); light taken by what the bodies' node holds; each extent is capped by
  its inputs held (`change::scarce`). A stand never asks a node for more than it holds.
- *Counts:* members are real numbers; deaths are shares (`thin`), never rounded.

---

## 4. Rules that change (proposed wording)

- **handover 7.1, a new last paragraph:** "Plants are the bodies of lineages without a brain. They live as **stands**:
  population nodes of a cell, young and adult, updated by the plant month (laws.md 6.10). A stand never decides, so
  it never moves, holds or enters; animals see it, bite it and stand under it as any body."
- **handover 7.3, the population nodes line:** "Plants (stands) update every `field_days`; their upkeep, light and
  growth over that time are one closed form, so the length of the step does not change the result."
- **laws.md 0 (intervals):** replace "a population node without a brain at least `node_days`" by "a stand (a body of
  a lineage without a brain) every `field_days`".
- **laws.md 4.4, the "Pace" bullet:** "**Pace**: a stand updates every `field_days` (6.10); its month integrates its
  interval exactly, so the cadence is a speed setting. A body with a brain updates by its `INTERVAL` gene."
- **laws.md 6.2, a new last sentence:** "A body with a brain pays its upkeep at its update, before its actions bring
  income; a stand pays upkeep and gets light together over its interval (6.10)."
- **laws.md 6.10 (new), "Plants: the month of a stand"**: the stages of 2.3 in five sentences, the three limits, the
  stock's phases, the reserve, the death rate when the stock is empty, the ground's temperature.
- **numbers.md:** the row of `node_days` becomes `field_days` (speed; days; "a stand updates every this many days";
  anchor: the pace test at ×2 and ÷2 and against 10 days). The row of `sample_every` goes. A new row `reserve_days`
  (tuned; days; "a plant keeps this many days of upkeep as fuel before it grows, and starves at this time when its
  stock is empty"; anchor: 36, today's reserve of a node's interval, so the world stays near today's; plants carry
  labile reserves for weeks to a season, Chapin, Schulze and Mooney 1990). The count of speed settings in the last
  table: 8.
- **CLAUDE.md 1, the table:** add a row "The plant month (stands) | `core/src/body/month.rs`".
- **CLAUDE.md 5.4 (golden test):** no new wording; P1 and P3 re-pin it with the reason.
- **Decisions (a new log for this round), to write at P1:** plants = lineages without a brain; stands stay `Body`
  values; the month is a closed form, no grid table (2.4); `reserve_days` 36; bodies with a brain keep paying first
  (their income comes from actions after the update); `sample_every` dropped.

---

## 5. How we check

**Level 3 (the same world)** for P1, P2 and P3. "The same world" means: the key numbers of `tape/tests/ranges.rs`
hold, the pace test passes, the books hold, and the climate checks pass. The references may be set again once, by the
owner's word, if P1 moves a number within its range but away from its old centre (as decision 27 did).

**Key numbers that must hold** (`ranges.rs:125-136`): light-eaters at year 30 (10.38 ± 0.3 log10 kg), their life
(1.81 yr, ×/÷ 2), eaters (5.354 ± 0.5), the heat-absorbing gas (388.8 ± 40 ppm), land temperature (± 1 K) and swing
(± 1.5 K), rain, rivers, land worn. The stored share (S50, 0.55 against "under 0.5") must not move beyond its seeds'
noise (the new reserve rule may move it: report it).

**Tests to add** (few, fast; `worldgen::testkit`):
1. `a_stand_pays_upkeep_and_gets_light_together` (`core/tests/body.rs`): the same stand over 60 days in one update, in
   two of 30 and in six of 10 ends with the same members, fuel and body matter within 2%. It states the law's main
   promise (the splitting test). Today's law fails it.
2. `a_stand_in_the_dark_dies_at_its_reserve_time`: a stand with no light and an empty stock loses members at the rate
   `(1 − P/U) / reserve_days`, whatever its updates.
3. `a_months_stands_keep_the_books`: a cell's stands over one month keep every element and the energy (1e-12), and
   take no more liquid than the ground holds.
4. (P3) `stands_of_a_cell_share_a_scarce_liquid_alike`: two stands in a dry cell get the same share of their ask.

**Tests to change:**
- `the_pace_of_nodes_keeps_life` (`ranges.rs:327-342`) becomes `the_pace_of_the_field_keeps_life`: `field_days` 10
  (the reference: every survey), the setting, ×2 and ÷2; life's key numbers (`LIFE`, `ranges.rs:322`) within their
  noise.
- `selection_inside_a_node_moves_its_mean_as_its_members_would` (`core/tests/populations.rs:766`): drop `sample_every`.
- `core/tests/clock.rs:154-155`: `node_days` → `field_days`.
- **Audit of tests that use light-eaters as generic bodies** (`core/tests/body.rs`, `choose.rs`, `copy.rs`,
  `populations.rs`, `store.rs`): a test of the heat network, keep-warm or cooling by sweat moves to a body with a brain
  (an eater founder); a test of roots, fixing, shade, light set aside or growth stays on a stand and now states the
  month's law. No tolerance is loosened.

**Tests to delete:** the `sample_every` checks (its ×2/÷2 run in `docs/numbers.md`'s anchor) and any test of
`node_days` alone.

**Each step also runs:** the normal suite (under 30 s), the golden test re-pinned (P1, P3; the commit says why),
clippy, the wasm build, and the key-number test once (level 3).

---

## 6. Expected gain

From the measures of 1.4 (one thread). Per plant update today: 17.0 µs (island) and 18.8 µs (globe), of which the body's
day 10.0 and 10.6, the sample 2.5 and 2.7, children 1.9 and 2.5, the rest 2.6 and 3.0.

| Step | Per plant update | Island: s a year saved, year after | Globe: s saved, year after |
|---|---|---|---|
| today | 17.0 / 18.8 µs | 1.03 s | 1.31 s |
| P1: the month (≈1.5 µs) and its sample (≈1.5 µs, now at every update), same cadence (36 days) | ≈7.5 / 8.5 µs | 18,369 × 9.5 µs = **0.17 → 0.86 s (1.20×)** | 10,590 × 10.3 µs = **0.11 → 1.20 s (1.09×)** |
| P2: `field_days` 60 (0.6 of the updates), if the pace test passes | same | 7,350 × 7.5 µs = 0.055 → 0.80 s (**1.29×**) | 4,240 × 8.5 µs = 0.036 → 1.165 s (**1.12×**) |
| P3: the field step (≈1.5 µs less per update; faces, schedule and site per cell; about 0.04 s of the bodies' stage) | ≈6 / 7 µs | 0.017 + 0.04 = 0.057 → 0.74 s (**1.39×**) | 0.01 + 0.035 = 0.045 → 1.12 s (**1.17×**) |
| the ceiling: plants for free | 0 | 0.39 → 0.64 s (1.6×) | 0.28 → 1.03 s (1.27×) |

**Honest ranges** (one thread): island **1.25 to 1.45×**, globe **1.1 to 1.2×** (the low ends: P1 and P2 only, or
a month that costs 3 µs; the high ends: P3 and a month under 1.5 µs). On 4 threads the gain is a little smaller
(plants already run in parallel domains; their share of the serial phase, the booking and the litters, falls too):
island about 1.2 to 1.4×, globe 1.08 to 1.17×. If the engine section removes the serial phase, the plants' wall
share follows their instruction share.

**Amdahl, for the whole plan:** after this section the island's year is about 0.74 s on one thread: animals about
0.31 s of updates plus their share of the bodies' stage (about 0.1 s), Flow 0.17 s, Change 0.06 s. **10× must come
mostly from the minds and the air sections.** Plants become about 8% of the island's year and 5% of the globe's.

**Long runs:** crowded cells (S66: 623 bodies in one globe cell, crowded years +14% of a 1,000-year run) are plant
stands of one lineage. They become about 3 times cheaper per stand-year; the field does not bound them (that is a
world rule, the owner's).

---

## 7. Line cost

| Step | Added | Removed | Net |
|---|---|---|---|
| P1 | `month.rs` ≈ +250; `apply_extent` +15; card numbers +15 | `gain_to_growth` −4, `sample_every` and its checks −12, the `node_days` branch and checks −6, `pay_at`'s inline booking −12 | **≈ +260 −35** |
| P2 | 0 | 0 | 0 |
| P3 | field step ≈ +130; fair shares +40; ring and snapshot +10 | `update`'s stand branches −15, per-stand site and schedule −10 | **≈ +180 −25** |
| **Total** | | | **≈ +380 net (range +300 to +450)** |

Today `core` is 9,591 of 9,700 lines. This section needs the ceiling raised to about 10,100 for itself (engine
section, choice 7). What it deletes: the selection sample's second body day (`gain_to_growth`), the speed setting
`sample_every`, the special pace of brainless nodes (`node_days`); P3 deletes plants from the ring and the per-batch
faces. It does not delete the body law (animals use it).

---

## 8. Risks and answers

| Risk | Answer |
|---|---|
| The world moves (plants' mass, life, the gas, land temperature) | Level 3 with the key numbers; the one-time grid check of P1 against the body law; the new rule's numbers (`reserve_days`) are anchored to today's pace; if a key number leaves its range, P1 is not merged until the owner rules |
| The pace test fails at 60 days (stand births, maturing and the light all at one time) | P2 stays at 36 (P1's gain stays: 1.20× and 1.09×) |
| The stored share (S50) moves | Reported; the reserve rule is the place where the owner can fix S50 as a law |
| Selection gets stronger or weaker (a sample at every update, a cheaper but different gain) | `selection_inside_a_node_moves_its_mean_as_its_members_would` still holds the law; `genes_do_not_drift_without_selection` (slow) runs |
| Tests that used light-eaters as generic bodies break | The audit in 5; each test is moved to the body it means |
| Two energy laws (stands: together; bodies with a brain: pay first) | Said in laws.md 6.2. Bodies' intervals are 5 to 20 days, their income comes from actions after the update; the owner may ask the same law for them later |
| A plant at the ground's temperature loses transpiration cooling and frost margins | P0 measures `|T_body − T_ground|` of plants today; if it is often over 3 K, stage 6 keeps the old relaxation (its closed form, `relax_heat`) at a small cost |
| The gain is small on the globe (plants 21%) | Said plainly; this section's job there is the cadence and the law, not the 10× |
| The month at 60 days takes a month's carbon gas from the air node in one go | The air node holds about 30 times a month of plant uptake (16 cells of 4 km²); the take is capped by the view; P3 spreads cells over the period |
| Over the line ceiling | Engine section raises it; P3 is gated and its lines are the first to give up |

---

## 9. Steps

| ID | What | Check level | Done when | Kill line | Depends on | Parallel with | Agent |
|---|---|---|---|---|---|---|---|
| **P0** | Measure on the three remade benches: plants' share of a year (updates, survey, faces, booking, outbox); in the plant reactor work, the share of reactions without a catalyst; `|T_plant − T_ground|` of plants; how often a plant's liquid time is over a tenth of its interval; how often a plant's stock hits the reserve or zero over 36 days; stands per cell. Rows in `measures.md` | — | the rows are written | if plants are under 20% of island22's year on one thread, P3 is not built | — | anything | Haiku |
| **P1** | The plant month (2.3), the card (2.4), the sample by the month, `sample_every` dropped, `node_days` → `field_days` (36), `reserve_days`; the one-time grid check against the body law (on a branch, results in `measures.md`); tests 1 to 3; the test audit; laws.md, numbers.md, handover 7.1 and 7.3; snapshot bumped; golden re-pinned | 3 | the new tests pass; books at 1e-12 over a century (slow suite); key numbers in range (or new references accepted by the owner); the pace test (10, 36, 72) passes; island22 −12% of a year's instructions on one thread (callgrind) | under 10% fewer instructions on island22, or key numbers out and not accepted: reverted | P0 | the air section; the minds section if it does not touch `survey::update`'s stand branch (agree first) | Opus, with an adversarial Opus review |
| **P2** | `field_days` 60 if `the_pace_of_the_field_keeps_life` passes at 10 against 30, 60, 120; else 36 | 3 | the pace test passes at the new setting; key numbers in range | the pace test fails at 60: stays 36 | P1 | P3's design | Sonnet |
| **P3** | The field step (2.5): stands out of the ring, per cell in the survey, staggered; one site per cell; fair shares; faces of stands at their update; snapshot reader; test 4; golden re-pinned | 3 | tests pass; key numbers in range; island22 at least −5% of a year's instructions on top of P2 | under 5%: reverted (stands stay in the ring) | P2; the engine's step that changes `on_domains` or the serial phase, if any (agree on the order) | the minds section | Opus |
| **P4** | Compact stands (a `Stand` struct without brain fields, per cell arrays) | 1 (bit for bit, same laws) | a profile after the other sections shows stand memory or clones at 5% or more of a year | under 3% of a year on 4 threads: not kept (E1's lesson) | P3 and the other sections | — | Opus |
| **P5** | A grid table per strain (discrete strains, a new evolution law): **only on a branch, only if the owner wants it tried** | 3 | faster than the month by 1.3× at the same error against the body law | not 1.3× faster, or the strain cap moves the world beyond noise: not kept | P1 | — | Opus |

Docs move with each step (laws.md, numbers.md, tape.md for `updates`, the handover lines, `CLAUDE.md`'s table).
Commit messages end with `Lines: +X -Y (core code N)`.

---

## 10. Interactions with the other sections

**What I need from the engine section**
- The core ceiling raised by about 400 lines for this section (choice 7).
- If the thread order becomes free (choice 4): the field step (P3) runs per domain in any order; within a cell the
  stands run in a fixed order. Stands need nothing from the outbox but dispersing litters.
- 32-bit inside a month (choice 5): the plant month books once per stand per update, so it can book in 64-bit as it
  does; it needs no 32-bit accumulation. A stand's update is already "exact per cell per month".

**What I give the engine section**
- About 63% (island) and 47% (globe) of today's body updates become cheap and cell-local; the serial phase loses the plants' booking
  of moves each batch and (P3) their per-batch faces.

**What I need from the air section**
- Keep `w.light` (the cumulative light per capture area per cell) and the shade correction of each body's mark at each
  air step (`air/book.rs:61-62`), or an equivalent, so a stand's light over a long interval stays exact.
- Keep reading the canopy, absorb and keep per cell from `survey::sums` (`step.rs:114-127`) and the bodies' draws
  (`Draws`). With a coarse air grid, a stand breathes from its cell's air node (`Node::air(j)`), whatever its size.

**What I give the air section**
- Per cell, the stands' leaf area and their transpiration's latent heat as today (`Draw`). If the air section wants
  plant transpiration inside its implicit ground balance (a canopy conductance, Monteith 1965), the stands' month can
  give a conductance per cell instead of a draw: a later option, not in these steps.

**What I need from the minds section**
- Keep `take_body`, `wound`, `give` and perception working on a target that is a `Body` in the cell's list (stands
  are). If "think on change" wakes an animal when the bodies around it change, stands change only at their update and
  when bitten.

**What I give the minds section**
- Stands are stable targets: they never move, and they change once a month (P3: their face too). The animals' share of
  a year (30 to 40% after this section) is the largest left: the minds section is where the next big factor is.

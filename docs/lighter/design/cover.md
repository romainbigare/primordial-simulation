# Design: light-eaters as a cover (idea 5)

The round "Lighter world", idea 5. The owner said yes to this law change. This file is the design; the plan
(`docs/lighter/plan.md`) and the decisions (`docs/lighter/decisions.md`, L1, L2, ...) take what the architect keeps.

## 0. The idea in five lines

Today a cell holds about 8 light-eater nodes (stands): young and adult nodes of each kind, split halves, nodes of
neighbours' litters. Each is a body with its own update every 40 days. The cover keeps light-eaters as **bodies**, but
only **one body per kind per cell** (at most `cover_kinds`, proposed 2), and that body holds its young and its adults
together in **two stages**. Its children stay in it (no new body), its seeds land in the neighbour cell's cover of their
kind (gene flow), and its gene means move by the sample member (Lande 1976, today's `copy::select`). The physics of
its day is today's `body::live`, unchanged. So most of the work goes: fewer updates, no young nodes, no churn of
splits and merges, few faces, few litters.

**Why a body and not a new store class.** Every reader of light-eaters reads them as bodies: the shade
(`survey::shade`), the cell sums (`survey::rows`, `Life::sum`, read by lanes, moving, erosion's `hold`, perception),
the light set aside and the draws of ground heat (`survey::Draws`, `draws::claims`), the bites (`feed::take_body`),
the wear of movers (`choose::wear`), the shelter (`body::cover_of`), the faces, the tape. A new store class would need
a second copy of each. A body per kind keeps one place for each idea (CLAUDE.md 2.1) and one law of physics for every
body. The gain comes from the count, not from a new formula.

**Name.** "cover" is taken in code: the soft cover (snow, `flow::land`), `body::cover_of` (the others over a body),
`cover_median`. This design calls the law "the cover" in the docs and uses only `Stages` (a type) and
`Config::cover_kinds` in code. The architect may rename (decision D4).

## 1. Inventory: where light-eaters as bodies are touched today

"Stays": the code is not touched; it reads the cover as it read stands. "Rewritten": it changes. "Goes": deleted
for bodies without a brain (it stays for animal nodes where they use it).

### 1.1 Their day (Flow and Change on a body)

| Place | What it does for a stand | Cover |
|---|---|---|
| `body::live`, `live::to_growth`, `settle_upkeep`, `keep_warm`, `check_stores`, `starved`, `shed` | the whole update | stays |
| `body/physics.rs`: `catch_light`, `store_light`, `rain_caught`, `gas_pool`, `react` (`Reactor::react`), `settle_gas`, `evaporate` (transpiration by `GAS`), `soak`, `soak_again`, `roots`, `relax_heat` | light, breath, reactions, liquids, heat | stays |
| `body/exchange.rs`: `film`, `surface`, `breath`, `through`, `leaf_area` | boundary layer, leaf | stays (`leaf_area` reads `equiv`, below) |
| `body/pay.rs`: `upkeep`, `plan`, `pay`, `Payer` (brainless: no mind cost) | paying | stays |
| `body/tissue.rs`: `adult`, `equiv`, `height`, `member_body`, `size_moments` | maturity, supply by mass^0.75, height | **rewritten** for a body with `Stages` (2.3) |
| `copy::grow` (`ALLOC` share into the brood past maturity) | growth | **rewritten**: the brood comes from the adult stage's growth (2.4) |
| `survey::shade` (light from the tallest down), rain shared the same way | light between bodies | stays; fewer bodies a cell |
| `survey::rows`, `caches` (canopy, absorb, keep) and `flow/air/cell.rs` reading them | the light bodies absorb warms the surface; their light reactions' share is set aside in `Class::Bodies` | stays |
| `survey/draws.rs` (`Draw`, `Draws::settle`, `owe`, `claims`) | ground heat drawn between updates | stays |
| `Body::interval` (a brainless node: at least `node_days`, 40) | pace | stays |

### 1.2 Copying

| Place | Today | Cover |
|---|---|---|
| `copy::copy`, `emit`, `give`, `provisions`, `start` | a node's children join its young node, a share `DISPERSE` go to a neighbour as a litter | **rewritten** for bodies with `Stages`: stayers join the cover itself as young (no stores move); dispersers are a litter as today |
| `copy::young` (`linked`, `link`, `mature`, `Grown`), `Body::link` | young node and adult node, members mature along the link | **goes for covers** (stays for animal nodes); covers mature inside (2.4) with the same `math::Pass` |
| `copy::kin` (the young or adult node of a kind) | stage must match | **rewritten**: a body with `Stages` matches any stage |
| `copy::land_litter`, `back`, `outbox::Entry::Litter` | a litter joins its kind's young node, else starts a body | stays; a litter that finds no kind and no room joins the most alike cover (3) |
| `nodes::add`, `rejoin`, `Lot`, `part` | pooling of genes, spreads, sizes | **rewritten**: they pool `Stages` too |
| `nodes::split` | a node whose halves are not alike splits while the cell has fewer than `crowd` - 1 bodies | stays; for covers the room is `cover_kinds` |
| `nodes::crowd`, `join_alike`, `merge_closest` (monthly) | merges alike nodes; past `crowd` the most alike | stays for animals; covers join alike covers monthly, and never pass `cover_kinds` (3) |
| `copy::select` (`sample`, `select`, `read`) | the sample member: the breeder's equation inside a node | stays, on each cover |
| `body::vary` (mutation of a group of children) | 127 draws a litter | stays, once a cover update (stayers) plus once a litter |
| `lineage::resize` (brainless branch) | | stays |

### 1.3 The survey

`survey::survey`: `worn` (wear of movers: `choose::wear` gives each body of the cell its share of the energy by mass,
`body::wound` with `BROKEN`), the monthly `nodes::crowd`, `shade`, `rows`: all stay. Fewer bodies make them cheaper.
The canopy check S83 (`the_ground_under_dense_canopies_stays_near_its_neighbours`) takes the densest 5% of land cells by
rank: it stays and keeps its meaning.

### 1.4 Bond

Light-eaters are never held, never inside a part, never form, join or anchor (they have no brain; `bond::form` uses
the former's own stores). Movers break them by `choose::wear`; bites break them by `Body::breaking`; both read
`Body::solids` and stay. The movement cost through what stands in a cell (handover 6.1, `lanes::resist`, `thickness`)
reads `Life::sum[1]`, which holds the covers: stays. Erosion's `hold` (`flow/mod.rs`) the same. No part is made of
plant matter directly: an eater that takes a solid it uses (`Body::uses`) and forms before it digests it could make
one, today and with the cover alike. **Nothing in Bond changes.**

### 1.5 Choose

Animals see light-eaters through faces (`survey::Faces`, `perceive::perceive`), take from them with `TAKE_FROM`
(`feed::take_body`: Holling's type II on the swept share of a node's members, the escape by intervals, `wound`), and
repeat by habit (`choose::Habit::prey`, by id). A cover is a body with a face: **all of this stays**. A cell shows
1 or 2 light-eater faces instead of about 8 (87% of the faces made today are plants, P0), so an animal's `SEEN` slots
hold more animals than before (an effect to watch, 6). A cover lives for many years, so habits keep their prey longer.

### 1.6 The tape, worldgen, the viewer

- Tape (`docs/tape.md`): `plants` (now covers), `plant_members`, `cover_median`, `cover_max`, `births`,
  `died_<cause>`, `root_0_*` (members, kg, bodies, life, life_real, births, genes): **all keep their meaning** (members
  and births are members). `root_0_bodies` falls by about 5 times. Proposed new column `root_<K>_adults`: the share of
  members in the adult stage (D11). The key numbers (light-eaters' mass, life of light-eaters, S50) keep their
  meaning.
- Snapshot: `Body` gains `stages`, loses nothing; bump `SNAPSHOT_VERSION` (49 to 50) in the law step.
- Worldgen: light-eaters already start as one node per land cell (`worldgen::life::kinds::LIGHT`, "one node per land
  cell"); fixers are whole cells. A founder gets `Stages` all adult. **No other change.**
- Viewer: reads only the tape: no change.

### 1.7 Tests that state plant laws

Listed with their fate in 6.3.

## 2. The new law

### 2.1 What a cover holds

A cover is a `Body` with no brain (its lineage has none, `Life::brainless`), one per kind per cell. It holds what a
stand holds: every substance (64-bit, all members together), heat `e`, the brood, genes (means), `spread` (variances),
marks of light and rain, `keep`, `draw`, members `count`, and its face traits `vis`. New: `stages: Option<Stages>`
(None for a body with a brain):

```
Stages {
    adults: f32,      // share of the members at or past the copy mass (0..1)
    young_body: f32,  // share of the living body (`built`) that is the young's (0..1)
    young_var: f32,   // variance of log10 of the young members' bodies, decades²
}
```

They are **shares**, so every law that acts on all members alike keeps them as they are: deaths of a share
(`lose_share`, `thin`), a bite that tears a share of every member (`wound`), shedding, splits by halves (`part`). Only
growth, maturing, births and joins change them. They hold no matter (32-bit is allowed: X4). From them:

- young members `n_y = (1 - adults) n`, adults `n_a = adults n`;
- a young member's mean body `m_y = young_body · built / n_y`; an adult's `m_a = (1 - young_body) · built / n_a`;
- the mean of log10 of the young bodies `mean_y = log10 m_y - ln10 · young_var / 2` (log-normal).

`size_lag` and `size_spread` stay for animal nodes; a cover leaves them at 0 (D5).

### 2.2 How it grows: today's day

The update is `survey::update` as today: `copy::sample` (on one update in `sample_every`), `body::live` (upkeep, the
physics of the time, keeping warm, store checks, growth, children, shedding), no decision, `copy::select`,
`lineage::resize`, then the stage step (2.4) where `copy::mature` ran, `nodes::split`, `Domain::see`, the next
update at `node_days`. The reactor of the cover is the body's reactor (`Reactor::react` with its catalysts, the bare
reactions too); upkeep is paid by its burnings (`body::pay`); death by a share (`lose_share`), remains to the ground
(`to_ground`, `remains`), where the litter constants rot them (laws.md 2.1). Light, rain, breath, transpiration,
soaking, roots and heat are today's stages. **No formula of the day changes.**

### 2.3 What the stages change in the body's numbers

- **Supply** (`Body::equiv`, read by intake, growth's allowance and the leaf): a cover sums its stages, each by
  mass^`intake_exp` (b = `intake_exp`): `n_a m_ref (m_a / m_ref)^b + n_y m_ref (m_y / m_ref)^b · exp(b (b - 1) (ln 10)²
  young_var / 2)`. The last factor is the mean of a power over a log-normal spread (Jensen). Without it a pool of many
  small members and a few large ones would get up to 50% more supply than the same members as two nodes.
- **Maturity** (`Body::adult`): a cover is adult when it has adults and `m_a` is at or past the copy mass. A hurt adult
  stage (`m_a` below it, after bites or wear) is immature until healed, as today.
- **Height** (`Body::height`, the order of light and rain): a cover's height is its adults' member (the top of the
  canopy); with no adults, its mean young member's. Its visible mass `vis[0]` stays the mean member's (what an animal
  meets).

### 2.4 Children, maturing and seeds: the stage step

At growth (`copy::grow`), the matter it makes splits by supply: the adults' share `a = E_a / (E_a + E_y)` (E the two
terms of 2.3; the growth allowance goes by supply). Of the adults' part, what heals `m_a` back to the copy mass is
body, and of the rest the share `ALLOC` is brood (today's rule, read on the adult stage). The young's part is young
body (`young_body` rises).

After the update's growth and children (where `copy::young::mature` runs today):

1. **Maturing.** The young's log10 bodies rose by `d = log10(new young body / old young body)` (members fixed, so the
   growth per member). `math::Pass::new(mean_y, young_var, log10 copy_mass, d)` gives the share of young members that
   pass the copy mass and the share of the young body they hold (`shares(0)`, `shares(ln 10)`), and the moments of
   those left (`moments`). `adults += passed members · (1 - adults)`, `young_body -= passed body share · young_body`,
   `young_var = left variance`. This is today's flux across a size (McKendrick 1926; von Foerster 1959), with the same
   function; nothing is placed or merged, two shares change.
2. **Births** (`copy::copy`, a cover is a node: one parent). `births = brood / child matter - BROOD_HELD · n_a`
   (today's `BROOD_HELD` per member, now per adult). A share `DISPERSE` leaves (3 below); the rest **stays**: their
   genes are drawn as today (`body::vary`: the mean of the group plus drift noise of its variance over its size, the
   variance left among them, mutation added; Lande 1976), and they join the cover as young members: genes and spreads
   pool by members (`nodes::add`'s formula, run on the cover itself: `join_self`), `n += births`, the young pool
   with `log10 child body` and variance 0 by members (`rejoin`'s formula), `young_body` takes the children's body
   (`births · child body`), `adults = n_a / n`. **No store moves**: the brood's matter becomes young body inside the
   same store, the provisions are already the cover's. The `Born` event is recorded as today.
3. **Seeds.** The dispersers are a litter as today (`emit` with `!here`: `give` takes their matter and provisions,
   with heat, out of the cover; `Entry::Litter` in the outbox; `land_litter` at the batch's end in the cell's domain).
   The litter's `Stages` are all young (`adults` 0, `young_body` 1, `young_var` 0). It joins the cover of its kind there
   (`kin`, no stage test) by `nodes::add`: genes, spreads and stages pool by members. This is **gene flow** between
   cells by seed spread, weighted by each cover's seed output (Wright's island model): a cover that makes more seeds
   spreads its genes more. A litter of less than one member with no cover to join goes back (`copy::back`), as
   today.

### 2.5 Competition, kinds and selection

- **Light.** Covers of a cell share light by height, a layer of leaf area x taking 1 - exp(-x), equal heights sharing
  a layer (`survey::shade`): a tall kind shades a short one. Inside a cover, young and adults are one layer: the cell
  absorbs the same light for the same leaf area, but a kind's seedlings are not shaded by their own adults (2.7).
- **Water and minerals.** Each cover soaks and takes roots' minerals from the same ground by its own genes, in id order
  inside the batch, as stands do.
- **Selection inside a cover** (`copy::select`): the sample member one standard deviation up or down in one drawn gene,
  its gain against the cover's: the breeder's equation (Lande 1976; Robertson 1966). Unchanged.
- **Selection between covers and cells.** The kinds of a cell compete for light, water and minerals, and the one that
  grows more takes the cell. Covers that make more seeds send more genes to their neighbours.
- **Kinds**: section 3.

### 2.6 Animals eat it, movers wear it, the books

- **Eaten**: `feed::take_body` on the cover's face: the taker's swept share of the cell holds that share of the cover's
  members; it takes by Holling's type II, pays `c_handle` and the breaking energy of the share it tears
  (`Body::breaking`), the moved stores and heat go from the cover to the taker in one `transfer` (both sides, 64-bit),
  the torn spill goes to the ground (`body::wound`, `thin`, `to_ground` by `d.moves.give_all`). Unchanged.
- **Worn**: `choose::wear` at the survey: the cell's push energy is shared by mass among its bodies, a cover takes its
  share and loses what that breaks (`BROKEN`). Unchanged. Lanes and their regrowth (`lanes::regrow`) read the cell's
  body mass: unchanged.
- **Books.** Every move of the cover is today's: the store view `store::Moves` of its domain, booked by
  `store::book_by_array`; body to body in `transfer`; remains by `give_all`. The births that stay move nothing. Seeds
  take their stores and heat out of the cover and into the litter body (`give`), and add them to the cover where they
  land (`rejoin`): sums of 64-bit numbers. `check::drift` holds them; the debug checks of overdraw stay.
- **Principles.** No calendar: seasons come from the tempo by temperature (`Body::pace`, the rate law). Real sizes:
  members, member bodies in kg, mass per m². No Earth word: the code names are `Stages`, `adults`, `young_body`,
  `young_var`, `cover_kinds`.

### 2.7 What the cover cannot do that stands do today

1. A cell holds at most `cover_kinds` kinds of light-eaters (today unbounded: up to 26 to 28 nodes).
2. A kind's young and adults share one store, one temperature and one light layer: seedlings are not shaded by their
   own adults, and do not starve, freeze or age apart from them (members of all stages die alike).
3. Seeds of a new kind that land in a full cell join the most alike cover (their genes mix with it) instead of
   starting a node (3, D3).
4. Inside a cell, the genes no update reads before growth (`CHILD`, `DISPERSE`, `WET`, `ALLOC`, `MUT`: `select::UNREAD`)
   are no longer selected between young nodes of different genes. They are selected between the kinds of a cell, and
   between cells by seed output (gene flow). Watch them (6).
5. The young of a kind are one group with one size spread, not several young nodes from different neighbours.

**What stays possible** (the watch-for of M1 to M3 and later): being eaten, trampled and worn into paths; shading,
transpiring and warming the ground; rotting into the minerals; dead frame on the ground as fuel; solids for parts
through an eater; kinds that differ by climate (between cells, and up to `cover_kinds` in a cell); adapting to places
(the sample, gene flow, clines: Haldane 1948, Slatkin 1973). **Tending** stays possible by physics: an animal can put
minerals or liquid on a cell's ground (`feed::put`), keep other eaters off (take from them), wear away the covers of
a cell by moving through it, and eat one kind and spare the other (two faces, `TAKE_FROM`): what a tended patch needs.
Moving seeds by hand is not possible, today or with the cover. No watch-for item is blocked.

## 3. Kinds: up to `cover_kinds` covers per cell

**Decision proposed: a few covers per cell, `Config::cover_kinds` = 2 (a speed setting with a physical side effect,
like `crowd`), set by the probe C0.**

- One gene distribution per cell (1) cannot hold two strategies: a fixer and a non-fixer, a tall slow kind and a short
  quick one. A fixer's seeds landing beside a non-fixer would pool into "half fixers", and disruptive selection could
  never split them. The watch-for wants kinds that differ; the start already has two (the fixers, a tenth of the
  cells).
- Today a planted cell holds about 2.7 (island22), 1.6 (globe36) and 3.5 (region24) adult nodes (P0: 659 / 377 /
  1,593 adult nodes over the planted cells), many of them split halves kept apart for selection between nodes, which
  the sample now does inside the cover. Two hold two strategies at a cost near one.
- **A new kind starts** in two ways, both only while the cell has fewer than `cover_kinds` covers: a cover whose halves
  would not be alike splits along its widest gene (`nodes::split`, today's rule, room from `cover_kinds`; the halves
  keep their stages); or a litter that finds no alike cover (`kin`) starts one (`copy::start`).
- **A full cell.** A litter that finds no alike cover in a full cell joins the most alike cover there (`nodes::add`;
  its gap widens that cover's spread, capped by `body::most` as today). Covers of a cell that became alike join at the
  monthly survey (`join_alike`, the key without stage). So a cell never passes `cover_kinds`, and `merge_closest`
  never runs for covers.
- Animal nodes keep `crowd` and every rule they have.

## 4. Cadence and threads

- **When.** A cover updates in the body batches at its pace: at least `node_days` (40) days, a whole number of
  batches, as a stand (`Body::interval`). The stage step runs inside that update. The monthly join runs in the
  survey's `worn` (`nodes::crowd`). Nothing new runs at the physics step: light, rain and heat reach the cover between
  updates through the marks and the bodies' node, as for stands.
- **Threads.** Covers are bodies of their domain. They update in the parallel phase of `survey::batch` (`on_domains`,
  the one runner `par::longest_first`), domain by domain, cell by cell, by id: one writer, the domain (CLAUDE.md
  3.6). Births that stay write only the cover. Seeds are `Entry::Litter` in the domain's outbox and land at the batch's
  end in the domain of their cell (`outbox::land`), in the order of the domains they come from, then the order made;
  the full-cell test reads only that domain. No new phase, no new order: the world is the same with and without
  threads.
- **Not chosen:** the cover in the survey or with Change every 10 days (four times the updates, and a second booking
  path for bodies); a slower pace (X64's `field_days` 60 or 90 needs growth in pieces: a later step if wanted).

## 5. Cost

Today (P0 at the speed x10 benches; per update after PL1 about 84k instructions, 16 µs on one thread):

| | island22 | globe36 | region24 |
|---|---|---|---|
| stands per planted cell (mean) | 8.2 | 4.6 | 10.3 |
| plant updates a year per planted cell | about 85 | about 46 | about 104 |
| plant updates, µs a planted cell-year (1 thread) | about 1,400 | about 760 | about 1,700 |
| the same, all included (faces, booking, litters, survey) | about 1,700 | about 1,000 | about 2,100 |
| plants, share of the year (1 thread, all included) | 37% | 19% | 60% |

The cover (estimates, to be measured):

- A cover update: today's stand update less maturing as a body (5.2k), the litter search and placing of stayers, the
  churn of `nodes::add` and `merge`; plus the stage step (one `Pass`, a few hundred instructions) and `equiv` with
  stages. About 75k instructions, 14 to 15 µs.
- Covers per planted cell with `cover_kinds` 2: about 1.3 to 1.8 (the probe C0 measures it). At 40 days, 9.1 updates
  a year: about 12 to 17 updates a planted cell-year, plus one landing litter an update (about 3 µs).
- **About 230 to 320 µs a planted cell-year all included, against 1,000 to 2,100: 5 to 8 times less.**

Expected share of the year after (one thread, the middle of the range, x6.5 on the plants all included):

| | island22 | globe36 | region24 |
|---|---|---|---|
| plants, share of today's year, after | about 5.7% | about 2.9% | about 9.2% |
| the year after, against today | x0.69 (x1.46 faster) | x0.84 (x1.19) | x0.49 (x2.0) |
| plants, share of the new year | about 8% | about 3.5% | about 19% |

At x5 the island gains x1.42 and the region x1.92; at x8, x1.48 and x2.1. Animals gain a little too: fewer faces to
look through and fewer plant entries in the booking and the outbox. Ideas 3 and 4 still act on the cover's update
(its reactor and its sample), on a much smaller base.

## 6. What must still hold (level 4)

### 6.1 Key numbers (`the_key_numbers_stay_in_their_ranges`, 3 seeds)

All keep their meaning and their ranges: ground temperature on land (291.79 +-1), seasonal swing (15.13 +-1.5), rain
on land, heat-absorbing gas (388.8 +-40), river cells, land worn down, **light-eaters 10.38 +-0.3 log10 kg**, eaters
5.354 +-0.5, **life of light-eaters 1.81 x/ 2** (members over deaths: young deaths count as today), life of eaters,
rain in m/yr. **S50** (stored share, today 0.556 to 0.558) is the known failure: it must not move past its seeds'
noise (about +-0.004) away from its range; a move toward 0.5 is welcome. Compare with "The key-number references
after AIR4.0" and "Z: the final checks" 1 of `docs/speed-x10/measures.md`. After the law lands, the architect takes
new references for the steps after it (D7).

### 6.2 Climate and life tests

`the_long_step_keeps_the_climate` (its margin is 2%: +0.293 K against 0.3, X85), `a_globe_without_life_keeps_an_earth_like_climate`
(no life: unchanged), `the_ground_under_dense_canopies_stays_near_its_neighbours` (S83), `the_pace_of_nodes_keeps_life`,
`the_batches_of_bodies_keep_life`, `the_eaters_live_and_stop_growing`, `the_laws_do_not_depend_on_the_size_of_the_cells`
(quarters hold four times the kinds per area: watch), `nodes_and_individuals_make_the_same_world` (animals only; the
covers are the same in both modes), `a_century_keeps_its_books`, the golden test (re-pinned in each law step).

### 6.3 Tests rewritten, deleted, new

Rewritten (they state the law of light-eater nodes, which becomes the cover's):
- `core/tests/populations.rs`: `a_node_spreads_with_its_children_and_splits_when_too_varied` (the children join the
  cover as young; it splits when too varied while the cell has room); `dispersing_children_of_a_varied_node_join_their_kind`
  (seeds join the cover of their kind in the next cell); `selection_inside_a_node_moves_its_mean_as_its_members_would`
  (on a cover; the law is unchanged).
- `core/tests/choose.rs`: `a_body_with_no_brain_lives_and_copies` (a cover lives and recruits).
- `core/tests/populations.rs` slow: `genes_do_not_drift_without_selection` (the churn of covers, `cover_kinds` 1 and 2
  in place of `crowd` 1 and 64); `selection_moves_a_nodes_brain_size` (it gives brainless founders 12 hidden units:
  check it; if the lineage change makes them animal nodes in a way the test no longer states, found it on eaters).
- `core/tests/copy.rs`: `crowd_bounds_the_nodes_of_a_cell` gains the covers' bound.

Stay (they use eaters: `testkit::eater`): the four young-node tests of `copy.rs`; `a_tall_member_shades_a_short_one`
and `a_big_node_stores_light_set_aside_as_it_fell` (check them on covers). Deleted: none (the young-node law stays for
animal nodes).

New (few, fast, one law each, in `core/tests/copy.rs`, setup from `worldgen::testkit`):
1. `a_covers_young_mature_by_their_growth_and_its_children_stay_in_it`: a cover with young and adults in steady light:
   the members passing are those `Pass` gives, births add young members at the child's size, elements, energy and
   members are kept (births less deaths).
2. `a_covers_supply_is_the_sum_of_its_stages`: `equiv` equals the sum over an adult stage and a log-normal young stage
   drawn as individuals (to 1%).
3. `seeds_join_the_cover_of_their_kind_or_start_one_where_there_is_room`: a third kind's seeds in a full cell join
   the most alike cover; in a cell with room they start one.

### 6.4 What to watch in a 1,000-year run (island, globe, region; seed 1)

- The books (element and energy drift under 1e-14), no run dies out, every key number in its range at year 1,000.
- Covers per planted cell (at most `cover_kinds`), `root_0_adults` (a steady share, not 0 or 1), births and deaths of
  light-eaters that cycle with the seasons, not with a calendar.
- Gene means of `root_0_*` move as today's do (gas, leaf, copy_log, interval, alloc, child, `cat_<r>_on`), and the
  fecundity genes (`alloc`, `child`, `wet_gene`) are not frozen (2.7, 4).
- Kinds differ by climate: on the globe, the covers' gene means by latitude band in the year-1,000 snapshot (a probe
  script, no tape change).
- The eaters' mass and life, and the share of perceived bodies that are animals (more animal faces per decision).
- Trails (`worn_edges`, `traffic_top2`) on the island as today.
- The land temperature and `evap_land`: plants' light and heat coupling is sensitive (P1.2 moved the land -0.78 K).

## 7. Steps

Each step is one commit on the round's branch. Lines are `core` code lines (`cli/tests/core_size.rs`; 10,444 today,
ceiling 10,643).

### C0. Probe: kinds and stages in today's world (branch `probe/cover-c0`, not merged)

- **What.** On the three bench saves (`benchZ`): per planted cell, the brainless nodes, the alike groups (young and
  adult nodes joined by `body::within`), the share of light-eater adult mass in the largest 1, 2 and 3 groups, young
  and adult members and mass, and over one year how many litters land where no node of their kind is. A test file
  that reads a save and prints.
- **Done when.** `cover_kinds` is chosen: the least K that holds at least 90% of the adult mass of the planted cells on
  all three benches (2 expected), and the expected covers per cell are known for 5.
- **Kill line.** None (a probe). If K must be 4 or more, the gain falls to about x3 a cell: tell the architect before C2.

### C1. Helpers without a law change (level 1)

- **What.** `nodes::add` splits into the pooling of genes and spreads on a body out of its slot (`join_self`) and the
  join of a stored node; `copy::emit`'s path for children who stay is cut out of it. No behaviour change.
- **Files.** `core/src/nodes.rs`, `core/src/copy/mod.rs`.
- **Check.** Level 1: golden bit for bit with and without threads.
- **Done when.** C2 calls them. **Kill line.** None. **Lines.** about +6.

### C2. The law: a light-eater node holds its young (level 4, the main step)

- **What.** `Body::stages` (2.1); `Body::adult`, `equiv`, `height` read them (2.3); `copy::grow` takes the brood from
  the adult stage (2.4); `copy::copy`: the stayers join the cover as young (`join_self`), the seeds are a litter of
  young; `survey::update` runs the stage step where `copy::mature` runs (covers mature inside by `math::Pass`;
  `young::linked`, `link` and `Body::link` only for nodes with a brain); `copy::kin` matches any stage for a cover;
  `nodes::Lot`, `rejoin`, `part` carry and pool stages; founders get all-adult stages; `SNAPSHOT_VERSION` 50; tape
  column `root_<K>_adults` (D11). Tests of 6.3 rewritten and the three new ones. Docs: laws.md 4.2, 4.4 (a light-eater
  node holds its young; young nodes are for nodes with a brain), 6.6, 3.6 (a cover's height); `docs/tape.md`;
  `docs/history.md` section 7; the measure in `docs/lighter/measures.md`.
- **Files.** `core/src/body/mod.rs`, `body/tissue.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`,
  `survey.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs.
- **Check.** Level 4: golden re-pinned (why: a law change); key numbers (3 seeds, about 5 minutes); the slow suite's
  life and climate tests of 6.2; benches remade (`make_benches.sh`, version 50); the debug checks.
- **Measure.** On remade benches, seeds 1 to 3 (PR-S method 3) and counted work, which does not depend on the noise
  of the worlds: light-eater nodes per planted cell, plant updates a year, instructions a plant update, plant
  instructions per planted cell-year (callgrind with `--toggle-collect` on the update of a body with no brain, as PL1).
- **Done when.** Plant instructions per planted cell-year fall by at least half on island22 and region24; every key
  number in range but S50, S50 within its noise; the climate and life tests of 6.2 pass.
- **Kill line.** Plant instructions per planted cell-year fall by less than 10% (owner's rule), or a key number leaves
  its range (not S50), or S50 moves past its noise away from 0.5, or the long-step test fails: then the fallback F1 (8).
- **Lines.** about +95 (stages and their reads +45, growth +8, births in place +15, stage step +20, pooling +10)
  and -20 (the young-node path for brainless nodes, the stage test in `kin`): **net about +75**.
- **Probe first?** No: the key-number test is the gate. But if the land temperature moves more than 0.3 K, stop and
  find why before C3 (the -0.78 K of P1.2 was never explained).

### C3. The bound: at most `cover_kinds` covers in a cell (level 4)

- **What.** `Config::cover_kinds` (validated at least 1; a row in `docs/numbers.md`, mark "speed", in the speed count
  of handover 14.3); `nodes::split` takes its room from it for covers; `copy::land_litter` and `emit`: a litter with no
  alike cover in a full cell joins the most alike (`nodes::add`); `nodes::crowd` for covers joins alike only. `crowd`'s
  row says it is for nodes with a brain.
- **Files.** `core/src/params.rs`, `nodes.rs`, `copy/mod.rs`, `docs/numbers.md`, `docs/laws.md` 4.4, tests
  (`crowd_bounds_the_nodes_of_a_cell`, new test 3).
- **Check.** Level 4 as C2; `SNAPSHOT_VERSION` 51 (a `Config` field).
- **Done when.** At most `cover_kinds` covers in every cell of the three benches; the year on one thread (all
  included) has plants under 10% on island22 and under 20% on region24.
- **Kill line.** The plant instructions per planted cell-year fall by less than 10% against C2, or a key-number or
  climate gate of C2 fails: then keep C2 alone and set `cover_kinds` aside (D2).
- **Lines.** about +20 -5.

### C4. The round's checks of the cover (no code)

- **What.** Callgrind of the three remade benches (the year and its stages, plants all included as in P0); wall time
  on 4 threads (5 interleaved runs under the lock); the key numbers again; the slow suite; 1,000-year runs of the
  island, the globe and the region (one at a time, under the lock), with the watch list of 6.4.
- **Done when.** The measure is in `docs/lighter/measures.md` with a verdict per bench; the architect logs the new
  references (D7).

### C5 (only if needed). The sample on every update (level 4)

If the 1,000-year runs show gene means of light-eaters slower than today's (the sample now carries all selection
inside a cell), `sample_every` 1: the sample is 13% of a cover update, about 1% of the year after C3. One number, its
row in `docs/numbers.md`. Lines 0.

**Core lines in all:** about +100 -25, net about +75 to +95: from 10,444 to about 10,520 to 10,540, under the
ceiling. The cover deletes little code: almost all of a stand's code is shared with animal nodes (the day, the young
law, splits, merges, litters), and it stays for them. The gain is in the work, not in the lines.

### Risks

1. **Climate coupling.** The light and heat of plants reach the ground through their stores, transpiration and
   draws; P1.2 moved the land -0.78 K with no found cause. The cover keeps the day's code whole, so the risk is in the
   mass and the liquid it holds, not in a new formula. Gate: C2's key numbers and the long-step test, whose margin is
   only 2% (X85).
2. **S50.** Pooled stores of young and adults: the stored share may move. Gate: its seeds' noise.
3. **Light-eaters' mass and life.** Supply with stages (2.3) keeps growth near two nodes'; without the log-normal
   factor it would be up to 50% higher. The members of all stages die alike (2.7): the life of light-eaters may move.
4. **Selection of fecundity genes** is weaker inside a cell (2.7, 4). Watch, C5.
5. **Animals.** Fewer plant faces leave more room for animal faces in `SEEN`; covers are larger prey nodes; the
   eaters' key number and the trails must hold. The eaters live mostly on dead matter (handover 13.2), so the risk is
   small.
6. **Judging.** A law step cannot run the base's save. The counted work (nodes, updates, instructions per update) is
   the main measure; the year across worlds counts only above PR-S's bands (19% island22, 11% globe36), which the
   expected cuts (31%, 16%, 51%) pass. A same-world judge needs a one-off converter of a version 49 save into covers on
   the step's branch (not merged: no compatibility code): optional, D6.
7. **Key-number references.** After C2 the world is a new world: later steps of the round need new references (D7).
8. **Size of cells.** Kinds per cell are per cell, so a refined region holds four times the kinds per km²; the scale
   test may notice.

## 8. Alternatives if the cover fails its gate

- **F1: the cover as two bodies per kind** (young and adult nodes, today's `copy::young` whole), with C3's bound on
  kinds (a kind is a linked pair) and the children of a node joining its young node only. It changes less law (no
  stages, no supply sum) and gains less: about 2 x 1.6 = 3.2 nodes a planted cell instead of 8.2, a plant update a
  little cheaper; the island's year about x1.34, the region's about x1.7. It is idea 2 in a strong form, and C3 lands
  on it as it is.
- **F2: ideas 3 and 4 on stands as bodies.** Living bodies do not rot (a reactor without the bare reactions, about 9 in
  10 of a stand's; probe first, X63's -0.78 K) and a cheaper sample (`sample_every` 4, or the sample's gain estimated):
  together perhaps 15% to 20% of a stand update. They also apply on top of the cover or of F1.
- If the stage law itself is wrong (C2's key numbers out), F1 keeps everything else of C3 (the bound, seeds as gene
  flow) and is the shortest way back.

## 9. Decisions for the architect

- **D1.** The cover as one body per kind with two stages (this design), or F1 (two bodies per kind) as the target.
- **D2.** `cover_kinds`: its value (2 proposed, from C0), and whether C3 is built at all.
- **D3.** Seeds of a new kind in a full cell: join the most alike cover (proposed: their matter and members live, the
  gap widens its spread) or die to the ground as dead matter (a cap that kills: a harder rule).
- **D4.** Names: "cover" is taken in code (soft cover, `cover_of`, `cover_median`); this design uses `Stages` and
  `Config::cover_kinds`. Keep, or another word.
- **D5.** Two ways of holding young: node pairs for animals (`copy::young`) and stages for covers, sharing
  `math::Pass`. Accept as one idea in two book-keepings, or plan to move animal nodes to stages later (it would delete
  `copy::young`'s links and maturing as a body, and change animals' laws: not in this round).
- **D6.** Judging: counted work on remade benches (proposed), or also a one-off converter for a same-world judge.
- **D7.** New key-number references after C2 (or C3) for the steps that follow.
- **D8.** The weaker selection of fecundity genes inside a cell (2.7, 4): accepted with the watch and C5.
- **D9.** Order with the other ideas: idea 2 is moot if C3 lands (it is F1 otherwise); ideas 3 and 4 act on the cover's
  update after it; idea 6 meets fewer plant faces.
- **D10.** Snapshot versions 50 (C2) and 51 (C3), and benches remade at each.
- **D11.** The tape column `root_<K>_adults` (the share of members in the adult stage), to watch the stages.

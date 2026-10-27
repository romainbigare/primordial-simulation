# Design: light-eaters as a cover (idea 5)

The round "Lighter world", idea 5. The owner said yes to this law change, and asked for at least three kinds per cell
that do not mix ("trees, shrubs, small plants"), made by genes, and for new kinds that can enter a full cell
(`architect-1.md`). This is the design after the first and second reviews (`review-cover.md`: 17 findings, then 9;
answers in 10). The plan's lane C takes its steps from 7.

## 0. The idea

Today a planted cell holds about 8 light-eater nodes (stands): an adult and a young node of each kind, and young nodes
of unlike litters from the neighbours. Each is a body that updates every 40 days. The cover keeps light-eaters as
**bodies** (one law of physics for every body), and changes two things, in two law steps:

1. **The bound on kinds (C1).** A cell holds at most `cell_kinds` + 1 kinds (`cell_kinds` = 3: the owner's three
   forms, and one place for a newcomer); a new kind starts while fewer are held. A kind is today's adult node with its
   linked young node. A new kind enters a full cell as a group; at a month's start, a cell with more than
   `cell_kinds` kinds with adults merges its most alike pair, so the newcomer place comes free again.
2. **The young inside (C2).** A kind is one body, the **cover**: its young and its adults are two stages of it
   (`Body::young: Option<Young>`). Its children stay in it; its seeds land in a neighbour's cover of their kind (gene
   flow); its young mature inside it by the same flux as today (`math::Pass`). The young keep their own height and
   their own share of the light.

The physics of the day (`body::live`) is unchanged. The gain is fewer bodies: from about 8 a planted cell to at most
4. Names: `Young`, `Young::adults`, `Young::body`, `Young::lit`, `Config::cell_kinds`; none of them is
used today ("cover" is the soft cover of `flow::land`, `cover_of` and `Site::cover`; "stages" are the stages of an
update; the code does not use them for this). "The cover" is a word of the docs only.

**Why bodies and not a field.** Shade (`survey::shade`), the cell sums (`survey::rows`, read by lanes, moving, erosion's
`hold`, perception), the light set aside and the ground's heat draws (`survey/draws.rs`), bites (`feed::take_body`),
the wear of movers (`choose::wear`), shelter (`body::cover_of`), faces and the tape all read light-eaters as bodies.
Half of a stand update is the body's day; a field would write it a second time (CLAUDE.md 2.1), and the last round's
cheaper day laws failed (P1.2, PR-C2).

## 1. Inventory: where light-eaters as bodies are touched today

"Stays": not touched. "Rewritten": changes in C1 or C2. Code that stays for animal nodes is never deleted.

| Place | Today, for stands | With the cover |
|---|---|---|
| `body::live`: `to_growth`, `settle_upkeep`, `keep_warm`, `check_stores`, `shed`; `body/physics.rs` (light, breath, `Reactor::react`, transpiration by `GAS`, soaking, roots, heat); `body/exchange.rs`; `body/pay.rs` | the day | stays (C2 adds the maturing between growth and children, 2.4) |
| `live::starved` | the mean member against the copy mass | rewritten: per stage, the young by their own energy balance (2.5) |
| `Body::adult`, `size_moments`, `set_size`, `height`, `equiv` (`body/tissue.rs`) | the mean member | rewritten for a body with `Young` (2.3) |
| `copy::grow` | past maturity `ALLOC` of growth is brood | rewritten: growth splits between stages by each stage's light less its upkeep, brood from the adults' part (2.4) |
| `copy::copy`, `emit`, `give`, `provisions`, `start` | children join the young node; `DISPERSE` leave as a litter | rewritten: stayers join the cover's young, nothing moves; dispersers are a litter of young (2.4) |
| `copy::kin`, `land_litter`, `back`, `outbox::Entry::Litter` | a litter joins its kind's young node or starts one | rewritten: the newcomer place (3); `kin` matches any stage of a cover |
| `copy::young` (`linked`, `link`, `mature`, `Grown`), `Body::link`, `survey::update`'s maturing | young and adult nodes | stays for animal nodes; a cover matures inside `live` with `math::Pass` |
| `nodes::add`, `rejoin`, `Lot`, `part` | pool genes, spreads, size moments | rewritten: they pool `Young`, and `rejoin` weighs the young's size moments by young members |
| `nodes::split` | room while the cell has fewer than `crowd` - 1 bodies | for light-eaters: room from the bound (3); animals unchanged |
| `nodes::crowd`, `join_alike`, `merge_closest` | monthly merge; covers and animals counted together | stays: covers still count in `crowd` (option (a) of finding 2, 3); for light-eaters, past `cell_kinds` kinds with adults `merge_closest`'s distance has no `alike` limit (3) |
| `copy::select` (sample member) | breeder's equation in a node | stays, `sample_every` 2 (2.6) |
| `body::vary` | children's genes | stays |
| `survey::shade` | one entry per body | rewritten: a cover is two entries, adults and young (2.3) |
| `survey::rows`, `caches`, `flow/air/cell.rs`, `survey/draws.rs`, `air/book.rs` (marks moved by shade) | light, heat, rain | stay (one shade per body, 2.3) |
| `survey::worn`, `choose::wear`, `body::wound`, `lanes` | wear, paths | stay |
| Bond (`bond/`) | light-eaters are never held, inside, formed or joined | stays: nothing changes |
| Choose: faces, `perceive`, `take_body`, `Habit::prey` | animals see and bite stands | stay; 1 to 4 light-eater faces a cell instead of about 8 |
| `Body::interval` (`node_days` 40), `lineage::resize` | pace, brain sizes | stay |
| worldgen founders (`life/kinds.rs`: one node per land cell; fixers whole cells; `life/make.rs`) | adults | C2: founders get `Young` with all members adult |
| tape (`docs/tape.md`): `plants`, `plant_members`, `cover_median`, `cover_max`, `births`, `died_<cause>`, `root_0_*` | | keep their meaning (members, mass, births); C2 adds `root_<K>_adults` |
| snapshot (`tape/src/snapshot.rs`) | version 49 | C1: 50 (a `Config` field); C2: 51 (a `Body` field) |
| viewer | reads the tape | stays |

**Readers of `member_body` and `adult`, and what a cover reads** (finding 4):

| Reader | Cover |
|---|---|
| `tissue.rs:73` `adult` | it holds adults and the adults' member body `m_a` is at or past the copy mass |
| `tissue.rs:83, 87` `size_moments`, `set_size` | the young's moments, about the young's mean member `m_y` (2.2) |
| `live.rs:101` `from`, `live.rs:132` `starved` | per stage: `m_a` and `m_y` before and after (2.5) |
| `grow.rs:20` `needs` | what the adult stage lacks to the copy mass |
| `copy/mod.rs:60` (only adults copy) | `adult` above |
| `copy/mod.rs:255` `kin`, `nodes.rs:36` and `:64` (stage in the key) | no stage: a cover is every stage of its kind |
| `young.rs:20, 47, 51`, `survey.rs:258` | not run for a cover (it matures inside `live`) |
| `mix.rs:26` (partner), `go.rs:62` (`land_move`) | not reached: covers are nodes and never move |

## 2. The new law (C2)

### 2.1 What a cover holds

A cover is a body of a lineage with no brain, one per kind in a cell. It holds what a stand holds: every substance
(64-bit, all members), heat, the brood, gene means and `spread`, the marks of light and rain, `keep`, `draw`, members
`count`, its face. New: `young: Option<Young>`, all `f64`:

```
Young {
    adults: f64,  // share of the members that are adults (at or past the copy mass)
    body: f64,    // share of the living body (`built`) that is the young's
    lit: f64,     // the young stage's shade over the cover's (survey::shade)
}
```

They are shares, so the laws that act on all members alike keep them: deaths of a share (`lose_share`, `thin`), a
bite or wear that tears a share of every member (`wound`), `part`. From them: `n_a = adults · n`, `n_y = n - n_a`,
`m_a = (1 - body) · built / n_a`, `m_y = body · built / n_y`. **One source:** `young` is set where a body is placed
(`Domain::add`) from `Life::brainless`, and set or cleared where `lineage::resize` moves a body between a lineage
with a brain and one without; `survey::update` asserts in debug builds that the two agree. So a body that loses its
brain (X15; none seen in PR-B's 1,000 years) becomes a cover whatever it eats: it starts with `Young` all adult, as a
founder, and counts among its cell's kinds (the bound counts bodies without a brain).

### 2.2 The young's sizes

The young's log10 bodies keep their mean and variance in today's fields `size_lag` and `size_spread`, about `m_y`
instead of the mean member (`size_moments`, `set_size`), pooled by `rejoin` as today with the young's members as the
weight. No new shape: `math::Pass` (the McKendrick steady state, a truncated exponential) fits them as it fits a young
node. A body without `Young` (animals) reads them about its mean member, as today.

### 2.3 What the stages change in the body's numbers

- **Supply** (`Body::equiv`: intake, growth's allowance, the leaf): the sum of the two stages, each by its mean member,
  `n_a m_ref (m_a / m_ref)^b + n_y m_ref (m_y / m_ref)^b`, b = `intake_exp`. This is exactly today's young node plus
  adult node. No spread factor.
- **Maturity** (`adult`): as in the table of 1. A hurt adult stage is immature until healed, as today.
- **Height and light** (`survey::shade`): a cover gives two entries, the adult stage at the height of `m_a` with its
  leaf (its share of `equiv` times the leaf gene) and the young stage at the height of `m_y` with its leaf. Light
  passes from the tallest down as today, so the young sit under their adults (Weiner 1990's one-sided competition) and
  under taller kinds, as today's young nodes do. The body's `shade` is the leaf-weighted mean of the two entries' and
  `Young::lit` the young's over it; marks, rows, rain and draws read the one `shade` as today. The face's height
  (`vis[6]`) is the adults'.
- **Only these go by stage** (finding 15): supply (and so growth, intake, leaf), maturity, height, light and
  starving. The coat (`n m^(2/3)`), skin area, boundary layer and `empty_at` read the mean member: for young at 0.3
  of the copy mass and adults at 1 to 1.5 times it, a few percent (about 5% for a size ratio of 4).

### 2.4 Growth, maturing, children, seeds (in this order, inside `live`)

1. **Growth** (`copy::grow`), by each stage's own energy balance, as a young node and an adult node pay apart today.
   The leaf shares are `ℓ_y = E_y / (E_a + E_y)` and `ℓ_a = 1 - ℓ_y` (E the two terms of `equiv`; each leaf is its
   `equiv` times one leaf gene). The young catch the share `ℓ_y · lit` of the cover's light, the adults `1 - ℓ_y · lit`
   (exact, as the body's shade is the leaf-weighted mean of the two entries'). Each stage's balance over the update is
   its share of the light its reactions stored (`store_light`) less its share by mass of the update's upkeep
   (`settle_upkeep`'s cost): `B_y = ℓ_y lit P - μ_y U`, `B_a = (1 - ℓ_y lit) P - (1 - μ_y) U`, μ_y = `Young::body`.
   What the cover builds splits by the positive balances: the young take `max(B_y, 0) / (max(B_y, 0) + max(B_a, 0))`
   (by mass when both are 0). Of the adults' part, what brings `m_a` back to the copy mass is body, and of the rest the
   share `ALLOC` is brood (today's rule on the adult stage). The young's part is young body. **The young hold no
   reserve:** a negative `B_y` is young body lost (`-B_y` over the energy of burning a kg of body matter, reaction 21),
   which `starved` reads (2.5). It breaks no book: the cover's store paid the upkeep, and the young that die go to the
   ground with their matter, as any dead.
2. **Maturing** (new, in `live` after `to_growth` and before `copy::copy`, so no newborn is in the young yet; finding
   3). The young's log10 bodies rose by `d = log10(m_y after / m_y before)`, members fixed. `Pass::new(mean, var,
   log10 copy_mass, d)` gives the share of young members that pass and the share of young body they hold, and the
   moments of those left. `adults += passed · (1 - adults)`, `body -= held · body`, the young's moments set to those
   left. **One size** (variance 0, a new cover, finding 16): `Pass` gives none, and all young pass together when `m_y`
   reaches the copy mass, as a young node turns adult whole today (`young.rs:47`).
3. **Children** (`copy::copy`; a cover has one parent). `births = brood / child matter - BROOD_HELD · n_a`. A share
   `DISPERSE` leave as seeds; the rest stay: their genes are drawn as today (`body::vary`: drift of the group's mean,
   variance left, mutation added; Lande 1976), and they join the cover's young: genes and spreads pool by members (the
   pooling of `nodes::add` on the body itself), `n += births`, the young's moments pool with log10 of the child's body
   and variance 0 (`rejoin`'s formula), `Young::body` takes the children's body, `adults = n_a / n`. **No store
   moves**: the brood's matter becomes young body in the same store; the provisions are already the cover's. The `Born`
   event is recorded as today.
4. **Seeds.** As today's litter (`emit`, `give` takes their matter, provisions and heat out of the cover,
   `Entry::Litter`, `land_litter` at the batch's end in their cell's domain), with `Young` all young. They join the
   cover of their kind there (`kin`) by `nodes::add`: genes, spreads, members and young pool. This is gene flow between
   cells weighted by seed output (Wright's island model). Where their kind is not held: the newcomer rule (3).

### 2.5 Starving per stage (finding 4)

`starved` today kills `1 - (now / was)^a` with `was = min(from, copy mass)`. A cover takes it per stage: the adults with
`now = m_a`, `was = min(m_a before, copy mass)` (an adult that shrinks toward the copy mass by `ALLOC` is not
starving), the young with `now = m_y` less the body their negative balance lost (2.4), `was = m_y before`. So young
under their adults or a taller kind thin by shade, as a shaded young node does today (Weiner 1990). The members that
die are `n_a f_a + n_y f_y`; they take their stages' body with them (matter by stage, the other stores by the share of
members), the brood falls by the adults' share `f_a` (it is the adults' matter; `Body::lose` would keep it at the
mean share), and `adults` and `body` are set from what is left. Then `lose_share`'s rule (one law for nodes and
individuals) as today.

### 2.6 Competition, kinds and selection

- **Light**: by height, two entries a cover (2.3). **Liquid and minerals**: each cover soaks and takes by its genes from
  the same ground, in id order in the batch, as stands do.
- **Selection inside a cover**: `copy::select`, the sample member one standard deviation off in one drawn gene; the
  breeder's equation (Lande 1976; Robertson 1966). **Cadence: `sample_every` stays 2.** Each cover is sampled as
  often as a stand was, so each gene's mean moves at today's rate per node; 1 would add about 10% to a cover update for
  no measured need; 4 was dropped (the cap makes a jump of 4 standard deviations, decision 4 of `architect-1.md`).
- **Selection between kinds and cells**: kinds of a cell compete for light, liquid and minerals; covers with more seeds
  send more genes to their neighbours. The genes no update reads before growth (`select::UNREAD`: `CHILD`, `MUT`,
  `DISPERSE`, `WET`, `ALLOC`) are selected only this way (inside a cell no sample runs on them, today or here). C0
  measures how much of today's adult mass came from unlike litters, which the bound acts on (finding 8).

### 2.7 Eaten, worn, the books

- **Eaten**: `feed::take_body` on the cover's face, unchanged: Holling's type II on the swept share of its members,
  `c_handle`, the breaking energy (`Body::breaking`), one `transfer` of stores and heat (both sides, 64-bit), the
  torn spill to the ground (`wound`, `thin`, `to_ground`).
- **Worn**: `choose::wear` shares the cell's push energy by mass among its bodies; unchanged.
- **Books**: every move is today's (`store::Moves`, `book_by_array`, `transfer`, `give_all`). Children that stay move
  nothing. Seeds leave by `give` and join by `rejoin`: sums of 64-bit numbers. `check::drift` and the debug checks of
  overdraw hold them.
- **Principles**: no calendar (the tempo by temperature); real sizes (members, kg, kg/m²); no Earth word.
- **The reactor**: the day's, so it follows B1 whatever lands first (L5 is dropped, decision 1): with B1's variant (d)
  the decay of reactions 1 and 21 does not run in a living cover, as in any living body.

### 2.8 What the cover cannot do that stands do today

1. A cell holds at most `cell_kinds` + 1 = 4 kinds (today unbounded: up to 26 to 28 nodes).
2. A kind's young and adults share one store and one temperature. They keep their own light, growth and starving (the
   young by their own energy balance, 2.4), but the young draw on no reserve of their own and the adults' store pays
   the upkeep of the whole cover.
3. Seeds of a kind not held, landing while the cell holds `cell_kinds` + 1 kinds, join the most alike cover: their
   genes mix. Past `cell_kinds` kinds with adults, the most alike pair of kinds merges (3).
4. The fecundity genes are selected only between kinds and between cells (2.6).
5. A kind's young are one group with one size spread.

**What stays possible**: eaten, trampled and worn into paths; shading, transpiring, warming the ground; rotting into
minerals; dead frame as fuel; solids for parts through an eater; kinds by climate and by form in one cell (trees,
shrubs, small plants by their genes, up to `cell_kinds`); adapting to places (the sample, gene flow, clines: Haldane
1948, Slatkin 1973); invasion of a full cell as a group (3). **Tending** stays possible by physics: an animal puts
minerals or liquid on a cell's ground (`feed::put`), keeps eaters off, wears away one kind by moving through, eats one
kind and spares another (each kind has a face). No watch-for item of M1 to M3 or later is blocked.

## 3. Kinds and the bound (C1, kept by C2)

- **A kind** in a cell: an adult node and its linked young node (C1), then one cover (C2). A kind **without adults** is a
  newcomer (a young node with no adult in C1, a cover with `adults` 0 in C2).
- **`Config::cell_kinds` = 3** (a speed setting with a physical side effect, like `crowd`; the owner's three forms,
  decision (4) of the second round). **The bound is one number: a cell holds at most `cell_kinds` + 1 kinds of bodies
  without a brain, whatever they eat, and a new kind starts only while fewer are held.** The extra place is the
  newcomer's.
- **Start of a kind.** A litter (staying or landing) with no kind of its genes in the cell (`kin`) starts a kind if
  the cell holds fewer than `cell_kinds` + 1; else it joins the most alike kind (`nodes::add`; its gap widens that
  kind's spread up to `body::most`). A split (`nodes::split`, halves not alike) needs two free places, so it never
  takes the newcomer's.
- **The place comes free** (second review, finding 1). At each month's start (`nodes::crowd`), a cell that holds more
  than `cell_kinds` kinds with adults merges its most alike pair, the smaller into the larger: `merge_closest`'s
  distance (`nodes.rs:59-94`) without its `alike` limit for bodies without a brain. It is a speed tool like `crowd`,
  not a choice by fitness: the most alike forms merge, so different forms stay apart while there are no more than
  `cell_kinds` of them. A resident that holds no adults (after a cold spell) is a kind like any other.
- **Invasion.** A newcomer grows in the cell as a group (its genes together, not pooled) under the residents' shade.
  When it matures the cell holds `cell_kinds` + 1 kinds with adults; the next month the most alike two merge and the
  place is free for the next newcomer, so a landscape can be invaded again and again. A kind also ends as any node
  ends: fewer than one member (outgrown in light, liquid or minerals, eaten, frozen). The cost stays bounded: at most
  `cell_kinds` + 1 bodies (C2) a cell, `2 (cell_kinds + 1)` in C1. Forms come from genes only (height from materials
  and size, `Body::height`); core names none.
- **Animals and `crowd`** (finding 2): option (a), covers still count in `crowd`'s two tests (`nodes.rs:16`, `:214`),
  no code change for animals. With 3 or 4 covers a planted cell is still at or past `crowd` (4) with one animal node,
  as with today's 8 stands, so animal splits stay rare and `merge_closest` runs where two animal nodes meet, as
  today. Where a cell holds fewer covers (the globe: about 1.6 kinds), animals find more room: C0 measures animal nodes
  per planted cell and animal splits a year today, and C1 and C2 are gated on them. If they move past their seeds'
  noise, option (b) (`crowd` counts nodes with a brain only, with `crowd` set from C0 so the counts hold) is a stated
  change of the animals' law in its own commit.

## 4. Cadence and threads

- A cover updates in the body batches at `node_days` (40), as a stand. Growth, maturing, children and seeds run inside
  its update, in the order of 2.4. The shade with two entries runs in the survey (`survey::shade`), monthly joins of
  alike covers in `nodes::crowd`. Nothing new at the physics step.
- Covers are bodies of their domain: they update in the parallel phase of `survey::batch` (`on_domains`,
  `par::longest_first`), domain by domain, cell by cell, by id; one writer, the domain (CLAUDE.md 3.6). Children that
  stay write only the cover. Seeds wait in the outbox and land at the batch's end in their cell's domain
  (`outbox::land`), in the order of the domains they come from, then the order made; the newcomer rule reads only that
  domain. The world is the same with and without threads.

## 5. Cost (finding 1)

**Prices** (one thread, instructions, after PL1): a stand update about 84k on average; an adult-node update about 100k
(the day 48k, copying about 32k: two `vary` of 127 draws, the join of the stayers, a litter; the sample 10k; shed, soil
and `see` 8k; the review's split, C0 measures it); a young-node update about 75k. **A cover update is an adult-node
update plus maturing (about 1k) and the second shade entry: about 100k, 19 µs.** It copies at every update.

**Today, plant updates per planted cell-year:** island22 85 x 84k = 7.1 M; globe36 46 x 88k = 4.0 M; region24 104 x 86k
= 8.9 M. Plants all included are 37% / 19% / 60% of the year (P0).

**Bodies a planted cell** (`cell_kinds` 3, so at most 4 kinds): after C1, about two per kind (adult and young) plus
a newcomer; after C2, one per kind, between today's kinds (adult nodes per planted cell: 2.7 / 1.6 / 3.5, P0) plus a
newcomer and the bound (4). Cells fill toward the bound (finding 11), so the estimate takes the upper end: island 3
to 4, globe 2 to 3, region 3.5 to 4 covers, at 9.1 updates a year.

**Litters** (second review, finding 5) go by copying updates, not by plant work: a litter leaves at each update of an
adult node or a cover. `outbox::deliver` is 3.4% / 5.0% / 3.0% of the year and the plants make 62% / 56% / 94% of its
litters: about 2.1% / 2.8% / 2.8% of the year today. Copying updates a planted cell-year: today about 28 / 15 / 32;
with covers 27 to 36 / 18 to 27 / 32 to 36. So the litters cost about 2.0 to 2.7% / 3.5 to 5.3% / 2.9 to 3.3% of
today's year after C2. Faces, booking and the survey fall with the plant work.

| | island22 | globe36 | region24 |
|---|---|---|---|
| C1: bodies a planted cell, mean (today 8.2 / 4.6 / 10.3) | about 6.4 to 7 | about 4.2 | about 7 to 8 |
| C1: plant work a planted cell-year | x1.15 to x1.4 | x1.0 to x1.1 | x1.25 to x1.45 |
| C1: the year, 1 thread | x1.05 to x1.12 | x1.00 to x1.02 | x1.14 to x1.23 |
| C2: covers a planted cell | 3 to 4 | 2 to 3 | 3.5 to 4 |
| C2: plant updates a planted cell-year, M instructions | 2.7 to 3.6 (x2.0 to x2.6) | 1.8 to 2.7 (x1.5 to x2.2) | 3.2 to 3.6 (x2.4 to x2.8) |
| C2: plants all included, share of today's year (litters too) | 15% to 21% | 11% to 16% | 23% to 27% |
| **C2: the year, 1 thread** | **x1.20 to x1.28 (-16 to -22%)** | **x1.03 to x1.09 (-3 to -8%)** | **x1.49 to x1.58 (-33 to -37%)** |

(The year after = 1 - p + (p - l) / x + l', p the plants' all-included share, l their litters today, l' after.) C1
cuts only the young nodes of unlike litters past one newcomer (today about 2.7 a planted cell on the island, `young`
5.4 against `adult` 2.7); it is the base of C2 and has no speed line. The globe gains least: its cells hold few stands
today, and its litters rise. G-C (-15% island22, -30% region24) passes at the estimate's both ends, with 1 to 7 points
on the island. **`cell_kinds` 4** (5 kinds) would fail G-C by these prices (island about -13%, region about -29%), so
if C0's forms need 4, G-C is restated before C1. **The next lever** is the copy work, a third of a cover update (two
`vary` calls, the join, the litter): a law step of its own after G-C (C4 of the plan), not in this design.

## 6. What must still hold (level 4)

**Key numbers** (`the_key_numbers_stay_in_their_ranges`, 3 seeds): ground temperature on land (291.79 +-1), seasonal
swing, rain on land, heat-absorbing gas, river cells, land worn down, light-eaters 10.38 +-0.3 log10 kg (today 10.24,
0.16 above the floor), eaters, life of light-eaters (1.81 x/ 2, members over deaths), life of eaters. S50 is the known
failure and must not move past its seeds' noise (about +-0.004) away from 0.5. References: "after AIR4.0" of
`docs/speed-x10/measures.md`, or the round's own if B1 lands first (decision 1). After C2 the architect takes new
references (D7).

**Animals**: animal nodes per planted cell and animal updates a year within their seeds' noise of today (3 seeds,
remade benches); the eaters' key numbers.

**Climate and life tests**: `the_long_step_keeps_the_climate` (margin 2%, X85), `a_globe_without_life_keeps_an_earth_like_climate`,
`the_ground_under_dense_canopies_stays_near_its_neighbours` (S83), `the_pace_of_nodes_keeps_life`,
`the_batches_of_bodies_keep_life`, `the_eaters_live_and_stop_growing`, `the_laws_do_not_depend_on_the_size_of_the_cells`
(a quarter holds as many kinds as a square: watch), `nodes_and_individuals_make_the_same_world`, `a_century_keeps_its_books`,
golden (re-pinned in C1 and C2).

**Tests rewritten** (they state the law of light-eater nodes):
- `populations.rs`: `a_node_spreads_with_its_children_and_splits_when_too_varied` (C2: children join the cover's young;
  it splits when too varied while there is a place), `dispersing_children_of_a_varied_node_join_their_kind` (seeds
  join their kind's cover), `selection_inside_a_node_moves_its_mean_as_its_members_would` (on a cover).
- `choose.rs`: `a_body_with_no_brain_lives_and_copies` (a cover lives and recruits).
- slow: `genes_do_not_drift_without_selection` (covers, `cell_kinds` 3 in place of `crowd` 1 and 64);
  `selection_moves_a_nodes_brain_size` (it gives brainless founders 12 hidden units: founded on eaters if the lineage
  change no longer states it).
- `copy.rs`: `crowd_bounds_the_nodes_of_a_cell` states both: covers count in `crowd` for animals, and the cell's kinds
  stay within `cell_kinds` + 1.
- Stay (eaters): the four young-node tests of `copy.rs`. Check on covers: `a_tall_member_shades_a_short_one`,
  `a_big_node_stores_light_set_aside_as_it_fell`. Deleted: none.

**Tests new** (fast, `worldgen::testkit` setup):
1. `a_better_kind_enters_full_cells` (C1): a strip of cells full of kinds; a kind that grows more seeded at one end;
   after N updates it holds the most mass in the far cell. Then a **second** better kind, of another form, seeded at
   the same end: it takes the strip too. No cell ever holds more than `cell_kinds` + 1 kinds at a month's start.
2. `a_cover_gives_what_its_young_and_adult_nodes_give` (C2): a cover and a young-adult pair of the same genes and
   sizes, about 10 updates under the shade of a taller kind: the members that mature, the births and the mass agree
   to 5%, the young's deaths within a factor of 2; both keep elements, energy and members (births less deaths).
3. `a_covers_young_mature_before_its_children_join` (C2): an update with many births matures the young members `Pass`
   gives from the growth alone; with one size, all young pass together when they reach the copy mass.

**Watch in 1,000 years** (island, globe, region; seed 1): books under 1e-14; no die-out; key numbers in range;
covers per planted cell (at most `cell_kinds` + 1) and how often a newcomer becomes a kind; `root_0_adults` steady;
gene means of `root_0_*` move as today's (gas, leaf, copy_log, interval, alloc, child, `cat_<r>_on`); forms side by
side (the spread of height and copy mass among a cell's covers in the year-1,000 snapshot, a probe script); kinds by
latitude on the globe; eaters' mass and life; trails (`worn_edges`, `traffic_top2`); land temperature and `evap_land`.

## 7. Steps (lane C)

Lines are `core` code lines (`cli/tests/core_size.rs`; about 10,444 today, ceiling 10,643). "Counted" = callgrind on
one thread, one year from the remade benches (`make_benches.sh`, the step's snapshot version), seeds 1 to 3, with
`--toggle-collect` on the update of a body without a brain (as PL1): the year's instructions, the plant work per
planted cell-year, the planted cells, the animal updates a year, bodies per planted cell, `outbox::deliver`. Across worlds the year alone
counts only above PR-S's bands (19% island22, 11% globe36); the counted parts carry the verdict.

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| C0 | Probe (branch `probe/cover-c0`, not merged), on the three bench saves and one year of each: stands per planted cell by stage; alike groups (adult and young by `body::within`); **growth forms side by side** (groups unlike in the form genes: `COPY_MASS`, `LEAF`, the keep genes of the frame and lime, `STRENGTH`; and the heights `Body::height`); share of adult mass in the largest 1 to 4 groups; litters a planted cell-year that land where no node of their kind is, and the adult mass a year later that came from them; animal nodes per planted cell, animal splits and `merge_closest` runs a year; the spread of the form genes' cell means across the land cells of globe36 (kinds by climate, C1's reference); callgrind of a stand update split by `b.adult()` | a test file on the branch | probe | the numbers in `docs/lighter/measures.md` C0; `cell_kinds` is 3 (decided); if the forms side by side need 4, the architect restates G-C before C1 (4 fails G-C as priced, 5) | none (if the adult update is over 120k, the architect is told before C1) |
| C1 | **The bound on kinds** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the cell holds fewer than `cell_kinds` + 1 kinds, else the litter joins the most alike; `nodes::split` of a light-eater node needs two free places; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (`merge_closest` without the `alike` limit for bodies without a brain); `crowd` unchanged for animals; snapshot 50; test 1 (two invasions) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (and the mean per planted cell measured); test 1 passes; animal nodes per planted cell and animal updates a year within their seeds' noise; key numbers in range (S50 within noise); the climate and life tests of 6 pass; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's); the counted year measured, not gated | **no speed line** (it is the base of C2; G-C judges the speed): animal updates a year or animal nodes per planted cell past their seeds' noise, a key number out of range or S50 past its noise, a climate test failing, test 1 failing, or kinds by climate lost: C stops; G-C fails |
| C2 | **The young inside** (2): `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth split by each stage's energy balance (light share less mass share of upkeep); maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood falling with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage for covers; founders and bodies that lose their brain all adult; snapshot 51; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4 (one law, the flux by `Pass`, with two holders: node pairs for animals, stages for covers), 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; counted plant work per planted cell-year x1.4 against C1 on island22 and region24 (`outbox::deliver` counted with it); light-eater members per planted cell within a factor of 2 of C1's; test 2 passes; animals, key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 (else stop and find why: P1.2's -0.78 K was never explained) | the counted year falls less than 5% against C1 on island22 or less than 8% on region24, or the animal, key-number, climate or kinds lines of C1 fail: C1 stays, C2 is not merged |
| C3 | **The checks** (no code): counted runs of the three benches, plants all included as in P0; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **The cover's gate** (after C3, `cell_kinds` 3): the whole year, one thread, counted, at least -15% on island22 and -30% on region24, and not worse on globe36 (estimate: -16 to -22%, -33 to -37%, -3 to -8%); animal nodes per planted cell and animal updates a year within their seeds' noise of today; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 alone may stay if it passed its own lines); A3 and A4 start (phase 2) |

**Lines.** C1 about +28 -5 (the newcomer rule, the split's room, the merge past `cell_kinds`). C2 about +123 -10
(`Young` and its reads 25, shade entries 12, growth split by balances 18, maturing in `live` 20, children in place 15,
starving per stage and the brood 14, pooling 10, placement and resize 6, the rest 3). **Net about +136: from about
10,444 to about 10,580, under the ceiling.** The cover cannot delete more than it adds: the young law, splits, merges
and litters stay for animal nodes, and the day is shared (L4 as rewritten).

**Risks.** (1) The climate coupling of plants (P1.2's -0.78 K; the long-step test's 2% margin): the day's code is
whole, so the risk is in mass and liquid held; gated in C1 and C2. (2) S50 with pooled stores. (3) Light-eaters' mass
(0.16 above its floor) and life: shared stores, starving per stage. (4) Animals: `crowd` on cells with fewer covers
(the globe), more animal faces in `SEEN`; gated. (5) Fecundity genes selected only between kinds and cells. (6) A
newcomer that never matures (kept alive by seed rain) holds the extra place, as the merge counts kinds with adults;
test 1 and the watch (how often a newcomer becomes a kind) measure it; if it shows, the plan's remedy (L23: the
place frees when the newcomer has not matured within its gene's life span). (9) The island's margin to G-C is 1 to 7
points.
(7) Judging across worlds: the counted parts carry the verdict. (8) Kinds are per cell: a refined region holds four
times the kinds per km² (the scale test).

## 8. Alternatives if the gate fails

- **C1 alone** (the bound on young-adult pairs) if C2 fails its lines: about x1.05 to x1.12 on the island, x1.14 to
  x1.23 on the region, about nothing on the globe. It keeps the owner's kinds and invasion rule.
- **A3 and A4** (phase 2, stands as bodies): stands of a full cell merge past `crowd`; the sample runs only the stages
  its gene reads. B1 (living bodies do not rot) acts on stands and covers alike.

## 9. Decisions D1 to D11

| | Decision |
|---|---|
| D1 | Settled (decision 9): the cover with stages, after the bound on young-adult pairs. |
| D2 | Settled: `cell_kinds` = 3 (the owner's three forms; with the newcomer place at most 4 kinds a cell). |
| D3 | Settled (second round): at most `cell_kinds` + 1 kinds, a new kind starts while fewer are held; past `cell_kinds` kinds with adults the most alike pair merges monthly, without the `alike` limit, so the place comes free and invasions repeat. |
| D4 | Settled (names that clash with nothing): `Body::young`, `Young`, `Config::cell_kinds`; "cover" only in docs. |
| D5 | Proposed: one law (the McKendrick flux by `math::Pass`) with two holders, node pairs for animals (their young and adults act apart) and stages for covers; `rejoin` is the one place that pools size moments. Said in `laws.md` 4.4. |
| D6 | Proposed: counted parts on remade benches (above), no converter of old saves. Why: a converter is code nobody keeps, and the counted parts are robust to the world's noise. |
| D7 | Proposed: new key-number references on C2's commit, for the steps after it (the cover changes the world). |
| D8 | Proposed: accept that `CHILD`, `MUT`, `DISPERSE`, `WET`, `ALLOC` are selected only between kinds and cells, with C0's number (the adult mass from unlike litters) and the 1,000-year watch. No C5: `sample_every` 1 cannot reach them (`select::UNREAD`). |
| D9 | Settled: idea 2 and A1 dropped before the cover; B1 and C in any order. |
| D10 | Snapshot 50 in C1 (a `Config` field), 51 in C2 (a `Body` field); benches remade at each. |
| D11 | Proposed: the tape column `root_<K>_adults` (share of members that are adults): one column to watch the stages. |

## 10. Answers to the review (`review-cover.md`)

1. Fixed: a cover update priced as an adult-node update (about 100k), covers at the upper end of the bound; gains
   restated (5); C0 measures the adult and young split.
2. Fixed: option (a), covers count in `crowd`; C0 measures the animals today; C1, C2 and G-C gate animal nodes and
   animal updates; option (b) only as its own stated law change (3).
3. Fixed: maturing runs in `live` after growth and before children (2.4, 4).
4. Fixed: `starved` per stage (2.5), and every reader of `member_body` and `adult` listed with what a cover reads (1).
5. Fixed: the young's log moments in `size_lag` and `size_spread`, pooled by `rejoin`; no log-normal (2.2).
6. Fixed: no supply factor; the stages summed as today's two nodes (2.3); the circular test dropped.
7. Fixed: two shade entries, the young at their own height under their adults (2.3), growth split by light (2.4).
8. Fixed: the newcomer place lets a new kind enter a full cell whole (3); test 1 (a better kind enters full cells); C0
   measures what the bound acts on; C5 removed, D8 says plainly how fecundity genes are selected.
9. Answered: the cover follows B1 through the shared day (2.7; L5 dropped); lines net about +125 and the plan's "delete
   more than it adds" must change (7); two holders of one law, `rejoin` the one place (D5).
10. Fixed: C1 is the bound on today's pairs, C2 the stages; each gated on the whole year and on animal updates (7).
11. Fixed: the estimate takes covers near the bound (5); C0 counts the litters into cells with no node of their kind
    as a rate.
12. Fixed: test 2 (cover against a pair) and test 1 (invasion) added; the circular test dropped; the `crowd` test
    states both laws (6).
13. Fixed: `Young`, `Body::young`, `Config::cell_kinds`; no `Stages`, no `cover_kinds`.
14. Fixed: `Young`'s numbers are `f64`.
15. Fixed: 2.3 says which laws go by stage and which read the mean member, with the size of the error.
16. Fixed: one size, all young pass together at the copy mass (2.4).
17. Fixed: `young` set from `brainless` at placement and at `lineage::resize`; a debug assertion that they agree (2.1).

**Second review** (same file, section "Second review"):

1. Fixed: the bound is one number (`cell_kinds` + 1 kinds); past `cell_kinds` kinds with adults the most alike pair
   merges at the monthly survey, without the `alike` limit, so the place comes free (3); test 1 has a second invasion.
2. Fixed: the young catch `ℓ_y · lit` of the cover's light, `ℓ_y = E_y / (E_a + E_y)`, written once (2.4).
3. Fixed by the first fix: each stage's growth is its light share less its mass share of upkeep; the young hold no
   reserve, a negative balance is young body lost and `starved` thins them (2.4, 2.5); no law breaks (the books hold,
   the dead go to the ground); test 2 runs about 10 updates under a taller kind; C2 gates light-eater members.
4. Fixed: `cell_kinds` = 3; C1 has no speed line (law, animals, key numbers, kinds by climate); G-C as written,
   which `cell_kinds` 4 would fail by these prices (5).
5. Fixed: litters priced by copying updates (5); `outbox::deliver` counted in C2 and G-C.
6. Fixed: C1 at most `2 (cell_kinds + 1)` = 8 and C2 at most `cell_kinds` + 1 = 4 bodies without a brain in every
   cell at each month's start; the mean per planted cell is a separate number (5, 7).
7. Fixed: the plan's section 1 and lane C take this design's gains and steps (same commit).
8. Fixed: a body that loses its brain becomes a cover, all adult, and counts among the kinds (2.1).
9. Fixed: when the stages die apart, the brood falls by the adults' share (2.5).

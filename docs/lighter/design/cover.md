# Design: light-eaters as a cover (idea 5)

The round "Lighter world", idea 5. The owner said yes to this law change, and asked for at least three kinds per cell
that do not mix ("trees, shrubs, small plants"), made by genes, and for new kinds that can enter a full cell
(`architect-1.md`). This is the design after the first and second reviews (`review-cover.md`: 17 findings, then 9;
answers in 10), restated from probe C0's counts (`docs/lighter/measures.md`, "C0: plants per cell today", branch
`probe/c0`) and the architect's decisions 26 and 37 (`architect-3.md`). The plan's lane C takes its steps from 7.

## 0. The idea

Today a planted cell holds about 6 light-eater nodes (stands; C0 on `benchZ`: 6.2 / 5.6 / 6.9): an adult and a young
node of each kind (2.1 to 2.4 kinds with adults), and 1.6 to 2.1 young-only kinds from unlike litters of the
neighbours, which hold about 1% of the young mass and rarely mature. Each is a body that updates every 40 days. The cover keeps light-eaters as
**bodies** (one law of physics for every body), and changes two things, in two law steps:

1. **The bound on kinds (C1).** A cell holds at most `cell_kinds` + 1 kinds (`cell_kinds` = 3: the owner's three
   forms, and one place for a newcomer, a kind that has not yet held adults); a new kind starts only while the place
   is free. A kind is today's adult node with its linked young node. A new kind enters a full cell as a group; at a
   month's start, a cell with more than `cell_kinds` kinds with adults merges its most alike pair, and a newcomer that
   has not matured within its genes' life span joins its most alike kind (the L23 clock, from the start), so the
   place comes free again. Before it, **C1a**: `crowd` counts bodies with a brain only (3).
2. **The young inside (C2).** A kind is one body, the **cover**: its young and its adults are two stages of it
   (`Body::young: Option<Young>`). Its children stay in it; its seeds land in a neighbour's cover of their kind (gene
   flow); its young mature inside it by the same flux as today (`math::Pass`). The young keep their own height and
   their own share of the light.

The physics of the day (`body::live`) is unchanged. The gain is fewer bodies: from about 6 a planted cell to about 4.3 to 5.8
(C1) and 2.6 to 3.4 (C2), today's count to the fill (5). Names: `Young`, `Young::adults`, `Young::body`, `Young::lit`, `Config::cell_kinds`; none of them is
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
| `nodes::split` | room while the cell has fewer than `crowd` - 1 bodies | for light-eaters: room from the bound (3); for animals `crowd` counts bodies with a brain (C1a) |
| `nodes::crowd`, `join_alike`, `merge_closest` | monthly merge; covers and animals counted together | C1a: `crowd` counts bodies with a brain only, value 1 (3); for light-eaters, past `cell_kinds` kinds with adults `merge_closest`'s distance has no `alike` limit, and the newcomer's clock (3) |
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
    held: bool,   // it has held adults (a newcomer has not: its clock runs, 3)
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
   reserve:** a negative `B_y` sets only the young's death share, by `starved`'s law (2.5), from the body the deficit
   would have cost (`-B_y` over the energy per kg of body matter that growth reads from the build, `chem.build`, as
   `copy::grow` does). No body moves between the stages: only the dead take body away (`thin`, to the ground), and
   `Young::body` changes only by deaths, growth and maturing. The survivors' upkeep is paid from the shared store.
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
starving), the young with `now = m_y` less the body their deficit would have cost (2.4), `was = m_y before`. So young
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
- **The reactor**: the day's, as every body's, with B1's skip of bare reactions too slow to matter (`kernel.rs`,
  `TOO_SLOW`, `195bbc2`): the cover takes it with no code of its own.

### 2.8 What the cover cannot do that stands do today

1. A cell holds at most `cell_kinds` + 1 = 4 kinds (today unbounded: up to 26 to 28 nodes).
2. A kind's young and adults share one store and one temperature. They keep their own light, growth and starving (the
   young by their own energy balance, 2.4), but the young draw on no reserve of their own: the shared store pays the
   upkeep of the whole cover, and a young deficit kills young members without taking body from the survivors.
3. Seeds of a kind not held, landing while the newcomer place is taken, join the most alike cover: their genes mix.
   A newcomer gets one life span to mature, else it joins its most alike kind (3). Past `cell_kinds` kinds with adults,
   the most alike pair of kinds merges (3). So a better kind of a form the cell holds enters only as gene flow: when it
   matures it is the smaller of the most alike pair and is merged into the resident. A new form enters whole.
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
- **Start of a kind.** A litter (staying or landing) with no kind of its genes in the cell (`kin`) starts a kind only
  if the newcomer place is free: the cell holds fewer than `cell_kinds` + 1 kinds and none that has not yet held
  adults. Else it joins the most alike kind (`nodes::add`; its gap widens that kind's spread up to `body::most`). A
  light-eater node splits (`nodes::split`, halves not alike) only while its cell holds fewer than `cell_kinds` kinds
  with adults, never into the newcomer place (decision 46). Unlike litters that find the place taken pool into a kind
  and widen its spread past the split threshold (third review 1), so splits refill the cell up to `cell_kinds` kinds
  with adults: the estimate (5) takes that fill as its low end.
- **The place comes free** (second review, finding 1). At each month's start (`nodes::crowd`), a cell that holds more
  than `cell_kinds` kinds with adults merges its most alike pair, the smaller into the larger: `merge_closest`'s
  distance (`nodes.rs:59-94`) without its `alike` limit for bodies without a brain. It is a speed tool like `crowd`,
  not a choice by fitness: the most alike forms merge, so different forms stay apart while there are no more than
  `cell_kinds` of them.
- **The clock (L23, from the start; decision 46).** C0: 2 to 4 litters a planted cell-year land where no node of their
  kind is, and young-only kinds hold about 1% of the young mass and almost never mature (0.005% to 0.2% of the adult
  mass a year later). So the place will almost always be taken. At each month's start, **every kind without adults**
  older than its life span joins its most alike kind. Its age counts from the later of the day it started and the
  last day it held adults (C1: the young node's `Body::born`, so an adult-less resident joins its nearest kind within
  a life span too; C2: `Young::held`, the last day it held adults, 0 for never). Its life span is the age law's: the
  life its genes allow at its cell's ground temperature now (`w.t_ground_last(i)`) and its activity, `e_life`
  10^`REPAIR` over its upkeep per kg and day there (`pay::upkeep`'s rate, through `upkeep_pace`): the rate law, no
  calendar and no new state. Its limit: the rate now, not its integral over the newcomer's life. A newcomer that
  matures in one life span becomes a kind; one that cannot is gene flow into its nearest kind.
- **Invasion.** A newcomer grows in the cell as a group (its genes together, not pooled) under the residents' shade.
  When it matures the cell holds `cell_kinds` + 1 kinds with adults; the next month the most alike two merge and the
  place is free for the next newcomer, so a landscape can be invaded again and again. A kind also ends as any node
  ends: fewer than one member. The cost stays bounded: at most `cell_kinds` + 1 bodies (C2) a cell,
  `2 (cell_kinds + 1)` in C1. Forms come from genes only (height from materials and size, `Body::height`); core names
  none. C0: today 3 form groups hold all the adult mass, and tall and short forms side by side are rare (one form at
  least twice as tall as another in 5% to 8% of cells): the bound keeps today's forms; "trees, shrubs, small plants"
  side by side is what it must allow, not what it keeps.
- **Animals and `crowd`: option (b), step C1a** (finding 2; C0 decides it; decision 46). Today no animal splits in a
  planted cell (a split needs fewer than `crowd` - 1 = 3 bodies in the cell, and planted cells hold 5 to 7 stands),
  and `merge_closest` merges animal pairs where stands fill the count (globe36: 815 animal nodes a year, 7% of its
  animal updates). Under option (a) the cover would change both. So the animals' rule stops reading the plants'
  bookkeeping. **C1a: `crowd` counts bodies with a brain only, and is set to 1**: the animal nodes of a cell merge their
  most alike pairs (within their spreads, as now) at each month's start. **An animal node splits only while it is the
  only body with a brain in its cell** (third review 2), so splits stay possible as handover 7.1 and `laws.md` 4.4 ask:
  in cells without plants, in young worlds and in the nodes check's runs. Its stated changes: a lone animal node in a
  planted cell may now split (today stands left no room), and in a cell without plants the merge runs past 1 animal
  node instead of past 4 bodies. C1a measures animal splits and merges a year in planted and unplanted cells, on the
  benches and in a young world (32 x 32, year 30); halves that split and merge back the next month would show there.
  The check of nodes against individuals (handover 7.1) takes the default and the next value, `--crowd 1` and `2`
  (`NOISE_CROWD` 2): at 2 the split rule is the same and the merge has more room. Light-eaters have their own bound,
  `cell_kinds`.

## 4. Cadence and threads

- A cover updates in the body batches at `node_days` (40), as a stand. Growth, maturing, children and seeds run inside
  its update, in the order of 2.4. The shade with two entries runs in the survey (`survey::shade`), monthly joins of
  alike covers in `nodes::crowd`. Nothing new at the physics step.
- Covers are bodies of their domain: they update in the parallel phase of `survey::batch` (`on_domains`,
  `par::longest_first`), domain by domain, cell by cell, by id; one writer, the domain (CLAUDE.md 3.6). Children that
  stay write only the cover. Seeds wait in the outbox and land at the batch's end in their cell's domain
  (`outbox::land`), in the order of the domains they come from, then the order made; the newcomer rule reads only that
  domain. The world is the same with and without threads.

## 5. Cost (from C0's counts on `benchZ`)

**Today** (C0; one thread; per planted cell-year; 226 / 236 / 450 planted cells):

| | island22 | globe36 | region24 |
|---|---|---|---|
| stands: adult / young, mean | 2.19 / 3.97 | 1.96 / 3.60 | 2.37 / 4.48 |
| kinds with adults (`within`), mean; young-only kinds | 2.13; 1.76 | 1.95; 1.61 | 2.35; 2.11 |
| adult updates x 105 to 107k + young updates x 75 to 78k | 19.6 + 35.5 | 17.4 + 31.5 | 22.4 + 40.6 |
| plant update work, M instructions | 4.73 | 4.24 | 5.54 |
| stand updates, share of the year | 21.7% | 18.6% | 24.0% |
| plants all included (faces, booking, survey, litters: about x1.25) | about 27% | about 23% | about 30% |

A stand updates about 9 times a year (8.9 / 8.9 / 9.4). Of the all-included part outside the updates, the litters
(`outbox::deliver`'s plant share) are taken as about 2.1 / 2.8 / 2.8% of the year (the old benches' "year in
instructions"; C1 counts it on `benchZ`) and go with the copying updates; the rest (faces, booking, the survey's
merges) goes with the number of bodies.

**C1 (the bound on pairs, the clock).** Each kind with adults is an adult and a young node; at most one newcomer,
about always taken (0.9). The kinds with adults lie between two ends (third review 1, decision 46): **today's count**
capped at 3 (1.72 / 1.49 / 1.95 a planted cell, from C0's cells by kinds with adults) and **the fill**: splits refill
every cell with adults to `cell_kinds` = 3 (3 x the share of planted cells with adults: 2.46 / 1.95 / 2.52). The gate
is measured on worlds that run C1's law from year 0, so the fill is in it whatever the estimate says.

**C2 (the young inside).** One cover per kind with adults (about 108k an update: an adult update plus maturing, the
balances and the second shade entry) and the newcomer (no adults: about a young update, 78k, and no litter). Copying
updates stay those of C1's adult nodes, so the litters do not rise against C1.

| | island22 | globe36 | region24 |
|---|---|---|---|
| C1: bodies a planted cell, fill to today's count (today 6.2 / 5.6 / 6.9) | 5.8 to 4.3 | 4.8 to 3.9 | 5.9 to 4.8 |
| C1: plant update work, M (today 4.73 / 4.24 / 5.54) | 4.57 to 3.38 | 3.74 to 2.99 | 4.94 to 3.96 |
| **C1 alone: the year, counted** | **-1% to -7.5%** | **-2.5% to -6.5%** | **-3% to -8%** |
| C2: covers a planted cell | 3.4 to 2.6 | 2.85 to 2.4 | 3.4 to 2.85 |
| C2: plant update work, M | 3.00 to 2.29 | 2.50 to 2.05 | 3.21 to 2.63 |
| C2 on top of C1 (updates, then faces, booking and survey; litters about 0) | -8.5% to -6% | -6% to -4.5% | -8.5% to -6.5% |
| **C1 + C2: the year, counted** | **-9% to -13.5%** | **-8.5% to -11%** | **-11.5% to -15%** |

In each range the first number is the fill, the second today's count. Lane H lands first and makes the animals
cheaper; the gates are measured from the world after C1a and H (decision 46), where the plants' share is about 8%
larger, so every number above grows by about a twelfth (C1 + C2 about -10% to -14.5% / -9% to -12% / -12.5% to -16%).

**Plainly.** At the fill, C1 alone gives about 1% on the island and 3% on the region: **it may fail decision 37's 3%
line on the island**, because splits put back the kinds it merges. C1 + C2 still reaches about -10% at the fill (after
H) and -14% at today's count: C2 does most of the work at the fill (the second body of every kind goes). **C2 can pass
the -10% line on the region at both ends, and on the island at today's count but only just at the fill.** Since C1's
speed and C2's depend on the same fill in opposite ways, I propose (D13) that C1 is judged on its law lines and on
C1 + C2's estimate from C1's counted world (kinds with adults and newcomers per planted cell at C1, the prices above),
and C2 then judged on the -10% line, rather than C1 alone on 3%. The next lever is the copy work, a third of a cover
update (C4 of the plan, after G-C). `cell_kinds` 4 is not needed (C0).

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
- `copy.rs`: `crowd_bounds_the_nodes_of_a_cell` states both: `crowd` counts bodies with a brain (C1a), and the cell's
  kinds stay within `cell_kinds` + 1 with one newcomer (C1).
- Stay (eaters): the four young-node tests of `copy.rs`. Check on covers: `a_tall_member_shades_a_short_one`,
  `a_big_node_stores_light_set_aside_as_it_fell`. Deleted: none.

**Tests new** (fast, `worldgen::testkit` setup):
1. `a_better_kind_enters_full_cells` (C1): a strip of cells full of kinds; a kind of a new form that grows more,
   seeded at one end. After N updates the far cell holds a kind of the invader's form whose gene means lie within a
   mutation step of the invader's (a merge keeps the resident's id, so the test reads genes, not ids). Then a
   **second** better kind, of another form, at the same end: it takes the strip too. **The speed:** the front's cells
   a decade under C1 within a factor of 2 of the same test on today's code (decision 46). No cell ever holds more than
   `cell_kinds` + 1 kinds at a month's start.
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

Lines are `core` code lines (`cli/tests/core_size.rs`, ceiling 10,643). "Counted" = callgrind on
one thread, one year from the remade benches (`make_benches.sh`, the step's snapshot version), seeds 1 to 3, with
`--toggle-collect` on the update of a body without a brain (as PL1): the year's instructions, the plant work per
planted cell-year, the planted cells, the animal updates a year, bodies per planted cell, `outbox::deliver`. Across worlds the year alone
counts only above PR-S's bands (19% island22, 11% globe36); the counted parts carry the verdict.

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| C0 | **Done** (`probe/c0`, `b462387`; `measures.md` "C0"): stands, kinds and forms per planted cell, litters into cells with no kind of theirs, animals and merges, a stand update by stage | | probe | `cell_kinds` 3 holds all the adult mass; an adult stand update 105k to 107k | |
| C1a | **`crowd` counts bodies with a brain only, set to 1; an animal node splits only while it is the only body with a brain in its cell** (3): `nodes::crowd` and `nodes::split`; `numbers.md`'s `crowd` row; `laws.md` 4.4; handover 7.1 (the check's pair `--crowd 1` and `2`); `tape/tests/nodes.rs` (`NOISE_CROWD` 2); snapshot | `nodes.rs`, `params.rs`, `tape/src/snapshot.rs`, `tape/tests/nodes.rs`, docs | 4 (animals) | animal nodes per planted cell, animal updates a year and animal splits and merges a year (planted and unplanted cells) within their seeds' noise of the world after lane H (counted, 3 seeds), and in a young world (32 x 32, year 30) splits as today's; the eaters' key numbers; the nodes check holds its rules; its own effect on the counted year reported apart | those move past their seeds' noise: option (a) stays, and C1 and C2 are gated on the animals as they are |
| C1 | **The bound on kinds and the clock** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the newcomer place is free, else the litter joins the most alike; a light-eater node splits only while its cell holds fewer than `cell_kinds` kinds with adults; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (no `alike` limit for bodies without a brain), and every kind without adults older than its life span at its ground's temperature now joins its most alike kind (`Body::life_span`); snapshot; test 1 (two invasions, the speed against today's code) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `body/tissue.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (kinds with adults and newcomers per planted cell measured apart); test 1; animals within their seeds' noise of the world after C1a and H; key numbers in range (S50 within noise); the climate and life tests of 6; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's); the counted year against the world after C1a and H (estimate -1% to -7.5% island22, -3% to -8% region24) | **kept if** the counted year falls at least 3% on island22 and region24 with all its law checks (decision 37), **or, if D13 is taken,** if its law checks pass and C1 + C2's estimate from its counted world reaches -10%; else C stops (A4 or nothing) |
| C2 | **The young inside** (2), **built only if C1 is kept and C1 + C2's estimate from C1's counted world (kinds with adults and newcomers per planted cell, the prices of 5) reaches -10% on island22 and region24**: `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth by each stage's balance, a young deficit only killing young (2.5); maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage; founders and bodies that lose their brain all adult; `Young::held` a day, read by the clock; snapshot; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4, 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; **C1 + C2 at least -10% of the year counted on island22 and region24 against the world after C1a and H** (decision 37; estimate -10% to -14.5%, -12.5% to -16%) with animal nodes and animal updates within their seeds' noise; light-eater members per planted cell within a factor of 2 of C1's; test 2; key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 | C1 + C2 under -10% on island22 or region24, or the animal, key-number, climate or kinds lines fail: C2 is not merged, C1 stays if it passed its own lines |
| C3 | **The checks** (no code): counted runs of the three benches, plants all included; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **The cover's gate** (decisions 37, 46), against the world after C1a and lane H: C1 at least -3% of the year counted on island22 and region24 with its law checks (or D13); with C2, C1 + C2 at least -10% on both, globe36 not worse; animal nodes per planted cell and animal updates a year within their seeds' noise; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 stays if it passed its own lines); A4 or nothing |

**Lines.** C1a about +4 (the alone-split test). C1 about +34 -5 (the newcomer rule, the split's room, the merge past
`cell_kinds`, the clock and `Body::life_span` at the ground's temperature). C2 about +125 -10 (`Young` and its reads 27,
shade entries 12, growth by balances 18, maturing in `live` 20, children in place 15, starving per stage and the brood
14, pooling 10, placement and resize 6, the rest 3). **Net about +33 for C1a and C1, about +148 with C2.** The cover
cannot delete more than it adds: the young law, splits, merges and litters stay for animal nodes (L4 as rewritten).

**Risks.** (1) The climate coupling of plants (P1.2's -0.78 K; the long-step test, judged on the mean of seeds 1 to 6,
L38): the day's code is whole, so the risk is in mass and liquid held; gated in C1 and C2. (2) S50 with pooled
stores. (3) Light-eaters' mass (0.16 above its floor) and life: shared stores, starving per stage. (4) Animals: C1a
moves the merge where few stands stood; more animal faces in `SEEN` after C2; gated. (5) Fecundity genes selected only
between kinds and cells. (6) The newcomer place is almost always taken (C0): an invader's litter finds it free about one time
in ten, so fronts move slower than today; test 1 measures the speed against today's code. (7) Judging across worlds: the counted parts carry
the verdict. (8) Kinds are per cell: a refined region holds four times the kinds per km² (the scale test). (9) The fill:
splits refill the kinds C1 merges, so C1 alone may give about 1% on the island (D13); C1 + C2 is near the -10% line
on the island at the fill. (10) C1a lets a lone animal node in a planted cell split, and its halves may merge back the
next month; C1a counts it.

## 8. Alternatives if the gate fails

- **C1 alone** (C1a, the bound and the clock on young-adult pairs) if C2 is not built or fails its line: about -1% to
  -7.5% / -2.5% to -6.5% / -3% to -8% of the year (the fill to today's count). It keeps the owner's kinds and the
  invasion rule.
- **A4** (the sample runs only the stages its gene reads) or nothing, if C1 fails its line (decision 37).

## 9. Decisions D1 to D13

| | Decision |
|---|---|
| D1 | **Decided, L35.** The cover with stages, after the bound on young-adult pairs. |
| D2 | **Decided, L17.** `cell_kinds` = 3 (the owner's three forms; with the newcomer place at most 4 kinds a cell). |
| D3 | **Decided, L23 and 46 (1), (4).** At most `cell_kinds` + 1 kinds, one of them a newcomer; a new kind starts only while that place is free; light-eater splits only under `cell_kinds` kinds with adults; past `cell_kinds` kinds with adults the most alike pair merges monthly, without the `alike` limit; the clock from the start, at the ground's temperature now, on every kind without adults. |
| D4 | **Decided** (names that clash with nothing; the architect's first decisions, 9): `Body::young`, `Young`, `Config::cell_kinds`; "cover" only in docs. |
| D5 | **Decided, L25.** One law (the McKendrick flux by `math::Pass`) with two holders, node pairs for animals (their young and adults act apart) and stages for covers; `rejoin` is the one place that pools size moments. Said in `laws.md` 4.4. |
| D6 | **Decided, L24.** Counted parts on remade benches (above), no converter of old saves. Why: a converter is code nobody keeps, and the counted parts are robust to the world's noise. |
| D7 | **Decided, L26.** New key-number references on C2's commit, for the steps after it (the cover changes the world). |
| D8 | **Decided, L27.** Accept that `CHILD`, `MUT`, `DISPERSE`, `WET`, `ALLOC` are selected only between kinds and cells, with C0's number (the adult mass from unlike litters) and the 1,000-year watch. No C5: `sample_every` 1 cannot reach them (`select::UNREAD`). |
| D9 | **Decided, L5, L6, L15, L18.** Idea 2 and A1 dropped before the cover; B1 and C in any order. |
| D10 | **Decided, L29.** C1a, C1 and C2 each bump the snapshot (the highest number plus one at each merge); benches remade at each. |
| D11 | **Decided, L28.** The tape column `root_<K>_adults` (share of members that are adults): one column to watch the stages. |
| D12 | **Decided, 46 (2), (3).** Option (b), step C1a: `crowd` counts bodies with a brain only, set to 1; an animal node splits only while it is the only body with a brain in its cell; the nodes check's pair is `--crowd 1` and `2`. |
| D13 | **Proposed (third review 1):** judge C1 on its law lines and on C1 + C2's estimate from C1's counted world, not on its own 3%: at the fill C1's splits put back the kinds it merges (about -1% on the island), while C2 then removes most of the plant work. Why: the two steps share one fill that moves their gains in opposite ways. Else C1 may fail at 3% and stop a C2 that would pass. |

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

**After C0** (`measures.md` "C0", decisions 26 and 37):

1. Cost restated from C0's counts (5); ranges to the fill after the third review.
2. Gate per decision 37 (7): C1 kept at 3% with its law checks; C2 built only if C1's counted cut makes -10% reachable.
3. `crowd`: option (b) in its own step C1a, as (a) would move the animals on the globe (3, D12).
4. The newcomer place: almost always taken; the L23 clock runs from the start (3, D3).

**Third review** (decision 46):

1. Split room for light-eaters only under `cell_kinds` kinds with adults, never into the newcomer place (3); the
   estimate is a range from the fill to today's count (5); D13 proposes how C1 is judged at the fill.
2. C1a keeps a split for an animal node alone among the bodies with a brain in its cell (3, C1a).
3. The nodes check's pair is `--crowd 1` and `2` (`NOISE_CROWD` 2).
4. The clock reads the ground's temperature now through `upkeep_pace`, on every kind without adults, from the later of
   its start and its last day with adults (`Young::held` a day) (3).
5. A young deficit only sets the young's death share; no body moves between stages; the energy per kg from the build,
   no reaction number (2.4, 2.5, 2.8).
6. Test 1 reads genes, not ids, and compares the front's speed with today's code; 2.8 says a better kind of a held
   form enters only as gene flow.
7. The gates are measured against the world after C1a and lane H; C1a's own effect is reported apart (7).
8. C2's newcomer is priced as a young update with no litter (5).

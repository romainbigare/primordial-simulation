# The next phase: a clean base, then animals as a field

For the owner and the next orchestrator. Simple English. This is a look at the whole project as it stands on `main`:
the spec (`docs/handover.md`), the laws (`docs/laws.md`), every file in `review/`, the two reports, and the code of
`core`, `worldgen`, `tape` and `web`. Two short runs were made for it (section 10). Numbers marked "est." are
estimates from reading, not measures. Work that lives only on other branches (the air and sunlight, the sorted start)
is taken from what `review/orchestrator.md` says of it.

It answers seven questions:

1. What is the shift that made everything better? (section 1)
2. What is left over from the older designs and should go? (section 2)
3. How do we bring the shift to the animals, and what must we watch? (sections 3, 4, 5)
4. What does the natural world still miss that later stages will need? (section 6)
5. A new roadmap. (section 7)
6. Do we need a clean-up cycle first? (section 8)
7. How do we build the animal world again? (section 4)

Words used here: **the books** are the accounts of matter and energy (nothing is made or lost). **A field** is a
number per cell (so many kg per m²) in place of a list of single things. **A slot** is one of the few kinds a cell
can hold. **A make of bed** is a kind of rock. **A push** is what drives a flow (a height, a pressure, a warmth).

## 0. In short

- **The shift has a name: "fields, a fixed ground, and a picture that is derived".** The world is no longer a crowd of
  single things that each run a full model, on cells that follow the camera. It is three layers. Each layer runs at
  its own cost and nothing flows back from the picture to the world.
- **A third of `core` still belongs to the old designs.** The old animal world alone is about 5,500 of `core`'s
  16,800 lines, about 2,500 lines of tests, and most of the tape's 631 columns. More leftovers come from the tree of
  cells (deleted as a design, but some of its tools remain), from plants-as-bodies, and from the docs: the spec still
  describes cells that split and a planet.
- **Do not refactor the animals. Replace them.** The plant cover is the pattern: a few slots per cell, each with
  amounts and genes, one month's pass, costs for every trait, exact books. An animal field built the same way is
  about 1,500 to 3,000 lines (est.) and has a fixed cost per cell, whatever the number of animals.
- **The code is not the hard part. Five design questions are:** what "Choose" means for a field; how animals move
  without a rule that already holds the answer; small numbers (the last ten animals of a kind); the balance between
  plants and grazers; and people.
- **A field is right for wild animals. It is not enough for people.** Houses, stores, tools and trade need single
  things with a place and a life. People will need a second layer of single agents on top of the field. That layer
  should be designed on paper before the field is built (section 4.8), so the field does not close the way to it.
- **The natural world misses things that animals and people will need:** years that differ (today every year is the
  same year), a store of seeds on the ground, fresh water against salt water, fire, life in the water, soil that
  builds up, and useful rocks (ores, fuels, clay, salt). None is urgent for plants. Several decide whether farming,
  storing and settling can ever appear.
- **Yes, a clean-up cycle first.** Two to three weeks of work (est.). Its largest step is to delete the old
  animals. That is less simple than it looks: the plants' month runs on the animals' machinery and must be cut free
  first. Done in the right order (section 8), a plants-only run stays the same to the last digit.
- **The roadmap changes shape.** Not M0 to M8 on one growing code base, but layers, each ended by a picture the owner
  can judge: finish nature, clean, what the land offers, animals as a field, the small game (god tools on a living
  island), then people.

## 1. The shift: what changed

### 1.1 The old ways

Four ideas ruled the project before. Each was dropped for a measured reason.

| Old idea | What it cost | Where the reason is written |
|---|---|---|
| **Every living thing is a body.** A plant stand ran the full organism model (reactions, melting, pores, roots, a brood). Speed came only from merging alike bodies into nodes | 60% of a year went to bodies. Three speed rounds gave 3.8 times, never 10. Every cheaper design "changed the world" and was refused by a test of sameness | `review/big-look.md` |
| **The cells follow the camera.** One tree of cells; a cell splits where one looks; the step follows the size; zoom slows time | A year went from 1 s to 3 to 7 s. The world flickered. The history of a world depended on where one looked. And "nothing new appears at fine grain" | `docs/reports/fine.md`, `review/roadmap.md` 3.1 |
| **The picture is made by the core.** One fine window, on the CPU, in the same worker as the world, with 25 layers that each kept the cell's mean exactly | 0.35 to 1.1 s per window; the world waited for its picture; steps at cell edges | `review/roadmap.md` 2.2, 3.2 |
| **The ground is a function of the seed.** Noise at any depth, never state | No valley, no lake, no river bed; erosion could never show | `review/roadmap.md` 3.3 |

### 1.2 The new way, in one picture

```
Layer 3  THE PICTURE     per pixel, on the graphics card, only where one looks.
                         It reads layers 1 and 2. It writes nothing back.
                              ^
Layer 2  THE GROUND      fixed samples of 125 m, made once at the world's start by running
                         the erosion law for 2 million years. One network of channels, ponds
                         with one level. It is state, and it changes slowly.
                              ^
Layer 1  THE BOOKS       slow physics and life as FIELDS on fixed cells of 2 km:
                         air, liquid, heat, chemistry, the plant cover.
                         Every element and the energy are kept to 1e-9.
```

Between the layers stands one tool: **a law inside a cell**. A cell holds means. A general law shares each mean over
the cell's own ground (how wet each point is, how deep its loose ground, which way it faces). Whoever needs detail
calls that law at a point: the picture today, a single animal tomorrow. Detail is **derived, not simulated**.

### 1.3 Seven rules that define it

These are the rules I would write at the top of the spec. Each is already true of the natural world on `main`.

1. **The world does not know the camera.** Cells are fixed for a world's whole life. The same seed gives the same
   world. Zoom is drawing.
2. **Fields first.** A kind of life in a cell is a few numbers: amounts of matter, a small vector of genes, a count.
   A fixed number of slots per cell. The cost of a cell is fixed, whatever lives in it.
3. **Derive the single thing from the field.** The cover holds kg per m²; the count and size of its plants follow
   from the self-thinning law, their height from the buckling limit, their place from the wetness their genes like.
   Nobody stores a tree.
4. **A slow thing is computed once.** The ground gets its history when the world is made. The sorted start does the
   same for plants. A fast thing that is always near balance is a rate, not a population (the rot of the open world
   stands for the microbes).
5. **Every trait has a cost, or nothing sorts.** The cover's law became good when leaves, stems, roots and frost
   hardiness each got a price. Then zones appeared by themselves (bare north, a tree line, old stands in the south;
   the frost gene follows the coldest month, r 0.7).
6. **Step at the pace of the thing.** Air every 15 days, cover and liquid monthly, creep yearly, the ground at
   creation.
7. **Judge a change of the laws by pictures and by ranges, not by sameness.** Before and after pictures at the
   owner's views; a few key numbers that must stay in a real range. Sameness is the right test for one thing only: a
   clean-up, which by its meaning changes no result, is still checked to the last digit.

And two rules of the owner stand above them: **things emerge from general laws**, and **do not compute what is not
needed**.

### 1.4 What it gave (from the repo's own numbers)

- The tree of cells went: 5,340 lines out, 383 in.
- The live page at a close view: 0.1 to 0.4 months a second before, 36 to 41 after.
- Valleys, lakes with one level, rivers that reach the sea, deltas' first form, drowned coasts, five makes of rock in
  layers: all from one erosion law run at the start, not one of them drawn.
- Plants sort into zones on a Wide island and stay steady from year 100 to 300.
- A plants-only year of a 32 x 32 island costs 235 ms: Flow 185, cover and survey 45, Change 8, tape 3 (section 10).

## 2. Leftovers: what should leave the code base

Rule 2.4 of `CLAUDE.md` says: no dead code, git keeps it. Rule 2.12 says: replace, do not patch. By those rules
the list below is overdue. The details and line numbers are in the appendix (section 11).

### 2.1 The old animal world (the largest leftover)

| Part | Where | Lines of code (no comments) |
|---|---|---|
| Bodies: the organism model | `core/src/body/` | 1,040 |
| Choose: brains, perception, actions, learning | `core/src/choose/`, `brain.rs`, `lineage.rs` | 1,355 |
| Copy for bodies, population nodes | `core/src/copy/`, `nodes.rs` | 675 |
| Bond: parts, lots, bonds, rooms, edges | `core/src/bond/` | 1,090 |
| The registry of bodies, the queue, the serial pass (not the chunks: the cover runs on them) | `core/src/life/` (most of it) | about 800 of 1,030 |
| The bodies' half of the survey, the step grid, lanes | `survey.rs`, `survey/steps.rs`, `lanes.rs` | about 300 |
| **Sum in `core`** | | **about 5,300 to 5,500 of 16,800** |
| Their tests | `core/tests/{body,bodies,bond,choose,copy,populations,damage,lots}.rs`, one test of `threads.rs` | about 2,500 of 5,070 |
| Their tape columns | `tape/src/rec/cols/{brains,kinds,parts,social}.rs`, `things.rs` | about 450 |
| Their start | `worldgen/src/life/{make,seeding}.rs` (the bodies' half), seven rows of `KINDS` | about 250 |
| Their drawing | `web/src/{avatars,sprites,blob}.ts` | about 680 |
| The pattern finder's three detectors (groups, routes, structures) | `analyze/src/` | 320 |

Why it is a leftover and not "the animal world to improve":

- It is the "every thing is a body" design that the plants left. It has all its weight: a record per body, an id per
  body, a brain per node, a queue for what crosses a cell, a serial pass to land it, three caps to bound the nodes.
- The owner's word is that it will be reworked. The live page and every long run go without it (`--plants`). Code
  that nobody looks after rots. It already has: only 5 of 7 kinds with real numbers at year 70 (section 10), the slow
  learning tests not run since the cover's law, the eaters that collapse under the sorted start.
- Yet the gate of the natural world still hangs on it. The main guard of the fast suite
  (`the_world_lives_and_keeps_its_books`, `tape/tests/ranges.rs`) fails with fewer than four animal kinds alive, and
  two of the nine key numbers are the eaters' mass and life. So a change of the air or of the cover can fail the
  gate through animals that nobody means to keep. This is the first thing the clean-up mends.
- Its two watch reports say what it could not do: no shelter in the cold, no building, tools at 0.04% of food,
  nothing that grain helps, and most "yes" verdicts equal to the null run (`docs/reports/m3.md`, `fine.md`).
- The laws of the natural world need little from it: three death-cause numbers, `WET_TISSUE`, `body::noise`,
  `body::solids_of`, the dispersal kernel `copy::lot`, `bond::Edge` in one function of the cover, `lanes::regrow`.
- **But the plants' month runs on its machinery, and that is the real work of the cut.** Each month, also with no
  animal, `survey::survey` runs first: it breaks what movers pushed, sums each cell (`Life::sum`, which the air reads
  for the canopy), lands the queue and builds Flow's state. The cover's own month is `survey::cover`, on the chunks
  and the queue of `life/`. Flow calls `survey::release` before each air step. And the plants' start draws are keyed
  by each kind's row in the table of kinds, where animal rows stand between plant rows (`worldgen/src/life/`
  `seeding.rs`, `kinds.rs`). So the month of the natural world must first be stated on its own, in the same order,
  and the plant kinds must keep their numbers. Then the animals can go without moving one digit.

What is lost with it, said plainly:

- The only user of the cover's grazing door (`Cover::take`, `Cover::breaking`, the budget's row "taken or broken by
  bodies"). Until the animal field uses it, it is unused code, which rule 2.4 forbids. Either the field's first
  step comes soon after, or that door goes too and comes back with the field.
- The only code for single agents that cross cells with exact books: the queue, the landing in an order fixed by the
  data, and the exact split and join of members (`nodes::Lot`, `nodes::rejoin`). People will need this idea again
  (section 4.8). Keep its description; a tag alone is soon forgotten.
- Rooms and held things with real physics (`bond/`). The same.

What must be **kept from it, as knowledge**: its anchored laws and numbers. They are good and the new animals reuse
them: Kleiber's upkeep, Holling's take law, the heat balance through a coat, the walk at one Froude number, the cost
of transport, the drag in liquid, one frost law for bodies and cover, wear by the rate of living, the brood, the
dispersal kernel, breaking and the edge. They live in `docs/laws.md` sections 3 to 6 and `docs/numbers.md`. Keep those
rows (in a file of "laws kept for the animal field") when the code goes.

### 2.2 The tree of cells: deleted as a design, its tools remain

The tree itself is still used and stays: cells of 1, 2 and 4 squares need its balance and its neighbours, and the
samples are its leaves of depth 6. What remains of the tree **that moved** is this:

- `core/src/space/tree.rs`: the most depth is 24 ("a leaf of 0.5 mm, finer than any body"). Cells need depth 2,
  samples 6, the tiles about 13.
- Ids by "cell serial" with a table from serial to index, "none once it split or joined" (`core/src/life/mod.rs`):
  no code adds or removes a cell, so the serial is always the index. About 45 lines, and it is saved.
- `Purpose::Quarter` in `rng.rs` ("the quarter a body goes to when its cell splits (`life::tree`)"): never used, and
  no such module.
- `choose/features.rs` (`Regions`, `across_corners`, about 75 lines), `Leaf::parent`: so that a cell finer than a
  square sees as far as a square. No world has such a cell.
- `Terrain::below` and the branches `SAMPLE_DEPTH.max(l.depth)` in `flow/`: the case of a cell finer than a sample.
- `survey/steps.rs`: a general grid of "2^halvings steps a month" that is only ever made with 0 and 2.
- `Mesh` of any number of levels (`space/mod.rs`): it always builds two.
- Comments that still tell of splits: the module doc of `cover/mod.rs` ("a split of a cell shares its cover out,
  `tree`"), `nodes.rs`, `worldgen/src/terrain.rs`, `flow/drain.rs` ("sub-stepped where a cell is too small").
- In the spec: 5.2, 7.2, 7.3 and 7.4 still describe cells that split and join, the camera as a recorded input, the
  step that follows the size, and zoom that slows time. `docs/laws.md` 1.8 is still titled "The step by the size".

### 2.3 Plants-as-bodies, still in the body code

- `Bodies::cat` in `worldgen/src/life/kinds.rs` still holds "the speed-up of the light reaction" for a body. It is
  off for every kind.
- The survey still keeps "the share of light set aside for the bodies' light reactions" (`Life::absorb`,
  `Life::keep`, rows 2, 8 and 9 of `Life::sum`). Only the cover fills them and only the air reads them; they belong
  in the cover. The store's node for it is still called `Class::Bodies`.
- `World::rained` ("its bodies catch what fell"): written, saved, and read by no law. `Chemistry::light_reactions`:
  no caller at all.
- The gene `WET` (dry, dormant children, as seeds and spores) and the wear floor "as dry seeds and spores do" are
  plant ideas on animal bodies.

Most of this leaves with 2.1.

### 2.4 Tools and plans of older rounds

- `core/src/inside/` (the law inside a cell): no law of the core reads it now. Only the tape and the page do. It
  is the right idea in the wrong crate until the animals need it (section 4.2); say so in its doc, or move it.
- `worldgen/src/generate.rs` and `richness.rs` (about 300 lines): the generator of random chemistries and its
  checks, for "M7". A random chemistry cannot grow life today (the spec says so). It is a plan, not a part.
- `ew view` (a window of a snapshot as a PNG, with a "depth" layer): made to look at fine cells.
- `--fine`, and the globe: see the owner's decision 3 in section 9.
- The 631 columns of `series.csv`: most are per-kind columns of bodies and counters of acts.
- Small dead things: `units::HOUR_S`; five classes in the sleep clock where two are used; test-only doors in the
  core (`Life::part_mut`, `rows_mut`, `Body::store_of`, `World::even_ground`).
- Names that mean two things (`CLAUDE.md` 2.8): "bodies" is animals, ponds (`flow/bodies.rs`), a store class and a
  worldgen struct; `Grid` is three types; `Books` is three.

### 2.5 The docs

- `docs/handover.md` is called "the single source of truth for the plan". About 15% of its lines are stale, and they
  are the heart of the architecture: 5.2, 5.4, 7.2 to 7.4, 12.6, 13, 13.1, 14.3. It still says "the whole planet
  down to one street", and that the decision taken is "the zoomable world in the order of `review/plan.md`". Its
  budget for `core` is 10,000 lines; `core` has 16,800.
- No document records the owner's word that the world need not be a planet.
- Three documents each say that they win: the spec ("this document wins"), `CLAUDE.md` ("ask the owner"), the
  orchestrator's handover ("the code and `docs/laws.md` are the truth").
- `review/` holds seven files. `plan.md`, `handover.md`, `top-30.md`, `big-look.md` and `objects.md` are fully
  replaced. `roadmap.md` sections 1 to 8 describe code that is gone, in the present tense; its sections 9 to 11 are
  current. `CLAUDE.md` still names `review/big-look.md` as "the latest look", and names the roadmap nowhere.
- `docs/reports/` holds two reports of the old world (13 files, 1.6 MB with pictures); `fine.md` is the watch report
  of the design that was deleted.
- `CLAUDE.md` 4.2 says "no history anywhere, not in docs". `review/roadmap.md` section 10 is 280 lines of "done"
  history. It was useful while the work ran; it now hides the 40 lines that matter (what is open).
- `docs/laws.md` is true to the code; under 2% of it is leftover wording ("its cell's step", "a finer cell").
  `docs/numbers.md` and `docs/tape.md` have a handful of wrong names. `README.md` gives an old speed with a date and
  no example of the run everyone uses (`--roster --plants`).

## 3. The animal world today, measured

A 32 x 32 island (793 cells, 409 of land), seed 1, the roster, 70 years (section 10):

- 219 bodies (200 of them nodes) on 409 land cells: half a node per land cell. 5 of 7 kinds hold real numbers; two
  hold 3 and 11 members.
- All animals together weigh 2.9e6 kg: 1.7 g per m² of land. The cover weighs 1.35e10 kg (8.3 kg per m²).
- They eat 7.8e6 kg of cover a year. By the cover's own budget in the tape, what bodies take or break is 1.7e14 J a
  year, 1% of what the cover's light stores less its upkeep and its building (1.6e16 J). On Earth grazers take a few
  percent in forests and 10 to 60% in grasslands (McNaughton and others 1989, from memory). The cover gives 45 times
  more to the ground through its roots than to all animals. So the animals hardly shape the plants.
- Eating other bodies: 2 to 3% of the animals' food.
- Cost: 40 to 50 ms a year more than plants only, at 26 to 33 µs per update. It is cheap only because the animals
  are few. The older runs with full populations paid 0.8 s a year at 3,700 bodies and 1.5 s at 16,000.

So today's animals are a thin layer that lives, eats a little and changes nothing. They are not wrong physics. They
are the wrong level: a costly model per node, and not enough nodes to make an ecosystem.

## 4. Animals as a field: the design I propose

### 4.1 The idea

Do for animals what was done for plants. Each land cell holds a **fauna**: a fixed number of slots (8 to start; a
number to measure). A slot is one kind of animal in that cell:

| A slot holds | Like the cover's | Notes |
|---|---|---|
| members: a real count, young and adult | `Cover::plants` | counts are real, never rounded up (`CLAUDE.md` 3.2) |
| amounts: body matter, fuel (fat), frame, with their heat | `Cover::mol` | 64-bit, every move booked on both sides |
| about 12 genes | `Cover::genes` (9) | adult mass; diet (leaf, wall, seed, remains, prey by size); warm or cold body and coat; range per day; child size; group size; wetness of the place it likes; repair |
| a running balance | `Cover::net` | a slot in deficit gives way, as for plants |
| the starting kind | `Cover::root` | a label for the tape only |
| a small vector of habits (4.4) | none | learned, not inherited by the body |

One **month** of a cell's fauna is one pass, as `cover::step` is, slot by slot in a fixed order:

1. **Food.** What each slot can reach: the cover's parts by height (grazers reach low, browsers high), seeds, remains
   on the ground, and the other slots by their member mass. One take law for all (Holling's, already anchored).
2. **Energy.** Upkeep by Kleiber at the pace of its body temperature. A cold-bodied kind follows the ground; a
   warm-bodied one pays the heat balance through its coat (the closed form of `live::Balance`, already written).
   Moving costs by the transport law. What is left builds young and fat.
3. **Deaths.** Unpaid upkeep, wear by the rate of living, frost, and what the eaters took. Remains go to the ground
   with their mineral, so grazers speed the cycle of the mineral as real ones do.
4. **Births** from the brood, into the young. Young grow into adults by a flux, as today.
5. **Books.** Gases with the air, liquid with the ground, heat to the ground: through the cell's view of the store,
   the same way the cover books.

Then, when every cell had its month, two passes between cells, each a plain gather in cell order:

6. **Movement as a flow.** Members flow to the 8 neighbours by two pushes. A spread, by the kind's range per day.
   And a pull by what the kind can **sense** next door: the food there, the crowd, the warmth, the eaters. How
   strongly each of these pulls or pushes is a gene, and moving costs energy by the transport law. So a kind does
   not know where it would fare better: kinds whose genes lead them well leave more young. Theory says where this
   ends when animals know everything (each member gains alike everywhere: the ideal free distribution, Fretwell and
   Lucas 1970). That is a **check** of the result, never the rule. A kind with a large range on a Wide island may
   then move with the seasons. Nobody writes "migrate".
7. **Dispersal and new kinds.** Young disperse by the kernel the seeds use. A share of the young are mutants; they
   join the most alike slot or found one, by the slot rule of the cover (`seed::lodge`, `merge_alike`), which
   becomes one shared helper. One warning: joining takes the mean of the genes. For plants on one axis of light that
   is mild. For animals a mean of a grazer and a hunter is good at nothing. Slots must join only when truly alike,
   and this must be tested before the rule is shared.

### 4.2 The law inside a cell, for animals

A cell of 2 km is large for an animal. Where its members are inside it comes from a general law, as for plants:

- Each kind has a **place it likes**: wet or dry ground, open or under tall cover, near or far from standing liquid,
  flat or steep. A cell already knows the share of its area in bins of loose depth (`World::loose_bins`); it gets
  the same for wetness, slope and distance to liquid, found once from the samples.
- Each kind has a **group size** (a gene). Members stand in groups of that size. A group lowers each member's risk
  from eaters (the eater takes one of many; Hamilton 1971) and raises the fight for food inside the group. Both are
  terms of the take law. So herds, packs and lone hunters are places on one axis, each with its price.
- **Meetings follow from overlap.** How often an eater meets its prey is the take law times how much the two kinds'
  places overlap inside the cell. A few numbers per pair, from the bins.
- **The picture uses the same law.** Close up, the page draws single animals: as many as the slot's count, in groups
  of its group size, on the ground its kind likes, each with a place seeded by (cell, slot, index), drifting along
  the cell's flow of movers. Far away, a herd is a soft mark, as today. No animal is stored.
- A drawn animal is a picture only. Next month the slot's count has changed and the same place may show another
  one. It has no name that lasts and holds no state. A single being that lasts is a different thing (section 4.8).

### 4.3 Small numbers

A field can fall to 1e-12 animals and grow back from that: a known fault of field models (a fox of 1e-18 that
comes back to life; Mollison 1991).
The rule against it is already in `CLAUDE.md` 3.2: counts are real. Where a slot holds few members, births, deaths
and movers are drawn as whole numbers with a keyed draw (the kind of draw `lose_share` makes today for an
individual), and a slot with less than one member is gone. So a kind can truly die out in a valley and come back
only from next door.

### 4.4 What "Choose" becomes

This is the real design question. Today each node has a small neural network that learns in its life. Two reports
say it learned nothing that beats chance beyond its hand-set start. I propose two steps.

- **Step one: habits as inherited numbers.** This is still Law 5, with the smallest brain: what a kind senses (its
  hunger, the warmth, the food, the crowd and the eaters, here and next door) times a few weights gives the shares
  of its time over the same general acts as today (take, move, rest). The weights are genes and evolve as every gene
  does. It is "choices as a spread of actions" (`laws.md` 4.4), which nodes already are, with no hidden layer, no
  learning, no trace and no memory. The list of what can be sensed and the list of acts are the only things written
  by hand, as they are today; no weight is set to get a wanted habit, and a run with all weights at zero is the
  null run to compare with.
- **Step two: habits that are learned and passed on.** A slot gets a small vector of learned habits. It moves, within
  a life, toward what paid (the gain of the last months). Young take a share of it from the adults of their cell; that
  share is a gene. Members that move carry it. So a habit can outlive its bodies and spread across the map faster
  than genes: that is culture as a field (Boyd and Richerson 1985). It is Law 4's imitation and Law 5's learning, in
  the mean. A kind with a high share and a high learning rate is the start of people.

Step two is research. It should start only after step one has made an ecosystem that stands.

### 4.5 What goes, what stays, what waits

- **Goes:** bodies as records, ids, brains per node, the trace, perception of faces, the queue and the serial pass,
  the caps, mating of pairs, parts, lots, bonds, rooms, edges, lanes.
- **Stays (as formulas in the new month):** every anchored law listed in 2.1.
- **Waits for the people stage:** made things and shelter (section 4.8). The old way made 20,000 to 300,000 parts
  that nobody used; the lesson is to make no object before a kind exists that wants one.

### 4.6 Cost (est., to measure first)

The cover's month and the survey cost 45 ms a year on 409 land cells: about 9 µs per land cell and month, for 4
slots. The animals' month is not simpler than the cover's: it has movement, eaters and prey in one cell, overlap and
groups, and steps inside the month (section 5.6). So plan for two to four times the cover: 0.1 to 0.2 s a year at
size 32, 0.4 to 0.8 s at size 64. These are guesses until a first version is timed. What is sure is the shape: the
cost is fixed by the cells and the slots, not by the animals. The old model with a full fauna (8 kinds, young and
adult, in every land cell of a 64 island: about 25,000 nodes) would cost 5 to 10 s a year (est. from 0.2 to 0.4 ms
per node and year, section 10).

### 4.7 How we will know it works

By numbers of real ecosystems, as ranges in one slow test, and by pictures:

| Check | The real range | Source (from memory; to verify when the test is written) |
|---|---|---|
| Grazers eat a real share of what plants grow | a few percent in forest, 10 to 60% in grassland | McNaughton and others 1989 |
| Grazer mass follows rain and plant growth | 1 to 20 g per m² in savanna | Coe, Cumming and Phillipson 1976 |
| Numbers fall with body mass as mass^-0.75 | across kinds | Damuth 1981 |
| Eater mass grows slower than prey mass (power near 0.75) | across places | Hatton and others 2015 |
| Size and coat follow the cold on a Wide island | larger and thicker in the north | Bergmann's rule |
| Eaters and prey swing | in some places, not all | Lotka and Volterra; against a run with no eaters |
| A wide-ranging grazer moves with the seasons | its mean latitude turns over the year | against a run with a small range |
| The books | drift under 1e-9 | `check::drift` |
| Cost | under 40 µs per land cell and month | section 4.6 |

Pictures: a herd at the 200 m view standing where its kind likes; a grazed plain against an ungrazed one; a map of
kinds by zone.

### 4.8 People: the field is not enough, and this should be said now

A field can hold "0.3 houses per km²". It cannot hold one house in one valley, the family that built it, what lies
in its store, or the path to the next one. The end goal (settlements, tools, trade, buildings) needs **single things
with a place and a life**. So the world will have two kinds of life in the end:

- **The field**, for everything that is many and alike: plants, wild animals, and most people most of the time.
- **A thin layer of single agents and single things**, for the few that matter one by one: a band, a household, a
  store, a building, a boat. Each has a place inside a cell (a point of the ground's samples), a memory, and what it
  holds.

The hard part is the passage between the two, with the project's rules kept: exact books, real counts, one writer
for each number. A member leaves a slot as an agent with its exact share of the slot's matter and heat, and joins it
again by sums. The old code already has this mathematics (`nodes::Lot`, `nodes::rejoin`), and it has the queue that
lands what crosses a cell in an order fixed by the data. That knowledge should be kept in writing.

What I ask for now is small: **one page of design for this passage before the field is built** (a step of stage C
or F1), so that the field's slots, counts and books are made in a way that an agent can leave and join. No code for
it until the people stage. I cannot promise the field will carry people; I can only say that wild animals do not
need more, and that the passage is the thing to get right.

## 5. What to be mindful of

1. **Do not script.** The danger of a field is that it is easy to write "herds migrate in winter" as a rule, or,
   more quietly, to write the answer of a theory as the law (members that flow to where they would fare best). A
   kind may only use what it can sense, at a cost, with weights that are genes (4.1, 4.4). The test of the spec
   holds: can you list all uses of the rule in advance? If yes, it is a named system.
2. **Plants and grazers must hold each other.** Today grazers take 1% of the growth. A field that covers the map
   can overshoot, eat a zone bare and crash. Real brakes are physical: food that is hard to digest (the frame needs
   an enzyme: reaction 13 exists), bites that get thin and slow to find (Holling), plants out of reach by height,
   eaters. Build one trophic level first and make it stand before the next.
3. **The cover must offer what animals eat.** See section 6: seeds as a store, leaf against wood. The grazing
   interface is the cover's `take` and `breaking`; it should take from a slot by its height and part, not from the
   cell's mean.
4. **A slot count that is too small hides variety.** 4 slots of cover work because alike kinds merge. Animals need
   room for several sizes at each trophic level. Measure 6, 8 and 12.
5. **One formula inside a cell.** The roadmap found two laws for soil liquid inside a cell (the core's and the
   drawing's). The animals' places must be one function that the core and the page both read.
6. **Time.** A month is long for a mouse and short for an elephant. Eaters and prey with the take law have no exact
   sum over a month, as the cover's simpler month has. One long step will swing or overshoot. Expect steps inside
   the month (a week, as the bodies' physics has today) or a step that stays stable at any length. This is what
   would fail first when built, with grazers that eat a zone bare (item 2). Kinds that live for days are rates, not
   slots.
7. **Seasons and bad years.** With every year the same (section 6, item 1), storing, moving and famine have less reason
   to appear. The animals will look too stable until that is mended.
8. **Threads.** A field makes this simple: one task per run of cells, movers gathered in cell order. The rule "one
   writer for each number" stays; the queue and its landing rounds go.
9. **The people stage needs single agents** (section 4.8). Write the page on the passage between a slot and an
   agent before the field's books are fixed.
10. **Deleting first leaves a time with no animals on `main`,** and some unused doors (section 2.1). That is already
    so in practice (`--plants` everywhere). Tag the last commit that has them, and keep the description of what is
    worth building again.
11. **Slots that join by the mean** can erase a kind that is splitting in two (4.1, step 7).

## 6. What the natural world misses for the later stages

Ranked by how much each one decides later. "Kind of work" says whether it is a law, data, or a start condition.

| # | Missing | Why it matters later | Kind of work | When |
|---|---|---|---|---|
| 1 | **Years that differ.** No law of Flow has a random term and the air's long step is smooth: each year repeats the last. Measured (section 10): rain on land changes by 0.02% from one year to the next, the winter by 0.005 K. On Earth rain differs by 10 to 30% and winters by about 1 K. No dry year, no hard winter, no flood | Boom and bust, famine, moving on, and above all **storing** food need bad years. Farming is an answer to them | The real cause is weather: storms that the air's 15-day step cannot hold. So it must enter as a random term with a physical shape, on the mixing by eddies (which stands for that weather), keyed by the seed, the year and the place; the sea's top layer then sums it into slow swings (Hasselmann 1976, from memory). Not noise put on the rain itself. To study before it is built | Before animals are judged |
| 2 | **Seeds on the ground as a store.** A seed lands and takes hold in the same month. The ground never holds a stock of seeds. (A plant does keep fuel, by its `STORE` gene, but inside itself, not as a part an eater can pick) | Seeds are the food that keeps over winter: the base of seed-eaters, of caching, of granaries and of every grain crop. Also a seed bank is how real plants come back after fire or drought | Law (cover): seeds lie as a pool per cell, with a life and an eater's price | With the first animals |
| 3 | **Fresh and salt.** No law reads how salty a liquid is: a body drinks from any ground that holds the key liquid, and so do roots. And the sea of chemistry 0 is kept nearly fresh on purpose (0.2 g per kg against a real 35), so that it turns over and freezes as it does now | Rivers, springs and lakes are why animals and towns stand where they do. Salt itself is something animals seek | Two steps. A term for saltiness in drinking and in roots (small). A truly salty sea changes how the sea mixes and freezes: a change of the climate's laws with its own long runs (not small) | The first with the first animals; the second when the coast matters |
| 4 | **Fast events: fire.** Change runs once a month in each cell. Burning exists as chemistry, but nothing starts it and a month cannot hold a fire | Fire shapes grassland against forest, clears land, cooks, fires clay, smelts. The spec parks it "for M4" | A law for a fast event inside a month: it starts (dry fuel, heat), spreads by the wind and the fuel next door, and ends in the same month, booked whole | Before people; good for god mode |
| 5 | **Life in the water.** Cover is placed on land only; the sea and the lakes are empty | 60% of the map. Fish and shore food feed coasts, river mouths and the first boats | Start condition and a floating kind of cover (no roots), then the animal field in liquid cells | After land animals stand |
| 6 | **Soil that builds up.** Remains rot in a week to a few years and are gone. No humus, no peat. The loose ground has one grain size | Fertility that farming uses up and floods bring back; peat, then coal; clay for pots and bricks; sand, silt and gravel are also what the law of mud and sand (spec 19) needs | Law (Change and Flow): a slow pool of dead matter; two or three grain sizes of loose ground | With the law of mud and sand |
| 7 | **Useful rocks.** Chemistry 0 was cut from 26 to 14 substances. It keeps a hard stone (7, as hard as flint), lime (9), a fat-like fuel (6) and a burning gas (4). It has no metal element at all (its six elements are those of air, life, sand and lime), so no ore and no metal; and no fuel laid down in the ground | Bronze, iron, coal and oil are in the game's first paragraph. The ground with layers (`space/strata.rs`) is the right home: an ore body, a coal seam, a salt bed are makes of bed | No new law, but not free: new elements, substances and a reaction that frees a metal with heat and fuel; every loop over substances grows (the cut to 14 was made for speed); the richness checks already say what must exist | When people are near, not before |
| 8 | **Wind with a direction.** The air mixes and overturns; a small island is one block with no wind at all. No wet and dry side | Rain shadows and deserts; later sails and mills | Law (the mean wind's momentum): already on the orchestrator's list | In the nature track |
| 9 | **Day and night, and the spread inside a month.** Frost kills by the month's mean and a tuned spread (`frost_k`) | Shelter at night, frost on clear nights, the daily round of animals | A law for the spread of temperature within a step, from the surface's heat budget (already noted in `review/plan.md`) | With habits (4.4) |
| 10 | **Patches inside a cell.** A cell's cover is one stack of layers over its deep ground. No cleared, burnt, grazed or tended patch | A field of crops is a patch of a cell. So is a burn scar or a grazing lawn | Law: a cell's land as a few tiles (the ground already has land, top and deep as tiles) | Before people |
| 11 | **Cycles over thousands of years.** The carbon gas has no source (it settled near 260 ppm in older runs); the mineral washes to the sea and only fixers bring it back | A run of 5,000 years must not drift into ice or bare ground by a missing term | A check first: one run of 3,000 years, plants only, reading the gas and the mineral | In the clean-up, as a measure |
| 12 | **Snow on food, and natural shelter.** A grazer reaches the cover under any depth of snow (only standing liquid hides it). No cave, no lee of a hill, no warmer air under tall cover: a body's heat reads the open ground alone | Winter is the hungry season because snow hides food; shelter in the cold was the old world's first failure (`docs/reports/m3.md`) | Law: reach under the soft cover, as under liquid; the warmth of a place inside a cell (the law inside a cell, 4.2) | With F1 and F3 |
| 13 | **Sickness** (bodies inside bodies) | Plagues, medicine. The spec parks it | Later, as a field inside the animal field | After people |

### 6.1 Counting what the land offers

The owner asked about quantifying resources. Two parts.

**In the tape, per cell and year (physics only, no names):** what the cover grows (the budget already has it), split
into what a grazer can digest and what needs the frame's enzyme; seeds made; liquid that is fresh and standing or
running; the depth of loose ground; the mineral in the soil; which make of bed crops out; the warmth and its
swing. Most are one line each in `tape/src/rec/`.

**In `analyze/`, as maps with familiar names (the one-way translation layer; the crate stays, only its three
detectors of bodies go):** "food for grazers, kg per km² and
year", "fresh water", "building stone", "good soil". From these one can say, before any animal is placed, how many
grazers of 100 kg a valley can feed. That number is the check of section 4.7, and later the first estimate of how
many people a land holds.

## 7. The roadmap, revamped

The old milestones (M0 to M8) were steps of one growing agent world. The new road is by layers. Every stage ends
with pictures at the owner's views and a short list of numbers in range. A stage that changes laws deletes what it
replaces in the same merge.

**The world's size.** Not a planet. The reference world is **one island or small continent in the planet's air**
(the Wide band), 64 x 64 squares to judge by and 128 x 128 to show. The globe stays a climate test only (decision 3).

| Stage | Goal | Ends when |
|---|---|---|
| **N. Nature, finished** (in work) | The air and sunlight; the sorted start; a wet and a dry side; mud and sand (plains, marsh, deltas); ice last | The orchestrator's list of section 5 is on `main` |
| **C. The clean base** (section 8) | The natural world's month and its gate stand without animals; the tree's tools and the stale docs are out; the spec says what is true. The old animals themselves go here or at the start of F1 (decision 1) | a plants-only run the same to the last digit; the spec rewritten; `core` under about 11,000 lines once the animals are out |
| **R. What the land offers** | Years that differ; seeds on the ground; a term for saltiness; snow on food; the count of resources in the tape and as maps | A map of "food for grazers" and of fresh water that reads right. Over 100 years, rain on land differs from year to year by 10% or more and the winter by 0.5 K or more (today 0.02% and 0.005 K), with the climate's key numbers still in their ranges |
| **F1. Grazers** | The page on the passage to single agents (4.8). Then the animal field with a few kinds of grazers of different sizes, no eaters of animals, no movement between cells: plants and grazers hold each other | Rows 1 and 2 of 4.7 (the share eaten, the grazers' mass) in range for 500 years; numbers fall with body mass across the kinds (row 3); the books; the cost |
| **F2. Movement** | The flow between cells by what is sensed; dispersal; new kinds by the slot rule | Kinds sort by zone (row 5); the gain per member comes out nearly even between neighbour cells, without being put in; a wide ranger's mean latitude turns over the year more than a short ranger's (row 7) |
| **F3. Eaters and the web** | Eaters of animals, eaters of remains; group size; overlap inside a cell | Rows 4 and 6 of 4.7; a kind dies out in one region and comes back from next door |
| **F4. Animals on the page** | Single animals and herds drawn from the field, in the Earth style; the inspect panel | The owner sees a herd at the 200 m view, where its kind likes |
| **G. The small game** | God tools on the living island: fire, flood, drought, raise land, add and remove life; save before each; "then and now" | An action has a result one can see after 50 years. This is a finished thing by itself (the spec's option 3) |
| **F5. Habits** | Learned habits as a field (4.4, step two); day and night; life in the water | A habit spreads across the map faster than a gene, against a null run |
| **P0. The people decision** | One design round with the owner: what a people-like kind needs beyond F5. My sketch: stores outside the body (a general law: some animals cache); ground that a kind changes (patches, item 10; some animals dam and dig); single agents and single things on top of the field (4.8) | A page of decisions, before any code |
| **P1 and on** | Tending and stores; settlements; routes; then metals and fuels | Each with its own watch report |
| **T. Names** (beside F4 and later) | The pattern finder and the library on what exists: biomes, kinds ("≈ deer"), later the rest | A person who does not know the project can say what they see |

What changes in the spirit:

- **G moves early.** A living island that one can poke is a good, whole thing. It also gives fire, flood and drought
  a use long before people.
- **Choose with brains is no longer the next big step.** It was the plan of October ("the only road to
  settlements"). The reports since say small brains per node did not learn. The road now goes through an ecosystem
  that stands (F1 to F3), then habits as a field (F5), then single agents only for the few who need them (P).
- **The world search (M7) and random chemistries wait** until one world makes something worth searching for.
- **Speed is a track of its own, with one number.** Flow is 80% of a plants-only year. At size 64 a year costs 1.5 s,
  so 5,000 years take 2 hours; at 128, about 8 hours (est.). For the people stage that is too slow. The honest
  options: Flow's own cost (the air's solve), cells of 4 km for the slow physics with the ground's samples staying
  at 125 m, or an air that is solved less often when its inputs did not move. Measure, then choose. Target to agree
  with the owner: **1,000 years of the 64 island in 10 minutes** (0.6 s a year), all layers.
- **People will also need short time.** A person acts in hours and days and at metres; the world steps in months on
  2 km. The single agents of 4.8 will need their own short steps in the few places where they are. That is a second
  speed number, to set at P0.

## 8. The clean-up cycle

Yes. Before animals, and before the spec misleads one more agent. In this order, each its own merge:

| Step | What | Check |
|---|---|---|
| C0 | **Wait for the two branches in flight** (the air and sunlight; the sorted start) or agree which files the clean-up may not touch (`flow/air/`, `chem/`, `cover/`, `worldgen/life/`) | no merge conflict by design |
| C1a | **The natural world's month on its own.** State what a plants-only month runs, in today's order, without the bodies' modules: the cell sums the air reads, the cover's month and its seeds' landing, the release of the light set aside. Move the shared helpers to a neutral place. Plant kinds keep the numbers their draws are keyed by | `ew run --roster --plants` at sizes 22 and 32: `series.csv` the same to the last digit, before and after |
| C1b | **The gate without animals.** The main guard and the key numbers plants-only (seven of the nine numbers stay as they are). The three thread tests of the natural world in `core/tests/threads.rs` stay and keep running | the fast suite passes with `--plants` worlds only |
| C1c | **Tag `animals-v2`, then delete the old animal world** (section 2.1): code, tests, tape columns, start rows, drawing, the pattern finder's three detectors. A short file keeps the anchored laws and what is worth building again (4.8) | as C1a, on every column that stays. `SNAPSHOT_VERSION` up |
| C2 | **The tree's tools** (section 2.2): ids without serials; the most depth; the cases of cells finer than a square or a sample; the stale comments. And the small dead things of 2.3 and 2.4 | bit for bit |
| C3 | **One helper per idea**, from the roadmap's section 8 and section 11 here: one solid surface of a cell (three formulas today); one heat capacity of a ground node (two); the light the cover catches (the air and the cover each have a law); one meaning of "sea"; seeds mixed by a real hash, not XOR; one name for one thing | bit for bit where no law changes; else its own law commit |
| C4 | **The spec rewritten** to the present: sections 3 to 8 and 12 to 15 of `docs/handover.md` by the seven rules of 1.3 and the roadmap of 7; it says which document wins; `docs/laws.md` loses sections 3, 5, 6 and the body half of 4 (kept in the tag, and their anchored rows in a short file); `README.md` checked line by line | the list of section 11.2 is empty |
| C5 | **`review/` down to two files**: the orchestrator's handover and one roadmap of what is open (this file's sections 6 to 9 can be its base). The older looks and both reports leave (git keeps them). `CLAUDE.md` section 1 points to what exists | a new agent reads under 15,000 words before work |
| C6 | **Measures**, no code, made before C3 so that it has a base to compare with: one plants-only run of 3,000 years (item 11 of section 6); the cost per layer at sizes 32, 64, 128 | two tables in the handover |

Two to three weeks with two helpers (est.). C1a is where a surprise can hide: it touches the order of the plants'
month. If it cannot be made the same to the last digit, it becomes a change of its own, judged by the key numbers,
and the owner is told.

What I would **not** clean now: the chunks of `life/` that the cover runs on (the animal field will use them), the
parts of a cell in Flow, the sleep of Change (it is used), the globe (decision 3), the thread tests of the natural
world.

## 9. Decisions for the owner

1. **Delete the old animal world now** (with a tag), and build the animal field new? Or keep it on `main` until the
   field replaces it? I recommend: do C1a and C1b now in any case (they free the natural world's gate), and delete
   (C1c) when F1 is about to start, so that no door stays unused for long.
2. **Animals as a field** (section 4), with no brain per animal in the first stages? I recommend: yes.
3. **The globe:** keep it as a climate test only, or delete it and the `--fine` window with it? I recommend: keep the
   test, delete `--fine`. It is the owner's word that the world need not be full scale.
4. **The chemistry generator and richness checks:** move to a branch until a world search is near? I recommend: yes.
5. **The order after the clean-up:** R (what the land offers) before F1 (grazers), or side by side? I recommend:
   side by side, with "seeds on the ground" and "snow on food" first, since F1 reads both.
6. **The small game (G) before people?** I recommend: yes.
7. **The speed target** of section 7 (1,000 years of the 64 island in 10 minutes)?

## 10. The measures behind this look

Release build, 4 cores, seed 1, `ew run --size 32 --roster`, the cover's law of `main`.

| Run | Years | ms a year (last tenth) | Flow | Change | Bodies stage (cover, survey, bodies) | Tape |
|---|---|---|---|---|---|---|
| `--plants` | 40 | 235 | 176 to 201 | 7 to 9 | 43 to 51 | 2 to 4 |
| with animals | 70 | 299 | 180 to 211 | 8 to 10 | 86 to 98 | 3 to 4 |

Element drift 1e-15, energy drift 1e-14 to 3e-14 in both. With animals at year 70: 219 bodies (200 nodes),
5.0e5 members, 2.9e6 kg; 7.8e6 kg of cover eaten a year, 2.2e6 kg from the ground, 2.0e5 kg of other bodies; 26 to
33 µs an update; 2,700 updates a year; kinds 6 and 9 of the roster hold 3 and 11 members. The cover's budget in that
year, J: stored by light 4.9e16; upkeep 2.8e16; building 4.8e15; died 6.5e15; given to the ground by roots 7.4e15;
shed 2.1e15; taken or broken by bodies 1.7e14.

How much one year differs from the next (the plants-only run, years 20 to 40, the spread of the yearly change about
its trend): rain on land 0.02% of its mean; the land's mean temperature 0.004 K, its winter 0.005 K, its summer
0.007 K.

Code lines without comments and blank lines: `core/src` 16,800 (Flow 5,380, of it the air 2,360; `space` 1,650;
`bond` 1,090; `choose` 1,040; `body` 1,040; `life` 1,030; `cover` 920; `store` 740; `change` 610; `copy` 510);
`core/tests` 5,070; `tape/src` 3,060; `web/src` 3,430 and its shaders 2,090; `worldgen/src` 1,540; `cli/src` 710.

## 11. Appendix: the leftovers, with their places

From two read-only audits made for this look (one of the code, one of the docs), with the main claims checked by
hand. (a) is dead: nothing reaches it. (b) is reached but does nothing under the fixed cells. (c) is live.

### 11.1 Code

| What | Where | Class | Lines (about) |
|---|---|---|---|
| `Purpose::Quarter` | `core/src/rng.rs:39` | a | 2 |
| `units::HOUR_S` | `core/src/units.rs:7` | a | 1 |
| `Chemistry::light_reactions`, `Derived::light` | `core/src/chem/roles.rs:60`, `chem/mod.rs:143` | a | 8 |
| `World::rained` (written and saved, read by no law) | `core/src/world.rs:122`, `flow/air/book.rs:99` | a | 6, a snapshot bump |
| `Terrain::below` and its branch in `Terrain::inside` | `core/src/space/below.rs:112`, `terrain.rs:249` | a | 8 |
| Cell serials (`at_serial`, `GONE`, `next_serial`, `Life::at`) | `core/src/life/mod.rs:28-265`, `life/saved.rs` | b | 45, a snapshot bump |
| `Regions`, `across_corners`, `Leaf::parent` | `core/src/choose/features.rs:110-195`, `space/tree.rs:44` | b | 75 |
| A cell finer than a sample | `flow/bodies.rs:68`, `flow/basin.rs:70`, `flow/curve.rs:262`, `space/samples.rs:116` | b | 15 |
| `Mesh` of any number of levels | `core/src/space/mod.rs:207-259` | b | 15 |
| `DEPTH_MOST` 24 | `core/src/space/tree.rs:8` | c | a number |
| `Bodies::anywhere` (always false), `Bodies::cat[0]` (always off) | `worldgen/src/life/kinds.rs:31-34` | b | 6 |
| `Life::absorb`, `Life::keep`, rows 2, 8, 9 of the sums | `core/src/life/mod.rs:86-95`, `survey.rs:122-158` | c, in the wrong place | 20 |
| The test of a tree of three depths | `core/tests/space.rs:227` | tests what no world has | 30 |
| Five `#[serde(default)]` on the chemistry file's fields | `worldgen/src/chemistry.rs:39-71` | c, against the letter of rule 2.5 | 5 |
| The solid surface of a cell, three formulas | `ground.rs:162`, `flow/land.rs:287`, `flow/stack.rs:42` | c | one helper |
| The heat capacity of a ground node, two sums | `ground.rs:94`, `flow/land.rs:143` | c | one helper |
| The light the cover catches, two laws | `flow/air/cells.rs:314`, `cover/grow.rs:102` | c | to study |

Checked and clean: no flag of the dropped designs is left in `cli/src/args.rs`; every export of `wasm/` is called;
every file and shader of `web/` is used; no `#[serde(default)]` reads an old save; no TODO.

The parts of the old animal world, to size the deletion of C1 (raw lines, comments counted): `body` 1,510, `bond`
1,500, `choose` 1,440, `life` 1,400 (most of it), `copy` 750, `brain.rs` 310, `nodes.rs` 250, `lineage.rs` 120,
`lanes.rs` 60. Inside them, parked for longer still: two parents (`copy/mix.rs`), watching (`choose/learn.rs`),
lots (`bond/lots.rs`), rooms and wading (`bond/sense.rs`). About 45 of the 130 fields of `Physics` and `Config` are
read by animal code only.

### 11.2 Docs

| Document | State | What to do |
|---|---|---|
| `docs/handover.md` | 15% stale, at its core: 5.2 (L166-183), 5.4, 7.2 (L435-450), 7.3, 7.4, 8.1, 12.3 (the guard needs animals), 12.6, 13, 13.1 (L1000), 13.2, 14.3, 19; L120 ("plants with no brain"); L291 ("bands come with the planet at M4") | rewrite (C4) |
| `docs/laws.md` | true; `life.rs` should be `life/` (L10); 1.8's title; "a finer cell" wording in 5.2, 5.3, 6.3 | small edits now; sections 3, 5, 6 leave with C1 |
| `docs/numbers.md` | true; `wind_ground` and `forced` (L40) are not in the code; three constants named in the wrong file | small edits |
| `docs/tape.md` | true; "the camera's path" in the header (L16) is gone | small edits |
| `docs/reports/m3.md`, `fine.md` | history of the old world | leave the repo (C5) |
| `README.md` | the speed line (L23) is old and dated; no example with `--roster --plants`; lists four docs | C4 |
| `CLAUDE.md` | L15 names `review/big-look.md` as the latest look; the table of places leaves out `cover/`, `space/`, `tape/src/ground/` and `web/` | C5 |
| `review/orchestrator.md` | current; items 1, 2 and 4 of its section 5 are the same item; 209 against 213 tests | stays, shortened |
| `review/roadmap.md` | sections 9 to 11 current; 1 to 8 are of code that is gone | its open items move to the one roadmap (C5) |
| `review/plan.md`, `handover.md`, `top-30.md`, `big-look.md`, `objects.md` | fully replaced | leave the repo (C5) |

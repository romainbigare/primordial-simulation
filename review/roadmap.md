# The roadmap: a clean base, then animals as a field

For the owner and the next orchestrator. Simple English. This is a look at the whole project as it stands on `main`:
the spec (`docs/handover.md`), the laws (`docs/laws.md`), the handover (`review/orchestrator.md`), and the code of
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
  things with a place and a life. People will need a second layer of single agents on top of the field (section
  4.8). By the owner's decision no work goes into it before the animal layer is done.
- **The natural world misses things that animals and people will need:** years that differ (today every year is the
  same year), a store of seeds on the ground, fresh water against salt water, fire, life in the water, soil that
  builds up, and useful rocks (ores, fuels, clay, salt). None is urgent for plants. Several decide whether farming,
  storing and settling can ever appear.
- **Yes, a clean-up cycle first.** Two to three weeks of work (est.). Its largest step is to delete the old
  animals. That is less simple than it looks: the plants' month runs on the animals' machinery and must be cut free
  first. Done in the right order (section 8), a plants-only run stays the same to the last digit.
- **The roadmap changes shape.** Not M0 to M8 on one growing code base, but layers, each ended by a picture the owner
  can judge. The owner's order: free the plants from the animals, delete the animals, build them again as a field;
  beside it, nature goes on (the air and sunlight, the sorted start, worlds that differ). Then the small game, and
  people after the animal layer. Section 7.1 is the one list of priorities, joined with the last session's handover.
- **A thin contract between the land and the bodies** is the first step (section 4.9): what a cell shows, and four
  ways to take from it and give to it, all through the books.

## 1. The shift: what changed

### 1.1 The old ways

Four ideas ruled the project before. Each was dropped for a measured reason.

| Old idea | What it cost | Where the reason is |
|---|---|---|
| **Every living thing is a body.** A plant stand ran the full organism model (reactions, melting, pores, roots, a brood). Speed came only from merging alike bodies into nodes | 60% of a year went to bodies. Three speed rounds gave 3.8 times, never 10. Every cheaper design "changed the world" and was refused by a test of sameness | git history |
| **The cells follow the camera.** One tree of cells; a cell splits where one looks; the step follows the size; zoom slows time | A year went from 1 s to 3 to 7 s. The world flickered. The history of a world depended on where one looked. And "nothing new appears at fine grain" | git history |
| **The picture is made by the core.** One fine window, on the CPU, in the same worker as the world, with 25 layers that each kept the cell's mean exactly | 0.35 to 1.1 s per window; the world waited for its picture; steps at cell edges | git history |
| **The ground is a function of the seed.** Noise at any depth, never state | No valley, no lake, no river bed; erosion could never show | git history |

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

### 2.1 The old animal world: deleted

The old animal world is gone from the code (the commit `16a774d` on main's history is the last that holds it): bodies, brains, Copy for bodies,
population nodes, Bond's parts and structures, lanes, the registry and queue of `life/`, the bodies' half of the
survey, their tests, tape columns, start rows and drawing, and the pattern finder with its three detectors. What the
cover's month runs on stays: the cells with their cover, the chunks and the parallel pass, the seeds' landing. The
contract (4.9) stays, with the plain taker of the tests (`worldgen::testkit::Taker`) as its user: what a cell shows,
take, give and press. The gate runs on worlds of plants.

What is lost with it, said plainly: the only code for single agents that cross cells with exact books (the queue,
the landing in an order fixed by the data, the exact split and join of members: section 13 keeps the last as
knowledge; people will need the idea again, 4.8), and rooms and held things with real physics (`bond/`, in the tag).
The anchored laws and numbers the new animals reuse are in section 13.

### 2.2 The tree of cells: deleted as a design, its tools remain

The tree itself is still used and stays: cells of 1, 2 and 4 squares need its balance and its neighbours, and the
samples are its leaves of depth 6. What remains of the tree **that moved** is this:

- `core/src/space/tree.rs`: the most depth is 24 ("a leaf of 0.5 mm, finer than any body"). Cells need depth 2,
  samples 6, the tiles about 13.
- `Leaf::parent`: so that a cell finer than a square sees as far as a square. No world has such a cell.
- `Terrain::below` and the branches `SAMPLE_DEPTH.max(l.depth)` in `flow/`: the case of a cell finer than a sample.
- `Mesh` of any number of levels (`space/mod.rs`): it always builds two.
- Comments that still tell of splits: `flow/drain.rs` ("sub-stepped where a cell is too small").
- In the spec: 5.2, 7.2, 7.3 and 7.4 still describe cells that split and join, the camera as a recorded input, the
  step that follows the size, and zoom that slows time. `docs/laws.md` 1.8 is still titled "The step by the size".

### 2.3 Plants-as-bodies: gone with the animals

### 2.4 Tools and plans of older rounds

- `core/src/inside/` (the law inside a cell): no law of the core reads it now. Only the tape and the page do. It
  is the right idea in the wrong crate until the animals need it (section 4.2); say so in its doc, or move it.
- `ew view` (a window of a snapshot as a PNG, with a "depth" layer): made to look at fine cells.
- `--fine`, and the globe: see the owner's decision 3 in section 9.
- Small dead things: five classes in the sleep clock where two are used.
- Names that mean two things (`CLAUDE.md` 2.8): "bodies" is the bodies of standing liquid (`flow/bodies.rs`) and
  the plants' bodies (`body matter`); `Grid` is three types; `Books` is three.

### 2.5 The docs

- The documents are `CLAUDE.md` (the rules), `docs/handover.md` (the spec), `docs/laws.md`, `docs/numbers.md`,
  `docs/tape.md`, this roadmap and `review/orchestrator.md` (the handover). The spec says which one wins (its
  section 0). `CLAUDE.md` still names `docs/analyze.md`, which is gone with the pattern finder.
- The spec's sections about the old animals (the laws of bodies in 6.1 to 6.6, the pattern finder and the library in
  9, god mode in 11) describe code that is gone (the commit `16a774d` on main's history is the last that holds it). They are rewritten or cut (C4).
- `docs/laws.md` is true to the code and describes no animal.

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

The first stage (F1, the takers of the cover) is designed in full in `review/f1-design.md`, with the
orchestrator's decisions at its top. Where it and this section differ on F1, it holds: eight genes, not twelve; the
young disperse by the seeds' kernel already in F1; a mutant never founds a slot by one generation's step (the slot
rule chooses between kinds that exist, for the cover as for takers: a kind that splits in two is a law still to
write); the refuge that keeps takers from eating a cell bare is what a mouth cannot reach.

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

This is the real design question. Today each node has a small neural network that learns in its life. Two watches
of it say it learned nothing that beats chance beyond its hand-set start. I propose two steps.

- **Step one: habits as inherited numbers.** This is still Law 5, with the smallest brain: what a kind senses (its
  hunger, the warmth, the food, the crowd and the eaters, here and next door) times a few weights gives the shares
  of its time over the same general acts as today (take, move, rest). The weights are genes and evolve as every gene
  does. It is "choices as a spread of actions" (`laws.md` 4.4 at the commit `16a774d`), which nodes already are, with no hidden layer, no
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
| Grazer mass follows rain and plant growth | 0.1 to 10 g per m² over African parks, typical 1.7 | Fritz and others 2002; Hempson and others 2015; Coe, Cumming and Phillipson 1976 |
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

The owner's decision: people come after the animal layer is done, and no work goes into them before. Two things
done for the animals also serve this passage, at no extra cost: counts that are real and split and join exactly
(4.3), and the contract of 4.9, where a single agent is one more taker beside the field. I cannot promise the field
will carry people; I can only say that wild animals do not need more, and that the passage is the thing to get right
when its time comes.

### 4.9 A contract between the land and the bodies

The owner's idea: a layer between the natural world (plants and what the land holds) and the bodies, so that each
side can change without the other. I looked at it with care. **It is a good idea, in a thin form.**

**What the code has today.** The ground and the air already have such a contract: a body never touches them, it
asks the cell's view of the store (`store::Moves`), which gives no more than is there and books both sides. The
plants have none. Bodies reach into the cover's own numbers in eight places: `Cover::take`, `Cover::breaking`,
`Cover::cut_by`, `cover::lose` (what movers break), `World::canopy` (hiding), and the cell's sums by position
(`Life::sum(i)[1]` is mass, `[7]` is the energy to break). And the land reads the bodies the same way (the mass
that holds the ground against the flow, the light kept). This is why the plants cannot be cut free in an hour.

**The contract, as I would write it.** One small module that both sides use. Neither side uses the other.

| Part | What it is | Who writes it | Who reads it |
|---|---|---|---|
| **What a cell shows** | Its layers of matter that can be taken: loose ground, remains, each slot of cover by the height it stands at, later the seeds on the ground and the standing liquid. For each: how much of each substance per m², its heat, and the energy to break a kg of it loose. And the place: ground warmth, depth of liquid, depth of snow, how hard it is to pass, how much it hides | the natural world, once a month, only if someone reads | every taker; the tape (this table **is** "what the land offers", section 6.1) |
| **Take** | "I ask for this much of this layer." The answer is what is given, with its heat, and what breaking it costs | a taker | booked on both sides through the store |
| **Give** | Matter and heat back to the ground: remains, droppings, seeds dropped | a taker | the same |
| **Press** | Work spent on a cell breaks its cover by one law (what movers trample) | a taker | the same |
| **What stands on a cell** | The mass on it, the light it catches and keeps | life (plants today, animals later) | Flow |

Three rules make it honest:

1. **It moves matter through the books, it holds no copy.** What a cell shows is read from the one source; a take
   is a move in the store. Nothing can differ between the two sides (`CLAUDE.md` 3.1, 3.3).
2. **It shares when takers ask for more than is there.** Each gets its share of what it asked for, so no cell is
   overdrawn (`CLAUDE.md` 3.4). The order is fixed by the data.
3. **It says when.** The cell shows itself as the month's start left it; takes land during the takers' month; the
   cover feels the loss in its own next month.

**What it gives.**

- The plants build, run and are tested with no body code. The world with no taker is the plants-only world.
- A **test taker** ("takes a tenth of the low layer each month") lets the cover's tests check what grazing does to
  plants before any animal exists. It also keeps the door in use between the deletion and the new animals, so no
  code lies unused.
- The animals can be deleted, built again, and changed later without one edit in the cover or in Flow.
- Later takers come through the same door: the animal field, single agents, a god's hand.
- A test can guard it, as the word ban is guarded: no file of the natural world names a module of bodies.

**Where it can go wrong.**

- **Too much machinery.** Version 1 died of layers. This must be plain data and four functions, about 150 to 250
  lines (est.), with no plug-in system and no switch. If it grows past that, it is wrong.
- **Too narrow.** Not all that passes between land and bodies is food. Shade, shelter, snow, trampling, seeds
  carried in a gut are in the table above because they are easy to forget. What is missing from the contract will be
  taken behind its back.
- **Fixed too early.** The first real taker (grazers) will show what is missing. The contract should change freely
  until then: no old form kept.
- **A cost with no taker.** The table of a cell is made only when a taker reads it, as the brains' features are
  today.
- **Blurred layers.** A grazer must be able to take the low leaves and not the trunk. So the cover shows itself by
  slot and height, not as one mean of the cell (today a bite takes "all its parts in the field's proportions").

So: yes. It is the right first step of "free the plants" (C1a), not an extra one.

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
5. **One formula inside a cell.** There are two laws for soil liquid inside a cell (the core's and the
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
9. **The people stage needs single agents** (section 4.8). No work on it now. Keep counts real and exact, and keep
   the contract (4.9) open to a taker that is one being.
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
| 9 | **Day and night, and the spread inside a month.** Frost kills by the month's mean and a tuned spread (`frost_k`) | Shelter at night, frost on clear nights, the daily round of animals | A law for the spread of temperature within a step, from the surface's heat budget | With habits (4.4) |
| 10 | **Patches inside a cell.** A cell's cover is one stack of layers over its deep ground. No cleared, burnt, grazed or tended patch | A field of crops is a patch of a cell. So is a burn scar or a grazing lawn | Law: a cell's land as a few tiles (the ground already has land, top and deep as tiles) | Before people |
| 11 | **Cycles over thousands of years.** The carbon gas has no source (it settled near 260 ppm in older runs); the mineral washes to the sea and only fixers bring it back | A run of 5,000 years must not drift into ice or bare ground by a missing term | A check first: one run of 3,000 years, plants only, reading the gas and the mineral | In the clean-up, as a measure |
| 12 | **Snow on food, and natural shelter.** A grazer reaches the cover under any depth of snow (only standing liquid hides it). No cave, no lee of a hill, no warmer air under tall cover: a body's heat reads the open ground alone | Winter is the hungry season because snow hides food; shelter in the cold was the old world's first failure | Law: reach under the soft cover, as under liquid; the warmth of a place inside a cell (the law inside a cell, 4.2) | With F1 and F3 |
| 13 | **Worlds that differ in shape.** The land falls away from the middle of the map by one rule (`terrain::fall`), so every world is one island in the middle with small islands off its drowned coast | Several islands make kinds that differ from island to island, crossings, and later boats and far lands. A game wants each seed to give another world | Start condition (`space::terrain`): the fall from the middle becomes a drawn field of several rises, with channels of sea between; the share of land, the count of islands and their sizes drawn from the seed | In the nature track, after the air |
| 14 | **Sickness** (bodies inside bodies) | Plagues, medicine. The spec parks it | Later, as a field inside the animal field | After people |

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

## 7. The roadmap

The old milestones (M0 to M8) were steps of one growing agent world. The new road is by layers. Every stage ends
with pictures at the owner's views and a short list of numbers in range. A stage that changes laws deletes what it
replaces in the same merge.

**The world's size and shape** (the owner's decision). No globe. A world is a square map of **128 x 128 squares**
(256 km a side), 64 to judge by; 256 or 512 at the very most. A world of 512 has 64 times the cells of a world of 64,
so today it would cost about 64 times as much per year: sizes above 128 wait until speed is looked at. Worlds should
differ as much as they can: one island, several, a ring, a long coast (section 6, item 13).

**Speed** (the owner's decision): no target now. Build the most true, most interesting and best-made system; cut
back when it has become too slow. Each merge still writes down its cost per year, so that the day of cutting back
has numbers to cut by.

### 7.1 The one list of priorities

This list joins the handover of the last coding session (`review/orchestrator.md` on `main`) with this look. Two
tracks run side by side, one helper each: **nature** (N) and **life** (L). The orchestrator does the docs between.

| # | Track | What | Why here | From |
|---|---|---|---|---|
| 1 | L | **Free the plants.** The contract (4.9); the natural world's month stated on its own; the gate and the key numbers without animals (C1a, C1b of section 8) | The owner's first priority. It must be on `main` before item 2, so that the references move once, on a gate that no animal can fail | this look |
| 2 | N | **The air's low cloud, then the air and sunlight to `main`.** A check on paper first (does a low cloud made by cooling from below give the pole its cloud and an ice line?), then the law, then the whole package in one merge. The references move once; tell the owner each value | The light sets how fast every plant grows (about 2.3 times slower with true light). Everything that feeds on plants is judged after it | handover 1 |
| 3 | L | **Delete the old animals** (C1c), then the tree's tools and the small dead things (C2) | The owner's second priority. The test taker keeps the contract in use | this look |
| 4 | — | **The docs say what is true after each merge**: the laws of the old animals leave `docs/laws.md` and the spec with item 3 (C4) | So that the next agent does not build toward a planet with cells that split | this look |
| 5 | N | **The sorted start**, after the air: first main and a sorted world run side by side to year 300; then its kinds and stands; then its look. To `main` only when its pictures are at least as good | Asked for by the owner. Its gardens change with the light, so it follows item 2 | handover 2 |
| 6 | L | **What the land offers, first part**: seeds that lie on the ground; snow that hides food; the contract's table recorded in the tape as the count of resources | The first animals read all three | this look |
| 7 | L | **Grazers** (F1): the animal field with a few kinds of grazers, no eaters of animals, no movement | The owner's third priority: build again, one level at a time | this look |
| 8 | N | **Worlds that differ**: several islands and other shapes from the seed; sizes to 128. The globe as a world leaves (`--globe`, `--fine`, its view on the page) | The owner's word on size and variety. After item 2, because the air's law is judged on the lifeless globe until it is merged | the owner; this look |
| 9 | N | **Small things on the page**: the four commits on small plants (`wip/lakes`), reviewed with pairs; dark-red specks and blocky dark patches; straight shores; a program without ice where there is none | Fill-in work between packages | handover 3, 4 |
| 10 | L | **Movement** (F2), then **eaters and the web** (F3) | | this look |
| 11 | N | **Years that differ** (to study first), **a wet and a dry side** (the mean wind) | Animals look too steady without bad years; rain shadows make dry land | this look; handover 6 |
| 12 | L | **Animals on the page** (F4): single animals and herds drawn from the field | The owner judges by pictures | this look |
| 13 | both | **The small game** (G): god tools on the living island; fire as a law | A whole thing by itself, before people | this look |
| 14 | N | **Channels that lay down their load** (plains, marsh, deltas), with grain sizes of loose ground and soil that builds up; then ice that cuts valleys; the exact solve of the surface's step; the True view's water | Large and already agreed to wait | handover 7; this look |
| 15 | L | **Habits that are learned** (F5); life in the water; a term for saltiness | | this look |
| 16 | — | **People** (P0, a design round with the owner, then P1 and on); useful rocks and metals with them; names (T) | After the animal layer is done, by the owner's word | this look |

What the handover's list held that is no longer needed: its item 5 (run the slow tests of bodies before the next
change of the laws for bodies). Those tests leave with the old animals; `tape/tests/scale.rs` stays and is run with
item 3.

Still asked of the owner by the handover, with no answer yet: do the summit lake of seed 1 at the 200 m bar and a
lake's shore at the 5 m bar look right on the live page; does the pale rounded band round a summit lake bother them;
does zooming and panning feel slower on their phone.

### 7.2 The stages of the life track, with their gates

| Stage | Goal | Ends when |
|---|---|---|
| **C. The clean base** (section 8) | The contract; the natural world's month and its gate stand without animals; the old animals, the tree's tools and the stale docs are out; the spec says what is true | a plants-only run the same to the last digit through C1; the spec rewritten; `core` under about 11,000 lines |
| **R. What the land offers** | Seeds on the ground; snow on food; the count of resources in the tape and as maps. Later: years that differ; a term for saltiness | A map of "food for grazers" and of fresh water that reads right. For years that differ: over 100 years, rain on land differs from year to year by 10% or more and the winter by 0.5 K or more (today 0.02% and 0.005 K), with the climate's key numbers still in their ranges |
| **F1. Grazers** | The animal field with a few kinds of grazers of different sizes, no eaters of animals, no movement between cells: plants and grazers hold each other | Rows 1 and 2 of 4.7 (the share eaten, the grazers' mass) in range for 500 years; numbers fall with body mass across the kinds (row 3); the books; the cost |
| **F2. Movement** | The flow between cells by what is sensed; dispersal; new kinds by the slot rule | Kinds sort by zone (row 5); the gain per member comes out nearly even between neighbour cells, without being put in; a wide ranger's mean latitude turns over the year more than a short ranger's (row 7) |
| **F3. Eaters and the web** | Eaters of animals, eaters of remains; group size; overlap inside a cell | Rows 4 and 6 of 4.7; a kind dies out in one region and comes back from next door |
| **F4. Animals on the page** | Single animals and herds drawn from the field, in the Earth style; the inspect panel | The owner sees a herd at the 200 m view, where its kind likes |
| **G. The small game** | God tools on the living island: fire, flood, drought, raise land, add and remove life; save before each; "then and now" | An action has a result one can see after 50 years |
| **F5. Habits** | Learned habits as a field (4.4, step two); day and night; life in the water | A habit spreads across the map faster than a gene, against a null run |
| **P. People** | After the animal layer. First one design round with the owner: stores outside the body; ground that a kind changes (patches); single agents and single things on top of the field (4.8) | A page of decisions, before any code |

**Why R and F1 go side by side, and not one after the other.** R is a list of things the land should offer. Only
two of them are read by the very first grazers: seeds that lie on the ground (a food that keeps) and snow that hides
food (why winter is hungry). Those two come first (item 6). The rest of R does not block grazers: years that differ
needs a study of its own, and saltiness, fire and life in the water matter later. If all of R had to be done before
any animal, the animals would wait for months for things they do not read yet. And the first grazers will show what
else the land must offer: it is better to learn that early. So: the two that F1 reads, then F1, with the rest of R
going on beside it in the nature track.

## 8. The clean-up cycle

Before the new animals, and before the spec misleads one more agent. Each step is its own merge.

| Step | What | Check |
|---|---|---|
| C2 | **The tree's tools** (section 2.2): the most depth; the cases of cells finer than a square or a sample; the stale comments. And the sleep clock's classes (2.4) | the same to the last digit |
| C3 | **One helper per idea**, one solid surface of a cell (three formulas today); one heat capacity of a ground node (two); the light the cover catches (the air and the cover each have a law); one meaning of "sea"; seeds mixed by a real hash, not XOR; one name for one thing | the same to the last digit where no law changes; else its own law commit |
| C4 | **The spec's sections about the old animals leave** (2.5; their anchored laws are in section 13) | the list of section 11.2 is empty |
| C6 | **Measures** before C3, so that it has a base to compare with: one plants-only world of 3,000 years (item 11 of section 6); the cost per layer at sizes 32, 64, 128 | two tables in the handover |

The two branches that are parked (`wip/air-light`, `wip/sorted-start`) touch `flow/air/`, `chem/`, `cover/` and
`worldgen/life/`: they merge `main` in.

What I would **not** clean now: the chunks of `life/` that the cover runs on (the animal field will use them), the
parts of a cell in Flow, the sleep of Change (it is used), the thread tests of the natural world, and the round
mesh in `space/`: the strip of the planet that gives an island its climate is built on it (`worldgen/src/planet.rs`),
and the air's law is judged on a lifeless globe until it is on `main`. Only the globe as a world to play leaves
(item 8 of 7.1).

## 9. The owner's decisions

1. **The order:** free the plants from the animals, then delete the animals, then build them again safely. A thin
   contract between the land and the bodies (4.9) is the first step of it.
2. **Animals as a field.** Yes.
3. **No globe.** A world of 128 x 128 is already very large for a game; 256 or 512 at the very most. Worlds should
   differ as much as they can, with several islands among their shapes.
4. **The chemistry generator and its checks move to a branch.** They make a chemistry of chance from a seed (its
   elements, its substances and their numbers, its reactions) and test that it could carry a world (a liquid, a gas
   that holds heat, a reaction that stores light, things that burn, a metal that heat can free). The spec planned
   them for a far stage: many worlds, each with its own matter. No world uses them today: every world runs on the
   one hand-made chemistry, and a chemistry of chance cannot grow life yet. They are not needed until then.
5. **"What the land offers" beside the first grazers,** with the two pieces the grazers read coming first (7.2).
6. **The small game before people.** People come after the animal layer is done.
7. **No speed target now.** Build well; cut back when it has become too slow.

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
| `Terrain::below` and its branch in `Terrain::inside` | `core/src/space/below.rs:112`, `terrain.rs:249` | a | 8 |
| `Leaf::parent` | `core/src/space/tree.rs:45` | b | 10 |
| A cell finer than a sample | `flow/bodies.rs:68`, `flow/basin.rs:70`, `flow/curve.rs:262`, `space/samples.rs:116` | b | 15 |
| `Mesh` of any number of levels | `core/src/space/mod.rs:207-259` | b | 15 |
| `DEPTH_MOST` 24 | `core/src/space/tree.rs:8` | c | a number |
| The test of a tree of three depths | `core/tests/space.rs:227` | tests what no world has | 30 |
| Five `#[serde(default)]` on the chemistry file's fields | `worldgen/src/chemistry.rs:39-71` | c, against the letter of rule 2.5 | 5 |
| The solid surface of a cell, three formulas | `ground.rs:162`, `flow/land.rs:287`, `flow/stack.rs:42` | c | one helper |
| The heat capacity of a ground node, two sums | `ground.rs:94`, `flow/land.rs:143` | c | one helper |
| The light the cover catches, two laws | `flow/air/cells.rs:314`, `cover/grow.rs:102` | c | to study |

Checked and clean: no flag of the dropped designs is left in `cli/src/args.rs`; every export of `wasm/` is called;
every file and shader of `web/` is used; no `#[serde(default)]` reads an old save; no TODO.

### 11.2 Docs

| Document | State | What to do |
|---|---|---|
| `docs/handover.md` | true to the owner's decisions, except the sections about the old animals (6.1 to 6.6, 9, 11 and the lines of 10 that name them), whose code is gone | rewrite or cut (C4) |
| `CLAUDE.md` | names `docs/analyze.md`, which is gone | the owner mends it |
| `review/orchestrator.md` | current; items 1, 2 and 4 of its section 5 are the same item | the orchestrator mends it |

## 12. Open items of the natural world and its page

What is still open in the land, the water, the plants and the map. What is done is in `review/orchestrator.md`
section 4, with its known faults; the lines here are the rest.

**The map and the tiles**
- A tile costs 7 to 10 ms, most of it the channels' field; the goal is 15 ms for a tile in all, and the rest is
  `Terrain::heights`, which builds its patch from the samples down at each call.
- The fitted view uses texels of 125 m; tapes keep the map of squares.
- Seasons blend from month to month; above 36 months a second the page shows the year's mean (A3).
- How long a zoomed view takes to be sharp is not known: the test browser draws on the CPU.
- At 10 m the ground between plants reads brown (herbs cover a fifth; flowers are too small to see). Autumn is mostly
  herbs going yellow; red shows only on the heights.

**The ground**
- Loose ground that settles during a run is not bounded by the `skin_m` rule.
- Building the samples costs 0.5 to 0.6 s on a 32 island.
- Channels that lay down their load (plains, marsh, deltas), and ice that cuts valleys: see the variety below.

**The water**
- A closed basin by the sea that loses more than it gains starts full and falls about 0.1 m a year to its own level
  (3 of 92 bodies in six runs). A body colder than its liquid's melting point at the start starts molten, as the sea
  does.
- Liquid goes straight from the cell it leaves to the body's cell; what it picks up lands with it.
- Liquid over a small pond's part of a cell stands over that part only, as a pillar (`curve::Part`, `Fill`): poured
  over a one-sample pond it stood 30 km high in a test. It should spread over the cell's other samples.
- The passes over the samples cost about 20 ms a month on a 64 x 64 island.
- The surface solve is backward Euler (`docs/laws.md` 1.1, known limit). The design: solve the linear balance exactly
  by pieces, T_e + (T_0 - T_e) e^(-h/τ), the fluxes from its mean over each piece, the end from its end; a plateau is
  a piece where the temperature stands while its fusion enthalpy is paid. It touches `Air::moves`' slopes, the
  cover's cap, `end_reflectance` and evaporation.

**Rivers**
- The faults of rivers and shores are listed in `review/orchestrator.md` section 4 (straight shores, thin tributaries
  from afar, bends between the 500 m and 200 m bars). A river's sediment does not yet build a delta at its mouth: see
  the variety below.

**Plants**
- From afar the kinds are one olive: leaves are alike, and chemistry 0 gives every part of a plant one reflectance;
  stems are pale for the same reason.
- A few lowland cells hold no liquid in their pores among wet ones, so their cover is dormant and they show as pale
  squares.
- The light the cover catches has two laws: `flow::air` computes the light per capture area itself where
  `cover::per_capture_area` states it. One should call the other.
- The mass of plants means two things in the two cover fields (with and without stored fuel).

**The core's speed**
- The parts of a cell are solved after each of the liquid's bookings (7 a month) and after each air step. One solve at
  the end of the liquid step changes the law's timing, not only its rounding: the runoff, the erosion and the mix
  through the thermocline each read a solve made after the step before them.
- The ground's full state is built each month at the survey. To build only what changed: a full thaw where the
  refresh changes the substances present, the fusion enthalpy held found from the amounts, and Flow's state saved
  with a snapshot, so a resumed run stays the unbroken one.
- The heat capacity of a node's amounts is one helper (`ground::amounts_cap`), but two orders of summing remain:
  `World::heat_cap_of` and `land::capacities` add the amounts onto the skin one by one; the parts and the reactor sum
  the amounts first. Joining them changes results in the last bit.
- The wasm build has no `wasm-opt`; SIMD gives 3 to 5%.

**Variety: landscapes and biomes on a small world**
- Wet and dry sides of the island: a mean wind across the map from the band's own circulation, with mountains across
  it, and a law for the mean wind's momentum.
- Years that differ: today rain on land changes by 0.02% from year to year.
- Flat wet ground: channels lay down their load where the slope falls (the law for mud and sand), so valley floors,
  fans, deltas and ground whose pores stay full appear. A delta's size is the load that reached the mouth over the
  depth of the water there (Gilbert 1885; Galloway 1975).
- Ice: where snow outlasts the summer it flows and cuts (a term of the wear's law at the world's start): U-shaped
  valleys, bowls, deep lakes, and with a risen sea, fjords.
- Starting kinds for the new places (soaked ground, cold thin ground, dry ground), and the law that roots in full
  pores get no gas: marsh plants, alpine mats, dry scrub.
- The sorted start, parked on `wip/sorted-start`, needs places that differ and two changes of the cover's law: a
  slot that does not pay its upkeep gives way to arriving seed, and succession (from equal mass, herbs now kill tree
  seedlings). Its first step is in `review/orchestrator.md` section 4.
- Worlds that differ in shape (several islands, a ring, a long coast) are item 8 of section 7.1.

## 13. Anchored laws kept for the animal field

The old animal world's laws whose numbers have an anchor, as `docs/numbers.md` and `docs/laws.md` held them before
the deletion (the code is at the commit `16a774d`). The new animals reuse them; nothing here is new. `M` is a
member's living body, kg; `T` its temperature, K; `pace(T)` the bare rate law of its main burning,
k e^(−Ea/T) over its value at `t_cat` (290 K; chemistry 0's reaction 1, Ea 7,760 K, gives 5.62 at 310 K).

| Law or number | Formula, one line | Value | Unit | Source |
|---|---|---|---|---|
| Upkeep by body mass | upkeep = days × M × (M_copy / `m_ref`)^(`intake_exp` − 1) × (`c_basal` + `c_strength` × STRENGTH) × repair × activity × pace(T) | `c_basal` 4.22e4, `c_strength` 1.12e4, `intake_exp` 0.75, `m_ref` 1 | J/(kg day) at `t_cat`; kg | Kleiber 1932, 1947 (3e5 J a day per kg^0.75 at 310 K, less resting muscle); Elia 1992 (muscle 13 kcal/kg a day); Ernest et al. 2003 (0.755) |
| Pace by temperature | pace(T) = e^(−Ea/T) / e^(−Ea/`t_cat`); force and walk × min(1, pace) | `t_cat` 290 | K | Gillooly et al. 2001 (E about 0.6 eV); Bennett 1990 (Q10 2 to 3 of muscle) |
| Take law | intake a day = `take` × found / (`take` + found) × supply, found = `sweep` × what lies / area | `take` 0.0089, `sweep` 1, `c_handle` 1e3 | share a day per kg at `m_ref`; m² per kg a day; J per kg handled | Holling 1959; van der Merwe and van Niekerk 1980 (sheep 42 to 80 g per kg^0.75 a day); `sweep`, `c_handle` tuned |
| Heat balance through a coat | C dT/dt = P − g (T − tg), surface `meeh` × m^(2/3), insulation `r_air` + HEAT × `r_coat` (in liquid `r_liquid` and `wet_coat` of the coat) | `meeh` 0.1, `r_coat` 0.9, `r_air` 0.12, `r_liquid` 0.005, `wet_coat` 0.2 | m² per kg^(2/3); m² K/W; share | Meeh 1879; Herreid and Kessel 1967; Campbell and Norman 1998; Incropera and DeWitt 2002; Scholander et al. 1950; Porter and Gates 1969 |
| Two heat states | a body whose own heat warms it past its surroundings by more than T² / Ea runs on to a warm state (its `COOL_T`), else lies cold (or at `KEEP_T` by burning) | T² / Ea about 12 K in chemistry 0 | K | the rate law's slope Ea / T² per K against the coat's loss |
| The walk | speed v = √(`WALK_FROUDE` g L), L the side of a cube of a member's volume; cost `c_move` × (m / `MOVE_REF`)^`MOVE_EXP` J per kg per m | `WALK_FROUDE` 0.25, `c_move` 3 at `MOVE_REF` 30 kg, `MOVE_EXP` −0.32 | none; J/(kg m); kg | Alexander and Jayes 1983; Alexander 1989; Taylor et al. 1982 (10.7 m^−0.32) |
| Drag in liquid | J per kg per m = ½ ρ `DRAG` (wet L²) v² / (m `MUSCLE`) | `DRAG` 1.05, `MUSCLE` 0.25 | none; share | Hoerner 1965 (a cube face on); Smith, Barclay and Loiselle 2005 (muscle's efficiency) |
| Wear by the rate of living | wear += energy turned over per kg / (`e_life` × 10^REPAIR); 1 − e^−wear dies (AGE); upkeep × (1 − `c_repair` + `c_repair` 2^REPAIR) | `e_life` 1e9, `c_repair` 0.25 | J/kg; share | Rubner 1908 (about 1,000 kJ per g in a life, reviewed by Speakman 2005); repair 20 to 30% of resting upkeep |
| The brood and the child | mature at `COPY_MASS`; growth puts the share `ALLOC` into a brood; a child is 10^`CHILD` × the copy mass; a node keeps `BROOD_HELD` of a child per member | `ALLOC` gene 0 to 1, `CHILD` gene −6 to log10 ½, `BROOD_HELD` 0.5 | share; log10 share | the brood as a share of a surplus (laws 4.2 at the tag) |
| Frost, one law with the cover | frozen share a month = clamp((limit − `harden` (1 − activity) − T) / `frost_k`, 0, 1); over t: 1 − (1 − f)^(t / month) | `frost_k` 3, `harden` 20 | K | Levitt 1980; Sakai and Larcher 1987 (kept in `Physics::frozen`, the cover uses it) |
| Breaking | energy to break a share f = f × Σ V × `rupture` over soft tissue and each built solid | `rupture` 1 MPa (matter), 120 MPa (the frame) | Pa | soft tissue 0.1 to 10 MPa; cell walls about 120 MPa (from memory) |
| A held edge | each material's energy × min(1, e / l) where the edge is harder (scratch hardness); the edge loses `abrade` × W / (`TABOR` × σ) | `abrade` 1e-5, `TABOR` 3 | none | Atkins 2009; Tabor 1951; Archard 1953; Archard and Hirst 1956; Rabinowicz 1995 |
| Growth's cost | building matter costs `c_copy` per kg, at most `c_build` × supply / `m_ref` × days × activity | `c_copy` 1e7, `c_build` 1e4 | J/kg; J/(day kg^0.75) | growth keeps 10 to 20% of what it eats; P/B of herbs once or twice a year; Ernest et al. 2003 |
| Exact split and join of members | see below | | | McKendrick 1926; von Foerster 1959 (maturity as a flux across a size) |

The exact split and join of the members of a group (`nodes::Lot`, `nodes::rejoin` at the tag):
- A split takes the share f of the members and the share fa of the stores, heat and brood (fa > f where those who
  leave hold more); the rest stays as the remainder, so the two add up to what was held to rounding.
- A join adds members, stores, heat and brood as sums.
- Genes, sizes and own brain rows become means by members; the last day of physics and the busy share too.
- The spread of a gene pools each one's spread plus its gap to the new mean (by members); the spread of body size and
  of brain rows pools the same way.
- Members who come back from acting apart rejoin by amounts and sizes alone: their genes and spreads are the node's.

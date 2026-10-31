# Top 30 changes to make the world run well below the target

For the owner, 31 October 2026. Assumed: the 64 x 64 roster watch confirms the review (`big-look.md`). The aim is the
spec's island of 128 x 128 cells at **0.15 s a year or less** on 4 cores (1,000 years in 2.5 minutes; the target was
10), with the same code running a planet of 2,000 air nodes in the same time. Today a settled 128 x 128 year costs
about 8 s.

A budget first, so each item has a size to hit. 0.15 s a year over 16,384 cells is **9 µs per cell per year**: about
0.8 µs per cell per month for everything (air, liquid, chemistry, plants, animals, brains, tape). Only plain passes
over arrays fit in that. Nothing per cell may solve, search, allocate or branch on 26 substances.

Each item says: what, where, what it deletes, what it gives, and what it costs or undoes. "Controversial" marks the
ones that undo good work or change the world on purpose.

## A. The shape of a step: how the world is processed

**1. One world step is one month, with one pass per layer over arrays.** `schedule.rs` becomes a fixed list of
about ten passes: air (daily inner loop, item 2), rain and evaporation (3), liquid (4), chemistry (6), plants (7),
animals (9 to 14), brains (15 to 17), parts (25), tape. Delete `clock/` (`Every`, sleep and wake), `Config::phys_days`,
`air_hours`, `flow_days`, `sleep_days`, `wake_share`, `batch_days`, `node_days`, `lump_days`, `think_days`,
`sample_every`. Gives: no scheduling code at all, every pass vectorises. Undoes the multi-rate clock (7.3 of the spec
said it was a default to measure, not a law).

**2. The air becomes an explicit daily loop over the air level, not an implicit solve.** Today `flow/air/` (about
1,300 lines) solves radiation, exchange, rain and wind together by Newton's method with a banded solve, up to 16
rounds, every 10 days, with a 9 + 8n-number row per cell per round. Replace it with: per air node per day, a
radiation balance (sun in, σT⁴ out through the gases' optical depth and 1 or 2 grey layers), the exchange with its
block of ground (sensible heat, a bulk evaporation), a pressure push between neighbours, and eddy mixing as one
smoothing pass (with `mix` of 1e6 m²/s over 8 km the columns even out in a minute, so a smoothing pass is the exact
limit, no solve needed). The column's heat capacity (about 1e7 J/m²K) and the skin's (4e6) make a daily explicit step
stable (relaxation times of 20 and 3 days). Cost: 1,024 air nodes x 365 x about 100 operations = 40 million a year,
about 15 ms. Today the air alone is about 1.5 s a year at 128 x 128. **Controversial:** it undoes the implicit solver
and its long-step guarantee. The check is the same the spec asks for: a globe without life holds Earth's climate
(`tape/tests/ranges.rs`, the climate rows).

**3. Cells exchange with their air node once a month.** Rain, snow, evaporation, the gases bodies gave off: one pass
over the cells using the month's mean air of their node. Delete `flow/air/cell.rs`, `book.rs`, the per-step bodies'
node booking (`Class::Bodies` becomes a monthly sum). Gives: 16k cells x 12 instead of 16k x 36 x rounds.

**4. Liquid monthly, one downhill pass and one Jacobi sweep.** Keep `solve::downhill` (the spill levels and
D-infinity are good) for runoff; replace the implicit `solve::spread` for pore liquid by one explicit Darcy sweep per
month with a flux limiter (pore flow is slow; a month's step at 2 km is far below its limit). Delete the sink every
physics step (`flow::sink`), fold it into the monthly pass. Gives: liquid under 20 ms a year at 128 x 128.

**5. The sea is one node per air block, not a cell.** The spec (7.2) says so and it was never built. A sea cell today
has a ground, a bed, a deep store, loose ground, chemistry and a surface, like land. On the island 70% of the cells
are sea. Make `Class::Ground` cells exist only on land (and the shelf), and the sea a per-air-block node with depth,
heat, salt and dissolved gas. Delete the sea's bed chemistry, creep and erosion. Gives: 3 times fewer cells doing
chemistry and liquid on an island. Costs: coasts need one rule for what a land cell drains into (its block's sea
node).

**6. Chemistry runs only where something can react.** Per cell a 32-bit mask of substances present and a per-cell
flag "at balance" (the lime-salt pair, the standard rock). A cell with only rock, liquid, lime and salt at balance
skips Change. Dead matter arrives in a few hundred cells a month, those react. Delete the sleep-and-wake alarms
(replaced by the mask), `frozen_below` per node, and the reactor's exact p-exponent integral for the open world: a
first-order explicit step with a cap is enough for rot and lime at monthly steps (keep the exact kernel only if a
body still needs it, see 9). Gives: Change from 0.3 s a year to about 20 ms at 128 x 128.

## B. Plants

**7. Plants are a cover field per cell** (being built now, Step 1 of the review): per cell up to 4 kind slots, each a
mass per m² and 6 to 8 genes, grown by light, liquid and one mineral at the temperature pace, killed by frost,
drought, grazing and wear, spread to neighbour cells with their genes; selection between cells replaces the sample
member, Price's equation, young and adult nodes and splits. Deletes about 20,000 body nodes on the 128 x 128 island
and the plant half of `body/`, `copy/young.rs`, `copy/select.rs`. Gives: plants from about 2 s a year to about 30 ms.

**8. One limiting mineral, not two.** Keep the nitrogen-like mineral (fixing, rot, wash to the sea: fertility by
place). Drop the rock salt from what life is built of (`Chemistry::build` loses its third input; the lime-salt pair
stays as rock chemistry). **Controversial:** it undoes the "rock nutrient" work and ends the "plants stiffen with
lime" problem by removing the option. One mineral is enough for fields to differ.

## C. Animals

**9. A short body** (being built, Step 1): fuel, fat, body matter, key liquid, one mineral, heat; three reactions
(burn, build, digest); Kleiber upkeep; temperature follows the ground with a lag; `KEEP_T` burns fuel, `COOL_T`
evaporates liquid; no stomata, films, roots, melting plateaus, no reactor call. From 130,000 to under 20,000
instructions an update.

**10. One node per kind per cell.** No `crowd` merging of the most alike pairs, no split along the widest gene. A kind
is a lineage; a new kind starts when a cell's gene mean leaves the alike range of its neighbours' (speciation by
place), as the field does for plants. Delete `nodes::merge_closest`, `nodes::split`, most of `nodes.rs`,
`Config::crowd`. Gives: the node bookkeeping (7% of body time) goes; the "small nodes act whole" and "nodes against
individuals" open items go with it. **Controversial:** selection inside a cell between two alike groups is lost; it
happens between cells instead.

**11. Animal nodes stored as columns.** `Vec<Body>` with ten heap vectors per body becomes one struct of arrays per
kind: count, fuel, body, liquid, heat, genes as a fixed `[f32; G]`, brain rows by index into a lineage pool. Delete
`Body::store: Vec<f64>` (26 entries), `spread: Vec<f32>`, `ms`, `trace`, `mem` as separate heap vectors. Gives: no
allocation per child, cache-friendly passes, SIMD on the gene maths.

**12. Individuals only where brains are big and the region is refined.** Everywhere else a body is a node, even one
member (a node of count 1 is an individual in the maths already). Individuals exist only for kinds with `HIDDEN` of 8
or more inside a refined region (item 21). Delete `--individuals`, `nodes::open` and `copy::open`.
**Controversial:** no individual bodies in the coarse world, ever.

**13. Two parents only for individuals.** Nodes mix inside; the partner search per mature body (`copy::mix::partner`)
goes for nodes. Delete `--no-mix` and its control test.

**14. Moves as a field flux.** A node's move is "a share of count to each of 8 directions", already so; make it a pass
that writes to 8 per-cell outflow arrays, then one gather pass in cell order. No outbox entry per move, no `leaving`
flag, no `Passed` entry, no lane clearing per entry (the wear is the summed outflow mass through the cell, one pass).

## D. Brains (the law the vision needs)

**15. Perception is a per-cell feature vector, computed once a month for all cells.** About 24 numbers per cell
(temperature, liquid, light, plant mass per slot, animal mass per kind, dead matter, slope, parts), in one array; a
body perceives its cell's vector and its 8 neighbours' by reading rows. Delete `choose/perceive.rs`'s per-body
walks over bodies and cells and `survey::Faces` (a body's face is its kind's row of its cell's vector). Gives:
perception from a per-body cost to a per-cell cost shared by all.

**16. Brains batched by lineage.** All nodes of one lineage perceive into one matrix and run one matrix product per
month (f32, SIMD, `par` over lineages). Learning (actor-critic) stays per node on the batched outputs. Gives: the
brain cost falls to the matrix product; it also makes a GPU step trivial later.

**17. Planning: one-step look-ahead with the critic** (the spec's 6.5, "not built yet"). A body evaluates its critic
on the 9 feature vectors (its cell and 8 neighbours) and adds the value to its direction scores. Nine extra critic
evaluations per decision, nothing else. This is the first real step toward "store for the season, go where food will
be", which nothing in the speed rounds touched. Not a speed item: the one item here that moves the vision.

**18. Drop signals and the memory of known individuals until planning works.** They cost brain rows, perception
inputs and `Body::known`, `sig`, `mem`, and no report shows them doing anything (the null run can tell). Add them back
when a kind can plan. **Controversial:** it undoes M2 work; the code is in git.

## E. Threads and books (Step 2 of the review)

**19. Delete the outbox, the two phases, the landing order, the views and the booking by array.** Bodies update in
parallel over cells (`par_chunks` over cells, the nodes of a cell in order). Each thread writes its deltas to a
thread-local sparse buffer (cell, substance, amount); after the pass the buffers are summed in cell order. Cross-cell
effects (moves, litters, watches, acts) go to per-thread lists, then are applied in one serial pass sorted by (cell,
id). The result is still the same for any thread count (sorted), and it is about 1,500 lines less: `outbox.rs`,
`store/moves.rs`, `store/batch.rs`, `store/book.rs`, `store/gather.rs`, most of `life/domain.rs`, the serial phase of
`survey::batch`, `World::settle`, provisional ids. Gives: 4 cores give near x3.5 on the body pass.

**20. Books at 1e-9, no carried rounding.** Delete `Store::carry` (Neumaier), the potential node (`Class::Potential`:
lifts and falls pay heat directly), `Move::loosen`, `Store::add_far_heat`. One `check::drift` a year at 1e-9 relative.
A real leak is 1e-3 or worse; 1e-9 catches it. Keep f64 for amounts.

## F. Space: rough far away, fine where it matters

**21. The planet is the air level; cells exist only in refined regions.** Today the whole map is cells (2 km) with
air nodes over 4 x 4 blocks. Turn it around: the base level of a planet is the air-node grid (64 x 32, about 600 km
per node, with its own land fraction, height, liquid, plants and animal nodes as fields), and a region of 16 x 16
nodes is refined into cells only where brains live or where the player looks (`Square::refine` exists for the
start; make it dynamic: collapse sums, expand by the base's values plus noise, both exact in the books, spec 5.2).
A 1,000-year planet then costs like a 22 x 22 island today, with detail in a few regions. **Controversial:** the
whole-planet laws run on 600 km nodes; the spec says that is the design (7.2), it was just never built.

**22. Erosion, creep and ground heat yearly, land only, where runoff is.** Already yearly for two of them; skip cells
with no runoff and the sea (5).

## G. Chemistry

**23. Chemistry 0 cut to what the world uses now.** Of 26 substances about 14 do something today (the liquid, 4
gases, sugar, fat, lime, salt, the two minerals, body matter, frame, the rock elements). Drop the metals, ores and
fuels until M7's world search (they are data; `chem0.toml` keeps them on a branch). Every loop over `ns()` in bodies,
cells, the tape and the viewer gets 2 times shorter. **Controversial:** it undoes the richness checks' content; the
checks stay as code.

**24. Bodies and plants of two materials: body matter and frame.** Lime leaves life (8); `Body::height` by Greenhill
and `breaking` by rupture stay as formulas on two materials.

## H. Parts and Bond

**25. Parts have no reactor.** A part is a record (material, mass, shape, hollow, strength from `tensile` x its
solid share); it rots monthly by the open world's rot constant of its material at its cell's temperature (the frame
by reaction 26, body matter by 21), and falls when its walls cannot bear it (the `objects` branch's O3). Delete
`bond/month.rs`'s reactor turn, the 1e6 K bug with it, and the half-built `objects` round's other 9 steps until a kind
wants parts. Keep: form, hold, take from, put to, join, break, enter, the room (`bond::room`) computed when bonds
change, not per update.

**26. Drop `afloat` drifting, `lever`, `dig`, and `anchor` until a brain uses them.** Each is a verb with rows in
every brain and a check in every move. The null run says whether any is ever chosen for a reason. **Controversial:**
it undoes parts of M3; git keeps them.

## I. Tape and tests

**27. The tape by default: the series yearly, frames every 10 years, a snapshot every 100 years, no `months.csv`,
no PNGs.** The tape is 2% today, but a 128 x 128 snapshot is 30 MB and the frames of a 1,000-year planet add up. Make
the rest flags. Keep the per-kind columns to 5 main genes.

**28. One speed number, printed at the end of every run and written in the README: ms per year at 128 x 128 over
1,000 years.** No callgrind, no kill lines, no measure files. A change is good if that number falls or the watch-for
list grows.

**29. Replace the golden hash by liveness and range tests.** `tape/tests/golden.rs` goes; `ranges.rs` keeps the
climate rows and gains: the field and two animal kinds alive at year 300 on the 64 x 64 roster, element and energy
drift under 1e-9, no NaN. Delete `cli/tests/core_size.rs` (the line ceiling) and the tuned-number budget asserts in
`core/tests/numbers.rs` (keep the row test: every number still gets a row and an anchor). Delete the tests of every
mechanism deleted above. The normal suite under 10 s.

**30. The viewer reads the live core (wasm in a worker), not only a tape.** Not a speed item: it is the "watch first"
tool the spec wanted at M6 and that would have shown, every day since M3, what is there. With items 1 to 22 a 22 x 22
island runs at about 100 years a second in the browser, so you can watch a thousand years in the time it takes to
read this page.

## What this gives, roughly

| Layer, 128 x 128 island, settled, 4 cores | Today (s a year) | After (s a year) | Items |
|---|---|---|---|
| Air | about 1.5 | about 0.02 | 1, 2, 3 |
| Liquid, land, sea | about 0.6 | about 0.03 | 4, 5, 22 |
| Change | 0.3 | 0.02 | 6, 23 |
| Plants | about 2 | 0.03 | 7, 8 |
| Animals and brains | about 3.5 | 0.05 | 9 to 18, 19 |
| Parts, tape | 0.1 | 0.01 | 25 to 27 |
| **Year** | **about 8** | **about 0.15** | |

So 1,000 years in about 2.5 minutes at the spec's size, and a planet of 2,000 air nodes with four refined regions in
about the same time (21). These are estimates from today's per-cell costs and the budget above, not measures; item 28
is how we will know.

## The order

1. Step 1 now (7, 9; running): plants as a field, the short body.
2. Step 2 (19, 20): the parallel pass and the books; the body code gets small enough to change fast.
3. The step (1 to 6, 22): the monthly world and the explicit air. Check the climate rows after 2.
4. Animals and brains (10 to 18): columns, batched brains, planning.
5. Space (21): the planet as the air level with refined regions.
6. Cuts (8, 23 to 27) wherever they fall; tests (28, 29) from the start; the live viewer (30) as soon as a year is
   under 0.3 s.

## What we lose, honestly

- The implicit air's long-step guarantee (2): a daily explicit loop is simpler and cheaper, but it is a new solver
  and the climate must be checked again.
- Selection inside a cell (10) and two parents in nodes (13): evolution works between cells. Kinds may split less.
- The bit-for-bit thread rule stays only by sorting (19); the 1e-12 books go to 1e-9 (20).
- Signals, memory of others, afloat, lever, dig (18, 26): parked until a brain can use them.
- The second mineral, lime in life, 12 substances (8, 23, 24): the world is simpler. M7's world search brings
  chemistry back when something can use it.
- The organism model and the node mathematics: good work, in git, not in the game.

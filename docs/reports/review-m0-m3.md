# Review of Part 1 (M0 to M3): fix before M4

Oct 12, 2026 · @Romain B

## Short answer

The build is good and honest. It follows the plan closely. The physics (Flow, Change, Bond) is strong and well tested. The weak part is Choose: bodies cannot really learn, and the world gives them no reason to walk, shelter or plan. That is why trails, seasonal travel, shelter use and learned behaviour did not appear.

My main finding: **three of the four missing things are not possible in the current code, even with perfect learning.** Trails cannot form because walking changes nothing that makes later walking cheaper. Shelter cannot pay because cold costs a body no energy. Learning cannot grow because what a group learns fades back to zero every few months, and the brain only changes by rare big jumps.

So I agree with the earlier recommendation (option 2: a fixed time on Choose and speed), but with a different list of work. Fix the **physics gaps** below first. They are small changes. Then measure learning with one tiny test the plan already asks for. Only then spend time on planning and memory.

One decision sits under everything: **the scale of a body.** Today a body is a 20 to 200 tonne blob, at most 2 per 4 km², so density, plant cover, size and variety cannot emerge. Bodies must become real-sized, and speed must come from merging many real bodies into population nodes (plan M4) and from better algorithms, not from giant bodies. The owner confirmed this choice on 12 October 2026.

This review is from reading the code and the two reports only. I did not build or run anything. Where I say "likely", I could not prove it without a run.

## How well the code follows the handover plan

Most "must" rules are kept. The two misses are the missing learning test and the check that shared brains give the same patterns as one brain per body.

| Plan rule | Kept? | What I saw in the code |
| --- | --- | --- |
| Laws, not outcomes (3.1) | Yes | No Earth words in core or worldgen. The word-ban test runs in CI. Actions are generic: move, take, put, join, break, signal, copy, rest. |
| Five laws, one file each (6, 14.1) | Yes | Flow is two files (flow.rs plus flow/air.rs, about 1,170 lines). The plan allows 100 to 400 lines per law. Flow is big and dense but still one law. |
| Everything is a graph (3.3) | Partly | Grid for space, sparse graph for bodies and parts: good. But there is no edge state at all for bodies. Movement cost is computed from the two cells only. A "route" cannot exist yet because no edge number can change. |
| Chemistry 0, numbers only (6.0) | Yes | Numbers only in core. Earth names only in library/chem0.toml. Richness checks pass. |
| Step is a pure function, fixed order, keyed random numbers (8.2) | Yes | Rng keyed by seed, day, node, purpose. No hash maps in core. Same-seed test passes. |
| Compiles to WebAssembly (8.2) | Yes | CI builds the core for wasm32. |
| Conservation checks (12.2) | Yes | Element and energy drift under 1e-9 in tests. A "clamped" counter exists. Good. |
| Tests: one tiny scene per law (12.3) | **No** | Flow, Change, Bond and Copy have their scene. **The Choose scene "a body learns to move towards food" is missing.** This is the one test that would have shown the learning problem early. |
| Shared group brains: check they give the same patterns as one brain per body (6.5) | **No** | There is a test that groups form. There is no comparison run. |
| Speed target M1 to M3 (12.6) | No, allowed | 5 to 6 s per year against 0.6 s. The plan says to expect this and not to chase it before things emerge. |
| Code budgets (14.3) | Yes | Core 5,014 lines, budget 5,000. Whole repo about 7,400. |
| Watch report after each milestone (12.4) | Partly | The check-point report is honest and clear. But watch reports for M0, M1, M2 are not in the repo. Only the handover is in docs/. |
| History tape (8.3) | Yes | Snapshots, yearly series, raw events, viewer frames. Save and load round-trip test passes. |
| Viewer: Earth view and true view (10) | Yes, small | Map classes are decided by the viewer, not the core. Good. No sprites yet (that is M5). |
| Do not add realism (14.5) | Yes | Numbers are rough. A few physics numbers are tuned by hand (see next section). |

The check-point report itself is honest and follows the plan's format. One thing it should say more plainly: **eaters were placed by hand in year 22, with a brain already set to take from other bodies.** They did not evolve from light-eaters. "Eaters of eaters" is real emergence, but "eaters" is a starting condition.

## Does the app keep its goal (emergence)?

Yes in the letter, and mostly in the spirit. Nothing in the core is named after an Earth thing. But a few hand-set numbers and limits quietly shape what can emerge. They are not "named rules", but they act like walls. You should know them.

**1. At most 2 bodies per cell. Remove it; density must emerge.** `max_bodies_per_cell: 2` in core/src/lib.rs line 88. A cell is 2 km by 2 km. So the whole island can never hold more than about 2 bodies per 4 km². A herd, a crowd, a camp or a village is impossible by this one number. It is called a "speed safeguard" in the code, but it is the strongest limit on social emergence in the whole system. Also, a body that decides to move into a full cell simply loses its turn (core/src/choose.rs line 377), so the brain learns "moving fails" without knowing why.

The owner does not want any hard-coded density. Agreed: density is an outcome and must come from physics. The physics that already exists can carry it, once the numbers are real: light per m² limits how much plant can grow in a cell; plant mass per m² limits how many eaters it feeds; body mass per m² in a cell raises the cost of entering it (`resist`) and of being taken from; a body's volume must fit in the cell's free space; and crowded bodies share one air pool. Remove the cap in `go()` and `copy()`, make the brain's crowd input the real mass per m² (not count over max), and let the ecology set the number. The speed safeguard then moves to where the plan put it: population nodes merge bodies when a cell gets crowded (M4).

**2. Bodies are huge. Make them real-sized.** A light-eater starts at 200,000 kg and an eater at 20,000 kg (worldgen/src/life.rs lines 55 and 58). With 2 per cell, a "body" is really a whole forest patch or a whole herd. The M2 and M3 watch-for items (trails, shelters, tools) are about individuals, and the world has no individuals. A 20-tonne body walking 2 km once a year will not make a trail.

The owner agrees the sizes are unrealistic and wants speed to come from better algorithms and concepts, not from giant bodies. So the target is: an individual body weighs what a real animal weighs, from grams to tonnes, set by its genes and selection. A 256 km island then holds hundreds of thousands to millions of animals. Most of them must live as population nodes (plan 7.1: count, average stores, one shared brain, the same laws times the count), and only the ones where something interesting happens (near made objects, high learning, where the player looks) run as individuals. This makes M4 more important, not less. The algorithmic gains to look for: brains of one group evaluated as one matrix operation, bodies stored as arrays of columns, idle bodies updated rarely, and population nodes that split only when their members start to differ.

**3. Plant cover is almost nothing per m². It must be 100 to 1,000 times higher and must matter.** 200,000 kg over 4,000,000 m² is 0.05 kg/m². A real grassland is about 1 kg/m², a forest 10 to 50 kg/m². Because of this, plants give no resistance to walking, little shade, no fuel and no soil cover.

The owner wants cover and density to matter a lot. The cleanest way: run light-eaters as a **per-cell population node from the start**, one per lineage per cell, holding real kg/m². Plants do not walk and barely decide, so this costs almost nothing, and it is the first use of the M4 speed tool on the easy case. The physics allows real numbers: a cell gets about 200 W/m² of light; at 1% stored, that is 1.6 × 10⁷ kg of plant per year per cell, so a forest of 2 × 10⁸ kg grows in about 12 years. Cover must then feed back into the laws it already touches: it shades the ground (`canopy` and reflectance, already there), resists walking (`RHO_PASS` set so 1 to 5 kg/m² of brush halves the speed), is worn by walking (the trail mechanism), holds loose soil against rain and creep, and is fuel for Change when hot. Each of these is one line in an existing law, not a new rule.

**4. The first brains are hand-set.** worldgen/src/life.rs gives each starting kind its action biases by hand: light-eaters rest (bias +2) and never move (bias -4); eaters take from bodies (bias +2) and rarely move (bias -1). This is allowed as a starting condition. But it means the eater's main behaviour, "stand still and eat the plant next to you", was written, not learned. The fragile ecology follows from it.

**5. The core classifies bodies.** `light_eater()` in core/src/body.rs line 284 is used inside Choose to fill statistics (`diag.intake`, `took`). It never changes a decision, so it is harmless. But it is analysis code living in the core. Better to move the classification to tape/.

**6. Many tuned constants.** C\_BRAIN, C\_MOVE, TAKE, RHO\_PASS, WEAR, FORGET, BLEND, LEAVE, JOIN, STAY, NEW\_LINEAGE and about 20 more are fixed numbers in the law files. None is an Earth rule. But together they fix the ecology. The plan says such numbers are "free physics numbers" to be tuned by the world search in M7. Keep a list of them in one place (a Config section), so M7 can search them.

None of these breaks the plan. Items 1 to 3 are one decision, now taken: real-sized bodies, real plant cover, no density cap, and population nodes as the speed tool. It must be done before M4, because M4 builds population nodes on top of the body scale.

## The four problems you named: causes and fixes

### Speed: 5 to 6 s per year

About 9,000 bodies update every 5 to 10 days. That is about 300,000 body updates per year. At 4.5 s per year, one update costs about 14 microseconds. The brain is small (12 hidden units), so the brain is likely not the cost. The cost is likely the physics done per body in `live()` (core/src/body.rs):

- `rc.react()` runs the full reaction solver (21 reactions, up to 48 sub-steps) for every body at every update.
- `b.mass()` is recomputed 5 to 8 times per update (it loops over all 22 substances each time).
- `wanted()`, `put()`, `give()` and `visible()` each allocate a new Vec per call.
- `bond::room()` and `walls_around()` are called several times per update and scan the cell again each time.
- `survey()` runs every 3 days over all bodies and all 16,384 cells, and `regroup()` runs monthly over all groups and bodies with a 5 by 5 scan each.

What to do, in order:

1. **Profile first.** One run of 20 years under `perf` or `cargo flamegraph` tells you where the 14 microseconds go. Do not guess. Put the result in the next report.
2. **Cheap wins** (likely 2 to 3 times): cache mass and heat capacity on the body for one update; make `react()` skip reactions the body does not catalyse and that are frozen at body temperature before the loop; reuse buffers instead of allocating; cache `room()` once per update.
3. **Then M4 as planned, but earlier for plants.** With real-sized bodies there will be far more of them, so population nodes are the only way to the speed target. Plants as per-cell population nodes (see hidden limits) are the first step and remove most of today's body updates at once, because 4,800 of the 9,000 bodies are plants.

### Fragile eaters: they die out in 1 run of 4

I could not run seeds 1 and 2, so this is a reading of the code, not a proof. The likely mechanism is:

- Eaters start with a brain set by hand to "take from a body" (bias +2) and to "move" rarely (bias -1), worldgen/src/life.rs line 58.
- A take can remove up to 5% of the eater's own mass per day of its interval, so up to 50% of its mass every 10 days (`TAKE` in core/src/choose.rs). The plant dies when it drops below half its mass.
- With 2 bodies per cell, an eater and its one plant sit alone. The eater eats the plant to death, then has nothing. It only moves by chance. If the cell next door is full, the move fails and the turn is lost.
- The eater's brain cannot fix this quickly, because the brain changes only when a new lineage splits off (2% of births, all weights shaken at once) and learning fades (see the learning section).

So the outcome depends on the first few hundred eaters finding plants by luck. That is why tiny changes flip it.

Fixes:

1. Check the cause with data you already record: the yearly `died_lack`, `died_spent`, `ate_light_eaters` and `ate_others` columns in series.csv for seeds 1 and 2 against seed 5. If eaters die of `lack` while plants are plenty two cells away, the mechanism above is confirmed.
2. Let the brain's action rows be **genes**, inherited by every child with small noise, not only by lineage jumps (see learning section). Then "move when hungry" can be selected in a few generations.
3. Replace the hard cell limit with a physical crowding cost (see hidden limits). Then a move never fails silently.
4. Start eaters with neutral biases (all near 0) and let selection set them. If that makes eaters never appear, say so in the report. That is also a result.

### Weak rivers after the speed work

Liquid now moves once every 15 days (`flow_days: 15`, core/src/lib.rs line 76) instead of every 3. Each call is one sweep from high head to low. Two things in the sweep get worse with fewer calls:

- The sweep order (`s.order`) is sorted only once every 3 months (`REFRESH_MONTHS`, core/src/flow.rs line 24). Water handed to a cell that was already visited in this sweep waits for the next call. With 5 times fewer calls, water waits 5 times longer in lakes, flats and any place whose head changed since the sort.
- Standing water now has 15 days to evaporate before it can move on.

The river picture also depends on `liquid_flux`, which is a moving average of `out / dt`. With a bigger dt the average is smoother and the peaks are lower, so the viewer threshold may simply hide rivers that still exist.

Fix: run seed 5 for 50 years with `flow_days` 3 and 15 and compare three numbers: `liquid_land` from series.csv, the largest `flux`, and the count of cells over the river threshold. If 15 is the cause, sort `s.order` every full step (it is one sort of 16,384 numbers, about 1 ms) or let liquid take 5 sub-sweeps inside one 15-day air step. Keep the air at 15 days; it is the expensive part.

### Ground heat below zero

I found 19 places that subtract energy from `e_ground`. None of them checks that the node has that much energy. The plan's stability rule ("scale down outgoing flows if they would take more than the node has") is applied to amounts but not to heat.

The most likely culprits, from reading:

- Boiling in core/src/flow/air.rs line 250: when a ground node is at or above a substance's boiling point, the whole amount evaporates in one call, and its full latent heat is taken from the ground with no limit.
- Gas released by reactions in core/src/change.rs line 511: for each mole of gas sent to the air, the ground pays the gas's fusion plus vaporisation heat, again with no limit.
- On a melting plateau, radiation uses only the cover's resistance (`r = rc`). If the cover is zero, the surface loses heat with no limit from the node's heat capacity.

Fix in two steps. First, find it: in debug builds, call `check::broken` after each `mark()` in `step_marked` and panic with the law name. Today the CLI checks only once a year (cli/src/main.rs line 79), so the error is gone by then. Second, add one small helper, `take_heat(i, q)`, that caps `q` at the node's energy above a floor, counts the difference in `ledger.clamped`, and use it in all 19 places. That is the same rule the amounts already follow.

## Why trails, seasonal travel, shelter and learning did not appear

The check-point report guesses "perception, ecology, learning". The code says something sharper: for three of the four, the physics that would make them possible is missing or switched off. Better learning alone would not help.

### Trails: walking changes nothing

The plan's key sub-rule is "moving matter wears the matter it moves through, so repeated movement makes the ground cheaper to cross" (6.1). In the code:

- The cost of a move is `mass × (distance + climb) × resist`. `resist = 1 + body mass per m² in the target cell / 5` (core/src/choose.rs line 384). Two 200-tonne plants in a 4 km² cell give 0.1 kg/m², so `resist` is about 1.02. Cover adds 2% to the cost.
- The energy that wears things is `cost × (1 - 1/resist)`, so about 2% of the cost. It breaks bonds of the **bodies** in the cell (plants), and nothing else (line 398 and `wear()` at line 414).
- No number on a cell or an edge records that a body passed. Nothing makes the next crossing cheaper.

So a trail is not possible in this code. Walking leaves busy patches (where the food is) and that is all the map can show.

Fix (a law, not a rule): give each cell 8 edge numbers, "how easy bodies pass", starting from the cell's cover and loose solid depth. Movement dissipation on an edge raises it. Creep, rain and plant growth lower it slowly over years (Flow and Copy already touch these). The move cost reads it. This is exactly the "edge whose conductance became high because it was used a lot" of plan 5.1. It also needs bodies that move more than once a year, see the next two points.

### Seasonal travel: no reason to move

Bodies see the season (sin and cos of the year), the temperature, the food smell in each neighbour cell and the height. That is enough perception. What is missing is a **reason**:

- Cold costs nothing. `pay()` (core/src/body.rs line 687) burns fuel on demand at any temperature. A body at 260 K pays its upkeep as easily as one at 300 K. Only freezing of a key liquid kills.
- Well-being counts temperature only in the last 5 K before freezing (line 255).
- Food is nearly everywhere (the report says so), and the hand-set eater brain almost never moves.

Fix: make metabolism depend on temperature, as the Change law already does for every other node. In `pay()`, cap the energy a body can release in `days` by its catalysed reaction rate at its own temperature (the same Arrhenius form `k · exp(-ea/T)` the solver uses). A cold body then cannot pay its upkeep and dies of `SPENT`. Winter now costs, the north empties in winter, and moving south or going inside pays. This is one physical change, and it is what makes the next item possible too.

### Shelter in cold: walls work, but warmth is worth nothing

The test `walls_keep_a_body_warm` passes and the report measured about 3 K. But with cold costing nothing (above), there is no gain from going inside, so no learning signal exists. Also `enter` needs a closed structure with space for the body's volume in the body's own cell. With 2 bodies per cell and 6 parts per cell at most (`MAX_PARTS`), a shelter is a rare object. Fix: the metabolism change above, then watch `inside_cold` against `inside_warm` again. No new rule is needed.

### Learning: three brakes in the code

The report says learning stays tiny. The code shows why:

1. **Forgetting.** Every month, each group's learned rows move 2% back toward the lineage's starting rows (`FORGET`, core/src/choose.rs line 330). Half of anything learned is gone in about 3 years. Learning cannot build up.
2. **The brain does not inherit.** Genes pass from parent to child with small noise, as the plan says. But the brain's starting rows do **not**. They belong to the lineage and change only when a child starts a new lineage (2% of births, core/src/copy.rs line 85), and then all weights are shaken at once by 0.1. That is why 80,000 lineages were born and 775 live: most jumps are bad. Plan 6.4 says genes should hold "the starting state of the brain". Make the rows (and ideally the hidden layer) genes, inherited by every child. The shared group brain can then be the average of its members' gene-rows, so the speed tool stays.
3. **Learning starts switched off.** The `LEARN` gene starts at -4.0 for light-eaters (off) and -2.0 for eaters (rate 0.01), worldgen/src/life.rs lines 55 and 58. Its range is -4 to 0 and it moves about 0.04 per generation. Each update also caps the step at 0.1 (`MAX_STEP`) and divides by the group's error spread. Together, learning is slow by design and then erased by forgetting.

Smaller points: imitation (`watch`) moves a group's rows by `0.3 × alike / members`, so in a group of 50 bodies one watch changes 0.6%. Signals carry noise of 0.3 on a value of about 1, which is fine, but with no learning there is nothing to settle.

Fixes, in order:

1. Add the plan's test (12.3): a tiny world, one body, food in one direction. After 200 decisions it must move toward food more often than chance. Run it with learning on and off. If it fails with learning on, fix the learner before anything else.
2. Set `FORGET` to 0, or make it a gene with range 0 to 0.02. Let evolution decide.
3. Make brain rows genes (item 2 above).
4. Record learning in the tape: per year, the mean absolute change of group rows from their start, and how often the chosen action was the most likely one. Then "learning is tiny" becomes a number you can watch.
5. Only after 1 to 4: planning and longer memory, as the report proposes. They cannot help while 1 to 3 are broken.

## Variety of beings, a food chain, and smarter brains

The owner noticed that there is no variety: two kinds, both designed, and they stay two kinds. The code explains it. Size has no physics, the world is the same everywhere, and prey have the same chemistry as plants. So there is one best way to live, and one kind wins.

### Why there is no variety now

- The genes could carry variety: body size (`COPY_MASS`), diet (one catalyst gene per reaction, one uptake gene per substance), strength, decision interval, learning rate. The gene space is wide enough.
- But **size has no physics.** Heat loss is `HEAT gene × mass`, so a mouse and an elephant cool at the same rate per kg. The contest when one body takes from another uses strength only, not mass (`take_body`, core/src/choose.rs: `sb / (sb + so)`). A 20 kg body eats a 200,000 kg body as easily as the reverse. Take, dig and move all scale linearly with mass. No size is better or worse than another, so nothing selects size.
- **The world is the same everywhere.** Food everywhere, cold free, no crowding. One strategy fits all places.
- **Prey are made of plant.** Plants and eaters both store sugar (substance 5). Eating a plant and eating an eater is the same reaction. There is no second rung in the chemistry, so no second rung in the food chain.

### Strategy for a range of beings

1. **Give size its physics.** Heat loss through a surface, so proportional to mass to the power 2/3, not to mass. Then small bodies cool fast and need more food per kg, big bodies stay warm but need a long time to reach `COPY_MASS`. Force in a contest is `strength × mass` (core/src/bond.rs `force()` already does this for parts; use it in `take_body` too), with the quicker body (shorter interval) acting first. These two changes alone give size variety by climate and by diet.
2. **Make the world varied.** The fixes above do this: cold costs, seasons bite, cover is real, rivers and mountains divide the island. Then there are niches: wet coast, cold hills, dry basin, deep forest.
3. **Use the second chemistry rung that already exists.** Chemistry 0 has sugar to fat (reaction 3) and fat burning (reaction 2). Fat holds about 3 times the energy per kg of sugar (47 against 16 MJ/kg). A body that makes and stores fat carries its food cheaper. An eater of such bodies then needs the fat-burning catalyst, a different diet gene. Nothing to add. Check only that both catalyst genes can turn on from their start at -1.0 and that uptake genes let a body prefer fat.
4. **Seed an ancestor, not two designed kinds.** Place one simple light-eater and one simple animal with a neutral brain, and give every seeded body 20 rounds of `mutate()` so the first generation is already a spray of variants. Selection then has material. Two-parent copying (plan 6.4, an option) mixes genes much faster; it is a medium change in core/src/copy.rs.
5. **Let prey escape and hide.** Today a take is instant when both bodies share a cell, and no cover hides anything. With the contest above, the quicker body acts first, and with real cover, bodies in deep cover are perceived less (the same way walls already hide bodies in `act()`). Then fleeing and hiding pay, and so do speed, senses and stealth.
6. **Measure it.** Each year in the tape: cluster the genes of live bodies into kinds (a cheap k-means on five genes), and report the number of kinds, the mass range (smallest, median, largest) and each kind's trophic position (share of what it took that came from animals, from `took`). Then "no variety" is a number that can move. The M5 pattern finder does the full job later.

### How a predator and prey chain emerges

A chain needs at least three rungs that differ in chemistry and size: plants holding sugar as cover; small, fast-breeding eaters that store fat; bigger eaters that burn fat and catch by force. It also needs prey that can run or hide, or the chain collapses instead of cycling. That is what the check-point report saw: rises and crashes, no cycles, because nothing escapes. Items 1 to 5 above give all four conditions without one new rule. Watch the `eat_animals` and `ate_others` columns, the mass distribution, and whether populations now cycle.

### Smarter brains for a people-like kind?

The owner asked whether to force a more intelligent brain for a people-like species, so that it surely emerges. The plan's rule 3.5 answers: physics makes things possible, it never makes them happen. A hard-coded "people" kind with special actions is a named outcome and the plan bans it. But there are two allowed moves, and both are worth doing.

1. **Make brain size a gene with a cost.** Today every body has the same brain: 12 hidden units (`H` in core/src/brain.rs), 6 memory slots, 3 bodies perceived, sight of one cell. Make hidden units (4 to 64), memory slots, bodies perceived and sight range genes, each with an energy cost per decision that grows with size (`C_BRAIN × H / 12` and so on). Intelligence is then selected where it pays, in a varied world with hard food and social groups, and lost where it does not. This needs a `Shape` per lineage and group rows sized per lineage: a medium change in brain.rs and choose.rs.
2. **Seed one clever lineage as a starting condition.** This is allowed, exactly like seeding eaters was: one lineage with a big brain, long memory and learning on, placed in a given year and reported as placed. It is an experiment: does a big brain pay for its cost in this world? If it dies out every time, that is the signal that the world does not reward intelligence yet, and the fix is in the world (the sections above), not in the brain.

What not to do: write a people-like kind with its own actions or rewards. In M7, "a lineage with a large brain lasted 1,000 years" can be one ingredient of the interest score. That chooses worlds; it does not write rules.

## Smaller code issues

None of these blocks M4. They cost time for the next agent, or hide bugs.

| Where | Issue | Suggestion |
| --- | --- | --- |
| core/src/flow.rs, `Scratch` | About 60 fields, many with one or two letter names (`al`, `be`, `x`, `cw`, `p`, `th`, `lh`, `xs`, `fl`) and shared between unrelated steps. The next agent will struggle. | Add a short table in the doc comment: field, unit, who writes it, who reads it. Rename the worst ones. |
| core/src/flow.rs | Flow is 1,170 lines in two files. The plan allows 100 to 400 per law. It is the law that works best, so do not rewrite it. | Note it in the report as over budget with a reason. |
| core/src/change.rs | About 250 lines decide which nodes may react this step (`LAZY`, `SLOW`, `Cand`, `fits`). Clever, but hard to verify, and Change is not the bottleneck. A wrong `fits` test would silently skip a hot node. | Add one test: a ground node heated to 600 K with sugar and air must react in the very next step. |
| core/src/body.rs line 509, core/src/change.rs line 352 | `Reactor::new` allocates its buffers on every call (daily for bodies). | Keep one Reactor in `World` or in a thread-local. Small win. |
| core/src/lib.rs `World.scratch` | Derived caches are saved in snapshots. A snapshot from an older code version keeps stale caches. | Skip `scratch` in serde and rebuild on load (the code already rebuilds when empty). Check the round-trip test still passes. |
| core/src/body.rs `thaw`, `t_ground` | `heat_cap.max(1.0)` hides nodes with almost no heat capacity. | Fine for now; the `take_heat` helper above makes it safe. |
| cli/src/main.rs line 79 | `check::broken` runs once a year. Bugs between steps are invisible. | Run it after every law part in debug builds (see the heat section). |
| tape/src/rec.rs `describe()` | The `Born` and `Died` arms are dead code; those events are counted earlier. | Remove or comment. |
| core (constants) | About 30 tuned constants spread over 5 files (`C_BRAIN`, `TAKE`, `RHO_PASS`, `FORGET`, `NEW_LINEAGE`, ...). M7's world search needs to vary them. | Move them into `Config` with a comment each. One place to read, one place to search. |
| docs/ | Only the handover is in the repo. The M0 to M3 watch reports live only as artifacts. | Save each report as docs/reports/mN.md with its pictures. The plan says the repo is the record. |
| worldgen/src/life.rs | Starting kinds carry hand-set brain biases (see hidden limits). | Start neutral, or state it in every report. |
| web/ | Good and small. No WebAssembly bindings yet, which plan 8.4 allows until M6. | Nothing now. |

Things that are good and should stay as they are: the keyed random numbers, the conservation ledger, the two-layer radiation with latent heat, the implicit wind solver, the stable body and part IDs, the word-ban test, and the honest tone of the reports.

## The list: what to fix before M4, in order

Size: S is under a day of agent work, M is a few days, L is about a week. Items 1 to 4 are bug fixes and tests. Items 5 to 13 close the physics and scale gaps; 7 to 9 are the body-scale decision the owner has taken. Items 14 and 15 are the problems named in the check-point report. Item 16 is the check that decides M4.

| # | Fix | Size | Why |
| --- | --- | --- | --- |
| 1 | Debug check after every law part in `step_marked`; find the negative ground heat; add `take_heat` so no node gives more heat than it has. | S | A known bug. The helper also protects M4's merged nodes. |
| 2 | Write the missing Choose test: one body learns to move toward food, learning on and off. | S | Tells you in minutes whether the learner works at all. |
| 3 | Record learning and variety in the tape: yearly change of group rows from their start; number of kinds, mass range, trophic position. | S | Turns "learning is tiny" and "no variety" into numbers. |
| 4 | Set `FORGET` to 0 or make it a gene. | S | Learning cannot build up while it is erased. |
| 5 | Make brain rows genes, inherited by every child with noise. Keep the shared group brain as the mean of members. | M | Behaviour can then evolve smoothly. Fixes fragile eaters too. |
| 6 | Metabolism rate depends on body temperature in `pay()`. | S | Cold costs energy. Shelter and seasonal travel can pay. |
| 7 | Remove `max_bodies_per_cell` entirely. The brain's crowd input becomes real body mass per m². | S | Density must emerge. No hard-coded limit. |
| 8 | Real-sized bodies: starting masses from grams to tonnes by genes; light-eaters as per-cell population nodes holding real kg/m² (the first use of the M4 tool). | L | The scale decision. Everything social and ecological depends on it. |
| 9 | Cover feeds back into the laws: shade, walking resistance, wear by walking, soil held against rain and creep, fuel when hot. | S | Cover must matter. One line in each existing law. |
| 10 | Size physics: heat loss proportional to mass to the power 2/3; contest force is strength times mass, the quicker body acts first. | S | Without it no size is better than another, so no variety. |
| 11 | Edge passability for bodies: 8 numbers per cell, raised by movement dissipation, lowered by physics over years; move cost reads it. | M | Trails are impossible without it. It is plan 5.1 and 6.1 word for word. |
| 12 | Seed one simple light-eater and one simple animal with neutral brains and a spray of gene variants. Say in every report what was placed and when. | S | Selection needs material. Honesty about starting conditions. |
| 13 | Brain size, memory slots, bodies perceived and sight range as genes with an energy cost. Optionally seed one clever lineage as a reported experiment. | M | Intelligence becomes possible and paid for, not forced. |
| 14 | Rivers: A/B run with `flow_days` 3 and 15; sort the sweep order every full step or sub-sweep liquid. | S | Cheap test, cheap fix. |
| 15 | Profile the body update; take the cheap wins (cache mass, skip idle reactions, reuse buffers). | S | Measure before M4 so you know what population nodes must buy. |
| 16 | Re-run the M1 to M3 watch on 3 seeds for 300 years: variety of kinds, a food chain, trails, seasonal travel, shelter in cold, learning spread. Write the report into docs/reports/. | M | This is the check point again, with the gaps closed. |

After item 16, decide M4 with real evidence. If kinds, a food chain, trails and shelter now appear, go to M4 with the body scale from item 8. If they still do not, the problem is in learning and planning, and option 2 of the check-point report (planning, longer memory) is the right next step.

What not to do before M4: do not add planning or longer memory yet (items 4 to 13 come first), do not add a sixth law, do not write a people-like kind, and do not spend time on the speed target beyond item 15.

## What this review did not check

- I did not build or run the code, as you asked. All numbers about speed and populations come from your check-point report. Where I estimated (14 microseconds per body update, 0.05 kg/m² of plant cover, 2% walking resistance), the arithmetic is in the text and you can check it.
- The causes I give for fragile eaters and weak rivers are readings of the code. Each has a cheap test next to it. Run the test before changing anything.
- I did not read the 300-year viewer artifact frame by frame. I trusted the report's pictures and tables.
- I did not check the chemistry numbers against reality, because the plan says rough numbers are enough.
- I did not review web/src/charts.ts and web/style.css in detail. The viewer is small and not on the critical path.

Files read in full: everything under core/, worldgen/, tape/, cli/, the viewer's main, data, map and earth modules, all tests, the CI file, and docs/handover.md.

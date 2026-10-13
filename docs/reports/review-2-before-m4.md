# Review 2: Part 1 after the fixes, before M4

12 October 2026. From a reading of the code only. Nothing was built or run. Where I say "likely", I could not prove it without a run. Where I give a number, the arithmetic is in the text, so you can check it.

## Short answer

The work since the first review is good. All 16 items on the list were worked on: 13 are done, the rivers and the profiling are half done, and the new watch runs are done but not written up. The fixes were built the way the plan asked: as physics, not as rules. The code is honest about what it places by hand. The handover was updated when decisions changed. This is the right way to work.

But the world still does not do what we hoped, and the code says why. The problems you named have three roots:

1. **Eating has no brake.** An eater can take up to half of a cell's plants in one decision, whatever the cover is. It gets about 9 times more energy than it needs, and a child costs almost nothing. So eaters boom, graze the plants down, and the plants cannot come back while any eater stays. Nothing in the physics makes the last plants hard to find. (Problem 1.)
2. **Evolution is switched off inside population nodes.** Children who stay in a node get their parent's genes with no change at all. Children who leave join the first node of their lineage in the next cell and are averaged into it. So a kind cannot adapt to cold, cannot split into kinds, and cannot grow a fat gene. (Problems 2, 4, 5, 8, 9.) The mutation and "spray" sizes are also far too small to give selection anything to pick from: the 20 rounds of spray move a brain bias by about 0.05, when a useful bias is about 2.
3. **Nodes cannot act with objects, and only bodies over 100 kg can make one.** Wherever eaters are dense enough to build anything together, they are merged into a node and lose the hold, form, join and enter actions. A body must put 10 kg into a part, so a 30 kg eater can never make a tool. There is also a hard cap of 6 made objects per cell. These are hidden rules of the same kind as the old body cap. (Problems 7, 9, and all of M3.)

My advice: do not start M4's speed work (coarse regions, planet) yet. First finish the **population node** design, because it is already carrying every result: give nodes a spread of genes, let them split, let them act. Then three small physics fixes for the ecology (an intake limit by area, a real cost for making a body, a convex cost for catalysts). Then run the watch again. The speed target is an M4 result, not a pre-M4 task; measure it, take the cheap wins, and stop there.

## What was done since the first review

The list had 16 items. Sizes are as in the first review.

| # | Fix | Done? | Where |
| --- | --- | --- | --- |
| 1 | Debug check after every law part; no ground heat below a floor | Yes | `lib.rs` `audit`, `take_heat`, `T_FLOOR`; test `take_heat_never_takes_a_node_below_the_floor` |
| 2 | Choose test: one body learns to move toward food | Yes | `core/tests/choose.rs`, learning on against off, 120 seeds |
| 3 | Learning and variety in the tape | Yes | `tape/src/measure.rs`: `learned`, `greedy`, `kinds`, mass range, trophic share |
| 4 | Forgetting a gene | Yes | `FORGET` gene, seeded off |
| 5 | Brain rows are genes | Yes | `Life::genomes`, `Group::innate` and `learned` |
| 6 | Cold costs: metabolism at Change's rate | Yes | `body::plan`; test `cold_bodies_release_too_little` |
| 7 | No body cap per cell | Yes | Only a volume check (`phys.layer`), which never binds |
| 8 | Real-sized bodies; plants as per-cell nodes | Yes | 1 kg plant members at 0.1 kg/m²; 30 kg eaters; `Body::count` |
| 9 | Cover in the laws: shade, resistance, wear, soil, fuel | Yes | `canopy`, `resist`, `wear`, `flow::held`, test `hot_cover_burns` |
| 10 | Size physics: surface heat loss, force by mass, quicker first | Yes | `Body::conductance`, `take_body` |
| 11 | Edges remember use | Yes | `World::pass`, `resist`, `eased`, `regrow`; test `pass.rs` |
| 12 | Neutral ancestor with a spray of variants, reported | Yes, but the spray is too small (see problem 2) | `life::Seeding` |
| 13 | Brain and sense sizes as genes with a cost; a clever lineage as a reported experiment | Yes | `HIDDEN..SIGHT`, `Lineage::cost`, `--clever-year` |
| 14 | Rivers: A/B of the flow steps | Yes, half taken | `Config` doc comment has the numbers; 3/9/15 chosen |
| 15 | Profile the body update; cheap wins | Partly | `mass_cap`, buffers reused; no profile numbers in the repo |
| 16 | Re-run the watch on 3 seeds, 300 years; report in `docs/reports/` | Runs done, report not yet in the repo | You have 8 runs; write them up |

Also done, beyond the list: 57 tuned numbers moved into `Config::phys`; body classification moved to the tape; `Scratch` documented; a Change test for a hot node; `--own-brains` and `--crowd` flags for the comparisons; population nodes with a crowd merge, ahead of M4, and the handover updated to say so.

## How well the code follows the handover

| Plan rule | Kept? | Note |
| --- | --- | --- |
| Laws, not outcomes (3.1); word ban (14.4) | Yes | No Earth words; the ban test runs in CI. But see the hidden rules below. |
| Five laws, one file each | Yes | Flow is still two files, about 1,280 lines. Known and accepted. |
| Everything is a graph (3.3) | Better | Edges now carry a number (`pass`). Good. |
| Pure step, keyed random numbers, fixed order (8.2) | Yes | |
| Compiles to WebAssembly | Yes | CI builds it. |
| Speed tools must give the same patterns (7) | **Not checked** | Plants are nodes from day 0, so for plants it can no longer be checked by comparison. For eaters it can: `--crowd 100000` against `--crowd 4` on a 64x64 world. Same for `--own-brains`. Neither run is in the repo. |
| Code budgets (14.3) | 19% over | `core` is 5,943 lines against 5,000 for M3. The plan allows 20% with a reason; the reason is the body scale and the nodes. The M7 budget is 8,000. Whole repo about 9,200 lines plus 780 of tests. |
| Watch reports in `docs/reports/` (12.4) | Still missing | M0, M1, M2, the M3 check point and your 8 new runs are not in the repo. The plan says the repo is the record. |
| Honesty about what is placed (12.4) | Yes | `seed_with` prints what it placed and how; the clever lineage says it uses hand-set biases. |

## Does the code keep the goal (emergence)?

In the letter, yes. No system is named after an Earth thing. But five things in the code act like rules and shape what can appear. None is a word on the ban list; all of them close doors.

| Where | What it does | Why it matters |
| --- | --- | --- |
| `bond.rs` `MAX_PARTS = 6` | At most 6 made objects per cell (4 km²). Forming fails past that. | A camp with 10 objects is impossible. This is the body cap again, for things. |
| `bond::form`: a part needs `10 × min_kg` = 10 kg, and a body puts at most `phys.form` = 10% of its mass into it | A body under 100 kg can never make a part. | The 30 kg eaters, and anything people-sized, can make nothing. Tool use is shut for them. |
| `choose::act`: a node (count > 1) cannot form, hold, let go, anchor, dig, or do any verb on a part | Wherever 5 or more eaters share a cell, they are merged and lose every Bond action. | Objects need a crowd to matter (someone to copy, someone to give to). The crowd is exactly where the actions are switched off. |
| `copy::copy` and `body::crowd`: a child joins the first node of **its lineage** in the cell, whatever its genes | Different variants of one lineage are averaged into one node per cell. | Kinds cannot separate. See problem 9. |
| `body::upkeep`: the brain's cost per decision is **per kg of body** | A 1,000 kg body pays 30 times more than a 30 kg body for the same 12-unit brain. | Big bodies are punished for thinking; tiny bodies think for free. In life a brain's cost goes with the brain, not the body. |

The first three are the ones to remove before M4. They are not laws; they are limits of the representation that became rules for most bodies once population nodes arrived.

## The problems you named: cause and fix

### 1. Eaters destroy the plant cover and keep it down for decades

This is three numbers and one missing piece of physics. The arithmetic, for a starting eater of 30 kg deciding every 10 days:

| What | Number | From |
| --- | --- | --- |
| Most it can take per decision | 15 kg | `phys.take` 0.05 × 10 days × 30 kg |
| Of that, sugar (it wants 1.0 sugar to 0.3 liquid) | 11.5 kg = 1.8 × 10⁸ J | 16 MJ/kg |
| What it needs per decision (strength, catalysts, brain) | 2.1 × 10⁷ J (about 24 W) | `c_strength` 10⁵ × 0.6 × 30 kg × 10 days + brain 10⁵ J/kg |
| Surplus per full meal | about 9 times its need | |
| Cost to make a child | 5 × 10⁵ J/kg | `phys.c_copy`; a 19 kg child costs 10⁷ J, one tenth of one meal |
| Plants killed per 11.5 kg of sugar eaten | 38 kg (whole members die; their water goes to the ground) | `take_body` for a node |
| Plant growth at low cover | a few times a year at most | gas exchange limits sugar to 3.8 g per kg per day |

So an eater that eats on most decisions grows from 30 to 63 kg in about a month and copies. The eater population can double every one or two months. Plants cannot. That is the boom.

The crash that does not end: `take_body` takes up to **half of the plant node's store** per decision (`wanted`, `.min(0.5 * from)`), at any cover. There is no search cost, no bite limit: the last 100 t of plants in a 4 km² cell are as easy to eat as the first. So a few eaters hold a cell's plants at the level where growth equals what they eat, which is far below the map's 0.03 kg/m² threshold, for as long as they live there. Cold metabolism and size physics do not touch any of this, which is what you saw.

What stabilises real grazing is that an animal can only crop a limited **area** per day. A cow crops about 50 to 100 m² a day. At low cover that area holds too little, so the animal starves or moves. This is physics (a body does work over a limited area in a limited time), not a rule, and it is one line in `take_body` and `take_node`:

```
kg taken ≤ (prey mass per m² in the cell) × (area worked per day per kg of eater) × interval × eater mass
```

with a new free number `phys.harvest` in m² per kg per day (try 1; a 30 kg eater then works 300 m² in 10 days). At cover 0.03 kg/m² that gives 9 kg of plant, about 2.7 kg of sugar, close to the eater's need. Below that it starves or must move. This gives the cover a floor near the visible threshold, and it is also the first real reason to move (problems 5 and 6).

Second: make a body cost energy to build. `c_copy` = 5 × 10⁵ J/kg is about 20 times cheaper than the food energy in a kilogram of animal (growth in life is about 10 to 20% efficient, so 10⁷ J per kg is the biological number). Try 2 × 10⁶ first and 10⁷ later; check that the eater's gas gene lets it release that in one interval (`plan` caps release by the air it exchanges). Both plants and eaters pay this, so both slow down, but the eater boom slows most.

Third, the crude lever if the two above are not enough: `phys.take` from 0.05 to 0.02 per day. Say in the report if you touch it.

Not a cause: the pile of dead plant on the ground. For the starting eater, eating puts no sugar on the ground (it takes all the sugar of the members it kills; only their water is dumped). Ground sugar comes from other deaths and oxidises at about 7% a year, so it does feed eaters for a decade, but it is small next to the live plants.

### 2. Neutral eaters always die out

A neutral eater draws one of about 22 equally likely options each decision: 5 fixed actions, 8 moves, and 3 verbs on each of up to 3 bodies it sees. "Take from a body" is about 3 of 22, so it eats on about 1 decision in 7. Each meal gives 11.5 kg of sugar; it needs 1.35 kg per decision; so 1 meal in 7 is about 1.2 times its need. Right at the edge, before any bad luck. The hand-set eater eats on about 80% of decisions. Between those two there is a gap of about +2 in the bias of one action.

Selection cannot cross that gap, because nothing varies:

| What should vary | How much it varies | Where |
| --- | --- | --- |
| A brain bias, per child | 0.003 (10% of children get a change of 0.01) | `row_chance` 0.1, `row_noise` 0.05 × mutation size 10^-1.2 |
| A brain bias, after the 20 rounds of spray at seeding | about 0.045 | `Seeding::rounds` × the same step |
| A catalyst gene, per child | 0.063 decades | step 1.0 × 10^-1.2 |
| A catalyst gene, after the spray | about 0.28 decades | |
| Learning within a life, at the seeded rate | at most 0.005 per decision per row | `LEARN` -2.0 gives rate 0.01; `learn_most` 0.5 × rate |

So all neutral eaters are the same eater to within 0.05, and the best of them cannot move a bias by 2 within a life (that would take 400 decisions, about 11 years) or within the few generations before the population is gone. The Choose test shows learning works, but it uses `LEARN` -1.0, ten times the seeded rate.

Fixes, in order:

1. **Spray that gives material.** Give `Seeding` its own spray size, separate from the per-generation step, and make the biases of the first population span about ±1 (for example 20 rounds at 0.2 on the rows). This costs nothing: one genome per placed body. Say in the report what the spread was.
2. **Seed `LEARN` at -1.0**, the rate the test passed with, and let it evolve. Its cost (`c_learn` × 0.1) is 5% of upkeep.
3. **Per-generation row mutation 3 to 6 times today's**, for example `row_chance` 0.3 and `row_noise` 0.3. Note the memory cost: a genome is about 7 KB, and today it is shared because changes are rare. With more changes, more bodies own a genome. Measure it; if it is too much, mutate the bias terms (130 numbers) often and the weights rarely.
4. **Node evolution** (problem 4). Without it, none of the above matters once eaters are in nodes, which they are wherever they are dense.

If after this neutral eaters still die out, that is a result worth reporting: this world does not reward eating enough for behaviour to find it from zero. Then seed `take from a body` at +1, say so in every report, and move on. Behaviour from zero is the hard research question of M7, not a gate for M4.

### 3. Speed: 12 to 15 s per year at year 100

The arithmetic. Light-eater nodes: about one per land cell, deciding every 20 days: roughly 8,000 × 18 = 140,000 updates a year. Eater nodes: up to `crowd` = 4 per cell, deciding every 10 days: up to 32,000 × 36 = 1.2 million. The `updates` column of `series.csv` has the real number. At about 10 microseconds per update, 1.3 million updates is 13 s a year. That matches what you see. So the cost is **count × cost per update**, and the count is bounded by the grid (16,384 cells × a few nodes × 18 to 36 decisions a year), not by the number of members. The target of 0.6 s a year means either 60,000 updates a year or 0.5 microseconds per update. Neither is reachable without M4's coarse regions and multi-rate steps. The plan says so (12.6): do not chase it before M4.

What to do before M4, all small:

1. **Measure.** Print microseconds per body update (bodies ms × 1,000 / `updates`) in the yearly line. Then you know whether the count or the cost moved.
2. **Let nodes decide less often.** `MAX_INTERVAL` is 60 days. The plan (7.3) puts population nodes at one month, and a plant node has nothing to decide. Raise `MAX_INTERVAL` to 360 and let the `INTERVAL` gene choose; the brain cost per decision already pushes plants toward long intervals (a plant deciding every 20 days spends about 30% of its upkeep on deciding). Light, rain and reactions are integrated over the interval already, and `vis` is refreshed after every take, so perception stays fresh. Expect plant updates to fall up to 18 times.
3. **Cheap wins in `live` and `act`.** `mass` is recomputed three times per update (`mass1`, `mass2`, `mass3`); `plan` runs twice; `wanted`, `thin`, `starve`, `wear` allocate a `Vec` each call; `visible` loops all substances with a phase test. Maybe 20 to 30%.
4. **Time the tape too.** `Recorder::month` runs every month over every body with a `BTreeMap` (about two tree operations per body). With 80,000 bodies that is roughly 0.2 to 0.4 s a year, and it is **outside** the four timed parts, so the printed speed does not show it. Keep the per-body monthly table only for the first years, or sample it.
5. **Rivers (problem 10).** Keep `exchange_days` 3 for watch runs. Rivers are a finished M0 result; 0.4 s is 3% of today's year. When the body side is the only cost left, make `exchange` cheaper: its condensation pass visits every cell for every condensing gas; a check per air node first would skip most cells.

### 4. Nodes limit evolution and actions

This is the one that matters most, because nodes now carry every result: plants are nodes from day 0, and eaters become nodes wherever five share a cell.

Evolution. In `copy::copy`, a node's children that stay are `b.count += births - m`: no mutation at all. Only the dispersing share (`DISPERSE` 0.1) gets `mutate`, and when they land they `join` the first node of their lineage, and `join` averages genes by count. So the genes of a node drift only by the averaged-in trickle from neighbours, and selection has nothing to act on inside a cell. The handover named this cost; here is the design to pay it, which is the M4 population node of plan 7.1 ("a small summary of how they differ"):

- A node keeps a **spread** per gene (one `f32` each), the standard deviation of its members around the mean. Children who stay add the mutation step to the spread (variance adds). A merge adds the gap between the two means. A merge also keeps the smaller node's genes in the spread, not only in the mean.
- A node **splits** in two when its spread in any gene passes `phys.alike` steps: two nodes with means at mean ± spread along that gene, half the count each, spread reduced. The crowd merge then re-merges the two most alike nodes in the cell. Selection acts between the nodes: the one that fits grows, the other shrinks and dies.
- A node also splits off **one individual** when the player looks, or when it draws an action a node cannot do (next point). The individual's genes are the mean plus noise from the spread.

This is "laws in average form": a population held as a few nodes per cell, each a cluster in gene space, re-clustered every month. It is about 100 lines in `copy.rs` and `body.rs`, and it is M4 work; do it first, before coarse regions, because without it M4 cannot pass its own check (M1 to M3 patterns still appear in merged form).

Actions. Instead of switching off the Bond actions for nodes, keep them in the scores. When a node draws one, split one member off as an individual (as above) and let the individual act. Individuals that hold or are inside something are already never merged (`crowd` skips them). So objects and crowds can share a cell: the node is the many, the individuals are the few doing something. This also answers "only individuals can build or hold": that stays true, but individuals appear where it matters, which is the plan's rule for splitting.

### 5. No seasonal travel

Animals live in the 287 to 305 K band because that is where their reactions release more than their upkeep at the body's temperature, and the body's temperature is the ground's plus about 1 K (its own heat over a conductance of about 30 W/K). Nothing can live further north until the catalyst or heat genes move, and they cannot (problem 4, and tiny steps). Where animals do live, winter does not bite. So there is no reason to move, and the brain's temperature feature (0.3 K per cell, over 5) is tiny anyway.

Expect seasonal movement only after: nodes evolve (4), intake has an area limit (1, so a grazed winter range is worth leaving), and catalyst upkeep is convex (12, so there are warm-adapted and cold-adapted kinds with an edge between them). Then the cold edge is where to watch. Nothing to build for it now.

### 6. No lasting trails

The physics is right and the test passes. The numbers: a 30 kg body crossing 2 km of cover at 1 kg/m² clears about 7 m² of a 2,000 m² lane, so a lane needs about 280 crossings to open and 25 m² a year to stay open (3 to 4 crossings a year). A node of 100 members clears 700 m² per crossing, so **herds make trails in a few trips**; single random walkers never will.

What is missing is a reason to walk the same way twice. Food is in every cell and liquid comes with the food. A trail needs two kinds of places (food here, liquid there, shelter there) and memory of them. Expect trails first where cover is patchy (after grazing) and liquid is in channels, and only from nodes. Do not tune `walk_wear` to force it; watch `worn_edges` against the `traffic` map once problems 1 and 4 are fixed.

One bug: `choose::regrow` is called from both `flow::liquid` (every 15 days, for 15 days) and `flow::creep` (once a year, for a year). So cover grows back into lanes at twice the documented 1 nm/s. Call it from one place.

### 7. Shelter

Walls work (+3.3 K thick). The gain is real now that cold costs: 3.3 K warmer is about 36% more energy released. But no body lives where it is cold (5), no body under 100 kg can make a part (hidden rule), nodes cannot enter (hidden rule), and building a closed shelter is a chain of about 12 specific actions (form, form, join ×5, anchor, enter) with no reward before the end. That chain is the main research risk of the plan (15), not a pre-M4 fix.

One cheap piece of physics that gives a first step: dense cover should shelter a body's heat the way walls do, by a conductance in series in `live` (the ground already gets `rc` from soft cover). Then "stay under the trees in winter" is the first shelter behaviour, and built shelters are an extension of it, not a leap.

### 8. No fat, no second rung

Do not speed up the open-world fat reaction. You saw why: uncatalysed fat-making turns the world's sugar into fat that nothing can burn (fat-burning without a catalyst runs at about 0.6% a year), so the food disappears. Fat must come from the **catalyst genes**: reaction 3 at gene +8 runs at about 10% a year inside the body, and the gene's range goes to 18. A body also needs the fat-burning gene (reaction 2) or it sees no well-being in fat (`wellbeing_at` counts only energy its own catalysts can release). That is a two-gene chain, normal for evolution, but it needs drift and selection inside nodes (4) and bigger steps (2). Fat's reward is real once winter bites (store energy across the lean season) and once a predator eats a fat-rich prey (3 times the energy per kilogram taken, under the same take cap).

Add to the yearly series the mean catalyst gene per reaction over live bodies ("which reactions life speeds up"). It is a raw fact, cheap, and it will show the fat genes moving long before fat shows on the map.

### 9. Little variety

Four causes, all above: genes averaged in nodes (4); children join by lineage, not by likeness (hidden rule); mutation steps too small (2); one narrow climate band (5). Fix 4 and the join rule first: `copy::copy` `joins` and `body::crowd` should test only `alike` in genes, never `lineage ==`. Then up to `crowd` = 4 variants can share a cell and compete, and the merge keeps the most alike together.

### 10. Rivers only half back

See 3.5. Keep the full moisture step for watch runs; buy the 0.4 s back later inside `exchange`.

### 11. Learning toward richer cells fell from 14 to 8 points

The test's own comment has the cause: a real-sized body eats its fill in a few takes, copies, and then gains almost no well-being from more food, so the learning signal fades. That is satiation, and it is fine. If you want a cleaner number, measure direction choice only while the body is hungry (`wb` low). Not a priority.

### 12. Cold tolerance is almost free

`upkeep` charges catalysts at `c_cat` × (sum of the genes in decades). A gene of 2.6 costs 520 J/kg/day; strength costs 60,000. Five more decades (100,000 times faster) cost 1,000 J/kg/day more, under 2% of upkeep, and buy any cold. In life, enzyme cost goes with the amount of enzyme, which goes with the rate itself, not with its logarithm. The right form is cost proportional to the speed-up factor. But chemistry 0's light reaction needs a factor of 10¹⁵ because its uncatalysed rate was set near zero, so a proportional cost cannot share one constant across reactions until the chemistry's rate constants are rescaled (M7, when chemistry is generated anyway).

For now: make the cost **convex in decades**, for example `c_cat × Σ gene²`, with `c_cat` divided by about 13 so the starting plants pay the same as today (the eaters pay less). Then 2.6 → 7.6 costs about 10 times more than today's step, and cold tolerance is a real trade-off. Note it in the handover as a decision.

### 15. The climate warmed

Two gases can do this from life alone, and both would be true emergence:

- **Water vapour.** It is two thirds of the air's heat absorption (0.004 of the air × 0.0007 per mol, against 0.0004 × 0.003 for the carbon gas). Plants evaporate liquid at their saturation pressure through a gas exchange of 316 mol of air per kg per day. More plants, more vapour, warmer air, more vapour. That is the vapour feedback.
- **The carbon gas.** Doubling it needs about 3 × 10¹¹ kg of sugar oxidised. If cover reached tens of kg/m² and then crashed under the eaters, that much plant carbon went to the air within decades. Climate change from an ecological crash, with no climate rule.

The yearly series has `absorber` and one `air_k` column per absorbing gas. Read which one doubled. If it is methane (substance 4), check that the air burns it (reaction 8 should remove it within months at the open world's rate); if it does not, the lazy reaction schedule in `change::step` for air nodes is the suspect. If it is vapour or the carbon gas, write it up as the first emergent climate change and keep the picture.

## The three biggest things that did not emerge, and the best guess why

As the check point asks (13.1):

1. **A plant and eater balance** (populations that cycle, not boom and crash). Physics: no intake limit by area, a child that costs nothing, and no third rung to eat the eaters.
2. **Kinds** (more than the two placed). Physics of the representation: nodes average genes, children join by lineage, mutation steps are a hundred times too small for the time available.
3. **Learned behaviour** (eating from zero, trails, shelter use). Learning and evolution scales (2); no reasons in the world (food and liquid everywhere, no cold where life is); and the hidden rules that take objects away from nodes and from small bodies.

None of these needs a new law. All of them are inside the five laws and the free numbers.

## Smaller code issues

| Where | Issue | Suggestion |
| --- | --- | --- |
| `flow.rs` `liquid` and `creep` | Both call `choose::regrow`; cover closes lanes at twice the documented rate | Call it from `creep` only, or from `liquid` only |
| `bond.rs` `MAX_PARTS` | Hard cap of 6 objects per cell | Remove, or merge identical loose parts in a cell into one part with a count, as bodies are |
| `bond::form` | A part needs 10 × `min_kg` and at most 10% of the body | Floor at `min_kg`; let `phys.form` be the share |
| `choose::act` | Nodes lose all Bond actions | Split one member off to act (see 4) |
| `copy::copy`, `body::crowd` | `joins` by lineage | Join by `alike` only |
| `body::upkeep` | Brain cost × body mass | Per member, by brain size; perhaps × mass^0.25 |
| `copy::copy` | Staying children of a node get no mutation | The spread design (see 4) |
| `tape/src/rec.rs` `month` | A `BTreeMap` over all bodies every month, outside the timers | Time it; sample or limit to early years |
| `tape/src/rec.rs` `describe` | Dead arms for `Born` and `Died` are now a comment | Fine |
| `lib.rs` `Config` doc comment | The river A/B numbers live in a comment | Also put them in the report |
| `docs/reports/` | No M0 to M3 reports, no report of the 8 runs | Save them with pictures; the repo is the record |

Things that are good and should stay: the keyed random numbers, the conservation ledger with `clamped_heat`, the debug audit after each law part, the honest seeding line, the free numbers in one `Physics` struct, the shared genomes, the per-row learning step, the direction features with the worn edge, and the tape's new columns.

## The list: what to fix before M4, in order

S is under a day, M a few days. Items 1 to 6 are the M4 population node, done first because it carries everything. Items 7 to 10 are small physics. Items 11 to 15 are measurement and records. Item 16 is the check point again.

| # | Fix | Size | Why |
| --- | --- | --- | --- |
| 1 | Children join nodes by gene likeness only, never by lineage | S | Kinds can separate inside a cell |
| 2 | A node keeps a per-gene spread; staying children add mutation to it; merges add the gap of means | M | Evolution inside nodes |
| 3 | A node splits in two along its widest gene when the spread passes `alike`; the crowd merge re-merges the most alike | M | Selection between variants with few nodes |
| 4 | Bond actions stay in a node's scores; drawing one splits one individual off to act | M | Objects where the crowd is |
| 5 | Remove `MAX_PARTS`; `form` floor at `min_kg` | S | Two hidden caps |
| 6 | Spray size of its own at seeding (biases spanning about ±1); `LEARN` seeded at -1.0; row mutation 3 to 6 times today's | S | Material for selection; measure genome memory |
| 7 | Intake limited by the area a body works: `kg ≤ density × harvest × interval × mass` in `take_body` and `take_node` | S | The brake on grazing; the first reason to move |
| 8 | `c_copy` to 2 × 10⁶ (then 10⁷) | S | A body costs energy to build |
| 9 | Catalyst upkeep convex in decades | S | Cold tolerance has a price; kinds by climate |
| 10 | Fix the double `regrow`; dense cover shelters a body's heat like walls | S | A bug; a first step to shelter |
| 11 | Print microseconds per body update; time the tape | S | Know what M4 must buy |
| 12 | `MAX_INTERVAL` to 360 days | S | Plant updates fall up to 18 times |
| 13 | `exchange_days` 3 for watch runs | S | Rivers whole; 3% of a year |
| 14 | Yearly series: mean catalyst gene per reaction; which absorbing gas rises | S | Fat and climate become numbers |
| 15 | Write up the 8 runs and the climate surprise in `docs/reports/`, with pictures; add the M0 to M3 reports | S | The record |
| 16 | Re-run the watch: 3 seeds, 300 years, neutral eaters; and the two comparisons (`--crowd` high against 4, `--own-brains` against shared) on 64x64 | M | The check point with the gaps closed; the M4 check |

After 16, decide M4 proper (coarse regions, multi-rate, planet). If the plant and eater balance, kinds and herds that move now appear, go. If neutral eaters still die, seed the eating bias, say so, and go anyway: that question belongs to M7's world search, not to the gate. Do not add planning, longer memory, a sixth law, or a people-like kind before then.

## What this review did not check

- Nothing was built or run. Speed and population numbers are yours; my estimates (10 microseconds per update, 1.3 million updates a year, 9 times surplus per meal) are arithmetic on the constants in `Config::phys` and `life::KINDS`, shown in the text.
- I did not read the viewer (`web/`) in detail. It is small and not on the path.
- I did not check chemistry 0 against reality beyond the rates I needed (sugar oxidation, fat-making, methane burning). Rough numbers are what the plan asks for.
- The climate cause (vapour or carbon gas) is a guess with a test next to it; the series will say.

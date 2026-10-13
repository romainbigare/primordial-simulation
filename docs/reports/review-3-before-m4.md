# Review 3: math, physics, chemistry and world concepts, before M4

13 October 2026. Code: `main` at 5ec1d40. I read all of `core`, `worldgen`, the tape's measures and
the three reports. I also ran three things: a 64 x 64 world for 40 years (seed 3), a profile of one
simulated year of that world, and seed 1 at 128 x 128 for 60 years, to check one finding. Where I
give a number, the arithmetic is in the text, so you can check it.

## Short answer

The round-2 work is good. The physics is now careful about heat and matter, the nodes can evolve
and act, and the reports are honest. Rivers, a living plant cover and animals that survive from a
neutral brain are real results.

But there are **four big things left**, and they are not speed work. They change what the world
means, so they must be fixed before M4 builds on top of them:

1. **Genes drift without selection.** The population-node math moves a gene's mean upward when
   the gene sits near the bottom of its range, and a node's spread grows with no limit when its
   cell is full. In my re-run of seed 1, 93% of all plant members carried a charcoal-burning gene
   (mean 2.3 decades) by year 20, with no eater and no charcoal in reach; by year 60 all of them
   did, at a mean of 8.9 decades, and 77% carried copper-smelting genes. The report's seed 1 shows
   the same: all 20 catalyst genes of the plants at 2 to 9 decades by year 60. The charcoal-burning story of seeds 5
   and 7 stands on the same genes, so part of that result is this artifact, not evolution. Next
   to it, gifts between bodies have no size limit, which makes plants of thousands of tonnes and
   is the engine behind the report's "giving" numbers.
2. **The greenhouse is saturated.** The air absorbs a share `1 - exp(-x)` of the heat leaving the
   ground and sends half back. At `x` 9 (seed 5) and `x` 17 (seed 7) that share is 1.0 in both.
   That is why both warm worlds sit at 295 K with twice as much gas in one as in the other. No
   amount of extra gas can warm this world any further. Climate change, ice ages and a hot world
   are out of reach until this is fixed.
3. **The world is starved of carbon, and the report misread the ecology.** Plants pull the carbon
   gas from 400 parts per million down to 150 within 10 years (to 3 by year 20 in my 128 x 128
   re-run) and then die of hunger, before any eater comes. The eaters live on dead plant matter on the ground (50 to 100% of what they eat),
   not on live plants. They help the plants, because they turn dead sugar back into carbon gas.
   "Eaters no longer wipe out the plants" is true, but the reason is that they mostly stopped
   eating plants.
4. **Catalysts cost almost nothing and never break.** Five decades of speed-up cost 0.6% of an
   eater's upkeep, so cold tolerance still has no price, and a body with a strong catalyst burns
   like a fire (974 K). Living catalysts fail above about 320 K; here they do not.

None of these needs a new law. Each is a change inside Copy, Flow, Change or the free numbers.
My advice: do the short list at the end (about one to two weeks), run the three seeds once more
with a null run next to them, and then go to M4. Going now would make the M4 check ("merged nodes
give the same patterns as individuals") meaningless, because the nodes have a bias of their own.

The owner's two questions are answered in their own section: yes, start with a roster of kinds,
with the limits I list there; and yes, plants should not pay for a brain.

## What is right and should stay

- The heat path of a body: one store of heat, the surface as a store of conductance, gas leaving
  with its heat, bodies radiating. The 6-year test with 974 K as the hottest body shows it holds.
- Conservation: drift 3 x 10⁻¹³ on energy and 5 x 10⁻¹⁵ on elements over 300 years, with the
  Neumaier sums and the `clamped_heat` ledger.
- The first-order reaction rate: methane now lives about 2 years in the air, as it should.
- Nodes that keep a spread, split, and act through one member: the right design. The bias below
  is in its details, not its shape.
- Eating limited by the area a body works. The honest seeding line in every log.
- The tape: catalyst means per reaction, kinds, trophic share, movement by month. These columns
  are what made this review possible.

## Math

### M1. The node spread math moves genes upward without selection

This is the most important finding. The evidence first, then the mechanism, then the fix.

**Evidence from the report's runs.** The yearly series holds `cat_0` to `cat_20`, the mean
catalyst gene per reaction over all live members. Plants are 99% of the members, so these are the
plants' genes. In seed 1 (`watch-2-before-m4/analysis.md`):

| year | cat_1 (burn sugar) | cat_5 (burn charcoal) | cat_11 (copper smelting) | cat_15 (lime burning) | cat_20 (copper tarnish) |
|---|---|---|---|---|---|
| 10 | 2.3 | 0.01 | 0.00004 | 0.002 | 0.002 |
| 30 | 2.3 | 0.0009 | 0.0000005 | 0.017 | 0.4 |
| 40 | 4.5 | 1.05 | 2.6 | 2.9 | 3.8 |
| 50 | 7.3 | 4.3 | 5.9 | 7.0 | 7.6 |
| 60 | 9.6 | 7.4 | 5.2 | 4.7 | 9.1 |

A gene of 9 is a speed-up of one thousand million times. Plants have no copper, no lime and no
charcoal in their stores, so no selection can want these. All twenty genes rise together,
between years 30 and 60. Seeds 5 and 7 show the same climb, slower, from year 60 on (all their
catalyst means are 2 to 7 at year 300).

**Evidence from my re-run** (seed 1, 128 x 128, today's code, read straight from the snapshots
with a small tool; the plants only, no eater exists before year 22):

| | year 20 | year 30 |
|---|---|---|
| plant bodies (all but 15 are nodes) | 12,313 | 29,933 |
| members | 5.5 x 10⁹ | 2.6 x 10¹⁰ |
| `cat_5` (charcoal) mean over members | 2.23 | 1.95 |
| share of members with `cat_5` on | 93% | 78% |
| `cat_11` (copper smelting) mean; share on | -1.37; 0% | 1.27; 60% |
| `cat_20` (copper tarnish) mean; share on | -1.16; 0% | 1.62; 60% |
| highest catalyst gene in any node | 8.1 | 18.0 (the top of the range) |
| standard deviation of the catalyst genes inside a node (member-weighted) | 2.2 to 2.4 decades | 4.0 to 4.4 decades |
| the same for the gas-exchange gene, in its own units | 0.65 | 1.16 |

At year 20, 93% of all plant members carry a charcoal-burning gene, mean 2.3 decades, in a world
with no charcoal in reach and no eater. By year 30 copper-smelting genes are on in 60% of the plant
members and some nodes sit at 18, the top of the range. The spread inside a node doubled in ten
years. Nothing selected any of this.

**Mechanism.** Five things in `copy.rs` act together:

1. **A catalyst gene's mutation step is one decade** (`range`: `(1.0, -2.0, 18.0)`), and two
   nodes are alike within `phys.alike` = 4 steps. So members whose catalysts differ by 10,000 times
   are one node, and a node sits at the split threshold with a spread of about 2.3 decades
   (`2 sqrt(2/π) x 2.3 = 3.7`, just under 4). The measured 2.2 to 2.4 is that threshold. For the
   scalar genes the step is 0.05 to 0.3, so the same rule holds them 3 to 20 times tighter.
2. **Clamping at the bottom of the range.** A catalyst gene starts at -1.0 ("off") and the range
   stops at -2, one step below. When a node splits (`split`, line 356) the halves are put at
   `mean ± d` and each is *clamped*. With `d` = 1.8 decades at the threshold, the low half is held
   at -2 while the high half goes to +0.8. A month later the halves are merged back by count
   (`body::crowd`) and the mean is +0.4 higher than before. Nothing was selected. `vary` (line
   96) clamps the children of a node the same way, with the node's full spread over `sqrt(m)`:
   for a small node sending two children, that is noise of 1.6 decades against a wall.
3. **The spread has no sink.** `widen` adds the mutation's variance at every generation; a merge
   adds the gap between the two means squared. Nothing takes variance away except a split (which
   keeps 36%). In life, selection and finite families remove variance every generation. Here it
   grows until the cap, which is `(half the range)²`: for a catalyst gene a standard deviation of
   10 decades.
4. **Splits are blocked in a full cell.** `split` returns when the cell has `crowd - 1` bodies or
   more (line 330). With `crowd` 4 that is any cell with a plant node and two animal nodes, or two
   plant nodes and one newcomer. There the spread grows (2.3 to 4.3 decades in the ten years
   above) and when a split finally comes, `d` is huge and so is the clamp bias.
5. **An off gene is never split along** (`on(g)`: a catalyst gene counts only when its mean is
   above 0), so its spread is never reduced, which feeds point 3 until the mean crosses zero by
   points 2 and 4. It is a one-way door.

There is also a tie in `split`: `widen` adds the same amount to every gene in step units, the
fold takes the *first* largest, so a node that can split tends to split along gene 0 (gas
exchange) rather than along the gene that matters. And since `step()` counts an off catalyst as
0, a node at -2 and a node at +3.9 are "alike" and merge, which stirs the means further.

**Why it matters.** The report's climate result ("life evolved a catalyst gene that burns
charcoal 100,000 times faster") stands on `cat_5` rising from 0.2 to 8 to 16 decades. The same
gene reaches 2.3 in plants by year 20 with nothing to select it. The direction in the eaters may
still be selection (charcoal is energy-rich and lies on the ground), but the size and speed of the
genes are inflated, and every other catalyst is inflated with them, each costing its carrier
upkeep (`c_cat` x gene²) for nothing. Every future evolution result from nodes is suspect until
this is fixed, and M4's own check (merged nodes against individuals) cannot pass with it in place.

**Fix (small, in `copy.rs`).**

- In `split`, keep the mean exactly: shrink `d` to `min(d, mean - lo, hi - mean)` before placing
  the halves, instead of clamping each half. In `vary`, draw the children's mean again (or shrink
  the noise) instead of clamping. Mutation itself may clamp: that is one small step at a time.
- Cap a gene's spread at the split threshold (about 6.3 step² from `2 sqrt(2/π x) = alike`),
  not at half the range. A node that cannot split does not keep widening. Better still, do not
  block a split by the cell's count: let the halves exist for the month and let the crowd merge
  take the most alike pair as it does now.
- Let the "off" region be wide: catalyst and uptake ranges down to about -6 (a gene below 0
  costs nothing and does nothing), so an off gene can wander without a wall one step below it.
- Catalyst mutation step 0.3 decades, not 1.0 (then "alike" is a factor of 16, not 10,000).
- Break the tie in `split` by spread over threshold with a keyed random pick among equals.
- Add a test: a node with no selection run for 1,000 updates keeps every gene mean within a few
  mutation steps of its start, with and without a full cell.

### M2. Gifts have no size limit, and an individual past its copy mass may never copy again

Two rules together make bodies of thousands of tonnes, and they are the engine of the "giving"
numbers of the report.

**Gifts.** `choose::give`: a body puts to another body a share of its spare stores up to
`phys.take x interval x its own mass`. For a node of 10⁷ members that is its whole spare. The only
other limit is the receiver's room, `level - store`, and `level` is *relative*: with a keep gene of
0.95 a body may hold 19 times the rest of its mass in one substance. So a node gives a 1 kg member
6 kg of sugar in one gift; rain then fills it with 3 times that in water; the next gift may be 20
times larger; and so on. The keep genes drift up to 0.95 (above it they do nothing, so nothing
holds them). In my re-run at year 20 the plants made 77,962 gifts in the year, moving 11 million
tonnes between plant bodies, all from neutral brains drawing "put to a body" by chance. The
heaviest plant member weighed 4,858 kg and was 90% water; at year 30 one weighed 306,778 kg. The
report's "giving yes, exchange yes" is this.

**Copying.** `copy::copy`: a node pays for children as far as its reactions can release the cost
now (`p`), an individual all or nothing (`p = 1.0`, line 143). A member far past `full` times its
copy mass owes all its children at once (93 times the copy mass is 309 children), its reactions
cannot release that in one interval, `pay` fails, and nothing is born. The body goes on growing.
The report's largest members at year 300: 11,138 kg (seed 1), 132,436 kg (seed 7), 1.87 million kg
(seed 5). Nothing weighs that, and a body with no death by age and no way to copy is an immortal
that only takes.

Fix: a gift is at most what the *receiver* could take by its own size (`take x interval x the
receiver's mass`), and a node gives per member, not per node. An individual copies as many whole
children as it can pay for now (`k = floor(k x p)`), the rest later; or better, P6's child store.
Three lines each.

### M3. The crowd merge joins unlike kinds, and a cell holds at most three animal nodes

`body::crowd`: when a cell has more than `crowd` (4) bodies, the two most alike merge, "however
unlike" (the handover says so). A new kind that walks into a cell with four bodies is averaged into
the nearest one every month. Children disperse to a random neighbour at every birth (`DISPERSE`
0.1), so every cell receives immigrants all the time, and the means get stirred. With the plant
node taking one place, at most three animal nodes live in any cell, and splits stop at two
(M1, point 3). I think this, more than mutation size, is why there are 1 to 3 kinds: the
representation has a hard ceiling of about three animal kinds per cell, and it blends what goes
over. Fix: merge only alike pairs; when a cell has more than `crowd` *unlike* nodes, keep them
(the number of truly unlike kinds in one cell is small in practice) and report the count of nodes;
raise `crowd` to 8 on a 64 x 64 test and watch the kinds column. The speed cost is linear in
nodes, so measure it.

### M4. Brain size cannot evolve where nodes are

`copy::copy` line 205: a node's children never start a new lineage, and the brain's sizes (hidden
units, memory, bodies seen, sight) live in the lineage. So the size genes of a node drift but do
nothing. Plants are always nodes, and eaters are nodes wherever they are dense, so in all three
runs `hidden_max` stayed 12 for 300 years. Review 1's item 13 ("intelligence becomes possible and
paid for") is switched off by the nodes. Fix: when a node's mean size gene is a whole unit from its
lineage's size, the node (not a child) moves to a new lineage with the brain carried over
(`rebase`, `rerow` exist). One place in `copy::split` or `body::day`.

### M5. The "kinds" measure cannot see diet

`tape::measure::point` clusters on five genes: copy mass, interval, strength, capture area and
the light catalyst. A charcoal-eater and a sugar-eater of the same size are one kind. Add the two
largest non-light catalyst genes and the uptake genes (which substances a body takes) to the
point. Then "kinds" can count the food chain, which is what the owner wants to see.

### M6. Watch-for items need a null run

Several "yes" and "weak" verdicts come from neutral brains drawing actions at random. A neutral
brain draws "put to a body" or "watch a body" about 1 in 20 decisions, so 50,000 plant gives a
year (78,000 in my re-run, moving 11 million tonnes, see M2) and watching that "reaches 96% of
groups" are what chance predicts, not learned giving or culture. The same holds for "busiest 2% of cells carry 100% of crossings" in seed 5, where almost
nobody moves. Fix: run one seed with learning off and the brain rows frozen (`LEARN` -4, no
`watch`) and print the same columns. An item counts as emerged only where the real run beats the
null run clearly. This is one flag and one table.

### M7. Smaller points

- `choose::act`: a body "goes back into the node" after a one-off action with
  `w.life.bodies[me].clone()` plus two `reborn` calls that compare 6 KB of rows each. About 23,000
  one-offs a year at 128 x 128; cheap enough, but `reborn` resets `Consts`, so the node's
  catalyst table is rebuilt at its next update every time. Keep `k` across `reborn` when the genes
  did not change.
- `Physics::think`: the cost of a decision goes with the brain and `mass^0.25`. A brain in life
  costs energy per unit of *time*, not per decision; here a body that decides every 360 days pays
  18 times less per day than one deciding every 20. That is a speed incentive dressed as physics.
  Keep it if you want (it pushes nodes toward monthly updates, as plan 7.3 asks), but say so in
  the handover, because it also means a plant's "thinking" is 40% of its upkeep at 20-day
  intervals and 2% at 360 (see Q2 below).
- `measure::kinds` samples *bodies*, so a node of 10⁸ members and a lone individual weigh the
  same. Weight the sample by members, or report both.

## Physics

### P1. The greenhouse saturates: more gas cannot warm the world

`flow/air.rs` line 58: the air's absorbing share is `f = 1 - exp(-Σ amount x ir / area)`, and the
air sends `f σ Ta⁴` down (line 140). This is a one-slab grey atmosphere. Its ceiling: when `f` is
1, the surface is `2^(1/4)` times the planet's effective temperature and no more. The runs are at
that ceiling:

| | start | seed 1, y300 | seed 5, y300 | seed 7, y300 |
|---|---|---|---|---|
| `absorber` (the exponent) | 1.46 | 1.22 | 9.4 | 17.5 |
| `f` | 0.77 | 0.70 | 0.9999 | 1.0000 |
| ground temperature | 288 | 287.3 | 295.4 | 295.2 |

Seed 7 has twice the gas of seed 5 and the same temperature. On Earth, 37 times the carbon gas
would be about five doublings, 15 to 20 K; here the whole effect is capped at about +7 K and
reached. For the game this closes the door on strong warming, on a hot world and on the
"industrial burning changes the climate" item of the watch list.

Fix (Flow, about 30 lines): split the air column into a few grey layers (4 is enough), each
absorbing `1 - exp(-τ / 4)` of what passes and emitting its own `σ T⁴` up and down, with the
layers' temperatures solved the same implicit way the surface is. Warming then keeps growing with
the gas (as `(1 + 3τ/4)^(1/4)` in the limit). Real gases warm by the *logarithm* of their amount,
because their absorption bands fill up; a cheap stand-in is to let a gas's optical depth grow as
the square root of its amount past some column amount. That is a free number to tune, and it is
chemistry data, not a climate rule.

### P2. The map's size and its latitudes disagree by 20 times

The island is 128 cells of 2 km: 256 km across. Its rows span latitude 5° to 55°, which on a
planet is about 5,500 km. The sun uses the latitudes; the air mixing (`mix` 2 x 10⁴ m²/s) and the
winds use the 256 km. Air crosses the whole map in about `L² / K` = 38 days, so no north-south
difference can survive: 3 to 4 K against 26 K at the start. This is the open decision 3 of the
last two reports. The honest fix is consistency, not a smaller mixing number: an island 256 km
across should span about 2° of latitude (set `lat_south` 40, `lat_north` 42.5), and its climate
bands then come from height (the lapse rate is already there) and from the sea. Real latitude
bands belong to the planet of M4, where a cell is about 80 km. Do not tune `mix` to fake them.

### P3. Nothing has height, so there can be no trees, no forest and no wood

Light in a cell is shared per m² of capture area by every body alike (`light[i]` in
`flow::air::radiation`, taken by `body::live` as `mass x leaf x (light[i] - mark)`). A 1,000 kg
plant member gets exactly the same light per m² of leaf as a 1 kg one beside it. There is no
reason to be tall or big, so "plants" stay a soup of 1 kg members (W1 below). On Earth the forest
is the result of a race for light by height; wood is what a tall plant is made of; and wood is the
first fuel, the first building material and the first fire. None of this can appear here.

Fix (physics, about 20 lines): order the bodies of a cell by the height of a member, taken as
`(member mass / density)^(1/3)` (a body has no geometry yet; parts do), and let the light pass
through them in that order, each layer absorbing `1 - exp(-a_layer)`. Then a big member shades a
small one, size pays, and selection can find trees. Keep the per-cell `canopy` for the ground's
reflectance and for cover.

### P4. The world is starved of carbon, and the eaters are decomposers

The arithmetic. An air column holds about 3.6 x 10⁵ mol of gas per m²; at 400 parts per million
that is 142 mol of carbon gas per m², which as plant sugar (CH₂O, 30 g/mol) is **4.3 kg/m²**. So
the whole air over a cell can make 4.3 kg/m² of cover, and the whole air over the map about 11
kg/m² on land. A real grassland is 1 to 2 kg/m², a forest 10 to 50. The plants' light reaction is
first order in the carbon gas and runs at 10¹⁵, so a plant node takes *all* the carbon gas in the
air it exchanges at every update (`body::live`, `tmp.pool`). Light does not limit them: at 400
ppm a plant's gas exchange brings 30 times less carbon than its light could fix.

What the runs show (`series.csv` of my 64 x 64 run; the 128 x 128 seeds show the same):

| year | plants, members | cover median kg/m² | carbon gas, mole fraction | plant deaths a year | of them `spent` |
|---|---|---|---|---|---|
| 5 | 3.3 x 10¹⁰ | 0.03 | 2.9 x 10⁻⁴ | 1.1 x 10¹¹ | 62% |
| 8 | 2.7 x 10¹⁰ | 0.30 | 1.9 x 10⁻⁴ | 1.4 x 10¹¹ | 71% |
| 10 | 2.0 x 10¹⁰ | 0.03 | 1.7 x 10⁻⁴ | 1.1 x 10¹¹ | 72% |
| 20 | 1.5 x 10¹⁰ | 0.04 | 1.5 x 10⁻⁴ | 7.5 x 10¹⁰ | 75% |
| 25 (eaters since 22) | 1.0 x 10¹⁰ | 0.03 | 1.7 x 10⁻⁴ | 5.7 x 10¹⁰ | 75% |
| 30 | 2.5 x 10¹⁰ | 0.51 | 2.4 x 10⁻⁴ | 1.5 x 10¹¹ | 71% |
| 40 | 3.1 x 10¹⁰ | 1.35 | 2.5 x 10⁻⁴ | 2.0 x 10¹¹ | 76% |

The cover crashes at year 8 to 10, before any eater exists, as the carbon gas falls below 200 ppm.
Plants then die of `spent` (they cannot pay their upkeep) at five to seven times their standing
number a year. When the eaters come, the carbon gas *rises* and the cover grows 30 times, because
the eaters eat the dead sugar on the ground and breathe it out as carbon gas. In my run the eaters
took 2 to 7 x 10⁹ kg a year from the ground and 2 x 10⁸ kg from live plants. In seed 1 at year 300
the ground is 100% of their intake and live plants 0.004 kg a year. So:

- The eaters are **decomposers** (detritus eaters), not grazers. The report's "the brake on grazing
  works" is the wrong reading; the right one is "eaters rarely graze".
- The ecology is a **carbon cycle with a tiny atmosphere and no other store**. Earth's ocean holds
  50 times the air's carbon and takes up a third of what is emitted within decades; its soils hold
  twice the air's carbon. Here the carbon gas has no way into the sea (gases do not dissolve in
  liquid anywhere in Flow; `solubility` is read only by bodies), and dead sugar on the ground
  oxidises at 7% a year with no decomposer. So the live plants, the dead sugar and the air fight
  over one small pool, and the only new carbon is the charcoal in the rock. That is also *why* life
  found charcoal burning in two seeds: it is the only way to get more carbon into the cycle. The
  warm seeds have five times the plants of seed 1. The real story is carbon fertilisation, not
  only warming.

Fixes, in order of size:

1. Gases dissolve into standing liquid by their `solubility` and the liquid's amount (Henry's law:
   dissolved share proportional to the gas's partial pressure), in `air::exchange`, where
   evaporation already lives; dissolved gas flows with the liquid and comes back out when the
   liquid warms. About 30 lines. It gives the ocean its carbon store and a slow buffer to the air,
   and it is physics (the same law as the vapour), so it also gives dissolved oxygen for future
   sea life.
2. Say in the report what the eaters eat (the `ate_*` columns already have it) and name the thing:
   the first food chain is plants, dead matter, decomposers. That is true emergence and worth a
   picture.
3. Consider raising the starting carbon gas (it is a free number of chemistry 0's `init`); this is
   a choice for the owner, because 400 ppm is Earth-like and the small pool is honest.

### P5. Catalysts are nearly free and never break

`body::upkeep` charges `c_cat` (15 J per kg per day) times the sum of the squares of the catalyst
genes. Strength costs `c_strength` (10⁵) times the strength gene. For the starting eater:

| | J per kg per day | share of upkeep |
|---|---|---|
| strength 0.6 | 60,000 | 93% |
| burn gene 2.6 | 101 | 0.2% |
| a charcoal gene of 5 (speed-up 100,000) | 375 | 0.6% |
| the same gene at 16 (seed 7's eaters) | 3,840 | 6% |
| cold tolerance: burning 7 times faster, from 290 K to 270 K (+0.86 decades) | +83 | +0.1% |

So the convex cost of round 2 did not give cold tolerance a price for eaters, and did not give kinds
by climate. The one constant must serve the light reaction (10¹⁵) and the burn reactions (10²–10⁵)
at once, and it cannot. The handover deferred the proportional cost to M7 because of the 10¹⁵. I
think that deferral is not needed: charge enzymes by their **capacity**, the catalysed rate at a
reference temperature, `c x Σ_r cat_r x k_r x exp(-Ea_r / 290 K)`. That is "cost by the amount of
enzyme" (the handover's own words) and one constant serves every reaction, because it measures
what the body can do per second, not how slow the bare reaction was. With it the plants would trim
the light gene from 15 to about 12 (they need 10⁻⁶ per second, not 10⁻³), the eaters would trim the
burn gene, and seven times faster burning in the cold would cost seven times the enzyme. Pick `c`
so the starting plants pay what they pay now.

The second half: in `change::react` a catalyst multiplies the rate at any temperature. A body with
a strong catalyst for an exothermic reaction that takes no gas (fat making, charring) runs it flat
out with no control, the body warms, the rate rises with the warmth, and the body is a fire. The
974 K bodies are this. Real enzymes denature above about 320 K: their rate falls off a cliff. Fix:
in `react`, apply a body's catalyst factor only below a denaturing temperature, and let that
temperature be a gene with a cost (a hot-adapted catalyst is a trade-off). One line in the rate and
one gene. It also gives, for free, a reason why a body cannot simply burn everything it holds.

### P6. Copy cost is paid in one go

`c_copy` is paid at the moment of copying. The report says 10⁷ J/kg (the biological number) "did
not fit: an eater cannot release that in one interval". In life the cost is spread over a
gestation. Let a body pay into a child store across updates (one `f64` per body: energy already
paid toward the next child), and copy when the store covers it. Then `c_copy` can be 10⁷, M2's
immortal giants go away, and copying is a process, not a jump.

### P7. Other physics notes, smaller

- The sea's whole depth counts as one mixed layer (`cap_ground` sums all loose liquid). An 800 m
  column has a heat capacity 10 times that of a real mixed layer, so the sea warms over centuries.
  Fine for now; wrong for a planet with ice ages. Note it.
- Water vapour is two thirds of the greenhouse (1,424 mol/m² x 0.0007 = 1.0 of the 1.46). The
  vapour feedback is in. Good.
- `body::live`: a plant member evaporates about 1.7 kg of water per kg per 20 days and catches
  rain on its capture area. Transpiration of about 300 kg of water per kg of sugar made is the
  right order. Good.
- Latent heat, fusion plateaus, lifting work: all booked. Good.

## Chemistry 0

### C1. Charcoal oxidises too fast in the open, and life can burn graphite

Substance 7 (pure carbon, "charcoal / coal") burns with oxygen (reaction 5) at `k` 10⁵, `Ea`
11,000 K. At 290 K that is 3.4 x 10⁻¹² per second times the oxygen share 0.21, a lifetime of about
45,000 years (the report says 6,400 from a run; either way, thousands of years). Coal seams on
Earth last hundreds of millions of years at the surface temperature: graphite is one of the most
inert forms of carbon, and no living thing can oxidise it. Here a catalyst gene of 5 to 16 decades
makes it the main food of two worlds. Chemistry 0 is allowed to be rough, but this is the reaction
that drives the only climate result, so it should be chosen on purpose. Options: raise `Ea` of
reaction 5 to about 20,000 K (ignition moves to about 800 K; the open-world lifetime at 290 K becomes
geological), and let the convex or capacity cost of P5 make a 10¹⁰ catalyst unaffordable. Then
burning charcoal needs heat first, which is a thing bodies could learn to make, and that is the
M7 story we want (fuel, fire), not a digestive trick.

### C2. One catalyst range for every reaction

`copy::range`: every catalyst gene can reach 18 decades at the same cost per decade. The light
reaction needs 15 because its bare `k` is 10⁻⁹; the burn reactions need 2 to 5. P5 fixes the cost.
If chemistry 0 is touched at all, also rescale the light reaction's `k` to about 10⁻⁶ (its
open-world rate at 290 K stays negligible, 6 x 10⁻⁸ a year) so the plants' gene is about 12 and
closer to the others.

### C3. The carbon gas has one slow sink and no store

Only reaction 18 (rock weathering, geological) and the plants remove carbon gas. See P4: dissolving
into the sea is the missing piece. Also: dead sugar on the ground is a real soil carbon pool, but
with no decomposers it oxidises at 7% a year at 290 K and about 0.5% at 270 K, so in cold places
the dead matter piles up for centuries. That is peat-like and fine. Say it in the report.

### C4. Rough numbers that are fine

Sugar burning 15.6 MJ/kg, fat 46.7, charcoal 32.8; methane 25 times the carbon gas per mole in
absorption; the vapour pressure curve; melting plateaus. All the right order. Nothing to change
here before M7.

## World concepts

### W1. "Plants" are a soup, not plants

A plant is a node of 1 kg members that copy at 2 kg, decide every 20 days, and die at 5 to 37
times their standing number a year (seed 1 year 40: 9.9 x 10¹² births and deaths for 2.7 x 10¹¹
members). A member lives about ten days. This is algae, not grass or trees. Four causes, all above:
no height (P3), carbon hunger (P4), a copy at every decision when the mass allows (the copy chance
is a brain row, kept at the old logit, and `full` forces it), and no advantage to size. The map
class "forest" of plan 10.3 cannot appear from this.

### W2. Nothing dies of age

There is no ageing. Plants die of hunger, animals of hunger or of being eaten, and a body that
avoids both lives forever (M2's giants). Lifespan sets generation time, which sets how fast
evolution and culture can move, and it is what makes "dynasties" and "knowledge lost when
specialists die" possible later. I do not have a clean physics for it, and I will not propose a
rule. The nearest honest thing is damage: a body's catalysts and strength lose a small share per
year that repair (upkeep) can buy back at a cost that grows with age; or a waste substance that
metabolism makes (chemistry 0 has none) which the body must put out. This is a decision for the
owner; I only note that the world has no lifespan and that it will show in every long run.

### W3. The decision interval is the reproduction interval

A body copies only at a decision, so a plant that decided once a year would have at most one
child a year while its members die ten times a year. That is why the interval gene cannot lengthen
(report problem 5) even though deciding costs a 1 kg plant 40% of its upkeep. Decouple them: let
copying at `full` happen in `body::live` (physics, no brain), and keep the brain's copy row for
copying *before* `full`. Then the interval gene is free to lengthen, and the plant updates fall
by itself, which is the speed gain the report hoped for from item 12.

### W4. Eaters are decomposers

See P4. The food chain today is plants, dead matter, eaters. It is a good result. Name it, and
measure grazing (`ate_light_eaters`) and predation (`ate_others`) against it in every report.

### W5. The three warm-world facts belong together

Warm seeds: more carbon gas, more plants (x5), charcoal eaten, ground 295 K. All four are one
story: a carbon-starved world found a carbon source. The report tells it as a climate story. Tell
it as a carbon story, with the greenhouse cap (P1) as a caveat, and with M1's artifact as a
caveat on the genes.

## Speed

The yearly line already prints the four parts. On my 64 x 64 world at year 40: flow 227 ms, change
25, bodies 953, parts 0.2, tape 26 ms a year; 14.7 µs per body update. Bodies are 78% of the
year, as in the 128 x 128 runs (90%). So the question is what one update spends. I profiled one
resumed year (year 41 of that world, 65,158 body updates) with `valgrind --tool=callgrind`.
It counts instructions, not time, which is good enough for shares.

**The first surprise is outside the simulation.** Of all the instructions of the run, 85% were
spent *saving one snapshot*, in `memset`. `tape::save_snapshot` writes with
`GzEncoder<BufWriter<File>>`: the buffer sits between the compressor and the file, so `bincode`
hands the compressor one 8-byte number at a time, and the compressor's write path zero-fills its
working buffer at every call. Measured natively on the saved worlds:

| snapshot | as in the code | with `BufWriter<GzEncoder<File>>` | size |
|---|---|---|---|
| 64 x 64, year 40 | 1.38 s | 0.16 s | 20.7 MB |
| 128 x 128, year 30 (30,000 plant nodes) | 22.6 s | 4.9 s | 279 MB |

The snapshot time is not in the printed seconds per year (it sits outside the four timers), but it
is in the run: 30 snapshots of 22 s are 11 minutes of the 118-minute seed 1 run, 10%. One line to
fix. The snapshot is also 279 MB because every group's brain rows (6 KB each, 30,000 groups) and
every genome are saved; most of those groups have one member. Worth a look, not urgent.

**Inside the simulated year** (`step_marked`, 8.7 x 10⁹ instructions; shares of it):

| part | share | inside it |
|---|---|---|
| bodies (`body::day`) | 66% | `choose::act` 46% of the year: perception and options about 19%, `copy::copy` **19%**, `brain::think` 6%, `brain::learn` 2%, `copy::split` 1.4%; `live`: `react` 7%, `plan` 2.4%, `heat_cap_ground` 9% (306,601 calls, 4.7 per update), the rest about 5% |
| flow | 26% | `air::exchange` 13% (evaporation and rain every 3 days), the rest 13% |
| survey and the monthly crowd merge | 5% | |
| change (ground and air) | 3% | |
| tape (`record_month`) | 1.3% | |

Two things stand out. `copy::copy` costs three times the brain: a node's dispersing children get a
new genome every time (`shake` over all 1,560 row weights with one random number each, then a 6 KB
allocation and a 6 KB compare in `join`), and `vary` draws four random numbers per gene for 103
genes. And `heat_cap_ground` (a walk over 2 x 22 substances) is recomputed about five times per
update: once in `live`, once per substance taken in `take_node` (`take_heat_with` calls it each
time), and in `draw` and `heat_room`.

Obvious gains, in order of size against risk:

1. **Buffer on the right side of the compressor** in `save_snapshot` (and in `rec.rs` line 589):
   18 s per snapshot at 128 x 128. One line, no risk.
2. **Plants without a brain (Q2 below) and copying decoupled from decisions (W3).** Plant updates
   are about a third of all updates (15,000 plant bodies x 18 a year against 9,000 to 14,000
   animal nodes x 36 to 72). A plant that decides nothing and updates every 90 days cuts the
   plant share by 4 to 5 times: about 25 to 35% of a year.
3. **Cheaper `copy`:** shake only the row biases (120 numbers) for dispersing children, since the
   rows' spread is one number for all rows anyway; draw the genes' noise with one 64-bit number
   per four genes as `shake` does; skip the new genome when nothing changed. About 15% of the year.
4. **One `heat_cap_ground` per cell per day,** kept in `Tmp` or in a per-cell cache that `take_heat`
   updates. About 8%.
5. **`live` and `act` recompute the same sums:** `mass` or `mass_cap` up to eight times per update
   (`live`, `act`, `wellbeing_at`, `take_node`, `wanted`, `reach`, `pay`, `visible`);
   `walls_around` scans the cell's bodies for every body. A `Tmp` that carries them, filled once:
   perhaps 5%.
6. **`react` tests all 21 reactions at every sub-step** for every body, though a body only has
   inputs for 3 or 4. Build the list of reactions whose inputs the body holds once per update
   (`Consts` could hold it). Perhaps 3 to 5%.
7. **The 128 x 128 cost is the count of updates** (1.3 million a year), as review 2 said. Items 2
   to 6 together are about a two-times gain on the body side. The 0.6 s a year target is M4's:
   coarse regions and monthly nodes. Do not chase it now.

## The re-run of seed 1 (60 years at 128 x 128, today's code)

`ew run --seed 1 --size 128 --years 60 --every 10 --frames-every 1000`, code 5ec1d40, 1,800 s.
Light-eaters placed in year 2, eaters in year 22, as in the report. The snapshots were read with a
small tool (not in the repository) that prints the genes and spreads of every live body.

| year | plant bodies | plant members | carbon gas (mole fraction) | cover median kg/m² | heaviest member kg | plant deaths a year | gifts a year (kg moved) |
|---|---|---|---|---|---|---|---|
| 10 | 5,723 | 4.7 x 10¹⁰ | 1.6 x 10⁻⁴ | 0.08 | 584 | 2.2 x 10¹¹ | 429 (1.2 x 10⁸) |
| 20 | 12,313 | 5.5 x 10⁹ | 3.1 x 10⁻⁶ | 0 | 4,858 | 2.8 x 10¹⁰ | 77,962 (1.1 x 10¹⁰) |
| 30 | 29,933 | 2.6 x 10¹⁰ | 3.4 x 10⁻⁶ | 0.36 | 306,778 | 1.4 x 10¹¹ | 316,539 (2.9 x 10¹⁰) |
| 40 | 31,100 | 6.5 x 10¹⁰ | 6.3 x 10⁻⁶ | 0.90 | 8,400 | 3.4 x 10¹¹ | 111,496 (1.0 x 10¹⁰) |
| 50 | 42,800 | 3.4 x 10¹¹ | 1.1 x 10⁻⁴ | 4.5 | 31,300 | 4.0 x 10¹² | 35,601 (1.5 x 10¹⁰) |
| 60 | 41,740 | 3.0 x 10¹¹ | 2.7 x 10⁻⁴ | 5.8 | 14,564 | 4.0 x 10¹² | 26,243 (1.3 x 10¹⁰) |

The catalyst gene means over plant members (the `cat_r` columns), and the standard deviation of
that gene inside a node at year 60:

| year | cat_1 burn sugar | cat_5 burn charcoal | cat_11 copper smelting | cat_15 lime burning | cat_20 copper tarnish |
|---|---|---|---|---|---|
| 10 | 2.27 | 0.02 | 0.0000000001 | 0.000004 | 0.0005 |
| 20 | 2.63 | 2.30 | 0.0004 | 0.06 | 0.00002 |
| 30 | 2.63 | 2.17 | 1.62 | 0.84 | 1.92 |
| 40 | 4.23 | 4.47 | 2.31 | 0.88 | 4.24 |
| 50 | 5.58 | 5.87 | 3.08 | 1.55 | 3.15 |
| 60 | 6.09 | 8.91 | 2.98 | 2.28 | 2.80 |
| share of plant members with the gene on, year 60 | 100% | 100% | 77% | 72% | 67% |
| spread inside a node at year 60 (standard deviation, decades) | 8.8 | 9.1 | 9.3 | 9.4 | 9.5 |

What this says:

- **M1 is reproduced.** The report's seed 1 had `cat_5` at 7.4 and `cat_1` at 9.6 by year 60; this
  run has 8.9 and 6.1, with every plant member carrying both. The spread inside a node is 9 decades,
  which is the cap `(half the range)²` (10 decades) nearly reached, and every catalyst gene spans
  the whole range -2 to 18 across the nodes. All of this is in plants, with no charcoal, copper or
  lime in reach. The copy-mass gene also sits at its top wall in the heaviest nodes (10,000 kg),
  so those members never reach their copy mass and never copy.
- **M2 is reproduced.** The heaviest plant member is 14,564 kg and is 100% water; a node of 1,572
  members averages 9,909 kg, 98% water, with a keep gene for sugar of 1.59 (above the 0.95 the code
  uses, so nothing holds it). 832 plant bodies have members over 100 kg. The gifts peak at 316,539
  a year moving 29 million tonnes (year 30), all between plants.
- **P4 is reproduced.** The carbon gas falls to 3 parts per million by year 20 with no eater; the
  cover is 0 and the plants fall ten times. It recovers only when the eaters recycle the dead sugar
  (year 40 to 60). The eaters took 7.5 x 10¹⁰ kg from the ground and 1.1 x 10¹⁰ from live plants in
  year 60: 87% decomposers.
- **M3 shows as churn.** There are 6.7 plant bodies per land cell against a `crowd` of 4: children
  drawn from nodes with a 9-decade spread are alike to nothing, start new bodies every update, and
  the monthly merge folds them back "however unlike". The body time is 52 s a year at year 60 for
  this reason, against 1.7 s at year 10.
- The greenhouse exponent stayed at 1.0 to 1.5 (no charcoal burning yet; 9 x 10¹¹ kg of charcoal
  lies loose on the ground from erosion, waiting), the ground at 285 to 289 K, kinds 1 to 2,
  `hidden_max` 13.

## The owner's questions

### Q1. Can we start with more kinds, a roster of animals and plants with set properties?

Yes. The handover allows it: what is placed at the start is an initial condition, not a law, as
long as every report says what was placed (the eaters of year 22 are already this). A roster of
ten kinds is the same kind of choice as two kinds, and it has three real benefits: the food chain
has its rungs on day one (grazers, browsers, predators, decomposers; small fast ones and big slow
ones); the comparison "do kinds stay distinct" becomes a measurable thing; and it is what the M7
world search would place anyway. I would do it, as `worldgen::life::KINDS` with ten rows and a
seeding line per kind.

Three limits, so the roster is not a disappointment:

1. **Only properties the physics reads can be set.** Today those are: member mass (copy mass),
   strength, decision interval, the four exchange genes (gas, liquid, heat, capture area), the
   catalysts (what it digests), the uptake genes (what it takes), dispersal, learning rate, and the
   brain's sizes. **Height, lifespan, running speed and "intelligence" beyond brain size do not
   exist in the physics.** A height number in the roster would do nothing until P3 is built; a
   lifespan until W2; a speed until a move takes time (today a move is a decision, so speed is the
   interval gene). Build the physics first, then the property means something.
2. **The kinds will not stay distinct until M3 and M4 are fixed.** The crowd merge blends unlike
   nodes in any cell with four bodies, and brain sizes are frozen in nodes. Ten kinds placed today
   would be three kinds by year 50, as the two placed kinds in three seeds were still two.
3. **Say what the roster is in every report**, and run one seed from two kinds beside it, so we
   know what the roster gave and what emerged. The emergence we want is in what the kinds *do*
   (herds, trails, tending), and a roster makes that more likely, not less.

### Q2. Why should a plant have its own brain? Should it not be a "kind brain"?

You are right, and the code already half agrees. A node is one brain for all its members, so a
plant node of 10⁷ members runs one brain, not 10⁷. What costs is that each of the 15,000 plant
nodes evaluates its brain every 20 days (about 2,400 multiplications) and that a 1 kg member pays
10⁵ J per decision, 40% of its upkeep, for decisions it cannot use: a plant cannot move, and its
light, gas, water and copying are physics. The only actions a plant's brain really takes are "put
to a body", "watch", and the one-off bond actions, which are noise from a neutral brain (M6).

A "kind brain" in the sense of one shared, evolving brain per kind is what the lineage base plus
the group rows already are. What I recommend instead is to let the brain **shrink to nothing**:
let the hidden-units gene go to 0, and a body with no brain takes no decisions and pays no decision
cost; its physics runs at its update (light, gas, water, upkeep, copying at `full` from W3). Then
plants evolve brainlessness by themselves (it saves 40% of their upkeep), the plant share of the
body time drops 4 to 5 times (Speed, point 2), and a plant that later *benefits* from a brain (a
tended kind, a kind that signals) can grow one back. That is a law (a brain has a size and a cost,
and zero is a size), not a rule about plants. For animals keep the group brain: it is the carrier
of culture.

## Go to M4?

Not yet, but close. The gaps that round 2 closed are real. The four findings above (M1, P1, P4,
P5) are not speed work and they change what the results mean: a biased node math, a capped
greenhouse, a misread ecology, and free catalysts. Fix them, run the three seeds again with a null
run beside them, and then go to M4 with its first job as the report says: parts held by nodes,
then coarse regions and multi-rate steps. If the owner prefers to start M4 now, do M1 and P1 first
anyway, in the first week of M4, because M4's own check (merged nodes against individuals) cannot
pass with M1 in place.

## The list, in order

S is under a day, M a few days.

| # | Fix | Size | Finding |
|---|---|---|---|
| 0 | `BufWriter` around the `GzEncoder` in `save_snapshot` and `rec.rs`: 18 s per snapshot at 128 x 128 | S | Speed |
| 1 | `split` keeps the mean; `vary` does not clamp; spread capped at the split threshold (or splits never blocked by the cell count); off ranges down to -6; catalyst mutation step 0.3; tie broken with a keyed random pick; a no-selection test | S | M1 |
| 2 | A gift is at most `take x interval x the receiver's mass`, per member of the giver; an individual copies as many whole children as it can pay (`k x p`); copy cost paid into a child store across updates, `c_copy` to 10⁷ | S | M2, P6 |
| 3 | Merge only alike nodes; report node counts; test `--crowd 8` on 64 x 64 for kinds and speed | S | M3 |
| 4 | A node whose mean size gene moved a whole unit changes lineage (brain carried over) | S | M4 |
| 5 | Catalyst cost by capacity (`cat x k x exp(-Ea/290)`), one constant; catalysts off above a denaturing temperature (a gene with a cost) | S | P5 |
| 6 | Four grey layers in radiation (or the `(1 + 3τ/4)` form); optical depth as the square root of amount past a column amount, as data | M | P1 |
| 7 | Latitude span 40° to 42.5° for the island; bands from height; planet latitudes at M4 | S | P2 |
| 8 | Light passes through a cell's bodies in order of member height (`(m/ρ)^(1/3)`) | S | P3, W1 |
| 9 | Gases dissolve into standing liquid by solubility (Henry), flow with it, come out when it warms | M | P4, C3 |
| 10 | Copying at `full` in `live`; the brain's copy row only before `full`; hidden-units gene to 0 means no brain, no decision, no cost | M | W3, Q2 |
| 11 | `kinds` with diet genes and member weights; a null run (`--null`: learning off, rows frozen) printed beside each seed | S | M5, M6 |
| 12 | Reaction 5: `Ea` to about 20,000 K (decide with the owner); light reaction `k` to 10⁻⁶ if chemistry 0 is touched | S | C1, C2 |
| 13 | Roster of ten starting kinds in `KINDS`, each with a seeding line; one seed from two kinds beside it | S | Q1 |
| 14 | Cheap speed: cheaper `copy` (biases only, one random number per four genes); one `heat_cap_ground` per cell per day; one `mass_cap` per update; the body's reaction list once per update; keep `Consts` across `reborn` | S | Speed |
| 15 | Re-run three seeds for 300 years with the null run; report the carbon story, the food chain, the gene means, the greenhouse exponent, and what the roster gave | M | all |

Then M4, as planned. Do not add planning, a sixth law, or a people-like kind before then.

## What this review did not check

- The viewer (`web/`) and the pictures beyond reading them.
- I did not reproduce the 300-year runs; my runs are 40 years at 64 x 64 and 60 years at
  128 x 128, and one profiled year. The 300-year numbers are the report's.
- The profile counts instructions, not time; memory-bound parts (flow) take more time per
  instruction than the table shows. The native timers of the CLI say bodies 78%, flow 18%.
- The re-run and the snapshot tool measured the gene drift directly (M1); the small model of the
  clamp alone gives a floor of the effect, not its full size.
- Chemistry 0 beyond the reactions that carry results (light, sugar, fat, charcoal, methane,
  weathering).

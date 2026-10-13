# Watch report 2: the second fix list before M4

13 October 2026. Code: `main` at 187d8de (seeds 1 and 7, and seed 5 to year 210) and deb620e (seed 5
from year 210). Review that started this work: `review-2-before-m4.md`. Full numbers:
`watch-2-before-m4/analysis.md`.

## Short answer

We did all 16 items of the second review, and one decision of the owner (the reaction rate). The
long runs then found seven more bugs. We fixed them all and ran the three seeds again.

What changed in the world:

- **Eaters no longer wipe out the plants.** The median plant cover now stays near zero for 0 to 4
  years after the eaters come, not 40 to 65 years.
- **Neutral eaters live.** With no hand-set brain (every action bias starts near 0), eaters now
  survive 300 years in all three seeds. Before, they died out every time. Behaviour from zero works
  in this world now.
- **Life changed the climate by itself.** In two of three seeds, eaters evolved to burn a
  charcoal-like solid that erosion brings out of the rock. The carbon gas rose 17 to 37 times and
  the ground warmed by about 7 K. In the third seed, life never found this: plants took the carbon
  gas in, it fell about 160 times, and the climate stayed cool. No climate rule did any of this.
- **Population nodes can now evolve.** A node keeps how much its members differ, splits when they
  differ too much, and joins others only by likeness, never by lineage. Nodes can now use objects,
  through one member at a time.

What did not change: there are still only 1 to 3 kinds at a time, no seasonal travel, and no
shelters or tools beyond a trace. The world is also slower: 18 to 24 s per simulated year, against
10 to 16 before (see "Speed").

## What we built

| # | Review item | Done | What it gave |
|---|---|---|---|
| 1 | Children join nodes by likeness only | yes | Variants of one lineage stay apart. This is also most of the slow-down (more nodes). |
| 2 | A node keeps a spread per gene | yes | Staying children add mutation to it. Merges keep the gap between the two means. |
| 3 | A node splits when its spread is too wide | yes | Selection between the halves. Splits by brain rows are an experiment, off (`--split-rows`): they were 2 times slower for little gain. |
| 4 | Nodes do Bond actions through one member | yes | One member splits off and acts. If it then holds nothing, it goes back at once. |
| 5 | No cap of 6 parts per cell; a part needs 1 kg, not 10 | yes | A 30 kg body can now make parts. |
| 6 | Bigger starting spray; eaters learn at 10^-1; 6 times more row mutation | yes | Starting action biases spread ±0.9 (was ±0.05). This is what lets neutral eaters live. |
| 7 | Eating limited by the area a body works | yes | `phys.sweep` = 1 m² per kg per day. The brake on grazing. |
| 8 | A body costs more to build | yes | `c_copy` 5×10⁵ → 2×10⁶ J/kg. 10⁷ did not fit: an eater cannot release that in one interval. |
| 9 | Catalysts cost more the stronger they are | yes | Cost goes with the square of the speed-up. Cold tolerance has a price now. |
| 10 | Lanes regrew twice too fast (bug); cover keeps bodies warm | yes | Cover gives a 30 kg body +3.4 K, like thick walls. |
| 11 | Microseconds per body update; time the tape | yes | Printed every 10 years. The tape's monthly table is 2.7 times cheaper. |
| 12 | Bodies may decide only once a year (360 days) | yes | No speed gain yet: plants keep deciding every ~20 days (see "Problems"). |
| 13 | Rain steps every 3 days in watch runs | yes | Rivers whole. |
| 14 | Mean catalyst gene per reaction in the yearly series | yes | Columns `cat_0` to `cat_20`. They showed the charcoal burning before the climate did. |
| 15 | Write up the runs and the river numbers | yes | This file, and the section on rivers below. The M0 to M3 reports cannot be added: they were never saved, and nobody has them now. |
| 16 | Watch again: 3 seeds, 300 years, neutral eaters; two comparisons | yes | Below. |

Also from the review's smaller list:

- A brain's cost now goes with the brain, not with the body. A 30 kg eater pays about 13 times
  less per kg for thinking than before; a 1 kg plant member pays the same.
- **The reaction rate is now "first order" in the input that runs out (owner decision, 13 Oct).**
  Before, a rare gas counted twice in its own burning speed, so methane lasted millions of years.
  Now methane stays at about 2×10⁻⁷, and it no longer warms the world.

### Bugs found by the long runs, all fixed

1. Making a part could burn the very solid that was going into the part, leaving a negative store.
2. Big plant nodes took all the light they stored from the ground at one moment: more heat than the
   ground held. Now that light is set aside as it falls.
3. Evaporation counted a body's light twice.
4. A plant lineage learned to make parts all the time. Each holder had to stay its own body, so
   seed 1 reached 1 million parts and 160,000 bodies by year 50. Fix (physics): a body can only
   hold what its own strength can carry. A group brain now learns by members, not by bodies.
5. Liquid flowing through a hot cell left with the wrong heat.
6. The ground's heat capacity was not updated between liquid steps, so cells read hundreds of K too
   hot.
7. Burning bodies could end with negative heat, or run away to 350,000 K. Fix: one heat path for all
   of a body's heat, gases leave with their heat, and **bodies now radiate** like every surface.
   In a 6-year test the hottest body was 974 K.

## The runs

128 × 128 island (256 km), 300 years, seeds 1, 5 and 7. Light-eaters placed in year 2, **eaters in
year 22, both with neutral brains** (action biases sprayed around 0, see the seeding line in each
log). No hand-set behaviour.

Seed 5 stopped at year 215 on bug 7. We fixed it and resumed from its year-210 snapshot with the
fixed code. Its years 0 to 210 were then run again with the same code as before, to rebuild the
series (the resume overwrote it; that tape bug is fixed too).

| At year 300 | seed 1 | seed 5 | seed 7 |
|---|---|---|---|
| Plant members | 2.1e10 | 1.3e11 | 1.3e11 |
| Animal members | 3.9e10 | 1.8e9 | 2.5e10 |
| Animal bodies (nodes + single) | 8,738 | 9,015 | 13,755 |
| Cover on land, median | 2.1 kg/m² | 5.3 kg/m² | 5.2 kg/m² |
| Kinds | 1 (most ever 2) | 3 (most 3) | 1 (most 3) |
| Brain lineages alive / ever | 2 / 21 | 2 / 191 | 2 / 232 |
| Ground temperature (start 288 K) | 287.3 K | 295.4 K | 295.2 K |
| Absorbing gases (start 1.46) | 1.2 | 9.4 | 17.5 |
| Land edges worn | 34% | 2% | 13% |
| Busiest 2% of cells carry | 42% of crossings | 100% (few movers) | 93% |
| Seconds per year (mean) | 23.7 | 16.9 | 17.7 |

Energy drift at year 300 was 3.4×10⁻¹³ or less, elements 5.4×10⁻¹⁵ or less. "Heat not given" (heat a
cell could not give, counted in the books) was 0 in seed 1 and about 10¹⁶ J in seeds 5 and 7. That is
about 1/50,000 of one year's sunlight (see "Problems").

## Pictures

Each picture has five panels: map, temperature, rain, life (green plants, red animals), and traffic.

- `watch-2-before-m4/chart_combined.png`: members, median cover, ground temperature and absorbing
  gases over the 300 years, all three seeds.
- `watch-2-before-m4/seed1_year0030.png`: seed 1, year 30. The plant boom after the eaters came:
  cover rose to about 9 kg/m² by year 33.
- `watch-2-before-m4/seed1_year0300.png`: seed 1, year 300. Lakes and rivers. The cool climate
  stayed (287 K).
- `watch-2-before-m4/seed7_year0090.png`: seed 7, year 90, as the charcoal burning starts (the
  carbon gas has just doubled).
- `watch-2-before-m4/seed7_year0150.png`: seed 7, year 150. The carbon gas has risen 16 times. The
  island is about 6 K warmer than at the start.
- `watch-2-before-m4/seed7_year0300.png` and `seed5_year0300.png`: year 300 of the two warm worlds.

## What emerged, and what did not

| Watch-for item | Seen? | Numbers (seeds 1 / 5 / 7) |
|---|---|---|
| Rivers and lakes | **yes** | 1,985 / 2,071 / 2,843 river cells. Water on land grows all 300 years in seed 1 (261 km³). |
| Climate bands | weak | North-south difference 3 to 4.5 K (26 K at the start). Unchanged: decision 1 of the last report is still open. |
| Climate change | **yes, from life** | Seeds 5 and 7: life burns a charcoal-like solid; carbon gas ×17 and ×37; about +7 K. Seed 1: the carbon gas fell about 160 times instead; the ground stayed at 287 K. See below. |
| Light-eaters spread | **yes** | 76 / 91 / 92% of land at year 300. |
| Plant cover survives the eaters | **yes** (new) | Median cover near 0 for 1 / 2 / 0 years after the eaters came. Before: 40 to 65 years. Plant members still dip (by 70% in seed 5 around year 80) and come back. |
| Neutral eaters live | **yes** (new) | Alive at year 300 in 3 of 3 seeds. Before: dead in 8 of 8 runs. |
| Eaters of eaters | yes in seed 1, weak in 5 and 7 | 10% / 2% / 2% of animal bodies take mostly from animals. 2% / 6% / 14% of animal deaths are "taken". |
| Kinds differ by climate | weak | 1 / 3 / 1 kinds at year 300. Seed 5 had 3 kinds in 19 of its last 100 years; the others never had 3 for long. |
| Populations rise, fall, cycle | rise and fall | Seed 7: 15 swings over 26% in 300 years, but irregular (no rhythm). |
| Extinctions | lineages yes | 90 to 99% of brain lineages died out. |
| Trails | a few routes | Seed 7: the busiest 2% of cells carry 93% of crossings, and the same cells stay busy (79% overlap year to year). Seed 1: 42%. Before: 10 to 38%. |
| Seasonal movement | **no** | Animals move about 1 cell in a year. Seasons are strong at the animals (13 to 17 K). |
| Learned behaviour spreads | partly | `learned` rises to 0.5 to 0.7. Watching reaches up to 96% of groups. |
| Signals settle | weak | 0.01 to 0.04 bit at the end. |
| Giving and exchange | giving yes; exchange a little (seed 1) | Seed 1: up to 1,826 pairs give both ways. |
| Held objects as tools | a trace | Holding a part makes a take 2.5 times bigger, but only 0.01 to 0.1% of takes use one. |
| Shelters in cold | a trace | At most 1.5% of animal body-months in the cold are spent inside. |
| Floating structures | yes, small | Up to 42 / 6 / 12 things afloat; up to 12 moves afloat in a year (seed 1). |

### The climate change, in simple words

Erosion slowly brings a charcoal-like solid (substance 7) out of the rock. Without life it lasts
about 6,400 years in the open. In seeds 5 and 7, eaters evolved a catalyst gene that burns it about
100,000 times faster: it gives a lot of energy, and the catalyst is cheap. In the first run of seed
7 (same physics, before the last bug fixes) we followed it body by body: the burning gene of those
eaters rose from about 0.2 to 8 to 16 decades between years 80 and 200, and the number of such
bodies from about 1,000 to 16,000. The carbon gas has almost no way out of the air in chemistry 0: rock takes it back over about
70 million years. So it piles up and the world warms.

The owner decided to keep this as a result (13 October). It is the first emergent climate change:
life burning buried carbon, with no climate rule. Seed 1's life never found the trick, so its climate
stayed cool. The same laws gave two different histories.

## The comparisons (64 × 64, seed 3, 100 years)

| At year 80 (individuals run stopped there) | nodes (crowd 4) | individuals (crowd 100,000) | one brain per body |
|---|---|---|---|
| Plant members | 3.8e10 | 2.8e10 | 1.5e10 |
| Animal members | 2.1e8 | 1.5e8 | 1.5e8 |
| Cover, median | 0.75 kg/m² | 1.3 kg/m² | 0.37 kg/m² |
| Kinds | 2 | 2 | 2 |
| Ground temperature | 290.2 K | 290.6 K | 289.9 K |
| Brain lineages alive | 3 | 4,164 | 2 |
| Seconds per year at year 80 | about 2.5 | 77 | about 2.1 |

- **Nodes against individuals:** plants and animals are within about 1.5 times of each other, and
  the climate and kinds are the same. The big difference is the number of brain lineages: only
  individuals start new ones. That is the M4 check of plan 7.1, passed roughly, for 80 years on a
  small world. Individuals are 25 times slower, which is why nodes exist. (These comparison runs
  used the code at 7d59c3f, before the last bug fixes.)
- **Shared against own brains:** the same speed now (1.6 s per year each, for 100 years). Patterns
  are close, but the own-brains run had about half the plants. One seed cannot say if this is
  noise.

## Speed

| | Before (last report) | Now |
|---|---|---|
| Seconds per simulated year, mean | 10.5 to 15.7 | 16.9 to 23.7 |
| Body updates per year at year 300 | not measured | about 1.3 million |
| Microseconds per update | not measured | 14 to 21 |
| Bodies' share of the time | 93 to 95% | about 90% |

Why it is slower: nodes now join only by likeness, so a cell often holds the full 4 nodes (item 1).
There is also more life (plants are not grazed away), and rain steps every 3 days (+0.6 s a year).
The review's arithmetic holds: the cost is the number of updates times the cost of one. The goal
(0.6 s a year) needs M4's coarse regions and multi-rate steps.

## Rivers: the A/B of the moisture step (from `Config::air_days`)

Seed 5, 50 years, 128 × 128, no life. Channel cells = dry land with a liquid flux over 9,000 mol/s.
Air / exchange / flow steps in days, and the extra cost per simulated year against 15/15/15:

| Steps | Channel cells | Extra ms per year |
|---|---|---|
| 15 / 15 / 15 | 240 | 0 |
| 3 / 9 / 15 | 484 | +120 |
| 3 / 6 / 15 | 776 | +185 |
| 3 / 3 / 15 (watch runs now) | 1,405 | +400 |
| 3 / 3 / 3 | 1,369 | +850 |

## Code size

Core is 6,406 lines (goal: 5,000 through M3, 8,000 through M7). That is 28% over the M3 goal. It grew
by about 460 lines since the review: the population nodes (M4 work, done early because they carry
every result), and the new physics and bug fixes.
Whole repository: about 8,400 lines of Rust plus 1,137 lines of tests.

## Current problems

1. **Few kinds.** Nodes can now evolve and split, but at most 3 kinds live at once (seed 5 at the
   end; the others never for long).
2. **No seasonal travel.** Animals live where it stays warm. The area limit on eating did not make
   them move with the seasons.
3. **Tools and shelters are only a trace.** Holding a part pays (takes are 2.5 times bigger), but it
   is rare. Shelters are almost never used.
4. **Parts and nodes.** A member holding a part must stay its own body. One plant lineage with a
   strong "make a part" habit can still create many thousands of bodies (seed 1 had up to 1,814
   parts at year 45, then fewer). The real fix is M4 design: a node that stands for many members
   holding the same kind of part.
5. **Speed got worse** (see above). Plants still decide every ~20 days: the decision-interval gene
   moves too slowly to reach longer intervals by itself. Starting plants at ~160 days would cut
   plant updates 4.5 times, but it is a hand-set starting choice (decision 2 below).
6. **Small heat shortfall in seeds 5 and 7** (about 10¹⁶ J, counted). It comes from a big node
   paying its costs into one cell's thin ground layer at once. Energy stays exact. It still needs a
   fix.
7. **Very hot bodies still exist** (up to about 970 K) where big nodes burn ground solids. Physics
   now holds them (radiation), but they are odd. 974 K was the hottest in a 6-year test.
8. **Old snapshots** load only when new fields were added at the end of the saved data.

## Decisions for the owner

1. **Go to M4?** Our advice: yes. The two biggest gaps of the review (plants wiped out, neutral
   eaters die) are closed. Kinds, travel and shelter did not come, and the review said to go anyway
   if that happens. M4's first job: parts held by nodes (problem 4), then coarse regions and
   multi-rate steps for speed.
2. **Plant decision interval.** Keep plants starting at 20 days (honest, slow), or start them at
   ~160 days (4.5 times fewer plant updates, a hand-set starting condition we would report)?
3. **Climate bands** (from the last report, still open): a narrower latitude range, or less air
   mixing?
4. **Carbon gas:** kept as a result (decided 13 Oct). No change.

## Numbers in detail

`watch-2-before-m4/analysis.md` has tables by decade per seed and every watch item with numbers. The
tapes are not in the repository (about 3 GB each). To make them again:
`ew run --seed N --years 300 --every 10 --frames-every 5 --out DIR` (code 187d8de; seed 5 resumed
at year 210 with deb620e: `ew resume --from DIR/snap_00210.bin.gz --years 90 --every 10
--frames-every 5 --out DIR`).

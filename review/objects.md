# Objects: what they are today, and how they can scale

For the owner. You asked two things. Why do shelters appear when no brain is clever? And how can we keep trillions of
objects once a civilisation comes? This note answers with numbers from two runs, then gives options and a plan.

## 1. Facts today

I ran seed 1, `ew run --size 64 --roster --years 300 --every 50 --snapshots-every 100`, once as it is ("real") and
once with `--null` (no learning, rows frozen), then the real run again from year 200 for 40 years with `--months`.
Numbers come from `series.csv`, `months.csv`, the snapshots of years 100, 200 and 300 (read by a small probe outside
the repository) and `ew-analyze`. The island has 1601 land cells, about 6,400 km² of land.

| | Real | Null |
|---|---|---|
| Animal kinds alive at year 300 (hidden units of the brain) | 4 (12, 12, 12, 16) | 1 (12) |
| Bodies / members at year 300 | 3,717 / 2.7e7 | 39 / 5.3e4 |
| "Form" drawn per body update, years 20 to 100 / 200 to 300 | 2.2% / 0.8% | 0.6% / 0.9% |
| Score of "form", by members, now against born (year 300) | small eaters of remains -2.1 vs -2.4; big ones -4.6 vs -2.6 | frozen: -2.5 |
| Parts formed a year (year 100 / year 300) | 486 / 51 | 33 / 4 |
| Parts at year 100 / 200 / 300 | 3,957 / 4,953 / 4,121 | 357 / 128 / 106 |
| Mean life of a part (parts over formed, years 200 to 300) | about 14 years | about 18 years |
| Mass of all parts; median part; largest (year 300) | 182 t; 1.9 kg; 16.6 t | 2.1 t; 1.9 kg; 1.4 t |
| Walls that are rock (substances 7, 8, 9), year 300 | 100% | 99.6% |
| Parts held (year 300) | 152 (147 by members of 0.6 kg) | 6 |
| Structures of 2 parts or more | 68 | 35 |
| Structures at least half closed ("rooms") | 2,868 | 12 |
| Rooms with a body inside | 34 | 3 |
| Parts holding liquid | 960 | 41 |
| Share of bodies inside, coldest month / warmest month (years 200 to 240, `months.csv`) | 0.94% / 0.94% (correlation with temperature 0.005) | not run |
| Time of parts / time of a year (years 200 to 300) | 58 ms / 3.3 s | 7 ms / 0.8 s |
| Books: element drift / energy drift | 1.7e-15 / 5.6e-12 | 1.1e-15 / 5.6e-12 |

What the numbers say, in short:

- **Who makes parts.** Every animal kind has a brain: 12 hidden units (16 for the two eaters of eaters), 6 memory
  numbers, sight of one cell. "Form" is one of the five fixed actions. The founders start with a hand-set score of
  -2.5 for it. With the other starting scores this gives a chance of about 1.4% per decision. The real run forms at
  about 2% of updates in years 20 to 100 and 0.8% in years 200 to 300; the null run, whose brains do not learn, forms
  at 0.6% to 0.9%. So a part is drawn by chance, at about the rate the start set. Learning moves the score a little:
  the big eaters of remains push it down (-2.6 to -4.6), the small ones keep it (-2.4 to -2.1). No kind learned to
  make more parts. Forming is cheap: its energy is tiny next to upkeep, and its real price is one step without eating
  (an act fills the step).
- **Small bodies make parts too.** One part takes at least 1 kg (`min_kg`) and at most a tenth of the body's mass
  (`form`). A node is one body: the members that draw "form" make *one* part of a tenth of all their mass. So 30
  members of 0.6 kg make one stone of 2 kg, and a big node made one shell of 16.6 t. At year 300 the smallest kind
  (0.6 kg members) holds 147 of the 152 held parts.
- **What they are made of.** Rock. The walls are 50% substance 7, 44% substance 8 and 6% substance 9 (the ground's
  loose solids). Body matter and fuel are under 2% at year 100 and about 0 at year 300: those rot, rock does not.
- **How many and why.** About 4,000 to 7,000 parts at any time, 0.6 to 1 per km² (fewer than the 2e4 to 3e5 that
  `review/plan.md` lists as an open item). 300 to 750 are made a year and about as many end. Rock does not rot, so a
  part ends when it sinks into the ground (5 mm a year, `bury`) or when a body takes its walls. A part lives about 14
  years on the mean. A bonded structure never sinks, and a big part needs centuries: the 16.6 t part made before year
  100 is still there at year 300.
- **Rooms.** A hollow part closes its own hollow. So most "rooms" are single hollow stones: 2,868 of about 4,050
  structures at year 300 close at least half their space. Only 68 structures have more than one part (the null run,
  with 1% of the bodies, has 35: joined parts never sink, so the few made early stay). `ew-analyze` finds 28 kinds of
  structure in 3,948 copies; the most common is a single rock shell, 88% hollow, 17 kg (674 copies); the largest kinds
  have 2 parts. 34 rooms have a body inside. The tape's `inside_cold` (6.3%) against `inside_warm` (1.0%) looks like
  shelter, but it is not: in `months.csv` the share inside is 0.94% in the coldest month and in the warmest. Bodies on
  cold ground live where shells lie; they do not go in when it gets cold (as `docs/reports/fine.md` found). Nothing is
  built or used on purpose.
- **Cost.** Parts take 33 to 72 ms of a year of 1.7 to 4.3 s (2% to 3%), about 13 µs a part a year on 2 threads. A
  part is 384 bytes (160 bytes and two lists of 14 substances).

**Why shelters appear without a clever brain.** The laws allow it, and chance does it. A body that draws "form" picks
up rock and shapes it; the brain's shape output, with noise (`shape_noise`), sets how hollow it is. A hollow rock
shell has a low conductance to the outside, so by Bond's containment law it *is* a room. Rain fills hollows, so they
"hold liquid". These are physics possibilities reached by chance acts, not by thought.

**Is that a problem?** Not for the physics. Animals without much brain make things in the real world too (nests,
burrows, mounds), and the law must allow that. It is a problem in three other ways:

1. It is noise, not emergence. The parts are made at a rate a person chose (the starting score), and they last
   because rock lasts. The watch-for list can mistake them for shelters and containers (`docs/reports/m3.md` did).
2. They cost compute for nothing. Each one that a node's members form or pick up also splits those members off as a
   body of their own, and at fine grain they raise a cell's interest score (`fine.md`, section 3).
3. They hide the real question: no brain today can sense, remember and learn enough to *use* a part (`fine.md`,
   section 6). Until the Choose round fixes that, more object code builds machinery before results (spec 2, lesson 4).

## 2. The scaling problem

The goal (spec 1) is settlements, cities, trade, metals and machines. A house made of Bond's parts is many parts: a
wall of bricks or beams is hundreds to thousands of parts with their bonds.

| Stage (a rough Earth match) | Beings on the planet | Objects each | Objects in all | Objects per km² of town |
|---|---|---|---|---|
| first tools and huts | 1e6 to 1e7 | 10 to 100 | 1e7 to 1e9 | 1e3 to 1e5 |
| towns, metals, trade | about 1e8 | 1e2 to 1e3 (a house of 1e3 to 1e4 parts, tools, pots, stores) | 1e10 to 1e11 | 1e6 to 1e7 |
| machines | about 1e9 | 1e3 to 1e4 | 1e12 to 1e13 | 1e7 to 1e8 |

(Rough guesses for scale only. For today, human-made mass is about 1.1e15 kg, more than all living mass: Elhacham et
al. 2020.)

At today's 384 bytes and 13 µs a part a year, 1e10 objects need 4 TB of memory and about 36 hours for one year of
the world. The target is 0.6 s a year. **One record per object cannot work**: it is 5 orders of magnitude too slow
before a single town of the machine stage.

The bodies already solved this. A population node holds `n` alike members as one record: at year 300 the real run
holds 2.7e7 members in 3,717 bodies (7,000 members a record). Parts have no such tool: 4,121 parts are 4,121 records.
A node of a million members can also hold only *one* part, which is odd physics (the spec 7.1 rule is not applied to
objects).

## 3. Options

| | a. Objects as lots | b. Objects as a field | c. Quiet start | d. Forget untouched litter |
|---|---|---|---|---|
| The idea | one record of `n` alike parts (same design, same cell); one part is split out only where a body holds it, acts on it, or the cell is fine | per cell, a stock of made matter in a few classes (loose made solids; walls by area and conductance; edges by mass and thickness); parts made up only for the viewer | the founders start with a low score for "form" (an initial condition, like all of `worldgen::life::kinds`); learning and selection may raise it | a lone part no body touched for some years goes back into the ground's loose solids, its shape lost (a collapse, spec 5.2) |
| What scales | records = designs × cells, bounded by a cap per cell, as `MAX_NODES` bounds bodies; so the cost follows the budget of cells, not the objects | records = cells × classes | nothing; it only stops the chance litter | records = parts in use |
| Five laws and the books | kept; amounts are `n` × one part, split and join by shares as `nodes::Lot`; rot is already one share per cell, so exact; burial needs a share per month or a depth band per lot | the books kept on the stock; Bond loses its graph: no structure that moves as one, no carrying, no floating, no rotating joint (spec 6.3 geometry v2, v3: wheels, sails) | all kept | kept, but it changes the world: a body could have found that shell later |
| What we lose | the history of each alike object (as for members of a node); exact sizes inside a band | the bond-graph pattern finder (spec 9.1), designs, machines. The classes are results ("wall", "tool") written into the law: lesson 1 of spec 2 | nothing of the law | detail where nobody looks; one more speed tool with a side effect |
| Code | adds about 300 to 500 lines (a count on `Part`, merge and split, holding by nodes); moves the design hash from `analyze/` into `bond` (one place for the idea) | deletes most of `bond/` (1,174 lines) and about 200 more; adds about 300 | one number in `worldgen`; no law changes | about 30 lines |

**On c, "pause making for small brains".** I looked for a cost a small brain cannot pay. I found none that is not a
rule in disguise. A gate by brain size would be a named threshold, and nests and burrows show small brains can make
things. Today forming already costs a step of eating (an act fills the step) and its energy. The honest small option is
the initial condition: start quiet, and let a kind earn the act. If making pays, selection raises the score.

**e. Designs as the unit of copy (spec 6.4).** Option a already makes the design the unit of *storage*. It must not
become the unit of *copy*: the spec says structures are not copied; bodies that learned to build build again. The
knowledge stays in the brains (the shape outputs and the act scores). A design key is only how alike objects find
each other, as a lineage is for bodies.

## 4. Recommendation

1. **Now: option c.** Start the founders with a low score for "form" and count, in the tape, who forms (kind, brain
   size, and whether "form" was the top-scored act when drawn). No law changes. The litter goes away, and any part
   that appears after that is a signal.
2. **Next: the Choose round** (perception, memory, learning a season: `fine.md` section 7). Objects matter only when a
   body can use them. No new object code before a kind uses parts in a way that beats the null run.
3. **Then: option a, lots.** It is the spec's own rule for bodies (7.1) applied to objects. It keeps the five laws,
   the books, the bond graph and the pattern finder, and it bounds the cost by the budget of cells.
4. **Never option b as the law.** A field of classes is the right picture for drawing the far planet (the viewer can
   sum lots into a few classes per cell), but as a law it writes results into the world.

| WP | Goal | Done when |
|---|---|---|
| O1 | Quiet start and who-forms counters (tape) | the gate (CLAUDE.md 6.3); 300 roster years real and null: parts formed per update below 1e-3 in both; the other watch-fors within noise of today |
| O2 | The design key in `bond` (bands and the bond-graph hash, moved from `analyze/structures.rs`) | `ew-analyze` gives the same cards, bit for bit |
| O3 | Lots of lone parts: a count on `Part`; alike lone parts of a cell join monthly, one is split out to be held or acted on; burial by a share (a law change, its own commit) | the books within 1e-9 across join and split (a test with `worldgen::testkit`); records at most designs × cells; 300 years with the watch-fors within noise of O1 |
| O4 | Nodes hold lots: a node of `n` members holds up to `n` alike parts, one each; nodes merge by the design they hold | a scene: a node of 100 with 100 edges bites as 100 individuals do, within noise |
| O5 | A cap of lots per cell (join the nearest designs), as `MAX_NODES` | a world with 1e6 parts placed by an edit steps within 10% of one with 1e3 |
| O6 | The viewer makes single parts from lots for its window (`tape::things`), draws keyed by the lot's id | the same window gives the same picture twice |
| O7 | Lots of bonded structures (one bond list per design) | only once a kind builds structures of 3 parts or more that last (a watch report) |

**What you must decide**

1. Start quiet (O1)? It makes the next watch report honest, but the island will show almost no parts.
2. Accept that alike objects lose their single history, as members of a node do?
3. The order: O1 now, then Choose, then O2 to O6. Or O2 to O6 first, before any kind wants parts. I recommend the first:
   it is the lesson of spec 2 (results before machinery).

**What the owner decided**

1. Start quiet (O1): yes.
2. Alike objects lose their single history, as members of a node do: yes.
3. Order: O1 now, and O2 to O6 now too, without waiting for a kind that uses parts: the object code must become
   simple and cheap before it is needed. O7 waits for structures that last.

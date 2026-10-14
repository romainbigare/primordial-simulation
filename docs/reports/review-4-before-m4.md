# Review 4: math, physics, chemistry and world concepts, after the third fix list

14 October 2026. Code: `main` at ad9c145 (watch report 4 at 444dabf plus that report). I read all of
`core`, `worldgen`, `chem0.toml`, the tape's measures, the handover and watch reports 3 and 4 with
their full numbers. **I ran nothing.** Every number below is from the reports or from arithmetic on
the code's constants, and the arithmetic is written out so you can check it.

## Short answer

**The physics is sound and the world is stable. You can go to M4.** But first fix one bug, one
measure and one thing in the reports (the list at the end, rows 1 to 3; about two days). They are
not speed work: they decide whether M4 can give what you want from it.

The honest state of the world:

- **Alive: yes.** Plants cover the land, 10¹⁰ eaters live on them, there is a food chain
  (plants, plant-eaters, eaters of dead matter, a little predation), the carbon gas sits at
  300 to 317 parts per million in all three seeds, the energy and element books balance to
  10⁻¹³, and nothing runs away any more (no giants, no fires, no gene drift).
- **Normal creatures: not yet.** A member lives about **one month** on average (plants and
  animals alike), not the 5 to 25 years the "expected life" columns say. The animals are 4 to
  17% of the plants by count (on Earth animals are about 0.5% of plants by mass). The plants are
  still an algae-like soup that turns over 12 to 14 times a year; there are no trees, and there
  cannot be yet (finding 4). The biggest animals are 9 to 15 tonne eaters that barely copy.
  There are 2 to 3 kinds.
- **A humanoid-like, tool-using, intelligent kind: no, and it cannot appear from this world as
  it is.** Three reasons, in order: brain size cannot evolve inside a population node (a bug,
  finding 1); nothing on this island pays for a tool, a shelter or a bigger brain (finding 6);
  and the brain has no planning (handover 6.5 said "add at M2 or M3"; it is not built). The
  first is a two-day fix. The second comes with M4's planet (cold, seasons, climate bands). The
  third is the owner's check-point decision 2 (a fixed time on Choose).

**Speed:** 16 to 27 s per simulated year at 128 x 128 is where the design says it should be
before M4. I found no single change that gives 10 times. The biggest honest gains are: let the
brain-size bug fix do its job (plants can then drop their brains: 25 to 35% of a year, the gain
review 3 already counted on), then a few cheap trims (a few per cent each). The real answer is
M4's coarse regions and monthly nodes, as planned.

## What is right and should stay

- The three seeds now agree with each other: ground 290.6 to 292.6 K, carbon gas 300 to 317
  ppm, eaters 8 x 10⁹ to 2 x 10¹⁰. Before, one seed lost its carbon and another warmed by a gene
  artifact. A world that gives the same climate from three starts is a world whose laws, not its
  noise, set the climate.
- The copy fix (a child costs what it is built of) removed the liquid giants, and the copy-mass
  gene's wall at 10⁴ kg no longer hides a selection for size (see finding 5 for what it shows now).
- Ageing as built is a law, not a rule: wear by energy turned over, repair as a gene with a cost,
  the same mean life in a node and in individuals. Note for the record: "rate of living" is an
  old idea that biology no longer holds as a cause, but as a game law it is fine, and the owner
  chose it.
- The enzyme cost is now real for everyone (an eater's catalysts cost a tenth of its upkeep;
  living 9 K colder costs 14% more). My arithmetic agrees with the handover's: burn gene 2.6 gives
  10^2.6 x 4,300 x e^(-7,760/290) = 4.1 x 10⁻⁶ /s, times `c_cat` 1.5 x 10⁹ = 6,200 J/kg/day.
- The conservation books: elements 4 to 10 x 10⁻¹⁵, energy 1.4 to 1.5 x 10⁻¹³ over 300 years.
- The null runs, the roster, the seeding line, the per-root columns. These made this review
  possible without running anything.

## Checks on realism (from the reports' numbers)

| What | Now | Earth-like? | Where it comes from |
|---|---|---|---|
| Ground temperature | 290.6 to 292.6 K at 32° latitude | yes (Earth at 32°: about 291 K) | the greenhouse layers, the vapour feedback |
| Carbon gas | 300 to 317 ppm, steady | yes | plants, dead matter, eaters; the sea holds 4% of the air's |
| Carbon cycle | closed: air, plants, dead sugar, eaters; no source, a geological sink of 10⁻⁸ a year | fine for an island over 300 years; a planet over 10,000 years needs a source (volcanism) and a bigger store (sea chemistry) | chemistry 0 reactions 0, 1, 9, 18 |
| Plant growth per kg | 2.8 g of sugar per kg per day at 300 ppm (gas gene 2.5: 316 mol of air per kg per day, 0.095 mol of carbon gas) | a real leaf fixes about 24 g of carbon gas per m² per day, so a 2 m²/kg plant would make about 33 g/kg/day: **the plants breathe 10 times less than a leaf** | `body::live`, the GAS gene (finding 4) |
| Light used by plants | 0.14% of the light they catch is stored (44 kJ of 35 MJ per kg per day) | real plants store 1 to 2% | light is 40 times in excess; the plants are starved of carbon gas at the body, not of light (finding 4) |
| Eater metabolism | 30 kg, 64 kJ/kg/day = 22 W | a 30 kg mammal rests at about 45 W; fine | `c_strength` |
| Eater breathing | 4 mol of air per kg per day | a mammal: about 16; fine | GAS gene 0.6 |
| Brain cost | 1.2% of a 30 kg eater's upkeep at 12 hidden units; 5% at 64 | real brains 2 to 20% of resting cost; fine, cheap enough that a big brain is affordable | `Lineage::cost`, `Physics::think` |
| Mean life of a member | **about one month** (deaths 12 to 14 times the standing members a year; see finding 2) | no: this is plankton, not grass and deer | copying when full, starvation |
| Expected life if nothing else kills | 5 to 25 years (eaters), 5 to 14 (plants) | the right order, but it is never reached | the REPAIR gene |
| Animals against plants, by members | 4 to 17% | Earth by mass: about 0.5% | plants turn over 14 times a year, so a thin crop feeds a fat herd (finding 2) |
| Biggest animal | 26 to 34 t members, copy size 9 to 15 t | no land animal is this big; size is nearly free here (finding 5) | nothing scales against size |
| Kinds | 2 to 3 (4 to 6 with the roster, by survival) | few | one climate, the merge, and a measure that cannot count past 6 |
| Predation | up to 4% of what eaters eat | low but real | strong eaters of eaters die out in the roster runs |
| Trees, forest | none | | finding 4 |
| Tools, shelters | none; the null run makes more parts than the real run | | finding 6 |
| Seasons | not reported | | the tilt is 23.4° at 32°: there should be a swing of about 10 K on land; nobody has measured it |

## Findings, biggest first

### 1. Brain size cannot evolve inside a population node (a bug)

Review 3 (M4 there) said brain size could not evolve in nodes, and the fix was `copy::resize`: a
node whose *mean* brain-size gene moves a whole unit from its lineage's size moves to a new
lineage. The test for it passes (`nodes.rs`, `a_node_whose_size_genes_moved_changes_lineage`).
But the mean can never move, because nothing in a node selects on these genes:

- `copy::step` (`copy.rs:105`) counts the four size genes (`HIDDEN..=SIGHT`) as 0 in every
  likeness test, and `spread_steps` as 0. So they never make two nodes unlike.
- `copy::split` (`copy.rs:491`) gives them a gap of 0, so **a node never splits along them**.
  Selection in nodes works only between the two halves of a split. No split, no selection.
- `copy::vary` keeps the mean exactly (that was review 3's M1 fix, and it is right).
- A merge moves the mean toward the other node's mean, but every node started at 12.

So inside a node the brain-size genes are frozen at their starting value for ever. The data
agree: `hidden` is 12 or 11.9 in all three seeds for 300 years, and in all six null and real
runs at 64 x 64. The only way a brain size can change is an individual's child (`copy`, the
`!node` branch), and the eaters are nodes almost everywhere (10¹⁰ members in 15,000 bodies).

Why it matters, three ways:

- **Intelligence.** The one gene that makes a bigger brain possible cannot be selected where
  the animals live. Nothing the world does can reward a brain while this holds.
- **Speed.** Review 3's Q2 and its item 10 (plants drop their brain, saving 40% of their
  upkeep and 25 to 35% of a year) depend on the hidden gene walking from 12 to 0 by selection.
  It cannot walk. That planned speed gain is dead until this is fixed.
- **The M4 check.** "Merged nodes give the same patterns as individuals" fails on this gene by
  construction.

**Fix (small, `copy.rs`).** Let a node split along a size gene too: give `HIDDEN..=SIGHT` their
step in `step` for the split's gap (not for likeness: the note in `alike` is right that a merge
carries a brain over to other sizes, so two nodes of different sizes may still merge). Then a
node held at the spread cap splits along HIDDEN now and then, the two halves differ by about a
third of a unit (step 0.3, `most` is about 2.5 steps), selection acts between them for a month,
and the one that fits grows. When a half's mean reaches a whole unit from its lineage, `resize`
does what it was built for. Keep the test with no selection (the mean must still stay put) and
add one with a cost difference: a node whose members pay for 12 hidden units they do not use
should reach a smaller brain within a few hundred updates.

### 2. A member lives about one month, whatever its genes say

Watch report 4 prints `died_age` and its share of all deaths. From those two numbers:

| seed | deaths by age a year | share | all deaths a year | standing members | deaths per member a year | mean life |
|---|---|---|---|---|---|---|
| 1 | 1.17 x 10¹⁰ | 0.8% | 1.5 x 10¹² | 1.1 x 10¹¹ | 13 | about 27 days |
| 5 | 1.57 x 10¹¹ | 2.1% | 7.5 x 10¹² | 5.3 x 10¹¹ | 14 | about 26 days |
| 7 | 1.35 x 10¹⁰ | 1.0% | 1.35 x 10¹² | 1.1 x 10¹¹ | 12 | about 30 days |

(The share is rounded in the report; the result is 12 to 14 either way.) Plants are 85 to 96% of
the members, so this is mostly plants, but the eaters cannot be far off: their expected life
fell to 1 to 2 years in the roster runs *because* almost none reach it (the report's own reading,
which I share). The chart's "expected life of eaters, 5 to 25 years" is what the REPAIR gene
would give if nothing else killed; it is not a life anyone lives.

What this means: the ecology is a **boom and starve churn**. A member is born, holds 5 to 50%
of its parent's stores (the CHILD gene), and most die of hunger (`SPENT`, `LACK`) or are eaten
within weeks. The plants, by my arithmetic, cannot even double in a month (finding 4: 2.8 g of
sugar per kg per day), so to turn over 13 times a year the members must be small or the children
tiny; the report does not say what a member weighs. Review 3's W1 ("plants are a soup, not
plants") still stands with the shade law in place, for the reason in finding 4.

This is not a bug, and it is not unphysical: plankton and bacteria live like this. But it is not
"normal creatures", and it hides every long-life result: a lifespan, a generation time, a culture
that lasts, a memory of individuals (KNOWN holds 4 bodies for a life of a month) all need lives of
years. The ageing law did not cause it (age is 1 to 2% of deaths); starvation did.

**Do now (reports only):** print, per starting kind (`root_k`), the deaths a year by cause, the
births, the mean member mass and copy mass, and the realized mean life (members over deaths a
year). The columns `died_*` and `a_*` exist; they are not per kind and were not in report 4. Until
they are, no report can say what the animals of this world are like.

**Then decide (M4):** whether a world of month-long lives is what you want. If not, the causes
are in Copy and Choose, not in a new law: copying when full happens whatever the food outlook
(physics, and right), but a child born with 5% of its parent's stores into a cell with no spare
food is a death, not a birth. The CHILD gene's floor (0.05) and the `full` factor (2) are free
numbers; the brain's copy logit (kept from the hand-set start) sets the pace of generations
(`life.rs`, "the old value keeps the pace of generations"). Any of these moves the churn.

### 3. The `cat_r` columns mix the kinds, and the "light gene fell" story is an artifact

Report 4 reads `cat_0` falling from 9.0 to 7.7 (seed 1), 8.6 (seed 5), 8.3 (seed 7), and report 3
read the same column ("the plants' light gene goes down a little, because a catalyst now has a
real price"). `cat_r` is the mean over **all** live members, with an off gene counted as 0
(handover 8.3), and the eaters' light gene is off. Their share of the members explains the whole
fall:

| seed | eaters / all members at year 300 | 9.0 x (1 - share) | `cat_0` in the report |
|---|---|---|---|
| 1 | 1.62 x 10¹⁰ / 1.09 x 10¹¹ = 14.9% | 7.68 | 7.73 |
| 5 | 1.91 x 10¹⁰ / 5.34 x 10¹¹ = 3.6% | 8.69 | 8.59 |
| 7 | 8.3 x 10⁹ / 1.13 x 10¹¹ = 7.3% | 8.35 | 8.34 |

So the plants' light gene has **not moved** from 9 in 300 years. That is the right result (they
need it: at gene 9 they fix 97% of the carbon gas they breathe in an update, `rate x dt` = 3.5;
at 7.7 they would fix 16% and starve), and it is good news about the enzyme cost. But the report
drew the opposite conclusion from a mixed column, and the same mixing sits in `cat_1` (the burn
gene: plants at 2.0, eaters at 2.6 and rising, mean 2.2) and in every `cat_r` of the roster runs.

**Fix (tape only):** print the catalyst means per starting kind (`root_k_cat_r`, or at least
per light-eater and per other), or weight the mean by the members that carry the gene. One
afternoon. Without it, every gene story in the next reports is suspect in the same way.

### 4. The plants are starved of carbon gas at the body, so height cannot pay and there can be no trees

Review 3 (P3) said size never paid because every body got the same light. Report 3 built light by
height (`body::shade`). Nothing changed (members stayed about 2 kg), and here is why.

A plant member breathes `GAS` x mass x days of air (`body::live`, `reach_mol`). The starting gas
gene is 2.5: 316 mol of air per kg per day. At 300 ppm that is 0.095 mol of carbon gas per kg per
day: **2.8 g of sugar per kg per day**, 44 kJ. Its light: 2 m²/kg (LEAF gene 0.3) under about
200 W/m² is 35 MJ per kg per day, of which the light reaction could store 5% (`LIGHT_YIELD`),
1.7 MJ. So light is **40 times in excess** of what the carbon gas allows; the plant stores 0.14%
of the light it catches, and a real leaf stores 1 to 2%. A real leaf also takes about 24 g of
carbon gas per m² per day, so a 2 m²/kg plant would fix about 33 g of sugar per kg per day, 12
times what these plants do: **the plants breathe about ten times less than a leaf.**

While the carbon gas is the limit, shade is nothing: a shaded plant loses light it was not using.
So there is no reason to be tall, size pays only through lower upkeep per kg, and the forest of
plan 10.3 cannot appear. This also sets the world's whole productivity, and so its churn (finding
2) and its animal count. The gene can evolve up (its range reaches 5.0), and selection should
push it; whether it did in 300 years no report says (GAS is not in the series).

**Fix, in two steps.** First (reports): print the per-kind means of GAS, LEAF, COPY_MASS and
INTERVAL; if GAS has climbed to 3.5 or more, light is already co-limiting and the second step
can wait. Second (a small law change, `body::live` and `plan`): a body exchanges gas through the
surface it has open to the air, its capture area plus its skin (6 (m/ρ)^(2/3) per member), and
the GAS gene becomes mol per m² of that per day. That is the physics (a leaf breathes through the
leaf; a lung is a surface too, the gene absorbs it), it costs nothing new, and it makes capture
area pay twice (light and gas), so the trade-off that makes a tree (more area, more height, more
structure to hold it) exists. Re-set the roster's GAS numbers so each kind breathes what it does
now (a 1 kg plant with 2 m²: gene 2.2 per m²; a 30 kg eater with 0.6 m² of skin: gene 2.3). Run
seed 1 at 64 x 64 for 60 years and look at the member mass and the shade.

### 5. Size is nearly free, so the copy-mass gene runs to its wall

Report 3 moved the copy-mass wall from 10⁴ to 10⁶ kg; now the eaters sit at 9 to 15 t (copy
size) and 26 to 34 t (heaviest member), and "barely copy". In the laws as written, almost
everything a body does goes with its mass (what it takes: `take` x mass; what it sweeps: `sweep`
x mass; what it breathes: GAS x mass; its force: strength x mass), and its costs go with its mass
too (strength, catalysts, per kg), while its brain costs m^0.25 and its heat loss m^(2/3). So a
bigger body pays less per kg for brain and heat and loses nothing anywhere: the only thing that
scales against size is nothing. In life, intake, metabolism and locomotion go with about m^0.75,
structure (bone) with m^1.3, and reproduction time grows with size; those set a largest size.

This is not wrong physics, it is missing physics, and it is not urgent: the giants are bounded now
(a member at twice its copy size, not 30 times). But every time a wall moves, selection will go
to it, and 10 t "eaters" that cannot copy are not a food chain. **Suggested (M4, one exponent):**
let what a member takes and sweeps go with its mass to the power 0.75 (`Body::work` is the one
place), so a 10 t member takes 180 times what a 10 kg one does, not 1,000. Measure before and
after on seed 1 at 64 x 64. Keep a note that this is a free number (the exponent) chosen for a
reason, not a rule.

### 6. Nothing on this island pays for a tool, a shelter or a brain

The handover's test for a humanoid-like kind is physics first: can the laws reward it? Today:

- **Tools.** A held hard part multiplies force (`bond::lever`): a 30 kg body holding a 10 kg hard
  part has 2.7 times the force. Force matters in two places: taking from a body (predation) and
  breaking bonded ground (`dig`). Predation is 0 to 4% of what eaters eat, and dug ground (rock)
  is food for nobody. So a tool buys almost nothing.
- **Shelters.** Walls cut heat loss (`shelter`, `walls_around`). The island spans 7 to 10 K over
  most of its land, the ground sits at 291 K, and the seasonal swing has not been measured. A
  body that is never cold gains nothing from walls. The null run makes 200 parts a year by chance
  and the real run learns to stop (0.02): learning says, correctly, that parts are a waste here.
- **Brains.** A bigger brain costs little (table above) but there is nothing to learn beyond
  "take from the ground, take from a plant" that a 12-unit brain cannot learn, and the brain-size
  gene is frozen anyway (finding 1).
- **Planning.** None. `choose::act` scores the next action only. Tool chains (make, hold, use)
  and shelters (build before the cold) need look-ahead or long memory. Handover 6.5 placed it at
  M2 or M3 and handover 13.1's option 2 is "a fixed time on Choose". The `--clever-year`
  experiment (a 48-unit brain placed as a kind) exists in the code, and reports 3 and 4 do not
  show its result. Run it once; it is the cheapest test of "does a big brain pay here".

So a humanoid-like kind is not a tuning away. The order that makes it possible: fix finding 1
now; M4's planet gives cold, seasons and climate bands (the reason for shelters, stores and
travel); then the check-point's option 2 (planning and longer memory) is the research step, and
it may fail (handover 15 names it the main risk). Say this plainly in the next report, so nobody
expects a people-like kind from M4 itself.

### 7. Smaller findings

- **The `kinds` measure cannot count past 6** (`MAX_K`) and needs kinds 3 radii apart (`SEP`);
  the roster run of seed 5 has six starting kinds alive and `kinds` says 1. Report the per-root
  survival (as report 4 did) as the kinds number for roster runs, and say the measure is a floor.
- **Seasons are not measured.** Print the summer and winter mean ground temperature on land and
  the swing. Seasonal travel, stores and shelters all need a season; nobody knows how big it is.
- **The carbon cycle has no source.** Reaction 18 (rock weathering) takes carbon gas out at about
  10⁻⁸ a year; nothing puts it back. Fine for 300 years on an island; a 10,000-year planet needs
  a source (the handover's "volcano" god action is the same physics) and a bigger store (sea
  chemistry, which chemistry 0 has no reaction for). Note it for M4, do not build it now.
- **Methane's life is short and temperature-bound.** Reaction 8 at 280 K: 5 x 10⁶ x e^(-28.6) x
  0.21² = 9 x 10⁻⁸ /s, a life of about 4 months; at 255 K about 6 years. The air's mean
  temperature decides which. Harmless, but if a warm world ever has a methane story, this is
  where to look.
- **A node that draws a Bond action splits one member off and takes it straight back**
  (`choose::act`, `copy::one`, `take_in`). A neutral plant brain draws one of the four Bond
  actions in about 4 of its 25 options, so about 16% of plant decisions pay for a split, a
  `reborn`, an `add` into the update ring and a merge, for nothing. Speed only (about a second
  a year); it goes away when plants are brainless.
- **The eaters' burn gene rises** (`cat_1` 2.09 to 2.23 in seed 1 with the plants' share fixed:
  the eaters went from 2.6 to about 3.5). That is selection for faster digestion against the
  enzyme cost. Good; print it per kind to be sure.
- **Lives: the roster's "small fast eaters of light-eaters" die in both seeds, the big slow
  ones too.** The report's reason for the big ones (a neutral brain decides too rarely to find
  food before its 30% store runs out) is right and is a seeding choice, not a law; give the big
  slow kinds a larger starting store or a shorter first interval, and say so in the seeding line.

## Physics and chemistry: what I checked and found right

Each line is one check I did on the code's constants.

- Insolation: `flow::air::insolation` is the daily-mean formula (S₀/π)(h₀ sin φ sin δ + cos φ
  cos δ sin h₀). Right.
- Greenhouse: four grey layers at the lapse rate, square-root saturation past a column amount;
  no ceiling. Carbon gas's `ir_sat` 10 mol/m² against 142 mol/m² in the column: it is well into
  the square-root regime, as the real gas is. Right.
- Henry's law: carbon gas 6 x 10⁻⁴ mole fraction under 1 atm at 298 K (measured: 6.1 x 10⁻⁴); the
  heat of solution as the vaporisation enthalpy (2,100 K against 2,400 measured). Right.
- The light reaction: ΔH = 467.5 kJ/mol (hf of sugar -212, oxygen 0, carbon gas -393.5, water
  -286). Sugar burns for 15.6 MJ/kg, fat 46.7, charcoal 32.8. Right.
- Charcoal in the open: reaction 5 at 290 K is 10⁵ x e^(-69) x 0.21 = 2 x 10⁻²⁶ /s: geological.
  Ignition at about 790 K. Right, as review 3 asked.
- Dead sugar on the ground oxidises at 7% a year (reaction 1 at 290 K with the oxygen share) and
  goes to methane at 0.8% a year (reaction 9). A soil pool with a life of about 14 years. Right.
- Ageing: a 1 kg light-eater at 17 kJ/kg/day and REPAIR -1 lives 10⁸ / 17,000 = 5,900 days =
  16 years; the handover says 18 (its upkeep is a little lower with the repair factor). Right.
- The age-death share in a node is a constant hazard; an individual's wear is a count. The same
  mean, different spread (a node's members die at all ages, an individual at exactly its life).
  Fine, and the test says so.
- The body's heat path, the light pool, the set-aside share: unchanged since review 3 and still
  exact (the energy drift says so).
- Eaters' breathing: `plan` caps the air a body takes at half its air node's content; at 10¹⁰
  members in 16-cell air nodes that is never reached (a cell's members breathe 0.2% of their air
  node's carbon gas in 20 days). Right.

What I did not find: any new place where matter or energy is made or lost, any new clamp that
biases a gene, any reaction that runs where it should not.

## Speed

Where the time goes (report 4: 16 to 27 s a year at 128 x 128 with 14,000 to 21,000 bodies;
review 3's profile: bodies 66 to 90% of a year, flow most of the rest; about 15 µs per body
update). The count of updates is the cost: about 20,000 bodies, animals at 10-day intervals
(36 updates a year) and plants at 20 days (18), roughly 600,000 to 1,300,000 updates a year.

Gains I can see from the code, biggest first, with what each is worth:

| # | Gain | Worth | Risk |
|---|---|---|---|
| 1 | Finding 1's fix, so plants can drop their brain by selection (review 3, Q2) | 25 to 35% of a year once they have (a plant with no brain takes no decision and pays no decision cost, so its interval can lengthen too) | none; it is the planned path |
| 2 | `--crowd 3` on big worlds (report 3 already suggests it) | report 3: a year 1.7 times faster at the cost of fewer nodes per cell | a speed knob, must be reported |
| 3 | Threads in Flow: `air::radiation` (per cell, sums per thread), `air::wind` (per line) and the air loop of `change::step` are independent per line or cell; `rayon` behind a feature flag as handover 8.2 allows | flow is 10 to 25% of a year; about half of it | small; the ordered liquid sweep stays single-threaded |
| 4 | Stop allocating in hot paths: `wanted` returns a `Vec` per call; `starve` and `thin` build a `dead` vector per update; `body::crowd` builds two vectors per body per month; `copy::part` clones a whole body (genes, stores, memory) to split one member | a few per cent | none |
| 5 | A node's one-off split for a Bond action (finding 7): skip the `reborn` of the node when the member comes straight back with the same rows (the fast path exists in `reborn`; check it is taken) | about 1 s a year with brained plants; nothing once they are brainless | none |
| 6 | `change::react` tests all 21 reactions each sub-step for every body (review 3, item 6; item 14 did not do this one): a per-body list of reactions whose inputs it holds, built in `prepare` | 3 to 5% | none |

What does not help: the snapshot (fixed in report 3), the tape (1%), the survey (every 3 days over
all bodies, cheap). What I would not do: `f32` state, skipping laws, or a cap on members. The
honest statement for the report: **without M4's coarse regions and monthly nodes, the island will
stay at 10 to 20 s a year**; the gains above take perhaps a third off. The handover's target
(1,000 years in 10 minutes) is 0.6 s a year, 20 to 40 times away, and M4 is the plan for it.

## Go to M4?

**Yes.** Nothing in the math, the physics or the chemistry is wrong in a way that M4 would build
on and have to undo. The four big findings of review 3 are fixed and the fixes hold over 300 years
in three seeds. What is left is of three kinds: a bug that freezes one gene (finding 1), measures
that mislead (findings 2 and 3), and physics that is missing rather than wrong (findings 4, 5, 6),
which M4's planet is the right place for. Do rows 1 to 3 first (about two days), because the
first M4 report must be able to say what the animals of this world are like, and because M4's
speed plan needs brainless plants.

Do not expect a people-like kind from M4. Expect: kinds by climate, seasons, travel, a reason for
shelters and stores. The intelligence step is the check-point's option 2, after M4, and it may
fail.

## The list, in order

S is under a day, M a few days.

| # | Do | Size | Finding |
|---|---|---|---|
| 1 | A node can split along its brain-size genes (their step counts in `split`'s gap, not in likeness); a test that a cost difference moves a node's hidden gene; keep the no-selection test | S | 1 |
| 2 | Per starting kind in the series: deaths by cause, births, realized mean life, mean member mass, and the means of GAS, LEAF, COPY_MASS, INTERVAL, HIDDEN and the catalyst genes; the seasonal swing of the land's ground temperature | S | 2, 3, 4, 7 |
| 3 | Rewrite report 4's gene story with the per-kind columns (the plants' light gene did not move; the eaters' burn gene rose); say the realized life is about a month | S | 2, 3 |
| 4 | Gas exchange through the open surface (capture area plus skin), the GAS gene per m²; re-set the roster's GAS numbers; seed 1 at 64 x 64 for 60 years, watch member mass and shade | M | 4 |
| 5 | Run `--clever-year` once at 64 x 64 beside the real run; report whether a big brain pays | S | 6 |
| 6 | Speed rows 3 to 6 above, each measured on the 64 x 64 benchmark | S each | Speed |
| 7 | Intake and sweep with mass^0.75 per member; measure the copy-mass gene and the heaviest member before and after | S | 5 |
| 8 | Then M4 as planned: parts held by nodes, coarse regions, monthly nodes, the planet with latitude bands; the first M4 report prints the row-2 columns and the realized lives by kind | | |

Rows 4, 5 and 7 can go into M4's first weeks; rows 1 to 3 before it.

## What this review did not check

- I ran nothing: no re-run, no profile, no snapshot read. The 300-year numbers are report 4's, the
  profile shares are review 3's, and the realized life is arithmetic on two of report 4's numbers.
- The viewer (`web/`), the recorder's frame layers, and the pictures beyond reading them.
- Chemistry 0's metal and alloy reactions (10 to 20): nothing uses them yet.
- Bond's load and failure in detail: nothing was built in the runs, so there is no result to check
  it against.
- Why plants with a fixed light gene and 40 times excess light do not raise their GAS gene: the
  series does not hold it (row 2 adds it).

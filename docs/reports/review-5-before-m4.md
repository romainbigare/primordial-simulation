# Review 5: why lives are short, and what happened to the roster world

14 October 2026. Code: `main` at 7043670 (watch report 5). Read from the code, the handover and
watch report 5's numbers. **I ran nothing.** Every number is from report 5 or from arithmetic on
the code's constants; the arithmetic is written out so you can check it, and the last section
lists what the tapes you already have can confirm.

## Short answer

Both problems have one root, and it is not the new laws. **Nothing in the laws makes building a
body take time.** A body turns every spare joule into children as fast as it eats, and it eats
three to five times its upkeep. So:

1. **Lives are short because births are fast.** When a population is steady, deaths equal births,
   so a member's real life is one over its birth rate. An eater of 57 kg eats 3.3 times its upkeep,
   grows from a 17 kg child to its 50 kg copy mass in 20 to 30 days, and copies on about a quarter
   of its decisions after that. That is a birth every 50 to 60 days per member, so a life of 50 to
   60 days. Report 5 measured 51. The plants are the same story at 80 to 90 days. The repair gene
   never comes into it: nothing lives long enough to wear out.
2. **The roster world did not lose its plants.** It lost plant *members*: the 20 kg light-eaters
   won by shade and grew to 115 to 142 kg, so the same plant mass sits in 100 times fewer
   members. By my arithmetic the roster's plants weigh 3 to 7 x 10¹⁰ kg at year 100 against
   6 x 10¹⁰ in the normal run, and the roster pictures are greener than the normal one, not barer.
   The tiny eaters of remains "take over" only by count: by mass they are about a third of the
   plants, which is what decomposers are on Earth. Two real faults sit under this, though: a kind
   with strength 0 pays no cost at all for its body, and a plant's leaf area and height grow with
   its whole mass, so one giant per cell is the best plant there is. Those two give the 20 g
   eaters and the 5,000 t members.

**What to do, in order:** one new free number in Copy (building takes time: a body pays for
children at most so much per day, by its mass^0.75), one in the upkeep (every body pays a basal
cost per kg), leaf area on the same mass^0.75 as intake, and the reports by mass as well as by
members. Then the 300-year runs, then M4. The owner asked whether lives should be years; this is
the law that makes them years for big bodies and months for small ones, by itself.

## 1. Why a member lives 50 to 90 days

### The arithmetic for an eater (report 5's normal run, year 100: 57 kg, burn gene 2.67)

| Item | Value | How |
|---|---|---|
| Upkeep | 66 kJ per kg per day, 3.75 MJ per day, 0.24 kg of sugar a day | strength 0.6 x 10⁵ = 60,000; catalysts 10^2.67 x 4,300 x e^(-7,760/290) x 1.5 x 10⁹ = 7,200; learning 3,000; brain 10⁵ x 57^0.25 / 10 days / 57 kg = 480; times the repair factor at gene -0.5, 0.93 |
| Most it can take | 1.04 kg a day, of it 0.8 kg of sugar | `take` 0.05 x 57^0.75 = 0.05 x 20.8; by uptake 1.0 sugar and 0.3 liquid |
| Food in reach | far more than that | plants are about 10 kg/m² (3.9 x 10¹⁰ members x 1.5 kg over about 5.6 x 10⁹ m² of land); `sweep` 20.8 m² a day finds 60 kg of them |
| Spare | 0.56 kg of sugar a day, 8.7 MJ: **3.3 times its upkeep** | |
| A child | 17 kg (the CHILD gene 0.3 of 57 kg), 5 kg of it sugar | `copy` |
| The work of a child | 51 MJ, 3.3 kg of sugar, paid in one interval | `c_copy` 10⁷ x 0.3 x 17 kg dry; the body can release 154 MJ in 10 days (58% of its 570 mol of sugar) |
| A child's growth to copy mass | 33 kg of body, 10 kg of sugar: about 20 days of eating, plus the work | copy mass gene 1.7 = 50 kg |
| Copying after that | on 27% of decisions (every 10 days): about every 37 days | the kept copy logit -1: 1 / (1 + e) |
| **Births per member** | **one every 55 to 60 days** | 20 to 30 days of growth, then 37 |
| Life at steady state | 55 to 60 days | deaths = births |
| Measured | 51 days | report 5, `root_1_life_real` 0.14 years |

Where do they die? 98% `SPENT`: the body's reactions could not release the next interval's upkeep.
At 290 K an eater releases 58% of its sugar in 10 days, so it dies when its sugar falls under
about 7% of its mass; at 285 K (the coldest month on land) under 11%, at 280 K under 19%. A
full eater (30% sugar) lasts about 55 days without eating. So an eater dies after about two
months of bad decisions, or one cold month with a half-empty store. Food in the world is not the
limit; getting it into the body on time is, and the null runs already said that (without
learning, eaters die out).

### The same for a plant (1.5 kg, leaf 4 m² per kg, gas gene 2.36)

Income: 10^2.36 = 229 mol of air per m² per day over 6.1 m² of open surface (4.07 m²/kg of leaf
on 1.5 kg, plus skin) is 1,390 mol a day per member, 930 per kg; at 195 ppm that is 0.18 mol of
carbon gas, 5.4 g of sugar, **85 kJ per kg per day**, and the light reaction fixes 97% of it
(rate x 20 days = 3.5). Upkeep: light catalyst 3,100, strength 0.05 x 10⁵ = 5,000, leaf 2 x 10³ x
4 m² = 8,000, brain 10⁵ / 20 days = 5,000: 21 kJ, times 0.875 at repair -1: **18.5 kJ per kg per
day**. Spare: 66 kJ per kg, **3.6 times its upkeep**, 100 kJ a day for the member. A child of a
2 kg parent is 0.6 kg with 0.18 kg of sugar (2.8 MJ) plus 1.8 MJ of work: 4.6 MJ at 100 kJ a day
is **46 days**, plus the wait for the copy logit at 20-day decisions. Report 5 measured 80 to 90
days. The plants die of `SPENT` (63%) and `LACK` (36%): the burn gene 2.0
releases only 31% of a plant's sugar in 20 days at 290 K and 19% at 285 K, so a plant with under
about 12% sugar cannot pay a cold month; and more leaf now means more evaporation (it goes with
the open surface too), so a dry cell dries a plant below its 30% liquid minimum.

### What this means

- The "expected life" (`root_<k>_life`) is a ceiling nothing reaches. Fine as a column; do not
  read it as the life.
- The real life is **one over the birth rate**, and the birth rate is **spare energy over the cost
  of a child**. Both are large here because a body eats three to six times its upkeep and nothing
  slows what it does with the rest. In life a 50 kg animal also eats two to three times its resting
  need, but the spare goes to moving and keeping warm, and growth and children come at a rate the
  body cannot exceed: a child a year, a young one that takes a year or two to grow up. The world
  has no such rate. That is the missing physics, and it is the same one review 4 named in finding
  5 ("longer generations of big bodies are not modelled").
- Report 5's note that selection shortened the repair gene is right and is a symptom: when nothing
  lives past a year, repair is a cost with no return.

### The law that gives lives of years (one free number)

**Building a body takes time.** A body pays into its child store (`Body::paid`, the work of
making children) at most `c_build` x (its mass)^0.75 joules a day, the same mass^0.75 as its
intake (`Body::intake`). That is the growth rate of life, which goes with mass^0.75 like intake
and metabolism (von Bertalanffy's law): small bodies grow fast and breed fast, big ones slowly.

What `c_build` is in life: a 1 kg animal makes about 1 kg of dry young a year (10⁷ J, 2.7 x 10⁴ J
a day); a 50 kg one about 1 kg of dry young a year (2.7 x 10⁴ J a day over 50^0.75 = 18.8, so
1,450 per kg^0.75); a 1 t one about 10 kg (1,500 per kg^0.75). So `c_build` is between about
1,500 and 30,000 J a day per kg^0.75. **Suggested: 10⁴.** Then:

| Body | A child's work | Days per child | Births a year | Life it gives |
|---|---|---|---|---|
| 1 kg plant, 0.6 kg child (1.8 MJ) | 1.8 x 10⁶ J at 10⁴ a day | 180 | 2 | about half a year |
| 57 kg eater, 17 kg child (51 MJ) | 5.1 x 10⁷ J at 10⁴ x 20.8 a day | 245 | 1.5 | about 8 months |
| 600 kg eater, 180 kg child (540 MJ) | at 10⁴ x 121 a day | 450 | 0.8 | over a year |
| 2,000 t plant member, 600 t child | 1.8 x 10¹² J at 10⁴ x 5.3 x 10⁴ a day | 3,400 | 0.1 | 9 years |

The CHILD gene (0.05 to 0.5) then lets a kind choose small frequent children or large rare ones,
and the real life, the generation time and the pace of evolution all come out of size. This is
what the owner's "lifespans that differ by kind" needs: today every kind lives two months.

**It needs one companion rule, or the giants come back.** A body past `full` that cannot pay for
its children this interval would keep eating at the rate of `full` and swell (review 3's M2 again).
So: *a body takes no more than its reactions can use*, its upkeep plus what it can build, over the
interval (in kg, by the energy of what it takes; liquid is not counted, it comes by area). That is
satiety, and it is physics: nothing eats ten times what it can burn or build. It replaces the "past
`full` it takes at the rate of `full`" rule in `Body::intake` with a cleaner one, and it keeps the
"not a hard stop" lesson of 13 October (burning leaves its liquid product in the body, so the
upkeep's worth must still come in). About ten lines in `copy::copy` and `choose::reach`.

With both, the intake exponent (row 7 of review 4) is no longer what bounds size; it can stay at
0.75 (it does no harm) or go back to 1. Note one thing it does today: intake goes with mass^0.75
but strength's upkeep with mass, so a strength-0.6 body heavier than about 7,000 kg cannot feed
itself even in unlimited food (0.05 x 0.77 x 15.6 MJ x m^0.75 = 0.066 MJ x m at m^0.25 = 9.1),
while a strength-0 body has no such limit at all. That asymmetry is part of problem 2.

## 2. The roster world

### It did not lose its plants; it lost small plants

| Year 100, 64 x 64, seed 1 | members | mean member | plant mass | source |
|---|---|---|---|---|
| normal run (two kinds) | 3.9 x 10¹⁰ | 1.5 kg | 5.9 x 10¹⁰ kg | report 5 |
| roster, old laws | 4.7 x 10⁸ | 142 kg | 6.7 x 10¹⁰ kg | report 5 |
| roster, both new laws | 2.9 x 10⁸ | 115 kg | 3.3 x 10¹⁰ kg | report 5 |
| roster, gas law alone | 1.6 x 10⁶ | (the 1 kg kind) | a few 10⁶ kg | report 5 |

The plant mass of the roster is the same order as the normal run's with the old laws and half of
it with both new laws. The maps agree: the life panel of `roster_seed1_year0100.png` and of the
old-laws run is a denser green than `seed1_year0100.png`. The members fell a hundredfold because
the plants that survived are the big slow light-eaters (20 kg at the start), grown to 115 to 142
kg, and the 1 kg and 0.05 kg kinds are gone. That is selection by shade, and it is right: since
report 3 light passes the tallest member first, a 140 kg member (0.5 m) stands over a 1 kg one
(0.1 m), and in a cell with 10 kg/m² of plants and 4 m²/kg of leaf (a canopy of 40) nothing
under the top layer sees light. The normal run has no big kind, so its 1 kg plants climb slowly
(1.2 to 1.7 kg, the heaviest 28 kg) instead.

The run with the gas law alone is the odd one: there the plants really did vanish, and it was the
1 kg kind that outlived the 20 kg one. I cannot say why from the numbers given. With both laws
the world is like the old one, so the pair is safe; the single-law run is worth one look with the
per-kind columns (deaths by cause of kind 4, the big slow plants, in its first 40 years), not a
reason to hold anything.

### Why there are members of thousands of tonnes

Nothing bounds a plant's size, and the shade law rewards the largest. A member's leaf area is its
mass up to `full` times its copy mass, times the LEAF gene (`visible`, `open_area`), and its height
is (mass / 1,000)^(1/3). So a member with a copy mass of 1,000 t (the gene's wall, 10⁶ kg) and the
big kind's LEAF of 0.5 m²/kg holds 10⁶ m² of leaf, a quarter of its cell, at 12 m, over everything
else. Its costs: leaf 2 x 10³ x 0.5 = 1 kJ, strength 5 kJ, catalyst 3 kJ per kg per day, a brain
of 0.2 J per kg per day (10⁵ x m^0.25 per 60 days over 2 x 10⁶ kg): 9 kJ per kg per day, 18 GJ a
day. Its income: 10^2.78 = 600 mol of air per m² per day over 10⁶ m² at 200 ppm is 1.2 x 10⁵ mol
of carbon gas, 3.6 t of sugar, 56 GJ a day, three times its upkeep. It is a viable tree of 2,000 t,
copies every few years (the work of a 600 t child, 1.8 x 10¹² J, is one interval's release), and
no smaller plant in its cell can live. So the copy-mass gene of the plant kind runs to its wall,
as it did for the eaters in report 4, and the wall is now 1,000 t (review 4 moved it). Review 4's
size fix (intake with mass^0.75) was written for what a body *takes*; light, gas and rain were left
by area on purpose, so it did nothing for plants.

**Fix (one line):** the capture area a member works goes with the same mass^0.75 as its intake,
not with its mass: in `visible` and `open_area`, `work x leaf` becomes `intake x leaf` (per
member, from the 1 kg reference, as `Body::intake` does). That is how leaf area goes with mass in
life (a 1,200 t tree carries about 10⁴ m² of leaf, not 10⁶; leaf and crown go with about
mass^0.75). Then a bigger plant catches less light and gas per kg while its upkeep stays per kg, so
the shade it gains and the area per kg it loses meet at some size, and that size, not the wall, is
where selection stops. With the build law of section 1 on top, a 2,000 t member also waits nine
years per child, and a 20 kg one does not.

I cannot tell from report 5 whether the 3,400 and 5,400 t members are plants or eaters of
remains (the next point). Print the kind of the heaviest member (`mass_max_root`); one column.

### Why tiny eaters of remains are most of the members

Their row has strength 0, and strength is the only cost of having a body. An eater's upkeep is
66 kJ per kg per day, of which 60,000 is strength; an eater of remains pays catalysts 7,200,
learning 950 (LEARN -1.5) and its brain: **about 8 kJ per kg per day, eight times less.** They
cannot take from a body (force 0 x mass = 0), but they need no force to take from the ground, and
the ground gets 1.8 x 10¹¹ dead plant members a year (report 5's normal run) with 30% sugar in
them. So they live on the churn of section 1 at an eighth of everyone's cost.

Why 20 to 90 g? A 20 g body's brain costs 10⁵ x 0.02^0.25 = 3.8 x 10⁴ J a decision, 190 kJ per kg
per day at 10-day intervals, 25 times its other upkeep: a 20 g eater with a 12-unit brain that
decides every 10 days cannot exist. So their interval gene must have gone long (the series has it:
`root_5_interval`; I expect it near the top, 100 to 360 days), and a 20 g body that decides once a
year eats once a year: `take` x 360 days x 0.18^0.75 is 4.9 kg against its 0.09 kg copy mass,
bounded only by its keep genes (up to nine times its mass in one store), and then it is tens of
times past `full` and has `k` = (m1 / full - 1 + 0.3) / 0.3, hundreds of children at once. That is
a spore. It is a sound strategy under the laws as written, and nothing in the laws says a body
cannot be 20 g. Nor should it: Earth's decomposers are bacteria and fungi, and by mass these are
about a third of the plants (1.2 x 10¹¹ members at about 0.1 kg, 10¹⁰ kg, against 3.3 x 10¹⁰), the
same share decomposers have on Earth (about 15 to 20% of plant carbon). By count they are 99.8% of
the animals, and that is how the report read them.

Two faults under it, though:

1. **A body with strength 0 costs nothing to keep.** Every living thing pays to stay alive (a
   resting cost of about 3.4 x m^0.75 W: 3.4 W for 1 kg, 70 W for 60 kg); here that cost is the
   strength gene, and the gene can be 0 by seeding or by evolution (its range starts at 0). Any
   kind can drop it and live eight times cheaper on dead matter. **Fix (one free number):** a basal
   upkeep every body pays, `c_basal` per kg per day, strength on top. At 2 x 10⁴ J per kg per day
   (a third of an eater's, about 0.2 W per kg) a 1 kg plant still keeps a spare of over twice its
   upkeep (66 - 20 = 46 kJ against 38.5) and a 20 g spore pays 3.5 times what it pays now. If you want life's scaling, charge it per
   kg^0.75 from the 1 kg reference (a 20 g body then pays 2.7 times more per kg, a 60 kg one 2.8
   times less); that also matches the intake law.
2. **Count by mass in the reports.** "Members" is what the tape counts because nodes count
   members, but a 20 g spore and a 140 kg plant are not one each. Print `root_<k>_kg` (members
   x mean mass) next to `root_<k>_members`, and read "who took over" from it.

### Two notes from report 5 that are decisions for the owner

- **The big-brain test.** Children join any node they are like in genes, and likeness leaves the
  brain-size genes out, so ordinary eaters' children averaged the 48-unit kind down to 12 in a
  year. The handover asks the owner to choose. My advice: count the size genes in likeness with a
  step of one whole unit (a 48-unit brain and a 12-unit one are not one kind; two halves of a
  split, 0.3 units apart, still are). That keeps the split-and-merge round of review 4's fix and
  keeps a placed kind apart, without a rule about kinds.
- **Lives of years or months.** Section 1's law makes big bodies live years and small ones months,
  by their size, and the CHILD gene lets each kind trade the two. If the owner prefers the world as
  it is (a fast churn), do not add it, but then do not expect lifespans, generation times or a
  memory of individuals to mean anything, and say so in every report.

## What the tapes you already have can confirm (no new run)

From report 5's `series.csv` files, with the per-kind columns:

1. Roster, old laws and both laws, year 100: `root_4_members x root_4_mass` (the big slow plants)
   is between 3 and 7 x 10¹⁰ kg, within a factor of two of the normal run's plant mass.
2. Roster: `root_5_interval` and `root_6_interval` (the eaters of remains) are near the top of the
   range (log10 above 2), or their `_hidden` is well under 12. One of the two must hold for a 20 g
   body to pay its brain.
3. Normal run: `root_1_births` is 7 to 10 times `root_1_members` a year, and `root_1_died_spent` is
   over 95% of its deaths; `root_0_births` is 4 to 5 times `root_0_members`.
4. The heaviest member's kind is not in the tape. Add `mass_max_root` and look.

If 1 fails (the plant mass really is a hundred times down), section 2's first reading is wrong and
the plants' deaths by cause (`root_4_died_*`) say what killed them. If 2 fails, the spores pay
their brain somehow I did not see, and `root_5_life` against `root_5_life_real` will say how.

## 3. A maturity range? No

The owner asked whether to add a range of sexual maturity (an age or a size between which a body
may copy). No. Maturity already exists in the laws as a size: the copy-mass gene, and the
child-share gene beside it, and selection moves both now (the size runaway was that). In life,
maturity is mostly a size too. What is missing is not a threshold but time: building a body is
instant here. An age range would be a rule written at the level of the result ("animals mature at
two years"), like the sowing windows of version 1 (handover 2, lesson 2); you can list all its uses
in advance, and it has one. The build rate of section 1 is the cause instead: one number inside
law 4 ("the parent must have the materials and the energy", and now the time), and age at
maturity, generation time, lifespan and recovery after a crash follow from mass and food. Nothing
is written down.

## 4. Two drifts from the handover's philosophy, with fixes

Reading the code against handover sections 2 and 3, the physics side keeps the philosophy well:
five laws in five files, no Earth words in the core, nothing made or lost, no "eat" action, honest
reports, null runs, and results that came out by themselves (rivers, a stable climate, a food
chain, decomposers, shade and size selection). Two things drift. Each gets a fix that is small and
keeps results the same, so they can go in now.

### Drift 1: free numbers pile up, and each is chosen for an outcome

`Physics` holds about sixty numbers. Since review 3 it gained the catalyst capacity, the unfolding
temperature, repair, the life energy, the intake exponent and its reference mass, and this review
asks for two more. The handover allows free numbers (6.0: "a free number to tune"; M7 varies them),
but many were set the way version 1's realism was (handover 2, lesson 3): "so the starting plants
pay what they paid before", "so the eaters live", "so the big slow kinds survive a neutral brain".
A number set for a result is a rule in a smaller coat. The fixes:

1. **Every number names its fact.** A short table, `docs/numbers.md`, one row per field of
   `Physics` and `Config`: the unit; the fact of matter or energy it is (a capacity, a cost, a
   conductance, a rate, a reference mass); the real-world size it is anchored to, with the source
   in a few words (Kleiber, Rubner, Henry, a leaf's 24 g per m² per day); and one of three marks:
   **physical** (a measured size, used as measured), **anchored** (a real order of magnitude,
   chosen within it), **tuned** (set so a run came out a certain way, with the run named). A test
   in `core/tests` reads the table and fails when a field of `Physics` has no row, so no number can
   be added without saying what it is. Half a day; most rows exist already as doc comments.
2. **A budget for tuned numbers, like the code budgets (14.3).** Today's tuned ones, as I read
   them: `full` (2: "copies at twice its copy mass"), the kept copy logit (-1, a behaviour, said so
   in `life.rs`), `take` and `sweep` (0.05 and 1: set for eaters to live), `crowd`, `alike`,
   `WITHIN` (speed-tool thresholds), `shelter`, `c_leaf`, `c_handle`, `c_move`, and the roster's
   stores and intervals. Say the number (about twelve) in the handover, and the rule: it may not
   grow. A new number must be physical or anchored, or it replaces a tuned one. The two numbers of
   this review are anchored (`c_build` to the growth of young in life, `c_basal` to resting
   metabolism); `full` and the "past `full` it takes at the rate of `full`" rule go away with
   satiety, so the count falls by one.
3. **Fewer numbers where one fact is written twice.** `m_ref`, `intake_ref` and `heat_mass` are
   three reference masses for one idea (the member at which a per-kg number is per kg); make them
   one, at 1 kg, and scale `HEAT` as the others are. `t_cat` (290 K) is `worldgen::life::T_SURF`
   again. `c_hidden`, `c_memory`, `c_seen` and `c_sight` already derive from `c_brain`; say so in
   the table rather than list four. About five numbers fewer, results unchanged (the scaling of
   `HEAT` needs the starting kinds' gene reset, as the gas gene was).
4. **Every run prints its numbers, every report prints what changed.** The recorder writes
   `physics.json` (the `Physics` and `Config` of the run) next to `series.csv`; a report's "what we
   built" table is then the diff of that file against the last report's. Today a number can
   change in a commit message and nowhere a reader of the reports sees it (the gas gene reset,
   the 60% stores, the intake exponent all did). An hour.
5. **The M7 world search varies only the tuned and anchored numbers, within their anchors**, and
   never the physical ones. Write that in the table's header now, so the search is designed from
   the table, not from the struct.

### Drift 2: the population node makes biology, and its check was never run

The node is a speed tool (handover 7.1: "it must give the same patterns as the detailed form"). It
now has about six hundred lines of rules of its own: when a node splits, when two merge, which
genes count as alike, who joins whom, a split-off member for Bond actions, a merge "however
unlike" past twice `crowd`, no split in a full cell. Three times those rules changed the biology:
genes drifted (review 3), brain size froze (review 4), a placed brain was diluted (report 5). The
check that the handover demands (nodes against individuals, before M4 builds on them) has never
been run, and today it cannot be: there is no way to run the animals as individuals. The fixes:

1. **An individuals mode, so the check can run.** `Config::nodes` (default true;
   `--individuals`): when false, an animal's children are placed as individuals and never join a
   node (`copy::copy` skips `within`), the monthly `body::crowd` merges nothing, and nothing
   splits. Light-eaters stay nodes in both modes (one node per cell is how they are placed, and
   4 x 10⁵ members of 1 kg per cell cannot be individuals; the handover says light-eaters first).
   About twenty lines, all guards. Then the check: seed 1 and seed 5 at 32 x 32 for 100 years, both
   modes, the same columns (`root_<k>_kg`, `_life_real`, `_mass`, `_copy_log`, `_interval`, the
   catalyst genes, the eaters' food shares, `kinds`). "The same patterns" said before the run, not
   after: biomass and real life within a factor of two, the gene means moving the same way, the
   same kind winning. Where they differ, the node rule that differs is the bug, not the biology.
   Three days with the runs. **This is M4's first job**, before coarse regions.
2. **The safeguard that writes biology gets counted.** The merge "however unlike" past twice
   `crowd` blends two kinds into one node whose spread then forgets which genes went together
   (handover 7.1's known limit). It may be rare or common; nobody knows. A series column,
   `merged_unlike` (members a year merged that way), and a line in each report. If it is more
   than a few per cent of the animals' members in a year, the ceiling is too low or the join rule
   too strict, and that is a speed-tool setting to raise, never a result to keep. An hour.
3. **Likeness counts the brain-size genes with a step of one unit** (section 2's note). A 48-unit
   brain and a 12-unit one are not one kind and must not merge or be joined; two halves of a split,
   0.3 units apart, still are. One line in `copy::step`, and the big-brain test can be run as it
   was meant.
4. **A node can be opened.** Handover 7.1 asks for "split it back into individuals when the player
   looks closely"; it is not built, and the check in fix 1 wants it too (to compare a node's
   members with real individuals, draw them from its spread). `copy::one` already draws one
   member; a `copy::open(node, n)` that draws `n` is the same code in a loop. A day. It is also
   what M6's close view will need.
5. **Each node rule is a setting, printed and varied.** `crowd`, `alike`, `WITHIN`, the twice-crowd
   ceiling and the split-only-with-room rule are in the numbers table (drift 1) as tuned, and the
   first M4 report runs one seed at `crowd` 4 and 8 and in individuals mode, side by side. A speed
   tool may change the speed; it may not change which kind wins, how long it lives or what it
   eats. If it does, that is the finding.

### What does not change

The node as a design stays (one count, mean genes, a spread, one brain; conservation exact), the
group brain stays, the roster stays as a reported start. The ageing law stays; it was added for
its result, which is the one thing the handover says not to do, but it is harmless, and with the
build rate bodies will live long enough for it to act, so it earns its place then.

## The list, in order

S is under a day, M a few days.

| # | Do | Size | Section |
|---|---|---|---|
| 1 | `c_build`: a body pays into its child store at most `c_build` x mass^0.75 a day (10⁴ J a day per kg^0.75); with it, a body takes no more than its upkeep plus what it can build over the interval (satiety), in place of "past `full` it takes at the rate of `full`" | S | 1, 3 |
| 2 | `c_basal`: a basal upkeep every body pays per kg (or per kg^0.75 from 1 kg), 2 x 10⁴ J per kg per day, strength on top; re-check the starting kinds' lives and the plants' spare | S | 2 |
| 3 | Leaf area with the same mass^0.75 as intake (`visible`, `open_area`) | S | 2 |
| 4 | Tape: `root_<k>_kg`, `mass_max_root`, `merged_unlike`; the recorder writes `physics.json`; reports read who took over by mass and print the diff of the numbers | S | 2, 4 |
| 5 | Likeness counts the brain-size genes with a step of one unit; then the big-brain test again | S | 2, 4 |
| 6 | `docs/numbers.md` with every number's fact, anchor and mark, and the test that no field of `Physics` is without a row; the tuned budget in the handover; one reference mass instead of three | S | 4 |
| 7 | Check the four predictions above on the tapes of report 5; then seed 1 at 64 x 64 for 100 years, normal and roster, and read `root_<k>_life_real`, `root_<k>_kg` and the heaviest member's kind | M | 1, 2 |
| 8 | `Config::nodes` (`--individuals`) and `copy::open`; the nodes-against-individuals check on two seeds at 32 x 32 for 100 years, with "the same patterns" defined before the run; one seed at `crowd` 4 and 8 beside it | M | 4 |
| 9 | The three 300-year seeds at 128 x 128, then M4 (coarse regions only after row 8 passes) | | |

Rows 1 to 3 are three free numbers and about thirty lines; they change what the results mean
(lives, sizes, who lives on what), so run them before the 300-year seeds, not after. Rows 4 to 6
keep every result the same. Row 8 is the handover's own condition for M4.

## What this review did not check

- I ran nothing. The roster's plant mass is members times mean mass from report 5's table; the
  birth rates are arithmetic on the code's constants and report 5's masses and genes.
- Why the gas-law-only roster run lost its plants for real.
- Which kind the 3,400 and 5,400 t members belong to.
- Whether the eaters of remains' long interval is what I expect (prediction 2).

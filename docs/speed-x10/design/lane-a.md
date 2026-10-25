# Speed x10, section "lane A": plants and animals after the month law

> **What this file replaces.** The month law (`month.md`) was built for stands as P1.2 and is not kept (X63,
> `measures.md` "P1.2"). This file holds lane A's alternatives: for plants, today's day law at a longer stand pace with
> C2's causes fixed inside it (PR-C2, P2.1, P2.2, G9a); for animals, G2 on today's day law (the brain floored, the body
> at its gene's pace). It replaces `month.md` as a whole, `animals.md` 3 (P1b), `plants.md` 2.3 and 2.4's month numbers,
> and `gap.md`'s G2 and G3. **Kept there:** `plants.md` 2.1, 2.2, 2.5 (P3), 2.6, 2.7, 2.8 (the sample is today's);
> `animals.md` 1, 2 (PR-A), 4; `month.md` 1's findings, as findings. Decisions: X63 to X70. The plan (`plan.md`) holds
> over this file.

## 0. In short

- **The month law costs more than today's law.** A stand update cost about 26 µs (173k instructions against 121k for
  today's 18 µs update), so a stand-year cost more than today's ~182 µs at every pace: about 316 µs at 30 days, 225 at
  60, 200 at 90. The cost is in the plan's stages, the booking and the sample, not in the fuel's closed form (P1.2's
  measures). So P1.2, P1.3, P1.4, P1b, the month-based G2 and G3 lose their base (X63, X66, X69).
- **Plants: fewer updates of today's cheap update.** The lever is the number of stand updates a stand-year. Today's day
  law at a pace of 60 or 90 days, with C2's causes fixed inside it: the time's light pays the upkeep before members die;
  members die at a rate over 36 days, not per update; growth keeps a reserve of 36 days, not of one update. If the
  pace test needs it, growth, maturing and children also keep the stand's 36-day grid (X55). A probe first (PR-C2).
- **Animals: G2 floors the brain only, on today's law.** A node runs today's body update at its gene's pace (eating,
  growth, maturing and children as today). It decides about every 30 days; at its other updates it takes from the
  ground by the chance of its last decision (a habit on a clock), with no thinking. PR-A2 measured this within the seed
  noise on all three worlds. Measured after MI5, the animal updates then cost x0.85 (island) and x0.79 (globe) of today.
  P1b is not needed (X66); MI3a is dropped, as G2 is its clock-only form (X68).
- **The outlook falls** (plan 5.0): likely island about 3.9 to 5.2 times, globe about 5.6 to 7.6 (was 5.8 to 8.1 and
  7.7 to 11.3). 10 times is out of reach on both benches with the steps planned.

## 1. What P1.2 teaches

| Finding (`measures.md` "P1.2") | What it means for lane A |
|---|---|
| a stand update 173k instructions (26 µs) against 121k (18 µs) today; 2.83 sub-steps a month | a law that does more work per update cannot win; only fewer updates of today's update can |
| the plan's stages 24k, sub-steps 29k each, booking 34k, sample 19k | booking and sample alone pass the 40k of a 6 µs update: a cheaper month would need a new booking and sample, not a new integrator |
| the splitting test passed (1.3%) only with a sub-step rule on the light reaction's liquid | the day law's own splitting errors at a long pace are real and must be measured (PR-C2) |
| land -0.78 K, S50 0.556 -> 0.636, light-eaters' life 1.18 -> 2.60 yr, cause not found | a law that moves stands to their ground's temperature and changes their reactions moves the world; the alternative keeps the stands' heat and reactions as today |

## 2. Plants: today's day law at a longer stand pace

### 2.1 What makes the day law depend on the length of an update

A stand is due every `node_days` (36) days (`Body::interval`). Its update (`body::live::to_growth`) depends on the
update's length `Δ` where it should depend only on time:

1. **Upkeep is paid before the time's light** (C2, S62): `settle_upkeep` pays `Δ` days of upkeep from the stock, then
   `physics` brings the light the stand stored over the same days. So the stock it needs grows with `Δ`.
2. **The unpaid share dies at once:** `short` dies in `lose_share`, so over a time `T` a starving stand keeps
   `(P/U)^(T/Δ)`: the death rate goes with `1/Δ`.
3. **Windows read the stand's interval:** growth's reserve is one interval's upkeep (`copy::grow::convert`, `spare`
   reads `upkeep(ivl)`), and `buildable`'s horizon (`soon`) is one interval's allowance. At a 60-day pace both double
   against 36.
4. **Maturing and children wait for the update** (C2's second cause): young that reach the copy mass join the adult node
   only at the young node's update, and children come only at the adult's. C2 at 140 days: births about half.
5. **The rate law's window** (PR-M 1, 2; X44): the release limit is read over `max(Δ, interval)` and the light
   reaction's catalyst contact over the breath of `Δ`, so their rate a day grows with `Δ`; the light reaction's liquid
   is a store, and roots and soaking have one room per update (PR-M 3, 6). PR-A's (g) holds for stands too: the
   dissolved mineral 23 a stand holds turns back to the solid 10 by a bare third-order reaction (about 17 days for an
   eater's pulse) through the update's physics, and roots refill it once, so over 60 to 90 days growth may find less of
   it than over 36. These were not fixed by C2's method, which still passed the pace test at 70 days (5 of 5 seeds):
   they are counted by the probe (the mineral 23 left at growth among its counters), not fixed first.

### 2.2 The law (P2.1, one law for every body)

Three changes in `body::live` and `copy::grow`, each a few lines; at today's pace only the first two change the world:

1. **Income before upkeep.** `settle_upkeep` pays from the stock what it can and the rest is owed; after `physics` the
   owed upkeep is paid from what the time brought in (C2's `pay_owed`, branch `speed/c2-not-kept`, 8acb757). Only what
   is still unpaid kills. Upkeep and income run together over the time (Thornley 1972's labile pool), so paying all of
   it first is a first-order splitting error.
2. **Deaths at a rate over `D`.** The unpaid share `u` of the time's upkeep kills the share `1 - e^(-u Δ / D)`, so a
   starving stand keeps `e^(-u T / D)` over a time `T`, whatever its updates. `D` is `Physics::reserve_days` (36, today's
   stand pace, a `numbers.md` row) for a stand, and the gene interval for a body with a brain, which is its update: for
   it the only change is `u -> 1 - e^-u` (PR-A (a): no effect beyond the noise).
3. **The stand's windows read `D`, not its interval:** growth's reserve is `D` days of upkeep, `buildable`'s horizon
   `D` days of allowance. `Body::interval` keeps meaning the pace.

**Announced law changes at today's pace:** a deep famine kills `1 - e^-u`, not `u` (as X14 (a) announced for the
month); a stand pays its upkeep from the time's light before members die. Stands keep their own temperature, their
reactions and their bounds, as today.

**The grid (variant (b), only if the pace test needs it).** A stand's maturing and children come at the points of its
own 36-day grid (X55): the days `d` with `d mod 36 = id mod 36` (no new state; ids spread the phases, so births are not
bunched; a merge keeps the receiving node's id and so its phase). Physics, upkeep and the sample run once an update;
**growth runs in pieces cut at the grid points inside the update** (each piece its days of allowance, from the stock the
update's physics left), and at each point a young stand matures with its piece's growth (`copy::mature`) and an adult
stand makes the children its brood covers (`copy::copy`). **What a grid point makes starts at its day** (C2's second
cause is the start, not only the count): the children and the passers of a point carry `last` set to that day and the
light and rain marks of that day (the parent's marks and the cell's totals now, interpolated, as the light is spread
evenly over the update), and they join their node through `nodes::rejoin`'s blend of `last` and the marks, so their
next update covers their time from that day and no member-days without light are counted (as `month.md` 5.1 had it).
The update's end is not a grid point. A growth piece may use
light the stand caught later in the update; that matters only where the stock limits growth, and a stand's stock is far
above its reserve in most updates (P0: the mean of log10(stock / reserve) +0.81; it crosses the reserve in 10% of
them), so growth is set by its allowance; the splitting test judges it. At today's pace (36 days) the update holds one
point, so maturing and births keep today's cadence; their day inside the update and the growth piece before it move.

**Not built:** the month (X63); a new integrator for the rate law's window (item 5) unless the probe names it; a new
sample (today's clone run stays; P1.4 is dropped).

### 2.3 The probe PR-C2

Branch `probe/pr-c2` from `speed/x10`. Switches by environment variables, all off by default (off: the base's world bit
for bit, as PR-A's). `C2_LAW=1` turns on 2.2's three changes, `C2_GRID=1` the grid, `C2_PACE=d` the stands' pace. So
both builds run the same saves and the cost is exact on one world (no snapshot change in the probe).

| Measure | How | Line |
|---|---|---|
| splitting test of the day law | `worldgen::testkit` stands (young and adult; light-limited, inhibited, release-limited at 275 K; constant forcing and a seasonal ramp; 60 and 90 days centred on the summer's peak and the winter's low): 1 x 60 against 6 x 10, 1 x 90 against 9 x 10 and 3 x 30; members and body matter, fuel reported (the day law is not converged on fuel, PR-M: 33% at 2 days); the same cases with today's law, and today's law at 1 x 36 against 6 x 6 | members and body matter: 90th percentile under 5%, and the fixed law at 60 and 90 no worse than today's law at 36 against 6 x 6 (90th percentile and worst) |
| pace test around today's pace | `the_pace_of_nodes_keeps_life` for stands with the law at 36: gene (0), 36, 72, 18 (C2's method broke this test at 72, seed 1, `air_3`) | within its noise (the test as it is) |
| pace test at the new pace | the same test: gene (0), 60, 120, 30; then 0, 90, 180, 46 (the half of 90 made a multiple of `batch_days`) | within its noise |
| selection | `selection_inside_a_node_moves_its_mean_as_its_members_would` and `genes_do_not_drift_without_selection` (slow) at 60 with `sample_every` 2 | pass; if they need `sample_every` 1, its cost (about 2.3 µs an update, 14 µs a stand-year at 60, 9 at 90) is counted against the cost line |
| key numbers | 3 seeds at 60 (and 90 if its pace test passes), against the references after AIR4.0 | in range; S50 not past its seeds' noise unless explained; the land reported |
| counters | per member-day at 36, 60, 90: deaths by unpaid upkeep, births, young that mature; the share of stand updates where the light catalyst's limit binds (item 5) | reported |
| bodies | the plant nodes at year 30 of the key-number runs against the reference (C2's remade globe held more bodies and parts and cost 45% more a year) | a rise of plant nodes past 10% is charged in full against the gain: the cost line below reads the stand-year ratio times that rise |
| cost | callgrind, island22 bench, one year, one thread, same save: instructions per stand update and per stand-year at 36, 60, 90, each variant | per stand-year at 60, times the plant nodes' rise past 10% and with the sample's fallback if needed, at most x0.85 of today's |

**Order:** (a) the law alone at 60; if its pace test fails, (b) with the grid at 60; then the passing variant at 90.
**What to expect:** C2's method (item 1 and a reserve net of income) passed the pace test at 70 days and failed at 140
(births about half): maturing and children waited. It also broke the pace test around today's 36 at 72 days (seed 1,
`air_3`), and its remade globe36 cost 45% more instructions a year (more bodies and parts): the two lines above. So (a) is expected to fail at 120 (the x2 of 60) and (b) is the
likely case; (a) costs the probe nothing more.

**Kill line:** the law fails the pace test around 36 (18, 36, 72); or neither (a) nor (b) passes the pace test at 60
with its x2 and /2, or the splitting line, or the key numbers, or the cost line (with the plant nodes' charge and the
sample's fallback): stands stay at 36 days on today's law, and P2.1, P2.2, G9a and P3 are not built. (b) passes at 60
but no variant at 90: P2 at 60, G9a not built. If the sample's fallback alone takes (b) at 60 past the line (about
x0.87), 60 is not built and 90 is judged on its own (about x0.68 with it).

**What it gains** (today ~182 µs a stand-year: 10.1 updates of 18 µs). Of today's 18 µs about 5.3 come once per grid
point (a growth piece about 2, children about 2.3, maturing 1.0) and 12.7 once per update (the rest of the body's day,
the sample, shedding, the rest). Variant (b) adds about 0.1 µs a grid point to start its children and passers at their
day (their `last` and marks), so 5.4.

| Variant | 60 days | 90 days |
|---|---|---|
| (a) maturing and children at the update | 6.08 x 18 = **about 110 µs (x0.60)** | 4.06 x 18 = **about 73 µs (x0.40)** |
| (b) the grid: 12.7 + 2 an update (a growth piece to its end), 5.4 a grid point (10.1 a year) | 6.08 x 14.7 + 55 = **about 144 µs (x0.79)** | 4.06 x 14.7 + 55 = **about 114 µs (x0.63)** |
| (b) with `sample_every` 1 (if selection needs it) | about 158 µs (x0.87): past the line | about 123 µs (x0.68) |

One thread, on the bench48 saves X50 judges against (plant updates 35.0% of island22's year and 16.3% of globe36's,
`measures.md` "Lane A replan"; R2: 30.0% and 14.7%): (b) at 60 -7.3% and -3.4%, then 90 -5.8% and -2.7%; (a) at 60
-14% and -6.5%, then 90 -7.1% and -3.3%. (b) at 90 is no better than (a) at 60.

**What it costs if it fails:** the probe's time (a branch, about a day of runs), no core lines. The plants' rows of plan
5.0 fall to none: P2, G9a, and P3 too (P3 needs a pace that is a multiple of `phys_days`; at 30 days stands would update
20% more often than today). Island about 65 ms of the likely year's saving lost (Z1 scale), globe about 45.

## 3. Animals: G2 on today's day law

### 3.1 The law

A population node with a brain runs **today's body update at its gene's pace** (`Body::interval` stays its gene's):
`body::live` (upkeep, physics, keep-warm, checks, growth, children, shedding), maturing, split, `see`, schedule. Only
the brain is floored:
- **It decides about every `think_days` = 30 days**: at the update nearest to 30 days after its last decision (it
  decides when its next update would lie further from 30 days than this one). An individual decides at every update,
  as today.
- **At its other updates it acts by its habit:** the share of its members that took from the ground at its last
  decision (`p[TAKE]`, kept with the body) takes again, as `spread` does for that one option (`part_acts`, `take_node`,
  sized as today by its interval). No perception, no network, no learning, no moves, no body takes, no gifts, no bond
  acts.
- **At a decision** every rate follows the time since the last decision (X16): the movers are `count x min(1, Σp x
  paces)` in one group (MI5's form), learning's discount is `discount^paces`, the prey's chance to get away reads the
  gene intervals. Body takes, gifts and bond acts come at decisions only.
- **Upkeep pays one decision per update, as today** (`pay::upkeep`'s `think`), so the energy law is today's. Paying
  per thought would be MI3b, a law change of its own, not made (X68).
- New saved state: the day of the last decision and `p[TAKE]` (snapshot bumped); `Config::think_days` (0: every update,
  the control; a `numbers.md` row, a speed setting with its pace test).

**This is PR-A2's passing run** ("pieces + takes + maturing": the body law at the gene's pace, ground takes between by
the last decision's chance, maturing at each piece, one decision a month): +0.015, +0.011, +0.001 against a seed noise
of 0.023, 0.054, 0.014. Two differences: (1) PR-A2 ran every case with (g) on (the eaten mineral kept from turning
solid); here the body is today's at its gene's pace, where (g) moved the eaters by -0.010, +0.006, +0.011 (PR-A),
inside the noise; (2) G2 scales a decision's movers by the paces since the last one and discounts learning by
`discount^paces` (X16), which PR-A2's run did not; PR-A's (d), the movers by the paces, moved the eaters by +0.002,
-0.024, +0.000 (noise 0.011, 0.064, 0.017). G2's check runs PR-A2's script with (g) off and both rules on.

### 3.2 How this relates to MI3a, X36, P1b and G3

- **MI3a** is a habit with wake rules (a sense moved, a merge, another lineage, the habit impossible, `sleep_most` 90
  days). Its gate failed (sleep share 14% island, 11% globe, the line 40%; merges wake 65% and 48%, MI0). G2 is a habit
  with one rule, the clock, for nodes only. After G2 a node's thoughts fall to about 0.30 to 0.33 of today; MI3a would
  only add habits for individuals (2% of the decisions on island and globe) and would wake nodes earlier, not later.
  **MI3a is dropped** (X68); its control `--full-minds` waits for the first smart kind (X25); MI3b with it. X36's
  order (G2 first, MI4 gated after it) holds; its MI3a part is closed.
- **P1b** is dropped (X66): its base was the month law (not kept); G2 no longer needs it (the body keeps today's law at
  its gene's pace); one energy law for every body (X6) holds, as every body keeps the day law with P2.1's changes; its
  own gain was an estimate on the month's cost, which P1.2 measured to be higher than today's.
- **G3** (the month's heat in closed form) is dropped (X69): its base was P1b. Its alternative is a measure: after G2, a
  profile of the animal updates by stage (`GB`, plan phase 2); a step is designed then only if one stage of the body's
  day is at least 25% of an animal node update.

### 3.3 What it gains

**Measured after MI5** (`speed/x10` at f85d626 with marks that keep the update kinds and the take apart; callgrind, one
year, one thread, the `bench48` saves; `measures.md` "Lane A replan: the decision after MI5"):

| | island22 | globe36 |
|---|---|---|
| `run_for`, million instructions | 6,685 | 6,058 |
| animal updates: million instructions (share of `run_for`); calls | 1,602 (24.0%); 9,151 | 2,071 (34.2%); 10,794 |
| an animal update, thousand instructions | 175 | 192 |
| the decision (`choose::act`): million; calls; thousand a call; share of the animal updates | 535; 9,110; 58.7; 33.4% | 857; 10,027; 85.5; 41.4% |
| the take path a habit keeps (the take option's split, `take_node`, rejoin and well-being reads): million; calls; thousand a decision | 191; 8,629; 21.0 | 225; 9,806; 22.5 |
| of it `take_node`: million; calls; thousand a call | 162; 8,755; 18.5 | 185; 9,945; 18.6 |

So a habit update saves the decision less the take: 37.7 (island) and 63.0 (globe) thousand instructions, 64% and 74%
of the decision. **Which updates decide:** with the rule "the update nearest to 30 days after the last decision", a node
of a 9 to 11-day gene decides at one update in 3, one of 7 or 8 days at one in 4; R2 measured that a 30-day floor keeps
0.30 / 0.31 of the node updates (the mean interval about 9.1 days), so 0.30 to 0.33 decide and **0.67 to 0.70 are
habits**; nodes make about 98% of the animal updates (MI0). So the animal updates cost **x0.85 to x0.86 (island) and
x0.79 to x0.80 (globe)** of today, and the year on one thread **-3.4 to -3.5% (island) and -6.9 to -7.1% (globe)**. At
20 days (0.45 / 0.46 of the updates decide) about 0.79 of that; for young nodes only about 0.57. The first estimate
(x0.77 / x0.68, from R2) took the decision before MI5 (MI5 took about 212 M of the globe's 1,290 M decision
instructions) and a take of 1.5 to 3 µs; the measured take is 21 to 22.5 thousand instructions a decision, about 4 µs,
as `take_node` pays its handling through a payer of its own (`body::pay`). A cheaper take is a later step, not part of
G2 (it would change no law: level 1).

### 3.4 The checks

PR-A2's script with G2 (5 seeds, the three benches, 2 years, (g) off) at 30 days: the eaters within the seed noise of
`think_days` 0, with PR-A2's counters (takes cut by what lies, young that mature per young member-day, starving deaths);
the globe's body takes (5 to 7% of its eaters' kg) reported, as they now come once a decision. The animals' pace test
(`think_days` 0, 15, 30, 60); the nodes check (30-day nodes against individuals at their gene); the learning scene; the
key numbers. **Fallbacks** (X58, now on today's body law at its gene's pace, which is what PR-A2 measured): 20 days
(PR-A2: within the noise on the globe and the region, -0.030 on the island); then nodes of young members only; then
revert. If the globe's eaters fall because body takes come once a decision, the habit repeats the last decision's body
takes on the same body while it lives in the cell (by its id), before any fallback.

## 4. The outlook

Plan 5.0 holds the rows. In short (four threads, Z1's scale): lane A's steps save island about 67 ms (plants 52, G2
15) and globe about 61 (plants 29, G2 32) in the likely case (b), against about 168 and 170 in the month's plan. Likely
with everything: **island about 3.9 to 5.2 times, globe about 5.6 to 7.6**; with variant (a) at 60 and 90, up to 5.8
and 8.0.

## 5. Lines

P2.1 about +30 (the owed upkeep, the rate, `reserve_days`); the grid about +60 if (b) (the growth pieces, maturing and
children at the points, their day's `last` and marks); P2.2 a rename and a default; G2 about +60 (the habit take, the
decision's day, the paces at a decision). Lane A about +150, against about +510 for the
month's plan (P1.x +500, P1b -170, G2 +80, G3 +100). MI3a's +145 are not written.

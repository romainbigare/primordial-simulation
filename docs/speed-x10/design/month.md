# Speed x10, section "month": one month law for every body, after the probes

> **What this file replaces.** It is the design of the month law after phase P (probes PR-M and PR-A,
> `measures.md`). Where it differs from `plants.md` or `animals.md`, this file holds; the plan (`plan.md`) holds over
> all three. **In `plants.md` it replaces:** 2.3 (the stages of the plant month), the month's numbers in 2.4 (the
> card; the case against a grid table stays), the sample's cost in 2.6, 3 items 1 to 4 (the law changes), 5 tests 1 to 3,
> 6 (the gains) and the P1 row of 9. **In `animals.md` it replaces:** 3.1 to 3.4 (the law, the pulse, the booking,
> the update) and 5's two tests. **Kept there:** `plants.md` 2.1, 2.2, 2.5 (P3), 2.6 (but the sample is the month's
> plan, below), 2.7, 2.8; `animals.md` 1, 2 (PR-A, done), 4 (why the acts stay after the update) and 6. Decisions:
> X41 to X53.

## 0. In short

- **One law for every body** (X6): a body's update runs its whole interval at once, as a *plan* worked out on numbers
  (no store is touched), then one *booking* of that plan (X41). Every body runs the same plan; a stand and a body with
  a brain differ in their reserve time `D`, their heat stage and whether they sweat (section 5).
- **Rates, not pulses.** Everything a body does between two updates is a rate over the interval. The only pulse is what
  it holds at the start, which for an eater holds the food it took at its last update (X42).
- **Two parts, one plan.** The fuel stock keeps X9's closed form: income with product inhibition, upkeep with the
  release limit, growth above the reserve, deaths by starving at a rate. The body's other reactions (catalysed and
  bare, with their true orders: the dissolved mineral's return to the solid is third order) run in **sub-steps** by the
  reactor's own kernel: roots and soaking first, then growth (capped by what its inputs hold plus their inflow), then
  the reactions; a sub-step ends where a store's net loss would reach 0.3 of it or the fuel's form changes phase (X42).
- **Bare reactions stay** inside bodies (X43, decided). PR-A's two causes, the eaten mineral that turns solid before
  growth and the body burnt between two updates while growth rebuilds only at the update, are a pulse meeting rates; in
  the sub-steps growth runs beside them.
- **Over a long interval** growth keeps the upkeep of the rest of the interval besides its reserve, and starving is
  judged on the body's path, sub-step by sub-step, so a dip that recovers kills as it does over short updates (X53).
- **One window `D`** for every rate read from Change's law inside a body, never the update's length (X44): 36 days for
  stands (`Physics::reserve_days`, decided by PR-M's measure), the `INTERVAL` gene for bodies with a brain. The window
  pins today's rate law; it is an artifact kept on purpose, so the world stays today's at today's pace.
- **The booking books the plan's net change per substance in one pass**, with exact sums, as the reactor does (X52).
- **Stands stand at their ground's temperature** as the month's last stage; P1.3 is folded into P1.2 (X45).
- **The reference is the month's own splitting test**, now with young stands, young animal nodes, a dip that recovers,
  a seasonal ramp of light and temperature, and long intervals centred on the summer's peak and the winter's low (X47).
- **Cost:** the stand's law about 4.5 to 5 µs (PR-M's v2: 7.0, without bare reactions; today 10.6), a sample run
  about 2.5 to 3 µs (about 1.1 µs on average), a whole stand update about 10 to 10.5 µs at 30 days and about 9 after P3.
  Section 5 of the plan assumed 6 µs (X46).
- **Gain now** (one thread, instructions of a year): P1.2 with P1.4 island -9 to -11%, globe -4.5 to -5.5%; with P2
  about -19% and -9%. P1b, an estimate with its sub-steps counted: an animal update -8 to -16% at the gene's pace, the
  year -2.5 to -5% and -3 to -6%.

## 1. What the probes and the review found, and the answer to each

| Finding | Answer here | Where |
|---|---|---|
| PR-M: a stand costs 7.0 µs; the cost is in the stages around the closed form | lean stages, a plan that books nothing for the sample | 7 |
| PR-M 1: the release limit read from one day's breath makes many cool stands release-limited; today's window is `max(days, interval)` | every rate from Change's law is read over the window `D`, then made a rate a day | 3.3 |
| PR-M 2: the light reaction's catalyst limit grows as the square of the interval | the same window `D`: the rate a day is fixed by the body, not by the update | 3.3 |
| PR-M 3: the light reaction's liquid is a store; over 36 days it needs many times what a stand holds | the liquid is a supply soaked in through the interval | 2, stage 2 |
| PR-M 4: the activity lags; a dry stand soaks to its level in a day | the liquids come first; the activity of the interval follows from them (one mean: an approximation) | 2, stage 2 |
| PR-M 5: product inhibition read at the start breaks the splitting test | inhibition is a term of the fuel's closed form, on the product substance | 3.1 |
| PR-M 6: roots and soaking are capped by one room per update | fluxes toward the level, before growth in each sub-step; growth at an input's minimum runs at its inflow | 3.2 |
| PR-M 7: sweating drained a stand's liquid in one long update | stands do not sweat | 5 |
| PR-M 8: heat booked by `energy_at` broke the books; growth's heat moved the land -0.52 K; the light's rounding | one place sets the heat, after growth; the bounds of today kept; the rounding counted | 4 |
| PR-M 9: the death rule is a law change for stands too | the rate `(1 - kR/U) / D` for every body (X14) | 3.1 |
| PR-M: `D` = gene moves S50 by +0.010 past its noise | stands take `D` = 36 days (decided, X44) | 3.3 |
| PR-A (a) to (d): no effect | kept as law choices (rate deaths, reserve by `D`; `away` and the movers by the gene in G2) | 5.1 |
| PR-A (g): the eaten mineral turns solid before growth uses it | growth runs beside the third-order return in the sub-steps | 3.2 |
| PR-A: adults burn their body all interval and rebuild only at the update | the same: growth repairs the body as it burns, sub-step by sub-step | 3.2 |
| PR-A: 20 days fails too; young-only keeps the eaters once the mineral is kept | G2's fallback is the young-only floor, if P1b keeps the mineral | 5.1 |
| PR-A2 (pending) | its place | 5.2 |

## 2. The law in physical words, and its stages

A body's update covers the `Δ` days since its last. It works per member: deaths take a share of every store (`thin`),
so they do not change what a member holds. Forcing over the interval is an exact total where the world keeps one (light,
rain, and the ground's temperature: a running total of each cell's ground temperature over time is kept beside
`w.light`, with the time of its last addition, and a stand keeps its mark of it as it keeps its light's, so a stand's
rates are read at the mean of its ground's temperature over its interval: the total's change over the time it covers
since the mark (exact to one flow step, as the total moves only at flow steps; a 90-day update centred on the summer's
peak has its two ends about 2 K below it, so the mean of the ends is not enough). The mark is set as `mark` and
`rained` are: at birth, at worldgen, and at the brain-loss switch. The new number per cell and the new field of `Body`
come under P1.2's snapshot bump. **Rates read at the mean temperature are not the mean of the rates** (Arrhenius'
curve is convex): an approximation, which the splitting test's points centred on the summer's peak and the winter's
low check. A body with a brain reads its rates at its own temperature at the update's start, as today. The air's shares and the ground's liquid are read at the
start. The card (3.4) holds what the genes fix.

| Kind | What |
|---|---|
| Pulse, at the interval's start | what the body holds when its update starts: for an eater, the food it took at its last update (digested into fuel at the take, `Body::digest`) |
| Exact total, spread evenly | light caught (`w.light`, the marks moved by shade at each air step), rain caught |
| Rate, read at a sub-step's start | the body's reactions, roots and soaking, growth's draw on its inputs |
| Rate over the interval, in the fuel's closed form | the light reaction (affine in its product by inhibition), upkeep, keep-warm's power (bodies with a brain), the release limit, growth's allowance, deaths by starving the fuel |
| Judged on the path | starving (`starved`, per sub-step), wear (a rate) |
| Rule on the end state | minimums (`LACK`), frost and boiling (`PHASE`), children, shedding, maturing |

**The stages, in order.** Stages 0 to 5 are the *plan*: they read the body, its card and the store's view, and write
nothing; they return what the booking needs, the new light and rain marks (`mark`, `rained`) and the light share
(`keep`) with it. Stages 6 to 8 book. The selection sample stops after stage 5 and drops the marks (X41).

| Stage | What | Kind |
|---|---|---|
| 0. Site | cell, air node, the temperature its rates are read at (a stand: its ground's mean over `Δ` from the running total and its mark), depth, cover, `Δ` (`live::site`) | plan |
| 1. Supplies | light caught, rain caught, the breath pool of the gases its reactions touch (`gas_pool`, capped by the air node), the soil, the light set aside in the bodies' node (`keep`, capped by what it holds), the heat held for it (`Draw::owed`) | plan |
| 2. Liquids | the key liquid's level where soaking meets what the pores pass (`through`); the activity of the interval, one mean (where the liquid's own time is over a tenth of `Δ`, about a quarter of the updates, the mean of the approach: an approximation, as activity is a power of the level); what the light reaction's liquid can bring; a stand's evaporation of its key liquids within today's `heat_bound` (no sweating) | plan |
| 3. Rates | upkeep `U` (`pay::upkeep`, plus keep-warm's power for a body with a brain), the release limit `k`, the light reaction's limits and inhibition, the Arrhenius factor of every reaction the body runs that is not negligible over `Δ` (the reactor's `NEGLIGIBLE` rule, at the interval's temperature), growth's allowance and what a kg takes of each input, wear | plan |
| 4. Sub-steps | the fuel's closed form and the other reactions with growth among them, sub-step by sub-step (3.1, 3.2) | plan |
| 5. End rules | on the plan's end stores and path: the shares that die by starving the fuel (`SPENT`, the rate), by the body's path (`starved`, `SPENT`), by wear (`AGE`), by minimums (`LACK`, `starve`), by frost and boiling (`PHASE`, at the temperature stage 7 will leave: the ground's for a stand); the selection gain (`buildable` at the end over at the start) | plan |
| 6. Booking | the net change per substance, in one pass (section 4) | book |
| 7. Heat | stands: at the ground's temperature; bodies with a brain: the day law's heat network with the month's released heat (`physics` without its reactor; G3 may change it) | book |
| 8. Deaths, then after | the shares of stage 5, by `thin`, at the state stage 7 left (so the dead leave at that temperature); then children (`copy::copy`), shedding, maturing, split, `see`, schedule | book |

## 3. The plan's two parts

### 3.1 The fuel: X9's closed form

The **fuel stock** `R` is `Body::fuel`, the energy its burnings can release from its stores: a sum over substances (in
chemistry 0 the light reaction's product; a body that catalyses the burning of its matter counts its matter too). Within
a sub-step the substances other than the light reaction's product are held, so `R` follows

    dR/dt = P(R) - min(U, k R) - G

- **Income** `P`: the least of the light caught a day x `light_yield` / its enthalpy, the breath's carbon gas a day, its
  liquid supply a day, and its catalyst's rate `c (1 - x_p / x_p*)`, `x_p` the product substance and `x_p*` its level
  (product inhibition, laws.md 2.4). With the other substances held, `x_p` is affine in `R`, so the last limit is affine
  and its knee with the others is a phase boundary (Blackman 1905, with inhibition).
- **Upkeep** `U` while `k R >= U`, `k R` while release-limited; the burnings pay it in `Consts::burn` order with the
  shares `pay::plan` gives at the sub-step's start.
- **Growth** `G`: the allowance while `R` is above the **reserve** `R_res(t) = U (D + Δ - t)`: `D` days of upkeep plus
  the upkeep of the rest of the interval, which over short updates would come from food not yet taken (X53). On that
  moving boundary growth takes the income, `G = P`, so over the interval growth spends what short updates spend in
  total. Before the fuel's form is solved for a sub-step, `G` is set to the least of the allowance and what growth's
  inputs allow over that sub-step (3.2), so the two agree.
- **Deaths** while release-limited, at `μ(t) = (1 - k R(t) / U) / D` (`SPENT`): the share `1 - e^(-∫μ dt)`.

Each phase is `dR/dt = a - b R`, solved exactly; the times to the reserve, to `U / k` and to the knee have closed
forms. This is today's labile pool (Thornley 1972; Sitch et al. 2003; Kooijman 2010) with its limits inside it.

### 3.2 The other reactions, in sub-steps

The body's other reactions are not linear: in chemistry 0 the dissolved mineral turns back into the solid by a third-
order law (reaction 23, `i = [[23,3]]`: PR-A's 46% lost after 9.7 days and 77% after 27 days fit it; a first-order law
would lose over 99%); the solid and the dissolved mineral are a two-way pair (22 and 23); the bare burning of matter (21)
feeds the minerals 24 and 10 at a rate that follows the matter; fixing (24) draws on the fuel and is inhibited
by its own product. So they run as the reactor runs them: **in sub-steps, each reaction's rate read at the sub-step's
start and applied as `Reactor::apply` does** (its order's power, `gone`; each limited by what is left), at the
interval's temperature, with the Arrhenius factors of stage 3. In each sub-step, in this order:
1. **Inflows first:** roots and soaking bring the minerals and the liquid toward their level at the soaking rate
   (today's exact form of `roots` and `soak`, now a flux, no room per update).
2. **Growth:** `G` is the least of the allowance and, for each input, what it holds above its safe minimum plus the
   sub-step's inflow, over the sub-step's length; the fuel's form is then solved with that `G`. **At an input's
   minimum growth runs at that input's inflow rate** (it stays there while the inflow lasts), so a store that roots refill
   never throttles growth to one room per sub-step (PR-M 6).
3. **The other reactions**, each limited by what is left, `gone` with its order's power. Within a sub-step `gone`
   integrates a reaction that runs alone on its scarcest input exactly, whatever the length (the third-order return of
   23 too); only its competition with growth and the other reactions is held at the start.

**A sub-step ends** where a store's **net loss** (what growth and the reactions take of it, less its inflow and what
reactions make of it) would reach `MONTH_SHARE` = 0.3 of the store at its starting rates, or where the fuel's closed form
changes phase (growth starts or stops, the release limit begins); at most `MONTH_STEPS` = 16, the last takes what is
left (both named `const`s). As in today's reactor (`far`, `rest`), what a store loses ends a sub-step, never what it
gains: **a store that only grows ends no sub-step** (the products of the bare reactions that `shed` empties at each
update, a dry key liquid soaking in), **unless a running rate reads it** (a product that inhibits its reaction, the solid
10 that feeds reaction 22); then its change is measured against the larger of the store and its level. A counter counts
the sub-steps, those ended by a growing store that a rate reads, and those cut by the cap. A store near its level, where
roots bring what growth takes, ends no sub-step. The fuel's closed form runs inside each sub-step with the other substances held, so its phases stay exact.
This is the reactor's kernel at a fixed temperature, not a second reactor.

**How many sub-steps.** A stand: its minerals sit near their level (roots refill what growth takes), its pair of 10 and
23 near its balance, its matter's bare burning about a thousandth a day; the fuel changes phase in about 10 to 15% of
the updates (P0: the stock crosses its reserve in 10%, runs low in 0.5 to 1.4%). So about 1.2 sub-steps a stand update
on average, rarely over 3; P1.2 counts them. An eater: the eaten mineral 23 falls 46% in 9.7 days and 77% in 27 (PR-A),
mostly at first. Its return is third order, so a sub-step that loses 0.3 at its starting rate keeps about
`1 / sqrt(1.6)`, 0.79, of the store, not 0.7 (`gone`); it takes about `ln(x0 / x) / 0.236` sub-steps: about 3 at the
gene's pace (about 9.5 days) and about 6 at 30 days, with the fuel's phase changes about 4 and about 8.

**Why it answers PR-A.** Today a body's reactor turns its stores through the whole interval and growth acts once, at
the update: the eaten mineral turns solid and the body is burnt (about 0.04 kg a member a day) with nothing rebuilding
it, so at a 30-day pace an adult is under its copy mass when `copy` runs. In the sub-steps growth takes the mineral as
it turns and repairs the body as it burns. Whether this is all of the adults' loss is PR-A2's question (5.2).

**Starving on the path** (X53). `starved` keeps the share `x^a` of the members when a member's body fell to `x` of what
it was below its copy mass; a body that dips and recovers over short updates loses members at the dip and does not get
them back. So it is applied **per sub-step** on the body's path (the sub-step ends where growth stops, so the path is
monotone inside one), not once on the interval's ends: a dip that recovers within a long update kills as it would over
short ones. 97 to 99% of the eaters' deaths are of this kind (PR-A), so P1b's splitting points include such a cycle.

**Growth's split** (one rule): of what growth builds, matter fills up to maturity as it stands at the interval's end;
past that the share `ALLOC` goes into the brood; the built solids first by their shares (`copy::grow`'s order).

**What is held at the start of the interval:** the living body (so upkeep, the allowance and `x_p*`), members, the
temperature (a stand: its ground's mean over the interval; a body with a brain: its own at the start). The splitting test bounds what that costs.

### 3.3 The window `D`

Change's rate law inside a body (`change::pace`, `frac`) is read on what `D` days bring (breath, soaking), and its yield
over `D` is made a rate a day: the release limit `k` (as `pay::plan` gives it over `D`, as a first-order rate,
`k = -ln(1 - released / R) / D`), the light reaction's catalyst rate `c`, each other reaction's rate. **This window is an
artifact of the rate law, kept on purpose.** Contact in that law grows with the breath pooled over the time it is read
on, so a rate read over `D` days is about `D` times its one-day value a day: the light reaction's catalyst limit over
`D` = 36 days is far higher than PR-M's prototype read it (one day's breath, times the days). The window pins the rate
law at today's value: at today's pace the update is `D`, so the world is today's. P1.2 measures the world with it
against PR-M's numbers (where the light or the gas limits, not the catalyst, the two agree).

`D` is `Physics::reserve_days`, 36 days, for every stand (decided by PR-M: `D` = gene moved S50 by +0.010 in every
seed, past its noise) and the `INTERVAL` gene in days, not rounded to batches, for a body with a brain. A body that
loses its brain (X15) takes `reserve_days` from its next update.

### 3.4 The card

`Body::k` (`Consts`), worked out from the genes at every change of the genes (`prepare`), gains what the month reads
that depends only on the genes: the life roles (PR-M: `life_roles` costs 430 instructions a call), the burnings' order
and energies (there), the fuel and work a kg of growth takes, the levels' shares, the catalysts' factors, the tissue
factor, `D`. What depends on the temperature or the air is worked out per update (per cell from P3, which helps only
after P3).

## 4. Booking and the books

The plan holds every reaction's extent over the interval and every supply taken. The booking (X52):

1. **Caps.** The supplies were in the plan's limits: the breath pool (air node), the liquid and the roots' minerals
   (ground node), the light set aside (bodies' node). If the plan's total take of a node's substance is over what it
   holds (a gas several reactions breathe; the ground's liquid or mineral that soaking and roots both take), stage 4
   runs once more with that cap as a limit (X29's second solve). What still exceeds it is cut by one share for each
   reaction as a whole, in a fixed order (X21's way), and counted; between two cut reactions this cut can pass through a
   negative store, as extents one after another do: rare, counted, and caught by the debug check.
2. **One pass of the net change.** Each substance of the body changes by the sum over all reactions and flows, summed
   exactly where terms cancel (`Reactor::made`, `math::add_times`), so every element is kept; the reactions' heat as one
   sum; the net gas to and from the air through `Moves::gas` with its heat and latent heat, the latent heat of gas put out
   paid only as far as heat pays (`settle_gas`'s bound, so a body is never pushed below `T_FLOOR`); soaking and roots by
   `soak_in` (their heat and fusion enthalpy, which the ground pays); the light taken from the bodies' node, capped by
   what the node holds (`store_light`'s rule; what rounding leaves below zero is set to zero and counted by
   `store::apply`, as today: PR-M saw 8e-3 J a year); a stand's evaporation within `heat_bound`.
3. **Heat** (stage 7). For a stand: every move above brought or took its own heat by the shared helpers; then, once,
   its heat is set to `Body::energy_at(T_ground)`, the difference taken from what its bodies' node holds for it, then
   from the ground (never below `T_FLOOR`), or given to the ground, as `relax_heat` books its `give`; if the ground
   cannot pay, the stand ends colder, as a body does today. Its draw until its next update is set as `relax_heat` sets
   it (`Draw::after`). Growth is in step 2, so its heat goes the same way as the rest (PR-M's v1 did so: land +0.13 K;
   v2 left it in the stand: -0.52 K).
4. **Deaths** (stage 8), at the state step 3 left.

A debug check holds that no store ends below zero before rounding; `counts.rounded` stays rounding only. Booking the net
change cannot pass through a negative store between two extents, as one extent after another can (one reaction's
output that another one needs, an outflow before an inflow). P1.1's helper (`apply_extent`) stays for `pay_at`; the
month books through one net-change helper beside `Reactor::made`.

## 5. Which body runs what

**A body without a brain is a stand, whatever its count** (`Life::brainless`; such a body is a node and dies below one
member). Every other body, node or individual, is a body with a brain.

| | Stand | Body with a brain |
|---|---|---|
| Plan and booking (stages 0 to 6, 8) | the month | the month |
| `D` | `reserve_days` (36) | its `INTERVAL` gene |
| Income | the light reaction | the light reaction if its catalyst is on (none of today's eaters) |
| Upkeep | `pay::upkeep` | `pay::upkeep` plus keep-warm's power (X14) |
| Evaporation | key liquids, in stage 2, within `heat_bound`, no sweating | in the heat stage, with sweating at `COOL_T` (until G3) |
| Heat (stage 7) | at its ground's temperature | the day law's heat network with the month's heat |
| Temperature its rates are read at | its ground's mean over the interval (the running total) | its own at the update's start |
| Frost reads | the temperature stage 7 leaves (the ground's, or colder if the ground could not pay) | its temperature after the heat network, as today |
| Selection sample | the plan for the sample's genes (below) | none |
| After the update | children, shed, mature, split | the decision (`choose::act`), then the same |

**The sample.** The node's own plan keeps stage 0 and stage 1 (stage 1 is made again only if the sample's gene is read
there: leaf, gas). The sample copies the node into a scratch `Body` kept in `LiveTmp` (no allocation after the first),
changes its gene, runs `prepare` and stages 2 to 5, and keeps only the gain. About 2.5 to 3 µs a run.

**If a climate test fails** (P1.2's fallback, X45), a stand keeps its own temperature by `relax_heat`'s closed form and
its rates are read at the temperature it holds, its own, as for a body with a brain.

### 5.1 Animals (P1b)

P1b gives the month to every body. In a body with a brain it replaces `settle_upkeep` (and its per-update death of the
unpaid share), the body's reactor in `physics` (`Reactor::react` leaves body updates), `settle_gas`, `store_light`,
`keep_warm`, the paths of `soak` and `roots`, `copy::grow`'s conversion and its paying, and `soak_again`. What stays of
`physics` for them: rain, evaporation with sweating, the heat network (`relax_heat`), fed the month's released heat.
The acts stay after the update (`animals.md` 4).

What P1b answers of PR-A: (a) deaths by the rate; (b) the reserve by `D` (now with the rest of the interval, X53); (g)
and the adults' burnt body by the sub-steps (3.2). What G2 adds: `away` and the movers by the gene's interval, learning's
discount, an act's heat. PR-A's check is P1b's done-when: its script again with P1b at a 30-day floor, the eaters within
the seed noise, its counters read (adults under their copy mass at `copy`, the mineral left at growth). **If it fails
but the mineral is kept** (the counter of the mineral left at growth at the gene pace's level), G2 takes the floor for
young nodes only (PR-A: within the noise once the mineral is kept; about 0.6 of the node updates). **If the mineral is
not kept either**, the young-only floor is not safe (PR-A measured it with the mineral kept): G2 waits for that cause
(PR-A2, or a fix of the sub-steps) and is not built until then (X48).

**G3 after P1b.** The month already takes the reactor, upkeep, keep-warm and growth out of an animal's day; what is
left is the heat network and the evaporation. G3 is gated again on P1b's profile (X49).

### 5.2 PR-A2: the adults' root cause (pending)

**Waiting for PR-A2's result; this section is replaced by it.** What each answer would mean here:
- **The cause is the timing of the body's own reactions between updates** (the bare burning of matter, the mineral
  turning solid, growth only at the update): the sub-steps answer it; P1b as written.
- **The cause is the birth rule** (children only at an update; an adult under its copy mass then makes none): the
  month knows the brood's path; children would be made from the brood the interval built, whether or not the mean
  member is at its copy mass at the update's end. A small change in `copy::copy`, its own law commit before G2.
- **Another cause:** a step of its own before G2, named by PR-A2, with its law change and the same 30-day check.

## 6. The checks

1. **The splitting test** (the month's reference, X47): one update of 60 days against six of 10, and 36 against six of
   6 (G9a: 90 against three of 30); members, fuel and body matter, the 90th percentile under 2%, the worst under 10%.
   Its points: PR-M's grid (3 adult stands; light 0 to 250 W/m², ground 275 to 305 K, wetness 0.1 to 0.9, fuel 0.3 to
   10 times `D` of upkeep) **and young growing stands**, each at constant forcing and with a **seasonal ramp** (the
   light and the ground's temperature of a spring and of an autumn month, as the air would give them), **and intervals
   of 60 and 90 days centred on the summer's peak and on the winter's low** (where Arrhenius' curve and the mean
   temperature matter most; G9a's 90 days is that case); P1b adds **young
   and adult animal nodes**, also at 260 K, with food pulses, and **a dip that recovers** (food short for a third of
   the time, then plenty).
2. **Against the day law** (a sanity check): members and body matter of adult stands that pay, at 2 days, the 90th
   percentile under 5% (PR-M: 3.6% and 3.7%). Not fuel: the day law is not converged there itself (PR-M: 33% at 2 days).
3. **Key numbers** at level 3 against the references after AIR4.0; S50 and S84 not past their seeds' noise; the land's
   temperature reported (a move over 0.3 K is looked into before merging); the window's effect on the light reaction
   (3.3) read against PR-M's runs.
4. **Books:** 1e-12 in the century (slow suite).

**Kept tests** (few, fast, `worldgen::testkit`): `a_months_stock_splits_alike` (four points: light-limited, inhibited,
release-limited at 275 K, a young stand on a ramp; 2%); `a_starving_body_dies_at_its_own_time` (`1/D` a day for an empty
stock and no growth, whatever its updates); `a_months_stands_keep_the_books` (a cell's stands over a month, 1e-12,
never more liquid than the ground holds); P1b adds an animal node at 260 K and a dip that recovers to the first. The
switch test of X15 stays.

## 7. Cost per stand, how it is reached, and what it gains

PR-M's v2 (island22, one thread, instructions a call of the stand's law) ran **no bare reactions**; today's
`Reactor::react` inside the stand's law is about 19,000 instructions a call (10.0% of island22's year), on about ten
bare reactions a stand update (P0). The month runs the same reactions (the reactor's `NEGLIGIBLE` rule freezes few at
living temperatures: the frame's bare rot, 26, only below about 190 K); it saves by reading each Arrhenius factor once
per update (once per cell after P3, not before), by no phase search per sub-step (the temperature is held) and by few
sub-steps: about 1,500 to 2,500 instructions for the factors and 1,000 to 1,500 a sub-step, about 1.2 sub-steps a stand
update on average (3.2).

| Stage | v2 | target | How |
|---|---|---|---|
| evaporation | 4,400 | 1,200 | key liquids only, no sweating, one closed form |
| deaths (two `lose_share`) | 3,400 | 1,500 | one pass for all causes |
| selection gain (`buildable` twice) | 2,400 | 1,200 | the end on the plan's stores |
| growth | 3,400 | 1,500 | in the sub-steps and the net booking; no `Payer`, `plan`, `spend` |
| upkeep's burnings and the release limit | 3,800 | 1,200 | `k` from the window, inside the fuel's form |
| `check_stores` | 2,000 | 1,800 | on the plan's end stores |
| gas pool | 1,700 | 900 | only the gases its reactions touch |
| soil, soaking, roots | 3,500 | 2,000 | one soil; fluxes in the sub-steps |
| activity (3 times) | 1,200 | 400 | once, after the liquids |
| heat | 2,400 | 1,500 | `energy_at` once |
| site, films, roles | 2,000 | 1,500 | roles on the card |
| the rest, the fuel's form | 3,900 | 3,500 | |
| net booking (new) | 0 | 1,500 | one pass, exact sums |
| **the other reactions in sub-steps (new: v2 had none)** | **0** | **3,000 to 6,000** | Arrhenius once, no phase search, about 1.2 sub-steps (up to 3) |
| **total** | **37,900 (7.0 µs)** | **about 23,000 to 26,000; with a margin for what the list misses, 4.5 to 5.0 µs** | at P1.2's line of 5 µs (its kill line is 6 µs) |

**The sample** today clones the body, runs its whole update in a checkpoint and rolls it back (2.3 µs a stand update on
average: one update in two, and `read` skips most). Here: a copy into a scratch body (about 1,000), `prepare` (about
3,000), stages 2 to 5 (about 11,000): **about 2.5 to 3 µs a run**; at every update (P1.4) with `read`'s skips (about 40%
of updates run one) **about 1.1 µs on average**.

**A whole stand update**, island22: today 18.0 µs (the law 10.6; children, shedding and soaking again 3.3; the sample
2.3; maturing 1.0; the rest 0.7). After P1.2 with P1.4: about 4.75 + 1.1 + 2.8 + 1.0 + 0.7 = **10 to 10.5 µs**; P3 takes
about 1.5 µs (site per cell, faces at the update, no ring): **about 9 µs**. At 60 and 90 days the sub-steps and the
children add about 0.5 to 1 µs an update.

**Per stand and year** (today 10.6 updates of 18.0 µs, 191 µs): P1.2 (`field_days` 30, 12.2 updates of about 10.4 µs):
about 127 µs (x0.66); P2 (60 days, about 11 µs): about 66 µs (x0.35); P3: about 57 µs; G9a (90 days): about 42 µs.

**Gain on one thread** (plant updates are 30.0% of island22's year and 14.7% of globe36's, R2):

| Step | island22 | globe36 | the plan said |
|---|---|---|---|
| P1.2 with P1.4 | -9 to -11% | -4.5 to -5.5% | -12% and -8% (P1.1 to P1.4) |
| P2 | -9 to -10% | -4.5% | -7% and -3% |
| P3 | -3 to -5% | -2 to -3% | -5% and -3% |
| P1b (an estimate: an animal update -8 to -16%, below) | -2.5 to -5% | -3 to -6% | about 0 (a law step) |

**P1b's estimate, with its sub-steps.** An animal update today, island22: 37.9 µs, of it the body's day 18.0. The month
takes out `settle_upkeep`, the reactor (about 19,000 instructions), `keep_warm` and growth's paying: about 50,000 to
55,000 instructions. It puts in the plan and the booking (about 20,000 to 25,000, as a stand's without its liquids'
closed form) and its sub-steps: at the gene's pace about 4 (4,000 to 6,000 instructions), at 30 days about 8 (8,000 to
12,000). So an animal update -8 to -16% at the gene's pace; at G2's 30 days the sub-steps cost about 4,000 to 6,000
instructions more an update, which G2's count of updates (x0.30) covers many times. P1b counts its sub-steps; if an
eater needs more than 10 at 30 days, the third-order return gets a looser rule (`gone` already integrates it exactly when
it runs alone).

P1.2's kill line (under 7% of island22's year) still has 2 to 4 points of room. On four threads (plan 5.0): the
plants' rows give about 14 ms less than the plan on the island and 7 on the globe; P1b moves part of G3's gain earlier.

## 8. Risks

| Risk | Answer |
|---|---|
| The splitting test fails (the living body held at the start of 60 to 90 days; Arrhenius' curve over a season) | `MONTH_SHARE` smaller; else P2 or G9a keeps the shorter pace |
| The stand's law over 5 µs (bare reactions, sub-steps) | P1.2's kill line is 6 µs, where its gain line (7% of island22's year) still holds; section 5 of the plan counts 10 to 10.5 µs a stand update |
| Sub-steps of a third-order return, a two-way pair, a rate that follows the matter, a product-inhibited fixing | the reactor's own kernel and its orders, rates read at each sub-step's start, ended at a net loss of `MONTH_SHARE`; the splitting test, with eaters' pulses of the mineral |
| More sub-steps than counted (stands over 3, eaters over 10 at 30 days) | the counters of P1.2 and P1b; a looser rule for a reaction `gone` integrates alone; the cost lines of P1.2 (6 µs) and P1b hold the step |
| The heat's path moves the land (PR-M: -0.52 K) | one place sets the heat, after growth; the land reported; fallback for stands: `relax_heat`'s closed form at their own temperature |
| P1b moves the eaters at today's pace (no dip between updates, deaths by the rate, the reserve with the rest of the interval) | announced as law effects, reported against the references; key numbers out and not accepted: not merged |
| PR-A2 finds a cause the sub-steps do not answer | 5.2; G2 waits for it or takes the young-only floor (only if the mineral is kept) |
| The window `D` gives the light reaction more catalyst than PR-M measured with | it is today's law at today's pace; P1.2 reads the key numbers with it |

## 9. Lines

`body/month/` (the plan, the fuel's form, the sub-steps on the reactor's kernel, the net booking, the scratch sample):
about +500 (PR-M's prototype was 392 lines with its probe code, without bare reactions or sub-steps), and about +15
for the running total of the ground's temperature and the stand's mark; P1.2 deletes
`gain_to_growth`, the sample's checkpoint path for stands and `node_days`'s branch (about -30); P1.4 deletes
`sample_every` (-12); P1b deletes `settle_upkeep`, `keep_warm`, the body's call of the reactor (`react`), `settle_gas`,
`store_light`, the body paths of `soak` and `roots`, `copy::grow`'s conversion and paying, and `soak_again` (about -230)
and adds the keep-warm drain and the heat hand-over (+60). Lane A net about **+290**; the core at the end about 11,110,
under the ceiling.

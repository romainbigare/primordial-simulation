# Speed x10, section "month": one month law for every body, after the probes

> **What this file replaces.** It is the design of the month law after phase P (probes PR-M and PR-A,
> `measures.md`). Where it differs from `plants.md` or `animals.md`, this file holds; the plan (`plan.md`) holds over
> all three. **In `plants.md` it replaces:** 2.3 (the stages of the plant month), the month's numbers in 2.4 (the
> card; the case against a grid table stays), the sample's cost in 2.6, 3 items 1 to 4 (the law changes), 5 tests 1 to 3,
> 6 (the gains) and the P1 row of 9. **In `animals.md` it replaces:** 3.1 to 3.4 (the law, the pulse, the booking,
> the update) and 5's two tests. **Kept there:** `plants.md` 2.1, 2.2, 2.5 (P3), 2.6 (but the sample is the month's
> plan, below), 2.7, 2.8; `animals.md` 1, 2 (PR-A, done), 4 (why the acts stay after the update) and 6. Decisions:
> X41 to X51.

## 0. In short

- **One law for every body** (X6): a body's update runs its whole interval at once, as a *plan* worked out on numbers
  (no store is touched), then one *booking* of that plan (X41). Stands and bodies with a brain run the same plan; they
  differ only in their reserve time `D`, their heat stage and whether they sweat (section 5).
- **Rates, not pulses.** Everything a body does between two updates is a rate over the interval: light, breath,
  soaking, roots, upkeep, keep-warm, its own reactions (catalysed and bare), growth, deaths by starving and by wear.
  The only pulse is what it holds at the start, which for an eater holds the food it took at its last update (X42).
- **The pools.** The stores that upkeep and growth draw on (the fuel stock, body matter, growth's other inputs) each
  follow a linear equation in each phase, and every rate that takes from or gives to a pool sits *inside* that
  equation: product inhibition, the release limit, the bare reactions, roots, growth. This is X9's stock equation made
  wide enough for what PR-M and PR-A found (X42).
- **Bare reactions stay** inside bodies, as first-order rates in the pools (X43). PR-A's two causes, the eaten mineral
  that turns solid before growth and the body burnt between two updates while growth rebuilds only at the update, are
  both a pulse meeting a rate; in the pools growth runs through the interval beside them.
- **One window `D`** for every rate read from Change's law inside a body: the release limit, the catalysts' limits,
  the bare reactions are read over `D` days, never over the update's length (X44). `D` is 36 days for stands
  (`Physics::reserve_days`, PR-M) and the `INTERVAL` gene for bodies with a brain. At today's pace this is today's law.
- **Stands stand at their ground's temperature** as the month's last stage; P1.3 is folded into P1.2 (X45).
- **The reference is the month's own splitting test** (1 x 60 days against 6 x 10, members, fuel and matter, 2%),
  not the day law, which is not converged itself (X47).
- **Cost:** the stand's law about 4 µs (PR-M's v2: 7.0; today 10.6), the sample a pure plan of about 1.5 µs, a whole
  stand update about 9 to 10 µs at 30 days and about 8.5 after P3. **Not the 6 µs section 5 assumed:** about
  +10 ms on the island's year at the end, +4 on the globe (X46).
- **Gain now** (one thread, instructions of a year): P1.2 with P1.4 island -10 to -12%, globe -5 to -6%; with P2 about
  -19% and -9%. P1b also takes the body's reactor out of animal updates (an animal update -10 to -20%): about -3 to -6% and -4 to -7%.

## 1. What the probes found, and the answer to each

| Finding | Answer here | Where |
|---|---|---|
| PR-M: a stand costs 7.0 µs; the cost is in the stages around the closed form | lean stages, the plan without booking for the sample | 7 |
| PR-M 1: the release limit read from one day's breath makes many cool stands release-limited; today's window is `max(days, interval)` | every rate from Change's law is read over the window `D`, then made a rate a day | 3.2 |
| PR-M 2: the light reaction's catalyst limit grows as the square of the interval | the same window `D`: the limit is a rate a day, fixed by the body, not by the update | 3.2 |
| PR-M 3: the light reaction's liquid is a store; over 36 days it needs many times what a stand holds | the liquid is a supply soaked in through the interval | 2, stage 2 |
| PR-M 4: the activity lags; a dry stand soaks to its level in a day | the liquids come first; the activity of the interval follows from them | 2, stage 2 |
| PR-M 5: product inhibition read at the start breaks the splitting test (fuel 90th percentile 466%) | inhibition is a term of the fuel pool's equation | 3.1 |
| PR-M 6: roots and soaking are capped by one room per update | roots and soaking are fluxes toward the level, inside the pools (capped by the ground) | 3.1 |
| PR-M 7: sweating drained a stand's liquid in one long update | stands do not sweat (`COOL_T` is not read for them) | 5 |
| PR-M 8: heat booked by `energy_at` broke the books; where growth's heat goes moved the land -0.52 K | one place sets the heat, at the end, after every move brought its own heat; growth is booked before it | 4, stage 7 |
| PR-M 9: the death rule is a law change for stands too | the rate `(1 - kR/U) / D` for every body (X14 (a)) | 3.1 |
| PR-M: `D` = gene moves S50 by +0.010 past its noise | stands take `D` = 36 days (`reserve_days`); X14 closed | 3.2 |
| PR-A (a) to (d): no effect | kept as law choices (rate deaths, reserve by `D`, `away` and movers by the gene, G2) | 5.1 |
| PR-A (g): the eaten mineral turns solid in the body before growth uses it | growth runs through the interval beside the bare reaction, in the pools | 3.1, 5.1 |
| PR-A: adults burn their body all interval and rebuild only at the update | the same: matter is a pool, growth repairs it as it is burnt | 3.1, 5.1 |
| PR-A: a 20-day floor fails too; a floor on young nodes only keeps the eaters (with (g) fixed) | G2's fallback is the young-only floor | 5.1 |
| PR-A2 (pending): the adults' root cause and the smallest law change | the place for it | 5.2 |

## 2. The law in physical words, and its stages

A body's update covers the `Δ` days since its last (`Δ = day - last`). It works per member: deaths take a share of
every store (`thin`), so they do not change what a member holds, and they are applied at the end as shares. Forcing
over the interval is an exact total where the world keeps one (light, rain), else its value at the update's start
(temperature, the air's shares, the ground's liquid). The card (3.3) holds what the genes fix.

**Rates and pulses.**

| Kind | What |
|---|---|
| Pulse, at the interval's start | what the body holds when its update starts: for an eater, the food it took at its last update (digested into fuel at the take, `Body::digest`) |
| Exact total, spread evenly | light caught (`w.light`, the marks moved by shade at each air step), rain caught |
| Rate, read at the start, constant over the interval | breath, soaking, roots, upkeep, keep-warm's power (bodies with a brain), the release limit, the light reaction (affine in the fuel by product inhibition), the other catalysed reactions, the bare reactions, growth's allowance, wear |
| Rule on the end state | minimums (`LACK`), frost and boiling (`PHASE`), starving (`starved`, already the same in one update or many), children, shedding, maturing |

**The stages, in order.** Stages 0 to 5 are the *plan*: they read the body, its card and the store's view, and write
nothing. Stages 6 and 7 are the *booking*. The selection sample stops after stage 5 (X41).

| Stage | What | Kind |
|---|---|---|
| 0. Site | cell, air node, ground temperature, depth, cover, `Δ` (`live::site`); P3 does it once per cell, with each reaction's `k e^(-Ea/T)` at the ground's temperature | plan |
| 1. Supplies | light caught, rain caught, the breath pool of the gases its reactions touch (`gas_pool`, capped by the air node), the soil (liquids that soak, their rate), the light set aside in the bodies' node (`keep`), the heat held for it (`Draw::owed`) | plan |
| 2. Liquids | the key liquid's level over the interval where soaking meets what the pores pass (`through`; the mean of the exponential approach where its time is over a tenth of `Δ`, P0: a quarter of the updates); the activity of the interval; what the light reaction's liquid and the roots can bring as rates (capped by what the ground holds); a stand's evaporation of its key liquids (no sweating) | plan |
| 3. Rates | per member and per day: upkeep `U` (`pay::upkeep`; plus keep-warm's power for a body with a brain, X14), the release limit `k`, the light reaction's three limits and its inhibition, the other catalysed reactions, the bare reactions, growth's allowance and what a kg of growth takes of each input, wear | plan |
| 4. Pools | the pools' phases over `Δ` (3.1): what each reaction ran, upkeep paid, growth built (body, brood, built solids), the death rate's integral, the end of each pool | plan |
| 5. End rules | on the plan's end stores: the shares that die by starving the stock (`SPENT`, the rate), wear (`AGE`), minimums (`LACK`, `starve`), frost and boiling (`PHASE`), and `starved` (body lost and not rebuilt); the selection gain (`buildable` at the end over at the start) | plan |
| 6. Booking | in X29's order (section 4) | book |
| 7. Heat | stands: at the ground's temperature (below); bodies with a brain: the day law's heat network with the month's released heat (`physics` without its reactor; G3 may change it) | book |
| 8. After | children (`copy::copy`), shedding, maturing, split, `see`, schedule: unchanged | book |

**Stage 7 for stands.** Every booked move brought or took its own heat by the shared helpers (`soak_in`, `Moves::gas`,
`apply_extent`, `thin`). Then, once, the stand's heat is set to `Body::energy_at(T_ground)`; the difference is taken
from what its bodies' node holds for it, then from the ground (never below `T_FLOOR`), or given to the ground, as
`relax_heat` books its `give`. Its draw until its next update is set as `relax_heat` sets it, from what it drew and
what its evaporation asked (`Draw::after`). Nothing else writes its heat. Growth is booked in stage 6, so its heat
goes the same way as the rest (PR-M's v1 did so: land +0.13 K; v2 left it in the stand: -0.52 K).

## 3. The pools

### 3.1 The equations

A **pool** is a store upkeep or growth draws on: the fuel stock `R` (J its burnings can release, `Body::fuel`; in
chemistry 0 the light reaction's product), the body matter `M` (its brood with it), and each other input of growth
`x_k` (in chemistry 0 the two minerals). Within a phase each pool follows

    dx/dt = a - b x

with `a` and `b` constant in the phase:

| Term | Pool | Form |
|---|---|---|
| light reaction | `R` (in) | the least of: light caught a day x `light_yield` / its enthalpy; the breath's carbon gas a day; its liquid supply a day; its catalyst's rate `c (1 - R / R*)` (`R*` the fuel at its keep level: product inhibition, laws.md 2.4). The first three are constants, the last is affine in `R`: Blackman's limits (Blackman 1905) with inhibition, the knee between them is a phase boundary |
| upkeep | `R` (out) | `U` while `k R >= U`; `k R` while `k R < U` (release-limited) |
| a bare reaction, or a catalysed one other than the light reaction and the burnings | its scarce input's pool (out) and its products' pools (in) | first order in its scarce input if that is a pool (`b += κ`); a constant from the start amount if the scarce input is not a pool or is a supply; a product's level inhibits it as for the light reaction (fixing, laws.md 6.9) |
| roots, soaking | an input's pool (in) | toward its level at the soaking rate, `liq (level - x)` (today's exact form of `roots` and `soak`, made a flux: no room per update), so affine; capped over `Δ` by what the ground holds |
| growth | `R` (out, the fuel a kg takes and its work), `x_k` (out, `ν_k` a kg), `M` (in) | the allowance `g` while `R > R_res = U D` and every `x_k` above its safe minimum |

**Phase boundaries:** `R` meets `R_res`, `U / k` or the inhibition knee; an input `x_k` meets its safe minimum. Where the
flow points into a boundary from both sides, the pool stays on it and growth takes what keeps it there: at `R_res`
growth eats the net income (the stands' phase 2 of `plants.md`), at an input's minimum growth runs at what that
input's inflow allows. Each phase is solved exactly (`x(t) = x_eq + (x_0 - x_eq) e^(-bt)`, or a line when `b = 0`);
the time to the next boundary has a closed form. At most a few phases a call (a named `const` cap; a cap reached is
counted). The integrals of every term come out of the same forms.

**Deaths.** While release-limited, members die at `μ(t) = (1 - k R(t) / U) / D` (`SPENT`), the share
`1 - e^(-∫μ dt)` (X14 (a): a deep famine kills `1 - e^-u`, not `u`). In the limit the stock tends to `P / k` and the
rate to `(1 - P / U) / D`. Wear kills `1 - e^(-wear Δ)`, wear `= U / (built x e_life x 10^REPAIR)` a day (today's rule,
already a rate). The kinds of body with no income (every eater today) have `P = 0`.

**Growth's split.** Of what growth builds, matter first fills what maturity lacks plus what the bare reactions burnt
of the body over `Δ`; past that the share `ALLOC` goes into the brood; the built solids first by their shares
(`copy::grow`'s order). Totals over `Δ` need no boundary for this.

**What is held at the start** (not compounded inside `Δ`): the living body, so upkeep, the allowance and `R*`;
members; the temperature. The splitting test bounds what that costs (section 6); if it fails at 60 or 90 days, the
pools run in equal sub-steps of at most 30 days (stages 0 to 3 once, only stage 4 again).

**Why it answers PR-A.** Today a body's reactor turns its stores through the whole interval, and growth acts once,
at the update: the eaten mineral is half gone after 10 days and three quarters after 27; the body loses about
0.04 kg a member a day to its bare burning and is rebuilt only at the update, so at a 30-day pace an adult is under
its copy mass when `copy` runs. In the pools growth runs beside both: it takes the mineral as it decays and repairs the
body as it is burnt, so the end of a long interval is the end of many short ones, to the splitting test's 2%.
Whether this is all of the adults' loss is PR-A2's question (5.2).

### 3.2 The window `D`

Change's rate law inside a body (`change::pace`, `frac`) is read on what `D` days bring (breath, soaking), and its
yield over `D` is made a rate a day. This gives the release limit `k` (as `pay::plan` gives it over `D`, turned into a
first-order rate: `k = -ln(1 - released / R) / D`), the light reaction's catalyst rate `c`, and each other reaction's
rate (`κ`). The window is the body's own time and never the update's length, so a long update does not grow a rate
(the square of PR-M 2). At today's pace the update is `D` (stands: `node_days` 36; a body with a brain: its gene), so
the world is today's.

**`D`:** for a stand `Physics::reserve_days`, 36 days (today's reserve and death time of a stand; a `numbers.md` row);
for a body with a brain its `INTERVAL` gene in days, not rounded to batches. PR-M measured `D` = gene for stands:
S50 +0.010 in every seed, past its noise and away from its range; so stands take 36 (X44). `D` = gene for stands is
left as a law question for the owner. A body that loses its brain (X15) takes `reserve_days` from its next update.

### 3.3 The card

`Body::k` (`Consts`), worked out from the genes at every change of the genes (`prepare`), gains the month's numbers that
depend only on the genes: the life roles of the chemistry (PR-M: `life_roles` costs 430 instructions a call), the
burnings' order and energies (there), the fuel and work a kg of growth takes (`per_kg`), the levels' shares, the
catalysts' factors, the tissue factor, `D`. What depends on the temperature or the air (the Arrhenius factors, the
breath) is worked out per update (per cell from P3). The card is made by a function of the genes, not of a body, so
the sample makes its own card without cloning the body.

## 4. Booking order and the books

The plan says how far each reaction runs and how much each supply brings. The booking (X29), in a fixed order:

1. **Supplies were in the plan's limits already:** the breath pool (capped by the air node), the liquid and the roots'
   minerals (capped by the ground node), the light set aside (capped by the bodies' node). The oxidising gas all the
   burnings take, less what the light reaction makes, is checked against the breath pool; if a cap cuts it, stage 4
   runs once more with that cap as a limit (X29's second solve).
2. **Inflows:** soaking and roots (`soak_in`, with their heat and fusion enthalpy, which the ground pays), the light
   taken from the bodies' node, the gas taken.
3. **Extents, one after another, each capped by what is left of its inputs:** the light reaction; the other catalysed
   reactions; the bare reactions; the burnings for upkeep (`Consts::burn` order); growth (`Chemistry::builds`, then
   `Chemistry::build`). Each by `body::apply_extent` (P1.1): every element by the stoichiometry, its reaction heat to
   the body, its gases with their latent heat (`Moves::gas`).
4. **Outflows:** gases put out, with their heat and latent heat; a stand's evaporation.
5. **Deaths**, in one pass over the stores for all causes (`thin`, each cause's share and event).
6. **Heat** (stage 7).

It never relies on `add_store`'s clamp: a debug check holds that no store ends below zero before rounding, and a
counter counts the extents cut by step 3's cap after the second solve (X29). Every move is booked on both sides by the
shared helpers (CLAUDE.md 3.1); the drift stays at rounding (PR-M: elements 4e-16, energy 8e-15 a year).

## 5. Stands and bodies with a brain

| | Stand (no brain, a node) | Body with a brain (node or individual) |
|---|---|---|
| Plan and booking (stages 0 to 6) | the month | the month |
| `D` | `reserve_days` (36) | its `INTERVAL` gene |
| Income | the light reaction | the light reaction if its catalyst is on (none of today's eaters) |
| Upkeep | `pay::upkeep` | `pay::upkeep` plus keep-warm's power (X14) |
| Evaporation | key liquids, in stage 2, no sweating | in the heat stage, with sweating at `COOL_T` (until G3) |
| Heat (stage 7) | at its ground's temperature | the day law's heat network with the month's heat (G3 may give nodes a cheaper form) |
| Temperature its rates are read at | its ground's | its own at the update's start |
| Selection sample | the plan (stages 0 to 5) for the sample's card | none |
| After the update | children, shed, mature, split | the decision (`choose::act`), then the same |

### 5.1 Animals (P1b)

P1b gives the month to every body. In a body with a brain it replaces `settle_upkeep` (and its per-update death of the
unpaid share), the body's reactor in `physics` (`Reactor::react` leaves body updates), `keep_warm`, `copy::grow`'s
paying (`convert`'s `spare`, `Payer`, `plan`, `spend`) and `soak_again`. What stays of `physics` for them: rain,
evaporation with sweating, the heat network (`relax_heat`), with the month's released heat as today's upkeep heat.
The acts stay after the update (`animals.md` 4).

What P1b answers of PR-A: (a) deaths by the rate; (b) the reserve by `D`; (g) and the adults' burnt body by the pools
(3.1). What G2 adds: `away` and the movers by the gene's interval, learning's discount, an act's heat (as the plan says).
PR-A's check is P1b's done-when: its script again, with P1b, at a 30-day floor, the eaters within the seed noise, and its
counters read (adults under their copy mass at `copy`, the mineral left at growth). If it fails, G2 takes the floor
for young nodes only (PR-A: within the noise once (g) is fixed; about 0.6 of the node updates), not 20 days (PR-A: 20
days fails too) (X48).

**G3 after P1b.** The month already takes the reactor, upkeep, keep-warm and growth out of an animal's day; what is
left is the heat network and the evaporation. G3 is gated again on P1b's profile (X49).

### 5.2 PR-A2: the adults' root cause (pending)

**Waiting for PR-A2's result; this section is replaced by it.** What each answer would mean here:
- **The cause is the timing of the body's own reactions between updates** (the bare burning of matter, the mineral
  turning solid, growth only at the update): the pools answer it; P1b as written.
- **The cause is the birth rule** (children only at an update; an adult under its copy mass then makes none): the
  month knows the brood's path; children would be made from the brood the interval built, whether or not the mean
  member is at its copy mass at the update's end. A small change in `copy::copy`, its own law commit before G2.
- **Another cause:** a step of its own before G2, named by PR-A2, with its law change and the same 30-day check.

## 6. The checks

1. **The splitting test** (the month's reference, X47). On the grid of PR-M (3 stands, light 0 to 250 W/m², ground 275
   to 305 K, wetness 0.1 to 0.9, fuel 0.3 to 10 times `D` of upkeep), with the forcing held, one update of 60 days
   against six of 10, and 36 against six of 6: members, fuel and body matter, the 90th percentile under 2%, the worst
   under 10%. P1b adds two animal nodes (also at 260 K, where keep-warm is most of `U`) with food pulses; G9a adds 90
   against three of 30. A measure in `measures.md`; the kept tests are 3 points of it (below).
2. **Against the day law** (a sanity check, not the reference): members and body matter of stands that pay, at 2 days,
   the 90th percentile under 5% (PR-M: 3.6% and 3.7%). Not fuel: the day law is not converged there itself (PR-M: 33%
   at 2 days).
3. **Key numbers** at the plan's level 3 against the references after AIR4.0; S50 and S84 not moved past their seeds'
   noise; the land's temperature reported (PR-M saw it move with the heat's path: a move over 0.3 K is looked into
   before merging).
4. **Books:** 1e-12 in the century (slow suite).

**Kept tests** (few, fast, `worldgen::testkit`): `a_months_stock_splits_alike` (3 points: light-limited, inhibited,
release-limited at 275 K; 2%); `a_starving_body_dies_at_its_own_time` (`1/D` a day for an empty stock, whatever its
updates); `a_months_stands_keep_the_books` (a cell's stands over a month, 1e-12, never more liquid than the ground
holds); P1b adds the animal point at 260 K to the first. The switch test of X15 stays.

## 7. Cost per stand, how it is reached, and what it gains

PR-M's v2 (island22, one thread, instructions a call of the stand's law) and the target, from its list of where the
cost is:

| Stage | v2 | target | How |
|---|---|---|---|
| evaporation | 4,400 | 1,200 | key liquids only, no sweating, one closed form |
| deaths (two `lose_share`) | 3,400 | 1,500 | one pass for all causes |
| selection gain (`buildable` twice) | 2,400 | 1,200 | the end on the plan's stores |
| growth booking | 3,400 | 2,000 | extents from the plan; no `Payer`, `plan`, `spend` |
| upkeep's burnings and the release limit | 3,800 | 1,200 | `k` from the window, the burnings as extents |
| `check_stores` | 2,000 | 1,800 | on the plan's end stores |
| gas pool | 1,700 | 900 | only the gases its reactions touch |
| soil, soaking, roots | 3,500 | 2,000 | one soil, supplies as rates |
| activity (3 times) | 1,200 | 400 | once, after the liquids |
| heat | 2,400 | 1,500 | `energy_at` once |
| site, films, roles | 2,000 | 1,500 | roles on the card |
| the rest | 3,600 | 3,000 | |
| the pools (new: inhibition, bare reactions, inputs) | a few hundred | 3,000 | a few pools, a few phases |
| **total** | **37,900 (7.0 µs)** | **about 21,000 (4.0 µs)** | |

**The sample** today clones the body, runs its whole update and rolls it back (13% of a stand update, 2.3 µs on
average: one update in two, and `read` skips most). Here it is the plan with its own card: about 1.5 µs a run, and at
every update (P1.4) about 0.7 µs on average.

**A whole stand update**, island22: today 18.0 µs (the law 10.6; children, shedding and soaking again 3.3; the sample
2.3; maturing 1.0; the rest 0.7). After P1.2 with P1.4: about 4.0 + 0.7 + 2.8 + 1.0 + 0.7 = **9 to 10 µs**. P3 takes
about 1.5 µs (site per cell, faces at the update, no ring): **about 8.5 µs**. The plan's section 5 assumed 6 µs.

**Per stand and year** (today 10.6 updates of 18.0 µs, 191 µs): P1.2 with `field_days` 30, 12.2 updates of about 9.5
µs: about 115 µs (x0.60); P2 (60 days): about 60 µs (x0.31); P3: about 50 µs; G9a (90 days): about 36 µs (x0.19).

**Gain on one thread** (plants are 30.0% of island22's year and 14.7% of globe36's in instructions, R2):

| Step | island22 | globe36 | the plan said |
|---|---|---|---|
| P1.2 with P1.4 | -10 to -12% | -5 to -6% | -12% and -8% (P1.1 to P1.4) |
| P2 | -8 to -9% | -4% | -7% and -3% |
| P3 | -3 to -5% | -2 to -3% | -5% and -3% |
| P1b | -3 to -6% | -4 to -7% | about 0 (a law step) |

On four threads (section 5.0 of the plan): the plants' rows give about 10 ms less than the plan on the island and 4
on the globe at the end; P1b moves part of G3's gain earlier.

## 8. Risks

| Risk | Answer |
|---|---|
| The pools fail the splitting test (the living body held at its start over 60 to 90 days; a scarce input that changes inside `Δ`; bare reactions on stores held at the start) | sub-steps of at most 30 days in stage 4 only; P2 and G9a run the test at 60 and 90 before they merge |
| The lean stages do not reach 4 µs | P1.2's own gain line (7% of island22's year) still holds at 5 µs; the plan counts 9 to 10 µs a stand update |
| Bare reactions as first-order rates give pairs (the lime and the dissolved salt) a different balance over a long interval | the pair's source from lime is a constant and its return first order: the salt pool relaxes to the pair's balance in closed form; the splitting test checks it |
| The heat's path moves the land (PR-M: -0.52 K) | one place sets the heat, after growth; the land reported; fallback for stands: `relax_heat`'s closed form at the start temperature |
| P1b moves the eaters at today's pace (no dip between updates: more children; deaths by the rate) | announced as law effects, reported against the references; key numbers out and not accepted: not merged |
| PR-A2 finds a cause the pools do not answer | 5.2; G2 waits for it or takes the young-only floor |
| `reserve_days` 36 makes stands one more `Physics` number | it is today's value; `D` = gene is the owner's law question (S50) |

## 9. Lines

`body/month/` (plan, pools, booking): about +450; P1.2 deletes `gain_to_growth`, the sample's checkpoint path for
stands, `node_days`'s branch, `soak_again` for stands (about -40); P1.4 deletes `sample_every` (-12); P1b deletes
`settle_upkeep`, `keep_warm`, the body's call of the reactor, `copy::grow`'s paying and `soak_again` (about -150) and
adds the keep-warm drain and the heat hand-over (+40). Lane A net about **+290** (the plan said +265).

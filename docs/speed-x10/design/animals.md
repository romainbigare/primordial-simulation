# Speed x10, section "animals": the month law for bodies with a brain (P1b)

Design section of the x10 plan (`plan.md` step P1b; decisions X6, X9, X14, X29). It gives bodies with a brain the
same energy law as stands: upkeep, income and growth paid together over the interval by one stock equation. It is the
step that makes G2 (every population node updates at least every 30 days, which replaces MI6) honest. Written in
simple words for the owner. Code at `2056434` (branch `speed/x10`).

## 0. In short

- **Today an animal eats ahead and pays behind.** At its update it first pays the upkeep of the days since its last
  update from its stores (`body::live::settle_upkeep`), then it decides and acts (`choose::act`), and its acts bring
  the food for the next interval (`feed::take_node`, `feed::take_body`, sized by its interval). So the food of an
  interval is in its stores when that interval's upkeep is paid. That part is already in the right order.
- **What breaks at a long pace is not the order of acts.** Three rules of today's body law depend on the length of an
  update, not on time: (1) a body that cannot pay loses the unpaid share of its members at once, per update (the
  death rate goes with 1/update, C2); (2) growth keeps a reserve of *one interval's* upkeep (`copy::grow::convert`,
  `spare`), so a longer interval keeps more and builds less; (3) other rates read the update's interval where they
  mean the gene's (the prey's chance to get away in `take_body`, the movers of one decision, learning's discount).
  The probe of `gap.md` 1.2 saw animal nodes lose half their members in 2 years at a 30 to 40 day pace.
- **P1b:** every body runs the stock equation of the stands (`month::balance`, P1.2): the stock is its fuel at the
  update's start, which holds what it ate at its last update (a pulse at the interval's start); income is the light
  reaction (none for most eaters); upkeep is a steady drain, **and it holds keep-warm's power** (taken at the update's
  start), so the on-demand `keep_warm` pass after the balance goes; growth uses only fuel above a reserve of one
  interval of its **gene** (`INTERVAL`, not rounded to batches); when the stock cannot pay, members die at a rate per
  day. The acts stay after the update (not moved before paying, section 4).
- **Law changes, announced** (X14): (a) today the unpaid share `u` of an update dies at once; P1b kills `1 - e^-u`
  over a gene interval, so a deep famine kills fewer (`u` = 1: 63%, not all); (b) for stands, `D` = gene gives most
  of them a reserve 1.8 to 3.6 times smaller than today's 36 days, and their `INTERVAL` gene starts to matter to
  selection; PR-M measures `D` = gene against `D` = 36 and the architect chooses (X14); (c) keep-warm is paid inside
  the balance, not after it.
- **First a probe (PR-A):** on a branch, a 30-day floor for animal nodes with today's law, then each suspected cause
  switched off one at a time. It says how much of the loss each cause makes, so P1b and G2 fix what is real.
- **Check:** a fast splitting test of the law (also at 260 K); the key numbers at today's pace in their ranges (the
  famine rule's shift reported as a law effect); PR-A run again with P1b at a 30-day floor (members within the seed
  noise). Level 3.
- **Lines:** about +75 and -25 (most of the law is P1.2's `month::balance`, shared; `keep_warm` goes).

---

## 1. What exists today (code)

The update (`survey::update`, `core/src/survey.rs:262-297`) of a body with a brain:

| Stage | Code | What it does with energy |
|---|---|---|
| site | `live::site` (`body/live.rs:69-77`) | `days` = days since the last update |
| upkeep | `live::settle_upkeep` (`live.rs:143-170`) | `cost = pay::upkeep(days)` (`body/pay.rs:17-29`), paid by `pay_at(.., part = true)` (`pay.rs:122-149`): the burnings run as far as `pay::plan` lets them (each at most `lim × frac(rate × days)`, Change's rate at the body's temperature, `pay.rs:91-119`). **The unpaid share `short` dies at once** (`lose_share(.., SPENT)`, `live.rs:160-168`); wear kills `1 - e^-worn` (`AGE`, already a rate) |
| physics | `physics::physics` (`body/physics.rs:43-73`) | light caught, gas, the reactor on the stores (`react`, all catalysed reactions, the light reaction too), evaporation, soaking, roots, the heat network |
| keep warm | `live::keep_warm` (`live.rs:190-202`) | burns on demand to hold `KEEP_T` over the span; if it cannot, the body ends colder (no deaths here; `check_stores` then judges frost). It runs after the upkeep is paid, so it spends from the stores outside the upkeep's account |
| checks | `check_stores`, `starve` (`live.rs:206-250`) | minimums kill the poorest share (a state rule, not a rate) |
| growth | `copy::grow` (`copy/grow.rs:13-39`, `convert` 87-139) | builds within the allowance over `min(days, interval)`; **`spare = fuel × (1 - upkeep(interval) / r)`** (`grow.rs:107`): a reserve of one interval's upkeep at the update's interval |
| starving | `live::starved` (`live.rs:131-139`) | body share lost and not rebuilt: "in one update or in many" (already independent of the step) |
| children | `copy::copy`, `shed` | births when the brood covers a child |
| decision | `choose::act` (`choose/mod.rs:79-108`) | acts: `take_node` (`choose/feed.rs:38-58`), `take_body` (92-129), `give`, `put`, `go`, part acts; each act pays its handling at once (`body::pay`, all or nothing) |

**Income from eating.** `feed::intake` (`feed.rs:26-34`) takes Holling's type II per day times `days =
b.interval(w)`, and the share of the cell it sweeps `min(1, sweep × supply × days / area)`; `Body::wanted`
(`body/intake.rs:25-39`) caps it by the room of each store, `room = gap × paces` (`intake.rs:18-21`). Body matter
taken is digested at once into its building inputs (`Body::digest`, `body/metab.rs:38-53`), so it is fuel in the
stores as soon as it is taken. **So what an animal eats at an update is fuel at the start of its next interval.**

**Rates that read the update's interval.** `take_body`'s `away = 1 - o.interval / b.interval` (`feed.rs:106`);
`pay::pay` holds the heat of a cost over `b.interval` (`pay.rs:66`); `spread::spread` sends `count × p[o]` members
per decision (`choose/spread.rs:14-28`); `learn::learn` discounts once per decision (`choose/learn.rs:19-38`).
Already by pace: intake, rooms, deaths over a span (`Body::over_span`, `body/mod.rs:255-260`).

## 2. The probe first (PR-A)

On a branch, from the year-200 snapshots of R2's benches, 2 years at 4 threads, three seeds:

1. Today's law, animal nodes at their gene's pace (the reference), and with a 30-day floor (`Body::interval` floors
   every population node, as G2 will). Members, births less deaths, eaters' mass.
2. With the floor, switch off one suspected cause at a time:
   - (a) the death of the unpaid share per update: replace it by the rate `(1 - paid/cost) / I_gene` over the days;
   - (b) the growth reserve of one interval: use the gene's interval;
   - (c) `away` by update intervals: use the gene's intervals;
   - (d) the movers of one decision: `count × min(1, p × paces)`;
   - (e) births and maturing waiting for the update (C2's second cause): count how many days late they come;
   - (f) the bulk take: how often a take is cut by what lies (`from(k)`) or by the share swept reaching 1.
3. A table: the members lost by each cause, and what is left with (a) to (d) all on.

If (a) to (d) do not bring the loss within the seed noise, the rest is (e) or (f): P1b still lands as the law (X6),
and G2 waits for that cause, or falls back to a 20-day floor (`gap.md` G2 risk 1).

## 3. The law

### 3.1 The stock equation, one for every body

P1b calls the stands' `month::balance` (P1.2, `plants.md` 2.3 stage 4) for every body. Per member and per day,
constant over the interval `Δ = day - last`:

- **The stock `R`**, J its burnings can release (`Body::fuel`, `body/metab.rs:63-70`), at the update's start. It holds
  what the body ate at its last update (digested at the take). **Food is a pulse at the start of the interval.**
- **Income `P`**: the light reaction's rate, the least of light caught, carbon gas and catalyst (Blackman 1905), with
  product inhibition, as for stands. For a body whose light catalyst is off (every eater today) `P = 0`. The light
  reaction runs here for every body; the reactor of `physics` leaves it out for a body on the month law (a filter on
  `Consts::cat`), so it is counted once.
- **Upkeep `U`**: `pay::upkeep` for one day (unchanged law), **plus keep-warm's power**: the heat its surface sheds
  when held at its `KEEP_T` gene (its conductance to the ground, the one `pay::hold` uses, times `KEEP_T` less the
  ground's temperature), taken at the update's start and held over the interval; zero when the ground is at or above
  `KEEP_T`. So keep-warm is a steady drain inside the stock
  equation: growth only uses fuel above a reserve that includes it, and a cold body does not grow from, or die by, a
  stock that keep-warm would drain later in the update (and then die of frost in `check_stores`). The heat that drain
  releases enters the body's heat before the day law's heat network (`physics`), as `spend` holds it; G3 later gives
  that heat its closed form. After MI3b (if built), the thoughts since the last update are one more steady drain,
  their cost over `Δ`.
- **The release limit:** the burnings release at most `k R` a day, `k` Change's rate of the first burning at the
  body's temperature and activity (what `pay::plan` uses, `frac(rate × days)`, as a rate). So a cold body cannot pay
  from a full stock, as today.
- **Growth `G`**: `grow::allowance` per day, only from fuel above the reserve `R_res = U × D`.
- **`D`, the body's own time:** its `INTERVAL` gene in days (`dec(INTERVAL)` clamped to its range, **not** rounded to
  batches, so `batch_days` does not move the law). For an animal at today's pace `D` is today's reserve. Its death
  time changes (a law change): today the unpaid share `u` dies at once; here the rate `u / D` over `D` days kills
  `1 - e^-u`, so a deep famine kills fewer. For stands it replaces `plants.md`'s `reserve_days`; today their reserve
  and death time are `max(gene, node_days)` = 36 days (`Body::interval`, `body/mod.rs:247`), and the light-eaters'
  genes are 20, 10 and 60 days (`worldgen/src/life/kinds.rs:58,101,102`), so `D` = gene gives most stands a reserve
  1.8 to 3.6 times smaller and makes their `INTERVAL` gene matter to selection. PR-M runs `D` = gene against `D` = 36
  for stands; `D` = gene is kept if the key numbers stay in their ranges (S50 and S84 not past their noise), else
  stands take `D` = 36, a `numbers.md` row (decision X14).

`dR/dt = P(R) - min(U, kR) - G(R)`, in phases, each linear (`dR/dt = α - βR`, solution `R_eq + (R0 - R_eq) e^-βt`,
or a line when `β = 0`):

| Phase | When | `dR/dt` | Deaths |
|---|---|---|---|
| 1, growing | `R > R_res`, `kR ≥ U` | `P - U - allowance` | none |
| 2, at the reserve | `R = R_res` | 0: growth takes the net income `max(0, P - U)` | none |
| 3, short | `kR < U` | `P - kR` | rate `μ(t) = (1 - kR(t)/U) / D`; the unpaid part of keep-warm leaves the body colder, as today |

In phase 3 the stock tends to `P/k`, so the death rate tends to `(1 - P/U) / D`: the plants' rule of `plants.md` 2.3
is its limit. The times where a phase ends have closed forms; at most three phases a call. The integrals of `P`, of
the upkeep paid, of `G` and of `μ` come out of the same forms. Members that die (`SPENT`) are the share
`1 - e^(-∫μ dt)`; a node loses that share of every store (`thin`), an individual dies with that chance (one rule,
`lose_share`). Wear (`AGE`) stays as today. Sources: the labile pool of a carbon balance (Thornley 1972; Sitch et al.
2003), applied to a consumer's reserve (dynamic energy budgets: Kooijman 2010, the reserve and its mobilisation).

### 3.2 Why the pulse is close over splits

Take `P = 0`, a stock that runs out, and food `F` a period. Without the release limit the stock runs out at `F/U` in
each period and is empty until `I`: one update of 3 periods with one pulse `3F` and three updates with pulses `F` are
both empty for `3(I - F/U)` days, so `∫μ dt` is the same. The release limit makes the end of the stock a decay
(`dR/dt = -kR` once `kR < U`), and that tail is not the same for one large pulse and three small ones. So the pulse is
**close, not exact**: the splitting test bounds it at 2%. With `P > 0` the phases are the stands' and the same test
bounds the error.
What does change with the pace is the pulse's size: it must fit the stores' room (`room = gap × paces`, already) and
what lies in the cell at the take (section 6, risk).

### 3.3 The order of booking (decision X29)

The balance only says how far each reaction runs. The booking (`body::apply_extent`, P1.1) goes in a fixed order and
never relies on `add_store`'s clamp:

1. **Supplies over the interval enter the limits first.** The carbon gas and the oxidising gas the burnings breathe
   (the gas pool of `physics::gas_pool`, capped by the view), the liquid soaked in (`physics::soak`'s exact form,
   capped by the ground), the light set aside (capped by the bodies' node). If a cap cuts a supply below what the
   phases used, the balance is solved again with that supply as its limit (at most one more solve: each cut only
   lowers a limit). If that second solve cuts a second input, the extents' caps of step 3 keep the books, and a debug
   counter counts how often it happens (X29), so a later step knows whether more solves are needed.
2. **Inflows** are booked (gas taken, liquid soaked, light taken).
3. **Extents**, one after another, each capped by what is left of its inputs then (`change::scarce`): the light
   reaction, the burnings in `Consts::burn` order, then growth (`Chemistry::builds`, then `Chemistry::build`).
4. **Outflows** last (gas put out with its heat and latent heat, `Moves::gas`).

A debug check asserts that no store ends below zero before rounding; `counts.rounded` stays what it is today
(rounding only).

### 3.4 The update of an animal after P1b

`site` → **`month::balance`** (upkeep with keep-warm's power, thoughts after MI3b, light income, growth energy,
starving deaths) booked as 3.3 → wear (`AGE`) → `physics` without the light reaction (the day law's heat network, with
keep-warm's heat already in) → `check_stores` → **`copy::grow` books the growth the balance allowed** (its `spare` reserve term goes:
the balance did it; its allowance and `reach` stay) → `starved` → children, `shed` → `choose::act`.

`settle_upkeep` and its per-update death share go, and so does the on-demand `keep_warm` pass. Individuals and full
minds run the same law (one energy law, X6); G3 later gives population nodes a cheaper closed form for their heat
(only the heat: keep-warm is already in the balance), individuals keep the day law's heat (rule 3 for smart bodies:
kinds of body code).

## 4. Why the acts are not moved before paying

| Choice | Why not |
|---|---|
| Decide and act at the update's start, pay after | The decision reads what the update leaves (`Lived`: light caught, spare, mass, gain, `survey.rs:277`); it would read the last update's. The food would still be eaten for the next interval (intake is sized by it), so nothing gets more exact; and it reorders every body's update (level 3) for no gain |
| A gut store digested over the interval | New state per body and a new law; `Body::digest` already turns body matter into fuel at the take, and the pulse is close over splits (3.2, within the 2% splitting test) |
| Pay the upkeep of the coming interval at the update (ahead) | Then upkeep is paid before the food it needs is eaten: C2's error again |

So the acts stay where they are, and the stock simply carries what they brought.

## 5. The pace test, and how P1b opens G2

**P1b's tests** (`worldgen::testkit`, fast):
1. `a_body_pays_and_eats_by_one_stock` (`core/tests/body.rs`): `month::balance` of an eater node with food pulses:
   one update of 60 days with a pulse of 60 days' food against six of 10 with six pulses: members, fuel and body
   matter within 2% (close for `P = 0`; the release limit's tail makes the difference). Also at 260 K, where
   keep-warm is most of `U`. Today's law fails it (its deaths go with the number of updates).
2. `a_starving_body_dies_at_its_own_time`: an empty stock with `P = 0` loses members at `1 / D` a day, whatever its
   updates.

**P1b's world check (level 3):** the key numbers at today's pace (references fixed at the branch's start, R2) in
their ranges; `D` is today's reserve for animals, but a deep famine kills fewer (`1 - e^-u`, not `u`) and keep-warm
is paid in the balance, so the commit reports how far the eaters moved as a law effect; and PR-A run again with P1b
at a 30-day floor: animal members and births less deaths within the seed noise of the gene-pace run.

**G2's pace test** (slow, lands with G2): eaters with `node_days` 10, 30, 60; three seeds; the eaters' mass at year
30, members, births less deaths of years 71 to 100 within the seed noise (the form of
`the_pace_of_nodes_keeps_life`, `tape/tests/ranges.rs:327`).

**How it opens G2.** After P1b: starving follows time (3.1), the growth reserve follows the gene (3.1), and the food
of an interval is a pulse whose take and room already follow the pace. G2 adds the rates still read by update: the
movers `count × min(1, p × paces)`, learning's discount `discount^paces`, `away` by the gene intervals, and the heat
of an act's cost held over the gene's interval. Then a node at a 30-day floor is the mean of its members over 30 days,
and every population node (plants and animals) updates at least every `node_days`: G2 replaces MI6.

## 6. Risks

| Risk | Answer |
|---|---|
| C2's second cause: children and maturing members wait for their node's update (up to 30 days late at G2's pace) | PR-A (e) measures it; G2's pace test reads births less deaths; if it fails, births at a rate over the interval (`copy::copy` already uses the brood's level, a closed form per interval is possible) is G2's fallback before 20 days |
| The bulk take: a 30-day take finds only what lies now, and the swept share stops at one cell | PR-A (f) counts it; prey stands hold far more than a month of eating; ground litter accumulates between takes (it rots slowly); G2's pace test decides |
| P1b moves today's world (S50, eaters) | Announced: a deep famine kills `1 - e^-u`, not `u`; keep-warm in the balance. `D` is today's reserve at today's pace; level 3 with the key numbers; reported as a law effect |
| Stands with `D` = gene keep a smaller reserve (1.8 to 3.6 times) | PR-M runs `D` = gene against 36 and reports the key numbers' shift; `D` = 36 is the fallback (X14) |
| A cold body with a full stock (release-limited) dies too fast or too slow | phase 3 uses the release limit `pay::plan` uses today; the splitting test runs at 260 K too |
| Keep-warm's power taken at the update's start misses a cold snap inside the interval | it is the steady drain of the interval, as upkeep is; the day law's heat network still sets the temperature until G3, and `check_stores` judges frost on it |
| Two laws for a while (stands from P1.2, animals at P1b) | `docs/laws.md` 6.2 and 6.10 side by side between P1.2 and P1b (X6) |

## 7. Files and lines

| File | Change |
|---|---|
| `core/src/body/month.rs` (P1.2) | `balance` gains phase 3 with the release limit and the pulse start (shared with stands) |
| `core/src/body/live.rs` | `settle_upkeep` and `keep_warm` go (keep-warm's power is worked out at the update's start and enters `U`); `to_growth` calls `month::balance` |
| `core/src/body/physics.rs` | `react` leaves out the light reaction for bodies on the month law |
| `core/src/copy/grow.rs` | `convert`'s `spare` reserve term goes; growth books what the balance allowed |
| `core/src/body/pay.rs` | unchanged law; `pay_at` books by `apply_extent` (P1.1) |
| tests | the two tests above; `core/tests/body.rs` tests of upkeep that relied on the per-update share move to the rate |

About +75 and -25 core lines. Docs: `laws.md` 6.2 (one law for all bodies), 6.10 (the phases), `numbers.md`
(`reserve_days` not added; `D` is the gene).

## 8. Step

| ID | What | Level | Done when | Kill line | Needs | Agent |
|---|---|---|---|---|---|---|
| PR-A | The probe of section 2 | none (branch) | its table in `measures.md` | none | R2 | Opus |
| P1b | Section 3: the stock equation for every body, food as a pulse, keep-warm a drain in `U`, the reserve and death time by the gene, booking order 3.3; tests of section 5; the law changes announced (0) | 3 | tests pass; key numbers at today's pace in their ranges, the shift reported; PR-A again with P1b at a 30-day floor within the seed noise; books 1e-12 in the century | key numbers out and not accepted by the owner: not merged. The 30-day check fails: P1b merges as the law (X6), G2 waits for the cause PR-A names or takes 20 days | P1.2 (`month::balance`), P2, PR-A | Opus, adversarial Opus review |

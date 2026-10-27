# Idea 3: living bodies do not rot (lane B)

Steps: PR3, B1, B2 (`plan.md` section 4). Decisions: L5, L9, L19. Handover 13.2 lists it as the owner's choice: "The
litter constants also run inside living bodies (real living stems and bones do not rot)." The owner now says yes.

## 1. What the code does today

**The rate law** (`laws.md` 2.1, `change/kernel.rs`, `Reactor::rates`): each reaction runs at `cr · k exp(-Ea/T) ·
contact · ...`. In the open world `cr` is the decomposers' factor `Body::catalyst(open, moist)` = `1 + (open - 1)
moist`. Inside a body `body/physics.rs`, `react`, gives a reaction whose gene is on `1 + (f - 1) a` (`f` = 10^gene), one
whose gene is off **1, the bare rate**, and a catalysed burning that breathes **1** too (it ran on demand in
`body::pay`; "the open air burns the stores bare"). `Reactor::rates` skips a reaction where `cr <= 0`, or where `cr <= 1`
and the node is colder than the reaction's frozen point (`thr`, `frozen_below`: where `k exp(-Ea/T)` would use less than
`NEGLIGIBLE` of an input in a year).

**The data** (`worldgen/src/chem0.toml`, `docs/numbers.md`):
- reaction 1, burning the energy-rich solid: `k` 1.72e4 /s, `Ea` 7,760 K: bare 0.27 a year at 290 K, **a litter
  constant** (Olson 1963); `open` 200;
- reaction 21, burning body matter: `k` 2.28e9 /s, `Ea` 9,000 K, orders 1 and 6: bare 0.2 a year at 290 K, Olson's
  constant again; `open` 5;
- reaction 26, the frame rots: `k` 1.72e-2 /s, `Ea` 7,760 K: bare 2.7e-7 a year at 290 K ("the frame of a living body
  does not rot"); `open` 1e6, so its decay in moist open ground is 0.27 a year. **26 is already written the honest
  way:** the decomposers' work is in `open`, the bare rate is near nothing.
- 22 and 23 (the salt and lime pair) and 25 (the mineral breaking down hot) are plain chemistry of a liquid and of heat.
- The founders' burning genes are lower by `LITTER_X` (0.602, `worldgen/src/life/make.rs`) so that a body's catalysed
  burning stays as it was set over the bare litter constant.

So the bare decay of 1 and 21 inside a body is the decomposers' rate run without decomposers. R2 (`measures.md`, "P0"):
about 90% of the reactions run in a stand's reactor are bare, and all of an animal's. PR-A: an eater's bare burning of
its own matter is 5 to 8 times its upkeep (`c_basal` is anchored to Kleiber), so today's eaters burn about 6 to 9 times
Kleiber, most of it this artifact (21's bare rate rises x3 to x7 from 290 to 300 to 310 K). On `benchZ` the bodies'
reactor called from their day is 6.4% / 5.0% / 7.0% of `run_for`.

## 2. What changes

**B1, a law (level 4), variant (d):** write 1 and 21 as 26 is written.
- Their bare `k` is divided by `X` and their `open` multiplied by `X`. `Ea` stays, so the decomposers' decay in a moist
  open node keeps its rate and its temperature law exactly (`k · open` unchanged). PR3 picks `X`: large enough that the
  frozen rule skips the bare decay at every living body's temperature (26 uses 1e6; for 21 at 1e6 the bare rate is
  under 1e-5 a year at 320 K), small enough that matter still burns at fire's heat (21 at 700 K: 6e-3 /s at 1e6).
- The founders' catalyst genes of 1 and 21 rise by log10 `X` (the rule `LITTER_X` states), so a body's catalysed
  burning (`Body::rate_at`), the catalysts' upkeep (`k.cats`: `contact_rates` times `f`) and product inhibition stay as
  set. Catalyst genes have room up to 18 (`body/genes.rs`).
- **No body code changes.** A gene that is off gives `cr` = 1, and a breathing burning in `react` gets 1: both are then
  colder than the frozen point and skipped. 22, 23 and 25 keep running in a body's liquid as real chemistry; at fire's
  heat matter still burns by 21 (`laws.md` 6.8 keeps its sentence, with "at fire's heat").
- The chemistry is saved in `World`, so the snapshot is bumped (an old save keeps its old data). B1 is judged per
  update on a turned save (a measuring branch rewrites the chemistry and raises the genes at load).

**Docs and tests.** `numbers.md` rows of 1, 21 and their `open` (the anchor: Olson's constant is the decomposers'),
`LITTER_X`'s doc; `laws.md` 2.1 ("no decomposer works in a body" gains: "the decay of 1, 21 and 26 is the decomposers'
work, so a living body runs it only by its own catalysts"), 6.8, 6.9; handover 13.2's "Inside bodies" item closed (the
rock salt still falls out of a body as lime: real chemistry). Tests: `a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses`
(its heat made counts no bare burning), `chemistry0.rs` and `numbers.rs` where they read 1 and 21, a new
`a_living_body_runs_no_decay_but_its_own_catalysts`.

**Variants PR3 sets beside it** (body code, kept only if (d) fails):
- (a) no bare rate inside a body at all: a body's factor `(f - 1) a`. It needs a new heat-damage law (21's bare rate is
  the only one, `laws.md` 6.8) and its unfolding must be `denature(1 + (f - 1) a) - 1` with `Reactor::rates` not
  unfolding it again (the review: `denature` pulls a factor above 1 toward 1, the bare rate);
- (b) only the decay (1, 21, 26) gets factor 0 in a body, by body code.
(c) (stands only) is dropped: tissue would rot by kind.

**B2 (level 1, only if PR3 shows it worth 1% of the year after B1):** `Reactor::react` returns at once when no reaction
can run (inside the reactor; `physics::react` stays whole). Under (d) `rates` already breaks at the first sub-step, so
B2 may not be needed.

## 3. Risks

- **The eaters' budget.** Without their bare self-burning they need far less food: they may grow until they eat most
  of the plants' production (handover 13.2), and their key numbers (5.354 ± 0.5 log10 kg; life 10.47 yr x/ 2) will
  likely leave their ranges. That is the honest effect (L19: new references with the cause said); the plants must not
  be eaten out (the watch).
- **`denature` under a larger factor.** It keeps `f^(1/(1+e^z))`: on a factor `X` times larger, a half-unfolded enzyme
  keeps a smaller share of its rate (at `td`, 1e-4 of it for f = 1e8, against 0.1 for f = 1e2). A heat-killed enzyme
  now leaves almost nothing, not the bare decay: honest, but hot bodies pay more and release less. PR3 compares a hot
  body's catalysed rate with today's.
- **Dry litter.** In the open world a dry node (moisture 0) has `cr` = 1: under (d) its decay falls `X` times (real dry
  litter decays slowly too). Less gas 3 from dry land; PR3 reads it.
- **Dormant bodies.** Upkeep and ageing go with activity; a dry body no longer loses anything bare. Watch bodies with
  activity under 1% that outlive a few of their kind's lives.
- **S50**, the light-eaters' stored share, rises if stored solid no longer burns bare (0.558 now, "under 0.5" wanted).
- **The land.** P1.2 moved it -0.78 K, cause not found; P1.2 changed several things at once. PR-M's prototype ran stands
  with no bare reactions: v1 moved the land +0.13 K (292.27 and 292.28 against 292.14), S50 0.573, light-eaters 10.43 to
  10.44; v2's -0.52 K was put on its heat stage (`measures.md`, "PR-M"). So the bare reactions are not the known cause.

## 4. What PR3 must show first

On a branch (the chemistry and the genes turned at load for (d); a switch for (a) and (b)):
1. On each `benchZ` save, one year, one thread, at the gene's pace: per reaction, in stands and animals apart, the
   moles, heat and gas a year; the eaters' self-burning against their upkeep (PR-A's number again, on today's world).
2. The cost of a stand update and an animal update under (d), (a), (b).
3. The key numbers on 3 seeds under (d) (and (a), (b) if (d) fails), with S50, the minerals in bodies, the gas 3 from
   dry land, dormant bodies; globe36 with life (3 seeds, 30 years) for the climate. Reaction by reaction only if the
   3-seed land mean moves past the seeds' noise.
4. `denature`: a body above its `DENATURE` gene, its catalysed release today and under (d).
5. `X`: the frozen points of 1 and 21 at 1e4, 1e6, 1e8, against the warmest living bodies.

**Gate for B1:** (d) with the climate keys in range and the year 1T -3% on island22.

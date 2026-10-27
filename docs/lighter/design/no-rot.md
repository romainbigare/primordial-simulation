# Idea 3: living bodies do not rot (lane B)

> **Status after phase P (L9):** the law is not built this round (PR3: every form makes the world heavier, eaters x35, updates +38%); it is an open item for the owner, best as (b0). Lane B builds the skip instead (B1, `plan.md` 4). This note records the candidates and why.

Steps: PR3, then a design review of lane B, then B1 and B2 (`plan.md` section 4). Decisions: L5, L9, L19, L32 to L34.
**B1's law is not chosen yet:** PR3 chooses between (b) and (d') below, or "not built" with its alternative (L9). Handover 13.2 lists it as the owner's choice: "The
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

**The frozen rule reads the bare constant only.** `frozen_below` freezes a reaction below `Ea / ln(k · year /
NEGLIGIBLE)` (`NEGLIGIBLE` 1e-12, `year` in seconds): no catalyst, no contact, no orders. So today 1 is frozen only
below 142 K, 21 below 135 K, 26 below 190 K: in every living body (270 to 320 K) all three run, 26 too.

**`denature`** (`kernel.rs`) keeps a catalyst factor as `f^(1/(1+e^z))`, `z = (T - td)/unfold`, `unfold` 3 K: the loss
grows with `ln f`, so the share of the rate an enzyme keeps near its `DENATURE` gene depends on how large `f` is.

## 2. The two candidates (PR3 chooses; then a design review)

**What the first plan said, and why it does not hold (variant (d), the second review S1 to S5).** (d) divided the bare
`k` of 1 and 21 by `X` and multiplied their `open` by `X`, the founders' genes raised by log10 `X`, "no body code". But:
- the frozen rule reads `k e^(-Ea/T)` against 1e-12 of a year, so at `X` 1e6 reaction 1 is frozen only below 190 K
  and 21 below 171 K (1e8: 214 and 187 K; 1e12: 287 and 232 K): **nothing is skipped in a living body**, so (d) gives
  no speed; freezing at 320 K needs `X` about 1.6e13 (1) and 4.4e16 (21), past the catalyst genes' top of 18;
- a factor `X` times larger unfolds more steeply under today's `denature` for every warm body (an eater at its `COOL_T`
  315 K, `td` 320 K: 48% of its rate today, 5% at `X` 1e6), so the temperature law of all bodies would hang on `X`, a
  speed number;
- heat damage at fire's heat: with the order 6 of the oxidising gas, 21 at 700 K and `X` 1e6 runs about 16 a year
  (5e-7 /s), a half-life of two weeks, not a fire (the first note's "6e-3 /s" left out the orders);
- the open world's decay is unchanged only in fully moist nodes: the factor is `1 + (open - 1) m`, so under (d) dead
  matter (21, `open` 5) rots 17% slower at m = 0.5, 44% at m = 0.2, not at all at m = 0;
- genes of 1 and 21 between 0 and log10 `X` are "on" (they put the reaction in `k.burn`) but burn about nothing.

So (d) as written is dropped. Two candidates remain.

**(b) The decomposers' reactions do not run in a living body (body code).** In `body::physics::react`, a reaction
whose decay is the decomposers' (`open > 1`: 1, 21, 26) gets factor 0 when its gene is off, and a catalysed burning
gets 0 in `react` (it runs on demand in `body::pay`); 22, 23, 25 run as today. The data do not change, so the open world
is unchanged at every moisture. **Heat damage** (`laws.md` 6.8: "heat burns matter by reaction 21", today the bare rate
of 21 in a body) must then be kept by a stated law in the same commit: the bare part of 21 above a temperature where
matter itself breaks down (an anchored number, about where proteins and walls char), or "heat damage of bodies ends
until fire's own reaction at M4", said plainly. Speed: the reactor of an animal runs nothing it needs; of a stand,
only its catalysed reactions and 22, 23, 25.

**(d') The decay in `open`, with an honest frozen test and the two-state `denature` (data plus kernel code).**
- The data of (d): 1 and 21 written as 26 (`k / X`, `open · X`, the founders' and every body's genes of 1 and 21
  raised by log10 `X` in the turned save);
- **the frozen test reads the rate the reactor would use:** `cr · k e^(-Ea/T)` times the reaction's contact and orders
  (`pace`) over the call's time, against a skip share: a speed number with a name, a one-line doc and a `numbers.md`
  row, larger than 1e-12. A level-2 rule for every node (ground, parts, bodies) in `change/kernel.rs`;
- **the two-state `denature`:** an enzyme is folded and works, or unfolded and does not (Lumry and Eyring 1954): the
  factor is `1 + (f - 1) / (1 + e^z)`, the folded share times the catalysed part, so unfolding no longer depends on
  how large `f` is. Part of the law in B1's commit (`laws.md` 2.4, `kernel.rs`'s doc).
- Costs, stated in B1's commit if chosen: dead matter rots slower on land that is not fully moist (above), which moves
  the mineral 24 and lime on land and the parts' walls; heat damage of bodies ends below about 1,000 K (fire needs its
  own abiotic reaction at M4; `laws.md` 6.8 says so); the useful range of the genes of 1 and 21 starts at log10 `X`
  (`laws.md` 2.4).

**"Not built"** if neither holds the climate keys and gives about 3% of the year (L2). Its alternative: the reactor
cheaper for what bodies do run (B2 and G1's kernel work), and handover 13.2's item stays open for the owner.

**B2 (level 1, only if worth 1% of the year after B1):** `Reactor::react` returns at once when no reaction can run
(inside the reactor; `physics::react` stays whole).

**Docs and tests (either candidate).** `laws.md` 2.1 (the decomposers' decay does not run in a living body), 2.4, 6.8,
6.9; handover 13.2's "Inside bodies" item closed; `numbers.md` rows of what changes; tests:
`a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses` (its heat made counts no bare burning), a new
`a_living_body_runs_no_decay_but_its_own_catalysts` (a body with its burning genes off keeps its matter and solid over
a year at 300 K; the same matter on moist ground rots at its litter constant); under (d') `chemistry0.rs` and
`numbers.rs` where they read 1 and 21, and a test of the two-state unfolding.

## 3. Risks

- **The eaters' budget.** Without their bare self-burning they need far less food: they may grow until they eat most
  of the plants' production (handover 13.2), and their key numbers will likely leave their ranges. That is the honest
  effect (L19: new references with the cause said); the plants must not be eaten out.
- **Unfolding.** Under (d') the two-state form changes every body's rate near its `DENATURE` gene (today's form keeps
  48% at 5 K below it for f = 100; the two-state form keeps 85%). PR3 measures it on the eaters (key numbers, the share
  of eater updates above `DENATURE - 10 K`).
- **Dormant bodies.** Upkeep and ageing go with activity; a dry body no longer loses anything bare. Watch bodies with
  activity under 1% that outlive a few of their kind's lives.
- **S50** rises if stored solid no longer burns bare (0.558 now, "under 0.5" wanted).
- **The land.** P1.2 moved it -0.78 K, cause not found; its prototype PR-M ran stands with no bare reactions and moved
  the land +0.13 K (v1); v2's -0.52 K was put on its heat stage (`measures.md`, "PR-M"). PR3 reads 3 seeds.
- **The kernel** (d') is shared by every node and by lanes G and B2: one at a time, B1 first (L34).

## 4. What PR3 must show (running now)

On a branch (switches for (b) and (d'); the chemistry and every body's genes of 1 and 21 turned at load for (d')):
1. On each `benchZ` save, one year, one thread, at the gene's pace: per reaction, in stands and animals apart, the
   moles, heat and gas a year; the eaters' self-burning against their upkeep (PR-A's number again).
2. **The frozen points** of 1, 21 and 26 under today's rule (the review's table) and under (d')'s test on the real
   rate, against the warmest living bodies; `X` from 1e6 up.
3. **`denature`:** today's form against the two-state form, on the eaters at 310 K and at their `COOL_T`, and the share
   of eater updates above `DENATURE - 10 K`.
4. **Rot by moisture class:** the ground's dead matter and the mineral 24 on land by moisture class (m 0 to 0.2, 0.2 to
   0.5, 0.5 to 1) under (d'), and the parts' walls' life on globe36.
5. **Genes of 1 or 21 between 0 and log10 `X`:** the living bodies that hold one on `benchZ`, by kind.
6. The cost of a stand update and an animal update under (b) and (d').
7. The key numbers on 3 seeds under (b) and (d'), with S50, the minerals in bodies, dormant bodies; globe36 with life
   (3 seeds, 30 years). Reaction by reaction only if the 3-seed land mean moves past the seeds' noise.

**Then a design review of lane B** chooses (b), (d') or "not built", with its numbers, before B1 is written.
**Gate for B1:** the climate keys in range and the year 1T -3% on island22.

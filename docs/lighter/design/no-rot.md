# Idea 3: living bodies do not rot (lane B)

Steps: PR3, B1, B2 (`plan.md` section 4). Decisions: L5, L9. Handover 13.2 lists it as the owner's choice: "The litter
constants also run inside living bodies (real living stems and bones do not rot). The rock salt cannot be kept in a
body (real bodies bind calcium in solids)." The owner now says yes.

## 1. What the code does today

**The rate law** (`laws.md` 2.1, `change/kernel.rs`, `Reactor::rates`): each reaction's rate is `cr · k exp(-Ea/T) ·
contact · ...`, `cr` its catalyst factor. In the open world `cr` is `Body::catalyst(open, moist)` = `1 + (open - 1)
moist`, the decomposers. Inside a body, `body/physics.rs`, `react`, sets the factors:
- a reaction whose gene is on (`k.cat[r]` = `10^gene` > 1): `Body::catalyst(f, act · inhibited)` = `1 + (f - 1) a`;
- a reaction whose gene is off (`k.cat[r]` = 1, `Body::prepare`): `Body::catalyst(1, act)` = **1, the bare rate**;
- a catalysed burning that breathes (`pay::breathes`): set to **1**, since it ran on demand in `body::pay` and "goes
  here at the bare rate" (the open air burns the stores bare).
`Reactor::rates` skips a reaction only where `cr <= 0`, or where `cr <= 1` and the node is colder than the reaction's
frozen point (`thr`, `frozen_below`). `Body::rate_at` (`body/metab.rs`) uses the same `1 + (f - 1) a` for what the
catalysts release in `pay` and supply; "a dry body's bare reactions still run" (its doc).

**What runs bare in a body** (chemistry 0, `worldgen/src/chem0.toml`): 1, the burning of the energy-rich solid (0.27 a
year at 290 K bare); 21, the burning of body matter (orders 1 and 6; the decomposers rot it 5 times faster outside); 22
and 23, the lime-salt pair (23 back to lime is third order, about 17 days, P1.2); 25, the mineral 24 breaking down hot;
26, the frame (bare a millionth of its rot). R2 (`measures.md`, "P0"): **89.9% / 90.4% / 90.8%** of the reactions that
run in a stand's reactor are bare; **all** of an animal's (its reactor work is its burnings at the bare rate). PR-A: the
bare burning of an eater's own matter is **5 to 8 times its upkeep** (X43). The reactor is about 14% of a stand update
(PL1: 12.6 of 93.7 thousand instructions before PL1).

## 2. What changes

**B1, a law (level 4), variant (a) of L9:** a living body runs only what its own catalysts run, and of a catalysed
reaction only the catalysed part:
- one helper gives a body's factor: `(f - 1) · a` (no bare `1`), used by `react` and by `Body::rate_at`; the open
  world keeps `Body::catalyst` (the decomposers, `change/kernel.rs` and `change/balance.rs`);
- a reaction whose gene is off has factor 0, so `Reactor::rates` skips it;
- a catalysed burning runs on demand only (`body::pay`): factor 0 in `react`.
So a dry body's reactions stop (it is dormant; it does not rot), and the factor is continuous in the gene (0 at the
gene's 0, as today's catalysed part is), which keeps selection smooth: no jump from 0 to the bare rate as a gene turns
on. For a strong catalyst (f of 10 or more) the rate changes by `1/f` or less.

The books do not change: the same `Reactor::react`, the same booking; fewer moves. Dead matter on the ground, in the
liquid and in parts rots as before (the open world). A body that dies leaves its stores to the ground, where they rot.
What it lost bare before (the gas 3, the liquid 0, heat, the minerals 24 and 23 back from matter) it now keeps.

**Variant (b)**, if PR3 chooses it: only the reactions with `open > 1` (1, 21, 26: the rot) get factor 0 in a body; 22,
23 and 25 run bare as today. **Variant (c):** (a) for stands only (two kinds of body code, said in `laws.md` 6).

**B2 (level 1 on B1's world):** in `body::physics::physics`, a body with no reaction of factor above 0 and no light
reaction to run does not call `Reactor::react` (its heat stage keeps `e`, `h.heat`, `brood_share` as `react` would
leave them with nothing reacted); `Reactor::rates` loops only over the reactions a body's genes turn on (a list made in
`Body::prepare`, beside `k.burn`). An animal then runs no reactor at all.

**Docs and tests.** `laws.md` 2.1 ("no decomposer works in a body: it runs its reactions by its own catalysts" gains
"and only those, and only their catalysed part"), 2.4 (the factor), 6.2 and 6.5 (a dry body is dormant), 6.9 (minerals
in bodies, under (a)); handover 13.2's "Inside bodies" item closed; `numbers.md` rows of reactions 1, 21, 26 (their
anchors say "outside bodies"). Tests: `a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses` (its heat
made counts no bare burning), `a_dry_bodys_catalysed_rate_falls_toward_the_bare_rate` (becomes "... falls to zero"),
`upkeep_and_supply_follow_the_rate_law_of_the_reactions` (the factor), a new
`a_living_body_runs_only_its_own_catalysed_reactions`.

## 3. Risks

- **The eaters' budget.** Their bare burning of matter is 5 to 8 times their upkeep (PR-A). Without it an eater needs
  far less food: eaters may grow until they eat most of the plants' production (handover 13.2, "What eaters eat"), and
  their key numbers (5.354 ± 0.5 log10 kg; life 10.47 yr x/ 2) may leave their ranges. That is the honest effect of
  the law, but a crash of the plants would be a broken world. Variant (c) is the fallback.
- **The land's temperature.** P1.2 moved the land -0.78 K, the swing -0.62 K, rain on land -2%, and the light-eaters'
  first years to a third; the cause was not found (X63). P1.2 changed several things at once: the month law, stands at
  their ground's temperature with no sweating (P1.3 folded in), and the reactions in sub-steps (its first round ran the
  bare ones too: 47% of its short sub-steps were cut by the salt 23 turning to lime). Its prototype PR-M ran **no** bare
  reactions in stands and moved the land -0.52 K, which PR-M put on where growth's heat goes and which draw is set (its
  stage 6), not on the bare reactions (`measures.md`, "PR-M" 8). So the bare reactions are one suspect among several.
  They put out the gas 3 and the liquid 0 to the air and heat to the body: their loss may lower the heat-absorbing gas
  (a colder land) or the land's evaporation. PR3 must say whether they alone move the land, and by how much.
- **S50**, the light-eaters' stored share, rises if stored solid no longer burns bare (it is 0.558 against "under
  0.5" already; P1.2 gave 0.636). Reported, with its cause.
- **The minerals in bodies (variant (a)).** The salt 23 no longer turns to lime in a body; growth uses 23 as an input
  (matter is made of 118 of 5, 3 of 24 and 1 of 23). PR-A found the eaten mineral turning solid before growth; under
  (a) it stays for growth. Read the minerals' columns (`min<q>_bodies`).
- **The long-step test** has 2% of room (+0.293 K of 0.3 K); a law that changes the land's response to the step length
  could pass it; it is run in B1's checks.

## 4. What PR3 must show first

On a branch with a switch (off by default), `#[inline(never)]` on `physics`, `react` and `Reactor::rates`:
1. On each `benchZ` save, one year, one thread: per reaction, in stands and in animals apart, how many calls run it and
   the moles and heat it moves a year; the gas 3 and 2, the liquid 0 and the heat it gives the air, against the
   world's flows of the same (the ground's rot, the light reaction, the land's evaporation).
2. The cost of a stand update and of an animal update, today and under (a), (b), (c) (instructions on the same save:
   B1 changes no saved state, so `benchZ` loads).
3. The key numbers (3 seeds) under (a), (b), (c), with the minerals in bodies and S50.
4. **P1.2's -0.78 K:** 32 x 32, seed 1, 30 years (P1.2's setting), today against (a): the land's temperature, the
   gas 3, the land's evaporation and the light-eaters' members and mass year by year; then each bare reaction switched
   off alone (1, 21, 22 and 23 together, 25, 26) until the shift over 0.3 K is named. If (a) moves the land by less
   than 0.3 K, P1.2's shift belonged to its other changes (the month's heat stage, PR-M 8), and PR3 says so.

**Gate for B1:** the variant whose key numbers hold (as `plan.md` B1 lists) with the cause of any land shift over 0.3
K named; among those, the cheapest. A stand update at least -5% on `benchZ`.

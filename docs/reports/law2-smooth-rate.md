# Law 2 without a jump: a study of a smooth rate law

15 October 2026. Branch `law2-smooth-rate`, from `main` at 03123d8. **A study, not a merge.** The owner
decides. Runs: 64 x 64, 60 years, `--every 1000`, seeds 1 to 9, `main` against the branch.

## Short answer

- **The problem is real.** Law 2 counts a reaction "per unit of its scarcest input". When another
  input becomes the scarcest, the rate jumps. The jump is the ratio of the two inputs' shares. For
  reaction 22 on land (rock + carbon gas + liquid) it is about 3,000 times. At year 60 (seed 1), 1,104
  of 1,573 land cells are within a factor 10 of that break, about half on each side.
- **The proposed law** is plain mass action with one idea, the **contact** of the phases:
  `rate = catalyst x k x exp(-Ea/T) x contact x (product of all inputs' shares to their order)`.
  `contact` = what the scarce amount would be if each input were its whole phase (its gas, its
  liquid, or the solid itself). One line of code. No jump. Still first order in a trace input. The
  exact exponential step is kept.
- **In words:** a reaction inside one phase goes with that phase (as now). A reaction between a solid
  and the air goes with the solid, like a surface. Nothing goes faster than its smaller phase allows.
- **Most anchors hold unchanged:** dead plant matter (Olson, 0.27 a year), dead body matter (0.2 a
  year), nitrogen fixing, every reaction with one input, every reaction inside one phase.
- **Three re-anchors**, from the same real figures:
  1. reaction 22 (the sea's carbon): k 1.0e-4 -> 1.62e-2;
  2. the light-eater founders' light gene: +2.34 (`LIGHT_X`), as `LITTER_X` was done;
  3. a catalyst's price counts every gas at its air share; `c_cat` 2.14e9 -> 1.02e10, so the eater
     founders pay exactly what they paid.
- **Results, 9 seeds** (medians, years 56 to 60, main > branch): carbon gas 324 > 329 ppm; **plants
  4.0e10 > 2.9e10 kg (-28%, lower on all 9 seeds)**; eaters 1.6e8 > 1.7e8 kg (alive on 8 > 9 seeds);
  salt on land x1.5 to x2.0; salt in the sea +2 to +8%. The books close (drift 1e-13).
- **Speed** (seed 1 and seed 3, run side by side): Change 198 > 321 and 228 > 366 ms a year (x1.6).
  The whole year is 11% faster, because there are fewer bodies.
- **Recommendation:** adopt the law. It is simpler, smoother and more physical. Before merging,
  the owner should accept (or ask to study) the one clear effect on life: plants are about a quarter
  lighter, because their light reaction is now limited by their enzyme, as real plants are.

## 1. The law as `main` writes it

`core/src/change.rs` (`Reactor::react`, `share`, `scarce`):

- **lim** = the least, over the inputs, of (amount / count). The first of the least wins a tie.
- **rate per unit of lim** = catalyst x k x exp(-Ea/T) x the product, over the inputs in a gas or a
  liquid, of their **share** of the phase to the power of their **order**. The scarce input uses
  order - 1 (it already counts once through lim).
- **order** = `Reaction::ord` if the data gives it, else the count (only reaction 21 has `ord`: the
  oxygen-like gas to the 6th power, not the 40th).
- **share** = amount / all of that phase in the node. A solid is its own phase: share 1.
- **extent** in a step = lim x (1 - exp(-rate x time)).
- **Phases** come from the node's temperature (melting and boiling points).
- **Gas for a ground node** = the node's own gas + its cell's part of the air node above (1/16 of it:
  an air node covers 4 x 4 cells). So a ground cell's gas phase is the whole air column over it,
  about 3.5 x 10^5 mol per m2. Its liquid phase is all its liquids (the common liquid, the two salts,
  the oil when liquid).
- **Gas for a body** = the air it exchanges in its interval (gas gene x open area x days), at the air's
  shares. In `body::plan` (burning on demand) each other input of a body is a phase of its own.

**Where it jumps.** Take two inputs A and B. Where they are equally scarce (per count), the rate with
A scarce, divided by the rate with B scarce, is **share(B) / share(A)**. Inside one phase this is
count(B) / count(A) (2 or 3). Between a gas and a solid it is 1 / (share of the gas): 2,500 to 3,000 for
the carbon gas, 4.8 for the oxygen-like gas, 66,000 for the reduced carbon gas (CO, 15 ppm).

**Which reactions cross the break in a run** (`main`, seed 1, year 60, all cells):

| reaction | what | land cells within x10 of the break | jump | matters? |
|---|---|---|---|---|
| 22 | rock + carbon gas + liquid -> salt | 1,104 of 1,573 (805 rock-scarce, 768 gas-scarce); sea 68 | 2,940 | **yes**: the land's rock fell from 2,600 to 103 mol/m2 (median) |
| 18 | silicate rock + carbon gas (weathering) | 999 | 2,940 | little: its rate is tiny |
| 10, 11, 12 | ores + reduced carbon gas | 400 to 600 | 66,000 | not now (cold); yes in fires |
| 5, 7 | charcoal + oxygen-like gas | 47 to 69 | 4.7 | little |
| 1, 21, 24, 17, 19, 20 | litter, body matter, fixing, rust... | none (the solid is always far scarcer) | | no |
| bodies, burning (`plan`) | fuel + air breathed | an eater founder: 300 mol of fuel, 252 mol of the gas breathed in 10 days: **at the break** | 4.8 | yes |
| bodies, light reaction | air breathed + body liquid | only a very dry body | 2,500 | rare |

Reaction 22 on a land cell at 338 ppm (118 mol/m2 of the gas over it, 1,705 mol/m2 of liquid), salt
made per m2 and year (computed from the two laws):

| rock, mol/m2 | 10 | 100 | 117 | **119** | 300 | 1,000 | 2,210 |
|---|---|---|---|---|---|---|---|
| main | 0.0006 | 0.006 | 0.0075 | **22.4** | 22.4 | 22.4 | 22.4 |
| branch | 0.10 | 1.04 | 1.22 | **1.24** | 3.1 | 10.4 | 17.8 |

Main jumps 3,000 times between 117 and 119 mol/m2. The branch grows with the rock, and stops growing
where the rock passes the liquid (then the liquid is the smaller phase).

## 2. Options I considered

1. **Keep the law.** It has the jump. No.
2. **"lim is only a cap; the rate always counts every fluid input's share."** This counts a trace
   gas twice: second order. It is the bug fixed on 13 October (methane lasting 2 million years). No.
3. **Blend the two sides of the break** (for example a harmonic mean). Smooth, but with no physical
   meaning, and one more number. No.
4. **Surfaces and areas** (a solid's surface as its amount^(2/3); a gas meets a liquid over the
   cell's area). More real, but it needs new numbers (grain size, areas) and new code. Not lean.
5. **Mass action with a fixed "extent" per reaction** (for example always the liquid). Not general:
   which phase for a gas and a solid?
6. **Mass action with the contact of the phases (chosen).** `contact` = the least, over the inputs,
   of (the amount of the input's **phase** / its count): `lim` with each amount replaced by its
   phase's amount.

**Why option 6.**
- **Simple:** one line, the twin of `lim`. No new number.
- **Exact where the old law was right:** inside one phase, contact = that phase, so the rate is the
  old rate. Every reaction with one input is unchanged.
- **Physical across phases:** the contact between two phases cannot be bigger than the smaller one.
  A solid in air is the smaller phase, so the rate goes with the solid: **a surface law**. Dead matter
  rots in proportion to what there is of it (Olson's first order). Rock dissolves in proportion to the
  rock present (real dissolution goes with the surface, so with the solid).
- **"Per unit" is kept:** a trace input at the first power makes the rate go in proportion to its
  amount, so the share of it lost per unit of time does not depend on how much there is (test). For a
  trace gas against a solid, that share per unit of time is k x exp(-Ea/T) x (solid / air column): it
  depends on how much solid there is, not on how much gas.
- **The step stays exact:** extent = lim x (1 - exp(-(rate / lim) x time)), exact for a first-order
  decay with the other amounts fixed, as before.
- **In bodies it gives a real co-limit:** the light reaction goes with the body's liquid (where its
  enzyme works) until the gas it breathes runs short. On `main` the rate was proportional to the gas
  breathed: a plant that breathed twice as much made twice as much with the same enzyme, and when not
  saturated its rate grew with the square of its interval. That is not physical.

## 3. What changes

**Code** (`git diff 03123d8 --numstat`; change.rs 710 -> 736 lines):

| file | lines + / - | what |
|---|---|---|
| `core/src/change.rs` | 66 / 40 | `pace` (the law, 12 lines) replaces `share`; `react`, `fastest`, `balance` use it; `balance` takes the rock the salt uses while it solves; the speed gate bounds the contact (16 lines) |
| `core/src/body.rs` | 13 / 14 | `plan` uses `pace`; a catalyst's price counts every gas at its air share to its order |
| `core/src/lib.rs` | 5 / 2 | `c_cat` 2.1429e9 -> 1.0204e10 |
| `worldgen/src/life.rs` | 8 / 1 | `LIGHT_X` = 2.34 added to the light-eater founders' light gene |
| `worldgen/src/chem0.toml` | 6 / 2 | k of reaction 22 |
| `cli/src/chem.rs` | 3 / 1 | `ew chem` uses the start's real loose ground (1 m by volume) |
| tests, docs | | `core/tests/rate.rs` (new), `dissolved.rs`, `numbers.md`, handover 6.2 |

**The speed gate needed the contact.** My first branch runs did not have it: every wet cell then
reacted every step, and Change took 1,700 ms a year instead of about 330. The gate now bounds a
reaction of the air's gas by what the node holds over the air column. A test
(`a_cell_at_balance_does_not_react_every_step`) gives 245 changes in 39 steps, as on `main`.

**Anchors, reaction by reaction** (chemistry 0; "same" = the same rate in the run's cells, measured
on main's year-60 world with a snapshot tool):

| reaction | anchor | under the new law | to keep the real figure |
|---|---|---|---|
| 0, light (bodies only) | the founders' speed-up (gene 9) | the contact is the body's liquid, not the air it breathes; a founder breathes 217 times its liquid in its interval | **genes +2.34** (`LIGHT_X`, test); its price is then 0.42 of before |
| 1, litter | Olson: 0.27 a year at 290 K | same (the litter is the contact) | nothing (test) |
| 21, body matter | 0.2 a year at 290 K | same | nothing (old test, 17 to 20.5%, passes) |
| 22, rock + gas + liquid | gross air-sea exchange: 0.19 a year per unit of gas over the sea, 288 K | the rock is the contact: per unit of gas, k x exp(-Ea/T) x rock / air column | **k 1.0e-4 -> 1.62e-2**, at the start's sea floor (2,710 mol/m2 of rock less the 500 the salt took at 500 m) under 3.59e5 mol/m2 of air |
| 23, the salt settling | balance 1.0 mol/m3 at 500 m, 400 ppm, 288 K | same (one phase) | nothing: 1.003 mol/m3 at the anchor on both |
| 24, fixing | founders' gene 8.6 (7.1e-9 mol/kg/s in a test) | same (the fuel is the contact) | nothing (test passes) |
| 25, 3, 4, 9, 15 (one input) | | same | nothing |
| 2, 5, 6, 7, 16, 17, 19, 20 | first guesses | same where they run (the scarce input is in the smallest phase) | nothing |
| 8, methane in air | none (k a first guess) | **x0.5** (the count of the oxygen-like gas, 2, now sets the contact) | k x2 to keep it; not done: it has no real figure, and a longer life is nearer the real 9 to 12 years |
| 18, weathering | none (k 0.01 a first guess) | **x0.03** (rock / air column) | k x30 to keep it; not done (no real figure) |
| 10, 11, 12, smelting with the reduced carbon gas | none | **x10^-3 to 10^-5** in open cells (the gas is a trace) | none; real smelting needs a gas rich in it, which a fire node can have |
| bodies, burning (`plan`) | the founders' burning genes (`LITTER_X`) | same while the fuel is far below the air breathed; slower when the fuel is between 21% and 100% of it: an eater founder releases 22% less over its 10 days (the same over 1 and 20 days) | not re-anchored (see 4.3) |
| catalyst price (`c_cat`) | the 30 kg founders pay what they paid | the capacity counts every gas at its air share to its order (it counted only gases beyond first order) | **2.14e9 -> 1.02e10**: eater founders pay exactly the same; light-eater founders 0.61 of before (the light enzyme is cheaper) |

**What the sea now does differently.** At balance in a wet cell: salt^3 = (k22 / k23) x
exp(4,847 / T) x (share of the gas) x (rock left) x (liquid)^2. On `main` the air column stood where the
rock is. So the store now goes with the rock to the power 1/3. The depth rule (power 2/3) is the same.
The anchor depends on this map's sea-floor rock, as it already depended on its 500 m.

## 4. Results

`main` at 03123d8 against the branch's commit. Medians and per-seed numbers are means of years 56 to
60, except where a year is named.

### 4.1 Carbon gas and salt

| seed | ppm, year 10 | ppm, years 56-60 | salt on land, year 60 (mol) | salt in the sea, year 60 (mol) |
|---|---|---|---|---|
| 1 | 266 > 292 | 337 > 324 | 5.8e10 > 1.15e11 | 4.31e12 > 4.40e12 |
| 2 | 246 > 339 | 287 > 331 | 5.5e10 > 9.0e10 | 4.34e12 > 4.69e12 |
| 3 | 268 > 283 | 324 > 309 | 4.1e10 > 6.3e10 | 4.17e12 > 4.27e12 |
| 4 | 310 > 322 | 342 > 335 | 2.1e10 > 4.0e10 | 4.90e12 > 5.02e12 |
| 5 | 191 > 309 | 299 > 315 | 3.2e10 > 4.8e10 | 4.79e12 > 5.04e12 |
| 6 | 261 > 288 | 290 > 329 | 6.2e10 > 1.18e11 | 4.28e12 > 4.56e12 |
| 7 | 310 > 309 | 316 > 346 | 3.7e10 > 6.5e10 | 4.50e12 > 4.75e12 |
| 8 | 264 > 315 | 328 > 308 | 4.8e10 > 8.0e10 | 4.60e12 > 4.69e12 |
| 9 | 303 > 330 | 366 > 368 | 2.5e10 > 4.6e10 | 5.56e12 > 5.70e12 |
| median | 266 > 309 | 324 > 329 | | |

- The carbon gas is higher in the first decade on 8 seeds of 9 (the plants start slower), and about
  the same by year 60 (median +5 ppm; lower on 4 seeds, higher on 5).
- Land holds 1.5 to 2 times more salt on every seed: where its rock is short it still makes salt,
  where main nearly stopped. The sea holds 2 to 8% more (it gave back less, the air being higher).
- Seed 1 decade by decade (main > branch): 266 > 292, 282 > 307, 294 > 317, 305 > 318, 326 > 322, 338 > 325
  ppm. Seed 2: 246 > 339, 264 > 322, 279 > 330, 283 > 333, 285 > 334, 286 > 330.
- Ground temperature, years 56 to 60: within 0.35 K on every seed (lower on 8 of 9).

### 4.2 Plants and eaters

| seed | plants kg | eaters kg |
|---|---|---|
| 1 | 3.2e10 > 2.8e10 | 2.9e9 > 4.9e8 |
| 2 | 4.5e10 > 3.2e10 | 2.5e7 > 4.1e6 |
| 3 | 4.3e10 > 4.2e10 | 1.4e9 > 1.3e6 |
| 4 | 3.1e10 > 2.8e10 | 1.6e8 > 1.7e8 |
| 5 | 5.6e10 > 2.7e10 | died out > 3.4e4 |
| 6 | 4.6e10 > 3.2e10 | 7.7e7 > 2.0e9 |
| 7 | 4.0e10 > 3.1e10 | 4.8e7 > 1.6e9 |
| 8 | 3.6e10 > 2.9e10 | 1.8e9 > 1.1e8 |
| 9 | 2.3e10 > 2.1e10 | 2.0e9 > 2.9e9 |
| median | **4.0e10 > 2.9e10** | **1.6e8 > 1.7e8** (alive on 8 > 9 seeds) |

- **Plants are lighter on all 9 seeds** (median -28%). The cause is the body side of the law. A
  counter of the light reaction (seed 2, years 3 to 10) shows it. The founders use the same share of
  their light at year 3 (3% on the branch, 4% on main). On main the plants then raise it to 44% by
  year 10: the share of the gas they breathe that they fix stays near 0.6 while they breathe more,
  because on main more gas breathed gives more product with the same enzyme. On the branch that share
  falls from 0.69 to 0.50 and they reach 24%: a body limited by its enzyme needs more enzyme, not more
  gas. The light gene hardly evolves in 60 years on either (seed 3: 9.25 on main; 11.6 = 9.25 + 2.34 on
  the branch). A run with the new law for the open world only (bodies on main's law) has plants
  -11%, +34%, -6% on seeds 1 to 3: no clear effect there.
- **Eaters do not change** over 9 seeds (median 1.6e8 > 1.7e8 kg). On seeds 1 to 3 alone they looked
  lower (x0.17, x0.16, x0.001), and I spent several runs on that (4.3). With 9 seeds it is the chaos
  of eaters: they swing by up to 1,000 times between runs that differ a little.

### 4.3 The eater runs (tried and not kept)

| variant (seeds 1 and 3 unless said) | eaters kg | plants kg |
|---|---|---|
| main | 2.9e9, 1.4e9 | 3.2e10, 4.3e10 |
| main, seed 3 with one more round of mutation at the start (`--spray 21`, a control) | 2.1e9 | 4.6e10 |
| branch | 4.9e8, 1.3e6 | 2.8e10, 4.2e10 |
| branch with main's law in `plan` | 9.7e8, 6.9e8 | 2.9e10, 4.0e10 |
| branch, eater founders' burning genes +0.6 (their 10-day release back, at 4 times the enzyme upkeep) | 6.7e6, died out | 2.8e10, 4.1e10 |
| branch + that, with `c_cat` 2.55e9 so they also pay what they paid (`EATER_X`); 9 seeds | median 1.7e8, alive on 6 of 9 | median 3.3e10 |
| the same, main's law in `plan` for animals only / for plants only | 7.3e8, 2.4e7 / 8.5e8, 8.0e4 | about 3.0e10, 4.6e10 |
| new law in the open world only (bodies on main's law, main's price); seeds 1, 2, 3 | 8.6e8, 1.7e8, 1.6e8 | 2.8e10, 6.1e10, 4.0e10 |

None gives a clear signal beyond the chaos. `EATER_X` made eaters die out more often and three tests
fail (two of the `--sate-all` switch's windows, one statistical), so it is not on the branch.

### 4.4 Books and speed

- **Drift at year 60** (seeds 1 to 3): elements 7.9e-14, 1.0e-13, 1.1e-13 (main 6.9e-14, 9.3e-14,
  8.0e-14); energy 1.5e-13, 1.1e-13, 1.4e-13 (main 1.4e-13, 1.5e-13, 8.5e-14). Rounding.
- **Repeatable:** main seed 1 run twice gives the same hash.
- **Speed**, pairs run side by side (4 cores, shared with another job):

| | main | branch |
|---|---|---|
| seed 1: s a year (all) | 4.35 | 3.87 |
| seed 1: Change, ms a year | 198 | 321 |
| seed 3: s a year | 4.50 | 3.98 |
| seed 3: Change, ms a year | 228 | 366 |
| cells that react every step, year 40, seed 1 | 158 to 165 | 216 to 245 |

Change costs 1.6 times more: more land cells hold salt, and the salt's way back (reaction 23) makes
them react every step. The whole year is 11% faster only because there are fewer bodies (body time
2.6 against 3.1 s a year). Seeds 4 to 9 ran four at a time: Change 307 to 341 ms a year on the branch,
227 to 426 on main (noisy).

## 5. Tests

- `core/tests/rate.rs` (new, 4, 0.02 s): (1) a sweep of rock from 1 to 10^5 mol/m2 in 2% steps on wet
  land: the salt made never falls and never grows faster than the rock, and the books close at every
  step (main's law would jump about 2,500 times at 144 mol/m2); (2) a trace gas against a solid loses
  the same share for 1 and for 100 mol; (3) litter is first order and loses 1 - e^-0.27 a year at
  290 K; (4) `LIGHT_X` equals the founders' log10(gas breathed / liquid) within 0.01.
- `core/tests/dissolved.rs`: the column has the start's rock (2,710 mol/m2, not 10,000); the balance
  law is stated with the rock in it (salt^3 / rock left goes with the gas, and with warmth as dH says);
  the long run starts from twice the salt made of the same rock. The anchor (1.0 mol/m3) and the
  speed test (245 changes) are unchanged.
- Whole workspace: 138 tests pass (`cargo test --release --workspace`).

## 6. Risks and doubts

1. **Plants are a quarter lighter.** It is the law (an enzyme limit is real), but the founders' genes
   were set under the old law, and the light gene hardly evolves in 60 years. If the owner wants the
   old plant mass, that is a question about the founders or about how fast genes evolve, not about
   the law. I did not tune `LIGHT_X` to an outcome.
2. **The sea's store now depends on the sea floor's rock** (cube root). The anchor uses this map's
   rock, like the 500 m. Real air-sea exchange does not depend on the floor; it goes through the
   dissolved gas. The cleaner way is the one `sea-carbon.md` already names: let the ground's reactions
   use the gas dissolved in the liquid (Flow's Henry law), not the air column.
3. **Land salt doubles.** On rock-poor land the branch makes salt 162 times faster per unit of rock
   than main did there (main's k22 counted per unit of rock only below the break). It costs Change time.
4. **Reactions with no real anchor move:** methane x0.5, weathering x0.03, smelting in open air
   x10^-3 to 10^-5.
5. **The price of catalysts changes:** light enzyme 0.42 of before, fixing 0.78, burning the same. An
   enzyme for a reaction with a trace gas becomes very cheap by capacity.
6. **Burning in bodies changes where the fuel is between 21% and 100% of the air breathed** (up to 4.8
   times slower per enzyme there). That is the physical side (the burning goes with the fuel and the
   gas's share), but the eater founders sit there. Over 9 seeds the eaters show no effect.
7. **Pairs that undo each other** (22 and 23) now have different contacts (the rock one way, the
   liquid the other). `Reactor::balance` handles it, but the balance depends on the rock.
8. **Chaos:** eaters swing by 1,000 times between near-identical runs. Three seeds were not enough to
   say anything about them; nine are barely enough.

## 7. Recommendation

1. **Adopt the law** (`change::pace`). It removes a 3,000-times jump with one line, keeps every
   one-phase reaction and the litter anchors exactly, and is closer to real chemistry (a surface law; an
   enzyme and supply co-limit in bodies).
2. **With it, keep the three re-anchors** (k22, `LIGHT_X`, the price and `c_cat`) and the speed gate's
   contact bound. Without the gate bound Change is 5 times slower.
3. **Decide on the plants' -28%** (risk 1): accept it as physics, or ask for a study of how fast the
   light gene can evolve.
4. **Later:** the dissolved-gas route for the sea (risk 2), and real figures for methane, weathering
   and smelting (risk 4).

**My confidence.** The law, its smoothness and the books: high (tests, 9 seeds). The re-anchors of
reactions 1, 21, 22, 24, the price and `LIGHT_X`: high (each from a real figure or the founders' own
rate). The plants' drop being caused by the light reaction's enzyme limit: medium-high (all 9 seeds, a
direct counter, and the open-world-only runs show no plant effect). Eaters unchanged: medium (chaos).

## Where things are

- `core/src/change.rs`: `pace`, `Reactor::pace`, `balance`, the gate in `step` (`fits`)
- `core/src/body.rs`: `plan`, `Body::prepare` (price)
- `core/src/lib.rs` (`c_cat`), `worldgen/src/life.rs` (`LIGHT_X`), `worldgen/src/chem0.toml` (k22)
- `core/tests/rate.rs`, `core/tests/dissolved.rs`
- `docs/numbers.md` (rows `c_cat`, `LIGHT_X`, reaction 22), `docs/handover.md` 6.2
- Run outputs (series and summaries; snapshots deleted): `/tmp/law2-main-s1` to `s9`, `/tmp/law2-new-s1`
  to `s9`; the variants `/tmp/law2-new2-*` (`EATER_X`), `-oldplan-*`, `-eatx-*`, `-open-*`, `-ctl-s3`. The
  snapshot tool and scripts: `/tmp/law2-tools`.

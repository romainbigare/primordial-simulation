# Design: G2, the parts' reactor law

The round "Lighter world", lane G, step G2 (decision 44 of `architect-4.md`; review-plan's third review T2 and T3;
PR8 in `docs/lighter/measures.md` on `probe/r0`; G1 in `measures.md`). A law fix for parts, level 4 for parts.

## 1. What is wrong today

Every 6 months (`REACT_MONTHS`) each part runs `bond::month::react_part` (`core/src/bond/month.rs`):

- **The gas.** Its pool is its cell's **whole** share of the air node (`w.mesh.share(i, AIR)`): about 3.3e12 kg of gas
  for one part on globe36. The open world's gas-phase reactions run on it for 6 months at once and turn over about
  1.1e8 kg of gas a call (used = made to 0.01%). This is the air's own chemistry, which Change already runs in the air
  node, run a second time inside a part.
- **The heat.** The heat of those reactions goes into the part alone (2.8e5 J/K: 223 kg of walls). The reactor halves
  its sub-step to keep 30 K a sub-step (`MAX_DT_K`), reaches its floor in 21% of the sub-steps, and the part passes 1e6
  K in every call (up to 2.8e8 K). At that heat `e^(-Ea/T)` is about 1, so whatever a wall can burn or rot burns inside
  the call: the walls do not rot at their litter constants.
- **Where the heat goes.** The gas leaves with heat at the part's temperature (`q_out`), and `settle` gives the ground
  the rest: heat pulses into the air and the ground of the cell twice a year. The books hold; the physics does not.
- **The cost** (after G1): the parts' reactor about 163 M instructions a year on globe36 (1,186 calls, about 137 k a
  call), 3.1% of the year; island22 has no parts, region24 20 calls.

## 2. The law

A part is a solid lying on its cell's ground in the still air. Over a call it reacts **its own matter** with **the air
over it**, and the heat of its reactions leaves through its surface as it is made.

1. **What it reacts.** Its walls (`Part::wall`), with the open world's reactions and the decomposers' catalyst as far
   as its ground is moist (`rc.moist`, today's). Its fill (the liquid in its hollow) stays a heat capacity only, as
   today.
2. **The gas it touches: the air column over its footprint**, not the cell's share. Pool = the air node's amounts
   x the cell's share x (footprint / cell area), the footprint the part's two longest outer sizes (`Part::size[0] x
   size[1]`). A part of 5 m² on a globe36 cell touches about 5e4 kg of air (about 1e4 kg of the oxidising gas), far more
   than its walls need, so a wall's rot is set by its solid, as for dead matter on the ground (a solid is a phase of its
   own, laws.md 2.1). Gas-phase reactions in the pool scale with it and fall about 1e8 times: the air's chemistry stays
   the air node's. The cell's ground node keeps its own claim on the cell's share (`change/rows.rs` `react_ground`);
   the part's is about 1e-6 of it or less, so no second full claim is made (T3.1).
3. **The heat path** (T3.2). The part reacts at its ground's temperature, held by what its surface passes in the time,
   as a body's reactions are held (`body/physics.rs` `react`, its `fixed`): the reactor gets the heat capacity
   `G · dt`, G the part's still-air conductance, `forced` x the air's molar heat capacity x sqrt(`wind_ground` / d) x
   its outer surface (d its longest size; Campbell and Norman 1998), the formula of a body's film. For the part above,
   G is about 75 W/K and `G · dt` over 6 months about 1.2e9 J/K, 4,000 times its own. A wall that rots an eighth of
   its frame in 6 months (about 4.5e8 J) then moves about 0.4 K: one sub-step, no search. After the call, `settle`
   books the part's heat beyond its ground's temperature to its ground node through the domain's `Moves`, as today;
   the gas it gives or takes crosses to the air node with its latent heat by `change::send_gas`, as today.
4. **The cadence** (T2): one call per 6 months, as today (`REACT_MONTHS`). Not Change's sleep (up to 18 calls a
   part in 6 months, which would save nothing). **Known limit:** the rates are read at the ground's temperature on the
   call's day for the 6 months before it (the rate at one temperature is not the mean rate over a season). The check
   in 5 measures that bias against the litter constants; if it is over 30%, the call reads the mean of its cell's ground
   temperature over the months since (one sum a cell, kept by the monthly part step).
5. **The books.** 64-bit, every move booked on both sides through `Moves` (`give`, `take`, `take_from_ground`, `gas`):
   the walls' changes, the pool's gas, the heat to the ground and the air. Nothing new is stored.

**Reuse.** The pattern of `react_ground` (a pool from the air node by a share, moisture, `send_gas`) and of a body's
reaction held by its surface (`fixed`). The still-air conductance per m² becomes one helper used by `body::exchange::film`
and by the part (`ground.rs` or `body/exchange.rs`, CLAUDE.md 2.1). Folding parts into Change's ground rows was
looked at and not kept: a part's walls are a node of their own (Bond's turns own them, by domain), and Change's rows
would need a second class and a second claim on the cell's air.

## 3. Cost

A call becomes one sub-step over the part's 13 or so running reactions: about a ground node's call, 18 to 24 k
instructions (PR8). On globe36: 1,186 calls x about 22 k = about 26 M a year, against about 163 M after G1. **About
-137 M, -2.6% of globe36's year** (`run_for` 5,329.5 M after G1). The island: 0; the region: about 0.05%.

## 4. Steps

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| G2 | The part reacts its walls with the air over its footprint, held at its ground's temperature by its surface (`G · dt`), once per 6 months; the still-air conductance one helper for bodies and parts; `laws.md` 3.1 and 2.2 (parts), `history.md`; tests below; golden re-pinned | `core/src/bond/month.rs`, `core/src/body/exchange.rs` (the helper), `core/tests/bond.rs`, docs | 4 (parts) | the globe36 year -2% counted (one thread, callgrind, the `benchZ` save: no state changes, so the base runs the same save); the checks of 5 pass | under -1.5% of globe36's year, or a check of 5 fails |

**Lines:** about +12 -6 (the footprint share, `G · dt`, the helper; the halving search stays for other nodes). No
`Config` or `World` field: no snapshot bump.

## 5. Checks

- **Test (fast, `core/tests/bond.rs`): `a_part_reacts_at_its_grounds_temperature`.** A part with walls of fresh frame
  on moist ground at 290 K, one 6-month call: its temperature after the reactions and before `settle` lies within
  `MAX_DT_K` of its ground's (the bound T3.5 asks; in fact under 1 K), the call takes one sub-step, and elements and
  energy are kept.
- **Test (fast, the same file): `a_walls_frame_rots_at_its_litter_constant`.** The same frame as a part's wall and as
  dead matter in its cell's ground over 6 months at one temperature: the two losses agree to 10% (one rate law).
- **globe36 with life, 3 seeds, 30 years** (T3.4): the walls' rot set against the litter constants at their cells'
  ground temperature (the mean life of a part's frame from `parts_mass` and `parts_formed`, against `1 / (k open
  moist)` there), not against today's runaway; zonal land temperature and rain; the air's gases (`air_<k>`) within
  their seeds' noise; the parts and shelter columns (`parts`, `parts_mass`, `structures`, `enclosed`, `inside_now`,
  `inside_cold`; M3's shelter watch). Walls will last longer than today: that is the fix, not a failure.
- The key-number world (an island) has no parts: the key numbers do not judge G2. Golden re-pinned with its reason;
  the debug checks of the books and overdraw; no thread dependence (Bond's turns are per domain, unchanged).

## 6. Open

- The seasonal bias of one temperature for 6 months (2.4): measured by the second check; the fix is in 2.4.
- A held part, or one inside a structure, still lies in its cell's air: the footprint holds for it too.
- Afloat parts lie on liquid: their footprint's air is the same column; their heat goes to the ground node of the cell
  as today (the liquid is part of it).

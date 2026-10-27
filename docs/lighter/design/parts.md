# The parts' reactor (lane G)

> **Status after phase P (L20):** G1 (the search start, level 1) is done; G2 (a part reacts as a ground node does, a law fix, about -3.5% of globe36) comes next with its own design and review.

Steps: PR8, G1 (`plan.md` section 4). Decision: L20. The first review found this cost; no owner's idea named it.

## 1. What the code does today

Parts (`core/src/bond/`, `laws.md` 3) react with the air every `REACT_MONTHS` (6) months (`bond/month.rs`, `step`,
`turns`; per domain on the pool, G7). `react_part` gives a part its cell's share of its air node's gas as a pool, the
moisture of its cell's ground (`rc.moist`: the decomposers of the open world work on it as far as the ground is moist),
and the heat capacity of what fills it (`fixed`), then calls `Reactor::react` on its walls (`p.wall`) for the whole 6
months (`dt`), with no catalysts (`cat` None: the open world's `Body::catalyst(open, moist)`). After it, the gas goes to
the air with its heat, as a ground node's does (`send_gas`).

`Reactor::react` (`change/kernel.rs`) runs sub-steps: each one is halved (`sub_step`) while the heat of the running
reactions would move the node's temperature by more than `MAX_DT_K` (30 K), down to `MIN_SUB` of the time left, at
most `MAX_SUBSTEPS` (48), and each sub-step works out the phases and rates again.

**Where the time goes** (`benchZ`, one year, one thread): on globe36 `Reactor::react` called from `bond::month` is
226.6 M instructions for 1,186 calls, **191 thousand a call**, about 10 times a ground node's (Change's rows: 257 M for
13,970 calls); **4.2% of globe36's `run_for`**, more than the air's band solve. The island has no parts and the region
17, so this is a globe item (and a later world's, as structures spread).

**A guess to test:** a part is walls of dead matter and frame with little to hold their heat (its own walls and what
fills it, no ground under its heat), rotting over 6 months in one call: the heat of the rot would warm it far past 30 K,
so the sub-steps halve many times, each with the phases and rates again. A ground node of the same matter has the
ground's heat capacity under it and reacts every 10 days.

## 2. What may change (G1, by PR8's finding)

Candidates, in order:
1. **Level 1 or 2:** the halving search of `sub_step` costs one `reach` per halving; if PR8 finds many halvings a
   sub-step, start each search from the last sub-step's length (the same sub-steps, found with fewer tries: level 1 if
   the lengths are the same, else level 2).
2. **Level 3:** the part's heat during its reaction is held with the ground it lies on (as its heat is settled with
   the ground after it), so its temperature moves as a ground node's does and the 30 K bound binds rarely. A law change
   of a part's heat (`laws.md` 3), with its effect on the parts' walls and the gas.
3. **Level 3:** parts react as Change runs a ground node (when due, then sleep), not every 6 months in one long call.

## 3. Risks

- The parts' walls are structures animals build and use (shelters, M3's watch-for): a change that makes them rot
  faster or slower changes the story. The walls' life (the share left after a year, by cell moisture) must stay within
  its seeds' noise.
- The books: what a part's reaction makes is booked as today; candidates 2 and 3 move heat between the part and the
  ground, which must go through the shared helpers (`Moves`).

## 4. What PR8 must show first

On a branch, counters (no world change), one year of globe36 (`benchZ`) and of the size-28 globe: per part reaction,
the sub-steps, the halvings per sub-step and in all, the temperature the part reaches, the reactions that run and the
one whose heat limits; what a part holds (walls, fill) and its heat capacity against its ground's; calls a year.
**Gate for G1:** a level-1 or level-2 candidate that cuts the parts' reactor by a third or more (1.5% of globe36's
year), or a law candidate that cuts three quarters or more (about 3%, L2).

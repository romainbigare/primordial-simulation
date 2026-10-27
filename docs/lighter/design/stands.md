# Idea 2: fewer stands a cell (lane A: A2, and A3 if the cover fails)

Steps: PR2, A2, A3 (`plan.md` section 4). Decisions: L6. A stand is a population node of a lineage without a brain
(the light-eaters).

## 1. What the code does today, and why a cell holds about 8 stands

R2 counted (`docs/speed-x10/measures.md`, "P0"): stands a cell that has plants, mean and most, 8.2 and 26 (island22),
4.6 and 14 (globe36), 10.3 and 28 (region24); one plant lineage in the world; young and adult stands 1,315 and 659 on
island22. Each stand updates every `node_days` (40), at about 84 thousand instructions after PL1, whatever its members.
So the plants' cost goes with the **number of stands**, not with the members.

What keeps stands apart:
- **Stage.** A kind in a cell is an adult node and a young node joined by a link (`copy/young.rs`: `linked`, `link`,
  `mature`): children are booked on the young node; the young members that pass the copy mass join the adult node. Two
  stands a kind at least.
- **Kin** (`nodes::Kin`): one lineage, one stage, the same room and held structure. For stands only the stage matters.
- **Alike genes.** A child joins the young node of its kind only if it lies within `alike` (4 mutation steps) plus
  `WITHIN` (0.5) standard deviations of the pooled spread in **every** gene (`body::within`, `copy::kin`); else it
  starts a young node of its own, which then matures into an adult node of its own.
- **The monthly merge** (`nodes::crowd`, from `survey::survey`'s cell pass): `join_alike` joins each body to the first
  larger alike node of its kin (`body::alike` on the means); then past `Config::crowd` (4) `merge_closest` merges the
  most alike pairs **while they are alike**: the pair's key is infinite when any gene lies past `within_steps`, so
  "unlike nodes never merge, however full the cell" (`laws.md` 4.4).
- **Splits** (`nodes::split`) add a stand only while its cell has fewer than `crowd - 1` bodies, so rarely in a full
  cell.

So a cell of 8 stands holds about four kinds, each young and adult, that differ by more than the alike limit in at least
one gene of the 127. `crowd` 4 bounds nothing for them.

## 2. What changes (A2, the tiny form; at most 30 core lines, L6)

**The rule:** in `merge_closest`, for stands (nodes of a lineage without a brain, `w.life.brainless`), the pair's key
past `crowd` is their squared distance in steps **whether or not they are alike**; same lineage and same stage are
still required. So past `crowd` stands a cell, the most alike pairs of one stage merge until `crowd` are left (about 2
kinds, young and adult). Animals keep today's rule (their kinds must stay apart for the nodes check and the brains).
`laws.md` 4.4's merging paragraph and `numbers.md`'s `crowd` row say it.

`nodes::add` pools the genes by members and pools each gene's spread with the gap to the new mean, capped at
`most(step, alike)` (`body::most`). So a merge of two unlike stands keeps their mean and **loses the variance beyond
the cap**: that is the cost the owner named (less gene variety held per cell). Selection inside the node
(`copy::select`) then moves the mean within the spread that is left; a split cannot follow in a full cell.

**Other forms PR2 may choose instead** (each within 30 lines):
- `crowd` larger for the alike merge and a second, larger count past which unlike stands merge (a new `Config`
  number, a speed number, with its `numbers.md` row);
- the cap of the pooled spread at merge raised for stands, so less variety is lost (a law change of its own).

**A3 (only if the cover fails its gate):** the wider rule PR2 names, for example one node for the young and adult of
one kind (the two stages as one node with a size spread across the copy mass; `copy::young` and `math::Pass` already
hold the flux across it). That is a larger change of `copy/young.rs`, `copy/mod.rs` (`kin`) and `nodes.rs`, and the
cover would delete it, so it waits.

## 3. Risks

- **Variety.** Plants must adapt to places (the goal). If unlike stands merge in every full cell, each cell keeps about
  two kinds; new kinds still arise by children that are not `within` (a young node of their own) until the next month's
  merge. The check: the light-eaters' mean spread per gene at year 30 at least half of today's, and kinds still differ
  by climate (the `root_*` columns by latitude or height).
- **Selection.** With wider means mixed, `copy::select`'s sample sees a node of mixed kinds; its step is one standard
  deviation of the spread left. `selection_inside_a_node_moves_its_mean_as_its_members_would` must pass.
- **Eaters.** Fewer, larger stands are fewer bodies to see and take from (`choose::perceive` lists faces; `take_body`
  takes one body by id); a cell's food does not change, but how it is met does. The eaters' key numbers say if it
  matters.
- **The pace and the canopy.** A merged stand has one height (`size_moments`) and one shade rank (`shade`); a canopy of
  two kinds of height becomes one mean. `the_ground_under_dense_canopies_stays_near_its_neighbours` (slow) must pass.
- **Order of work.** A2 changes the world (level 4), re-pins golden and re-runs the key numbers; B1 and A2 merge one at
  a time (`plan.md` 5).

## 4. What PR2 must show first

On a branch, counters (no world change), one year of each `benchZ` save, at each monthly merge:
1. stands a cell with plants: mean, 90th percentile, most; split by stage; the kinds a cell holds (groups that
   `join_alike` would join);
2. after `join_alike`: how many are left, and how many of the cells over `crowd`; after `merge_closest`: how many are
   left and why (every pair unlike); the distance in steps of the closest unlike pair, as a spread over cells;
3. a year's births: the share of litters that start a young node of their own (not `within`), and in which gene they
   fail most often;
4. the stand updates of the year, and their instructions.

Then A2's rule on the branch, 30 years of 64 x 64 (the key-number world) for seeds 1 to 3: stands a cell, stand
updates a year, the light-eaters' mean spread per gene at year 30 against today's, the key numbers. **Gate for A2:**
stands a cell -25% or more, the key numbers in range, the spread at least half of today's. If the gate fails at
`crowd` 4, PR2 tries `crowd` 6 for the unlike merge; if that fails, A2 is not built and the plants rely on C (A3 if C
fails).

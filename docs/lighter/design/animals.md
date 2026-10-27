# Lane H: the animals' bookkeeping in lumps (H1, H2; H4 a probe)

Steps: H1, H2, H4 (`plan.md` section 4). Decision: L39. Probe: PA (`measures.md`, "PA"; branch `probe/pa`).

## 1. What the code does today

**Every animal updates every batch.** Every animal's `INTERVAL` gene is at or under 10 days (selection took it to the
batch), so a node updates 36 times a year (PA). A young node's update costs 144 thousand instructions, an adult's 185
thousand. Animal updates are 40.2 / 32.7 / 43% of the year (island22 / globe36 / region24).

**Children** (`copy::copy`, `copy/mod.rs`; `laws.md` 4.2). Growth (`copy::grow`) puts the share `ALLOC` of what a
mature body builds into its brood (`Body::brood`). At each update `copy` reads the births the brood covers: for a node
`k - BROOD_HELD · n` (`BROOD_HELD` 0.5 a member: the members' broods lie evenly between none and one child). If any,
it draws the partner (individuals only), the disperse share and the direction, and calls `emit` twice (the dispersers,
the stayers). Each `emit` costs per call, not per child: `genes::vary` over all 127 genes, the children's brain rows
(`child_rows`, `brain::shake` over every row), the provisions, and the join: `start` then `nodes::add` into the young
node (`young::linked` or `kin`), or a litter in the outbox that lands at the batch's end (`land_litter`: `kin`, `start`,
`nodes::add`). An adult node makes children at 67% of its updates (PA); two `emit`s of one birth cost about 107
thousand instructions on the island. Children are 7.3 / 4.8 / 7.8% of the year, their litters' landing 1.8 / 1.1 /
2.2%. The brood leaves with dead members (`Body::lose` calls `brood_share`).

**Maturing** (`copy::young::mature`; `laws.md` 4.4). At each update of a young node, `survey::update` passes the mean
member body it had before the update; `mature` fits the young's log sizes (`math::Pass`, the McKendrick flux across the
copy mass) to the growth of this update (`grew` = log10 of now over before), splits off the members that passed
(`nodes::part`, `place_part`) and merges them into the adult node (`nodes::merge`, which is `nodes::add` and
`rejoin`). 69% of a young node's updates do this (PA). Maturing is 4.0 / 2.7 / 4.1% of the year.

**The join** (`nodes::add`): pools the joiner's genes, spreads and brain rows into the node by members, runs
`Body::prepare` and `Domain::see`: about 21 thousand instructions a call, 23,350 / 15,726 / 49,509 calls a year, 10.1 /
6.3 / 9.9% of the year with all its callers (children, maturing, the landing of moves and litters, stands, `crowd`).

## 2. What changes

**H1, children in lumps (a law step, level 4).** The brood fills at every update as today (growth, `ALLOC`, the
losses with dead members). A **node** makes children at most once in a window of `Config::lump_days` (30, a speed
number with its `numbers.md` row; the snapshot bumped): at the update whose window differs from the window of the update
before it, the window of a day `t` being `floor((t + phase) / lump_days)` with `phase` = `id mod lump_days` (its own
phase: no date that every body shares, so no world-wide pulse; no new state: the day before is `t - interval`). At that
update it makes all the births its brood covers, as today's rule does (the brood above `BROOD_HELD`): so the mean flow
of children is the same, in lumps of about three updates' worth, and `vary`, the rows and the join run once per lump
with a larger `m`. Individuals keep making children at every update (individuals stay real; they are under 1% of the
year); stands update every 40 days and so give birth at each update as today. `laws.md` 4.2 says it.

**H2, maturing in lumps (a law step, level 4).** A young node runs its maturing flux at most once in its window (the
same rule). Its growth over the window is kept in one `Body` field, the log10 growth of its mean member summed over
the updates since its last flux (each update adds today's `grew`), so joins between fluxes (children, litters) change
the size moments (`rejoin`) but not the growth; the flux uses the sum and resets it. The snapshot is bumped and the
benches remade. `laws.md` 4.4 says it.

**H4 (a probe, then maybe a step at level 1 or 2).** Count the joins whose joiner has the node's genes (a group that
moved and joins a node of its own kind, a part that matured back into its adult node) or genes within rounding of
them, and split `nodes::add`'s 21 thousand into the per-gene pooling, `prepare`, the rows' blend and `see`. A step only
if a cheaper path for those joins (skip what pools to the same numbers) is worth L2's line.

## 3. Risks

- **The brood and the timing of births.** Children come up to a window later than today, and the brood waits longer:
  it is lost with members who die in between (honest: a dead parent's brood dies with it), so births a year may fall a
  little where deaths are high. The young node gets children in lumps, so its size spread and its maturing see a step
  every window. Births a year, the young node's members and the eaters' key numbers must stay within their noise.
- **No calendar rule.** A month is the world's clock step, not a season: the window is a step length, and each body's
  window has its own phase from its id. A rule tied to a date shared by every body (all births at the month's first
  batch) would make a world-wide pulse of births and of prey for the eaters of eaters: not built.
- **Predators and prey.** Children that come in lumps are prey in lumps; a taker that meets a young node just after a
  lump finds more. The nodes check and watch-for M1 (populations rise, fall and cycle) say whether that matters.
- **The books.** Each lump is booked by today's code (`give`, `provisions`, `start`, `land_litter`) with a larger `m`;
  nothing new moves matter. `a_century_keeps_its_books` and the golden runs' books.
- **H2's field.** The young and adult counts differ by up to a window of flux from today's; the field is new state
  (the snapshot); a node that loses its young stage (all grown) resets it.
- **Individuals against nodes.** Individuals keep today's per-update births; the nodes check compares the two: a
  difference past the `--crowd` noise is a bug of the lumps (handover 7.1).

## 4. What the law checks must show

For H1 and H2 each (seeds 1 to 3; counted on the remade benches):
1. the counted year -3% (H1) and -2% on top (H2) on island22 and region24 (L2);
2. the key numbers, the eaters within their range (5.354 ± 0.5; life 10.47 yr x/ 2);
3. the nodes check (nodes against individuals), the pace and batch tests, `the_eaters_live_and_stop_growing`;
4. births a year, the young nodes' members and the share of members that are young, within their seeds' noise;
5. the books in every golden run and in `a_century_keeps_its_books`;
6. the tests restated: the copy tests that make children in one update state the window
   (`a_mature_nodes_children_gather_in_its_young_node`, `reproducing_updates_keep_elements_and_energy`), the maturing
   tests state the flux over the window (`young_members_mature_at_the_rate_their_growth_gives`,
   `any_spread_of_sizes_matures_a_share_of_the_young`); a new test: a node's births over 90 days are the same in lumps
   as at every update, within the brood held.

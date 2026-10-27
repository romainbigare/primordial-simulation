# Lane H: the animals' bookkeeping (H1, H5; H4 a probe)

Steps: H1, H5, then H4 (`plan.md` section 4). Decisions: L39, L41. Probe: PA (`measures.md`, "PA"; branch
`probe/pa`). H2 (maturing in lumps) and H3 (`think_days` 40) are not built (L39).

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
number like `batch_days`, with its `numbers.md` row; the snapshot bumped).
- **The window** is the world's batch index: `w = floor((batch + phase) / n)`, `n = lump_days / batch_days` (3), with
  `phase = id mod n` (each node its own phase: ids are `domain << 40 | placed`, and the phases spread evenly; no date
  that every body shares, so no world-wide pulse).
- **The node keeps the window of its last lump** (one small `Body` field, 0 for a new node; a node that joins another
  takes the receiver's, as a merge keeps the receiver's fields). It makes children at an update when `w` is past that
  value, then sets it. So a node lumps at most once a window, and never skips a window it updates in, whatever its
  interval gene does between updates (a rule on "the update before, `t - interval`" could lump twice or skip when the
  gene changes).
- At that update it makes all the births its brood covers, as today's rule does (the brood above `BROOD_HELD`): the
  mean flow of children is the same, in lumps, and `vary`, the rows and the join run once per lump with a larger `m`.
- **Dispersal:** one direction is drawn per birth call, as today (a node's dispersers go one way at a call; not one
  draw per child today either). So a node's dispersers go one way a window instead of about two: the mean flow to each
  neighbour is the same, its grain coarser. Not one draw per child: that would need an `emit` per direction, the cost
  H1 removes. Checked: litters landing where no node of their kind is, and the eaters' range on globe36.
- **The guard:** `Config::validate` refuses a `lump_days` that is not a multiple of `batch_days` or is over `node_days`
  (stands update every `node_days` 40, so they give birth at each update as today).
- Individuals keep making children at every update (individuals stay real; they are under 1% of the year).
- `laws.md` 4.2 says it.

**Its gain, an upper bound** (third review T1): an adult node makes children at 67% of its updates, about 24 birth
updates a year with two `emit`s each; a window allows 12 lumps a year, so the calls fall by at most half: about **-4.4
/ -2.9 / -4.9%** of the year (half of the children's 6.9 / 4.7 / 7.5% and of their litters' landing).

**H5, one draw per birth (level 2 or 3, L41).** `copy::copy` calls `emit` twice per birth, the dispersers and the
stayers, and each draws the genes (`vary`) and the brain rows (`child_rows`) again. H5 draws them once for the birth and
splits the members, the gap between the two groups' means going into their spread (`vary`'s spread left), so the
mean and the variance of the litter are as one draw gives them. About -2.3 / -1.5 / -2.4% alone, about half of that
after H1. No change of timing.

**H4 (a probe after H1 and H5, then maybe a step at level 1 or 2).** Count the joins whose joiner has the node's genes
(a group that moved and joins a node of its own kind, a part that matured into its adult node) or genes within
rounding of them, and split `nodes::add`'s 21 thousand into the per-gene pooling, `prepare`, the rows' blend and `see`.
A step only if a cheaper path for those joins is worth L2's line; before the cover (both edit `nodes.rs`).

**Not built: H2, maturing in lumps.** Its upper bound is about -2.1 / -1.4 / -2.1% (maturing comes at 69% of a young
node's updates, about 2.1 fluxes a window), under a law step's 3%; and its field (the growth since the last flux)
would need two rules the third review found missing: what a merge of two young nodes does with it (else the joiner's
crossed members stay young above the copy mass), and a window of growth then loss (maturing is final; a summed growth
would undo a crossing).

## 3. Risks (H1)

- **The brood and the timing of births.** Children come up to a window later than today, about 10 to 15 days later on
  average, at a season's end into colder weather; the brood waits longer and is lost with members who die in between
  (honest: a dead parent's brood dies with it), so births a year may fall a little where deaths are high. The young
  node gets children in lumps. Births a year, **births by month** (`months.csv`), the young node's members and the
  eaters' key numbers must stay within their noise.
- **No calendar rule.** A month is the world's clock step, not a season: the window is a speed number, judged against
  the per-update law (the nodes check against individuals), and each node's window has its own phase. All births at a
  month's first batch would be a world-wide pulse of births and of prey: not built; the pulse test guards it.
- **Predators and prey.** Children that come in lumps are prey in lumps. The nodes check and watch-for M1 (populations
  rise, fall and cycle).
- **The books.** Each lump is booked by today's code (`give`, `provisions`, `start`, `land_litter`) with a larger `m`;
  nothing new moves matter. `a_century_keeps_its_books` and the golden runs' books.
- **Individuals against nodes.** Individuals keep per-update births; the nodes check compares the two: a difference past
  its noise is a bug of the lumps (handover 7.1).

## 4. What the law checks must show (H1; H5 by the same list where it applies)

1. The counted year -3% on island22 and region24 (L2; the upper bound -4.4 / -4.9%).
2. The key numbers, the eaters within their range (5.354 ± 0.5; life 10.47 yr x/ 2).
3. The nodes check, the pace and batch tests, `the_eaters_live_and_stop_growing`.
4. Births a year, births by month, the young nodes' members and the share of members that are young, within their
   seeds' noise; litters landing where no node of their kind is, and the eaters' range on globe36.
5. The books in every golden run and in `a_century_keeps_its_books`.
6. Tests: the copy tests that make children in one update state the window
   (`a_mature_nodes_children_gather_in_its_young_node`, `reproducing_updates_keep_elements_and_energy`); new: a node's
   births over 90 days are the same in lumps as at every update, within the brood held; over N windows a node lumps at
   most N times, also when its interval gene changes between updates; the pulse test: no batch holds more than about
   one in `n` of the year's lumps, within a stated margin; `Config::validate` refuses a `lump_days` over `node_days`.

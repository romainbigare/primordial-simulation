# The pattern finder (`analyze/`)

`ew-analyze TAPE_DIR [--out cards.json]` reads a run's folder and writes fact cards (handover 9.1) as JSON. It only reads
the tape; nothing goes back into the simulation. It is its own command in its own crate, not a subcommand of `ew`: it
reads files and never runs a world, so it needs none of the run code (images, allocator, run flags), and the pattern
finder stays apart from the shell that runs the laws.

The output is `{"source": {"frames": N, "snapshots": [years]}, "cards": [...]}`. The cards hold numbers and categories,
no Earth names; naming them is the matcher's job (handover 9.3). Cards come in a fixed order, and the same folder gives the
same file. The thresholds are named constants at the top of `analyze/src/lib.rs`; they are settings of the analysis, not of
the world.

## What it reads

| File | Used for |
|---|---|
| `world.ewf.gz` | the frames (`docs/tape.md`): layers `animals`, `bodies`, `worn`, `traffic`. A square is a square of the map (a cell of many squares shows as many) |
| `snap_NNNNN.bin.gz` | the parts and bonds of `World::life`; every snapshot of the folder, oldest first. A snapshot of another format version is refused |

`series.csv` is not read: the three detectors need no yearly number.

## Cards that last (groups and routes)

A square is **on** in a frame by a rule on one layer. A square has a **run** when it is on for at least `LAST_FRAMES` (3)
frames in a row that span at least `LAST_YEARS` (20) years (the first years have a frame a month, so frames alone do not
say how long). Squares with a run that touch (eight neighbours, east to west round a globe) are one pattern.

| Category | On when | At least | Fields besides the common ones |
|---|---|---|---|
| `group` | `animals` (kg/m²) over `GROUP_DENSITY` (4) times the mean of all squares in that frame | `GROUP_MIN_SQUARES` (4) | `centre` (x, y in squares, mean of the squares), `peak_bodies` (most of the `bodies` layer summed over its squares in one frame; nodes and individuals, not members: the tape has no members layer) |
| `route` | `worn` (how much cheaper than uncleared ground, `lanes::eased`) at least `LANE_EASE` (2) | `LANE_MIN_SQUARES` (3) | `ends` (the two squares farthest apart, by the squares), `mean_traffic` (the `traffic` layer, 1/yr, over its squares and the frames of their runs) |

Common fields: `pattern_id` (`group-<first square>` or `route-<first square>`, the lowest index of its squares),
`category`, `cells` (squares in the pattern; for a route its length), `years` (the longest run of any of its squares),
`first_seen_year` (the earliest start of a run), `evidence_refs` (`frame:<year>` of the first and of the last frame of the
longest run).

## Cards of structures

The parts of a snapshot are grouped by structure (`Part::root`), each with its design (`bond::designs`, laws.md 3.1):
each part gets a label from its **material band** (the hardness and the bond strength of its walls, each cut into
`BANDS` (4) bands) and its **shape band** (hollow share and elongation, `BANDS` bands each), and a structure is hashed by
`WL_ROUNDS` (3) rounds of neighbour hashing over its bonds (Weisfeiler and Lehman 1968; Shervashidze et al. 2011), then
by the sorted labels: two structures with the same bond graph and the same bands have the same hash, wherever they lie.
A structure of one part hashes its two bands.

A card is made for a hash with at least `REPEAT_MIN` (2) copies in the last snapshot.

| Field | Meaning |
|---|---|
| `pattern_id` | `struct-<hash, 16 hex digits>` |
| `count`, `parts` | copies in the last snapshot; parts of each |
| `mass`, `hollow` | mean mass of a copy (kg); mean hollow share of its parts (0..1) |
| `enclosed_share`, `held_share` | share of the copies that are enclosed (`tape::ENCLOSED`); share with a part held by a body |
| `first_seen_year` | the earliest snapshot with the hash |
| `first_seen_exact` | true when an earlier snapshot lacks the hash; false when the earliest snapshot has it already (it appeared before) |
| `note` | only when the folder has one snapshot: the first seen is the end of the run |
| `evidence_refs` | `snap:<year>` of the first snapshot with it and of the last |

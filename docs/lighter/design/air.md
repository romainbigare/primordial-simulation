# Idea 7: a coarser air on globes (lane E)

> **Status after phase P (L12):** E1 is not built (PR7: -4.7% of globe36, but rain on land +6% to +12% on every seed and world); the code stays on `probe/pr7`. The alternative is lane G (G2).

Steps: PR7, E1 (`plan.md` section 4). Decision: L12. E2 (a new solver) is dropped; the alternative if E1 fails is lane G
(`design/parts.md`). The last round chose not to build a coarser air (X10,
`docs/speed-x10/design/air.md` 0 and 6.5); this note says what changed and why the form here is different.

## 1. What the code does today

- **The air level.** One air node over a block of `AIR_SCALE`² = 4 x 4 squares (`worldgen/src/terrain.rs`,
  `AIR_SCALE`; saved as `World::air_scale`; `World::mesh_of` builds the mesh with `Square::mesh(&[air_scale], ...)`).
  `Square::mesh` (`core/src/space/square.rs`) numbers the blocks row by row, on a globe in the folded order (0, w-1, 1,
  w-2, ...), "so the blocks either side of the wrap are numbered side by side and a banded solve of the level stays one
  row wide", and asserts that a globe's row is a whole number of blocks. Globe36 (`--globe --size 18`: 36 x 18 squares)
  has 9 x 5 = 45 air nodes; the size-28 globe (56 x 28) 14 x 7 = 98; the test globe of
  `a_globe_without_life_keeps_an_earth_like_climate` (64 x 32) 16 x 8 = 128.
- **The air's step** (`core/src/flow/air/step.rs`, `newton.rs`; `laws.md` 1.4): backward Euler by Newton's method; each
  round makes every ground cell's row (`pass`, on the pool) and solves the air level's system by a banded solve
  (`core/src/solve/band.rs`: "the cost is unknowns times the band squared"; the band is one row of air nodes wide).
  A cell reads its four nearest air nodes bilinearly on the map (`flow/scratch.rs`, `tables`: distances from
  `Mesh::on_map`, so any block shape works).
- **Domains are the cells under one air node** (`life/mod.rs`, handover 7.4): each domain owns its bodies, its ring
  and its view, and threads split by domain. A larger air node is a larger domain.

**Where the time goes** (one thread, `benchZ`, the end's profile): the air (`flow::air`) is 11.0% / 17.2% / 9.0% of a
whole run (island22 / globe36 / region24), 1,121 M on globe36 (21% of `run_for`); the Newton part (`newton.rs`, with
the band solve) 199 M on globe36 (3.7% of `run_for`), 90 M on island22; of globe36's 199 M the band solve is about 76 M
(`Band::factor` 39 M, `solve` 16 M, `step` 9 M, `zero` 8 M), **1.4% of the year**; the cells' rows (`pass`) 452 M on globe36
after G11. Rounds a step on globe36: 3.25 (AIR1). On a 72 x 36 globe the last round measured 6.9 rounds and `solve` 99
ms a year: the per-node part grows with the band's width, so it matters most on large globes, where the 15-minute
planet lives (size 28 today).

## 2. What changes

**E1 (level 4): air nodes wider than tall on a globe.** A block of `sx x sy` squares: `sy` stays 4 (the air keeps its
5 rows on globe36, 7 on size 28, 8 on the test globe: the rain belts and the heat carried to the poles stay), `sx` is
about twice that and must divide the row (globe36: 6, so 6 x 5 = 30 nodes; size 28: 8, so 7 x 7 = 49; the test globe:
8, so 8 x 8 = 64). The island and regions keep 4 x 4. The band is the row's nodes, so it is about half as wide: the
band solve's cost (unknowns x band²) falls to about an eighth, the rest of the per-node work by about half.

**The physical reason:** the map of a globe is equal-angle, so a cell at latitude φ is `cos φ` as wide as it is tall; at
60° a 8 x 4 block is about as wide as tall in metres. Today's 4 x 4 nodes are tall and thin away from the equator, so
wider nodes are closer to square there; the air's resolution in latitude, which carries the climate's bands, is kept.

**Code.** `Square::mesh` takes a block shape per coarser level (`(sx, sy)`; the square case is `(s, s)`); `block` reads
`x / sx` and `y / sy`; the globe's assert checks `w % sx`. `World::air_scale` becomes the pair (snapshot bumped);
`terrain.rs` picks it (4 x 4, or `(sx, 4)` on a globe, `sx` the largest divisor of the row from 5 to 8: 8 on size 28
and the test globe, 6 on globe36; **4 x 4 where none divides the row**, as 58 columns (size 29); a named `const` for the
8 with its one-line reason); `terrain.rs:282` (a node's centre row) reads `sy`. `core/tests/flow.rs:179` reads the width of the air
level from the mesh. Domains follow by themselves (the cells under one air node).

**How E1 is judged.** It bumps the snapshot, so `benchZ` cannot load in its build, and a 1 to 2% gain cannot be seen
across remade worlds (PR-S: only cuts over 11% count on globe36). So E1 is judged by **counted work** on same-seed
worlds of the same age made by both builds: the air's instructions a year and per air step, Newton rounds a step (the
air's work goes with the cells and the rounds, not with life's noise); and by the **4T year** on globe36, interleaved
under the lock, which must not be slower.

**Not chosen: E2**, a solve that grows with the unknowns (a Krylov solve): a large new solver for the band solve's 1.4%
of globe36's year. Only if PR7 found the band solve at 10% or more of the size-28 globe's year would it come back, by a
decision.

## 3. Risks

- **The climate.** Wider nodes smooth the east-west contrasts: monsoon-like land and liquid contrasts, rain behind
  mountains in longitude, the wind's east-west part. Checks: the globe test; on globe36 (3 seeds, years 21 to 30) the
  land's temperature and rain within their seeds' noise, zonal rain and land temperature by row within 1 K and 15%,
  ice within its noise; watch-for M0 (rain belts, deserts behind mountains, ice at the poles). The orographic rain stays
  (each cell takes its air at its own height, `step.rs`).
- **Threads.** 30 domains on globe36 (45 today) and 49 on size 28: the busiest domain's share on 4 threads (R2: 0.189
  on globe36) may rise. The globe already runs only x1.75 faster on 4 threads than on 1 and its body batch is 47% of
  its year, so a worse balance can cost more wall time than the air saves. Line: at most 0.3, and the 4T year not
  slower; PR7 measures both first.
- **Rounds.** Fewer, larger nodes may need more or fewer Newton rounds; PR7 counts them. If the rounds rise, the gain
  may vanish on globe36.
- **The long-step test** runs on the island (unchanged by E1).
- **The gain on globe36 is small** (about 1 to 3% of its year); E1 is for large globes. If it fails there, nothing is
  lost but the probe.

## 4. What PR7 must show first

On a branch: `Square::mesh` with a block shape, `AIR_SCALE` set per run by an environment variable (probe only).
1. On globe36 and on a size-28 globe (made at the start's code, 50 years, seed 1), one year, one thread, with
   `#[inline(never)]` on `newton`'s parts: the air's instructions by part (`pass`, the gather and booking, the
   Newton step, the band solve), rounds a step, the busiest domain's share; at 4 x 4 and at 6 x 4 (globe36) or 8 x 4
   (size 28).
2. The globe test (64 x 32) at 8 x 4; the 4T year of globe36 at 4 x 4 and 6 x 4 (5 interleaved runs, under the lock).
3. Globe36 with life at 4 x 4 and at 6 x 4, seeds 1 to 3, 30 years from worldgen: land temperature, rain on land, ice share, and
   zonal rain and land temperature by row over years 21 to 30.

**Gate for E1:** the climate checks of section 3 hold, the busiest share at most 0.3, the 4T year of globe36 not slower,
and the air's instructions -8% on globe36 or -15% on size 28. Else idea 7 is dropped and the owner told (the air on
globes keeps 4 x 4); lane G is the globe's other background cost.

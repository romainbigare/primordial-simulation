# Foundation, Phases E to L: the final check

Watch report for the owner (handover 12.4; `docs/foundation/plan.md`, Phase L; it covers Phases E to K, decision 41).
Pictures and run files are in `docs/reports/foundation-l/` (what is kept: `docs/reports/README.md`).

## In short

- **The foundation is built.** Bodies book through the store, plants are population nodes without a brain, young
  grow up inside their kind, the air step is implicit and converged at 10-day steps, the globe and refined regions run,
  liquid follows the spill levels of the ground, and the books are exact over 1,000 years.
- **The books hold.** Over 1,000 years on an island, a globe and a refined region, the worst element drift is 4e-16 and
  the worst energy drift 9e-14 (the limit is 1e-12). No law takes from a node more than it holds ("heat not given" 0 every year).
- **The world is a world.** The climate is steady for 1,000 years (land 290 to 294 K, no runaway ice, the sea keeps
  its volume), the absorbing gas stays between 290 and 440 ppm, plants and eaters live for 1,000 years.
- **The time budget is met only on a small planet.** 1,000 years fit 15 minutes at about 290 cells (a 24 x 12 globe in
  9 minutes, a 17 x 17 island in 12 to 18), not at 480 or more. Bodies are 60 to 80% of a year and run on one thread; their number, not the
  cost of one update, sets the time.
- **Open, and your choice:** the plants still keep too much stored sugar, a globe with life is too wet on land, and
  cells crowd with kinds that cannot merge. The final review's proposals are in the plan, section 10.3. The decisions
  taken for you are in `docs/foundation/decisions.md` (72 rows): please read them.

## The runs (seed 1, the final code)

| Run | Cells | 1,000 years took | Per year at year 1,000 | Bodies at year 1,000 (most in a cell) |
|---|---|---|---|---|
| Island 17 x 17 | 289 | 18.2 min (beside another run) | 1.6 s | 2,251 (46) |
| Globe 24 x 12 | 288 | 9.0 min | 0.8 s | 1,334 (27) |
| Island 22 x 22 | 484 | 25.1 min | 1.7 s | 2,351 (35) |
| Globe 36 x 18 | 648 | 36.6 min | 2.1 s | 1,559 (31) |
| Island 24 x 24, refined 8 x 8 region | 768 | 86 min | 9.6 s | 11,536 (73) |

The 17 x 17 island and the refined region ran on the final code; the others ran before the last three fixes (wear
once a survey, decisions 70 to 72), which change their bodies little. On the refined region the bodies still grow
slowly by members (2,891 at year 100, 11,536 at year 1,000), and the cost of a year with them. Each run used one core (CPU time = wall time): only the air's radiation and Change run on threads. The machine was
not loaded by other work.

Climate and life over 1,000 years:

| Run | Land K (year 100 → 1,000) | Ice | Absorbing gas, ppm | Light-eaters, kg | Eaters, kg | Land rain, m a year |
|---|---|---|---|---|---|---|
| Island 22 x 22 | 293.5 → 293.2 | none | 436 → 410 | 2.4e9 → 2.5e9 | 4.5e6 → 6.1e5 | 1.43 → 1.46 |
| Globe 36 x 18 | 290.6 → 290.5 | 1.2 → 1.8% of the area | 367 → 291 | 7.2e14 → 1.8e15 | 3.7e12 → 2.3e12 | 3.2 → 3.3 |
| Refined region | 293.5 → 293.8 | none | 442 → 417 | 2.5e9 → 5.0e9 | 1.1e6 → 3.5e7 | about 1.5 |

Earth for comparison: land about 288 K on average, rain on land 0.76 m a year, the sea about 1.1.

Pictures, each a row of five maps (the world, temperature, water, light-eaters, eaters):

- `island22-year0010.png`, `island22-year0100.png`, `island22-year1000.png`
- `globe18-year0010.png`, `globe18-year0100.png`, `globe18-year1000.png`
- `region24-year0010.png`, `region24-year0100.png`, `region24-year1000.png`
- `viewer-globe18-year0500.png`, `viewer-globe18-charts-year1000.png`, `viewer-region24-plants-year1000.png`: the
  viewer (from the first Phase L runs, before the last fixes).

## The reference runs (L1)

A2's runs on the new engine (64 x 64, seeds 1 to 3, 60 years; 128 x 128, seeds 1, 5, 7, 100 years, decision 62).
The key numbers held. Against A2: the swing is 3 K bigger (the converged air step and clouds that absorb heat
radiation), rivers are fewer, the absorbing gas is higher (plants pay their upkeep), and eaters live 3.7 times longer
(costs and intake brought to the body's temperature). Since L1 the laws changed again (carbon, stores, snow, the
selection gain), and the climate references were set again (decision 67): land 291.8 K on the 64 x 64 island.

## What the final check found and fixed

The long runs found what the short tests could not. Each was fixed in the law, not hidden:

| Found | Cause | Fix |
|---|---|---|
| Energy books broke on a globe (year 868) and at 128 x 128 (year 94) | Melting heat taken twice; heat of a falling liquid booked to the node it left; liquid leaving at the wrong temperature; evaporating rain the air had cut | Four fixes in the liquid and air steps; no overdraw since |
| Nitrogen and carbon slowly appeared in the books | Tiny amounts added to huge ones rounded away | A rounding carry for air and ground amounts |
| Ice piled on mountains; a globe froze to 250 K | Ice held melt like soil and refroze; a thick cover never met summer air | Snow and ice hold no liquid and melt at the top; rain below freezing falls as snow |
| The air's carbon fell to 43 ppm | A fixing gas nothing removed, and plants hoarding sugar that selection rewarded | Fixing gives its carbon back; a full store stops filling (decision 63); selection counts only what a store can build in one interval (decision 69) |
| Bodies grew to 236 in a cell, and to 33,000 on the refined region | Plants' unused brain-size genes drifted and each drift began a lineage that could never merge; every move wounded every body in its cell | Bodies without a brain keep those genes still (decision 72), a mixed child joins a lineage it fits (71), wear is shared once a survey (70), crowded cells merge across rooms (56): lineages at year 780 412 → 4 |
| Animals' and plants' threads made runs slower at planet size | A day holds too little work to share | Built and taken out again (decision 59); one update made 19% cheaper instead |

## Open

- **Stored sugar:** plants keep about 55% of their dry mass as sugar (real plants about 10%); the key-number test
  fails on its 50% bound on purpose.
- **Water on a globe with life:** land gets about 4 times Earth's rain; the air carries the sea's vapour onto land.
- **Crowded cells:** kinds of one lineage that differ in their genes do not merge; on long runs this sets the time.
- **Rivers on coarse and fine cells** still differ (G5); nodes and individuals still differ on rare causes of death
  (F7).
- **Missing laws** (M4): ice that flows, a slow carbon source, nitrogen that returns to the air, warm blood.

## What you decide

1. The decisions taken for you (`docs/foundation/decisions.md`), then M4 (plan 10.2).
2. The final review's proposals (plan 10.3). The one with the most value: one node per kind per cell, with its young
   inside it (about 80 lines fewer, about half the bodies), which would let a larger planet fit the time budget.

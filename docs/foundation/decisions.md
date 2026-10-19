# Decisions taken for the owner

The owner asked the architect to finish the foundation plan on its own: where a decision is needed, the architect
makes a suggestion and takes it as the owner's decision, and lists it here for the owner to review at the end. Each
line: what was decided, why, and what to look at if the owner disagrees. When the owner has reviewed this file, its
kept decisions move to the plan's Section 3 and the file goes.

| # | Phase | Decision | Why | If you disagree |
|---|---|---|---|---|
| 1 | E | E4 (the pace of nodes) merged with a known plant-life step error, before the reproduction work | The reproduction rework rewrote the booking where the error sat, and it was waiting for E4 | Fixed since (E11: the pace runs are flat) |
| 2 | E | Selection inside a node (E12) uses one sample member per update, not two; the node's own update is the other side | Two samples cost twice; the side is drawn, so the estimate stays unbiased | `copy/select.rs` |
| 3 | E | E12 skips the sample for a gene the update never reads | It changes nothing in the world (the same to four digits) and saves time | `copy/select.rs` |
| 4 | E | The sample of selection inside a node runs on one update in 2 (`sample_every` 2, a speed setting): the samples cost 12% of the bodies' time instead of 22% | Speed first; each gene still moves at the same mean rate | At ×2 and ÷2, plant members and life move 2 to 5%, about twice the noise in one seed of two: set `sample_every` to 1 (`params.rs`) and the world is the one before, bit for bit |
| 5 | J | The air's one implicit step (J1) lands although Flow is about 1.4 to 1.8 times slower than the old 3-day air | The old air was not converged (rain 3 to 4 times off at short steps); the new one is converged at 12 days against 6-hour steps and gives real land rain (0.88 m a year). Speed comes back by threads for the cell pass, cheaper booking and a second-order step | Keep the old air: revert J1's commit (the A2 references come back) |
| 6 | J | The air steps 12 days, not 15 to 30 | Backward Euler is first order: at 15 days the warmest month is off by more than the noise. 15 to 30 days needs a second-order step (BDF2), left as a later task | `air_hours` in `params.rs` |
| 7 | J | `rain_time` (1 day, tuned) replaces the old explicit limits of the air: with the hour or two real cloud water lives, rain falls on a tenth of the land | The tuned count stays the same (`mix` became anchored) | `docs/numbers.md`, row `rain_time` |

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

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

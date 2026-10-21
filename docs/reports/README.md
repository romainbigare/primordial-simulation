# Reports

This folder holds the watch reports: one short report for the owner at the end of each
milestone or check point, in simple English, with its pictures (handover 12.4). Pictures go in
a folder with the same name as the report.

- `foundation-a.md`: the foundation's Phase A (17 October 2026): where the time goes, the reference runs
  (`foundation-a/`), the key-number test, two bugs checked and one found by the long runs.
- `foundation-bc.md`: the foundation's Phases B and C: the new base (space, store, clock, the shared solvers), the
  owner's bite decision, sizes, and what Phase D must do.
- `foundation-d.md`: the foundation's Phase D: the physics laws on the new base; the same world, but the speed and
  size goals of the phase are not met (the owner decides before Phase E).

- `foundation-l/`: the runs of Phase L (`docs/history.md`, L1 to L4), with no report text of their own. It keeps the
  11 pictures, and per run only the numbers the history cites: the full `series.csv.gz` for the runs of 60 and 100
  years, `decades.csv.gz` (the rows of every tenth year, all columns) for the 1,000-year runs (`globe18`, `globe8`,
  `island22`, `region24`), and `run.log.gz` (its drift lines) where the run has one. Read them with `zcat`. Snapshots,
  `months.csv` and the viewer's files are not kept. The folder is 2.4 MB (it was 14 MB).

Folder sizes today: `foundation-a/` 11 MB (series and pictures of the A2 runs; it is not trimmed yet),
`foundation-bc/` 84 KB, `foundation-d/` 320 KB.

Old reports (the reviews and watch reports of M0 to M3, the studies and their patches): see
`docs/history.md`, section 4. Git keeps them in commit 154554a.

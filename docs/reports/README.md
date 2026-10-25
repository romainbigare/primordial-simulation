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
- `foundation-l.md`: the foundation's Phases E to L and its final check: the books over 1,000 years, the time budget,
  what the long runs found, what is open and what the owner decides.
- `foundation-l/`: the pictures and runs of that report. It keeps the
  maps of the long runs (`globe18`, `island22`, `region24` at years 10, 100 and the last year) and the viewer's 3
  pictures, and per run only the numbers the history cites: the full `series.csv.gz` for the runs of 60 and 100
  years, `decades.csv.gz` (the rows of every tenth year, all columns) for the long runs (`globe18`, `island22` and
  `region24`, seed 1, final foundation code; `globe8`, older), and `run.log.gz` (the whole log) where the run has one.
  `region24` runs to year 1,000 after decision 72. Read them with `zcat`. Snapshots, `months.csv` and the viewer's
  files are not kept. The viewer's pictures show the first Phase L runs (before the final foundation code). The
  folder is 2.6 MB.

- `speed.md`: the speed work after the foundation and its final check: where the speed stands against the baseline and
  the aim of 10 times, the 1,000-year runs, the largest planet that fits 15 minutes, what each phase gave, what was not
  kept, why 10 times was not reached, and what you decide.
- `speed/`: its pictures (SVG, made from the runs' `series.csv`): a year then and now, where a year goes, minutes for
  1,000 years, the cost of a year and the bodies over 1,000 years, the land temperature. 70 KB. The runs themselves
  are not kept; their numbers are in `docs/speed-post-foundation/measures.md`.

Folder sizes today: `foundation-a/` 11 MB (series and pictures of the A2 runs; it is not trimmed yet),
`foundation-bc/` 84 KB, `foundation-d/` 320 KB.

Old reports (the reviews and watch reports of M0 to M3, the studies and their patches): see
`docs/history.md`, section 4. Git keeps them in commit 154554a.

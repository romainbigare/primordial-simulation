# Speed x10: measures

The measures of this round. The plan is `plan.md`; how a step is judged is in its section 2 and X24, X33.

### PR-B: brain loss in 1,000 years

A counter on the branch `probe/brain-loss` (`efc5c74`) printed a line each time `lineage::resize` moved a body that had
a brain to a lineage with no hidden unit. `ew run --seed 1 --years 1000`, default settings, 4 threads, one run at a
time; the hashes are those of the last round's Z3, so the counter changes nothing.

| Run | Brain losses in 1,000 years | Hash |
|---|---|---|
| island 22 x 22 | 0 | `ff6d5114eac40e83` |
| globe 36 x 18 | 0 | `85b93203a4b1b704` |

The mean hidden units stayed between 1.5 and 4.6. Brain loss is possible by the law (X15) but did not happen in these
worlds; P1.2's switch is tested by a test kit case, not by a run.

### AIR4-R1: why 30-day air steps warm the land and the winters

On the long-step test's world (6 seeds against 6-hour air steps; branch `probe/air4-r1`, `a74a2f5`, switches off by
default, the world bit for bit the same), the 30-day Euler air step warms the land +0.71 K, summer +0.42, winter
+1.17. The parts taken at the step's start each move the land under 0.1 K (radiation, clouds, the air column, gas
heights, the wind, the rain or snow share), and they cancel. Most of the warming is the ground's melting plateau inside
a long step (`flow/air/radiation.rs`, `Surface::balance`): when a node melts or freezes all its plateau in one step, the
surface solve restarts on the wrong side of the fusion enthalpy (a sign error), and the plateau and the snow cover's cap
hold for the whole step instead of until the snow or ice is gone. Fixing both gives +0.29 / -0.10 / +0.86 K at 30
days, 0.118 K instead of 0.145 K at 10 days, and no change at 6 hours. What is left is backward Euler's first-order
phase error (winter +0.47 K at 15 days, +0.86 K at 30), positive in rising months and negative in falling ones; calm
months picked by hand only move it to another season. A Richardson estimate puts a second-order step near the 0.3 K
limit (+0.19 / +0.26 / +0.08 K). Expected gain of TR-BDF2 in calm months: Flow x1.2 to x1.5 for 6 to 12 calm months.

| Part (switch, paired over 6 seeds) | land (of +0.705) | summer (of +0.417) | winter (of +1.168) |
|---|---|---|---|
| Plateau: sign and split, both fixed | -0.41 ±0.16 | -0.51 ±0.19 | -0.31 ±0.12 |
| End reflectance (mean of start and end) | -0.13 ±0.02 | -0.12 | -0.13 |
| Sun over the step instead of at its end | -0.09 ±0.04 | -0.04 | -0.40 ±0.03 (moves the error) |
| Each other explicit part | under 0.05 | under 0.08 | under 0.08 |
| Left after the plateau fix (Euler's error) | +0.29 | -0.10 | +0.86 |

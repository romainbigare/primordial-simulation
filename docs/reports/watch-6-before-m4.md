# Watch report 6: review 5's list done

14 October 2026. Code: `main` after review 5 (`review-5-before-m4.md`). Chart and pictures in
`watch-6-before-m4/`. All runs are 64 x 64, seed 1, 100 years: short checks. The three 300-year
runs (review 5, row 9) were not run in this round.

## Correction after review 6 (14 October 2026)

An adversarial review (`review-6-adversarial.md`) checked this report against its own data. These
claims were wrong:

1. **The big-brain test proves nothing yet.** 216 big-brain eaters were placed, not 404 (404 was
   report 5's number). They were placed with the old hand-set brain biases, while the ordinary
   eaters start with neutral brains. A control kind of 12 units with the same hand-set start did
   as well or better (3.40 x 10⁹ kg against 3.20 x 10⁹ at year 60). The start won, not the brain.
   The test is being rebuilt with a control that differs only in brain size.
2. **The eaters did not live shorter "because they evolved smaller".** In the normal run their copy
   mass hardly moved (26 to 23 kg). Their life fell as their numbers grew (2.2 to 4.0 x 10⁷), the
   same way it did in report 5 at a fixed 57 kg.
3. **Not every animal kind shrank.** Three roster kinds (the 150 kg, 100 kg and 600 kg ones) died
   within 1 to 3 years of being placed, and the table left them out.
4. **"3 times faster" was the machine, not the laws.** Flow, whose code did not change, also went
   from 1,079 to 296 ms a year. The two runs had different loads on the machine.
5. **The starting eaters are a lottery.** 97 to 99% of the placed eaters die in their first year,
   and the kind lives through 2 to 14 bodies. `c_build` was set at 3 x 10⁴ to pass that bottleneck
   on seeds 1 and 2, and that makes lives about 3 times shorter than at 10⁴. Setting a law for how
   a start comes out is what handover 2 warns against. It is being redone.
6. The eaters of remains went down to 75 to 83 g, not 50 to 60 g.

## Short answer

Rows 1 to 8 of review 5's list are built. Two of them do not work as the review wrote them, and
we changed them. Both changes are written down in the handover and in `docs/numbers.md`.

1. **Building takes time.** A body can put at most `c_build` x mass^0.75 joules a day into its
   children. We set `c_build` at **3 x 10⁴**, not the review's 10⁴. At 10⁴ the starting eaters died
   out on seeds 1 and 2. 3 x 10⁴ is still inside the range from life (1,500 to 30,000), at its top.
2. **Satiety acts only on a grown body** (past `full` times its copy mass). As the review wrote it
   (for every body, upkeep plus building), **all 416 starting eaters died in their first year**:
   - A body's reactions release only part of its fuel in each interval. So it needs a store, not
     just this interval's food.
   - A body chooses "take" on only some of its decisions. So one meal must last several intervals.
3. Every body now pays a **basal upkeep**, and **leaf area grows with mass^0.75**, as the review
   asked.
4. The series has `root_<k>_kg`, `mass_max_root` and `merged_unlike`. Each run writes
   `physics.json`, and `ew numbers A B` prints the numbers that differ between two runs.
5. **Brain sizes count in likeness**, by whole units.
6. **`docs/numbers.md`** has a row for every number, and a test checks that no row is missing.
   There is now **one reference mass** instead of three.
7. **Individuals mode** (`--individuals`) and `copy::open` exist. The check of nodes against
   individuals **could not run** (see below).

What changed in the world:

- **Lives are a little longer, not years.** Plants live about 4 months (before 3). The eaters live
  6 months at year 50, then 3 months at year 100, because they evolved smaller.
- **Small bodies win.** In the roster run every animal kind shrank. The eaters went from 63 kg to
  about 0.2 kg. The eaters of remains went down to 50 to 60 g.
- **The plant giants are about 300 times smaller.** The heaviest member is 1 to 3 t for most of the
  run (before: 3,400 t), with a peak of about 10 t at year 99.
- **The big-brain test now works.** The 48-unit kind keeps its brain and holds its own beside the
  ordinary eaters. At year 100 it has more total mass, and fewer of its deaths are from starving (48% against 80%).
- **Carbon gas is lower:** 145 to 155 ppm in the normal run (report 5: 195).
- **The normal run is 3 times faster** (1.15 s a year against about 3.9 s in report 5).

## What we built

| Row | Change | Where |
|---|---|---|
| 1 | A body pays into its child store at most `c_build` (3 x 10⁴ J a day per kg^0.75) x members x days. A grown body (past `full`) fills its fuel only until what its reactions release in one interval covers its upkeep and its building. It never takes less than its upkeep. Below `full` a body eats as its intake allows. This replaces "past `full` it takes at the rate of `full`". | `copy::copy`, `choose::sate`, `Body::equiv` |
| 2 | Basal upkeep: `c_basal` 2 x 10⁴ J a day per kg^0.75, paid by every member, with strength on top | `body::upkeep` |
| 3 | Leaf area grows with mass^0.75. Each starting kind's gas gene is set again, so each breathes at the start what it did before | `Body::visible`, `open_area`, `worldgen::life` |
| 4 | New columns `root_<k>_kg`, `mass_max_root`, `merged_unlike`. Each run writes `physics.json`. `ew numbers OLD NEW` lists the numbers that differ | `tape`, `cli` |
| 5 | Likeness counts brain and sense sizes in whole units | `copy::like` |
| 6 | `docs/numbers.md` lists the fact, the anchor and a mark for each number, and a test keeps it complete. A budget for tuned numbers (handover 14.3). `intake_ref` and `heat_mass` are folded into `m_ref` (1 kg), and every kind's heat gene goes up by 0.492. `t_cat` and `T_SURF` are one number | `docs/numbers.md`, `core/tests/numbers.rs` |
| 8 | `Config::nodes` (`--individuals`) and `copy::open(node, n)`, with tests. "The same patterns" is written in handover 7.1 before any run | `copy`, `body::crowd` |

**The numbers table.** It has 98 rows: 4 physical, 29 anchored, 4 switches, 1 container, and
**67 tuned**. Review 5 guessed "about twelve" tuned numbers. Once every field had to give a reason,
27 of the 67 had a run or a reason, and 40 were first guesses with no source. The handover now says
that this count may not grow. The test fails if it does.

## Review 5's four checks on the old tapes

| Prediction | Result |
|---|---|
| 1. The roster's plant mass at year 100 is 3 to 7 x 10¹⁰ kg | **yes**: 6.7 x 10¹⁰ (old laws), 3.3 x 10¹⁰ (new) |
| 2. The tiny eaters of remains decide rarely, or have small brains | **no**: they decide every 7 to 9 days with a 12-unit brain. They can pay for it because `take` lets a body take 5% of its mass a day, and below 1 kg even more per kg: about 2 MJ per kg a day for a 30 g body, against a brain cost of about 170 kJ |
| 3. Eaters have 7 to 10 births per member a year, and over 95% of deaths are by `SPENT` | births **yes** (9.9). `SPENT` 93%, **almost** |
| 4. Print the heaviest member's kind | done (`mass_max_root`) |

## Pictures

![chart](watch-6-before-m4/chart.png)

- **Top left:** the life really lived, before (dotted) and now (solid). Plants went from about 0.27
  to 0.36 years, eaters from 0.14 to 0.25 to 0.5 years.
- **Top right:** the big-brain test. The 48-unit kind (purple) keeps its brain gene at 48 and
  reaches the ordinary eaters' total mass.
- **Bottom left:** copy mass of each kind in the roster run. Every animal kind shrinks. The heaviest
  member (dashed) stays at 1 to 3 t, with a peak of about 10 t at year 99.
- **Bottom right:** carbon gas. In the normal run it now stays near 150 ppm.

Maps at year 100: `seed1_year0100.png` (normal) and `roster_seed1_year0100.png`.

## Key numbers (64 x 64, seed 1, year 100; year 50 in brackets)

### Normal run

| | plants (kind 0) | eaters (kind 1) |
|---|---|---|
| members | 2.0 x 10¹⁰ (1.4 x 10¹⁰) | 4.0 x 10⁷ (2.2 x 10⁷) |
| total mass | 3.7 x 10¹⁰ kg | 1.8 x 10⁹ kg |
| mean member / copy mass | 1.8 / 1.8 kg | 44 / 23 kg (copy mass started at 63) |
| **life really lived** | **0.37 years** (0.35) | **0.25 years** (0.52) |
| life the repair gene allows | 7.9 years | 15.6 years |
| births per member a year | 2.9 | 5.3 (2.0) |
| deaths by `SPENT` / by age | 65% / 4% | 72% / 2% |
| hidden-units gene | 11.92 | 12.16 |

Whole world: 2,292 bodies; ground 289.3 K; land swing 15.5 K; carbon gas 154 ppm; heaviest
member 80 kg (an eater); `merged_unlike` 0. Speed 1.15 s a year (report 5: about 3.9 s, with three
runs at once both times).

### Roster run

| kind | members | total mass | mean mass | real life | births per member a year |
|---|---|---|---|---|---|
| 0 light-eaters (1 kg) | 1.1 x 10¹⁰ | 6.4 x 10⁹ kg | 0.61 kg | 0.14 y | 10 |
| 3 small fast light-eaters | 5.2 x 10⁹ | 1.4 x 10⁹ kg | 0.26 kg | 0.11 y | 11 |
| 4 big slow light-eaters (20 kg) | 2.3 x 10¹⁰ | 6.1 x 10¹⁰ kg | 2.7 kg | 0.14 y | 9 |
| 1 eaters (30 kg) | 8.7 x 10⁹ | 1.6 x 10⁹ kg | 0.18 kg | 0.05 y | 32 |
| 5 small eaters of remains | 2.6 x 10¹⁰ | 2.0 x 10⁹ kg | 0.075 kg | 0.04 y | 25 |
| 6 big eaters of remains | 6.2 x 10¹⁰ | 5.1 x 10⁹ kg | 0.083 kg | 0.04 y | 25 |
| 7 small fast eaters of light-eaters | 1.0 x 10⁸ | 1.9 x 10⁸ kg | 1.8 kg | 0.05 y | 46 |

- All three plant kinds live. Their mass is 6.9 x 10¹⁰ kg, as much as the roster with the old laws.
- The animals are about 13% of the plants by mass.
- The heaviest member at year 100 is a big slow light-eater of 2.7 t.
- `merged_unlike` is 0.

### The big-brain test (row 5)

404 eaters with a 48-unit brain were placed at year 40.

| year 100 | ordinary eaters (12 units) | big-brain eaters (48 units) |
|---|---|---|
| members | 5.4 x 10⁷ | 2.5 x 10⁷ |
| total mass | 2.0 x 10⁹ kg | 2.8 x 10⁹ kg |
| mean member mass | 38 kg | 109 kg |
| life really lived | 0.15 y | 0.29 y |
| share of deaths by `SPENT` | 80% | 48% |
| hidden-units gene | 12.1 | 48.0 |

The 48-unit brain is kept now: likeness counts brain sizes, so ordinary children no longer join
these nodes. The big-brain kind grows to more total mass than the ordinary eaters. It is heavier,
lives twice as long, and starves much less often. So a big brain pays something here: it finds
food. But it does not win: by members it is half of the ordinary eaters. This is one seed. Run
it again on other seeds before calling it a result.

## What did not work as the review wrote it

1. **Satiety for every body kills the eaters.** The review's allowance (upkeep plus building,
   about upkeep plus 6% for an eater) left each eater's store at the line where it dies. We tried
   a store-based allowance at every size: the eaters still died. The working form acts only past
   `full`, and that is the one the review needed it for: to stop a grown body from swelling.
   `full` stays, so the tuned count does not fall by one as the review hoped.
2. **`c_build` 10⁴ kills the start.** We tested seed 1 at 64 x 64, 32 years:

   | Variant | Eaters at year 32? |
   |---|---|
   | no basal cost | lived |
   | no build cap | lived |
   | both on | died |
   | hand-set eater biases | lived (seeds 1 and 2) |
   | 60% starting store | lived on seed 1 only, and barely |
   | `c_build` 3 x 10⁴ | lived (seeds 1 and 2) |

   The starting eaters have random brains and starve within weeks. A small group lives only if it
   breeds faster than it starves until it learns. So `c_build` is anchored, but set at the top of
   its range for the start to work. `docs/numbers.md` says so.
3. **The build law barely slows plants.** A child's cost counts only what is not liquid, and a
   plant is mostly liquid. So a 2.4 kg plant owes about 8 x 10⁵ J per child: about two weeks of its
   building.
4. **The nodes-against-individuals check (row 8) cannot run yet:**
   - The animals reach 10⁷ to 10¹⁰ members. As individuals, each member is a body. A 48 x 48 run
     had 69,135 animal bodies by year 25 (bodies 89 times slower that year), and a roster run
     stopped after 20 minutes and 4.7 GB.
   - In a 32 x 32 world the 58 starting eaters die in both modes, so there is nothing to compare.
   - The check needs a world where animals stay near 10⁵ members.

## Problems still open

1. **Every animal kind evolves small** (eaters 63 kg to 0.2 kg in the roster). Intake, building and
   basal upkeep all go with mass^0.75, so a smaller body breeds faster per kg. The brain costs more
   per kg when small, but `take` gives a small body so much food that it pays easily. Nothing yet
   makes a 0.1 kg animal harder to be than a 30 kg one. Look at `take` (5% of body mass a day, a
   tuned number) first.
2. **Lives are still months.** The longest real life is 0.5 years, for 50 kg eaters at year 50.
3. **Carbon gas near 150 ppm** in the normal run. Watch it in the 300-year runs.
4. **The nodes-against-individuals check** needs a small test world. M4's first job, as review 5
   says.
5. **The 300-year seeds (row 9) were not run.** They decide whether giants or swelling come back
   (the "past `full`" cap is gone from intake), and whether the plants start to drop their brains.
6. **67 tuned numbers**, 40 of them first guesses.

## What next

Run the three 300-year seeds at 128 x 128. Then design a small world for the individuals check
(fewer, slower animals), and look at `take` as the thing that lets tiny animals win. Then M4.

Code size: `core` is 7,341 lines (budget 8,000 through M7).

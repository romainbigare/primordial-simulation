# Watch analysis: 128x128 island, `--eater-bias`, commit 7de55c0 (after the NaN fix and the light-lump fix)

Data: `watch/seed1`, `seed5`, `seed7`, all complete (300 years each, no crash). Source: `series.csv` (one row per year, state at the start of the year; the flow columns such as births, deaths, ate_*, crossings, take_*, gave, watched describe the year before), `months.csv`, the logs `watch/seedN.log`, and the year*.png pictures (every 10 years). Earlier runs for comparison: "v1" = `watch_v1/` (commit a6bb180, before the two fixes; seed 7 crashed at year ~221, tape to 220); "old baseline" = `baseline/seed1, seed2, seed5` (before all fixes, 120 years).
Everything here was computed with python3 from the files; nothing in the repo was changed. matplotlib is not installed, so there are no charts. Scripts: `scratchpad/work2/*.py` (reused from `work/`).

Words used: a "body" is one node in the world. A plant or animal body can hold many "members" (real individuals); `plants`/`animals` count bodies, `plant_members`/`animal_members` count individuals. "Animals" means every body that is not a light-eater (`animal_nodes` are crowds, `animal_single` are single bodies). `cover_*` is light-eater mass per m2 over land cells (median and max). A map cell is 2 km by 2 km. "v2" below means these new runs.

Notes on the data. The speed (ms per year per law part) is only in the logs, once every 10 years (so no value for years 75, 150, 250). The three runs ran at the same time on a 4-core machine (all started about 01:47 and ended 02:44-03:12), so the times are slightly pessimistic; the v1 runs were also run in parallel. `worn_persist`/`traffic_persist` are NaN in the first years (they compare with a decade ago). `hidden`, `memory`, `seen`, `sight` are constant (see M2). The spatial facts (climate bands, rivers, land area, hot cells, plant share of land) are not in the series; I read them from the pictures with a script (pixel colours back to numbers), so treat them as approximate (about +-1-2 K, cell counts +-5%). "Worn share" = `worn_edges` divided by 4 x land cells (the tape has 4 forward edges per cell); coast edges are fewer, so the share is a little low. I checked my formulas by recomputing the v1 numbers (eat_animals, ate_others, rivers, hot cells) and they agree with the v1 report.

## 1. Short summary

What changed against v1 (same seeds 1, 5, 7, same island): both fixes did what they were meant to do, but the world they give is not the world v1 gave.

- **Fixes confirmed.** No crash: seed 7 now runs 300 years (v1: crashed at ~221). `heat not given` is exactly 0 J in all three seeds for all 300 years (v1 seed 1: 8.4e16 J at year 300). Element drift is at most 3e-16 and energy drift at most 3.4e-13 at year 300. Hot cells are gone: land cells at 298 K or more are 0-1% at year 50 (v1: 14-17%) and 6 / 14 / 11% at year 300 (v1: 24-25%). The single-cell `cover_max` spikes of 500-3,500 kg/m2 are gone (largest `cover_max` ever: 231 kg/m2).
- **A new, strong event: cover collapse in all three seeds.** After the eaters are placed (year 22) the median plant cover stays at or below 0.01 kg/m2 for 40 / 52 / 65 years (seeds 1 / 5 / 7; v1: 9 / 12 / 8 years, and only seed 5 had a real collapse). Plant members fall 43 / 76 / 88% (to 3.7e10, 2.2e10, 9.5e9 at years 58, 53, 75) and plants cover only 25-41% of land cells in years 40-70 (15% in seed 7 at year 70). Recovery starts around year 80-90 and the median passes 1 kg/m2 at year 109 / 109 / 154. By year 300 the median is 3.4 / 3.3 / 3.1 kg/m2 and plants are on 71-82% of land.
- **All three seeds kept eaters to the end** (hand-set start in year 22; neutral eaters died in 8 of 8 test runs, not rerun here). `kinds` is 2 in every seed from year 23 (what we placed); in seed 5 a third cluster appears from year 207 and is there in 79 of the last 100 years (seed 7: flickers in 7 of 100 years). Animal bodies: 16.8k / 13.0k / 12.4k at year 300.
- **Eaters of eaters: appeared, in all three seeds (v1: only two).** Bodies that take more from animals than from plants: mean 2,191 / 863 / 1,856 from year 50 (14% / 8% / 15% of animal bodies); animals killed by being taken are 26% / 10% / 24% of animal deaths (mean year 50+; 35% in seed 1 at year 300). v1 seed 1: 4 bodies and 0.0% of deaths.
- **Climate: the greenhouse gas (`absorber`) ends higher than in v1** although it starts lower. Year 100: 4.03 / 3.67 / 3.46 (v1: 6.40 / 4.48 / 5.63). Year 300: 16.56 / 11.14 / 10.76 (v1: 9.67 / 9.05). The rise after year 100 is carbon dioxide (flat at 0.3e-3 until year 106 / 115 / 154, then up 4.5 to 16 times by year 300) switching on when the median cover passes about 1 kg/m2, and methane (60-70% of the absorber, 3e-6 at year 10 to 2.9e-4 at year 290 in seed 1). The ground does not follow: `t_ground` is 294.7 / 295.6 / 295.1 K at year 300, +9.1 / +8.0 / +8.2 K from year 5, 82-89% of it by year 100. The sensitivity falls from 2.4-2.8 K per e-fold of absorber (years 5-300) to 0.4-1.1 K per e-fold (years 100-300). Seed 5's methane starts a fast fall in the last 5 years (see 3.2).
- **Rivers and lakes: yes, same as v1 and growing.** Standing water on land 253 / 83 / 79 km3 at year 300 (from 13.5 / 6.2 / 9.3 at year 10). Land cells with river-sized flow (web threshold): 2,001 / 1,437 / 2,079 (v1: 1,574 / 1,026).
- **Climate bands: present but fading** as in v1. West-edge sea strip, north to south: 26 K at year 0, 9 K at year 10, 5 K at year 50, 3-4 K from year 100. Ice: never above 0.15% of cells and below 0.03% from year 30.
- **Trails: a worn-edge network now covers nearly all land** (91 / 82 / 88% of land edges at year 300; v1: 76 / 60 / 55%), is stable (85 / 82 / 84% of a decade's worn edges are still worn ten years later; v1: 77 / 64 / 60%), and grows after year 100 (v1 was already high at year 50). Focused routes: the top 2% of cells carry 10 / 38 / 16% of crossings, and 17 / 7 / 28% of them are the same cells ten years later (chance would be 2%). Some persistence, but it is a net, not a few routes.
- **Seasons are strong in the data** (14-18 K swing where animals live) but there is no seasonal movement (centroid moves 0.1-0.5 cells within a year, 0.2-1 km).
- **Imitation is happening** (watch events a year at year 300: 20.9k / 6.9k / 10.9k; the widest spread reaches 1,246 groups, 39% of all groups, in seed 1) but nothing shows adoption. `learned` peaks at 0.12-0.13 (years 70-120) and then falls to 0.065-0.08 (v1: kept rising to 0.17). **Signals:** still weak, `mi_low` 0.07-0.16 bit (v1: 0.016-0.09), `mi_hurt` 0.02-0.04, `mi_cold` about 0.
- **Giving: yes; exchange: no.** In seed 1 animals started giving at a high rate from year ~215 (23-47k per year from year 233, 92% by animals) but reciprocal pairs are at most 166 a year (0.6% of pairs).
- **Tools, shelters, floating: a trace, not a pattern.** Parts exist only in years 23-115 (max 11 / 28 / 46) and then vanish when animals become crowds. Held objects raise the yield per take 11-36% but sit in 0.01-0.09% of takes. `inside_cold` is exactly 0 in every year of every seed; one structure for two years in each seed (years 66-77); at most 2 things afloat, in 6-7 years.
- **Speed:** 15.7 / 10.5 / 12.7 s per simulated year over 300 years (78.6 / 52.7 / 63.3 min). At year 300: 22.0 / 14.7 / 17.0 s per year. The M1-M3 target is 0.6 s per year, so the whole-run mean is 26 / 18 / 21 times too slow and year 300 is 37 / 24 / 28 times too slow. The bodies law is 93-95% of the time at the end.
- **Things to look at:** (1) why the cover collapse is now so deep and long in every seed; (2) why the ground stops warming while the gas keeps rising; (3) seed 5's methane fall in years 295-300 (v1 seed 7 had the same kind of fall before its crash, which has since been fixed); (4) `learned` falling after year 100; (5) the traffic picture is still saturated.

### Key numbers at year 300

| at year 300 | seed 1 | seed 5 | seed 7 | v1 seed 1 | v1 seed 5 |
|---|---|---|---|---|---|
| plant members | 4.81e+11 | 1.27e+11 | 2.39e+11 | 3.38e+11 | 9.79e+10 |
| animal members | 3.36e+10 | 3.27e+12 | 3.14e+10 | 1.74e+09 | 2.80e+12 |
| animal nodes | 16821 | 12824 | 12449 | 5672 | 13065 |
| animal singles | 0 | 199 | 0 | 0 | 0 |
| cover median (kg/m2) | 3.39 | 3.32 | 3.08 | 3.55 | 2.57 |
| kinds | 2 | 3 | 2 | 2 | 2 |
| lineages alive | 6 | 12 | 8 | 4 | 11 |
| absorber | 16.56 | 11.14 | 10.76 | 9.67 | 9.05 |
| t_ground (K) | 294.7 | 295.6 | 295.1 | 294.4 | 294.2 |
| learned | 0.067 | 0.065 | 0.081 | 0.169 | 0.177 |
| greedy | 0.70 | 0.84 | 0.69 | 0.78 | 0.90 |
| worn_edges (share of 4 x land cells) | 18830 (91%) | 12479 (82%) | 14058 (88%) | 15892 (76%) | 9251 (60%) |
| s/yr, whole run mean | 15.7 | 10.5 | 12.7 | 9.5 | 17.0 |
| s/yr at year 300 (log) | 22.0 | 14.7 | 17.0 | 13.2 | 22.7 |
| total run time (min) | 79 | 53 | 63 | 47 | 85 |

The v1 seed 7 run stopped at year 220 and is left out. `worn_edges` share as defined above.

### Verdict by watch-for item

| item | v2 verdict | key number |
|---|---|---|
| Rivers and lakes | appeared, still growing | 253 / 83 / 79 km3 on land; 2,001 / 1,437 / 2,079 river-sized land cells |
| Climate bands | appeared, fading | 26 K to 3-4 K north-south in 100 years |
| Light-eaters spread | appeared, slowly; then cover collapse | plants on 11-20% of land at year 10, 30-41% at year 50, 57-72% at year 100, 73-84% at year 200 |
| Eaters of eaters | appeared in 3 of 3 | 26 / 10 / 24% of animal deaths are "taken" |
| Kinds differ by climate | not shown | `kinds` 2, a third kind in seed 5 from year 207 |
| Populations rise, fall, cycle | rise and fall yes; no cycle | animal bodies 4.5k to 17.4k (seed 1); no regular period |
| Extinctions | lineages yes, kinds no | 2,617 / 3,001 / 4,854 lineages ever, 6 / 12 / 8 alive |
| Trails | worn net yes; focused routes weak | 82-91% of edges worn; top 2% cells carry 10-38% |
| Seasonal movement | no | centroid moves 0.1-0.5 cells |
| Imitation spreading | imitation yes, adoption not shown | 1,246 groups reached (39%) |
| Signals settling | weak | `mi_low` 0.07-0.16 bit |
| Giving / exchange | giving yes, exchange no | up to 47k gives a year; reciprocal at most 166 |
| Held objects as tools | trace | +11-36% yield in 0.01-0.09% of takes |
| Shelters in cold | no | `inside_cold` 0 always |
| Floating structures | trace | at most 2 afloat, 6-7 years |

## 2. The M0-M3 watch-for items, with numbers

### M0

**Rivers and lakes: appeared, as in v1.**
- `liquid_land` (standing liquid on land, km3), years 10/50/100/200/300: seed 1: 13.5 / 70 / 132 / 227 / 253; seed 5: 6.2 / 30 / 42 / 70 / 83; seed 7: 9.3 / 34 / 59 / 82 / 79. Seeds 5 and 7 level off after year 200 (+0.02 and -0.04 km3/yr over years 250-300); seed 1 is still rising (+0.14 km3/yr). v1 at year 300: 248 / 87 / (84 at 220), so the water on land is about the same as before.
- Logs (end of run): max land flux 9.8e5 / 9.5e5 / 8.4e5 mol/s; dry land cells over 9e3 mol/s (web threshold): 2,001 / 1,437 / 2,079, i.e. 39% / 38% / 52% of land cells; over 3e4 (png threshold): 561 / 250 / 616. v1: 1,574 / 1,026 (30%, 27%) and 485 / 203. Mean land rain 5.3 / 5.7 / 5.6e-4 mol/m2/s (v1: 4.5 / 4.3e-4). More rivers than v1 because there is more rain (mean rain 6.9e-4 at year 10 to 13.4e-4 at year 300 in seed 1, +94%).
- Pictures (map panel, inland blue not connected to a border sea): seed 1 goes from 280 cells at year 0 to 607 (y10), 932 (y50), 1,153 (y100) and 1,331 (y300); the largest connected inland water grows 144 to 217 cells (868 km2). Seed 5: 152 to 634, largest 94 to 352 cells. Seed 7: 86 to 459, largest 57 to 244. v1 at year 300: 1,297 / 613.
- Land (non-blue map area) shrinks 17% in seed 1 (6,258 to 5,163 cells), 14% in seed 5 and 17% in seed 7: low ground floods.
- Old baseline: 5-16 km3 at year 50 falling to 12 / 4 / 3 km3 at year 120. At year 100: 132 / 42 / 59 now against 15.5 / 5.8 / 3.4.
- Not measured: valleys carved over centuries (no height column).

**Climate bands: appeared, but weak and fading.**
- Temperature panel, ocean strip at the west edge, 8 bands north to south: year 0: 275 to 301 K (26 K); year 10: 285 to 294 K (9 K); year 50: 292 to 296 K (4-5 K); year 100: 294 to 297 K (3 K); year 300: 295 to 298 K (3 K). Seeds 5 and 7 the same (294 to 298 K at year 300). Same shape as v1 (293 to 297 K).
- Rain over the same strip: log10 of rain goes from 3.6 (north) to 4.7 (south) at year 10, and 4.56 to 4.86 at year 100 (a factor 2 between the north and south ends); at year 300 4.61 to 4.89. The sea has more rain than the land (the rain panel is cream over land, teal over sea).
- Ice: `ice_frac` is at most 0.12% / 0.03% / 0.15% of cells (the peaks are in years 2-82) and is below 0.03% from year 30. No ice age, no polar ice. In seed 7 about 113-118 map cells are white in every picture from year 0 to 300 while `ice_frac` is about 0, so that white cap is the colour of high ground, not ice. Not tested: weaker sun, more or less gas by hand (the runs did not change the sun).

### M1

**Light-eaters spread where conditions fit: appeared, slowly; then held down by the eaters.**
- Placed in year 2: 6,152 / 4,326 / 4,750 bodies (2.46e9 / 1.73e9 / 1.90e9 members). They merge to 1,063 / 1,196 / 1,305 bodies by year 10 and grow to 4,095 / 3,425 / 3,359 by year 30.
- Plants on land (cells with a green mark in the life panel, share of land cells), years 10 / 20 / 30 / 50 / 100 / 200 / 300: seed 1: 11 / 18 / 35 / 41 / 72 / 82 / 76%; seed 5: 20 / 30 / 39 / 30 / 63 / 84 / 82%; seed 7: 19 / 26 / 30 / 31 / 57 / 73 / 71%. So it spreads to 20-35% of land before the eaters, then stalls (30-41%) until year 70-80 (seed 7 falls to 15% at year 70) and then covers 71-84%.
- Median land cover is 0 until year 30 / 26 / 28 (more than half of land cells had no plant for 26-30 years; v1: 24-25). Note that this is before the eaters arrive in year 22: the first 25 years are bare in the median in v1 and v2 alike. It stays at or below 0.01 kg/m2 for 40 / 52 / 65 of the years 24-90 (v1: 9 / 12 / 8). It passes 0.1 at year 78 / 82 / 96 (v1: 39 / 36 / 41) and 1 kg/m2 at year 109 / 109 / 154 (v1: 54 / 114 / 148). Median at year 300: 3.39 / 3.32 / 3.08.
- Plant bodies: 3.3-7.1k (seed 1; 6,946 at year 300), 3.2-4.4k (seed 5), 1.8-5.4k (seed 7). Plant members at year 300: 4.8e11 / 1.3e11 / 2.4e11.
- Where cover is high the ground is warmer, but see 3.2: the hot dots are much fewer than in v1.

**Eaters of eaters: appeared, in all three seeds.**
- `eat_animals` (bodies that took more mass from animals than from plants), mean from year 50: 2,191 / 863 / 1,856 (14% / 8% / 15% of animal bodies); peak 3,837 (y107) / 1,856 (y44) / 3,264 (y115). v1: 3.9 / 698 / 381. They are already there in the first years (286 / 986 / 665 at year 30).
- `ate_others` (kg taken from animals) is 4.7% / 1.7% / 4.7% of everything animals took (mean year 50+; at year 300: 7.4% / 2.3% / 6.0%); v1: 0.0% / 2.0% / 0.7%. About half of an animal's intake is ground (`ate_ground` 55% / 53% / 58%), so animals do not live from plants alone.
- Animals killed by being taken (share of animal deaths): seed 1: 0.9% (y50), 20% (y100), 33% (y200), 36% (y300); seed 5: 9% / 9% / 11% / 15%; seed 7: 6% / 37% / 19% / 27%. v1: 12-14% in seed 5, 0% in seed 1.
- `trophic_high` (highest share any kind took from animals) is 0.63 / 1.0 / 1.0 at its peak; `trophic_max` (kind with most bodies) mean from year 100: 0.12 / 0.04 / 0.07.

**Kinds differ by climate (variety): not shown.**
- `kinds` is 2 in every seed from year 23 (light-eater and eater, the two things we placed). Seed 5: a third cluster from year 207; from year 207 to 300 `kinds` is 3 in 79 of the 100 years (it flips between 2 and 3 in years 207-230, then stays at 3 from year 230). Seed 7: 3 in 7 of the last 100 years (e.g. years 245, 248, 261-262, 281, 286, 288), flickering. Seed 1: never. In seed 5 the smallest member mass falls from 0.109 kg (year 200) to 0.0012 kg (year 300) and the median from 1.8 to 0.43 kg, so the third kind is probably a small-bodied animal; I did not look at the genes.
- The series has no per-climate numbers, so a link between kind and climate cannot be tested from it. In v1 `kinds` never reached 3 (seeds 1 and 5).
- Sizes: median member mass falls from about 26-47 kg (year 30-50) to 3.7 / 0.43 / 2.5 kg (year 300). Members per animal body at year 300: 2.0e6 / 2.5e8 / 2.5e6.
- Live brain lineages: peak 82 (y68) / 71 (y65) / 93 (y60), final 6 / 12 / 8; v1 final 4 / 11 / 32 (at y220).

**Populations rise, fall and cycle: rise and fall yes; no cycles beyond the season.**
- Animal bodies: seed 1: 4,452 (y30) -> 8,331 (y50) -> 17,350 (y100, peak 17,851 at y102) -> 16,173 (y200) -> 16,821 (y300). Seed 5: 5,942 -> 9,217 (y50) -> 8,746 (y75) -> 11,089 (y100) -> 13,023 (y300). Seed 7: 5,113 -> 9,097 -> 9,987 (y75) -> 12,229 (y100) -> 14,697 (peak y151) -> 12,449. Bodies grow 25-30% between years 75 and 100 as the cover comes back (seed 1 doubles from year 50 to year 100).
- Animal members: seed 1: 3.3e8 (y30) -> 5.1e8 (y75) -> 2.8e9 (y100) -> 3.4e10 (y300); seed 5: 2.3e8 -> 3.1e8 (y75) -> 4.3e9 (y100) -> 5.5e10 (y200) -> 3.3e12 (y300); seed 7: 2.1e8 -> 1.6e8 (y75) -> 1.3e9 (y100) -> 3.1e10 (y300). A steady rise of 2-4 orders of magnitude after year 75, with no break.
- Within the year (`months.csv` `mobile`): the yearly range is only 2-7% of the mean in all periods (seed 1: 5%, 4%, 5%, 7% for years 25-50, 50-100, 150-200, 250-300; seed 5: 5, 3, 2, 2%; seed 7: 2, 4, 5, 6%). v1 had 10-42%.
- Years-long cycles: peaks of log10(animal_members) (3-year smoothed, 41-year trend removed, swing at least 26%), not counting the year-30 start: seed 1: years 102, 177; seed 5: 79, 120, 163, 213, 261; seed 7: 98, 122, 156, 259. With a swing of at least 58%: 102 / 120 / 98, 156. The spacing in seed 5 (41, 43, 50, 48 years) looks regular, but the autocorrelation of the detrended series at lags 43-56 years is -0.1 to 0 for seed 5, so it is not a rhythm. Seed 1: 0.41-0.50 at lags 54-63, seed 7: 0.42 at lag 72; short series (230 years), so I would not call these cycles either. For animal bodies (not members) there is no peak above 26% in any seed. Late in the runs the animal body count is steady (years 150-300, scatter around the trend 3.7% / 2.2% / 2.5%).

**Extinctions: lineages yes, kinds no.**
- `lineages_ever` at year 300: 2,617 / 3,001 / 4,854, with 6 / 12 / 8 alive, so over 99.7% are gone (v1: 112 / 4,461 / 9,806). A lineage here is a brain lineage; about 2% of births start a new one.
- No kind died out: both placed kinds are present to year 300 in every seed.
- Animal deaths by cause (share of members, year 50 / 100 / 300; seed 1): upkeep not paid (SPENT) 73% / 74% / 53%; taken by another body 0.9% / 20% / 36%; bonds broken by moving 6% / 4% / 11%; low store (LACK) 19% / 1.4% / 0.1%; frozen or boiled (PHASE) 0.9% / 0.2% / 0.1%. Cold deaths are almost absent from the start (v1: 21% at year 50), because the world is warmer at the start and land is warmer (see 3.2).

### M2

**Trails: a worn network appeared and is nearly total; focused routes only weakly.**
- `worn_edges` (edges cleared by movers so that crossing costs under half of uncleared ground), years 10 / 30 / 50 / 100 / 150 / 200 / 300: seed 1: 1,212 / 5,875 / 5,106 / 6,876 / 14,762 / 17,544 / 18,830; seed 5: 1,600 / 3,585 / 1,837 / 4,288 / 7,699 / 8,102 / 12,479; seed 7: 2,210 / 3,329 / 2,195 / 3,274 / 6,985 / 13,184 / 14,058. As a share of 4 x land cells: seed 1: 5% (y10), 23% (y50), 32% (y100), 71% (y150), 91% (y300); seed 5: 11% (y50), 27% (y100), 49% (y150), 82% (y300); seed 7: 13% (y50), 20% (y100), 43% (y150), 88% (y300). v1 at year 300: 76 / 60 / 55%. So almost the whole land is "worn", which is not the same as a few trails. The worn net dropped during the cover collapse (years 50-75: seed 7 had 743 worn edges at year 75) and grew when the cover and animals came back.
- `worn_persist` (share of those worn a decade ago still worn): seed 1: 0.30 (y50), 0.39 (y100), 0.78 (y150), 0.85 (y300); seed 5: 0.13, 0.28, 0.57, 0.82; seed 7: 0.14, 0.29, 0.62, 0.84. Durable and rising (v1: 0.52 / 0.63 / 0.77 for seed 1, 0.16-0.64 for seed 5). Each year about 2,250-2,560 new worn edges still appear (`worn_new`), so the net is also reshuffling at the margin.
- `traffic_top2` (share of crossings on the busiest 2% of cells): seed 1: 0.35 (y10), 0.27 (y50), 0.26 (y100), 0.10 (y300); seed 5: 0.41, 0.56, 0.40, 0.38 (y300); seed 7: 0.36, 0.52, 0.42, 0.16. `traffic_persist` (the same top cells as a decade ago, decade values): seed 1: 0.13-0.24 (y100-150), 0.30-0.36 (y150-200), 0.13-0.27 later (0.17 at y300); seed 5: 0.07-0.22 (0.07 at y300); seed 7: 0.20-0.38 (0.28 at y300). If the top cells were random the overlap would be 0.02, so seeds 1 and 7 show some fixed busy places (6-19 times chance), but seed 5 at year 300 is close to moving around. Body mass per m2 in the busiest cells is 1.3 times other land (seed 1: 14.8 vs 11.4), 1.4 times (seed 7: 16.0 vs 11.8) and 2.2 times (seed 5: 32.1 vs 14.6). v1 had `traffic_top2` 0.12 / 0.51 / 0.36 at year 300 and `traffic_persist` 0.125 / 0.07-0.19 / 0.11-0.32. So focus is about the same as in v1, a little higher in persistence.
- The picture cannot show it: the traffic panel (the fifth) is saturated cream over the whole land from year 30 on in seed 1, because the colour scale tops at 1,000 crossings per cell per year and `crossings` counts members (1e12-2e13 per year). Seed 7 at years 60-80 (crossings 6e10-1e11 a year, the lowest) is the only picture where the traffic panel shows structure (orange-pink patches).

**Seasonal movement: did not appear.**
- Seasons are real: in `months.csv`, `t_mobile` (temperature at the animals) goes from 285-295 K (month 2) to 301-311 K (month 7 or 8): swings of 17.0 / 16.3 / 17.9 / 18.3 K in seed 1 (years 25-50, 50-100, 150-200, 250-300), 14.0-15.9 K in seeds 5 and 7. The animal count follows only weakly (2-7% range).
- Deaths by SPENT (upkeep not paid) in years 25-50 peak in month 1 (the coldest part) at 2.2-2.8 times the lowest month (month 6-7); from year 150 on they peak in months 9-11 (after the hot month) at 1.6-2.8 times the lowest, so they no longer follow the cold (v1: 1.8-3.8 times, coldest months).
- But the centroid (`mean_y`, mean row of the animals) moves only 0.1-0.5 cells within a year (seed 1 years 250-300: 59.3 to 59.8; 0.2-1 km); over the 50-year windows in years 150-300 the yearly mean moves 0.65-1.5 cells; `mean_dy` (net monthly movement) is at most 0.015 cells in absolute value. No migration.

**Learned behaviours spread by imitation: imitation happens and spreads; adoption is not shown.**
- `watched` (watch events per year): seed 1: 1,160 (y10), 5,270 (y50), 2,234 (y100), 9,917 (y200), 20,876 (y300). Seed 5: 3,758, 5,458, 2,523, 3,044, 6,850. Seed 7: 1,600, 2,231, 2,105, 16,327, 10,866. `watched_groups` is 3,214 of 3,445 groups in seed 1 (93%), 1,286 of 2,009 (64%) in seed 5 and 2,375 of 3,371 (70%) in seed 7 at year 300.
- `spread_max` (the most groups that watched one group): seed 1: 6 (y10), 82 (y50), 215 (y100), 554 (y200), peak 1,246 (y259; 39% of groups), 733 (y300); seed 5: 17, 47, 191, 123, 238 (y300; peak 309 at y293); seed 7: 10, 63, 60, 118, 116 (peak 193). v1: 593 / 244 / 137. Old baseline: at most 26, 44, 62.
- `watched_kin` share (watching inside the same lineage): 50% / 56% / 71% at year 300 (v1: 63 / 68 / 41%).
- `learned` (the group's learned offsets): seed 1: 0.012 (y10), 0.065 (y50), 0.090 (y100), 0.109 (y150), 0.080 (y200), 0.067 (y300), max 0.129 (y72); seed 5: 0.015, 0.079, 0.128, 0.087, 0.071, 0.065 (max 0.129 at y103); seed 7: 0.010, 0.088, 0.111, 0.123, 0.101, 0.081 (max 0.124 at y122). It rises until year 70-120 and then falls to about half; v1 kept rising to 0.17. `greedy` (the best scored action is the one drawn): 0.10-0.16 before eaters; 0.60-0.67 at year 30; 0.90-0.92 at year 100; 0.70 / 0.84 / 0.69 at year 300 (v1: 0.78-0.90). So decisions are mostly "best scored", and the share of random choice is growing again late in seeds 1 and 7.
- What is missing: a column that says whether a watched behaviour was adopted. `learned` cannot tell individual learning from copying.
- Brain size does not change: `hidden` 11.6-12.1 (the mean), `memory` 6, `seen` 3, `sight` 1; `hidden_max` 12-14. No brain growth.

**Signals settling: weak; a plateau, not a settling.**
- Mutual information (bits; 1 bit is the maximum for a yes/no context; the small-sample bias is already taken off), mean over year 100 to the end: `mi_low` 0.076 / 0.154 / 0.113; `mi_hurt` 0.018 / 0.037 / 0.028; `mi_cold` 0.0000 / 0.0002 / 0.0000. Largest `mi_low`: 0.111 (seed 1, y96), 0.171 (seed 5, y140), 0.144 (seed 7, y141). v1: mean `mi_low` 0.016 / 0.052 / 0.089, `mi_hurt` 0.017 / 0.021 / 0.020; the old baseline 0.015-0.062. So `mi_low` is now 3-5 times higher in seeds 1 and 5 than v1 and 2 times the baseline's, but still at most 0.17 of 1 bit.
- `mi_low` by 50-year window (0-50, 50-100, ..., 250-300): seed 1: 0.029, 0.087, 0.091, 0.071, 0.069, 0.071; seed 5: 0.055, 0.086, 0.157, 0.146, 0.153, 0.159; seed 7: 0.032, 0.080, 0.128, 0.122, 0.106, 0.098. It rises until about year 100-150 and then stays flat or falls slightly. `mi_cold` falls to 0 (the cold is gone).

**First giving or exchange: giving yes, exchange no.**
- `gave` (puts into other bodies per year), years 10 / 30 / 50 / 100 / 200 / 300: seed 1: 24 / 1,084 / 1,261 / 622 / 1,034 / 28,680; seed 5: 17 / 1,362 / 1,133 / 178 / 478 / 227; seed 7: 30 / 1,179 / 1,673 / 10,039 / 6,434 / 3,543. Mean from year 50: 10,406 / 388 / 5,680. Mass given: 1.4e8 kg a year at year 30 to 1.3e10 (seed 1, year 300); 5e7-3e9 in seeds 5 and 7.
- Who gives: seed 1: 88% of gives by animals (eaters) from year 50 and 92% in years 250-300; seed 5: 65%; seed 7: 7% (33% in years 250-300). Seed 1 shows a clear event: `gave` is 340 a year at year 150, a spike of 4,246 at year 160 (3,429 by animals), 450-1,000 in years 170-200, 1,856 at year 210, and then 19k (y230), 43k (y240) and 23-47k a year to year 300 (`gave_eaters` 20-38k), about 17-28k distinct pairs a year.
- Reciprocal pairs (A gave to B and B gave to A): seed 1 mean 10.2 a year from year 50, with a burst of 99-133 a year in years 235-245 (max 166 at y242, 0.6% of that year's pairs) and 2-47 a year after (0.01-0.2% of pairs); seed 5: mean 0.13, max 22 (y26); seed 7: mean 5.5, max 25 (y204), 0.1% of pairs at year 300. So giving is one-way. Old baseline: gave about 5,000 a year at year 100 (seeds 1, 2), 764 in seed 5.

### M3

**Held objects as tools: a small sign, only while single animals exist.**
- Parts: first in year 23, last in year 115 / 98 / 97; max 11 (y74) / 28 (y41) / 46 (y67). `made_held` totals 912 / 2,773 / 4,315 over the run; `made_joined` 4 / 4 / 8; `put_to_parts` 4.8e3 / 1.2e4 / 1.7e4.
- Yield per take, years 20-60: bare 0.154 / 0.156 / 0.144 against held 0.171 / 0.183 / 0.162 (+11% / +18% / +13%); years 60-100: bare 0.094 / 0.095 / 0.086, held 0.117 / 0.128 / 0.112 (+24% / +36% / +30%). But held takes are 0.01-0.09% of all takes (1,056 / 4,923 / 2,967 held takes against 3.8 / 5.5 / 4.4 million bare in years 20-60). Dig yields: held digs 12 / 34 / 31 in the run, nothing to compare. After the parts vanish, held takes are 0 (last held take: year 116 / 163 / 98).
- v1: +13 / +31 / +41% in years 20-60 with 363-1,466 held takes, parts last year 75 / 90 / 199, max 4 / 13 / 7. So v2 has 2-7 times more parts at the peak (last year with a part 115 / 98 / 97 against 75 / 90 / 199), but the pattern is the same: a trace.
- The old baseline had far more: seed 5 had up to 1,491 parts and ~6,000 held takes a year, but its held yield was no better than bare.

**Shelters used in cold: did not appear.**
- `inside_cold` (share of cold animal body-months inside walls) is 0 in every year of every seed; `inside_warm` is at most 1.3e-4; `inside_now` at most 2 / 1 / 4 bodies; `enclosed` at most 7 / 12 / 24; `structures` is 1 in just two years in each seed (years 66-76 / 71-74 / 71-77; `bonds_joined` 2 in each). In `months.csv` the `inside` column is 0-5 bodies in years 25-100 and 0 after.
- The cold itself goes away early: `cold_months` (animal body-months below 278 K per year) is 343 / 74 / 90 at year 30, 136 / 151 / 11 at year 100 and 3 / 88 / 3 at year 300 (v1: 4,854 at year 30 in seed 1). So there is hardly any cold to shelter from, from the start.
- Old baseline seed 5: structures up to 34, enclosed up to 1,059, inside_cold up to 2.5% at year 120. That did not carry over.

**Containers: seen at the smallest level.** `holding_liquid` is at most 5 / 7 / 18 bodies and `liquid_held` at most 23 / 38 / 102 kg in the run (all before year 95). The baseline seed 5 had up to 363 holders.

**Floating structures: a trace.** `afloat` is 1-2 in 6 / 7 / 6 years (years 24-73) and `afloat_moves` is at most 1. The baseline seed 5 had up to 12.

Why parts vanish is not in the data. All parts, structures and holders are gone by year ~100-115 in every seed, when single (non-node) animals are gone or few: singles are 330 / 2,478 / 1,768 at year 30, 2,893 / 4,998 / 4,778 at year 50, 25 / 143 / 500 at year 100, 0 / 146 / 0 at year 200 (seed 5 keeps 146-199 singles to year 300 but has no parts after year 98). So the vanishing is tied to the move from single bodies to crowds, as in v1, but not just to "no singles left".

## 3. Notable events

### 3.1 Eaters and the plant cover (the cover collapse)

Eaters were placed in year 22 (842 / 1,008 / 874 bodies). Animal bodies grow to about 4,500-6,000 within 8 years (seed 1: 1,486 at year 23, 4,452 at year 30).

- **Immediate dip.** Plant members fall in the 3 years after placement: seed 1: 6.5e10 (y22) -> 3.9e10 (y26), -41%; seed 5: 9.3e10 -> 3.2e10 (y25), -65%; seed 7: 7.8e10 -> 3.3e10 (y39), -58%. (v1: -22 / -52 / -29%.)
- **The collapse is long, in all three seeds.** Plant members stay low for 50-80 years: seed 1: 3.7e10 at year 58 (-43% from year 22; they are 5.5e10 at year 50, 7.7e10 at year 30 and never above 8.6e10 until year 100); seed 5: 2.2e10 at year 53 (-76%); seed 7: 9.5e9 at year 75 (-88%). Median cover at or below 0.01 kg/m2 for 40 / 52 / 65 years. Plant bodies fall as well in seed 7 (3,598 at year 50 -> 1,921 at year 70, lowest 1,830). Plants on land: 41 / 30 / 31% at year 50; seed 7 is down to 15% at year 70. Body mass in the world is 1.4e8 t (y30) -> 1.1e8 (y50) -> 1.6e8 (y100) (seed 1), 8.7e7 -> 5.1e7 -> 1.1e8 (seed 5), and 8.3e7 -> 2.4e7 (y70) -> 6.6e7 (seed 7), so the whole living mass falls, not only the plants. `mass_land` (mean body mass per m2) in seed 7: 3.8 (y30), 0.8 (y75), 2.9 (y100), 11.8 (y300).
- **Animals during the collapse.** Animal bodies keep rising (seed 1: 4,452 -> 13,424 by year 75; seed 5: 5,942 -> 9,384 at year 60; seed 7: 5,113 -> 10,242 at year 70) while animal members stay flat at 1.5e8-5e8, so many crowds, few members each. Animal deaths are mostly SPENT (55-75%) and LACK (19-32% in year 50). Eaters that take from animals are already 286 / 986 / 665 at year 30.
- **Recovery.** Median cover (seed 1): 0.0015 (y60) -> 0.14 (y80) -> 0.32 (y90) -> 0.70 (y100) -> 1.1 (y110) -> 2.0 (y150) -> 3.4 (y300). Seed 5: 0.0007 (y60-70) -> 0.05 (y80) -> 0.35 (y100) -> 1.2 (y110) -> 2.4 (y120) -> 3.3. Seed 7: 0 (y70) -> 0.02 (y90) -> 0.23 (y100) -> 0.57 (y150) -> 1.4 (y160) -> 2.3 (y200) -> 3.1. Seed 7's recovery is slower and has a step around year 155-160 (plant members 4.4e10 at y150 -> 6.7e10 at y160). In each seed the animals are still there during recovery: animal bodies at year 100 are 17,350 / 11,089 / 12,229 (seed 1 reaches 3.9 times its year-30 count). The data cannot say what ends the collapse. What happens at the same time: `eat_animals` and `a_taken` rise (seed 1: `a_taken` 0.9% of deaths at y50, 20% at y100; seed 7: 6% / 37%), `lack` deaths fall (19-32% at year 50 to 1-8% at year 100), the world warms +2.2 K in seed 1 over years 50-100 (see 3.2), and the animal bodies grow 25-30% between year 75 and year 100 (they doubled from year 50 to year 100 in seed 1). That fits "predators and smaller animals relieve the plants" but I did not test it.
- **Why v2 is different from v1.** In v1 the seed 1 cover rose all the time (median 0.8 at y50, 2.4 at y100) and only seed 5 (years 55-85) and, slowly, seed 7 (-63%) dipped. Both fixes happened between the runs: the emptied-body NaN and the lumps of light dumped on the ground. The data show what changed (plants lower for 50-80 years in all seeds, hot cells nearly gone, gas lower early) but cannot say which part of the fix caused it. A guess: the lumps of light that heated cells were an extra energy source for plants and warmed the ground near them; I did not test this.
- **Cover spikes** are gone: `cover_max` is 54-231 kg/m2 over the whole runs (v1 seed 7: 738-3,492).

### 3.2 The heat-absorbing gas and the temperature

- **What happened.** `absorber` (seed 1 / 5 / 7): year 0: 1.46 / 1.49 / 1.47; year 50: 2.57 / 2.80 / 2.76; year 100: 4.03 / 3.67 / 3.46; year 200: 11.48 / 8.33 / 7.81; year 300: 16.56 / 11.14 / 10.76. Against v1 (year 50: 2.75 / 3.10 / 3.13; year 100: 6.40 / 4.48 / 5.63; year 300: 9.67 / 9.05) it is lower early and higher late in seed 1 (1.7 times) and seed 5 (1.2 times).
- **The split** (Appendix A; a fit of `absorber` on the three gas columns reproduces it to within 0.003): water vapour 0.98 -> 1.61 (seed 1; it is flat at 1.6 from year 150), CO2 0.42 -> 0.31 (y30) -> 0.34 (y100) -> 5.01 (y300), methane 0.06 -> 2.18 (y100) -> 9.95 (y300). Methane is 54-63% of the absorber from year 100 on. CO2 first falls 40-50% in years 0-20 (0.40e-3 to 0.20-0.24e-3: plants take it up), then sits at 0.29-0.34e-3 until year 100-150.
- **CO2 timing.** CO2 rises above 1.5 times its year 40-90 level at year 106 / 115 / 154; the median cover is 0.95 / 1.9 / 1.1 kg/m2 then, i.e. when the plants have recovered (3.1: median passes 1 kg/m2 at year 109 / 109 / 154). After that CO2 climbs 0.3-0.6e-3 per 20 years in seed 1 (0.33e-3 at y100 to 4.78e-3 at y300; 14 times) and in seed 5 (3.24e-3 at y300), and in seed 7 it jumps from 0.3e-3 to 1.1e-3 in years 150-180 and then creeps to 1.4e-3 (4.5 times). Standing living mass is at its highest then (seed 1: 3.0e8 t at year 300 against 1.1e8 at year 50). In v1 the rise started at year 50 (when cover was already high) and was slower. Carbon is conserved (total drift 3e-8 over 300 years), so it is moving from ground and bodies into the air; I did not read the chemistry by reaction.
- **Methane** starts from 1.8e-6, is 3.1e-6 at year 10 and rises about steadily: 6.2e-5 (y100), 2.0e-4 (y200), 2.85e-4 (y300) in seed 1; 4.8e-5 / 1.4e-4 / 1.7e-4 in seed 5 (peak 2.14e-4 at y281); 4.6e-5 / 1.4e-4 / 2.2e-4 in seed 7. Growth per year is at most 2.8e-6 / 1.6e-6 / 1.6e-6 (years 185, 230, 172). In v1 seed 1 methane stopped rising at year 150 (139e-6 -> 146e-6) and here it kept rising to year 290: there is no visible loss, so it accumulates as long as the source runs (the project notes say its lifetime in the model is about 2 million years, which fits). 
- **What the ground does.** `t_ground` (seed 1): 287.9 K (y0), 285.6 (y5), 291.5 (y50), 293.7 (y100), 294.7 (y200), 294.7 (y300). Seed 5: 288.9, 287.7, 293.0, 294.4, 295.5, 295.6. Seed 7: 288.2, 286.9, 292.5, 293.6, 295.0, 295.1. So the warm-up is 8.0-9.1 K from year 5, 82-89% of it in the first 100 years, then only 1.0-1.5 K in the next 200 years, while the absorber goes up 2-4 times (seed 1: 4.03 to 16.56). Fit T = a + b ln(absorber): over years 5-300 b = 2.6 / 2.4 / 2.8 K per e-fold (rms 0.8-1.2 K); over years 100-300 b = 0.43 / 0.59 / 1.08 K per e-fold (rms 0.14). v1 over years 100-300: 2.7 / 1.3 / 0.6. With the early slope the late gas rise would have given +3.7 K (seed 1) and it gave +1.0 K, so something else damps the response. The data cannot say what (the water vapour is flat, the land is covered, the rain is up 94%).
- **Land cells at 298 K or more** (read from the temperature panel, approximate): seed 1: 0% (y10, y30, y50), 3% (y100), 7% (y200), 6% (y300); seed 5: 0, 0, 1, 4, 11, 14%; seed 7: 0, 0, 1, 2, 11, 11%. v1: 14-17% at year 50, 24-25% at year 300. Mean land temperature (panel): seed 1: 278.8 K (y10), 286.5 (y50), 290.1 (y100), 291.5 (y300) (v1: 279.0, 286.1, 289.0, 292.0). So the hot spots are what the light fix removed; the land mean temperature is about the same as in v1 (within 1.0 K at every year read).
- **Ice and rain.** Land cells at 280 K or less (panel): seed 1: 64% (y10), 18% (y30), 0% (y50). Mean rain 6.9e-4 -> 13.4e-4 mol/m2/s (+94%, seed 1), 7.8 -> 15.2 (+95%, seed 5), 7.6 -> 14.5 (+92%, seed 7); standing water on land x19 / x14 / x8.5 (years 10 to 300). Land area shrinks 14-17%.
- **Seed 5 methane fall at the end.** From year 295 to 300 methane falls 2.05e-4 -> 1.67e-4 (-19%), with the rate speeding up each year (-2, -5, -7, -7, -8, -11e-6 a year) and the absorber dropping 12.35 -> 11.14 in 5 years, while CO2 keeps rising. It is not in seeds 1 or 7. v1 seed 7 had the same pattern before its crash (methane -97% in 22 years), and the crash cause was fixed since, so this may be harmless; but a run of 400 years from seed 5 would show it. In the same years the median cover goes from 3.7 to 3.3 kg/m2 and animal members swing between 1.1e12 and 3.6e12; no cause shown.
- **Energy books.** `heat not given` is 0.00e0 J in all log lines of all three seeds (v1 seed 1: 8.4e16 J at year 300; v1 seed 5 1.55e16). Element drift at most 3.0e-16, energy drift at most 3.4e-13 at year 300 (seed 1; v1 seed 7 had 1e-10). The NaN crash of v1 seed 7 did not come back.

### 3.3 Population cycles

See M1 above. In short: no regular cycle in years. The slow rise of the animal members (2-4 orders of magnitude) dominates; after removing the trend, the animal members swing 0.06-0.08 in log10 (15-20%) with no steady period (autocorrelation peaks 0.4-0.5 at lags 54-72 years in seeds 1 and 7, none in seed 5). Animal bodies are steady after year 100 (+-2-4%). The strong cycle is the season in the temperature, not in the numbers (2-7% range).

## 4. Comparison with v1 and with the old baseline

The new code's `plants`, `animals` count bodies, and a body can hold up to 1e8-1e11 members; the old baseline body was one 200-tonne blob. So body counts are not like for like between the old baseline and v1/v2. v1 and v2 are comparable.

| run | year | plant bodies | animal bodies | plant members | cover median | t_ground K | absorber | liquid_land km3 | live lineages | parts | ms/yr (log) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| old baseline seed 1 | 50 | 1391 | 0 | nan | nan | 284.3 | 1.44 | 16.4 | 58 | 4 | 732 |
| old baseline seed 1 | 100 | 9610 | 0 | nan | nan | 293.2 | 2.74 | 15.5 | 363 | 43 | 8745 |
| old baseline seed 1 | 120 | 10026 | 0 | nan | nan | 294.7 | 3.51 | 12.1 | 406 | 83 | 13548 |
| old baseline seed 2 | 50 | 4191 | 0 | nan | nan | 288.7 | 1.84 | 12.2 | 158 | 10 | 1236 |
| old baseline seed 2 | 100 | 8115 | 0 | nan | nan | 297.6 | 4.32 | 5.8 | 303 | 25 | 4845 |
| old baseline seed 2 | 120 | 8477 | 0 | nan | nan | 298.3 | 4.54 | 4.0 | 301 | 23 | 11527 |
| old baseline seed 5 | 50 | 3505 | 1130 | nan | nan | 288.7 | 1.74 | 4.7 | 158 | 166 | 1646 |
| old baseline seed 5 | 100 | 3252 | 4521 | nan | nan | 290.3 | 2.06 | 3.4 | 249 | 722 | 14878 |
| old baseline seed 5 | 120 | 3654 | 4359 | nan | nan | 291.0 | 2.26 | 3.1 | 263 | 703 | 4946 |
| v1 seed 1 | 50 | 4966 | 4338 | 2.11e+11 | 0.801 | 291.3 | 2.75 | 61.9 | 6 | 1 | 7431 |
| v1 seed 1 | 100 | 5332 | 4833 | 1.87e+11 | 2.41 | 293.3 | 6.40 | 119.5 | 4 | 0 | 8518 |
| v1 seed 1 | 120 | 5490 | 5120 | 1.91e+11 | 2.97 | 293.8 | 7.15 | 140.8 | 4 | 0 | 8790 |
| v1 seed 5 | 50 | 3924 | 4600 | 1.47e+11 | 0.744 | 293.4 | 3.10 | 28.2 | 17 | 7 | 6711 |
| v1 seed 5 | 100 | 4232 | 12744 | 2.85e+10 | 0.486 | 293.2 | 4.48 | 41.7 | 44 | 0 | 16792 |
| v1 seed 5 | 120 | 4298 | 12197 | 3.93e+10 | 1.14 | 293.4 | 4.96 | 46.7 | 23 | 0 | 18834 |
| v1 seed 7 | 50 | 3920 | 3646 | 1.25e+11 | 0.392 | 293.5 | 3.13 | 29.9 | 7 | 1 | 6297 |
| v1 seed 7 | 100 | 4052 | 7582 | 7.36e+10 | 0.35 | 293.8 | 5.63 | 53.7 | 45 | 1 | 10411 |
| v1 seed 7 | 120 | 4184 | 9984 | 4.54e+10 | 0.507 | 293.1 | 6.42 | 62.0 | 27 | 0 | 13516 |
| v2 (new) seed 1 | 50 | 5036 | 8331 | 5.51e+10 | 0.00871 | 291.5 | 2.57 | 70.0 | 17 | 5 | 5774 |
| v2 (new) seed 1 | 100 | 5889 | 17350 | 7.59e+10 | 0.699 | 293.7 | 4.03 | 132.0 | 17 | 0 | 16923 |
| v2 (new) seed 1 | 120 | 6044 | 17325 | 9.38e+10 | 1.53 | 294.3 | 5.60 | 157.7 | 12 | 0 | 18150 |
| v2 (new) seed 5 | 50 | 3543 | 9217 | 2.57e+10 | 0.0021 | 293.0 | 2.80 | 29.6 | 54 | 9 | 6494 |
| v2 (new) seed 5 | 100 | 4193 | 11089 | 4.67e+10 | 0.348 | 294.4 | 3.67 | 41.7 | 20 | 0 | 10207 |
| v2 (new) seed 5 | 120 | 4166 | 11734 | 9.76e+10 | 2.42 | 295.0 | 4.48 | 47.1 | 17 | 0 | 11723 |
| v2 (new) seed 7 | 50 | 3598 | 9097 | 2.96e+10 | 0.000545 | 292.5 | 2.76 | 33.6 | 39 | 9 | 6116 |
| v2 (new) seed 7 | 100 | 4054 | 12229 | 2.41e+10 | 0.231 | 293.6 | 3.46 | 58.8 | 26 | 0 | 11641 |
| v2 (new) seed 7 | 120 | 4314 | 13214 | 2.81e+10 | 0.284 | 294.0 | 3.83 | 68.3 | 10 | 0 | 13615 |

(`plant members` and `cover median` do not exist in the baseline tape.)

- **Eaters surviving:** old baseline: died out in seed 1 (last year with an animal: 27) and seed 2 (28), within 5-6 years of placement; seed 5 survived (4,359 animal bodies at year 120). v1: alive to the end in 3 of 3 (300, 300, 220 years). v2: alive in 3 of 3 to year 300, with 16,821 / 13,023 / 12,449 bodies at the end (v1: 5,672 / 13,065 / 1,658). All with the hand-set biases; neutral eaters died in 8 of 8 runs (from the brief), not rerun.
- **Plants:** v2 has far lower plant cover for years 22-90 (median at year 50: 0.0087 / 0.0021 / 0.0005 against v1 0.80 / 0.74 / 0.39, i.e. 90-700 times lower), then the same or higher at year 300 (3.4 / 3.3 / 3.1 against 3.6 / 2.6 / 14.5). v1 seed 7 ended with plants winning (14.5 kg/m2, animals down 85%); v2 seed 7 ends balanced (3.1 kg/m2, 12,449 animal bodies). v1 had one big cover collapse (seed 5); v2 has three.
- **Climate:** same year 0. Year 100: absorber v2 4.03 / 3.67 / 3.46 vs v1 6.40 / 4.48 / 5.63 vs old baseline 2.74 (seed 1) / 4.32 (seed 2) / 2.06 (seed 5); year 300: v2 16.56 / 11.14 / 10.76 vs v1 9.67 / 9.05 / (6.82 at y220). Ground temperature at year 300: v2 294.7 / 295.6 / 295.1 K, v1 294.4 / 294.2 / 295.0 (y220). Hot cells (>=298 K): v1 14-25%, v2 0-14%. So the lump fix removed the early burn gas and the hot cells; the later rise (CO2 after cover recovery, methane) is more than before.
- **Standing water on land at year 100:** seed 1: v2 132, v1 119.5, baseline 15.5 km3; seed 5: 41.7 / 41.7 / 3.4; seed 7: 58.8 / 53.7. The same as v1.
- **Lineages (live) at year 100:** seed 1: v2 17, v1 4, baseline 363; seed 5: 20 / 44 / 249; seed 7: 26 / 45. Ever: v2 2,617 / 3,001 / 4,854 against v1 112 / 4,461 / 9,806.
- **Parts and structures:** baseline parts at year 100: 43 / 25 / 722; structures up to 34, afloat up to 12, inside_cold up to 2.5% (seed 5); v1 parts max 4 / 13 / 7; v2 parts max 11 / 28 / 46 (more than v1, still 30-100 times fewer than baseline seed 5), structures 1 in two years, afloat at most 2, inside_cold 0.
- **Trails:** v2 worn share 91 / 82 / 88% vs v1 76 / 60 / 55%; `worn_persist` at year 300 0.85 / 0.82 / 0.84 vs 0.77 / 0.64 / 0.60 (seed 7: year 200). `traffic_top2` v2 0.10 / 0.38 / 0.16 vs v1 0.12 / 0.51 / 0.36; old baseline 0.35-0.59.
- **Imitation:** `spread_max` old baseline at most 26 / 44 / 62; v1 593 / 244 / 137; v2 1,246 / 309 / 193 (peaks), 733 / 238 / 116 at year 300. `watched` v2 at year 300: 20.9k / 6.9k / 10.9k. `learned` is lower in v2 (0.065-0.081 vs 0.17 at year 300).
- **Signals, giving:** `mi_low` v2 mean from year 100: 0.076 / 0.154 / 0.113 vs v1 0.016 / 0.052 / 0.089 vs old baseline 0.015-0.062; `gave` old baseline about 5,000 a year at year 100 in seeds 1, 2 (v2: 622 / 178 / 10,039 in seeds 1 / 5 / 7); v2 seed 1 has the new animal giving burst (23-47k a year from year ~230).
- **Eaters of eaters:** v2 `eat_animals` mean 2,191 / 863 / 1,856 vs v1 3.9 / 698 / 381; `a_taken` share of animal deaths 26 / 10 / 24% vs v1 0.0 / 11 / 7%.
- **Speed:** see 6. The first 120 years take on average 9.3 / 6.9 / 7.6 s per year in v2 (v1: 6.5 / 9.7 / 7.0; old baseline 3.7 / 3.4 / 4.4). v2 seed 1 is 65% slower than v1 seed 1 over the whole run (15.7 vs 9.5 s per year) because it keeps 17k animal bodies (v1: 5.7k; 23.8k bodies in the world against 12.0k). v2 seed 5 is 38% faster than v1 seed 5 (10.5 vs 17.0 s per year) with the same number of bodies in the world at year 300 (17.4k vs 17.5k): the bodies law cost per body fell from 1.30 to 0.84 ms.

## 5. Pictures worth looking at

Each picture has five panels, left to right: map (height, water; flowing water in blue), temperature (blue cold, red hot), rain, life (green = light-eaters, red/orange dots = animals), traffic over the map. The six chosen pictures are copied unchanged to `docs/reports/watch-pre-m4/`.

1. `seed1_year0000.png` (= `watch/seed1/year0000.png`) - the start. Bare brown island (life panel all brown, no plants, no animals), strong temperature gradient from pale blue in the north (275 K) to red in the south (301 K), rain panel uniformly cream (no rain yet), traffic panel only the map. The reference for everything else: 280 inland water cells and 6,258 land cells.
2. `seed1_year0050.png` - the cover collapse. The life panel is mostly brown land with scattered green dots (plants on 41% of land cells) and a mesh of orange animal dots over about 75% of the land; the green is thickest in the west and north, and the central and eastern lowlands are brown. The land in the temperature panel is pale yellow (mean 286.5 K, cooler than the orange sea at 293 K), the rain panel shows dry cream land and green-teal sea, with faint north-south streaks over the northern sea. The traffic panel is already solid cream over all land.
3. `seed7_year0070.png` - the deepest collapse (median cover 0, plant members 1.2e10, plants on 15% of land). The island has a bright white high-ground cap in the middle and blue valley lakes around it. Life panel: almost all brown with animal dots, with green only in small patches on the south coast and on the west islands. The temperature panel has red dots in the south where the plants are. The traffic panel is the only one that is not saturated: orange-pink patches in the north-centre and the east. Different island from seed 1.
4. `seed1_year0120.png` - recovered. Green is back over 79% of land with brown gaps in the middle and a dense brown patch in the north-centre; red dots in the south of the temperature panel (but only 0-3% of land at 298 K or more); lakes along the valleys in the map. Traffic panel solid cream. Compare with picture 2 (70 years earlier).
5. `seed1_year0300.png` - rivers and lakes. Valleys filled with long light-blue lakes and rivers (1,331 inland water cells, up from 280), a dark lake in the centre, flooded coast (land 5,163 cells, down 17%); the land is orange (291.5 K mean) with a few red dots in the south, only 6% of land at 298 K or more; green with red animal dots over 76% of the land; the traffic panel solid cream (saturated, so trails cannot be read).
6. `seed5_year0300.png` - another island, 300 years: a large inland water body in the east (the largest inland water, 352 cells, about 1,400 km2), a long narrow lake in the north-centre, many river lines; the hottest cells (red) crowd in the south and south-east where life is dense (14% of land at 298 K or more); life panel dense green with red dots; traffic panel saturated. This is the run with a third kind and 3.3e12 animal members.

## 6. Speed

ms per simulated year (mean over the 10 years before the log line; from the logs; "bodies" is the bodies law, "parts" the bond law):

| seed | year | flow | change | bodies | parts | total ms/yr | bodies in world | ms per body-year | years per s |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 50 | 671 | 190 | 4912 | 0.9 | 5774 | 13367 | 0.43 | 0.173 |
| 1 | 100 | 686 | 202 | 16035 | 0.0 | 16923 | 23239 | 0.73 | 0.059 |
| 1 | 200 | 820 | 237 | 19240 | 0.0 | 20297 | 22952 | 0.88 | 0.049 |
| 1 | 300 | 796 | 234 | 20927 | 0.0 | 21957 | 23767 | 0.92 | 0.046 |
| 5 | 50 | 679 | 206 | 5606 | 3.5 | 6494 | 12760 | 0.51 | 0.154 |
| 5 | 100 | 663 | 197 | 9347 | 0.0 | 10207 | 15282 | 0.67 | 0.098 |
| 5 | 200 | 784 | 232 | 11370 | 0.0 | 12386 | 15612 | 0.79 | 0.081 |
| 5 | 300 | 810 | 237 | 13634 | 0.0 | 14681 | 17439 | 0.84 | 0.068 |
| 7 | 50 | 701 | 196 | 5217 | 1.6 | 6116 | 12695 | 0.48 | 0.164 |
| 7 | 100 | 676 | 182 | 10783 | 0.3 | 11641 | 16283 | 0.71 | 0.086 |
| 7 | 200 | 813 | 228 | 14777 | 0.0 | 15818 | 18102 | 0.87 | 0.063 |
| 7 | 300 | 810 | 230 | 15912 | 0.0 | 16952 | 17741 | 0.96 | 0.059 |
| 7 | 220 (last line) | 814 | 231 | 15420 | 0.0 | 16465 | 18262 | 0.90 | 0.061 |
| baseline 1 | 50 | 309 | 109 | 313 | 0.6 | 732 | 1391 | 0.53 | 1.367 |
| baseline 1 | 100 | 749 | 463 | 7517 | 15.7 | 8745 | 9610 | 0.91 | 0.114 |
| baseline 1 | 120 | 1028 | 650 | 11829 | 40.5 | 13548 | 10026 | 1.35 | 0.074 |
| baseline 2 | 50 | 334 | 171 | 730 | 1.3 | 1236 | 4191 | 0.29 | 0.809 |
| baseline 2 | 100 | 640 | 463 | 3737 | 5.4 | 4845 | 8115 | 0.60 | 0.206 |
| baseline 2 | 120 | 1390 | 1031 | 9094 | 11.9 | 11527 | 8477 | 1.36 | 0.087 |
| baseline 5 | 50 | 345 | 164 | 1121 | 16.3 | 1646 | 4635 | 0.36 | 0.607 |
| baseline 5 | 100 | 1292 | 688 | 12609 | 288.5 | 14878 | 7773 | 1.91 | 0.067 |
| baseline 5 | 120 | 424 | 233 | 4196 | 93.5 | 4946 | 8013 | 0.62 | 0.202 |

- Whole-run mean from the last log line: seed 1: 300 years in 4,714 s (78.6 min), 15,714 ms per year; seed 5: 3,163 s (52.7 min), 10,543 ms per year; seed 7: 3,796 s (63.3 min), 12,654 ms per year. v1: 47.5 / 84.9 / (about 49, estimate for 220 years) min. Old baseline: 120 years in 445 / 408 / 530 s (3,707 / 3,396 / 4,418 ms per year).
- Flow (about 0.7-0.8 s) and change (about 0.2 s) are flat for 300 years and smaller than in v1 (1.1-1.3 s and 0.4 s): flow is 30-40% and change about 50% cheaper. The bodies law grows: 4.9 / 5.6 / 5.2 s at year 50 to 20.9 / 13.6 / 15.9 s at year 300 (95% / 93% / 94% of the year-300 time). Cost per body in the world is 0.43-0.51 ms at year 50 and 0.84-0.96 ms at year 300, so the cost per body rises as the world gets crowded.
- Compared with the M1-M3 target (1,000 years in under 10 minutes = 0.6 s per year): the whole-run mean is 26 / 18 / 21 times too slow; at year 300, 37 / 24 / 28 times too slow. 1,000 years at the year-300 rate would take about 6.1 / 4.1 / 4.7 hours.
- The runs were not alone on the machine (three at once on 4 cores), so a single run alone may be a little faster; the old baseline and v1 were run the same way.

## Appendix A. The absorber split

**Where the absorber comes from.** `absorber` = sum over heat-absorbing gases of (amount x its absorption number) / area. Only three gases have a column (`air_0`, `air_3`, `air_4`; indices 0, 3, 4 = water, carbon dioxide, methane in the viewer names of `library/chem0.toml`; the core knows no names). A least-squares fit of `absorber` on the three columns reproduces it to within 0.003, so the split below is reliable. Units: absorber units.

| seed | year | water vapour | CO2 | methane | total |
|---|---|---|---|---|---|
| 1 | 0 | 0.98 | 0.42 | 0.06 | 1.46 |
| 1 | 30 | 1.17 | 0.31 | 0.54 | 2.01 |
| 1 | 50 | 1.34 | 0.32 | 0.92 | 2.57 |
| 1 | 100 | 1.51 | 0.34 | 2.18 | 4.03 |
| 1 | 200 | 1.61 | 2.75 | 7.11 | 11.48 |
| 1 | 300 | 1.61 | 5.01 | 9.95 | 16.56 |
| 1 | fit weights | 243 | 1046 | 34933 | |
| 5 | 0 | 1.00 | 0.43 | 0.06 | 1.49 |
| 5 | 30 | 1.38 | 0.32 | 0.70 | 2.40 |
| 5 | 50 | 1.48 | 0.35 | 0.97 | 2.80 |
| 5 | 100 | 1.61 | 0.36 | 1.71 | 3.67 |
| 5 | 200 | 1.72 | 1.63 | 4.98 | 8.33 |
| 5 | 300 | 1.73 | 3.45 | 5.96 | 11.14 |
| 5 | fit weights | 248 | 1065 | 35622 | |
| 7 | 0 | 0.99 | 0.42 | 0.06 | 1.47 |
| 7 | 30 | 1.31 | 0.30 | 0.67 | 2.28 |
| 7 | 50 | 1.43 | 0.32 | 1.01 | 2.76 |
| 7 | 100 | 1.52 | 0.32 | 1.62 | 3.46 |
| 7 | 200 | 1.65 | 1.19 | 4.97 | 7.81 |
| 7 | 300 | 1.66 | 1.47 | 7.62 | 10.76 |
| 7 | fit weights | 245 | 1051 | 35162 | |

## Appendix B. Tables by decade (years 10, 20, 30, 50, 75, 100, 150, 200, 250, 300)

All values are read from `series.csv` of each seed, one row per year, at the listed year. The flow columns (births, deaths, ate_*, crossings, take_*, gave, watched) count the year before. `rain` is mol/m2/s x 1e-4; `air_0`, `air_3`, `air_4` are the mean mole fractions of the three heat-absorbing gases (indices 0, 3, 4 = water, carbon dioxide, methane in the viewer names of `library/chem0.toml`); `liquid_land` is km3; `cover_*` is kg/m2; `a_*` are animal deaths per year by cause (members): lack = a store fell below its minimum, phase = a key liquid froze or boiled, spent = the body could not pay its upkeep, taken = taken apart by another body, broken = bonds broken by moving bodies. `mass_*` is the mass of one member (kg), over all bodies. `ate_*` are kg per year (the code label; the earlier baseline analysis noted the magnitudes look too large, so use them as relative values). `ice_frac` 0 means none.

### Seed 1
**Climate**

| year | t_ground | t_air | absorber | air_0 x1e-3 | air_3 x1e-3 | air_4 x1e-6 | ice_frac | liquid_land | rain x1e-4 |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 285.9 | 253.5 | 1.42 | 3.99 | 0.329 | 3.06 | 4.3e-04 | 13.5 | 6.91 |
| 20 | 287.2 | 255.0 | 1.58 | 4.30 | 0.242 | 8.16 | 6.1e-05 | 27.9 | 7.34 |
| 30 | 289.2 | 257.2 | 2.01 | 4.80 | 0.294 | 15.5 | 3.1e-04 | 42.1 | 8.49 |
| 50 | 291.5 | 260.0 | 2.57 | 5.51 | 0.304 | 26.2 | 0 | 70.0 | 10.7 |
| 75 | 292.4 | 261.0 | 3.11 | 5.78 | 0.263 | 40.8 | 0 | 101 | 11.6 |
| 100 | 293.7 | 262.6 | 4.03 | 6.20 | 0.326 | 62.4 | 6.1e-05 | 132 | 12.5 |
| 150 | 294.6 | 263.9 | 7.75 | 6.55 | 1.63 | 127 | 0 | 190 | 13.3 |
| 200 | 294.7 | 264.1 | 11.5 | 6.62 | 2.63 | 204 | 0 | 227 | 13.4 |
| 250 | 294.7 | 264.0 | 14.4 | 6.59 | 3.54 | 260 | 0 | 246 | 13.4 |
| 300 | 294.7 | 264.0 | 16.6 | 6.62 | 4.78 | 285 | 0 | 253 | 13.4 |

**Life**

| year | plants | plant_members | animals | animal_nodes | animal_single | animal_members | cover_median | cover_max | kinds | lineages |
|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 1063 | 3.23e+10 | 0 | 0 | 0 | 0 | 0 | 126 | 1 | 1 |
| 20 | 1695 | 6.01e+10 | 0 | 0 | 0 | 0 | 0 | 126 | 1 | 1 |
| 30 | 4095 | 7.74e+10 | 4452 | 4122 | 330 | 3.29e+08 | 1.5e-04 | 143 | 2 | 4 |
| 50 | 5036 | 5.51e+10 | 8331 | 5438 | 2893 | 4.09e+08 | 0.009 | 110 | 2 | 17 |
| 75 | 5201 | 7.30e+10 | 13424 | 9279 | 4145 | 5.05e+08 | 0.047 | 144 | 2 | 63 |
| 100 | 5889 | 7.59e+10 | 17350 | 17325 | 25 | 2.85e+09 | 0.699 | 87.3 | 2 | 17 |
| 150 | 6326 | 1.20e+11 | 17339 | 17339 | 0 | 7.02e+09 | 1.99 | 58.7 | 2 | 10 |
| 200 | 6779 | 1.62e+11 | 16173 | 16173 | 0 | 2.27e+10 | 2.30 | 53.9 | 2 | 9 |
| 250 | 6929 | 3.01e+11 | 15511 | 15511 | 0 | 3.97e+10 | 3.34 | 69.0 | 2 | 8 |
| 300 | 6946 | 4.81e+11 | 16821 | 16821 | 0 | 3.36e+10 | 3.39 | 61.5 | 2 | 6 |

**Sizes and feeding**

| year | mass_min | mass_median | mass_max | trophic_max | trophic_high | eat_animals | ate_light_eaters | ate_others | ate_ground |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 0.371 | 1.45 | 4.39 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0.239 | 1.60 | 6.10 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 0.090 | 26.4 | 284 | 0.004 | 0.084 | 286 | 1.11e+10 | 7.47e+07 | 1.85e+10 |
| 50 | 0.102 | 38.6 | 710 | 6.0e-05 | 0.136 | 407 | 2.04e+10 | 7.59e+05 | 1.75e+10 |
| 75 | 0.036 | 39.0 | 929 | 0.016 | 0.095 | 2058 | 1.25e+10 | 8.37e+08 | 4.24e+10 |
| 100 | 0.023 | 19.2 | 4794 | 0.064 | 0.103 | 3064 | 4.43e+10 | 3.44e+09 | 9.06e+10 |
| 150 | 0.027 | 13.6 | 2.21e+05 | 0.071 | 0.071 | 1651 | 1.14e+11 | 9.29e+09 | 1.23e+11 |
| 200 | 0.015 | 5.29 | 5808 | 0.126 | 0.126 | 1748 | 1.36e+11 | 2.08e+10 | 1.41e+11 |
| 250 | 0.040 | 4.20 | 7.36e+05 | 0.164 | 0.164 | 2141 | 1.17e+11 | 2.07e+10 | 1.43e+11 |
| 300 | 0.024 | 3.68 | 4759 | 0.165 | 0.165 | 2857 | 1.12e+11 | 1.88e+10 | 1.23e+11 |

**Animal deaths per year (members, by cause)**

| year | a_lack | a_phase | a_spent | a_taken | a_broken | births_animals | died_animals |
|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 2.83e+08 | 1.11e+07 | 5.28e+08 | 1.70e+07 | 5.42e+07 | 9.35e+08 | 8.93e+08 |
| 50 | 2.70e+08 | 1.19e+07 | 1.02e+09 | 1.27e+07 | 8.30e+07 | 1.40e+09 | 1.40e+09 |
| 75 | 7.85e+08 | 3.80e+07 | 1.41e+09 | 4.90e+08 | 1.35e+08 | 2.81e+09 | 2.86e+09 |
| 100 | 4.19e+08 | 5.31e+07 | 2.30e+10 | 6.12e+09 | 1.36e+09 | 3.12e+10 | 3.09e+10 |
| 150 | 2.77e+08 | 3.13e+07 | 4.99e+10 | 1.55e+10 | 5.81e+09 | 7.19e+10 | 7.16e+10 |
| 200 | 1.72e+08 | 8.56e+07 | 1.50e+11 | 8.75e+10 | 3.09e+10 | 2.69e+11 | 2.69e+11 |
| 250 | 4.76e+08 | 2.20e+08 | 2.55e+11 | 1.90e+11 | 6.19e+10 | 5.10e+11 | 5.08e+11 |
| 300 | 6.76e+08 | 3.52e+08 | 2.56e+11 | 1.71e+11 | 5.45e+10 | 4.82e+11 | 4.83e+11 |

**Learning and brains**

| year | learned | greedy | hidden | memory | seen | sight | hidden_max |
|---|---|---|---|---|---|---|---|
| 10 | 0.012 | 0.120 | 12 | 6 | 3 | 1 | 12 |
| 20 | 0.015 | 0.116 | 12 | 6 | 3 | 1 | 12 |
| 30 | 0.022 | 0.668 | 12 | 6 | 3 | 1 | 12 |
| 50 | 0.065 | 0.818 | 12 | 6 | 3 | 1 | 12 |
| 75 | 0.118 | 0.914 | 12.0 | 6 | 3 | 1 | 13 |
| 100 | 0.090 | 0.908 | 11.7 | 6 | 3 | 1 | 13 |
| 150 | 0.109 | 0.882 | 11.9 | 6 | 3 | 1 | 13 |
| 200 | 0.080 | 0.828 | 11.9 | 6 | 3 | 1 | 13 |
| 250 | 0.073 | 0.725 | 11.9 | 6 | 3 | 1 | 13 |
| 300 | 0.067 | 0.699 | 11.9 | 6 | 3 | 1 | 13 |

**Trails**

| year | worn_edges | worn_persist | traffic_top2 | traffic_persist | crossings | mass_top2 | mass_land |
|---|---|---|---|---|---|---|---|
| 10 | 1212 | nan | 0.351 | nan | 2.62e+11 | 9.01 | 1.57 |
| 20 | 2684 | 0.194 | 0.302 | 0.080 | 4.14e+11 | 13.8 | 3.14 |
| 30 | 5875 | 0.311 | 0.250 | 0.095 | 4.40e+11 | 15.7 | 5.10 |
| 50 | 5106 | 0.296 | 0.266 | 0.104 | 3.17e+11 | 12.1 | 3.91 |
| 75 | 5351 | 0.274 | 0.304 | 0.061 | 4.71e+11 | 13.4 | 4.38 |
| 100 | 6876 | 0.389 | 0.262 | 0.193 | 3.47e+11 | 11.7 | 5.96 |
| 150 | 14762 | 0.784 | 0.194 | 0.303 | 1.01e+12 | 15.6 | 9.40 |
| 200 | 17544 | 0.859 | 0.139 | 0.260 | 2.59e+12 | 16.6 | 10.6 |
| 250 | 18928 | 0.868 | 0.111 | 0.211 | 9.27e+12 | 19.6 | 11.9 |
| 300 | 18830 | 0.853 | 0.102 | 0.174 | 1.77e+13 | 14.8 | 11.4 |

**Imitation, signals, giving**

| year | watched | watched_kin | watched_groups | spread_max | gave | gave_eaters | gave_pairs | gave_reciprocal | mi_cold | mi_hurt | mi_low |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 1160 | 1160 | 417 | 6 | 24 | 0 | 24 | 0 | 0.001 | 2.0e-04 | 0.059 |
| 20 | 2167 | 2167 | 606 | 10 | 50 | 0 | 50 | 0 | 0.003 | 0 | 0.015 |
| 30 | 7014 | 3156 | 1661 | 17 | 1084 | 129 | 1046 | 7 | 2.9e-04 | 0.013 | 0.017 |
| 50 | 5270 | 2087 | 1043 | 82 | 1261 | 4 | 1223 | 0 | 6.1e-05 | 0.018 | 0.045 |
| 75 | 4289 | 1352 | 783 | 226 | 1062 | 51 | 1025 | 1 | 1.5e-04 | 0.010 | 0.095 |
| 100 | 2234 | 861 | 652 | 215 | 622 | 18 | 597 | 0 | 1.6e-04 | 0.019 | 0.110 |
| 150 | 5813 | 2412 | 1445 | 250 | 340 | 13 | 331 | 0 | 1.8e-05 | 0.017 | 0.081 |
| 200 | 9917 | 4798 | 1986 | 554 | 1034 | 696 | 947 | 0 | 0 | 0.017 | 0.066 |
| 250 | 23617 | 8818 | 2874 | 1130 | 23143 | 19860 | 17265 | 9 | 1.8e-05 | 0.019 | 0.074 |
| 300 | 20876 | 10539 | 3214 | 733 | 28680 | 27299 | 22078 | 2 | 8.9e-06 | 0.018 | 0.074 |

**Parts, shelter, tools**

| year | parts | parts_held | structures | enclosed | afloat | inside_cold | inside_warm | take_held | take_bare | take_held_yield | take_bare_yield |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 43 | 0 | 0.176 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 54 | 0 | 0.172 |
| 30 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 8 | 63449 | 0.327 | 0.182 |
| 50 | 5 | 4 | 0 | 2 | 0 | 0 | 0 | 40 | 1.33e+05 | 0.227 | 0.147 |
| 75 | 4 | 1 | 0 | 1 | 0 | 0 | 0 | 7 | 1.99e+05 | 0.052 | 0.098 |
| 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 3.06e+05 | 0.114 | 0.080 |
| 150 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3.97e+05 | 0 | 0.065 |
| 200 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4.33e+05 | 0 | 0.058 |
| 250 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3.42e+05 | 0 | 0.061 |
| 300 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3.55e+05 | 0 | 0.066 |

**Speed (ms per simulated year, mean of the 10 years before; logs print every 10 years, so 75/150/250 are not available)**

| year | flow | change | bodies | parts | total | bodies in world |
|---|---|---|---|---|---|---|
| 10 | 388 | 43 | 254 | 0.0 | 685 | 1063 |
| 20 | 459 | 58 | 324 | 0.0 | 841 | 1695 |
| 30 | 595 | 132 | 1712 | 0.4 | 2439 | 8547 |
| 50 | 671 | 190 | 4912 | 0.9 | 5774 | 13367 |
| 100 | 686 | 202 | 16035 | 0.0 | 16923 | 23239 |
| 200 | 820 | 237 | 19240 | 0.0 | 20297 | 22952 |
| 300 | 796 | 234 | 20927 | 0.0 | 21957 | 23767 |

### Seed 5
**Climate**

| year | t_ground | t_air | absorber | air_0 x1e-3 | air_3 x1e-3 | air_4 x1e-6 | ice_frac | liquid_land | rain x1e-4 |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 288.4 | 256.1 | 1.59 | 4.53 | 0.302 | 3.92 | 0 | 6.15 | 7.80 |
| 20 | 290.3 | 258.3 | 1.89 | 5.07 | 0.222 | 11.1 | 0 | 12.3 | 9.15 |
| 30 | 291.9 | 260.2 | 2.40 | 5.56 | 0.297 | 19.6 | 6.1e-05 | 18.6 | 10.8 |
| 50 | 293.0 | 261.5 | 2.80 | 5.96 | 0.324 | 27.3 | 0 | 29.6 | 12.4 |
| 75 | 293.5 | 262.2 | 3.17 | 6.15 | 0.302 | 36.9 | 0 | 36.1 | 13.1 |
| 100 | 294.4 | 263.3 | 3.67 | 6.49 | 0.334 | 47.9 | 0 | 41.7 | 13.9 |
| 150 | 295.3 | 264.6 | 6.09 | 6.85 | 1.06 | 91.7 | 6.1e-05 | 56.1 | 15.0 |
| 200 | 295.5 | 264.9 | 8.33 | 6.93 | 1.53 | 140 | 6.1e-05 | 69.9 | 15.2 |
| 250 | 295.5 | 264.9 | 11.3 | 6.93 | 2.50 | 195 | 6.1e-05 | 82.5 | 15.2 |
| 300 | 295.6 | 265.1 | 11.1 | 6.95 | 3.24 | 167 | 1.2e-04 | 83.4 | 15.2 |

**Life**

| year | plants | plant_members | animals | animal_nodes | animal_single | animal_members | cover_median | cover_max | kinds | lineages |
|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 1196 | 6.22e+10 | 0 | 0 | 0 | 0 | 0 | 231 | 1 | 1 |
| 20 | 1748 | 8.61e+10 | 0 | 0 | 0 | 0 | 0 | 217 | 1 | 1 |
| 30 | 3425 | 4.57e+10 | 5942 | 3464 | 2478 | 2.30e+08 | 0.010 | 115 | 2 | 12 |
| 50 | 3543 | 2.57e+10 | 9217 | 4219 | 4998 | 1.50e+08 | 0.002 | 167 | 2 | 54 |
| 75 | 3679 | 5.03e+10 | 8746 | 5598 | 3148 | 3.13e+08 | 0.009 | 161 | 2 | 64 |
| 100 | 4193 | 4.67e+10 | 11089 | 10946 | 143 | 4.30e+09 | 0.348 | 125 | 2 | 20 |
| 150 | 4226 | 1.00e+11 | 11315 | 11138 | 177 | 1.48e+10 | 2.30 | 90.1 | 2 | 15 |
| 200 | 4292 | 8.78e+10 | 11320 | 11174 | 146 | 5.47e+10 | 2.74 | 121 | 2 | 16 |
| 250 | 4333 | 1.33e+11 | 12352 | 12156 | 196 | 2.42e+11 | 3.40 | 107 | 3 | 14 |
| 300 | 4416 | 1.27e+11 | 13023 | 12824 | 199 | 3.27e+12 | 3.32 | 94.0 | 3 | 12 |

**Sizes and feeding**

| year | mass_min | mass_median | mass_max | trophic_max | trophic_high | eat_animals | ate_light_eaters | ate_others | ate_ground |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 0.364 | 1.43 | 5.87 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0.069 | 1.58 | 10.5 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 0.022 | 39.2 | 523 | 0.004 | 0.083 | 986 | 9.68e+09 | 4.51e+07 | 1.08e+10 |
| 50 | 0.004 | 47.4 | 667 | 3.1e-05 | 0.871 | 1291 | 6.23e+09 | 4.14e+05 | 7.86e+09 |
| 75 | 0.006 | 27.5 | 846 | 0.015 | 0.369 | 888 | 4.54e+09 | 1.19e+08 | 1.78e+10 |
| 100 | 0.078 | 11.1 | 5019 | 0.020 | 0.957 | 1193 | 2.43e+10 | 6.98e+08 | 3.65e+10 |
| 150 | 0.125 | 4.57 | 3552 | 0.024 | 0.024 | 747 | 7.91e+10 | 2.64e+09 | 6.90e+10 |
| 200 | 0.109 | 1.79 | 2678 | 0.034 | 0.051 | 626 | 1.01e+11 | 4.52e+09 | 7.45e+10 |
| 250 | 0.015 | 0.869 | 1250 | 0.050 | 0.050 | 857 | 8.77e+10 | 4.59e+09 | 9.95e+10 |
| 300 | 0.001 | 0.427 | 302 | 0.103 | 0.103 | 1211 | 6.80e+10 | 4.72e+09 | 1.33e+11 |

**Animal deaths per year (members, by cause)**

| year | a_lack | a_phase | a_spent | a_taken | a_broken | births_animals | died_animals |
|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 2.29e+08 | 5.18e+06 | 3.14e+08 | 5.54e+07 | 2.45e+07 | 6.45e+08 | 6.29e+08 |
| 50 | 1.38e+08 | 4.52e+06 | 2.38e+08 | 4.07e+07 | 1.47e+07 | 4.44e+08 | 4.37e+08 |
| 75 | 4.38e+08 | 2.96e+07 | 4.84e+08 | 1.57e+08 | 3.46e+07 | 1.19e+09 | 1.14e+09 |
| 100 | 1.38e+09 | 6.42e+07 | 1.77e+10 | 1.86e+09 | 8.85e+08 | 2.30e+10 | 2.18e+10 |
| 150 | 5.22e+09 | 7.37e+08 | 9.84e+10 | 7.93e+09 | 8.97e+09 | 1.21e+11 | 1.21e+11 |
| 200 | 1.40e+10 | 6.82e+08 | 3.29e+11 | 4.78e+10 | 3.19e+10 | 4.24e+11 | 4.23e+11 |
| 250 | 2.42e+10 | 9.80e+09 | 1.65e+12 | 2.14e+11 | 2.15e+11 | 2.15e+12 | 2.11e+12 |
| 300 | 2.30e+11 | 6.48e+10 | 2.36e+13 | 4.60e+12 | 2.40e+12 | 3.13e+13 | 3.09e+13 |

**Learning and brains**

| year | learned | greedy | hidden | memory | seen | sight | hidden_max |
|---|---|---|---|---|---|---|---|
| 10 | 0.015 | 0.159 | 12 | 6 | 3 | 1 | 12 |
| 20 | 0.017 | 0.153 | 12 | 6 | 3 | 1 | 12 |
| 30 | 0.028 | 0.599 | 12 | 6 | 3 | 1 | 12 |
| 50 | 0.079 | 0.747 | 12 | 6 | 3 | 1 | 12 |
| 75 | 0.117 | 0.854 | 12.0 | 6 | 3 | 1 | 13 |
| 100 | 0.128 | 0.912 | 12 | 6 | 3 | 1 | 12 |
| 150 | 0.087 | 0.925 | 12 | 6 | 3 | 1 | 12 |
| 200 | 0.071 | 0.914 | 12 | 6 | 3 | 1 | 12 |
| 250 | 0.063 | 0.883 | 12 | 6 | 3 | 1 | 12 |
| 300 | 0.065 | 0.840 | 12 | 6 | 3 | 1 | 12 |

**Trails**

| year | worn_edges | worn_persist | traffic_top2 | traffic_persist | crossings | mass_top2 | mass_land |
|---|---|---|---|---|---|---|---|
| 10 | 1600 | nan | 0.413 | nan | 2.55e+11 | 25.0 | 3.40 |
| 20 | 2996 | 0.233 | 0.363 | 0.177 | 3.02e+11 | 26.2 | 6.19 |
| 30 | 3585 | 0.221 | 0.408 | 0.141 | 1.52e+11 | 13.2 | 4.20 |
| 50 | 1837 | 0.133 | 0.556 | 0.141 | 6.87e+10 | 7.64 | 2.45 |
| 75 | 2870 | 0.196 | 0.474 | 0.168 | 1.78e+11 | 16.0 | 3.58 |
| 100 | 4288 | 0.283 | 0.396 | 0.199 | 1.85e+11 | 11.5 | 5.47 |
| 150 | 7699 | 0.572 | 0.277 | 0.153 | 3.96e+11 | 18.6 | 11.3 |
| 200 | 8102 | 0.586 | 0.296 | 0.159 | 3.35e+11 | 16.5 | 11.0 |
| 250 | 11037 | 0.757 | 0.236 | 0.101 | 8.19e+11 | 22.2 | 13.8 |
| 300 | 12479 | 0.818 | 0.383 | 0.073 | 1.31e+12 | 32.1 | 14.6 |

**Imitation, signals, giving**

| year | watched | watched_kin | watched_groups | spread_max | gave | gave_eaters | gave_pairs | gave_reciprocal | mi_cold | mi_hurt | mi_low |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 3758 | 3758 | 495 | 17 | 17 | 0 | 17 | 0 | 1.4e-04 | 2.0e-04 | 0.090 |
| 20 | 5953 | 5953 | 622 | 17 | 26 | 0 | 26 | 0 | 1.7e-04 | 4.9e-04 | 0.053 |
| 30 | 9376 | 3890 | 1590 | 21 | 1362 | 227 | 1328 | 12 | 7.1e-04 | 0.032 | 0.033 |
| 50 | 5458 | 1628 | 771 | 47 | 1133 | 28 | 1101 | 0 | 1.4e-04 | 0.033 | 0.047 |
| 75 | 4329 | 1678 | 811 | 67 | 662 | 166 | 571 | 2 | 1.8e-04 | 0.024 | 0.089 |
| 100 | 2523 | 875 | 526 | 191 | 178 | 23 | 176 | 0 | 1.2e-04 | 0.036 | 0.134 |
| 150 | 2636 | 1538 | 590 | 127 | 495 | 461 | 244 | 0 | 2.8e-04 | 0.035 | 0.161 |
| 200 | 3044 | 1936 | 671 | 123 | 478 | 439 | 253 | 0 | 6.3e-05 | 0.041 | 0.152 |
| 250 | 4398 | 2746 | 966 | 186 | 83 | 58 | 83 | 0 | 1.2e-04 | 0.037 | 0.158 |
| 300 | 6850 | 3837 | 1286 | 238 | 227 | 185 | 222 | 0 | 1.3e-04 | 0.042 | 0.135 |

**Parts, shelter, tools**

| year | parts | parts_held | structures | enclosed | afloat | inside_cold | inside_warm | take_held | take_bare | take_held_yield | take_bare_yield |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 21 | 0 | 0.120 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 32 | 0 | 0.251 |
| 30 | 11 | 8 | 0 | 5 | 1 | 0 | 0 | 88 | 1.19e+05 | 0.247 | 0.193 |
| 50 | 9 | 8 | 0 | 5 | 0 | 0 | 2.7e-05 | 140 | 1.80e+05 | 0.167 | 0.142 |
| 75 | 11 | 8 | 0 | 6 | 0 | 0 | 3.8e-05 | 56 | 1.52e+05 | 0.127 | 0.099 |
| 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1.68e+05 | 0 | 0.082 |
| 150 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1.87e+05 | 0 | 0.062 |
| 200 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.26e+05 | 0 | 0.058 |
| 250 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1.87e+05 | 0 | 0.054 |
| 300 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.22e+05 | 0 | 0.058 |

**Speed (ms per simulated year, mean of the 10 years before; logs print every 10 years, so 75/150/250 are not available)**

| year | flow | change | bodies | parts | total | bodies in world |
|---|---|---|---|---|---|---|
| 10 | 388 | 43 | 278 | 0.0 | 709 | 1196 |
| 20 | 476 | 65 | 365 | 0.0 | 906 | 1748 |
| 30 | 604 | 165 | 2278 | 1.7 | 3049 | 9367 |
| 50 | 679 | 206 | 5606 | 3.5 | 6494 | 12760 |
| 100 | 663 | 197 | 9347 | 0.0 | 10207 | 15282 |
| 200 | 784 | 232 | 11370 | 0.0 | 12386 | 15612 |
| 300 | 810 | 237 | 13634 | 0.0 | 14681 | 17439 |

### Seed 7 
**Climate**

| year | t_ground | t_air | absorber | air_0 x1e-3 | air_3 x1e-3 | air_4 x1e-6 | ice_frac | liquid_land | rain x1e-4 |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 287.7 | 255.5 | 1.52 | 4.41 | 0.308 | 3.32 | 7.9e-04 | 9.34 | 7.58 |
| 20 | 289.5 | 257.6 | 1.78 | 4.90 | 0.203 | 10.4 | 3.7e-04 | 16.8 | 8.75 |
| 30 | 291.0 | 259.3 | 2.28 | 5.35 | 0.281 | 19.1 | 0 | 22.5 | 10.3 |
| 50 | 292.5 | 261.1 | 2.76 | 5.83 | 0.308 | 28.7 | 1.8e-04 | 33.6 | 12.0 |
| 75 | 292.7 | 261.3 | 3.07 | 5.90 | 0.318 | 36.8 | 6.1e-05 | 47.2 | 12.5 |
| 100 | 293.6 | 262.4 | 3.46 | 6.21 | 0.304 | 46.2 | 0 | 58.8 | 13.1 |
| 150 | 294.5 | 263.6 | 4.64 | 6.53 | 0.313 | 77.3 | 0 | 79.9 | 14.0 |
| 200 | 295.0 | 264.3 | 7.81 | 6.74 | 1.14 | 141 | 6.1e-05 | 81.9 | 14.5 |
| 250 | 295.0 | 264.4 | 9.53 | 6.77 | 1.14 | 190 | 0 | 80.9 | 14.5 |
| 300 | 295.1 | 264.4 | 10.8 | 6.78 | 1.40 | 217 | 0 | 79.1 | 14.5 |

**Life**

| year | plants | plant_members | animals | animal_nodes | animal_single | animal_members | cover_median | cover_max | kinds | lineages |
|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 1305 | 6.26e+10 | 0 | 0 | 0 | 0 | 0 | 175 | 1 | 1 |
| 20 | 1739 | 8.38e+10 | 0 | 0 | 0 | 0 | 0 | 189 | 1 | 1 |
| 30 | 3359 | 4.39e+10 | 5113 | 3345 | 1768 | 2.13e+08 | 5.0e-04 | 129 | 2 | 12 |
| 50 | 3598 | 2.96e+10 | 9097 | 4319 | 4778 | 1.66e+08 | 5.4e-04 | 182 | 2 | 39 |
| 75 | 2657 | 9.47e+09 | 9987 | 5964 | 4023 | 1.57e+08 | 0 | 95.6 | 2 | 72 |
| 100 | 4054 | 2.41e+10 | 12229 | 11729 | 500 | 1.27e+09 | 0.231 | 121 | 2 | 26 |
| 150 | 4494 | 4.39e+10 | 14412 | 14187 | 225 | 4.45e+09 | 0.573 | 98.7 | 2 | 11 |
| 200 | 5027 | 1.06e+11 | 13075 | 13075 | 0 | 1.06e+10 | 2.27 | 89.2 | 2 | 8 |
| 250 | 5275 | 1.47e+11 | 13015 | 13015 | 0 | 1.66e+10 | 2.98 | 58.4 | 2 | 8 |
| 300 | 5292 | 2.39e+11 | 12449 | 12449 | 0 | 3.14e+10 | 3.08 | 71.3 | 2 | 8 |

**Sizes and feeding**

| year | mass_min | mass_median | mass_max | trophic_max | trophic_high | eat_animals | ate_light_eaters | ate_others | ate_ground |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 0.101 | 1.58 | 9.40 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0.081 | 1.60 | 15.6 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 0.028 | 34.8 | 1794 | 0.012 | 0.012 | 665 | 5.52e+09 | 7.28e+07 | 1.37e+10 |
| 50 | 0.004 | 44.0 | 1303 | 4.5e-05 | 0.019 | 954 | 5.71e+09 | 2.51e+05 | 1.02e+10 |
| 75 | 0.117 | 32.2 | 663 | 0.005 | 0.998 | 1904 | 2.83e+09 | 5.17e+07 | 6.28e+09 |
| 100 | 0.009 | 18.8 | 988 | 0.061 | 0.061 | 2797 | 1.70e+10 | 3.04e+09 | 2.51e+10 |
| 150 | 0.005 | 11.7 | 3.64e+05 | 0.030 | 0.030 | 2363 | 3.56e+10 | 3.79e+09 | 5.27e+10 |
| 200 | 0.043 | 7.62 | 1.58e+05 | 0.070 | 0.070 | 1566 | 7.10e+10 | 9.70e+09 | 1.03e+11 |
| 250 | 0.060 | 4.19 | 2.12e+05 | 0.062 | 0.062 | 1471 | 7.35e+10 | 1.03e+10 | 1.06e+11 |
| 300 | 0.077 | 2.51 | 7523 | 0.089 | 0.089 | 1607 | 6.37e+10 | 1.22e+10 | 1.27e+11 |

**Animal deaths per year (members, by cause)**

| year | a_lack | a_phase | a_spent | a_taken | a_broken | births_animals | died_animals |
|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 30 | 2.11e+08 | 7.36e+06 | 3.05e+08 | 4.02e+07 | 2.10e+07 | 6.02e+08 | 5.84e+08 |
| 50 | 1.31e+08 | 8.94e+06 | 3.19e+08 | 3.19e+07 | 2.00e+07 | 5.09e+08 | 5.11e+08 |
| 75 | 2.99e+08 | 8.09e+05 | 7.16e+07 | 4.55e+07 | 1.00e+07 | 4.57e+08 | 4.27e+08 |
| 100 | 8.74e+08 | 5.78e+06 | 5.63e+09 | 4.02e+09 | 2.95e+08 | 1.08e+10 | 1.08e+10 |
| 150 | 1.28e+08 | 7.04e+06 | 2.96e+10 | 1.20e+10 | 2.57e+09 | 4.44e+10 | 4.43e+10 |
| 200 | 8.93e+07 | 1.54e+08 | 9.24e+10 | 2.52e+10 | 1.30e+10 | 1.31e+11 | 1.31e+11 |
| 250 | 8.56e+07 | 1.95e+08 | 1.59e+11 | 5.63e+10 | 2.28e+10 | 2.38e+11 | 2.39e+11 |
| 300 | 4.61e+08 | 4.04e+08 | 2.40e+11 | 1.09e+11 | 4.88e+10 | 3.99e+11 | 3.99e+11 |

**Learning and brains**

| year | learned | greedy | hidden | memory | seen | sight | hidden_max |
|---|---|---|---|---|---|---|---|
| 10 | 0.010 | 0.101 | 12 | 6 | 3 | 1 | 12 |
| 20 | 0.009 | 0.112 | 12 | 6 | 3 | 1 | 12 |
| 30 | 0.024 | 0.625 | 12 | 6 | 3 | 1 | 12 |
| 50 | 0.088 | 0.809 | 12.0 | 6 | 3 | 1 | 13 |
| 75 | 0.106 | 0.746 | 12.0 | 6 | 3 | 1 | 13 |
| 100 | 0.111 | 0.822 | 11.8 | 6 | 3 | 1 | 13 |
| 150 | 0.123 | 0.829 | 11.7 | 6 | 3 | 1 | 13 |
| 200 | 0.101 | 0.735 | 11.7 | 6 | 3 | 1 | 12 |
| 250 | 0.097 | 0.719 | 11.7 | 6 | 3 | 1 | 12 |
| 300 | 0.081 | 0.689 | 11.6 | 6 | 3 | 1 | 12 |

**Trails**

| year | worn_edges | worn_persist | traffic_top2 | traffic_persist | crossings | mass_top2 | mass_land |
|---|---|---|---|---|---|---|---|
| 10 | 2210 | nan | 0.362 | nan | 3.86e+11 | 17.8 | 4.16 |
| 20 | 3449 | 0.273 | 0.299 | 0.162 | 4.84e+11 | 19.0 | 5.62 |
| 30 | 3329 | 0.222 | 0.376 | 0.110 | 2.16e+11 | 9.82 | 3.79 |
| 50 | 2195 | 0.143 | 0.515 | 0.107 | 1.27e+11 | 13.4 | 2.22 |
| 75 | 743 | 0.081 | 0.630 | 0.190 | 5.78e+10 | 5.40 | 0.778 |
| 100 | 3274 | 0.293 | 0.419 | 0.232 | 9.93e+10 | 9.40 | 2.91 |
| 150 | 6985 | 0.624 | 0.285 | 0.263 | 2.63e+11 | 12.6 | 5.53 |
| 200 | 13184 | 0.844 | 0.194 | 0.324 | 1.81e+12 | 16.5 | 10.6 |
| 250 | 13562 | 0.832 | 0.169 | 0.266 | 3.85e+12 | 15.3 | 10.2 |
| 300 | 14058 | 0.840 | 0.156 | 0.284 | 7.35e+12 | 16.0 | 11.8 |

**Imitation, signals, giving**

| year | watched | watched_kin | watched_groups | spread_max | gave | gave_eaters | gave_pairs | gave_reciprocal | mi_cold | mi_hurt | mi_low |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 1600 | 1600 | 479 | 10 | 30 | 0 | 30 | 0 | 5.9e-04 | 2.5e-04 | 0.038 |
| 20 | 2363 | 2363 | 639 | 9 | 32 | 0 | 32 | 0 | 3.2e-04 | 1.4e-04 | 0.012 |
| 30 | 4711 | 2142 | 1339 | 19 | 1179 | 125 | 1148 | 4 | 3.3e-04 | 0.019 | 0.030 |
| 50 | 2231 | 915 | 572 | 63 | 1673 | 22 | 1555 | 4 | 1.9e-04 | 0.016 | 0.047 |
| 75 | 1209 | 536 | 490 | 17 | 9142 | 64 | 6956 | 12 | 2.4e-04 | 0.014 | 0.087 |
| 100 | 2105 | 947 | 684 | 60 | 10039 | 94 | 7332 | 11 | 2.9e-05 | 0.029 | 0.110 |
| 150 | 3776 | 1943 | 972 | 129 | 9717 | 62 | 7280 | 2 | 1.2e-05 | 0.033 | 0.138 |
| 200 | 16327 | 6531 | 2009 | 118 | 6434 | 2596 | 5451 | 14 | 1.9e-05 | 0.026 | 0.111 |
| 250 | 10954 | 8221 | 2219 | 108 | 2910 | 499 | 2549 | 5 | 1.4e-05 | 0.024 | 0.100 |
| 300 | 10866 | 7695 | 2375 | 116 | 3543 | 1819 | 3266 | 4 | 1.8e-05 | 0.027 | 0.100 |

**Parts, shelter, tools**

| year | parts | parts_held | structures | enclosed | afloat | inside_cold | inside_warm | take_held | take_bare | take_held_yield | take_bare_yield |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 24 | 0 | 0.110 |
| 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 28 | 0 | 0.050 |
| 30 | 4 | 2 | 0 | 1 | 0 | 0 | 0 | 21 | 88928 | 0.232 | 0.185 |
| 50 | 9 | 7 | 0 | 2 | 0 | 0 | 0 | 65 | 1.44e+05 | 0.153 | 0.130 |
| 75 | 7 | 6 | 0 | 6 | 0 | 0 | 2.5e-05 | 42 | 1.77e+05 | 0.145 | 0.084 |
| 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.85e+05 | 0 | 0.082 |
| 150 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3.26e+05 | 0 | 0.062 |
| 200 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.44e+05 | 0 | 0.059 |
| 250 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.32e+05 | 0 | 0.059 |
| 300 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2.11e+05 | 0 | 0.060 |

**Speed (ms per simulated year, mean of the 10 years before; logs print every 10 years, so 75/150/250 are not available)**

| year | flow | change | bodies | parts | total | bodies in world |
|---|---|---|---|---|---|---|
| 10 | 418 | 47 | 275 | 0.0 | 740 | 1305 |
| 20 | 477 | 63 | 368 | 0.0 | 908 | 1739 |
| 30 | 641 | 149 | 1904 | 0.9 | 2695 | 8472 |
| 50 | 701 | 196 | 5217 | 1.6 | 6116 | 12695 |
| 100 | 676 | 182 | 10783 | 0.3 | 11641 | 16283 |
| 200 | 813 | 228 | 14777 | 0.0 | 15818 | 18102 |
| 300 | 810 | 230 | 15912 | 0.0 | 16952 | 17741 |

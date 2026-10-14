# Watch report 4: full numbers

Code: `main` at 444dabf. Big runs: `ew run --seed N --size 128 --years 300 --every 25 --frames-every 50` (seeds 1, 5, 7). Small runs: `--size 64 --years 150`, with `--null` and `--roster` as named. Columns as in `series.csv` (handover 8.3).

## 128 x 128, 300 years
```

s1
year  died_age  t_ground  absorber     air_3  ice_frac plant_mem animal_me    bodies cover_med  mass_max     kinds kinds_mem ate_light ate_groun ate_other     cat_0     cat_1     cat_5     cat_7    cat_11    cat_15    cat_20    hidden      gave   watched crossings
   1         0       286      1.26  0.000389         0         0         0         0         0       nan         0         0         0         0         0         0         0         0         0         0         0         0         0         0         0        -0
   5  6.22e+09       287      1.24  0.000292  0.000183  1.06e+11         0  2.92e+03         0      16.4         1         1         0         0         0      9.03      2.09         0         0         0         0         0        12       106   6.2e+03  5.23e+11
  10  6.42e+09       287      1.18  0.000165  0.000366  1.09e+11         0     3e+03         0      25.8         1         1         0         0         0      9.03      2.09         0         0         0         0         0        12        88  7.04e+03  6.28e+11
  20  4.37e+09       287      1.16  0.000127   6.1e-05  8.39e+10         0  2.77e+03         0      14.4         1         1         0         0         0      9.04      2.09         0         0         0         0         0        12       121  5.64e+03  4.43e+11
  30  7.78e+09       288      1.26  0.000237  0.000183  1.27e+11  2.69e+08  8.21e+03  1.26e-06       425         2         1   1.8e+09  3.97e+10  1.57e+06      9.02      2.08         0         0         0         0         0        12  1.08e+03  8.74e+03  6.31e+11
  50  7.16e+09       289      1.33  0.000275  0.000122  1.15e+11  2.34e+08   9.7e+03   0.00171       543         2         1     4e+09  3.04e+10  2.35e+06      9.01      2.08  5.32e-12         0         0         0         0        12   1.3e+03  9.32e+03  6.13e+11
  75  5.49e+09       290      1.36  0.000305         0  9.06e+10  2.77e+08   1.1e+04    0.0237       417         2         1  7.85e+09  2.64e+10  1.81e+04         9      2.07  9.98e-12         0         0         0         0        12  1.27e+03  9.75e+03  5.05e+11
 100   9.2e+09       290      1.38   0.00033  0.000122  7.54e+10  3.13e+10  1.73e+04    0.0196       554         2         2   7.3e+09  3.64e+10  1.01e+08      6.38      2.17  4.93e-12         0         0   1.6e-12         0      11.9  5.15e+03  1.97e+04  3.63e+11
 150   1.5e+10       291      1.38  0.000304  0.000244  9.67e+10   5.4e+10  1.95e+04    0.0769  2.54e+03         1         2   1.6e+10  4.51e+10  3.51e+08       5.8      2.22  3.23e-11  1.92e-12         0  2.62e-12  1.03e-11        12  1.76e+03  1.06e+04   5.6e+11
 200  1.21e+10       291      1.37  0.000298  0.000305  1.03e+11  4.29e+10  2.01e+04    0.0775  1.25e+04         1         2  1.68e+10  4.72e+10  6.33e+08      6.39      2.21  2.11e-11  8.91e-08         0  3.88e-14  3.21e-12        12  1.32e+03  1.08e+04  6.58e+11
 250  1.08e+10       291      1.38  0.000307  0.000244  9.16e+10  3.12e+10   2.1e+04     0.113  6.18e+04         1         2  2.09e+10  4.77e+10  1.71e+09      6.76      2.23  4.16e-11  8.83e-12  3.17e-12  1.03e-11         0      11.9  3.58e+03  1.36e+04  7.67e+11
 300  1.17e+10       291      1.37    0.0003   6.1e-05  9.26e+10  1.62e+10  2.15e+04    0.0851  3.42e+04         2         2  2.47e+10  4.81e+10   3.3e+09      7.73      2.23  4.67e-10  4.61e-12         0  1.67e-11  6.11e-12      11.9  6.27e+03  1.54e+04  9.91e+11

s5
year  died_age  t_ground  absorber     air_3  ice_frac plant_mem animal_me    bodies cover_med  mass_max     kinds kinds_mem ate_light ate_groun ate_other     cat_0     cat_1     cat_5     cat_7    cat_11    cat_15    cat_20    hidden      gave   watched crossings
   1         0       287      1.32  0.000387         0         0         0         0         0       nan         0         0         0         0         0         0         0         0         0         0         0         0         0         0         0        -0
   5     1e+10       289       1.3  0.000275         0  1.68e+11         0  2.63e+03    0.0122      11.8         1         1         0         0         0      9.01      2.06  6.67e-07         0         0         0         0        12        41  8.44e+03  3.87e+11
  10  8.68e+09       290      1.26  0.000164         0  1.45e+11         0  2.37e+03         0      14.7         1         1         0         0         0      9.02      2.06         0         0         0         0         0        12        29  7.21e+03  4.09e+11
  20  8.54e+09       290      1.27  0.000144         0   1.5e+11         0  2.37e+03         0      17.6         1         1         0         0         0      9.01      2.07         0         0         0         0         0        12        30  7.02e+03   3.9e+11
  30  1.07e+10       291      1.38  0.000277  0.000122  1.73e+11  2.79e+08  6.79e+03      0.25       325         2         1  4.73e+09  3.84e+10  5.99e+06         9      2.07         0         0         0         0         0        12       775  9.06e+03  4.81e+11
  50  4.82e+09       292      1.43  0.000318         0  8.12e+10  1.74e+08  7.74e+03    0.0704       381         2         1  8.25e+09  1.66e+10  9.01e+05      8.99      2.06         0         0         0         0         0        12       856  1.02e+04  2.26e+11
  75  2.87e+09       292      1.44  0.000338         0  5.72e+10  1.73e+08  7.84e+03    0.0655       282         2         1  1.06e+10   8.9e+09  9.88e+06      8.97      2.07  1.98e-12         0         0         0         0        12       966  9.95e+03  1.45e+11
 100  5.98e+09       292      1.45  0.000325   6.1e-05  1.06e+11  3.86e+08  9.15e+03     0.563       406         2         1  2.67e+10  2.27e+10  2.42e+07      8.98      2.18         0         0  2.31e-12         0         0        12  1.13e+03  1.34e+04  2.64e+11
 150  3.95e+10       293      1.47  0.000293   6.1e-05  3.33e+11  6.14e+08  1.22e+04      10.3       293         2         1  3.39e+10  3.74e+10  1.42e+08      8.89      2.33  2.45e-13         0  1.55e-12  2.38e-12  8.99e-15        12  1.53e+03  2.28e+04  9.09e+11
 200  1.12e+11       293      1.48  0.000301  0.000122  6.01e+11  1.21e+10  1.29e+04      1.51       513         2         2  2.86e+10  2.31e+10  1.35e+09      8.67      2.37  1.34e-13         0  1.04e-13         0  5.04e-13        12   6.6e+03   1.8e+04  1.33e+12
 250  1.36e+11       293      1.47  0.000314   6.1e-05  5.11e+11   2.8e+10  1.39e+04     0.805  6.74e+03         2         2  3.21e+10  1.95e+10   1.2e+09      8.44      2.45  1.44e-12  3.78e-13         0         0  1.17e-14        12  3.96e+03  1.51e+04  1.47e+12
 300  1.57e+11       293      1.46  0.000314         0  5.15e+11  1.91e+10  1.45e+04     0.744  2.66e+04         2         2  2.86e+10  2.21e+10  2.02e+09      8.59      2.46   1.7e-12   1.8e-12         0         0     6e-14        12  2.84e+03   1.5e+04  1.59e+12

s7
year  died_age  t_ground  absorber     air_3  ice_frac plant_mem animal_me    bodies cover_med  mass_max     kinds kinds_mem ate_light ate_groun ate_other     cat_0     cat_1     cat_5     cat_7    cat_11    cat_15    cat_20    hidden      gave   watched crossings
   1         0       287       1.3  0.000389   0.00183         0         0         0         0       nan         0         0         0         0         0         0         0         0         0         0         0         0         0         0         0        -0
   5  6.68e+09       288       1.3  0.000304  0.000183  1.21e+11         0  2.45e+03         0      12.5         1         1         0         0         0      9.01      2.11         0         0         0         0         0        12        55  4.34e+03  5.33e+11
  10  6.85e+09       289      1.26  0.000197         0  1.12e+11         0  2.13e+03         0      13.3         1         1         0         0         0      9.01      2.11         0         0         0         0         0        12        42  3.98e+03  6.02e+11
  20  5.97e+09       289      1.25  0.000165   6.1e-05  1.03e+11         0  1.98e+03         0      10.8         1         1         0         0         0      9.01      2.11         0         0         0         0         0        12        51  3.58e+03  5.33e+11
  30  7.77e+09       290      1.34  0.000267  0.000122  1.34e+11  2.17e+08   6.7e+03         0       471         2         1   1.6e+09  2.56e+10  2.57e+06      8.98      2.11         0  1.98e-07         0         0         0        12       855  5.13e+03  6.39e+11
  50   8.3e+09       291      1.37  0.000279         0  1.43e+11  1.73e+08   8.3e+03   1.7e-06       622         2         1   9.8e+08  2.66e+10  1.54e+06      8.98      2.11         0         0         0         0         0        12  1.04e+03  4.55e+03  7.17e+11
  75  8.92e+09       291      1.38  0.000285  0.000122  1.52e+11  1.88e+08  8.96e+03  4.33e-05       694         2         1  1.02e+09  2.76e+10   2.4e+04      8.97      2.12         0  3.89e-13         0         0         0        12       812  4.39e+03  7.83e+11
 100  7.52e+09       291       1.4  0.000301   6.1e-05  1.31e+11  2.45e+08  9.06e+03   0.00166       728         2         1  2.29e+09  2.52e+10   3.9e+06      8.96      2.13         0         0         0         0  1.74e-12        12  1.12e+03  4.74e+03  6.72e+11
 150  8.45e+09       291      1.41  0.000317         0  9.27e+10  1.62e+10  1.48e+04    0.0286  1.57e+03         3         2  4.07e+09  2.78e+10  1.38e+08      7.65      2.17  1.72e-11  8.48e-12     6e-12         0  6.82e-12        12  2.39e+03  5.23e+03  5.14e+11
 200  7.74e+09       291      1.41  0.000321         0  9.37e+10  1.12e+10  1.52e+04    0.0278  6.03e+03         3         2  5.45e+09  2.61e+10  1.04e+08      8.03      2.16  3.37e-08   1.1e-11  9.08e-08         0     1e-11        12   2.6e+03  6.14e+03  4.98e+11
 250  1.93e+10       291      1.41   0.00031   6.1e-05  1.74e+11  1.04e+10  1.54e+04    0.0918  1.32e+04         3         2  8.11e+09  3.33e+10  2.53e+07      8.48      2.23  3.42e-09  9.56e-13   5.3e-10  1.74e-12  4.95e-12        12  3.21e+03  6.85e+03  1.13e+12
 300  1.35e+10       291      1.41  0.000317         0  1.05e+11  8.32e+09  1.62e+04    0.0873  2.56e+04         3         2  1.32e+10  2.66e+10  1.39e+07      8.34      2.22  3.81e-09  2.07e-11  5.04e-08  1.06e-11   2.3e-08        12  3.51e+03  7.13e+03  7.65e+11
```

## Real runs against null runs, 64 x 64
```

### seed 1 (64x64): real / null, mean of years 101-150 (and 51-100)
died_age             5.77e+09 /   2.47e+09   (4.5e+09 / 2.54e+09)
animal_members       5.86e+09 /          0   (1.05e+08 / 0)
plant_members        8.01e+10 /   5.22e+10   (7.42e+10 / 5.43e+10)
kinds                     2.5 /        2.1   (2 / 1.46)
kinds_members            1.64 /        2.1   (1 / 1.78)
gave                      183 /        9.4   (111 / 9.02)
gave_kg                   113 /       1.07   (64.7 / 1.24)
gave_reciprocal          0.28 /       0.06   (0.04 / 0.04)
watched               2.3e+03 /   2.63e+03   (2.23e+03 / 2.73e+03)
watched_groups            265 /        287   (285 / 285)
crossings            3.29e+11 /    9.4e+10   (2.98e+11 / 9.93e+10)
traffic_top2            0.184 /      0.268   (0.189 / 0.262)
traffic_persist         0.115 /      0.114   (0.0793 / 0.0731)
worn_edges           3.21e+03 /   1.26e+03   (3.06e+03 / 1.3e+03)
learned                 0.447 /     0.0477   (0.293 / 0.0492)
greedy                  0.934 /      0.171   (0.885 / 0.172)
inside_cold                 0 /          0   (0 / 0)
inside_warm                 0 /          0   (0 / 0)
parts                       0 /          0   (0 / 0)
structures                  0 /          0   (0 / 0)
ate_light_eaters      1.9e-05 /          0   (0 / 0)
ate_ground           1.69e+10 /          0   (1.46e+10 / 0)
ate_others           1.39e+05 /          0   (0 / 0)
mass_max                  815 /       4.12   (334 / 6.05)
hidden                     12 /         12   (12 / 12)

### seed 5 (64x64): real / null, mean of years 101-150 (and 51-100)
died_age             4.71e+09 /    4.5e+09   (4.1e+09 / 4.31e+09)
animal_members        8.7e+07 /          0   (6.77e+07 / 0)
plant_members        8.93e+10 /   7.64e+10   (7.95e+10 / 7.42e+10)
kinds                       2 /       2.38   (2 / 2.7)
kinds_members               1 /          2   (1 / 2.22)
gave                      101 /       3.04   (56.2 / 3.56)
gave_kg                   143 /      0.281   (50.7 / 0.387)
gave_reciprocal          0.74 /          0   (0.08 / 0)
watched              1.86e+03 /   2.64e+03   (1.82e+03 / 2.61e+03)
watched_groups            215 /        213   (226 / 212)
crossings             1.2e+11 /   1.04e+11   (1.16e+11 / 1.05e+11)
traffic_top2            0.289 /      0.291   (0.296 / 0.297)
traffic_persist         0.127 /      0.107   (0.14 / 0.0963)
worn_edges           2.14e+03 /   1.48e+03   (2.04e+03 / 1.46e+03)
learned                 0.318 /     0.0337   (0.226 / 0.0343)
greedy                  0.902 /      0.152   (0.855 / 0.153)
inside_cold                 0 /          0   (0 / 0)
inside_warm          1.33e-06 /          0   (1.5e-06 / 0)
parts                    0.08 /          0   (0.54 / 0)
structures               0.02 /          0   (0 / 0)
ate_light_eaters     3.44e+08 /          0   (3.45e+08 / 0)
ate_ground            6.8e+09 /          0   (6.24e+09 / 0)
ate_others           5.18e+06 /          0   (1.48e+04 / 0)
mass_max                  210 /       7.54   (282 / 9.88)
hidden                     12 /         12   (12 / 12)

### seed 7 (64x64): real / null, mean of years 101-150 (and 51-100)
died_age             9.21e+08 /   4.58e+09   (1.79e+09 / 3.95e+09)
animal_members       9.59e+08 /   4.95e+07   (7.33e+07 / 3.78e+07)
plant_members        1.61e+10 /   8.52e+10   (3.33e+10 / 7.36e+10)
kinds                    2.36 /          2   (2 / 2)
kinds_members            1.72 /          1   (1 / 1)
gave                 1.73e+03 /        199   (669 / 231)
gave_kg              1.17e+03 /   1.76e+03   (409 / 2.57e+03)
gave_reciprocal          19.5 /       3.24   (0.5 / 3.36)
watched              1.28e+03 /   4.63e+03   (986 / 5.73e+03)
watched_groups            176 /        292   (195 / 325)
crossings            6.21e+10 /    8.1e+10   (1.35e+11 / 7.08e+10)
traffic_top2            0.402 /      0.315   (0.314 / 0.332)
traffic_persist         0.265 /     0.0953   (0.155 / 0.0978)
worn_edges           1.08e+03 /   2.01e+03   (1.65e+03 / 2.06e+03)
learned                 0.459 /      0.107   (0.307 / 0.082)
greedy                  0.864 /      0.586   (0.855 / 0.476)
inside_cold                 0 /   0.000204   (0 / 0)
inside_warm                 0 /   0.000115   (0 / 0.000301)
parts                    0.02 /        200   (0.48 / 265)
structures                  0 /       0.28   (0 / 1.28)
ate_light_eaters     2.04e+09 /   1.76e+08   (1.3e+09 / 4.2e+08)
ate_ground           4.18e+09 /   6.41e+09   (6.35e+09 / 4.96e+09)
ate_others           6.06e+07 /    3.1e+04   (1.98e+04 / 9.99e+04)
mass_max                  601 /        242   (198 / 255)
hidden                     12 /         12   (12 / 12)
```

## Roster runs, 64 x 64: survival, expected life and repair gene of each starting kind's descendants
```

k1 (seed 1): kinds at year 150 5 by bodies, 1 by members
   light-eaters                        last alive year 50; year 150: members 0, life nan y, repair nan; at placement: life 17.3 y
   eaters                              last alive year 150; year 150: members 3.4e+08, life 0.906 y, repair -1.85; at placement: life 17.7 y
   small fast light-eaters             last alive year 46; year 150: members 0, life nan y, repair nan; at placement: life 9.98 y
   big slow light-eaters               last alive year 150; year 150: members 2.05e+06, life 7.03 y, repair -1.12; at placement: life 35.5 y
   small eaters of remains             last alive year 150; year 150: members 1.45e+09, life 1.17 y, repair -1.83; at placement: life 0.765 y
   big eaters of remains               last alive year 150; year 150: members 2.3e+10, life 1.14 y, repair -1.83; at placement: life 2.45 y
   small fast eaters of light-eaters   last alive year 30; year 150: members 0, life nan y, repair nan; at placement: life 1.53 y
   big slow eaters of light-eaters     last alive year 48; year 150: members 0, life nan y, repair nan; at placement: life 37.7 y
   strong fast eaters of eaters        last alive year 29; year 150: members 0, life nan y, repair nan; at placement: life 15.8 y
   strong big slow eaters of eaters    last alive year 29; year 150: members 0, life nan y, repair nan; at placement: life 34 y

k5 (seed 5): kinds at year 150 1 by bodies, 1 by members
   light-eaters                        last alive year 150; year 150: members 8.69e+06, life 34.6 y, repair -0.736; at placement: life 18.5 y
   eaters                              last alive year 150; year 150: members 8.72e+09, life 2.01 y, repair -1.12; at placement: life 22.5 y
   small fast light-eaters             last alive year 10; year 150: members 0, life nan y, repair nan; at placement: life 8.38 y
   big slow light-eaters               last alive year 150; year 150: members 3.62e+08, life 13.9 y, repair -1.14; at placement: life 28.2 y
   small eaters of remains             last alive year 150; year 150: members 1.01e+10, life 1.77 y, repair -1.15; at placement: life 1.98 y
   big eaters of remains               last alive year 150; year 150: members 1.01e+09, life 1.99 y, repair -1.09; at placement: life 1.21 y
   small fast eaters of light-eaters   last alive year 109; year 150: members 0, life nan y, repair nan; at placement: life 0.915 y
   big slow eaters of light-eaters     last alive year 25; year 150: members 0, life nan y, repair nan; at placement: life 43.2 y
   strong fast eaters of eaters        last alive year 69; year 150: members 0, life nan y, repair nan; at placement: life 17.1 y
   strong big slow eaters of eaters    last alive year 150; year 150: members 4.23e+09, life 2.22 y, repair -1.16; at placement: life 58.3 y
```

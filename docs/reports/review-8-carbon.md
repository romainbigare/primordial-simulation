# Review 8: why the carbon gas kept falling

15 October 2026. Code: `main` at d143782 (after partial damage). Written by an agent with a snapshot
inspector, reaction-flow tallies and runs (64 x 64, seeds 1 to 3, up to 150 years). Saved by the
orchestrator. Patches: `review-8-carbon-litter.patch` (the proposed fix) and
`review-8-carbon-sea-prototype.patch` (a rough sea store, not for merging as is).

## Short answer

- **No carbon leaves the world.** The carbon ledger holds. Rock weathering takes about 10⁻⁸ of what
  plants move a year.
- **The carbon moves from the air into dead plant matter:** dead sugar and dead body matter, on
  land and more and more on the sea floor.
  - The air is the only free carbon store, because the sea holds only 4% of the air's carbon as
    dissolved gas.
  - So carbon locked in dead matter comes out of the air one for one.
- **It does not fall for ever.** On seed 3 it settles at 43 to 50 ppm from year 50 to year 150. Each
  new law lowered that level.
- **Each law moved 1 to 4 x 10¹¹ mol into dead matter.** 1 x 10¹¹ mol is 17 ppm.
  - Reproduction by surplus: 167 to 71 ppm.
  - The energy ceiling: 71 to 59 ppm.
  - Partial damage: 59 to 44 ppm.

## Carbon by store, seed 3 (10¹¹ mol of carbon)

| year | ppm | air | dead sugar (land + sea) | dead body matter (land + sea) | plants | animals |
|---|---|---|---|---|---|---|
| 0 | 402 | 23.2 | 0 | 0 | 0 | 0 |
| 10 | 188 | 10.8 | 6.4 | 3.6 | 2.1 | 0 |
| 30 | 85 | 4.9 | 10.1 | 6.6 | 1.6 | 0 |
| 60 | 44 | 2.5 | 12.7 (7.0 + 5.7) | 6.0 (3.1 + 3.0) | 1.7 | 0.3 |
| 150 | 43 | 2.5 | 12.2 | 5.7 | 2.3 | 0.6 |

At year 60:

- **Dead matter on the sea floor is 37% of all carbon.** Rivers carry about 0.6 x 10¹¹ mol a year
  there. Nothing eats it, and plain chemistry turns it over in about 12 years, against 3 on land.
- **Animals eat half of the dead matter on land,** but they hold only 1.4% of the carbon.

## Compared with Earth

| | island | Earth |
|---|---|---|
| air carbon per m² | 1.7 kg C | 1.7 kg C |
| plant production (NPP) a year, over the air's carbon | 7.1% | 6.9% |
| NPP per m² of land | 0.34 kg C a year | about 0.4 |
| dead carbon on land | 2.1 kg C/m² | 10 to 16 |
| land carbon over air carbon | 0.9 | about 2.9 |
| the ocean's carbon over the air's | 0.04 | about 44 |

The island behaves like Earth would with no ocean store.

## Causes, ranked

1. **The air is the only free carbon store.** Chemistry 0 has no carbonate reaction, so the sea
   cannot buffer the air (handover 6.1, known limit).
2. **Dead matter on the sea floor:** the largest pool with nothing to eat it.
3. **Dead matter on land waits for a thin group of eaters.** With the roster's eaters of remains the
   air is at 177 ppm at year 60, against 44.
4. **Dead plant matter decays at 5 to 7% a year in the open:** a figure for wood. Real leaf and
   forest-floor litter decays at about 25% a year (Olson 1963: litter about 0.25 a year, leaves 0.3
   to 1, wood 0.02 to 0.1).

## The proposed fix (tested)

- **Anchor open-world litter decay to Olson.**
  - The reactions that burn sugar (1) and body matter (21) get a `k` 4 times higher: 24% and 19% a
    year at 290 K.
  - The starting kinds' burning genes are lowered by log10 4 = 0.602, so a body's own burning and its
    enzyme cost stay the same.
  - The factor 4 was chosen from the litter range before any run.
- **Year 60, carbon gas:**

| seed | now | with the fix |
|---|---|---|
| 1 | 113 ppm | 169 ppm |
| 2 | 181 ppm | 230 ppm |
| 3 | 44 ppm | 179 ppm |

  Seed 3 stays near 205 to 218 ppm from year 70 to year 150.
- **Side effects:**
  - The bare (uncatalysed) decay in living bodies is also 4 times faster.
  - Faster decay competes with animals for the same dead matter: seed 3's animals grow later.
  - Three tests need new fixtures.

## The structural fix (for the owner)

- **The sea needs a carbon store,** as Earth's ocean has: a carbonate system, with a dissolved form
  of carbon and the reaction that makes it.
- **A rough prototype** (data only) gives 178 ppm at year 60.
- **It needs a decision on the start:** the air begins at about 580 ppm, so air and sea start
  balanced at 400.
- **It is crude:** linear, and the whole sea depth counts as one layer. It should be built properly,
  not merged as is.

## Doubts

- **No humus pool.** A litter rate for all dead matter may be too fast for older soil.
- **No moisture effect on decay.** Dry cells should keep dead matter longer: the third use of the
  dormancy law.
- **The sea floor uses the air's oxygen to decay dead matter.** Deep water would have none.

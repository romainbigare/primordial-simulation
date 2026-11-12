# Handover to the next orchestrator

For the next agent who leads the work. Simple English, as the owner asks. Read this first, then `CLAUDE.md`, then
`review/roadmap.md` (why the work has this shape, and section 10: where each stage stands), then `docs/laws.md`. The
spec is `docs/handover.md`; where it and the code differ, the code and `docs/laws.md` are the truth. The other files
in `review/` (`handover.md`, `plan.md`, `top-30.md`, `objects.md`, `big-look.md`) are older looks; this file and the
roadmap are the present.

## 1. The owner and how they work

- The owner writes short messages and reads very simple English only: short sentences, common words.
- They give a direction and expect it done with helpers: you are the orchestrator, architect and reviewer; helpers
  (subagents) write the code; you check, merge, run the gate and push. They asked for at most two helpers at once, Opus
  and Sonnet where each fits (Sonnet did one pure clean-up well; check its claims, one was wrong).
- They judge by pictures. Ask every helper for before | after pictures (same seed, date and place) and send the best
  to the owner. Look at them yourself first: twice a helper's work passed every test and looked worse.
- **Their two rules, above all others:**
  1. **Things emerge from general physics and chemistry laws.** No rule made for one thing.
  2. **Do not compute what is not needed.**
- The "Earth" view is the pretty one: vivid, a little cartoony, tricks allowed if they follow the world's numbers. The
  "True" view stays physically true.
- Tests: few, and always without animals for now (`ew run --roster --plants`). Never loosen a tolerance.
- They said: push to main often.

## 2. How to run helpers

- Give each helper a full brief: the goal, the owner's own words, what is known, the files, what not to touch (the
  other helper's area), what to measure before and after, the gate, the report's form (simple English, bad news
  first), and: commit, never push, no pull request. Each works in its own worktree (`isolation: worktree`). A new
  worktree starts from `origin/main`, not from your newest commit: tell every helper to merge the work branch first.
- Measure first. The briefs that worked asked for a table of numbers before any change, then one change per commit.
- A helper stops and asks when a key number leaves its range. The decision is yours; tell the owner plainly.
- A helper cannot give you permission. If it says an action was denied to it, do not do that action for it.
- Before merging: every commit signed (`git cat-file -p <commit> | grep -c '^gpgsig'` gives 1), the pictures looked
  at, the law commits' messages read. Merge with `git merge --no-ff`. On a conflict of `SNAPSHOT_VERSION`
  (`tape/src/snapshot.rs`) take a number above both sides.
- For a "bit for bit" claim run your own pair: the `ew` binary before and after, same run, equal `hash` and equal
  `series.csv` but the `ms_*` columns.
- Work on the session's branch and push it freely. Push to `main` only as a fast-forward of a commit whose gate
  passed: `git fetch -q origin main && git merge-base --is-ancestor origin/main HEAD && git push origin HEAD:main`.
- Worktrees and scratch files die with the container. Anything worth keeping is merged or pushed before the session
  ends.

## 3. The gate

Before every push to main (CLAUDE.md 6.3). Builds take 10 to 25 minutes when helpers build too: run it in the
background.

```
cargo build --release --workspace --all-targets
cargo test --release --workspace --no-fail-fast 2>&1 | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
cargo clippy --release --workspace --all-targets      # no warning
cargo build -p pcore --target wasm32-unknown-unknown
cd web && npm run build && npm run wasm
```

The fast suite has 207 tests. Then open the page (section 6) and look. For a change of the laws, also the slow tests
(`-- --ignored`): `the_key_numbers_stay_in_their_ranges`, `a_globe_without_life_keeps_an_earth_like_climate`,
`the_long_step_keeps_the_climate`.

**The key numbers' references** (`tape/tests/ranges.rs`) are the means of three runs of the world itself, a guard that
the world stays near itself; the Earth anchors in that test are separate and fixed. After the new ground, the bodies
and the rain, all nine references were measured again (ranges unchanged). Measure them again only with a change of the
laws that moves the world on purpose, in a commit of its own, and tell the owner each time.

## 4. Where things stand

On main, all gated (details in `review/roadmap.md` section 10):
- **Fixed cells** of 2 km; the tree of cells is deleted. The fine picture is drawn per pixel from ground tiles and a
  table of the cells' state.
- **A worn ground** (`core/src/space/`): stream power erosion at the world's start, ways free of the grid, hard and
  soft bed, loose ground per sample, a shelf under the shore. A cell's ground node takes the top `skin_m` (2 m) of
  each sample's loose ground at most.
- **Water on one network**: bodies of standing liquid are ponds of the terrain's samples, each at one level, full to
  their pass at the world's start; channels drawn as smooth lines in their valleys.
- **Rain that drifts** with the wind before it lands (`flow/air/seen.rs`), so every cell under raining air gets some.
- **Plants from their numbers**: six starting kinds, a colour and a design per kind, leaves that turn.
- **The Earth view's style**: water by depth, beaches, lakes and rivers as water with soft shores, bare bed as rock
  where the loose ground is thin, the ground's grain.
- **Speed**: a month of the flow takes a quarter fewer instructions than before this round; on the page the world
  waits for the page to draw its frame, so the date shown is the world's.

- **Steep slopes shed their loose ground** (`space/mantle.rs`): bare bed on ridges and steep walls.

In work when this was written (each in its own worktree; see section 8 for what is merged):
- **Plants seen close up**: crown forms from the kinds' numbers, colour variety, autumn reds, herbs as green ground.
- **Climate bands on a small world**, an experiment the owner asked for on a branch (item 2 below).
- **The land's large shape** (roadmap section 11): a range, hills and a plain on one island; a drowned coast; more
  makes of bed in layers.
- Waiting for the plants' package to end (same shader files): lake squares where channels enter a full lake, rivers
  drawn at their flood bed's width, bare bed drawn smooth.

## 5. What to do next, in order

1. Finish and judge the two packages above by their pictures.
2. **More varied climate on small worlds** (the owner said go, as an experiment on a branch). The facts:
   - The map's rows are spread evenly from `Config::lat_south` to `lat_north` (40° to 42.3° by default), whatever the
     map's size in metres. Sunlight, the wind's turning and the starting temperatures read that latitude. So a wider
     band is two numbers.
   - But the air undoes it: the eddies even out a block of sqrt(`mix` × day) = 340 km each day (`daily::blocks`), the
     mixing and the overturning on each link go by the true distance between air nodes (8 km on an island), so a
     44 km island has one air temperature whatever the sun does at its two ends.
   - The plan: (a) one run with a wide band and nothing else changed, to measure how much contrast survives (ground
     temperature, rain and the kinds alive by row); (b) one number, the map's stretch = the band's true length over
     the map's length, by which the air's sideways laws (the links' lengths in `wind::pushes`, the blocks' side) take
     their distances, so the air behaves as over the band while the ground keeps its true metres; the rain's drift
     stays in true metres; (c) the default band for small worlds chosen with the owner, the starting kinds checked
     to live somewhere in it, the key numbers' references measured again; (d) wet and dry belts, if (b) does not
     give them: the hard part, a design of its own (what the air beyond the map's edge is).
   - Effort: (a) an hour; (a) to (c) one or two helper packages; (d) two or three more, uncertain.
3. Open on the look (roadmap section 10): rivers pale beside lakes, marks where a channel leaves a pond, snow in
   blocks close up, bare ground shaded in blocks of a sample's width.
4. Open on the ground and the water: loose ground that settles during a run is not bounded by the `skin_m` rule; a
   closed basin by the sea starts full and falls for decades; a tile costs 7 to 10 ms.
5. Open on the speed: the parts' solves and the monthly full state (both need a design, see the roadmap).
6. Later: seasons blended on the page, tapes through the tile path, a law for mud and sand, animals back in view.

## 6. The live world for the owner

The owner tests an artifact: https://claude.ai/artifact/GKxSQCDfgBAwwL2B8RvEuC (private to them). To update it after
a push to main:
1. `cd web && npm run build && npm run wasm` (makes `web/dist/index.html` and `web/dist/ew.wasm`).
2. Copy `ew.wasm` to a scratch folder, and the page with these removed (regexes, case-insensitive, with `\b` after
   the tag names): the doctype, the `html`, `head` and `body` tags, the `meta charset` and `meta viewport` tags, the
   `link rel=icon`.
3. Publish with the Artifact tool to that URL, the page as `file_path` and
   `files: {"ew.wasm": {"from": <the copied wasm>, "contentType": "application/wasm"}}`. If it refuses because the
   published version was not read in this session, read it first (`action: read`), then publish.

To look at the page before publishing: serve `web/dist` (`python3 -m http.server PORT`) and drive it with Playwright
(`require('/opt/node22/lib/node_modules/playwright')`, `chromium.launch()` with no options): open
`index.html?seed=1&size=22`, wait 6 s, click the last button of `#speeds`, wait, click `#play` to pause, wait 6 s,
screenshot `#map`; to zoom, move the mouse over `#cv` and send wheel steps of -600, then wait 12 s for the tiles. Count
console errors (0 expected; ignore "GPU stall" lines) and read `.stats`. Pause before every picture. The headless
browser draws in software, so its frames per second mean nothing; the world's pace in months per second does.

## 7. Choices the owner should know of

- **Two references were moved this round**, both told to the owner, neither answered yet: river cells with the
  rain's drift (309 -> 214), then all nine with the bodies' start. Each is one commit and can be reverted alone.
- **The page's world waits for its frame.** In a browser without a graphics card the 50 m view runs at half the pace;
  on a real card it should not. If the owner finds close views slow, let the world run up to a second ahead
  (`waiting()` in `web/src/worker.ts`).
- **The shapes of animals** ("From above" or "Angled") and **a law for mud and sand** are still open from before.
- Animals are out of every run and test for now, by the owner's word.

## 8. Branches kept for the next session

- `experiment/climate-bands` (the owner's word: push it): an island's rows between two latitudes (`ew run --band S,N`,
  the page's `?band=S,N`), the air's stretch (`Config::stretch`, a switch with default 1, bit for bit with main), and,
  in work, the island in the planet's air (a strip of the planet run once as the reference climate, open edges, the
  mean wind through the box). First numbers: band 30 to 55 keeps a third of a globe's contrast in the year's mean
  (6 K of 19 K), rain falls five times from south to north, a band past 58° freezes while the box is closed, plants do
  not yet split by row. Not for main until the owner has seen it work; then the stretch becomes a derived number.

Everything else of this session that passed its gate is on main.

# Emergent World

A game in which a whole world lives by itself over thousands of years, and the player watches it on a 2D map. The
world runs on five physical laws (Flow, Change, Bond, Copy, Choose) over one chemistry; everything else (climate,
rivers, plants, animals, and later people and their cultures) must emerge. Nothing in the core names an Earth thing.

The code is Rust (the core, native and WebAssembly) and TypeScript (the page), with shaders in WGSL.

## Quick start

Build and run a world headless (`ew`, in `cli/`):

```
cargo build --profile fast -p cli                                           # target/fast/ew
target/fast/ew                                                              # every command and flag
target/fast/ew run --roster --years 300 --out out/run
target/fast/ew resume --from out/run/snap_00300.bin.gz --years 100 --out out/run
target/fast/ew view --from out/run/snap_00300.bin.gz --px 800 --out look.png
```

`fast` builds in seconds and gives the same worlds as `--release`, which is kept for the gate before main
(`docs/workflow.md`).

`ew run` writes the tape to `--out`: `series.csv`, `physics.json`, the viewer's file `world.ewf.gz` and a snapshot at
the end. It prints its speed at the end. Other commands: `cells`, `card`, `planet`, `chem`, `numbers`.

Build and open the page (needs Node and cargo; the build translates the shaders with `web/shade`):

```
cd web
npm run wasm       # the core as WebAssembly: dist/ew.wasm
npm run build      # the page: dist/index.html
python3 -m http.server --directory dist
```

Open `http://localhost:8000` for a live world (seed, size and shape in the World panel), `?f=world.ewf.gz` to play a
tape copied into `dist/`, or `?world=FILE` to open a snapshot. A file can also be dropped on the page.

## The docs

| File | What it holds |
|---|---|
| `CLAUDE.md` | the rules of work: read it first |
| `docs/handover.md` | the spec: the goal, the principles, the decisions, the world model, the architecture |
| `docs/laws.md` | how each law works now, with the code that does it |
| `docs/numbers.md` | every free number: its unit, its anchor, its mark |
| `docs/tape.md` | what a run records |
| `docs/workflow.md` | how we work: build profiles, tests, the gate, the picture tool, the cloud machine, helpers |
| `review/roadmap.md` | the plan, the priorities and the owner's decisions |
| `web/look/README.md` | the picture tool: pictures and numbers of the page in a headless browser |

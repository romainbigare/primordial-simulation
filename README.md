# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records
- `docs/history.md`: what changed in M0 to M3, and where old files are in git

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run   # simulate, write PNGs + tape
cargo run --release -p cli --bin ew                                                       # all flags
cargo run --release -p cli --features threads --bin ew -- run ...                          # the same run, flow and change on threads (same hash)
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                             # viewer: open dist/index.html via a local server
```

By default animals start with hand-set brain biases and plants with neutral brains
(`--neutral-eaters` and `--plant-bias` change this).

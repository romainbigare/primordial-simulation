# Emergent World

A world that runs on five physical laws; everything else must emerge. Plan: `docs/handover.md`.

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run   # simulate, write PNGs + tape
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                             # viewer: open dist/index.html via a local server
```

// Time the ground's tiles in wasm: node tiletime.js WASM SEED  (tiles about a coast and a lake, depths 9, 11, 13)
const fs = require('fs');
const [file, seed] = process.argv.slice(2);
(async () => {
  const core = (await WebAssembly.instantiate(fs.readFileSync(file), {})).instance.exports;
  core.ground(+seed, 22);
  const places = [[35.8, 11.7], [27.9, 22.2], [16.1, 9.5], [37.1, 19.1], [18.3, 32.7]];
  for (const d of [9, 11, 13]) {
    const span = 256 * 2000 / 2 ** (d - 2);
    const ts = [];
    for (const [x, y] of places) for (let k = 0; k < 4; k++) {
      const tx = Math.floor(x * 1000 / span) + (k & 1), ty = Math.floor(y * 1000 / span) + (k >> 1);
      let best = Infinity;
      for (let r = 0; r < 5; r++) { const t0 = process.hrtime.bigint(); core.tile(d, tx, ty); best = Math.min(best, Number(process.hrtime.bigint() - t0) / 1e6); }
      ts.push(best);
    }
    ts.sort((a, b) => a - b);
    console.log(`depth ${d}: median ${ts[ts.length >> 1].toFixed(1)} ms, mean ${(ts.reduce((a, b) => a + b) / ts.length).toFixed(1)} ms`);
  }
})();

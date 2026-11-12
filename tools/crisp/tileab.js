// Tiles of two wasm builds timed in turn in one process: node tileab.js A.wasm B.wasm SEED
const fs = require('fs');
const [fa, fb, seed] = process.argv.slice(2);
(async () => {
  const load = async f => { const c = (await WebAssembly.instantiate(fs.readFileSync(f), {})).instance.exports; c.ground(+seed, 22); return c; };
  const cores = [await load(fa), await load(fb)];
  const places = [[35.8, 11.7], [27.9, 22.2], [16.1, 9.5], [37.1, 19.1], [18.3, 32.7], [20, 20], [12, 14]];
  for (const d of [9, 11, 13]) {
    const span = 256 * 2000 / 2 ** (d - 2), sum = [0, 0];
    for (const [x, y] of places) for (let k = 0; k < 4; k++) {
      const tx = Math.floor(x * 1000 / span) + (k & 1), ty = Math.floor(y * 1000 / span) + (k >> 1);
      const best = [Infinity, Infinity];
      for (let r = 0; r < 8; r++) for (const i of [0, 1]) {
        const t0 = process.hrtime.bigint(); cores[i].tile(d, tx, ty); best[i] = Math.min(best[i], Number(process.hrtime.bigint() - t0) / 1e6);
      }
      sum[0] += best[0]; sum[1] += best[1];
    }
    const n = places.length * 4;
    console.log(`depth ${d}: ${(sum[0] / n).toFixed(2)} ms | ${(sum[1] / n).toFixed(2)} ms a tile (${((sum[1] / sum[0] - 1) * 100).toFixed(0)}%)`);
  }
})();

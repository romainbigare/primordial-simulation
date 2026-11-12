// Run the page's core (ew.wasm) in node: a new world (seed 1, size 22), MONTHS months, the time per month, and a hash of
// the frame and the cells' table at the end, to compare two builds of the core bit for bit.
// node wasmrun.js EW_WASM MONTHS
const fs = require('fs');
const crypto = require('crypto');
const [file, months] = process.argv.slice(2);
(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(file), {});
  const c = instance.exports;
  let t = performance.now();
  if (c.new_world(1, 22) !== 0) throw new Error('refused');
  const made = performance.now() - t;
  const h = crypto.createHash('sha256');
  const take = at => h.update(new Uint8Array(c.memory.buffer, at, c.out_len()));
  t = performance.now();
  const lap = [];
  for (let m = 0; m < +months; m++) {
    c.step_month();
    if ((m + 1) % 12 === 0) { lap.push(performance.now() - t); t = performance.now(); }
  }
  take(c.frame()); take(c.table());
  const late = lap.slice(Math.floor(lap.length / 2));
  console.log(JSON.stringify({ file: file.split('/').slice(-3).join('/'), makeMs: Math.round(made), msPerYearLateHalf: Math.round(late.reduce((a, x) => a + x, 0) / late.length), msPerYearMin: Math.round(Math.min(...late)), year: c.year().toFixed(3), hash: h.digest('hex').slice(0, 16) }));
})();

// What the page spends, for the cost line (main.ts `showCost`): amounts (ms of the page's thread, frames, tiles, ...)
// summed by name over one whole second, and the last whole second's sums, per second.

/** The time the sums are taken over, ms. */
const COST_WINDOW_MS = 1000;
let sums = new Map<string, number>(), lastSums = new Map<string, number>(), sumsSince = performance.now();

/** Close the second once it is over: its sums, per second, are the last ones. */
function rollCost(): void {
  const now = performance.now();
  if (now - sumsSince < COST_WINDOW_MS) return;
  lastSums = new Map([...sums].map(([key, v]) => [key, (v * 1000) / (now - sumsSince)]));
  sums = new Map();
  sumsSince = now;
}

/** Add `amount` to the sum of `key`. */
export function spend(key: string, amount: number): void {
  rollCost();
  sums.set(key, (sums.get(key) ?? 0) + amount);
}

/** The sum of `key` over the last whole second, per second. */
export function spent(key: string): number {
  rollCost();
  return lastSums.get(key) ?? 0;
}

/** Run `f`, its time on the page's thread added to `key` (ms) and one to `key` + ' n'. */
export function timed<T>(key: string, f: () => T): T {
  const t0 = performance.now();
  try { return f(); } finally { spend(key, performance.now() - t0); spend(`${key} n`, 1); }
}

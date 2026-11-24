// The fire front of a stroke of Burn, played on the map though the world stands still: the core says which cells its
// flames reached and when (god.ts `Settled`); the frame that follows says which squares burnt now, the core is asked
// the cell under each of them (`probe`), and each square burns on the map when the front reached its cell (fire.ts
// `play`).
import { World, decode } from './data';
import type { Fire } from './fire';
import type { Probe } from './god';

/** A square burnt now: its months since it last burnt are fewer than this. */
const NOW_MONTHS = 1;

export class FireFront {
  /** When the front reached each cell, s after its strike, till the frame after the stroke comes. */
  private waiting: Map<number, number> | null = null;

  /** `core`: what asks the live world's core about a place (squares), its answer to `answer`. */
  constructor(private core: () => { probe(x: number, y: number, answer: (p: Probe) => void): void } | null, private fire: Fire) {}

  /** The cells a stroke burnt and when (`Settled`): played once the frame after it comes. */
  burnt(burnt: [number, number][]): void { this.waiting = new Map(burnt); }

  /** A frame of world `w`: where a front waits, the squares it burnt are found and it plays. The core is asked the cell
   * of one square at a time: its answer names the cell's edges, so the other squares of that cell are not asked. */
  frame(w: World): void {
    const reached = this.waiting;
    this.waiting = null;
    if (!reached || !w.index.has('burnt_age')) return;
    const age = decode(w, 0, 'burnt_age'), side = w.meta.w, at = new Float32Array(age.length).fill(NaN);
    const left = [...age.keys()].filter(i => age[i] < NOW_MONTHS);
    const next = (): void => {
      const i = left.shift(), core = this.core();
      // (All found, or the world gone: what was found plays.)
      if (i === undefined || !core) return this.fire.play(at);
      core.probe((i % side) + 0.5, Math.floor(i / side) + 0.5, p => {
        const [x0, y0, x1, y1] = p.bounds, a = reached.get(p.cell) ?? NaN;
        const inside = (k: number) => { const x = (k % side) + 0.5, y = Math.floor(k / side) + 0.5; return x > x0 && x < x1 && y > y0 && y < y1; };
        at[i] = a;
        for (let k = left.length - 1; k >= 0; k--) if (inside(left[k])) { at[left[k]] = a; left.splice(k, 1); }
        next();
      });
    };
    next();
  }
}

// The fire of a stroke of Burn, played on the map though the world stands still: the core says which cells its flames
// reached (god-types.ts `Settled`); the frame that follows says which squares burnt now, the core is asked the cell under
// each of them (`probe`), and the burn plays outward from its strike over the squares of the cells it reached, to the
// land its flames took (the share of each cell's land burning this month, the core's `flame`, times its land; fire.ts
// `play`).
import { World, decode } from './data';
import type { Fire } from './fire';
import type { Probe } from './god-types';

/** A square burnt now: its months since it last burnt are fewer than this. */
const NOW_MONTHS = 1;

export class FireFront {
  /** The cells a stroke's flames reached and where it struck (squares), till the frame after the stroke comes. */
  private waiting: { cells: Set<number>; strike: [number, number] } | null = null;

  /** `core`: what asks the live world's core about a place (squares), its answer to `answer`. */
  constructor(private core: () => { probe(x: number, y: number, answer: (p: Probe) => void): void } | null, private fire: Fire) {}

  /** The cells a stroke burnt (`Settled`) from its strike at `strike` (squares): played once the frame after it comes. */
  burnt(burnt: [number, number][], strike: [number, number]): void {
    this.waiting = { cells: new Set(burnt.map(b => b[0])), strike };
    this.fire.hold();
  }

  /** A frame of world `w`: where a burn waits, the squares it burnt are found and it plays. The core is asked the cell
   * of one square at a time: its answer names the cell's edges, so the other squares of that cell are not asked. */
  frame(w: World): void {
    const burn = this.waiting;
    this.waiting = null;
    if (!burn) return;
    // (No fire told: the planes are let go as they are.)
    if (!w.index.has('burnt_age')) return this.fire.still();
    const age = decode(w, 0, 'burnt_age'), flame = decode(w, 0, 'flame'), side = w.meta.w, reached = new Uint8Array(age.length);
    const left = [...age.keys()].filter(i => age[i] < NOW_MONTHS);
    let m2 = 0;
    const next = (): void => {
      const i = left.shift(), core = this.core();
      // (All found, or the world gone: what was found plays.)
      if (i === undefined || !core) return this.fire.play(reached, burn.strike, m2);
      core.probe((i % side) + 0.5, Math.floor(i / side) + 0.5, p => {
        const [x0, y0, x1, y1] = p.bounds, hit = burn.cells.has(p.cell) ? 1 : 0;
        m2 += hit * flame[i] * p.land;
        const inside = (k: number) => { const x = (k % side) + 0.5, y = Math.floor(k / side) + 0.5; return x > x0 && x < x1 && y > y0 && y < y1; };
        reached[i] = hit;
        for (let k = left.length - 1; k >= 0; k--) if (inside(left[k])) { reached[left[k]] = hit; left.splice(k, 1); }
        next();
      });
    };
    next();
  }
}

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

  /** `ask` asks the core about a place (squares), its answer to `answer`. */
  constructor(private ask: (x: number, y: number, answer: (p: Probe) => void) => void, private fire: Fire) {}

  /** The cells a stroke burnt and when (`Settled`): played once the frame after it comes. */
  burnt(burnt: [number, number][]): void { this.waiting = new Map(burnt); }

  /** A frame of world `w`: where a front waits, the squares it burnt are found and it plays. */
  frame(w: World): void {
    const reached = this.waiting;
    this.waiting = null;
    if (!reached || !w.index.has('burnt_age')) return;
    const age = decode(w, 0, 'burnt_age'), side = w.meta.w, at = new Float32Array(age.length).fill(NaN);
    const squares = [...age.keys()].filter(i => age[i] < NOW_MONTHS);
    let left = squares.length;
    for (const i of squares) {
      this.ask((i % side) + 0.5, Math.floor(i / side) + 0.5, p => {
        at[i] = reached.get(p.cell) ?? NaN;
        if (--left === 0) this.fire.play(at);
      });
    }
  }
}

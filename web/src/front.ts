// The fire of a stroke of Burn, played on the map though the world stands still: the core says which squares lie in the
// cells its flames reached (god-types.ts `Settled`); the frame that follows says which of them burnt now, and the burn
// grows from its strike over them, each to the share of its land the core says burnt (fire.ts `play`, burns.ts).
import { World, decode } from './data';
import type { Fire } from './fire';

/** A square burnt now: its months since it last burnt are fewer than this. */
const NOW_MONTHS = 1;

export class FireFront {
  /** The squares in the cells a stroke's flames reached, where it struck (squares) and where its heat lit the ground
   * (m), till the frame after the stroke comes. */
  private waiting: { squares: number[]; strike: [number, number]; lit: [number, number][] } | null = null;

  constructor(private fire: Fire) {}

  /** The squares a stroke burnt (`Settled`) from its strike at `strike` (squares), its heat having lit the ground at
   * `lit` (m): played once the frame after it comes. */
  burnt(squares: number[], strike: [number, number], lit: [number, number][]): void {
    this.waiting = { squares, strike, lit };
    this.fire.hold();
  }

  /** A frame of world `w`: where a burn waits, the squares it burnt now play. */
  frame(w: World): void {
    const burn = this.waiting;
    this.waiting = null;
    if (!burn) return;
    // (No fire told: the planes are let go as they are.)
    if (!w.index.has('burnt_age')) return this.fire.still();
    const age = decode(w, 0, 'burnt_age'), reached = new Uint8Array(age.length);
    for (const i of burn.squares) if (age[i] < NOW_MONTHS) reached[i] = 1;
    this.fire.play(reached, burn.strike, burn.lit);
  }
}

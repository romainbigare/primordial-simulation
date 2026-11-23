// The pointers on the map as one gesture (map.ts): one pans the map, two or more pan it and zoom it about their middle.
// The camera follows the pointers from where the gesture began, set once a frame from where they are now against where
// they were then (`apply`): the moves two fingers send one after the other in a frame are one move, about one middle.
// A pointer's first move after it went down is not applied: the browser sends none till the pointer has passed its
// slop, which that first move holds all at once; the gesture begins anew there, and where a pointer comes or goes.

/** The camera: css px a square, and the map's corner, css px. */
export interface Camera { s: number; ox: number; oy: number }
interface Point { x: number; y: number }
/** A pointer down on the map: where it is, css px, and whether it moved since it went down. */
interface Pointer extends Point { live: boolean }

/** Pointers spread less than this about their middle, css px, do not zoom (two fingers at one place). */
const SPREAD_LEAST_PX = 1;

export class Gesture {
  private ptrs = new Map<number, Pointer>();
  /** The pointers' middle and their mean distance from it, and the camera, as the gesture began (null: no pointer). */
  private from: { mid: Point; spread: number; cam: Camera } | null = null;
  /** Whether a pointer moved since the camera was last set. */
  private due = false;
  /** How far the pointers moved since the first went down, css px (a second pointer counts as far). */
  moved = 0;

  /** The camera now and its setter, and the scale a camera may take nearest `s` (`limit`). */
  constructor(private cam: () => Camera, private set: (c: Camera) => void, private limit: (s: number) => number) {}

  /** The pointers down. */
  get size(): number { return this.ptrs.size; }

  /** Pointer `id` went down at (`x`, `y`). */
  down(id: number, x: number, y: number): void {
    this.apply();
    this.ptrs.set(id, { x, y, live: false });
    this.moved = this.ptrs.size > 1 ? Infinity : 0;
    this.begin();
  }

  /** Pointer `id` moved to (`x`, `y`); false where it is not down. */
  move(id: number, x: number, y: number): boolean {
    const p = this.ptrs.get(id);
    if (!p) return false;
    this.moved += Math.abs(x - p.x) + Math.abs(y - p.y);
    if (p.live) { p.x = x; p.y = y; this.due = true; return true; }
    this.apply();
    p.x = x; p.y = y; p.live = true;
    this.begin();
    return true;
  }

  /** Pointer `id` went up (or was cancelled). */
  up(id: number): void {
    this.apply();
    this.ptrs.delete(id);
    this.begin();
  }

  /** Set the camera from where the pointers are now against where the gesture began: scaled by how far they spread
   * about their middle, about their middle then, and moved as their middle moved; nothing where none moved since. */
  apply(): void {
    if (!this.due || !this.from) return;
    this.due = false;
    const { mid, spread, cam } = this.from, now = this.shape();
    const s = this.limit(spread >= SPREAD_LEAST_PX && now.spread >= SPREAD_LEAST_PX ? (cam.s * now.spread) / spread : cam.s);
    this.set({ s, ox: now.mid.x - ((mid.x - cam.ox) / cam.s) * s, oy: now.mid.y - ((mid.y - cam.oy) / cam.s) * s });
  }

  /** Begin the gesture anew from where the pointers and the camera are now. */
  private begin(): void {
    this.due = false;
    this.from = this.ptrs.size ? { ...this.shape(), cam: { ...this.cam() } } : null;
  }

  /** The pointers' middle and their mean distance from it, css px. */
  private shape(): { mid: Point; spread: number } {
    const all = [...this.ptrs.values()], n = all.length;
    const mid = { x: all.reduce((t, p) => t + p.x, 0) / n, y: all.reduce((t, p) => t + p.y, 0) / n };
    return { mid, spread: all.reduce((t, p) => t + Math.hypot(p.x - mid.x, p.y - mid.y), 0) / n };
  }
}

// What god mode (god.ts) and the live world (live.ts) say to each other, and what a tool of god mode is
// (god-tools.ts): the core's answers about a place, a brush and a stroke's settling; the worker's side of the tools; a
// tool; and what the page gives god mode.
import type { IconName } from './icons';

/** What the core says of the cell under a place (wasm/src/lib.rs `probe`): the cell, its edges in squares (west, north,
 * east, south), the centre of its sample under the place (squares), its area and land (m²), its patches, and whether a
 * strike lights it ("yes", or the core's reason). */
export interface Probe { cell: number; bounds: [number, number, number, number]; at: [number, number]; area: number; land: number; patches: number; fire: string }
/** A starting kind the world's plan places (wasm/src/lib.rs `kinds`): its row, the core's name, kg per m² placed. */
export interface StartKind { row: number; name: string; kg: number }
/** What a brush did (wasm/src/lib.rs `brush`), as god mode reads it: of a planting, each cell newly under it, the share
 * of it under the brush, whether each of its edits was made and the core's reason where not; of heat, each cell under
 * it, its share, how much of the heat that lights its remains it holds (`progress`, 0 to 1) and why none would light.
 * (A brush of the ground, or a lake, answers its cells as numbers alone.) */
export interface Brushed { cells: { cell: number; share: number; done?: boolean[]; progress?: number; why: string }[] }
/** What a stroke's settling did (wasm/src/lib.rs `settle`): whether the ground was built again, and each cell the
 * flames of its strikes reached, and when, s after the strike. */
export interface Settled { ground: boolean; burnt: [number, number][] }
/** The worker's side of the tools (live.ts `Live`): ask what lies under a place, paint, end a stroke, settle a stroke
 * held so far and go on with it, and undo the last. */
export interface Editor {
  probe(x: number, y: number, answer: (p: Probe) => void): void; brush(tool: number, x: number, y: number, radius: number, amount: number, row: number): void;
  settle(): void; soften(): void; undo(): void;
}

/** A tool of god mode. The strip shows the tools in the order of their list; a new tool is one more entry. */
export interface GodTool {
  /** Its key (its button's `data-tool`), its icon, its short name, and its colour on the map. */
  key: string; icon: IconName; label: string; ink: string;
  /** How to use it with a mouse, in a short line; and its big button's words on a phone (`GodPage.aim`). */
  hint: string; verb: string;
  /** The core's brush tool it paints with (wasm/src/lib.rs `brush`), and the starting kind it places (-1: none). */
  code: number; row(): number;
  /** What it adds at the brush's centre each second it is held, in its core tool's unit (m of ground, up or down where
   * less than 0; J/m² of heat): it paints again and again while the pointer is down, held still or not. */
  rate?: number;
  /** What it adds at the brush's centre in steps, in its core tool's unit (m of ground, up or down where less than 0):
   * one where the stroke begins and one more at the start of each `every` ms it is held. */
  step?: { amount: number; every: number };
  /** Whether a stroke of it held settles what it did so far every `SOFTEN_MS` (god.ts): the ground's tools, so the
   * water follows the ground while it moves. */
  softens?: boolean;
  /** The time over which its rate doubles while it is held, s (none: it stays), and the most times it doubles. */
  doubling?: number; doublings?: number;
  /** Whether what it starts runs on its own once it took (a fire): its stroke ends as soon as a cell lights. */
  settlesLit?: boolean;
  /** The amount it paints once, where the stroke begins (m of a lake's depth): a tool that makes one thing a stroke. */
  amount?(): number;
  /** Why it cannot act at the place the core answered about (null: none yet), or ''. */
  why(p: Probe | null): string;
  /** The core's reason a cell was not edited (`Brushed`), in plain words. */
  reason(why: string): string;
  /** Its own choices, shown under the brush's sizes (`show` draws the card anew after one is picked). */
  choices?(show: () => void): HTMLElement;
}
/** What the page gives god mode: its strip and card; the map drawn anew; that a tool was picked or a stroke begins (the
 * world pauses); that god mode is done; the side of a square, m; the cells a stroke burnt with when (`Settled`), and
 * where it struck (squares); and on a phone the place at the sight in the middle of the map, squares (null: the brush
 * is under the pointer). */
export interface GodPage {
  strip: HTMLElement; card: HTMLElement;
  redraw(): void; picked(): void; close(): void; squareM(): number; burnt(burnt: [number, number][], strike: [number, number]): void;
  aim(): [number, number] | null;
}

// What god mode (god.ts) and the live world (live.ts) say to each other, and what a tool of god mode is
// (god-tools.ts): the core's answers about a place, a brush and a stroke's settling; the worker's side of the tools; a
// tool, its setting, and the classes and categories that hold the tools; and what the page gives god mode.
import type { IconName } from './icons';

/** What the core says of the cell under a place (wasm/src/edit.rs `probe`): the cell, its edges in squares (west,
 * north, east, south), its land (m²), its patches, and whether heat added there lights it ("yes", or the core's
 * reason). */
export interface Probe { cell: number; bounds: [number, number, number, number]; land: number; patches: number; fire: string }
/** A starting kind the world's plan places (wasm/src/edit.rs `kinds`): its row and the core's name. */
export interface StartKind { row: number; name: string }
/** What a brush did (wasm/src/edit.rs `brush`), as god mode reads it: of a planting, each cell newly under it, the share
 * of it under the brush, whether each of its edits was made and the core's reason where not; of heat, each cell under
 * it, its share, how much of the heat that lights its remains it holds (`progress`, 0 to 1) and why none would light.
 * (A brush of the ground, or a lake, answers no cells.) And whether it changed the world. */
export interface Brushed { cells: { cell: number; share: number; done?: boolean[]; progress?: number; why: string }[]; changed: boolean }
/** The reason god mode gives a tool (`GodTool.reason`) where a paint whose cells hold no reason changed nothing. */
export const UNCHANGED = 'Unchanged';
/** What a stroke's settling did (wasm/src/edit.rs `settle`): each cell the flames of its strikes reached, and when, s
 * after the strike, and the squares whose centre lies in one of them; why its planting placed nothing ('' where it
 * placed some, or there was none); and whether the stroke can be undone (it changed the world). */
export interface Settled { burnt: [number, number][]; squares: number[]; unplanted: string; undo: boolean }
/** The worker's side of the tools (live.ts `Live`): ask what lies under a place, paint at one or more places (each its
 * x and y in squares and its amount) in one message, end a stroke, and undo the last. */
export interface Editor {
  probe(x: number, y: number, answer: (p: Probe) => void): void; brush(tool: number, stamps: [number, number, number][], radius: number, row: number): void;
  settle(): void; undo(): void;
}

/** A tool of god mode. It stands in a category of a class (`GodClass`); a new tool is one more entry there. */
export interface GodTool {
  /** Its icon, its name, its picture (`god/<pic>.jpg` beside the page), and its colour on the map. */
  icon: IconName; label: string; pic: string; ink: string;
  /** How to use it with a mouse, in a short line; and its big button's words on a phone (`GodPage.aim`). */
  hint: string; verb: string;
  /** The core's brush tool it paints with (wasm/src/edit.rs `brush`), and the starting kind it places (-1: none). */
  code: number; row(): number;
  /** What it adds at the centre of a brush of radius `radius` m each second it is held, in its core tool's unit (m of
   * ground, up or down where less than 0; J/m² of heat): it paints at each of the page's frames while the pointer is
   * down, held still or not, once the core answered its last paint (by what the time since then adds). */
  rate?(radius: number): number;
  /** A stroke of it adds at least what a hold this long adds, s, so a tap shows; none: a tap does nothing (it begins
   * to paint once held `HOLD_MS`, god.ts). */
  tapS?: number;
  /** The most time one paint of it covers, s (none: all the time since its last paint): what the core does for a
   * paint grows with the time it covers, so a slow frame or a slow computer slows the tool, never the brush. */
  mostS?: number;
  /** Whether a paint of it lays its amount along the brush's path since its last paint (god.ts `along`), so a fast drag
   * leaves an unbroken trace; else all of it where the brush is. */
  follows?: boolean;
  /** Whether its stroke settles only once the brush rested `SHAPE_REST_MS` after it was let go (god.ts): a stroke of it
   * begun before then goes on as the same stroke of the core (they share one undo), so the world settles once, not at
   * each lift of the brush. */
  rests?: boolean;
  /** The time over which its rate doubles while it is held, s (none: it stays), and the most times it doubles. */
  doubling?: number; doublings?: number;
  /** Whether what it starts runs on its own once it took (a fire): its stroke ends as soon as a cell lights; and its
   * brush is drawn as its rim alone, so the ground under it shows as it burns. */
  settlesLit?: boolean;
  /** Whether what it does is made when its stroke ends (a planting): the brush's trail is drawn till then. */
  trails?: boolean;
  /** The amount it paints once, where the stroke begins (m of a lake's depth): a tool that makes one thing a stroke. */
  amount?(): number;
  /** The least radius of its brush, m (none: the smallest of the sizes). */
  least?: number;
  /** Why it cannot act at the place the core answered about (null: none yet), or ''. */
  why(p: Probe | null): string;
  /** The core's reason a cell was not edited (`Brushed`), in plain words; of `UNCHANGED`, why a paint that names no
   * cell's reason changed nothing. */
  reason(why: string): string;
  /** Its own setting besides the brush's size, as it stands now. */
  setting?(): GodSetting;
}
/** A setting of a tool: its name, its choices, the one on, and what picks one. Its choices are drawn as `look` says:
 * cuts through the ground, each deeper than the one before (`depth`), or each its own picture (`picture`). */
export interface GodSetting {
  name: string; look: 'depth' | 'picture'; on: number; pick(i: number): void;
  /** Each choice's short name, the line that says more of it, and its picture (`god/<pic>.jpg`, of `picture`). */
  choices: { label: string; title: string; pic?: string }[];
}
/** A tool to come: its name and picture; its row shows, and cannot be picked. */
export interface SoonTool { soon: true; label: string; pic: string }
/** A category of tools: its name, its icon and its tools. */
export interface GodCategory { label: string; icon: IconName; tools: (GodTool | SoonTool)[] }
/** A class of god mode's tools (a part of the list): its name, its icon and its categories. */
export interface GodClass { label: string; icon: IconName; categories: GodCategory[] }
/** What the page gives god mode: the box of the big button held for a stroke (a phone's), and its panel; the map drawn anew; that a tool was picked or a
 * stroke begins (the world pauses); that god mode is done; the side of a square, m; the squares a stroke burnt (`Settled`), where it struck
 * (squares) and where its heat lit the ground, m (god-draw.ts `litSites`, the most lit first); on a phone the
 * place at the sight in the middle of the map, squares (null: the brush is under the pointer); and which samples the
 * world holds and which the ground in view shows. */
export interface GodPage {
  holdBox: HTMLElement; panel: HTMLElement;
  redraw(): void; picked(): void; close(): void; squareM(): number; burnt(squares: number[], strike: [number, number], lit: [number, number][]): void;
  aim(): [number, number] | null;
  /** The version of the live world's samples, and the version the ground in view is made of (`Land::told`). */
  ground(): [number, number];
}

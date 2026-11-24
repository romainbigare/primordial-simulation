// The tools of god mode (god.ts `GodTool`), in the order of the strip: each paints with the round brush
// (wasm/src/lib.rs `brush`): Raise and Lower land move the ground under it while held, and the world builds its
// samples and settles its liquid over the new ground when the stroke ends (`settle`); Plant clears a share of each
// cell under it and plants a starting kind there; Burn strikes each, and the strikes burn at once when the stroke ends.
// What the world cannot do is said in plain
// words, from the core's own reasons. A new tool is one more entry of `godTools`.
import type { GodTool, Probe, StartKind } from './god';

/** The core's brush tools (wasm/src/lib.rs `SHAPE`, `PLANT`, `BURN`). */
const BRUSH_SHAPE = 0, BRUSH_PLANT = 2, BRUSH_BURN = 3;
/** How fast Raise and Lower move the ground at the brush's centre while held, m/s: a second's hold makes a hill or a
 * pit a hillside's shading shows from a few kilometres away. */
const SHAPE_M_S = 8;
/** The starting kinds in plain words, by the core's name (worldgen/src/life/kinds.rs `KINDS`): a short name and a line. */
const PLAIN_KINDS: Record<string, [string, string]> = {
  'small fast light-eaters': ['Short grass', 'Small plants that grow and spread fast.'],
  'light-eaters': ['Herbs', 'Soft plants of middle size.'],
  'big slow light-eaters': ['Trees', 'Tall woody plants. They grow slowly and live long.'],
  'big light-eaters of wet ground': ['Swamp trees', 'Trees that stand in wet ground.'],
  'woody light-eaters of dry ground': ['Dry shrubs', 'Woody bushes of dry ground.'],
  'tall fast light-eaters': ['Tall grass', 'Tall soft plants that grow fast.'],
};
/** The kind Plant starts with, by its plain name. */
const FIRST_KIND = 'Trees';
const OPEN_WATER = 'This is open water. Nothing grows here.';
/** Why a strike does not light a cell, in plain words, by the core's reason (core/src/flame/mod.rs `Unlit`). */
const UNLIT: Record<string, string> = {
  NoLand: 'This is open water. Nothing here can burn.',
  Lifeless: 'Nothing in this world can burn.',
  Bare: 'Nothing lies on the ground here that can burn.',
  Sparse: 'Too little dead leaves and stems lie here. They do not cover the ground, so a fire cannot spread.',
  Thin: 'Too little dead leaves and stems lie here to keep a flame going.',
  Snow: 'Snow covers the ground.',
  Unread: 'Let the world run one month first.',
  Wet: 'The ground is too wet. Much of it is soaked.',
  Damp: 'The dead leaves and stems are too damp to burn.',
};
/** Why a cell was not planted, in plain words, by the core's reason (wasm/src/lib.rs `brush`). */
const UNPLANTED: Record<string, string> = { NoLand: OPEN_WATER, NoRoom: 'The plants found no room to grow.' };

const plainKind = (k: StartKind): [string, string] => PLAIN_KINDS[k.name] ?? [k.name, ''];
const cells = (n: number) => `${n} ${n === 1 ? 'cell' : 'cells'}`;

/** Today's tools; Plant plants one of the world's starting kinds (`kinds`). */
export function godTools(kinds: () => StartKind[]): GodTool[] {
  let row = -1;
  /** The kind Plant plants: the one picked, else the first kind (`FIRST_KIND`) of the world. */
  const plantRow = () => {
    const ks = kinds();
    if (!ks.some(k => k.row === row)) row = ks.find(k => plainKind(k)[0] === FIRST_KIND)?.row ?? ks[0]?.row ?? -1;
    return row;
  };
  const kindName = () => { const k = kinds().find(d => d.row === plantRow()); return k ? plainKind(k)[0].toLowerCase() : 'plants'; };
  const plant: GodTool = {
    key: 'plant', icon: 'plant', label: 'Plant', ink: '#7ed957', code: BRUSH_PLANT, row: plantRow,
    hint: 'Drag to plant. Right-drag or two fingers move the map.',
    why: (p: Probe | null) => (!kinds().length ? 'This world has no plants to plant.' : p && p.land <= 0 ? OPEN_WATER : ''),
    reason: why => UNPLANTED[why] ?? why,
    choices: show => {
      const list = document.createElement('div'), on = plantRow();
      list.className = 'chips';
      for (const k of kinds()) {
        const b = document.createElement('button');
        b.textContent = plainKind(k)[0]; b.title = plainKind(k)[1]; b.classList.toggle('on', k.row === on);
        b.onclick = () => { row = k.row; show(); };
        list.append(b);
      }
      return list;
    },
    told: (done, why) => (done ? `Planted ${kindName()} in ${cells(done)}. They grow as the world plays.` : `Nothing was planted. ${why}`),
  };
  const burn: GodTool = {
    key: 'burn', icon: 'fire', label: 'Burn', ink: '#ff7a1a', code: BRUSH_BURN, row: () => -1,
    hint: 'Drag to set fire. Right-drag or two fingers move the map.',
    why: (p: Probe | null) => (!p || p.fire === 'yes' ? '' : (UNLIT[p.fire] ?? p.fire)),
    reason: why => UNLIT[why] ?? why,
    told: (done, why) => (done ? `Lightning struck ${cells(done)}. The fire spreads as far as the dry litter carries it.` : `Nothing caught fire. ${why}`),
  };
  /** A ground tool: it moves the ground up or down (`sign`) under the brush while held. */
  const shape = (sign: number): GodTool => ({
    key: sign > 0 ? 'raise' : 'lower', icon: sign > 0 ? 'raise' : 'lower', label: sign > 0 ? 'Raise land' : 'Lower land',
    ink: sign > 0 ? '#e8c46a' : '#6ab4e8', code: BRUSH_SHAPE, row: () => -1, rate: sign * SHAPE_M_S,
    hint: `Press and hold to ${sign > 0 ? 'raise' : 'lower'} the land; the longer, the more. Right-drag or two fingers move the map.`,
    why: () => '', reason: why => why,
    told: done => (done ? `${sign > 0 ? 'Raised' : 'Lowered'} the land. Water finds its new way.` : 'The ground did not move here.'),
  });
  return [shape(1), shape(-1), plant, burn];
}

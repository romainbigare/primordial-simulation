// The tools of god mode (god.ts `GodTool`), in the order of the strip: each paints with the round brush
// (wasm/src/lib.rs `brush`): Raise and Lower land move the ground under it while held, and the world builds its
// samples and settles its liquid over the new ground when the stroke ends (`settle`); Lake digs a basin under it at
// once, its floor a chosen depth under the brush's rim, and fills it; Plant clears a share of each
// cell under it and plants a grown stand of a starting kind there; Burn heats the remains under it while held, and
// those that hold enough heat light and burn at once when the stroke ends. What the world cannot do is said in plain
// words, from the core's own reasons. A new tool is one more entry of `godTools`.
import type { GodTool, Probe, StartKind } from './god';

/** The core's brush tools (wasm/src/lib.rs `SHAPE`, `LAKE`, `PLANT`, `BURN`). */
const BRUSH_SHAPE = 0, BRUSH_LAKE = 1, BRUSH_PLANT = 2, BRUSH_BURN = 3;
/** The depths a lake may have, m under the lowest point of the brush's rim, with their names; the second the first on. */
const LAKE_DEPTHS: [string, number][] = [['Shallow', 2], ['Middle', 5], ['Deep', 15]];
/** How fast Raise and Lower move the ground at the brush's centre while held, m/s: a second's hold makes a hill or a
 * pit a hillside's shading shows from a few kilometres away. */
const SHAPE_M_S = 20;
/** How much heat Burn adds at the brush's centre while held, J/m² a second: at the first radius, the remains of a
 * typical place light in about a second (the heat they need: core/src/flame `needed`; a cell holds the heat over all
 * its land, of which the brush covers a share), the wettest in tens. */
const HEAT_J_M2_S = 3e6;
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
    told: (done, why, grown) => (!done ? `Nothing was planted. ${why}`
      : grown ? `Planted grown ${kindName()} in ${cells(done)}.` : `No grown ${kindName()} yet: young ones planted.`),
  };
  const burn: GodTool = {
    key: 'burn', icon: 'fire', label: 'Burn', ink: '#ff7a1a', code: BRUSH_BURN, row: () => -1, rate: HEAT_J_M2_S,
    hint: 'Press and hold to heat the ground till it lights; the ring fills as it nears. Right-drag or two fingers move the map.',
    why: (p: Probe | null) => (!p || p.fire === 'yes' ? '' : (UNLIT[p.fire] ?? p.fire)),
    reason: why => UNLIT[why] ?? why,
    told: (done, why) => (done ? `Fire took in ${cells(done)}. It spreads as far as the dry litter carries it.`
      : `Nothing caught fire. ${why || 'Hold longer: the remains need more heat to light.'}`),
  };
  /** A ground tool: it moves the ground up or down (`sign`) under the brush while held. */
  const shape = (sign: number): GodTool => ({
    key: sign > 0 ? 'raise' : 'lower', icon: sign > 0 ? 'raise' : 'lower', label: sign > 0 ? 'Raise land' : 'Lower land',
    ink: sign > 0 ? '#e8c46a' : '#6ab4e8', code: BRUSH_SHAPE, row: () => -1, rate: sign * SHAPE_M_S,
    hint: `Press and hold to ${sign > 0 ? 'raise' : 'lower'} the land; the longer, the more. Right-drag or two fingers move the map.`,
    why: () => '', reason: why => why,
    told: done => (done ? `${sign > 0 ? 'Raised' : 'Lowered'} the land. Water finds its new way.` : 'The ground did not move here.'),
  });
  let depth = LAKE_DEPTHS[1][1];
  const lake: GodTool = {
    key: 'lake', icon: 'lake', label: 'Lake', ink: '#4aa3e8', code: BRUSH_LAKE, row: () => -1, amount: () => depth,
    hint: 'Click or tap to dig a lake under the brush; it fills with water at once.',
    why: (p: Probe | null) => (p && p.land <= 0 ? 'This is open water already.' : ''), reason: why => why,
    choices: show => {
      const list = document.createElement('div');
      list.className = 'chips';
      for (const [name, m] of LAKE_DEPTHS) {
        const b = document.createElement('button');
        b.textContent = `${name}, ${m} m`; b.classList.toggle('on', m === depth);
        b.onclick = () => { depth = m; show(); };
        list.append(b);
      }
      return list;
    },
    told: done => (done ? 'Dug a basin for a lake; water fills it where the ground holds it.' : 'No lake could be made here.'),
  };
  return [shape(1), shape(-1), lake, plant, burn];
}

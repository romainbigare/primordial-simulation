// The tools of god mode (god-types.ts `GodTool`) and the classes and categories that hold them, as the page shows them
// (god-menu.ts); the tools to come stand among them as cards that cannot be picked. Each tool paints with the round brush
// (wasm/src/edit.rs `brush`): Raise and Lower land move the ground under it while held, the faster the larger the
// brush, along the brush's path, the map showing the new ground at each of the page's frames, and the world builds its
// samples and settles its liquid over the new ground once the brush rested after the stroke (`settle`); Lake brush digs
// a basin under it at once, its floor a chosen depth under the lowest point of the brush's rim (or under the level of a
// lake it reaches, which grows over it), and fills it; Plants clears a share of each cell under it and plants a grown
// stand of a starting kind there; Fire heats the remains under it while held, the longer the faster, and those that
// hold enough heat light and burn at once when the stroke ends. What the world cannot do is said in plain words, from
// the core's own reasons. A new tool is one more entry of `godClasses`.
import type { IconName } from './icons';
import { UNCHANGED, type GodClass, type GodTool, type Probe, type SoonTool, type StartKind } from './god-types';

/** The core's brush tools (wasm/src/edit.rs `SHAPE`, `LAKE`, `PLANT`, `BURN`). */
const BRUSH_SHAPE = 0, BRUSH_LAKE = 1, BRUSH_PLANT = 2, BRUSH_BURN = 3;
/** The depths a lake may have, m under its level (the lowest point of the brush's rim, or a lake it reaches), with their
 * names (a choice shows the depth, its title the name); the second the first on. */
const LAKE_DEPTHS: [string, number][] = [['Shallow', 2], ['Middle', 5], ['Deep', 15]];
/** The least radius of a lake, m: it is dug on the terrain's samples, 125 m apart (core/src/space/terrain.rs
 * `SAMPLE_DEPTH`), so a smaller one is a few samples, drawn as a rounded square, not a disc. */
const LAKE_LEAST_M = 300;
/** How fast Raise and Lower move the ground at the brush's centre while it is held still, m/s for each m of the brush's
 * radius (the owner's choice: 250 m/s at a radius of 300 m, so a second's hold makes a hill of 250 m there, and a brush
 * twice as wide moves it twice as fast). */
const SHAPE_SPEED_PER_M = 250 / 300;
/** One paint of Raise or Lower covers at most this long a hold, s: a twentieth of a second (`GodTool.mostS`). */
const SHAPE_PAINT_MOST_S = 0.05;
/** A stroke of Raise or Lower adds at least what this long a hold adds, s: a tap makes a bump that shows. */
const SHAPE_TAP_S = 0.1;
/** How much heat Fire adds at the brush's centre while held, J/m² a second: at the first radius, the remains of a
 * typical place light in about a second (the heat they need: core/src/flame `needed`; a cell holds the heat over all
 * its land, of which the brush covers a share), the wettest in tens. */
const HEAT_J_M2_S = 3e6;
/** Fire's heat doubles every this many seconds it is held, so damp remains light in a few seconds too, not tens; at
 * most this many times (256 times its first rate, about 8e8 J/m² a second: what boils off some 300 kg of liquid a m²).
 * (The core adds no more than lights the remains, and none where nothing can light: core/src/edit/heat.rs.) */
const HEAT_DOUBLING_S = 1, HEAT_DOUBLINGS = 8;
/** The starting kinds in plain words, by the core's name (worldgen/src/life/kinds.rs `KINDS`): a short name and a line. */
const PLAIN_KINDS: Record<string, [string, string]> = {
  'small fast light-eaters': ['Short grass', 'Small plants that grow and spread fast.'],
  'light-eaters': ['Herbs', 'Soft plants of middle size.'],
  'big slow light-eaters': ['Trees', 'Tall woody plants. They grow slowly and live long.'],
  'big light-eaters of wet ground': ['Swamp trees', 'Trees that stand in wet ground.'],
  'woody light-eaters of dry ground': ['Dry shrubs', 'Woody bushes of dry ground.'],
  'tall fast light-eaters': ['Tall grass', 'Tall soft plants that grow fast.'],
};
/** The kind Plants starts with, by its plain name. */
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
/** Why a cell was not planted, in plain words, by the core's reason (wasm/src/edit.rs `brush`). */
const UNPLANTED: Record<string, string> = { NoLand: OPEN_WATER, NoRoom: 'The plants found no room to grow.' };

const plainKind = (k: StartKind): [string, string] => PLAIN_KINDS[k.name] ?? [k.name, ''];
/** The picture of a starting kind, by its plain name ("Swamp trees": `kind-swamp-trees`). */
const kindPic = (plain: string) => `kind-${plain.toLowerCase().replace(/ /g, '-')}`;
/** A tool to come: its name and its picture. */
const soon = (label: string, pic: string): SoonTool => ({ soon: true, label, pic });

/** The classes of god mode's tools, with today's tools and those to come; Plants plants one of the world's starting kinds
 * (`kinds`). */
export function godClasses(kinds: () => StartKind[]): GodClass[] {
  const [raise, lower, lake, plant, burn] = godTools(kinds);
  /** A category with no tools yet. */
  const empty = (label: string, icon: IconName, pic: string) => ({ label, icon, pic, tools: [] });
  return [
    { label: 'Human', icon: 'human', categories: [
      empty('Population', 'population', 'human-population'), empty('War', 'war', 'human-war'), empty('Culture', 'culture', 'human-culture'),
      empty('Technology', 'technology', 'human-technology'), empty('Built environment', 'built', 'human-built'),
    ] },
    { label: 'Animals', icon: 'animals', categories: [empty('Population', 'population', 'animal-population')] },
    { label: 'Nature', icon: 'nature', categories: [
      { label: 'Create', icon: 'create', tools: [plant, soon('Mountains', 'mountains'), soon('Rivers', 'rivers'), soon('Lakes', 'lakes')] },
      { label: 'Edit', icon: 'edit', tools: [raise, lower, lake, soon('Smooth land', 'smooth')] },
    ] },
    { label: 'Destruction', icon: 'destruction', categories: [
      { label: 'Fire', icon: 'fire', tools: [soon('Meteor shower', 'meteor'), soon('Volcano', 'volcano'), burn] },
      { label: 'Water', icon: 'lake', tools: [soon('Tsunami', 'tsunami'), soon('Flood', 'flood'), soon('Drought', 'water-drought')] },
      { label: 'Climate', icon: 'heatwave', tools: [
        soon('Drought', 'climate-drought'), soon('Heatwave', 'heatwave'), soon('Glacial times', 'glacial'), soon('Tornado & storm', 'storm'),
      ] },
    ] },
  ];
}

/** Today's tools: Raise and Lower land, Lake brush, Plants (of the world's starting kinds, `kinds`), and Fire. */
function godTools(kinds: () => StartKind[]): GodTool[] {
  let row = -1;
  /** The kind Plants plants: the one picked, else the first kind (`FIRST_KIND`) of the world. */
  const plantRow = () => {
    const ks = kinds();
    if (!ks.some(k => k.row === row)) row = ks.find(k => plainKind(k)[0] === FIRST_KIND)?.row ?? ks[0]?.row ?? -1;
    return row;
  };
  const plant: GodTool = {
    icon: 'plant', label: 'Plants', pic: 'plants', ink: '#7ed957', code: BRUSH_PLANT, row: plantRow, trails: true,
    hint: 'Drag on the map where to plant. The plants grow when you let go. Right-drag moves the map.', verb: 'Hold to plant',
    why: (p: Probe | null) => (!kinds().length ? 'This world has no plants to plant.' : p && p.land <= 0 ? OPEN_WATER : ''),
    reason: why => UNPLANTED[why] ?? why,
    setting: () => {
      const ks = kinds(), on = plantRow();
      return {
        name: 'Kind', look: 'picture', on: ks.findIndex(k => k.row === on), pick: i => { row = ks[i].row; },
        choices: ks.map(k => { const [label, line] = plainKind(k); return { label, title: `${label}. ${line}`, pic: kindPic(label) }; }),
      };
    },
  };
  const burn: GodTool = {
    icon: 'fire', label: 'Fire', pic: 'fire', ink: '#ff8a3d', code: BRUSH_BURN, row: () => -1, rate: () => HEAT_J_M2_S, doubling: HEAT_DOUBLING_S, doublings: HEAT_DOUBLINGS, settlesLit: true,
    hint: 'Hold on the map. The ring fills, then the ground lights.', verb: 'Hold to burn',
    why: (p: Probe | null) => (!p || p.fire === 'yes' ? '' : (UNLIT[p.fire] ?? p.fire)),
    reason: why => UNLIT[why] ?? why,
  };
  /** A ground tool: it moves the ground up or down (`sign`) under the brush while held. */
  const shape = (sign: number): GodTool => ({
    icon: sign > 0 ? 'raise' : 'lower', label: sign > 0 ? 'Raise land' : 'Lower land', pic: sign > 0 ? 'raise' : 'lower',
    ink: sign > 0 ? '#e8c46a' : '#6ab4e8', code: BRUSH_SHAPE, row: () => -1,
    rate: radius => sign * SHAPE_SPEED_PER_M * radius, tapS: SHAPE_TAP_S, mostS: SHAPE_PAINT_MOST_S, follows: true, rests: true,
    hint: `Hold on the map to ${sign > 0 ? 'raise' : 'lower'} the land. Right-drag moves the map.`, verb: `Hold to ${sign > 0 ? 'raise' : 'lower'}`,
    why: () => '', reason: why => (why === UNCHANGED ? 'No ground is left here to take away.' : why),
  });
  let deep = 1;
  const lake: GodTool = {
    icon: 'lake', label: 'Lake brush', pic: 'lake-brush', ink: '#4aa3e8', code: BRUSH_LAKE, row: () => -1, amount: () => LAKE_DEPTHS[deep][1], least: LAKE_LEAST_M,
    hint: 'Click the map to dig a lake. It fills with water at once.', verb: 'Tap to dig',
    why: (p: Probe | null) => (p && p.land <= 0 ? 'This is open water already.' : ''),
    reason: why => (why === UNCHANGED ? 'No lake can be dug here.' : why),
    setting: () => ({
      name: 'Depth', look: 'depth', on: deep, pick: i => { deep = i; },
      choices: LAKE_DEPTHS.map(([name, m]) => ({ label: `${m} m`, title: `${name}: ${m} m deep` })),
    }),
  };
  return [shape(1), shape(-1), lake, plant, burn];
}

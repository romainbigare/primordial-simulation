// The tools of god mode (god.ts `GodTool`), in the order of the strip: a share of the cell under the cursor cleared,
// the same cleared and planted with a starting kind, or a strike that lights the cell in the coming month
// (wasm/src/lib.rs `edit`). What the world cannot do there is greyed out with the reason, from the core's own test
// (`probe`: the flames' `lights`). A new tool is one more entry of `godTools`.
import type { Brush, Editor, GodTool, Probe, StartKind, Sizes } from './god';

/** The core's code of each edit (wasm/src/lib.rs `edit`). */
const ACT_CLEAR = 0, ACT_PLANT = 1, ACT_STRIKE = 2;
/** The shares of a cell a clearing may take: the stops of the brush size of Clear and Plant, the second the first on. */
const SHARES: Sizes = { stops: [0.01, 0.05, 0.1, 0.25], first: 1, text: (s, p) => `${pct(s)} of the cell${p ? `, ${hectares(s * p.area)}` : ''}` };
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
const OPEN_WATER = 'This is open water. Nothing grows here.';

const pct = (s: number) => `${Math.round(s * 100)}%`;
const plainKind = (k: StartKind): [string, string] => PLAIN_KINDS[k.name] ?? [k.name, ''];
const hectares = (m2: number) => (m2 >= 1e4 ? `${(m2 / 1e4).toFixed(m2 >= 1e5 ? 0 : 1)} ha` : `${Math.round(m2)} m²`);

/** The cell under the brush outlined, dashed: its corner and size, css px. */
function cellBox(ctx: CanvasRenderingContext2D, { ox, oy, s, probe }: Brush): [number, number, number, number] | null {
  if (!probe) return null;
  const [w, n, e, so] = probe.bounds, box: [number, number, number, number] = [ox + w * s, oy + n * s, (e - w) * s, (so - n) * s];
  ctx.setLineDash([4, 4]); ctx.lineWidth = 1; ctx.strokeStyle = 'rgba(255,255,255,.7)';
  ctx.strokeRect(...box);
  ctx.setLineDash([]);
  return box;
}

/** The patch a clearing makes, a circle of the share's area of the cell about the sample under the cursor (the patch's
 * ground is the share of the cell's samples nearest that sample, the core's `patch_ground`), cut to the cell, filled
 * with `fill`. */
function patch(ctx: CanvasRenderingContext2D, b: Brush, fill: string): void {
  const box = cellBox(ctx, b), p = b.probe;
  if (!box || !p) return;
  const [w, n, e, so] = p.bounds, r = Math.sqrt((b.size * (e - w) * (so - n)) / Math.PI) * b.s;
  ctx.beginPath(); ctx.rect(...box); ctx.clip();
  ctx.beginPath(); ctx.arc(b.ox + p.at[0] * b.s, b.oy + p.at[1] * b.s, r, 0, 2 * Math.PI);
  ctx.fillStyle = b.no ? 'rgba(120,120,120,.25)' : fill;
  ctx.fill();
  ctx.lineWidth = 2; ctx.strokeStyle = 'rgba(0,0,0,.45)'; ctx.stroke();
  ctx.lineWidth = 1.5; ctx.strokeStyle = b.ink; ctx.stroke();
}

/** Today's tools, editing through `editor`; Plant plants one of the world's starting kinds (`kinds`). */
export function godTools(editor: Editor, kinds: () => StartKind[]): GodTool[] {
  /** The share and kind of the last edit sent, for the words of its answer. */
  let sent: [number, number] = [0, -1];
  let row = -1;
  /** The kind Plant plants: the one picked, else the first kind (`FIRST_KIND`) of the world. */
  const plantRow = () => {
    const ks = kinds();
    if (!ks.some(k => k.row === row)) row = ks.find(k => plainKind(k)[0] === FIRST_KIND)?.row ?? ks[0]?.row ?? -1;
    return row;
  };
  const growsNot = (p: Probe | null) => (p && p.land <= 0 ? OPEN_WATER : '');
  const clear: GodTool = {
    key: 'clear', icon: 'clear', label: 'Clear', ink: '#fff', sizes: SHARES,
    hint: 'Click a place. The plants there die; their remains stay on the ground.',
    why: growsNot,
    edit: (x, y, share) => { sent = [share, -1]; editor.edit(ACT_CLEAR, x, y, share, -1); },
    edited: r => (r.done.every(d => d) ? `Cleared ${pct(sent[0])} of this cell. It will grow back.` : 'Nothing was cleared here.'),
    draw: (ctx, b) => patch(ctx, b, 'rgba(255,255,255,.18)'),
  };
  const plant: GodTool = {
    key: 'plant', icon: 'plant', label: 'Plant', ink: '#7ed957', sizes: SHARES,
    hint: 'Click a place. It is cleared, then seeded.',
    why: p => (kinds().length ? growsNot(p) : 'This world has no plants to plant.'),
    choices: show => {
      const box = document.createElement('div'), list = document.createElement('div'), on = plantRow();
      list.className = 'chips';
      box.innerHTML = '<p class="lbl">What to plant</p>';
      for (const k of kinds()) {
        const b = document.createElement('button');
        b.textContent = plainKind(k)[0]; b.title = plainKind(k)[1]; b.classList.toggle('on', k.row === on);
        b.onclick = () => { row = k.row; show(); };
        list.append(b);
      }
      const what = kinds().find(k => k.row === on);
      box.append(list);
      if (what) { const p = document.createElement('p'); p.className = 'hint about'; p.textContent = plainKind(what)[1]; box.append(p); }
      return box;
    },
    edit: (x, y, share) => { sent = [share, plantRow()]; editor.edit(ACT_PLANT, x, y, share, sent[1]); },
    edited: r => {
      const kind = kinds().find(k => k.row === sent[1]), name = kind ? plainKind(kind)[0].toLowerCase() : 'plants';
      return r.done.every(d => d) ? `Cleared ${pct(sent[0])} of this cell and planted ${name} in it.`
        : r.done[0] ? `Cleared ${pct(sent[0])} of this cell, but ${name} found no room to grow.` : 'Nothing was planted here.';
    },
    draw: (ctx, b) => patch(ctx, b, 'rgba(126,217,87,.22)'),
  };
  const fire: GodTool = {
    key: 'fire', icon: 'fire', label: 'Set on fire', ink: '#ff7a1a',
    hint: 'Click a place. Lightning strikes it in the coming month, and the fire spreads as far as the dry litter carries it.',
    why: p => (!p || p.fire === 'yes' ? '' : (UNLIT[p.fire] ?? p.fire)),
    edit: (x, y) => editor.edit(ACT_STRIKE, x, y, 0, -1),
    edited: r => (r.done.every(d => d) ? 'Lightning will strike here this coming month. Play to watch it burn.' : 'This place cannot burn now.'),
    // The cell that would be struck, inside the map's own outline of the square under the pointer.
    draw: (ctx, b) => {
      const box = cellBox(ctx, b);
      if (!box) return;
      const [bx, by, bw, bh] = box;
      if (!b.no) { ctx.fillStyle = 'rgba(255,122,26,.12)'; ctx.fillRect(bx, by, bw, bh); }
      ctx.lineWidth = 2.5; ctx.strokeStyle = b.ink; ctx.strokeRect(bx + 4, by + 4, bw - 8, bh - 8);
    },
  };
  return [clear, plant, fire];
}

// The controls of god mode (god.ts): its strip of tools and its card, the brush's sizes as chips (a mouse) or between a
// smaller and a larger button (a phone), the big button held for a stroke at the sight on a phone, and the reason most
// cells of a paint give.
import { IconName, iconSvg } from './icons';
import type { Brushed, GodTool, Probe } from './god-types';

/** A button of the strip: its icon, its short name, its title, and what it does. */
function stripButton(icon: IconName, label: string, title: string, act: () => void): HTMLButtonElement {
  const b = document.createElement('button');
  b.className = 'ib tool'; b.innerHTML = `${iconSvg(icon)}<span class="lb">${label}</span>`;
  b.title = title; b.setAttribute('aria-label', label); b.onclick = act;
  return b;
}

/** The strip `el` of god mode: Done (`close`), a button for each of `tools` (`pick` with it), and Undo (`undo`), which
 * it returns. */
export function makeStrip(el: HTMLElement, tools: GodTool[], close: () => void, pick: (t: GodTool) => void, undo: () => void): HTMLButtonElement {
  const done = stripButton('close', 'Done', 'Done: close god mode', close);
  done.classList.add('done');
  const undoButton = stripButton('undo', 'Undo', 'Undo the last stroke (Ctrl+Z)', undo);
  el.replaceChildren(done, ...tools.map(t => {
    const b = stripButton(t.icon, t.label, t.label, () => pick(t));
    b.dataset.tool = t.key;
    return b;
  }), undoButton);
  return undoButton;
}

/** The strip `el`'s buttons of `tools` as they stand: the tool on (`on`) pressed, a tool that cannot act at the place
 * the core answered about (`probe`) greyed with its reason; Undo (`undoButton`) greyed unless `undoable`. */
export function showStrip(el: HTMLElement, tools: GodTool[], on: GodTool | null, probe: Probe | null, undoable: boolean, undoButton: HTMLButtonElement): void {
  undoButton.disabled = !undoable;
  for (const b of el.querySelectorAll<HTMLElement>('button[data-tool]')) {
    const tool = tools.find(d => d.key === b.dataset.tool)!, why = tool.why(probe);
    b.setAttribute('aria-pressed', String(tool === on));
    b.classList.toggle('no', why !== '');
    b.title = why ? `${tool.label}: ${why}` : tool.label;
  }
}

/** The card `el` of tool `t` (null: none) at the place the core answered about (`probe`): its title, the brush's sizes
 * (`sizes`: those it may take, the one on, and what picks one), the tool's choices (`again` draws the card anew after
 * one is picked), a line of how to use it or why it cannot act there, and the button held for a stroke (`hold`). That
 * button stays in the card while a tool is on: a button taken out loses the finger on it, and its stroke would end. */
export function showCard(el: HTMLElement, hold: HTMLButtonElement, t: GodTool | null, probe: Probe | null, sizes: { radii: number[]; radius: number; pick: (m: number) => void }, again: () => void): void {
  const parts: HTMLElement[] = [];
  const line = (cls: string, text: string) => { const d = document.createElement('p'); d.className = cls; d.textContent = text; parts.push(d); };
  const title = document.createElement('h2');
  title.textContent = t?.label ?? 'God mode';
  parts.push(title);
  if (t) {
    parts.push(sizeChips(sizes.radii, sizes.radius, sizes.pick), sizeStepper(sizes.radii, sizes.radius, sizes.pick));
    if (t.choices) parts.push(t.choices(again));
    const why = t.why(probe);
    line(why ? 'why' : 'hint', why || t.hint);
    const face = `${iconSvg(t.icon)}<span>${t.verb}</span>`;
    if (hold.innerHTML !== face) hold.innerHTML = face;
    hold.style.setProperty('--tool', t.ink);
  } else line('hint', 'Pick a tool, then use it on the map.');
  for (const c of [...el.children]) if (c !== hold) c.remove();
  if (!t) hold.remove();
  else if (hold.parentElement !== el) el.append(hold);
  for (const part of parts) el.insertBefore(part, t ? hold : null);
}

/** The core's reason most of `cells` give ('' for none). */
export function mostWhy(cells: Brushed['cells']): string {
  const n = new Map<string, number>();
  for (const c of cells) if (c.why) n.set(c.why, (n.get(c.why) ?? 0) + 1);
  let best = '', most = 0;
  for (const [k, c] of n) if (c > most) { best = k; most = c; }
  return best;
}

/** A brush's radius `m` (m) in words. */
export const radiusText = (m: number) => (m >= 1000 ? `${m / 1000} km` : `${m} m`);

/** The brush's radii `radii` (m) as chips, the one on (`on`) pressed; a chip pressed calls `pick` with its radius. */
export function sizeChips(radii: number[], on: number, pick: (m: number) => void): HTMLElement {
  const list = document.createElement('div');
  list.className = 'chips sizes'; list.setAttribute('aria-label', 'Brush size');
  for (const m of radii) {
    const b = document.createElement('button');
    b.textContent = radiusText(m); b.classList.toggle('on', m === on);
    b.onclick = () => pick(m);
    list.append(b);
  }
  return list;
}

/** The brush's radius `on` (one of `radii`, m) between a smaller and a larger button, which call `pick` with the next. */
export function sizeStepper(radii: number[], on: number, pick: (m: number) => void): HTMLElement {
  const box = document.createElement('div'), k = radii.indexOf(on);
  box.className = 'stepper';
  const step = (text: string, label: string, to: number) => {
    const b = document.createElement('button');
    b.textContent = text; b.setAttribute('aria-label', label); b.disabled = to < 0 || to >= radii.length;
    b.onclick = () => pick(radii[to]);
    return b;
  };
  const size = document.createElement('b');
  size.textContent = radiusText(on);
  box.append(step('−', 'Smaller brush', k - 1), size, step('+', 'Larger brush', k + 1));
  return box;
}

/** A button held for a stroke: `start` when pressed (true: a stroke began, and the button keeps the pointer till it
 * lifts), `end` when let go or lost. */
export function holdButton(start: () => boolean, end: () => void): HTMLButtonElement {
  const b = document.createElement('button');
  b.className = 'hold';
  b.addEventListener('pointerdown', e => {
    if (!start()) return;
    b.setPointerCapture(e.pointerId); b.classList.add('held');
  });
  const up = () => { if (!b.classList.contains('held')) return; b.classList.remove('held'); end(); };
  for (const k of ['pointerup', 'pointercancel', 'lostpointercapture']) b.addEventListener(k, up);
  b.addEventListener('contextmenu', e => e.preventDefault());
  return b;
}

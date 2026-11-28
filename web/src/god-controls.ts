// The controls of god mode's menu (god-menu.ts, god-list.ts), drawn as pictures: the brush's sizes as small dots that
// grow, a tool's setting as cuts through the ground or as pictures, the pictures beside the page, the big button held
// for a stroke at the sight on a phone, and the reason most cells of a paint give.
import type { Brushed, GodSetting } from './god-types';

/** The drawn side of the smallest and of the largest brush's dot, css px: the dots between grow evenly. */
const DOT_LEAST_PX = 6, DOT_MOST_PX = 17;
/** Of a depth setting's cut through the ground, in its 40 x 24 drawing: the top of the ground, and the depth of the
 * shallowest and of the deepest cut under it. */
const CUT_TOP = 6, CUT_LEAST = 5, CUT_MOST = 16;
/** The side of the smaller copy of a setting's picture, px. */
export const KIND_SIDE = 160;
/** An ink lighter than this (its luminance, 0 to 1) takes dark words on it, a darker one white words. */
const LIGHT_INK = 0.55;

/** The picture `pic` beside the page (`god/<pic>.jpg`, or its smaller copy `god/<pic>@<side>.jpg`) as a css value. */
export const picture = (pic: string, side = 0) => `url("god/${pic}${side ? `@${side}` : ''}.jpg")`;

/** An element of tag `tag` and class `cls`, holding `html` where given. */
export function make<K extends keyof HTMLElementTagNameMap>(tag: K, cls: string, html = ''): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  el.className = cls;
  if (html) el.innerHTML = html;
  return el;
}

/** A row of a setting: its name and the value on at its left, its choices `items` at its right. */
function settingRow(name: string, value: string, items: HTMLElement[]): HTMLElement {
  const row = make('div', 'set'), head = make('span', 'head');
  head.append(make('span', 'label'), make('b', 'value'));
  head.firstChild!.textContent = name; head.lastChild!.textContent = value;
  const list = make('div', 'choices');
  list.append(...items);
  row.append(head, list);
  return row;
}

/** A button of class `cls` holding `html`, named `label` (its title and its words for a screen reader), doing `act`. */
export function tapButton(cls: string, html: string, label: string, act: () => void): HTMLButtonElement {
  const b = make('button', cls, html);
  b.type = 'button'; b.title = label; b.setAttribute('aria-label', label);
  b.onclick = act;
  return b;
}

/** A choice of a setting: its words for a screen reader, the line a mouse shows, whether it is on, and what picks it. */
function choice(cls: string, label: string, title: string, on: boolean, pick: () => void): HTMLButtonElement {
  const b = tapButton(cls, '', label, pick);
  b.title = title; b.setAttribute('aria-pressed', String(on));
  return b;
}

/** The brush's sizes in a row: the radii the tool may take (`radii`, m, of all the sizes `all`), each a dot as large as
 * its rank among all, the one on (`radius`) in the tool's ink; a dot pressed calls `pick` with its radius. */
export function sizeDots(all: number[], radii: number[], radius: number, pick: (m: number) => void): HTMLElement {
  const row = make('div', 'dots');
  row.setAttribute('role', 'group'); row.setAttribute('aria-label', `Brush size: ${radiusText(radius)}`);
  row.append(...radii.map(m => {
    const b = choice('dot', `Brush ${radiusText(m)}`, `Brush ${radiusText(m)}`, m === radius, () => pick(m));
    const d = DOT_LEAST_PX + ((DOT_MOST_PX - DOT_LEAST_PX) * all.indexOf(m)) / Math.max(all.length - 1, 1);
    b.append(make('span', ''));
    b.style.setProperty('--d', `${d}px`);
    return b;
  }));
  return row;
}

/** A tool's own setting `s`: each choice a cut through the ground, deeper than the one before, with its words under it
 * (`depth`), or its picture (`picture`); `again` shows the panel again once one is picked. */
export function ownRow(s: GodSetting, again: () => void): HTMLElement {
  const items = s.choices.map((c, i) => {
    const on = i === s.on, b = choice(s.look, c.label, c.title, on, () => { s.pick(i); again(); });
    if (s.look === 'depth') {
      const d = CUT_TOP + CUT_LEAST + ((CUT_MOST - CUT_LEAST) * i) / Math.max(s.choices.length - 1, 1);
      b.innerHTML = `<svg viewBox="0 0 40 24" aria-hidden="true"><path class="ground" d="M0 ${CUT_TOP}h40v${24 - CUT_TOP}H0z"/><path class="water" d="M6 ${CUT_TOP}C11 ${d} 29 ${d} 34 ${CUT_TOP}Z"/></svg><span></span>`;
      b.lastChild!.textContent = c.label;
    } else if (c.pic) b.style.backgroundImage = picture(c.pic, KIND_SIDE);
    return b;
  });
  return settingRow(s.name, s.choices[s.on]?.label ?? '', items);
}

/** Whether ink `ink` (#rrggbb) is light, so dark words stand on it (its luminance, ITU-R BT.709). */
export function lightInk(ink: string): boolean {
  const n = parseInt(ink.slice(1), 16);
  return (0.2126 * ((n >> 16) & 255) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255 > LIGHT_INK;
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

/** A button held for a stroke: `start` when pressed (true: a stroke began, and the button keeps the pointer till it
 * lifts), `end` when let go or lost. */
export function holdButton(start: () => boolean, end: () => void): HTMLButtonElement {
  const b = make('button', 'hold');
  b.addEventListener('pointerdown', e => {
    if (!start()) return;
    b.setPointerCapture(e.pointerId); b.classList.add('held');
  });
  const up = () => { if (!b.classList.contains('held')) return; b.classList.remove('held'); end(); };
  for (const k of ['pointerup', 'pointercancel', 'lostpointercapture']) b.addEventListener(k, up);
  b.addEventListener('contextmenu', e => e.preventDefault());
  return b;
}

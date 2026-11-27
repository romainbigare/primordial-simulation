// God mode's menu (god.ts): the rail of the tools' classes, and the panel. On a phone the rail is the dock at the bottom,
// two classes on each side of the big button held for a stroke; with a mouse it is a row over the panel, at the left.
// The panel shows the tool on (`tool`: its picture, its name and where it lives, its sizes and its setting drawn as
// pictures, why it cannot act here) or the tools of a class to pick from (`pick`: its categories as tabs, their tools as
// picture cards; a tool to come shows, dimmed, and cannot be picked). A finger swiping the panel down makes it small: one
// bar of the tool on and its settings in words, so the map shows more; swiping it up, or a tap on it, shows it whole
// again. The tools stand in the classes of god-tools.ts.
import { iconSvg } from './icons';
import { lightInk, make, ownRow, picture, radiusText, sizeRow } from './god-controls';
import type { GodCategory, GodClass, GodTool, Probe, SoonTool } from './god-types';

/** The side of the smaller copy of a tool's picture, its card's, px. */
const CARD_SIDE = 480;
/** The least move of a finger up or down the panel that makes it small or whole, px; a shorter one is a tap. */
const SWIPE_PX = 40;

/** What the menu asks of god mode: take a tool, undo the last stroke, close god mode, take a brush's radius (m). */
export interface MenuActs { pick(t: GodTool): void; undo(): void; close(): void; size(m: number): void }
/** God mode as it stands: the tool on, the core's answer about the place under the brush, the brush's radius and those
 * the tool may take of all of them (m), and whether the last stroke can be undone. */
export interface MenuState { tool: GodTool | null; probe: Probe | null; radius: number; radii: number[]; all: number[]; undoable: boolean }

/** Whether `t` is a tool of today, not one to come. */
const isTool = (t: GodTool | SoonTool): t is GodTool => !('soon' in t);

export class GodMenu {
  /** What the panel shows, the class open in it and its category (indices). */
  private mode: 'tool' | 'pick' = 'pick';
  private open: number;
  private category = 0;
  /** Whether the panel is small: one bar (`barParts`). */
  private small = false;
  /** The rail's button of each class, in order. */
  private classButtons: HTMLButtonElement[];
  /** What the panel was last built of (`show`), and the state it shows. */
  private built = '';
  private state: MenuState | null = null;

  /** `hold` starts a stroke at the sight; it stands in the rail, which is never built anew (a button taken out of the
   * page loses the finger on it, and its stroke would end). */
  constructor(private rail: HTMLElement, private panel: HTMLElement, private classes: GodClass[], private hold: HTMLButtonElement, private acts: MenuActs) {
    // (At first the first class that holds a tool is open.)
    this.open = Math.max(0, classes.findIndex(c => c.categories.some(k => k.tools.some(isTool))));
    this.classButtons = classes.map((c, i) => {
      const b = make('button', 'class', `<span class="disc">${iconSvg(c.icon)}</span>`);
      b.type = 'button'; b.append(c.label);
      b.onclick = () => this.classPressed(i);
      return b;
    });
    const half = Math.ceil(classes.length / 2);
    rail.replaceChildren(...this.classButtons.slice(0, half), hold, ...this.classButtons.slice(half));
    this.listenSwipes();
  }

  /** A finger swiping the panel down makes it small, where what it touched stands scrolled to its top (else the swipe
   * scrolls it); swiping the small bar up shows the panel whole. The tap a swipe ends with is not taken as one. */
  private listenSwipes(): void {
    let from: { x: number; y: number; top: boolean } | null = null;
    const scrolledTop = (el: EventTarget | null) => this.panel.scrollTop <= 0 && ((el as Element | null)?.closest('.cards')?.scrollTop ?? 0) <= 0;
    this.panel.addEventListener('touchstart', e => {
      const p = e.touches[0];
      from = e.touches.length === 1 ? { x: p.clientX, y: p.clientY, top: scrolledTop(e.target) } : null;
    }, { passive: true });
    this.panel.addEventListener('touchend', e => {
      const p = e.changedTouches[0], start = from;
      from = null;
      if (!start) return;
      const dy = p.clientY - start.y, dx = p.clientX - start.x;
      if (Math.abs(dy) < SWIPE_PX || Math.abs(dx) > Math.abs(dy)) return;
      if (!this.small && dy > 0 && start.top && scrolledTop(e.target)) this.small = true;
      else if (this.small && dy < 0) this.small = false;
      else return;
      e.preventDefault();
      this.again();
    });
  }

  /** Show god mode as it stands (`s`): the panel is built anew only where what it shows changed, so a button stays under
   * the finger while the brush's place changes. */
  show(s: MenuState): void {
    this.state = s;
    if (!s.tool) this.mode = 'pick';
    const t = s.tool, set = t?.setting?.(), key = JSON.stringify([this.small, this.mode, this.open, this.category, t?.label, s.radius, s.radii, set?.on, set?.choices]);
    if (key !== this.built) {
      this.built = key;
      this.panel.replaceChildren(...(this.small ? this.barParts(t) : [this.grip(), ...(this.mode === 'tool' && t ? this.toolParts(t) : this.pickParts())]));
      this.panel.classList.toggle('small', this.small);
    }
    this.classButtons.forEach((b, i) => b.setAttribute('aria-expanded', String(this.mode === 'pick' && i === this.open)));
    this.panel.querySelector<HTMLButtonElement>('.undo')?.toggleAttribute('disabled', !s.undoable);
    const why = this.panel.querySelector('.why'), where = this.panel.querySelector<HTMLElement>('.where');
    if (why && where && t) { why.textContent = t.why(s.probe); where.hidden = why.textContent !== ''; }
    this.holdFace(t);
  }

  /** Show god mode again as it stood last, after the menu's own state changed. */
  private again(): void { if (this.state) this.show(this.state); }

  /** A class pressed on the rail: its tools open, at the category of the tool on where it is of this class, else its
   * first; pressed again, the panel shows the tool on. */
  private classPressed(i: number): void {
    const place = this.placeOf(this.state?.tool);
    if (this.small) { this.small = false; this.mode = 'pick'; this.open = i; this.category = place?.[0] === i ? place[1] : 0; }
    else if (this.mode === 'pick' && i === this.open) { if (this.state?.tool) this.mode = 'tool'; }
    else { this.mode = 'pick'; this.open = i; this.category = place?.[0] === i ? place[1] : 0; }
    this.again();
  }

  /** The class and the category (indices) that hold tool `t`, where one does. */
  private placeOf(t: GodTool | null | undefined): [number, number] | null {
    for (const [i, c] of this.classes.entries()) for (const [k, cat] of c.categories.entries()) if (t && cat.tools.includes(t)) return [i, k];
    return null;
  }

  /** The big button's face: the tool's icon and words in its ink; with no tool on, a plain one that asks for one. */
  private holdFace(t: GodTool | null): void {
    const face = t ? `${iconSvg(t.icon)}<span>${t.verb}</span>` : `${iconSvg('wand')}<span>Pick a tool</span>`;
    if (this.hold.innerHTML !== face) this.hold.innerHTML = face;
    this.hold.classList.toggle('none', !t);
    this.hold.classList.toggle('light', t !== null && lightInk(t.ink));
    for (const el of [this.rail, this.panel]) if (t) el.style.setProperty('--tool', t.ink); else el.style.removeProperty('--tool');
  }

  /** The grip at the top of the whole panel (a phone's): a tap on it makes the panel small, as a swipe down does. */
  private grip(): HTMLElement {
    return this.button('grip', '<span></span>', 'Make the panel small', () => { this.small = true; this.again(); });
  }

  /** The small panel: one bar of the tool on (its icon in its ink, its name, its size and setting in words, why it
   * cannot act here), or a plain one with no tool on; a tap on it shows the panel whole. */
  private barParts(t: GodTool | null): HTMLElement[] {
    const s = this.state!, set = t?.setting?.(), bar = this.button('mini', '<span class="mark"></span><span class="words"><b></b><span class="where"></span><span class="why"></span></span>', 'Show the whole panel', () => { this.small = false; this.again(); });
    bar.querySelector('.mark')!.innerHTML = iconSvg(t?.icon ?? 'wand');
    bar.querySelector('b')!.textContent = t?.label ?? 'Pick a tool';
    bar.querySelector('.where')!.textContent = t ? [radiusText(s.radius), set?.choices[set.on]?.label].filter(Boolean).join(' · ') : 'Tap or swipe up to see the tools';
    bar.insertAdjacentHTML('beforeend', iconSvg('up'));
    return [bar];
  }

  /** The panel of tool `t`: its picture with its name, where it lives, why it cannot act here, the buttons that change
   * the tool, undo and close god mode; its sizes, its own setting, and with a mouse how to use it. */
  private toolParts(t: GodTool): HTMLElement[] {
    const s = this.state!, place = this.placeOf(t), hero = make('div', 'hero');
    hero.style.backgroundImage = picture(t.pic);
    const change = this.button('over change', `${iconSvg('grid')}Change tool`, 'Change tool', () => {
      this.mode = 'pick';
      if (place) [this.open, this.category] = place;
      this.again();
    });
    const acts = make('div', 'acts');
    acts.append(this.button('over undo', iconSvg('undo'), 'Undo the last stroke (Ctrl+Z)', () => this.acts.undo()),
      this.button('over done', iconSvg('close'), 'Done: close god mode', () => this.acts.close()));
    const plate = make('div', 'over plate');
    plate.append(make('b', ''), make('span', 'where'), make('span', 'why'));
    plate.children[0].textContent = t.label;
    plate.children[1].textContent = place ? `${this.classes[place[0]].label} › ${this.classes[place[0]].categories[place[1]].label}` : '';
    hero.append(change, acts, plate);
    const parts = [hero, sizeRow(s.all, s.radii, s.radius, m => this.acts.size(m))];
    const own = t.setting?.();
    if (own) parts.push(ownRow(own, () => this.again()));
    const hint = make('p', 'hint');
    hint.textContent = t.hint;
    return [...parts, hint];
  }

  /** The panel of the class open: its name, the button back to the tool on (or, with none, Done), its categories as
   * tabs, and the tools of the one open as picture cards, or the category's picture where it holds none yet. */
  private pickParts(): HTMLElement[] {
    const c = this.classes[this.open], cat = c.categories[this.category], t = this.state?.tool ?? null;
    const head = make('div', 'pickhead'), title = make('h2', '');
    title.textContent = c.label;
    head.append(title, t
      ? this.button('round back', iconSvg(t.icon), `Back to ${t.label}`, () => { this.mode = 'tool'; this.again(); })
      : this.button('round done', iconSvg('close'), 'Done: close god mode', () => this.acts.close()));
    const tabs = make('div', 'tabs');
    tabs.setAttribute('role', 'tablist'); tabs.setAttribute('aria-label', 'Categories');
    tabs.append(...c.categories.map((k, i) => {
      const b = this.button('tab', `${iconSvg(k.icon)}<span></span>`, k.label, () => { this.category = i; this.again(); });
      b.lastChild!.textContent = k.label; b.setAttribute('role', 'tab'); b.setAttribute('aria-selected', String(i === this.category));
      return b;
    }));
    return [head, tabs, cat.tools.length ? this.cards(cat, t) : this.empty(cat)];
  }

  /** The tools of category `cat` as picture cards, two to a row; `on` marked, a tool to come dimmed with a badge. */
  private cards(cat: GodCategory, on: GodTool | null): HTMLElement {
    const grid = make('div', 'cards');
    grid.append(...cat.tools.map(t => {
      const real = isTool(t), b = this.button('gcard', '<span class="pic"></span><b></b><small></small>', t.label, () => { if (real) { this.mode = 'tool'; this.acts.pick(t); } });
      (b.firstChild as HTMLElement).style.backgroundImage = picture(t.pic, CARD_SIDE);
      b.children[1].textContent = t.label;
      b.children[2].textContent = real ? t.verb : 'Coming soon';
      b.setAttribute('aria-pressed', String(t === on));
      if (real) b.style.setProperty('--tool', t.ink);
      else { b.disabled = true; b.title = `${t.label}: coming soon`; b.classList.add('soon'); b.firstChild!.appendChild(make('i', 'badge')).textContent = 'Soon'; }
      return b;
    }));
    return grid;
  }

  /** A category with no tools yet: its picture, and that they come soon. */
  private empty(cat: GodCategory): HTMLElement {
    const box = make('div', 'empty'), words = make('p', '');
    if (cat.pic) box.style.backgroundImage = picture(cat.pic);
    words.textContent = `Tools for ${cat.label.toLowerCase()} come soon.`;
    box.append(words);
    return box;
  }

  /** A button of class `cls` holding `html`, named `label` (its title and its words for a screen reader), doing `act`. */
  private button(cls: string, html: string, label: string, act: () => void): HTMLButtonElement {
    const b = make('button', cls, html);
    b.type = 'button'; b.title = label; b.setAttribute('aria-label', label);
    b.onclick = act;
    return b;
  }
}

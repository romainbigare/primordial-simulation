// God mode's list of all its tools (god-menu.ts), grouped by class and category, so every tool is one tap away. Its
// head is a row of the classes' icons: a tap scrolls the list to that class, and the class in view is marked as the list
// scrolls. Each tool is one slim row: its small round picture, its name, and its verb, or "Soon" for a tool to come,
// which is dimmed and cannot be picked. A category with no tools yet says so in one quiet line.
import { iconSvg } from './icons';
import { make, picture, tapButton } from './god-controls';
import type { GodClass, GodTool, SoonTool } from './god-types';

/** The side of the smaller copy of a tool's picture, px (drawn as a small disc). */
const THUMB_SIDE = 480;
/** A class counts as in view once its head is this near the list's top, px. */
const IN_VIEW_PX = 12;

/** Whether `t` is a tool of today, not one to come. */
const isTool = (t: GodTool | SoonTool): t is GodTool => !('soon' in t);

export class GodList {
  /** The sheet: the classes' icons, and the list that scrolls under them. */
  readonly el = make('div', 'gsheet glass');
  private list = make('div', 'glist');
  /** Each class's part of the list, and its icon in the head, in the order of the classes. */
  private parts: HTMLElement[];
  private jumps: HTMLButtonElement[];
  /** The row of each tool of today. */
  private rows = new Map<GodTool, HTMLButtonElement>();

  /** `pick` takes the tool of a row tapped. */
  constructor(private classes: GodClass[], pick: (t: GodTool) => void) {
    this.parts = classes.map(c => this.classPart(c, pick));
    this.jumps = classes.map((c, i) => {
      const b = tapButton('jumpto', `${iconSvg(c.icon)}<span></span>`, `Go to ${c.label}`, () => this.jump(i, true));
      b.lastChild!.textContent = c.label;
      return b;
    });
    const head = make('div', 'jump');
    head.setAttribute('role', 'group'); head.setAttribute('aria-label', 'Classes');
    head.append(...this.jumps);
    this.list.append(...this.parts);
    this.list.addEventListener('scroll', () => this.markInView(), { passive: true });
    this.el.append(head, this.list);
    this.markJump(0);
  }

  /** Mark the row of the tool on (`on`; none: no row). */
  mark(on: GodTool | null): void {
    for (const [t, b] of this.rows) b.setAttribute('aria-pressed', String(t === on));
  }

  /** Scroll the list to class `i`, smoothly or at once, and mark it. */
  jump(i: number, smooth: boolean): void {
    this.list.scrollTo({ top: this.parts[i].offsetTop, behavior: smooth ? 'smooth' : 'instant' });
    this.markJump(i);
  }

  /** The class (index) that holds tool `t`, else the first. */
  classOf(t: GodTool | null): number {
    return Math.max(0, this.classes.findIndex(c => c.categories.some(k => t !== null && k.tools.includes(t))));
  }

  /** Whether the list stands scrolled to its top, or `target` (what a finger touched) is not in it: a swipe down there
   * means the sheet, not the list. */
  atTop(target: EventTarget | null): boolean {
    return this.list.scrollTop <= 0 || !this.list.contains(target as Node | null);
  }

  /** A class's part of the list: its head (icon and name), and each category's name with its tools' rows; a category
   * with none, one quiet line (its icon, and that its tools come soon). */
  private classPart(c: GodClass, pick: (t: GodTool) => void): HTMLElement {
    const part = make('section', 'gclass'), head = make('h3', '', iconSvg(c.icon));
    head.append(c.label);
    part.append(head);
    for (const cat of c.categories) {
      if (cat.tools.length) {
        const name = make('h4', '');
        name.textContent = cat.label;
        part.append(name, ...cat.tools.map(t => this.row(t, pick)));
      } else {
        const none = make('p', 'gnone', iconSvg(cat.icon));
        none.append(`Tools for ${cat.label.toLowerCase()} come soon`);
        part.append(none);
      }
    }
    return part;
  }

  /** The row of tool `t`: its picture, its name and its verb; a tool to come dimmed, with "Soon". */
  private row(t: GodTool | SoonTool, pick: (t: GodTool) => void): HTMLButtonElement {
    const real = isTool(t);
    const b = tapButton('trow', '<span class="thumb"></span><b></b><small></small>', real ? t.label : `${t.label}: coming soon`, () => { if (real) pick(t); });
    (b.firstChild as HTMLElement).style.backgroundImage = picture(t.pic, THUMB_SIDE);
    b.children[1].textContent = t.label;
    b.children[2].textContent = real ? t.verb : 'Soon';
    if (real) { b.style.setProperty('--tool', t.ink); this.rows.set(t, b); } else { b.disabled = true; b.classList.add('soon'); }
    return b;
  }

  /** Mark the class in view: the last whose head reached the list's top, or the last class once the list is at its end. */
  private markInView(): void {
    const l = this.list, end = l.scrollTop + l.clientHeight >= l.scrollHeight - 1;
    let i = 0;
    this.parts.forEach((p, k) => { if (p.offsetTop - l.scrollTop <= IN_VIEW_PX) i = k; });
    this.markJump(end && l.scrollTop > 0 ? this.parts.length - 1 : i);
  }

  /** Mark class `i`'s icon in the head as the one in view. */
  private markJump(i: number): void {
    this.jumps.forEach((b, k) => b.setAttribute('aria-current', String(k === i)));
  }
}

// God mode's menu (god.ts): one slim bar, and above it, when asked, the list of all the tools (god-list.ts). The bar
// shows the tool on (its icon in its ink and its name: a tap opens the list), Undo and Done, and under them the brush's
// sizes as small dots and the tool's own setting as a chip, whose choices open in a small card above the bar. On a phone
// the big button held for a stroke stands at the bar's right end, rising over its top edge. The list opens by itself
// when no tool is on; picking a tool closes it. A finger swiping the list down closes it, where the list stands scrolled
// to its top; swiping the bar up opens it.
import { iconSvg } from './icons';
import { KIND_SIDE, lightInk, make, ownRow, picture, sizeDots, tapButton } from './god-controls';
import { GodList } from './god-list';
import type { GodClass, GodSetting, GodTool, Probe } from './god-types';

/** The least move of a finger up or down that opens or closes the list, px; a shorter one is a tap. */
const SWIPE_PX = 40;

/** What the menu asks of god mode: take a tool, undo the last stroke, close god mode, take a brush's radius (m). */
export interface MenuActs { pick(t: GodTool): void; undo(): void; close(): void; size(m: number): void }
/** God mode as it stands: the tool on, the core's answer about the place under the brush, the brush's radius and those
 * the tool may take of all of them (m), and whether the last stroke can be undone. */
export interface MenuState { tool: GodTool | null; probe: Probe | null; radius: number; radii: number[]; all: number[]; undoable: boolean }

export class GodMenu {
  /** Whether the list of tools is open, and the card of the tool's setting. */
  private listing = false;
  private choosing = false;
  /** What the bar was last built of (`show`), the state it shows, and the tool on before it (undefined: none shown). */
  private built = '';
  private state: MenuState | null = null;
  private had: GodTool | null | undefined = undefined;
  /** The bar, and in it how to use the tool with a mouse; the reason the tool cannot act here; the card of the tool's
   * setting. */
  private bar = make('div', 'gbar glass');
  private why = make('p', 'gwhy glass');
  private choices = make('div', 'gpop glass');
  private hint = make('p', 'ghint');
  private list: GodList;

  /** `hold` starts a stroke at the sight; it stands in `holdBox`, which is never built anew (a button taken out of the
   * page loses the finger on it, and its stroke would end). */
  constructor(holdBox: HTMLElement, private panel: HTMLElement, classes: GodClass[], private hold: HTMLButtonElement, private acts: MenuActs) {
    holdBox.replaceChildren(hold);
    this.list = new GodList(classes, t => { this.listing = false; this.acts.pick(t); this.again(); });
    panel.replaceChildren(this.bar, this.why, this.choices, this.list.el);
    this.listenSwipes();
  }

  /** A finger swiping the list down closes it, where what it touched stands scrolled to its top (else the swipe scrolls
   * the list); swiping the bar up opens it. The tap a swipe ends with is not taken as one. */
  private listenSwipes(): void {
    let from: { x: number; y: number; top: boolean } | null = null;
    this.panel.addEventListener('touchstart', e => {
      const p = e.touches[0];
      from = e.touches.length === 1 ? { x: p.clientX, y: p.clientY, top: this.list.atTop(e.target) } : null;
    }, { passive: true });
    this.panel.addEventListener('touchend', e => {
      const p = e.changedTouches[0], start = from;
      from = null;
      if (!start) return;
      const dy = p.clientY - start.y, dx = p.clientX - start.x;
      if (Math.abs(dy) < SWIPE_PX || Math.abs(dx) > Math.abs(dy)) return;
      if (this.listing && dy > 0 && start.top && this.list.atTop(e.target)) this.listing = false;
      else if (!this.listing && dy < 0 && this.bar.contains(e.target as Node)) this.listing = true;
      else return;
      e.preventDefault();
      this.opened();
    });
  }

  /** Show god mode as it stands (`s`): the bar is built anew only where what it shows changed, so a button stays under
   * the finger while the brush's place changes. With no tool on, the list opens. */
  show(s: MenuState): void {
    this.state = s;
    const t = s.tool, set = t?.setting?.();
    const lost = !t && this.had !== null;
    if (lost) { this.listing = true; this.choosing = false; }
    this.had = t;
    const key = JSON.stringify([this.listing, this.choosing, t?.label, s.radius, s.radii, set?.on, set?.choices]);
    if (key !== this.built) {
      this.built = key;
      this.bar.replaceChildren(...this.barLines(t, set));
      this.choices.replaceChildren(...(set && this.choosing ? [ownRow(set, () => { this.choosing = false; this.again(); })] : []));
      this.choices.hidden = !this.choosing;
      this.list.el.hidden = !this.listing;
      this.panel.classList.toggle('listing', this.listing);
      this.list.mark(t);
      this.hint.textContent = t?.hint ?? 'Pick a tool in the list.';
      if (lost) this.list.jump(0, false);
    }
    this.bar.querySelector<HTMLButtonElement>('.undo')?.toggleAttribute('disabled', !s.undoable);
    this.why.textContent = t?.why(s.probe) ?? '';
    this.holdFace(t);
  }

  /** Show god mode again as it stood last, after the menu's own state changed. */
  private again(): void { if (this.state) this.show(this.state); }

  /** Show the list opened or closed (`listing`); an opened one stands at the class of the tool on. */
  private opened(): void {
    this.choosing = false;
    this.again();
    if (this.listing) this.list.jump(this.list.classOf(this.state?.tool ?? null), false);
  }

  /** The big button's face: the tool's icon and words in its ink; with no tool on, a plain one that asks for one. */
  private holdFace(t: GodTool | null): void {
    const face = t ? `${iconSvg(t.icon)}<span>${t.verb}</span>` : `${iconSvg('wand')}<span>Pick a tool</span>`;
    if (this.hold.innerHTML !== face) this.hold.innerHTML = face;
    this.hold.classList.toggle('none', !t);
    this.hold.classList.toggle('light', t !== null && lightInk(t.ink));
    for (const el of [this.hold, this.panel]) if (t) el.style.setProperty('--tool', t.ink); else el.style.removeProperty('--tool');
  }

  /** The bar's two lines: the tool on (a tap opens or closes the list), Undo and Done; under them its sizes and its
   * setting, or with no tool on a line that says where the tools are. Then how to use it (with a mouse alone). */
  private barLines(t: GodTool | null, set: GodSetting | undefined): HTMLElement[] {
    const name = tapButton(`gtool${t ? '' : ' none'}`, `<span class="disc">${iconSvg(t?.icon ?? 'wand')}</span><b></b>${iconSvg(this.listing ? 'down' : 'up')}`,
      this.listing ? 'Hide the tools' : 'All tools', () => { this.listing = !this.listing; this.opened(); });
    name.querySelector('b')!.textContent = t?.label ?? 'Pick a tool';
    name.setAttribute('aria-expanded', String(this.listing));
    const top = make('div', 'gline'), low = make('div', 'gline');
    top.append(name, tapButton('gact undo', iconSvg('undo'), 'Undo the last stroke (Ctrl+Z)', () => this.acts.undo()),
      tapButton('gact done', iconSvg('close'), 'Done: close god mode', () => this.acts.close()));
    const s = this.state!;
    if (t) low.append(sizeDots(s.all, s.radii, s.radius, m => this.acts.size(m)), ...(set ? [this.chip(set)] : []));
    else low.append(make('span', 'gnote', this.listing ? 'Tap a tool in the list' : 'Tap the name to see the tools'));
    return [top, low, this.hint];
  }

  /** The chip of setting `set`: its name (or the picture of the choice on) and the choice on; a tap opens or closes the
   * card of its choices (and closes the list). */
  private chip(set: GodSetting): HTMLButtonElement {
    const on = set.choices[set.on], pic = set.look === 'picture' ? on?.pic : undefined;
    const b = tapButton('gchip', '<b></b>', `${set.name}: ${on?.label ?? ''}`, () => {
      this.choosing = !this.choosing;
      if (this.choosing) this.listing = false;
      this.again();
    });
    b.lastChild!.textContent = on?.label ?? '';
    const lead = make(pic ? 'span' : 'small', pic ? 'pic' : '');
    if (pic) lead.style.backgroundImage = picture(pic, KIND_SIDE); else lead.textContent = set.name;
    b.prepend(lead);
    b.setAttribute('aria-expanded', String(this.choosing));
    return b;
  }
}

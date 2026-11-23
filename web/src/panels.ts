// The floating panels of the top bar: a button that names a panel (`data-panel="name"`) opens the panel
// `#panel-name` under itself (a sheet from the bottom on a phone, style.css). One is open at a time; it closes on a
// second click of its button, on its close button, on Escape (main.ts) and when the map is touched, which still works
// as the map.

export class Panels {
  private shown: HTMLElement | null = null;
  private button: HTMLElement | null = null;

  constructor(bar: HTMLElement, map: HTMLElement) {
    bar.querySelectorAll<HTMLElement>('[data-panel]').forEach(b => {
      b.setAttribute('aria-expanded', 'false');
      b.addEventListener('click', () => this.toggle(b));
    });
    document.querySelectorAll<HTMLElement>('.panel [data-close]').forEach(b => b.addEventListener('click', () => this.close()));
    map.addEventListener('pointerdown', () => this.close());
    addEventListener('resize', () => this.place());
  }

  /** Open the panel of button `b`, or close it if it is open. */
  private toggle(b: HTMLElement): void {
    const was = this.button;
    this.close();
    if (was === b) return;
    const panel = document.getElementById(`panel-${b.dataset.panel}`);
    if (!panel) return;
    this.shown = panel; this.button = b;
    b.setAttribute('aria-expanded', 'true');
    this.place();
    panel.classList.add('open');
  }

  /** Close the open panel: true if one was open. */
  close(): boolean {
    if (!this.shown) return false;
    this.shown.classList.remove('open');
    this.button?.setAttribute('aria-expanded', 'false');
    this.shown = this.button = null;
    return true;
  }

  /** The open panel's left edge under its button, kept inside the window (its css `--x`). */
  private place(): void {
    const p = this.shown, b = this.button;
    if (!p || !b) return;
    const r = b.getBoundingClientRect(), margin = parseFloat(getComputedStyle(p).getPropertyValue('--edge')) || 0;
    p.style.setProperty('--x', `${Math.max(margin, Math.min(r.left, innerWidth - p.offsetWidth - margin))}px`);
  }
}

// Sparklines for the summary series, each with a cursor at the current year.
import { fmt, unitText } from './data';

interface Card { name: string; xs: number[]; ys: number[]; cv: HTMLCanvasElement; cur: HTMLElement; val: HTMLElement; unit: string }

export class Charts {
  private cards: Card[] = [];
  private x0 = 0; private x1 = 1; private year = 0;

  constructor(private root: HTMLElement, private onSeek: (year: number) => void) {
    new ResizeObserver(() => this.draw()).observe(root);
  }

  set(series: Record<string, number[]>, units: Record<string, string>, years: number[]): void {
    this.root.replaceChildren();
    const ys = series.year ?? years;
    this.x0 = Math.min(ys[0] ?? 0, years[0] ?? 0); this.x1 = Math.max(ys[ys.length - 1] ?? 1, years[years.length - 1] ?? 1);
    this.cards = Object.keys(series).filter(k => k !== 'year' && series[k].length > 1).map(name => {
      const el = document.createElement('div');
      el.className = 'card';
      el.innerHTML = `<div class="hd"><span></span><b></b></div><div class="plot"><canvas></canvas><i></i></div>`;
      el.querySelector('span')!.textContent = name.replace(/_/g, ' ');
      const plot = el.querySelector<HTMLElement>('.plot')!;
      const seek = (e: PointerEvent) => { if (e.buttons) this.onSeek(this.x0 + clamp01(e.offsetX / plot.clientWidth) * (this.x1 - this.x0)); };
      plot.addEventListener('pointerdown', e => { plot.setPointerCapture(e.pointerId); seek(e); });
      plot.addEventListener('pointermove', seek);
      this.root.append(el);
      return { name, xs: series.year && series.year.length === series[name].length ? series.year : series[name].map((_, i, a) => this.x0 + (i / (a.length - 1)) * (this.x1 - this.x0)),
        ys: series[name], cv: el.querySelector('canvas')!, cur: el.querySelector('i')!, val: el.querySelector('b')!, unit: unitText(units[name]) };
    });
    this.draw(); this.setYear(this.year);
  }

  setYear(year: number): void {
    this.year = year;
    for (const c of this.cards) {
      let lo = 0, hi = c.xs.length - 1; // last sample at or before the year
      while (lo < hi) { const m = (lo + hi + 1) >> 1; if (c.xs[m] <= year) lo = m; else hi = m - 1; }
      c.val.textContent = `${fmt(c.ys[lo])} ${c.unit}`.trim();
      c.cur.style.left = `${clamp01((year - this.x0) / (this.x1 - this.x0 || 1)) * 100}%`;
    }
  }

  draw(): void {
    const css = getComputedStyle(document.documentElement), line = css.getPropertyValue('--accent'), d = window.devicePixelRatio || 1;
    for (const c of this.cards) {
      const W = c.cv.clientWidth, H = c.cv.clientHeight;
      if (!W) continue;
      c.cv.width = W * d; c.cv.height = H * d;
      const g = c.cv.getContext('2d')!;
      g.scale(d, d);
      const lo = c.ys.reduce((a, b) => Math.min(a, b)), hi = c.ys.reduce((a, b) => Math.max(a, b)), pad = 4, span = hi - lo || 1;
      const px = (i: number) => ((c.xs[i] - this.x0) / (this.x1 - this.x0 || 1)) * W;
      const py = (v: number) => (hi === lo ? H / 2 : H - pad - ((v - lo) / span) * (H - 2 * pad));
      g.beginPath();
      c.ys.forEach((v, i) => (i ? g.lineTo(px(i), py(v)) : g.moveTo(px(i), py(v))));
      g.strokeStyle = line; g.lineWidth = 1.5; g.lineJoin = 'round'; g.stroke();
      g.lineTo(px(c.ys.length - 1), H); g.lineTo(px(0), H); g.closePath();
      g.globalAlpha = 0.12; g.fillStyle = line; g.fill();
    }
  }
}
const clamp01 = (x: number) => Math.min(Math.max(x, 0), 1);

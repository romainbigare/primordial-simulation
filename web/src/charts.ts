// Sparklines for the summary series, each with a cursor at the current year. A missing value (null: a NaN the
// tape wrote, a mean of nothing) is a gap in the line and prints as a dash, never as 0.
import { fmt, missing, unitText } from './data';

interface Card { name: string; xs: number[]; ys: (number | null)[]; cv: HTMLCanvasElement; cur: HTMLElement; val: HTMLElement; unit: string }

export class Charts {
  private cards: Card[] = [];
  private x0 = 0; private x1 = 1; private year = 0;

  constructor(private root: HTMLElement, private onSeek: (year: number) => void) {
    new ResizeObserver(() => this.draw()).observe(root);
  }

  set(series: Record<string, (number | null)[]>, units: Record<string, string>, years: number[]): void {
    this.root.replaceChildren();
    const xs = series.year as number[];
    this.x0 = Math.min(xs[0] ?? 0, years[0] ?? 0); this.x1 = Math.max(xs[xs.length - 1] ?? 1, years[years.length - 1] ?? 1);
    this.cards = Object.keys(series).filter(k => k !== 'year' && !/^elem_|^energy$/.test(k) && series[k].length > 1).map(name => {
      const el = document.createElement('div');
      el.className = 'card';
      el.innerHTML = `<div class="hd"><span></span><b></b></div><div class="plot"><canvas></canvas><i></i></div>`;
      el.querySelector('span')!.textContent = name.replace(/_/g, ' ');
      const plot = el.querySelector<HTMLElement>('.plot')!;
      const seek = (e: PointerEvent) => { if (e.buttons) this.onSeek(this.x0 + clamp01(e.offsetX / plot.clientWidth) * (this.x1 - this.x0)); };
      plot.addEventListener('pointerdown', e => { plot.setPointerCapture(e.pointerId); seek(e); });
      plot.addEventListener('pointermove', seek);
      this.root.append(el);
      return { name, xs, ys: series[name], cv: el.querySelector('canvas')!, cur: el.querySelector('i')!, val: el.querySelector('b')!, unit: unitText(units[name]) };
    });
    this.draw(); this.setYear(this.year);
  }

  setYear(year: number): void {
    this.year = year;
    for (const c of this.cards) {
      let lo = 0, hi = c.xs.length - 1; // last sample at or before the year
      while (lo < hi) { const m = (lo + hi + 1) >> 1; if (c.xs[m] <= year) lo = m; else hi = m - 1; }
      const v = c.ys[lo];
      c.val.textContent = missing(v) ? fmt(v) : `${fmt(v)} ${c.unit}`.trim();
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
      const known = c.ys.filter((v): v is number => !missing(v));
      if (!known.length) continue;
      const lo = known.reduce((a, b) => Math.min(a, b)), hi = known.reduce((a, b) => Math.max(a, b)), pad = 4, span = hi - lo || 1;
      const px = (i: number) => ((c.xs[i] - this.x0) / (this.x1 - this.x0 || 1)) * W;
      const py = (v: number) => (hi === lo ? H / 2 : H - pad - ((v - lo) / span) * (H - 2 * pad));
      g.strokeStyle = g.fillStyle = line; g.lineWidth = 1.5; g.lineJoin = 'round';
      // Each run of known values is its own line with its own fill; a lone value is a dot.
      let run: number[] = [];
      const flush = () => {
        if (run.length === 1) { g.globalAlpha = 1; g.beginPath(); g.arc(px(run[0]), py(c.ys[run[0]]!), 1.5, 0, 2 * Math.PI); g.fill(); }
        else if (run.length > 1) {
          g.globalAlpha = 1; g.beginPath();
          run.forEach((i, k) => (k ? g.lineTo(px(i), py(c.ys[i]!)) : g.moveTo(px(i), py(c.ys[i]!))));
          g.stroke();
          g.lineTo(px(run[run.length - 1]), H); g.lineTo(px(run[0]), H); g.closePath();
          g.globalAlpha = 0.12; g.fill();
        }
        run = [];
      };
      c.ys.forEach((v, i) => (missing(v) ? flush() : run.push(i)));
      flush();
      g.globalAlpha = 1;
    }
  }
}
const clamp01 = (x: number) => Math.min(Math.max(x, 0), 1);

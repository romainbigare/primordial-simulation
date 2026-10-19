// Sparklines for the summary series, each with a cursor at the current year, grouped in sections by name and
// filtered by a search. A missing value (null: a NaN the tape wrote, a mean of nothing) is a gap in the line and prints
// as a dash, never as 0.
import { fmt, missing, unitText } from './data';

interface Card { name: string; xs: number[]; ys: (number | null)[]; el: HTMLElement; cv: HTMLCanvasElement; cur: HTMLElement; val: HTMLElement; unit: string }
interface Section { title: string; el: HTMLDetailsElement; cards: Card[]; open: boolean }

/** Sections in display order: the first pattern a column name matches decides (tape/src/rec/cols/ names). */
const SECTIONS: [string, RegExp][] = [
  ['Climate', /^(t_|absorber$|ice_frac$|liquid_land$|rain|rivers$)/],
  ['Air gases', /^air_/],
  ['Minerals', /^min\d/],
  ['Dead matter', /^dead_/],
  ['Population', /^(bodies|plants|plant_members|animal_|cover_|body_mass|lineages|updates|kinds|mass_(min|median|max)|trophic_|eat_animals)/],
  ['Births and deaths', /^(births|died_)/],
  ['Brains', /^(learned|greedy|hidden|memory|seen|sight|mi_)/],
  ['Movement and paths', /^(crossings|traffic_|worn_|mass_top2|mass_land)/],
  ['Imitation and giving', /^(watched|spread_max|gave|ate_)/],
  ['Parts and structures', /^(parts|put_to_parts|taken_from_parts|bonds_|structures|enclosed|made_|inside_|cold_months|holding_liquid|liquid_held|afloat)/],
  ['Catalysts', /^cat_/],
];
const KIND = /^root_(\d+)_/;
const OPEN_FIRST = 2; // sections open when no search is active

/** Section title and card title of a column. */
function sectionOf(name: string): [string, string] {
  const kind = KIND.exec(name);
  if (kind) return [`Kind ${kind[1]}`, name.slice(kind[0].length)];
  return [SECTIONS.find(([, re]) => re.test(name))?.[0] ?? 'Other', name];
}
const sectionRank = (t: string) => {
  const i = SECTIONS.findIndex(([n]) => n === t);
  return i >= 0 ? i : t.startsWith('Kind ') ? SECTIONS.length + 1 + Number(t.slice(5)) / 1e6 : 1e9;
};

export class Charts {
  private cards: Card[] = [];
  private sections: Section[] = [];
  private query = '';
  private raf = 0;
  private x0 = 0; private x1 = 1; private year = 0;

  constructor(private root: HTMLElement, private onSeek: (year: number) => void) {
    new ResizeObserver(() => this.draw()).observe(root);
  }

  set(series: Record<string, (number | null)[]>, units: Record<string, string>, years: number[]): void {
    this.root.replaceChildren();
    const xs = series.year as number[];
    this.x0 = Math.min(xs[0] ?? 0, years[0] ?? 0); this.x1 = Math.max(xs[xs.length - 1] ?? 1, years[years.length - 1] ?? 1);
    const byTitle = new Map<string, Card[]>();
    this.cards = Object.keys(series).filter(k => k !== 'year' && !/^elem_|^energy$/.test(k) && series[k].length > 1).map(name => {
      const [section, title] = sectionOf(name);
      const el = document.createElement('div');
      el.className = 'card';
      el.innerHTML = `<div class="hd"><span></span><b></b></div><div class="plot"><canvas></canvas><i></i></div>`;
      el.querySelector('span')!.textContent = title.replace(/_/g, ' ');
      const plot = el.querySelector<HTMLElement>('.plot')!;
      const seek = (e: PointerEvent) => { if (e.buttons) this.onSeek(this.x0 + clamp01(e.offsetX / plot.clientWidth) * (this.x1 - this.x0)); };
      plot.addEventListener('pointerdown', e => { plot.setPointerCapture(e.pointerId); seek(e); });
      plot.addEventListener('pointermove', seek);
      const card = { name, xs, ys: series[name], el, cv: el.querySelector('canvas')!, cur: el.querySelector('i')!, val: el.querySelector('b')!, unit: unitText(units[name]) };
      byTitle.set(section, [...(byTitle.get(section) ?? []), card]);
      return card;
    });
    this.sections = [...byTitle].sort((a, b) => sectionRank(a[0]) - sectionRank(b[0])).map(([title, cards], k) => {
      const el = document.createElement('details');
      el.className = 'sec';
      el.innerHTML = '<summary></summary><div class="grid"></div>';
      el.querySelector('summary')!.textContent = `${title} (${cards.length})`;
      el.querySelector('.grid')!.append(...cards.map(c => c.el));
      el.addEventListener('toggle', () => this.later());
      this.root.append(el);
      return { title, el, cards, open: k < OPEN_FIRST };
    });
    this.filter(this.query);
    this.setYear(this.year);
  }

  /** Show the charts whose name (or section) holds every word of the query; with a query, matching sections open. */
  filter(query: string): void {
    this.query = query;
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    for (const s of this.sections) {
      let shown = 0;
      for (const c of s.cards) {
        const text = `${s.title} ${c.name}`.toLowerCase().replace(/_/g, ' ');
        c.el.hidden = !words.every(w => text.includes(w));
        if (!c.el.hidden) shown++;
      }
      s.el.hidden = !shown;
      s.el.open = words.length ? shown > 0 : s.open;
    }
    this.draw();
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

  /** Draw once after a burst of section toggles. */
  private later(): void { if (!this.raf) this.raf = requestAnimationFrame(() => { this.raf = 0; this.draw(); }); }

  draw(): void {
    const css = getComputedStyle(document.documentElement), line = css.getPropertyValue('--accent'), d = window.devicePixelRatio || 1;
    for (const c of this.cards) {
      if (c.el.hidden) continue;
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

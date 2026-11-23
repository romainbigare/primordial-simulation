// The live world's numbers in the World panel: the date, the pace, the cells, the ground's tiles; and, at a click, the
// cost line of where the time and the memory go.
import { Meta, fmt } from './data';
import type { HeldGpu } from './gpu';
import type { Land } from './land';
import type { Stats } from './live';
import { spent } from './cost';
import { detailFailure, detailSent } from './detail';

/** What the numbers are of: the world's header, whether it plays and at what speed (months/s), its live map, the GPU
 * that draws it and the share of the screen's pixels it is drawn at. */
export interface Shown { meta: Meta; playing: boolean; speed: number; land: Land | null; drawing: HeldGpu | null; drawnAt: number }

export class Numbers {
  /** Whether the cost line shows. */
  private costShown = false;

  /** The numbers in `stats`, the cost line in `cost`; `again` shows them anew after the cost button is clicked. */
  constructor(private stats: HTMLElement, private cost: HTMLElement, again: () => void) {
    stats.onclick = e => {
      if ((e.target as HTMLElement).id === 'costs') { this.costShown = !this.costShown; again(); }
    };
  }

  /** The date, the pace, the cells and the ground's tiles kept and still to come in view (nothing without `at`). */
  show(s: Stats, at: Shown | null): void {
    const el = this.stats;
    if (!at) { el.hidden = true; return; }
    const { meta: m, land, drawing } = at, y = m.frames[0].year, day = Math.floor((y - Math.floor(y)) * m.days_per_year) + 1;
    el.hidden = false;
    el.innerHTML = [['date', `year ${Math.floor(y)}, day ${day}`], ['pace', `${at.playing ? fmt(s.pace) : '0'} of ${at.speed === Infinity ? 'max' : at.speed} months/s`],
      ['cells', String(s.cells)],
      ...(land ? [['ground', `${land.tiles.count()} tiles of ${fmt(m.cell_m / 2 ** (land.tiles.depth - (m.square_depth ?? 0)))} m, ${land.tiles.toCome()} to come`]] : []),
      ...(drawing ? [['map', `drawn at ${fmt(at.drawnAt)}`]] : [])].map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('')
      + `<button class="costs" id="costs">${this.costShown ? 'hide cost' : 'cost'}</button>`;
    this.showCost(s, at);
  }

  /** The cost line: where the time and the memory go, over the last whole second, so a screenshot of it tells where a
   * slow device spends: the world's worker (its months, the time a month takes, the share of its time it steps), the
   * page's thread (the world's frames taken, the map's draws), the GPU's time a frame, the ground's tiles (made in
   * their worker, taken on the page), the detail sent to the GPU, and the memory of the GPU's textures by what they
   * hold, of each worker's core and of the page's script where the browser says it. */
  private showCost(s: Stats, { land, drawing }: Shown): void {
    const el = this.cost;
    el.hidden = !this.costShown;
    if (!this.costShown) return;
    const ms = (v: number) => v.toFixed(v < 10 ? 1 : 0), mb = (bytes: number) => (bytes / 2 ** 20).toFixed(bytes < 2 ** 23 ? 1 : 0);
    const per = (key: string) => `${ms(spent(key))} ms on ${spent(`${key} n`).toFixed(1)}`;
    const [sent, layers, side] = detailSent(), gpuN = spent('gpu n');
    const heap = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize;
    const cards = drawing?.memory() ?? [], cardAll = cards.reduce((n, [, b]) => n + b, 0);
    el.innerHTML = [
      ['world', `${fmt(s.pace)} months/s, ${ms(s.monthMs)} ms a month, worker busy ${Math.round(100 * s.busy)}%`],
      ['page /s', `world frames ${per('world frames')}, draws ${per('draw')}`],
      ['gpu', `${gpuN > 0 ? ms(spent('gpu') / gpuN) : '-'} ms a frame; programs: ${drawing?.making() ?? '-'}`],
      ['tiles /s', `${ms(spent('tile ms'))} ms in the worker, ${per('tiles')} on the page`],
      ['detail', `layers of ${side} texels; ${sent} of ${layers} sent, ${per('detail')} frames /s${detailFailure() ? `; failed: ${detailFailure()}` : ''}`],
      ['gpu MB', `${mb(cardAll)}: ${cards.map(([label, b]) => `${label} ${mb(b)}`).join(', ')}`],
      ['wasm MB', `world ${mb(s.memory)}${land ? `, tiles ${land.tiles.memory().map(mb).join(' + ')}` : ''}${heap ? `; js ${mb(heap)}` : ''}`],
    ].map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('');
  }
}

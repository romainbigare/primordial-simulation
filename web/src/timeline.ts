// The timeline of a tape: its first and last year, a mark at each of its events (a tip with its words; a click goes
// to it), and the frame nearest a year.
import { Meta, fmtYear } from './data';

const byId = (id: string) => document.getElementById(id)!;

/** The frame of `meta` nearest the year `year`. */
export function nearestFrame(meta: Meta, year: number): number {
  const ys = meta.frames.map(f => f.year);
  return ys.reduce((best, y, i) => (Math.abs(y - year) < Math.abs(ys[best] - year) ? i : best), 0);
}

/** The timeline of the tape of `meta`; a click on an event's mark goes to its frame (`seek`). */
export function setTimeline(meta: Meta, seek: (frame: number) => void): void {
  const years = meta.frames.map(f => f.year), last = years.length - 1, tl = byId('tl') as HTMLInputElement;
  tl.max = String(last); tl.disabled = last < 1;
  byId('y0').textContent = `year ${fmtYear(years[0])}`; byId('y1').textContent = `year ${fmtYear(years[last])}`;
  byId('marks').replaceChildren(...meta.events.map(ev => {
    const i = nearestFrame(meta, ev.year), el = document.createElement('i');
    el.style.setProperty('--p', String(last ? i / last : 0));
    const text = `Year ${fmtYear(ev.year)}: ${ev.text}`;
    el.onmouseenter = () => tip(el, text); el.onmouseleave = () => (byId('tip').hidden = true);
    el.onclick = () => { seek(i); tip(el, text); };
    return el;
  }));
}

/** The words `text` in the tip over the element `at`. */
function tip(at: HTMLElement, text: string): void {
  const t = byId('tip'), r = at.getBoundingClientRect();
  t.textContent = text; t.hidden = false;
  t.style.left = `${Math.min(Math.max(r.left + r.width / 2 - t.offsetWidth / 2, 8), innerWidth - t.offsetWidth - 8)}px`;
  t.style.top = `${r.top - t.offsetHeight - 8}px`;
}

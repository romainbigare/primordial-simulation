// The form of a new live world (the World panel): its seed, size, variety, climate, shape and weather, first from the
// page's URL (`?seed=1&size=32&variety=8&band=20,65&shape=drawn&weather=strong`), each kept within what the core runs;
// and the files the page opens: a saved world, or a tape picked or dropped on it.

/** The live world's first size and its least and most, squares a side: the first shows a land with its valleys, a
 * lake and an outlier; the most is the largest island the laws are run on, or on a screen touched with a finger (a
 * phone's, whose browser gives a page a few hundred MB) the largest whose workers fit there (each of the ground's
 * workers holds about 0.4 kB a sample: 110 MB at 32, 250 MB at 48). */
const LIVE_SIZE = 32, SIZE_LEAST = 8, SIZE_MOST = matchMedia('(pointer: coarse)').matches ? 48 : 128;
/** The live world's kinds of cover a cell may hold, a setting of the world (the core's `Config::variety`), and its
 * most (the core's `cover::SLOTS`). */
const LIVE_VARIETY = 8, VARIETY_MOST = 12;
/** The climates a live island may have: a name, and the band of latitudes it stands for (south, north), its air the
 * planet's air over that band. The first is the page's. */
const BANDS: string[][] = [['Wide: 20° to 65°', '20,65'], ['Mild: 30° to 55°', '30,55'], ['Warm: 5° to 50°', '5,50']];
/** The large shapes a live island may have: a name, and the URL's word for it. The first is the page's. */
const SHAPES: string[][] = [['Drawn from the seed', 'drawn'], ['One island', 'one']];
/** The weathers a live island may have: a name, and the URL's word for it (`ew run --weather`). The first is the
 * page's: strong, the eddies' measured speed; soft, the speed their diffusion alone gives (the core's `soft_weather`). */
const WEATHERS: string[][] = [['Strong', 'strong'], ['Soft', 'soft']];

const field = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

/** What the form asks of a new world (live.ts `Live.newWorld`). */
export interface WorldAsked { seed: number; size: number; band: [number, number]; drawn: boolean; variety: number; soft: boolean }

/** Fill the form from the URL's query `q`, else the page's first choices. */
export function fillForm(q: URLSearchParams): void {
  field<HTMLInputElement>('seed').value = q.get('seed') ?? '1';
  Object.assign(field<HTMLInputElement>('size'), { min: String(SIZE_LEAST), max: String(SIZE_MOST), value: q.get('size') ?? String(LIVE_SIZE) });
  Object.assign(field<HTMLInputElement>('variety'), { min: '1', max: String(VARIETY_MOST), value: q.get('variety') ?? String(LIVE_VARIETY) });
  // The climate: a band of latitudes the island stands for, from the URL's `band=S,N` or the first preset (`BANDS`).
  const bands = field<HTMLSelectElement>('band'), asked = q.get('band');
  const presets = asked && !BANDS.some(b => b[1] === asked) ? [...BANDS, [`${asked.replace(',', '° to ')}°`, asked]] : BANDS;
  bands.innerHTML = presets.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  bands.value = asked ?? BANDS[0][1];
  // The large shape: from the URL's `shape=one|drawn` or the first (`SHAPES`).
  const shapes = field<HTMLSelectElement>('shape');
  shapes.innerHTML = SHAPES.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  shapes.value = SHAPES.some(s => s[1] === q.get('shape')) ? (q.get('shape') as string) : SHAPES[0][1];
  // The weather: from the URL's `weather=strong|soft` or the first (`WEATHERS`).
  const weathers = field<HTMLSelectElement>('weather');
  weathers.innerHTML = WEATHERS.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  weathers.value = WEATHERS.some(s => s[1] === q.get('weather')) ? (q.get('weather') as string) : WEATHERS[0][1];
}

/** The band of latitudes `S,N`, degrees, south first; [NaN, NaN] (the core's default band, its edges closed) for none,
 * or where S is not below N. */
function bandOf(text: string | null): [number, number] {
  const v = (text ?? '').split(',').map(Number);
  return v.length === 2 && v[0] < v[1] ? [v[0], v[1]] : [NaN, NaN];
}

/** The world the form asks for, each number kept within its least and most. */
export function readForm(): WorldAsked {
  return {
    seed: Math.max(0, Math.floor(+field<HTMLInputElement>('seed').value) || 0),
    size: Math.min(SIZE_MOST, Math.max(SIZE_LEAST, Math.floor(+field<HTMLInputElement>('size').value) || LIVE_SIZE)),
    variety: Math.min(VARIETY_MOST, Math.max(1, Math.floor(+field<HTMLInputElement>('variety').value) || LIVE_VARIETY)),
    band: bandOf(field<HTMLSelectElement>('band').value),
    drawn: field<HTMLSelectElement>('shape').value === 'drawn',
    soft: field<HTMLSelectElement>('weather').value === 'soft',
  };
}

/** A new world (`fresh`) as the form is sent or its choices change; a saved world (`open`, its gzip) as one is picked;
 * a tape (`tape`, its gzip) as one is picked or dropped on the page. */
export function listenForm(fresh: () => void, open: (gz: ArrayBuffer) => void, tape: (gz: ArrayBuffer) => void): void {
  const picked = (to: (gz: ArrayBuffer) => void) => (e: Event) => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) f.arrayBuffer().then(to); };
  field<HTMLInputElement>('saved').onchange = picked(open);
  field<HTMLInputElement>('file').onchange = picked(tape);
  field<HTMLFormElement>('live').onsubmit = e => { e.preventDefault(); fresh(); };
  for (const id of ['band', 'shape', 'weather']) field<HTMLSelectElement>(id).onchange = () => fresh();
  // (Entering and leaving the page's elements while dragging: the drop zone shows while the drag is over the page.)
  let depth = 0;
  const zone = field('drop');
  addEventListener('dragenter', e => { e.preventDefault(); depth++; zone.hidden = false; });
  addEventListener('dragleave', () => { if (--depth <= 0) zone.hidden = true; });
  addEventListener('dragover', e => e.preventDefault());
  addEventListener('drop', e => { e.preventDefault(); depth = 0; zone.hidden = true; e.dataTransfer?.files[0]?.arrayBuffer().then(tape); });
}

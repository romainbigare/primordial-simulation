// Textures of the plant cover (vegetation.ts): for each kind of pattern a tile that repeats, made once, at a few
// sizes. A tile holds the pattern's marks (crowns, clumps or tufts) twice, split by a seeded rank: the marks of an open
// cover and the extra ones that close it. Each is a coverage (where a mark lies, its cast shadow half covered) and a
// shading (light from the north-west on the mark, its shadow dark) drawn over its colour. From above a crown is a soft
// round blob; angled, it is lifted over its shadow, its lower side in shade.
import { Style, seeded } from './sprites';

/** The kinds of pattern the cover is drawn as. */
export type Kind = 'grass' | 'steppe' | 'tundra' | 'shrubs' | 'woodland' | 'forest' | 'jungle';
/** A tile: per level (0: the marks of an open cover, 1: the extra marks that close it) its coverage and shading, each
 * at sizes halving from the largest; the pixels a mark takes across at the largest size, and its side, px. */
export interface Tile { cov: HTMLCanvasElement[][]; shade: HTMLCanvasElement[][]; mark: number; side: number }

/** The side of a tile at its smallest size, px. */
const SMALLEST_PX = 16;
/** The share of a pattern's marks an open cover holds (the rest close it). */
export const OPEN_SHARE = 0.4;
/** Of each kind: the side of its tile at the largest size and the pixels a mark takes across there (some 40 to 80
 * marks along a side, so the tile's repeat does not show), the spacing of the marks over their size, and whether a
 * mark is a crown (round, with a shadow) or a tuft (fine, flat). */
const SPEC: Record<Kind, { tile: number; mark: number; spacing: number; crown: boolean }> = {
  grass: { tile: 512, mark: 6, spacing: 0.9, crown: false }, steppe: { tile: 512, mark: 8, spacing: 1.6, crown: false },
  tundra: { tile: 512, mark: 8, spacing: 1.1, crown: false }, shrubs: { tile: 1024, mark: 18, spacing: 1.1, crown: true },
  woodland: { tile: 1024, mark: 20, spacing: 1.25, crown: true }, forest: { tile: 1024, mark: 22, spacing: 0.78, crown: true },
  jungle: { tile: 1024, mark: 22, spacing: 0.72, crown: true },
};
/** The share of a jungle's crowns that stand over the rest, and how much wider they are. */
const EMERGENT = 0.15, EMERGENT_WIDE = 1.7;
/** An angled crown is lifted over its shadow by this share of its radius. */
const ANGLED_LIFT = 0.7;

const made = new Map<string, Tile>();

/** The tile of `kind` in `style`, made the first time it is asked for. */
export function tileOf(kind: Kind, style: Style): Tile {
  const key = `${kind}:${style}`;
  let t = made.get(key);
  if (!t) { t = makeTile(kind, style); made.set(key, t); }
  return t;
}

/** The size of tile `t` to draw a mark `px` pixels across from: the smallest whose marks are not smaller than that. */
export function sizeFor(t: Tile, px: number): number {
  return Math.min(Math.max(Math.floor(Math.log2(t.mark / Math.max(px, 1e-6))), 0), t.cov[0].length - 1);
}

const canvas = (side: number) => { const c = document.createElement('canvas'); c.width = c.height = side; return c; };
/** The sizes of a picture halving down to `SMALLEST_PX`, each drawn from the one before. */
function halvings(c: HTMLCanvasElement): HTMLCanvasElement[] {
  const out = [c];
  for (let side = c.width / 2; side >= SMALLEST_PX; side /= 2) {
    const h = canvas(side), x = h.getContext('2d')!;
    x.imageSmoothingEnabled = true; x.imageSmoothingQuality = 'high';
    x.drawImage(out[out.length - 1], 0, 0, side, side);
    out.push(h);
  }
  return out;
}

/** A mark of a tile: its place, radius and turn, px, and its level. */
interface Mark { x: number; y: number; r: number; turn: number; level: number; tone: number }

/** Make the tile of `kind` in `style`: its marks on a jittered grid, drawn back to front, each also across the tile's
 * edges so it repeats without a seam. */
function makeTile(kind: Kind, style: Style): Tile {
  const spec = SPEC[kind], side = spec.tile, r = seeded(kind.length * 7919 + (style === 'angled' ? 1 : 0)), step = spec.mark * spec.spacing;
  const marks: Mark[] = [];
  const cells = Math.round(side / step), cell = side / cells;
  for (let j = 0; j < cells; j++) for (let i = 0; i < cells; i++) {
    const wide = kind === 'jungle' && r() < EMERGENT ? EMERGENT_WIDE : 1;
    marks.push({ x: (i + 0.5 + (r() - 0.5) * 0.8) * cell, y: (j + 0.5 + (r() - 0.5) * 0.8) * cell, r: (spec.mark / 2) * (0.75 + 0.5 * r()) * wide, turn: r() * Math.PI, level: r() < OPEN_SHARE ? 0 : 1, tone: r() });
  }
  marks.sort((a, b) => a.y - b.y);
  const cov = [canvas(side), canvas(side)], shade = [canvas(side), canvas(side)];
  for (const m of marks) for (const dx of [-side, 0, side]) for (const dy of [-side, 0, side]) {
    const x = m.x + dx, y = m.y + dy;
    if (x + 3 * m.r < 0 || y + 3 * m.r < 0 || x - 3 * m.r > side || y - 3 * m.r > side) continue;
    const c = cov[m.level].getContext('2d')!, s = shade[m.level].getContext('2d')!;
    if (spec.crown) crown(c, s, { ...m, x, y }, style);
    else tuft(c, s, { ...m, x, y }, style);
  }
  return { cov: cov.map(halvings), shade: shade.map(halvings), mark: spec.mark, side };
}

/** A crown at mark `m` into coverage `c` and shading `s`: its shadow cast to the south-east (half covered, dark), then
 * the crown, a soft-edged disc, lit from the north-west and shaded at its far rim; angled, lifted over its shadow. */
function crown(c: CanvasRenderingContext2D, s: CanvasRenderingContext2D, m: Mark, style: Style): void {
  const lift = style === 'angled' ? m.r * ANGLED_LIFT : 0, cy = m.y - lift;
  const [sx, sy] = style === 'angled' ? [m.x + m.r * 0.35, m.y + m.r * 0.15] : [m.x + m.r * 0.3, m.y + m.r * 0.3];
  for (const [ctx, fill] of [[c, 'rgba(0,0,0,.5)'], [s, 'rgba(0,0,0,.3)']] as const) {
    ctx.fillStyle = fill; ctx.beginPath(); ctx.ellipse(sx, sy, m.r * 1.05, m.r * (style === 'angled' ? 0.6 : 0.95), 0, 0, 2 * Math.PI); ctx.fill();
  }
  const soft = c.createRadialGradient(m.x, cy, m.r * 0.75, m.x, cy, m.r);
  soft.addColorStop(0, 'rgba(0,0,0,1)'); soft.addColorStop(1, 'rgba(0,0,0,0)');
  c.fillStyle = soft; c.beginPath(); c.arc(m.x, cy, m.r, 0, 2 * Math.PI); c.fill();
  // The shading under the crown is its own, not the shadow's under it.
  s.globalCompositeOperation = 'destination-out'; s.fillStyle = '#000'; s.beginPath(); s.arc(m.x, cy, m.r * 0.9, 0, 2 * Math.PI); s.fill();
  s.globalCompositeOperation = 'source-over';
  const light = s.createRadialGradient(m.x - m.r * 0.35, cy - m.r * 0.4, 0, m.x - m.r * 0.1, cy - m.r * 0.1, m.r * 1.1);
  const dark = style === 'angled' ? 0.3 : 0.2;
  light.addColorStop(0, `rgba(255,255,255,${0.14 + 0.1 * m.tone})`); light.addColorStop(0.5, 'rgba(255,255,255,0)'); light.addColorStop(1, `rgba(0,0,0,${dark})`);
  s.fillStyle = light; s.beginPath(); s.arc(m.x, cy, m.r, 0, 2 * Math.PI); s.fill();
}

/** A tuft at mark `m` into coverage `c` and shading `s`: from above a small turned blob of a few blades, angled a few
 * blades standing up; each a little lighter or darker by its seed. */
function tuft(c: CanvasRenderingContext2D, s: CanvasRenderingContext2D, m: Mark, style: Style): void {
  const shadeOf = `rgba(${m.tone < 0.5 ? '0,0,0' : '255,255,255'},${Math.abs(m.tone - 0.5) * 0.5})`;
  for (const [ctx, fill] of [[c, 'rgba(0,0,0,.9)'], [s, shadeOf]] as const) {
    ctx.fillStyle = fill;
    ctx.beginPath();
    if (style === 'angled') for (const lean of [-0.5, 0, 0.5]) {
      ctx.moveTo(m.x - m.r * 0.25, m.y); ctx.lineTo(m.x + m.r * 0.25, m.y); ctx.lineTo(m.x + lean * m.r, m.y - m.r * 1.4); ctx.closePath();
    }
    else ctx.ellipse(m.x, m.y, m.r, m.r * 0.55, m.turn, 0, 2 * Math.PI);
    ctx.fill();
  }
}

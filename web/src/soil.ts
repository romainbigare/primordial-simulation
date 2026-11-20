// The soil's texture classes for the page's legend and read-out: the twelve classes of the soil texture triangle by
// the shares of sand (coarse grain) and clay (fine grain), silt the rest (Soil Survey Staff 1993). Names for people
// only: the world knows the shares, never a class.

/** A class: its name, where its name is written and its corners, as (sand %, clay %). */
interface TextureClass { name: string; label: [number, number]; corners: [number, number][] }
const TEXTURE_CLASSES: TextureClass[] = [
  { name: 'sand', label: [91, 3], corners: [[85, 0], [100, 0], [90, 10]] },
  { name: 'loamy sand', label: [80, 7], corners: [[70, 0], [85, 0], [90, 10], [85, 15]] },
  { name: 'sandy loam', label: [62, 11], corners: [[50, 0], [70, 0], [85, 15], [80, 20], [52, 20], [52, 7], [43, 7]] },
  { name: 'loam', label: [40, 18], corners: [[43, 7], [52, 7], [52, 20], [45, 27], [23, 27]] },
  { name: 'silt loam', label: [22, 14], corners: [[50, 0], [20, 0], [8, 12], [0, 12], [0, 27], [23, 27]] },
  { name: 'silt', label: [7, 5], corners: [[20, 0], [0, 0], [0, 12], [8, 12]] },
  { name: 'sandy clay loam', label: [58, 27], corners: [[45, 27], [52, 20], [80, 20], [65, 35], [45, 35]] },
  { name: 'clay loam', label: [32, 33], corners: [[20, 27], [45, 27], [45, 40], [20, 40]] },
  { name: 'silty clay loam', label: [10, 33], corners: [[0, 27], [20, 27], [20, 40], [0, 40]] },
  { name: 'sandy clay', label: [51, 41], corners: [[45, 35], [65, 35], [45, 55]] },
  { name: 'silty clay', label: [6, 47], corners: [[0, 40], [20, 40], [0, 60]] },
  { name: 'clay', label: [22, 60], corners: [[20, 40], [45, 40], [45, 55], [0, 100], [0, 60]] },
];

/** Whether point (x, y) lies inside the polygon `p` (even-odd rule; a point on an edge counts for one of its sides). */
function inPolygon(p: [number, number][], x: number, y: number): boolean {
  let at = false;
  for (let i = 0, j = p.length - 1; i < p.length; j = i++) {
    const [xi, yi] = p[i], [xj, yj] = p[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) at = !at;
  }
  return at;
}

/** The name of the class of ground whose shares of sand and clay are `sand` and `clay` (0..1). */
export function textureClass(sand: number, clay: number): string {
  const s = 100 * sand, c = Math.min(100 * clay, 99.999);
  return (TEXTURE_CLASSES.find(k => inPolygon(k.corners, s, c)) ?? TEXTURE_CLASSES[TEXTURE_CLASSES.length - 1]).name;
}

/** The legend's side, px, and the height of its triangle (equal sides). */
const LEGEND_SIDE = 220, LEGEND_PAD = 14, LEGEND_HIGH = (LEGEND_SIDE * Math.sqrt(3)) / 2;
/** Where (sand %, clay %) lies in the legend: sand's corner at the bottom left, silt's at the bottom right, clay's on top. */
function legendPlace(sand: number, clay: number): [number, number] {
  const silt = 100 - sand - clay;
  return [LEGEND_PAD + (LEGEND_SIDE * (silt + clay / 2)) / 100, LEGEND_PAD + LEGEND_HIGH * (1 - clay / 100)];
}
/** A class's fill: the colours of sand and clay of the layers' ramps mixed by its middle's shares (earth.ts SANDY,
 * CLAYEY ends), towards a silt grey. */
function classFill(k: TextureClass): string {
  const n = k.corners.length, s = k.corners.reduce((a, c) => a + c[0], 0) / n, c = k.corners.reduce((a, q) => a + q[1], 0) / n;
  const mix = (sand: number[], silt: number[], clay: number[]) => sand.map((v, i) => Math.round((v * s + silt[i] * (100 - s - c) + clay[i] * c) / 100));
  return `rgb(${mix([238, 214, 150], [196, 190, 172], [168, 82, 62])})`;
}

/** The legend: the triangle with its classes and their names, and a mark where the place `at` (sand, clay; 0..1)
 * lies, where given. SVG text. */
export function textureLegend(at: [number, number] | null): string {
  const poly = (k: TextureClass) => k.corners.map(c => legendPlace(c[0], c[1]).map(v => v.toFixed(1)).join(',')).join(' ');
  // The classes first, then their names over them all.
  const classes = TEXTURE_CLASSES.map(k => `<polygon points="${poly(k)}" fill="${classFill(k)}"/>`).join('') + TEXTURE_CLASSES.map(k => {
    const [x, y] = legendPlace(...k.label);
    return `<text x="${x.toFixed(1)}" y="${y.toFixed(1)}">${k.name}</text>`;
  }).join('');
  const [mx, my] = at ? legendPlace(100 * at[0], 100 * at[1]) : [0, 0];
  const mark = at ? `<circle class="at" cx="${mx.toFixed(1)}" cy="${my.toFixed(1)}" r="4"/>` : '';
  const [lx, ly] = legendPlace(100, 0), [rx, ry] = legendPlace(0, 0), [tx, ty] = legendPlace(0, 100);
  const corners = `<text class="end left" x="${lx}" y="${ly + 11}">all sand</text><text class="end right" x="${rx}" y="${ry + 11}">all silt</text><text class="end" x="${tx}" y="${ty - 4}">all clay</text>`;
  return `<svg viewBox="0 0 ${LEGEND_SIDE + 2 * LEGEND_PAD} ${LEGEND_HIGH + 2 * LEGEND_PAD + 6}" role="img" aria-label="Soil texture classes">${classes}${corners}${mark}</svg>`;
}
